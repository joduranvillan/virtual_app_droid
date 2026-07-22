//! Los tres puntos de extensión que separan la lógica de negocio (acá,
//! en `vault-core`) de cómo cada sistema operativo la implementa
//! (`vault-linux`, `vault-windows`, `vault-macos`).
//!
//! Diseño deliberado: los tres traits son **síncronos**, no `async`.
//! Las operaciones que representan (formatear un volumen, leer/escribir
//! un archivo de secreto, arrancar una VM) son inherentemente
//! bloqueantes a nivel de sistema operativo de todas formas — envolver
//! eso en `async fn` en el trait hubiera obligado a sumar la
//! dependencia `async-trait` (boxing de futures) sin ganar nada real.
//! El código que llama a estos traits desde un contexto async (los
//! binarios de cada plataforma) los ejecuta con `tokio::task::spawn_blocking`
//! cuando corresponde.

use std::io;
use thiserror::Error;
use vault_crypto::StaticKeypair;

#[derive(Debug, Error)]
pub enum PlatformError {
    #[error("io: {0}")]
    Io(#[from] io::Error),
    #[error("{0}")]
    Other(String),
}

pub type PlatformResult<T> = Result<T, PlatformError>;

/// Opaco a propósito: cada plataforma decide qué necesita guardar sobre
/// un volumen abierto (un path de dispositivo mapeado en Linux, un
/// identificador de VHDX en Windows, etc.) — `vault-core` no necesita
/// saber el contenido, solo pasarlo de vuelta a `close()`.
pub struct MountedVolume {
    pub mount_path: String,
    /// Datos específicos de la plataforma, opacos para `vault-core`.
    pub platform_handle: Box<dyn std::any::Any + Send + Sync>,
}

/// Abre/cierra/formatea el volumen cifrado que contiene el filesystem
/// del Android Runtime. LUKS2 en Linux, BitLocker/VHDX en Windows, un
/// volumen APFS cifrado en macOS.
pub trait EncryptedStorage: Send + Sync {
    fn format_new(&self, key: &[u8]) -> PlatformResult<()>;
    fn open(&self, key: &[u8]) -> PlatformResult<MountedVolume>;
    fn close(&self, volume: MountedVolume) -> PlatformResult<()>;
}

/// Arranca/detiene el runtime Android aislado y expone el canal por el
/// que `vault-core` habla con él. Firecracker+Android-x86 en Linux,
/// Hyper-V en Windows, Hypervisor.framework en macOS.
///
/// **Estado actual: sin implementación real en ninguna plataforma
/// todavía** (Fase 1 del roadmap, ver ARCHITECTURE.md). El trait se
/// define acá para que el resto del sistema (enrolamiento, RPC de
/// servicios) pueda escribirse contra la interfaz ya, pero cualquier
/// implementación existente hoy es un stub que documenta el hueco en
/// vez de fingir que funciona.
pub trait AndroidHypervisor: Send + Sync {
    fn boot(&self, volume: &MountedVolume) -> PlatformResult<RunningInstance>;
    fn stop(&self, instance: RunningInstance) -> PlatformResult<()>;
}

pub struct RunningInstance {
    pub platform_handle: Box<dyn std::any::Any + Send + Sync>,
}

/// Dónde y cómo se protege la identidad estática de `vault-runtime` y el
/// pin de la clave del frontend. La implementación mínima honesta es
/// "archivo con permisos restrictivos del sistema operativo" (ver
/// `vault-linux::secret_store` para el detalle de qué protege eso y qué
/// no) — evoluciona a TPM (Linux/Windows) o Secure Enclave/Keychain
/// (macOS) sin que el resto del sistema tenga que cambiar una línea.
pub trait SecretStore: Send + Sync {
    /// Carga la identidad persistida, o genera una nueva si es la
    /// primera vez que corre `vault-runtime` en esta máquina.
    fn load_or_generate_identity(&self) -> PlatformResult<StaticKeypair>;

    /// `None` si todavía no se completó ningún enrolamiento.
    fn load_pinned_frontend_key(&self) -> PlatformResult<Option<Vec<u8>>>;

    fn persist_pinned_frontend_key(&self, key: &[u8]) -> PlatformResult<()>;
}
