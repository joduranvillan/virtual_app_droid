# Documentación de Fase B: Streaming de Video/Audio e Inyección de Input (Espejo de vault-stream)

La **Fase B (Opción B)** se centra en establecer el contrato de datos binario y los flujos de comunicación bidireccional en tiempo real para la transmisión de pantalla, audio y la inyección remota de eventos de teclado y gestos táctiles.

---

## 1. Objetivos de Diseño de la Fase B

### Espejo Binario Multiplataforma (`vault-stream` <-> Android Client)
Se ha implementado en la aplicación de Android la misma especificación de serialización binaria utilizando **CBOR (Concise Binary Object Representation)** a través de Jackson, asegurando una compatibilidad nativa del 100% con los modelos definidos en Rust:
*   **`VideoFramePayload`**: Transporta el flujo de frames comprimidos (H.265 / H.264), banderas de fotograma clave (keyframe), metadatos de dimensiones y timestamps de alta precisión.
*   **`AudioFramePayload`**: Transporta muestras de audio comprimidas con Opus, con validación de frecuencias de muestreo estándar (8kHz - 48kHz) y canales estéreo/monoaurales.
*   **`InputEventPayload`**: Define la inyección de eventos interactivos mediante coordenadas normalizadas `[0.0, 1.0]`. Esto garantiza la independencia absoluta de la resolución física de la pantalla del cliente y del dispositivo remoto.

---

## 2. Inyección Interactiva de Eventos (Estructura de Datos)

El protocolo de inyección interactiva se compone de cuatro variantes principales transmitidas por canal cifrado seguro:

```
                  ┌──────────────────────────────────────────────┐
                  │              InputEventPayload               │
                  └──────┬────────────────────────────────┬──────┘
                         │                                │
        ┌────────────────▼────────────────┐      ┌────────▼────────────────────────┐
        │       Eventos Táctiles          │      │       Eventos de Teclado        │
        │  (TouchDown, TouchMove, TouchUp)│      │             (Key)               │
        └─────────────────────────────────┘      └─────────────────────────────────┘
```

1.  **TouchDown**: Registra el inicio de un toque. Envía `pointer_id` (para soporte multitáctil), coordenadas normalizadas `x` e `y`, y el timestamp Unix en milisegundos.
2.  **TouchMove**: Transmite el arrastre continuo en el touchpad con coordenadas normalizadas en tiempo real.
3.  **TouchUp**: Notifica la liberación del toque, cerrando el ciclo de interacción de un pointer específico.
4.  **Key**: Envía códigos de teclas de hardware (`keycode`) con su respectivo estado físico (presionado o liberado).

---

## 3. Implementación Interactiva en la UI del Cliente Móvil

Se ha transformado el monitor estático en una **consola remota interactiva en tiempo real**:

*   **Touchpad Dinámico Normalizado**: Al tocar o arrastrar el dedo sobre el monitor remoto de la app, el sistema calcula de forma dinámica las coordenadas relativas del canvas y transmite inmediatamente eventos `TouchDown` / `TouchMove` / `TouchUp` codificados en CBOR sobre la conexión activa.
*   **Consola HUD del Stream**: Muestra en tiempo real estadísticas avanzadas como la resolución simulada, tasa de refresco, códec activo, identificador de secuencia, y latencia estimada.
*   **Botonera de Teclas Físicas**: Añade soporte directo para inyectar eventos de botones de hardware esenciales de Linux/Android (`POWER`, `BACK`, `HOME`, `APPS`) utilizando sus códigos de tecla estándar de la API de Linux (por ejemplo, `KEY_POWER = 116`, `KEY_BACK = 158`, etc.).
*   **Simulador de Framebuffer H.265**: Integra un generador de tramas virtuales con progresión secuencial activa para facilitar pruebas dinámicas del pipeline sin depender de un entorno KVM físico en el sandbox.
*   **Log de Consola de Eventos Transmitidos**: Un visor tipo terminal que muestra en tiempo real los bytes transmitidos por el cable para auditar visualmente cada toque y pulsación de tecla inyectada.
