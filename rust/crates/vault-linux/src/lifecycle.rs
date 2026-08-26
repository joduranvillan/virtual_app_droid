//! Gestión mínima del ciclo de vida de `vault_runtime` desde el host.
//!
//! En producción esto lanzaría `vault_runtime` dentro de un namespace
//! aislado (`unshare` de PID/mount/net, o una microVM `crosvm`+`ARCVM`
//! — ver ARCHITECTURE.md §13 Fase 1) cada vez que se detecta
//! el primer intento de conexión, y lo apagaría tras `IDLE_TIMEOUT`.
//!
//! Para este entregable, `vault_runtime` se asume lanzado por separado
//! (systemd unit / manualmente) y este módulo solo verifica que su
//! socket esté disponible antes de aceptar conexiones del frontend.

use std::path::Path;
use std::time::Duration;
use tokio::net::UnixStream;
use tracing::warn;

pub async fn assert_runtime_reachable(socket_path: &str) -> anyhow::Result<()> {
    if !Path::new(socket_path).exists() {
        warn!(
            socket_path,
            "el socket de vault_runtime todavía no existe — \
             asegurate de que el proceso vault_runtime esté corriendo \
             antes de que lleguen conexiones del frontend"
        );
        return Ok(()); // no fatal: puede levantarse después
    }

    match tokio::time::timeout(Duration::from_secs(2), UnixStream::connect(socket_path)).await {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(e)) => {
            warn!(error = %e, "no se pudo conectar al socket de vault_runtime");
            Ok(())
        }
        Err(_) => {
            warn!("timeout esperando a vault_runtime");
            Ok(())
        }
    }
}
