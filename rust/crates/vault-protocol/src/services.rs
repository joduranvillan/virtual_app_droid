//! Tipos de request/response para cada servicio virtual expuesto por el
//! frontend Android y consumido desde dentro de la bóveda.
//!
//! Para agregar un servicio nuevo (cámara, NFC, biometría, portapapeles...):
//! 1. Agregar variante a `ServiceId`.
//! 2. Definir su par Request/Response con `#[derive(Serialize, Deserialize)]`.
//! 3. Implementar el handler en el frontend Kotlin (ver `services/` allá).
//! 4. Implementar el cliente Rust en `vault_runtime` que arma el `Frame`
//!    con `MsgType::ServiceRequest` y este `ServiceId`.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ServiceId {
    Location,
    Camera,
    Nfc,
    Biometrics,
    Clipboard,
    Notifications,
    Accelerometer,
    Admin,
    ClusterMigration,
    VmResourceHotPlug,
    CodecNegotiation,
    SensorVirtioPassthrough,
}

/// Envoltorio genérico de un pedido de servicio: identifica qué servicio
/// se está invocando y lleva el cuerpo específico serializado aparte
/// (en el payload CBOR del `Frame`, junto a este header).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceRequestEnvelope {
    pub service: ServiceId,
    /// Nombre del app que originó el pedido dentro del Android Runtime,
    /// usado por el frontend para decidir políticas de permisos por app.
    pub requesting_package: String,
    /// CBOR del tipo específico del servicio. `serde_bytes` fuerza a que
    /// esto viaje como byte-string CBOR real (major type 2) en lugar del
    /// array de enteros que produce un `Vec<u8>` sin anotar — importante
    /// para que el decoder Kotlin (Jackson CBOR) lo lea sin ambigüedad.
    #[serde(with = "serde_bytes")]
    pub body: Vec<u8>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceResponseEnvelope {
    pub service: ServiceId,
    pub result: ServiceResult,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ServiceResult {
    Ok(#[serde(with = "serde_bytes")] Vec<u8>), // CBOR del tipo específico del servicio
    PermissionDenied,
    Unavailable,
    Error(String),
}

// ---------------------------------------------------------------------
// VirtualLocationService — implementado end-to-end en este entregable
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationRequest {
    /// "coarse" o "fine", refleja el permiso Android original que pidió la app.
    pub accuracy: LocationAccuracy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LocationAccuracy {
    Coarse,
    Fine,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationResponse {
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy_meters: f32,
    pub timestamp_unix_ms: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn location_request_roundtrips_through_cbor() {
        let req = LocationRequest {
            accuracy: LocationAccuracy::Fine,
        };
        let bytes = serde_cbor::to_vec(&req).unwrap();
        let back: LocationRequest = serde_cbor::from_slice(&bytes).unwrap();
        assert_eq!(back.accuracy, LocationAccuracy::Fine);
    }

    #[test]
    fn admin_request_roundtrips_through_cbor() {
        let req = AdminRequestPayload {
            action: AdminAction::ChangeNetwork,
            target_network: Some("192.168.1.100".to_string()),
            update_version: None,
        };
        let bytes = serde_cbor::to_vec(&req).unwrap();
        let back: AdminRequestPayload = serde_cbor::from_slice(&bytes).unwrap();
        assert_eq!(back, req);
    }

    #[test]
    fn admin_response_roundtrips_through_cbor() {
        let resp = AdminResponsePayload {
            success: true,
            message: "Logs retrieved successfully".to_string(),
            logs: vec!["[12:00] system initialized".to_string()],
        };
        let bytes = serde_cbor::to_vec(&resp).unwrap();
        let back: AdminResponsePayload = serde_cbor::from_slice(&bytes).unwrap();
        assert_eq!(back, resp);
    }
}

// ---------------------------------------------------------------------
// Administración Headless — Espejo de Fase E
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AdminAction {
    #[serde(rename = "RebootVault")]
    RebootVault,
    #[serde(rename = "GetLogs")]
    GetLogs,
    #[serde(rename = "ChangeNetwork")]
    ChangeNetwork,
    #[serde(rename = "FactoryReset")]
    FactoryReset,
    #[serde(rename = "UpdateRuntime")]
    UpdateRuntime,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdminRequestPayload {
    pub action: AdminAction,
    #[serde(rename = "target_network")]
    pub target_network: Option<String>,
    #[serde(rename = "update_version")]
    pub update_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdminResponsePayload {
    pub success: bool,
    pub message: String,
    pub logs: Vec<String>,
}

// ---------------------------------------------------------------------
// Orquestación Multi-Nodo & Migración en Caliente (Fase 2.1)
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClusterNodeType {
    HighPerfX86_64,
    AmpereArm64,
    EdgeMicroHost,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClusterNodeInfo {
    pub node_id: String,
    pub node_type: ClusterNodeType,
    pub ip_address: String,
    pub load_percent: f32,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveMigrationRequestPayload {
    pub session_id: String,
    pub source_node_id: String,
    pub target_node_id: String,
    pub preserve_framebuffer_state: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LiveMigrationResponsePayload {
    pub success: bool,
    pub downtime_ms: f32,
    pub transferred_dirty_pages_mb: f32,
    pub active_node_id: String,
    pub status_message: String,
}

// ---------------------------------------------------------------------
// Asignación Dinámica de Recursos VM (Hot-Plug vCPU & vRAM - Fase 2.2)
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VmResourceHotPlugRequestPayload {
    /// Número objetivo de núcleos virtuales (2 a 16 cores).
    pub target_vcpu_cores: u32,
    /// Memoria RAM objetivo en Gigabytes (4 a 32 GB).
    pub target_vram_gb: u32,
    /// Comando QMP/KVM subyacente para hotplug en tiempo de ejecución.
    pub qmp_command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct VmResourceHotPlugResponsePayload {
    pub success: bool,
    pub active_vcpu_cores: u32,
    pub active_vram_gb: u32,
    pub message: String,
}

// ---------------------------------------------------------------------
// Negociación Automática Multicodec con Fallback Hardware (Fase 3.1)
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoCodecType {
    Av1,
    HevcH265,
    Vp9ZeroLag,
    AvcH264,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodecNegotiationRequestPayload {
    pub supported_hardware_codecs: Vec<VideoCodecType>,
    pub preferred_codec: VideoCodecType,
    pub soc_model: String,
    pub mediacodec_ndk_acceleration: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CodecNegotiationResponsePayload {
    pub active_codec: VideoCodecType,
    pub is_hardware_accelerated: bool,
    pub decoder_mime: String,
    pub fallback_triggered: bool,
    pub reason: String,
}

// ---------------------------------------------------------------------
// Passthrough de Sensores a Virtio-Input (120 Hz) (Fase 3.2)
// ---------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GpsCoordinatesPayload {
    pub latitude: f64,
    pub longitude: f64,
    pub altitude_meters: f32,
    pub speed_kmh: f32,
    pub accuracy_meters: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BatteryTelemetryPayload {
    pub level_percent: u8,
    pub voltage_mv: u32,
    pub temperature_c: f32,
    pub is_charging: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct VirtioInputSensorPayload {
    pub timestamp_ns: u64,
    pub sample_rate_hz: u32, // 120 Hz
    pub accel_x: f32,
    pub accel_y: f32,
    pub accel_z: f32,
    pub gyro_yaw: f32,
    pub gyro_pitch: f32,
    pub gyro_roll: f32,
    pub pressure_hpa: f32, // Barómetro (ej. 1013.25)
    pub battery: BatteryTelemetryPayload,
    pub gps: GpsCoordinatesPayload,
    pub target_virtio_device: String, // ej. "/dev/input/event2"
}


