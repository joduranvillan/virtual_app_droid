//! Tokens de un solo uso para el flujo de enrolamiento por QR.
//! No son secretos criptográficos de largo plazo — solo prueban que
//! quien los presenta vio el QR que se mostró en la ventana de tiempo
//! correcta (mitigan reuso/replay del QR, no reemplazan la autenticación
//! real que da el handshake Noise sobre las claves estáticas).

use rand::RngCore;

pub fn generate_pairing_token() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    hex::encode(bytes)
}

pub fn now_unix_ms() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("el reloj del sistema está antes de 1970")
        .as_millis() as u64
}
