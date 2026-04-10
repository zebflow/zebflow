//! Local filesystem object store descriptor.

use std::path::{Path, PathBuf};

use super::interface::ObjectStore;

/// Local filesystem-backed object store.
#[derive(Debug, Clone)]
pub struct LocalFsObjectStore {
    root: PathBuf,
}

impl LocalFsObjectStore {
    /// Build a local filesystem object store rooted at `root`.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }
}

impl ObjectStore for LocalFsObjectStore {
    fn id(&self) -> &'static str {
        "object.local_fs"
    }

    fn root(&self) -> &Path {
        &self.root
    }
}
