//! Platform-facing cluster orchestration services.
//!
//! This namespace is intentionally thin. It is where Studio, project lifecycle flows, and
//! control-plane policy interact with the lower-level cluster and execution abstractions from
//! `crate::infra`.
//!
//! What belongs here:
//!
//! - worker registration and presentation logic for the product UI
//! - project placement defaults and policy orchestration
//! - cluster bootstrap flows and role-aware startup helpers
//! - project runtime bundle preparation and sync orchestration
//!
//! What does *not* belong here:
//!
//! - mTLS certificate machinery
//! - control-stream protocols
//! - shared state bus internals
//! - storage-engine specifics
//!
//! Those lower-level mechanics live under `crate::infra::cluster` and `crate::infra::io`.

pub mod bootstrap;
pub mod join_token;
pub mod local_authority;
pub mod placement;
pub mod registry;
pub mod runtime_sync;

pub use bootstrap::ClusterBootstrapService;
pub use join_token::{ClusterJoinTokenService, OfficeJoinIdentity};
pub use local_authority::{OfficeJoinFile, OfficeLocalAuthorityService};
pub use placement::ClusterPlacementService;
pub use registry::ClusterRegistryService;
pub use runtime_sync::ClusterRuntimeSyncService;

/// Normalizes an office address, refusing one that names nothing.
///
/// `kinds/office-topology/README.md`, Rejections: "An office without a
/// `base_url` — an office nothing can reach is not an office." Every writer of
/// a `PlatformOffice` row comes through here, so a directory entry nothing can
/// address cannot be created by minting, by registering, or by a heartbeat.
///
/// The check is presence, not shape: the contract names the missing address as
/// the rejection and says nothing about scheme or host, and an operator's
/// `ZEBFLOW_ADVERTISE_URL` is theirs to spell.
pub fn normalize_office_base_url(
    raw: &str,
) -> Result<String, crate::platform::error::PlatformError> {
    let value = raw.trim().trim_end_matches('/').trim().to_string();
    if value.is_empty() {
        return Err(crate::platform::error::PlatformError::new(
            "CLUSTER_OFFICE_BASE_URL_REQUIRED",
            "an office needs a base_url; an office nothing can reach is not an office",
        ));
    }
    Ok(value)
}
