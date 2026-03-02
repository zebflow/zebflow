//! Shared automaton-layer domain model.

use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// High-level objective for an automaton run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomatonObjective {
    /// Stable objective id.
    pub objective_id: String,
    /// Natural-language goal.
    pub goal: String,
    /// Optional structured input payload.
    #[serde(default)]
    pub input: Value,
}

/// Runtime context supplied when planning/executing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomatonContext {
    /// Owner/tenant id.
    pub owner: String,
    /// Project id.
    pub project: String,
    /// Run id for traceability.
    pub run_id: String,
    /// Maximum allowed execution steps.
    pub step_budget: u32,
    /// Additional metadata envelope.
    #[serde(default)]
    pub metadata: Value,
}

/// One explicit automaton plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomatonPlan {
    /// Source objective id.
    pub objective_id: String,
    /// Ordered plain-language steps.
    pub steps: Vec<String>,
    /// Optional engine-specific metadata.
    #[serde(default)]
    pub metadata: Value,
}

/// Final execution result status.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AutomatonResult {
    Succeeded,
    Failed,
    Cancelled,
}

/// Automaton execution output envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomatonExecutionOutput {
    /// Final result state.
    pub result: AutomatonResult,
    /// Final output payload.
    #[serde(default)]
    pub output: Value,
    /// Trace entries emitted by the engine.
    #[serde(default)]
    pub trace: Vec<String>,
}

/// Automaton layer error model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomatonError {
    /// Stable error code.
    pub code: &'static str,
    /// Human-readable error message.
    pub message: String,
}

impl AutomatonError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl Display for AutomatonError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for AutomatonError {}
