//! `vault-host` — corre en el Linux "caja negra". Es deliberadamente
//! tonto: acepta conexiones del frontend Android por TCP (QUIC queda como
//! mejora futura, ver ARCHITECTURE.md §13) y reenvía los bytes, sin
//! inspeccionarlos, hacia el socket Unix donde escucha `vault-runtime`.
//!
//! `vault-host` NUNCA:
//! - hace el handshake Noise,
//! - ve claves de sesión ni la clave de desbloqueo del volumen,
//! - decodifica frames de protocolo.
//!
//! Solo sabe: "hay un cliente conectado" y "hay bytes yendo de un lado
//! al otro". Eso es lo que permite decir honestamente que el operador
//! del host no puede ver el contenido (ver ARCHITECTURE.md §5).
//!
//! Sí sabe, y le corresponde a él filtrar, cuántas conexiones nuevas
//! está abriendo cada IP: cada conexión le hace hacer a `vault-runtime`
//! un handshake Noise completo (costo real de CPU) antes de llegar a
//! cualquier verificación de más alto nivel — por eso el rate-limiting
//! por IP (`vault_core::rate_limit`, compartido entre plataformas) y el
//! tope de concurrencia viven acá, no en `vault-runtime`.
//!
//! `vault-runtime` se lanza como proceso aparte (en su propio namespace
//! de PID/mount/red en producción) y expone un socket Unix. Este binario
//! asume que ya está corriendo y escuchando en `RUNTIME_SOCKET_PATH`;
//! el arranque/apagado de ese proceso vive en `vault_linux::lifecycle`.

use std::sync::Arc;
use std::time::Duration;
use tokio::io;
use tokio::net::{TcpListener, UnixStream};
use tokio::sync::Semaphore;
use tracing::{info, warn};

use vault_core::rate_limit::{now_unix_ms, IpRateLimiter, RateLimitDecision};
use vault_linux::{enrollment_http, lifecycle};

const LISTEN_ADDR: &str = "0.0.0.0:7443";
const RUNTIME_SOCKET_PATH: &str = "/run/vault/runtime.sock";
const IDLE_TIMEOUT: Duration = Duration::from_secs(120);

/// Tope de conexiones relayadas simultáneamente. Protege a
/// `vault-runtime` (y a los recursos del host) de agotarse con muchas
/// conexiones abiertas a la vez, más allá de cuántas por segundo permite
/// `IpRateLimiter`. Conexiones que llegan con el cupo lleno se rechazan
/// de inmediato (no se encolan) para no dar pie a un ataque tipo
/// slow-loris acumulando esperas.
const MAX_CONCURRENT_CONNECTIONS: usize = 32;

const PRUNE_INTERVAL: Duration = Duration::from_secs(5 * 60);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    info!(
        "vault-host arrancando — modo forwarder ciego, sin visibilidad del canal cifrado"
    );
    lifecycle::assert_runtime_reachable(RUNTIME_SOCKET_PATH).await?;
    enrollment_http::spawn_enrollment_http_server();

    let rate_limiter = Arc::new(IpRateLimiter::new());
    let connection_slots = Arc::new(Semaphore::new(MAX_CONCURRENT_CONNECTIONS));

    spawn_pruning_task(rate_limiter.clone());

    let listener = TcpListener::bind(LISTEN_ADDR).await?;
    info!(addr = LISTEN_ADDR, "escuchando conexiones del frontend");

    loop {
        let (frontend_stream, peer_addr) = listener.accept().await?;

        match rate_limiter.check(peer_addr.ip(), now_unix_ms()) {
            RateLimitDecision::Deny { retry_after_ms } => {
                warn!(
                    %peer_addr,
                    retry_after_ms,
                    "conexión rechazada por rate-limit — demasiados intentos desde esta IP"
                );
                drop(frontend_stream);
                continue;
            }
            RateLimitDecision::Allow => {}
        }

        let permit = match connection_slots.clone().try_acquire_owned() {
            Ok(p) => p,
            Err(_) => {
                warn!(
                    %peer_addr,
                    max = MAX_CONCURRENT_CONNECTIONS,
                    "conexión rechazada: cupo de conexiones concurrentes agotado"
                );
                drop(frontend_stream);
                continue;
            }
        };

        info!(%peer_addr, "nueva conexión entrante, aceptada");

        tokio::spawn(async move {
            let _permit = permit; // se libera al terminar la task
            if let Err(e) = handle_connection(frontend_stream).await {
                warn!(%peer_addr, error = %e, "conexión terminada con error");
            } else {
                info!(%peer_addr, "conexión cerrada limpiamente");
            }
        });
    }
}

/// Relay bidireccional ciego entre el socket TCP del frontend y el socket
/// Unix de `vault-runtime`. No decodifica nada; `io::copy_bidirectional`
/// mueve bytes tal cual en ambas direcciones hasta que una punta cierra
/// o pasa `IDLE_TIMEOUT` sin actividad.
async fn handle_connection(mut frontend_stream: tokio::net::TcpStream) -> anyhow::Result<()> {
    let mut runtime_stream = UnixStream::connect(RUNTIME_SOCKET_PATH).await?;

    let copy_result = tokio::time::timeout(
        IDLE_TIMEOUT,
        io::copy_bidirectional(&mut frontend_stream, &mut runtime_stream),
    )
    .await;

    match copy_result {
        Ok(Ok((frontend_to_runtime, runtime_to_frontend))) => {
            info!(
                frontend_to_runtime,
                runtime_to_frontend, "relay finalizado normalmente"
            );
        }
        Ok(Err(e)) => return Err(e.into()),
        Err(_) => {
            warn!("timeout de inactividad, cerrando conexión — vault vuelve a LOCKED");
        }
    }
    Ok(())
}

fn spawn_pruning_task(rate_limiter: Arc<IpRateLimiter>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(PRUNE_INTERVAL);
        loop {
            interval.tick().await;
            rate_limiter.prune_stale(now_unix_ms());
        }
    });
}
