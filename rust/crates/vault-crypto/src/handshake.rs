//! Wrapper sobre `snow` para el handshake Noise_XX entre el frontend
//! Android (initiator) y `vault_runtime` (responder — NO `vault_host`,
//! ver ARCHITECTURE.md §5).
//!
//! Noise_XX se eligió porque:
//! - Da autenticación mutua (ambas partes prueban posesión de su clave
//!   estática) sin necesitar una PKI previa completa: las claves públicas
//!   se intercambian y verifican dentro del propio handshake ("pairing"),
//!   y en producción se combinan con un paso de verificación fuera de
//!   banda (ej. QR o número de confirmación) en el primer enrolamiento.
//! - Da forward secrecy: las claves de sesión no comprometen sesiones
//!   pasadas si se filtran las claves estáticas.

use snow::{Builder, HandshakeState, TransportState};
use thiserror::Error;

const NOISE_PATTERN: &str = "Noise_XX_25519_ChaChaPoly_SHA256";

#[derive(Debug, Error)]
pub enum HandshakeError {
    #[error("error de snow: {0}")]
    Snow(#[from] snow::Error),
    #[error("el handshake todavía no terminó")]
    NotFinished,
}

pub struct StaticKeypair {
    pub private: Vec<u8>,
    pub public: Vec<u8>,
}

/// Genera un par de claves estáticas X25519. En el frontend Android esto
/// se hace idealmente respaldado por StrongBox/hardware keystore; acá se
/// deja como generación en software con el builder de snow, y queda a
/// cargo de la integración final decidir dónde vive la privada.
pub fn generate_static_keypair() -> Result<StaticKeypair, HandshakeError> {
    let builder = Builder::new(NOISE_PATTERN.parse().expect("patrón Noise válido"));
    let kp = builder.generate_keypair()?;
    Ok(StaticKeypair {
        private: kp.private,
        public: kp.public,
    })
}

pub enum Role {
    Initiator,
    Responder,
}

pub struct VaultHandshake {
    state: HandshakeState,
}

impl VaultHandshake {
    pub fn new(role: Role, local_static_private: &[u8]) -> Result<Self, HandshakeError> {
        let builder = Builder::new(NOISE_PATTERN.parse().expect("patrón Noise válido"))
            .local_private_key(local_static_private);
        let state = match role {
            Role::Initiator => builder.build_initiator()?,
            Role::Responder => builder.build_responder()?,
        };
        Ok(Self { state })
    }

    /// Escribe el siguiente mensaje de handshake saliente en `out_buf`,
    /// devuelve la cantidad de bytes escritos.
    pub fn write_message(&mut self, payload: &[u8], out_buf: &mut [u8]) -> Result<usize, HandshakeError> {
        Ok(self.state.write_message(payload, out_buf)?)
    }

    /// Procesa un mensaje de handshake entrante.
    pub fn read_message(&mut self, msg: &[u8], out_buf: &mut [u8]) -> Result<usize, HandshakeError> {
        Ok(self.state.read_message(msg, out_buf)?)
    }

    pub fn is_finished(&self) -> bool {
        self.state.is_handshake_finished()
    }

    /// Clave pública estática remota, disponible una vez recibido el
    /// segundo mensaje del patrón XX. Se usa para verificar contra el
    /// pin/pairing conocido antes de confiar en la sesión.
    pub fn remote_static_public_key(&self) -> Option<Vec<u8>> {
        self.state.get_remote_static().map(|k| k.to_vec())
    }

    /// Handshake hash final — insumo para derivar claves de aplicación
    /// (ver `keys::derive_vault_unlock_material`).
    pub fn handshake_hash(&self) -> Vec<u8> {
        self.state.get_handshake_hash().to_vec()
    }

    /// Convierte el estado en modo transporte (cifrado de datos). Consume
    /// el handshake; solo se puede llamar una vez terminado.
    pub fn into_transport(self) -> Result<VaultTransport, HandshakeError> {
        if !self.state.is_handshake_finished() {
            return Err(HandshakeError::NotFinished);
        }
        let transport = self.state.into_transport_mode()?;
        Ok(VaultTransport { transport })
    }
}

pub struct VaultTransport {
    transport: TransportState,
}

impl VaultTransport {
    pub fn encrypt(&mut self, plaintext: &[u8], out_buf: &mut [u8]) -> Result<usize, HandshakeError> {
        Ok(self.transport.write_message(plaintext, out_buf)?)
    }

    pub fn decrypt(&mut self, ciphertext: &[u8], out_buf: &mut [u8]) -> Result<usize, HandshakeError> {
        Ok(self.transport.read_message(ciphertext, out_buf)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_xx_handshake_and_transport_roundtrip() {
        let initiator_kp = generate_static_keypair().unwrap();
        let responder_kp = generate_static_keypair().unwrap();

        let mut initiator = VaultHandshake::new(Role::Initiator, &initiator_kp.private).unwrap();
        let mut responder = VaultHandshake::new(Role::Responder, &responder_kp.private).unwrap();

        let mut buf_a = [0u8; 1024];
        let mut buf_b = [0u8; 1024];

        // -> e
        let len = initiator.write_message(&[], &mut buf_a).unwrap();
        responder.read_message(&buf_a[..len], &mut buf_b).unwrap();

        // <- e, ee, s, es
        let len = responder.write_message(&[], &mut buf_b).unwrap();
        initiator.read_message(&buf_b[..len], &mut buf_a).unwrap();

        // -> s, se
        let len = initiator.write_message(&[], &mut buf_a).unwrap();
        responder.read_message(&buf_a[..len], &mut buf_b).unwrap();

        assert!(initiator.is_finished());
        assert!(responder.is_finished());

        // ambas partes deben ver la clave pública real de la otra
        assert_eq!(
            initiator.remote_static_public_key().unwrap(),
            responder_kp.public
        );
        assert_eq!(
            responder.remote_static_public_key().unwrap(),
            initiator_kp.public
        );

        // los handshake hashes deben coincidir — insumo para derivar claves
        assert_eq!(initiator.handshake_hash(), responder.handshake_hash());

        let mut a_transport = initiator.into_transport().unwrap();
        let mut b_transport = responder.into_transport().unwrap();

        let mut ct = [0u8; 256];
        let mut pt = [0u8; 256];
        let msg = b"hola desde el frontend";
        let ct_len = a_transport.encrypt(msg, &mut ct).unwrap();
        let pt_len = b_transport.decrypt(&ct[..ct_len], &mut pt).unwrap();
        assert_eq!(&pt[..pt_len], msg);
    }
}
