//! Orquestación: máquina de estados de enrolamiento, rate-limiting de
//! conexión, router de llamadas RPC a servicios virtuales. Define los
//! traits (`traits.rs`) que cada plataforma (`vault-linux`,
//! `vault-windows`, `vault-macos`) implementa con su tecnología nativa.
//!
//! Deliberadamente sin nada específico de un sistema operativo: nada de
//! rutas de archivo fijas, nada de `std::process::Command`, nada de
//! sockets Unix. Eso es exactamente lo que separaba este crate de
//! `vault_runtime`/`vault_host` en la versión anterior del proyecto, y
//! es lo que ahora permite escribir `vault-windows`/`vault-macos` sin
//! reimplementar la lógica de enrolamiento o el rate-limiting.

pub mod cluster;
pub mod enrollment;
pub mod hotplug;
pub mod multicodec;
pub mod rate_limit;
pub mod sensor_passthrough;
pub mod services;
pub mod traits;

pub use cluster::{ClusterError, ClusterOrchestrator};
pub use enrollment::{EnrollmentState, PendingEnrollmentInfo};
pub use hotplug::{
    HotPlugError, VmResourceController, MAX_VCPU_CORES, MAX_VRAM_GB, MIN_VCPU_CORES, MIN_VRAM_GB,
};
pub use multicodec::CodecNegotiator;
pub use rate_limit::{now_unix_ms, IpRateLimiter, RateLimitDecision};
pub use sensor_passthrough::{VirtioInputSensorDispatcher, TARGET_SAMPLE_RATE_HZ};


pub use services::{request_location, dispatch_service_request, handle_admin_request};
pub use traits::{AndroidHypervisor, EncryptedStorage, MountedVolume, PlatformError, PlatformResult, RunningInstance, SecretStore};
