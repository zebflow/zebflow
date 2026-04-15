//! `n.trigger.function` — marks a pipeline as a callable function unit.
//!
//! # Pipeline position
//!
//! Always the first (and only trigger) node in a function pipeline.
//! Function pipelines are reusable callable units invoked from other pipelines
//! via `n.function.call` or exposed as Project Operator tools.
//!
//! # User-facing config
//! | Field | Type | Required | Description |
//! |---|---|---|---|
//! | `params` | object | no | JSON Schema properties describing function inputs |
//!
//! # DSL
//! ```text
//! | trigger.function --params '{"user_id": {"type": "string"}}'
//! | script -- return { greeting: "hello " + input.user_id }
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::pipeline::model::{DslFlag, DslFlagKind, NodeFieldDef, NodeFieldType};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "n.trigger.function";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// JSON Schema `properties` object defining input parameters.
    /// Example: `{"user_id": {"type": "string", "description": "User to look up"}}`
    #[serde(default)]
    pub params: Value,
}

pub struct Node {
    #[allow(dead_code)]
    config: Config,
}

impl Node {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Function Trigger".to_string(),
        description: "Marks this pipeline as a callable function. The pipeline can be invoked \
            from other pipelines via n.function.call or exposed as a Project Operator tool. \
            Define the input parameters it accepts in the params field.".to_string(),
        input_pins: vec![],
        output_pins: vec!["out".to_string()],
        config_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "params": {
                    "type": "object",
                    "description": "JSON Schema properties object describing the function's inputs."
                }
            }
        }),
        dsl_flags: vec![
            DslFlag {
                flag: "--params".to_string(),
                config_key: "params".to_string(),
                description: "JSON Schema properties object for this function's inputs. \
                    Example: {\"user_id\": {\"type\": \"string\"}}"
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: vec![
            NodeFieldDef {
                name: "params".to_string(),
                label: "Input Parameters".to_string(),
                field_type: NodeFieldType::ParamsBuilder,
                help: Some(
                    "Define the inputs this function expects. Callers will see these as labeled fields.".to_string(),
                ),
                span: Some("full".to_string()),
                ..Default::default()
            },
        ],
        ..Default::default()
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
        &["out"]
    }

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        // Passthrough — the caller's payload is injected before dispatch.
        Ok(NodeExecutionOutput {
            output_pins: vec!["out".to_string()],
            payload: input.payload,
            trace: vec![format!("node_kind={NODE_KIND}: passthrough")],
        })
    }
}
