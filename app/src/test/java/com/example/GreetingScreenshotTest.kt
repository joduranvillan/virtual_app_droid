package com.example

import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onRoot
import com.example.ui.theme.MyApplicationTheme
import com.github.takahirom.roborazzi.RobolectricDeviceQualifiers
import com.github.takahirom.roborazzi.captureRoboImage
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.annotation.GraphicsMode

@RunWith(RobolectricTestRunner::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
@Config(qualifiers = RobolectricDeviceQualifiers.Pixel8, sdk = [36])
class DashboardScreenshotTest {

    @get:Rule val composeTestRule = createComposeRule()

    @Test
    fun dashboard_screenshot() {
        composeTestRule.setContent {
            MyApplicationTheme {
                VaultDashboardScreen(
                    connectionStatus = "Bóveda conectada (Handshake exitoso)",
                    isConnecting = false,
                    isConnected = true,
                    isPaired = true,
                    vaultHost = "127.0.0.1",
                    vaultPort = 7443,
                    deviceKeyFingerprint = "a1b2c3d4e5f67890",
                    remoteKeyFingerprint = "0987654321fedcba",
                    latestLocation = "Lat: -34.60, Lon: -58.38 (Acc: 5m)",
                    latestVideoFrame = null,
                    inputLogs = listOf("TouchDown [x=0.5, y=0.5]", "Key [code=66, press=true]"),
                    isSimulating = false,
                    adminLogs = listOf("[Linux-crosvm] Kernel initialized", "[Linux-crosvm] Hypervisor active"),
                    adminStatusMessage = "Sistema verificado",
                    adminActionInProgress = false,
                    onSendInputEvent = {},
                    onToggleSimulation = {},
                    onSendAdminAction = { _, _, _ -> },
                    onScanQrClick = {},
                    onConnectClick = {},
                    onResetPairingClick = {}
                )
            }
        }

        composeTestRule.waitForIdle()
        composeTestRule.onRoot().captureRoboImage(filePath = "src/test/screenshots/dashboard.png")
    }
}
