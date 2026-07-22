# Documentación de Pruebas de Integración End-to-End (E2E)

## Resumen Ejecutivo

Esta fase establece una suite integral de **Pruebas de Integración End-to-End (E2E)** ejecutadas localmente sobre JVM mediante Robolectric y un servidor de daemons simulados (`MockMultiplatformDaemonServer`). 

Permite validar de forma autónoma, determinista y sin necesidad de emuladores ni dispositivos físicos, la totalidad del stack de comunicación entre la aplicación Android Kotlin (`virtual_app_droid`) y los daemons hipervisores de backend (**Linux / crosvm**, **Windows / Hyper-V** y **macOS / Virtualization.framework**).

---

## Arquitectura del Entorno de Pruebas

```
+-----------------------------------------------------------------------------------+
|                        Entorno de Pruebas Robolectric (JVM)                        |
|                                                                                   |
|  +---------------------------------+        Socket Loopback TCP (127.0.0.1)      |
|  |     App Android Kotlin          | <=========================================>  |
|  |  - EnrollmentClient             |         Handshake Noise_XX (Cifrado)         |
|  |  - VaultConnectionManager       |                                              |
|  |  - VaultChannel (CBOR Wire)     |                                              |
|  +---------------------------------+                                              |
|                                                                                   |
|  +-----------------------------------------------------------------------------+  |
|  |                     MockMultiplatformDaemonServer                           |  |
|  |  - Responder Handshake Noise_XX (Noise_XX_25519_ChaChaPoly_SHA256)         |  |
|  |  - Simulador de Linux (crosvm), Windows (Hyper-V) y macOS (Virtualization)  |  |
|  |  - Manejo de ENROLLMENT_CONFIRM -> ENROLLMENT_ACK                           |  |
|  |  - Procesamiento de RPC ADMIN_REQUEST -> ADMIN_RESPONSE                     |  |
|  |  - Recepción y verificación de INPUT_EVENT (Touch / Key)                    |  |
|  |  - Despacho de SERVICE_REQUEST -> SERVICE_RESPONSE                          |  |
|  +-----------------------------------------------------------------------------+  |
+-----------------------------------------------------------------------------------+
```

---

## Flujos Probados y Cobertura E2E

### 1. Enrolamiento y Pairing Criptográfico por Código QR (`testEnrollmentFlow_*`)
* **Flujo Exitoso**:
  1. El Daemon genera un payload QR con versión `1`, clave pública del servidor en hexadecimal, token secreto y marca de expiración.
  2. `EnrollmentClient` establece socket TCP, realiza el handshake Noise_XX como `INITIATOR`, valida la coincidencia exacta de la clave pública remota enviada en el QR.
  3. Envía el frame `ENROLLMENT_CONFIRM` codificado en CBOR con el token.
  4. El daemon mock procesa el token y responde con `ENROLLMENT_ACK` exitoso.
* **Manejo de Errores y Seguridad**:
  * **QR Expirado**: Detecta marcas de tiempo pasadas y detiene el proceso antes de abrir sockets.
  * **Descalce de Clave Pública (PubkeyMismatch / Anti-MITM)**: Si un daemon malicioso o distinto responde durante la ventana de enrolamiento, se rechaza la conexión inmediatamente.
  * **Token Rechazado**: Retorna estado `Rejected` si el token presentado no coincide con la bóveda esperada.

### 2. Conexión Segura e Intercambio Cifrado (`testEndToEndConnectionAndHandshake_*`)
* Establecimiento del canal cifrado persistente mediante `VaultConnectionManager`.
* **Public Key Pinning**: Garantiza que conexiones subsiguientes solo se permitan si la clave estática remota coincide exactamente con la clave pinda durante el enrolamiento previo.
* Verificación de estados reactivos en la interfaz (`onStatusChange`, `onConnectionState`).

### 3. Comandos RPC de Administración Headless (`testRpcAdminCommands_*`)
* La app envía peticiones de administración `AdminRequestPayload` (`GET_LOGS`, `REBOOT_VAULT`, `CHANGE_NETWORK`, `UPDATE_RUNTIME`).
* El daemon mock recibe el frame `ADMIN_REQUEST` en CBOR, ejecuta la instrucción requerida en el hipervisor correspondiente y responde con `ADMIN_RESPONSE` que incluye logs del kernel y estado.
* La app procesa la respuesta y actualiza la consola administrativa.

### 4. Inyección de Eventos de Entrada en Tiempo Real (`testRpcInputEvents_*`)
* Transmisión de eventos `TouchDown`, `TouchMove`, `TouchUp` y `Key` mediante `InputEventPayload`.
* Mapeo CBOR de coordenadas normalizadas `(x, y)` e identificadores de puntero.
* Aserciones precisas en el daemon receptor para verificar coordenadas y códigos de tecla.

### 5. Simulación de Daemons Multiplataforma (`testMultiplatformDaemons_WindowsAndMacOS`)
* Verificación simultánea contra instancias independientes de daemons simulando:
  * **Linux** (`crosvm` / KVM)
  * **Windows** (`Hyper-V` / WSL2)
  * **macOS** (`Virtualization.framework`)

---

## Ejecución de Pruebas

Para ejecutar la suite completa de pruebas unitarias e integración en el entorno de desarrollo:

```bash
gradle :app:testDebugUnitTest
```

---

## Estado de la Implementación
* **Archivos Creados/Actualizados**:
  * `/app/src/test/java/com/example/MockMultiplatformDaemonServer.kt`
  * `/app/src/test/java/com/example/EndToEndDaemonIntegrationTest.kt`
  * `/app/src/test/java/com/example/GreetingScreenshotTest.kt` -> `DashboardScreenshotTest`
  * `/app/src/main/java/com/vault/net/VaultTypes.kt` (CBOR simétrico bidireccional)
