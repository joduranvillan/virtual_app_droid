//! `vault-runtime` — corre DENTRO del namespace aislado con el filesystem
//! descifrado montado. Acá es donde termina el handshake Noise (no en
//! `vault-host`, ver ARCHITECTURE.md §5): este proceso es el único que
//! llega a tener las claves de sesión y, eventualmente, la clave de
//! desbloqueo del volumen LUKS2 en memoria.
//!
//! Responsabilidades:
//! - escuchar en un socket Unix (`vault-host` lo relaya ciegamente),
//! - hacer de responder del handshake Noise_XX,
//! - en la primera conexión (sin nada pineado): correr el flujo de
//!   enrolamiento por QR (lógica en `vault_core::EnrollmentState`, este
//!   binario solo la conecta con `FileSecretStore` y publica la info
//!   pendiente por el socket que `vault-host` consulta),
//! - en conexiones siguientes: rechazar cualquier clave que no sea la
//!   ya pineada, y despachar RPC de servicios virtuales.
//!
//! Este binario NO debería correr con acceso a la red externa: solo
//! habla por el socket Unix local que `vault-host` conecta.

use std::path::Path;
use std::sync::Arc;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use vault_core::{request_location, EnrollmentState, SecretStore};
use vault_crypto::{run_responder_handshake, NoiseChannel};
use vault_linux::FileSecretStore;
use vault_protocol::{
    EnrollmentAckBody, EnrollmentConfirmBody, Frame, LocationAccuracy, MsgType, QR_PAYLOAD_VERSION,
};

const SOCKET_PATH: &str = "/run/vault/runtime.sock";
const IDENTITY_PATH: &str = "/var/lib/vault/runtime_identity.key";
const PIN_PATH: &str = "/var/lib/vault/pinned_frontend.pub";
const ENROLLMENT_INFO_SOCKET: &str = "/run/vault/enrollment_info.sock";

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let secret_store: Arc<dyn SecretStore> =
        Arc::new(FileSecretStore::new(IDENTITY_PATH, PIN_PATH));

    // Antes esta clave se regeneraba en cada arranque, lo cual invalidaba
    // cualquier pairing existente (el teléfono queda con un pin que ya no
    // corresponde a nada). Ahora se persiste — ver
    // vault_linux::secret_store para el detalle exacto de qué nivel de
    // protección da (permisos de archivo 0600/0700, NO cifrado real: eso
    // requiere un TPM).
    let keypair = secret_store.load_or_generate_identity()?;
    info!(
        public_key_hex = hex::encode(&keypair.public),
        "vault-runtime arrancó"
    );

    let enrollment_state = Arc::new(Mutex::new(EnrollmentState::new(secret_store.clone())));

    if enrollment_state.lock().await.is_enrolled()? {
        info!("ya hay un frontend pineado — modo normal, solo esa clave puede conectarse");
    } else {
        info!("sin frontend pineado — entrando en modo ENROLLING");
        let info = enrollment_state.lock().await.begin_enrollment();
        spawn_enrollment_info_socket(keypair.public.clone(), info.token, info.expires_unix_ms)?;
    }

    if let Some(parent) = Path::new(SOCKET_PATH).parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let _ = std::fs::remove_file(SOCKET_PATH);
    let listener = UnixListener::bind(SOCKET_PATH)?;
    info!(socket = SOCKET_PATH, "escuchando conexiones de vault-host");

    loop {
        let (stream, _) = listener.accept().await?;
        let local_private = keypair.private.clone();
        let state = enrollment_state.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_session(stream, &local_private, state).await {
                error!(error = %e, "sesión terminada con error");
            }
        });
    }
}

