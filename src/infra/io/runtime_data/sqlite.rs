//! SQLite runtime-data store descriptor.

use std::path::{Path, PathBuf};

use super::interface::RuntimeDataStore;

/// Local SQLite-backed runtime data descriptor.
#[derive(Debug, Clone)]
pub struct SqliteRuntimeDataStore {
    root: PathBuf,
}

impl SqliteRuntimeDataStore {
    /// Build a runtime-data descriptor rooted at `root`.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

impl RuntimeDataStore for SqliteRuntimeDataStore {
    fn id(&self) -> &'static str {
        "runtime_data.sqlite"
    }

    fn root(&self) -> &Path {
        &self.root
    }
}
