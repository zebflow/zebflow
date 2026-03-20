//! `n.trigger.ws` — trigger a pipeline when a WebSocket event arrives.
//!
//! This node is a **routing declaration**, not an active processor.  At
//! runtime the WS route handler scans all active pipelines for their
//! [`WsTriggerSpec`](crate::platform::services::WsTriggerSpec) and fires
//! matching ones when a client sends an event.  The node itself is a
//! passthrough — the WS context flows downstream unchanged.
//!
//! # Config flags
//!
//! | Flag | Type | Default | Description |
//! |---|---|---|---|
//! | `--room` | string | `""` | Room id pattern to match; empty = any room |
//! | `--event` | string | `""` | Event name pattern to match; empty = any event |
//!
//! # Injected payload fields
//!
//! The WS route handler populates the initial payload before execution:
//!
//! | Field | Type | Description |
//! |---|---|---|
//! | `room_id` | string | The room id from the WS URL (`/ws/{owner}/{project}/rooms/{room_id}`) |
//! | `session_id` | string | Unique identifier for the connected client session |
//! | `event` | string | The event name sent by the client |
//! | `payload` | object | The event body sent by the client |
//!
//! These fields are consumed by downstream nodes (`n.ws.sync_state`,
//! `n.ws.emit`) automatically — pipeline authors do not need to extract them
//! manually in most cases.
//!
//! # Matching rules
//!
//! - `--room ""` (default) matches **any** room id.
//! - `--room lobby` matches only clients connected to the `lobby` room.
//! - `--event ""` (default) matches **any** event sent by the client.
//! - `--event move` matches only `{ "event": "move" }` messages.
//!
//! Matching is exact string equality (no wildcards or regex).
//!
//! # Example pipelines
//!
//! **Match all events in any room:**
//! ```text
//! | n.trigger.ws
//! | n.ws.emit --event echo --to session
//! ```
//!
//! **Multiplayer 3D position update (batched at 30 fps):**
//! ```text
//! | n.trigger.ws --event move
//! | n.ws.sync_state --op merge --path /players/{session_id} --silent
//! ```
//!
//! **Chat message in a specific room:**
//! ```text
//! | n.trigger.ws --room lobby --event chat
//! | n.ws.emit --event message --to all
//! ```
//!
//! **Classroom action (any room, specific event):**
//! ```text
//! | n.trigger.ws --event classroom_action
//! | n.script -- "/* validate role, build response */"
//! | n.ws.sync_state --op merge --path /classroom
//! | n.ws.emit --event classroom_updated --to all
//! ```

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::pipeline::{
    PipelineError, NodeDefinition,
    nodes::{NodeHandler, NodeExecutionInput, NodeExecutionOutput},
};
use crate::pipeline::model::{DslFlag, DslFlagKind, LayoutItem, NodeFieldDef, NodeFieldType};

pub const NODE_KIND: &str = "n.trigger.ws";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

/// Return the [`NodeDefinition`] for `n.trigger.ws`.
pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "WebSocket Trigger".to_string(),
        description: "Triggers a pipeline when a WebSocket event arrives from a connected client. \
            Use --room to scope to a specific room (empty = any room). \
            Use --event to match a specific event name (empty = any event). \
            Downstream nodes receive room_id, session_id, event, and payload fields."
            .to_string(),
        input_schema: json!({
            "type": "object",
            "properties": {
                "room_id": {
                    "type": "string",
                    "description": "Room id from the WS URL (/ws/{owner}/{project}/rooms/{room_id})."
                },
                "session_id": {
                    "type": "string",
                    "description": "Unique identifier for the connected client session."
                },
                "event": {
                    "type": "string",
                    "description": "Event name sent by the client, e.g. \"move\", \"chat\"."
                },
                "payload": {
                    "type": "object",
                    "description": "Event body sent by the client."
                }
            }
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "room_id":    { "type": "string" },
                "session_id": { "type": "string" },
                "event":      { "type": "string" },
                "payload":    { "type": "object" }
            }
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: json!({
            "type": "object",
            "properties": {
                "room": {
                    "type": "string",
                    "description": "Room id to match. Empty string (default) matches any room. Exact string equality — no wildcards."
                },
                "event": {
                    "type": "string",
                    "description": "Event name to match. Empty string (default) matches any event sent by the client. Exact string equality."
                }
            }
        }),
        dsl_flags: vec![
            DslFlag {
                flag: "--room".to_string(),
                config_key: "room".to_string(),
                description: "Room id to scope this trigger to. Omit to match any room.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--event".to_string(),
                config_key: "event".to_string(),
                description: "Event name to match. Omit to match any event from connected clients.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: vec![
            NodeFieldDef { name: "title".to_string(), label: "Title".to_string(), field_type: NodeFieldType::Text, help: Some("Override display title for this node.".to_string()), ..Default::default() },
            NodeFieldDef { name: "room".to_string(), label: "Room".to_string(), field_type: NodeFieldType::Text, help: Some("Room name pattern to listen on.".to_string()), ..Default::default() },
            NodeFieldDef { name: "event".to_string(), label: "Event".to_string(), field_type: NodeFieldType::Text, help: Some("WebSocket event name to listen for.".to_string()), ..Default::default() },
        ],
        layout: vec![
            LayoutItem::Row { row: vec![LayoutItem::Field("title".to_string()), LayoutItem::Field("room".to_string())] },
            LayoutItem::Field("event".to_string()),
        ],
        ai_tool: Default::default(),
    }
}

/// Configuration for `n.trigger.ws`.
///
/// Both fields are used only for **route matching** at pipeline dispatch time;
/// they have no effect during node execution.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Room id pattern to match.  Empty string (default) matches any room.
    ///
    /// Matched against the `room_id` segment of the WS URL.
    #[serde(default)]
    pub room: String,

    /// Event name pattern to match.  Empty string (default) matches any event.
    ///
    /// Matched against the `"event"` field in the client's JSON message.
    #[serde(default)]
    pub event: String,
}

/// `n.trigger.ws` node instance.
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
        &[INPUT_PIN_IN]
    }
    fn output_pins(&self) -> &'static [&'static str] {
        &[OUTPUT_PIN_OUT]
    }

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        // Passthrough — the WS context (room_id, session_id, event, payload)
        // was injected into the payload by the WS route handler before
        // dispatch.  Downstream nodes consume those fields directly.
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: input.payload,
            trace: vec!["n.trigger.ws: passthrough".to_string()],
        })
    }
}
