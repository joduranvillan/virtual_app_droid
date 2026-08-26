//! Tipos de payload para `MsgType::VideoFrame` (definido en
//! `vault-protocol`). Este crate NO codifica ni decodifica video de
//! verdad — son los tipos que viajan por el wire una vez que algo
//! (todavía sin implementar, depende de la Fase 1 / `AndroidHypervisor`)
//! produce frames codificados desde el framebuffer del Android Runtime.
//!
//! Se eligieron coordenadas normalizadas (`x`/`y` en `[0.0, 1.0]`) en
//! `input.rs`, no píxeles absolutos, para que el frontend no tenga que
//! saber la resolución exacta del framebuffer remoto — importante
//! porque esa resolución puede cambiar (rotación, distintos tamaños de
//! pantalla entre dispositivos) sin que el protocolo tenga que
//! renegociarla.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoCodec {
    /// Preferido: mejor compresión, pero requiere que el hipervisor
    /// elegido (`crosvm`/`ARCVM`, ver ROADMAP_MULTIPLATFORM.md) exponga
    /// un encoder por hardware o que se pague el costo de software.
    H265,
    /// Fallback más compatible — casi cualquier decoder en Android lo
    /// soporta sin depender de extensiones, a costa de peor compresión
    /// para la misma calidad.
    H264,
}

#[derive(Debug, Error)]
pub enum VideoFrameError {
    #[error("dimensiones inválidas: {width}x{height} (ninguna puede ser 0)")]
    InvalidDimensions { width: u16, height: u16 },
    #[error("el payload de datos está vacío")]
    EmptyData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoFramePayload {
    pub codec: VideoCodec,
    /// Los decoders necesitan saber esto para poder empezar a decodificar
    /// en medio de un stream (ej. tras una reconexión) sin esperar al
    /// próximo keyframe si ya se tiene uno bufferizado.
    pub is_keyframe: bool,
    /// Número de secuencia monótono creciente — permite detectar frames
    /// perdidos o fuera de orden en el transporte, independientemente
    /// del `req_id` del `Frame` que lo contiene.
    pub sequence: u64,
    pub timestamp_unix_ms: u64,
    pub width: u16,
    pub height: u16,
    /// Bitstream crudo del encoder (Annex-B para H.264/H.265). Viaja
    /// como byte-string CBOR real, no como array de enteros — mismo
    /// motivo que en `vault_protocol::services::ServiceRequestEnvelope`.
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

impl VideoFramePayload {
    pub fn new(
        codec: VideoCodec,
        is_keyframe: bool,
        sequence: u64,
        timestamp_unix_ms: u64,
        width: u16,
        height: u16,
        data: Vec<u8>,
    ) -> Result<Self, VideoFrameError> {
        if width == 0 || height == 0 {
            return Err(VideoFrameError::InvalidDimensions { width, height });
        }
        if data.is_empty() {
            return Err(VideoFrameError::EmptyData);
        }
        Ok(Self {
            codec,
            is_keyframe,
            sequence,
            timestamp_unix_ms,
            width,
            height,
            data,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_zero_dimensions() {
        let result = VideoFramePayload::new(VideoCodec::H265, true, 0, 0, 0, 720, vec![1, 2, 3]);
        assert!(matches!(result, Err(VideoFrameError::InvalidDimensions { .. })));
    }

    #[test]
    fn rejects_empty_data() {
        let result = VideoFramePayload::new(VideoCodec::H265, true, 0, 0, 1280, 720, vec![]);
        assert!(matches!(result, Err(VideoFrameError::EmptyData)));
    }

    #[test]
    fn roundtrips_through_cbor_as_real_bytestring() {
        let frame =
            VideoFramePayload::new(VideoCodec::H265, true, 42, 1_700_000_000_000, 1280, 720, vec![0xDE, 0xAD, 0xBE, 0xEF])
                .unwrap();
        let bytes = serde_cbor::to_vec(&frame).unwrap();
        let back: VideoFramePayload = serde_cbor::from_slice(&bytes).unwrap();
        assert_eq!(back.data, vec![0xDE, 0xAD, 0xBE, 0xEF]);
        assert_eq!(back.sequence, 42);
        assert!(back.is_keyframe);
    }
}
