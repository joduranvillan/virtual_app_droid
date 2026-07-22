package com.example

import android.os.Bundle
import android.widget.Toast
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.animation.*
import androidx.compose.foundation.Canvas
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.*
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.Path
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp
import androidx.lifecycle.lifecycleScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch

data class VideoFramePayload(
    val timestamp: Long,
    val resolutionWidth: Int,
    val resolutionHeight: Int,
    val frameIndex: Long,
    val bitrateKbps: Int
)

data class InputEventPayload(
    val type: String,
    val x: Float = 0f,
    val y: Float = 0f,
    val keyName: String = ""
)

enum class AdminActionType {
    NET_BLOCK, NET_ALLOW, STATUS_CHECK, ROLLBACK, UPDATE_LATEST
}

class MainActivity : ComponentActivity() {

    private val _isConnected = MutableStateFlow(true)
    private val isConnected = _isConnected.asStateFlow()

    private val _connectionHost = MutableStateFlow("192.168.1.100")
    private val connectionHost = _connectionHost.asStateFlow()

    private val _connectionPort = MutableStateFlow("8000")
    private val connectionPort = _connectionPort.asStateFlow()

    private val _pairingCodeInput = MutableStateFlow("")
    private val pairingCodeInput = _pairingCodeInput.asStateFlow()

    private val _isPairingActive = MutableStateFlow(false)
    private val isPairingActive = _isPairingActive.asStateFlow()

    private val _deviceKeyFingerprint = MutableStateFlow("SHA256:7f9a2b1c3d4e5f6a7b8c9d0e1f2a3b4c")
    private val deviceKeyFingerprint = _deviceKeyFingerprint.asStateFlow()

    private val _remoteKeyFingerprint = MutableStateFlow("SHA256:a1b2c3d4e5f67890123456789abcdef0")
    private val remoteKeyFingerprint = _remoteKeyFingerprint.asStateFlow()

    private val _latestVideoFrame = MutableStateFlow<VideoFramePayload?>(
        VideoFramePayload(System.currentTimeMillis(), 1920, 1080, 1024, 6500)
    )
    private val latestVideoFrame = _latestVideoFrame.asStateFlow()

    private val _consoleLogs = MutableStateFlow(
        listOf(
            "[SYS] Bóveda Kernel v2.4.0 iniciado.",
            "[NET] Escuchando en mTLS + CBOR puerto 8000.",
            "[SEC] handshake Noise_XX completado.",
            "[STREAM] Framebuffer virtio_gpu activo."
        )
    )
    private val consoleLogs = _consoleLogs.asStateFlow()

    // VM Performance States
    private val _vCpuUsagePercent = MutableStateFlow(18.5f)
    private val vCpuUsagePercent = _vCpuUsagePercent.asStateFlow()

    private val _ramUsageMb = MutableStateFlow(2450)
    private val ramUsageMb = _ramUsageMb.asStateFlow()

    private val _hypervisorTempC = MutableStateFlow(42.0f)
    private val hypervisorTempC = _hypervisorTempC.asStateFlow()

    private val _virtIoIoReadMbps = MutableStateFlow(12.4f)
    private val virtIoIoReadMbps = _virtIoIoReadMbps.asStateFlow()

    // Sensor Telemetry States
    private val _gpsLatitude = MutableStateFlow(19.4326f)
    private val gpsLatitude = _gpsLatitude.asStateFlow()

    private val _gpsLongitude = MutableStateFlow(-99.1332f)
    private val gpsLongitude = _gpsLongitude.asStateFlow()

    private val _accelX = MutableStateFlow(0.12f)
    private val accelX = _accelX.asStateFlow()

    private val _accelY = MutableStateFlow(9.81f)
    private val accelY = _accelY.asStateFlow()

    private val _accelZ = MutableStateFlow(0.05f)
    private val accelZ = _accelZ.asStateFlow()

    private val _batteryLevelPercent = MutableStateFlow(88)
    private val batteryLevelPercent = _batteryLevelPercent.asStateFlow()

    // Streaming & Codec Controls
    private val _streamFpsTarget = MutableStateFlow(60)
    private val streamFpsTarget = _streamFpsTarget.asStateFlow()

    private val _streamResolution = MutableStateFlow("1080p (1920x1080)")
    private val streamResolution = _streamResolution.asStateFlow()

    private val _streamBitrateMbps = MutableStateFlow(6.5f)
    private val streamBitrateMbps = _streamBitrateMbps.asStateFlow()

    private val _isStreamPaused = MutableStateFlow(false)
    private val isStreamPaused = _isStreamPaused.asStateFlow()

    private val _isAudioSyncEnabled = MutableStateFlow(true)
    private val isAudioSyncEnabled = _isAudioSyncEnabled.asStateFlow()

    // Security & Scalability States
    private val _selectedClusterNode = MutableStateFlow("Node-Alpha (x86_64 High-Perf)")
    private val selectedClusterNode = _selectedClusterNode.asStateFlow()

    private val _allocatedVCpus = MutableStateFlow(4)
    private val allocatedVCpus = _allocatedVCpus.asStateFlow()

    private val _allocatedRamGb = MutableStateFlow(8)
    private val allocatedRamGb = _allocatedRamGb.asStateFlow()

    private val _hardwareCodec = MutableStateFlow("AV1 (Hardware Accel)")
    private val hardwareCodec = _hardwareCodec.asStateFlow()
    private val _lastKeyRotationTimestamp = MutableStateFlow("Hace 2 min (Noise_XX)")
    private val lastKeyRotationTimestamp = _lastKeyRotationTimestamp.asStateFlow()

    private val _teeAttestationVerified = MutableStateFlow(true)
    private val teeAttestationVerified = _teeAttestationVerified.asStateFlow()

    // Dynamic Framebuffer Volume Re-Keying States (1 GB Threshold)
    private val _transferredFramebufferMb = MutableStateFlow(420.5f)
    private val transferredFramebufferMb = _transferredFramebufferMb.asStateFlow()

    private val _rekeyVolumeThresholdMb = MutableStateFlow(1024.0f) // 1 GB = 1024 MB
    private val rekeyVolumeThresholdMb = _rekeyVolumeThresholdMb.asStateFlow()

    private val _isAutoRekeyEnabled = MutableStateFlow(true)
    private val isAutoRekeyEnabled = _isAutoRekeyEnabled.asStateFlow()

    private val _rekeyCountTotal = MutableStateFlow(3)
    private val rekeyCountTotal = _rekeyCountTotal.asStateFlow()

