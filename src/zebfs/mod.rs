//! ZebFS: Zebflow-native object-style filesystem for project artifacts.
//!
//! ZebFS gives Zebflow one object-storage mental model while the first backend
//! stays a fast local filesystem implementation.

pub mod acl;
pub mod error;
pub mod local;
pub mod model;

pub use acl::{ZebFsAccess, ZebFsAclScope};
pub use error::ZebFsError;
pub use local::{LocalZebFs, normalize_object_path};
pub use model::{ZebFsEntry, ZebFsEntryKind, ZebFsObject, ZebFsStat};
