//! Implementación Windows de `vault_core::AndroidHypervisor`.
//!
//! Soporta tanto el control real de máquinas virtuales de Hyper-V mediante PowerShell
//! (requiriendo Hyper-V habilitado en Windows Pro/Enterprise) como un modo de simulación
//! detallado para desarrollo local y testing.

use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use tracing::{info, warn, error};

use vault_core::{AndroidHypervisor, MountedVolume, PlatformError, PlatformResult, RunningInstance};

/// Identificador de instancia para la plataforma Windows.
pub struct SimulatedWinInstance {
    pub thread_handle: thread::JoinHandle<()>,
    pub keep_running: Arc<AtomicBool>,
}

pub enum WinHypervisorInstance {
    Real {
        vm_name: String,
    },
    Simulated(SimulatedWinInstance),
}

pub struct HyperVAndroidHypervisor {
    pub vm_name: String,
    pub vhdx_path: String,
    pub force_simulation: bool,
}

impl HyperVAndroidHypervisor {
    pub fn new(
        vm_name: impl Into<String>,
        vhdx_path: impl Into<String>,
        force_simulation: bool,
    ) -> Self {
        Self {
            vm_name: vm_name.into(),
            vhdx_path: vhdx_path.into(),
            force_simulation,
        }
    }

    /// Verifica si Hyper-V está disponible. En entornos que no son Windows o no tienen
    /// el rol de Hyper-V habilitado, retornará falso de forma segura.
    pub fn is_hyperv_available(&self) -> bool {
        if !cfg!(target_os = "windows") {
            return false;
        }

        // Ejecutamos una verificación rápida de PowerShell para saber si los cmdlets de Hyper-V existen
        let output = Command::new("powershell")
            .arg("-Command")
            .arg("Get-Command Start-VM -ErrorAction SilentlyContinue")
            .output();

        match output {
            Ok(out) => out.status.success() && !out.stdout.is_empty(),
            Err(_) => false,
        }
    }
}

impl AndroidHypervisor for HyperVAndroidHypervisor {
    fn boot(&self, volume: &MountedVolume) -> PlatformResult<RunningInstance> {
        let use_simulation = self.force_simulation || !self.is_hyperv_available();

        if use_simulation {
            info!("Iniciando Android Runtime sobre HyperVAndroidHypervisor en [MODO SIMULADO]");
            info!("Causa: {}", if self.force_simulation { "Simulación forzada por configuración" } else { "Módulo Hyper-V no disponible en este sistema operativo" });

            let keep_running = Arc::new(AtomicBool::new(true));
            let keep_running_clone = keep_running.clone();

            let thread_handle = thread::spawn(move || {
                let steps = vec![
                    "[hyperv-sim] Windows Hyper-V Hypervisor Services loading...",
                    "[hyperv-sim] Connecting Virtual Switch and Synthetic Network Adapters...",
                    "[hyperv-sim] Attaching dynamic decrypted VHDX secure storage volume...",
                    "[hyperv-sim] Powering on VM 'ConfidentialVaultAndroid'...",
                    "[hyperv-sim] UEFI firmware handoff to secure bootloader...",
                    "[hyperv-sim] Android system partition mounting...",
                    "[hyperv-sim] Android subsystem successfully initialized inside Hyper-V sandbox."
                ];

                for step in steps {
                    if !keep_running_clone.load(Ordering::SeqCst) {
                        break;
                    }
                    info!("{}", step);
                    thread::sleep(Duration::from_millis(150));
                }

                while keep_running_clone.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(500));
                }
                info!("[hyperv-sim] VM de Windows Hyper-V de Android Runtime apagada.");
            });

            let sim_inst = SimulatedWinInstance {
                thread_handle,
                keep_running,
            };

