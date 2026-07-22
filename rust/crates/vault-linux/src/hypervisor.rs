//! Implementación Linux de `vault_core::AndroidHypervisor`.
//!
//! Soporta tanto la ejecución real del hipervisor `crosvm` con `ARCVM` (requiriendo `/dev/kvm`)
//! como un modo de simulación completamente funcional y detallado para pruebas locales o
//! entornos de integración continua (como contenedores de Docker/AI Studio) donde la
//! virtualización de hardware no está expuesta.

use std::fs;
use std::path::Path;
use std::process::{Child, Command};
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use tracing::{info, warn, error};

use vault_core::{AndroidHypervisor, MountedVolume, PlatformError, PlatformResult, RunningInstance};

/// Identificador único para una instancia simulada en memoria.
pub struct SimulatedInstance {
    pub thread_handle: thread::JoinHandle<()>,
    pub keep_running: Arc<AtomicBool>,
}

pub enum HypervisorInstance {
    Real(Child),
    Simulated(SimulatedInstance),
}

pub struct CrosvmAndroidHypervisor {
    pub kernel_path: String,
    pub rootfs_path: String,
    pub socket_path: String,
    pub memory_mb: u32,
    pub cpus: u32,
    pub force_simulation: bool,
}

impl CrosvmAndroidHypervisor {
    pub fn new(
        kernel_path: impl Into<String>,
        rootfs_path: impl Into<String>,
        socket_path: impl Into<String>,
        memory_mb: u32,
        cpus: u32,
        force_simulation: bool,
    ) -> Self {
        Self {
            kernel_path: kernel_path.into(),
            rootfs_path: rootfs_path.into(),
            socket_path: socket_path.into(),
            memory_mb,
            cpus,
            force_simulation,
        }
    }

    /// Verifica si la virtualización por hardware mediante KVM está disponible en el host.
    pub fn is_kvm_available(&self) -> bool {
        Path::new("/dev/kvm").exists()
    }
}

impl AndroidHypervisor for CrosvmAndroidHypervisor {
    fn boot(&self, volume: &MountedVolume) -> PlatformResult<RunningInstance> {
        let use_simulation = self.force_simulation || !self.is_kvm_available();

        if use_simulation {
            info!("Iniciando Android Runtime sobre CrosvmAndroidHypervisor en [MODO SIMULADO]");
            info!("Causa: {}", if self.force_simulation { "Simulación forzada por configuración" } else { "/dev/kvm no está presente en el host" });

            let keep_running = Arc::new(AtomicBool::new(true));
            let keep_running_clone = keep_running.clone();
            
            // Simular logs de booteo reales de ARCVM y el kernel de Android en un hilo dedicado
            let thread_handle = thread::spawn(move || {
                let steps = vec![
                    "[crosvm-sim] [0.000000] Linux version 6.1.50-android-crosvm (android-build@google.com) ...",
                    "[crosvm-sim] [0.045120] CPU0: Intel(R) Xeon(R) Gold ...",
                    "[crosvm-sim] [0.125192] virtio-gpu: initialized with virglrenderer GPU acceleration",
                    "[crosvm-sim] [0.245811] ARCVM secure volume detected on mounted device mapper.",
                    "[crosvm-sim] [0.551023] init: entering stage 1 initialization ...",
                    "[crosvm-sim] [0.892110] init: Loading SELinux policies ...",
                    "[crosvm-sim] [1.240591] init: Starting Android main services (surfaceflinger, system_server) ...",
                    "[crosvm-sim] [2.012543] zygote: preloading classes ...",
                    "[crosvm-sim] [3.512093] SystemUI: starting custom desktop interface ...",
                    "[crosvm-sim] [4.102319] Android Virtual Device successfully booted. secure-channel-ready."
                ];

                for step in steps {
                    if !keep_running_clone.load(Ordering::SeqCst) {
                        break;
                    }
                    info!("{}", step);
                    thread::sleep(Duration::from_millis(150));
                }
                
                info!("[crosvm-sim] Android Runtime simulado corriendo en segundo plano...");
                while keep_running_clone.load(Ordering::SeqCst) {
                    thread::sleep(Duration::from_millis(500));
                }
                info!("[crosvm-sim] Hilo del Android Runtime simulado detenido.");
            });

            let sim_inst = SimulatedInstance {
                thread_handle,
                keep_running,
            };

            Ok(RunningInstance {
                platform_handle: Box::new(HypervisorInstance::Simulated(sim_inst)),
            })
        } else {
            info!(
                kernel = %self.kernel_path,
                rootfs = %self.rootfs_path,
                mount = %volume.mount_path,
                "Iniciando Android Runtime real con crosvm (ARCVM)..."
            );

            // Construir los argumentos para lanzar crosvm de manera nativa con aislamiento
            let mut cmd = Command::new("crosvm");
            cmd.arg("run")
                .arg("--cpus")
                .arg(self.cpus.to_string())
                .arg("--mem")
                .arg(self.memory_mb.to_string())
                .arg("--root")
                .arg(&self.rootfs_path)
                .arg("--socket")
                .arg(&self.socket_path)
                .arg(&self.kernel_path);

            match cmd.spawn() {
                Ok(child) => {
                    info!("Proceso de crosvm lanzado exitosamente con PID {}", child.id());
                    Ok(RunningInstance {
                        platform_handle: Box::new(HypervisorInstance::Real(child)),
                    })
                }
                Err(e) => {
                    error!("Error lanzando el proceso crosvm: {:?}", e);
                    Err(PlatformError::Io(e))
                }
            }
        }
    }

