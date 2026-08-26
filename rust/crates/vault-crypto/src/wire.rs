//! Conecta el handshake Noise (`handshake.rs`) y el framing de protocolo
//! (`vault_protocol::framing`) sobre cualquier stream async
//! (`AsyncRead + AsyncWrite`), sea un `UnixStream` en `vault_runtime` o,
//! del lado Kotlin, el equivalente sobre un socket QUIC/TCP.
//!
//! Formato en el stream subyacente (antes de llegar acá, `vault_host`
//! solo ve estos bytes opacos y los reenvía sin tocarlos):
//!
//! ```text
//! [u32 BE len][ ...len bytes de mensaje Noise (handshake o transporte)... ]
//! ```

use bytes::BytesMut;
use thiserror::Error;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::handshake::{HandshakeError, Role, VaultHandshake, VaultTransport};
use vault_protocol::{Frame, FramingError};

pub const MAX_WIRE_MESSAGE_LEN: u32 = 16 * 1024 * 1024 + 1024; // payload máx + overhead Noise

#[derive(Debug, Error)]
pub enum WireError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("handshake: {0}")]
    Handshake(#[from] HandshakeError),
    #[error("framing: {0}")]
    Framing(#[from] FramingError),
    #[error("mensaje excede el máximo permitido en el wire ({0} bytes)")]
    MessageTooLarge(u32),
    #[error("frame incompleto tras desencriptar")]
    IncompleteFrame,
}

async fn write_len_prefixed<W: AsyncWrite + Unpin>(w: &mut W, bytes: &[u8]) -> Result<(), WireError> {
    if bytes.len() as u32 > MAX_WIRE_MESSAGE_LEN {
        return Err(WireError::MessageTooLarge(bytes.len() as u32));
    }
    w.write_u32(bytes.len() as u32).await?;
    w.write_all(bytes).await?;
    w.flush().await?;
    Ok(())
}

async fn read_len_prefixed<R: AsyncRead + Unpin>(r: &mut R) -> Result<Vec<u8>, WireError> {
    let len = r.read_u32().await?;
    if len > MAX_WIRE_MESSAGE_LEN {
        return Err(WireError::MessageTooLarge(len));
    }
    let mut buf = vec![0u8; len as usize];
    r.read_exact(&mut buf).await?;
    Ok(buf)
}

/// Ejecuta el lado responder del handshake Noise_XX (tres mensajes) sobre
/// un stream ya conectado. Devuelve el estado de transporte listo para
/// cifrar/descifrar frames de aplicación, junto con la clave pública
/// estática que presentó el otro extremo — es lo que hay que verificar
/// contra el pin guardado (o contra el token de enrolamiento en el
/// primer pairing) antes de confiar en la sesión.
pub async fn run_responder_handshake<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    local_static_private: &[u8],
) -> Result<(VaultTransport, Vec<u8>), WireError> {
    let mut hs = VaultHandshake::new(Role::Responder, local_static_private)?;
    let mut buf = [0u8; 1024];

    // <- e
    let msg = read_len_prefixed(stream).await?;
    hs.read_message(&msg, &mut buf)?;

    // -> e, ee, s, es
    let len = hs.write_message(&[], &mut buf)?;
    write_len_prefixed(stream, &buf[..len]).await?;

    // <- s, se
    let msg = read_len_prefixed(stream).await?;
    hs.read_message(&msg, &mut buf)?;

    let remote_pubkey = hs
        .remote_static_public_key()
        .ok_or(WireError::Handshake(HandshakeError::NotFinished))?;
    let transport = hs.into_transport()?;
    Ok((transport, remote_pubkey))
}

