//! Common framework-node execution contract.

use serde_json::Value;

use crate::framework::FrameworkError;

/// Input envelope received by a node when it is triggered by an incoming edge.
#[derive(Debug, Clone)]
pub struct NodeExecutionInput {
    /// Runtime node id from pipeline graph.
    pub node_id: String,
    /// Target input pin name on the node.
    pub input_pin: String,
    /// Payload received from upstream node/trigger.
    pub payload: Value,
    /// Additional metadata envelope carried by the framework.
    pub metadata: Value,
}

/// Output envelope produced by a node execution.
#[derive(Debug, Clone)]
pub struct NodeExecutionOutput {
    /// Selected output pin to route next edges.
    pub output_pin: String,
    /// Output payload for downstream nodes.
    pub payload: Value,
    /// Node-local execution trace entries.
    pub trace: Vec<String>,
}

/// Node interface implemented by every framework node kind.
pub trait FrameworkNode: Send + Sync {
    /// Stable node kind id (for example `x.n.web.render`).
    fn kind(&self) -> &'static str;
    /// Supported input pin names.
    fn input_pins(&self) -> &'static [&'static str];
    /// Supported output pin names.
    fn output_pins(&self) -> &'static [&'static str];

    /// Executes node business logic for one input envelope.
    fn execute(&self, input: NodeExecutionInput) -> Result<NodeExecutionOutput, FrameworkError>;
}
