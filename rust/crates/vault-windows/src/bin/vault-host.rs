//! `vault-host` (Windows) — corre en el host Windows principal.
//!
//! Acepta las conexiones del dispositivo Android por TCP en el puerto 7443
//! y actúa como relay ciego reenviando los bytes al puerto TCP local 7444 de `vault-runtime`.
//! No puede ver el contenido del canal cifrado por Noise (Zero-Visibility Forwarder).
//!
//! También expone un servidor HTTP de enrolamiento en el puerto 8088 que
//! muestra un código QR autogenerado si `vault-runtime` está en modo ENROLLING.

use std::sync::Arc;
use std::time::Duration;
use tokio::io;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Semaphore;
use tracing::{info, warn};

use vault_core::rate_limit::{now_unix_ms, IpRateLimiter, RateLimitDecision};

const LISTEN_ADDR: &str = "0.0.0.0:7443";
const RUNTIME_ADDR: &str = "127.0.0.1:7444";
const IDLE_TIMEOUT: Duration = Duration::from_secs(120);
const MAX_CONCURRENT_CONNECTIONS: usize = 32;
const PRUNE_INTERVAL: Duration = Duration::from_secs(5 * 60);

const HTTP_ADDR: &str = "0.0.0.0:8088";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = std::env::args().collect();
    let is_service = args.iter().any(|arg| arg == "--service");

    if is_service {
        info!("Ejecutando vault-host como Windows Service nativo...");
        // Iniciamos el dispatcher del servicio nativo
        vault_windows::run_service("ConfidentialVaultHost", move || {
            let rt = tokio::runtime::Runtime::new().expect("No se pudo iniciar el runtime de Tokio para el servicio");
            rt.block_on(async {
                if let Err(e) = run_host_server().await {
                    tracing::error!("Error ejecutando el servidor principal en el servicio: {:?}", e);
                }
            });
        })?;
    } else {
        info!(
            "vault-host Windows arrancando en modo interactivo — modo forwarder ciego, sin visibilidad del canal cifrado"
        );
        run_host_server().await?;
    }

    Ok(())
}

async fn run_host_server() -> anyhow::Result<()> {
    // Arrancamos el servidor HTTP local para servir el QR de vinculación
    spawn_enrollment_http_server();

    let rate_limiter = Arc::new(IpRateLimiter::new());
    let connection_slots = Arc::new(Semaphore::new(MAX_CONCURRENT_CONNECTIONS));

    spawn_pruning_task(rate_limiter.clone());

    let listener = TcpListener::bind(LISTEN_ADDR).await?;
    info!(addr = LISTEN_ADDR, "escuchando conexiones del frontend Android");

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
                    "conexión de relay rechazada: cupo de conexiones concurrentes agotado"
                );
                drop(frontend_stream);
                continue;
            }
        };

        info!(%peer_addr, "nueva conexión entrante, aceptada");

        tokio::spawn(async move {
            let _permit = permit;
            if let Err(e) = handle_connection(frontend_stream).await {
                warn!(%peer_addr, error = %e, "conexión terminada con error");
            } else {
                info!(%peer_addr, "conexión cerrada limpiamente");
            }
        });
    }
}

async fn handle_connection(mut frontend_stream: TcpStream) -> anyhow::Result<()> {
    // Intenta conectar al runtime local que corre en segundo plano
    let mut runtime_stream = TcpStream::connect(RUNTIME_ADDR).await?;

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
            warn!("timeout de inactividad, cerrando conexión");
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

// --- Servidor de Enrolamiento HTTP para Windows ---

pub fn spawn_enrollment_http_server() {
    std::thread::spawn(|| {
        let server = match tiny_http::Server::http(HTTP_ADDR) {
            Ok(s) => s,
            Err(e) => {
                warn!(error = %e, "no se pudo levantar el servidor HTTP de enrolamiento");
                return;
            }
        };
        info!(addr = HTTP_ADDR, "servidor de enrolamiento Windows (QR) escuchando");

        for request in server.incoming_requests() {
            let host_header = request
                .headers()
                .iter()
                .find(|h| h.field.equiv("Host"))
                .map(|h| h.value.as_str().to_string());

            let body = render_enroll_page(host_header.as_deref());
            let header = tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..])
                .expect("header válido");
            let response = tiny_http::Response::from_string(body).with_header(header);

            if let Err(e) = request.respond(response) {
                warn!(error = %e, "error respondiendo request HTTP de enrolamiento");
            }
        }
    });
}

