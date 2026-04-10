//! SQLite-backed catalog store placeholder.
//!
//! This type is the future `infra/io` wrapper around the real platform catalog.
//! The active implementation still lives in `platform/adapters/data/sqlite.rs`.

use std::path::{Path, PathBuf};

use super::interface::CatalogStore;

/// Placeholder SQLite catalog descriptor.
#[derive(Debug, Clone)]
pub struct SqliteCatalogStore {
    root: PathBuf,
}

impl SqliteCatalogStore {
    /// Create a new SQLite catalog descriptor rooted at `root`.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

impl CatalogStore for SqliteCatalogStore {
    fn id(&self) -> &'static str {
        "catalog.sqlite"
    }

    fn root(&self) -> &Path {
        &self.root
    }
}
