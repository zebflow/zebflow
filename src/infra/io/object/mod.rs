//! Object storage contracts.
//!
//! This layer will eventually cover:
//!
//! - repo bundles shipped from master to worker
//! - project files and assets
//! - future object-storage backends such as S3-compatible stores
//!
//! The first implementation remains local filesystem oriented because standalone installs
//! and single-node deployments are still primary.

pub mod interface;
pub mod local_fs;

pub use interface::{DynObjectStore, ObjectStore};
pub use local_fs::LocalFsObjectStore;
