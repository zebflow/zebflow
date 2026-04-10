//! Schema migration primitives for catalog-backed stores.
//!
//! The current SQLite catalog historically relied on one large `CREATE TABLE IF NOT EXISTS`
//! batch. Cluster-aware evolution needs an explicit migration ledger so worker registry,
//! placement, and execution metadata can be added predictably.

/// Declarative migration plan metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchemaMigrationPlan {
    /// Human-readable migration series name.
    pub series: &'static str,
    /// Latest expected schema version.
    pub latest_version: u32,
}

impl SchemaMigrationPlan {
    /// Build a new migration plan descriptor.
    pub const fn new(series: &'static str, latest_version: u32) -> Self {
        Self {
            series,
            latest_version,
        }
    }
}
