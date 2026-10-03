//! `kv.entry.delete` — remove one entry: `entry: { key, deleted }`.
//!
//! Deleting a key that is not there succeeds with `deleted: false`. Use it
//! to consume a one-time value — an OAuth state, a reset token — right after
//! reading it, so it cannot be used twice.

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

pub const NODE_KIND: &str = "kv.entry.delete";
/// The store could not be written: the world's side.
pub const CODE: &str = "FW_NODE_KV_ENTRY_DELETE";
/// A flag set wrong.
pub const CONFIG_CODE: &str = "FW_NODE_KV_ENTRY_DELETE_CONFIG";
/// `--key` is empty.
pub const KEY_CODE: &str = "FW_NODE_KV_ENTRY_DELETE_KEY";
/// The engine has no state bus.
pub const UNAVAILABLE_CODE: &str = "FW_NODE_KV_ENTRY_DELETE_UNAVAILABLE";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Database],
        title: "KV Delete".to_string(),
        description: "Delete the entry `--key` from the project's key-value store (`--durable` for the disk-backed one). Adds \
            `entry: { key, deleted }` and keeps the rest of the payload; a key that is not there answers `deleted: false`. Use it to \
            consume a one-time value — an OAuth state, a reset token — right after `kv.entry.get` read it, so it cannot be used twice."
            .to_string(),
        input_schema: json!({ "type": "object", "description": "Any payload; it is kept and `entry` is added." }),
        output_schema: json!({ "type": "object", "properties": { "entry": { "type": "object", "properties": {
            "key": { "type": "string" },
            "deleted": { "type": "boolean", "description": "Whether the key was there." }
        } } } }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        dsl_flags: vec![key_flag("The entry to delete. Usually a {{ }} expression."), durable_flag()],
        fields: vec![field("key", "Key", "The entry to delete: a literal or {{ expr }}."), durable_field()],
        layout: vec![LayoutItem::Row { row: vec![LayoutItem::Field("key".to_string()), LayoutItem::Field("durable".to_string())] }],
        examples: vec![
            NodeExample::dsl("Consume a one-time state", r#"kv.entry.delete --key "oauth:state:{{ $trigger.query.state }}""#)
                .output(json!({ "entry": { "key": "oauth:state:3f9c", "deleted": true } })),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// The entry to delete.
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
        let deleted = if self.durable {
            self.bus.durable_del(&owner, &project, &self.key)
        } else {
            self.bus.del(&owner, &project, &self.key)
        }
        .map_err(bus_error(CODE))?;
        Ok(answer(
            &input.payload,
            json!({ "entry": { "key": self.key, "deleted": deleted } }),
            format!("{NODE_KIND}: key={} deleted={deleted} durable={}", self.key, self.durable),
        ))
    }
}
