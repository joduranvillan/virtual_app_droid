//! Tipos de payload para video, audio e input — la parte del protocolo
//! que todavía no tiene ninguna implementación funcional detrás.
//!
//! **Alcance real de este crate, para no generar expectativas de más:**
//! esto define *qué forma tienen los datos* que viajarían por el canal
//! ya cifrado (dentro de `MsgType::VideoFrame`/`InputEvent` de
//! `vault-protocol`), con validación básica y tests de serialización.
//! No incluye:
//! - captura del framebuffer del Android Runtime (depende de la Fase 1
//!   / `AndroidHypervisor`, todavía sin implementar),
//! - codificación/decodificación real de H.265/H.264/Opus,
//! - la decisión de si el transporte final es QUIC datagram custom o se
//!   adapta el frontend WebRTC que ya trae Cuttlefish (ver
//!   ROADMAP_MULTIPLATFORM.md, hallazgo del spike de Linux) — esa
//!   decisión sigue abierta y bloqueada por lo mismo: no hay forma de
//!   comparar latencia/calidad real entre ambos caminos sin una VM de
//!   Android corriendo para generar frames de verdad.
//!
//! Es decir: esto es el "contrato de datos", no el pipeline.

pub mod audio;
pub mod input;
pub mod video;

pub use audio::{AudioCodec, AudioFrameError, AudioFramePayload};
pub use input::{InputEventError, InputEventPayload};
pub use video::{VideoCodec, VideoFrameError, VideoFramePayload};
