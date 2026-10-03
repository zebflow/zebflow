//! `kv.message.publish` — publish a message on a named channel in the project KV bus.
//!
//! All active pipelines subscribed via `trigger.topic` on the same
//! channel will receive the message and fire.
//!
//! # Config flags
//!
//! | Flag | Type | Default | Description |
//! |---|---|---|---|
//! | `--channel` | string | required | Channel name |
//! | `--payload` | any | whole payload | What to publish — a literal or `{{ expr }}` |
//!
//! # Example
//!
//! ```text
//! | trigger.webhook --route /alert --method POST
//! | kv.message.publish --channel notifications
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

pub const NODE_KIND: &str = "kv.message.publish";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Database],
        title: "KV Publish".to_string(),
        description: "Publish a message on a named channel in the project KV bus. \
            All pipelines listening via trigger.topic on the same channel receive the message. \
            Use --payload to send a literal or {{ expr }}; omit it to send the whole payload."
            .to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: json!({ "type": "object", "description": "Payload passed through unchanged." }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: json!({
            "type": "object",
            "required": ["channel"],
            "properties": {
                "channel": { "type": "string", "description": "Channel name. Supports {{ expr }}." },
                "payload": { "description": "What to publish — a literal or {{ expr }}. Omit to publish the whole payload." },
            }
        }),
        dsl_flags: vec![
            DslFlag {
                flag: "--channel".to_string(),
                config_key: "channel".to_string(),
                description: "Channel name to publish to. Supports {{ expr }}.".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
                ..Default::default()
            },
            DslFlag {
                flag: "--payload".to_string(),
                config_key: "payload".to_string(),
                description: "What to publish — a literal or {{ expr }}. Omit to publish the whole payload.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
                ..Default::default()
            },
        ],
        fields: vec![
            NodeFieldDef { name: "channel".to_string(), label: "Channel".to_string(), field_type: NodeFieldType::Text, help: Some("Channel name. Supports {{ expr }}.".to_string()), ..Default::default() },
            NodeFieldDef { name: "payload".to_string(), label: "Payload Path".to_string(), field_type: NodeFieldType::Text, help: Some("What to publish — a literal or {{ expr }}. Omit to publish the whole payload.".to_string()), ..Default::default() },
        ],
        layout: vec![],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Hand off slow work after answering", r#"kv.message.publish --channel order.placed --payload "{{ { order_id: input.query.rows[0]._key, email: $trigger.body.email } }}""#)
                .note("A pipeline starting with `trigger.topic --topic order.placed` receives it as `input.topic.message`."),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub channel: String,
    #[serde(default)]
    pub payload: Value,
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
        let channel = self.config.channel.trim();

        if channel.is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_KV_PUBLISH_CHANNEL",
                "kv.message.publish: --channel is required",
            ));
        }

        // The value arrives final — a whole `{{ }}` carries its typed
        // value (NodeIO §Value resolution). Null means "not set", so the
        // whole payload is stored, matching the old empty-path default.
        let message = if self.config.payload.is_null() {
            input.payload.clone()
        } else {
            self.config.payload.clone()
        };

        let receivers = self
            .state_bus
            .publish(owner, project, channel, message)
            .map_err(|err| PipelineError::new("FW_NODE_KV_PUBLISH_STATE_BUS", err.to_string()))?;

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: input.payload,
            trace: vec![format!(
                "kv.message.publish: channel={} receivers={}",
                channel, receivers
            )],
        })
    }
}
