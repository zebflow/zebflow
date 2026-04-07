//! `n.mem.exists` — check whether a key exists in the per-project in-memory KV store.
//!
//! Replaces the payload with `{ [out_key]: boolean }` (default out_key: "exists").
//! Useful for cache-check patterns before expensive lookups.
//!
//! # Config flags
//!
//! | Flag | Type | Default | Description |
//! |---|---|---|---|
//! | `--key` | string | required | Key to check (supports `{{ expr }}`) |
//! | `--out-key` | string | `"exists"` | Payload key to write the boolean result into |
//!
//! # Example
//!
//! ```text
//! | n.trigger.webhook --path /profile --method GET
//! | n.mem.exists --key "profile:{{ input.user_id }}" --out-key cached
//! | n.logic.if --cond "input.cached" --then cached-branch --else fetch-branch
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

pub const NODE_KIND: &str = "n.mem.exists";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Mem Exists".to_string(),
        description: "Check whether a key exists and is not expired in the per-project \
            in-memory KV store. Replaces the payload with { [out_key]: boolean } (default out_key: \"exists\"). \
            Useful for cache-hit checks before expensive DB queries or API calls. \
            Use $trigger or $nodes references for upstream data."
            .to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: json!({
            "type": "object",
            "description": "Fresh object with boolean result under out_key. Replaces entire payload."
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: json!({
            "type": "object",
            "required": ["key"],
            "properties": {
                "key": { "type": "string", "description": "Key to check. Supports {{ expr }}." },
                "out_key": { "type": "string", "description": "Payload key for the boolean result. Default: \"exists\"." },
            }
        }),
        dsl_flags: vec![
            DslFlag {
                flag: "--key".to_string(),
                config_key: "key".to_string(),
                description: "Key to check. Supports {{ expr }}.".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
            },
            DslFlag {
                flag: "--out-key".to_string(),
                config_key: "out_key".to_string(),
                description: "Payload key for the boolean result (default: \"exists\").".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: vec![
            NodeFieldDef { name: "title".to_string(), label: "Title".to_string(), field_type: NodeFieldType::Text, help: Some("Override display title.".to_string()), ..Default::default() },
            NodeFieldDef { name: "key".to_string(), label: "Key".to_string(), field_type: NodeFieldType::Text, help: Some("Key to check. Supports {{ expr }}.".to_string()), ..Default::default() },
            NodeFieldDef { name: "out_key".to_string(), label: "Output Key".to_string(), field_type: NodeFieldType::Text, help: Some("Payload key for the boolean result. Default: \"exists\".".to_string()), ..Default::default() },
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
            return Err(PipelineError::new("MEM_EXISTS_KEY", "n.mem.exists: --key is required"));
        }

        let exists = self.mem_hub.exists(owner, project, key);

        let out_key = if self.config.out_key.trim().is_empty() {
            "exists".to_string()
        } else {
            self.config.out_key.trim().to_string()
        };

        let trace = format!("n.mem.exists: key={} exists={} out_key={}", key, exists, out_key);
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: json!({ out_key: exists }),
            trace: vec![trace],
        })
    }
}
