//! `n.mem.get` — retrieve a value from the per-project in-memory KV store.
//!
//! The retrieved value is merged into the flowing payload at `--out-key`
//! (defaults to the storage key itself).
//!
//! # Config flags
//!
//! | Flag | Type | Default | Description |
//! |---|---|---|---|
//! | `--key` | string | required | Storage key to retrieve |
//! | `--out-key` | string | `""` | Payload key to write into (default = same as `--key`) |
//! | `--default` | string | `null` | JSON value to inject if key is missing |
//!
//! # Example
//!
//! ```text
//! | n.mem.get --key "user:{{ input.user_id }}" --out-key profile
//! | n.script -- "return { name: input.profile?.name ?? 'Guest' };"
//! ```

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::infra::mem::MemHub;
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::pipeline::model::{DslFlag, DslFlagKind, NodeFieldDef, NodeFieldType};

pub const NODE_KIND: &str = "n.mem.get";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Mem Get".to_string(),
        description: "Read a value from the per-project in-memory KV store and merge it into the payload. \
            Use --out-key to control the payload key name (defaults to the storage key). \
            Use --default to supply a fallback JSON value when the key is missing or expired."
            .to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: json!({ "type": "object", "description": "Payload with the retrieved value merged in." }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: json!({
            "type": "object",
            "required": ["key"],
            "properties": {
                "key": { "type": "string", "description": "Storage key to retrieve." },
                "out_key": { "type": "string", "description": "Payload key to write the value into. Defaults to --key value." },
                "default": { "description": "Value to inject if key is missing. Accepts any JSON." },
            }
        }),
        dsl_flags: vec![
            DslFlag {
                flag: "--key".to_string(),
                config_key: "key".to_string(),
                description: "Storage key to retrieve.".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
            },
            DslFlag {
                flag: "--out-key".to_string(),
                config_key: "out_key".to_string(),
                description: "Payload key to write the value into (default = same as --key).".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: vec![
            NodeFieldDef { name: "title".to_string(), label: "Title".to_string(), field_type: NodeFieldType::Text, help: Some("Override display title.".to_string()), ..Default::default() },
            NodeFieldDef { name: "key".to_string(), label: "Key".to_string(), field_type: NodeFieldType::Text, help: Some("Storage key to retrieve.".to_string()), ..Default::default() },
            NodeFieldDef { name: "out_key".to_string(), label: "Output Key".to_string(), field_type: NodeFieldType::Text, help: Some("Payload key to inject the value under. Defaults to --key.".to_string()), ..Default::default() },
        ],
        layout: vec![],
        ai_tool: Default::default(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub out_key: String,
    #[serde(default)]
    pub default: Option<Value>,
}

pub struct Node {
    config: Config,
    mem_hub: Arc<MemHub>,
}

impl Node {
    pub fn new(config: Config, mem_hub: Arc<MemHub>) -> Self {
        Self { config, mem_hub }
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
            return Err(PipelineError::new("MEM_GET_KEY", "n.mem.get: --key is required"));
        }

        let value = self
            .mem_hub
            .get(owner, project, key)
            .or_else(|| self.config.default.clone())
            .unwrap_or(Value::Null);

        let out_key = if self.config.out_key.trim().is_empty() {
            key.to_string()
        } else {
            self.config.out_key.trim().to_string()
        };

        let mut payload = match input.payload {
            Value::Object(map) => map,
            other => {
                let mut m = serde_json::Map::new();
                m.insert("value".to_string(), other);
                m
            }
        };
        payload.insert(out_key.clone(), value);

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: Value::Object(payload),
            trace: vec![format!("n.mem.get: key={} out_key={}", key, out_key)],
        })
    }
}
