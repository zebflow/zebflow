//! Trait definitions for platform catalog storage.

use std::path::Path;
use std::sync::Arc;

/// Platform metadata storage contract.
///
/// This trait is intentionally small in the first scaffold. The concrete CRUD surface will
/// be carved out of the current `DataAdapter` implementation incrementally as cluster-aware
/// orchestration moves onto the new `infra/io` layer.
pub trait CatalogStore: Send + Sync {
    /// Stable implementation identifier, e.g. `"catalog.sqlite"`.
    fn id(&self) -> &'static str;

    /// Root directory or storage root associated with this catalog implementation.
    fn root(&self) -> &Path;
}

/// Shared trait object used by higher-level orchestration layers.
pub type DynCatalogStore = Arc<dyn CatalogStore>;
