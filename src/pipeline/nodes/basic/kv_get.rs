//! `n.kv.get` — retrieve a value from the project-scoped KV store.
//!
//! Merges `{ [out_key]: value }` into the flowing payload.
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
use crate::pipeline::model::NodeCapability;
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
        capabilities: vec![NodeCapability::Database],
        title: "KV Get".to_string(),
        description: "Read a value from the project-scoped KV store. \
            Ephemeral by default, use --durable for persistence across restarts. \
            Merges { [out_key]: value } into the flowing payload. \
            Use --out-key to control the output key name (defaults to the storage key). \
            Use --default to supply a fallback JSON value when the key is missing or expired. \
            Use $trigger or $nodes references for upstream data."
            .to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: json!({ "type": "object", "description": "The incoming payload with the retrieved value merged in under out_key." }),
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


/// The read result joins the flowing payload instead of erasing it.
///
/// `n.kv.set` keeps the payload it was handed; a `get` that threw everything
/// away made the pair asymmetric, and the Google-login callback had to reach
/// backwards with `ctx.nodes` to recover a value one read had destroyed. A
/// reader now behaves like a reader: everything that arrived is still there,
/// plus the value under `out_key` (which wins any name collision — asking for
/// a key called `code` means you want the stored one).
///
/// A non-object payload (a bare string or array flowing through) has nothing
/// to merge into, so it is replaced by `{ [out_key]: value }` exactly as
/// before.
fn merge_into_payload(
    payload: serde_json::Value,
    out_key: &str,
    value: serde_json::Value,
) -> serde_json::Value {
    match payload {
        serde_json::Value::Object(mut map) => {
            map.insert(out_key.to_string(), value);
            serde_json::Value::Object(map)
        }
        _ => serde_json::json!({ out_key: value }),
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
            payload: merge_into_payload(input.payload, &out_key, value),
            trace: vec![trace],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::merge_into_payload;
    use serde_json::json;

    /// The read joins the payload; it does not erase what was flowing.
    ///
    /// This is the manner fix the Google-login callback demanded: before it,
    /// `kv.get` destroyed the authorization code that arrived two nodes
    /// earlier, and the pipeline had to reach backwards with `ctx.nodes` to
    /// recover its own data.
    #[test]
    fn a_read_keeps_the_payload_it_was_handed() {
        let out = merge_into_payload(
            json!({ "code": "4/0AY", "state": "f381" }),
            "state_record",
            json!("stored"),
        );
        assert_eq!(
            out,
            json!({ "code": "4/0AY", "state": "f381", "state_record": "stored" })
        );
    }

    /// Asking for a key that already exists in the payload means you want the
    /// stored one — the read wins the collision.
    #[test]
    fn the_stored_value_wins_a_name_collision() {
        let out = merge_into_payload(json!({ "code": "from-upstream" }), "code", json!("stored"));
        assert_eq!(out, json!({ "code": "stored" }));
    }

    /// A bare string or array has nothing to merge into, so the old behaviour
    /// holds for it.
    #[test]
    fn a_non_object_payload_is_replaced_as_before() {
        let out = merge_into_payload(json!("just a string"), "value", json!(42));
        assert_eq!(out, json!({ "value": 42 }));
    }
}
