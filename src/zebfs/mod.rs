//! ZebFS: Zebflow-native object-style filesystem for project artifacts.
//!
//! ZebFS gives Zebflow one object-storage mental model with two backends: a
//! fast local filesystem, and any S3-compatible bucket.
//!
//! Which backend a project uses is declared, not assumed: `spec.files.backend`
//! in `repo/zebflow.yaml` names it, and [`backend::open`] is the one place that
//! declaration becomes an implementation.

pub mod acl;
pub mod backend;
pub mod error;
pub mod local;
pub mod model;
pub mod s3;
pub mod store;

pub use acl::{ZebFsAccess, ZebFsAclScope};
pub use backend::{BACKEND_S3, BACKEND_ZEBFS, FILE_BACKENDS, FileBackend, FileStore};
pub use error::ZebFsError;
pub use local::{LocalZebFs, normalize_object_path};
pub use model::{ZebFsEntry, ZebFsEntryKind, ZebFsObject, ZebFsStat};
pub use s3::{S3Config, S3ZebFs};
pub use store::ZebFs;
