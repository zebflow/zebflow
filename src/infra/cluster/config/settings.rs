//! Cluster settings used by the unified `zebflow` binary.

use serde::{Deserialize, Serialize};

use super::role::ClusterRole;

/// Cluster bootstrap and runtime settings.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClusterSettings {
    /// Runtime role for this process.
    pub role: ClusterRole,
    /// Stable node id when running as master or worker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_id: Option<String>,
    /// Human-readable node label for inventory and operator UI.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_label: Option<String>,
    /// Master URL used by workers to join and connect.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub master_url: Option<String>,
    /// Public/internal base URL this node advertises to the control plane.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub advertise_url: Option<String>,
    /// One-time join token supplied to a worker during bootstrap.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub join_token: Option<String>,
}

impl Default for ClusterSettings {
    fn default() -> Self {
        Self {
            role: ClusterRole::Standalone,
            node_id: None,
            node_label: None,
            master_url: None,
            advertise_url: None,
            join_token: None,
        }
    }
}
