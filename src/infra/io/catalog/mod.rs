//! Catalog store contracts.
//!
//! The catalog is the platform-owned metadata layer:
//!
//! - users
//! - projects
//! - credentials
//! - DB connection records
//! - worker registry and placement records
//! - invocation metadata and related control-plane state
//!
//! Current production code still uses the older `platform/adapters/data/*` path as the
//! active implementation. This module is the new home for the catalog abstraction that
//! future cluster-aware code should depend on.

pub mod interface;
pub mod migrations;
pub mod sqlite;

pub use interface::{CatalogStore, DynCatalogStore};
pub use migrations::SchemaMigrationPlan;
pub use sqlite::SqliteCatalogStore;
