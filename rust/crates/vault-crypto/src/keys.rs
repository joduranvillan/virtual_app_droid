//! Derivación de material de clave a partir del handshake Noise.
//!
//! Idea central (ver ARCHITECTURE.md §3): la clave de desbloqueo del
//! volumen LUKS2 nunca viaja por la red ni se guarda en el host. Se
//! deriva localmente en cada extremo combinando:
//!
//! - el `handshake_hash` de la sesión Noise ya autenticada (liga la clave
//!   a esta sesión específica, con las identidades ya verificadas),
//! - un secreto de dispositivo que en el frontend real vive en
//!   StrongBox/Keystore y nunca sale de ahí en claro.
//!
//! Como ambos extremos calculan el mismo `handshake_hash`, y el frontend
//! aporta el secreto de dispositivo por un canal ya autenticado (dentro
//! del payload cifrado de transporte, no en claro), el host nunca ve el
//! secreto de dispositivo ni la clave resultante fuera de la memoria
//! efímera de `vault_runtime`.

use hkdf::Hkdf;
use sha2::Sha256;

pub const VAULT_UNLOCK_KEY_LEN: usize = 32; // apto para LUKS2 / XChaCha20

/// Deriva la clave de desbloqueo del vault.
///
/// `handshake_hash`: salida de `VaultHandshake::handshake_hash()`.
/// `device_master_secret`: secreto persistente del dispositivo (ideal:
///   respaldado por StrongBox), nunca debería loggearse ni serializarse
///   fuera de este proceso.
pub fn derive_vault_unlock_key(
    handshake_hash: &[u8],
    device_master_secret: &[u8],
) -> [u8; VAULT_UNLOCK_KEY_LEN] {
    let hk = Hkdf::<Sha256>::new(Some(handshake_hash), device_master_secret);
    let mut okm = [0u8; VAULT_UNLOCK_KEY_LEN];
    hk.expand(b"vault-unlock-key-v1", &mut okm)
        .expect("largo de salida válido para HKDF-SHA256");
    okm
}

/// Deriva una subclave de aplicación separada de la de desbloqueo, para
/// no reutilizar la misma clave con dos propósitos distintos (ej. cifrar
/// metadatos de sesión, no el volumen). Mismo `handshake_hash`, distinto
/// `info` label.
pub fn derive_subkey(handshake_hash: &[u8], device_master_secret: &[u8], label: &[u8]) -> [u8; 32] {
    let hk = Hkdf::<Sha256>::new(Some(handshake_hash), device_master_secret);
    let mut okm = [0u8; 32];
    hk.expand(label, &mut okm)
        .expect("largo de salida válido para HKDF-SHA256");
    okm
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_inputs_give_same_key_deterministically() {
        let hh = [1u8; 32];
        let secret = [2u8; 32];
        let k1 = derive_vault_unlock_key(&hh, &secret);
        let k2 = derive_vault_unlock_key(&hh, &secret);
        assert_eq!(k1, k2);
    }

    #[test]
    fn different_handshake_hash_gives_different_key() {
        let secret = [2u8; 32];
        let k1 = derive_vault_unlock_key(&[1u8; 32], &secret);
        let k2 = derive_vault_unlock_key(&[9u8; 32], &secret);
        assert_ne!(k1, k2);
    }

    #[test]
    fn unlock_key_and_subkey_differ() {
        let hh = [3u8; 32];
        let secret = [4u8; 32];
        let unlock = derive_vault_unlock_key(&hh, &secret);
        let sub = derive_subkey(&hh, &secret, b"session-metadata-v1");
        assert_ne!(unlock.to_vec(), sub.to_vec());
    }
}
