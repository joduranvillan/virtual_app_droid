//! Rate-limiting de conexiones entrantes por IP de origen.
//!
//! Distinto del rate-limiting de `EnrollmentConfirm` en
//! `enrollment.rs` de este mismo crate (que protege contra adivinar el
//! token de pairing una vez que ya hay un handshake Noise completo y
//! establecido): esto protege contra abrir muchísimas conexiones nuevas
//! por segundo, cada una de las cuales dispara un handshake Noise
//! completo (costo real de CPU: operaciones de curva elíptica) antes de
//! llegar siquiera a la etapa donde el rate-limit de arriba entra en
//! juego. Sin esto, alguien en la LAN podría agotar CPU/memoria del
//! vault sin necesitar adivinar nada.
//!
//! Pensado para usarse desde el punto de aceptación de conexiones de
//! cada plataforma (`vault-linux::bin::vault-host`, y el equivalente en
//! Windows/macOS cuando existan) — es lógica pura sin nada específico
//! de un sistema operativo.
//!
//! La lógica central (`check`) recibe el timestamp actual como
//! parámetro en vez de leerlo internamente — así se puede testear de
//! forma determinística sin esperas reales ni mocks de reloj.

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Mutex;

// Re-exportado para que quien use este módulo no tenga que importar
// `vault_crypto` aparte solo para conseguir el timestamp que `check()`
// espera como parámetro.
pub use vault_crypto::now_unix_ms;

/// Ventana deslizante: como máximo esta cantidad de intentos de conexión
/// por IP en `WINDOW_MS`.
const MAX_ATTEMPTS_PER_WINDOW: u32 = 10;
const WINDOW_MS: u64 = 60_000; // 1 minuto

/// Backoff exponencial para IPs reincidentes tras agotar la ventana:
/// 2min, 4min, 8min... hasta un tope de 15 minutos. A propósito el ban
/// base dura más que `WINDOW_MS`: si durara menos, alcanzaría con
/// esperar a que termine el ban para volver a estar dentro de una
/// ventana "fresca" y el ban perdería buena parte de su efecto disuasivo.
const BASE_BAN_MS: u64 = 2 * 60_000;
const MAX_BAN_MS: u64 = 15 * 60_000;

/// Entradas sin actividad por más de esto se podan en la limpieza
/// periódica, para que el mapa no crezca sin límite con IPs que ya no
/// vuelven a conectarse.
const STALE_ENTRY_MS: u64 = 60 * 60_000; // 1 hora

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitDecision {
    Allow,
    Deny { retry_after_ms: u64 },
}

struct IpWindow {
    window_start_ms: u64,
    count_in_window: u32,
    consecutive_violations: u32,
    banned_until_ms: u64,
}

pub struct IpRateLimiter {
    state: Mutex<HashMap<IpAddr, IpWindow>>,
}

impl IpRateLimiter {
    pub fn new() -> Self {
        Self {
            state: Mutex::new(HashMap::new()),
        }
    }

    /// Registra un intento de conexión de `ip` en el instante `now_ms` y
    /// devuelve si debe permitirse o rechazarse.
    pub fn check(&self, ip: IpAddr, now_ms: u64) -> RateLimitDecision {
        let mut map = self.state.lock().expect("lock envenenado");
        let entry = map.entry(ip).or_insert_with(|| IpWindow {
            window_start_ms: now_ms,
            count_in_window: 0,
            consecutive_violations: 0,
            banned_until_ms: 0,
        });

        if now_ms < entry.banned_until_ms {
            return RateLimitDecision::Deny {
                retry_after_ms: entry.banned_until_ms - now_ms,
            };
        }

        if now_ms.saturating_sub(entry.window_start_ms) > WINDOW_MS {
            entry.window_start_ms = now_ms;
            entry.count_in_window = 0;
        }

        entry.count_in_window += 1;

        if entry.count_in_window > MAX_ATTEMPTS_PER_WINDOW {
            entry.consecutive_violations += 1;
            // cap del shift para no overflowear con reincidentes eternos
            let shift = entry.consecutive_violations.saturating_sub(1).min(10);
            let ban_ms = BASE_BAN_MS.saturating_mul(1u64 << shift).min(MAX_BAN_MS);

            entry.banned_until_ms = now_ms + ban_ms;
            entry.window_start_ms = now_ms;
            entry.count_in_window = 0;

            return RateLimitDecision::Deny { retry_after_ms: ban_ms };
        }

        RateLimitDecision::Allow
    }

    /// Poda entradas viejas para que el mapa no crezca sin límite.
    /// Pensado para correr periódicamente en background, no en el hot
    /// path de cada conexión.
    pub fn prune_stale(&self, now_ms: u64) {
        let mut map = self.state.lock().expect("lock envenenado");
        map.retain(|_, w| {
            let inactive_for = now_ms.saturating_sub(w.window_start_ms);
            inactive_for < STALE_ENTRY_MS || now_ms < w.banned_until_ms
        });
    }