    // Anti-Tamper & Enclave HSM Verification States
    private val _apkBinaryHash = MutableStateFlow("SHA256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
    private val apkBinaryHash = _apkBinaryHash.asStateFlow()

    private val _hsmEnclaveVerificationStatus = MutableStateFlow("VERIFICADO (Enclave HSM Remote OK)")
    private val hsmEnclaveVerificationStatus = _hsmEnclaveVerificationStatus.asStateFlow()

    private val _isAntiTamperVerified = MutableStateFlow(true)
    private val isAntiTamperVerified = _isAntiTamperVerified.asStateFlow()

    private val _isDebuggerDetected = MutableStateFlow(false)
    private val isDebuggerDetected = _isDebuggerDetected.asStateFlow()

    private val _isBinaryModified = MutableStateFlow(false)
    private val isBinaryModified = _isBinaryModified.asStateFlow()

    private fun verifyAntiTamperWithHsm(): Boolean {
        if (_isBinaryModified.value || _isDebuggerDetected.value) {
            _remoteKeyFingerprint.value = "[DESTRUIDO - TAMPER DETECTADO]"
            _transferredFramebufferMb.value = 0f
            _isConnected.value = false
            _isPairingActive.value = false
            _isAntiTamperVerified.value = false
            _hsmEnclaveVerificationStatus.value = "BLOQUEADO (Violación de Integridad / Debugger)"
            _consoleLogs.value = _consoleLogs.value + "[SECURITY ALERT] ¡VIOLACIÓN ANTI-TAMPER! Firma APK/Binario alterada o depurador detectado por Enclave HSM Remoto. Claves efímeras destruidas en memoria y emparejamiento CBOR bloqueado."
            return false
        } else {
            _isAntiTamperVerified.value = true
            _hsmEnclaveVerificationStatus.value = "VERIFICADO (Enclave HSM Remote OK)"
            _consoleLogs.value = _consoleLogs.value + "[SEC] Verificación Anti-Tamper exitosa: Firma APK '${_apkBinaryHash.value.take(24)}...' validada contra Enclave HSM Remoto."
            return true
        }
    }

    private fun simulateTamperAttack(isBinaryTamper: Boolean) {
        if (isBinaryTamper) {
            _isBinaryModified.value = true
            _apkBinaryHash.value = "SHA256:MODIFIED_UNAUTHORIZED_APK_SIGNATURE_HASH_TAMPERED"
            _consoleLogs.value = _consoleLogs.value + "[ATTACK] Simulación de modificación de binario APK realizada."
        } else {
            _isDebuggerDetected.value = true
            _consoleLogs.value = _consoleLogs.value + "[ATTACK] Simulación de depurador/hooking (Frida/ptrace) detectado."
        }
        verifyAntiTamperWithHsm()
    }

    private fun resetAntiTamperState() {
        _isBinaryModified.value = false
        _isDebuggerDetected.value = false
        _isAntiTamperVerified.value = true
        _apkBinaryHash.value = "SHA256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        _hsmEnclaveVerificationStatus.value = "VERIFICADO (Enclave HSM Remote OK)"
        _remoteKeyFingerprint.value = "SHA256:b9f8e7d6c5b4a392019283746501fe3a"
        _consoleLogs.value = _consoleLogs.value + "[SEC] Integridad Anti-Tamper y Enclave HSM restaurada a estado seguro."
        Toast.makeText(this, "Integridad Anti-Tamper restaurada", Toast.LENGTH_SHORT).show()
    }

    // Post-Quantum Hybrid Handshake States (ML-KEM-768 + Curve25519)
    private val _isMlKem768Enabled = MutableStateFlow(true)
    private val isMlKem768Enabled = _isMlKem768Enabled.asStateFlow()

    private val _quantumAttestationStatus = MutableStateFlow("PQC OK (ML-KEM-768 + X25519 Activo)")
    private val quantumAttestationStatus = _quantumAttestationStatus.asStateFlow()

    private val _hybridKemCiphertextHash = MutableStateFlow("KYBER768:9a2f4c1e8b7d60315a0e91c2d3e4f5a6b7c8d9e0f1a2b3c4d5e6f7a8b9c0d1e2")
    private val hybridKemCiphertextHash = _hybridKemCiphertextHash.asStateFlow()

    private val _hybridCurve25519Point = MutableStateFlow("X25519:7e4a1b0c9f8d7e6a5b4c3d2e1f0a9b8c7d6e5f4a3b2c1d0e9f8a7b6c5d4e3f2a")
    private val hybridCurve25519Point = _hybridCurve25519Point.asStateFlow()

    private val _quantumSecretBitStrength = MutableStateFlow(512) // 256-bit X25519 + 256-bit ML-KEM-768
    private val quantumSecretBitStrength = _quantumSecretBitStrength.asStateFlow()

    private val _hybridHandshakeLatencyMs = MutableStateFlow(1.8f)
    private val hybridHandshakeLatencyMs = _hybridHandshakeLatencyMs.asStateFlow()

    private val _isQuantumAttestationVerified = MutableStateFlow(true)
    private val isQuantumAttestationVerified = _isQuantumAttestationVerified.asStateFlow()

    private fun executeHybridHandshake(): Boolean {
        if (!_isAntiTamperVerified.value) {
            _quantumAttestationStatus.value = "RECHAZADO (Violación Anti-Tamper activa)"
            _isQuantumAttestationVerified.value = false
            _consoleLogs.value = _consoleLogs.value + "[PQC ERROR] Handshake cancelado. Integridad Anti-Tamper/HSM comprometida."
            return false
        }

        val startTime = System.nanoTime()
        if (_isMlKem768Enabled.value) {
            val randomKyber = (10000000..99999999).random().toString(16) + (10000000..99999999).random().toString(16)
            val randomX25519 = (10000000..99999999).random().toString(16) + (10000000..99999999).random().toString(16)
            _hybridKemCiphertextHash.value = "KYBER768:$randomKyber"
            _hybridCurve25519Point.value = "X25519:$randomX25519"
            _quantumSecretBitStrength.value = 512
            _quantumAttestationStatus.value = "PQC OK (ML-KEM-768 + X25519 Activo)"
            _isQuantumAttestationVerified.value = true
            val elapsedMs = (System.nanoTime() - startTime) / 1_000_000f + 1.4f + (0..8).random() / 10.0f
            _hybridHandshakeLatencyMs.value = elapsedMs

            val derivedFingerprint = "SHA256:pqc_" + (10000000..99999999).random().toString(16)
            _remoteKeyFingerprint.value = derivedFingerprint
            _lastKeyRotationTimestamp.value = "PQC ML-KEM-768 (#${_rekeyCountTotal.value})"

            _consoleLogs.value = _consoleLogs.value + listOf(
                "[PQC] Initiating Hybrid Handshake: ML-KEM-768 (Kyber) + Curve25519 (ECDH)...",
                "[PQC] Generada matriz reticular KEM-768 (1184 B ciphertext) y punto X25519 (32 B).",
                "[PQC] Secreto derivado combinado HKDF-SHA3-256 (512-bit entropy) en ${String.format("%.1f", elapsedMs)} ms.",
                "[PQC] Canal Noise_XX fortalecido contra Algoritmo de Shor (Atestación Cuántica OK)."
            )
        } else {
            val randomX25519 = (10000000..99999999).random().toString(16)
            _hybridCurve25519Point.value = "X25519:$randomX25519"
            _hybridKemCiphertextHash.value = "KYBER768:DESACTIVADO (Modo Clásico Pure ECDH)"
            _quantumSecretBitStrength.value = 256
            _quantumAttestationStatus.value = "MODO CLÁSICO (Sólo Curve25519 - No PQC)"
            _isQuantumAttestationVerified.value = false
            _hybridHandshakeLatencyMs.value = 0.9f

            _consoleLogs.value = _consoleLogs.value + "[PQC WARNING] Handshake realizado en Modo Clásico (Curve25519 únicamente). No resistente a computación cuántica."
        }
        return true
    }

    // Live Migration States (Migración en Caliente de Sesión Framebuffer)
    private val _isMigratingSession = MutableStateFlow(false)
    private val isMigratingSession = _isMigratingSession.asStateFlow()

    private val _migrationProgressPercent = MutableStateFlow(0)
    private val migrationProgressPercent = _migrationProgressPercent.asStateFlow()

    private val _migrationStatusText = MutableStateFlow("LISTO (Sesión activa en nodo local)")
    private val migrationStatusText = _migrationStatusText.asStateFlow()

    private val _migrationDowntimeMs = MutableStateFlow(1.4f)
    private val migrationDowntimeMs = _migrationDowntimeMs.asStateFlow()

    private val _transferredDirtyPagesMb = MutableStateFlow(0.0f)
    private val transferredDirtyPagesMb = _transferredDirtyPagesMb.asStateFlow()

    private fun executeLiveMigration(targetNode: String) {
        if (_selectedClusterNode.value == targetNode) {
            Toast.makeText(this, "El nodo $targetNode ya es el nodo activo", Toast.LENGTH_SHORT).show()
            return
        }
        if (!_isAntiTamperVerified.value) {
            Toast.makeText(this, "ERROR: Migración cancelada por violación Anti-Tamper/HSM", Toast.LENGTH_LONG).show()
            return
        }
        if (_isMigratingSession.value) return

        val sourceNode = _selectedClusterNode.value
        _isMigratingSession.value = true
        _migrationProgressPercent.value = 15
        _migrationStatusText.value = "Fase 1/3: Copiando dirty-pages de framebuffer $sourceNode -> $targetNode..."

        lifecycleScope.launch {
            _consoleLogs.value = _consoleLogs.value + "[LIVE MIGRATION] Iniciando migración en caliente de sesión: $sourceNode -> $targetNode"
            delay(500)

            _transferredDirtyPagesMb.value = 512.0f
            _migrationProgressPercent.value = 60
            _migrationStatusText.value = "Fase 2/3: Sincronizando contexto VCPU KVM y llaves PQC ML-KEM-768..."
            delay(600)

            val downtime = (10..18).random() / 10.0f
            _transferredDirtyPagesMb.value = 819.2f
            _migrationProgressPercent.value = 100
            _migrationDowntimeMs.value = downtime
            _selectedClusterNode.value = targetNode
            _migrationStatusText.value = "COMPLETADO: Conmutación en caliente exitosa (Downtime: ${String.format("%.1f", downtime)} ms)"
            _isMigratingSession.value = false

            _consoleLogs.value = _consoleLogs.value + listOf(
                "[LIVE MIGRATION] Transfiriendo 819.2 MB de páginas de framebuffer sin interrupción de stream.",
                "[LIVE MIGRATION] Handshake PQC resincronizado en $targetNode.",
                "[LIVE MIGRATION] Conmutación de contexto VCPU ejecutada con downtime de ${String.format("%.1f", downtime)} ms.",
                "[LIVE MIGRATION] Sesión activa migrada exitosamente a: $targetNode."
            )

            Toast.makeText(this@MainActivity, "Migración en caliente completada a $targetNode (Downtime: ${String.format("%.1f", downtime)} ms)", Toast.LENGTH_LONG).show()
        }
    }

    private var simulationJob: kotlinx.coroutines.Job? = null

    private fun startSimulation() {
        simulationJob = lifecycleScope.launch {
            var counter = 1025L
            while (true) {
                delay(1000)
                if (_isConnected.value && !_isStreamPaused.value) {
                    counter++
                    _latestVideoFrame.value = VideoFramePayload(
                        timestamp = System.currentTimeMillis(),
                        resolutionWidth = if (_streamResolution.value.contains("720p")) 1280 else if (_streamResolution.value.contains("1440p")) 2560 else 1920,
                        resolutionHeight = if (_streamResolution.value.contains("720p")) 720 else if (_streamResolution.value.contains("1440p")) 1440 else 1080,
                        frameIndex = counter,
                        bitrateKbps = (_streamBitrateMbps.value * 1000).toInt() + (-200..200).random()
                    )

                    // Accumulate framebuffer volume transferred (simulating ~18.5 MB/s VirtIO stream)
                    if (_isAutoRekeyEnabled.value) {
                        val addedMb = 18.5f + (0..10).random() / 2.0f
                        val newVol = _transferredFramebufferMb.value + addedMb
                        if (newVol >= _rekeyVolumeThresholdMb.value) {
                            _transferredFramebufferMb.value = 0f
                            _rekeyCountTotal.value += 1
                            _lastKeyRotationTimestamp.value = "Auto 1 GB alcanzado (#${_rekeyCountTotal.value})"
                            _remoteKeyFingerprint.value = "SHA256:" + (10000000..99999999).random().toString(16) + (10000000..99999999).random().toString(16)
                            _consoleLogs.value = _consoleLogs.value + "[SEC] Re-keying dinámico ejecutado: Límite de 1 GB (1,024 MB) de framebuffer transferido alcanzado. Claves efímeras Noise_XX regeneradas (#${_rekeyCountTotal.value})."
                        } else {
                            _transferredFramebufferMb.value = newVol
                        }
                    }

                    _vCpuUsagePercent.value = (15.0f + (0..150).random() / 10.0f)
                    _ramUsageMb.value = 2400 + (-50..100).random()
                    _hypervisorTempC.value = 41.5f + (0..20).random() / 10.0f
                    _virtIoIoReadMbps.value = 10.0f + (0..50).random() / 10.0f

                    _accelX.value = (0..20).random() / 100.0f
                    _accelY.value = 9.78f + (0..10).random() / 100.0f
                }
            }
        }
    }

    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        startSimulation()

        setContent {
            MaterialTheme(
                colorScheme = darkColorScheme(
                    primary = Color(0xFF00E676),
                    onPrimary = Color.Black,
                    primaryContainer = Color(0xFF1B5E20),
                    secondary = Color(0xFF00B0FF),
                    onSecondary = Color.Black,
                    secondaryContainer = Color(0xFF004D40),
                    surface = Color(0xFF1E2638),
                    onSurface = Color.White,
                    background = Color(0xFF0F172A),
                    onBackground = Color.White
                )
            ) {
                Surface(
                    modifier = Modifier.fillMaxSize(),
                    color = MaterialTheme.colorScheme.background
                ) {
                    BovedaDigitalScreen(
                        isConnected = isConnected.collectAsState().value,
                        connectionHost = connectionHost.collectAsState().value,
                        connectionPort = connectionPort.collectAsState().value,
                        pairingCodeInput = pairingCodeInput.collectAsState().value,
                        isPairingActive = isPairingActive.collectAsState().value,
                        deviceKeyFingerprint = deviceKeyFingerprint.collectAsState().value,
                        remoteKeyFingerprint = remoteKeyFingerprint.collectAsState().value,
                        latestVideoFrame = latestVideoFrame.collectAsState().value,
                        consoleLogs = consoleLogs.collectAsState().value,
                        vCpuUsagePercent = vCpuUsagePercent.collectAsState().value,
                        ramUsageMb = ramUsageMb.collectAsState().value,
                        hypervisorTempC = hypervisorTempC.collectAsState().value,
                        virtIoIoReadMbps = virtIoIoReadMbps.collectAsState().value,
                        gpsLatitude = gpsLatitude.collectAsState().value,
                        gpsLongitude = gpsLongitude.collectAsState().value,
                        accelX = accelX.collectAsState().value,
                        accelY = accelY.collectAsState().value,
                        accelZ = accelZ.collectAsState().value,
                        batteryLevelPercent = batteryLevelPercent.collectAsState().value,
                        streamFpsTarget = streamFpsTarget.collectAsState().value,
                        streamResolution = streamResolution.collectAsState().value,
                        streamBitrateMbps = streamBitrateMbps.collectAsState().value,
                        isStreamPaused = isStreamPaused.collectAsState().value,
                        isAudioSyncEnabled = isAudioSyncEnabled.collectAsState().value,
                        selectedClusterNode = selectedClusterNode.collectAsState().value,
                        allocatedVCpus = allocatedVCpus.collectAsState().value,
                        allocatedRamGb = allocatedRamGb.collectAsState().value,
                        hardwareCodec = hardwareCodec.collectAsState().value,
                        lastKeyRotationTimestamp = lastKeyRotationTimestamp.collectAsState().value,
                        teeAttestationVerified = teeAttestationVerified.collectAsState().value,
                        transferredFramebufferMb = transferredFramebufferMb.collectAsState().value,
                        rekeyVolumeThresholdMb = rekeyVolumeThresholdMb.collectAsState().value,
                        isAutoRekeyEnabled = isAutoRekeyEnabled.collectAsState().value,
                        rekeyCountTotal = rekeyCountTotal.collectAsState().value,
                        apkBinaryHash = apkBinaryHash.collectAsState().value,
                        hsmEnclaveStatus = hsmEnclaveVerificationStatus.collectAsState().value,
                        isAntiTamperVerified = isAntiTamperVerified.collectAsState().value,
                        isDebuggerDetected = isDebuggerDetected.collectAsState().value,
                        isBinaryModified = isBinaryModified.collectAsState().value,
                        isMlKem768Enabled = isMlKem768Enabled.collectAsState().value,
                        quantumAttestationStatus = quantumAttestationStatus.collectAsState().value,
                        hybridKemCiphertextHash = hybridKemCiphertextHash.collectAsState().value,
                        hybridCurve25519Point = hybridCurve25519Point.collectAsState().value,
                        quantumSecretBitStrength = quantumSecretBitStrength.collectAsState().value,
                        hybridHandshakeLatencyMs = hybridHandshakeLatencyMs.collectAsState().value,
                        isQuantumAttestationVerified = isQuantumAttestationVerified.collectAsState().value,
                        isMigratingSession = isMigratingSession.collectAsState().value,
                        migrationProgressPercent = migrationProgressPercent.collectAsState().value,
                        migrationStatusText = migrationStatusText.collectAsState().value,
                        migrationDowntimeMs = migrationDowntimeMs.collectAsState().value,
                        transferredDirtyPagesMb = transferredDirtyPagesMb.collectAsState().value,
                        onExecuteLiveMigration = { target -> executeLiveMigration(target) },
                        onVerifyHsm = { verifyAntiTamperWithHsm() },
                        onSimulateTamper = { isBinary -> simulateTamperAttack(isBinary) },
                        onResetAntiTamper = { resetAntiTamperState() },
                        onToggleMlKem768 = { _isMlKem768Enabled.value = !_isMlKem768Enabled.value },
                        onExecuteHybridHandshake = { executeHybridHandshake() },
                        onToggleAutoRekey = { _isAutoRekeyEnabled.value = !_isAutoRekeyEnabled.value },
                        onSimulateAddVolume = {
                            val nextVol = _transferredFramebufferMb.value + 250f
                            if (nextVol >= _rekeyVolumeThresholdMb.value) {
                                _transferredFramebufferMb.value = 0f
                                _rekeyCountTotal.value += 1
                                executeHybridHandshake()
                                Toast.makeText(this@MainActivity, "Re-keying por volumen ejecutado (PQC Handshake)", Toast.LENGTH_SHORT).show()
                            } else {
                                _transferredFramebufferMb.value = nextVol
                            }
                        },
                        onSendInputEvent = { logAndSendInputEvent(it) },
                        onToggleSimulation = { toggleSimulation() },
                        onSendAdminAction = { act, net, ver -> sendAdminAction(act, net, ver) },
                        onChangeHost = { _connectionHost.value = it },
                        onChangePort = { _connectionPort.value = it },
                        onChangePairingCode = { _pairingCodeInput.value = it },
                        onStartPairing = { executePairing() },
                        onChangeFpsTarget = { _streamFpsTarget.value = it },
                        onChangeResolution = { _streamResolution.value = it },
                        onChangeBitrate = { _streamBitrateMbps.value = it },
                        onTogglePauseStream = { _isStreamPaused.value = !_isStreamPaused.value },
                        onToggleAudioSync = { _isAudioSyncEnabled.value = !_isAudioSyncEnabled.value },
                        onCaptureScreenshot = {
                            Toast.makeText(this@MainActivity, "Captura de pantalla de framebuffer guardada", Toast.LENGTH_SHORT).show()
                        },
                        onTriggerKeyRotation = {
                            executeHybridHandshake()
                            Toast.makeText(this@MainActivity, "Handshake Híbrido ML-KEM-768 + Curve25519 ejecutado", Toast.LENGTH_SHORT).show()
                        },
                        onChangeClusterNode = { _selectedClusterNode.value = it },
                        onChangeAllocatedCores = { _allocatedVCpus.value = it },
                        onChangeHardwareCodec = { _hardwareCodec.value = it }
                    )
                }
            }
        }
    }

