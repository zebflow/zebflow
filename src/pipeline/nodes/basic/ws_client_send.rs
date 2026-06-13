//! `n.ws.client.send` — send a message through an active outbound WebSocket client connection.
//!
//! This node sends a message through a WS connection managed by `WsClientManager`.
//! The connection must be established by an `n.trigger.ws.client` node in an active pipeline.
//!
//! # Config flags
//!
//! | Flag | Type | Default | Description |
//! |---|---|---|---|
//! | `--connection` | string | (required) | Node ID of the `n.trigger.ws.client` trigger that owns the connection |
//! | `--message-path` | string | `""` | JSON pointer into payload to extract the message body |
//!
//! # Example
//!
//! ```text
//! | n.trigger.ws.client --url wss://stream.example.com/feed
//! | n.script -- "return { reply: 'pong' };"
//! | n.ws.client.send --connection trigger_node_id --message-path /reply
//! ```

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::infra::ws_client::WsClientManager;
use crate::pipeline::model::{DslFlag, DslFlagKind, NodeFieldDef, NodeFieldType};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "n.ws.client.send";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "WS Client Send".to_string(),
        description: "Send a message through an active outbound WebSocket client connection. \
            The connection must be owned by an n.trigger.ws.client node in an active pipeline. \
            Use --connection to specify which trigger's connection to send through. \
            Use --message-path to extract a specific field from the payload as the message body."
            .to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: json!({ "type": "object" }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: json!({
            "type": "object",
            "required": ["connection"],
            "properties": {
                "connection": { "type": "string", "description": "Connection reference: 'pipeline/path:node_id' for cross-pipeline, or just node_id for same pipeline." },
                "message_path": { "type": "string", "description": "JSON pointer into payload to extract the message. Empty = whole payload." }
            }
        }),
        dsl_flags: vec![
            DslFlag {
                flag: "--connection".to_string(),
                config_key: "connection".to_string(),
                description: "Node ID of the WS client trigger owning the connection.".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
            },
            DslFlag {
                flag: "--message-path".to_string(),
                config_key: "message_path".to_string(),
                description: "JSON pointer into payload to extract the message body.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: vec![
            NodeFieldDef {
                name: "connection".to_string(),
                label: "Connection".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Node ID of the WS client trigger.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "message_path".to_string(),
                label: "Message Path".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("JSON pointer into payload. Empty = whole payload.".to_string()),
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
    /// Node ID of the `n.trigger.ws.client` node whose connection to use.
    #[serde(default)]
    pub connection: String,
    /// JSON pointer into the payload to extract the message body.
    #[serde(default)]
    pub message_path: String,
}

pub struct Node {
    config: Config,
    ws_client_manager: Arc<WsClientManager>,
}

impl Node {
    pub fn new(
        config: Config,
        ws_client_manager: Arc<WsClientManager>,
    ) -> Result<Self, PipelineError> {
        Ok(Self {
            config,
            ws_client_manager,
        })
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

        // Extract the message from the payload.
        let message = if self.config.message_path.is_empty() {
            input.payload.clone()
        } else {
            let ptr = if self.config.message_path.starts_with('/') {
                self.config.message_path.clone()
            } else {
                format!("/{}", self.config.message_path)
            };
            input.payload.pointer(&ptr).cloned().unwrap_or(Value::Null)
        };

        let message_str = match &message {
            Value::String(s) => s.clone(),
            other => serde_json::to_string(other).unwrap_or_default(),
        };

        // Build the connection key matching WsClientManager's format.
        // If connection contains '/' it's a fully-qualified reference: "pipeline/path:node_id"
        // Otherwise it's just a node_id within the same pipeline (use input.payload context).
        let connection_key = if self.config.connection.contains('/') {
            format!("{}/{}/{}", owner, project, self.config.connection)
        } else {
            // Infer pipeline from the trigger context if available.
            let pipeline_file = input
                .payload
                .get("__zf_pipeline_file")
                .and_then(Value::as_str)
                .unwrap_or_default();
            if pipeline_file.is_empty() {
                // Fall back to scanning all senders for a match.
                format!("{}/{}/:{}", owner, project, self.config.connection)
            } else {
                format!(
                    "{}/{}/{}:{}",
                    owner, project, pipeline_file, self.config.connection
                )
            }
        };

        match self
            .ws_client_manager
            .send(&connection_key, message_str)
            .await
        {
            Ok(()) => Ok(NodeExecutionOutput {
                output_pins: vec![OUTPUT_PIN_OUT.to_string()],
                payload: input.payload,
                trace: vec![format!(
                    "n.ws.client.send: connection={} sent",
                    self.config.connection
                )],
            }),
            Err(err) => Err(PipelineError::new("FW_WS_CLIENT_SEND", err)),
        }
    }
}
