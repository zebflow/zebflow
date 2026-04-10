//! Trait definitions for object storage backends.

use std::path::Path;
use std::sync::Arc;

/// Generic object storage contract for repo/files/assets/bundles.
pub trait ObjectStore: Send + Sync {
    /// Stable implementation identifier.
    fn id(&self) -> &'static str;

    /// Root directory or mount associated with the store.
    fn root(&self) -> &Path;
}

/// Shared object-store trait object.
pub type DynObjectStore = Arc<dyn ObjectStore>;