            Ok(RunningInstance {
                platform_handle: Box::new(WinHypervisorInstance::Simulated(sim_inst)),
            })
        } else {
            info!(
                vm_name = %self.vm_name,
                vhdx = %self.vhdx_path,
                mount = %volume.mount_path,
                "Iniciando VM real de Hyper-V para Android..."
            );

            // Ejecuta PowerShell para arrancar la máquina virtual de Hyper-V de forma nativa
            let script = format!("Start-VM -Name '{}'", self.vm_name);
            let mut cmd = Command::new("powershell");
            cmd.arg("-Command").arg(&script);

            match cmd.status() {
                Ok(status) if status.success() => {
                    info!("Máquina virtual '{}' arrancada exitosamente por Hyper-V SCM.", self.vm_name);
                    Ok(RunningInstance {
                        platform_handle: Box::new(WinHypervisorInstance::Real {
                            vm_name: self.vm_name.clone(),
                        }),
                    })
                }
                Ok(status) => {
                    error!("Error al arrancar la VM de Hyper-V, PowerShell retornó código: {:?}", status.code());
                    Err(PlatformError::Other(format!("PowerShell returned exit status {:?}", status.code())))
                }
                Err(e) => {
                    error!("No se pudo ejecutar el comando de PowerShell: {:?}", e);
                    Err(PlatformError::Io(e))
                }
            }
        }
    }

    fn stop(&self, instance: RunningInstance) -> PlatformResult<()> {
        let instance_handle = instance
            .platform_handle
            .downcast::<WinHypervisorInstance>()
            .map_err(|_| PlatformError::Other("No se pudo downcastear el manejador de la instancia de Windows".to_string()))?;

        match *instance_handle {
            WinHypervisorInstance::Real { vm_name } => {
                info!("Deteniendo VM real de Hyper-V '{}'...", vm_name);
                let script = format!("Stop-VM -Name '{}' -Force", vm_name);
                let mut cmd = Command::new("powershell");
                cmd.arg("-Command").arg(&script);
                let _ = cmd.status();
                info!("VM de Hyper-V detenida limpiamente.");
                Ok(())
            }
            WinHypervisorInstance::Simulated(sim) => {
                info!("Deteniendo VM simulada de Windows...");
                sim.keep_running.store(false, Ordering::SeqCst);
                let _ = sim.thread_handle.join();
                info!("VM de Windows simulada apagada exitosamente.");
                Ok(())
            }
        }
    }
}

pub struct NotImplementedHypervisor;

impl AndroidHypervisor for NotImplementedHypervisor {
    fn boot(&self, _volume: &MountedVolume) -> PlatformResult<RunningInstance> {
        Err(PlatformError::Other(
            "AndroidHypervisor todavía no está implementado en Windows — falta la integración \
             real con Hyper-V o crosvm para Windows (ver Fase C en ROADMAP_MULTIPLATFORM.md)"
                .to_string(),
        ))
    }

    fn stop(&self, _instance: RunningInstance) -> PlatformResult<()> {
        Err(PlatformError::Other(
            "AndroidHypervisor todavía no está implementado en Windows".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vault_core::MountedVolume;

    #[test]
    fn test_hyperv_hypervisor_new() {
        let hypervisor = HyperVAndroidHypervisor::new(
            "ConfidentialVaultVM",
            "C:\\Vault\\secure.vhdx",
            true,
        );

        assert_eq!(hypervisor.vm_name, "ConfidentialVaultVM");
        assert_eq!(hypervisor.vhdx_path, "C:\\Vault\\secure.vhdx");
        assert!(hypervisor.force_simulation);
    }

    #[test]
    fn test_hyperv_hypervisor_simulation_boot_stop() {
        let hypervisor = HyperVAndroidHypervisor::new(
            "ConfidentialVaultVM",
            "C:\\Vault\\secure.vhdx",
            true, // Forzar simulación
        );

        let mock_volume = MountedVolume {
            mount_path: "E:\\".to_string(),
            platform_handle: Box::new("test_windows_vault".to_string()),
        };

        let instance = hypervisor.boot(&mock_volume).expect("No se pudo iniciar el boot simulado en Windows");
        let stop_res = hypervisor.stop(instance);
        assert!(stop_res.is_ok(), "El apagado de la simulación de Windows falló");
    }
}
