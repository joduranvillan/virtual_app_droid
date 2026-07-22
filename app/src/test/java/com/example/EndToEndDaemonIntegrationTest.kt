package com.example

import com.vault.crypto.generateStaticKeyPair
import com.vault.net.*
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.Job
import kotlinx.coroutines.delay
import kotlinx.coroutines.runBlocking
import org.junit.After
import org.junit.Assert.*
import org.junit.Before
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [36])
class EndToEndDaemonIntegrationTest {

    private lateinit var mockDaemonLinux: MockMultiplatformDaemonServer
    private lateinit var mockDaemonWindows: MockMultiplatformDaemonServer
    private lateinit var mockDaemonMacOS: MockMultiplatformDaemonServer

    private val clientKeyPair = generateStaticKeyPair()
    private val clientPrivateKey = clientKeyPair.first
    private val clientPublicKey = clientKeyPair.second

    @Before
    fun setUp() {
        mockDaemonLinux = MockMultiplatformDaemonServer("Linux-crosvm")
        mockDaemonLinux.start()

        mockDaemonWindows = MockMultiplatformDaemonServer("Windows-HyperV")
        mockDaemonWindows.start()

        mockDaemonMacOS = MockMultiplatformDaemonServer("macOS-VirtualizationFramework")
        mockDaemonMacOS.start()
    }

    @After
    fun tearDown() {
        mockDaemonLinux.stop()
        mockDaemonWindows.stop()
        mockDaemonMacOS.stop()
    }

    @Test
    fun testEnrollmentFlow_Success() {
        val qrPayload = mockDaemonLinux.createQrPayload(expiresInMs = 60_000)

        val result = EnrollmentClient.enroll(qrPayload, clientPrivateKey)

        assertTrue("El enrolamiento debe ser exitoso", result is EnrollmentResult.Success)
        assertEquals(1, mockDaemonLinux.receivedConfirmTokens.size)
        assertEquals(mockDaemonLinux.enrollmentExpectedToken, mockDaemonLinux.receivedConfirmTokens[0])
    }

    @Test
    fun testEnrollmentFlow_ExpiredQR() {
        val expiredQr = mockDaemonLinux.createQrPayload(expiresInMs = -1_000)

        val result = EnrollmentClient.enroll(expiredQr, clientPrivateKey)

        assertTrue("El enrolamiento con QR expirado debe retornar QrExpired", result is EnrollmentResult.QrExpired)
        assertEquals(0, mockDaemonLinux.receivedConfirmTokens.size)
    }

    @Test
    fun testEnrollmentFlow_PubkeyMismatch() {
        val qrPayload = mockDaemonLinux.createQrPayload(expiresInMs = 60_000)
        // Alterar la clave pública remota esperada en el payload QR
        val fakeKeyHex = clientPublicKey.joinToString("") { "%02x".format(it) }
        val corruptedQr = qrPayload.copy(runtimePubkeyHex = fakeKeyHex)

        val result = EnrollmentClient.enroll(corruptedQr, clientPrivateKey)

        assertTrue("Clave pública no coincidente debe retornar PubkeyMismatch", result is EnrollmentResult.PubkeyMismatch)
    }

    @Test
    fun testEnrollmentFlow_RejectedToken() {
        mockDaemonLinux.acceptEnrollment = false
        val qrPayload = mockDaemonLinux.createQrPayload(expiresInMs = 60_000)

        val result = EnrollmentClient.enroll(qrPayload, clientPrivateKey)

        assertTrue("Token rechazado por el daemon debe retornar Rejected", result is EnrollmentResult.Rejected)
    }

