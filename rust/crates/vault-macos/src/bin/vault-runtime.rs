//! `vault-runtime` (macOS) — corre dentro de la sesión o servicio seguro con el
//! almacenamiento cifrado montado. Se integra con `AppleKeychainSecretStore` para
//! cargar de forma segura la identidad criptográfica y el pin del frontend.
//!
//! Al igual que en Linux, la comunicación inter-proceso de red local en macOS
//! utiliza sockets de dominio UNIX locales (`/tmp/vault_runtime.sock` y
//! `/tmp/vault_enrollment_info.sock`) que ofrecen máximo aislamiento y seguridad.

use std::path::PathBuf;
use std::sync::Arc;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::Mutex;
use tracing::{error, info, warn};

use vault_core::{request_location, EnrollmentState, SecretStore};
use vault_crypto::{run_responder_handshake, NoiseChannel};
use vault_protocol::{
    EnrollmentAckBody, EnrollmentConfirmBody, Frame, LocationAccuracy, MsgType, QR_PAYLOAD_VERSION,
};
use vault_macos::AppleKeychainSecretStore;

const RUNTIME_SOCKET_PATH: &str = "/tmp/vault_runtime.sock";
const ENROLLMENT_SOCKET_PATH: &str = "/tmp/vault_enrollment_info.sock";

fn get_backup_identity_path() -> PathBuf {
    std::env::temp_dir().join("vault_macos_backup_identity.key")
}

fn get_backup_pin_path() -> PathBuf {
    std::env::temp_dir().join("vault_macos_backup_pinned.pub")
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    info!("vault-runtime macOS arrancando");

    // Limpiamos sockets previos si quedaron de ejecuciones anteriores
    let _ = std::fs::remove_file(RUNTIME_SOCKET_PATH);
    let _ = std::fs::remove_file(ENROLLMENT_SOCKET_PATH);

    // En macOS real, las llaves se guardan en el Keychain de Apple.
    // Pasamos rutas temporales de respaldo para la simulación multiplataforma (tests locales).
    let secret_store: Arc<dyn SecretStore> = Arc::new(AppleKeychainSecretStore::new(
        get_backup_identity_path(),
        get_backup_pin_path(),
    ));

    let keypair = secret_store.load_or_generate_identity()?;
    info!(
        public_key_hex = hex::encode(&keypair.public),
        "Identidad cargada/generada exitosamente en macOS"
    );

    let enrollment_state = Arc::new(Mutex::new(EnrollmentState::new(secret_store.clone())));

    if enrollment_state.lock().await.is_enrolled()? {
        info!("dispositivo frontend pineado encontrado — modo normal de sesión");
    } else {
        info!("ningún dispositivo pineado — activando canal de ENROLAMIENTO (QR)");
        let info = enrollment_state.lock().await.begin_enrollment();
        spawn_enrollment_info_server(keypair.public.clone(), info.token, info.expires_unix_ms)?;
    }

    let listener = UnixListener::bind(RUNTIME_SOCKET_PATH)?;
    info!(path = RUNTIME_SOCKET_PATH, "escuchando conexiones de vault-host por socket UNIX");

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
    info!("handshake completo — canal de sesión cifrado establecido");

    let mut chan = NoiseChannel::new(stream, transport);

    let already_enrolled = state.lock().await.is_enrolled()?;

    if !already_enrolled {
        if !run_enrollment_exchange(&mut chan, &state, &remote_pubkey).await? {
            warn!("enrolamiento rechazado o fallido — cerrando conexión");
            return Ok(());
        }
        info!("dispositivo enrolado exitosamente en macOS");
        return Ok(());
    }

    let pinned = state.lock().await.pinned_public_key()?;
    if pinned.as_deref() != Some(remote_pubkey.as_slice()) {
        warn!("clave pública remota no coincide con la pineada — rechazando conexión");
        return Ok(());
    }

    // Demo de RPC virtual para GPS
    match request_location(&mut chan, 1, "com.example.demo", LocationAccuracy::Fine).await {
        Ok(loc) => info!(
            lat = loc.latitude,
            lon = loc.longitude,
            accuracy_m = loc.accuracy_meters,
            "ubicación recibida del frontend en macOS"
        ),
        Err(e) => warn!(error = %e, "error obteniendo ubicación por RPC"),
    }

    Ok(())
}

async fn run_enrollment_exchange(
    chan: &mut NoiseChannel<UnixStream>,
    state: &Arc<Mutex<EnrollmentState>>,
    remote_pubkey: &[u8],
) -> anyhow::Result<bool> {
    let frame = chan.recv_frame().await?;
    if frame.msg_type != MsgType::EnrollmentConfirm {
        warn!(msg_type = ?frame.msg_type, "se esperaba confirmación de enrolamiento");
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
    let listener = std::os::unix::net::UnixListener::bind(ENROLLMENT_SOCKET_PATH)?;
    info!(
        path = ENROLLMENT_SOCKET_PATH,
        "sirviendo info de enrolamiento para vault-host"
    );

    let pubkey_hex = hex::encode(&runtime_public_key);

    tokio::spawn(async move {
        loop {
            let (mut stream, _) = match listener.accept() {
                Ok(v) => v,
                Err(e) => {
                    warn!(error = %e, "error aceptando en socket de enrolamiento");
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
