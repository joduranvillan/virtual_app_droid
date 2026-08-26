# FASE E: Documentación de Administración Headless (Remota)

Esta fase formaliza y detalla la arquitectura de **Administración Headless (Remota)** para el orquestador remoto de la Bóveda de Seguridad, permitiendo controlar el hipervisor (`crosvm` / `ARCVM`) de manera segura utilizando el canal criptográfico cifrado por Noise_XX con codificación binaria CBOR.

---

## 1. Arquitectura y Enrutamiento de Mensajes

La administración se integra dentro del motor unificado de `VaultConnectionManager`, utilizando dos niveles de enrutamiento:

1. **Mensajes Directos del Canal (mTLS/mNoise):**
   Para operaciones asíncronas de control global, se introdujeron nuevos códigos de trama en el encabezado de red de bajo nivel (`WireFraming.kt`):
   - `ADMIN_REQUEST (0x50)`: Tramas salientes enviadas por el cliente Android solicitando acciones específicas al hipervisor.
   - `ADMIN_RESPONSE (0x51)`: Respuestas asíncronas devueltas por el hipervisor con el estado de la operación y registros.

2. **Servicio RPC Registrado (`ServiceId.ADMIN`):**
   Para peticiones síncronas o de estado dentro del esquema genérico de despachadores de servicios, se registró el servicio `ADMIN` con valor único de cadena `"Admin"` en `VaultTypes.kt`.
   El manejador `AdminService.kt` actúa como el receptor e inyector de telemetría de estas peticiones.

---

## 2. Estructura del Contrato de Datos (CBOR)

Los payloads se codifican en binario CBOR a través de Jackson, manteniendo una compatibilidad estricta con la deserialización de estructuras Rust (`serde(rename_all = "snake_case")`).

### A. Acciones Administrativas (`AdminActionType`)
Define el catálogo de operaciones soportadas por el hipervisor remoto:
- `RebootVault`: Reinicio ordenado de la máquina virtual y servicios de la bóveda.
- `GetLogs`: Extracción de registros detallados de los componentes (`crosvm`, `cryptsetup`, orquestador core).
- `ChangeNetwork`: Modificación y reaplicación de interfaces de puente y políticas de red virtuales de ARCVM.
- `FactoryReset`: Proceso crítico que destruye las cabeceras de volumen LUKS, revoca llaves Noise estáticas y desenlaza el dispositivo.
- `UpdateRuntime`: Actualización y verificación de integridad criptográfica de la imagen del firmware.

### B. Payload de Petición (`AdminRequestPayload`)
Estructura serializada para solicitar acciones:
```json
{
  "action": "RebootVault",
  "target_network": "192.168.120.10/24 (VLAN 10)", // Opcional (para ChangeNetwork)
  "update_version": "v1.2.0-stable"                 // Opcional (para UpdateRuntime)
}
```

### C. Payload de Respuesta (`AdminResponsePayload`)
Respuesta con el resultado de la operación:
```json
{
  "success": true,
  "message": "Actualización de firmware completada con éxito",
  "logs": [
    "[INFO] Recibido UpdateRuntime comando",
    "[INFO] Descargando imagen de runtime v1.2.0-stable...",
    "[INFO] Verificando hashes SHA256 e integridad...",
    "[INFO] Imagen montada como activa"
  ]
}
```

---

## 3. Implementación en el Cliente Android

### A. Capa de Servicio (`AdminService.kt`)
Es el receptor y simulador de telemetría remota. Procesa las solicitudes en formato binario CBOR y emite eventos en tiempo real con las respuestas del sistema.

### B. Gestión de Estado (`MainActivity.kt`)
Se crearon tres flujos observables (`StateFlow`) para conectar el ciclo asíncrono con la interfaz gráfica reactiva:
- `adminLogs`: Cola reactiva de logs que alimenta la terminal de texto integrada.
- `adminStatusMessage`: Banner temporal con estados descriptivos y animaciones de progreso.
- `adminActionInProgress`: Control de estados de carga y bloqueo de botones durante transiciones críticas.

### C. Consola de Administración UI (`AdministrativePanelCard`)
Un componente Material Design 3 de alta fidelidad que ofrece:
1. **Acciones Rápidas con un Solo Toque:** Botones dedicados con iconos descriptivos para obtener logs, reiniciar, cambiar la red, actualizar runtime o ejecutar un borrado total.
2. **Terminal de Logs Interactiva:** Un visor de terminal con tipografía monoespaciada, fondo ultra oscuro (`Obsidian Black`) y coloreado sintáctico de acuerdo al nivel del registro (`CRITICAL` / `ALERT` en carmesí, `RPC` en turquesa, `INFO` en verde cyber).
3. **Control de Flujo de Operaciones:** Desactivación de controles mientras se procesa un comando y visualización de barras de progreso integradas en el banner.
4. **Validación Biométrica Crítica:** Para acciones destructivas o de alta relevancia (como `Factory Reset` o cambios de red), se diseñó un cuadro de diálogo que simula un lector biométrico de huellas dactilares. Requiere validación física simulada antes de autorizar el envío de tramas cifradas.
5. **Enlace con Borrado Local:** Si se ejecuta y confirma un `Factory Reset`, el cliente ejecuta `resetPairing()`, eliminando de inmediato la clave privada del dispositivo, la clave pública pinned y reiniciando el estado de la aplicación a vinculación por código QR.

---

## 4. Flujo de Trabajo en Acción (Casos de Uso)

### Caso 1: Obtención de Logs
1. El usuario presiona **Ver Logs** en la consola.
2. Se envía un paquete `ADMIN_REQUEST` con la acción `GetLogs`.
3. El socket devuelve la respuesta en formato CBOR.
4. La terminal del hipervisor se actualiza en tiempo real mostrando los registros asíncronos del arranque de `crosvm` y los discos cifrados.

### Caso 2: Restablecimiento de Fábrica (Factory Reset)
1. El usuario presiona **Factory Reset**.
2. Aparece un prompt de seguridad con autenticación biométrica.
3. El usuario confirma presionando el sensor de huella.
4. Se envía la petición destructiva sobre la red y paralelamente se limpia la base de claves e identidad en las preferencias locales (`SharedPreferences`).
5. La interfaz hace una transición animada de vuelta a la pantalla de emparejamiento por cámara QR.
