//! Worker heartbeat payload model.

use serde::{Deserialize, Serialize};

use crate::infra::execution::runner::RunnerCapabilities;

/// Health report sent from a worker to the control plane.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerHeartbeat {
    /// Worker identifier.
    pub node_id: String,
    /// Unix timestamp seconds.
    pub at: i64,
    /// Human-readable status summary.
    pub status: String,
    /// Current worker base URL if it changed since registration.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub base_url: String,
    /// Optional capability refresh carried with the heartbeat.
    #[serde(default)]
    pub capabilities: RunnerCapabilities,
}