fn render_enroll_page(host_header: Option<&str>) -> String {
    match fetch_enrollment_info() {
        Some(info) => {
            let host_only = host_header
                .and_then(|h| h.split(':').next())
                .unwrap_or("<IP-DEL-VAULT>")
                .to_string();

            let payload = vault_protocol::EnrollmentQrPayload {
                v: info.v,
                runtime_pubkey_hex: info.runtime_pubkey_hex,
                host: host_only,
                port: 7443,
                token: info.token,
                expires_unix_ms: info.expires_unix_ms,
            };
            let payload_json = serde_json::to_string(&payload).unwrap_or_default();
            let svg = render_qr_svg(&payload_json);

            format!(
                r#"<!doctype html>
<html><head><meta charset="utf-8"><title>Vincular Android Confidential Vault (Windows)</title>
<style>body{{font-family:sans-serif;text-align:center;margin-top:40px}}
pre{{max-width:500px;margin:20px auto;white-space:pre-wrap;word-break:break-all;
     background:#f0f0f0;padding:12px;border-radius:6px;text-align:left}}</style>
</head><body>
<h1>Vinculá tu teléfono con Windows</h1>
<p>Abrí la app y escaneá este código con la opción "Vincular con QR".</p>
{svg}
<p>Este QR expira en unos minutos y deja de servir apenas se vincula un dispositivo.</p>
<details><summary>Fallback manual (si no podés escanear)</summary><pre>{payload_json}</pre></details>
</body></html>"#
            )
        }
        None => {
            r#"<!doctype html><html><body style="font-family:sans-serif;text-align:center;margin-top:40px">
<h1>No hay enrolamiento pendiente en Windows</h1>
<p>O ya hay un dispositivo vinculado, o <code>vault_runtime</code> todavía no arrancó.</p>
</body></html>"#
                .to_string()
        }
    }
}

struct RawEnrollmentInfo {
    runtime_pubkey_hex: String,
    token: String,
    expires_unix_ms: u64,
    v: u8,
}

fn fetch_enrollment_info() -> Option<RawEnrollmentInfo> {
    use std::io::Read;
    let mut stream = std::net::TcpStream::connect("127.0.0.1:7445").ok()?;
    stream.set_read_timeout(Some(Duration::from_secs(2))).ok()?;

    let mut buf = String::new();
    stream.read_to_string(&mut buf).ok()?;

    let json: serde_json::Value = serde_json::from_str(&buf).ok()?;
    Some(RawEnrollmentInfo {
        runtime_pubkey_hex: json.get("runtime_pubkey_hex")?.as_str()?.to_string(),
        token: json.get("token")?.as_str()?.to_string(),
        expires_unix_ms: json.get("expires_unix_ms")?.as_u64()?,
        v: json.get("v")?.as_u64()? as u8,
    })
}

fn render_qr_svg(data: &str) -> String {
    use qrcode::render::svg;
    use qrcode::QrCode;

    match QrCode::new(data.as_bytes()) {
        Ok(code) => code
            .render()
            .min_dimensions(280, 280)
            .dark_color(svg::Color("#000000"))
            .light_color(svg::Color("#ffffff"))
            .build(),
        Err(e) => format!("<p>No se pudo generar el QR: {e}</p>"),
    }
}
