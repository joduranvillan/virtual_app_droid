//! Implementación Linux de los traits de `vault-core`: LUKS2 para
//! almacenamiento cifrado, archivos con permisos `0600`/`0700` para
//! secretos, y (todavía) un stub para el hipervisor Android.
//!
//! Los binarios reales (`vault-host`, `vault-runtime`) viven en
//! `src/bin/` y son los que combinan estas implementaciones con la
//! lógica de `vault-core`.

pub mod enrollment_http;
pub mod hypervisor;
pub mod lifecycle;
pub mod secret_store;
pub mod storage;

pub use hypervisor::{NotImplementedHypervisor, CrosvmAndroidHypervisor};
pub use secret_store::FileSecretStore;
pub use storage::LuksEncryptedStorage;
