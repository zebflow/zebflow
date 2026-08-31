//! ZebFS: Zebflow-native object-style filesystem for project artifacts.
//!
//! ZebFS gives Zebflow one object-storage mental model while the first backend
//! stays a fast local filesystem implementation.
//!
//! Which backend a project uses is declared, not assumed: `spec.files.backend`
//! in `repo/zebflow.yaml` names it, and [`backend::open`] is the one place that
//! declaration becomes an implementation.

pub mod acl;
pub mod backend;
pub mod error;
pub mod local;
pub mod model;

pub use acl::{ZebFsAccess, ZebFsAclScope};
pub use backend::{BACKEND_ZEBFS, FILE_BACKENDS, FileBackend};
pub use error::ZebFsError;
pub use local::{LocalZebFs, normalize_object_path};
pub use model::{ZebFsEntry, ZebFsEntryKind, ZebFsObject, ZebFsStat};
