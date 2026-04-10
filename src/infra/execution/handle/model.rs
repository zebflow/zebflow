//! Execution handle and status model.

use serde::{Deserialize, Serialize};

/// Current lifecycle state of an execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionStatus {
    Pending,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

/// One execution log line or event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionLogEntry {
    /// Unix timestamp seconds.
    pub at: i64,
    /// Human-readable line payload.
    pub line: String,
}

/// Stable handle for a pipeline execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExecutionHandle {
    /// Opaque execution identifier.
    pub execution_id: String,
    /// Current lifecycle state.
    pub status: ExecutionStatus,
    /// Optional runner currently handling the execution.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runner_id: Option<String>,
    /// Freeform progress label.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<String>,
}
