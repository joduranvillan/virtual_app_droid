//! `vault-runtime` (Windows) — corre dentro de la sesión o servicio seguro con el
//! almacenamiento cifrado montado. Es el responsable de terminar la conexión Noise_XX,
//! almacenar de manera segura la identidad y el pin del frontend (vía DpapiSecretStore),
//! y despachar las llamadas RPC.
//!
//! En Windows, la comunicación inter-proceso local se realiza mediante sockets TCP locales
//! en la interfaz Loopback (127.0.0.1) para máxima compatibilidad con todos los entornos.

use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Mutex;
use tracing::{debug, error, info, warn};

use vault_core::{request_location, EnrollmentState, SecretStore};
use vault_crypto::{run_responder_handshake, NoiseChannel};
use vault_protocol::{
    EnrollmentAckBody, EnrollmentConfirmBody, Frame, LocationAccuracy, MsgType, QR_PAYLOAD_VERSION,
    ServiceRequestEnvelope,
};
use vault_windows::DpapiSecretStore;

const RUNTIME_LISTEN_ADDR: &str = "127.0.0.1:7444";
const ENROLLMENT_INFO_ADDR: &str = "127.0.0.1:7445";

fn get_identity_path() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from("C:\\ProgramData\\ConfidentialVault\\runtime_identity.key")
    } else {
        std::env::temp_dir().join("vault_windows_runtime_identity.key")
    }
}

fn get_pin_path() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from("C:\\ProgramData\\ConfidentialVault\\pinned_frontend.pub")
    } else {
        std::env::temp_dir().join("vault_windows_pinned_frontend.pub")
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let args: Vec<String> = std::env::args().collect();
    let is_service = args.iter().any(|arg| arg == "--service");

    if is_service {
        info!("Ejecutando vault-runtime como Windows Service nativo...");
        vault_windows::run_service("ConfidentialVaultRuntime", move || {
            let rt = tokio::runtime::Runtime::new().expect("No se pudo iniciar el runtime de Tokio para el servicio");
            rt.block_on(async {
                if let Err(e) = run_runtime_server().await {
                    tracing::error!("Error ejecutando el servidor de runtime en el servicio: {:?}", e);
                }
            });
        })?;
    } else {
        info!("vault-runtime Windows arrancando en modo interactivo...");
        run_runtime_server().await?;
    }

    Ok(())
}

async fn run_runtime_server() -> anyhow::Result<()> {
    let identity_path = get_identity_path();
    let pin_path = get_pin_path();

    info!(
        identity_path = %identity_path.display(),
        pin_path = %pin_path.display(),
        "Iniciando servidores de vault-runtime"
    );

    let secret_store: Arc<dyn SecretStore> =
        Arc::new(DpapiSecretStore::new(identity_path, pin_path));

    let keypair = secret_store.load_or_generate_identity()?;
    info!(
        public_key_hex = hex::encode(&keypair.public),
        "Identidad cargada/generada exitosamente"
    );

    let enrollment_state = Arc::new(Mutex::new(EnrollmentState::new(secret_store.clone())));

    if enrollment_state.lock().await.is_enrolled()? {
        info!("ya hay un frontend pineado — modo normal, solo esa clave puede conectarse");
    } else {
        info!("sin frontend pineado — entrando en modo ENROLLING");
        let info = enrollment_state.lock().await.begin_enrollment();
        spawn_enrollment_info_server(keypair.public.clone(), info.token, info.expires_unix_ms)?;
    }

    let listener = TcpListener::bind(RUNTIME_LISTEN_ADDR).await?;
    info!(addr = RUNTIME_LISTEN_ADDR, "escuchando conexiones locales de vault-host");

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
    mut stream: TcpStream,
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
        info!("nuevo dispositivo enrolado exitosamente en Windows");
        return Ok(());
    }

    let pinned = state.lock().await.pinned_public_key()?;
    if pinned.as_deref() != Some(remote_pubkey.as_slice()) {
        warn!("clave pública remota NO coincide con la pineada — rechazando conexión");
        return Ok(());
    }

    // Demo RPC virtual GPS service
    match request_location(&mut chan, 1, "com.example.demo", LocationAccuracy::Fine).await {
        Ok(loc) => info!(
            lat = loc.latitude,
            lon = loc.longitude,
            accuracy_m = loc.accuracy_meters,
            "ubicación recibida del frontend en Windows"
        ),
        Err(e) => warn!(error = %e, "no se pudo obtener ubicación"),
    }

    info!("esperando pedidos o frames del frontend en Windows (modo sesión activo)...");
    loop {
        let frame = match chan.recv_frame().await {
            Ok(f) => f,
            Err(e) => {
                info!("conexión de sesión cerrada por el frontend o error en Windows: {}", e);
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
                info!(service = ?request_envelope.service, package = %request_envelope.requesting_package, "recibida petición de servicio en Windows");
                
                let response_envelope = vault_core::services::dispatch_service_request(request_envelope);
                let resp_frame = match Frame::new_serde(MsgType::ServiceResponse, frame.req_id, &response_envelope) {
                    Ok(f) => f,
                    Err(e) => {
                        warn!("error serializando ServiceResponseEnvelope: {}", e);
                        continue;
                    }
                };
                if let Err(e) = chan.send_frame(&resp_frame).await {
                    warn!("error enviando respuesta de servicio en Windows: {}", e);
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
                debug!(msg_type = ?frame.msg_type, "frame ignorado en el loop de sesión de Windows");
            }
        }
    }

    Ok(())
}

async fn run_enrollment_exchange(
    chan: &mut NoiseChannel<TcpStream>,
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

fn spawn_enrollment_info_server(
    runtime_public_key: Vec<u8>,
    token: String,
    expires_unix_ms: u64,
) -> anyhow::Result<()> {
    let listener = std::net::TcpListener::bind(ENROLLMENT_INFO_ADDR)?;
    info!(
        addr = ENROLLMENT_INFO_ADDR,
        "sirviendo info de enrolamiento para que vault-host arme el QR"
    );

    let pubkey_hex = hex::encode(&runtime_public_key);

    tokio::spawn(async move {
        loop {
            let (mut stream, _) = match listener.accept() {
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
            use std::io::Write;
            let _ = stream.write_all(info.to_string().as_bytes());
        }
    });

    Ok(())
}