async fn handle_session(
    mut stream: UnixStream,
    local_private: &[u8],
    state: Arc<Mutex<EnrollmentState>>,
) -> anyhow::Result<()> {
    info!("iniciando handshake Noise_XX con el frontend (vía relay de vault-host)");
    let (transport, remote_pubkey) = run_responder_handshake(&mut stream, local_private).await?;
    info!("handshake completo — canal autenticado y cifrado establecido");

    let mut chan = NoiseChannel::new(stream, transport);

    let already_enrolled = state.lock().await.is_enrolled()?;

    if !already_enrolled {
        if !run_enrollment_exchange(&mut chan, &state, &remote_pubkey).await? {
            warn!("enrolamiento falló (token inválido/expirado) — cerrando conexión");
            return Ok(());
        }
        info!("nuevo dispositivo enrolado exitosamente");
        // Una vez enrolado, el socket de info del QR ya no debe servir
        // nada más — el propio EnrollmentState invalidó el `pending`
        // internamente; acá solo limpiamos el socket físico.
        let _ = std::fs::remove_file(ENROLLMENT_INFO_SOCKET);
        // A partir de acá seguiría el flujo normal de servicios si el
        // mismo teléfono se queda conectado tras el pairing; para este
        // MVP cortamos acá y esperamos la próxima conexión ya pineada.
        return Ok(());
    }

    let pinned = state.lock().await.pinned_public_key()?;
    if pinned.as_deref() != Some(remote_pubkey.as_slice()) {
        warn!("clave pública remota NO coincide con la pineada — rechazando conexión (posible dispositivo no autorizado)");
        return Ok(());
    }

    // Demo end-to-end de servicios ya establecida en el MVP anterior.
    match request_location(&mut chan, 1, "com.example.demo", LocationAccuracy::Fine).await {
        Ok(loc) => info!(
            lat = loc.latitude,
            lon = loc.longitude,
            accuracy_m = loc.accuracy_meters,
            "ubicación recibida del frontend"
        ),
        Err(e) => warn!(error = %e, "no se pudo obtener ubicación"),
    }

    info!("esperando pedidos o frames del frontend (modo sesión activo)...");
    loop {
        let frame = match chan.recv_frame().await {
            Ok(f) => f,
            Err(e) => {
                info!("conexión de sesión cerrada por el frontend o error: {}", e);
                break;
            }
        };

        match frame.msg_type {
            MsgType::ServiceRequest => {
                let request_envelope: ServiceRequestEnvelope = match frame.decode_body() {
                    Ok(env) => env,
                    Err(e) => {
                        warn!("error decodificando ServiceRequestEnvelope: {}", e);
                        continue;
                    }
                };
                info!(service = ?request_envelope.service, package = %request_envelope.requesting_package, "recibida petición de servicio");
                
                let response_envelope = vault_core::services::dispatch_service_request(request_envelope);
                let resp_frame = match Frame::new_serde(MsgType::ServiceResponse, frame.req_id, &response_envelope) {
                    Ok(f) => f,
                    Err(e) => {
                        warn!("error serializando ServiceResponseEnvelope: {}", e);
                        continue;
                    }
                };
                if let Err(e) = chan.send_frame(&resp_frame).await {
                    warn!("error enviando respuesta de servicio: {}", e);
                    break;
                }
            }
            MsgType::Heartbeat => {
                let hb_frame = Frame {
                    msg_type: MsgType::Heartbeat,
                    req_id: frame.req_id,
                    payload: vec![],
                };
                let _ = chan.send_frame(&hb_frame).await;
            }
            _ => {
                debug!(msg_type = ?frame.msg_type, "frame ignorado en el loop de sesión");
            }
        }
    }

    Ok(())
}

/// Corre el intercambio `EnrollmentConfirm` -> `EnrollmentAck` sobre el
/// canal ya cifrado. Devuelve `true` si el enrolamiento se completó.
async fn run_enrollment_exchange(
    chan: &mut NoiseChannel<UnixStream>,
    state: &Arc<Mutex<EnrollmentState>>,
    remote_pubkey: &[u8],
) -> anyhow::Result<bool> {
    let frame = chan.recv_frame().await?;
    if frame.msg_type != MsgType::EnrollmentConfirm {
        warn!(msg_type = ?frame.msg_type, "se esperaba EnrollmentConfirm como primer mensaje del enrolamiento");
        return Ok(false);
    }
    let body: EnrollmentConfirmBody = frame.decode_body()?;

    let result = state
        .lock()
        .await
        .try_complete_enrollment(&body.token, remote_pubkey);

    let ack = match &result {
        Ok(()) => EnrollmentAckBody {
            success: true,
            reason: None,
        },
        Err(e) => EnrollmentAckBody {
            success: false,
            reason: Some(e.to_string()),
        },
    };

    let ack_frame = Frame::new_serde(MsgType::EnrollmentAck, frame.req_id, &ack)?;
    chan.send_frame(&ack_frame).await?;

    Ok(result.is_ok())
}

/// Publica en un socket Unix local (sin cifrar — no es secreto, es
/// justamente lo que se va a mostrar en el QR) la info que `vault-host`
/// necesita para armar la página de enrolamiento. Este es el reemplazo
/// de lo que antes vivía dentro de `EnrollmentState::serve_enrollment_info`:
/// ahora `vault-core` solo decide *qué* publicar (`begin_enrollment()`),
/// y este binario decide *cómo* — un socket Unix acá, lo que corresponda
/// en Windows/macOS el día que existan.
fn spawn_enrollment_info_socket(
    runtime_public_key: Vec<u8>,
    token: String,
    expires_unix_ms: u64,
) -> anyhow::Result<()> {
    if let Some(parent) = Path::new(ENROLLMENT_INFO_SOCKET).parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let _ = std::fs::remove_file(ENROLLMENT_INFO_SOCKET);
    let listener = UnixListener::bind(ENROLLMENT_INFO_SOCKET)?;
    info!(
        socket = ENROLLMENT_INFO_SOCKET,
        "sirviendo info de enrolamiento para que vault-host arme el QR"
    );

    let pubkey_hex = hex::encode(&runtime_public_key);

    tokio::spawn(async move {
        loop {
            let (mut stream, _) = match listener.accept().await {
                Ok(v) => v,
                Err(e) => {
                    warn!(error = %e, "error aceptando en el socket de info de enrolamiento");
                    continue;
                }
            };
            let info = serde_json::json!({
                "runtime_pubkey_hex": pubkey_hex,
                "token": token,
                "expires_unix_ms": expires_unix_ms,
                "v": QR_PAYLOAD_VERSION,
            });
            use tokio::io::AsyncWriteExt;
            let _ = stream.write_all(info.to_string().as_bytes()).await;
        }
    });

    Ok(())
}