    @Test
    fun testEndToEndConnectionAndHandshake_LinuxDaemon() = runBlocking {
        val pinnedPubkey = mockDaemonLinux.publicKeyBytes
        val manager = VaultConnectionManager(
            host = "127.0.0.1",
            port = mockDaemonLinux.port,
            localStaticPrivateKey = clientPrivateKey,
            pinnedRemotePublicKey = pinnedPubkey
        )

        val connectedLatch = CountDownLatch(1)
        var lastStatus = ""
        var isConnectedState = false

        manager.onStatusChange = { status -> lastStatus = status }
        manager.onConnectionState = { state ->
            isConnectedState = state
            if (state) connectedLatch.countDown()
        }

        val job = Job()
        val scope = CoroutineScope(Dispatchers.IO + job)
        manager.connectAndServe(scope)

        val connected = connectedLatch.await(3, TimeUnit.SECONDS)
        assertTrue("La conexión e handshake deben completarse en <3s", connected)
        assertTrue("El estado de conexión debe ser true", isConnectedState)
        assertTrue("El estado debe indicar Handshake exitoso", lastStatus.contains("Handshake exitoso"))

        job.cancel()
    }

    @Test
    fun testEndToEndConnection_MitmPublicKeyMismatch() = runBlocking {
        val wrongPinnedKey = clientPublicKey // Usa la clave del cliente en vez de la del daemon
        val manager = VaultConnectionManager(
            host = "127.0.0.1",
            port = mockDaemonLinux.port,
            localStaticPrivateKey = clientPrivateKey,
            pinnedRemotePublicKey = wrongPinnedKey
        )

        var isConnectedState = true
        var statusMsg = ""
        val latch = CountDownLatch(1)

        manager.onStatusChange = { msg -> statusMsg = msg }
        manager.onConnectionState = { state ->
            if (!state) {
                isConnectedState = false
                latch.countDown()
            }
        }

        val job = Job()
        val scope = CoroutineScope(Dispatchers.IO + job)
        manager.connectAndServe(scope)

        latch.await(3, TimeUnit.SECONDS)
        assertFalse("Debe rechazar la conexión si el pin de la clave remota no coincide", isConnectedState)
        assertTrue("Mensaje debe advertir de descalce/MITM", statusMsg.contains("MITM") || statusMsg.contains("no coincide"))

        job.cancel()
    }

    @Test
    fun testRpcAdminCommands_GetLogsAndReboot() = runBlocking {
        val manager = VaultConnectionManager(
            host = "127.0.0.1",
            port = mockDaemonLinux.port,
            localStaticPrivateKey = clientPrivateKey,
            pinnedRemotePublicKey = mockDaemonLinux.publicKeyBytes
        )

        val responseLatch = CountDownLatch(1)
        var receivedAdminResp: AdminResponsePayload? = null

        manager.onAdminResponseReceived = { resp ->
            receivedAdminResp = resp
            responseLatch.countDown()
        }

        val connectedLatch = CountDownLatch(1)
        manager.onConnectionState = { if (it) connectedLatch.countDown() }

        val job = Job()
        val scope = CoroutineScope(Dispatchers.IO + job)
        manager.connectAndServe(scope)

        assertTrue("Servidor debe conectarse", connectedLatch.await(3, TimeUnit.SECONDS))

        // Enviar AdminRequest de GetLogs
        val adminReq = AdminRequestPayload(AdminActionType.GET_LOGS)
        manager.sendAdminRequest(adminReq)

        val gotResponse = responseLatch.await(3, TimeUnit.SECONDS)
        assertTrue("Debe recibir AdminResponse del daemon en <3s", gotResponse)
        assertNotNull(receivedAdminResp)
        assertTrue(receivedAdminResp!!.success)
        assertTrue(receivedAdminResp!!.message.contains("Linux-crosvm"))
        assertEquals(3, receivedAdminResp!!.logs.size)

        // Verificación en el lado del Daemon Mock
        assertEquals(1, mockDaemonLinux.receivedAdminRequests.size)
        assertEquals(AdminActionType.GET_LOGS, mockDaemonLinux.receivedAdminRequests[0].action)

        job.cancel()
    }

