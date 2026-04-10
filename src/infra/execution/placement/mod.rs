//! Placement policy model.
//!
//! Placement answers *where* a run should go once a backend kind is known.
//! It is intentionally separate from backend kind so the same backend can be routed locally,
//! pinned to a worker, or later resolved through tags/selectors.

pub mod policy;
pub mod record;
pub mod runtime;
pub mod selector;

pub use policy::{PlacementPolicy, PlacementPolicyKind};
pub use record::{ProjectRuntimePlacement, ProjectRuntimePlacementTarget};
pub use runtime::{
    ProjectRuntimeMode, ProjectRuntimeProfile, ResourceProfile, RuntimeResourceSpec,
};
pub use selector::RunnerSelector;
