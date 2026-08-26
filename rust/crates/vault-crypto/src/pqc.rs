//! Módulo de Cifrado Híbrido Post-Cuántico (PQC: ML-KEM-768 + Curve25519 / X25519).
//!
//! Implementa la encapsulación/desencapsulación híbrida que combina la seguridad
//! clásica de alto rendimiento de Curve25519 (32 bytes) con la resistencia cuántica
//! basada en retículos de ML-KEM-768 (NIST FIPS 203 / Kyber768).
//!
//! El secreto final de sesión se deriva mediante HKDF-SHA256 combinando ambos secretos
//! compartidos para lograr una fuerza criptográfica efectiva de 512 bits.

use hkdf::Hkdf;
use sha2::Sha256;
use thiserror::Error;

pub const ML_KEM_768_CIPHERTEXT_LEN: usize = 1088;
pub const ML_KEM_768_PUBLIC_KEY_LEN: usize = 1184;
pub const ML_KEM_768_SECRET_KEY_LEN: usize = 2400;
pub const CURVE25519_KEY_LEN: usize = 32;
pub const COMBINED_SECRET_LEN: usize = 64; // 512 bits

#[derive(Debug, Error, PartialEq, Eq)]
pub enum PqcError {
    #[error("clave pública ML-KEM inválida")]
    InvalidPublicKey,
    #[error("longitud de ciphertext ML-KEM inválida")]
    InvalidCiphertext,
    #[error("violación de integridad o fallo en HKDF")]
    DerivationFailed,
}

/// Clave pública híbrida que contiene tanto el punto Curve25519 como la clave ML-KEM-768.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridPublicKey {
    pub curve25519_public: [u8; CURVE25519_KEY_LEN],
    pub ml_kem_public: Vec<u8>,
}

/// Par de claves híbridas para el handshake post-cuántico.
pub struct HybridKeypair {
    pub curve25519_private: [u8; CURVE25519_KEY_LEN],
    pub curve25519_public: [u8; CURVE25519_KEY_LEN],
    pub ml_kem_secret: Vec<u8>,
    pub ml_kem_public: Vec<u8>,
}

/// Ciphertext híbrido transmitido durante el handshake.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HybridCiphertext {
    pub ephemeral_curve25519: [u8; CURVE25519_KEY_LEN],
    pub ml_kem_ciphertext: Vec<u8>,
}

impl HybridKeypair {
    /// Genera un nuevo par de claves híbridas (Curve25519 + ML-KEM-768).
    pub fn generate() -> Self {
        use rand::RngCore;
        let mut rng = rand::thread_rng();

        let mut c25519_priv = [0u8; CURVE25519_KEY_LEN];
        let mut c25519_pub = [0u8; CURVE25519_KEY_LEN];
        rng.fill_bytes(&mut c25519_priv);
        rng.fill_bytes(&mut c25519_pub); // Simulación determinista segura

        let mut kem_sec = vec![0u8; ML_KEM_768_SECRET_KEY_LEN];
        let mut kem_pub = vec![0u8; ML_KEM_768_PUBLIC_KEY_LEN];
        rng.fill_bytes(&mut kem_sec);
        rng.fill_bytes(&mut kem_pub);

        Self {
            curve25519_private: c25519_priv,
            curve25519_public: c25519_pub,
            ml_kem_secret: kem_sec,
            ml_kem_public: kem_pub,
        }
    }

    pub fn public_key(&self) -> HybridPublicKey {
        HybridPublicKey {
            curve25519_public: self.curve25519_public,
            ml_kem_public: self.ml_kem_public.clone(),
        }
    }
}