    fn stop(&self, instance: RunningInstance) -> PlatformResult<()> {
        let instance_handle = instance
            .platform_handle
            .downcast::<HypervisorInstance>()
            .map_err(|_| PlatformError::Other("No se pudo downcastear el manejador de la instancia de Hypervisor".to_string()))?;

        match *instance_handle {
            HypervisorInstance::Real(mut child) => {
                info!("Deteniendo proceso crosvm real con PID {}...", child.id());
                match child.kill() {
                    Ok(_) => {
                        let _ = child.wait();
                        info!("Proceso crosvm detenido de forma limpia.");
                        Ok(())
                    }
                    Err(e) => {
                        error!("Error matando el proceso crosvm: {:?}", e);
                        Err(PlatformError::Io(e))
                    }
                }
            }
            HypervisorInstance::Simulated(sim) => {
                info!("Deteniendo Android Runtime simulado...");
                sim.keep_running.store(false, Ordering::SeqCst);
                let _ = sim.thread_handle.join();
                info!("Instancia de Android Runtime simulado apagada exitosamente.");
                Ok(())
            }
        }
    }
}

pub struct NotImplementedHypervisor;

impl AndroidHypervisor for NotImplementedHypervisor {
    fn boot(&self, _volume: &MountedVolume) -> PlatformResult<RunningInstance> {
        Err(PlatformError::Other(
            "AndroidHypervisor todavía no está implementado en Linux — falta la integración \
             real con crosvm+ARCVM (ver Fase 1 en ARCHITECTURE.md)"
                .to_string(),
        ))
    }

    fn stop(&self, _instance: RunningInstance) -> PlatformResult<()> {
        Err(PlatformError::Other(
            "AndroidHypervisor todavía no está implementado en Linux".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vault_core::MountedVolume;

    #[test]
    fn test_crosvm_hypervisor_new() {
        let hypervisor = CrosvmAndroidHypervisor::new(
            "kernel.bin",
            "rootfs.img",
            "crosvm.sock",
            2048,
            4,
            true,
        );

        assert_eq!(hypervisor.kernel_path, "kernel.bin");
        assert_eq!(hypervisor.rootfs_path, "rootfs.img");
        assert_eq!(hypervisor.socket_path, "crosvm.sock");
        assert_eq!(hypervisor.memory_mb, 2048);
        assert_eq!(hypervisor.cpus, 4);
        assert!(hypervisor.force_simulation);
    }

    #[test]
    fn test_crosvm_hypervisor_simulation_boot_stop() {
        let hypervisor = CrosvmAndroidHypervisor::new(
            "kernel.bin",
            "rootfs.img",
            "crosvm.sock",
            1024,
            2,
            true, // Forzar simulación
        );

        let mock_volume = MountedVolume {
            mount_path: "/dev/mapper/test_vault".to_string(),
            platform_handle: Box::new("test_vault".to_string()),
        };

        // Arrancar
        let instance = hypervisor.boot(&mock_volume).expect("No se pudo iniciar el boot simulado");

        // Detener
        let stop_res = hypervisor.stop(instance);
        assert!(stop_res.is_ok(), "El apagado de la simulación falló");
    }
}
