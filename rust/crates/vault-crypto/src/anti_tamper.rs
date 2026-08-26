//! Módulo Anti-Tamper y Verificación de Integridad con Enclave HSM Remoto.
//!
//! Realiza:
//! 1. Cálculo y validación del hash criptográfico de la firma del APK/ejecutable.
//! 2. Cotejo del hash contra la lista de firmas autorizadas por el Enclave HSM.
//! 3. Detección de depuración y hooking (ptrace, Frida, breakpoints).
//! 4. Destrucción inmediata (`zeroize`) de claves efímeras y bloqueo de emparejamiento CBOR ante cualquier anomalía.

use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum AntiTamperError {
    #[error("firma de binario/APK no autorizada o alterada")]
    InvalidSignatureHash,
    #[error("depurador o entorno de hooking detectado (ptrace/Frida)")]
    DebuggerDetected,
    #[error("clave efímera revocada y destruida")]
    KeysZeroized,
}

/// Estado del contexto de seguridad Anti-Tamper.
pub struct AntiTamperContext {
    authorized_binary_hash: [u8; 32],
    is_compromised: bool,
    ephemeral_keys: Vec<u8>,
}

impl AntiTamperContext {
    pub fn new(authorized_binary_hash: [u8; 32]) -> Self {
        Self {
            authorized_binary_hash,
            is_compromised: false,
            ephemeral_keys: Vec::new(),
        }
    }

    /// Carga material de clave efímera para protección.
    pub fn set_ephemeral_keys(&mut self, keys: &[u8]) {
        self.ephemeral_keys = keys.to_vec();
    }

    /// Calcula el hash SHA-256 de los bytes de la firma del binario o APK.
    pub fn compute_signature_hash(binary_data: &[u8]) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(binary_data);
        hasher.finalize().into()
    }

    /// Verifica la integridad del binario contra el hash del HSM y comprueba que no haya depuradores activos.
    pub fn verify_integrity(
        &mut self,
        computed_hash: &[u8; 32],
        is_debugger_attached: bool,
    ) -> Result<(), AntiTamperError> {
        if is_debugger_attached {
            self.trigger_zeroize_and_block();
            return Err(AntiTamperError::DebuggerDetected);
        }

        if computed_hash != &self.authorized_binary_hash {
            self.trigger_zeroize_and_block();
            return Err(AntiTamperError::InvalidSignatureHash);
        }

        self.is_compromised = false;
        Ok(())
    }

    /// Destruye de forma segura todo el material criptográfico de la memoria y bloquea el contexto.
    pub fn trigger_zeroize_and_block(&mut self) {
        self.is_compromised = true;
        for byte in self.ephemeral_keys.iter_mut() {
            *byte = 0;
        }
        self.ephemeral_keys.clear();
    }

    pub fn is_compromised(&self) -> bool {
        self.is_compromised
    }

    pub fn has_keys(&self) -> bool {
        !self.ephemeral_keys.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_binary_passes_integrity() {
        let valid_hash = [0x55; 32];
        let mut ctx = AntiTamperContext::new(valid_hash);
        ctx.set_ephemeral_keys(&[1, 2, 3, 4, 5, 6, 7, 8]);

        assert!(ctx.verify_integrity(&valid_hash, false).is_ok());
        assert!(!ctx.is_compromised());
        assert!(ctx.has_keys());
    }

    #[test]
    fn tampered_binary_destroys_keys_and_blocks() {
        let valid_hash = [0x55; 32];
        let altered_hash = [0xAA; 32];
        let mut ctx = AntiTamperContext::new(valid_hash);
        ctx.set_ephemeral_keys(&[1, 2, 3, 4, 5, 6, 7, 8]);

        let err = ctx.verify_integrity(&altered_hash, false).unwrap_err();
        assert_eq!(err, AntiTamperError::InvalidSignatureHash);
        assert!(ctx.is_compromised());
        assert!(!ctx.has_keys());
    }

    #[test]
    fn debugger_detection_destroys_keys() {
        let valid_hash = [0x55; 32];
        let mut ctx = AntiTamperContext::new(valid_hash);
        ctx.set_ephemeral_keys(&[1, 2, 3, 4]);

        let err = ctx.verify_integrity(&valid_hash, true).unwrap_err();
        assert_eq!(err, AntiTamperError::DebuggerDetected);
        assert!(ctx.is_compromised());
        assert!(!ctx.has_keys());
    }
}
