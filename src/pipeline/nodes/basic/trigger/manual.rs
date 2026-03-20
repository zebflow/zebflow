//! Manual trigger node.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::pipeline::{
    PipelineError, NodeDefinition,
    nodes::{NodeHandler, NodeExecutionInput, NodeExecutionOutput},
};
use crate::pipeline::model::{LayoutItem, NodeFieldDef, NodeFieldType};

pub const NODE_KIND: &str = "n.trigger.manual";
pub const OUTPUT_PIN_OUT: &str = "out";

/// Unified node-definition metadata for `n.trigger.manual`.
pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Manual Trigger".to_string(),
        description: "Start pipeline run from explicit manual execute requests.".to_string(),
        input_schema: serde_json::json!({
            "type":"object",
            "description":"Manual execution payload."
        }),
        output_schema: serde_json::json!({
            "type":"object",
            "description":"Unmodified manual payload for downstream nodes."
        }),
        input_pins: vec![],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: Default::default(),
        fields: vec![
            NodeFieldDef { name: "title".to_string(), label: "Title".to_string(), field_type: NodeFieldType::Text, help: Some("Override display title for this node.".to_string()), ..Default::default() },
            NodeFieldDef { name: "__manual_note".to_string(), label: "Manual Trigger".to_string(), field_type: NodeFieldType::Text, readonly: true, default_value: Some(serde_json::json!("Runs only when pipeline execute trigger=manual.")), ..Default::default() },
        ],
        layout: vec![
            LayoutItem::Field("title".to_string()),
            LayoutItem::Field("__manual_note".to_string()),
        ],
        ai_tool: Default::default(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {}

pub struct Node;

impl Node {
    pub fn new(_config: Config) -> Self {
        Self
    }
}

#[async_trait]
impl NodeHandler for Node {
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
    ) -> Result<NodeExecutionOutput, PipelineError> {
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: input.payload,
            trace: vec![format!("node_kind={NODE_KIND}")],
        })
    }
}
