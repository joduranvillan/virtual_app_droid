//! Implementación macOS de `vault_core::EncryptedStorage`.
//!
//! En macOS, el almacenamiento de datos confidenciales del Android Runtime se implementa
//! utilizando imágenes de disco virtual cifradas en formato **Sparse Bundle** con cifrado
//! AES-256 de nivel de sistema y el sistema de archivos de Apple **APFS** (o HFS+).
//!
//! Se automatiza por completo usando comandos de la herramienta nativa del sistema **`hdiutil`**:
//! - `hdiutil create`: Genera el contenedor cifrado dinámico de 4GB.
//! - `hdiutil attach`: Desbloquea y monta el contenedor virtual.
//! - `hdiutil detach`: Desmonta y bloquea el contenedor, protegiendo los datos en reposo.
//!
//! Para sistemas que no son macOS, provee una implementación simulada que permite correr
//! todos los tests unitarios de integración en el entorno de desarrollo Linux.

use std::process::{Command, Stdio};
use std::io::Write;
use vault_core::{EncryptedStorage, MountedVolume, PlatformError, PlatformResult};

pub struct MacosEncryptedStorage {
    pub sparsebundle_path: String,
    pub mount_point: String, // ej. "/Volumes/ConfidentialVault"
}

impl MacosEncryptedStorage {
    pub fn new(sparsebundle_path: impl Into<String>, mount_point: impl Into<String>) -> Self {
        Self {
            sparsebundle_path: sparsebundle_path.into(),
            mount_point: mount_point.into(),
        }
    }

    #[cfg(target_os = "macos")]
    fn run_hdiutil_with_key(&self, args: &[&str], key: &[u8]) -> PlatformResult<String> {
        // En macOS pasamos la contraseña por stdin de forma segura para evitar filtraciones en logs de procesos
        let mut child = Command::new("hdiutil")
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        {
            let stdin = child.stdin.as_mut().ok_or_else(|| {
                PlatformError::Other("no se pudo abrir stdin de hdiutil".to_string())
            })?;
            // Escribimos la contraseña en formato hex o plano. Dado que hdiutil toma la pass en plano,
            // pasamos la contraseña (clave de 256 bits codificada en hexadecimal para consistencia)
            let key_hex = hex::encode(key);
            stdin.write_all(key_hex.as_bytes())?;
            stdin.write_all(b"\n")?;
        }

        let output = child.wait_with_output()?;
        if !output.status.success() {
            return Err(PlatformError::Other(format!(
                "hdiutil terminó con error: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

impl EncryptedStorage for MacosEncryptedStorage {
    fn format_new(&self, key: &[u8]) -> PlatformResult<()> {
        #[cfg(target_os = "macos")]
        {
            // hdiutil create -size 4g -fs APFS -volname ConfidentialVault -encryption AES-256 -stdinpass path
            let args = [
                "create",
                "-size",
                "4g",
                "-fs",
                "APFS",
                "-volname",
                "ConfidentialVault",
                "-encryption",
                "AES-256",
                "-stdinpass",
                &self.sparsebundle_path,
            ];
            self.run_hdiutil_with_key(&args, key)?;
            Ok(())
        }

        #[cfg(not(target_os = "macos"))]
        {
            // Simulación en Linux/Windows
            std::fs::File::create(&self.sparsebundle_path)?;
            Ok(())
        }
    }

    fn open(&self, key: &[u8]) -> PlatformResult<MountedVolume> {
        #[cfg(target_os = "macos")]
        {
            // hdiutil attach -stdinpass -mountpoint path sparsebundle
            let args = [
                "attach",
                "-stdinpass",
                "-mountpoint",
                &self.mount_point,
                &self.sparsebundle_path,
            ];
            self.run_hdiutil_with_key(&args, key)?;

            Ok(MountedVolume {
                mount_path: self.mount_point.clone(),
                platform_handle: Box::new(self.sparsebundle_path.clone()),
            })
        }

        #[cfg(not(target_os = "macos"))]
        {
            // Simulación en Linux/Windows
            let dummy_mount = format!("{}_mounted_dir", self.sparsebundle_path);
            std::fs::create_dir_all(&dummy_mount)?;
            Ok(MountedVolume {
                mount_path: dummy_mount,
                platform_handle: Box::new(self.sparsebundle_path.clone()),
            })
        }
    }

    fn close(&self, volume: MountedVolume) -> PlatformResult<()> {
        let _path = volume
            .platform_handle
            .downcast_ref::<String>()
            .cloned()
            .unwrap_or_else(|| self.sparsebundle_path.clone());

        #[cfg(target_os = "macos")]
        {
            // hdiutil detach mountpoint
            let output = Command::new("hdiutil")
                .args(&["detach", &self.mount_point, "-force"])
                .output()?;

            if !output.status.success() {
                return Err(PlatformError::Other(format!(
                    "error desmontando sparsebundle en macOS: {}",
                    String::from_utf8_lossy(&output.stderr)
                )));
            }

            Ok(())
        }

        #[cfg(not(target_os = "macos"))]
        {
            // Simulación en Linux/Windows
            let dummy_mount = format!("{}_mounted_dir", _path);
            let _ = std::fs::remove_dir_all(&dummy_mount);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_image_path(name: &str) -> String {
        std::env::temp_dir()
            .join(format!("vault_macos_test_{name}.sparsebundle"))
            .to_string_lossy()
            .to_string()
    }

    #[test]
    fn format_open_close_roundtrip() {
        let image = temp_image_path("storage_roundtrip");
        let storage = MacosEncryptedStorage::new(&image, "/Volumes/ConfidentialVaultTest");

        let key = b"esta-es-una-clave-de-256-bits-super-segura-12345".to_vec();

        // Debe formatear
        storage.format_new(&key).unwrap();

        // Debe poder abrir
        let vol = storage.open(&key).unwrap();
        assert!(!vol.mount_path.is_empty());

        // Debe poder cerrar
        storage.close(vol).unwrap();

        let _ = std::fs::remove_file(&image);
    }
}
