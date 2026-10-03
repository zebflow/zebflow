//! `kv.entry.expire` — set or clear an entry's lifetime without touching its
//! value: `entry: { key, ttl?, updated }`.
//!
//! `--ttl 30m` gives the entry thirty minutes from now; without `--ttl` the
//! expiry is removed and the entry lives until deleted. A key that is not
//! there answers `updated: false`.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{INPUT_PIN_IN, OUTPUT_PIN_OUT, answer, bus_error, durable_field, durable_flag, field, flag, key_flag, required_key, scope, ttl_seconds, ttl_text};
use crate::infra::io::state::DynStateBus;
use crate::pipeline::model::{LayoutItem, NodeCapability, NodeExample};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "kv.entry.expire";
/// The store could not be written: the world's side.
pub const CODE: &str = "FW_NODE_KV_ENTRY_EXPIRE";
/// A flag set wrong: a `--ttl` that is not a duration.
pub const CONFIG_CODE: &str = "FW_NODE_KV_ENTRY_EXPIRE_CONFIG";
/// `--key` is empty.
pub const KEY_CODE: &str = "FW_NODE_KV_ENTRY_EXPIRE_KEY";
/// The engine has no state bus.
pub const UNAVAILABLE_CODE: &str = "FW_NODE_KV_ENTRY_EXPIRE_UNAVAILABLE";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Database],
        title: "KV Expire".to_string(),
        description: "Give the entry `--key` a new lifetime `--ttl` (a duration such as 30m, counted from now) without changing its \
            value; without `--ttl` the expiry is removed and it lives until deleted. `--durable` for the disk-backed store. Adds \
            `entry: { key, ttl?, updated }` and keeps the rest of the payload; a key that is not there answers `updated: false`."
            .to_string(),
        input_schema: json!({ "type": "object", "description": "Any payload; it is kept and `entry` is added." }),
        output_schema: json!({ "type": "object", "properties": { "entry": { "type": "object", "properties": {
            "key": { "type": "string" },
            "ttl": { "type": "string", "description": "The new lifetime, e.g. 1800s; absent when the expiry was removed." },
            "updated": { "type": "boolean", "description": "Whether the key was there." }
        } } } }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        dsl_flags: vec![
            key_flag("The entry whose lifetime changes. Usually a {{ }} expression."),
            flag("--ttl", "ttl", "The new lifetime from now, e.g. 30m or 1d. Omitted: the expiry is removed.", "duration"),
            durable_flag(),
        ],
        fields: vec![
            field("key", "Key", "The entry: a literal or {{ expr }}."),
            field("ttl", "TTL", "The new lifetime, e.g. 30m. Empty: never expires."),
            durable_field(),
        ],
        layout: vec![
            LayoutItem::Field("key".to_string()),
            LayoutItem::Row { row: vec![LayoutItem::Field("ttl".to_string()), LayoutItem::Field("durable".to_string())] },
        ],
        examples: vec![
            NodeExample::dsl("Keep an active session alive", r#"kv.entry.expire --key "session:{{ $trigger.auth.sub }}" --ttl 30m"#)
                .output(json!({ "entry": { "key": "session:u_1", "ttl": "1800s", "updated": true } })),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// The entry.
    #[serde(default)]
    pub key: Value,
    /// A duration; unset removes the expiry.
    #[serde(default)]
    pub ttl: Value,
    /// The disk-backed store.
    #[serde(default)]
    pub durable: bool,
}

pub struct Node {
    key: String,
    ttl: Option<u64>,
    durable: bool,
    bus: DynStateBus,
}

impl Node {
    pub fn new(config: Config, bus: DynStateBus) -> Result<Self, PipelineError> {
        let key = required_key(&config.key, CONFIG_CODE, KEY_CODE)?;
        let ttl = ttl_seconds(&config.ttl, CONFIG_CODE)?;
        Ok(Self { key, ttl, durable: config.durable, bus })
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
        let updated = if self.durable {
            self.bus.durable_expire(&owner, &project, &self.key, self.ttl)
        } else {
            self.bus.expire(&owner, &project, &self.key, self.ttl)
        }
        .map_err(bus_error(CODE))?;
        let mut entry = json!({ "key": self.key, "updated": updated });
        if let Some(seconds) = self.ttl {
            entry["ttl"] = json!(ttl_text(seconds));
        }
        Ok(answer(
            &input.payload,
            json!({ "entry": entry }),
            format!("{NODE_KIND}: key={} ttl={:?} updated={updated} durable={}", self.key, self.ttl, self.durable),
        ))
    }
}
