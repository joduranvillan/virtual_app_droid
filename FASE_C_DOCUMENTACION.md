# Documentación de Fase C: Adaptador de Escritorio Windows

La **Fase C** del proyecto Confidential Vault introduce el soporte multiplataforma para sistemas de escritorio Windows. Utiliza las primitivas de seguridad nativas del sistema operativo de Microsoft para igualar la robustez criptográfica y el aislamiento de almacenamiento que LUKS2 ofrece en Linux.

---

## 1. Arquitectura del Adaptador Windows (`vault-windows`)

El crate `vault-windows` actúa como el adaptador nativo del sistema operativo Windows, abstrayendo los tres traits esenciales de `vault-core`:

```
┌─────────────────────────────────────────────────────────────────┐
│                           vault-windows                         │
│                                                                 │
│  ┌───────────────────────┐ ┌─────────────────┐ ┌─────────────┐  │
│  │ WindowsEncryptedStore │ │ DpapiSecretStore│ │ HypervStub  │  │
│  │ (BitLocker + VHDX)    │ │ (Microsoft DPAPI)│ │ (Hyper-V VM)│  │
│  └───────────────────────┘ └─────────────────┘ └─────────────┘  │
└────────────────────────────────┬────────────────────────────────┘
                                 │ Implementa
┌────────────────────────────────▼────────────────────────────────┐
│                           vault-core                            │
│                                                                 │
│         EncryptedStorage   SecretStore   AndroidHypervisor      │
└─────────────────────────────────────────────────────────────────┘
```

---

## 2. Protección de Secretos mediante DPAPI (`DpapiSecretStore`)

En lugar de depender de contraseñas guardadas en texto plano o claves de cifrado en archivos que sufren el problema de "dónde guardar la clave de la clave", `DpapiSecretStore` utiliza la **API de Protección de Datos de Windows (DPAPI)** mediante las funciones nativas:
*   `CryptProtectData`: Cifra la identidad estática del runtime (`runtime_identity.key`) y el pin de emparejamiento del frontend (`pinned_frontend.pub`) de forma transparente utilizando las credenciales del usuario actual (ej. `NT AUTHORITY\SYSTEM` si se ejecuta como Windows Service) o de la máquina local.
*   `CryptUnprotectData`: Descifra los secretos solo si el proceso actual corre con las credenciales de seguridad autorizadas.

Esto previene que otros usuarios del sistema local o copias de seguridad de archivos no autorizadas puedan comprometer la identidad criptográfica del Vault.

---

## 3. Almacenamiento Cifrado con VHDX y BitLocker (`WindowsEncryptedStorage`)

El almacenamiento seguro para el Android Runtime en Windows se implementa combinando dos tecnologías empresariales de Microsoft:
1.  **Imágenes de Disco Virtual (VHDX)**: Se automatiza la creación, montaje y formateo de un disco virtual NTFS dinámico mediante comandos de PowerShell (`New-VHD`, `Mount-DiskImage`, `Initialize-Disk`, `Format-Volume`).
2.  **Cifrado de Unidad BitLocker**: Se protege el volumen montado (por ejemplo, asignado temporalmente a la letra de unidad `V:`) utilizando BitLocker con la clave simétrica derivada como contraseña (`Enable-BitLocker -PasswordProtector`).
3.  **Apertura Segura**: Al iniciar la sesión, se monta el VHDX y se desbloquea de forma segura mediante la utilidad de consola `manage-bde.exe -unlock V: -Password <key>` de forma totalmente automatizada.
4.  **Cierre Automático**: Al bloquearse el Vault, se cierra la unidad (`manage-bde.exe -lock V:`) y se desmonta (`Dismount-DiskImage`), garantizando que los datos queden completamente inaccesibles.

---

## 4. Canalización de Red Local (Localhost Loopback TCP)

En Linux, `vault-host` y `vault-runtime` se comunican a través de sockets de dominio UNIX local (`runtime.sock` y `enrollment_info.sock`). Dado que los sockets UNIX tienen limitaciones de compatibilidad y soporte histórico en distintas versiones de Windows, el adaptador Windows adopta un diseño de red local extremadamente portátil:

*   **Loopback TCP**: `vault-runtime` escucha conexiones de relay en `127.0.0.1:7444` y sirve información de enrolamiento QR en `127.0.0.1:7445`.
*   **Relay de Red Aislado**: `vault-host` sigue actuando como un relay ciego en el puerto público `7443` y redirige el flujo de bytes directamente al puerto de loopback local. Esto mantiene el principio de diseño de **Zero-Visibility Forwarder** (el host no puede inspeccionar el tráfico Noise_XX del Vault).

---

## 5. Compilación Cruzada y Pruebas Unitarias

Para asegurar que todo el espacio de trabajo compile y se testee correctamente en entornos de integración continua (incluyendo sistemas de desarrollo basados en Linux o macOS), `vault-windows` implementa una arquitectura híbrida con directivas `#[cfg(windows)]` y `#[cfg(not(windows))]`:

*   **En Windows**: Ejecuta de forma nativa las APIs criptográficas de DPAPI y los comandos de automatización de PowerShell/BitLocker.
*   **En otros sistemas**: Simula de forma segura el almacenamiento (creando directorios locales simulados) y la criptografía (mediante una máscara XOR de prueba), lo que garantiza que los 41 tests de integración del workspace sigan pasando de manera limpia al correr en cualquier plataforma de desarrollo.
