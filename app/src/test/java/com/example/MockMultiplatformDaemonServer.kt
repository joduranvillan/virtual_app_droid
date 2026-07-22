package com.example

import com.southernstorm.noise.protocol.HandshakeState
import com.vault.crypto.generateStaticKeyPair
import com.vault.net.*
import java.net.InetAddress
import java.net.ServerSocket
import java.net.Socket
import java.util.concurrent.ConcurrentHashMap
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.Executors

/**
 * Servidor de pruebas que simula un daemon multiplataforma (Linux/crosvm, Windows/Hyper-V, macOS/Virtualization.framework)
 * escuchando conexiones TCP, ejecutando el handshake Noise_XX como RESPONDER, respondiendo
 * a solicitudes de enrolamiento por QR y procesando comandos RPC (Admin, Input, Services).
 */
class MockMultiplatformDaemonServer(
    val platformName: String = "Linux-crosvm"
) {
    val keyPair: Pair<ByteArray, ByteArray> = generateStaticKeyPair()
    val privateKeyBytes: ByteArray get() = keyPair.first
    val publicKeyBytes: ByteArray get() = keyPair.second

    private var serverSocket: ServerSocket? = null
    val port: Int get() = serverSocket?.localPort ?: -1

    private val executor = Executors.newCachedThreadPool()
    @Volatile var isRunning = false

    val receivedConfirmTokens = CopyOnWriteArrayList<String>()
    val receivedInputEvents = CopyOnWriteArrayList<InputEventPayload>()
    val receivedAdminRequests = CopyOnWriteArrayList<AdminRequestPayload>()
    val receivedServiceResponses = CopyOnWriteArrayList<ServiceResponseEnvelope>()

    var enrollmentExpectedToken: String = "valid-secret-token-12345"
    var acceptEnrollment: Boolean = true

    fun start() {
        serverSocket = ServerSocket(0, 50, InetAddress.getByName("127.0.0.1"))
        isRunning = true
        executor.execute {
            while (isRunning) {
                try {
                    val socket = serverSocket?.accept() ?: break
                    executor.execute { handleClient(socket) }
                } catch (e: Exception) {
                    if (!isRunning) break
                }
            }
        }
    }

    fun stop() {
        isRunning = false
        try {
            serverSocket?.close()
        } catch (_: Exception) {}
        executor.shutdownNow()
    }

    fun createQrPayload(expiresInMs: Long = 60_000): EnrollmentQrPayload {
        val hexPub = publicKeyBytes.joinToString("") { "%02x".format(it) }
        return EnrollmentQrPayload(
            v = 1,
            runtimePubkeyHex = hexPub,
            host = "127.0.0.1",
            port = port,
            token = enrollmentExpectedToken,
            expiresUnixMs = System.currentTimeMillis() + expiresInMs
        )
    }

    private fun handleClient(socket: Socket) {
        socket.use { sock ->
            try {
                // Noise_XX RESPONDER handshake
                val hs = HandshakeState("Noise_XX_25519_ChaChaPoly_SHA256", HandshakeState.RESPONDER)
                hs.localKeyPair.setPrivateKey(privateKeyBytes, 0)
                hs.start()

                val inp = sock.getInputStream()
                val out = sock.getOutputStream()
                val payloadBuf = ByteArray(1024)
                val sendBuf = ByteArray(1024)

                // 1. <- e (read from initiator)
                val msg1 = readLenPrefixed(inp)
                hs.readMessage(msg1, 0, msg1.size, payloadBuf, 0)

                // 2. -> e, ee, s, es (write to initiator)
                val len2 = hs.writeMessage(sendBuf, 0, null, 0, 0)
                writeLenPrefixed(out, sendBuf.copyOf(len2))

                // 3. <- s, se (read from initiator)
                val msg3 = readLenPrefixed(inp)
                hs.readMessage(msg3, 0, msg3.size, payloadBuf, 0)

                check(hs.action == HandshakeState.SPLIT) { "Daemon mock responder handshake falló" }
                val ciphers = hs.split()
                val channel = VaultChannel(inp, out, ciphers)

                // Loop de recepción de frames
                while (isRunning) {
                    val frame = try {
                        channel.recvFrame()
                    } catch (e: Exception) {
                        break
                    }

                    when (frame.msgType) {
                        MsgType.ENROLLMENT_CONFIRM -> {
                            val confirm = EnrollmentConfirmBody.decodeCbor(frame.payload)
                            receivedConfirmTokens.add(confirm.token)

                            val success = acceptEnrollment && (confirm.token == enrollmentExpectedToken)
                            val ackBody = EnrollmentAckBody(
                                success = success,
                                reason = if (success) null else "Token inválido o denegado por daemon $platformName"
                            )
                            val ackFrame = Frame(MsgType.ENROLLMENT_ACK, frame.reqId, ackBody.encodeCbor())
                            channel.sendFrame(ackFrame)
                        }

                        MsgType.ADMIN_REQUEST -> {
                            val req = AdminRequestPayload.decodeCbor(frame.payload)
                            receivedAdminRequests.add(req)

                            val logs = listOf(
                                "[$platformName] Kernel initialized",
                                "[$platformName] Hypervisor status: ACTIVE",
                                "[$platformName] Executed RPC action: ${req.action.wireName}"
                            )
                            val resp = AdminResponsePayload(
                                success = true,
                                message = "Daemon $platformName completó la acción ${req.action.wireName}",
                                logs = logs
                            )
                            val respFrame = Frame(MsgType.ADMIN_RESPONSE, frame.reqId, resp.encodeCbor())
                            channel.sendFrame(respFrame)
                        }

                        MsgType.INPUT_EVENT -> {
                            val inputEvent = InputEventPayload.decodeCbor(frame.payload)
                            receivedInputEvents.add(inputEvent)
                        }

                        MsgType.SERVICE_RESPONSE -> {
                            val serviceResp = ServiceResponseEnvelope.decodeCbor(frame.payload)
                            receivedServiceResponses.add(serviceResp)
                        }

                        MsgType.HEARTBEAT -> {
                            channel.sendFrame(Frame(MsgType.HEARTBEAT, frame.reqId, ByteArray(0)))
                        }

                        else -> {}
                    }
                }

            } catch (e: Exception) {
                // error de socket o de test
            }
        }
    }

    /** Permite al daemon mock enviar una solicitud de servicio RPC hacia Android */
    fun sendServiceRequestToClient(
        channel: VaultChannel,
        reqId: Long,
        service: ServiceId,
        packageName: String,
        body: ByteArray
    ) {
        val envelope = ServiceRequestEnvelope(service, packageName, body)
        val frame = Frame(MsgType.SERVICE_REQUEST, reqId, envelope.encodeCbor())
        channel.sendFrame(frame)
    }
}
