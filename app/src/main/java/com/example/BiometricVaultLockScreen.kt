package com.example

import androidx.compose.animation.*
import androidx.compose.animation.core.*
import androidx.compose.foundation.background
import androidx.compose.foundation.border
import androidx.compose.foundation.clickable
import androidx.compose.foundation.interaction.MutableInteractionSource
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
import androidx.compose.ui.draw.scale
import androidx.compose.ui.graphics.Brush
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.PasswordVisualTransformation
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp
import androidx.compose.ui.unit.sp

@Composable
fun BiometricVaultLockScreen(
    hardwareStatus: BiometricHardwareStatus,
    failedAttempts: Int,
    isLockout: Boolean,
    lockoutSecondsRemaining: Int,
    lastAuthMessage: String?,
    onTriggerBiometricPrompt: () -> Unit,
    onSimulateFingerprintScan: (isSuccess: Boolean) -> Unit,
    onSimulateFaceRecognitionScan: (isSuccess: Boolean) -> Unit,
    onUnlockWithPin: (pin: String) -> Boolean
) {
    var pinInput by remember { mutableStateOf("") }
    var pinError by remember { mutableStateOf(false) }
    var showPinFallback by remember { mutableStateOf(false) }
    var isScanningFace by remember { mutableStateOf(false) }
    var isScanningFinger by remember { mutableStateOf(false) }

    // Pulsing animation for the biometric shield ring
    val infiniteTransition = rememberInfiniteTransition(label = "pulse_transition")
    val pulseScale by infiniteTransition.animateFloat(
        initialValue = 0.95f,
        targetValue = 1.05f,
        animationSpec = infiniteRepeatable(
            animation = tween(1200, easing = FastOutSlowInEasing),
            repeatMode = RepeatMode.Reverse
        ),
        label = "pulse_scale"
    )
    val glowAlpha by infiniteTransition.animateFloat(
        initialValue = 0.25f,
        targetValue = 0.65f,
        animationSpec = infiniteRepeatable(
            animation = tween(1200, easing = FastOutSlowInEasing),
            repeatMode = RepeatMode.Reverse
        ),
        label = "glow_alpha"
    )

    Column(
        modifier = Modifier
            .fillMaxSize()
            .background(
                Brush.verticalGradient(
                    colors = listOf(
                        Color(0xFF0A0E1A),
                        Color(0xFF0F172A),
                        Color(0xFF090D16)
                    )
                )
            )
            .verticalScroll(rememberScrollState())
            .padding(horizontal = 24.dp, vertical = 32.dp),
        horizontalAlignment = Alignment.CenterHorizontally
    ) {
        Spacer(modifier = Modifier.height(16.dp))

        // 1. TOP SECURITY BADGE
        Surface(
            shape = RoundedCornerShape(20.dp),
            color = Color(0xFF1E293B),
            border = androidx.compose.foundation.BorderStroke(1.dp, Color(0xFF00E676).copy(alpha = 0.4f)),
            modifier = Modifier.testTag("security_enclave_badge")
        ) {
            Row(
                verticalAlignment = Alignment.CenterVertically,
                modifier = Modifier.padding(horizontal = 16.dp, vertical = 8.dp)
            ) {
                Box(
                    modifier = Modifier
                        .size(8.dp)
                        .clip(CircleShape)
                        .background(Color(0xFF00E676))
                )
                Spacer(modifier = Modifier.width(8.dp))
                Text(
                    text = "ENCLAVE SEGURO ACTIVO (CLASS 3 BIOMETRICS)",
                    style = MaterialTheme.typography.labelSmall,
                    fontWeight = FontWeight.Bold,
                    color = Color(0xFF00E676),
                    letterSpacing = 1.1.sp
                )
            }
        }

        Spacer(modifier = Modifier.height(28.dp))

        // 2. BIOMETRIC RADAR SCANNER ANIMATION
        Box(
            contentAlignment = Alignment.Center,
            modifier = Modifier
                .size(160.dp)
                .scale(pulseScale)
        ) {
            // Outer Glowing Ring
            Box(
                modifier = Modifier
                    .fillMaxSize()
                    .clip(CircleShape)
                    .background(
                        Brush.radialGradient(
                            colors = listOf(
                                Color(0xFF00E676).copy(alpha = glowAlpha),
                                Color(0xFF00B0FF).copy(alpha = glowAlpha * 0.4f),
                                Color.Transparent
                            )
                        )
                    )
            )

            // Inner Ring Border
            Box(
                modifier = Modifier
                    .size(120.dp)
                    .clip(CircleShape)
                    .border(
                        width = 2.dp,
                        brush = Brush.sweepGradient(
                            listOf(Color(0xFF00E676), Color(0xFF00B0FF), Color(0xFF00E676))
                        ),
                        shape = CircleShape
                    )
                    .background(Color(0xFF1E2638)),
                contentAlignment = Alignment.Center
            ) {
                Icon(
                    imageVector = Icons.Default.Lock,
                    contentDescription = "Bóveda Bloqueada",
                    tint = if (isLockout) Color(0xFFFF5252) else Color(0xFF00E676),
                    modifier = Modifier.size(52.dp)
                )
            }
        }

        Spacer(modifier = Modifier.height(24.dp))

        // 3. TITLE & SUBTITLE
        Text(
            text = "BÓVEDA CONFIDENCIAL",
            style = MaterialTheme.typography.headlineMedium,
            fontWeight = FontWeight.Black,
            color = Color.White,
            letterSpacing = 1.5.sp
        )

        Spacer(modifier = Modifier.height(6.dp))

        Text(
            text = "Autenticación Biométrica Obligatoria para Acceso al Hipervisor y Claves Criptográficas",
            style = MaterialTheme.typography.bodyMedium,
            color = Color(0xFF94A3B8),
            textAlign = TextAlign.Center,
            modifier = Modifier.padding(horizontal = 16.dp)
        )

        Spacer(modifier = Modifier.height(24.dp))

        // 4. LOCKOUT WARNING OR FEEDBACK MESSAGE
        if (isLockout) {
            Surface(
                shape = RoundedCornerShape(16.dp),
                color = Color(0xFF450A0A),
                border = androidx.compose.foundation.BorderStroke(1.dp, Color(0xFFFF5252)),
                modifier = Modifier
                    .fillMaxWidth()
                    .testTag("lockout_warning_banner")
            ) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    modifier = Modifier.padding(16.dp)
                ) {
                    Icon(
                        imageVector = Icons.Default.Warning,
                        contentDescription = "Bloqueo",
                        tint = Color(0xFFFF5252),
                        modifier = Modifier.size(24.dp)
                    )
                    Spacer(modifier = Modifier.width(12.dp))
                    Column {
                        Text(
                            text = "ACCESO BLOQUEADO POR REINTENTOS",
                            style = MaterialTheme.typography.labelMedium,
                            fontWeight = FontWeight.Bold,
                            color = Color(0xFFFF5252)
                        )
                        Text(
                            text = "Espera $lockoutSecondsRemaining segundos para reintentar o usa el PIN de respaldo.",
                            style = MaterialTheme.typography.bodySmall,
                            color = Color(0xFFFECACA)
                        )
                    }
                }
            }
            Spacer(modifier = Modifier.height(20.dp))
        } else if (lastAuthMessage != null) {
            Surface(
                shape = RoundedCornerShape(12.dp),
                color = Color(0xFF1E293B),
                modifier = Modifier.fillMaxWidth()
            ) {
                Text(
                    text = lastAuthMessage,
                    style = MaterialTheme.typography.bodySmall,
                    color = Color(0xFFE2E8F0),
                    textAlign = TextAlign.Center,
                    modifier = Modifier.padding(12.dp)
                )
            }
            Spacer(modifier = Modifier.height(20.dp))
        }

        // 5. PRIMARY ACTION: BIOMETRIC PROMPT BUTTON
        Button(
            onClick = onTriggerBiometricPrompt,
            enabled = !isLockout,
            shape = RoundedCornerShape(16.dp),
            colors = ButtonDefaults.buttonColors(
                containerColor = Color(0xFF00E676),
                contentColor = Color.Black,
                disabledContainerColor = Color(0xFF334155),
                disabledContentColor = Color(0xFF64748B)
            ),
            modifier = Modifier
                .fillMaxWidth()
                .height(56.dp)
                .testTag("biometric_unlock_button")
        ) {
            Icon(
                imageVector = Icons.Default.Fingerprint,
                contentDescription = "Huella",
                modifier = Modifier.size(26.dp)
            )
            Spacer(modifier = Modifier.width(12.dp))
            Text(
                text = "Desbloquear con Huella / Rostro",
                style = MaterialTheme.typography.titleMedium,
                fontWeight = FontWeight.Bold
            )
        }

        Spacer(modifier = Modifier.height(20.dp))

        // 6. BIOMETRIC SENSORS SIMULATOR & DIAGNOSTIC CARD
        Card(
            shape = RoundedCornerShape(20.dp),
            colors = CardDefaults.cardColors(containerColor = Color(0xFF1E2638)),
            modifier = Modifier
                .fillMaxWidth()
                .testTag("biometric_sensors_card")
        ) {
            Column(modifier = Modifier.padding(20.dp)) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween,
                    modifier = Modifier.fillMaxWidth()
                ) {
                    Text(
                        text = "SENSORES BIOMÉTRICOS DEL DISPOSITIVO",
                        style = MaterialTheme.typography.labelSmall,
                        fontWeight = FontWeight.Bold,
                        color = Color(0xFF00B0FF),
                        letterSpacing = 1.1.sp
                    )

                    Surface(
                        shape = RoundedCornerShape(6.dp),
                        color = if (hardwareStatus.isReady) Color(0xFF00E676).copy(alpha = 0.2f) else Color(0xFFFFB300).copy(alpha = 0.2f)
                    ) {
                        Text(
                            text = if (hardwareStatus.isReady) "LISTO" else "SIMULADOR",
                            style = MaterialTheme.typography.labelSmall,
                            fontWeight = FontWeight.Bold,
                            color = if (hardwareStatus.isReady) Color(0xFF00E676) else Color(0xFFFFB300),
                            modifier = Modifier.padding(horizontal = 8.dp, vertical = 2.dp)
                        )
                    }
                }

                Spacer(modifier = Modifier.height(6.dp))
                Text(
                    text = hardwareStatus.label,
                    style = MaterialTheme.typography.bodySmall,
                    color = Color(0xFF94A3B8)
                )

                Spacer(modifier = Modifier.height(16.dp))

                // Dual Sensor Tap Actions: Fingerprint & Face Recognition
                Row(
                    modifier = Modifier.fillMaxWidth(),
                    horizontalArrangement = Arrangement.spacedBy(12.dp)
                ) {
                    // Fingerprint Quick-Scan Pad
                    Surface(
                        shape = RoundedCornerShape(16.dp),
                        color = Color(0xFF0F172A),
                        border = androidx.compose.foundation.BorderStroke(1.dp, Color(0xFF00E676).copy(alpha = 0.3f)),
                        modifier = Modifier
                            .weight(1f)
                            .clickable(
                                interactionSource = remember { MutableInteractionSource() },
                                indication = null,
                                enabled = !isLockout
                            ) {
                                isScanningFinger = true
                                onSimulateFingerprintScan(true)
                            }
                            .testTag("fingerprint_sensor_tap")
                    ) {
                        Column(
                            horizontalAlignment = Alignment.CenterHorizontally,
                            modifier = Modifier.padding(16.dp)
                        ) {
                            Icon(
                                imageVector = Icons.Default.Fingerprint,
                                contentDescription = "Sensor de Huella",
                                tint = Color(0xFF00E676),
                                modifier = Modifier.size(36.dp)
                            )
                            Spacer(modifier = Modifier.height(8.dp))
                            Text(
                                text = "Sensor Huella",
                                style = MaterialTheme.typography.labelMedium,
                                fontWeight = FontWeight.Bold,
                                color = Color.White
                            )
                            Text(
                                text = "Toque para validar",
                                style = MaterialTheme.typography.labelSmall,
                                fontSize = 10.sp,
                                color = Color(0xFF64748B)
                            )
                        }
                    }

                    // Face Recognition Scan Pad
                    Surface(
                        shape = RoundedCornerShape(16.dp),
                        color = Color(0xFF0F172A),
                        border = androidx.compose.foundation.BorderStroke(1.dp, Color(0xFF00B0FF).copy(alpha = 0.3f)),
                        modifier = Modifier
                            .weight(1f)
                            .clickable(
                                interactionSource = remember { MutableInteractionSource() },
                                indication = null,
                                enabled = !isLockout
                            ) {
                                isScanningFace = true
                                onSimulateFaceRecognitionScan(true)
                            }
                            .testTag("face_unlock_button")
                    ) {
                        Column(
                            horizontalAlignment = Alignment.CenterHorizontally,
                            modifier = Modifier.padding(16.dp)
                        ) {
                            Icon(
                                imageVector = Icons.Default.Face,
                                contentDescription = "Reconocimiento Facial",
                                tint = Color(0xFF00B0FF),
                                modifier = Modifier.size(36.dp)
                            )
                            Spacer(modifier = Modifier.height(8.dp))
                            Text(
                                text = "Face ID / Rostro",
                                style = MaterialTheme.typography.labelMedium,
                                fontWeight = FontWeight.Bold,
                                color = Color.White
                            )
                            Text(
                                text = "Escanear rostro",
                                style = MaterialTheme.typography.labelSmall,
                                fontSize = 10.sp,
                                color = Color(0xFF64748B)
                            )
                        }
                    }
                }
            }
        }

        Spacer(modifier = Modifier.height(16.dp))

        // 7. PIN / PASSPHRASE FALLBACK EXPANDER
        Card(
            shape = RoundedCornerShape(20.dp),
            colors = CardDefaults.cardColors(containerColor = Color(0xFF1E2638).copy(alpha = 0.7f)),
            modifier = Modifier
                .fillMaxWidth()
                .testTag("pin_fallback_card")
        ) {
            Column(modifier = Modifier.padding(16.dp)) {
                Row(
                    verticalAlignment = Alignment.CenterVertically,
                    horizontalArrangement = Arrangement.SpaceBetween,
                    modifier = Modifier
                        .fillMaxWidth()
                        .clickable { showPinFallback = !showPinFallback }
                ) {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Icon(
                            imageVector = Icons.Default.Pin,
                            contentDescription = "PIN de Respaldo",
                            tint = Color(0xFFFFD600),
                            modifier = Modifier.size(20.dp)
                        )
                        Spacer(modifier = Modifier.width(10.dp))
                        Text(
                            text = "PIN de Respaldo de Emergencia",
                            style = MaterialTheme.typography.titleSmall,
                            fontWeight = FontWeight.Bold,
                            color = Color.White
                        )
                    }
                    Icon(
                        imageVector = if (showPinFallback) Icons.Default.ExpandLess else Icons.Default.ExpandMore,
                        contentDescription = "Expandir PIN",
                        tint = Color(0xFF94A3B8)
                    )
                }

                AnimatedVisibility(visible = showPinFallback) {
                    Column(modifier = Modifier.padding(top = 16.dp)) {
                        Text(
                            text = "Ingresa el PIN maestro de la Bóveda (por defecto: 123456)",
                            style = MaterialTheme.typography.bodySmall,
                            color = Color(0xFF94A3B8)
                        )
                        Spacer(modifier = Modifier.height(12.dp))

                        Row(
                            verticalAlignment = Alignment.CenterVertically,
                            horizontalArrangement = Arrangement.spacedBy(10.dp)
                        ) {
                            OutlinedTextField(
                                value = pinInput,
                                onValueChange = {
                                    if (it.length <= 8) {
                                        pinInput = it
                                        pinError = false
                                    }
                                },
                                visualTransformation = PasswordVisualTransformation(),
                                placeholder = { Text("PIN maestro") },
                                singleLine = true,
                                isError = pinError,
                                modifier = Modifier
                                    .weight(1f)
                                    .testTag("pin_fallback_input"),
                                shape = RoundedCornerShape(12.dp)
                            )

                            Button(
                                onClick = {
                                    val success = onUnlockWithPin(pinInput)
                                    if (!success) {
                                        pinError = true
                                    }
                                },
                                shape = RoundedCornerShape(12.dp),
                                colors = ButtonDefaults.buttonColors(containerColor = Color(0xFFFFD600), contentColor = Color.Black),
                                modifier = Modifier.testTag("pin_unlock_button")
                            ) {
                                Text("Acceder", fontWeight = FontWeight.Bold)
                            }
                        }

                        if (pinError) {
                            Spacer(modifier = Modifier.height(6.dp))
                            Text(
                                text = "PIN incorrecto. Reintentos fallidos: $failedAttempts",
                                style = MaterialTheme.typography.labelSmall,
                                color = Color(0xFFFF5252)
                            )
                        }
                    }
                }
            }
        }

        Spacer(modifier = Modifier.height(24.dp))

        // 8. FOOTER ISOLATION NOTE
        Text(
            text = "🔒 Cifrado de memoria en reposo activo. La sesión y los descriptores de sockets VirtIO permanecen aislados hasta verificar la identidad biométrica.",
            style = MaterialTheme.typography.labelSmall,
            color = Color(0xFF64748B),
            textAlign = TextAlign.Center,
            modifier = Modifier.padding(horizontal = 12.dp)
        )
    }
}
