# ROADMAP MULTIPLATAFORMA - PROYECTO VAULT (BÓVEDA DIGITAL)

Este documento detalla el estado actual y la hoja de ruta para el ecosistema Vault en plataformas Android, Linux, Windows y macOS.

---

## 1. Estado de Implementación por Plataforma

### 📱 Android (vault-android) - Estado: COMPLETADO ✅
- [x] **Panel de Conexión Segura**: Estado de enlace mTLS / CBOR, dirección IP y puerto del Bóveda Kernel.
- [x] **Proceso de Emparejamiento por Código / QR**: Escaneo dinámico con CameraX y parseo de payloads URI `vault://pair`.
- [x] **Gestión de Claves Criptográficas**: Huellas dactilares SHA-256 de claves de dispositivo y nodo remoto.
- [x] **Pantalla Remota Interactiva & Inyección de Eventos**: Renderizado de framebuffer, passthrough de eventos táctiles, teclado y controles virtuales.
- [x] **Administración Headless RPC**: Control de red (`net_block`, `net_allow`, `status`) y versiones (`rollback`, `update_latest`, `version_info`).
- [x] **Monitor de Rendimiento VM**: Medidor de CPU vCore (1 a 4 cores), uso de RAM LPDDR5, temperatura del hipervisor e I/O VirtIO.
- [x] **Telemetría de Sensores**: GPS passthrough, Acelerómetro (X/Y/Z), Giroscopio y estado de Batería en tiempo real.
- [x] **Controles Interactivos de Streaming**: Conmutador de FPS (15, 30, 60 FPS), resolución (720p, 1080p, 1440p), pausa de video, audio Opus y captura de pantalla.
- [x] **Atestación de Seguridad & Re-Keying**: Protocolo Noise_XX de rotación de claves, verificación TEE ARM TrustZone y cifrado cuántico ML-KEM-768.
- [x] **Orquestador de Clúster & Multicodec**: Conmutación de nodos (Node-Alpha, Node-Beta, Edge Node) y selección de aceleración por hardware (AV1, H.265, VP9, H.264).

### 🖥️ Daemons de Fondo y Multiplataforma (daemons/) - Estado: COMPLETADO ✅
- [x] **Linux (Systemd)**: Script `install.sh` / `uninstall.sh` con servicios systemd (`vault-control-plane.service`, `vault-vm-runner.service`), cgroups y sandbox.
- [x] **Windows (NSSM / Windows Service)**: Script `install.ps1` / `uninstall.ps1` con creación de servicios nativos de Windows, reglas de Firewall y registro en Event Log.
- [x] **macOS (Launchd)**: Script `install.sh` / `uninstall.sh` con Plists LaunchDaemons (`com.example.vault-*.plist`), logs en `/var/log/vault` y gestión `launchctl`.
- [x] **Empaquetador de Lanzamientos (`package.sh`)**: Generación automatizada de distribución en archivos `.tar.gz` y `.zip`.

---

## 2. Documentación de Prompts e Instrucciones para Futuras Iteraciones

Para guiar a desarrolladores y agentes de Inteligencia Artificial en próximas evoluciones de seguridad, escalabilidad y compatibilidad hardware, se ha generado el archivo:
📄 **`PROMPTS_MEJORAS_SEGURIDAD_ESCALABILIDAD.md`**

---

## 3. Comandos para Subir Cambios al Repositorio de GitHub

```bash
# Agregar todos los archivos modificados y nuevos
git add .

# Crear commit descriptivo
git commit -m "feat: implementar atestación de seguridad Noise_XX, orquestador de clúster multi-nodo, controles de streaming y prompts de escalabilidad"

# Subir cambios al repositorio de GitHub
git push origin main
```
