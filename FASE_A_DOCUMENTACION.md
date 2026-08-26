# Documentación de Fase A: Reestructuración y Enlace Seguro de Bóveda

La **Fase A (Opción A)** del proyecto establece los cimientos de seguridad y la separación limpia de responsabilidades del sistema Confidential Vault, habilitando un flujo de emparejamiento descentralizado y de confianza mutua.

---

## 1. Objetivos de Diseño de la Fase A

### Separación de Responsabilidades (Crates Multiplataforma)
El motor en Rust del backend ha sido desacoplado para asegurar la portabilidad y robustez criptográfica:
*   **`vault-protocol`**: Tipos puros de datos, mensajes y de/serialización CBOR. Sin lógica de I/O.
*   **`vault-crypto`**: Criptografía pura. Implementa el handshake `Noise_XX` y derivación de llaves por HKDF.
*   **`vault-core`**: Orquestación y lógica pura de enrolamiento y límites de tasa (*rate-limiting*). Define las interfaces de plataforma (`EncryptedStorage`, `AndroidHypervisor`, `SecretStore`).
*   **`vault-linux`**: Adaptador de sistema operativo que implementa los traits de `vault-core` (LUKS2, permisos de archivos, llamadas de kernel).

### Arquitectura del Frontend (Android Físico)
El cliente Android actúa como la única interfaz de usuario y raíz de confianza (*Root of Trust*). Sella las llaves criptográficas del dispositivo y alimenta los sensores virtuales de la bóveda.

```
┌─────────────────────────────────┐          ┌──────────────────────────────────┐
│      Android Client (App)       │          │          Confidential Host       │
│                                 │          │                                  │
│  ┌───────────────────────────┐  │  Noise   │  ┌────────────────────────────┐  │
│  │  VaultConnectionManager   │◄─┼─XX/TCP───┼─►│  vault-host (Daemon/Relay) │  │
│  │   - Handshake Noise_XX    │  │  Cifrado │  │   - locked/unlocked state  │  │
│  │   - Session Key Exchange  │  │          │  │   - opens LUKS2 vault.img  │  │
│  └───────────────────────────┘  │          │  └──────────────┬─────────────┘  │
│  ┌───────────────────────────┐  │          │                 │ Unix Socket    │
│  │  VirtualLocationService   │  │          │  ┌──────────────▼─────────────┐  │
│  │   - Local GPS provider    │  │          │  │  vault-runtime (Aislado)   │  │
│  │   - Encoded CBOR telemetry│  │          │  │   - RPC Service Router     │  │
│  └───────────────────────────┘  │          │  │   - Cryptographic Term.    │  │
└─────────────────────────────────┘          └──────────────────────────────────┘
```

---

## 2. Flujo de Emparejamiento Seguro (Pairing por QR)

El flujo sigue un modelo **TOFU (Trust-On-First-Use)**, similar al utilizado por Signal o WhatsApp Web:

1.  **Generación de Desafío**: Al iniciar, `vault-runtime` sin emparejar genera un token efímero de un solo uso válido por 10 minutos.
2.  **Exposición del Código**: `vault-host` genera y sirve un código QR local con los metadatos de red y la llave pública del entorno remoto.
3.  **Escaneo de Confianza**: El usuario utiliza la cámara del teléfono para escanear el QR, importando de manera segura la dirección, el puerto, el token y la firma pública del host.
4.  **Handshake Inicial**: El teléfono inicia un protocolo `Noise_XX` hacia el host y verifica que la firma pública coincida exactamente con la extraída del QR (previniendo ataques MITM).
5.  **Confirmación y Sello**: Tras establecer el canal cifrado temporal, el teléfono envía la confirmación de enrolamiento. El host valida el token de un solo uso, guarda de forma permanente la firma pública del teléfono en su almacén seguro (*SecretStore*) y cierra permanentemente el proceso de emparejamiento para futuras conexiones.

---

## 3. Implementación y Robustez en el Frontend (Compose UI)

La aplicación de control móvil ha sido implementada con un diseño premium utilizando **Jetpack Compose** y Material Design 3, optimizada para ofrecer retroalimentación de estado y visualización criptográfica:

*   **Esquema de Color "Obsidian Dark"**: Un fondo oscuro de alto contraste y acentos cian y verde eléctrico que reducen la fatiga visual.
*   **Lector de Credenciales Integrado**: Utiliza un motor de escaneo de cámara adaptivo para interpretar las credenciales del QR con validación de caducidad integrada.
*   **Conexión en Tiempo Real**: Visualización dinámica del estado del handshake `Noise` mediante un hilo de registro en pantalla ("Preparando canal...", "Iniciando Handshake...", "Enlace seguro activo").
*   **Telemetría de Ubicación Virtual**: Sincroniza las coordenadas GPS locales obtenidas mediante `play-services-location`, las serializa a CBOR y las envía al router de la bóveda para responder las peticiones de ubicación de manera confiable.
*   **Visor de Firmas Públicas**: Muestra de forma legible el *fingerprint* hexadecimal de las firmas públicas local y remota para que el usuario pueda auditarlas físicamente.