    private fun toggleSimulation() {
        _isConnected.value = !_isConnected.value
        val statusText = if (_isConnected.value) "CONECTADO mTLS" else "DESCONECTADO"
        _consoleLogs.value = _consoleLogs.value + "[SYS] Estado cambiado: $statusText"
    }

    private fun executePairing() {
        if (_pairingCodeInput.value.length >= 6) {
            val isIntact = verifyAntiTamperWithHsm()
            if (!isIntact) {
                Toast.makeText(this@MainActivity, "ERROR ANTI-TAMPER: Firma/Depuración rechazada por Enclave HSM. Claves destruidas.", Toast.LENGTH_LONG).show()
                return
            }

            _isPairingActive.value = true
            lifecycleScope.launch {
                _consoleLogs.value = _consoleLogs.value + "[PAIR] Verificando firma APK contra Enclave HSM para código: ${_pairingCodeInput.value}"
                delay(800)
                executeHybridHandshake()
                delay(400)
                _isPairingActive.value = false
                _isConnected.value = true
                _consoleLogs.value = _consoleLogs.value + "[PAIR] Emparejamiento CBOR exitoso. Canal cifrado con atestación PQC + HSM activa."
                Toast.makeText(this@MainActivity, "Emparejamiento Exitoso (Handshake ML-KEM-768 + Curve25519 OK)", Toast.LENGTH_SHORT).show()
            }
        }
    }

    private fun logAndSendInputEvent(event: InputEventPayload) {
        _consoleLogs.value = _consoleLogs.value + "[INPUT] Evento: ${event.type} (${event.x}, ${event.y}) ${event.keyName}"
    }

    private fun sendAdminAction(action: AdminActionType, netArg: String?, verArg: String?) {
        lifecycleScope.launch {
            when (action) {
                AdminActionType.NET_BLOCK -> {
                    _consoleLogs.value = _consoleLogs.value + "[RPC] Comando 'net_block' enviado."
                    Toast.makeText(this@MainActivity, "Red de la Bóveda Bloqueada", Toast.LENGTH_SHORT).show()
                }
                AdminActionType.NET_ALLOW -> {
                    _consoleLogs.value = _consoleLogs.value + "[RPC] Comando 'net_allow' enviado."
                    Toast.makeText(this@MainActivity, "Red de la Bóveda Permitida", Toast.LENGTH_SHORT).show()
                }
                AdminActionType.STATUS_CHECK -> {
                    _consoleLogs.value = _consoleLogs.value + "[RPC] Consulta 'status' -> Kernel OK | VM Active | CBOR Sync"
                }
                AdminActionType.ROLLBACK -> {
                    _consoleLogs.value = _consoleLogs.value + "[RPC] Comando 'rollback' a versión anterior ejecutado."
                    Toast.makeText(this@MainActivity, "Rollback completado exitosamente", Toast.LENGTH_SHORT).show()
                }
                AdminActionType.UPDATE_LATEST -> {
                    _consoleLogs.value = _consoleLogs.value + "[RPC] Comando 'update_latest' iniciado..."
                    delay(1000)
                    _consoleLogs.value = _consoleLogs.value + "[RPC] Actualización completada a v2.4.1"
                    Toast.makeText(this@MainActivity, "Actualización a v2.4.1 Completada", Toast.LENGTH_SHORT).show()
                }
            }
        }
    }

    override fun onDestroy() {
        super.onDestroy()
        simulationJob?.cancel()
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun BovedaDigitalScreen(
    isConnected: Boolean,
    connectionHost: String,
    connectionPort: String,
    pairingCodeInput: String,
    isPairingActive: Boolean,
    deviceKeyFingerprint: String,
    remoteKeyFingerprint: String,
    latestVideoFrame: VideoFramePayload?,
    consoleLogs: List<String>,
    vCpuUsagePercent: Float,
    ramUsageMb: Int,
    hypervisorTempC: Float,
    virtIoIoReadMbps: Float,
    gpsLatitude: Float,
    gpsLongitude: Float,
    accelX: Float,
    accelY: Float,
    accelZ: Float,
    batteryLevelPercent: Int,
    streamFpsTarget: Int,
    streamResolution: String,
    streamBitrateMbps: Float,
    isStreamPaused: Boolean,
    isAudioSyncEnabled: Boolean,
    selectedClusterNode: String,
    allocatedVCpus: Int,
    allocatedRamGb: Int,
    hardwareCodec: String,
    lastKeyRotationTimestamp: String,
    teeAttestationVerified: Boolean,
    transferredFramebufferMb: Float,
    rekeyVolumeThresholdMb: Float,
    isAutoRekeyEnabled: Boolean,
    rekeyCountTotal: Int,
    apkBinaryHash: String,
    hsmEnclaveStatus: String,
    isAntiTamperVerified: Boolean,
    isDebuggerDetected: Boolean,
    isBinaryModified: Boolean,
    isMlKem768Enabled: Boolean,
    quantumAttestationStatus: String,
    hybridKemCiphertextHash: String,
    hybridCurve25519Point: String,
    quantumSecretBitStrength: Int,
    hybridHandshakeLatencyMs: Float,
    isQuantumAttestationVerified: Boolean,
    isMigratingSession: Boolean,
    migrationProgressPercent: Int,
    migrationStatusText: String,
    migrationDowntimeMs: Float,
    transferredDirtyPagesMb: Float,
    onExecuteLiveMigration: (String) -> Unit,
    onVerifyHsm: () -> Unit,
    onSimulateTamper: (Boolean) -> Unit,
    onResetAntiTamper: () -> Unit,
    onToggleMlKem768: () -> Unit,
    onExecuteHybridHandshake: () -> Unit,
    onToggleAutoRekey: () -> Unit,
    onSimulateAddVolume: () -> Unit,
    onSendInputEvent: (InputEventPayload) -> Unit,
    onToggleSimulation: () -> Unit,
    onSendAdminAction: (AdminActionType, String?, String?) -> Unit,
    onChangeHost: (String) -> Unit,
    onChangePort: (String) -> Unit,
    onChangePairingCode: (String) -> Unit,
    onStartPairing: () -> Unit,
    onChangeFpsTarget: (Int) -> Unit,
    onChangeResolution: (String) -> Unit,
    onChangeBitrate: (Float) -> Unit,
    onTogglePauseStream: () -> Unit,
    onToggleAudioSync: () -> Unit,
    onCaptureScreenshot: () -> Unit,
    onTriggerKeyRotation: () -> Unit,
    onChangeClusterNode: (String) -> Unit,
    onChangeAllocatedCores: (Int) -> Unit,
    onChangeHardwareCodec: (String) -> Unit
) {
    val context = LocalContext.current
    val clipboardManager = LocalClipboardManager.current

    Scaffold(
        topBar = {
            TopAppBar(
                title = {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Icon(
                            imageVector = Icons.Default.Shield,
                            contentDescription = "Bóveda Logo",
                            tint = MaterialTheme.colorScheme.primary,
                            modifier = Modifier.size(28.dp)
                        )
                        Spacer(modifier = Modifier.width(12.dp))
                        Column {
                            Text(
                                text = "BÓVEDA DIGITAL",
                                style = MaterialTheme.typography.titleMedium,
                                fontWeight = FontWeight.Bold,
                                color = MaterialTheme.colorScheme.onSurface
                            )
                            Text(
                                text = "Panel de Control RPC / VirtIO Framebuffer",
                                style = MaterialTheme.typography.labelSmall,
                                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f)
                            )
                        }
                    }
                },
                actions = {
                    IconButton(onClick = onToggleSimulation) {
                        Icon(
                            imageVector = if (isConnected) Icons.Default.PowerSettingsNew else Icons.Default.PowerOff,
                            contentDescription = "Estado Conexión",
                            tint = if (isConnected) Color(0xFF00E676) else Color(0xFFFF5252)
                        )
                    }
                },
                colors = TopAppBarDefaults.topAppBarColors(
                    containerColor = MaterialTheme.colorScheme.surface
                )
            )
        }
    ) { paddingValues ->
        Column(
            modifier = Modifier
                .fillMaxSize()
                .padding(paddingValues)
                .background(MaterialTheme.colorScheme.background)
                .verticalScroll(rememberScrollState())
                .padding(16.dp)
        ) {
            // 1. CONNECTION STATUS & HOST SETTINGS CARD
            ConnectionHostCard(
                isConnected = isConnected,
                connectionHost = connectionHost,
                connectionPort = connectionPort,
                onChangeHost = onChangeHost,
                onChangePort = onChangePort,
                onToggleSimulation = onToggleSimulation
            )

            Spacer(modifier = Modifier.height(20.dp))

            // 2. PAIRING PROCESS CARD
            PairingCard(
                pairingCodeInput = pairingCodeInput,
                isPairingActive = isPairingActive,
                onChangePairingCode = onChangePairingCode,
                onStartPairing = onStartPairing
            )

            Spacer(modifier = Modifier.height(20.dp))

            // 3. VM PERFORMANCE MONITOR CARD
            VmPerformanceMonitorCard(
                vCpuUsagePercent = vCpuUsagePercent,
                ramUsageMb = ramUsageMb,
                hypervisorTempC = hypervisorTempC,
                virtIoIoReadMbps = virtIoIoReadMbps
            )

            Spacer(modifier = Modifier.height(20.dp))

            // 4. SENSOR TELEMETRY PASSTHROUGH CARD
            SensorTelemetryCard(
                gpsLatitude = gpsLatitude,
                gpsLongitude = gpsLongitude,
                accelX = accelX,
                accelY = accelY,
                accelZ = accelZ,
                batteryLevelPercent = batteryLevelPercent
            )

            Spacer(modifier = Modifier.height(20.dp))

            // 5. CRYPTOGRAPHIC KEYS & SECURITY ATTESTATION PANEL
            CryptoKeysCard(
                deviceKeyFingerprint = deviceKeyFingerprint,
                remoteKeyFingerprint = remoteKeyFingerprint,
                onCopyDeviceKey = {
                    clipboardManager.setText(AnnotatedString(deviceKeyFingerprint))
                    Toast.makeText(context, "Huella copiada al portapapeles", Toast.LENGTH_SHORT).show()
                }
            )

            Spacer(modifier = Modifier.height(20.dp))

            SecurityRekeyingAttestationCard(
                teeAttestationVerified = teeAttestationVerified,
                lastKeyRotationTimestamp = lastKeyRotationTimestamp,
                transferredFramebufferMb = transferredFramebufferMb,
                rekeyVolumeThresholdMb = rekeyVolumeThresholdMb,
                isAutoRekeyEnabled = isAutoRekeyEnabled,
                rekeyCountTotal = rekeyCountTotal,
                apkBinaryHash = apkBinaryHash,
                hsmEnclaveStatus = hsmEnclaveStatus,
                isAntiTamperVerified = isAntiTamperVerified,
                isDebuggerDetected = isDebuggerDetected,
                isBinaryModified = isBinaryModified,
                isMlKem768Enabled = isMlKem768Enabled,
                quantumAttestationStatus = quantumAttestationStatus,
                hybridKemCiphertextHash = hybridKemCiphertextHash,
                hybridCurve25519Point = hybridCurve25519Point,
                quantumSecretBitStrength = quantumSecretBitStrength,
                hybridHandshakeLatencyMs = hybridHandshakeLatencyMs,
                isQuantumAttestationVerified = isQuantumAttestationVerified,
                onTriggerKeyRotation = onTriggerKeyRotation,
                onToggleAutoRekey = onToggleAutoRekey,
                onSimulateAddVolume = onSimulateAddVolume,
                onVerifyHsm = onVerifyHsm,
                onSimulateTamper = onSimulateTamper,
                onResetAntiTamper = onResetAntiTamper,
                onToggleMlKem768 = onToggleMlKem768,
                onExecuteHybridHandshake = onExecuteHybridHandshake
            )

            Spacer(modifier = Modifier.height(20.dp))

            // 6. CLUSTER SCALABILITY & ACCELERATION CODEC ORCHESTRATOR
            ClusterScalabilityOrchestratorCard(
                selectedNode = selectedClusterNode,
                allocatedCores = allocatedVCpus,
                allocatedRamGb = allocatedRamGb,
                hardwareCodec = hardwareCodec,
                isMigratingSession = isMigratingSession,
                migrationProgressPercent = migrationProgressPercent,
                migrationStatusText = migrationStatusText,
                migrationDowntimeMs = migrationDowntimeMs,
                transferredDirtyPagesMb = transferredDirtyPagesMb,
                onChangeClusterNode = onChangeClusterNode,
                onChangeAllocatedCores = onChangeAllocatedCores,
                onChangeHardwareCodec = onChangeHardwareCodec,
                onExecuteLiveMigration = onExecuteLiveMigration
            )

            Spacer(modifier = Modifier.height(20.dp))

            // 7. INTERACTIVE REMOTE CONSOLE & STREAMING CONTROLS
            InteractiveRemoteScreenCard(
                isConnected = isConnected,
                latestVideoFrame = latestVideoFrame,
                streamFpsTarget = streamFpsTarget,
                streamResolution = streamResolution,
                streamBitrateMbps = streamBitrateMbps,
                isStreamPaused = isStreamPaused,
                isAudioSyncEnabled = isAudioSyncEnabled,
                onChangeFpsTarget = onChangeFpsTarget,
                onChangeResolution = onChangeResolution,
                onChangeBitrate = onChangeBitrate,
                onTogglePauseStream = onTogglePauseStream,
                onToggleAudioSync = onToggleAudioSync,
                onCaptureScreenshot = onCaptureScreenshot,
                onSendInputEvent = onSendInputEvent
            )

            Spacer(modifier = Modifier.height(20.dp))

            // 8. HEADLESS RPC CONTROL CARD
            HeadlessRpcControlCard(
                onSendAdminAction = onSendAdminAction
            )

            Spacer(modifier = Modifier.height(20.dp))

            // 9. CONSOLE LOGS TERMINAL CARD
            ConsoleLogsCard(consoleLogs = consoleLogs)

            Spacer(modifier = Modifier.height(30.dp))
        }
    }
}

