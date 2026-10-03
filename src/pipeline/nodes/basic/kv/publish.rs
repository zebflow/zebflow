//! `kv.message.publish` — send `--body` on `--topic` to every pipeline whose
//! `trigger.topic --topic` listens there: `message: { topic, delivered }`.
//!
//! Messages are not stored: a listener that is not active when the publish
//! happens never sees it, and `delivered` counts the ones that were.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{INPUT_PIN_IN, OUTPUT_PIN_OUT, answer, bus_error, field, flag, scope, text_of};
use crate::infra::io::state::DynStateBus;
use crate::pipeline::model::{DslFlag, LayoutItem, NodeCapability, NodeExample};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "kv.message.publish";
/// The bus could not take the message: the world's side.
pub const CODE: &str = "FW_NODE_KV_MESSAGE_PUBLISH";
/// A flag set wrong.
pub const CONFIG_CODE: &str = "FW_NODE_KV_MESSAGE_PUBLISH_CONFIG";
/// `--topic` is empty.
pub const TOPIC_CODE: &str = "FW_NODE_KV_MESSAGE_PUBLISH_TOPIC";
/// `--body` is empty.
pub const BODY_CODE: &str = "FW_NODE_KV_MESSAGE_PUBLISH_BODY";
/// The engine has no state bus.
pub const UNAVAILABLE_CODE: &str = "FW_NODE_KV_MESSAGE_PUBLISH_UNAVAILABLE";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Database],
        title: "KV Publish".to_string(),
        description: "Send `--body` on `--topic` to every active pipeline that starts with `trigger.topic --topic <same>` — the \
            project's in-process pub/sub, for work that should happen after the request is answered. Adds \
            `message: { topic, delivered }` (how many listeners received it) and keeps the rest of the payload. Messages are not \
            stored; an empty `--body` is refused."
            .to_string(),
        input_schema: json!({ "type": "object", "description": "Any payload; it is kept and `message` is added." }),
        output_schema: json!({ "type": "object", "properties": { "message": { "type": "object", "properties": {
            "topic": { "type": "string" },
            "delivered": { "type": "integer", "description": "Listeners that received it." }
        } } } }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        dsl_flags: vec![
            DslFlag { required: true, ..flag("--topic", "topic", "The topic to send on — the one `trigger.topic --topic` listens to.", "text") },
            DslFlag { required: true, ..flag("--body", "body", "What to send — a literal or {{ expr }}; a whole {{ }} keeps its type.", "json") },
        ],
        fields: vec![
            field("topic", "Topic", "The topic trigger.topic listens to: a literal or {{ expr }}."),
            field("body", "Body", "What to send: a literal or {{ expr }}."),
        ],
        layout: vec![LayoutItem::Field("topic".to_string()), LayoutItem::Field("body".to_string())],
        examples: vec![
            NodeExample::dsl("Hand off slow work after answering", r#"kv.message.publish --topic order.placed --body "{{ { order_id: input.query.rows[0]._key, email: $trigger.body.email } }}""#)
                .output(json!({ "message": { "topic": "order.placed", "delivered": 1 } }))
                .note("A pipeline starting with `trigger.topic --topic order.placed` receives the body as `input.topic.message`."),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// The topic.
    #[serde(default)]
    pub topic: Value,
    /// What to send, arriving final.
    #[serde(default)]
    pub body: Value,
}

pub struct Node {
    topic: String,
    body: Value,
    bus: DynStateBus,
}

impl Node {
    pub fn new(config: Config, bus: DynStateBus) -> Result<Self, PipelineError> {
        let topic = text_of(&config.topic, "--topic", CONFIG_CODE)?;
        if topic.is_empty() {
            return Err(PipelineError::new(TOPIC_CODE, "--topic is empty; it needs a value"));
        }
        if config.body.is_null() || config.body.as_str().is_some_and(|s| s.trim().is_empty()) {
            return Err(PipelineError::new(BODY_CODE, "--body is empty; it needs a value"));
        }
        Ok(Self { topic, body: config.body, bus })
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
        let delivered = self.bus.publish(&owner, &project, &self.topic, self.body.clone()).map_err(bus_error(CODE))?;
        Ok(answer(
            &input.payload,
            json!({ "message": { "topic": self.topic, "delivered": delivered } }),
            format!("{NODE_KIND}: topic={} delivered={delivered}", self.topic),
        ))
    }
}
