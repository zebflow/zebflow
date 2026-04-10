//! Resident remote-worker execution backend.
//!
//! This backend is intended for the first cluster-capable release where a worker hosts a long-lived
//! runtime and receives pipeline runs over a secure control transport.

use async_trait::async_trait;

use crate::infra::execution::backend::interface::{
    ExecutionBackend, ExecutionBackendKind, ExecutionError, ExecutionRequest,
};
use crate::infra::execution::handle::ExecutionHandle;

/// Whole-pipeline execution on a long-lived remote worker runtime.
#[derive(Debug, Clone, Default)]
pub struct ResidentWorkerBackend;

#[async_trait]
impl ExecutionBackend for ResidentWorkerBackend {
    fn id(&self) -> &'static str {
        "execution.resident_worker"
    }

    fn kind(&self) -> ExecutionBackendKind {
        ExecutionBackendKind::ResidentWorker
    }

    async fn execute(&self, request: ExecutionRequest) -> Result<ExecutionHandle, ExecutionError> {
        let _ = request;
        Err(ExecutionError::new(
            "EXECUTION_BACKEND_UNWIRED",
            "resident worker backend scaffold exists but secure remote execution is not wired yet",
        ))
    }
}