    @Test
    fun testRpcInputEvents_TouchAndKey() = runBlocking {
        val manager = VaultConnectionManager(
            host = "127.0.0.1",
            port = mockDaemonLinux.port,
            localStaticPrivateKey = clientPrivateKey,
            pinnedRemotePublicKey = mockDaemonLinux.publicKeyBytes
        )

        val connectedLatch = CountDownLatch(1)
        manager.onConnectionState = { if (it) connectedLatch.countDown() }

        val job = Job()
        val scope = CoroutineScope(Dispatchers.IO + job)
        manager.connectAndServe(scope)

        assertTrue(connectedLatch.await(3, TimeUnit.SECONDS))

        // Enviar eventos de entrada
        val touchDown = InputEventPayload.TouchDown(pointerId = 0, x = 100.5f, y = 200.5f, timestampUnixMs = System.currentTimeMillis())
        val keyEvent = InputEventPayload.Key(keycode = 66, pressed = true, timestampUnixMs = System.currentTimeMillis())

        manager.sendInputEvent(touchDown)
        manager.sendInputEvent(keyEvent)

        delay(300) // tiempo para que los frames viajen por el socket loopback

        assertEquals(2, mockDaemonLinux.receivedInputEvents.size)
        assertTrue("Primer evento debe ser TouchDown", mockDaemonLinux.receivedInputEvents[0] is InputEventPayload.TouchDown)
        val receivedTouch = mockDaemonLinux.receivedInputEvents[0] as InputEventPayload.TouchDown
        assertEquals(100.5f, receivedTouch.x, 0.01f)
        assertEquals(200.5f, receivedTouch.y, 0.01f)

        assertTrue("Segundo evento debe ser Key", mockDaemonLinux.receivedInputEvents[1] is InputEventPayload.Key)
        val receivedKey = mockDaemonLinux.receivedInputEvents[1] as InputEventPayload.Key
        assertEquals(66, receivedKey.keycode)
        assertTrue(receivedKey.pressed)

        job.cancel()
    }

    @Test
    fun testMultiplatformDaemons_WindowsAndMacOS() = runBlocking {
        // Test con Windows Hyper-V
        val managerWin = VaultConnectionManager(
            host = "127.0.0.1",
            port = mockDaemonWindows.port,
            localStaticPrivateKey = clientPrivateKey,
            pinnedRemotePublicKey = mockDaemonWindows.publicKeyBytes
        )
        val winLatch = CountDownLatch(1)
        var winResp: AdminResponsePayload? = null
        managerWin.onAdminResponseReceived = {
            winResp = it
            winLatch.countDown()
        }
        val winConnLatch = CountDownLatch(1)
        managerWin.onConnectionState = { if (it) winConnLatch.countDown() }

        val jobWin = Job()
        managerWin.connectAndServe(CoroutineScope(Dispatchers.IO + jobWin))
        assertTrue(winConnLatch.await(3, TimeUnit.SECONDS))

        managerWin.sendAdminRequest(AdminRequestPayload(AdminActionType.REBOOT_VAULT))
        assertTrue(winLatch.await(3, TimeUnit.SECONDS))
        assertTrue(winResp!!.message.contains("Windows-HyperV"))

        jobWin.cancel()

        // Test con macOS Virtualization.framework
        val managerMac = VaultConnectionManager(
            host = "127.0.0.1",
            port = mockDaemonMacOS.port,
            localStaticPrivateKey = clientPrivateKey,
            pinnedRemotePublicKey = mockDaemonMacOS.publicKeyBytes
        )
        val macLatch = CountDownLatch(1)
        var macResp: AdminResponsePayload? = null
        managerMac.onAdminResponseReceived = {
            macResp = it
            macLatch.countDown()
        }
        val macConnLatch = CountDownLatch(1)
        managerMac.onConnectionState = { if (it) macConnLatch.countDown() }

        val jobMac = Job()
        managerMac.connectAndServe(CoroutineScope(Dispatchers.IO + jobMac))
        assertTrue(macConnLatch.await(3, TimeUnit.SECONDS))

        managerMac.sendAdminRequest(AdminRequestPayload(AdminActionType.CHANGE_NETWORK, targetNetwork = "WiFi_5G"))
        assertTrue(macLatch.await(3, TimeUnit.SECONDS))
        assertTrue(macResp!!.message.contains("macOS-VirtualizationFramework"))

        jobMac.cancel()
    }
}
