//! `n.mem.publish` — publish a message to a named in-memory channel.
//!
//! All active pipelines subscribed via `n.trigger.memsubscribe` on the same
//! channel will receive the message and fire.
//!
//! # Config flags
//!
//! | Flag | Type | Default | Description |
//! |---|---|---|---|
//! | `--channel` | string | required | Channel name |
//! | `--payload-path` | string | `""` | JSON pointer to extract from payload; empty = whole payload |
//!
//! # Example
//!
//! ```text
//! | n.trigger.webhook --path /alert --method POST
//! | n.mem.publish --channel notifications
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

pub const NODE_KIND: &str = "n.mem.publish";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Mem Publish".to_string(),
        description: "Publish a message to a named in-memory channel. \
            All pipelines listening via n.trigger.memsubscribe on the same channel receive the message. \
            Use --payload-path to send a sub-value; empty = whole payload."
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
                "payload_path": { "type": "string", "description": "JSON pointer to extract the message body. Empty = whole payload." },
            }
        }),
        dsl_flags: vec![
            DslFlag {
                flag: "--channel".to_string(),
                config_key: "channel".to_string(),
                description: "Channel name to publish to. Supports {{ expr }}.".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
            },
            DslFlag {
                flag: "--payload-path".to_string(),
                config_key: "payload_path".to_string(),
                description: "JSON pointer to extract message body. Empty = whole payload.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: vec![
            NodeFieldDef { name: "title".to_string(), label: "Title".to_string(), field_type: NodeFieldType::Text, help: Some("Override display title.".to_string()), ..Default::default() },
            NodeFieldDef { name: "channel".to_string(), label: "Channel".to_string(), field_type: NodeFieldType::Text, help: Some("Channel name. Supports {{ expr }}.".to_string()), ..Default::default() },
            NodeFieldDef { name: "payload_path".to_string(), label: "Payload Path".to_string(), field_type: NodeFieldType::Text, help: Some("JSON pointer to extract message body. Empty = whole payload.".to_string()), ..Default::default() },
        ],
        layout: vec![],
        ai_tool: Default::default(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub channel: String,
    #[serde(default)]
    pub payload_path: String,
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
                "MEM_PUBLISH_CHANNEL",
                "n.mem.publish: --channel is required",
            ));
        }

        let message = if self.config.payload_path.is_empty() {
            input.payload.clone()
        } else {
            let ptr = if self.config.payload_path.starts_with('/') {
                self.config.payload_path.clone()
            } else {
                format!("/{}", self.config.payload_path)
            };
            input.payload.pointer(&ptr).cloned().unwrap_or(Value::Null)
        };

        let receivers = self
            .state_bus
            .publish(owner, project, channel, message)
            .map_err(|err| PipelineError::new("MEM_PUBLISH_STATE_BUS", err.to_string()))?;

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: input.payload,
            trace: vec![format!(
                "n.mem.publish: channel={} receivers={}",
                channel, receivers
            )],
        })
    }
}
