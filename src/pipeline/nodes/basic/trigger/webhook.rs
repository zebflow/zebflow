//! Webhook trigger node.

use crate::pipeline::{
    FrameworkError, NodeDefinition,
    nodes::{FrameworkNode, NodeExecutionInput, NodeExecutionOutput},
};
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

pub const NODE_KIND: &str = "n.trigger.webhook";
pub const OUTPUT_PIN_OUT: &str = "out";

/// Unified node-definition metadata for `n.trigger.webhook`.
pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Webhook Trigger".to_string(),
        description: "Start pipeline run from inbound HTTP path + method.".to_string(),
        input_schema: serde_json::json!({
            "type":"object",
            "description":"Request payload forwarded from webhook ingress."
        }),
        output_schema: serde_json::json!({
            "type":"object",
            "description":"Unmodified request payload for downstream nodes."
        }),
        input_pins: vec![],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        ai_tool: Default::default(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub path: String,
    #[serde(default = "default_method")]
    pub method: String,
}

fn default_method() -> String {
    "GET".to_string()
}

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
    fn kind(&self) -> &'static str {
        NODE_KIND
    }
    fn input_pins(&self) -> &'static [&'static str] {
        &[]
    }
    fn output_pins(&self) -> &'static [&'static str] {
        &[OUTPUT_PIN_OUT]
    }

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, FrameworkError> {
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: input.payload,
            trace: vec![
                format!("node_kind={NODE_KIND}"),
                format!("method={}", self.config.method),
                format!("path={}", self.config.path),
            ],
        })
    }
}
