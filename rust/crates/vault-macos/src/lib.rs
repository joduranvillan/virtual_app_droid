//! Implementación macOS de los traits de `vault-core`: Apple Keychain para
//! almacenamiento de secretos, Sparse Bundle cifrado para almacenamiento seguro,
//! y un stub para el hipervisor Android.
//!
//! Los binarios reales (`vault-host`, `vault-runtime`) viven en
//! `src/bin/` y combinan estas implementaciones con la lógica común.

pub mod hypervisor;
pub mod secret_store;
pub mod storage;

pub use hypervisor::{NotImplementedHypervisor, AppleVirtualizationHypervisor};
pub use secret_store::AppleKeychainSecretStore;
pub use storage::MacosEncryptedStorage;
