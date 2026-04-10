//! `n.mem.del` — delete a key from the per-project in-memory KV store.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::infra::io::state::DynStateBus;
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::pipeline::model::{DslFlag, DslFlagKind, NodeFieldDef, NodeFieldType};

pub const NODE_KIND: &str = "n.mem.del";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Mem Del".to_string(),
        description: "Delete a key from the per-project in-memory KV store. Passes the payload through unchanged.".to_string(),
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
        ],
        fields: vec![
            NodeFieldDef { name: "title".to_string(), label: "Title".to_string(), field_type: NodeFieldType::Text, help: Some("Override display title.".to_string()), ..Default::default() },
            NodeFieldDef { name: "key".to_string(), label: "Key".to_string(), field_type: NodeFieldType::Text, help: Some("Key to delete.".to_string()), ..Default::default() },
        ],
        layout: vec![],
        ai_tool: Default::default(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub key: String,
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
    fn kind(&self) -> &'static str { NODE_KIND }
    fn input_pins(&self) -> &'static [&'static str] { &[INPUT_PIN_IN] }
    fn output_pins(&self) -> &'static [&'static str] { &[OUTPUT_PIN_OUT] }

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        let owner = input.metadata.get("owner").and_then(Value::as_str).unwrap_or_default();
        let project = input.metadata.get("project").and_then(Value::as_str).unwrap_or_default();
        let key = self.config.key.trim();

        if key.is_empty() {
            return Err(PipelineError::new("MEM_DEL_KEY", "n.mem.del: --key is required"));
        }

        let existed = self
            .state_bus
            .del(owner, project, key)
            .map_err(|err| PipelineError::new("MEM_DEL_STATE_BUS", err.to_string()))?;

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: input.payload,
            trace: vec![format!("n.mem.del: key={} existed={}", key, existed)],
        })
    }
}
