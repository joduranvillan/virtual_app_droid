package com.example

import android.content.Context
import androidx.biometric.BiometricManager
import androidx.biometric.BiometricPrompt
import androidx.core.content.ContextCompat
import androidx.fragment.app.FragmentActivity
import java.security.MessageDigest
import java.text.SimpleDateFormat
import java.util.Date
import java.util.Locale

enum class BiometricHardwareStatus(val label: String, val isReady: Boolean) {
    STRONG_HARDWARE_READY("Sensor Hardware OK (Biometría Clase 3 Fuerte)", true),
    WEAK_HARDWARE_READY("Sensor Hardware OK (Biometría Clase 2 Estándar)", true),
    NO_BIOMETRICS_ENROLLED("Sin Huella/Rostro Registrado en el Dispositivo", false),
    NO_HARDWARE("Sin Hardware Biométrico en el Dispositivo", false),
    HARDWARE_UNAVAILABLE("Sensor Biométrico Ocupado / No Disponible", false),
    SECURITY_UPDATE_REQUIRED("Requiere Actualización de Seguridad de Android", false),
    UNKNOWN("Estado de Sensor Desconocido", false)
}

data class BiometricAuthEvent(
    val timestamp: String,
    val type: String,
    val isSuccess: Boolean,
    val message: String,
    val hardwareTokenHash: String
)

object BiometricAuthManager {

    fun checkBiometricStatus(context: Context): BiometricHardwareStatus {
        val biometricManager = BiometricManager.from(context)
        return when (biometricManager.canAuthenticate(BiometricManager.Authenticators.BIOMETRIC_STRONG)) {
            BiometricManager.BIOMETRIC_SUCCESS -> BiometricHardwareStatus.STRONG_HARDWARE_READY
            BiometricManager.BIOMETRIC_ERROR_NONE_ENROLLED -> BiometricHardwareStatus.NO_BIOMETRICS_ENROLLED
            BiometricManager.BIOMETRIC_ERROR_NO_HARDWARE -> BiometricHardwareStatus.NO_HARDWARE
            BiometricManager.BIOMETRIC_ERROR_HW_UNAVAILABLE -> BiometricHardwareStatus.HARDWARE_UNAVAILABLE
            BiometricManager.BIOMETRIC_ERROR_SECURITY_UPDATE_REQUIRED -> BiometricHardwareStatus.SECURITY_UPDATE_REQUIRED
            else -> {
                // Check if weak biometrics or device credentials are valid
                when (biometricManager.canAuthenticate(BiometricManager.Authenticators.BIOMETRIC_WEAK or BiometricManager.Authenticators.DEVICE_CREDENTIAL)) {
                    BiometricManager.BIOMETRIC_SUCCESS -> BiometricHardwareStatus.WEAK_HARDWARE_READY
                    BiometricManager.BIOMETRIC_ERROR_NONE_ENROLLED -> BiometricHardwareStatus.NO_BIOMETRICS_ENROLLED
                    else -> BiometricHardwareStatus.UNKNOWN
                }
            }
        }
    }

    fun promptBiometricAuthentication(
        activity: FragmentActivity,
        title: String = "Desbloquear Bóveda Confidencial",
        subtitle: String = "Autenticación Biométrica Requerida (Huella / Rostro)",
        description: String = "Acceso seguro a hipervisores, claves PQC y stream VirtIO cifrado.",
        onSuccess: (token: String) -> Unit,
        onError: (errorCode: Int, errorMsg: String) -> Unit,
        onFailed: () -> Unit
    ) {
        val executor = ContextCompat.getMainExecutor(activity)

        val biometricPrompt = BiometricPrompt(
            activity,
            executor,
            object : BiometricPrompt.AuthenticationCallback() {
                override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                    super.onAuthenticationSucceeded(result)
                    val derivedToken = generateBiometricHardwareToken(result.authenticationType)
                    onSuccess(derivedToken)
                }

                override fun onAuthenticationError(errorCode: Int, errString: CharSequence) {
                    super.onAuthenticationError(errorCode, errString)
                    onError(errorCode, errString.toString())
                }

                override fun onAuthenticationFailed() {
                    super.onAuthenticationFailed()
                    onFailed()
                }
            }
        )

        val promptInfo = BiometricPrompt.PromptInfo.Builder()
            .setTitle(title)
            .setSubtitle(subtitle)
            .setDescription(description)
            .setNegativeButtonText("Cancelar")
            .setAllowedAuthenticators(BiometricManager.Authenticators.BIOMETRIC_STRONG or BiometricManager.Authenticators.BIOMETRIC_WEAK)
            .build()

        try {
            biometricPrompt.authenticate(promptInfo)
        } catch (e: Exception) {
            onError(-1, e.message ?: "Error iniciando el sensor biométrico")
        }
    }

    fun generateBiometricHardwareToken(authType: Int = 1): String {
        val now = System.currentTimeMillis()
        val seed = "VAULT_BIOMETRIC_HW_TOKEN_${now}_AUTH_TYPE_${authType}_${(100000..999999).random()}"
        val md = MessageDigest.getInstance("SHA-256")
        val digest = md.digest(seed.toByteArray())
        val prefix = when (authType) {
            1 -> "BIO-FP-"
            2 -> "BIO-PIN-"
            3 -> "BIO-FACE-"
            else -> "BIO-HW-"
        }
        return prefix + digest.take(8).joinToString("") { "%02x".format(it) }.uppercase()
    }

    fun currentTimestamp(): String {
        val sdf = SimpleDateFormat("HH:mm:ss", Locale.getDefault())
        return sdf.format(Date())
    }
}
