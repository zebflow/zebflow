//! `kv.entry.put` — write one entry: `entry: { key, ttl? }`.
//!
//! `--value` is required and an empty one is refused: the whole payload is
//! never stored by default.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{INPUT_PIN_IN, OUTPUT_PIN_OUT, answer, bus_error, durable_field, durable_flag, field, flag, key_flag, required_key, scope, ttl_seconds, ttl_text};
use crate::infra::io::state::DynStateBus;
use crate::pipeline::model::{DslFlag, LayoutItem, NodeCapability, NodeExample};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "kv.entry.put";
/// The store could not be written: the world's side.
pub const CODE: &str = "FW_NODE_KV_ENTRY_PUT";
/// A flag set wrong: a `--ttl` that is not a duration.
pub const CONFIG_CODE: &str = "FW_NODE_KV_ENTRY_PUT_CONFIG";
/// `--key` is empty.
pub const KEY_CODE: &str = "FW_NODE_KV_ENTRY_PUT_KEY";
/// `--value` is empty.
pub const VALUE_CODE: &str = "FW_NODE_KV_ENTRY_PUT_VALUE";
/// The engine has no state bus.
pub const UNAVAILABLE_CODE: &str = "FW_NODE_KV_ENTRY_PUT_UNAVAILABLE";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Database],
        title: "KV Put".to_string(),
        description: "Writes one entry into the project's key-value store. Write `--value` under `--key` in the project's key-value store (`--durable` for the disk-backed one), expiring \
            after `--ttl` (a duration such as 600s or 1d; omitted, it never expires). Adds `entry: { key, ttl? }` and keeps the rest \
            of the payload. An empty `--value` is refused."
            .to_string(),
        input_schema: json!({ "type": "object", "description": "Any payload; it is kept and `entry` is added." }),
        output_schema: json!({ "type": "object", "properties": { "entry": { "type": "object", "properties": {
            "key": { "type": "string" },
            "ttl": { "type": "string", "description": "The lifetime it was given, e.g. 600s; absent when it never expires." }
        } } } }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        dsl_flags: vec![
            key_flag("The entry to write. Usually a {{ }} expression."),
            DslFlag { required: true, ..flag("--value", "value", "What to store — a literal or {{ expr }}; a whole {{ }} keeps its type.", "json") },
            flag("--ttl", "ttl", "How long the entry lives, e.g. 600s, 10m or 1d. Omitted: it never expires.", "duration"),
            durable_flag(),
        ],
        fields: vec![
            field("key", "Key", "The entry to write: a literal or {{ expr }}."),
            field("value", "Value", "What to store: a literal or {{ expr }}."),
            field("ttl", "TTL", "How long it lives, e.g. 600s or 1d. Empty: forever."),
            durable_field(),
        ],
        layout: vec![
            LayoutItem::Field("key".to_string()),
            LayoutItem::Field("value".to_string()),
            LayoutItem::Row { row: vec![LayoutItem::Field("ttl".to_string()), LayoutItem::Field("durable".to_string())] },
        ],
        examples: vec![
            NodeExample::dsl("Remember an OAuth state for ten minutes", r#"kv.entry.put --key "oauth:state:{{ input.random.value }}" --value "{{ { started: new Date().toISOString() } }}" --ttl 10m"#)
                .output(json!({ "entry": { "key": "oauth:state:3f9c", "ttl": "600s" } }))
                .note("Read it back with `kv.entry.get --key …`; the payload passes on with `entry` added."),
            NodeExample::dsl("Cache a query result across restarts", r#"kv.entry.put --key "home:posts" --value "{{ input.query.rows }}" --ttl 5m --durable"#),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// The entry to write.
    #[serde(default)]
    pub key: Value,
    /// What to store, arriving final.
    #[serde(default)]
    pub value: Value,
    /// A duration; unset never expires.
    #[serde(default)]
    pub ttl: Value,
    /// The disk-backed store.
    #[serde(default)]
    pub durable: bool,
}

pub struct Node {
    key: String,
    value: Value,
    ttl: Option<u64>,
    durable: bool,
    bus: DynStateBus,
}

impl Node {
    pub fn new(config: Config, bus: DynStateBus) -> Result<Self, PipelineError> {
        let key = required_key(&config.key, CONFIG_CODE, KEY_CODE)?;
        let empty = config.value.is_null() || config.value.as_str().is_some_and(|s| s.trim().is_empty());
        if empty {
            return Err(PipelineError::new(VALUE_CODE, "--value is empty; it needs a value"));
        }
        let ttl = ttl_seconds(&config.ttl, CONFIG_CODE)?;
        Ok(Self { key, value: config.value, ttl, durable: config.durable, bus })
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

    async fn execute_async(&self, input: NodeExecutionInput) -> Result<NodeExecutionOutput, PipelineError> {
        let (owner, project) = scope(&input);
        if self.durable {
            self.bus.durable_set(&owner, &project, &self.key, self.value.clone(), self.ttl)
        } else {
            self.bus.set(&owner, &project, &self.key, self.value.clone(), self.ttl)
        }
        .map_err(bus_error(CODE))?;
        let mut entry = json!({ "key": self.key });
        if let Some(seconds) = self.ttl {
            entry["ttl"] = json!(ttl_text(seconds));
        }
        Ok(answer(
            &input.payload,
            json!({ "entry": entry }),
            format!("{NODE_KIND}: key={} ttl={:?} durable={}", self.key, self.ttl, self.durable),
        ))
    }
}