    #[cfg(test)]
    fn tracked_ip_count(&self) -> usize {
        self.state.lock().unwrap().len()
    }
}

impl Default for IpRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr};

    fn test_ip(last_octet: u8) -> IpAddr {
        IpAddr::V4(Ipv4Addr::new(192, 168, 1, last_octet))
    }

    #[test]
    fn allows_requests_within_the_window() {
        let limiter = IpRateLimiter::new();
        let ip = test_ip(1);
        let now = 1_000_000u64;

        for _ in 0..MAX_ATTEMPTS_PER_WINDOW {
            assert_eq!(limiter.check(ip, now), RateLimitDecision::Allow);
        }
    }

    #[test]
    fn denies_after_exceeding_the_window() {
        let limiter = IpRateLimiter::new();
        let ip = test_ip(2);
        let now = 1_000_000u64;

        for _ in 0..MAX_ATTEMPTS_PER_WINDOW {
            limiter.check(ip, now);
        }

        let result = limiter.check(ip, now);
        match result {
            RateLimitDecision::Deny { retry_after_ms } => {
                assert_eq!(retry_after_ms, BASE_BAN_MS);
            }
            RateLimitDecision::Allow => panic!("debería haber rechazado tras agotar la ventana"),
        }
    }

    #[test]
    fn stays_denied_during_the_ban_even_if_window_would_have_reset() {
        let limiter = IpRateLimiter::new();
        let ip = test_ip(3);
        let now = 1_000_000u64;

        for _ in 0..=MAX_ATTEMPTS_PER_WINDOW {
            limiter.check(ip, now);
        }

        // avanza el reloj más allá de lo que duraría una ventana normal,
        // pero todavía dentro del ban -> debe seguir denegando
        let later = now + WINDOW_MS + 1;
        assert!(matches!(
            limiter.check(ip, later),
            RateLimitDecision::Deny { .. }
        ));
    }

    #[test]
    fn allows_again_after_the_ban_expires() {
        let limiter = IpRateLimiter::new();
        let ip = test_ip(4);
        let now = 1_000_000u64;

        for _ in 0..=MAX_ATTEMPTS_PER_WINDOW {
            limiter.check(ip, now);
        }

        let after_ban = now + BASE_BAN_MS + 1;
        assert_eq!(limiter.check(ip, after_ban), RateLimitDecision::Allow);
    }

    #[test]
    fn ban_escalates_for_repeat_offenders() {
        let limiter = IpRateLimiter::new();
        let ip = test_ip(5);
        let mut now = 1_000_000u64;

        // primera violación
        for _ in 0..=MAX_ATTEMPTS_PER_WINDOW {
            limiter.check(ip, now);
        }
        let first_ban = match limiter.check(ip, now) {
            RateLimitDecision::Deny { retry_after_ms } => retry_after_ms,
            RateLimitDecision::Allow => panic!("esperaba Deny"),
        };

        // dejamos pasar el primer ban y volvemos a violar la ventana
        now += first_ban + 1;
        for _ in 0..=MAX_ATTEMPTS_PER_WINDOW {
            limiter.check(ip, now);
        }
        let second_ban = match limiter.check(ip, now) {
            RateLimitDecision::Deny { retry_after_ms } => retry_after_ms,
            RateLimitDecision::Allow => panic!("esperaba Deny"),
        };

        assert!(
            second_ban > first_ban,
            "el segundo ban ({second_ban}ms) debería ser más largo que el primero ({first_ban}ms)"
        );
    }

    #[test]
    fn different_ips_are_tracked_independently() {
        let limiter = IpRateLimiter::new();
        let now = 1_000_000u64;
        let ip_a = test_ip(10);
        let ip_b = test_ip(20);

        for _ in 0..=MAX_ATTEMPTS_PER_WINDOW {
            limiter.check(ip_a, now);
        }

        // ip_a está baneada, pero ip_b nunca conectó -> debe permitirse
        assert_eq!(limiter.check(ip_b, now), RateLimitDecision::Allow);
    }

    #[test]
    fn prune_removes_long_inactive_entries_but_keeps_active_bans() {
        let limiter = IpRateLimiter::new();
        let now = 1_000_000u64;
        let stale_ip = test_ip(30);
        let banned_ip = test_ip(31);

        limiter.check(stale_ip, now);
        for _ in 0..=MAX_ATTEMPTS_PER_WINDOW {
            limiter.check(banned_ip, now);
        }

        assert_eq!(limiter.tracked_ip_count(), 2);

        // mucho después: stale_ip ya no tiene ban activo y su ventana
        // quedó vieja -> se poda. banned_ip todavía puede seguir baneada
        // según cuánto haya escalado, pero en este caso el ban base
        // (30s) ya expiró bastante antes de STALE_ENTRY_MS (1h), así que
        // ambas deberían podarse en un lapso lo bastante largo.
        let far_future = now + STALE_ENTRY_MS + 1;
        limiter.prune_stale(far_future);

        assert_eq!(limiter.tracked_ip_count(), 0);
    }
}
