//! Implementación Linux de `vault_core::EncryptedStorage`: delega en el
//! binario `cryptsetup` del sistema para abrir/cerrar/formatear un
//! volumen LUKS2. Se ejecuta *solo* desde el binario `vault-runtime`
//! (dentro del namespace aislado), nunca desde `vault-host`.
//!
//! Nota: esto asume que el host tiene `cryptsetup` instalado y que el
//! proceso corre con las capabilities necesarias (CAP_SYS_ADMIN) dentro
//! de su propio namespace — configuración de despliegue, no de este crate.

use std::io::Write;
use std::process::{Command, Stdio};
use vault_core::{EncryptedStorage, MountedVolume, PlatformError, PlatformResult};

pub struct LuksEncryptedStorage {
    pub image_path: String,
    pub mapper_name: String,
}

impl LuksEncryptedStorage {
    pub fn new(image_path: impl Into<String>, mapper_name: impl Into<String>) -> Self {
        Self {
            image_path: image_path.into(),
            mapper_name: mapper_name.into(),
        }
    }

    fn mapper_device_path(&self) -> String {
        format!("/dev/mapper/{}", self.mapper_name)
    }

    fn run_cryptsetup(&self, args: &[&str], key: Option<&[u8]>) -> PlatformResult<()> {
        let mut command = Command::new("cryptsetup");
        command.args(args);
        if key.is_some() {
            command.stdin(Stdio::piped());
        }
        command.stdout(Stdio::piped()).stderr(Stdio::piped());

        let mut child = command.spawn()?;

        if let Some(key) = key {
            if let Some(mut stdin) = child.stdin.take() {
                stdin.write_all(key)?;
            }
        }

        let output = child.wait_with_output()?;
        if !output.status.success() {
            return Err(PlatformError::Other(format!(
                "cryptsetup terminó con error: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        Ok(())
    }
}

impl EncryptedStorage for LuksEncryptedStorage {
    /// Se usa una única vez, en el aprovisionamiento inicial de la
    /// bóveda (justo después de que se completa el primer enrolamiento),
    /// nunca en el arranque normal.
    fn format_new(&self, key: &[u8]) -> PlatformResult<()> {
        self.run_cryptsetup(
            &[
                "luksFormat",
                "--type",
                "luks2",
                "--batch-mode",
                "--key-file",
                "-",
                &self.image_path,
            ],
            Some(key),
        )
    }

    /// Abre el volumen pasando la clave por stdin (nunca por argv, para
    /// no dejarla en `/proc/*/cmdline`).
    fn open(&self, key: &[u8]) -> PlatformResult<MountedVolume> {
        self.run_cryptsetup(
            &[
                "open",
                "--type",
                "luks2",
                "--key-file",
                "-",
                &self.image_path,
                &self.mapper_name,
            ],
            Some(key),
        )?;

        Ok(MountedVolume {
            mount_path: self.mapper_device_path(),
            platform_handle: Box::new(self.mapper_name.clone()),
        })
    }

    fn close(&self, volume: MountedVolume) -> PlatformResult<()> {
        let mapper_name = volume
            .platform_handle
            .downcast_ref::<String>()
            .cloned()
            .unwrap_or_else(|| self.mapper_name.clone());

        self.run_cryptsetup(&["close", &mapper_name], None)
    }
}
