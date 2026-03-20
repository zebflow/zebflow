//! `n.logic.merge` — fan-in node.
//!
//! The merge node itself is a passthrough — it receives a payload and forwards it to `out`.
//! The actual fan-in strategy (wait_all, first_completed, pass_through) is handled by
//! the engine before this node fires. By the time execute_async is called, the payload
//! is already the combined/selected value.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::pipeline::{
    PipelineError, NodeDefinition,
    nodes::{NodeHandler, NodeExecutionInput, NodeExecutionOutput},
};
use crate::pipeline::model::LayoutItem;

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
        config_schema: Default::default(),
        dsl_flags: Default::default(),
        fields: {
            use crate::pipeline::model::{NodeFieldDef, NodeFieldType, SelectOptionDef};
            vec![
                NodeFieldDef { name: "title".to_string(), label: "Title".to_string(), field_type: NodeFieldType::Text, help: Some("Override display title for this node.".to_string()), ..Default::default() },
                NodeFieldDef { name: "strategy".to_string(), label: "Merge Strategy".to_string(), field_type: NodeFieldType::Select, options: vec![
                    SelectOptionDef { value: "first".to_string(), label: "First — pass through first arriving payload".to_string() },
                    SelectOptionDef { value: "all".to_string(), label: "All — wait for all inputs, emit array".to_string() },
                    SelectOptionDef { value: "merge".to_string(), label: "Merge — deep-merge all arriving objects".to_string() },
                ], ..Default::default() },
            ]
        },
        layout: vec![
            LayoutItem::Row { row: vec![LayoutItem::Field("title".to_string()), LayoutItem::Field("strategy".to_string())] },
        ],
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
impl NodeHandler for Node {
    fn kind(&self) -> &'static str { NODE_KIND }
    fn input_pins(&self) -> &'static [&'static str] { &[] }
    fn output_pins(&self) -> &'static [&'static str] { &[OUTPUT_PIN_OUT] }

    async fn execute_async(&self, input: NodeExecutionInput) -> Result<NodeExecutionOutput, PipelineError> {
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
