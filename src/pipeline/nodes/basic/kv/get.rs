//! `kv.entry.get` — read one entry: `entry: { key, value, found }`.
//!
//! A missing or expired key is an answer, not an error: `found: false` and
//! `value` is `--default` (or `null`).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{INPUT_PIN_IN, OUTPUT_PIN_OUT, answer, bus_error, durable_field, durable_flag, field, flag, key_flag, required_key, scope};
use crate::infra::io::state::DynStateBus;
use crate::pipeline::model::{LayoutItem, NodeCapability, NodeExample};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "kv.entry.get";
/// The store could not be read: the world's side.
pub const CODE: &str = "FW_NODE_KV_ENTRY_GET";
/// A flag set wrong.
pub const CONFIG_CODE: &str = "FW_NODE_KV_ENTRY_GET_CONFIG";
/// `--key` is empty.
pub const KEY_CODE: &str = "FW_NODE_KV_ENTRY_GET_KEY";
/// The engine has no state bus.
pub const UNAVAILABLE_CODE: &str = "FW_NODE_KV_ENTRY_GET_UNAVAILABLE";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Database],
        title: "KV Get".to_string(),
        description: "Read the entry `--key` from the project's key-value store (`--durable` for the disk-backed one). Adds \
            `entry: { key, value, found }` and keeps the rest of the payload; a missing or expired key answers `found: false` with \
            `value` set to `--default` (or null)."
            .to_string(),
        input_schema: json!({ "type": "object", "description": "Any payload; it is kept and `entry` is added." }),
        output_schema: json!({ "type": "object", "properties": { "entry": { "type": "object", "properties": {
            "key": { "type": "string" },
            "value": { "description": "The stored value, or --default when not found." },
            "found": { "type": "boolean" }
        } } } }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        dsl_flags: vec![
            key_flag("The entry to read. Usually a {{ }} expression."),
            flag("--default", "default", "The value answered when the key is missing or expired — a literal or {{ expr }}.", "json"),
            durable_flag(),
        ],
        fields: vec![
            field("key", "Key", "The entry to read: a literal or {{ expr }}."),
            field("default", "Default", "Answered as the value when the key is missing or expired (any JSON)."),
            durable_field(),
        ],
        layout: vec![
            LayoutItem::Field("key".to_string()),
            LayoutItem::Row { row: vec![LayoutItem::Field("default".to_string()), LayoutItem::Field("durable".to_string())] },
        ],
        examples: vec![
            NodeExample::dsl("Read a cached value with a fallback", r#"kv.entry.get --key "settings:{{ $trigger.params.site }}" --default "{{ { theme: 'light' } }}""#)
                .input(json!({ "webhook": { "params": { "site": "site-a" } } }))
                .output(json!({ "webhook": { "params": { "site": "site-a" } }, "entry": { "key": "settings:site-a", "value": { "theme": "dark" }, "found": true } }))
                .note("Read it downstream as `input.entry.value`; the rest of the payload stays."),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// The entry to read.
    #[serde(default)]
    pub key: Value,
    /// Answered when the key is missing; `null` when unset.
    #[serde(default)]
    pub default: Value,
    /// The disk-backed store.
    #[serde(default)]
    pub durable: bool,
}

pub struct Node {
    key: String,
    default: Value,
    durable: bool,
    bus: DynStateBus,
}

impl Node {
    pub fn new(config: Config, bus: DynStateBus) -> Result<Self, PipelineError> {
        let key = required_key(&config.key, CONFIG_CODE, KEY_CODE)?;
        Ok(Self { key, default: config.default, durable: config.durable, bus })
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
        let stored = if self.durable {
            self.bus.durable_get(&owner, &project, &self.key)
        } else {
            self.bus.get(&owner, &project, &self.key)
        }
        .map_err(bus_error(CODE))?;
        let found = stored.is_some();
        let value = stored.unwrap_or_else(|| self.default.clone());
        Ok(answer(
            &input.payload,
            json!({ "entry": { "key": self.key, "value": value, "found": found } }),
            format!("{NODE_KIND}: key={} found={found} durable={}", self.key, self.durable),
        ))
    }
}
