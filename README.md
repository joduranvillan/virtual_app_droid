# 🛡️ Virtual App Droid — Android Confidential Execution Vault

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Android](https://img.shields.io/badge/Platform-Android_12%2B-green.svg)](https://developer.android.com/)
[![Rust](https://img.shields.io/badge/Core Engine-Rust_1.75%2B-orange.svg)](https://www.rust-lang.org/)
[![Post-Quantum Security](https://img.shields.io/badge/Security-ML--KEM--768%20%2B%20Curve25519-purple.svg)](#-seguridad-y-criptografía-post-cuántica)
[![Build Status](https://img.shields.io/badge/Build-Passing-brightgreen.svg)](#)

**Virtual App Droid** es una solución integral de alta seguridad y ejecución confidencial descentralizada que une un cliente móvil nativo en **Android (Jetpack Compose & Material Design 3)** con un motor de hipervisión y orquestación multi-nodo escrito en **Rust (Cargo Workspaces)**.

Diseñado bajo la filosofía **Zero-Trust**, el sistema permite el empaquetado, streaming e interacción en tiempo real de instancias aisladas de ejecución remota (KVM / Hypervisor) con garantías criptográficas avanzadas de nivel gubernamental/financiero.

---

## 📁 Índice de Documentación del Proyecto

El proyecto cuenta con una suite completa de documentos técnicos detallados por fases:

* 📄 [**ARQUITECTURA.md**](./ARQUITECTURA.md) — **Documento Maestro de Arquitectura y Especificaciones Técnicas del Sistema**.
* 🛡️ [**FASE_F_SEGURIDAD_POST_CUANTICA_DOCUMENTACION.md**](./FASE_F_SEGURIDAD_POST_CUANTICA_DOCUMENTACION.md) — Handshake Post-Cuántico (ML-KEM-768), Atestación HSM Anti-Tamper y Migración en Caliente.
* ⚡ [**DAEMONS_DOCUMENTACION.md**](./DAEMONS_DOCUMENTACION.md) — Servicios nativos de fondo en Linux (Systemd), macOS (Launchd) y Windows (SCM).
* 🔄 [**ROADMAP_MULTIPLATFORM.md**](./ROADMAP_MULTIPLATFORM.md) — Hoja de ruta para expansión multiplataforma (Desktop/Web/iOS).
* 🧪 [**FASE_E2E_PRUEBAS_INTEGRACION_DOCUMENTACION.md**](./FASE_E2E_PRUEBAS_INTEGRACION_DOCUMENTACION.md) — Pruebas de integración E2E e infraestructura de simulación.
* 🔒 [**FASE_A_DOCUMENTACION.md**](./FASE_A_DOCUMENTACION.md) — Reestructuración, crates Rust y protocolo de emparejamiento por QR (TOFU).
* 📺 [**FASE_B_DOCUMENTACION.md**](./FASE_B_DOCUMENTACION.md) — Streaming binario CBOR (H.265/Opus) e inyección interactiva de eventos.
* 🖥️ [**FASE_1_HIPERVISORES_DOCUMENTACION.md**](./FASE_1_HIPERVISORES_DOCUMENTACION.md) — Integración de hipervisores nativos KVM, Apple Hypervisor Framework y Windows Hypervisor Platform.
* 🛡️ [**FASE_C_DOCUMENTACION.md**](./FASE_C_DOCUMENTACION.md), [**FASE_D_DOCUMENTACION.md**](./FASE_D_DOCUMENTACION.md) y [**FASE_E_DOCUMENTACION.md**](./FASE_E_DOCUMENTACION.md) — Almacenamiento seguro encriptado (LUKS2/Keychain/DPAPI) y atestación de plataforma.

---

## 🚀 Características Principales

### 1. Criptografía Híbrida Post-Cuántica (PQC)
* **ML-KEM-768 + Curve25519**: Intercambio de llaves seguro frente a computadoras cuánticas según especificaciones NIST FIPS 203.
* **Handshake Cifrado `Noise_XX`**: Autenticación mutua de ambos extremos sin transmitir identidades en texto plano.
* **Derivación de Claves HKDF-SHA256**: Generación de material clave de sesión con re-keying dinámico periódico.

### 2. Atestación de Integridad Anti-Tamper y Enclave HSM
* **Verificación de Binarios y Hardware**: Firma criptográfica de binarios contra hardware HSM (Hardware Security Module / TPM 2.0 / Android Keystore).
* **Defensa Anti-Depuración**: Detección en tiempo real de inyección de código, hooking de funciones o Frida/gdb, revocando llaves de sesión al instante.

### 3. Migración en Caliente de Sesión (Live Migration Zero-Downtime)
* **Conmutación en Caliente entre Nodos del Clúster**: Migra sesiones de framebuffer en ejecución activa entre `Node-Alpha` (x86_64 High-Perf), `Node-Beta` (ARM64 Ampere) y `Node-Gamma` (Edge Micro-Host).
* **Downtime < 2.0 ms**: Transfiera ~819.2 MB de páginas de memoria alteradas sin cortar el stream ni perder contexto de VCPU.

### 4. Streaming y Control Interactivo Multiplataforma
* **Protocolo de Streaming CBOR**: Codificación binaria compacta para frames H.265/H.264 y audio Opus.
* **Inyección de Entrada Normalizada `[0.0, 1.0]`**: Eventos táctiles (`TouchDown`, `TouchMove`, `TouchUp`) y teclas físicas de hardware independentes de la resolución remota.
* **Telemetría Virtual de GPS**: Envío seguro de coordenadas simuladas o reales codificadas en CBOR desde el cliente móvil hacia el contenedor.

---

## 🏗️ Estructura del Proyecto

```
virtual_app_droid/
├── app/                                  # Aplicación Cliente Android (Kotlin & Jetpack Compose)
│   ├── src/main/java/com/example/        # MainActivity.kt & UI Theme Obsidian Dark
│   └── src/test/                         # Pruebas Robolectric & Roborazzi Screenshot Testing
├── rust/                                 # Motor Core en Rust (Cargo Workspace)
│   ├── crates/vault-protocol/            # Modelos de datos y serialización CBOR sin I/O
│   ├── crates/vault-crypto/              # Handshake Noise_XX, ML-KEM-768 y HKDF
│   ├── crates/vault-core/                # Orquestador, rate-limiting y traits de plataforma
│   ├── crates/vault-stream/              # Pipeline binario de audio/video e input
│   ├── crates/vault-linux/               # Adaptador OS Linux (Systemd, KVM, LUKS2)
│   ├── crates/vault-macos/               # Adaptador OS macOS (Launchd, Hypervisor.framework)
│   └── crates/vault-windows/             # Adaptador OS Windows (SCM Service, WHP)
├── daemons/                              # Scripts de instalación de servicios de fondo
│   ├── linux/                            # Systemd (.service, install.sh, uninstall.sh)
│   ├── macos/                            # Launchd (.plist, install.sh, uninstall.sh)
│   └── windows/                          # PowerShell SCM setup (setup.ps1, uninstall.ps1)
├── README.md                             # Este archivo
├── ARQUITECTURA.md                       # Documento Maestro de Arquitectura
└── FASE_*_DOCUMENTACION.md               # Documentación técnica específica por fases
```

---

## 🛠️ Requisitos e Instalación

### Cliente Móvil (Android)
* **Android Studio**: Ladybug / Hedgehog o posterior.
* **Android SDK**: `compileSdk = 35`, `minSdk = 26` (Android 8.0+).
* **Build System**: Gradle 8.x con Kotlin DSL (`build.gradle.kts`).

Para compilar e instalar el cliente Android:
```bash
./gradlew :app:assembleDebug
```

### Motor en Rust y Daemons
* **Rust**: `1.75+` (`cargo`, `rustc`).
* **Dependencias de Sistema**: `OpenSSL` (si aplica), cabeceras de compilación C.

Para compilar todos los crates de Rust:
```bash
cd rust
cargo build --release
```

Para instalar los daemons de fondo en Linux:
```bash
cd daemons/linux
sudo ./install.sh
```

---

## 🔐 Seguridad y Licencia

El código fuente se distribuye bajo la licencia **MIT**. Consulte el archivo `LICENSE` para más detalles.

---
*Desarrollado con enfoque en Seguridad Confidencial, Rendimiento Multi-nodo y Criptografía de Grado Militar.*
