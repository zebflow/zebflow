//! `n.kv.get` — retrieve a value from the project-scoped KV store.
//!
//! Replaces the payload with `{ [out_key]: value }`.
//! Use `$trigger` or `$nodes` references for upstream data.
//!
//! # Config flags
//!
//! | Flag | Type | Default | Description |
//! |---|---|---|---|
//! | `--key` | string | required | Storage key to retrieve |
//! | `--out-key` | string | `""` | Payload key to write into (default = same as `--key`) |
//! | `--default` | string | `null` | JSON value to inject if key is missing |
//! | `--durable` | bool | `false` | Read from durable storage (survives restart) |
//!
//! # Example
//!
//! ```text
//! | n.kv.get --key "user:{{ input.user_id }}" --out-key profile
//! | n.script -- "return { name: input.profile?.name ?? 'Guest' };"
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

pub const NODE_KIND: &str = "n.kv.get";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "KV Get".to_string(),
        description: "Read a value from the project-scoped KV store. \
            Ephemeral by default, use --durable for persistence across restarts. \
            Replaces the payload with { [out_key]: value }. \
            Use --out-key to control the output key name (defaults to the storage key). \
            Use --default to supply a fallback JSON value when the key is missing or expired. \
            Use $trigger or $nodes references for upstream data."
            .to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: json!({ "type": "object", "description": "Fresh object with the retrieved value under out_key. Replaces entire payload." }),
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
                description: "Payload key to write the value into (default = same as --key)."
                    .to_string(),
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
                help: Some("Storage key to retrieve. Supports {{ expr }}.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "out_key".to_string(),
                label: "Output Key".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Payload key to inject the value under. Defaults to --key.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "default".to_string(),
                label: "Default".to_string(),
                field_type: NodeFieldType::Text,
                help: Some(
                    "Fallback value (any JSON) to inject when the key is missing or expired."
                        .to_string(),
                ),
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
    pub out_key: String,
    #[serde(default)]
    pub default: Option<Value>,
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
                "KV_GET_KEY",
                "n.kv.get: --key is required",
            ));
        }

        let value = if self.config.durable {
            self.state_bus
                .durable_get(owner, project, key)
                .map_err(|err| PipelineError::new("KV_GET_STATE_BUS", err.to_string()))?
                .or_else(|| self.config.default.clone())
                .unwrap_or(Value::Null)
        } else {
            self.state_bus
                .get(owner, project, key)
                .map_err(|err| PipelineError::new("KV_GET_STATE_BUS", err.to_string()))?
                .or_else(|| self.config.default.clone())
                .unwrap_or(Value::Null)
        };

        let out_key = if self.config.out_key.trim().is_empty() {
            key.to_string()
        } else {
            self.config.out_key.trim().to_string()
        };

        let trace = format!(
            "n.kv.get: key={} out_key={} durable={}",
            key, out_key, self.config.durable
        );
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: json!({ out_key: value }),
            trace: vec![trace],
        })
    }
}
