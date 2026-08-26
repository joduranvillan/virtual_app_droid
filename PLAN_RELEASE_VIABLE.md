# Plan de Release Viable — Virtual App Droid
## Análisis de Estado + 175 Prompts de Implementación

**Fecha:** 2026-08-01  
**Versión objetivo:** v1.0.0-MVP  
**Repositorio:** virtual_app_droid  

---

## ANÁLISIS DE ESTADO ACTUAL

### Estado General del Proyecto

Virtual App Droid es un ecosistema de ejecución confidencial Android que permite aislar y transmitir entornos virtuales de forma segura entre un cliente móvil Android y servidores host multiplataforma (Linux, Windows, macOS). El stack combina criptografía post-cuántica (ML-KEM-768 + Noise_XX), virtualización nativa (crosvm/ARCVM, Hyper-V, VZVirtualMachine) y un protocolo binario CBOR sobre canal cifrado.

### Componentes Implementados (Base Sólida)

| Componente | Estado | Notas |
|---|---|---|
| `vault-protocol` | COMPLETO | Esquemas CBOR puros, sin deuda técnica |
| `vault-crypto` | COMPLETO | Noise_XX + ML-KEM-768 (híbrido, ver gap #3) |
| `vault-core` | COMPLETO | Traits, enrolamiento, rate-limiting |
| `vault-stream` | ESQUELETO | Tipos definidos, sin pipeline real |
| `vault-linux` | STUB | Código existe, hipervisor no arranca VM real |
| `vault-windows` | STUB | Cmdlets mapeados, sin ejecución real |
| `vault-macos` | STUB | Swift helper no compilado |
| Android Client UI | COMPLETO | 50+ paneles Compose, telemetría simulada |
| Daemons sistema | COMPLETO | Systemd/Launchd/SCM listos |

### Gaps Críticos (Bloqueantes para Release)

1. **Hipervisores son stubs** — Los 3 adaptadores (crosvm, Hyper-V, VZVirtualMachine) simulan la VM. Sin instancia virtualizada real no hay producto.
2. **Codecs no funcionales** — H.265/H.264/Opus definidos como tipos pero sin pipeline de encode/decode. El framebuffer nunca llega al cliente.
3. **ML-KEM-768 no es nativo** — La crate `snow` no soporta Kyber. Se necesita `ml-kem` o bindings de liboqs para cumplir el claim PQC.
4. **Anti-Tamper es simulación** — La atestación HSM y la integridad APK son animaciones de UI. Sin enclave remoto real el sistema es vulnerable.
5. **Live Migration no existe** — El dirty-page tracking y VCPU sync son solo widgets de UI. No hay migración real entre nodos.
6. **Sin transporte de framebuffer real** — El cliente Android nunca recibe frames del servidor. El streaming completo está pendiente.
7. **Sin CI/CD** — No hay pipeline de integración continua. Sin tests automatizados en PR la calidad es no verificable.
8. **Sin manejo de errores robusto** — Faltan rutas de reconexión, backoff y recuperación ante fallos de red/hypervisor.

### Brechas Importantes (No Bloqueantes pero Necesarias)

- Tests de integración end-to-end reales
- Benchmarks de latencia y throughput  
- Firmado y distribución del APK
- Documentación de operador/instalación
- Monitoreo y observabilidad en producción
- Cliente de escritorio (CLI mínimo)
- Rotación automática de claves programada
- QUIC como transporte alternativo

---

## 175 PROMPTS DE IMPLEMENTACIÓN

Los prompts están organizados por fase de desarrollo y etiquetados con prioridad: 🔴 CRITICO · 🟠 IMPORTANTE · 🟡 DESEABLE

---

### FASE 1 — HIPERVISOR LINUX (crosvm / ARCVM)
*Implementar el backend de virtualización real para Linux usando crosvm*

**PROMPT 1** 🔴  
En `rust/crates/vault-linux/src/hypervisor.rs`, reemplaza el stub `CrosvmHypervisor::boot()` con una implementación real que: (1) verifique que el módulo KVM esté cargado con `std::fs::metadata("/dev/kvm")`, (2) construya el comando `crosvm run` con flags: `--disable-sandbox`, `--mem 2048`, `--cpus 2`, `--rwdisk /path/to/android.img`, `--serial type=stdout`, (3) lance el proceso con `tokio::process::Command` guardando el handle, (4) emita un evento `HypervisorEvent::Started { pid }` por el canal de eventos existente. Si `/dev/kvm` no existe devuelve `Err(HypervisorError::KvmUnavailable)`.

**PROMPT 2** 🔴  
Agrega el trait method `AndroidHypervisor::shutdown()` en `vault-core/src/traits.rs` con firma `async fn shutdown(&self) -> Result<(), HypervisorError>`. En `vault-linux/src/hypervisor.rs` implementa enviando SIGTERM al proceso crosvm con `child.kill()`, esperando máximo 5 segundos, y forzando SIGKILL si no termina. Emite `HypervisorEvent::Stopped { exit_code }`.

**PROMPT 3** 🔴  
Implementa `CrosvmHypervisor::wait_ready()` en `vault-linux` que: (1) intente conectar vía vsock al guest Android en el puerto 5555, (2) reintente con backoff exponencial comenzando en 500ms hasta 30 segundos máximo, (3) retorne `Ok(())` al primer éxito o `Err(HypervisorError::Timeout)`. Usa `tokio::net::UnixStream` o socket raw vsock según disponibilidad del kernel.

**PROMPT 4** 🔴  
Crea `rust/crates/vault-linux/src/android_image.rs` con una struct `AndroidImage` que gestione la imagen de disco QCOW2/raw del guest. Implementa: `AndroidImage::validate(path)` verificando magic bytes del formato, `AndroidImage::resize(size_gb)` usando `qemu-img resize`, y `AndroidImage::snapshot_create(name)` / `snapshot_restore(name)` usando las funciones de snapshot de qemu-img. Propaga errores con `thiserror`.

**PROMPT 5** 🔴  
Implementa el mapeo de `VirtioInput` en `vault-linux/src/hypervisor.rs`: dado un `InputEventPayload` del protocolo CBOR, convierte las coordenadas normalizadas `[0.0, 1.0]` a coordenadas de pantalla absolutas del guest, y escribe el evento en el socket de input de crosvm (`/run/crosvm-input.sock`) usando el formato de evento Linux `input_event` (type, code, value). Implementa para touch (EV_ABS) y teclado (EV_KEY).

**PROMPT 6** 🔴  
En `vault-linux/src/hypervisor.rs`, implementa `CrosvmHypervisor::capture_framebuffer()` que: (1) conecte al socket de framebuffer de crosvm (`--gpu type=virglrenderer`), (2) capture un frame completo como buffer RGBA, (3) lo comprima con el encoder H.265 (ver Fase 3), (4) lo empaquete como `VideoFramePayload` CBOR y lo envíe por el canal de streaming. Debe mantener el target FPS configurado usando `tokio::time::interval`.

**PROMPT 7** 🟠  
Implementa `CrosvmHypervisor::get_vm_stats()` que lea las métricas de la VM via `/sys/fs/cgroup/` (para CPU time) y el socket de control de crosvm (para memoria): retorna `VmStats { vcpu_usage_percent: f32, ram_used_mb: u32, io_read_mbps: f32, io_write_mbps: f32 }`. Expón este método en el trait `AndroidHypervisor` en `vault-core/src/traits.rs`.

**PROMPT 8** 🟠  
En `vault-linux/src/lifecycle.rs`, implementa el watcher de estado de la VM: usa `tokio::process::Child::wait()` en un task separado para detectar muerte inesperada del proceso crosvm. Al detectar exit code != 0, emite `HypervisorEvent::Crashed { exit_code, stderr_tail }` y activa el proceso de auto-restart con límite de 3 intentos y backoff de 5 segundos. Registra el evento en el log con `tracing::error!`.

**PROMPT 9** 🟠  
Crea `rust/crates/vault-linux/src/vm_config.rs` con struct `VmConfig` serializable con serde: `{ memory_mb: u32, vcpu_count: u8, disk_path: PathBuf, enable_gpu: bool, display_resolution: (u32, u32), enable_audio: bool, network_mode: NetworkMode }` donde `NetworkMode` es enum `{ Isolated, Nat, Bridged(String) }`. Implementa `VmConfig::from_file(path)` y `VmConfig::save(path)` con formato TOML.

---

### FASE 2 — HIPERVISOR WINDOWS (Hyper-V / WHP)

**PROMPT 10** 🔴  
En `rust/crates/vault-windows/src/hypervisor.rs`, reemplaza el stub con una implementación real usando Windows Hypervisor Platform (WHP) via `windows-sys`. Llama a `WHvCreatePartition`, `WHvSetPartitionProperty` (para vCPU count y RAM), `WHvSetupPartition`. Si WHv no está disponible, retorna `Err(HypervisorError::WhpUnavailable)` con mensaje indicando cómo habilitarlo en Windows Features.

**PROMPT 11** 🔴  
Implementa `HyperVHypervisor::boot_android_vm()` en `vault-windows/src/hypervisor.rs` usando PowerShell via `std::process::Command`: ejecuta `New-VM -Name VaultAndroid -MemoryStartupBytes 2GB`, `Add-VMHardDiskDrive -VMName VaultAndroid -Path android.vhdx`, `Start-VM -Name VaultAndroid`. Parsea la salida para confirmar estado "Running" antes de retornar `Ok(())`. Implementa limpieza en `Drop`.

**PROMPT 12** 🔴  
En `vault-windows/src/storage.rs`, implementa `VhdxStorage::create(path, size_gb)` que: (1) llame `New-VHD -Path $path -SizeBytes ${size_gb}GB -Dynamic` via PowerShell, (2) formatee con NTFS, (3) active BitLocker con `Enable-BitLocker -MountPoint $drive -EncryptionMethod XtsAes256 -RecoveryPasswordProtector`, (4) guarde el recovery key en `DpapiSecretStore`. Implementa `VhdxStorage::unlock(path, key)` para el flujo inverso.

**PROMPT 13** 🟠  
Agrega soporte de Enhanced Session Mode en `vault-windows/src/hypervisor.rs`: conecta al VM a través del protocolo RDP Enhanced Session (`vmconnect.exe`) para obtener acceso al framebuffer sin necesidad de VirtIO GPU. Implementa `capture_frame_rdp()` que use la API RDP Client `MSTSCLib` via COM interop para capturar el desktop del guest y retornarlo como buffer de imagen.

**PROMPT 14** 🟠  
En `vault-windows/src/service_manager.rs`, implementa la recuperación automática del servicio: tras crash del worker, el SCM debe reintentar 3 veces con delays de 1min/5min/15min. Configura esto en `setup.ps1` usando `sc.exe failure VaultRuntime reset= 86400 actions= restart/60000/restart/300000/restart/900000`. Agrega detección de estado en `ServiceManager::health_check()`.

---

### FASE 3 — HIPERVISOR MACOS (Virtualization.framework)

**PROMPT 15** 🔴  
Crea `rust/crates/vault-macos/src/vm_helper.swift` con una clase `VaultVMHelper` que use `Virtualization.framework`: (1) crea `VZVirtualMachineConfiguration` con `VZLinuxBootLoader`, `VZVirtioBlockDeviceConfiguration` para el disco, `VZVirtioNetworkDeviceConfiguration`, (2) instancia `VZVirtualMachine` y llama `start(completionHandler:)`, (3) expone una función C-compatible `vault_vm_start()` que pueda llamarse desde Rust via FFI.

**PROMPT 16** 🔴  
Crea `rust/crates/vault-macos/build.rs` que compile automáticamente `vm_helper.swift` usando `swiftc -emit-library -target arm64-apple-macos13.0` y el flag `-framework Virtualization`. Añade el linking de la library al build de Rust con `cargo:rustc-link-lib=static=vault_vm_helper`. Declara los symbols FFI en `vault-macos/src/ffi.rs`.

**PROMPT 17** 🔴  
En `vault-macos/src/hypervisor.rs`, implementa `MacOsHypervisor` que llame a la biblioteca Swift via FFI: `extern "C" { fn vault_vm_start(config: *const VmConfigC) -> i32; fn vault_vm_stop() -> i32; fn vault_vm_get_state() -> VmStateC; }`. Convierte entre los tipos Rust y los C-structs, propagando errores del código de retorno.

**PROMPT 18** 🟠  
Implementa el audio passthrough en macOS usando Core Audio. En `vault-macos/src/audio.rs`, crea `CoreAudioCapture` que: abra un `AudioUnit` de tipo `kAudioUnitType_Output`, capture el audio del guest VM via `VZVirtioSoundDeviceConfiguration`, encodifique con Opus (ver Fase 4), y lo empaquete en `AudioFramePayload` para enviar al cliente.

---

### FASE 4 — CODECS DE VIDEO Y AUDIO

**PROMPT 19** 🔴  
Agrega `ffmpeg-sys-next = "7"` a `Cargo.toml` del workspace. En `rust/crates/vault-stream/src/video.rs`, implementa `H265Encoder` que: (1) inicialice el codec `libx265` via libavcodec, (2) configure `AVCodecContext` con los parámetros de `VideoConfig` (width, height, fps, bitrate), (3) exponga `H265Encoder::encode_frame(rgba_buffer: &[u8]) -> Result<Vec<u8>>` que retorne el NAL unit comprimido. Habilita hardware acceleration si detecta VAAPI (Linux) o VideoToolbox (macOS).

**PROMPT 20** 🔴  
En `rust/crates/vault-stream/src/video.rs`, implementa `H265Decoder` para el lado Android (via JNI). Crea `VaultDecoder.kt` en `app/src/main/java/com/example/` que: (1) use `MediaCodec` con mime type `video/hevc`, (2) configure con `MediaFormat` desde el SPS/PPS del primer keyframe, (3) retorne `SurfaceTexture` que pueda renderizarse en el `Canvas` del composable de pantalla remota.

**PROMPT 21** 🔴  
En `rust/crates/vault-stream/src/audio.rs`, implementa `OpusEncoder`: usa `audiopus` crate (0.3+), configura `Bitrate::BitsPerSecond(64000)`, `SampleRate::Hz48000`, `Channels::Stereo`. Expón `OpusEncoder::encode(pcm_samples: &[i16]) -> Result<Vec<u8>>`. En el cliente Android, implementa `OpusPlayerKt` usando `AudioTrack` con `AudioFormat.ENCODING_PCM_16BIT` y `AudioManager.STREAM_MUSIC`.

**PROMPT 22** 🔴  
Crea el pipeline de streaming completo en `vault-linux/src/streaming_pipeline.rs`: un task de Tokio que cada `1/fps` segundos: (1) captura framebuffer del guest, (2) encodifica con `H265Encoder`, (3) empaqueta en `VideoFramePayload` CBOR, (4) cifra con `WireChannel::send()`, (5) envía por TCP. Implementa control de flujo: si el buffer de envío supera 10 frames, descarta frames B y reduce calidad temporalmente.

**PROMPT 23** 🔴  
En `app/src/main/java/com/example/MainActivity.kt`, reemplaza la simulación del framebuffer con la recepción real: (1) abre una corrutina que lee del `VaultConnectionManager` los `VideoFramePayload` desencriptados, (2) los pasa al `VaultDecoder` (H.265), (3) renderiza cada frame en un `AndroidView` que envuelve un `SurfaceView` con `Surface`. Mantén el FPS counter real basado en timestamps de frame recibidos.

**PROMPT 24** 🟠  
Implementa adaptación de bitrate automática (ABR) en `vault-stream/src/video.rs`: mide el RTT del canal cada 2 segundos via pings CBOR, si RTT > 150ms reduce bitrate 20%, si RTT < 50ms y bitrate < max sube 10%. Usa el `AdminAction::AdjustBitrate` del protocolo existente para notificar el cambio al encoder en el servidor.

**PROMPT 25** 🟠  
Agrega soporte para hardware encoding en `vault-linux/src/streaming_pipeline.rs`: detecta VAAPI con `vaQueryEntrypoints()`, NVENC con CUDA toolkit, y AMF con AMD headers. Usa el encoder disponible con fallback a libx265 software. Reporta el encoder activo en `VmStats::encoder_backend: EncoderBackend` enum `{ Software, Vaapi, Nvenc, Amf }`.

**PROMPT 26** 🟠  
Implementa captura de audio del guest Android en Linux via PulseAudio/PipeWire: en `vault-linux/src/audio_capture.rs`, crea `PipeWireAudioCapture` que: (1) conecte al server PipeWire usando la crate `pipewire`, (2) capture el sink del guest VM, (3) capture frames PCM 48kHz/stereo, (4) los pase al `OpusEncoder` y envíe el payload de audio por el canal paralelo de streaming.

---

### FASE 5 — CRIPTOGRAFÍA POST-CUÁNTICA REAL

**PROMPT 27** 🔴  
Reemplaza el mock ML-KEM-768 en `rust/crates/vault-crypto/src/handshake.rs`: agrega la dependencia `ml-kem = "0.3"` (crate FIPS 203 puro-Rust). Implementa `PqcKeyPair::generate()` usando `ml_kem::MlKem768::generate`, `PqcKeyPair::encapsulate(pk) -> (ciphertext, shared_secret)` y `PqcKeyPair::decapsulate(sk, ciphertext) -> shared_secret`. Combina el shared_secret con el X25519 via HKDF-SHA256 para el key material final del handshake Noise.

**PROMPT 28** 🔴  
Actualiza el protocolo de handshake en `vault-crypto/src/handshake.rs` para transmitir el ML-KEM ciphertext dentro del payload del primer mensaje Noise: extiende `EnrollmentRequest` en `vault-protocol/src/enrollment.rs` con campo `pqc_ciphertext: Option<Bytes>`. En el server, si el campo está presente, realiza decapsulación y mezcla el PQC shared_secret antes del KDF final. Mantén compatibilidad con clientes que no soporten PQC (campo None).

**PROMPT 29** 🔴  
Implementa verificación real de atestación TPM 2.0 en `vault-linux/src/attestation.rs`: (1) usa la crate `tpm2-tss-sys` para conectar al TPM via `/dev/tpm0`, (2) solicita un PCR quote firmado con la Identity Key del TPM, (3) verifica la firma contra el certificado EK del fabricante, (4) incluye el quote en el `EnrollmentRequest` como campo `tpm_attestation: Option<TpmAttestation>`. Si no hay TPM, omite el campo.

**PROMPT 30** 🔴  
En el cliente Android, implementa la verificación de integridad APK real: en `MainActivity.kt`, al conectar, calcula el SHA-256 del APK propio con `context.packageManager.getPackageInfo()` + `MessageDigest`, envía el hash al server en el `EnrollmentRequest` como `apk_fingerprint`. El server compara contra hashes firmados almacenados en `SecretStore` y rechaza conexiones con APK no reconocido con código de error `ErrorCode::UnknownApk`.

**PROMPT 31** 🟠  
Implementa rotación automática de claves por sesión en `vault-crypto/src/wire.rs`: lleva un contador de bytes transmitidos en `WireChannel`. Cuando supere `rekey_threshold_bytes` (default 1 GB, configurable), genera nuevo material de clave con HKDF usando el secreto actual como IKM, re-inicializa los estados ChaCha20-Poly1305, y emite `ControlMessage::RekeyComplete` al peer. Sincroniza ambos lados antes de continuar.

**PROMPT 32** 🟠  
Implementa Certificate Pinning en el cliente Android: en `VaultConnectionManager.kt`, al primer pairing exitoso, guarda el hash SHA-256 de la clave pública del servidor en Android Keystore (no en SharedPreferences). En conexiones subsecuentes, verifica que la clave presentada coincida con el pin almacenado. Si no coincide, bloquea la conexión y emite `SecurityEvent::PinMismatch` con alerta visible en la UI.

**PROMPT 33** 🟠  
Agrega rate-limiting al servidor contra ataques de enrolamiento: en `vault-core/src/rate_limit.rs`, extiende `RateLimiter` con política por IP: máximo 5 intentos de pairing en 10 minutos, bloqueo de 1 hora tras exceder. Persiste el estado de bloqueo en `SecretStore` para sobrevivir reinicios. Devuelve `ErrorCode::TooManyRequests` con header `Retry-After` en segundos.

**PROMPT 34** 🟠  
Implementa sealed mode en `vault-core/src/traits.rs`: agrega `AndroidHypervisor::seal()` que: (1) suspende la VM, (2) encripta el disco con una clave derivada del estado actual del hardware (TPM PCR measurements), (3) el sistema sólo puede desbloquear en el mismo hardware. Agrega `unseal(tpm_context)` para el camino inverso. Documenta en qué escenarios de seguridad aplica.

---

### FASE 6 — CLIENTE ANDROID (Funcionalidad Real)

**PROMPT 35** 🔴  
Crea `app/src/main/java/com/example/network/VaultWebSocket.kt` con una clase que gestione la conexión TCP real al servidor: usa `OkHttp` WebSocket o `java.net.Socket` raw con corrutinas. Implementa (1) `connect(host, port)` con timeout de 10s, (2) `sendFrame(payload: ByteArray)` para enviar datos CBOR, (3) `receiveFrame(): Flow<ByteArray>` para recibir frames como Flow, (4) reconexión automática con backoff exponencial (1s, 2s, 4s, 8s, máximo 60s).

**PROMPT 36** 🔴  
Integra el handshake Noise_XX real en el cliente Android: crea `app/src/main/java/com/example/crypto/NoiseHandshake.kt` que use la JNI binding de la librería `vault-crypto` compilada para Android (NDK). Implementa `NoiseHandshake.performHandshake(socket, pairingCode): HandshakeResult` que ejecute los 3 mensajes Noise_XX, verifique el fingerprint de la clave del servidor, y retorne el `WireChannel` establecido.

**PROMPT 37** 🔴  
Configura el build para Android NDK en `app/build.gradle.kts`: agrega el task de Cargo que compile el workspace Rust con targets `aarch64-linux-android`, `x86_64-linux-android`, `armv7-linux-androideabi`. Usa `cargo-ndk` o el NDK toolchain directamente. Empaqueta los `.so` resultantes en `app/src/main/jniLibs/`. Declara el módulo JNI en `CMakeLists.txt` si es necesario.

**PROMPT 38** 🔴  
Crea `app/src/main/java/com/example/VaultJNI.kt` con los bindings JNI para las funciones criptográficas de Rust: `external fun noiseHandshakeInit(): Long`, `external fun noiseHandshakeMessage1(handle: Long, payload: ByteArray): ByteArray`, `external fun noiseHandshakeMessage3(handle: Long, msg2: ByteArray): HandshakeResultJni`, `external fun wireEncrypt(handle: Long, plaintext: ByteArray): ByteArray`, `external fun wireDecrypt(handle: Long, ciphertext: ByteArray): ByteArray`. Implementa el lado Rust en `vault-crypto/src/jni_bindings.rs`.

**PROMPT 39** 🔴  
Reemplaza la simulación de telemetría en `MainActivity.kt` con datos reales del servidor: crea `VaultTelemetryRepository.kt` que suscriba al `Flow` de mensajes `ServiceResponseEnvelope` con type `TelemetryResponse`. Deserializa los campos CBOR y actualiza los `StateFlow` de `vmCpuUsagePercent`, `vmRamUsageMb`, `hypervisorTemperatureC`. Mantén la simulación como fallback cuando la conexión no está activa, con indicador visual `[SIMULATED]`.

**PROMPT 40** 🔴  
Implementa la inyección real de eventos táctiles en `MainActivity.kt`: en el `Canvas` del framebuffer remoto, captura `PointerInputChange` con `detectTransformGestures` y `detectTapGestures`. Para cada evento, crea un `InputEventPayload` con coordenadas normalizadas, action type (`DOWN/MOVE/UP`), pointer_id y pressure. Envíalo al servidor via `VaultWebSocket` inmediatamente (sin buffering). Mide la latencia touch-to-frame e imprímela en el HUD.

**PROMPT 41** 🟠  
Implementa el escáner QR real en `MainActivity.kt`: usa `androidx.camera.mlkit.vision` con `BarcodeScanner` de ML Kit. Al detectar un QR con esquema `vault://pair?code=XXX&host=YYY&port=ZZZ&pk=AAAA`, extrae los campos, valida el formato, y los precarga en los campos del formulario de conexión. Muestra una animación de "QR detectado" y pide confirmación antes de iniciar el pairing.

**PROMPT 42** 🟠  
Agrega persistencia de sesión en el cliente Android: crea `VaultSessionRepository.kt` usando `DataStore<Preferences>` para guardar: host, port, último fingerprint de clave del servidor, nombre del dispositivo. Al re-abrir la app, carga la última sesión y muestra opción "Reconectar a [host]" con el fingerprint para verificación del usuario. Usa Android Keystore para cifrar el fingerprint guardado.

**PROMPT 43** 🟠  
Implementa notificaciones push en el cliente Android para eventos del servidor: crea `VaultNotificationService.kt` extendiendo `Service` que corra en foreground con `startForeground()`. Muestra notificación persistente "Vault activo — [host]" con acciones rápidas "Desconectar" y "Re-keyear". Para eventos de seguridad (`PinMismatch`, `TamperDetected`), muestra notificación de alta prioridad con canal `VAULT_SECURITY`.

**PROMPT 44** 🟠  
Crea `app/src/main/java/com/example/ui/RemoteDisplayScreen.kt` como composable dedicado para la pantalla remota: separado de `MainActivity.kt`, maneja solo el framebuffer + input injection. Debe incluir: (1) `SurfaceView` de pantalla completa con gestos multi-touch, (2) barra de estado flotante con FPS/latencia/calidad, (3) gestos de sistema: swipe-up para menú flotante de controles, doble-tap para toggle pantalla completa. Navega a esta pantalla al conectar exitosamente.

**PROMPT 45** 🟠  
Agrega Deep Link support en `AndroidManifest.xml`: declara intent-filter para scheme `vault://` con action `CONNECT`. En `MainActivity.kt`, si la app se abre con un deep link `vault://pair?...`, inicia automáticamente el flujo de pairing. Si ya hay una sesión activa, muestra dialog "Ya conectado a [host]. ¿Desconectar y re-conectar?". Esto permite iniciar conexión desde email/QR externo.

---

### FASE 7 — PROTOCOLO DE RED Y TRANSPORTE

**PROMPT 46** 🔴  
Implementa el servidor TCP real en `vault-linux/src/bin/vault-runtime.rs`: reemplaza el placeholder con un `TcpListener::bind("127.0.0.1:7444")` (socket interno). Para cada conexión acepta en una tarea Tokio separada: (1) ejecuta handshake Noise_XX, (2) establece `WireChannel`, (3) inicia el loop de lectura/escritura de frames CBOR. El `vault-host` relay debe redirigir de `0.0.0.0:7443` a `127.0.0.1:7444` sin descifrar.

**PROMPT 47** 🔴  
Implementa el framing de mensajes sobre TCP en `vault-protocol/src/framing.rs`: usa length-prefix de 4 bytes big-endian antes de cada frame CBOR. Implementa `FrameReader::read_frame(reader: &mut impl AsyncRead) -> Result<Bytes>` y `FrameWriter::write_frame(writer: &mut impl AsyncWrite, data: &[u8]) -> Result<()>`. Maneja el caso de frames parciales correctamente con buffer interno.

**PROMPT 48** 🔴  
En `vault-linux/src/bin/vault-host.rs`, implementa el relay ciego TCP real: usa `tokio::io::copy_bidirectional` para hacer proxy entre la conexión del cliente Android (`0.0.0.0:7443`) y el socket local de `vault-runtime` (`127.0.0.1:7444`). Registra métricas de bytes transferidos con `tracing::debug!` sin inspeccionar el contenido. Aplica TCP keepalive de 30s para detectar desconexiones silenciosas.

**PROMPT 49** 🟠  
Agrega soporte QUIC como transporte alternativo en `vault-linux`: usa la crate `quinn` (0.11+). Crea `vault-linux/src/quic_transport.rs` con `QuicServer::bind(addr, cert, key)` y `QuicClient::connect(addr, server_name)`. Negocia el transporte en el handshake de pairing: si el cliente soporta QUIC (campo en `EnrollmentRequest`), migra la sesión a QUIC post-handshake. QUIC mejora el streaming en redes con pérdida de paquetes.

**PROMPT 50** 🟠  
Implementa multiplexación de canales sobre la misma conexión TCP: define `ChannelType` enum en `vault-protocol/src/framing.rs`: `{ Control, VideoStream, AudioStream, InputEvents, Telemetry }`. Cada frame incluye un byte de channel_id. En el servidor, un `Demultiplexer` distribuye frames al handler correcto. Esto evita head-of-line blocking del video por mensajes de control lentos.

**PROMPT 51** 🟠  
Implementa reconexión transparente en `vault-core/src/session.rs`: guarda el estado de sesión (session_id, último seq_number, PQC keys) en `SecretStore`. Si la conexión TCP se cae, el cliente puede reconectar en ≤5 segundos sin re-hacer el handshake completo: valida el session_id y retoma desde el último seq_number conocido. Define el tiempo de validez de sesión suspendida (default 5 minutos).

**PROMPT 52** 🟡  
Implementa compresión adaptativa de frames en `vault-stream/src/video.rs`: para redes lentas (<5 Mbps), reduce resolución a 720p y FPS a 15 automáticamente. Para redes rápidas (>20 Mbps), permite 1440p a 60fps. La decisión se basa en el throughput medido de los últimos 10 segundos. Expón el estado en `ConnectionStats::adaptive_quality: QualityLevel`.

---

### FASE 8 — LIVE MIGRATION REAL

**PROMPT 53** 🟠  
Implementa live migration con KVM en `vault-linux/src/migration.rs`: usa la interfaz `KVM_SET_USER_MEMORY_REGION` para rastrear dirty pages. Crea `LiveMigrationController::start_precopy(source_vm, target_addr)` que: (1) obtiene el bitmap de dirty pages cada 100ms, (2) copia las páginas modificadas al nodo destino via TCP cifrado, (3) cuando el ratio dirty/total < 5%, ejecuta `KVM_GET_VCPU_EVENTS` para capturar estado de CPU y finaliza la migración con stop-and-copy.

**PROMPT 54** 🟠  
Implementa el servidor de recepción de migración en el nodo destino: `MigrationReceiver::listen(port)` que: (1) acepta la conexión del nodo origen, (2) recibe páginas de memoria en orden, (3) reconstruye el mapa de memoria del guest, (4) recibe el estado final de VCPU, (5) arranca la VM en el estado recibido con `KVM_CREATE_VCPU` + `KVM_SET_REGS`. La migración debe completarse en < 2 segundos de downtime según la arquitectura.

**PROMPT 55** 🟠  
Actualiza `vault-core/src/services.rs` para que el `AdminAction::LiveMigrate { target_node }` ejecute la migración real: (1) verifica conectividad con el nodo destino, (2) negocia handshake Noise_XX con el destino, (3) inicia `LiveMigrationController`, (4) al completar, actualiza en `SecretStore` el host activo, (5) notifica al cliente Android el nuevo endpoint con `ServiceResponse::MigrationComplete { new_host, new_port }`.

---

### FASE 9 — TESTING Y CALIDAD

**PROMPT 56** 🔴  
Crea `rust/tests/integration_handshake.rs` como test de integración end-to-end del handshake: levanta un `vault-runtime` en un thread de test en puerto 19443, conecta un cliente mock que ejecute el handshake Noise_XX completo con ML-KEM-768, verifica que el `WireChannel` resultante puede encriptar y desencriptar correctamente 1000 mensajes de prueba. El test debe pasar en < 5 segundos.

**PROMPT 57** 🔴  
Crea `rust/tests/integration_enrollment.rs`: test que simula el flujo completo de pairing: (1) server genera QR con `vault://pair` URI, (2) cliente parsea el URI y envía `EnrollmentRequest`, (3) server responde con `EnrollmentConfirm`, (4) cliente envía `EnrollmentAck`, (5) verifica que la clave del cliente queda persistida en el `MemorySecretStore` de test. Valida también el rechazo de un segundo cliente con código de pairing diferente.

**PROMPT 58** 🔴  
Crea tests unitarios para `vault-crypto/src/handshake.rs`: test `test_noise_xx_round_trip` que crea dos instancias `NoiseHandshake` (initiator y responder), intercambia los 3 mensajes, y verifica que ambos terminan con el mismo `HandshakeHash`. Test `test_pqc_kdf_output_length` que verifica que el material de clave final tiene exactamente 64 bytes. Test `test_rekey_changes_cipher_state` que verifica que el re-keying produce cifrados diferentes.

**PROMPT 59** 🔴  
Crea `app/src/test/java/com/example/VaultConnectionTest.kt` con tests Robolectric: mock el `VaultWebSocket` con una implementación en memoria, simula el handshake exitoso, y verifica que `MainActivity` transiciona al estado `CONNECTED`. Test de reconexión: simula desconexión, verifica que el estado cambia a `RECONNECTING`, espera backoff, simula reconexión exitosa y verifica vuelta a `CONNECTED`.

**PROMPT 60** 🔴  
Crea `app/src/androidTest/java/com/example/PairingFlowTest.kt` como test instrumentado E2E: usa `ComposeTestRule` para (1) verificar que la pantalla de inicio muestra botón "Escanear QR", (2) simula input del código de pairing manual, (3) verifica que aparece la pantalla de confirmación con fingerprint, (4) acepta la conexión, (5) verifica que aparece la pantalla de framebuffer. Usa `MockWebServer` para simular el servidor.

**PROMPT 61** 🟠  
Crea benchmarks de latencia en `rust/benches/streaming_bench.rs` usando `criterion`: mide el tiempo de `H265Encoder::encode_frame()` para frames de 1280x720 y 1920x1080. Mide el tiempo de `WireChannel::send()` incluyendo ChaCha20-Poly1305. El objetivo: encode 720p < 16ms (60fps budget), cifrado < 1ms. Si los benchmarks fallan el threshold, el CI debe marcar el build como degraded.

**PROMPT 62** 🟠  
Crea `rust/tests/integration_rate_limit.rs`: test que envía 100 requests de enrollment consecutivos desde la misma IP simulada y verifica: (1) los primeros 5 tienen éxito, (2) del 6to en adelante recibe `ErrorCode::TooManyRequests`, (3) tras 600 segundos simulados (con `tokio::time::pause()`), los intentos vuelven a aceptarse. Verifica que el estado de bloqueo se persiste entre reinicios del servidor.

**PROMPT 63** 🟠  
Crea tests de property-based testing para el protocolo de framing usando `proptest`: genera secuencias aleatorias de frames de tamaño variable (0 a 65535 bytes), verifica que `FrameWriter` + `FrameReader` round-trip produce exactamente los datos originales. Test específico para frames fragmentados: divide el byte stream en chunks de tamaño aleatorio y verifica que el reader ensambla los frames correctamente.

**PROMPT 64** 🟠  
Agrega screenshot tests en Android con Roborazzi: crea `app/src/test/java/com/example/ScreenshotTests.kt` que capture los estados principales: pantalla de bienvenida, pairing QR activo, conectado con framebuffer, panel de seguridad, panel de admin, y error de conexión. Configura Roborazzi para comparar contra golden images en `app/src/test/snapshots/`. El CI falla si hay diferencias visuales inesperadas.

**PROMPT 65** 🟡  
Implementa fuzzing del parser CBOR en `rust/fuzz/fuzz_targets/cbor_parser.rs`: usa `cargo-fuzz` con LibFuzzer. El fuzz target llama a `vault_protocol::framing::parse_frame(data)` con bytes arbitrarios y verifica que nunca panic (solo `Err`). Agrega el corpus inicial con frames válidos e inválidos conocidos. Incluye instrucciones en `README.md` para correr el fuzzer localmente.

---

### FASE 10 — CI/CD Y DEVOPS

**PROMPT 66** 🔴  
Crea `.github/workflows/rust-ci.yml`: en cada PR y push a `main`, ejecuta: (1) `cargo fmt --check` en todo el workspace, (2) `cargo clippy -- -D warnings` con todos los features, (3) `cargo test --workspace` con output XML para GitHub test reporter, (4) `cargo build --release --workspace` para verificar compilación sin errores. Cache de `~/.cargo/registry` y `target/` entre runs. Fail fast en el primer error.

**PROMPT 67** 🔴  
Crea `.github/workflows/android-ci.yml`: en cada PR, ejecuta: (1) `./gradlew lint` y falla si hay errores, (2) `./gradlew test` con JUnit reporter, (3) `./gradlew connectedAndroidTest` en emulador `system-images;android-34;google_apis;x86_64`, (4) `./gradlew assembleRelease` para verificar el build de producción. Cachea el Gradle daemon entre runs. Publica el APK debug como artefacto del workflow.

**PROMPT 68** 🔴  
Crea `.github/workflows/security-scan.yml`: ejecuta semanalmente y en cada PR a `main`: (1) `cargo audit` contra la advisory database de RustSec, (2) `cargo deny check` con política definida en `deny.toml`, (3) `mobsfscan` sobre el código Android, (4) `semgrep --config=auto` sobre Kotlin y Rust. Genera reporte SARIF y lo sube a GitHub Security tab. El build falla si hay vulnerabilidades CRITICAL o HIGH.

**PROMPT 69** 🔴  
Crea `deny.toml` para cargo-deny: configura `[licenses]` para permitir solo MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, y MPL-2.0. En `[advisories]`, rechaza advisories de severidad >= medium. En `[bans]`, deniega duplicados de crates criptográficos críticos (`ring`, `openssl`). Agrega excepciones documentadas para cualquier dependencia que no cumpla la política de licencias.

**PROMPT 70** 🟠  
Crea `.github/workflows/release.yml` que se activa con tags `v*.*.*`: (1) ejecuta el full CI suite, (2) cross-compila el binario `vault-host` para Linux x86_64/aarch64, Windows x86_64, macOS x86_64/arm64 usando `cross` tool, (3) construye el APK de release firmado con keystore en GitHub Secrets, (4) crea GitHub Release con los binarios adjuntos y changelog generado desde commits convencionales. Genera checksums SHA-256 para cada artefacto.

**PROMPT 71** 🟠  
Configura Docker multi-stage build en `Dockerfile` para el servidor Linux: `FROM rust:1.80 AS builder` compila el workspace, `FROM ubuntu:24.04 AS runtime` copia solo los binarios, instala dependencias mínimas (libssl, crosvm), define entrypoints `ENTRYPOINT ["vault-runtime"]`. Agrega `docker-compose.yml` para desarrollo local con volúmenes para la imagen Android y configuración. Publica imagen en GHCR en el workflow de release.

**PROMPT 72** 🟠  
Crea `scripts/dev-setup.sh` que prepare el entorno de desarrollo en una máquina limpia Linux/macOS: (1) verifica dependencias (Rust, Android SDK, NDK, crosvm), (2) descarga imagen Android-x86 de prueba, (3) compila el workspace Rust, (4) compila el APK debug, (5) configura el daemon de desarrollo con configuración mínima. El script debe ser idempotente. Incluye verificación de compatibilidad de versiones.

**PROMPT 73** 🟡  
Configura análisis de cobertura de código: usa `cargo-llvm-cov` para Rust y `kover` para Kotlin/Android. En el CI, tras los tests, genera reportes de cobertura en formato LCOV y los sube a Codecov. Define thresholds mínimos: 70% línea coverage para `vault-crypto` y `vault-protocol`, 60% para los demás crates. El build falla si la cobertura cae por debajo del threshold en los módulos críticos.

---

### FASE 11 — MANEJO DE ERRORES Y RESILIENCIA

**PROMPT 74** 🔴  
Audita y completa el manejo de errores en `vault-linux/src/bin/vault-runtime.rs`: reemplaza todos los `unwrap()` y `expect()` con propagación de errores apropiada usando `?`. Para errores irrecuperables (fallo de init del hipervisor), log con `tracing::error!` + exit code 1. Para errores recuperables (conexión caída), log con `tracing::warn!` y reinicia el handler sin matar el proceso principal.

**PROMPT 75** 🔴  
Implementa health check endpoint en `vault-runtime`: agrega un servidor HTTP mínimo en `127.0.0.1:8080` que responda `GET /health` con `{ "status": "ok", "vm_state": "running", "connections": 1, "uptime_seconds": 1234 }`. Usado por Systemd `ExecStartPost` para confirmar que el daemon está listo antes de marcar el servicio como activo. Si la VM no está corriendo, retorna `{ "status": "degraded" }` con HTTP 503.

**PROMPT 76** 🔴  
Implementa graceful shutdown en todos los daemons: en `vault-linux/src/lifecycle.rs`, registra handler de señal `SIGTERM` con `tokio::signal::unix`. Al recibir SIGTERM: (1) deja de aceptar nuevas conexiones, (2) envía `ControlMessage::ServerShutdown` a clientes conectados, (3) espera máximo 5 segundos que los clientes confirmen o se desconecten, (4) suspende la VM (no la apaga, para retomar rápido en el próximo start), (5) hace flush de logs, (6) retorna exit code 0.

**PROMPT 77** 🟠  
Implementa circuit breaker para la conexión a la VM en `vault-core/src/services.rs`: si el hypervisor falla 3 veces en 60 segundos, abre el circuit (estado `Open`) y rechaza todas las peticiones con `ErrorCode::HypervisorUnavailable` durante 30 segundos. Luego pasa a `HalfOpen`, permite una petición de prueba: si tiene éxito vuelve a `Closed`, si falla extiende el estado `Open`. Emite métricas de estado del circuit.

**PROMPT 78** 🟠  
Implementa backpressure en el pipeline de streaming: si el cliente no consume frames tan rápido como el servidor los produce, limita la captura de framebuffer en lugar de acumular en buffer. Usa `tokio::sync::Semaphore` con `permits = 3` (máximo 3 frames en vuelo). Si el semaphore está lleno (cliente lento), descarta el frame actual y registra `StreamEvent::FrameDropped`. Expón el frame drop rate en las métricas.

**PROMPT 79** 🟠  
Agrega watchdog en los daemons Systemd: en `vault-runtime.service`, agrega `WatchdogSec=30`. En `vault-linux/src/lifecycle.rs`, implementa el llamado periódico a `sd_notify(SD_NOTIFY_WATCHDOG)` cada 10 segundos. Si el proceso está bloqueado y no envía el watchdog, Systemd lo reinicia automáticamente. Registra cada envío de watchdog con `tracing::debug!` para diagnóstico.

**PROMPT 80** 🟠  
Implementa logging estructurado completo: reemplaza todos los `println!` en el código Rust con `tracing::info/warn/error!`. En `vault-linux`, configura `tracing_subscriber` con output JSON para producción y pretty print para desarrollo (controlado por `VAULT_LOG_FORMAT=json`). Cada span de request debe incluir `request_id` generado al inicio del handling. Rotación de logs diaria en `/var/log/vault/` con retención de 30 días.

---

### FASE 12 — OBSERVABILIDAD Y MONITOREO

**PROMPT 81** 🟠  
Implementa métricas Prometheus en `vault-runtime`: usa la crate `prometheus` (0.13+). Expón en `0.0.0.0:9090/metrics`: `vault_connections_total` counter, `vault_frames_sent_total` counter con label `codec`, `vault_frame_latency_ms` histogram, `vault_vm_cpu_percent` gauge, `vault_vm_ram_mb` gauge, `vault_rekey_operations_total` counter, `vault_errors_total` counter con label `error_code`. Agrega documentación de cada métrica.

**PROMPT 82** 🟠  
Crea `monitoring/docker-compose.yml` para el stack de observabilidad de desarrollo: Prometheus scrapeando `vault-runtime:9090`, Grafana con datasource pre-configurado, y dashboard JSON exportado en `monitoring/dashboards/vault-overview.json` con los paneles: latencia de frame P50/P95/P99, throughput MB/s, CPU/RAM de VM, tasa de error por tipo, y estado de las conexiones activas.

**PROMPT 83** 🟡  
Implementa distributed tracing con OpenTelemetry: agrega `opentelemetry` y `opentelemetry-jaeger` crates. Instrumenta el span del handshake completo, el span de encode+transmit de cada frame, y el span de handling de cada RPC. Configura el exporter para enviar a Jaeger en `http://localhost:14268/api/traces`. Añade `OTEL_EXPORTER_OTLP_ENDPOINT` como variable de entorno configurable.

---

### FASE 13 — SEGURIDAD ADICIONAL (HARDENING)

**PROMPT 84** 🔴  
Implementa sandboxing del proceso `vault-runtime` en Linux: en `vault-runtime.service`, agrega `CapabilityBoundingSet=CAP_NET_BIND_SERVICE`, `NoNewPrivileges=true`, `PrivateTmp=true`, `ProtectSystem=strict`, `ProtectHome=true`, `RestrictNamespaces=true`, `SystemCallFilter=@system-service`. En el código Rust, llama `prctl(PR_SET_NO_NEW_PRIVS)` al inicio. Documenta cada restricción y por qué es segura.

**PROMPT 85** 🔴  
Agrega validación estricta de todos los inputs en `vault-core/src/services.rs`: antes de procesar cualquier `ServiceRequestEnvelope`, valida: tamaño del payload < 10MB, campos string < 1024 bytes, coordenadas de input en rango [0.0, 1.0], timestamps razonables (no en el futuro, no hace más de 60 segundos). Rechaza con `ErrorCode::InvalidInput` y loguea el intento con IP del cliente.

**PROMPT 86** 🔴  
Implementa anti-replay protection en `vault-protocol/src/framing.rs`: cada frame incluye un `seq_number: u64` incrementado monótonamente. El receptor mantiene una ventana de los últimos 1000 sequence numbers vistos. Si llega un frame con seq_number repetido o fuera de la ventana (demasiado viejo), lo rechaza con `FrameError::ReplayDetected` y lo loguea como evento de seguridad. El seq_number se reinicia en 0 tras cada re-keying.

**PROMPT 87** 🟠  
Implementa SecureMemory para claves criptográficas en `vault-crypto`: usa `zeroize` crate. Marca los structs que contienen material de clave con `#[derive(Zeroize, ZeroizeOnDrop)]`: `NoisePrivateKey`, `PqcKeyPair`, `WireChannelState`. Esto asegura que el material de clave se borre de la memoria al hacer drop, reduciendo el riesgo de key material en dumps de memoria.

**PROMPT 88** 🟠  
Configura Content Security Policy en el cliente Android: agrega `android:networkSecurityConfig` en `AndroidManifest.xml` referenciando `res/xml/network_security_config.xml`. En ese archivo, pínea el certificado TLS del servidor (domain pinning) con `<pin-set>`. Para debug builds, permite cleartext en localhost. Para release builds, requiere HTTPS/TLS para todo el tráfico excepto el socket raw del vault (que ya va cifrado con Noise).

**PROMPT 89** 🟠  
Audita el uso de `unsafe` en el código Rust: ejecuta `cargo geiger` y genera reporte de todos los bloques `unsafe`. Para cada uso, añade un comentario `// SAFETY:` explicando por qué es correcto. Los bloques unsafe en JNI bindings son inevitables, pero los del resto del código deben minimizarse. Si hay unsafe en `vault-crypto` que no sea JNI, refactoriza para eliminarlo.

**PROMPT 90** 🟡  
Implementa key stretching en el almacén de secretos: en `vault-linux/src/secret_store.rs`, al guardar una nueva clave derivada de contraseña del usuario, aplica Argon2id con `m=65536, t=3, p=4`. Usa `argon2` crate. Esto protege contra ataques de fuerza bruta al almacén si el disco es comprometido. Documenta los parámetros elegidos y cómo actualizarlos en futuras versiones.

---

### FASE 14 — DOCUMENTACIÓN Y ONBOARDING

**PROMPT 91** 🔴  
Crea `docs/INSTALL.md` con guía de instalación completa para Linux: requisitos del sistema (kernel 5.15+, KVM habilitado, 8GB RAM mínimo, imagen Android-x86), pasos de instalación del daemon (`./daemons/linux/install.sh`), descarga de la imagen Android (`download-android-image.sh`), verificación de la instalación, y troubleshooting de los 10 errores más comunes con sus soluciones.

**PROMPT 92** 🔴  
Crea `docs/SECURITY_MODEL.md` que documente el modelo de seguridad del sistema: threat model (atacantes considerados y no considerados), flujo de confianza desde el QR hasta el framebuffer cifrado, ciclo de vida de las claves criptográficas, qué datos están cifrados en reposo vs en tránsito, qué puede y no puede ver el relay daemon, y las limitaciones conocidas de la implementación actual.

**PROMPT 93** 🟠  
Actualiza el `README.md` principal con: (1) arquitectura diagram ASCII actualizado con el estado real de implementación, (2) quick start en 5 pasos (instalar daemon, descargar imagen Android, instalar APK, escanear QR, conectar), (3) tabla de compatibilidad de hardware con hipervisores probados, (4) badge de CI que muestre estado del build, (5) link a cada documento de fase. Elimina las promesas de funcionalidades aún no implementadas.

**PROMPT 94** 🟠  
Crea `docs/API.md` documentando el protocolo CBOR entre cliente y servidor: para cada tipo de mensaje en `vault-protocol`, documenta el schema CBOR, los campos obligatorios/opcionales, los valores de enum válidos, los posibles códigos de error, y un ejemplo de flujo de request/response. Incluye diagram de secuencia del handshake completo y del flujo de pairing.

**PROMPT 95** 🟡  
Crea `CONTRIBUTING.md` con guía para contribuidores: estructura del repo, cómo configurar el entorno de desarrollo, convenciones de commits (Conventional Commits), cómo correr los tests, cómo añadir soporte para un nuevo codec, cómo añadir soporte para una nueva plataforma, y el proceso de review de código con checklist de seguridad.

---

### FASE 15 — DISTRIBUCIÓN Y EMPAQUETADO

**PROMPT 96** 🔴  
Configura el signing del APK de release: crea el Keystore en Android Studio, guarda el alias y passwords como secretos en GitHub Actions (`KEYSTORE_FILE`, `KEY_ALIAS`, `KEY_PASSWORD`, `STORE_PASSWORD`). En `app/build.gradle.kts`, configura `signingConfigs { release { ... } }` leyendo las variables de entorno. El APK de release generado en CI debe estar firmado y verificable con `apksigner verify`.

**PROMPT 97** 🔴  
Crea `scripts/package-release.sh` que genere el release package completo: (1) compila binarios Rust para todas las plataformas con `cross`, (2) construye APK de release firmado, (3) genera checksums SHA-256, (4) crea `vault-vX.X.X-linux-x86_64.tar.gz` con: binarios, scripts de daemon, imagen de Android (o script de descarga), y docs. Verifica que cada artefacto tiene el checksum correcto antes de publicar.

**PROMPT 98** 🟠  
Implementa auto-update en el servidor: en `vault-core/src/services.rs`, el `AdminAction::UpdateRuntime` debe: (1) descargar el nuevo binario desde una URL firmada, (2) verificar la firma digital con la clave pública del equipo (`ed25519_verify`), (3) reemplazar el binario actual con el nuevo, (4) notificar al cliente que el servidor va a reiniciar, (5) llamar `systemctl restart vault-runtime`. Implementa rollback si el nuevo binario no levanta en 30 segundos.

**PROMPT 99** 🟡  
Crea `flatpak/com.example.VaultRuntime.yml` para distribución del servidor Linux via Flatpak: incluye los binarios `vault-runtime` y `vault-host`, los scripts de daemon, y las dependencias del sistema. Configura el sandbox Flatpak con los permisos mínimos necesarios (`--device=kvm` para virtualización). Esto simplifica la instalación en distribuciones modernas.

---

### FASE 16 — PERFORMANCE Y OPTIMIZACIÓN

**PROMPT 100** 🟠  
Optimiza la latencia de input injection: actualmente el path es `touch event → Android → TCP → Rust → VM`. Mide el latency breakdown con timestamps en cada hop. El objetivo es < 20ms touch-to-render. Si el encoding es el cuello de botella, experimenta con H.264 baseline profile que tiene menor latencia que H.265. Si TCP es el cuello de botella, considera datagrams UDP para input events (son pequeños y se puede perder alguno).

**PROMPT 101** 🟠  
Implementa CPU affinity para el thread de captura de framebuffer: en `vault-linux/src/streaming_pipeline.rs`, usa `nix::sched::sched_setaffinity` para fijar el thread de capture al core 0, y el thread de encoding al core 1. Esto reduce los cache misses de CPU entre capture y encode. Mide el impacto con el benchmark de `criterion`.

**PROMPT 102** 🟠  
Implementa zero-copy para el pipeline de framebuffer: usa `memfd_create` para crear buffers de memoria compartida entre crosvm y vault-runtime. En lugar de copiar el framebuffer, pasa el file descriptor del buffer. Solo copia al comprimir (donde es inevitable). Esto reduce la memoria usada y el tiempo de captura de frame significativamente para resoluciones 1440p.

**PROMPT 103** 🟡  
Implementa connection pooling para las conexiones a la VM: en lugar de crear/destruir conexiones vsock para cada operación, mantén un pool de 4 conexiones reutilizables con `deadpool` o implementación custom. Mide el impacto en el tiempo de respuesta de los RPCs `AdminAction`.

---

### FASE 17 — CARACTERÍSTICAS ADICIONALES PARA VIABILIDAD COMERCIAL

**PROMPT 104** 🟠  
Implementa Multi-tenant support en `vault-runtime`: permite que múltiples clientes Android conecten a VMs independientes en el mismo host. Cada conexión autentica con su propio pairing code y accede solo a su VM aislada. Necesita: pool de VMs pre-calentadas, asignación de ports por tenant (7444, 7445, ...), y aislamiento de recursos via cgroups (`MemoryLimit`, `CPUQuota`).

**PROMPT 105** 🟠  
Implementa panel de administración web minimal: crea `vault-admin-api/` como nuevo crate Rust con `axum` que exponga: `GET /api/sessions` lista de sesiones activas, `DELETE /api/sessions/:id` desconecta una sesión, `GET /api/metrics` métricas JSON, `POST /api/vms` crea nueva VM. Protege el endpoint con Basic Auth o API key almacenada en `SecretStore`. Solo accesible desde localhost.

**PROMPT 106** 🟠  
Agrega soporte para múltiples resoluciones de pantalla dinámicas: permite al cliente solicitar cambio de resolución sin reconectar. En `AdminAction`, agrega `ChangeResolution { width: u32, height: u32 }`. El servidor reconfigura el display virtual del guest (via `xrandr` o VirtIO GPU), actualiza el encoder, y envía un keyframe inmediato. Valida que la resolución esté en la lista de resoluciones soportadas (`720p`, `1080p`, `1440p`, `4K`).

**PROMPT 107** 🟡  
Implementa clipboard sharing bidireccional: agrega `ClipboardSync { content: String, direction: ClipboardDirection }` al protocolo. En Android, usa `ClipboardManager` para leer/escribir. En el guest Linux, usa `xclip` o `wl-copy`. La sincronización se activa manualmente desde el menú flotante de la app Android para evitar exfiltración accidental de datos sensibles.

**PROMPT 108** 🟡  
Implementa soporte para controladores USB virtuales: via VirtIO-Input en crosvm, permite conectar el acelerómetro y giroscopio del teléfono Android como joystick/gamepad en la VM guest. Mapea los datos del sensor (de `TelemetryPayload`) a eventos HID estándar. Útil para aplicaciones de gaming o simulación que usen sensores.

---

### FASE 18 — COMPATIBILIDAD Y PORTABILIDAD

**PROMPT 109** 🟠  
Implementa detección de capacidades del host en `vault-core/src/traits.rs`: agrega método `HostCapabilities::detect()` que retorne `{ has_kvm: bool, has_whp: bool, has_vz_framework: bool, has_vaapi: bool, has_nvenc: bool, cpu_cores: u8, ram_gb: u32 }`. Usa esta información para elegir la configuración óptima de VM y codec. Si las capacidades son insuficientes (< 4 cores o < 4GB RAM), log advertencia y ajusta la config por defecto.

**PROMPT 110** 🟠  
Agrega soporte para Android 14+ en el cliente: verifica compatibilidad con el nuevo modelo de permisos (permissions foto, video, audio cambiados en API 33+). En `AndroidManifest.xml`, declara `uses-permission` apropiados con `maxSdkVersion` donde aplique. Crea clase `PermissionHelper.kt` con solicitud de permisos runtime agrupada por funcionalidad (camera para QR, audio para streaming de audio).

**PROMPT 111** 🟡  
Implementa modo offline/local para desarrollo: cuando `VAULT_DEV_MODE=1`, el servidor levanta sin imagen Android real y simula respuestas para todas las APIs. El cliente Android en `BuildConfig.DEBUG = true` conecta a `localhost:7443` automáticamente. Esto permite desarrollar la UI del cliente sin hardware de servidor disponible.

---

### FASE 19 — AUDITORÍA DE CÓDIGO

**PROMPT 112** 🔴  
Ejecuta una auditoría de todas las dependencias criptográficas del proyecto: verifica que `snow` (Noise_XX), `ml-kem`, `chacha20poly1305`, y `hkdf` usan implementaciones auditadas y sin vulnerabilidades conocidas. Para cada crate, documenta su versión, última auditoría conocida, y alternativas consideradas. Si alguna crate tiene un advisory abierto, planifica la migración.

**PROMPT 113** 🔴  
Revisa y corrige todos los usos de `rand` en el código: todos los valores criptográficamente sensibles (nonces, claves efímeras, IVs) deben usar `rand::rngs::OsRng` o `getrandom` directamente. Nunca `thread_rng()` para valores de seguridad. Busca con `grep -rn "thread_rng\|StdRng::from_entropy"` y evalúa cada uso.

**PROMPT 114** 🟠  
Verifica que no haya timing side-channels en las comparaciones criptográficas: busca comparaciones de tipo `==` sobre byte arrays que contengan MACs, hashes o claves con `grep -rn "== key\|== mac\|== hash"`. Reemplaza con `subtle::ConstantTimeEq::ct_eq()` para prevenir timing attacks. Especialmente crítico en la verificación del handshake Noise.

---

### FASE 20 — PREPARACIÓN FINAL DE RELEASE

**PROMPT 115** 🔴  
Crea `CHANGELOG.md` siguiendo el formato Keep a Changelog: documenta la versión `v1.0.0-beta.1` con las secciones Added, Changed, Fixed, Security. Para la sección Security, lista explícitamente los algoritmos criptográficos implementados y sus propiedades de seguridad. Para Changed, lista las diferencias respecto al diseño original documentado en las FASE_*.md. 

**PROMPT 116** 🔴  
Define y documenta los SLOs (Service Level Objectives) del sistema en `docs/SLO.md`: latencia de frame P99 < 50ms en red local, latencia de frame P99 < 150ms en red de area metropolitana, tiempo de pairing < 10 segundos, tiempo de reconexión < 5 segundos, downtime de live migration < 2 segundos, throughput de streaming >= 15 Mbps. Implementa los tests que verifiquen estos SLOs en CI.

**PROMPT 117** 🔴  
Crea el script `scripts/pre-release-check.sh` que valide todos los criterios de release: (1) todos los tests pasan, (2) `cargo audit` limpio, (3) versiones en `Cargo.toml` coinciden con el tag git, (4) `CHANGELOG.md` actualizado, (5) APK firmado con la clave de producción, (6) checksums generados, (7) smoke test end-to-end en CI pasa. Si algún check falla, lista todos los fallos antes de abortar.

---

### PROMPTS DE FUNCIONALIDADES AVANZADAS (POST-MVP)

**PROMPT 118** 🟡  
Implementa soporte multi-display: permite que la VM tenga hasta 4 displays virtuales independientes. El cliente Android navega entre displays con swipe horizontal. Cada display es un surface renderizado independiente. En crosvm, usa `--gpu displays=4` y configura `VirtioGpu` con múltiples scanouts.

**PROMPT 119** 🟡  
Implementa grabación de sesión: agrega `AdminAction::StartRecording { path }` que grabe el stream de video/audio en formato MKV usando FFmpeg pipes. La grabación se guarda en el almacenamiento cifrado del host. `AdminAction::StopRecording` finaliza y retorna el path del archivo. El cliente puede solicitar descarga del archivo via `AdminAction::DownloadFile`.

**PROMPT 120** 🟡  
Implementa port forwarding desde la VM: permite redirigir un puerto del guest a uno del host via VirtIO Network. `AdminAction::PortForward { guest_port: u16, host_port: u16 }` configura el forwarding. Útil para acceder a servicios corriendo en la VM desde otras máquinas. Implementa con `iptables` / `nftables` en Linux con limpieza automática al terminar la sesión.

**PROMPT 121** 🟡  
Implementa notificaciones de sistema del guest Android en el cliente: usando `ADB` o un helper APK pre-instalado en el guest, captura las notificaciones del guest y las muestra como notificaciones locales en el cliente Android. Permite al usuario ver/descartar notificaciones del guest desde la barra de notificaciones del host, sin abrir el stream de video.

**PROMPT 122** 🟡  
Implementa soporte para stylus/lápiz digital: en el cliente Android, detecta eventos `MotionEvent.ACTION_DOWN` de `InputDevice.SOURCE_STYLUS`. Incluye `pressure`, `tiltX`, `tiltY`, y `tool_type` en el `InputEventPayload`. En el guest, mapeado a HID stylus device via VirtIO-Input. Útil para aplicaciones de dibujo o firma digital en el guest.

**PROMPT 123** 🟡  
Implementa passthrough de cámara: el cliente Android captura frames de la cámara trasera/delantera y los envía al servidor como `CameraFramePayload`. El servidor los inyecta en el device virtual de cámara del guest via V4L2 loopback device (`v4l2loopback` kernel module). Apps en la VM pueden usar la cámara del teléfono Android directamente.

**PROMPT 124** 🟡  
Implementa backup y restore de la VM: `AdminAction::BackupVm { destination_path }` crea un snapshot QCOW2 del disco y lo cifra con AES-256-GCM usando una clave derivada del pairing. `AdminAction::RestoreVm { backup_path }` verifica la integridad del backup antes de restaurar. Implementa backup incremental para reduzir el tamaño: solo almacena las páginas modificadas respecto al último backup.

---

### PROMPTS DE COMPATIBILIDAD EXTENDIDA

**PROMPT 125** 🟡  
Agrega soporte para Chromebook (Chrome OS) como cliente: crea `app-chromeos/` con un Android app modificado que aproveche el factor de forma de laptop: soporte para teclado físico con mapeo correcto de teclas especiales, soporte para trackpad como dispositivo de puntero, y window management que permita la app en modo ventana (non-fullscreen). Usa Android 13+ Chromebook APIs.

**PROMPT 126** 🟡  
Implementa cliente CLI mínimo en Rust: crea `cli-client/` como nuevo crate con binario `vault-connect`. Acepta flags `--host`, `--port`, `--pairing-code`. Realiza el handshake Noise_XX, y abre un forwarding SSH-like: `vault-connect --ssh-forward localhost:2222` redirige el puerto SSH del guest al host local. Útil para acceso headless a la VM sin la app Android.

**PROMPT 127** 🟡  
Implementa soporte para Raspberry Pi 5 como host: verifica compatibilidad con KVM en ARM64 (funciona en Pi 5 con 64-bit OS). En `vault-linux/src/hypervisor.rs`, detecta arquitectura aarch64 y ajusta el comando crosvm: usa `--cpu type=host` en lugar de emulación, configura `VirtIO-GPU` apropiado para la GPU VideoCore VII del Pi. Documenta los requisitos de cooling para uso continuo.

**PROMPT 128** 🟡  
Agrega soporte para NixOS: crea `nix/vault.nix` con el módulo NixOS que declare los servicios `vault-runtime` y `vault-host` como opciones de `services.vault.*`. Define el package `vault` en `nix/package.nix` compilando desde fuente con las dependencias correctas del nixpkgs. Incluye tests de integración NixOS en `nix/tests/`.

---

### PROMPTS DE TESTING AVANZADO

**PROMPT 129** 🟠  
Implementa chaos testing para el servidor: crea `rust/tests/chaos.rs` que use `tokio::time::pause()` para simular condiciones adversas: (1) pérdida de red en medio del streaming (verifica reconexión), (2) muerte del proceso de VM durante una sesión activa (verifica recovery), (3) agotamiento de RAM (verifica graceful degradation), (4) clock skew de 30 segundos (verifica que el protocolo tolera desincronización de tiempo).

**PROMPT 130** 🟠  
Implementa mutation testing: usa `cargo-mutants` para identificar código con coverage insuficiente. Ejecuta en CI semanalmente sobre los módulos `vault-crypto` y `vault-protocol`. Reporta los mutantes sobrevivientes como TODOs de test. El objetivo: 0 mutantes sobrevivientes en funciones de verificación criptográfica.

**PROMPT 131** 🟠  
Crea un framework de load testing: usa `criterion` + `tokio_test` para simular N clientes concurrentes conectando al servidor. Mide: tiempo de handshake bajo carga (1, 10, 50 clientes), throughput de framebuffer agregado, latencia de RPCs bajo carga. Define los thresholds de regresión: el P99 no debe incrementar > 20% respecto al baseline medido en hardware de referencia.

**PROMPT 132** 🟡  
Crea test de penetración automatizado: usa `nuclei` con templates customizados para el protocolo vault. Verifica: que el relay no expone información del protocolo interno, que el rate limiting funciona contra conexiones flood, que frames con seq_numbers manipulados son rechazados, que el servidor responde correctamente a payloads CBOR malformados. Integra en CI como job semanal.

---

### PROMPTS DE EXPERIENCIA DE USUARIO

**PROMPT 133** 🟠  
Mejora el flujo de onboarding en el cliente Android: crea `OnboardingActivity.kt` con 4 pantallas: (1) "Bienvenido a Vault" con descripción del producto, (2) "Instala el servidor" con instrucciones por plataforma y QR de descarga, (3) "Escanea el código de pairing" con tutorial interactivo del escáner, (4) "Conexión establecida" con resumen de seguridad. Guarda el flag `onboarding_completed` en DataStore para no mostrarlo de nuevo.

**PROMPT 134** 🟠  
Implementa modo de accesibilidad en el cliente Android: agrega soporte para TalkBack (screen reader) en los paneles de control. El framebuffer remoto no puede ser accesible por naturaleza, pero todos los controles (botones, sliders, indicadores de estado) deben tener `contentDescription`. Implementa modo alto contraste verificando `WindowManager.isHighContrastTextEnabled()`.

**PROMPT 135** 🟠  
Agrega soporte para tablet en el cliente Android: usa `WindowSizeClass` de Compose para adaptar el layout. En tablets (expanded width), muestra el framebuffer a la izquierda y los paneles de control a la derecha en split-view. En teléfonos, mantiene el layout actual de pantalla completa con menú flotante. Verifica en emuladores de tablet 10" y 12.9".

**PROMPT 136** 🟡  
Implementa temas de UI en el cliente Android: además del tema Obsidian Dark actual, agrega Light Theme y Dynamic Color (Material You) que adapta los colores al wallpaper del sistema en Android 12+. En `app/src/main/java/com/example/ui/theme/Theme.kt`, usa `dynamicDarkColorScheme` / `dynamicLightColorScheme` cuando estén disponibles, con fallback al tema Obsidian.

**PROMPT 137** 🟡  
Implementa historial de conexiones en el cliente Android: guarda las últimas 10 conexiones exitosas en DataStore con: host, port, timestamp, fingerprint, nombre asignado por el usuario. En la pantalla principal, muestra la lista con botón "Reconectar" y opción de eliminar. Permite nombrar cada server ("Mi PC", "Trabajo", "Servidor Cloud").

---

### PROMPTS FINALES DE INTEGRACIÓN

**PROMPT 138** 🔴  
Realiza una prueba de integración completa del sistema end-to-end: (1) servidor Linux con crosvm corriendo imagen Android-x86, (2) cliente Android físico conectando por red local, (3) handshake Noise_XX + ML-KEM-768 completo, (4) streaming H.265 a 1080p30 por 5 minutos sin drops, (5) input injection de 100 eventos táctiles verificando respuesta visual, (6) re-keying manual exitoso, (7) desconexión y reconexión en < 5 segundos. Documenta los resultados.

**PROMPT 139** 🔴  
Corrige todos los warnings de compilación en el workspace Rust: ejecuta `cargo build --workspace 2>&1 | grep "^warning"` y corrige cada warning. Los warnings más comunes en este tipo de código: variables no usadas, imports no usados, dead code en stubs. Configura `#![deny(warnings)]` en los crates de producción (`vault-protocol`, `vault-crypto`, `vault-core`) para prevenir regresiones.

**PROMPT 140** 🔴  
Realiza la auditoría final de dependencias: ejecuta `cargo tree --duplicate` para detectar versiones múltiples de la misma crate. Para dependencias críticas de seguridad (`ring`, `openssl-sys`, `rustls`), asegura que solo haya una versión en el árbol. Actualiza `Cargo.lock` con `cargo update` y verifica que todos los tests siguen pasando tras la actualización.

**PROMPT 141** 🔴  
Prepara el `metadata.json` con información de release correcta: actualiza `version`, `min_server_version`, `protocol_version`, `build_date`, `supported_platforms`, y el SHA-256 del APK de release. Este archivo es leído por el mecanismo de auto-update. Firma el archivo con la clave ED25519 del equipo y incluye la firma en `metadata.json.sig`.

**PROMPT 142** 🔴  
Verifica que el APK release pasa el análisis de Play Store: ejecuta `bundletool validate --bundle app.aab` (o `apksigner verify app.apk`). Verifica que no hay APIs deprecadas en `minSdk` (26). Verifica con `android lint --abortOnError` que no hay issues graves. Verifica `targetSdk=35` tiene todos los cambios de comportamiento de Android 15 compatibles (gestos de sistema, etc.).

**PROMPT 143** 🟠  
Crea `SECURITY.md` con la política de disclosure de vulnerabilidades: dirección de email para reporte privado, tiempo esperado de respuesta (48 horas), proceso de coordinación de divulgación, versiones soportadas (solo la más reciente), y agradecimiento a investigadores de seguridad en el changelog. Referencia a CVE numbering authority si aplica.

**PROMPT 144** 🟠  
Implementa el comando de diagnóstico en `vault-host`: `vault-runtime --diagnose` ejecuta una batería de checks y reporta: (1) estado de KVM/WHP/VZ, (2) permisos de `/dev/kvm`, (3) espacio en disco disponible, (4) versión de crosvm instalada, (5) conectividad al puerto 7443 desde localhost, (6) estado de los servicios Systemd. Formato JSON para parsing automático + texto para consumo humano.

**PROMPT 145** 🟠  
Implementa el self-update del daemon: `vault-runtime --update` descarga la última versión desde la URL configurada, verifica la firma ED25519, la instala en `/usr/local/bin/vault-runtime.new`, reinicia el servicio via `systemctl restart`. Implementa rollback automático si el nuevo binario no levanta en 60 segundos: `systemctl restart vault-runtime` con el binario previo.

---

### PROMPTS DE ESCALABILIDAD

**PROMPT 146** 🟡  
Implementa clustering de servidores: permite que múltiples instancias de `vault-runtime` en diferentes hosts formen un cluster. El cliente se registra con el cluster y puede ser asignado a cualquier nodo. Implementa con un coordinator simple en Redis (o archivo compartido en NFS): cada nodo registra su capacidad disponible, el coordinator asigna nuevas sesiones al nodo con más capacidad libre.

**PROMPT 147** 🟡  
Implementa auto-scaling de VMs: cuando el número de sesiones activas supera el 80% de la capacidad, el daemon lanza una nueva instancia de VM pre-calentada en background. Cuando cae por debajo del 20%, destruye las VMs idle más antiguas. Configura el pool mínimo/máximo con `VmPoolConfig { min: 1, max: 5, warm_standby: 1 }` en la configuración del daemon.

**PROMPT 148** 🟡  
Implementa rate limiting por usuario en lugar de por IP: tras el pairing, cada sesión tiene un `session_id` único. El rate limiter aplica los límites de RPC por session_id, no por IP. Esto es más correcto para casos donde múltiples usuarios comparten un NAT. Define límites por tier: `free: 100 req/min`, `premium: 1000 req/min`, configurable en `vault.toml`.

---

### PROMPTS DE INTERNACIONALIZACIÓN

**PROMPT 149** 🟡  
Agrega internacionalización (i18n) al cliente Android: extrae todos los strings hardcoded de `MainActivity.kt` a `res/values/strings.xml`. Crea traducciones para: `res/values-es/strings.xml` (español), `res/values-de/strings.xml` (alemán), `res/values-ja/strings.xml` (japonés). Usa `LocaleList` para el fallback correcto. Los mensajes de error del servidor (en inglés) muestran el código de error junto con la traducción.

**PROMPT 150** 🟡  
Implementa Right-to-Left (RTL) support en el cliente Android: en `app/build.gradle.kts`, verifica que `supportsRtl="true"` está en `AndroidManifest.xml`. En los composables de `MainActivity.kt`, reemplaza `Arrangement.Start/End` con `Arrangement.Start` y `layoutDirection`-aware equivalents. Verifica el layout en emulador con locale `ar` (árabe) o `he` (hebreo).

---

### PROMPTS DE PRUEBAS DE PLATAFORMA

**PROMPT 151** 🟠  
Crea matrix de testing en CI para múltiples versiones de Android: en `.github/workflows/android-ci.yml`, agrega jobs paralelos para API levels 26 (mínimo), 30, 33, 34, y 35. Usa emuladores de AVD con snapshots para acelerar el arranque. Reporta compatibilidad por API level. Si algún test falla solo en API 26-28 documenta el bug como "known issue" en esos niveles.

**PROMPT 152** 🟠  
Crea testing de compatibilidad de hipervisor en CI: usa GitHub Actions con `ubuntu-24.04` (tiene KVM disponible en algunos runners) o un self-hosted runner con `/dev/kvm`. El test levanta `vault-runtime` con la imagen Android mínima de test, verifica que la VM arranca, y corre los integration tests contra la VM real. Marca los tests como `#[cfg(feature = "kvm_required")]` para skippearlos cuando no hay KVM.

**PROMPT 153** 🟡  
Implementa testing contra múltiples versiones de crosvm: en `.github/workflows/`, descarga crosvm stable (última release), crosvm nightly, y crosvm de distribución (el incluido en Ubuntu 24.04). Corre los integration tests contra cada versión para detectar regresiones de API. Si alguna versión falla, loguea un warning pero no bloquea el CI.

---

### PROMPTS DE DOCUMENTACIÓN TÉCNICA ADICIONAL

**PROMPT 154** 🟠  
Documenta el protocolo de pairing con diagramas de secuencia en `docs/PAIRING_PROTOCOL.md`: (1) flujo completo del QR TOFU con cada mensaje CBOR, (2) condiciones de error y cómo manejarlas en el cliente, (3) cómo regenerar el código QR si expira, (4) qué información se almacena en el SecretStore tras el pairing exitoso, (5) cómo revocar un pairing existente. Incluye diagramas ASCII o Mermaid.

**PROMPT 155** 🟡  
Crea `docs/PERFORMANCE_TUNING.md`: guía para optimizar el rendimiento del sistema según el hardware disponible. Secciones: tuning de crosvm para máxima performance de VM, elección de codec y encoder según GPU disponible, configuración de CPU affinity, ajuste de buffer sizes para diferentes latencias de red, configuración de QoS para priorizar tráfico de streaming.

**PROMPT 156** 🟡  
Crea ADR (Architecture Decision Records) en `docs/adr/`: un documento por cada decisión arquitectónica importante. Mínimo: ADR-001 elección de Noise_XX sobre TLS, ADR-002 ML-KEM-768 sobre X25519 puro, ADR-003 CBOR sobre JSON/Protobuf, ADR-004 crosvm sobre QEMU, ADR-005 Rust sobre C++ para el backend. Formato: contexto, decisión, consecuencias.

---

### PROMPTS DE HARDENING ADICIONAL

**PROMPT 157** 🟠  
Implementa isolation del storage por sesión: cada sesión tiene su propio LUKS2 volume montado solo durante la sesión. Usa `dm-crypt` con `--sector-size 4096` para performance. La clave de cada volume se deriva del `session_id` + `device_secret` via HKDF. Al terminar la sesión, el volume se desmonta automáticamente y la clave se descarta de memoria.

**PROMPT 158** 🟠  
Implementa audit log inmutable: todos los eventos de seguridad (conexiones, intentos fallidos, re-keying, migrations) se escriben en un append-only log firmado digitalmente. Cada entrada incluye timestamp, event_type, session_id, y firma HMAC-SHA256 encadenada al hash del entry anterior (similar a una blockchain simple). El log puede exportarse con `vault-runtime --export-audit-log` para auditoría externa.

**PROMPT 159** 🟡  
Implementa detection de entornos de análisis en el cliente Android: verifica si la app corre en emulador o entorno de análisis dinámico (Frida, Magisk con Zygisk, debugging activo). Si detecta análisis potencial, reduce la funcionalidad a modo demo y advierte al usuario. Implementa detecciones: `Debug.isDebuggerConnected()`, checks de emulador IMEI, detección de hooks Frida via `/proc/self/maps`.

**PROMPT 160** 🟡  
Implementa certificate transparency para las actualizaciones: cuando el daemon descarga una actualización, verifica que el certificado TLS del servidor de updates aparece en CT logs (Sunlight, Google CT, Cloudflare Nimbus). Usa la API de CT directamente o via `ctclient` library. Esto previene ataques de suplantación con certificados válidos pero no transparentes.

---

### PROMPTS DE ECOSISTEMA

**PROMPT 161** 🟡  
Crea SDK cliente para desarrolladores externos: extrae las clases `VaultWebSocket`, `NoiseHandshake`, y `VaultJNI` en un módulo `vault-android-sdk` publicable en Maven Central. Define la API pública con anotaciones `@PublishedApi` en Kotlin. Publica documentación KDoc en GitHub Pages. Esto permite a terceros construir clientes Android alternativos sobre el protocolo vault.

**PROMPT 162** 🟡  
Crea plugin de Wireshark para el protocolo vault: el plugin disecciona frames CBOR del protocolo (asumiendo que el analista tiene la clave de descifrado). Implementa en Lua: parsea el length-prefix, identifica el `msg_type` del frame, muestra los campos CBOR formateados. Útil para debugging del protocolo en desarrollo. Incluye instrucciones de instalación del plugin.

**PROMPT 163** 🟡  
Crea extensión de VS Code para desarrollo con vault: (1) syntax highlighting para los tipos de protocolo en `.cbor` files, (2) snippets para implementar `EncryptedStorage` y `AndroidHypervisor` traits, (3) tarea de build que compile el workspace Rust + APK Android en un paso, (4) debugger launch configuration para `vault-runtime` con variables de entorno preconfiguradas.

---

### PROMPTS DE COMPLIANCE Y REGULATORIO

**PROMPT 164** 🟠  
Documenta el cumplimiento con GDPR/privacidad: identifica qué datos personales procesa el sistema (ubicación GPS del guest, datos del sensor, metadata de sesión). Define política de retención: logs de sesión por 30 días, audit log por 1 año, sin retención de datos del framebuffer. Implementa `AdminAction::DeleteAllSessionData` para el derecho al olvido. Documenta en `docs/PRIVACY.md`.

**PROMPT 165** 🟡  
Verifica cumplimiento con FIPS 140-3: los módulos criptográficos usados deben tener certificación FIPS o usar primitivas FIPS-aprobadas. AES-256, SHA-256, y ECDH son FIPS-aprobados. ML-KEM-768 es FIPS 203. ChaCha20-Poly1305 no es FIPS-aprobado; documenta esto en `SECURITY.md` como limitación para ambientes que requieran FIPS estricto y ofrece un modo alternativo con AES-256-GCM.

---

### PROMPTS DE OPERACIONES

**PROMPT 166** 🟠  
Crea runbook de operaciones en `docs/RUNBOOK.md`: procedimientos para los escenarios más comunes: (1) reinicio del daemon tras actualización de kernel, (2) recovery tras crash de VM, (3) revocación de un cliente comprometido, (4) rotación de claves de servidor, (5) backup y restore de una sesión, (6) diagnosis y resolución de alta latencia de streaming. Cada procedimiento incluye comandos exactos y verificación de éxito.

**PROMPT 167** 🟡  
Implementa modo de mantenimiento: `vault-runtime --maintenance` acepta conexiones existentes pero rechaza nuevas con `ErrorCode::MaintenanceMode`. El cliente Android muestra "Servidor en mantenimiento — intentando reconectar en [countdown]". Permite al operador hacer actualizaciones sin cortar conexiones activas abruptamente. Define tiempo máximo de mantenimiento en la configuración (default 30 minutos).

---

### PROMPTS FINALES

**PROMPT 168** 🔴  
Realiza el smoke test final de release: con el APK firmado de release y los binarios de producción, sigue exactamente los pasos del `INSTALL.md` en una máquina Linux limpia (Ubuntu 24.04 minimal install). Documenta cualquier paso que falle o sea confuso. Corrige el `INSTALL.md` basándote en la experiencia. El objetivo: un desarrollador que nunca ha visto el proyecto puede instalarlo y conectarse en < 30 minutos.

**PROMPT 169** 🔴  
Crea el tag `v1.0.0-beta.1` en git: verifica con `scripts/pre-release-check.sh` que todos los criterios pasan, crea el tag anotado con `git tag -a v1.0.0-beta.1 -m "Primera beta pública"`, ejecuta el workflow de release en GitHub Actions, verifica que los artefactos se publican correctamente, y redacta las release notes en GitHub destacando: qué funciona, qué es experimental, y qué está fuera de scope para esta beta.

**PROMPT 170** 🔴  
Configura el repositorio de GitHub para el proyecto: activa GitHub Security Advisories, configura Dependabot para Rust (`cargo`) y Android (`gradle`) con PRs automáticos de updates de seguridad. Activa Code Scanning con CodeQL para Kotlin y la extensión para Rust. Configura branch protection en `main`: require 1 review, require status checks CI/Android/Security passing, no force push.

**PROMPT 171** 🟠  
Implementa telemetría opt-in en el cliente Android: al primer uso, pregunta si el usuario acepta enviar telemetría anónima (crash reports, tiempo de conexión, codecs usados). Si acepta, usa Firebase Crashlytics para crashes y un endpoint propio para métricas de uso. Los datos de sesión (host, streams, contenido) nunca se envían. La telemetría es opt-out fácil desde Settings.

**PROMPT 172** 🟠  
Crea `app/src/main/java/com/example/settings/SettingsScreen.kt` con pantalla de configuración del cliente: (1) información del servidor conectado, (2) gestión de servidores guardados (editar/eliminar), (3) configuración de calidad de streaming (auto/manual), (4) configuración de seguridad (re-keying manual, revocar pairing), (5) diagnóstico de red (latencia, jitter, packet loss), (6) opt-in/out de telemetría, (7) versión de app y servidor.

**PROMPT 173** 🟠  
Implementa modo de bajo consumo de batería en el cliente Android: cuando la batería < 20% o el modo ahorro de energía está activo (`PowerManager.isPowerSaveMode()`), reduce automáticamente: FPS a 15, resolución a 720p, audio desactivado, telemetría pausada. Notifica al usuario del cambio con un snackbar. Restaura la configuración original cuando la batería sube > 30% o se conecta a cargador.

**PROMPT 174** 🟡  
Implementa exportación del perfil de conexión como QR: en la app Android, permite compartir los datos de conexión de un servidor (host, port, fingerprint, nombre) como QR code. Otro dispositivo Android puede escanear este QR para importar el servidor sin necesitar el código de pairing de nuevo (solo transfiere la dirección y fingerprint, no la clave de sesión). Usa el mismo scheme `vault://connect?...`.

**PROMPT 175** 🟡  
Crea demo interactivo para el repositorio: en `demo/` crea un `docker-compose.yml` que levante: un servidor vault simulado (con respuestas mockeadas pero handshake real), y un cliente web mínimo (HTML/JS) que realice el handshake y muestre un "Hello from Vault" animado. Esto permite a cualquiera evaluar el protocolo sin hardware especial. Documenta cómo correrlo en `demo/README.md`.

---

## RESUMEN DE PRIORIDADES

### Para Beta Pública (v1.0.0-beta.1)

**BLOQUEANTES (Deben completarse):**
- Prompts 1-6: Hipervisor Linux funcional
- Prompts 19-23: Codecs de video/audio funcionales  
- Prompts 27-28: ML-KEM-768 real
- Prompts 35-40: Cliente Android con conexión real
- Prompts 46-48: Servidor TCP real
- Prompts 56-60: Tests de integración básicos
- Prompts 66-68: CI/CD básico
- Prompts 74-76: Manejo de errores crítico
- Prompts 96-97: APK firmado y empaquetado
- Prompt 168-170: Release checklist

**IMPORTANTES (Mejorar calidad):**
- Prompts 7-9: Métricas y configuración de VM
- Prompts 29-33: Security hardening
- Prompts 41-45: UX del cliente Android
- Prompts 49-52: Mejoras de red
- Prompts 61-64: Testing adicional
- Prompts 80-83: Observabilidad
- Prompts 84-89: Hardening de seguridad

**DESEABLES (Post-beta):**
- Prompts 53-55: Live Migration real
- Prompts 118-132: Features avanzados
- Prompts 146-175: Escalabilidad y ecosistema

---

## HOJA DE RUTA TEMPORAL

| Semana | Hitos |
|--------|-------|
| 1-2 | Hipervisor Linux funcional (crosvm real) + Pipeline de video básico |
| 3-4 | Codecs H.265/Opus + Streaming completo servidor→cliente |
| 5-6 | ML-KEM-768 real + Cliente Android con conexión real |
| 7-8 | CI/CD completo + Tests de integración |
| 9-10 | Security hardening + APK firmado |
| 11-12 | Testing E2E + Documentación de instalación |
| 13 | Release v1.0.0-beta.1 |

---

*Documento generado el 2026-08-01 para el proyecto virtual_app_droid*  
*Total: 175 prompts de implementación*
