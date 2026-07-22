//! Implementación macOS de `vault_core::SecretStore` usando Apple Keychain.
//!
//! En macOS, los secretos persistentes (claves de identidad, PIN del frontend)
//! se almacenan de forma segura en el llavero del sistema de Apple (Keychain) a través
//! del framework nativo `Security`. Esto garantiza que los datos se cifren con claves
//! respaldadas por hardware (Secure Enclave en Macs modernas) y aisladas por usuario.
//!
//! Para posibilitar compilación cruzada y testing en entornos Linux/macOS sin hardware de Apple,
//! se incluye una capa condicional de simulación para sistemas no macOS.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use vault_core::{PlatformError, PlatformResult, SecretStore};
use vault_crypto::{generate_static_keypair, StaticKeypair};

const SERVICE_NAME: &str = "com.example.confidentialvault";
const ACCOUNT_IDENTITY: &str = "identity_keypair";
const ACCOUNT_PINNED_KEY: &str = "pinned_frontend";

const PRIVATE_LEN: usize = 32;
const PUBLIC_LEN: usize = 32;

pub struct AppleKeychainSecretStore {
    // Estas rutas de respaldo se usan SOLAMENTE en sistemas no macOS (simulación para desarrollo)
    backup_identity_path: PathBuf,
    backup_pin_path: PathBuf,
}

impl AppleKeychainSecretStore {
    pub fn new(backup_identity_path: impl Into<PathBuf>, backup_pin_path: impl Into<PathBuf>) -> Self {
        Self {
            backup_identity_path: backup_identity_path.into(),
            backup_pin_path: backup_pin_path.into(),
        }
    }
}

impl SecretStore for AppleKeychainSecretStore {
    fn load_or_generate_identity(&self) -> PlatformResult<StaticKeypair> {
        #[cfg(target_os = "macos")]
        {
            use security_framework::passwords::{find_generic_password, set_generic_password};

            match find_generic_password(SERVICE_NAME, ACCOUNT_IDENTITY) {
                Ok((bytes, _)) => {
                    if bytes.len() != PRIVATE_LEN + PUBLIC_LEN {
                        return Err(PlatformError::Other(format!(
                            "identidad del llavero de Apple tiene largo inválido: {} bytes (esperados {})",
                            bytes.len(),
                            PRIVATE_LEN + PUBLIC_LEN
                        )));
                    }
                    let private = bytes[..PRIVATE_LEN].to_vec();
                    let public = bytes[PRIVATE_LEN..].to_vec();
                    return Ok(StaticKeypair { private, public });
                }
                Err(e) if e.code() == -25300 => {
                    // errSecItemNotFound: no existe, lo generamos y persistimos
                    let keypair = generate_static_keypair().map_err(|err| {
                        PlatformError::Other(format!("no se pudo generar la identidad: {err}"))
                    })?;

                    let mut to_persist = Vec::with_capacity(PRIVATE_LEN + PUBLIC_LEN);
                    to_persist.extend_from_slice(&keypair.private);
                    to_persist.extend_from_slice(&keypair.public);

                    set_generic_password(SERVICE_NAME, ACCOUNT_IDENTITY, &to_persist).map_err(|err| {
                        PlatformError::Other(format!("error guardando identidad en Apple Keychain: {err}"))
                    })?;

                    return Ok(keypair);
                }
                Err(e) => {
                    return Err(PlatformError::Other(format!(
                        "error leyendo Apple Keychain: {e}"
                    )));
                }
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            // Simulación para Linux/Windows en desarrollo/tests
            if self.backup_identity_path.exists() {
                let bytes = fs::read(&self.backup_identity_path)?;
                if bytes.len() != PRIVATE_LEN + PUBLIC_LEN {
                    return Err(PlatformError::Other("largo de identidad simulada inválido".to_string()));
                }
                let private = bytes[..PRIVATE_LEN].to_vec();
                let public = bytes[PRIVATE_LEN..].to_vec();
                return Ok(StaticKeypair { private, public });
            }

            let keypair = generate_static_keypair()
                .map_err(|e| PlatformError::Other(format!("no se pudo generar identidad: {e}")))?;

            if let Some(parent) = self.backup_identity_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let mut to_persist = Vec::with_capacity(PRIVATE_LEN + PUBLIC_LEN);
            to_persist.extend_from_slice(&keypair.private);
            to_persist.extend_from_slice(&keypair.public);
            fs::write(&self.backup_identity_path, &to_persist)?;

            Ok(keypair)
        }
    }

    fn load_pinned_frontend_key(&self) -> PlatformResult<Option<Vec<u8>>> {
        #[cfg(target_os = "macos")]
        {
            use security_framework::passwords::find_generic_password;

            match find_generic_password(SERVICE_NAME, ACCOUNT_PINNED_KEY) {
                Ok((bytes, _)) => Ok(Some(bytes)),
                Err(e) if e.code() == -25300 => Ok(None), // errSecItemNotFound
                Err(e) => Err(PlatformError::Other(format!(
                    "error leyendo pin del llavero de Apple: {e}"
                ))),
            }
        }

        #[cfg(not(target_os = "macos"))]
        {
            // Simulación para Linux/Windows en desarrollo/tests
            if !self.backup_pin_path.exists() {
                return Ok(None);
            }
            let key = fs::read(&self.backup_pin_path)?;
            Ok(Some(key))
        }
    }

    fn persist_pinned_frontend_key(&self, key: &[u8]) -> PlatformResult<()> {
        #[cfg(target_os = "macos")]
        {
            use security_framework::passwords::set_generic_password;
            // set_generic_password pisa/actualiza de forma transparente si ya existe
            set_generic_password(SERVICE_NAME, ACCOUNT_PINNED_KEY, key).map_err(|e| {
                PlatformError::Other(format!("error guardando pin en Apple Keychain: {e}"))
            })?;
            Ok(())
        }

        #[cfg(not(target_os = "macos"))]
        {
            // Simulación para Linux/Windows en desarrollo/tests
            if let Some(parent) = self.backup_pin_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            fs::write(&self.backup_pin_path, key)?;
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "vault_macos_test_{name}_{}_{}",
            std::process::id(),
            name.len()
        ))
    }

