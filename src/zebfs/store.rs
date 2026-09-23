//! The store a project's files live in, whichever backend it declared.
//!
//! One enum rather than a trait object so that every caller keeps writing
//! `layout.open_files().get(path)` and the compiler, not a vtable, says which
//! implementation answers. A third backend is one more variant here and its
//! arm in each verb; the callers do not change.

use std::path::{Path, PathBuf};

use super::backend::FileBackend;
use super::error::ZebFsError;
use super::local::LocalZebFs;
use super::model::{ZebFsEntry, ZebFsObject, ZebFsStat};
use super::s3::S3ZebFs;

/// A project's file store.
#[derive(Debug, Clone)]
pub enum ZebFs {
    Local(LocalZebFs),
    S3(S3ZebFs),
}

impl ZebFs {
    /// The declared word this store answers to.
    pub fn backend(&self) -> FileBackend {
        match self {
            Self::Local(_) => FileBackend::Zebfs,
            Self::S3(_) => FileBackend::S3,
        }
    }

    /// The directory a local store keeps its bytes in; `None` for a bucket.
    ///
    /// For the few callers that stream from a file path — a map engine, a
    /// table engine — and can only do so when there is one.
    pub fn local_root(&self) -> Option<&Path> {
        match self {
            Self::Local(local) => Some(local.root()),
            Self::S3(_) => None,
        }
    }

    /// The local filesystem path of an object, when the store is a
    /// directory. A bucket has no such path, and answers `None` rather than
    /// a path that does not exist.
    pub fn local_path(&self, path: &str) -> Result<Option<PathBuf>, ZebFsError> {
        match self {
            Self::Local(local) => local.resolve_object_path(path).map(|(_, abs)| Some(abs)),
            Self::S3(_) => Ok(None),
        }
    }

    pub fn put(&self, path: &str, bytes: &[u8]) -> Result<ZebFsStat, ZebFsError> {
        match self {
            Self::Local(s) => s.put(path, bytes),
            Self::S3(s) => s.put(path, bytes),
        }
    }

    pub fn get(&self, path: &str) -> Result<ZebFsObject, ZebFsError> {
        match self {
            Self::Local(s) => s.get(path),
            Self::S3(s) => s.get(path),
        }
    }

    pub fn head(&self, path: &str) -> Result<ZebFsStat, ZebFsError> {
        match self {
            Self::Local(s) => s.head(path),
            Self::S3(s) => s.head(path),
        }
    }

    pub fn list(&self, prefix: &str) -> Result<Vec<ZebFsEntry>, ZebFsError> {
        match self {
            Self::Local(s) => s.list(prefix),
            Self::S3(s) => s.list(prefix),
        }
    }

    pub fn create_prefix(&self, prefix: &str) -> Result<ZebFsStat, ZebFsError> {
        match self {
            Self::Local(s) => s.create_prefix(prefix),
            Self::S3(s) => s.create_prefix(prefix),
        }
    }

    pub fn delete(&self, path: &str) -> Result<(), ZebFsError> {
        match self {
            Self::Local(s) => s.delete(path),
            Self::S3(s) => s.delete(path),
        }
    }

    pub fn copy(&self, from: &str, to: &str) -> Result<ZebFsStat, ZebFsError> {
        match self {
            Self::Local(s) => s.copy(from, to),
            Self::S3(s) => s.copy(from, to),
        }
    }

    /// A document under the reserved `.zebfs/` prefix — the ACL manifest —
    /// which the user verbs above refuse. `None` when it was never written.
    pub fn read_reserved(&self, path: &str) -> Result<Option<Vec<u8>>, ZebFsError> {
        match self {
            Self::Local(s) => s.read_reserved(path),
            Self::S3(s) => s.read_reserved(path),
        }
    }

    pub fn write_reserved(&self, path: &str, bytes: &[u8]) -> Result<(), ZebFsError> {
        match self {
            Self::Local(s) => s.write_reserved(path, bytes),
            Self::S3(s) => s.write_reserved(path, bytes),
        }
    }
}
