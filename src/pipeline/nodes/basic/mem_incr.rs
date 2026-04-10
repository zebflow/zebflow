//! `n.mem.incr` — atomically increment (or decrement) an integer counter.
//!
//! The counter starts at 0 if the key doesn't exist.
//! Non-integer values are reset to 0 before applying the increment.
//! Replaces the payload with `{ [out_key]: new_value }`.
//!
//! # Config flags
//!
//! | Flag | Type | Default | Description |
//! |---|---|---|---|
//! | `--key` | string | required | Counter key |
//! | `--amount` | number | `1` | Increment amount (use negative to decrement) |
//! | `--out-key` | string | `""` | Payload key to write new counter value (defaults to `--key`) |
//!
//! # Example
//!
//! ```text
//! | n.trigger.webhook --path /click --method POST
//! | n.mem.incr --key "clicks:{{ input.button }}" --out-key total
//! | n.script -- "return { total: input.total };"
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::infra::io::state::DynStateBus;
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::pipeline::model::{DslFlag, DslFlagKind, NodeFieldDef, NodeFieldType};

pub const NODE_KIND: &str = "n.mem.incr";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Mem Incr".to_string(),
        description: "Atomically increment (or decrement with a negative amount) an integer counter \
            in the per-project in-memory store. Counter starts at 0 if the key is missing. \
            Replaces the payload with { [out_key]: new_value }. \
            Use $trigger or $nodes references for upstream data."
            .to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: json!({ "type": "object" }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: json!({
            "type": "object",
            "required": ["key"],
            "properties": {
                "key": { "type": "string" },
                "amount": { "type": "number", "description": "Increment amount. Use negative to decrement. Default: 1." },
                "out_key": { "type": "string", "description": "Payload key for the new counter value. Defaults to --key." },
            }
        }),
        dsl_flags: vec![
            DslFlag {
                flag: "--key".to_string(),
                config_key: "key".to_string(),
                description: "Counter key.".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
            },
            DslFlag {
                flag: "--amount".to_string(),
                config_key: "amount".to_string(),
                description: "Increment by this amount (default 1, negative to decrement).".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--out-key".to_string(),
                config_key: "out_key".to_string(),
                description: "Payload key for new counter value (default = same as --key).".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: vec![
            NodeFieldDef { name: "title".to_string(), label: "Title".to_string(), field_type: NodeFieldType::Text, help: Some("Override display title.".to_string()), ..Default::default() },
            NodeFieldDef { name: "key".to_string(), label: "Key".to_string(), field_type: NodeFieldType::Text, help: Some("Counter key. Supports {{ expr }}. Created at 0 if missing.".to_string()), ..Default::default() },
            NodeFieldDef { name: "amount".to_string(), label: "Amount".to_string(), field_type: NodeFieldType::Number, help: Some("Default 1. Use negative to decrement.".to_string()), ..Default::default() },
            NodeFieldDef { name: "out_key".to_string(), label: "Output Key".to_string(), field_type: NodeFieldType::Text, help: Some("Payload key for the new value. Defaults to --key.".to_string()), ..Default::default() },
        ],
        layout: vec![],
        ai_tool: Default::default(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub key: String,
    #[serde(default)]
    pub amount: Option<serde_json::Value>,
    #[serde(default)]
    pub out_key: String,
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
    fn kind(&self) -> &'static str { NODE_KIND }
    fn input_pins(&self) -> &'static [&'static str] { &[INPUT_PIN_IN] }
    fn output_pins(&self) -> &'static [&'static str] { &[OUTPUT_PIN_OUT] }

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        let owner = input.metadata.get("owner").and_then(Value::as_str).unwrap_or_default();
        let project = input.metadata.get("project").and_then(Value::as_str).unwrap_or_default();
        let key = self.config.key.trim();

        if key.is_empty() {
            return Err(PipelineError::new("MEM_INCR_KEY", "n.mem.incr: --key is required"));
        }

        let amount: i64 = match &self.config.amount {
            Some(Value::Number(n)) => n.as_i64().unwrap_or(1),
            Some(Value::String(s)) => s.trim().parse().unwrap_or(1),
            _ => 1,
        };

        let new_val = self
            .state_bus
            .incr(owner, project, key, amount)
            .map_err(|err| PipelineError::new("MEM_INCR_STATE_BUS", err.to_string()))?;

        let out_key = if self.config.out_key.trim().is_empty() {
            key.to_string()
        } else {
            self.config.out_key.trim().to_string()
        };

        let trace = format!(
            "n.mem.incr: key={} amount={} new_val={} out_key={}",
            key, amount, new_val, out_key
        );
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: json!({ out_key: new_val }),
            trace: vec![trace],
        })
    }
}
