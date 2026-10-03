//! `kv.entry.head` — whether one entry exists: `entry: { key, exists }`.
//!
//! The value is not read. The store keeps no remaining lifetime it can
//! report, so the answer carries no `ttl`.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{INPUT_PIN_IN, OUTPUT_PIN_OUT, answer, bus_error, durable_field, durable_flag, field, key_flag, required_key, scope};
use crate::infra::io::state::DynStateBus;
use crate::pipeline::model::{LayoutItem, NodeCapability, NodeExample};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "kv.entry.head";
/// The store could not be read: the world's side.
pub const CODE: &str = "FW_NODE_KV_ENTRY_HEAD";
/// A flag set wrong.
pub const CONFIG_CODE: &str = "FW_NODE_KV_ENTRY_HEAD_CONFIG";
/// `--key` is empty.
pub const KEY_CODE: &str = "FW_NODE_KV_ENTRY_HEAD_KEY";
/// The engine has no state bus.
pub const UNAVAILABLE_CODE: &str = "FW_NODE_KV_ENTRY_HEAD_UNAVAILABLE";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Database],
        title: "KV Head".to_string(),
        description: "Check whether the entry `--key` exists and is still live in the project's key-value store (`--durable` for the \
            disk-backed one), without reading its value. Adds `entry: { key, exists }` and keeps the rest of the payload — branch on \
            `input.entry.exists` with `logic.if`."
            .to_string(),
        input_schema: json!({ "type": "object", "description": "Any payload; it is kept and `entry` is added." }),
        output_schema: json!({ "type": "object", "properties": { "entry": { "type": "object", "properties": {
            "key": { "type": "string" },
            "exists": { "type": "boolean" }
        } } } }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        dsl_flags: vec![key_flag("The entry to check. Usually a {{ }} expression."), durable_flag()],
        fields: vec![field("key", "Key", "The entry to check: a literal or {{ expr }}."), durable_field()],
        layout: vec![LayoutItem::Row { row: vec![LayoutItem::Field("key".to_string()), LayoutItem::Field("durable".to_string())] }],
        examples: vec![
            NodeExample::dsl("Skip work that is already cached", r#"kv.entry.head --key "profile:{{ $trigger.params.id }}""#)
                .output(json!({ "entry": { "key": "profile:7", "exists": false } }))
                .note("Then `logic.if --expression \"input.entry.exists\"`."),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// The entry to check.
    #[serde(default)]
    pub key: Value,
    /// The disk-backed store.
    #[serde(default)]
    pub durable: bool,
}

pub struct Node {
    key: String,
    durable: bool,
    bus: DynStateBus,
}

impl Node {
    pub fn new(config: Config, bus: DynStateBus) -> Result<Self, PipelineError> {
        let key = required_key(&config.key, CONFIG_CODE, KEY_CODE)?;
        Ok(Self { key, durable: config.durable, bus })
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
        let exists = if self.durable {
            self.bus.durable_exists(&owner, &project, &self.key)
        } else {
            self.bus.exists(&owner, &project, &self.key)
        }
        .map_err(bus_error(CODE))?;
        Ok(answer(
            &input.payload,
            json!({ "entry": { "key": self.key, "exists": exists } }),
            format!("{NODE_KIND}: key={} exists={exists} durable={}", self.key, self.durable),
        ))
    }
}
