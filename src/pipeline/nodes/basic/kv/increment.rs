//! `kv.entry.increment` — add to an integer counter: `entry: { key, value }`.
//!
//! A missing key starts at 0. `--amount` is a whole number (negative counts
//! down); unset it is 1, and anything that is not a whole number is refused,
//! never read as 1.

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

pub const NODE_KIND: &str = "kv.entry.increment";
/// The store could not be written: the world's side.
pub const CODE: &str = "FW_NODE_KV_ENTRY_INCREMENT";
/// A flag set wrong.
pub const CONFIG_CODE: &str = "FW_NODE_KV_ENTRY_INCREMENT_CONFIG";
/// `--key` is empty.
pub const KEY_CODE: &str = "FW_NODE_KV_ENTRY_INCREMENT_KEY";
/// `--amount` is not a whole number.
pub const AMOUNT_CODE: &str = "FW_NODE_KV_ENTRY_INCREMENT_AMOUNT";
/// The engine has no state bus.
pub const UNAVAILABLE_CODE: &str = "FW_NODE_KV_ENTRY_INCREMENT_UNAVAILABLE";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Database],
        title: "KV Increment".to_string(),
        description: "Counts an integer counter in the project's key-value store up or down. Add `--amount` (a whole number, default 1, negative to count down) to the integer counter `--key` in the \
            project's key-value store (`--durable` for the disk-backed one); a missing key starts at 0. Adds `entry: { key, value }` — \
            the new count — and keeps the rest of the payload."
            .to_string(),
        input_schema: json!({ "type": "object", "description": "Any payload; it is kept and `entry` is added." }),
        output_schema: json!({ "type": "object", "properties": { "entry": { "type": "object", "properties": {
            "key": { "type": "string" },
            "value": { "type": "integer", "description": "The count after the increment." }
        } } } }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        dsl_flags: vec![
            key_flag("The counter. Usually a {{ }} expression."),
            flag("--amount", "amount", "What to add: a whole number, default 1, negative to count down.", "number"),
            durable_flag(),
        ],
        fields: vec![
            field("key", "Key", "The counter: a literal or {{ expr }}."),
            field("amount", "Amount", "A whole number to add (default 1; negative counts down)."),
            durable_field(),
        ],
        layout: vec![
            LayoutItem::Field("key".to_string()),
            LayoutItem::Row { row: vec![LayoutItem::Field("amount".to_string()), LayoutItem::Field("durable".to_string())] },
        ],
        examples: vec![
            NodeExample::dsl("Count clicks per button", r#"kv.entry.increment --key "clicks:{{ $trigger.body.button }}""#)
                .output(json!({ "entry": { "key": "clicks:buy", "value": 42 } }))
                .note("The new count is `input.entry.value`."),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// The counter.
    #[serde(default)]
    pub key: Value,
    /// A whole number; unset is 1.
    #[serde(default)]
    pub amount: Value,
    /// The disk-backed store.
    #[serde(default)]
    pub durable: bool,
}

pub struct Node {
    key: String,
    amount: i64,
    durable: bool,
    bus: DynStateBus,
}

impl Node {
    pub fn new(config: Config, bus: DynStateBus) -> Result<Self, PipelineError> {
        let key = required_key(&config.key, CONFIG_CODE, KEY_CODE)?;
        let refuse = |raw: &str| PipelineError::new(AMOUNT_CODE, format!("--amount '{raw}' is not a whole number"));
        let amount = match &config.amount {
            Value::Null => 1,
            Value::Number(n) => n.as_i64().ok_or_else(|| refuse(&n.to_string()))?,
            Value::String(s) if s.trim().is_empty() => 1,
            Value::String(s) => s.trim().parse().map_err(|_| refuse(s))?,
            other => return Err(refuse(&other.to_string())),
        };
        Ok(Self { key, amount, durable: config.durable, bus })
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
        let value = if self.durable {
            self.bus.durable_incr(&owner, &project, &self.key, self.amount)
        } else {
            self.bus.incr(&owner, &project, &self.key, self.amount)
        }
        .map_err(bus_error(CODE))?;
        Ok(answer(
            &input.payload,
            json!({ "entry": { "key": self.key, "value": value } }),
            format!("{NODE_KIND}: key={} amount={} value={value} durable={}", self.key, self.amount, self.durable),
        ))
    }
}