@Composable
fun ConnectionHostCard(
    isConnected: Boolean,
    connectionHost: String,
    connectionPort: String,
    onChangeHost: (String) -> Unit,
    onChangePort: (String) -> Unit,
    onToggleSimulation: () -> Unit
) {
    Card(
        shape = RoundedCornerShape(24.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
        modifier = Modifier
            .fillMaxWidth()
            .testTag("connection_host_card")
    ) {
        Column(modifier = Modifier.padding(24.dp)) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween,
                modifier = Modifier.fillMaxWidth()
            ) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Box(
                        modifier = Modifier
                            .size(12.dp)
                            .clip(CircleShape)
                            .background(if (isConnected) Color(0xFF00E676) else Color(0xFFFF5252))
                    )
                    Spacer(modifier = Modifier.width(10.dp))
                    Text(
                        text = if (isConnected) "BÓVEDA KERNEL CONECTADO" else "ENLACE DESCONECTADO",
                        style = MaterialTheme.typography.labelMedium,
                        fontWeight = FontWeight.Bold,
                        color = if (isConnected) Color(0xFF00E676) else Color(0xFFFF5252),
                        letterSpacing = 1.2.sp
                    )
                }

                Surface(
                    shape = RoundedCornerShape(8.dp),
                    color = MaterialTheme.colorScheme.primaryContainer.copy(alpha = 0.5f)
                ) {
                    Text(
                        text = "mTLS / CBOR v2",
                        style = MaterialTheme.typography.labelSmall,
                        color = MaterialTheme.colorScheme.primary,
                        modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp)
                    )
                }
            }

            Spacer(modifier = Modifier.height(16.dp))

            Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                OutlinedTextField(
                    value = connectionHost,
                    onValueChange = onChangeHost,
                    label = { Text("Dirección Host / IP") },
                    singleLine = true,
                    modifier = Modifier
                        .weight(2f)
                        .testTag("host_input_field"),
                    shape = RoundedCornerShape(12.dp)
                )

                OutlinedTextField(
                    value = connectionPort,
                    onValueChange = onChangePort,
                    label = { Text("Puerto") },
                    singleLine = true,
                    modifier = Modifier
                        .weight(1f)
                        .testTag("port_input_field"),
                    shape = RoundedCornerShape(12.dp)
                )
            }

            Spacer(modifier = Modifier.height(16.dp))

            Button(
                onClick = onToggleSimulation,
                shape = RoundedCornerShape(12.dp),
                colors = ButtonDefaults.buttonColors(
                    containerColor = if (isConnected) Color(0xFF334155) else MaterialTheme.colorScheme.primary
                ),
                modifier = Modifier
                    .fillMaxWidth()
                    .testTag("toggle_connection_button")
            ) {
                Icon(
                    imageVector = if (isConnected) Icons.Default.LinkOff else Icons.Default.Link,
                    contentDescription = "Conectar",
                    modifier = Modifier.size(18.dp)
                )
                Spacer(modifier = Modifier.width(8.dp))
                Text(
                    text = if (isConnected) "Desconectar Bóveda Kernel" else "Establecer Enlace mTLS",
                    fontWeight = FontWeight.Bold
                )
            }
        }
    }
}

@Composable
fun PairingCard(
    pairingCodeInput: String,
    isPairingActive: Boolean,
    onChangePairingCode: (String) -> Unit,
    onStartPairing: () -> Unit
) {
    Card(
        shape = RoundedCornerShape(24.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
        modifier = Modifier
            .fillMaxWidth()
            .testTag("pairing_card")
    ) {
        Column(modifier = Modifier.padding(24.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(
                    imageVector = Icons.Default.QrCodeScanner,
                    contentDescription = "Emparejamiento",
                    tint = MaterialTheme.colorScheme.secondary,
                    modifier = Modifier.size(22.dp)
                )
                Spacer(modifier = Modifier.width(10.dp))
                Text(
                    text = "EMPAREJAMIENTO DE DISPOSITIVO",
                    style = MaterialTheme.typography.titleSmall,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.onSurface
                )
            }

            Spacer(modifier = Modifier.height(12.dp))

            Text(
                text = "Introduce el código de 6 dígitos mostrado en la terminal de la Bóveda o escanea la clave de emparejamiento URI `vault://pair`.",
                style = MaterialTheme.typography.bodySmall,
                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f)
            )

            Spacer(modifier = Modifier.height(16.dp))

            Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
                OutlinedTextField(
                    value = pairingCodeInput,
                    onValueChange = onChangePairingCode,
                    placeholder = { Text("Ej: 849204") },
                    singleLine = true,
                    modifier = Modifier
                        .weight(1f)
                        .testTag("pairing_code_field"),
                    shape = RoundedCornerShape(12.dp)
                )

                Button(
                    onClick = onStartPairing,
                    enabled = pairingCodeInput.length >= 6 && !isPairingActive,
                    shape = RoundedCornerShape(12.dp),
                    colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.secondary),
                    modifier = Modifier.testTag("confirm_pair_button")
                ) {
                    if (isPairingActive) {
                        CircularProgressIndicator(
                            modifier = Modifier.size(18.dp),
                            color = Color.White,
                            strokeWidth = 2.dp
                        )
                    } else {
                        Text("Emparejar", fontWeight = FontWeight.Bold)
                    }
                }
            }
        }
    }
}

@Composable
fun VmPerformanceMonitorCard(
    vCpuUsagePercent: Float,
    ramUsageMb: Int,
    hypervisorTempC: Float,
    virtIoIoReadMbps: Float
) {
    Card(
        shape = RoundedCornerShape(24.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
        modifier = Modifier
            .fillMaxWidth()
            .testTag("vm_performance_card")
    ) {
        Column(modifier = Modifier.padding(24.dp)) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween,
                modifier = Modifier.fillMaxWidth()
            ) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(
                        imageVector = Icons.Default.Memory,
                        contentDescription = "Rendimiento VM",
                        tint = Color(0xFFFFD600),
                        modifier = Modifier.size(22.dp)
                    )
                    Spacer(modifier = Modifier.width(10.dp))
                    Text(
                        text = "MONITOR DE RENDIMIENTO VM",
                        style = MaterialTheme.typography.titleSmall,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.onSurface
                    )
                }

                Text(
                    text = "Hypervisor QEMU/KVM",
                    style = MaterialTheme.typography.labelSmall,
                    color = Color(0xFFFFD600)
                )
            }

            Spacer(modifier = Modifier.height(16.dp))

            Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                MetricTile(
                    title = "Carga vCPU",
                    value = "${String.format("%.1f", vCpuUsagePercent)}%",
                    subtitle = "4 Cores Asignados",
                    modifier = Modifier.weight(1f)
                )

                MetricTile(
                    title = "RAM Usada",
                    value = "${ramUsageMb} MB",
                    subtitle = "8192 MB Totales",
                    modifier = Modifier.weight(1f)
                )
            }

            Spacer(modifier = Modifier.height(10.dp))

            Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                MetricTile(
                    title = "Temp. Hipervisor",
                    value = "${String.format("%.1f", hypervisorTempC)} °C",
                    subtitle = "Estado Térmico Normal",
                    modifier = Modifier.weight(1f)
                )

                MetricTile(
                    title = "I/O VirtIO Disk",
                    value = "${String.format("%.1f", virtIoIoReadMbps)} MB/s",
                    subtitle = "Lectura / Escritura",
                    modifier = Modifier.weight(1f)
                )
            }
        }
    }
}

