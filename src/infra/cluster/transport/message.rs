//! Typed master/worker control messages.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Control request sent over the master/worker transport.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ControlRequest {
    /// Worker registration request.
    RegisterWorker { node_id: String, metadata: Value },
    /// Heartbeat message from worker to master.
    Heartbeat { node_id: String, metadata: Value },
    /// Project bundle sync request.
    SyncProject {
        owner: String,
        project: String,
        bundle: Value,
    },
}

/// Control response sent back over the master/worker transport.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ControlResponse {
    /// Generic success response.
    Ack { message: String },
    /// Error-like response payload.
    Rejected { code: String, message: String },
}
