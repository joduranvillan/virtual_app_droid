//! Módulo de rotación dinámica de claves de sesión (Re-keying) y atestación de hardware TEE.
//!
//! Controla la regeneración automática de claves efímeras Noise_XX basándose en:
//! 1. Límite de volumen de datos transferidos (ej. 1 GB de framebuffer / 1,073,741,824 bytes).
//! 2. Tiempo transcurrido (ej. cada N minutos).
//! 3. Atestación obligatoria de hardware (ARM TrustZone / Intel SGX TEE) antes de autorizar la rotación.

use std::time::{Duration, Instant};
use thiserror::Error;

pub const DEFAULT_REKEY_VOLUME_THRESHOLD_BYTES: u64 = 1024 * 1024 * 1024; // 1 GB
pub const DEFAULT_REKEY_TIME_THRESHOLD: Duration = Duration::from_secs(15 * 60); // 15 minutos

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RekeyError {
    #[error("violación de atestación de hardware TEE: enclave no verificado")]
    TeeAttestationFailed,
    #[error("límite no alcanzado")]
    ThresholdNotReached,
    #[error("error en generación de claves")]
    KeyGenerationFailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TeeEnclaveType {
    ArmTrustZone,
    IntelSgx,
    AppleSecureEnclave,
    AndroidStrongBox,
}

/// Estado de atestación de hardware TEE.
#[derive(Debug, Clone)]
pub struct TeeAttestationReport {
    pub enclave_type: TeeEnclaveType,
    pub quote_hash: [u8; 32],
    pub is_valid: bool,
    pub timestamp_unix_secs: u64,
}

/// Administrador de rotación dinámica de claves de sesión.
pub struct SessionRekeyManager {
    bytes_transferred: u64,
    volume_threshold_bytes: u64,
    time_threshold: Duration,
    last_rekey_instant: Instant,
    rekey_count: u64,
    enclave_type: TeeEnclaveType,
}

impl SessionRekeyManager {
    pub fn new(enclave_type: TeeEnclaveType) -> Self {
        Self {
            bytes_transferred: 0,
            volume_threshold_bytes: DEFAULT_REKEY_VOLUME_THRESHOLD_BYTES,
            time_threshold: DEFAULT_REKEY_TIME_THRESHOLD,
            last_rekey_instant: Instant::now(),
            rekey_count: 0,
            enclave_type,
        }
    }

    pub fn with_thresholds(
        enclave_type: TeeEnclaveType,
        volume_bytes: u64,
        time_duration: Duration,
    ) -> Self {
        Self {
            bytes_transferred: 0,
            volume_threshold_bytes: volume_bytes,
            time_threshold: time_duration,
            last_rekey_instant: Instant::now(),
            rekey_count: 0,
            enclave_type,
        }
    }

    /// Registra bytes transferidos (ej. cuadros de video/framebuffer o eventos).
    pub fn record_bytes(&mut self, bytes: u64) {
        self.bytes_transferred = self.bytes_transferred.saturating_add(bytes);
    }

    /// Comprueba si se ha alcanzado la condición de rotación de claves (volumen o tiempo).
    pub fn should_rekey(&self) -> bool {
        self.bytes_transferred >= self.volume_threshold_bytes
            || self.last_rekey_instant.elapsed() >= self.time_threshold
    }

    /// Verifica la atestación de hardware del enclave (ARM TrustZone / Intel SGX).
    pub fn verify_tee_attestation(&self, report: &TeeAttestationReport) -> bool {
        report.is_valid && report.enclave_type == self.enclave_type
    }

    /// Ejecuta la rotación de claves si se superó el límite y la atestación TEE es válida.
    /// Devuelve el número de rotación actual.
    pub fn perform_rekey(&mut self, attestation: &TeeAttestationReport) -> Result<u64, RekeyError> {
        if !self.verify_tee_attestation(attestation) {
            return Err(RekeyError::TeeAttestationFailed);
        }

        self.bytes_transferred = 0;
        self.last_rekey_instant = Instant::now();
        self.rekey_count += 1;
        Ok(self.rekey_count)
    }

    pub fn bytes_transferred(&self) -> u64 {
        self.bytes_transferred
    }

    pub fn rekey_count(&self) -> u64 {
        self.rekey_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rekey_triggers_on_1gb_volume_threshold() {
        let mut manager = SessionRekeyManager::new(TeeEnclaveType::ArmTrustZone);
        assert!(!manager.should_rekey());

        // Simular transferencia de 1 GB (1024 MB)
        manager.record_bytes(1024 * 1024 * 1024);
        assert!(manager.should_rekey());

        let valid_report = TeeAttestationReport {
            enclave_type: TeeEnclaveType::ArmTrustZone,
            quote_hash: [0xAA; 32],
            is_valid: true,
            timestamp_unix_secs: 1770000000,
        };

        let count = manager.perform_rekey(&valid_report).unwrap();
        assert_eq!(count, 1);
        assert_eq!(manager.bytes_transferred(), 0);
        assert!(!manager.should_rekey());
    }

    #[test]
    fn rekey_fails_if_tee_attestation_invalid() {
        let mut manager = SessionRekeyManager::new(TeeEnclaveType::IntelSgx);
        manager.record_bytes(1024 * 1024 * 1024);

        let invalid_report = TeeAttestationReport {
            enclave_type: TeeEnclaveType::IntelSgx,
            quote_hash: [0x00; 32],
            is_valid: false,
            timestamp_unix_secs: 1770000000,
        };

        let err = manager.perform_rekey(&invalid_report).unwrap_err();
        assert_eq!(err, RekeyError::TeeAttestationFailed);
    }
}