@Composable
fun MetricTile(
    title: String,
    value: String,
    subtitle: String,
    modifier: Modifier = Modifier
) {
    Surface(
        shape = RoundedCornerShape(16.dp),
        color = MaterialTheme.colorScheme.background,
        modifier = modifier
    ) {
        Column(modifier = Modifier.padding(14.dp)) {
            Text(
                text = title.uppercase(),
                style = MaterialTheme.typography.labelSmall,
                fontSize = 9.sp,
                color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.5f),
                fontWeight = FontWeight.Bold
            )
            Spacer(modifier = Modifier.height(4.dp))
            Text(
                text = value,
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.ExtraBold,
                color = MaterialTheme.colorScheme.onBackground
            )
            Spacer(modifier = Modifier.height(2.dp))
            Text(
                text = subtitle,
                style = MaterialTheme.typography.labelSmall,
                fontSize = 10.sp,
                color = MaterialTheme.colorScheme.onBackground.copy(alpha = 0.4f)
            )
        }
    }
}

@Composable
fun SensorTelemetryCard(
    gpsLatitude: Float,
    gpsLongitude: Float,
    accelX: Float,
    accelY: Float,
    accelZ: Float,
    batteryLevelPercent: Int
) {
    Card(
        shape = RoundedCornerShape(24.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
        modifier = Modifier
            .fillMaxWidth()
            .testTag("sensor_telemetry_card")
    ) {
        Column(modifier = Modifier.padding(24.dp)) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween,
                modifier = Modifier.fillMaxWidth()
            ) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(
                        imageVector = Icons.Default.Sensors,
                        contentDescription = "Telemetría de Sensores",
                        tint = Color(0xFFFF4081),
                        modifier = Modifier.size(22.dp)
                    )
                    Spacer(modifier = Modifier.width(10.dp))
                    Text(
                        text = "TELEMETRÍA DE SENSORES",
                        style = MaterialTheme.typography.titleSmall,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.onSurface
                    )
                }

                Surface(
                    shape = RoundedCornerShape(8.dp),
                    color = Color(0xFFFF4081).copy(alpha = 0.15f)
                ) {
                    Text(
                        text = "120Hz Realtime",
                        style = MaterialTheme.typography.labelSmall,
                        color = Color(0xFFFF4081),
                        modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp)
                    )
                }
            }

            Spacer(modifier = Modifier.height(16.dp))

            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(16.dp))
                    .background(MaterialTheme.colorScheme.background)
                    .padding(14.dp)
            ) {
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween
                ) {
                    Column {
                        Text(
                            text = "GPS PASSTHROUGH",
                            style = MaterialTheme.typography.labelSmall,
                            fontSize = 9.sp,
                            color = MaterialTheme.colorScheme.primary,
                            fontWeight = FontWeight.Bold
                        )
                        Text(
                            text = "Lat: $gpsLatitude | Lon: $gpsLongitude",
                            style = MaterialTheme.typography.bodySmall,
                            fontFamily = FontFamily.Monospace,
                            color = MaterialTheme.colorScheme.onBackground
                        )
                    }

                    Column(horizontalAlignment = Alignment.End) {
                        Text(
                            text = "BATERÍA DISPOSITIVO",
                            style = MaterialTheme.typography.labelSmall,
                            fontSize = 9.sp,
                            color = MaterialTheme.colorScheme.primary,
                            fontWeight = FontWeight.Bold
                        )
                        Text(
                            text = "$batteryLevelPercent%",
                            style = MaterialTheme.typography.bodySmall,
                            fontFamily = FontFamily.Monospace,
                            fontWeight = FontWeight.Bold,
                            color = Color(0xFF00E676)
                        )
                    }
                }

                Spacer(modifier = Modifier.height(10.dp))

                Text(
                    text = "ACELERÓMETRO X/Y/Z (m/s²)",
                    style = MaterialTheme.typography.labelSmall,
                    fontSize = 9.sp,
                    color = MaterialTheme.colorScheme.secondary,
                    fontWeight = FontWeight.Bold
                )
                Text(
                    text = "X: ${String.format("%.2f", accelX)}  |  Y: ${String.format("%.2f", accelY)}  |  Z: ${String.format("%.2f", accelZ)}",
                    style = MaterialTheme.typography.bodySmall,
                    fontFamily = FontFamily.Monospace,
                    color = MaterialTheme.colorScheme.onBackground
                )
            }
        }
    }
}

@Composable
fun CryptoKeysCard(
    deviceKeyFingerprint: String,
    remoteKeyFingerprint: String,
    onCopyDeviceKey: () -> Unit
) {
    Card(
        shape = RoundedCornerShape(24.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
        modifier = Modifier
            .fillMaxWidth()
            .testTag("crypto_keys_card")
    ) {
        Column(modifier = Modifier.padding(24.dp)) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween,
                modifier = Modifier.fillMaxWidth()
            ) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(
                        imageVector = Icons.Default.VpnKey,
                        contentDescription = "Claves Criptográficas",
                        tint = MaterialTheme.colorScheme.primary,
                        modifier = Modifier.size(22.dp)
                    )
                    Spacer(modifier = Modifier.width(10.dp))
                    Text(
                        text = "HUELLAS DIGITALES MOK",
                        style = MaterialTheme.typography.titleSmall,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.onSurface
                    )
                }

                IconButton(onClick = onCopyDeviceKey, modifier = Modifier.size(32.dp)) {
                    Icon(
                        imageVector = Icons.Default.ContentCopy,
                        contentDescription = "Copiar",
                        tint = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f),
                        modifier = Modifier.size(18.dp)
                    )
                }
            }

            Spacer(modifier = Modifier.height(12.dp))

            Text(
                text = "CLAVE DISPOSITIVO LOCAL:",
                style = MaterialTheme.typography.labelSmall,
                fontSize = 10.sp,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.primary
            )
            Text(
                text = deviceKeyFingerprint,
                style = MaterialTheme.typography.bodySmall,
                fontFamily = FontFamily.Monospace,
                fontSize = 11.sp,
                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.8f)
            )

            Spacer(modifier = Modifier.height(10.dp))

            Text(
                text = "CLAVE BÓVEDA REMOTA AUTORIZADA:",
                style = MaterialTheme.typography.labelSmall,
                fontSize = 10.sp,
                fontWeight = FontWeight.Bold,
                color = MaterialTheme.colorScheme.secondary
            )
            Text(
                text = remoteKeyFingerprint,
                style = MaterialTheme.typography.bodySmall,
                fontFamily = FontFamily.Monospace,
                fontSize = 11.sp,
                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.8f)
            )
        }
    }
}

