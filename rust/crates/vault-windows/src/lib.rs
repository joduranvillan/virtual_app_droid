//! Implementación Windows de los traits de `vault-core`: DPAPI para
//! almacenamiento de secretos, BitLocker / VHDX para almacenamiento cifrado,
//! y un stub para el hipervisor Android.
//!
//! Los binarios reales (`vault-host`, `vault-runtime`) viven en
//! `src/bin/` y son los que combinan estas implementaciones con la
//! lógica de `vault-core`.

pub mod hypervisor;
pub mod secret_store;
pub mod storage;
pub mod service_manager;

pub use hypervisor::{NotImplementedHypervisor, HyperVAndroidHypervisor};
pub use secret_store::DpapiSecretStore;
pub use storage::WindowsEncryptedStorage;
pub use service_manager::run_service;
