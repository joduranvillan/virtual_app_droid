//! Negociador Multicodec con Fallback por Hardware (MediaCodec NDK).
//!
//! Evalúa las capacidades del SoC móvil/desktop:
//! 1. Prioridad: AV1 por hardware (bajísimo consumo / máxima compresión).
//! 2. Fallback 1: H.265 / HEVC Low-Latency (MediaCodec NDK).
//! 3. Fallback 2: VP9 Zero-Lag.
//! 4. Fallback 3: AVC / H.264 Universal.

use vault_protocol::services::{
    CodecNegotiationRequestPayload, CodecNegotiationResponsePayload, VideoCodecType,
};

pub struct CodecNegotiator;

impl CodecNegotiator {
    /// Negocia el mejor codec disponible basándose en las capacidades hardware del SoC.
    pub fn negotiate(req: &CodecNegotiationRequestPayload) -> CodecNegotiationResponsePayload {
        let supports_av1_hw = req.supported_hardware_codecs.contains(&VideoCodecType::Av1);
        let supports_hevc_hw = req
            .supported_hardware_codecs
            .contains(&VideoCodecType::HevcH265);
        let supports_vp9_hw = req
            .supported_hardware_codecs
            .contains(&VideoCodecType::Vp9ZeroLag);

        if req.preferred_codec == VideoCodecType::Av1 && supports_av1_hw {
            return CodecNegotiationResponsePayload {
                active_codec: VideoCodecType::Av1,
                is_hardware_accelerated: true,
                decoder_mime: "video/av01 (c2.android.av1.decoder)".to_string(),
                fallback_triggered: false,
                reason: "AV1 Hardware Decoder disponible en SoC".to_string(),
            };
        }

        // Si AV1 no está soportado por hardware, ejecutar fallback transparente
        if supports_hevc_hw {
            return CodecNegotiationResponsePayload {
                active_codec: VideoCodecType::HevcH265,
                is_hardware_accelerated: true,
                decoder_mime: "video/hevc (c2.qcom.hevc.decoder / MediaCodec NDK)".to_string(),
                fallback_triggered: true,
                reason: "AV1 HW no soportado por SoC -> Conmutado transparentemente a H.265 (HEVC Low-Latency NDK)".to_string(),
            };
        }

        if supports_vp9_hw {
            return CodecNegotiationResponsePayload {
                active_codec: VideoCodecType::Vp9ZeroLag,
                is_hardware_accelerated: true,
                decoder_mime: "video/x-vnd.on2.vp9 (c2.android.vp9.decoder)".to_string(),
                fallback_triggered: true,
                reason: "Conmutado a VP9 Zero-Lag MediaCodec NDK".to_string(),
            };
        }

        CodecNegotiationResponsePayload {
            active_codec: VideoCodecType::AvcH264,
            is_hardware_accelerated: true,
            decoder_mime: "video/avc (c2.android.avc.decoder)".to_string(),
            fallback_triggered: true,
            reason: "Fallback a AVC/H.264 Universal Hardware".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_negotiate_av1_when_supported() {
        let req = CodecNegotiationRequestPayload {
            supported_hardware_codecs: vec![VideoCodecType::Av1, VideoCodecType::HevcH265],
            preferred_codec: VideoCodecType::Av1,
            soc_model: "Snapdragon 8 Gen 3".to_string(),
            mediacodec_ndk_acceleration: true,
        };

        let res = CodecNegotiator::negotiate(&req);
        assert_eq!(res.active_codec, VideoCodecType::Av1);
        assert!(!res.fallback_triggered);
        assert!(res.is_hardware_accelerated);
    }

    #[test]
    fn test_fallback_to_hevc_when_av1_lacks_hw() {
        let req = CodecNegotiationRequestPayload {
            supported_hardware_codecs: vec![VideoCodecType::HevcH265, VideoCodecType::AvcH264],
            preferred_codec: VideoCodecType::Av1,
            soc_model: "Snapdragon 888".to_string(),
            mediacodec_ndk_acceleration: true,
        };

        let res = CodecNegotiator::negotiate(&req);
        assert_eq!(res.active_codec, VideoCodecType::HevcH265);
        assert!(res.fallback_triggered);
        assert!(res.reason.contains("H.265"));
    }
}
