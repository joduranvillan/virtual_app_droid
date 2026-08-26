# PROMPTS & GUÍA DE MEJORAS: SEGURIDAD, COMPATIBILIDAD Y ESCALABILIDAD

Este documento proporciona **prompts estructurados y directivas de arquitectura** diseñadas para guiar a desarrolladores y agentes de IA en la implementación continua de mejoras de seguridad, compatibilidad multiplataforma y escalabilidad en el ecosistema **Vault (Bóveda Digital)**.

---

## 🛡️ 1. MEJORAS DE SEGURIDAD (SECURITY ENHANCEMENTS)

### Prompt 1.1: Rotación de Claves Noise_XX en Tiempo Real
> **Prompt:**  
> *"Implementa la rotación dinámica de claves de sesión (re-keying) para el canal de transporte cifrado Noise_XX en Vault. Asegúrate de que las claves efímeras se regeneren automáticamente cada N minutos o tras la transferencia de 1 GB de datos de framebuffer. Agrega atestación de hardware mediante ARM TrustZone / Intel SGX TEE para verificar la integridad del enclave antes de autorizar la rotación."*

### Prompt 1.2: Atestación de Integridad y Anti-Tamper
> **Prompt:**  
> *"Añade un módulo de verificación Anti-Tamper en la aplicación cliente (Android / macOS / Windows) que compute el hash de la firma APK/Binario contra un enclave HSM remoto antes de iniciar el canal CBOR. Si se detecta modificación o depuración no autorizada, destruye las claves efímeras en memoria y bloquea el intento de emparejamiento."*

### Prompt 1.3: Cifrado Híbrido Resistente a Computación Cuántica (Post-Quantum)
> **Prompt:**  
> *"Integra un esquema de intercambio de claves híbrido utilizando ML-KEM-768 (Kyber) junto con Curve25519 para el canal Handshake de emparejamiento. Muestra el estado de atestación cuántica en el panel de telemetría de la interfaz cliente."*

---

## ⚡ 2. MEJORAS DE ESCALABILIDAD (SCALABILITY ENHANCEMENTS)

### Prompt 2.1: Orquestador y Balanceador de Cómputo Multi-Nodo
> **Prompt:**  
> *"Diseña e implementa una capa de orquestación multi-nodo para el backend de Máquinas Virtuales de Vault. Permite al cliente Android/Desktop seleccionar dinámicamente o migrar la sesión en vivo entre nodos del clúster (High-Perf x86_64, Ampere ARM64 o Edge Micro-Hosts) sin pérdida de estado de framebuffer."*

### Prompt 2.2: Asignación Dinámica de Recursos VM (Hot-Plug vCPU & vRAM)
> **Prompt:**  
> *"Crea controles interactivos y mensajes RPC en CBOR/QMP para solicitar ajuste dinámico de recursos de hardware en la VM invitada (de 2 a 16 cores vCPU y de 4 GB a 32 GB vRAM) según las demandas del juego o aplicación en ejecución."*

---

## 🌐 3. MEJORAS DE COMPATIBILIDAD (COMPATIBILITY ENHANCEMENTS)

### Prompt 3.1: Motor Multicodec con Fallback Hardware (AV1 / H.265 / VP9 / H.264)
> **Prompt:**  
> *"Implementa negociación automática de codecs de video en el cliente Android. Si el chip SoC no soporta decodificación por hardware de AV1, conmuta transparentemente a H.265 (HEVC) o VP9 de baja latencia con aceleración MediaCodec NDK."*

### Prompt 3.2: Passthrough Completo de Sensores y Dispositivos de Entrada
> **Prompt:**  
> *"Sincroniza la telemetría de hardware (acelerómetro, giroscopio, sensores de presión, batería y coordenadas GPS passthrough) mediante mensajes CBOR de alta frecuencia (120 Hz) inyectados directamente a los drivers virtuales virtio-input de la máquina virtual invitada."*

---

## 📦 4. INSTRUCCIONES PARA DESPLIEGUE Y SUBIDA A REPOSITORIO DE GITHUB

Para sincronizar y subir los últimos avances al repositorio de GitHub:

```bash
# 1. Verificar estado de cambios locales
git status

# 2. Agregar archivos de funcionalidades, UI y documentación
git add .

# 3. Registrar el commit con resumen de cambios
git commit -m "feat(vault-android): incorporar monitor de rendimiento VM, telemetría de sensores, controles de streaming, atestación de seguridad, orquestador de clúster y prompts de escalabilidad"

# 4. Enviar cambios a la rama principal en GitHub
git push origin main
```