    #[test]
    fn generates_and_persists_identity_on_first_call() {
        let identity_path = temp_path("identity_first");
        let _ = fs::remove_file(&identity_path);
        let store = AppleKeychainSecretStore::new(identity_path.clone(), temp_path("pin_unused_1"));

        let kp = store.load_or_generate_identity().unwrap();
        assert_eq!(kp.private.len(), 32);
        assert_eq!(kp.public.len(), 32);

        // En simulación debe haberse escrito el archivo
        #[cfg(not(target_os = "macos"))]
        assert!(identity_path.exists());

        let _ = fs::remove_file(&identity_path);
    }

    #[test]
    fn returns_same_identity_across_calls() {
        let identity_path = temp_path("identity_same");
        let _ = fs::remove_file(&identity_path);
        let store = AppleKeychainSecretStore::new(identity_path.clone(), temp_path("pin_unused_2"));

        let first = store.load_or_generate_identity().unwrap();
        let second = store.load_or_generate_identity().unwrap();
        assert_eq!(first.private, second.private);
        assert_eq!(first.public, second.public);

        let _ = fs::remove_file(&identity_path);
    }

    #[test]
    fn pinned_key_round_trips() {
        let pin_path = temp_path("pin_roundtrip");
        let _ = fs::remove_file(&pin_path);
        let store = AppleKeychainSecretStore::new(temp_path("identity_unused_1"), pin_path.clone());

        assert_eq!(store.load_pinned_frontend_key().unwrap(), None);

        let key = b"clave-publica-del-telefono-macos".to_vec();
        store.persist_pinned_frontend_key(&key).unwrap();
        assert_eq!(store.load_pinned_frontend_key().unwrap(), Some(key));

        let _ = fs::remove_file(&pin_path);
    }
}
