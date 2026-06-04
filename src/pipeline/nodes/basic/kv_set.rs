//! `n.kv.set` — store a value in the project-scoped KV store.
//!
//! # Config flags
//!
//! | Flag | Type | Default | Description |
//! |---|---|---|---|
//! | `--key` | string | required | Storage key (supports `{{ expr }}`) |
//! | `--value-path` | string | `""` | JSON pointer into payload to extract value; empty = whole payload |
//! | `--ttl` | number | `0` | TTL in seconds; 0 = no expiry |
//! | `--durable` | bool | `false` | Persist to durable storage (survives restart) |
//!
//! # Example
//!
//! ```text
//! | n.trigger.webhook --path /save --method POST
//! | n.kv.set --key "user:{{ input.user_id }}" --value-path /data --ttl 3600
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::infra::io::state::DynStateBus;
use crate::pipeline::model::{DslFlag, DslFlagKind, NodeFieldDef, NodeFieldType};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "n.kv.set";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "KV Set".to_string(),
        description: "Store a value in the project-scoped KV store. \
            Ephemeral by default, use --durable for persistence across restarts. \
            Use --key to name the slot (supports {{ expr }}). \
            Use --value-path to extract a sub-value from the payload; empty = whole payload. \
            Use --ttl for automatic expiry in seconds (0 = forever)."
            .to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: json!({ "type": "object", "description": "Payload passed through unchanged." }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: json!({
            "type": "object",
            "required": ["key"],
            "properties": {
                "key": { "type": "string", "description": "Storage key. Supports {{ expr }}." },
                "value_path": { "type": "string", "description": "JSON pointer into payload (e.g. /user/name). Empty = whole payload." },
                "ttl": { "type": "number", "description": "TTL in seconds. 0 = no expiry." },
            }
        }),
        dsl_flags: vec![
            DslFlag {
                flag: "--key".to_string(),
                config_key: "key".to_string(),
                description: "Storage key. Supports {{ expr }}.".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
            },
            DslFlag {
                flag: "--value-path".to_string(),
                config_key: "value_path".to_string(),
                description: "JSON pointer into payload to store. Empty = whole payload."
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--ttl".to_string(),
                config_key: "ttl".to_string(),
                description: "TTL in seconds (0 = no expiry).".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
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
            NodeFieldDef {
                name: "key".to_string(),
                label: "Key".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Storage key. Supports {{ expr }}.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "value_path".to_string(),
                label: "Value Path".to_string(),
                field_type: NodeFieldType::Text,
                help: Some(
                    "JSON pointer into payload (e.g. /data). Empty = whole payload.".to_string(),
                ),
                ..Default::default()
            },
            NodeFieldDef {
                name: "ttl".to_string(),
                label: "TTL (seconds)".to_string(),
                field_type: NodeFieldType::Number,
                help: Some("Auto-expire after N seconds. 0 = forever.".to_string()),
                ..Default::default()
            },
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
    pub value_path: String,
    #[serde(default)]
    pub ttl: Option<u64>,
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
                "KV_SET_KEY",
                "n.kv.set: --key is required",
            ));
        }

        let value = if self.config.value_path.is_empty() {
            input.payload.clone()
        } else {
            let ptr = if self.config.value_path.starts_with('/') {
                self.config.value_path.clone()
            } else {
                format!("/{}", self.config.value_path)
            };
            input.payload.pointer(&ptr).cloned().unwrap_or(Value::Null)
        };

        let ttl = self.config.ttl.filter(|&t| t > 0);
        if self.config.durable {
            self.state_bus
                .durable_set(owner, project, key, value, ttl)
                .map_err(|err| PipelineError::new("KV_SET_STATE_BUS", err.to_string()))?;
        } else {
            self.state_bus
                .set(owner, project, key, value, ttl)
                .map_err(|err| PipelineError::new("KV_SET_STATE_BUS", err.to_string()))?;
        }

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: input.payload,
            trace: vec![format!("n.kv.set: key={} ttl={:?} durable={}", key, ttl, self.config.durable)],
        })
    }
}
