//! Framing binario de mensajes sobre el stream cifrado por Noise.
//!
//! Formato en cable:
//! ```text
//! ┌──────────┬──────────┬──────────────┬─────────────────┐
//! │ msg_type │ req_id   │ payload_len  │ payload (CBOR)   │
//! │  u8      │  u64 BE  │  u32 BE      │  variable        │
//! └──────────┴──────────┴──────────────┴─────────────────┘
//! ```
//! Este framing va *dentro* del canal ya cifrado por Noise transport;
//! no aporta confidencialidad por sí mismo, solo delimita mensajes.

use bytes::{Buf, BufMut, BytesMut};
use thiserror::Error;

pub const HEADER_LEN: usize = 1 + 8 + 4;
pub const MAX_PAYLOAD_LEN: u32 = 16 * 1024 * 1024; // 16 MiB, cota defensiva

#[derive(Debug, Error)]
pub enum FramingError {
    #[error("payload excede el máximo permitido ({0} bytes)")]
    PayloadTooLarge(u32),
    #[error("buffer incompleto, se necesitan más bytes")]
    Incomplete,
    #[error("error de serialización CBOR: {0}")]
    Cbor(#[from] serde_cbor::Error),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MsgType {
    HandshakeInit = 0x01,
    HandshakeResp = 0x02,
    ServiceRequest = 0x10,
    ServiceResponse = 0x11,
    VideoFrame = 0x20,
    InputEvent = 0x21,
    ApkInstallRequest = 0x30,
    ApkInstallDecision = 0x31,
    EnrollmentConfirm = 0x40,
    EnrollmentAck = 0x41,
    Heartbeat = 0xF0,
}

impl MsgType {
    pub fn from_u8(v: u8) -> Option<Self> {
        use MsgType::*;
        Some(match v {
            0x01 => HandshakeInit,
            0x02 => HandshakeResp,
            0x10 => ServiceRequest,
            0x11 => ServiceResponse,
            0x20 => VideoFrame,
            0x21 => InputEvent,
            0x30 => ApkInstallRequest,
            0x31 => ApkInstallDecision,
            0x40 => EnrollmentConfirm,
            0x41 => EnrollmentAck,
            0xF0 => Heartbeat,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone)]
pub struct Frame {
    pub msg_type: MsgType,
    pub req_id: u64,
    pub payload: Vec<u8>,
}

impl Frame {
    pub fn new_serde<T: serde::Serialize>(
        msg_type: MsgType,
        req_id: u64,
        body: &T,
    ) -> Result<Self, FramingError> {
        let payload = serde_cbor::to_vec(body)?;
        Ok(Self {
            msg_type,
            req_id,
            payload,
        })
    }

    pub fn decode_body<T: serde::de::DeserializeOwned>(&self) -> Result<T, FramingError> {
        Ok(serde_cbor::from_slice(&self.payload)?)
    }

    pub fn encode(&self, out: &mut BytesMut) -> Result<(), FramingError> {
        let len = self.payload.len();
        if len as u64 > MAX_PAYLOAD_LEN as u64 {
            return Err(FramingError::PayloadTooLarge(len as u32));
        }
        out.reserve(HEADER_LEN + len);
        out.put_u8(self.msg_type as u8);
        out.put_u64(self.req_id);
        out.put_u32(len as u32);
        out.put_slice(&self.payload);
        Ok(())
    }

    /// Intenta decodificar un frame desde el inicio de `buf`. Si hay
    /// suficientes bytes, consume el frame de `buf` y lo devuelve.
    pub fn try_decode(buf: &mut BytesMut) -> Result<Option<Frame>, FramingError> {
        if buf.len() < HEADER_LEN {
            return Ok(None);
        }
        let msg_type_byte = buf[0];
        let req_id = u64::from_be_bytes(buf[1..9].try_into().unwrap());
        let payload_len = u32::from_be_bytes(buf[9..13].try_into().unwrap());

        if payload_len > MAX_PAYLOAD_LEN {
            return Err(FramingError::PayloadTooLarge(payload_len));
        }
        let total_len = HEADER_LEN + payload_len as usize;
        if buf.len() < total_len {
            return Ok(None);
        }

        let msg_type = MsgType::from_u8(msg_type_byte).ok_or(FramingError::Incomplete)?;
        buf.advance(HEADER_LEN);
        let payload = buf.split_to(payload_len as usize).to_vec();

        Ok(Some(Frame {
            msg_type,
            req_id,
            payload,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Ping {
        n: u32,
    }

    #[test]
    fn roundtrip_encode_decode() {
        let frame = Frame::new_serde(MsgType::Heartbeat, 42, &Ping { n: 7 }).unwrap();
        let mut buf = BytesMut::new();
        frame.encode(&mut buf).unwrap();

        let decoded = Frame::try_decode(&mut buf).unwrap().unwrap();
        assert_eq!(decoded.req_id, 42);
        assert_eq!(decoded.msg_type, MsgType::Heartbeat);
        let body: Ping = decoded.decode_body().unwrap();
        assert_eq!(body, Ping { n: 7 });
        assert!(buf.is_empty());
    }

    #[test]
    fn incomplete_buffer_returns_none() {
        let frame = Frame::new_serde(MsgType::Heartbeat, 1, &Ping { n: 1 }).unwrap();
        let mut full = BytesMut::new();
        frame.encode(&mut full).unwrap();

        let mut partial = BytesMut::from(&full[..full.len() - 1]);
        assert!(Frame::try_decode(&mut partial).unwrap().is_none());
        // el buffer no debe haberse consumido
        assert_eq!(partial.len(), full.len() - 1);
    }

    #[test]
    fn rejects_oversized_payload_len() {
        let mut buf = BytesMut::new();
        buf.put_u8(MsgType::Heartbeat as u8);
        buf.put_u64(1);
        buf.put_u32(MAX_PAYLOAD_LEN + 1);
        let err = Frame::try_decode(&mut buf).unwrap_err();
        assert!(matches!(err, FramingError::PayloadTooLarge(_)));
    }
}
