//! Execution handle model.
//!
//! Handles are the stable status/progress/log/cancel surface for long-running or remote work.
//! Even when the first clustered release still executes mostly synchronously, the handle model
//! should exist early so future remote, batch, or agent-team workloads do not need another abstraction.

pub mod model;

pub use model::{ExecutionHandle, ExecutionLogEntry, ExecutionStatus};
