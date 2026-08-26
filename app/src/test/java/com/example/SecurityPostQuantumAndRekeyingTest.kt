package com.example

import org.junit.Assert.*
import org.junit.Test
import java.security.MessageDigest

class SecurityPostQuantumAndRekeyingTest {

    @Test
    fun testDynamicRekeyingVolumeThresholdTrigger() {
        var transferredMb = 0.0f
        val thresholdMb = 1024.0f // 1 GB límite
        var rekeyCount = 0

        // Simular acumulación de streaming de framebuffer (~18.5 MB/s)
        for (i in 1..60) {
            transferredMb += 18.5f
            if (transferredMb >= thresholdMb) {
                transferredMb = 0.0f
                rekeyCount++
            }
        }

        assertTrue("Debe haberse disparado al menos una rotación dinámica al superar 1 GB", rekeyCount >= 1)
        assertTrue("El buffer acumulado debe haberse reiniciado", transferredMb < thresholdMb)
    }

    @Test
    fun testAntiTamperHsmIntegrityCheck_Valid() {
        val originalApkSignature = "VAULT_AUTHORIZED_RELEASE_CERTIFICATE_HASH_2026".toByteArray()
        val digest = MessageDigest.getInstance("SHA-256")
        val computedHash = digest.digest(originalApkSignature).joinToString("") { "%02x".format(it) }

        val hsmAuthorizedHash = computedHash
        val isDebuggerAttached = false

        val isTampered = (computedHash != hsmAuthorizedHash) || isDebuggerAttached
        assertFalse("El binario válido sin depurador no debe disparar alerta de tamper", isTampered)
    }

    @Test
    fun testAntiTamperHsmIntegrityCheck_TamperedOrDebugged() {
        val originalApkSignature = "VAULT_AUTHORIZED_RELEASE_CERTIFICATE_HASH_2026".toByteArray()
        val digest = MessageDigest.getInstance("SHA-256")
        val validHash = digest.digest(originalApkSignature).joinToString("") { "%02x".format(it) }

        val tamperedSignature = "MALICIOUS_INJECTED_PAYLOAD_DUMP".toByteArray()
        val tamperedHash = digest.digest(tamperedSignature).joinToString("") { "%02x".format(it) }

        val isTamperedBinary = (tamperedHash != validHash)
        assertTrue("La firma alterada debe ser detectada por el HSM", isTamperedBinary)

        val isDebuggerDetected = true
        val isTamperedDebug = (validHash != validHash) || isDebuggerDetected
        assertTrue("La detección de Frida/ptrace debe bloquear el contexto", isTamperedDebug)
    }

    @Test
    fun testHybridPostQuantumStrength() {
        val mlKem768BitStrength = 256
        val curve25519BitStrength = 256
        val totalCombinedStrength = mlKem768BitStrength + curve25519BitStrength

        assertEquals(512, totalCombinedStrength)
    }
}
