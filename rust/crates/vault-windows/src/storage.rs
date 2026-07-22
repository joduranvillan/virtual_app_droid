//! Implementación Windows de `vault_core::EncryptedStorage`.
//!
//! En Windows, el almacenamiento seguro para el Android Runtime se implementa
//! creando un disco virtual VHDX que luego se cifra con BitLocker (usando `manage-bde.exe`
//! o PowerShell cmdlets como `Enable-BitLocker`).
//!
//! En sistemas no Windows, provee una implementación simulada que permite correr
//! tests unitarios de integración en el entorno de desarrollo.

use std::process::Command;
use vault_core::{EncryptedStorage, MountedVolume, PlatformError, PlatformResult};

pub struct WindowsEncryptedStorage {
    pub vhdx_path: String,
    pub drive_letter: String, // ej. "V:"
}

impl WindowsEncryptedStorage {
    pub fn new(vhdx_path: impl Into<String>, drive_letter: impl Into<String>) -> Self {
        Self {
            vhdx_path: vhdx_path.into(),
            drive_letter: drive_letter.into(),
        }
    }

    #[cfg(windows)]
    fn run_powershell(&self, script: &str) -> PlatformResult<String> {
        let output = Command::new("powershell")
            .args(&["-NoProfile", "-Command", script])
            .output()?;

        if !output.status.success() {
            return Err(PlatformError::Other(format!(
                "PowerShell terminó con error: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    #[cfg(windows)]
    fn run_manage_bde(&self, args: &[&str]) -> PlatformResult<()> {
        let output = Command::new("manage-bde.exe")
            .args(args)
            .output()?;

        if !output.status.success() {
            return Err(PlatformError::Other(format!(
                "manage-bde.exe terminó con error: {}",
                String::from_utf8_lossy(&output.stderr)
            )));
        }
        Ok(())
    }
}

impl EncryptedStorage for WindowsEncryptedStorage {
    fn format_new(&self, key: &[u8]) -> PlatformResult<()> {
        #[cfg(windows)]
        {
            use std::io::Write;
            // 1. Crear el disco virtual VHDX
            let size_mb = 4096; // 4 GB default para el runtime
            let create_script = format!(
                "New-VHD -Path '{}' -SizeBytes {}MB -Dynamic -Confirm:$false",
                self.vhdx_path, size_mb
            );
            self.run_powershell(&create_script)?;

            // 2. Montar/Adjuntar disco para formatearlo
            let mount_script = format!(
                "Mount-DiskImage -ImagePath '{}' -StorageType VHDX",
                self.vhdx_path
            );
            self.run_powershell(&mount_script)?;

            // 3. Inicializar y formatear el disco con NTFS y asignarle la letra
            let init_script = format!(
                "Get-DiskImage -ImagePath '{}' | Get-Disk | Initialize-Disk -PartitionStyle GPT -PassThru | \
                 New-Partition -DriveLetter {} -UseMaximumSize | \
                 Format-Volume -FileSystem NTFS -NewFileSystemLabel 'ConfidentialVault' -Confirm:$false",
                self.vhdx_path, self.drive_letter.chars().next().unwrap()
            );
            self.run_powershell(&init_script)?;

            // 4. Activar BitLocker sobre el volumen nuevo
            // Para pasar la clave de forma segura, la convertimos a un SecureString en PowerShell
            // o usamos manage-bde con stdin/pipe si es posible. Dado que manage-bde pide contraseña de forma interactiva,
            // podemos usar PowerShell Enable-BitLocker con Password:
            let key_str = hex::encode(key);
            let bitlocker_script = format!(
                "$secpasswd = ConvertTo-SecureString '{}' -AsPlainText -Force; \
                 Enable-BitLocker -MountPoint '{}' -EncryptionMethod XtsAes256 -PasswordProtector -Password $secpasswd -SkipHardwareTest",
                key_str, self.drive_letter
            );
            self.run_powershell(&bitlocker_script)?;

            // 5. Desmontar para dejarlo seguro
            let dismount_script = format!(
                "Dismount-DiskImage -ImagePath '{}'",
                self.vhdx_path
            );
            self.run_powershell(&dismount_script)?;

            Ok(())
        }

        #[cfg(not(windows))]
        {
            // Simulación en Linux/macOS
            std::fs::File::create(&self.vhdx_path)?;
            Ok(())
        }
    }

    fn open(&self, key: &[u8]) -> PlatformResult<MountedVolume> {
        #[cfg(windows)]
        {
            // 1. Montar disco virtual (estará bloqueado por BitLocker)
            let mount_script = format!(
                "Mount-DiskImage -ImagePath '{}' -StorageType VHDX",
                self.vhdx_path
            );
            self.run_powershell(&mount_script)?;

            // 2. Desbloquear volumen usando BitLocker con la contraseña (llave)
            let key_str = hex::encode(key);
            self.run_manage_bde(&[
                "-unlock",
                &self.drive_letter,
                "-Password",
                &key_str,
            ])?;

            Ok(MountedVolume {
                mount_path: format!("{}\\", self.drive_letter),
                platform_handle: Box::new(self.vhdx_path.clone()),
            })
        }

        #[cfg(not(windows))]
        {
            // Simulación en Linux/macOS
            let dummy_mount = format!("{}_mounted_dir", self.vhdx_path);
            std::fs::create_dir_all(&dummy_mount)?;
            Ok(MountedVolume {
                mount_path: dummy_mount,
                platform_handle: Box::new(self.vhdx_path.clone()),
            })
        }
    }

    fn close(&self, volume: MountedVolume) -> PlatformResult<()> {
        let _vhdx_path = volume
            .platform_handle
            .downcast_ref::<String>()
            .cloned()
            .unwrap_or_else(|| self.vhdx_path.clone());

        #[cfg(windows)]
        {
            // 1. Bloquear el volumen primero
            self.run_manage_bde(&["-lock", &self.drive_letter])?;

            // 2. Desmontar la imagen de disco virtual
            let dismount_script = format!(
                "Dismount-DiskImage -ImagePath '{}'",
                _vhdx_path
            );
            self.run_powershell(&dismount_script)?;

            Ok(())
        }

        #[cfg(not(windows))]
        {
            // Simulación en Linux/macOS
            let dummy_mount = format!("{}_mounted_dir", _vhdx_path);
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
            .join(format!("vault_windows_test_{name}.vhdx"))
            .to_string_lossy()
            .to_string()
    }

    #[test]
    fn format_open_close_roundtrip() {
        let image = temp_image_path("storage_roundtrip");
        let storage = WindowsEncryptedStorage::new(&image, "V:");

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
