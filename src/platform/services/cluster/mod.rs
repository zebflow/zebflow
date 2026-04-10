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
pub mod placement;
pub mod registry;
pub mod runtime_sync;

pub use bootstrap::ClusterBootstrapService;
pub use placement::ClusterPlacementService;
pub use registry::ClusterRegistryService;
pub use runtime_sync::ClusterRuntimeSyncService;