@Composable
fun SecurityRekeyingAttestationCard(
    teeAttestationVerified: Boolean,
    lastKeyRotationTimestamp: String,
    transferredFramebufferMb: Float,
    rekeyVolumeThresholdMb: Float,
    isAutoRekeyEnabled: Boolean,
    rekeyCountTotal: Int,
    apkBinaryHash: String,
    hsmEnclaveStatus: String,
    isAntiTamperVerified: Boolean,
    isDebuggerDetected: Boolean,
    isBinaryModified: Boolean,
    isMlKem768Enabled: Boolean,
    quantumAttestationStatus: String,
    hybridKemCiphertextHash: String,
    hybridCurve25519Point: String,
    quantumSecretBitStrength: Int,
    hybridHandshakeLatencyMs: Float,
    isQuantumAttestationVerified: Boolean,
    onTriggerKeyRotation: () -> Unit,
    onToggleAutoRekey: () -> Unit,
    onSimulateAddVolume: () -> Unit,
    onVerifyHsm: () -> Unit,
    onSimulateTamper: (Boolean) -> Unit,
    onResetAntiTamper: () -> Unit,
    onToggleMlKem768: () -> Unit,
    onExecuteHybridHandshake: () -> Unit
) {
    Card(
        shape = RoundedCornerShape(24.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
        modifier = Modifier
            .fillMaxWidth()
            .testTag("security_attestation_card")
    ) {
        Column(modifier = Modifier.padding(24.dp)) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween,
                modifier = Modifier.fillMaxWidth()
            ) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Box(
                        modifier = Modifier
                            .size(36.dp)
                            .clip(CircleShape)
                            .background(if (isAntiTamperVerified) Color(0xFF00E676).copy(alpha = 0.15f) else Color(0xFFFF5252).copy(alpha = 0.15f)),
                        contentAlignment = Alignment.Center
                    ) {
                        Icon(
                            imageVector = Icons.Default.Security,
                            contentDescription = "Security Attestation",
                            tint = if (isAntiTamperVerified) Color(0xFF00E676) else Color(0xFFFF5252),
                            modifier = Modifier.size(20.dp)
                        )
                    }
                    Spacer(modifier = Modifier.width(12.dp))
                    Column {
                        Text(
                            text = "ATESTACIÓN DE SEGURIDAD & ANTI-TAMPER",
                            style = MaterialTheme.typography.labelSmall,
                            fontWeight = FontWeight.Bold,
                            color = if (isAntiTamperVerified) Color(0xFF00E676) else Color(0xFFFF5252),
                            letterSpacing = 1.2.sp
                        )
                        Text(
                            text = "Enclave HSM Remoto & ARM TrustZone TEE",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.5f)
                        )
                    }
                }

                Surface(
                    shape = RoundedCornerShape(8.dp),
                    color = if (isAntiTamperVerified) Color(0xFF00E676).copy(alpha = 0.12f) else Color(0xFFFF5252).copy(alpha = 0.12f)
                ) {
                    Text(
                        text = if (isAntiTamperVerified) "HSM OK" else "BLOQUEADO",
                        style = MaterialTheme.typography.labelSmall,
                        fontWeight = FontWeight.Bold,
                        color = if (isAntiTamperVerified) Color(0xFF00E676) else Color(0xFFFF5252),
                        fontSize = 10.sp,
                        modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp)
                    )
                }
            }

            Spacer(modifier = Modifier.height(16.dp))

            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(14.dp))
                    .background(MaterialTheme.colorScheme.background)
                    .padding(14.dp)
            ) {
                // 1. NOISE_XX & FRAMEBUFFER RE-KEYING SECTION
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Column {
                        Text(
                            text = "PROTOCOLO NOISE_XX RE-KEYING",
                            style = MaterialTheme.typography.labelSmall,
                            fontSize = 9.sp,
                            fontWeight = FontWeight.Bold,
                            color = MaterialTheme.colorScheme.primary
                        )
                        Spacer(modifier = Modifier.height(2.dp))
                        Text(
                            text = "Última rotación: $lastKeyRotationTimestamp",
                            style = MaterialTheme.typography.bodySmall,
                            fontFamily = FontFamily.Monospace,
                            color = MaterialTheme.colorScheme.onBackground
                        )
                    }

                    Button(
                        onClick = onTriggerKeyRotation,
                        shape = RoundedCornerShape(10.dp),
                        colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.primary),
                        contentPadding = PaddingValues(horizontal = 12.dp, vertical = 6.dp)
                    ) {
                        Icon(
                            imageVector = Icons.Default.VpnKey,
                            contentDescription = "Rotar Claves",
                            modifier = Modifier.size(14.dp)
                        )
                        Spacer(modifier = Modifier.width(6.dp))
                        Text(text = "Rotar Claves", fontSize = 11.sp, fontWeight = FontWeight.Bold)
                    }
                }

                Spacer(modifier = Modifier.height(14.dp))

                // Framebuffer Volume Meter & 1 GB Threshold Progress
                val progressFraction = (transferredFramebufferMb / rekeyVolumeThresholdMb).coerceIn(0f, 1f)
                val percentageInt = (progressFraction * 100).toInt()

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Icon(
                            imageVector = Icons.Default.Storage,
                            contentDescription = "Framebuffer Volume",
                            tint = MaterialTheme.colorScheme.secondary,
                            modifier = Modifier.size(14.dp)
                        )
                        Spacer(modifier = Modifier.width(6.dp))
                        Text(
                            text = "VOLUMEN FRAMEBUFFER (1 GB LÍMITE)",
                            style = MaterialTheme.typography.labelSmall,
                            fontSize = 9.sp,
                            fontWeight = FontWeight.Bold,
                            color = MaterialTheme.colorScheme.secondary
                        )
                    }

                    Text(
                        text = "${String.format("%.1f", transferredFramebufferMb)} / ${rekeyVolumeThresholdMb.toInt()} MB ($percentageInt%)",
                        style = MaterialTheme.typography.labelSmall,
                        fontSize = 10.sp,
                        fontFamily = FontFamily.Monospace,
                        fontWeight = FontWeight.Bold,
                        color = if (percentageInt > 85) Color(0xFFFF5252) else MaterialTheme.colorScheme.onSurface
                    )
                }

                Spacer(modifier = Modifier.height(6.dp))

                LinearProgressIndicator(
                    progress = { progressFraction },
                    modifier = Modifier
                        .fillMaxWidth()
                        .height(8.dp)
                        .clip(RoundedCornerShape(4.dp)),
                    color = if (percentageInt > 85) Color(0xFFFF5252) else MaterialTheme.colorScheme.primary,
                    trackColor = MaterialTheme.colorScheme.surface
                )

                Spacer(modifier = Modifier.height(12.dp))

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    FilterChip(
                        selected = isAutoRekeyEnabled,
                        onClick = onToggleAutoRekey,
                        label = {
                            Text(
                                text = if (isAutoRekeyEnabled) "Auto Re-Key @ 1GB: ON" else "Auto Re-Key: OFF",
                                fontSize = 10.sp,
                                fontWeight = FontWeight.Bold
                            )
                        },
                        leadingIcon = {
                            Icon(
                                imageVector = if (isAutoRekeyEnabled) Icons.Default.Autorenew else Icons.Default.Block,
                                contentDescription = "Auto Rekey",
                                modifier = Modifier.size(12.dp)
                            )
                        },
                        modifier = Modifier.height(30.dp)
                    )

                    OutlinedButton(
                        onClick = onSimulateAddVolume,
                        shape = RoundedCornerShape(8.dp),
                        contentPadding = PaddingValues(horizontal = 8.dp, vertical = 4.dp),
                        modifier = Modifier.height(30.dp)
                    ) {
                        Icon(
                            imageVector = Icons.Default.AddCircleOutline,
                            contentDescription = "Simular Datos",
                            modifier = Modifier.size(12.dp)
                        )
                        Spacer(modifier = Modifier.width(4.dp))
                        Text(text = "+250 MB", fontSize = 10.sp, fontWeight = FontWeight.Bold)
                    }

                    Surface(
                        shape = RoundedCornerShape(8.dp),
                        color = MaterialTheme.colorScheme.surface
                    ) {
                        Text(
                            text = "Rotaciones: $rekeyCountTotal",
                            style = MaterialTheme.typography.labelSmall,
                            fontSize = 9.sp,
                            fontWeight = FontWeight.Bold,
                            color = MaterialTheme.colorScheme.secondary,
                            modifier = Modifier.padding(horizontal = 8.dp, vertical = 6.dp)
                        )
                    }
                }

                // 2. ANTI-TAMPER & REMOTE HSM ENCLAVE AUDIT SUB-SECTION
                HorizontalDivider(
                    modifier = Modifier.padding(vertical = 14.dp),
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.12f)
                )

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Icon(
                            imageVector = Icons.Default.VerifiedUser,
                            contentDescription = "HSM Enclave",
                            tint = if (isAntiTamperVerified) Color(0xFF00E676) else Color(0xFFFF5252),
                            modifier = Modifier.size(16.dp)
                        )
                        Spacer(modifier = Modifier.width(8.dp))
                        Column {
                            Text(
                                text = "VERIFICACIÓN ANTI-TAMPER (HSM REMOTO)",
                                style = MaterialTheme.typography.labelSmall,
                                fontSize = 9.sp,
                                fontWeight = FontWeight.Bold,
                                color = if (isAntiTamperVerified) Color(0xFF00E676) else Color(0xFFFF5252)
                            )
                            Text(
                                text = hsmEnclaveStatus,
                                style = MaterialTheme.typography.bodySmall,
                                fontSize = 10.sp,
                                fontFamily = FontFamily.Monospace,
                                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.8f)
                            )
                        }
                    }

                    Surface(
                        shape = RoundedCornerShape(8.dp),
                        color = if (isAntiTamperVerified) Color(0xFF00E676).copy(alpha = 0.15f) else Color(0xFFFF5252).copy(alpha = 0.15f)
                    ) {
                        Text(
                            text = if (isAntiTamperVerified) "HSM ENCLAVE OK" else "VIOLACIÓN DETECTADA",
                            style = MaterialTheme.typography.labelSmall,
                            fontSize = 9.sp,
                            fontWeight = FontWeight.Bold,
                            color = if (isAntiTamperVerified) Color(0xFF00E676) else Color(0xFFFF5252),
                            modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp)
                        )
                    }
                }

                Spacer(modifier = Modifier.height(10.dp))

                // Hash & Debug status row
                Surface(
                    shape = RoundedCornerShape(8.dp),
                    color = MaterialTheme.colorScheme.surface,
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Column(modifier = Modifier.padding(10.dp)) {
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.SpaceBetween,
                            modifier = Modifier.fillMaxWidth()
                        ) {
                            Text(
                                text = "Hash Firma APK/Binario:",
                                style = MaterialTheme.typography.labelSmall,
                                fontSize = 9.sp,
                                color = MaterialTheme.colorScheme.secondary,
                                fontWeight = FontWeight.Bold
                            )
                            Text(
                                text = if (isBinaryModified) "ALTERADO ❌" else "INTEGRO ✅",
                                fontSize = 9.sp,
                                fontWeight = FontWeight.Bold,
                                color = if (isBinaryModified) Color(0xFFFF5252) else Color(0xFF00E676)
                            )
                        }
                        Spacer(modifier = Modifier.height(2.dp))
                        Text(
                            text = apkBinaryHash,
                            style = MaterialTheme.typography.bodySmall,
                            fontSize = 9.sp,
                            fontFamily = FontFamily.Monospace,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                            color = if (isBinaryModified) Color(0xFFFF5252) else MaterialTheme.colorScheme.onSurface.copy(alpha = 0.8f)
                        )
                    }
                }

                Spacer(modifier = Modifier.height(10.dp))

                // Interactive Anti-Tamper actions
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(6.dp)
                ) {
                    OutlinedButton(
                        onClick = onVerifyHsm,
                        shape = RoundedCornerShape(8.dp),
                        contentPadding = PaddingValues(horizontal = 6.dp, vertical = 4.dp),
                        modifier = Modifier
                            .weight(1f)
                            .height(30.dp)
                    ) {
                        Icon(
                            imageVector = Icons.Default.CheckCircleOutline,
                            contentDescription = "Audit HSM",
                            modifier = Modifier.size(12.dp)
                        )
                        Spacer(modifier = Modifier.width(2.dp))
                        Text(text = "Validar", fontSize = 9.sp, fontWeight = FontWeight.Bold)
                    }

                    OutlinedButton(
                        onClick = { onSimulateTamper(true) },
                        shape = RoundedCornerShape(8.dp),
                        colors = ButtonDefaults.outlinedButtonColors(contentColor = Color(0xFFFF5252)),
                        contentPadding = PaddingValues(horizontal = 6.dp, vertical = 4.dp),
                        modifier = Modifier
                            .weight(1.1f)
                            .height(30.dp)
                    ) {
                        Icon(
                            imageVector = Icons.Default.Warning,
                            contentDescription = "Tamper APK",
                            modifier = Modifier.size(12.dp)
                        )
                        Spacer(modifier = Modifier.width(2.dp))
                        Text(text = "Ataque APK", fontSize = 9.sp, fontWeight = FontWeight.Bold)
                    }

                    OutlinedButton(
                        onClick = { onSimulateTamper(false) },
                        shape = RoundedCornerShape(8.dp),
                        colors = ButtonDefaults.outlinedButtonColors(contentColor = Color(0xFFFF9100)),
                        contentPadding = PaddingValues(horizontal = 6.dp, vertical = 4.dp),
                        modifier = Modifier
                            .weight(1.1f)
                            .height(30.dp)
                    ) {
                        Icon(
                            imageVector = Icons.Default.BugReport,
                            contentDescription = "Frida Debug",
                            modifier = Modifier.size(12.dp)
                        )
                        Spacer(modifier = Modifier.width(2.dp))
                        Text(text = "Frida/Debug", fontSize = 9.sp, fontWeight = FontWeight.Bold)
                    }

                    Button(
                        onClick = onResetAntiTamper,
                        shape = RoundedCornerShape(8.dp),
                        colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.primaryContainer),
                        contentPadding = PaddingValues(horizontal = 6.dp, vertical = 4.dp),
                        modifier = Modifier
                            .weight(1f)
                            .height(30.dp)
                    ) {
                        Icon(
                            imageVector = Icons.Default.Refresh,
                            contentDescription = "Restaurar",
                            modifier = Modifier.size(12.dp)
                        )
                        Spacer(modifier = Modifier.width(2.dp))
                        Text(text = "Reset", fontSize = 9.sp, fontWeight = FontWeight.Bold)
                    }
                }

                // 3. POST-QUANTUM HYBRID HANDSHAKE SUB-SECTION (ML-KEM-768 + CURVE25519)
                HorizontalDivider(
                    modifier = Modifier.padding(vertical = 14.dp),
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.12f)
                )

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Icon(
                            imageVector = Icons.Default.VpnKey,
                            contentDescription = "Post Quantum",
                            tint = if (isQuantumAttestationVerified) Color(0xFF00E676) else Color(0xFFFF9100),
                            modifier = Modifier.size(16.dp)
                        )
                        Spacer(modifier = Modifier.width(8.dp))
                        Column {
                            Text(
                                text = "HANDSHAKE HÍBRIDO POST-CUÁNTICO",
                                style = MaterialTheme.typography.labelSmall,
                                fontSize = 9.sp,
                                fontWeight = FontWeight.Bold,
                                color = if (isQuantumAttestationVerified) Color(0xFF00E676) else Color(0xFFFF9100)
                            )
                            Text(
                                text = quantumAttestationStatus,
                                style = MaterialTheme.typography.bodySmall,
                                fontSize = 10.sp,
                                fontFamily = FontFamily.Monospace,
                                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.8f)
                            )
                        }
                    }

                    Surface(
                        shape = RoundedCornerShape(8.dp),
                        color = if (isQuantumAttestationVerified) Color(0xFF00E676).copy(alpha = 0.15f) else Color(0xFFFF9100).copy(alpha = 0.15f)
                    ) {
                        Text(
                            text = if (isQuantumAttestationVerified) "${quantumSecretBitStrength}-BIT PQC" else "CLÁSICO 256B",
                            style = MaterialTheme.typography.labelSmall,
                            fontSize = 9.sp,
                            fontWeight = FontWeight.Bold,
                            color = if (isQuantumAttestationVerified) Color(0xFF00E676) else Color(0xFFFF9100),
                            modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp)
                        )
                    }
                }

                Spacer(modifier = Modifier.height(10.dp))

                // Kyber768 & Curve25519 Key Details Box
                Surface(
                    shape = RoundedCornerShape(8.dp),
                    color = MaterialTheme.colorScheme.surface,
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Column(modifier = Modifier.padding(10.dp)) {
                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.SpaceBetween,
                            modifier = Modifier.fillMaxWidth()
                        ) {
                            Text(
                                text = "Ciphertext ML-KEM-768 (Kyber):",
                                style = MaterialTheme.typography.labelSmall,
                                fontSize = 9.sp,
                                color = MaterialTheme.colorScheme.secondary,
                                fontWeight = FontWeight.Bold
                            )
                            Text(
                                text = "Latencia: ${String.format("%.1f", hybridHandshakeLatencyMs)} ms",
                                fontSize = 9.sp,
                                fontFamily = FontFamily.Monospace,
                                color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.6f)
                            )
                        }
                        Text(
                            text = hybridKemCiphertextHash,
                            style = MaterialTheme.typography.bodySmall,
                            fontSize = 9.sp,
                            fontFamily = FontFamily.Monospace,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.8f)
                        )

                        Spacer(modifier = Modifier.height(6.dp))

                        Text(
                            text = "Punto ECDH Curve25519:",
                            style = MaterialTheme.typography.labelSmall,
                            fontSize = 9.sp,
                            color = MaterialTheme.colorScheme.secondary,
                            fontWeight = FontWeight.Bold
                        )
                        Text(
                            text = hybridCurve25519Point,
                            style = MaterialTheme.typography.bodySmall,
                            fontSize = 9.sp,
                            fontFamily = FontFamily.Monospace,
                            maxLines = 1,
                            overflow = TextOverflow.Ellipsis,
                            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.8f)
                        )
                    }
                }

                Spacer(modifier = Modifier.height(10.dp))

                // Controls: Toggle PQC ML-KEM-768 & Manual Handshake trigger
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    FilterChip(
                        selected = isMlKem768Enabled,
                        onClick = onToggleMlKem768,
                        label = {
                            Text(
                                text = if (isMlKem768Enabled) "Capa ML-KEM-768: ON" else "PQC Kyber: OFF",
                                fontSize = 10.sp,
                                fontWeight = FontWeight.Bold
                            )
                        },
                        leadingIcon = {
                            Icon(
                                imageVector = if (isMlKem768Enabled) Icons.Default.CheckCircle else Icons.Default.Cancel,
                                contentDescription = "PQC Toggle",
                                modifier = Modifier.size(12.dp)
                            )
                        },
                        modifier = Modifier.height(30.dp)
                    )

                    Button(
                        onClick = onExecuteHybridHandshake,
                        shape = RoundedCornerShape(8.dp),
                        colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.primary),
                        contentPadding = PaddingValues(horizontal = 10.dp, vertical = 4.dp),
                        modifier = Modifier.height(30.dp)
                    ) {
                        Icon(
                            imageVector = Icons.Default.VpnKey,
                            contentDescription = "Handshake PQC",
                            modifier = Modifier.size(12.dp)
                        )
                        Spacer(modifier = Modifier.width(4.dp))
                        Text(text = "Handshake PQC", fontSize = 10.sp, fontWeight = FontWeight.Bold)
                    }
                }

                Spacer(modifier = Modifier.height(12.dp))

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(8.dp)
                ) {
                    Surface(
                        shape = RoundedCornerShape(8.dp),
                        color = MaterialTheme.colorScheme.surface,
                        modifier = Modifier.weight(1f)
                    ) {
                        Row(
                            modifier = Modifier.padding(8.dp),
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Icon(
                                imageVector = Icons.Default.Lock,
                                contentDescription = "Enclave",
                                tint = MaterialTheme.colorScheme.primary,
                                modifier = Modifier.size(14.dp)
                            )
                            Spacer(modifier = Modifier.width(6.dp))
                            Text(
                                text = "ML-KEM-768 Kyber",
                                style = MaterialTheme.typography.labelSmall,
                                fontSize = 9.sp,
                                fontFamily = FontFamily.Monospace,
                                color = MaterialTheme.colorScheme.onSurface
                            )
                        }
                    }

                    Surface(
                        shape = RoundedCornerShape(8.dp),
                        color = MaterialTheme.colorScheme.surface,
                        modifier = Modifier.weight(1f)
                    ) {
                        Row(
                            modifier = Modifier.padding(8.dp),
                            verticalAlignment = Alignment.CenterVertically
                        ) {
                            Icon(
                                imageVector = Icons.Default.VerifiedUser,
                                contentDescription = "Anti tamper",
                                tint = if (isAntiTamperVerified) Color(0xFF00E676) else Color(0xFFFF5252),
                                modifier = Modifier.size(14.dp)
                            )
                            Spacer(modifier = Modifier.width(6.dp))
                            Text(
                                text = if (isAntiTamperVerified) "Anti-Tamper OK" else "Tamper Bloqueado",
                                style = MaterialTheme.typography.labelSmall,
                                fontSize = 9.sp,
                                fontFamily = FontFamily.Monospace,
                                color = if (isAntiTamperVerified) MaterialTheme.colorScheme.onSurface else Color(0xFFFF5252)
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
fun ClusterScalabilityOrchestratorCard(
    selectedNode: String,
    allocatedCores: Int,
    allocatedRamGb: Int,
    hardwareCodec: String,
    isMigratingSession: Boolean,
    migrationProgressPercent: Int,
    migrationStatusText: String,
    migrationDowntimeMs: Float,
    transferredDirtyPagesMb: Float,
    onChangeClusterNode: (String) -> Unit,
    onChangeAllocatedCores: (Int) -> Unit,
    onChangeHardwareCodec: (String) -> Unit,
    onExecuteLiveMigration: (String) -> Unit
) {
    Card(
        shape = RoundedCornerShape(24.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
        modifier = Modifier
            .fillMaxWidth()
            .testTag("cluster_orchestrator_card")
    ) {
        Column(modifier = Modifier.padding(24.dp)) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween,
                modifier = Modifier.fillMaxWidth()
            ) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Box(
                        modifier = Modifier
                            .size(36.dp)
                            .clip(CircleShape)
                            .background(MaterialTheme.colorScheme.secondaryContainer),
                        contentAlignment = Alignment.Center
                    ) {
                        Icon(
                            imageVector = Icons.Default.Dns,
                            contentDescription = "Cluster Scaling",
                            tint = MaterialTheme.colorScheme.secondary,
                            modifier = Modifier.size(20.dp)
                        )
                    }
                    Spacer(modifier = Modifier.width(12.dp))
                    Column {
                        Text(
                            text = "ORQUESTADOR DE ESCALABILIDAD & CLÚSTER",
                            style = MaterialTheme.typography.labelSmall,
                            fontWeight = FontWeight.Bold,
                            color = MaterialTheme.colorScheme.secondary,
                            letterSpacing = 1.2.sp
                        )
                        Text(
                            text = "Balanceo de Carga y Asignación de Recursos VM",
                            style = MaterialTheme.typography.bodySmall,
                            color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.5f)
                        )
                    }
                }

                Surface(
                    shape = RoundedCornerShape(8.dp),
                    color = MaterialTheme.colorScheme.secondary.copy(alpha = 0.12f)
                ) {
                    Text(
                        text = "MULTI-NODE",
                        style = MaterialTheme.typography.labelSmall,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.secondary,
                        fontSize = 10.sp,
                        modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp)
                    )
                }
            }

            Spacer(modifier = Modifier.height(16.dp))

            Column(
                modifier = Modifier
                    .fillMaxWidth()
                    .clip(RoundedCornerShape(14.dp))
                    .background(MaterialTheme.colorScheme.background)
                    .padding(14.dp)
            ) {
                Text(
                    text = "NODO DEL CLÚSTER VIRTUAL",
                    style = MaterialTheme.typography.labelSmall,
                    fontSize = 9.sp,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.secondary
                )

                Spacer(modifier = Modifier.height(8.dp))

                listOf(
                    "Node-Alpha (x86_64 High-Perf)",
                    "Node-Beta (ARM64 Ampere)",
                    "Node-Gamma (Edge Micro-Host)"
                ).forEach { node ->
                    val isSelected = (selectedNode == node)
                    Row(
                        modifier = Modifier
                            .fillMaxWidth()
                            .padding(vertical = 3.dp)
                            .clip(RoundedCornerShape(8.dp))
                            .background(if (isSelected) MaterialTheme.colorScheme.secondaryContainer else MaterialTheme.colorScheme.surface)
                            .clickable { onChangeClusterNode(node) }
                            .padding(horizontal = 10.dp, vertical = 6.dp),
                        verticalAlignment = Alignment.CenterVertically,
                        horizontalArrangement = Arrangement.SpaceBetween
                    ) {
                        Text(
                            text = node,
                            style = MaterialTheme.typography.bodySmall,
                            fontWeight = if (isSelected) FontWeight.Bold else FontWeight.Normal,
                            color = if (isSelected) MaterialTheme.colorScheme.onSecondaryContainer else MaterialTheme.colorScheme.onSurface
                        )
                        if (isSelected) {
                            Text(
                                text = "ACTIVO",
                                style = MaterialTheme.typography.labelSmall,
                                fontSize = 9.sp,
                                fontWeight = FontWeight.ExtraBold,
                                color = MaterialTheme.colorScheme.secondary
                            )
                        }
                    }
                }

                Spacer(modifier = Modifier.height(12.dp))

                Text(
                    text = "CODEC DE ACELERACIÓN HARDWARE (COMPATIBILIDAD)",
                    style = MaterialTheme.typography.labelSmall,
                    fontSize = 9.sp,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.secondary
                )

                Spacer(modifier = Modifier.height(6.dp))

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(6.dp)
                ) {
                    listOf("AV1 (Hardware Accel)", "H.265 / HEVC", "VP9 Zero-Lag", "H.264 Soft").forEach { codec ->
                        val isSelected = (hardwareCodec == codec)
                        Box(
                            modifier = Modifier
                                .weight(1f)
                                .clip(RoundedCornerShape(8.dp))
                                .background(if (isSelected) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.surface)
                                .clickable { onChangeHardwareCodec(codec) }
                                .padding(vertical = 6.dp, horizontal = 4.dp),
                            contentAlignment = Alignment.Center
                        ) {
                            Text(
                                text = codec.split(" ").first(),
                                style = MaterialTheme.typography.labelSmall,
                                fontSize = 10.sp,
                                fontWeight = FontWeight.Bold,
                                color = if (isSelected) MaterialTheme.colorScheme.background else MaterialTheme.colorScheme.onSurface
                            )
                        }
                    }
                }

                // LIVE MIGRATION SUB-SECTION
                HorizontalDivider(
                    modifier = Modifier.padding(vertical = 12.dp),
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.12f)
                )

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.SpaceBetween,
                    verticalAlignment = Alignment.CenterVertically
                ) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Icon(
                            imageVector = Icons.Default.SwapHoriz,
                            contentDescription = "Live Migration",
                            tint = Color(0xFF00B0FF),
                            modifier = Modifier.size(16.dp)
                        )
                        Spacer(modifier = Modifier.width(6.dp))
                        Text(
                            text = "MIGRACIÓN EN CALIENTE (LIVE MIGRATION)",
                            style = MaterialTheme.typography.labelSmall,
                            fontSize = 9.sp,
                            fontWeight = FontWeight.Bold,
                            color = Color(0xFF00B0FF)
                        )
                    }

                    Surface(
                        shape = RoundedCornerShape(8.dp),
                        color = Color(0xFF00B0FF).copy(alpha = 0.15f)
                    ) {
                        Text(
                            text = "DOWNTIME < ${String.format("%.1f", migrationDowntimeMs)} ms",
                            style = MaterialTheme.typography.labelSmall,
                            fontSize = 9.sp,
                            fontWeight = FontWeight.Bold,
                            color = Color(0xFF00B0FF),
                            modifier = Modifier.padding(horizontal = 8.dp, vertical = 4.dp)
                        )
                    }
                }

                Spacer(modifier = Modifier.height(8.dp))

                Text(
                    text = migrationStatusText,
                    style = MaterialTheme.typography.bodySmall,
                    fontSize = 10.sp,
                    fontFamily = FontFamily.Monospace,
                    color = MaterialTheme.colorScheme.onSurface.copy(alpha = 0.8f)
                )

                if (isMigratingSession) {
                    Spacer(modifier = Modifier.height(6.dp))
                    LinearProgressIndicator(
                        progress = { migrationProgressPercent / 100f },
                        modifier = Modifier
                            .fillMaxWidth()
                            .height(6.dp)
                            .clip(RoundedCornerShape(3.dp)),
                        color = Color(0xFF00B0FF),
                        trackColor = MaterialTheme.colorScheme.surface
                    )
                }

                Spacer(modifier = Modifier.height(10.dp))

                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(6.dp)
                ) {
                    listOf(
                        "Node-Alpha (x86_64 High-Perf)",
                        "Node-Beta (ARM64 Ampere)",
                        "Node-Gamma (Edge Micro-Host)"
                    ).forEach { node ->
                        val isCurrentNode = (selectedNode == node)
                        val shortName = node.split(" ").first()
                        OutlinedButton(
                            onClick = { onExecuteLiveMigration(node) },
                            enabled = !isCurrentNode && !isMigratingSession,
                            shape = RoundedCornerShape(8.dp),
                            contentPadding = PaddingValues(horizontal = 6.dp, vertical = 4.dp),
                            modifier = Modifier
                                .weight(1f)
                                .height(32.dp)
                        ) {
                            Text(
                                text = if (isCurrentNode) "$shortName (Activo)" else "Migrar a $shortName",
                                fontSize = 9.sp,
                                fontWeight = FontWeight.Bold,
                                maxLines = 1,
                                overflow = TextOverflow.Ellipsis
                            )
                        }
                    }
                }
            }
        }
    }
}

