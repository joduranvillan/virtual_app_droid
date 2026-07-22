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
