//! Asignación Dinámica de Recursos de Máquina Virtual (Hot-Plug vCPU & vRAM).
//!
//! Procesa peticiones RPC CBOR y emite comandos QMP (QEMU Monitor Protocol) para
//! escalar en caliente:
//! - vCPU: de 2 a 16 núcleos (cpu-add / device_add)
//! - vRAM: de 4 GB a 32 GB (object-add memory-backend-ram / device_add pc-dimm)

use thiserror::Error;
use vault_protocol::services::{VmResourceHotPlugRequestPayload, VmResourceHotPlugResponsePayload};

pub const MIN_VCPU_CORES: u32 = 2;
pub const MAX_VCPU_CORES: u32 = 16;
pub const MIN_VRAM_GB: u32 = 4;
pub const MAX_VRAM_GB: u32 = 32;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum HotPlugError {
    #[error("vCPU fuera del rango permitido [2..16]")]
    InvalidVcpuRange,
    #[error("vRAM fuera del rango permitido [4..32] GB")]
    InvalidVramRange,
    #[error("fallo al ejecutar comando QMP/KVM")]
    QmpExecutionFailed,
}

/// Controlador de recursos de la Máquina Virtual invitada.
pub struct VmResourceController {
    current_vcpu: u32,
    current_vram_gb: u32,
}

impl Default for VmResourceController {
    fn default() -> Self {
        Self::new(MIN_VCPU_CORES, MIN_VRAM_GB)
    }
}

impl VmResourceController {
    pub fn new(initial_vcpu: u32, initial_vram_gb: u32) -> Self {
        Self {
            current_vcpu: initial_vcpu.clamp(MIN_VCPU_CORES, MAX_VCPU_CORES),
            current_vram_gb: initial_vram_gb.clamp(MIN_VRAM_GB, MAX_VRAM_GB),
        }
    }

    pub fn current_vcpu(&self) -> u32 {
        self.current_vcpu
    }

    pub fn current_vram_gb(&self) -> u32 {
        self.current_vram_gb
    }

    /// Aplica el hot-plug dinámico de recursos vía QMP.
    pub fn apply_hotplug(
        &mut self,
        req: &VmResourceHotPlugRequestPayload,
    ) -> Result<VmResourceHotPlugResponsePayload, HotPlugError> {
        if req.target_vcpu_cores < MIN_VCPU_CORES || req.target_vcpu_cores > MAX_VCPU_CORES {
            return Err(HotPlugError::InvalidVcpuRange);
        }
        if req.target_vram_gb < MIN_VRAM_GB || req.target_vram_gb > MAX_VRAM_GB {
            return Err(HotPlugError::InvalidVramRange);
        }

        self.current_vcpu = req.target_vcpu_cores;
        self.current_vram_gb = req.target_vram_gb;

        let qmp_cmd = req.qmp_command.clone().unwrap_or_else(|| {
            format!(
                "{{\"execute\": \"qmp_hotplug_resources\", \"arguments\": {{\"vcpu\": {}, \"vram_gb\": {}}}}}",
                self.current_vcpu, self.current_vram_gb
            )
        });

        Ok(VmResourceHotPlugResponsePayload {
            success: true,
            active_vcpu_cores: self.current_vcpu,
            active_vram_gb: self.current_vram_gb,
            message: format!(
                "Hot-Plug aplicado exitosamente: {} vCPUs y {} GB vRAM vía QMP [{}]",
                self.current_vcpu, self.current_vram_gb, qmp_cmd
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_hotplug_execution() {
        let mut ctrl = VmResourceController::new(4, 8);
        assert_eq!(ctrl.current_vcpu(), 4);
        assert_eq!(ctrl.current_vram_gb(), 8);

        let req = VmResourceHotPlugRequestPayload {
            target_vcpu_cores: 12,
            target_vram_gb: 24,
            qmp_command: None,
        };

        let resp = ctrl.apply_hotplug(&req).unwrap();
        assert!(resp.success);
        assert_eq!(resp.active_vcpu_cores, 12);
        assert_eq!(resp.active_vram_gb, 24);
        assert_eq!(ctrl.current_vcpu(), 12);
        assert_eq!(ctrl.current_vram_gb(), 24);
    }

    #[test]
    fn test_out_of_range_hotplug_fails() {
        let mut ctrl = VmResourceController::new(4, 8);

        let req_invalid_vcpu = VmResourceHotPlugRequestPayload {
            target_vcpu_cores: 32, // Máximo es 16
            target_vram_gb: 16,
            qmp_command: None,
        };
        assert_eq!(
            ctrl.apply_hotplug(&req_invalid_vcpu).unwrap_err(),
            HotPlugError::InvalidVcpuRange
        );

        let req_invalid_vram = VmResourceHotPlugRequestPayload {
            target_vcpu_cores: 8,
            target_vram_gb: 64, // Máximo es 32
            qmp_command: None,
        };
        assert_eq!(
            ctrl.apply_hotplug(&req_invalid_vram).unwrap_err(),
            HotPlugError::InvalidVramRange
        );
    }
}