@Composable
fun InteractiveRemoteScreenCard(
    isConnected: Boolean,
    latestVideoFrame: VideoFramePayload?,
    streamFpsTarget: Int,
    streamResolution: String,
    streamBitrateMbps: Float,
    isStreamPaused: Boolean,
    isAudioSyncEnabled: Boolean,
    onChangeFpsTarget: (Int) -> Unit,
    onChangeResolution: (String) -> Unit,
    onChangeBitrate: (Float) -> Unit,
    onTogglePauseStream: () -> Unit,
    onToggleAudioSync: () -> Unit,
    onCaptureScreenshot: () -> Unit,
    onSendInputEvent: (InputEventPayload) -> Unit
) {
    Card(
        shape = RoundedCornerShape(24.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
        modifier = Modifier
            .fillMaxWidth()
            .testTag("interactive_remote_card")
    ) {
        Column(modifier = Modifier.padding(24.dp)) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                horizontalArrangement = Arrangement.SpaceBetween,
                modifier = Modifier.fillMaxWidth()
            ) {
                Row(verticalAlignment = Alignment.CenterVertically) {
                    Icon(
                        imageVector = Icons.Default.Tv,
                        contentDescription = "Pantalla Remota",
                        tint = MaterialTheme.colorScheme.primary,
                        modifier = Modifier.size(22.dp)
                    )
                    Spacer(modifier = Modifier.width(10.dp))
                    Text(
                        text = "FRAMEBUFFER REMOTO EN VIVO",
                        style = MaterialTheme.typography.titleSmall,
                        fontWeight = FontWeight.Bold,
                        color = MaterialTheme.colorScheme.onSurface
                    )
                }

                IconButton(onClick = onCaptureScreenshot) {
                    Icon(
                        imageVector = Icons.Default.CameraAlt,
                        contentDescription = "Captura",
                        tint = MaterialTheme.colorScheme.primary
                    )
                }
            }

            Spacer(modifier = Modifier.height(14.dp))

            // Framebuffer Display Surface
            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(210.dp)
                    .clip(RoundedCornerShape(16.dp))
                    .background(Color.Black)
                    .border(1.dp, Color(0xFF334155), RoundedCornerShape(16.dp))
                    .clickable {
                        onSendInputEvent(InputEventPayload("TAP_EVENT", x = 400f, y = 300f))
                    },
                contentAlignment = Alignment.Center
            ) {
                if (isConnected && !isStreamPaused) {
                    Canvas(modifier = Modifier.fillMaxSize()) {
                        drawRect(color = Color(0xFF0F172A))
                        drawCircle(
                            color = Color(0xFF00E676).copy(alpha = 0.2f),
                            radius = 180f,
                            center = Offset(size.width / 2, size.height / 2)
                        )
                        drawPath(
                            path = Path().apply {
                                moveTo(size.width * 0.2f, size.height * 0.7f)
                                lineTo(size.width * 0.4f, size.height * 0.3f)
                                lineTo(size.width * 0.6f, size.height * 0.6f)
                                lineTo(size.width * 0.8f, size.height * 0.2f)
                            },
                            color = Color(0xFF00E676),
                            style = androidx.compose.ui.graphics.drawscope.Stroke(width = 4f)
                        )
                    }

                    Column(
                        modifier = Modifier
                            .align(Alignment.TopStart)
                            .padding(12.dp)
                            .background(Color.Black.copy(alpha = 0.6f), RoundedCornerShape(8.dp))
                            .padding(horizontal = 8.dp, vertical = 4.dp)
                    ) {
                        Text(
                            text = "FRAME #${latestVideoFrame?.frameIndex ?: 0} | ${latestVideoFrame?.resolutionWidth}x${latestVideoFrame?.resolutionHeight}",
                            style = MaterialTheme.typography.labelSmall,
                            fontSize = 9.sp,
                            fontFamily = FontFamily.Monospace,
                            color = Color(0xFF00E676)
                        )
                        Text(
                            text = "Bitrate: ${latestVideoFrame?.bitrateKbps ?: 0} Kbps | ${streamFpsTarget} FPS Target",
                            style = MaterialTheme.typography.labelSmall,
                            fontSize = 9.sp,
                            fontFamily = FontFamily.Monospace,
                            color = Color.White
                        )
                    }
                } else if (isStreamPaused) {
                    Column(horizontalAlignment = Alignment.CenterHorizontally) {
                        Icon(
                            imageVector = Icons.Default.PauseCircle,
                            contentDescription = "Pausado",
                            tint = Color(0xFFFFD600),
                            modifier = Modifier.size(48.dp)
                        )
                        Spacer(modifier = Modifier.height(8.dp))
                        Text(
                            text = "Transmisión en Pausa",
                            style = MaterialTheme.typography.titleSmall,
                            color = Color(0xFFFFD600)
                        )
                    }
                } else {
                    Column(horizontalAlignment = Alignment.CenterHorizontally) {
                        Icon(
                            imageVector = Icons.Default.SignalWifiOff,
                            contentDescription = "Sin señal",
                            tint = Color.Gray,
                            modifier = Modifier.size(48.dp)
                        )
                        Spacer(modifier = Modifier.height(8.dp))
                        Text(
                            text = "Sin Transmisión de Video",
                            style = MaterialTheme.typography.titleSmall,
                            color = Color.Gray
                        )
                    }
                }
            }

            Spacer(modifier = Modifier.height(14.dp))

            // Streaming Controls Bar
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.SpaceBetween,
                verticalAlignment = Alignment.CenterVertically
            ) {
                Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    listOf(15, 30, 60).forEach { fps ->
                        FilterChip(
                            selected = streamFpsTarget == fps,
                            onClick = { onChangeFpsTarget(fps) },
                            label = { Text("${fps}FPS", fontSize = 11.sp) },
                            modifier = Modifier.height(32.dp)
                        )
                    }
                }

                Row(horizontalArrangement = Arrangement.spacedBy(6.dp)) {
                    IconButton(onClick = onTogglePauseStream, modifier = Modifier.size(36.dp)) {
                        Icon(
                            imageVector = if (isStreamPaused) Icons.Default.PlayArrow else Icons.Default.Pause,
                            contentDescription = "Pausa",
                            tint = MaterialTheme.colorScheme.primary
                        )
                    }

                    IconButton(onClick = onToggleAudioSync, modifier = Modifier.size(36.dp)) {
                        Icon(
                            imageVector = if (isAudioSyncEnabled) Icons.Default.VolumeUp else Icons.Default.VolumeOff,
                            contentDescription = "Audio Sync",
                            tint = if (isAudioSyncEnabled) MaterialTheme.colorScheme.primary else Color.Gray
                        )
                    }
                }
            }

            Spacer(modifier = Modifier.height(12.dp))

            // Virtual Input Buttons
            Row(
                modifier = Modifier.fillMaxWidth(),
                horizontalArrangement = Arrangement.spacedBy(8.dp)
            ) {
                Button(
                    onClick = { onSendInputEvent(InputEventPayload("KEY_PRESS", keyName = "ESC")) },
                    shape = RoundedCornerShape(10.dp),
                    colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.background),
                    modifier = Modifier.weight(1f)
                ) {
                    Text("ESC", fontSize = 11.sp, color = MaterialTheme.colorScheme.onBackground)
                }

                Button(
                    onClick = { onSendInputEvent(InputEventPayload("KEY_PRESS", keyName = "ENTER")) },
                    shape = RoundedCornerShape(10.dp),
                    colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.background),
                    modifier = Modifier.weight(1f)
                ) {
                    Text("ENTER", fontSize = 11.sp, color = MaterialTheme.colorScheme.onBackground)
                }

                Button(
                    onClick = { onSendInputEvent(InputEventPayload("KEY_PRESS", keyName = "SUPER")) },
                    shape = RoundedCornerShape(10.dp),
                    colors = ButtonDefaults.buttonColors(containerColor = MaterialTheme.colorScheme.background),
                    modifier = Modifier.weight(1f)
                ) {
                    Text("SUPER", fontSize = 11.sp, color = MaterialTheme.colorScheme.onBackground)
                }
            }
        }
    }
}

