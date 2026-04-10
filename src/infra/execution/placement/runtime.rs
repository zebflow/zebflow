//! Portable project runtime profile.
//!
//! This model captures repo-owned runtime intent that should survive movement between standalone,
//! shared-worker, pinned-worker, dedicated-runtime, and future independent-platform deployments.
//!
//! It intentionally does not include concrete worker ids or cluster-local addresses. Those are
//! environment-owned placement details.

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::infra::execution::backend::ExecutionProfileKind;

/// Runtime mode chosen for one project.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ProjectRuntimeMode {
    /// Project runs in a shared worker pool.
    #[default]
    Shared,
    /// Project is pinned to one chosen worker or worker-class.
    Pinned,
    /// Project gets its own dedicated runtime deployment or VM.
    Dedicated,
}

impl ProjectRuntimeMode {
    /// Stable string form used by UI payloads and portable config displays.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Shared => "shared",
            Self::Pinned => "pinned",
            Self::Dedicated => "dedicated",
        }
    }
}

impl fmt::Display for ProjectRuntimeMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Named resource sizing profile.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ResourceProfile {
    Tiny,
    #[default]
    Small,
    Medium,
    Large,
    /// Explicit resource numbers are supplied in `custom_resources`.
    Custom,
}

/// Explicit runtime resource overrides.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct RuntimeResourceSpec {
    /// Requested CPU in millicores.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_request_millis: Option<u32>,
    /// Requested memory in megabytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_request_mb: Option<u32>,
    /// Hard CPU limit in millicores.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cpu_limit_millis: Option<u32>,
    /// Hard memory limit in megabytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub memory_limit_mb: Option<u32>,
    /// Optional ephemeral disk in megabytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ephemeral_disk_mb: Option<u32>,
    /// Optional accelerator/GPU count.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accelerator_count: Option<u16>,
}

/// Portable runtime profile stored in repo config and migration plans.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectRuntimeProfile {
    /// Runtime isolation mode.
    #[serde(default)]
    pub mode: ProjectRuntimeMode,
    /// Portable execution profile.
    #[serde(default)]
    pub execution: ExecutionProfileKind,
    /// Named resource sizing tier.
    #[serde(default)]
    pub resource_profile: ResourceProfile,
    /// Desired minimum number of resident replicas.
    #[serde(default = "default_min_replicas")]
    pub min_replicas: u32,
    /// Optional ceiling for autoscaling.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_replicas: Option<u32>,
    /// Required runner tags/capabilities for placement.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_tags: Vec<String>,
    /// Explicit resource numbers used when `resource_profile == custom`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_resources: Option<RuntimeResourceSpec>,
}

const fn default_min_replicas() -> u32 {
    1
}

impl Default for ProjectRuntimeProfile {
    fn default() -> Self {
        Self {
            mode: ProjectRuntimeMode::Shared,
            execution: ExecutionProfileKind::Resident,
            resource_profile: ResourceProfile::Small,
            min_replicas: default_min_replicas(),
            max_replicas: None,
            required_tags: Vec::new(),
            custom_resources: None,
        }
    }
}

impl ProjectRuntimeProfile {
    /// Whether this profile carries explicit resource overrides.
    pub fn uses_custom_resources(&self) -> bool {
        self.resource_profile == ResourceProfile::Custom && self.custom_resources.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::{ProjectRuntimeMode, ProjectRuntimeProfile, ResourceProfile};
    use crate::infra::execution::backend::ExecutionProfileKind;

    #[test]
    fn default_runtime_profile_is_portable() {
        let profile = ProjectRuntimeProfile::default();
        assert_eq!(profile.mode, ProjectRuntimeMode::Shared);
        assert_eq!(profile.execution, ExecutionProfileKind::Resident);
        assert_eq!(profile.resource_profile, ResourceProfile::Small);
        assert_eq!(profile.min_replicas, 1);
        assert!(profile.max_replicas.is_none());
    }
}
