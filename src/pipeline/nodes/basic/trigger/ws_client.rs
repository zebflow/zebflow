//! `n.trigger.ws.client` — connect to an external WebSocket server and fire the pipeline on each message.
//!
//! This is a **passthrough trigger node** — it does not do any work at execution time.
//! The real connection happens in [`crate::infra::ws_client::WsClientManager`],
//! which spawns a background tokio task when the pipeline is activated.
//!
//! When a message arrives from the external WS server, the background task fires
//! this pipeline with:
//!
//! ```json
//! {
//!   "trigger": "ws_client",
//!   "node_id": "<node-id>",
//!   "url": "wss://...",
//!   "message": { ... }
//! }
//! ```
//!
//! Downstream nodes access the received data via `input.message`.
//!
//! # Example
//!
//! ```text
//! | n.trigger.ws.client --url wss://stream.example.com/feed
//! | n.script -- "return { event: input.message };"
//! | n.ws.emit --event feed --room dashboard --to all
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::pipeline::model::{DslFlag, DslFlagKind, NodeFieldDef, NodeFieldType};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "n.trigger.ws.client";
const OUTPUT_PIN_OUT: &str = "out";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "WS Client".to_string(),
        description: "Connect to an external WebSocket server on pipeline activate. \
            Fires the pipeline for every received message. Auto-reconnects with \
            exponential backoff on disconnect. \
            Output payload: { trigger: \"ws_client\", url, node_id, message }. \
            Access the received data via input.message in downstream nodes."
            .to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "trigger": { "type": "string", "enum": ["ws_client"] },
                "url": { "type": "string" },
                "node_id": { "type": "string" },
                "message": { "description": "Received WebSocket message payload." }
            }
        }),
        input_pins: vec![],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: json!({
            "type": "object",
            "required": ["url"],
            "properties": {
                "url": { "type": "string", "description": "WebSocket server URL (ws:// or wss://)." },
                "credential_id": { "type": "string", "description": "Credential for auth headers/tokens." },
                "reconnect": { "type": "boolean", "description": "Auto-reconnect on disconnect. Default: true." },
                "reconnect_delay_ms": { "type": "integer", "description": "Base reconnect delay in ms. Default: 5000." },
                "max_reconnect_attempts": { "type": "integer", "description": "Max reconnect attempts (0 = infinite). Default: 0." },
                "heartbeat_interval_ms": { "type": "integer", "description": "Ping interval in ms. Default: 30000." },
                "message_format": { "type": "string", "enum": ["json", "text"], "description": "Message format hint. Default: json." }
            }
        }),
        dsl_flags: vec![
            DslFlag {
                flag: "--url".to_string(),
                config_key: "url".to_string(),
                description: "WebSocket server URL (ws:// or wss://).".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
            },
            DslFlag {
                flag: "--credential".to_string(),
                config_key: "credential_id".to_string(),
                description: "Credential ID for auth.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--reconnect".to_string(),
                config_key: "reconnect".to_string(),
                description: "Auto-reconnect on disconnect. Default: true.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--reconnect-delay-ms".to_string(),
                config_key: "reconnect_delay_ms".to_string(),
                description: "Base reconnect delay in ms. Default: 5000.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--max-reconnect-attempts".to_string(),
                config_key: "max_reconnect_attempts".to_string(),
                description: "Max reconnect attempts (0 = infinite). Default: 0.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--heartbeat-interval-ms".to_string(),
                config_key: "heartbeat_interval_ms".to_string(),
                description: "Heartbeat ping interval in ms. Default: 30000.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--message-format".to_string(),
                config_key: "message_format".to_string(),
                description: "Message format: json or text. Default: json.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: vec![
            NodeFieldDef {
                name: "url".to_string(),
                label: "URL".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("WebSocket server URL (ws:// or wss://).".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "credential_id".to_string(),
                label: "Credential".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Credential ID for auth headers/tokens.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "message_format".to_string(),
                label: "Format".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Message format: json or text.".to_string()),
                ..Default::default()
            },
        ],
        layout: vec![],
        ai_tool: Default::default(),
        ..Default::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub credential_id: String,
    #[serde(default = "default_reconnect")]
    pub reconnect: bool,
    #[serde(default = "default_reconnect_delay_ms")]
    pub reconnect_delay_ms: u64,
    #[serde(default)]
    pub max_reconnect_attempts: u64,
    #[serde(default = "default_heartbeat_interval_ms")]
    pub heartbeat_interval_ms: u64,
    #[serde(default = "default_message_format")]
    pub message_format: String,
}

fn default_reconnect() -> bool {
    true
}
fn default_reconnect_delay_ms() -> u64 {
    5000
}
fn default_heartbeat_interval_ms() -> u64 {
    30000
}
fn default_message_format() -> String {
    "json".to_string()
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
        // Passthrough — payload already injected by WsClientManager background task.
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: input.payload,
            trace: vec![format!("n.trigger.ws.client: url={}", self.config.url)],
        })
    }
}
