//! Tipos del flujo de enrolamiento (pairing) por QR.
//!
//! Modelo de confianza: es un TOFU (trust-on-first-use) igual al de
//! WhatsApp Web o el pairing de Signal Desktop — el QR viaja por un
//! canal que asumimos controlado físicamente por el dueño (la LAN local
//! + su propia pantalla), y esa es la raíz de confianza inicial. No hay
//! forma de tener "zero trust" en el primerísimo pairing sin una PKI
//! externa; lo que sí se garantiza es que, una vez pineadas las claves,
//! ningún tercero puede suplantar a ninguna de las dos partes después.

use serde::{Deserialize, Serialize};

/// Contenido exacto que se codifica en el QR, como JSON plano (no CBOR:
/// así es trivialmente debuggeable si alguien necesita pegarlo a mano
/// como fallback cuando escanear no es posible).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrollmentQrPayload {
    /// Versión del formato del QR, para poder evolucionarlo sin romper
    /// apps viejas contra runtimes nuevos o viceversa.
    pub v: u8,
    /// Clave pública estática X25519 de `vault_runtime`, codificada en
    /// hexadecimal (NO base64 — se usa hex en todo el proyecto para no
    /// sumar una dependencia extra solo para esto).
    pub runtime_pubkey_hex: String,
    pub host: String,
    pub port: u16,
    pub token: String,
    pub expires_unix_ms: u64,
}

pub const QR_PAYLOAD_VERSION: u8 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrollmentConfirmBody {
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnrollmentAckBody {
    pub success: bool,
    pub reason: Option<String>,
}