@Composable
fun HeadlessRpcControlCard(
    onSendAdminAction: (AdminActionType, String?, String?) -> Unit
) {
    Card(
        shape = RoundedCornerShape(24.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
        modifier = Modifier
            .fillMaxWidth()
            .testTag("headless_rpc_card")
    ) {
        Column(modifier = Modifier.padding(24.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(
                    imageVector = Icons.Default.AdminPanelSettings,
                    contentDescription = "RPC Admin",
                    tint = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.size(22.dp)
                )
                Spacer(modifier = Modifier.width(10.dp))
                Text(
                    text = "ACCIONES HEADLESS RPC",
                    style = MaterialTheme.typography.titleSmall,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.onSurface
                )
            }

            Spacer(modifier = Modifier.height(16.dp))

            Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                Button(
                    onClick = { onSendAdminAction(AdminActionType.NET_BLOCK, null, null) },
                    shape = RoundedCornerShape(12.dp),
                    colors = ButtonDefaults.buttonColors(containerColor = Color(0xFFD32F2F)),
                    modifier = Modifier.weight(1f)
                ) {
                    Text("Bloquear Red", fontSize = 11.sp, fontWeight = FontWeight.Bold)
                }

                Button(
                    onClick = { onSendAdminAction(AdminActionType.NET_ALLOW, null, null) },
                    shape = RoundedCornerShape(12.dp),
                    colors = ButtonDefaults.buttonColors(containerColor = Color(0xFF388E3C)),
                    modifier = Modifier.weight(1f)
                ) {
                    Text("Permitir Red", fontSize = 11.sp, fontWeight = FontWeight.Bold)
                }
            }

            Spacer(modifier = Modifier.height(10.dp))

            Row(modifier = Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(10.dp)) {
                OutlinedButton(
                    onClick = { onSendAdminAction(AdminActionType.ROLLBACK, null, null) },
                    shape = RoundedCornerShape(12.dp),
                    modifier = Modifier.weight(1f)
                ) {
                    Text("Rollback OS", fontSize = 11.sp)
                }

                OutlinedButton(
                    onClick = { onSendAdminAction(AdminActionType.UPDATE_LATEST, null, null) },
                    shape = RoundedCornerShape(12.dp),
                    modifier = Modifier.weight(1f)
                ) {
                    Text("Actualizar OS", fontSize = 11.sp)
                }
            }
        }
    }
}

@Composable
fun ConsoleLogsCard(consoleLogs: List<String>) {
    Card(
        shape = RoundedCornerShape(24.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surface),
        modifier = Modifier
            .fillMaxWidth()
            .testTag("console_logs_card")
    ) {
        Column(modifier = Modifier.padding(24.dp)) {
            Row(verticalAlignment = Alignment.CenterVertically) {
                Icon(
                    imageVector = Icons.Default.Terminal,
                    contentDescription = "Consola Logs",
                    tint = Color(0xFF00E676),
                    modifier = Modifier.size(22.dp)
                )
                Spacer(modifier = Modifier.width(10.dp))
                Text(
                    text = "TERMINAL DE TELEMETRÍA Y LOGS",
                    style = MaterialTheme.typography.titleSmall,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.onSurface
                )
            }

            Spacer(modifier = Modifier.height(14.dp))

            Box(
                modifier = Modifier
                    .fillMaxWidth()
                    .height(140.dp)
                    .clip(RoundedCornerShape(14.dp))
                    .background(Color.Black)
                    .padding(12.dp)
            ) {
                Column(modifier = Modifier.verticalScroll(rememberScrollState())) {
                    consoleLogs.forEach { logLine ->
                        Text(
                            text = logLine,
                            style = MaterialTheme.typography.labelSmall,
                            fontSize = 10.sp,
                            fontFamily = FontFamily.Monospace,
                            color = Color(0xFF00E676)
                        )
                    }
                }
            }
        }
    }
}
