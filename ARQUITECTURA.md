# 🏛️ Documento Maestro de Arquitectura — Virtual App Droid

## 1. Visión General del Sistema

**Virtual App Droid** (Android Confidential Execution Vault) es un ecosistema distribuido de ejecución confidencial multi-nodo. Permite aislar, transmitir y controlar aplicaciones y entornos de ejecución virtualizados de manera segura entre clientes móviles Android y servidores remotos (hosts) multiplataforma (Linux, macOS, Windows).

El diseño prioriza la **seguridad Zero-Trust**, la **resiliencia criptográfica post-cuántica**, la **portabilidad nativa** y la **conmutación de nodos sin interrupción de servicio (Live Migration)**.

```
┌───────────────────────────────────────────────────────────────────────────────────┐
│                           CLIENTE MÓVIL (ANDROID)                                 │
│                                                                                   │
│   ┌───────────────────────────┐    Visualización     ┌─────────────────────────┐  │
│   │   Jetpack Compose UI      │◄────────────────────┤  Stream HUD / Frame     │  │
│   │   - Dark Obsidian Theme   │                     │  Decoder H.265 / CBOR   │  │
│   │   - Interactive Touchpad  │────────────────────►│  Input Injector         │  │
│   └─────────────┬─────────────┘    Coordenadas [0..1]└────────────┬────────────┘  │
│                 │                                                 │               │
│                 ▼                                                 ▼               │
│   ┌───────────────────────────┐                      ┌─────────────────────────┐  │
│   │  VaultConnectionManager   │                      │ VirtualLocationService  │  │
│   │  - Handshake Noise_XX     │                      │ - GPS CBOR Telemetry    │  │
│   │  - PQC ML-KEM-768         │                      └─────────────────────────┘  │
│   └─────────────┬─────────────┘                                                   │
└─────────────────┼─────────────────────────────────────────────────────────────────┘
                  │
                  │ Canal Cifrado Mutuo (Noise_XX / ML-KEM-768 / TCP 7443)
                  ▼
┌───────────────────────────────────────────────────────────────────────────────────┐
│                         CONFIDENTIAL HOST / CLUSTER                               │
│                                                                                   │
│   ┌───────────────────────────────────────────────────────────────────────────┐   │
│   │                       vault-host (Blind Relay Daemon)                     │   │
│   │   - Escucha en puerto público 7443                                        │   │
│   │   - Redirección ciega a bucle local (sin visibilidad de claves)           │   │
│   └─────────────────────────────────────┬─────────────────────────────────────┘   │
│                                         │ Domain Socket / Named Pipe              │
│   ┌─────────────────────────────────────▼─────────────────────────────────────┐   │
│   │                       vault-runtime (Aislado / Root)                      │   │
│   │   - Desencriptador y Router RPC                                           │   │
│   │   - Gestor de Enclave Anti-Tamper & Atestación HSM                        │   │
│   │   - Almacén encriptado LUKS2 / Keychain / DPAPI                           │   │
│   └─────────────────────────────────────┬─────────────────────────────────────┘   │
│                                         │                                         │
│   ┌─────────────────────────────────────▼─────────────────────────────────────┐   │
│   │                     Orquestador de Hipervisores (KVM / WHP)               │   │
│   │   - Live Migration Controller (Zero-Downtime < 2 ms)                      │   │
│   │   - Instancia virtualizada en aislamiento de hardware                     │   │
│   └───────────────────────────────────────────────────────────────────────────┘   │
└───────────────────────────────────────────────────────────────────────────────────┘
```

---

## 2. Capas de la Arquitectura

### 2.1 Cliente Móvil (Android - Kotlin & Jetpack Compose)
* **Root of Trust de Usuario**: Administra las claves efímeras del usuario, autoriza conexiones y escanea el código QR de emparejamiento (TOFU - Trust On First Use).
* **Consola Interactiva Remota**: Canvas dinámico que traduce gestos táctiles a coordenadas normalizadas `[0.0, 1.0]` y retransmite eventos en tiempo real.
* **Módulo de Seguridad Post-Cuántica**: Realiza la encapsulación de claves híbridas (ML-KEM-768 + Curve25519) y verifica la atestación HSM de la plataforma remota.
* **Orquestador de Clúster y Migración**: Interfaz M3 para conmutar dinámicamente el nodo activo de la sesión (`Node-Alpha`, `Node-Beta`, `Node-Gamma`) con telemetría de downtime en tiempo real.

