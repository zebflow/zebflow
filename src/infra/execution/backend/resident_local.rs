//! Resident local execution backend.
//!
//! This backend represents the current monolithic execution shape where a pipeline run executes
//! inside the local process without a network hop.

use async_trait::async_trait;

use crate::infra::execution::backend::interface::{
    ExecutionBackend, ExecutionBackendKind, ExecutionError, ExecutionRequest,
};
use crate::infra::execution::handle::ExecutionHandle;

/// Whole-pipeline execution inside the local process.
#[derive(Debug, Clone, Default)]
pub struct ResidentLocalBackend;

#[async_trait]
impl ExecutionBackend for ResidentLocalBackend {
    fn id(&self) -> &'static str {
        "execution.resident_local"
    }

    fn kind(&self) -> ExecutionBackendKind {
        ExecutionBackendKind::ResidentLocal
    }

    async fn execute(&self, request: ExecutionRequest) -> Result<ExecutionHandle, ExecutionError> {
        let _ = request;
        Err(ExecutionError::new(
            "EXECUTION_BACKEND_UNWIRED",
            "resident local backend scaffold exists but is not wired to pipeline execution yet",
        ))
    }
}
