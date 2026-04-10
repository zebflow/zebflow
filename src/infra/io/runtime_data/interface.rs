//! Trait definitions for per-project runtime data storage.

use std::path::Path;
use std::sync::Arc;

/// Per-project runtime data contract.
pub trait RuntimeDataStore: Send + Sync {
    /// Stable implementation identifier.
    fn id(&self) -> &'static str;

    /// Root directory or mount for this runtime-data implementation.
    fn root(&self) -> &Path;
}

/// Shared runtime-data trait object.
pub type DynRuntimeDataStore = Arc<dyn RuntimeDataStore>;
