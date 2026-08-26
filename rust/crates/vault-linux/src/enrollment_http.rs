//! Único endpoint HTTP que expone `vault_host`: una página con el QR de
//! enrolamiento, servida SOLO mientras `vault_runtime` está en modo
//! ENROLLING (es decir, mientras exista `/run/vault/enrollment_info.sock`).
//! No hay autenticación acá a propósito — el contenido no es secreto
//! (es literalmente lo que se va a mostrar como QR) y el acceso ya está
//! acotado a quien esté en la LAN local durante la ventana de pairing.
//!
//! Usa `tiny_http` (sync, sin dependencias pesadas) en su propio hilo
//! bloqueante en vez de sumar un framework async completo solo para
//! esta única página.

use std::io::Read;
use std::os::unix::net::UnixStream as StdUnixStream;
use std::time::Duration;
use tiny_http::{Header, Response, Server};
use tracing::{info, warn};

const HTTP_ADDR: &str = "0.0.0.0:8088";
const ENROLLMENT_INFO_SOCKET: &str = "/run/vault/enrollment_info.sock";

pub fn spawn_enrollment_http_server() {
    std::thread::spawn(|| {
        let server = match Server::http(HTTP_ADDR) {
            Ok(s) => s,
            Err(e) => {
                warn!(error = %e, "no se pudo levantar el servidor HTTP de enrolamiento");
                return;
            }
        };
        info!(addr = HTTP_ADDR, "servidor de enrolamiento (QR) escuchando");

        for request in server.incoming_requests() {
            let host_header = request
                .headers()
                .iter()
                .find(|h| h.field.equiv("Host"))
                .map(|h| h.value.as_str().to_string());

            let body = render_enroll_page(host_header.as_deref());
            let header = Header::from_bytes(&b"Content-Type"[..], &b"text/html; charset=utf-8"[..])
                .expect("header válido");
            let response = Response::from_string(body).with_header(header);

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
                port: 7443, // debe coincidir con LISTEN_ADDR de main.rs
                token: info.token,
                expires_unix_ms: info.expires_unix_ms,
            };
            let payload_json = serde_json::to_string(&payload).unwrap_or_default();
            let svg = render_qr_svg(&payload_json);

            format!(
                r#"<!doctype html>
<html><head><meta charset="utf-8"><title>Vincular Android Confidential Vault</title>
<style>body{{font-family:sans-serif;text-align:center;margin-top:40px}}
pre{{max-width:500px;margin:20px auto;white-space:pre-wrap;word-break:break-all;
     background:#f0f0f0;padding:12px;border-radius:6px;text-align:left}}</style>
</head><body>
<h1>Vinculá tu teléfono</h1>
<p>Abrí la app y escaneá este código con la opción "Vincular con QR".</p>
{svg}
<p>Este QR expira en unos minutos y deja de servir apenas se vincula un dispositivo.</p>
<details><summary>Fallback manual (si no podés escanear)</summary><pre>{payload_json}</pre></details>
</body></html>"#
            )
        }
        None => {
            r#"<!doctype html><html><body style="font-family:sans-serif;text-align:center;margin-top:40px">
<h1>No hay enrolamiento pendiente</h1>
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

/// Consulta el socket local (sin cifrar, sin autenticación — ver nota de
/// módulo) donde `vault_runtime` publica los datos del QR mientras está
/// en modo ENROLLING.
fn fetch_enrollment_info() -> Option<RawEnrollmentInfo> {
    let mut stream = StdUnixStream::connect(ENROLLMENT_INFO_SOCKET).ok()?;
    stream
        .set_read_timeout(Some(Duration::from_secs(2)))
        .ok()?;
    stream.shutdown(std::net::Shutdown::Write).ok(); // uni-directional: solo leemos

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
