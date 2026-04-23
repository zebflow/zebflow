//! Environment-owned project runtime placement records.
//!
//! These records complement the portable `ProjectRuntimeProfile` stored in `zebflow.json`.
//! The portable profile answers:
//!
//! - how isolated the project wants to be
//! - which execution backend class it expects
//! - what resource profile it prefers
//!
//! This module answers the environment-owned side:
//!
//! - which runtime authority currently owns the project
//! - whether the authority is local to the current node or remote
//! - which worker id should receive runtime traffic
//!
//! The intent is to keep repo-owned configuration portable while still allowing the
//! control plane to dispatch projects differently in each environment.

use serde::{Deserialize, Serialize};

use super::runtime::ProjectRuntimeMode;

/// Where the project's resident runtime currently lives.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProjectRuntimePlacementTarget {
    /// The project runs on the current node/runtime.
    #[default]
    Local,
    /// The project runs on a registered remote worker.
    Worker,
}

/// Durable environment-owned placement record for one project runtime.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectRuntimePlacement {
    /// Stable project id in the platform catalog.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub project_id: String,
    /// Owner identifier.
    pub owner: String,
    /// Project slug.
    pub project: String,
    /// Portable runtime mode echoed here for easy operator inspection.
    pub mode: ProjectRuntimeMode,
    /// Placement target class.
    pub target: ProjectRuntimePlacementTarget,
    /// Stable office id that owns the runtime.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_office_id: Option<String>,
    /// Stable node id selected for runtime traffic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_node_id: Option<String>,
    /// Stable worker id when `target == worker`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub worker_id: Option<String>,
    /// Requested resource profile for operator inspection.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub resource_profile: String,
    /// Desired replica count for the runtime target.
    #[serde(default)]
    pub desired_replicas: u32,
    /// Current effective state summary.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub effective_state: String,
    /// Unix timestamp when the placement was first created.
    pub created_at: i64,
    /// Unix timestamp when the placement was most recently updated.
    pub updated_at: i64,
}
