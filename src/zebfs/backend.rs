//! The declared file storage backend, and the one place it becomes an
//! implementation.
//!
//! Two different questions get asked with the same words, and this module
//! answers only the first:
//!
//! 1. **Where does this project keep its own files?** That is the *native*
//!    backend. It owns everything under the project's `files/` tree, it is what
//!    a [`FileRef`]'s `backend` field names, and it is what
//!    `spec.files.backend` declares. Local disk, or a bucket.
//! 2. **Which outside bucket may this pipeline read and write?** That is an
//!    external data source, reached through a connection and a credential, the
//!    same shape as a Postgres connection, and chosen per pipeline. It is not a
//!    backend and nothing here resolves it.
//!
//! The consequence, because it is the part that is easy to get wrong: a node
//! that reads from an *external* bucket produces bytes that land in the
//! *native* store, so the FileRef it emits carries the native backend value.
//! It never carries `s3` on account of where the bytes came from. If it did,
//! `ref` would stop meaning one consistent thing — sometimes a key in the store
//! Zebflow owns, sometimes a key in a bucket it does not.
//!
//! [`FileRef`]: crate::pipeline::nodes::shared::file_ref

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use super::error::ZebFsError;
use super::local::LocalZebFs;
use super::s3::{S3Config, S3ZebFs};
use super::store::ZebFs;

/// The native backend whose bytes live on this machine's disk.
///
/// One word names it in two places that must agree: `spec.files.backend`
/// declares it, and every FileRef written for locally-stored bytes carries it
/// in `backend`. They name the same thing, so they are the same constant.
pub const BACKEND_ZEBFS: &str = "zebfs";

/// The native backend whose bytes live in an S3-compatible bucket.
pub const BACKEND_S3: &str = "s3";

/// Backends a project may declare, in the order an error message lists them.
pub const FILE_BACKENDS: &[&str] = &[BACKEND_ZEBFS, BACKEND_S3];

/// The backend a project declared, or the default it inherits by declaring
/// nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileBackend {
    /// Bytes on this machine's disk, under the project's `files/` directory.
    #[default]
    Zebfs,
    /// Bytes in an S3-compatible bucket, reached through a credential of
    /// kind `s3` that the instance selects for the project.
    S3,
}

impl FileBackend {
    /// The declared word for this backend.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Zebfs => BACKEND_ZEBFS,
            Self::S3 => BACKEND_S3,
        }
    }

    /// A human label for status displays.
    pub fn label(self) -> &'static str {
        match self {
            Self::Zebfs => "Local disk",
            Self::S3 => "Object store (S3)",
        }
    }

    /// Reads one declared word, refusing an unknown one by name.
    ///
    /// An unknown value is refused rather than defaulted, because a project
    /// that asked for a store this build cannot open must not quietly write its
    /// bytes somewhere else.
    pub fn parse(value: &str) -> Result<Self, ZebFsError> {
        match value {
            BACKEND_ZEBFS => Ok(Self::Zebfs),
            BACKEND_S3 => Ok(Self::S3),
            other => Err(ZebFsError::new(
                "ZEBFS_UNKNOWN_BACKEND",
                unknown_backend_message(other),
            )),
        }
    }

    /// Resolves an optional declaration: absent means the default.
    pub fn resolve(declared: Option<&str>) -> Result<Self, ZebFsError> {
        match declared {
            Some(value) => Self::parse(value),
            None => Ok(Self::default()),
        }
    }
}

/// The refusal text for a backend this build does not provide.
///
/// It is one function so that the contract reader, the resolver, and any future
/// caller all name the offending value and list the same accepted set.
pub fn unknown_backend_message(value: &str) -> String {
    format!(
        "'{value}' is not a file storage backend this build provides; accepted: {}",
        FILE_BACKENDS.join(", ")
    )
}

/// A declared backend with everything it needs, resolved once when a
/// project's layout is built, so that opening the store afterwards cannot
/// fail: the local directory, or the bucket and the credential that reaches
/// it.
#[derive(Debug, Clone)]
pub enum FileStore {
    Local(PathBuf),
    S3(S3Config),
}

impl FileStore {
    pub fn backend(&self) -> FileBackend {
        match self {
            Self::Local(_) => FileBackend::Zebfs,
            Self::S3(_) => FileBackend::S3,
        }
    }
}

/// The seam: a resolved store becomes the implementation that owns the bytes.
///
/// Every caller that needs a project's storage passes through here, so a
/// backend is added by returning a different implementation from this one
/// function rather than by finding the places that named the old one.
pub fn open(store: &FileStore) -> ZebFs {
    match store {
        FileStore::Local(files_dir) => ZebFs::Local(LocalZebFs::new(files_dir.clone())),
        FileStore::S3(config) => ZebFs::S3(S3ZebFs::new(config.clone())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The declaration and the FileRef field name the same thing, so they are
    /// spelled the same. A test says so, because two constants that drifted
    /// apart would make a stored FileRef unreadable by the backend that wrote
    /// it.
    #[test]
    fn the_declared_word_is_the_word_a_file_ref_carries() {
        assert_eq!(
            FileBackend::default().as_str(),
            crate::pipeline::nodes::shared::file_ref::BACKEND_ZEBFS
        );
    }

    #[test]
    fn an_absent_declaration_resolves_to_the_local_backend() {
        assert_eq!(FileBackend::resolve(None).unwrap(), FileBackend::Zebfs);
        assert_eq!(FileBackend::default().as_str(), "zebfs");
        assert_eq!(FileBackend::resolve(Some("s3")).unwrap(), FileBackend::S3);
    }

    #[test]
    fn a_resolved_local_store_opens_on_its_directory() {
        let files_dir = PathBuf::from("/tmp/zebflow-backend-test/files");
        let store = open(&FileStore::Local(files_dir.clone()));
        assert_eq!(store.backend(), FileBackend::Zebfs);
        assert_eq!(store.local_root(), Some(files_dir.as_path()));
    }

    #[test]
    fn an_unknown_backend_is_refused_by_name_with_the_accepted_list() {
        let err = FileBackend::parse("ftp").unwrap_err();
        assert_eq!(err.code, "ZEBFS_UNKNOWN_BACKEND");
        assert!(err.message.contains("'ftp'"), "{}", err.message);
        assert!(err.message.contains("accepted: zebfs, s3"), "{}", err.message);
    }
}
