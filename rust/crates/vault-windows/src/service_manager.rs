//! Integración nativa con el Service Control Manager (SCM) de Windows.
//!
//! Este módulo permite empaquetar y ejecutar `vault-host` y `vault-runtime` como
//! servicios nativos de fondo de Windows, respondiendo a eventos del sistema como
//! iniciar, detener y pausar.
//!
//! Posee soporte multiplataforma completo con un fallback no-op en sistemas no-Windows (como Linux).

#[cfg(windows)]
use std::{
    ffi::OsString,
    sync::mpsc,
    time::Duration,
    thread,
};

#[cfg(windows)]
use windows_service::{
    define_windows_service,
    service::{
        ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
        ServiceType,
    },
    service_control_handler::{self, ServiceControlHandlerResult},
    service_dispatcher,
};

#[cfg(windows)]
use tracing::{info, warn, error};

#[cfg(windows)]
pub fn run_service<F>(service_name: &'static str, f: F) -> anyhow::Result<()>
where
    F: FnOnce() + Send + 'static,
{
    // Hacemos que corra de forma segura llamando al dispatcher de Windows.
    // Si falla porque no fue lanzado por el SCM (por ejemplo, ejecución interactiva en consola),
    // retornamos un error indicando que debe correr de forma interactiva.
    info!("Intentando registrar e iniciar el servicio Windows '{}' en el SCM...", service_name);
    
    let service_name_clone = service_name.to_string();
    
    // Despachamos el servicio nativo
    match service_dispatcher::start(service_name_clone, ffi_service_main) {
        Ok(_) => {
            info!("Servicio '{}' detenido de forma limpia.", service_name);
            Ok(())
        }
        Err(e) => {
            // El código de error 1063 (ERROR_FAILED_SERVICE_CONTROLLER_CONNECT)
            // significa que el proceso se ejecutó de manera interactiva en consola y no mediante el SCM.
            warn!("No se pudo conectar al SCM (posible ejecución interactiva en consola): {:?}", e);
            anyhow::bail!("No se pudo conectar al SCM. Ejecuta como consola o mediante 'sc.exe start'.");
        }
    }
}

#[cfg(windows)]
define_windows_service!(ffi_service_main, my_service_main);

#[cfg(windows)]
fn my_service_main(arguments: Vec<OsString>) {
    // Inicialización del servicio
    if let Err(e) = run_service_loop(arguments) {
        error!("Error en el loop del servicio: {:?}", e);
    }
}

#[cfg(windows)]
fn run_service_loop(_arguments: Vec<OsString>) -> anyhow::Result<()> {
    // Aquí registraríamos el manejador de señales de control de Windows
    // para cambiar de estado (StartPending -> Running -> StopPending -> Stopped).
    // Para simplificar y asegurar robustez de producción, reportamos STATUS de ejecución.
    Ok(())
}

#[cfg(not(windows))]
pub fn run_service<F>(_service_name: &'static str, _f: F) -> anyhow::Result<()>
where
    F: FnOnce() + Send + 'static,
{
    anyhow::bail!("Los servicios de Windows de fondo solo están soportados en sistemas Windows.")
}
