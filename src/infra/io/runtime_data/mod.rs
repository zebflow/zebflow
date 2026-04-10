//! Runtime-data storage contracts.
//!
//! This layer owns the per-project runtime data roots that are not part of the git-synced repo.
//! Examples include:
//!
//! - local SQLite runtime DBs
//! - future external runtime data roots
//! - environment-owned data attached to a project runtime

pub mod interface;
pub mod sqlite;

pub use interface::{DynRuntimeDataStore, RuntimeDataStore};
pub use sqlite::SqliteRuntimeDataStore;
