//! Implementación Linux de `vault_core::SecretStore`.
//!
//! **Sobre el nivel real de protección acá:** esto NO es "encriptación
//! en reposo" en ningún sentido fuerte — son archivos en texto plano con
//! permisos `0600` (solo el usuario dueño del proceso puede leerlos) y
//! un directorio `0700` alrededor. Eso alcanza para que otro usuario sin
//! privilegios en la misma máquina no pueda leerlos, y para que una
//! copia de backup mal configurada no los exponga por accidente — pero
//! CUALQUIER proceso con acceso root en el host, o acceso directo al
//! disco, los lee sin esfuerzo. Es exactamente el mismo nivel de
//! protección que usa, por ejemplo, una clave de host de SSH
//! (`/etc/ssh/ssh_host_ed25519_key`, también `0600`).
//!
//! La protección real contra un host comprometido requiere sellar estos
//! secretos en un TPM (`tpm2_seal`) — eso sigue siendo trabajo pendiente
//! (ver ARCHITECTURE.md). Envolver estos archivos con una segunda clave
//! derivada de OTRO archivo en el mismo disco no sería más seguro: solo
//! movería el problema, no lo resolvería. Por eso se optó por ser
//! honestos con permisos de OS en vez de "encriptación de juguete" que
//! da una falsa sensación de seguridad.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use vault_core::{PlatformError, PlatformResult, SecretStore};
use vault_crypto::{generate_static_keypair, StaticKeypair};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

const PRIVATE_LEN: usize = 32;
const PUBLIC_LEN: usize = 32;

pub struct FileSecretStore {
    identity_path: PathBuf,
    pin_path: PathBuf,
}

impl FileSecretStore {
    pub fn new(identity_path: impl Into<PathBuf>, pin_path: impl Into<PathBuf>) -> Self {
        Self {
            identity_path: identity_path.into(),
            pin_path: pin_path.into(),
        }
    }
}

impl SecretStore for FileSecretStore {
    /// Carga la identidad persistida en `identity_path`, o genera una
    /// nueva y la persiste si el archivo no existe todavía (primer
    /// arranque). Sin esto, cada arranque regeneraba el par de claves y
    /// todo pairing anterior quedaba inválido.
    fn load_or_generate_identity(&self) -> PlatformResult<StaticKeypair> {
        if self.identity_path.exists() {
            let bytes = fs::read(&self.identity_path)?;
            if bytes.len() != PRIVATE_LEN + PUBLIC_LEN {
                return Err(PlatformError::Other(format!(
                    "el archivo de identidad tiene un largo inválido: {} bytes (se esperaban {})",
                    bytes.len(),
                    PRIVATE_LEN + PUBLIC_LEN
                )));
            }
            let private = bytes[..PRIVATE_LEN].to_vec();
            let public = bytes[PRIVATE_LEN..].to_vec();
            return Ok(StaticKeypair { private, public });
        }

        let keypair = generate_static_keypair()
            .map_err(|e| PlatformError::Other(format!("no se pudo generar la identidad: {e}")))?;

        // Se guardan privada y pública concatenadas (`priv(32) || pub(32)`)
        // en vez de recalcular la pública a partir de la privada al
        // cargar: la API pública de `snow` no expone el tipo `Dh25519`
        // necesario para hacer esa derivación por fuera de un handshake
        // completo, y sumar otra dependencia (`x25519-dalek`) solo para
        // esto no valía la pena.
        let mut to_persist = Vec::with_capacity(PRIVATE_LEN + PUBLIC_LEN);
        to_persist.extend_from_slice(&keypair.private);
        to_persist.extend_from_slice(&keypair.public);
        persist_secret_file(&self.identity_path, &to_persist)?;

        Ok(keypair)
    }

    fn load_pinned_frontend_key(&self) -> PlatformResult<Option<Vec<u8>>> {
        if !self.pin_path.exists() {
            return Ok(None);
        }
        let hex_str = fs::read_to_string(&self.pin_path)?;
        let key = hex::decode(hex_str.trim())
            .map_err(|e| PlatformError::Other(format!("pin corrupto: {e}")))?;
        Ok(Some(key))
    }

