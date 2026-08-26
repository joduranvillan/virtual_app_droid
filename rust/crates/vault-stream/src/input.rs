//! Tipos de payload para `MsgType::InputEvent` — eventos de entrada que
//! viajan del frontend hacia el Android Runtime (dirección inversa a
//! `VideoFrame`).
//!
//! Coordenadas normalizadas `[0.0, 1.0]`, no píxeles: ver nota en
//! `video.rs` sobre por qué. `0.0` es el borde superior/izquierdo,
//! `1.0` el inferior/derecho, sea cual sea la resolución real del
//! framebuffer remoto en ese momento.

use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum InputEventError {
    #[error("coordenada fuera de rango [0.0, 1.0]: {axis}={value}")]
    CoordinateOutOfRange { axis: &'static str, value: f32 },
}

fn validate_normalized(axis: &'static str, value: f32) -> Result<(), InputEventError> {
    if !(0.0..=1.0).contains(&value) || value.is_nan() {
        return Err(InputEventError::CoordinateOutOfRange { axis, value });
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum InputEventPayload {
    TouchDown { pointer_id: u32, x: f32, y: f32, timestamp_unix_ms: u64 },
    TouchMove { pointer_id: u32, x: f32, y: f32, timestamp_unix_ms: u64 },
    TouchUp { pointer_id: u32, timestamp_unix_ms: u64 },
    /// Códigos de tecla estilo Linux input-event (`KEY_*` de
    /// `linux/input-event-codes.h`) para no reinventar una numeración
    /// propia — Android los entiende directamente vía el driver
    /// `evdev` virtual del lado del runtime.
    Key { keycode: u32, pressed: bool, timestamp_unix_ms: u64 },
}

impl InputEventPayload {
    pub fn touch_down(pointer_id: u32, x: f32, y: f32, timestamp_unix_ms: u64) -> Result<Self, InputEventError> {
        validate_normalized("x", x)?;
        validate_normalized("y", y)?;
        Ok(Self::TouchDown { pointer_id, x, y, timestamp_unix_ms })
    }

    pub fn touch_move(pointer_id: u32, x: f32, y: f32, timestamp_unix_ms: u64) -> Result<Self, InputEventError> {
        validate_normalized("x", x)?;
        validate_normalized("y", y)?;
        Ok(Self::TouchMove { pointer_id, x, y, timestamp_unix_ms })
    }

    pub fn touch_up(pointer_id: u32, timestamp_unix_ms: u64) -> Self {
        Self::TouchUp { pointer_id, timestamp_unix_ms }
    }

    pub fn key(keycode: u32, pressed: bool, timestamp_unix_ms: u64) -> Self {
        Self::Key { keycode, pressed, timestamp_unix_ms }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_boundary_coordinates() {
        assert!(InputEventPayload::touch_down(1, 0.0, 1.0, 0).is_ok());
        assert!(InputEventPayload::touch_down(1, 1.0, 0.0, 0).is_ok());
    }

    #[test]
    fn rejects_out_of_range_coordinates() {
        assert!(InputEventPayload::touch_down(1, 1.5, 0.5, 0).is_err());
        assert!(InputEventPayload::touch_down(1, -0.1, 0.5, 0).is_err());
        assert!(InputEventPayload::touch_move(1, 0.5, f32::NAN, 0).is_err());
    }

    #[test]
    fn roundtrips_through_cbor() {
        let ev = InputEventPayload::touch_down(7, 0.25, 0.75, 123456).unwrap();
        let bytes = serde_cbor::to_vec(&ev).unwrap();
        let back: InputEventPayload = serde_cbor::from_slice(&bytes).unwrap();
        assert_eq!(back, ev);
    }

    #[test]
    fn key_event_does_not_need_coordinate_validation() {
        // no debería poder fallar por construcción, a diferencia de touch
        let _ = InputEventPayload::key(30, true, 0); // KEY_A
    }
}
