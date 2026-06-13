//! `n.kv.expire` — update the TTL of an existing key without changing its value.
//!
//! Pass `--ttl 0` to remove the expiry (persist the key forever).
//! Payload passes through unchanged.
//!
//! # Config flags
//!
//! | Flag | Type | Default | Description |
//! |---|---|---|---|
//! | `--key` | string | required | Key to update (supports `{{ expr }}`) |
//! | `--ttl` | number | required | New TTL in seconds; 0 = remove expiry (persist) |
//! | `--durable` | bool | `false` | Expire in durable storage (survives restart). Default: ephemeral. |
//!
//! # Example
//!
//! ```text
//! | n.trigger.webhook --path /refresh --method POST
//! | n.kv.expire --key "session:{{ input.token }}" --ttl 1800
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

pub const NODE_KIND: &str = "n.kv.expire";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "KV Expire".to_string(),
        description: "Update the TTL of an existing key in the per-project KV store \
            without changing its value. Pass --ttl 0 to remove the expiry (persist forever). \
            No-ops silently if the key is missing or already expired. \
            Use --durable to target durable (disk-backed) storage instead of ephemeral. \
            Payload passes through unchanged."
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
                "key": { "type": "string", "description": "Key to update. Supports {{ expr }}." },
                "ttl": { "type": "number", "description": "New TTL in seconds. 0 = remove expiry (persist)." },
            }
        }),
        dsl_flags: vec![
            DslFlag {
                flag: "--key".to_string(),
                config_key: "key".to_string(),
                description: "Key to update. Supports {{ expr }}.".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
            },
            DslFlag {
                flag: "--ttl".to_string(),
                config_key: "ttl".to_string(),
                description: "New TTL in seconds. 0 = remove expiry (persist forever).".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--durable".to_string(),
                config_key: "durable".to_string(),
                description: "Expire in durable storage (survives restart). Default: ephemeral."
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
                help: Some("Key to update. Supports {{ expr }}.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "ttl".to_string(),
                label: "TTL (seconds)".to_string(),
                field_type: NodeFieldType::Number,
                help: Some(
                    "New TTL in seconds. 0 = remove expiry and persist forever.".to_string(),
                ),
                ..Default::default()
            },
            NodeFieldDef {
                name: "durable".to_string(),
                label: "Durable".to_string(),
                field_type: NodeFieldType::Checkbox,
                help: Some("Check durable storage. Default: ephemeral.".to_string()),
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
                "KV_EXPIRE_KEY",
                "n.kv.expire: --key is required",
            ));
        }

        let updated = if self.config.durable {
            self.state_bus
                .durable_expire(owner, project, key, self.config.ttl)
                .map_err(|err| PipelineError::new("KV_EXPIRE_STATE_BUS", err.to_string()))?
        } else {
            self.state_bus
                .expire(owner, project, key, self.config.ttl)
                .map_err(|err| PipelineError::new("KV_EXPIRE_STATE_BUS", err.to_string()))?
        };

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: input.payload,
            trace: vec![format!(
                "n.kv.expire: key={} ttl={:?} updated={} durable={}",
                key, self.config.ttl, updated, self.config.durable
            )],
        })
    }
}
