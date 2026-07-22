//! Estado de enrolamiento, independiente de plataforma.
//!
//! Mientras no haya ninguna clave de frontend pineada, el vault está en
//! modo `ENROLLING`: acepta el primer handshake Noise_XX exitoso que
//! además presente un token de pairing válido (visto en el QR), y a
//! partir de ahí pinea esa clave para siempre. Cualquier conexión
//! posterior de una clave distinta se rechaza.
//!
//! Incluye rate-limiting sobre `EnrollmentConfirm`: cada intento fallido
//! impone un backoff exponencial antes de aceptar el siguiente, y tras
//! `MAX_ATTEMPTS_BEFORE_LOCKOUT` fallos consecutivos el token queda
//! invalidado por completo (hace falta reiniciar el proceso para
//! generar uno nuevo). Los 128 bits de entropía del token ya hacían
//! impracticable la fuerza bruta, pero esto cierra la ventana de todos
//! modos: sin esto, alguien en la LAN podía intentar indefinidamente sin
//! ningún costo.
//!
//! **Diferencia clave con la versión anterior (pre-reestructuración):**
//! esto ya NO toca el filesystem directamente ni abre sockets Unix —
//! delega la persistencia del pin en un `Arc<dyn SecretStore>` (así cada
//! plataforma decide cómo protegerlo) y expone la info del enrolamiento
//! pendiente vía `pending_info()` para que el binario de cada plataforma
//! decida cómo publicarla (un socket Unix en Linux, lo que corresponda
//! en Windows/macOS).

use crate::traits::SecretStore;
use anyhow::{anyhow, Result};
use std::sync::Arc;
use std::time::Duration;
use tracing::warn;
use vault_crypto::{generate_pairing_token, now_unix_ms};

const TOKEN_TTL: Duration = Duration::from_secs(10 * 60);

/// Tras este número de intentos fallidos consecutivos, se invalida el
/// token pendiente por completo (no solo un backoff temporal) — hace
/// falta reiniciar el proceso para volver a entrar en ENROLLING.
const MAX_ATTEMPTS_BEFORE_LOCKOUT: u32 = 5;
/// Backoff exponencial entre intentos: 1s, 2s, 4s, 8s, 16s...
const BASE_BACKOFF: Duration = Duration::from_secs(1);
const MAX_BACKOFF: Duration = Duration::from_secs(30);
/// Bloqueo largo una vez agotados los intentos.
const LOCKOUT_AFTER_MAX_ATTEMPTS: Duration = Duration::from_secs(15 * 60);

#[derive(Debug, Clone)]
struct PendingEnrollment {
    token: String,
    expires_unix_ms: u64,
}

/// Lo que el binario de cada plataforma necesita para publicar el QR
/// (junto con la clave pública de la identidad del runtime, que vive
/// aparte porque no es responsabilidad de `EnrollmentState`).
#[derive(Debug, Clone)]
pub struct PendingEnrollmentInfo {
    pub token: String,
    pub expires_unix_ms: u64,
}

pub struct EnrollmentState {
    secret_store: Arc<dyn SecretStore>,
    pending: Option<PendingEnrollment>,
    failed_attempts: u32,
    locked_until_unix_ms: u64,
}

impl EnrollmentState {
    pub fn new(secret_store: Arc<dyn SecretStore>) -> Self {
        Self {
            secret_store,
            pending: None,
            failed_attempts: 0,
            locked_until_unix_ms: 0,
        }
    }

    pub fn is_enrolled(&self) -> Result<bool> {
        Ok(self.secret_store.load_pinned_frontend_key()?.is_some())
    }

    pub fn pinned_public_key(&self) -> Result<Option<Vec<u8>>> {
        Ok(self.secret_store.load_pinned_frontend_key()?)
    }

    /// Genera un token nuevo y lo guarda como "pendiente". El llamador
    /// (el binario de la plataforma) es quien decide cómo publicar esta
    /// info para que `vault-host` arme el QR.
    pub fn begin_enrollment(&mut self) -> PendingEnrollmentInfo {
        let token = generate_pairing_token();
        let expires_unix_ms = now_unix_ms() + TOKEN_TTL.as_millis() as u64;

        self.pending = Some(PendingEnrollment {
            token: token.clone(),
            expires_unix_ms,
        });
        self.failed_attempts = 0;
        self.locked_until_unix_ms = 0;

        PendingEnrollmentInfo { token, expires_unix_ms }
    }

