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
//!     "topic": "<the --topic it listens on>",
//!     "message": { ... },
//!     "node_id": "<node-id>"
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
        title: "Topic Trigger".to_string(),
        description: "Runs every time `kv.message.publish` sends a message on `--topic` (the publisher's channel). It is the project's in-process pub/sub, for work that \
            should happen after a request answered (send the mail, resize the image, recompute a total) without making the request \
            wait. Answers one key, `topic`: `{ topic, message, node_id }` — `topic` the `--topic` it listens on, `message` what was \
            published (`input.topic.message`, `$trigger.message`). Messages are not stored: a subscriber that is not active when the publish happens never sees it."
            .to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "topic": {
                    "type": "object",
                    "properties": {
                        "topic": { "type": "string", "description": "The `--topic` this trigger listens on." },
                        "message": { "description": "Published message payload." },
                        "node_id": { "type": "string" }
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
                .output(serde_json::json!({ "topic": { "topic": "order.placed", "message": { "order_id": "o_91", "email": "a@example.com" }, "node_id": "n0" } }))
                .note("The publisher: `| kv.message.publish --topic order.placed --body \"{{ { order_id: input.query.rows[0]._key, email: $trigger.body.email } }}\"`."),
        ],
        ..Default::default()
    }
}

/// The envelope the subscriber starts a run with: the topic the message came
/// on (the word `--topic` uses), the message, and the trigger node.
pub fn envelope(topic: &str, node_id: &str, message: serde_json::Value) -> serde_json::Value {
    json!({ "topic": topic, "message": message, "node_id": node_id })
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::nodes::NodeExecutionInput;

    /// `topic: { topic, message, node_id }` — the word `--topic` uses, no
    /// trigger name or channel left over from the old vocabulary.
    #[tokio::test]
    async fn the_answer_is_topic_message_and_node_id() {
        let node = Node::new(Config { topic: "order.placed".into() });
        let out = node
            .execute_async(NodeExecutionInput {
                node_id: "t".into(),
                input_pin: String::new(),
                payload: envelope("order.placed", "t", json!({ "order_id": "o_1" })),
                metadata: json!({}),
                bus: None,
            })
            .await
            .expect("answer");
        assert_eq!(
            out.payload,
            json!({ "topic": { "topic": "order.placed", "message": { "order_id": "o_1" }, "node_id": "t" } })
        );
    }
}
