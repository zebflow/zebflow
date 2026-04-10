//! Placeholder in-process control transport.
//!
//! This is a scaffold implementation used to keep the first refactor compiling while the real
//! secure stream transport is designed.

use async_trait::async_trait;

use super::interface::{ControlTransport, ControlTransportError};
use super::message::{ControlRequest, ControlResponse};

/// Temporary in-process control transport.
#[derive(Debug, Clone, Default)]
pub struct InProcessControlTransport;

#[async_trait]
impl ControlTransport for InProcessControlTransport {
    fn id(&self) -> &'static str {
        "cluster_transport.in_process"
    }

    async fn send(&self, request: ControlRequest) -> Result<ControlResponse, ControlTransportError> {
        Ok(ControlResponse::Rejected {
            code: "CONTROL_TRANSPORT_UNWIRED".to_string(),
            message: format!("control transport scaffold received request: {request:?}"),
        })
    }
}
