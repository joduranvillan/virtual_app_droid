package com.example

import org.junit.Assert.*
import org.junit.Test

class BiometricVaultAuthTest {

    @Test
    fun testGenerateBiometricHardwareToken() {
        val tokenFingerprint = BiometricAuthManager.generateBiometricHardwareToken(authType = 1)
        assertTrue(tokenFingerprint.startsWith("BIO-FP-"))
        assertEquals(23, tokenFingerprint.length)

        val tokenFace = BiometricAuthManager.generateBiometricHardwareToken(authType = 3)
        assertTrue(tokenFace.startsWith("BIO-FACE-"))

        val tokenPin = BiometricAuthManager.generateBiometricHardwareToken(authType = 2)
        assertTrue(tokenPin.startsWith("BIO-PIN-"))
    }

    @Test
    fun testBiometricEventModel() {
        val token = BiometricAuthManager.generateBiometricHardwareToken(authType = 1)
        val event = BiometricAuthEvent(
            timestamp = "14:35:10",
            type = "Sensor de Huella Dactilar",
            isSuccess = true,
            message = "Desbloqueo exitoso",
            hardwareTokenHash = token
        )

        assertTrue(event.isSuccess)
        assertEquals("Sensor de Huella Dactilar", event.type)
        assertEquals(token, event.hardwareTokenHash)
    }
}
