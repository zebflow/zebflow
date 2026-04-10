//! Trait definitions for master/worker control transport.

use std::fmt;
use std::sync::Arc;

use async_trait::async_trait;

use super::message::{ControlRequest, ControlResponse};

/// Control transport error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlTransportError {
    /// Stable error code.
    pub code: String,
    /// Human-readable message.
    pub message: String,
}

impl ControlTransportError {
    /// Build a new transport error.
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for ControlTransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for ControlTransportError {}

/// Secure control-plane transport contract.
#[async_trait]
pub trait ControlTransport: Send + Sync {
    /// Stable implementation identifier.
    fn id(&self) -> &'static str;

    /// Send one control request and await the reply.
    async fn send(&self, request: ControlRequest) -> Result<ControlResponse, ControlTransportError>;
}

/// Shared control-transport trait object.
pub type DynControlTransport = Arc<dyn ControlTransport>;
