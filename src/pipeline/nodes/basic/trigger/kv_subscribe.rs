//! `n.trigger.kv.subscribe` — listen for messages published on a named KV channel.
//!
//! This is a **passthrough trigger node** — it does not do any work at execution time.
//! The real subscription happens in [`crate::infra::mem::subscriber::MemSubscriber`],
//! which spawns a background task when the pipeline is activated.
//!
//! When a message is published to the channel via `n.kv.publish`, the background
//! task fires this pipeline with:
//!
//! ```json
//! {
//!   "trigger": "kv.subscribe",
//!   "channel": "<channel-name>",
//!   "node_id": "<node-id>",
//!   "message": { ... }
//! }
//! ```
//!
//! Downstream nodes access the published data via `input.message`.
//!
//! # Example
//!
//! ```text
//! | n.trigger.kv.subscribe --channel alerts
//! | n.script -- "return { alert: input.message };"
//! | n.ws.emit --event alert --room dashboard --to all
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

pub const NODE_KIND: &str = "n.trigger.kv.subscribe";
const OUTPUT_PIN_OUT: &str = "out";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Database],
        title: "KV Subscribe".to_string(),
        description: "Runs every time `kv.publish` sends a message on `--channel` — the project's in-process pub/sub, for work that \
            should happen after a request answered (send the mail, resize the image, recompute a total) without making the request \
            wait. The payload is `{ trigger: \"kv.subscribe\", channel, node_id, message }` — what was published is `input.message`, \
            not `input`. Messages are not stored: a subscriber that is not active when the publish happens never sees it."
            .to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "trigger": { "type": "string", "enum": ["kv.subscribe"] },
                "channel": { "type": "string" },
                "message": { "description": "Published message payload." }
            }
        }),
        input_pins: vec![],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: json!({
            "type": "object",
            "required": ["channel"],
            "properties": {
                "channel": { "type": "string", "description": "Channel to subscribe to." },
            }
        }),
        dsl_flags: vec![DslFlag {
            flag: "--channel".to_string(),
            config_key: "channel".to_string(),
            description: "Channel name to subscribe to.".to_string(),
            kind: DslFlagKind::Scalar,
            required: true,
        }],
        fields: vec![NodeFieldDef {
            name: "channel".to_string(),
            label: "Channel".to_string(),
            field_type: NodeFieldType::Text,
            help: Some("Channel name to subscribe to.".to_string()),
            ..Default::default()
        }],
        layout: vec![],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Do the slow part after the request", "trigger.kv.subscribe --channel order.placed")
                .output(serde_json::json!({ "trigger": "kv.subscribe", "channel": "order.placed", "node_id": "n0", "message": { "order_id": "o_91", "email": "a@x.io" } }))
                .note("The publisher: `| kv.publish --channel order.placed --payload \"{{ { order_id: input.rows[0]._key, email: input.body.email } }}\"`."),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub channel: String,
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
        // Passthrough — payload already injected by MemSubscriber background task.
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: input.payload,
            trace: vec![format!(
                "n.trigger.kv.subscribe: channel={}",
                self.config.channel
            )],
        })
    }
}
