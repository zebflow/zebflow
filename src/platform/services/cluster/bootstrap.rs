//! Cluster bootstrap service.
//!
//! This service is the product-facing owner of process role intent. In the first clustered
//! release it is expected to decide whether Zebflow should behave as:
//!
//! - standalone all-in-one
//! - control-plane controller
//! - execution-plane office
//!
//! The actual transport and certificate machinery belongs in `crate::infra::cluster`.

use crate::infra::cluster::config::{ClusterRole, ClusterSettings};

/// Product-facing bootstrap helper for cluster-aware startup.
#[derive(Debug, Clone, Default)]
pub struct ClusterBootstrapService {
    settings: ClusterSettings,
}

impl ClusterBootstrapService {
    /// Build a bootstrap service from explicit cluster settings.
    pub fn new(settings: ClusterSettings) -> Self {
        Self { settings }
    }

    /// Borrow the configured cluster settings.
    pub fn settings(&self) -> &ClusterSettings {
        &self.settings
    }

    /// Return the runtime role requested for this process.
    pub fn role(&self) -> ClusterRole {
        self.settings.role
    }

    /// Whether the process should run in standalone all-in-one mode.
    pub fn is_standalone(&self) -> bool {
        self.role() == ClusterRole::Standalone
    }

    /// Whether the process should behave as the control plane/controller.
    pub fn is_master(&self) -> bool {
        self.role() == ClusterRole::Master
    }

    /// Whether the process should behave as an execution-plane office node.
    pub fn is_worker(&self) -> bool {
        self.role() == ClusterRole::Worker
    }

    /// Effective stable node id, falling back to a role-derived local identifier.
    pub fn node_id(&self) -> String {
        self.settings
            .node_id
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| match self.role() {
                ClusterRole::Standalone => "standalone".to_string(),
                ClusterRole::Master => "controller".to_string(),
                ClusterRole::Worker => "office".to_string(),
            })
    }

    /// Human-readable node label.
    pub fn node_label(&self) -> String {
        self.settings
            .node_label
            .clone()
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| self.node_id())
    }

    /// Cluster join token used for internal control-plane requests.
    pub fn join_token(&self) -> Option<&str> {
        self.settings
            .join_token
            .as_deref()
            .filter(|value| !value.trim().is_empty())
    }

    /// Controller URL used by offices for registration and heartbeats.
    pub fn master_url(&self) -> Option<&str> {
        self.settings
            .master_url
            .as_deref()
            .filter(|value| !value.trim().is_empty())
    }

    /// Base URL this node advertises to the control plane.
    pub fn advertise_url(&self) -> Option<&str> {
        self.settings
            .advertise_url
            .as_deref()
            .filter(|value| !value.trim().is_empty())
    }
}