    #[cfg(test)]
    fn set_pending(&mut self, token: String, expires_unix_ms: u64) {
        self.pending = Some(PendingEnrollment { token, expires_unix_ms });
        self.failed_attempts = 0;
        self.locked_until_unix_ms = 0;
    }

    pub fn pending_info(&self) -> Option<PendingEnrollmentInfo> {
        self.pending.as_ref().map(|p| PendingEnrollmentInfo {
            token: p.token.clone(),
            expires_unix_ms: p.expires_unix_ms,
        })
    }

    /// Valida el token recibido en `EnrollmentConfirm` y, si es válido,
    /// pinea la clave pública remota a través del `SecretStore`. Aplica
    /// rate-limiting: si todavía está dentro de la ventana de backoff de
    /// un intento fallido anterior, rechaza sin siquiera comparar el
    /// token (así un intento "gratis" durante el bloqueo no cuenta doble).
    pub fn try_complete_enrollment(&mut self, presented_token: &str, remote_pubkey: &[u8]) -> Result<()> {
        let now = now_unix_ms();

        if now < self.locked_until_unix_ms {
            let wait_secs = (self.locked_until_unix_ms - now) / 1000;
            return Err(anyhow!(
                "demasiados intentos recientes — esperá {wait_secs}s antes de reintentar"
            ));
        }

        let pending = self
            .pending
            .as_ref()
            .ok_or_else(|| anyhow!("no hay un enrolamiento pendiente"))?;

        if now > pending.expires_unix_ms {
            self.pending = None;
            return Err(anyhow!("el token de pairing expiró — generar un QR nuevo"));
        }

        if !constant_time_eq(presented_token.as_bytes(), pending.token.as_bytes()) {
            self.register_failed_attempt();
            return Err(anyhow!("token de pairing inválido"));
        }

        self.secret_store.persist_pinned_frontend_key(remote_pubkey)?;

        self.pending = None;
        self.failed_attempts = 0;
        self.locked_until_unix_ms = 0;

        tracing::info!("enrolamiento completado — clave del frontend pineada, QR ya no es válido");
        Ok(())
    }

    fn register_failed_attempt(&mut self) {
        self.failed_attempts += 1;
        let now = now_unix_ms();

        if self.failed_attempts >= MAX_ATTEMPTS_BEFORE_LOCKOUT {
            warn!(
                attempts = self.failed_attempts,
                "demasiados intentos fallidos de EnrollmentConfirm — invalidando el token pendiente, hace falta reiniciar el proceso para generar uno nuevo"
            );
            self.pending = None;
            self.locked_until_unix_ms = now + LOCKOUT_AFTER_MAX_ATTEMPTS.as_millis() as u64;
            return;
        }

        let backoff_ms = BASE_BACKOFF
            .as_millis()
            .saturating_mul(1u128 << (self.failed_attempts - 1))
            .min(MAX_BACKOFF.as_millis()) as u64;
        warn!(
            attempts = self.failed_attempts,
            backoff_ms, "token de pairing incorrecto — aplicando backoff"
        );
        self.locked_until_unix_ms = now + backoff_ms;
    }

    #[cfg(test)]
    fn bypass_lockout_for_test(&mut self) {
        self.locked_until_unix_ms = 0;
    }
}

fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a.iter().zip(b.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::traits::{PlatformResult, SecretStore};
    use std::sync::Mutex as StdMutex;
    use vault_crypto::StaticKeypair;

    /// `SecretStore` en memoria para tests — evita todo el boilerplate
    /// de archivos temporales que tenía la versión anterior de estos
    /// tests, y de paso deja mejor separado qué es lógica de negocio
    /// (acá) vs. qué es "cómo se guarda en disco" (eso ahora se testea
    /// en `vault-linux`, contra la implementación real basada en archivos).
    struct InMemorySecretStore {
        pinned: StdMutex<Option<Vec<u8>>>,
    }

    impl InMemorySecretStore {
        fn new() -> Arc<Self> {
            Arc::new(Self {
                pinned: StdMutex::new(None),
            })
        }
    }

    impl SecretStore for InMemorySecretStore {
        fn load_or_generate_identity(&self) -> PlatformResult<StaticKeypair> {
            unimplemented!("no lo usan los tests de enrolamiento")
        }

        fn load_pinned_frontend_key(&self) -> PlatformResult<Option<Vec<u8>>> {
            Ok(self.pinned.lock().unwrap().clone())
        }

        fn persist_pinned_frontend_key(&self, key: &[u8]) -> PlatformResult<()> {
            *self.pinned.lock().unwrap() = Some(key.to_vec());
            Ok(())
        }
    }

    #[test]
    fn rejects_confirmation_with_no_pending_enrollment() {
        let mut state = EnrollmentState::new(InMemorySecretStore::new());
        let result = state.try_complete_enrollment("cualquier-token", b"pubkey");
        assert!(result.is_err());
    }

    #[test]
    fn rejects_wrong_token() {
        let mut state = EnrollmentState::new(InMemorySecretStore::new());
        state.set_pending("token-correcto".to_string(), now_unix_ms() + 60_000);

        let result = state.try_complete_enrollment("token-incorrecto", b"pubkey-del-telefono");
        assert!(result.is_err());
        assert_eq!(state.pinned_public_key().unwrap(), None, "no debe pinear nada si el token es incorrecto");
    }

    #[test]
    fn rejects_expired_token() {
        let mut state = EnrollmentState::new(InMemorySecretStore::new());
        // ya expiró (expiró "ayer")
        state.set_pending("token".to_string(), now_unix_ms().saturating_sub(1_000));

        let result = state.try_complete_enrollment("token", b"pubkey-del-telefono");
        assert!(result.is_err());
        assert_eq!(state.pinned_public_key().unwrap(), None);
    }

    #[test]
    fn accepts_valid_token_and_pins_the_key() {
        let mut state = EnrollmentState::new(InMemorySecretStore::new());
        state.set_pending("token-bueno".to_string(), now_unix_ms() + 60_000);

        let pubkey = b"pubkey-del-telefono-real".to_vec();
        let result = state.try_complete_enrollment("token-bueno", &pubkey);
        assert!(result.is_ok());

        assert!(state.is_enrolled().unwrap());
        assert_eq!(state.pinned_public_key().unwrap(), Some(pubkey));
    }

    #[test]
    fn token_cannot_be_reused_after_success() {
        let mut state = EnrollmentState::new(InMemorySecretStore::new());
        state.set_pending("token-unico".to_string(), now_unix_ms() + 60_000);

        assert!(state.try_complete_enrollment("token-unico", b"primer-dispositivo").is_ok());

        // un segundo intento con el mismo token, aunque no haya expirado,
        // ya no tiene un `pending` activo (se limpió al completarse)
        let second = state.try_complete_enrollment("token-unico", b"otro-dispositivo");
        assert!(second.is_err());
    }

    #[test]
    fn backoff_blocks_immediate_retry_even_with_correct_token() {
        let mut state = EnrollmentState::new(InMemorySecretStore::new());
        state.set_pending("token-correcto".to_string(), now_unix_ms() + 600_000);

        // primer intento fallido
        assert!(state.try_complete_enrollment("token-incorrecto", b"pubkey").is_err());

        // reintento INMEDIATO con el token correcto -> igual debe fallar,
        // porque todavía no pasó la ventana de backoff de 1s
        let retry = state.try_complete_enrollment("token-correcto", b"pubkey");
        assert!(retry.is_err());
        assert_eq!(state.pinned_public_key().unwrap(), None, "no debería haber pineado nada mientras está en backoff");
    }

    #[test]
    fn locks_out_and_invalidates_pending_after_max_attempts() {
        let mut state = EnrollmentState::new(InMemorySecretStore::new());
        state.set_pending("token-correcto".to_string(), now_unix_ms() + 600_000);

        for _ in 0..MAX_ATTEMPTS_BEFORE_LOCKOUT {
            let result = state.try_complete_enrollment("token-incorrecto", b"pubkey");
            assert!(result.is_err());
            // simula que pasó suficiente tiempo como para no seguir
            // bloqueado por el backoff exponencial entre intentos
            state.bypass_lockout_for_test();
        }

        // tras agotar los intentos, el pending debe haberse invalidado
        // por completo — ni siquiera el token correcto sirve ya
        let final_try = state.try_complete_enrollment("token-correcto", b"pubkey");
        assert!(final_try.is_err());
        assert_eq!(state.pinned_public_key().unwrap(), None);
    }

    #[test]
    fn pending_info_reflects_current_pending_state() {
        let mut state = EnrollmentState::new(InMemorySecretStore::new());
        assert!(state.pending_info().is_none());

        let info = state.begin_enrollment();
        let reflected = state.pending_info().unwrap();
        assert_eq!(reflected.token, info.token);
        assert_eq!(reflected.expires_unix_ms, info.expires_unix_ms);
    }
}
