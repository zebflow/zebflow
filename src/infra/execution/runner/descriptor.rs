//! Runner identity descriptor.

use serde::{Deserialize, Serialize};

use super::capabilities::RunnerCapabilities;

/// Concrete execution target metadata.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RunnerDescriptor {
    /// Stable runner identifier.
    pub runner_id: String,
    /// Human-readable label.
    pub label: String,
    /// Capability declaration used by placement logic.
    #[serde(default)]
    pub capabilities: RunnerCapabilities,
}
