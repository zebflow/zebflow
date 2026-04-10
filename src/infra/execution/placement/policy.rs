//! Placement policy definitions.

use serde::{Deserialize, Serialize};

/// Placement policy kinds for whole-pipeline execution routing.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlacementPolicyKind {
    /// Execute on the local node/runtime.
    Local,
    /// Execute on one explicitly selected runner/worker.
    Pinned,
    /// Execute on a runner matching selector tags/capabilities.
    Selector,
}

/// Placement policy attached to a project or a pipeline override.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlacementPolicy {
    /// Placement mode.
    pub kind: PlacementPolicyKind,
    /// Explicit pinned runner id when `kind == pinned`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pinned_runner_id: Option<String>,
    /// Required tags or labels when `kind == selector`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_tags: Vec<String>,
}

impl PlacementPolicy {
    /// Local execution policy.
    pub fn local() -> Self {
        Self {
            kind: PlacementPolicyKind::Local,
            pinned_runner_id: None,
            required_tags: Vec::new(),
        }
    }
}
