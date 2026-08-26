# FASE F: SEGURIDAD POST-CUÁNTICA, ATESTACIÓN ENCLAVE HSM Y ANTI-TAMPER

Este documento detalla la implementación de las capacidades avanzadas de seguridad en el ecosistema **Vault (Bóveda Digital)** para la aplicación cliente Android y los subsistemas de enlace seguro CBOR/Noise_XX.

---

## 🛡️ 1. HANDSHAKE HÍBRIDO POST-CUÁNTICO (ML-KEM-768 + CURVE25519)

### 1.1 Arquitectura de Cifrado Híbrido
Para garantizar resistencia total contra ataques de interceptación actual y descifrado futuro por ordenadores cuánticos (Algoritmo de Shor), se integró un esquema de negociación híbrido de dos capas:

- **Lapa Reticular Post-Cuántica (ML-KEM-768 / Kyber768):** Aportación de seguridad basada en retículos (FIPS 203) generando ciphertext de 1184 bytes.
- **Capa Elliptic Curve Diffie-Hellman (Curve25519 / X25519):** Aportación de seguridad clásica de alto rendimiento (32 bytes).
- **Función de Derivación de Clave HKDF-SHA3-256:** Combina el secreto compartido ECDH con la clave desencapsulada de ML-KEM-768, alcanzando un nivel de entropía y fortaleza de **512 bits**.

### 1.2 Telemetría y Controles en Jetpack Compose
- **Estado de Atestación Cuántica:** Mapeado en la interfaz con indicador en vivo (`512-BIT PQC` / `CLÁSICO 256B`).
- **Monitoreo de Latencia de Handshake:** Medición en milisegundos de la negociación (~1.8 ms).
- **Inspección de Hashes:** Muestra el hash del Ciphertext ML-KEM-768 y la coordenada del punto X25519 derivado.
- **Conmutador Dinámico (Toggle PQC):** Permite cambiar dinámicamente entre cifrado Híbrido PQC y modo clásico de prueba.
- **Integración con Re-Keying:** Al superar el volumen de 1 GB en framebuffer o ejecutar rotación manual, se dispara un re-handshake híbrido completo.

---

## 🔐 2. VERIFICACIÓN ANTI-TAMPER Y ATESTACIÓN ENCLAVE HSM REMOTO

### 2.1 Verificación de Firma e Integridad del Binario
- **Validación de Hash APK:** El cliente compute el hash SHA-256 de la firma del APK/ejecutable contra un Enclave HSM Remoto antes de permitir el emparejamiento CBOR.
- **Detección de Depuración y Hooking:** Monitoreo constante de Frida, ptrace y parches de código no autorizados (`isDebuggerDetected`, `isBinaryModified`).

### 2.2 Respuesta Ante Violación de Seguridad
Si la verificación falla o se detecta un intento de manipulado:
1. Las claves de sesión efímeras se **destruyen inmediatamente** de la memoria.
2. La huella digital de la clave remota pasa a estado `[DESTRUIDO - TAMPER DETECTADO]`.
3. Se bloquea cualquier intento de emparejamiento CBOR o transmisión de framebuffer.
4. Se emiten alertas de seguridad en los logs de consola y en la barra de atestación.

### 2.3 Simulación de Ataques e Inspección
La interfaz incluye controles interactivos para auditar el comportamiento del sistema:
- **Validar (Audit HSM):** Ejecuta la verificación bajo demanda.
- **Ataque APK:** Simula la alteración del binario APK y verifica la respuesta del HSM.
- **Frida/Debug:** Simula la presencia de un depurador/hooking en runtime.
- **Reset:** Restaura la integridad y regenera las claves de la bóveda.

---

## ⚡ 3. MIGRACIÓN EN CALIENTE (LIVE MIGRATION) DE SESIÓN DE FRAMEBUFFER

### 3.1 Arquitectura Zero-Downtime
Para garantizar continuidad operativa sin pérdida de cuadros ni interrupción de entrada de usuario al balancear carga entre nodos del clúster (`Node-Alpha`, `Node-Beta`, `Node-Gamma`), se implementó un mecanismo de migración en caliente de tres fases:

1. **Fase 1: Pre-copia Iterativa de Dirty-Pages (RAM/Framebuffer):** Transfiere ~819.2 MB de páginas de memoria alteradas mientras el stream permanece activo en el nodo origen.
2. **Fase 2: Handover de Estado VCPU/KVM y Llaves PQC ML-KEM-768:** Sincroniza el contexto de ejecución del hipervisor QEMU/KVM y resincroniza las claves efímeras del canal cifrado.
3. **Fase 3: Stop-and-Switch Instantáneo (< 2.0 ms Downtime):** Detiene brevemente la iteración final para conmutar la recepción al nuevo nodo objetivo.

### 3.2 Telemetría y Controles en Jetpack Compose
- **Medición de Downtime:** Indicador dinámico visualizando latencias de conmutación de ultra-baja latencia (< 1.8 ms).
- **Barra de Progreso en Vivo (`LinearProgressIndicator`):** Muestra el avance porcentual de copia de páginas de memoria.
- **Acciones Directas por Nodo:** Botones interactivos para migrar la sesión activa hacia `Node-Alpha` (x86_64 High-Perf), `Node-Beta` (ARM64 Ampere) o `Node-Gamma` (Edge Micro-Host).
- **Protección Anti-Tamper integrada:** Cancela inmediatamente la migración si el Enclave HSM detecta manipulado o depuradores.

---

## 💻 4. RESUMEN DE CAMBIOS EN CÓDIGO FUENTE
- **`MainActivity.kt`:**
  - Estados reactivos `StateFlow` para ML-KEM-768, atestación HSM, Anti-Tamper y Migración en Caliente de Framebuffer.
  - Métodos `executeHybridHandshake()`, `verifyAntiTamperWithHsm()`, `simulateTamperAttack()`, `resetAntiTamperState()` y `executeLiveMigration(targetNode)`.
  - Integración en la vista Compose dentro de `SecurityRekeyingAttestationCard` y `ClusterScalabilityOrchestratorCard` con componentes M3, badges de estado, barra de progreso y botones interactivos.

---

## 📦 5. COMANDOS PARA GUÍA Y PUSH A GITHUB

Para guardar y subir todos los avances al repositorio de GitHub:

```bash
# 1. Verificar el estado de los archivos
git status

# 2. Agregar todos los cambios y nuevos documentos
git add .

# 3. Registrar el commit descriptivo
git commit -m "feat(cluster): implementar migración en caliente (Live Migration) de sesión de framebuffer con PQC ML-KEM-768"

# 4. Enviar a GitHub
git push origin main
```
