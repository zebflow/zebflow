//! Framework domain model for pipeline graph execution.

use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Pipeline graph contract for framework orchestration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineGraph {
    /// Graph contract kind marker.
    #[serde(default = "default_pipeline_kind")]
    pub kind: String,
    /// Graph contract version.
    #[serde(default = "default_pipeline_version")]
    pub version: String,
    /// Unique pipeline id.
    pub id: String,
    /// Node ids that can start execution.
    #[serde(default)]
    pub entry_nodes: Vec<String>,
    /// Node list.
    pub nodes: Vec<PipelineNode>,
    /// Directed pin edges.
    pub edges: Vec<PipelineEdge>,
}

/// Executable node definition in a pipeline graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineNode {
    /// Unique node id in graph scope.
    pub id: String,
    /// Node kind id (for example `x.n.web.render`).
    pub kind: String,
    /// Input pin names.
    #[serde(default, alias = "inputs")]
    pub input_pins: Vec<String>,
    /// Output pin names.
    #[serde(default, alias = "outputs")]
    pub output_pins: Vec<String>,
    /// Node-specific configuration blob.
    #[serde(default)]
    pub config: Value,
}

/// Directed pin-like connection between nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineEdge {
    /// Source node id.
    #[serde(alias = "from")]
    pub from_node: String,
    /// Source output pin.
    pub from_pin: String,
    /// Target node id.
    #[serde(alias = "to")]
    pub to_node: String,
    /// Target input pin.
    pub to_pin: String,
}

/// Runtime context for a framework run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkContext {
    /// Owner/tenant id.
    pub owner: String,
    /// Project id.
    pub project: String,
    /// Pipeline id.
    pub pipeline: String,
    /// Request/run id.
    pub request_id: String,
    /// Trigger payload.
    pub input: Value,
}

/// Standard framework execution output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkOutput {
    /// Final output payload.
    pub value: Value,
    /// Ordered trace entries emitted by framework.
    pub trace: Vec<String>,
}

/// Framework layer error model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrameworkError {
    /// Stable error code.
    pub code: &'static str,
    /// Human-readable error message.
    pub message: String,
}

impl FrameworkError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

impl Display for FrameworkError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

impl std::error::Error for FrameworkError {}

fn default_pipeline_kind() -> String {
    "zebflow.pipeline".to_string()
}

fn default_pipeline_version() -> String {
    "0.1".to_string()
}
