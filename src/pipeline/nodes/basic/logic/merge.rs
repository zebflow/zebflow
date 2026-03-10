//! `n.logic.merge` — fan-in node.
//!
//! The merge node itself is a passthrough — it receives a payload and forwards it to `out`.
//! The actual fan-in strategy (wait_all, first_completed, pass_through) is handled by
//! the engine before this node fires. By the time execute_async is called, the payload
//! is already the combined/selected value.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::pipeline::{
    FrameworkError, NodeDefinition,
    nodes::{FrameworkNode, NodeExecutionInput, NodeExecutionOutput},
};

pub const NODE_KIND: &str = "n.logic.merge";
pub const OUTPUT_PIN_OUT: &str = "out";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Merge".to_string(),
        description: "Fan-in node. Collects multiple branch results and forwards to a single output. strategy: wait_all | first_completed | pass_through.".to_string(),
        input_schema: serde_json::json!({ "type": "object" }),
        output_schema: serde_json::json!({ "type": "object" }),
        input_pins: vec![], // dynamic — defined per instance in the graph
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        ai_tool: Default::default(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default = "default_strategy")]
    pub strategy: String,
}

fn default_strategy() -> String { "pass_through".to_string() }

pub struct Node {
    config: Config,
}

impl Node {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

#[async_trait]
impl FrameworkNode for Node {
    fn kind(&self) -> &'static str { NODE_KIND }
    fn input_pins(&self) -> &'static [&'static str] { &[] }
    fn output_pins(&self) -> &'static [&'static str] { &[OUTPUT_PIN_OUT] }

    async fn execute_async(&self, input: NodeExecutionInput) -> Result<NodeExecutionOutput, FrameworkError> {
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: input.payload,
            trace: vec![
                format!("node_kind={NODE_KIND}"),
                format!("strategy={}", self.config.strategy),
            ],
        })
    }
}