/// Lado initiator del handshake (usado por el frontend; se incluye acá
/// también para poder testear el roundtrip completo en este crate, y
/// como referencia 1:1 de lo que debe implementar el cliente Kotlin).
pub async fn run_initiator_handshake<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    local_static_private: &[u8],
) -> Result<(VaultTransport, Vec<u8>), WireError> {
    let mut hs = VaultHandshake::new(Role::Initiator, local_static_private)?;
    let mut buf = [0u8; 1024];

    // -> e
    let len = hs.write_message(&[], &mut buf)?;
    write_len_prefixed(stream, &buf[..len]).await?;

    // <- e, ee, s, es
    let msg = read_len_prefixed(stream).await?;
    hs.read_message(&msg, &mut buf)?;

    // -> s, se
    let len = hs.write_message(&[], &mut buf)?;
    write_len_prefixed(stream, &buf[..len]).await?;

    let remote_pubkey = hs
        .remote_static_public_key()
        .ok_or(WireError::Handshake(HandshakeError::NotFinished))?;
    let transport = hs.into_transport()?;
    Ok((transport, remote_pubkey))
}

/// Canal de aplicación ya autenticado y cifrado: envía/recibe `Frame`s.
pub struct NoiseChannel<S> {
    stream: S,
    transport: VaultTransport,
}

impl<S: AsyncRead + AsyncWrite + Unpin> NoiseChannel<S> {
    pub fn new(stream: S, transport: VaultTransport) -> Self {
        Self { stream, transport }
    }

    pub async fn send_frame(&mut self, frame: &Frame) -> Result<(), WireError> {
        let mut plain = BytesMut::new();
        frame.encode(&mut plain)?;
        // el ciphertext de Noise agrega hasta 16 bytes de tag de autenticación
        let mut ct = vec![0u8; plain.len() + 16];
        let ct_len = self.transport.encrypt(&plain, &mut ct)?;
        write_len_prefixed(&mut self.stream, &ct[..ct_len]).await?;
        Ok(())
    }

    pub async fn recv_frame(&mut self) -> Result<Frame, WireError> {
        let ct = read_len_prefixed(&mut self.stream).await?;
        let mut pt = vec![0u8; ct.len()];
        let pt_len = self.transport.decrypt(&ct, &mut pt)?;
        let mut buf = BytesMut::from(&pt[..pt_len]);
        Frame::try_decode(&mut buf)?.ok_or(WireError::IncompleteFrame)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handshake::generate_static_keypair;
    use tokio::net::UnixStream;
    use vault_protocol::{LocationAccuracy, LocationRequest, MsgType};

    #[tokio::test]
    async fn full_handshake_and_frame_roundtrip_over_unix_socket() {
        let (mut a, mut b) = UnixStream::pair().unwrap();

        let initiator_kp = generate_static_keypair().unwrap();
        let responder_kp = generate_static_keypair().unwrap();

        let initiator_priv = initiator_kp.private.clone();
        let responder_priv = responder_kp.private.clone();

        let (initiator_transport, responder_transport) = tokio::join!(
            run_initiator_handshake(&mut a, &initiator_priv),
            run_responder_handshake(&mut b, &responder_priv),
        );
        let (initiator_transport, responder_remote_pub) = initiator_transport.unwrap();
        let (responder_transport, initiator_remote_pub) = responder_transport.unwrap();

        // cada lado debe haber visto la clave pública real del otro
        assert_eq!(responder_remote_pub, responder_kp.public);
        assert_eq!(initiator_remote_pub, initiator_kp.public);

        let mut initiator_chan = NoiseChannel::new(a, initiator_transport);
        let mut responder_chan = NoiseChannel::new(b, responder_transport);

        let req = LocationRequest {
            accuracy: LocationAccuracy::Fine,
        };
        let frame = Frame::new_serde(MsgType::ServiceRequest, 1, &req).unwrap();

        initiator_chan.send_frame(&frame).await.unwrap();
        let received = responder_chan.recv_frame().await.unwrap();

        assert_eq!(received.msg_type, MsgType::ServiceRequest);
        let body: LocationRequest = received.decode_body().unwrap();
        assert_eq!(body.accuracy, LocationAccuracy::Fine);
    }
}
