//! Tipos de payload para el canal de audio. No hay `MsgType` propio en
//! `vault-protocol` todavía para esto — se agregaría un `MsgType::AudioFrame`
//! cuando se implemente de verdad; se define acá primero porque el tipo
//! de payload no depende de esa decisión de framing.
//!
//! Opus, no un codec sin comprimir ni algo como AAC: es el estándar de
//! facto para audio de baja latencia en videollamadas/streaming
//! interactivo (WebRTC lo usa por default), con buen soporte tanto en
//! codificación por software liviana como en decoders de Android.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioCodec {
    Opus,
}

#[derive(Debug, Error)]
pub enum AudioFrameError {
    #[error("sample_rate_hz inválido: {0} (valores típicos: 8000/16000/24000/48000)")]
    InvalidSampleRate(u32),
    #[error("channels inválido: {0} (se espera 1 = mono o 2 = estéreo)")]
    InvalidChannels(u8),
    #[error("el payload de datos está vacío")]
    EmptyData,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioFramePayload {
    pub codec: AudioCodec,
    pub sequence: u64,
    pub timestamp_unix_ms: u64,
    pub sample_rate_hz: u32,
    pub channels: u8,
    #[serde(with = "serde_bytes")]
    pub data: Vec<u8>,
}

impl AudioFramePayload {
    pub fn new(
        codec: AudioCodec,
        sequence: u64,
        timestamp_unix_ms: u64,
        sample_rate_hz: u32,
        channels: u8,
        data: Vec<u8>,
    ) -> Result<Self, AudioFrameError> {
        // Opus solo define estas tasas de muestreo estándar; validar acá
        // temprano evita mandar algo que ningún decoder Opus va a aceptar.
        const VALID_RATES: [u32; 5] = [8000, 12000, 16000, 24000, 48000];
        if !VALID_RATES.contains(&sample_rate_hz) {
            return Err(AudioFrameError::InvalidSampleRate(sample_rate_hz));
        }
        if channels == 0 || channels > 2 {
            return Err(AudioFrameError::InvalidChannels(channels));
        }
        if data.is_empty() {
            return Err(AudioFrameError::EmptyData);
        }
        Ok(Self {
            codec,
            sequence,
            timestamp_unix_ms,
            sample_rate_hz,
            channels,
            data,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_standard_opus_sample_rates() {
        for rate in [8000, 12000, 16000, 24000, 48000] {
            assert!(AudioFramePayload::new(AudioCodec::Opus, 0, 0, rate, 2, vec![1]).is_ok());
        }
    }

    #[test]
    fn rejects_non_opus_sample_rate() {
        let result = AudioFramePayload::new(AudioCodec::Opus, 0, 0, 44100, 2, vec![1]);
        assert!(matches!(result, Err(AudioFrameError::InvalidSampleRate(44100))));
    }

    #[test]
    fn rejects_invalid_channel_count() {
        assert!(AudioFramePayload::new(AudioCodec::Opus, 0, 0, 48000, 0, vec![1]).is_err());
        assert!(AudioFramePayload::new(AudioCodec::Opus, 0, 0, 48000, 3, vec![1]).is_err());
    }

    #[test]
    fn roundtrips_through_cbor_as_real_bytestring() {
        let frame = AudioFramePayload::new(AudioCodec::Opus, 5, 1000, 48000, 2, vec![0xAA, 0xBB]).unwrap();
        let bytes = serde_cbor::to_vec(&frame).unwrap();
        let back: AudioFramePayload = serde_cbor::from_slice(&bytes).unwrap();
        assert_eq!(back.data, vec![0xAA, 0xBB]);
    }
}
