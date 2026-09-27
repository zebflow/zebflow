//! Worker registry models.

use serde::{Deserialize, Serialize};

use crate::infra::execution::runner::RunnerCapabilities;

/// One registered worker/runtime entry.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerRegistryRecord {
    /// Stable office identifier that owns this runtime node.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub office_id: String,
    /// Human-readable office slug.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub office_slug: String,
    /// Stable node identifier.
    pub node_id: String,
    /// Human-readable label.
    pub label: String,
    /// Base URL advertised by the worker for runtime/control traffic.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub base_url: String,
    /// Current status summary reported by the worker.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub status: String,
    /// Declared capabilities.
    #[serde(default)]
    pub capabilities: RunnerCapabilities,
    /// Unix timestamp when the worker first registered.
    #[serde(default)]
    pub registered_at: i64,
    /// Unix timestamp of the most recent heartbeat or registration refresh.
    #[serde(default)]
    pub last_heartbeat_at: i64,
    /// The projects this office holds, as of its last heartbeat.
    ///
    /// `offices.md` §3a: the office is the source of the directory, because a
    /// project may be created on it without the controller's **place**. The
    /// office reports its whole list on every heartbeat.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub projects: Vec<OfficeHeldProject>,
}

/// One project an office reports holding.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OfficeHeldProject {
    /// Owner slug on that office.
    pub owner: String,
    /// Project slug on that office.
    pub project: String,
    /// Display title.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub title: String,
}

/// Snapshot of all known workers.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct WorkerRegistrySnapshot {
    /// Known worker entries.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub workers: Vec<WorkerRegistryRecord>,
}