### 2.2 Motor de Backend en Rust (Cargo Workspace)
El backend está organizado en crates con estricta separación de responsabilidades:

1. **`vault-protocol`**:
   * Define los esquemas de datos puros y mensajes CBOR.
   * Totalmente libre de dependencias de I/O o llamadas al sistema.
2. **`vault-crypto`**:
   * Implementa el protocolo de handshake `Noise_XX`.
   * Integra el intercambio de claves Post-Cuántico ML-KEM-768 y la derivación HKDF-SHA256.
3. **`vault-core`**:
   * Define los contratos abstractos (`traits`): `EncryptedStorage`, `AndroidHypervisor`, `SecretStore`.
   * Contiene la lógica de negocio de enrolamiento, control de tasa de peticiones (*rate-limiting*) y estados de bloqueo.
4. **`vault-stream`**:
   * Pipeline de baja latencia para empaquetado de cuadros de video (H.265/H.264), muestras de audio Opus y eventos de entrada.
5. **`vault-linux` / `vault-macos` / `vault-windows`**:
   * Adaptadores específicos de sistema operativo.
   * Implementan la integración directa con hipervisores nativos (KVM, Hypervisor.framework, WHP), utilidades de cifrado de disco (LUKS2, Keychain, DPAPI) y daemons de sistema.

---

## 3. Protocolo de Seguridad Criptográfica y Anti-Tamper

### 3.1 Handshake Híbrido Post-Cuántico (NIST FIPS 203 ML-KEM-768 + Curve25519)
El establecimiento del canal cifrado combina la velocidad del intercambio clásico mediante Curve25519 con la resistencia cuántica de ML-KEM-768:

1. **Efímero Clásico + Cuántico**: El cliente genera un par de claves efímeras Curve25519 y un ciphered secret ML-KEM-768.
2. **Mezcla HKDF-SHA256**: Se derivan las llaves de sesión mediante la concatenación del secreto compartido Diffie-Hellman y el secreto des-encapsulado cuánticamente:
   $$\text{MasterKey} = \text{HKDF-Extract}(\text{Salt}, \text{SS}_{\text{Curve25519}} \mathbin{\Vert} \text{SS}_{\text{ML-KEM-768}})$$
3. **Cifrado de Tramas ChaCha20-Poly1305**: Todas las tramas de streaming y control RPC utilizan AEAD ChaCha20-Poly1305 con vector de inicialización incrementado por secuencia.

### 3.2 Atestación HSM y Protección Anti-Tamper
* **Sello por Hardware (Hardware Root of Trust)**: El daemon verifica la integridad del binario contra el módulo TPM 2.0 / Enclave HSM antes de habilitar la llave de descifrado del volumen.
* **Monitoreo en Tiempo Real**: Si se detecta un depurador adjunto (`ptrace`), inyección de memoria o manipulación binaria, el enclave destruye inmediatamente las claves de sesión en RAM (`zeroize`).

---

## 4. Live Migration Zero-Downtime (Migración en Caliente)

Para permitir el balanceo de carga o mantenimiento de servidores sin interrumpir la experiencia del usuario, el sistema implementa un protocolo de migración en caliente de tres fases:

1. **Pre-copia Iterativa de Dirty-Pages (RAM/Framebuffer)**: El nodo origen transfiere incrementalmente las páginas de memoria modificadas (~819.2 MB) mientras el stream de video continúa activo.
2. **Sincronización de Contexto de VCPU e Intercambio PQC**: Se transfiere el estado de ejecución del procesador virtual (KVM/QEMU) y se resincroniza la clave de sesión ML-KEM-768 en el nodo destino.
3. **Stop-and-Switch Instantáneo (< 2.0 ms Downtime)**: Se conmuta la recepción de entrada y streaming al nuevo nodo objetivo sin desconectar el cliente móvil.

---

## 5. Matriz de Compatibilidad de Plataformas

| Componente | Linux | macOS | Windows | Android Client |
| :--- | :---: | :---: | :---: | :---: |
| **Daemon Service** | Systemd (`.service`) | Launchd (`.plist`) | SCM (Windows Service) | — |
| **Hipervisor Nativo** | KVM / QEMU | Hypervisor.framework | WHP (Windows Hypervisor Platform) | — |
| **Almacén Seguro** | LUKS2 / libsecret | Keychain / Secure Enclave | DPAPI / Windows Hello | Android Keystore |
| **Cliente de Control UI** | — | — | — | Jetpack Compose (Material Design 3) |

---
