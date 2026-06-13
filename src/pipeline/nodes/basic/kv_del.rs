//! `n.kv.del` — delete a key from the project-scoped KV store.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::infra::io::state::DynStateBus;
use crate::pipeline::model::{DslFlag, DslFlagKind, NodeFieldDef, NodeFieldType};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "n.kv.del";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "KV Del".to_string(),
        description: "Delete a key from the project-scoped KV store. Ephemeral by default, use --durable for persistence across restarts. Passes the payload through unchanged.".to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: json!({ "type": "object" }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: json!({
            "type": "object",
            "required": ["key"],
            "properties": {
                "key": { "type": "string", "description": "Key to delete." },
            }
        }),
        dsl_flags: vec![
            DslFlag {
                flag: "--key".to_string(),
                config_key: "key".to_string(),
                description: "Key to delete from the store.".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
            },
            DslFlag {
                flag: "--durable".to_string(),
                config_key: "durable".to_string(),
                description: "Persist to durable storage (survives restart). Default: ephemeral."
                    .to_string(),
                kind: DslFlagKind::Bool,
                required: false,
            },
        ],
        fields: vec![
            NodeFieldDef { name: "key".to_string(), label: "Key".to_string(), field_type: NodeFieldType::Text, help: Some("Key to delete.".to_string()), ..Default::default() },
            NodeFieldDef {
                name: "durable".to_string(),
                label: "Durable".to_string(),
                field_type: NodeFieldType::Checkbox,
                help: Some(
                    "Persist to durable storage (survives restart). Default: ephemeral."
                        .to_string(),
                ),
                ..Default::default()
            },
        ],
        layout: vec![],
        ai_tool: Default::default(),
        ..Default::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub durable: bool,
}

pub struct Node {
    config: Config,
    state_bus: DynStateBus,
}

impl Node {
    pub fn new(config: Config, state_bus: DynStateBus) -> Self {
        Self { config, state_bus }
    }
}

#[async_trait]
impl NodeHandler for Node {
    fn kind(&self) -> &'static str {
        NODE_KIND
    }
    fn input_pins(&self) -> &'static [&'static str] {
        &[INPUT_PIN_IN]
    }
    fn output_pins(&self) -> &'static [&'static str] {
        &[OUTPUT_PIN_OUT]
    }

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        let owner = input
            .metadata
            .get("owner")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let project = input
            .metadata
            .get("project")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let key = self.config.key.trim();

        if key.is_empty() {
            return Err(PipelineError::new(
                "KV_DEL_KEY",
                "n.kv.del: --key is required",
            ));
        }

        let existed = if self.config.durable {
            self.state_bus
                .durable_del(owner, project, key)
                .map_err(|err| PipelineError::new("KV_DEL_STATE_BUS", err.to_string()))?
        } else {
            self.state_bus
                .del(owner, project, key)
                .map_err(|err| PipelineError::new("KV_DEL_STATE_BUS", err.to_string()))?
        };

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: input.payload,
            trace: vec![format!(
                "n.kv.del: key={} existed={} durable={}",
                key, existed, self.config.durable
            )],
        })
    }
}
