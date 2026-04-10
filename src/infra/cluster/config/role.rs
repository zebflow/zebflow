//! Cluster role definitions.
//!
//! `Master` and `Worker` are the current internal role names used by the first clustered
//! implementation.
//!
//! Product-facing terminology is moving toward:
//!
//! - `controller` for the control plane
//! - `office` for the execution-plane Zebflow instance

use serde::{Deserialize, Serialize};

/// Runtime role of the current process.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ClusterRole {
    /// One all-in-one process: current Zebflow behavior.
    #[default]
    Standalone,
    /// Control-plane role, also surfaced as `controller`.
    Master,
    /// Execution-plane role, also surfaced as `office`.
    Worker,
}
