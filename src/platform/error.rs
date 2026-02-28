//! Shared error model for Zebflow platform module.

use std::fmt::{Display, Formatter};

/// Error returned by platform adapters/services/web composition.
#[derive(Debug, Clone)]
pub struct PlatformError {
    /// Stable machine-readable code.
    pub code: &'static str,
    /// Human-readable detail.
    pub message: String,
}

impl PlatformError {
    /// Creates a new error with code + message.
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl Display for PlatformError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for PlatformError {}

impl From<std::io::Error> for PlatformError {
    fn from(value: std::io::Error) -> Self {
        Self::new("PLATFORM_IO", value.to_string())
    }
}

impl From<serde_json::Error> for PlatformError {
    fn from(value: serde_json::Error) -> Self {
        Self::new("PLATFORM_JSON", value.to_string())
    }
}
