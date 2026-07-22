//! Implementación Windows de `vault_core::SecretStore` usando DPAPI (Data Protection API).
//!
//! En Windows, los secretos se cifran usando `CryptProtectData`, lo que vincula
//! la clave a las credenciales del usuario actual (en producción, la cuenta de sistema
//! del servicio de Windows) o de la máquina local. Esto ofrece una protección real
//! en reposo sin necesidad de almacenar una clave maestra adicional en el disco.
//!
//! Para posibilitar que compilen y pasen los tests en entornos Linux/macOS sin Windows,
//! se incluye una capa condicional de simulación en sistemas no Windows.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use vault_core::{PlatformError, PlatformResult, SecretStore};
use vault_crypto::{generate_static_keypair, StaticKeypair};

const PRIVATE_LEN: usize = 32;
const PUBLIC_LEN: usize = 32;

pub struct DpapiSecretStore {
    identity_path: PathBuf,
    pin_path: PathBuf,
}

impl DpapiSecretStore {
    pub fn new(identity_path: impl Into<PathBuf>, pin_path: impl Into<PathBuf>) -> Self {
        Self {
            identity_path: identity_path.into(),
            pin_path: pin_path.into(),
        }
    }
}

impl SecretStore for DpapiSecretStore {
    fn load_or_generate_identity(&self) -> PlatformResult<StaticKeypair> {
        if self.identity_path.exists() {
            let bytes = load_secret_file(&self.identity_path)?;
            if bytes.len() != PRIVATE_LEN + PUBLIC_LEN {
                return Err(PlatformError::Other(format!(
                    "el archivo de identidad descifrado tiene un largo inválido: {} bytes (se esperaban {})",
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
        let hex_bytes = load_secret_file(&self.pin_path)?;
        let hex_str = String::from_utf8(hex_bytes)
            .map_err(|e| PlatformError::Other(format!("pin no es UTF-8 válido: {e}")))?;
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
    }
    let protected = protect_data(contents)
        .map_err(|e| PlatformError::Other(format!("error cifrando con DPAPI: {e}")))?;
    fs::write(path, protected)?;
    Ok(())
}

fn load_secret_file(path: &Path) -> PlatformResult<Vec<u8>> {
    let bytes = fs::read(path)?;
    let unprotected = unprotect_data(&bytes)
        .map_err(|e| PlatformError::Other(format!("error descifrando con DPAPI: {e}")))?;
    Ok(unprotected)
}

#[cfg(windows)]
fn protect_data(data: &[u8]) -> io::Result<Vec<u8>> {
    use std::ptr;
    use windows_sys::Win32::Security::Cryptography::{CryptProtectData, CRYPTPROTECT_UI_FORBIDDEN, DATA_BLOB};
    use windows_sys::Win32::System::Memory::LocalFree;

    let mut input = DATA_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr() as *mut u8,
    };
    let mut output = DATA_BLOB {
        cbData: 0,
        pbData: ptr::null_mut(),
    };

    let success = unsafe {
        CryptProtectData(
            &mut input,
            ptr::null(), // Sin descripción
            ptr::null(), // Sin entropía adicional
            ptr::null_mut(), // Reservado
            ptr::null_mut(), // Estructura de prompt de UI
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };

    if success == 0 {
        return Err(io::Error::last_os_error());
    }

    let result = unsafe {
        std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec()
    };

    unsafe {
        LocalFree(output.pbData as _);
    }

    Ok(result)
}

#[cfg(windows)]
fn unprotect_data(data: &[u8]) -> io::Result<Vec<u8>> {
    use std::ptr;
    use windows_sys::Win32::Security::Cryptography::{CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, DATA_BLOB};
    use windows_sys::Win32::System::Memory::LocalFree;

    let mut input = DATA_BLOB {
        cbData: data.len() as u32,
        pbData: data.as_ptr() as *mut u8,
    };
    let mut output = DATA_BLOB {
        cbData: 0,
        pbData: ptr::null_mut(),
    };

    let success = unsafe {
        CryptUnprotectData(
            &mut input,
            ptr::null_mut(), // Sin descripción
            ptr::null(), // Sin entropía
            ptr::null_mut(), // Reservado
            ptr::null_mut(), // Estructura de prompt
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };

    if success == 0 {
        return Err(io::Error::last_os_error());
    }

    let result = unsafe {
        std::slice::from_raw_parts(output.pbData, output.cbData as usize).to_vec()
    };

    unsafe {
        LocalFree(output.pbData as _);
    }

    Ok(result)
}

#[cfg(not(windows))]
fn protect_data(data: &[u8]) -> io::Result<Vec<u8>> {
    // Simulación simple para no-Windows (XOR para ocultamiento en testing)
    Ok(data.iter().map(|&b| b ^ 0xAA).collect())
}

#[cfg(not(windows))]
fn unprotect_data(data: &[u8]) -> io::Result<Vec<u8>> {
    Ok(data.iter().map(|&b| b ^ 0xAA).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_path(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "vault_windows_test_{name}_{}_{}",
            std::process::id(),
            name.len()
        ))
    }

    #[test]
    fn generates_and_persists_identity_on_first_call() {
        let identity_path = temp_path("identity_first");
        let _ = fs::remove_file(&identity_path);
        let store = DpapiSecretStore::new(identity_path.clone(), temp_path("pin_unused_1"));

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
        let store = DpapiSecretStore::new(identity_path.clone(), temp_path("pin_unused_2"));

        let first = store.load_or_generate_identity().unwrap();
        let second = store.load_or_generate_identity().unwrap();
        assert_eq!(first.private, second.private);
        assert_eq!(first.public, second.public);

        let _ = fs::remove_file(&identity_path);
    }

    #[test]
    fn rejects_corrupted_identity_file() {
        let identity_path = temp_path("identity_corrupt");
        fs::write(&identity_path, b"no son 64 bytes de clave cifrada").unwrap();
        let store = DpapiSecretStore::new(identity_path.clone(), temp_path("pin_unused_3"));

        assert!(store.load_or_generate_identity().is_err());

        let _ = fs::remove_file(&identity_path);
    }

    #[test]
    fn pinned_key_round_trips() {
        let pin_path = temp_path("pin_roundtrip");
        let _ = fs::remove_file(&pin_path);
        let store = DpapiSecretStore::new(temp_path("identity_unused_1"), pin_path.clone());

        assert_eq!(store.load_pinned_frontend_key().unwrap(), None);

        let key = b"clave-publica-del-telefono-windows".to_vec();
        store.persist_pinned_frontend_key(&key).unwrap();
        assert_eq!(store.load_pinned_frontend_key().unwrap(), Some(key));

        let _ = fs::remove_file(&pin_path);
    }
}
