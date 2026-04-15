//! `n.trigger.memsubscribe` — subscribe a pipeline to an in-memory channel.
//!
//! This is a **passthrough trigger node** — it does not do any work at execution time.
//! The real subscription happens in [`crate::infra::mem::subscriber::MemSubscriber`],
//! which spawns a background task when the pipeline is activated.
//!
//! When a message is published to the channel via `n.mem.publish`, the background
//! task fires this pipeline with:
//!
//! ```json
//! {
//!   "trigger": "memsubscribe",
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
//! | n.trigger.memsubscribe --channel alerts
//! | n.script -- "return { alert: input.message };"
//! | n.ws.emit --event alert --room dashboard --to all
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::pipeline::model::{DslFlag, DslFlagKind, NodeFieldDef, NodeFieldType};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "n.trigger.memsubscribe";
const OUTPUT_PIN_OUT: &str = "out";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Mem Subscribe".to_string(),
        description: "Subscribe this pipeline to an in-memory pub/sub channel. \
            Fires whenever n.mem.publish sends a message on the same channel. \
            Output payload: { trigger: \"memsubscribe\", channel, node_id, message }. \
            Access the published data via input.message in downstream nodes."
            .to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "trigger": { "type": "string", "enum": ["memsubscribe"] },
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
        fields: vec![
            NodeFieldDef {
                name: "title".to_string(),
                label: "Title".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Override display title.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "channel".to_string(),
                label: "Channel".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Channel name to subscribe to.".to_string()),
                ..Default::default()
            },
        ],
        layout: vec![],
        ai_tool: Default::default(),
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
                "n.trigger.memsubscribe: channel={}",
                self.config.channel
            )],
        })
    }
}
