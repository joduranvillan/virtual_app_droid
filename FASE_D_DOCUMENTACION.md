# Documentación de Fase D: Adaptador de Escritorio macOS

La **Fase D** del proyecto Confidential Vault introduce el soporte multiplataforma para sistemas de escritorio Apple macOS (OS X), extendiendo el núcleo de seguridad común de `vault-core` mediante las primitivas criptográficas y de almacenamiento nativas de Apple.

---

## 1. Arquitectura del Adaptador macOS (`vault-macos`)

El crate `vault-macos` actúa como el adaptador nativo para macOS, abstrayendo de manera limpia los tres traits esenciales de `vault-core`:

```
┌─────────────────────────────────────────────────────────────────┐
│                           vault-macos                           │
│                                                                 │
│  ┌───────────────────────┐ ┌─────────────────┐ ┌─────────────┐  │
│  │ MacosEncryptedStorage │ │ AppleKeychainSS │ │ HypervStub  │  │
│  │ (APFS Sparse Bundle)  │ │ (Apple Keychain)│ │ (Hypervisor)│  │
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

## 2. Protección de Secretos mediante Apple Keychain (`AppleKeychainSecretStore`)

En macOS, los secretos persistentes (claves de identidad y PIN del frontend) se delegan al **Keychain del Sistema de Apple** utilizando el framework nativo `Security` a través de Rust bindings:

*   **Keychain de Apple**: La clave de identidad estática (`identity_keypair`) y la clave pública del frontend vinculada (`pinned_frontend`) se guardan de forma nativa como contraseñas genéricas (`GenericPassword`) bajo el servicio corporativo `com.example.confidentialvault`.
*   **Aislamiento de Seguridad**: Esto permite que el sistema operativo proteja los secretos utilizando características criptográficas respaldadas por hardware (como **Secure Enclave** en dispositivos Apple Silicon de arquitectura ARM64 o chips de seguridad T2 en dispositivos Intel antiguos).
*   **Capa Multiplataforma**: Al igual que el adaptador de Windows, se provee un fallback automático basado en archivos en reposo para sistemas que no sean macOS, garantizando la compatibilidad con el ecosistema de integración continua (CI) de Linux.

---

## 3. Almacenamiento Cifrado con APFS y Sparse Bundle (`MacosEncryptedStorage`)

El almacenamiento seguro para la VM de Android se realiza utilizando imágenes de disco virtuales dinámicas en macOS:

1.  **APFS Sparse Bundle**: Se automatiza la creación de un Sparse Bundle cifrado (`.sparsebundle`) de tamaño dinámico máximo de 4GB mediante el comando nativo `hdiutil create -size 4g -fs APFS -volname ConfidentialVault -encryption AES-256 -stdinpass`.
2.  **Montaje Seguro**: Al desbloquearse la sesión, se monta la imagen en el punto de montaje local de volumen (habitualmente `/Volumes/ConfidentialVault`) mediante `hdiutil attach -stdinpass -mountpoint <PATH>`, pasando la clave simétrica derivada por stdin para evitar que quede registrada en el historial del shell de procesos.
3.  **Cierre y Desmontaje**: Al bloquearse, se desmonta con `hdiutil detach` forzando la liberación de archivos abiertos, garantizando que el contenedor quede cerrado criptográficamente de inmediato en el disco.

---

## 4. Comunicación Segura de Sockets UNIX POSIX

Dado que macOS es un sistema operativo POSIX robusto certificado, se conserva el diseño limpio de sockets locales de dominio UNIX implementado para Linux, en lugar de recurrir al Loopback de red TCP:

*   `vault-runtime` abre el socket local en `/tmp/vault_runtime.sock` para sesiones seguras y `/tmp/vault_enrollment_info.sock` para la consulta del estado de emparejamiento.
*   `vault-host` actúa como el **Zero-Visibility Forwarder** escuchando el puerto TCP público `7443` de red y reenviando el flujo de bytes de forma transparente al socket de runtime de macOS, manteniendo aislado al operador de las claves Noise_XX.
*   El servidor de enrolamiento local se expone en el puerto local `8088` de macOS, consultando inter-proceso el socket UNIX para obtener la identidad de emparejamiento temporal de la sesión activa y renderizar de forma interactiva el código QR en formato SVG.
