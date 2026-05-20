//! Common framework-node execution contract.
//!
//! Every node receives a [`NodeExecutionInput`] and returns a [`NodeExecutionOutput`].
//! The engine wraps each execution with a timeout (default 30s, overridable per-node
//! via `--timeout <seconds>` in DSL or `timeout_secs` in the node's config JSON).

use async_trait::async_trait;
use serde_json::Value;

use crate::pipeline::PipelineError;
use crate::pipeline::model::ExecutionBus;

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
    /// Execution-scoped broadcast bus.  Nodes that want to emit observable signals
    /// (progress, thinking steps, metrics) call `bus.emit(Signal { .. })`.
    /// `None` when the caller did not request signal streaming.
    pub bus: Option<std::sync::Arc<ExecutionBus>>,
}

/// Output envelope produced by a node execution.
#[derive(Debug, Clone)]
pub struct NodeExecutionOutput {
    /// Output pin(s) to route next edges.
    /// Single-output nodes return one entry. Choice nodes return one named output pin.
    pub output_pins: Vec<String>,
    /// Output payload for downstream nodes.
    pub payload: Value,
    /// Node-local execution trace entries.
    pub trace: Vec<String>,
}

/// Node interface implemented by every framework node kind.
#[async_trait]
pub trait NodeHandler: Send + Sync {
    /// Stable node kind id (for example `n.web.response`).
    fn kind(&self) -> &'static str;
    /// Supported input pin names.
    fn input_pins(&self) -> &'static [&'static str];
    /// Supported output pin names.
    fn output_pins(&self) -> &'static [&'static str];

    /// Executes node business logic asynchronously for one input envelope.
    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError>;

    /// Executes node business logic and may emit multiple downstream outputs from one input.
    ///
    /// Default behavior wraps `execute_async` as a single emission so existing nodes do not
    /// need any changes. Nodes like `n.logic.foreach` override this to emit many item runs.
    async fn execute_many_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<Vec<NodeExecutionOutput>, PipelineError> {
        Ok(vec![self.execute_async(input).await?])
    }

    /// Blocking wrapper for non-async call sites.
    fn execute(&self, input: NodeExecutionInput) -> Result<NodeExecutionOutput, PipelineError> {
        if tokio::runtime::Handle::try_current().is_ok() {
            return Err(PipelineError::new(
                "FW_NODE_SYNC_IN_ASYNC",
                "synchronous NodeHandler::execute used inside async runtime; call execute_async instead",
            ));
        }
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|err| PipelineError::new("FW_NODE_RUNTIME", err.to_string()))?;
        runtime.block_on(self.execute_async(input))
    }
}
