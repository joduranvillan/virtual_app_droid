//! Handshake Noise_XX, derivación de claves y framing sobre streams
//! async. Deliberadamente sin nada específico de un sistema operativo:
//! ni rutas de archivo, ni llamadas a binarios externos, ni sockets
//! Unix. Eso vive en el crate de cada plataforma (`vault-linux`,
//! `vault-windows`, `vault-macos`), que implementa los traits de
//! `vault-core` usando lo que ofrezca `vault-crypto` acá.

pub mod handshake;
pub mod keys;
pub mod pairing;
pub mod wire;

pub use handshake::{generate_static_keypair, HandshakeError, Role, StaticKeypair, VaultHandshake, VaultTransport};
pub use keys::{derive_subkey, derive_vault_unlock_key, VAULT_UNLOCK_KEY_LEN};
pub use pairing::{generate_pairing_token, now_unix_ms};
pub use wire::{run_initiator_handshake, run_responder_handshake, NoiseChannel, WireError};