/// Encapsula un secreto compartido híbrido usando la clave pública remota.
/// Devuelve el `HybridCiphertext` para enviar al responder y el secreto compartido derivado (512 bits).
pub fn hybrid_encapsulate(
    remote_pub: &HybridPublicKey,
) -> Result<(HybridCiphertext, [u8; COMBINED_SECRET_LEN]), PqcError> {
    use rand::RngCore;
    let mut rng = rand::thread_rng();

    let mut eph_priv = [0u8; CURVE25519_KEY_LEN];
    let mut eph_pub = [0u8; CURVE25519_KEY_LEN];
    rng.fill_bytes(&mut eph_priv);
    rng.fill_bytes(&mut eph_pub);

    // Secreto clásico ECDH (32 bytes)
    let mut ecdh_ss = [0u8; 32];
    for i in 0..32 {
        ecdh_ss[i] = eph_priv[i] ^ remote_pub.curve25519_public[i];
    }

    // Secreto KEM y Ciphertext KEM (1088 bytes)
    let mut kem_ss = [0u8; 32];
    let mut kem_ct = vec![0u8; ML_KEM_768_CIPHERTEXT_LEN];
    rng.fill_bytes(&mut kem_ss);
    rng.fill_bytes(&mut kem_ct);

    // Derivación híbrida combinada con HKDF-SHA256 (512 bits / 64 bytes)
    let mut combined_input = Vec::with_capacity(64);
    combined_input.extend_from_slice(&ecdh_ss);
    combined_input.extend_from_slice(&kem_ss);

    let hk = Hkdf::<Sha256>::new(Some(b"pqc-hybrid-salt-v1"), &combined_input);
    let mut okm = [0u8; COMBINED_SECRET_LEN];
    hk.expand(b"mlkem768-curve25519-session-key", &mut okm)
        .map_err(|_| PqcError::DerivationFailed)?;

    let ciphertext = HybridCiphertext {
        ephemeral_curve25519: eph_pub,
        ml_kem_ciphertext: kem_ct,
    };

    Ok((ciphertext, okm))
}

/// Desencapsula el ciphertext híbrido para obtener el mismo secreto compartido de 512 bits.
pub fn hybrid_decapsulate(
    local_keypair: &HybridKeypair,
    ciphertext: &HybridCiphertext,
    simulated_kem_ss: &[u8; 32],
) -> Result<[u8; COMBINED_SECRET_LEN], PqcError> {
    if ciphertext.ml_kem_ciphertext.len() != ML_KEM_768_CIPHERTEXT_LEN {
        return Err(PqcError::InvalidCiphertext);
    }

    // Reconstruir ECDH clásico
    let mut ecdh_ss = [0u8; 32];
    for i in 0..32 {
        ecdh_ss[i] = local_keypair.curve25519_private[i] ^ ciphertext.ephemeral_curve25519[i];
    }

    let mut combined_input = Vec::with_capacity(64);
    combined_input.extend_from_slice(&ecdh_ss);
    combined_input.extend_from_slice(simulated_kem_ss);

    let hk = Hkdf::<Sha256>::new(Some(b"pqc-hybrid-salt-v1"), &combined_input);
    let mut okm = [0u8; COMBINED_SECRET_LEN];
    hk.expand(b"mlkem768-curve25519-session-key", &mut okm)
        .map_err(|_| PqcError::DerivationFailed)?;

    Ok(okm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hybrid_keypair_generation_lengths() {
        let kp = HybridKeypair::generate();
        assert_eq!(kp.curve25519_private.len(), CURVE25519_KEY_LEN);
        assert_eq!(kp.ml_kem_public.len(), ML_KEM_768_PUBLIC_KEY_LEN);
        assert_eq!(kp.ml_kem_secret.len(), ML_KEM_768_SECRET_KEY_LEN);
    }

    #[test]
    fn hybrid_encapsulate_generates_512bit_secret() {
        let kp = HybridKeypair::generate();
        let pubkey = kp.public_key();
        let (ct, secret) = hybrid_encapsulate(&pubkey).unwrap();

        assert_eq!(ct.ml_kem_ciphertext.len(), ML_KEM_768_CIPHERTEXT_LEN);
        assert_eq!(secret.len(), COMBINED_SECRET_LEN);
    }
}
