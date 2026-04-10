//! Worker-join token model.

use serde::{Deserialize, Serialize};

/// One-time bootstrap token used by a worker to join a cluster.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JoinToken {
    /// Token identifier.
    pub token: String,
    /// Human-readable purpose or note.
    #[serde(default)]
    pub note: String,
}
