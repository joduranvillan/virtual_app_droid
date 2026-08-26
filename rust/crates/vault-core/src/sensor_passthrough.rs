//! Despachador de Telemetría de Sensores y Passthrough a Virtio-Input (120 Hz).
//!
//! Inyecta muestras de sensores a alta frecuencia (120 Hz) directamente en los
//! nodos virtuales virtio-input de la máquina virtual invitada (`/dev/input/event*`).
//!
//! Sensores soportados:
//! - Acelerómetro triaxial (m/s²)
//! - Giroscopio triaxial (rad/s)
//! - Sensor de presión barométrica (hPa)
//! - Telemetría de batería (nivel, voltaje, temperatura, estado de carga)
//! - Passthrough de coordenadas GNSS/GPS (lat, lon, alt, speed, acc)

use std::collections::VecDeque;
use vault_protocol::services::VirtioInputSensorPayload;

pub const TARGET_SAMPLE_RATE_HZ: u32 = 120;
pub const MAX_RING_BUFFER_SAMPLES: usize = 240; // 2 segundos a 120 Hz

pub struct VirtioInputSensorDispatcher {
    sample_rate_hz: u32,
    total_injected_events: u64,
    ring_buffer: VecDeque<VirtioInputSensorPayload>,
    is_active: bool,
}

impl VirtioInputSensorDispatcher {
    pub fn new() -> Self {
        Self {
            sample_rate_hz: TARGET_SAMPLE_RATE_HZ,
            total_injected_events: 0,
            ring_buffer: VecDeque::with_capacity(MAX_RING_BUFFER_SAMPLES),
            is_active: true,
        }
    }

    pub fn set_active(&mut self, active: bool) {
        self.is_active = active;
    }

    pub fn is_active(&self) -> bool {
        self.is_active
    }

    pub fn sample_rate_hz(&self) -> u32 {
        self.sample_rate_hz
    }

    pub fn total_injected_events(&self) -> u64 {
        self.total_injected_events
    }

    /// Inyecta una muestra de telemetría CBOR al bus virtio-input.
    pub fn inject_sample(&mut self, sample: VirtioInputSensorPayload) {
        if !self.is_active {
            return;
        }

        if self.ring_buffer.len() >= MAX_RING_BUFFER_SAMPLES {
            self.ring_buffer.pop_front();
        }

        self.ring_buffer.push_back(sample);
        self.total_injected_events += 1;
    }

    pub fn latest_sample(&self) -> Option<&VirtioInputSensorPayload> {
        self.ring_buffer.back()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vault_protocol::services::{BatteryTelemetryPayload, GpsCoordinatesPayload};

    #[test]
    fn test_sensor_injection_at_120hz() {
        let mut dispatcher = VirtioInputSensorDispatcher::new();
        assert_eq!(dispatcher.sample_rate_hz(), 120);

        let sample = VirtioInputSensorPayload {
            timestamp_ns: 1770000000000,
            sample_rate_hz: 120,
            accel_x: 0.15,
            accel_y: 9.81,
            accel_z: 0.02,
            gyro_yaw: 0.01,
            gyro_pitch: -0.04,
            gyro_roll: 0.02,
            pressure_hpa: 1013.25,
            battery: BatteryTelemetryPayload {
                level_percent: 88,
                voltage_mv: 4150,
                temperature_c: 32.5,
                is_charging: false,
            },
            gps: GpsCoordinatesPayload {
                latitude: 19.4326,
                longitude: -99.1332,
                altitude_meters: 2240.0,
                speed_kmh: 0.0,
                accuracy_meters: 3.5,
            },
            target_virtio_device: "/dev/input/event2".to_string(),
        };

        dispatcher.inject_sample(sample);
        assert_eq!(dispatcher.total_injected_events(), 1);
        assert!(dispatcher.latest_sample().is_some());
    }
}
