//! Common framework-node execution contract.

use async_trait::async_trait;
use serde_json::Value;

use crate::framework::FrameworkError;
use crate::framework::model::StepEvent;

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
    /// When set, node may stream step events (e.g. Zebtune: thinking, tool_call, external).
    pub step_tx: Option<tokio::sync::mpsc::UnboundedSender<StepEvent>>,
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
#[async_trait]
pub trait FrameworkNode: Send + Sync {
    /// Stable node kind id (for example `n.web.render`).
    fn kind(&self) -> &'static str;
    /// Supported input pin names.
    fn input_pins(&self) -> &'static [&'static str];
    /// Supported output pin names.
    fn output_pins(&self) -> &'static [&'static str];

    /// Executes node business logic asynchronously for one input envelope.
    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, FrameworkError>;

    /// Blocking wrapper for non-async call sites.
    fn execute(&self, input: NodeExecutionInput) -> Result<NodeExecutionOutput, FrameworkError> {
        if tokio::runtime::Handle::try_current().is_ok() {
            return Err(FrameworkError::new(
                "FW_NODE_SYNC_IN_ASYNC",
                "synchronous FrameworkNode::execute used inside async runtime; call execute_async instead",
            ));
        }
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|err| FrameworkError::new("FW_NODE_RUNTIME", err.to_string()))?;
        runtime.block_on(self.execute_async(input))
    }
}
