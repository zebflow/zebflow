//! `trigger.topic` — listen for messages published on a named KV topic.
//!
//! This is a **passthrough trigger node** — it does not do any work at execution time.
//! The real subscription happens in [`crate::infra::mem::subscriber::MemSubscriber`],
//! which spawns a background task when the pipeline is activated.
//!
//! When a message is published on the topic via `kv.message.publish`, the
//! background task fires this pipeline, and the trigger answers one key,
//! `topic`:
//!
//! ```json
//! {
//!   "topic": {
//!     "trigger": "kv.subscribe",
//!     "channel": "<topic-name>",
//!     "node_id": "<node-id>",
//!     "message": { ... }
//!   }
//! }
//! ```
//!
//! Downstream nodes read the published data at `input.topic.message` (or
//! `$trigger.message`).
//!
//! # Example
//!
//! ```text
//! | trigger.topic --topic alerts
//! | ws.message.send --room dashboard --event alert --body "{{ input.topic.message }}"
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::pipeline::model::NodeCapability;
use crate::pipeline::model::{DslFlag, DslFlagKind, NodeFieldDef, NodeFieldType};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "trigger.topic";
const OUTPUT_PIN_OUT: &str = "out";
/// The key this trigger answers under.
pub const ANSWER_KEY: &str = "topic";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Database],
        title: "KV Subscribe".to_string(),
        description: "Runs every time `kv.message.publish` sends a message on `--topic` (the publisher's channel) — the project's in-process pub/sub, for work that \
            should happen after a request answered (send the mail, resize the image, recompute a total) without making the request \
            wait. Answers one key, `topic`: `{ trigger: \"kv.subscribe\", channel, node_id, message }` — what was published is \
            `input.topic.message` (`$trigger.message`). Messages are not stored: a subscriber that is not active when the publish happens never sees it."
            .to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "topic": {
                    "type": "object",
                    "properties": {
                        "trigger": { "type": "string", "enum": ["kv.subscribe"] },
                        "channel": { "type": "string" },
                        "node_id": { "type": "string" },
                        "message": { "description": "Published message payload." }
                    }
                }
            }
        }),
        input_pins: vec![],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: json!({
            "type": "object",
            "required": ["topic"],
            "properties": {
                "topic": { "type": "string", "description": "Topic (the publisher's channel) to subscribe to." },
            }
        }),
        dsl_flags: vec![DslFlag {
            flag: "--topic".to_string(),
            config_key: "topic".to_string(),
            description: "The topic to listen on — the channel `kv.message.publish` sends on.".to_string(),
            kind: DslFlagKind::Scalar,
            required: true,
            value: "text".to_string(),
            ..Default::default()
        }],
        fields: vec![NodeFieldDef {
            name: "topic".to_string(),
            label: "Topic".to_string(),
            field_type: NodeFieldType::Text,
            help: Some("The topic to listen on — the channel kv.message.publish sends on.".to_string()),
            ..Default::default()
        }],
        layout: vec![],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Do the slow part after the request", "trigger.topic --topic order.placed")
                .output(serde_json::json!({ "topic": { "trigger": "kv.subscribe", "channel": "order.placed", "node_id": "n0", "message": { "order_id": "o_91", "email": "a@example.com" } } }))
                .note("The publisher: `| kv.message.publish --channel order.placed --payload \"{{ { order_id: input.query.rows[0]._key, email: $trigger.body.email } }}\"`."),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub topic: String,
}

pub struct Node {
    #[allow(dead_code)]
    config: Config,
}

impl Node {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

#[async_trait]
impl NodeHandler for Node {
    fn kind(&self) -> &'static str {
        NODE_KIND
    }
    fn input_pins(&self) -> &'static [&'static str] {
        &[]
    }
    fn output_pins(&self) -> &'static [&'static str] {
        &[OUTPUT_PIN_OUT]
    }

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        // The envelope was injected by the MemSubscriber background task.
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: super::answer_under(ANSWER_KEY, input.payload),
            trace: vec![format!("trigger.topic: topic={}", self.config.topic)],
        })
    }
}
