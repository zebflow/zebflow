//! Runner capability model.

use serde::{Deserialize, Serialize};

/// Declared runner capabilities used for matching workloads.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct RunnerCapabilities {
    /// Human-readable tags such as `web`, `batch`, `browser`, `spark`, or `gpu`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    /// Whether the runner can host resident workloads.
    #[serde(default)]
    pub supports_resident: bool,
    /// Whether the runner may eventually create Kubernetes jobs.
    #[serde(default)]
    pub supports_k8s_job: bool,
    /// Whether the runner may eventually handle Spark-submit style workloads.
    #[serde(default)]
    pub supports_spark_submit: bool,
}