    fn persist_pinned_frontend_key(&self, key: &[u8]) -> PlatformResult<()> {
        persist_secret_file(&self.pin_path, hex::encode(key).as_bytes())
    }
}

fn persist_secret_file(path: &Path, contents: &[u8]) -> PlatformResult<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
        harden_secret_directory(parent)?;
    }
    fs::write(path, contents)?;
    harden_secret_file(path)?;
    Ok(())
}

#[cfg(unix)]
fn harden_secret_directory(path: &Path) -> io::Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))
}

#[cfg(unix)]
fn harden_secret_file(path: &Path) -> io::Result<()> {
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn harden_secret_directory(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(not(unix))]
fn harden_secret_file(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "vault_linux_test_{name}_{}_{}",
            std::process::id(),
            name.len()
        ))
    }

    #[test]
    fn generates_and_persists_identity_on_first_call() {
        let identity_path = temp_path("identity_first");
        let _ = fs::remove_file(&identity_path);
        let store = FileSecretStore::new(identity_path.clone(), temp_path("pin_unused_1"));

        let kp = store.load_or_generate_identity().unwrap();
        assert!(identity_path.exists());
        assert_eq!(kp.private.len(), 32);
        assert_eq!(kp.public.len(), 32);

        let _ = fs::remove_file(&identity_path);
    }

    #[test]
    fn returns_same_identity_across_calls() {
        let identity_path = temp_path("identity_same");
        let _ = fs::remove_file(&identity_path);
        let store = FileSecretStore::new(identity_path.clone(), temp_path("pin_unused_2"));

        let first = store.load_or_generate_identity().unwrap();
        let second = store.load_or_generate_identity().unwrap();
        assert_eq!(first.private, second.private);
        assert_eq!(first.public, second.public);

        let _ = fs::remove_file(&identity_path);
    }

    #[cfg(unix)]
    #[test]
    fn identity_file_has_owner_only_permissions() {
        let identity_path = temp_path("identity_perms");
        let _ = fs::remove_file(&identity_path);
        let store = FileSecretStore::new(identity_path.clone(), temp_path("pin_unused_3"));

        store.load_or_generate_identity().unwrap();
        let mode = fs::metadata(&identity_path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);

        let _ = fs::remove_file(&identity_path);
    }

    #[test]
    fn rejects_corrupted_identity_file() {
        let identity_path = temp_path("identity_corrupt");
        fs::write(&identity_path, b"no son 64 bytes de clave").unwrap();
        let store = FileSecretStore::new(identity_path.clone(), temp_path("pin_unused_4"));

        assert!(store.load_or_generate_identity().is_err());

        let _ = fs::remove_file(&identity_path);
    }

    #[test]
    fn pinned_key_round_trips() {
        let pin_path = temp_path("pin_roundtrip");
        let _ = fs::remove_file(&pin_path);
        let store = FileSecretStore::new(temp_path("identity_unused_1"), pin_path.clone());

        assert_eq!(store.load_pinned_frontend_key().unwrap(), None);

        let key = b"clave-publica-del-telefono".to_vec();
        store.persist_pinned_frontend_key(&key).unwrap();
        assert_eq!(store.load_pinned_frontend_key().unwrap(), Some(key));

        let _ = fs::remove_file(&pin_path);
    }

    #[cfg(unix)]
    #[test]
    fn pin_file_has_owner_only_permissions() {
        let pin_path = temp_path("pin_perms");
        let _ = fs::remove_file(&pin_path);
        let store = FileSecretStore::new(temp_path("identity_unused_2"), pin_path.clone());

        store.persist_pinned_frontend_key(b"pubkey").unwrap();
        let mode = fs::metadata(&pin_path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);

        let _ = fs::remove_file(&pin_path);
    }
}
