//! Trait definitions for whole-pipeline execution backends.

use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::infra::execution::handle::ExecutionHandle;
use crate::infra::execution::placement::PlacementPolicy;

/// Named execution backend kinds reserved by the architecture.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionBackendKind {
    ResidentLocal,
    ResidentWorker,
    DockerJob,
    K8sJob,
    SparkSubmit,
}

/// Portable execution profile vocabulary stored in project-level configuration and migration plans.
///
/// This intentionally hides whether resident execution resolves to a local process or a remote
/// worker. That resolution is environment-owned and should happen later during placement.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionProfileKind {
    /// Long-lived resident runtime, local or remote depending on placement.
    #[default]
    Resident,
    /// One isolated Docker-style job/container per execution.
    DockerJob,
    /// One Kubernetes Job/Pod per execution.
    K8sJob,
    /// One Spark-submit style execution.
    SparkSubmit,
}

impl ExecutionBackendKind {
    /// Portable execution profile represented by this concrete backend.
    pub fn profile_kind(self) -> ExecutionProfileKind {
        match self {
            Self::ResidentLocal | Self::ResidentWorker => ExecutionProfileKind::Resident,
            Self::DockerJob => ExecutionProfileKind::DockerJob,
            Self::K8sJob => ExecutionProfileKind::K8sJob,
            Self::SparkSubmit => ExecutionProfileKind::SparkSubmit,
        }
    }
}

/// Execution request passed to a backend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionRequest {
    /// Owner identifier.
    pub owner: String,
    /// Project identifier.
    pub project: String,
    /// Canonical pipeline file path.
    pub file_rel_path: String,
    /// JSON payload supplied to the run.
    #[serde(default)]
    pub input: Value,
    /// Optional placement hint or policy.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub placement: Option<PlacementPolicy>,
    /// Backend-specific metadata.
    #[serde(default)]
    pub metadata: Value,
}

/// Backend execution error.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionError {
    /// Stable error code.
    pub code: String,
    /// Human-readable explanation.
    pub message: String,
}

impl ExecutionError {
    /// Build a new execution error.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    /// Convenience constructor for scaffold backends that are not yet implemented.
    pub fn not_implemented(name: &str) -> Self {
        Self::new(
            "EXECUTION_BACKEND_NOT_IMPLEMENTED",
            format!("execution backend '{name}' is not implemented yet"),
        )
    }
}

impl fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ExecutionError {}

/// Contract for whole-pipeline execution backends.
#[async_trait]
pub trait ExecutionBackend: Send + Sync {
    /// Stable backend identifier.
    fn id(&self) -> &'static str;

    /// Backend kind for routing and serialization.
    fn kind(&self) -> ExecutionBackendKind;

    /// Start executing a whole-pipeline request.
    async fn execute(&self, request: ExecutionRequest) -> Result<ExecutionHandle, ExecutionError>;
}

/// Shared backend trait object.
pub type DynExecutionBackend = Arc<dyn ExecutionBackend>;
