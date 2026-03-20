//! `n.ws.emit` — broadcast a named event to sessions in a WebSocket room.
//!
//! Unlike `n.ws.sync_state`, this node does **not** mutate shared state.
//! It sends a transient `event` message to one or more connected clients.
//! Clients receive the full JSON envelope and are responsible for filtering
//! by `to` / `target_session`.
//!
//! # Config flags
//!
//! | Flag | Type | Default | Description |
//! |---|---|---|---|
//! | `--event` | string | `"event"` | Application-level event name |
//! | `--to` | `"all"` \| `"session"` \| `"others"` | `"all"` | Recipient selection |
//! | `--payload-path` | string | `""` | JSON pointer into the payload to extract the emit body |
//! | `--room` | string | `""` | Static room id override for server-initiated pipelines |
//!
//! # Room resolution
//!
//! 1. If `--room` is set, use `{owner}/{project}/{room}` as the room key.
//! 2. Otherwise read `room_id` from `input.payload.room_id` (injected by `n.trigger.ws`).
//!
//! A missing room (no clients have ever joined) is silently skipped.
//!
//! # Recipient targeting (`--to`)
//!
//! | Value | Meaning |
//! |---|---|
//! | `all` | Broadcast to every connected session (default) |
//! | `session` | Deliver to the triggering session only |
//! | `others` | Deliver to all sessions *except* the triggering session |
//!
//! For `session` and `others`, `session_id` is read from `input.payload.session_id`
//! (injected by `n.trigger.ws`).  For server-initiated pipelines without a
//! WS trigger, `all` is the natural choice.
//!
//! # Wire format (server → client)
//!
//! ```json
//! {
//!   "type": "event",
//!   "event": "<event-name>",
//!   "payload": { ... },
//!   "to": "all" | "session" | "others",
//!   "target_session": "<session_id>" | null
//! }
//! ```
//!
//! # Example pipelines
//!
//! **Broadcast chat message to everyone:**
//! ```text
//! | n.trigger.ws --event chat
//! | n.ws.emit --event message --to all
//! ```
//!
//! **Echo move acknowledgment only to sender:**
//! ```text
//! | n.trigger.ws --event move
//! | n.ws.sync_state --op merge --path /players/{session_id} --silent
//! | n.ws.emit --event move_ack --to session
//! ```
//!
//! **AI agent broadcasting a narration from a scheduled job:**
//! ```text
//! | n.trigger.schedule --cron "0 * * * *"
//! | n.script -- "return { text: 'The hour strikes...' }"
//! | n.ws.emit --event narration --to all --room lobby
//! ```
//!
//! **Propagate a game event to all other players:**
//! ```text
//! | n.trigger.ws --event shoot
//! | n.ws.emit --event player_shot --to others
//! ```

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::pipeline::{
    PipelineError, NodeDefinition,
    nodes::{NodeHandler, NodeExecutionInput, NodeExecutionOutput},
};
use crate::pipeline::model::{DslFlag, DslFlagKind, LayoutItem};
use crate::infra::transport::ws::{EmitTarget, RoomCmd, WsHub};

pub const NODE_KIND: &str = "n.ws.emit";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

/// Return the [`NodeDefinition`] for `n.ws.emit`.
pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "WS Emit".to_string(),
        description: "Broadcasts a named event to WebSocket sessions in a room. \
            Does not modify shared state — use n.ws.sync_state for state changes. \
            Use --room to target rooms from server-initiated pipelines (scheduled jobs, \
            webhooks, AI agents) where no WS client is the trigger. \
            --to: all (everyone), session (sender only), others (all except sender)."
            .to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: json!({ "type": "object" }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: json!({
            "type": "object",
            "properties": {
                "event": {
                    "type": "string",
                    "description": "Application-level event name sent to clients. Examples: chat, player_shot, door_opened. Default: 'event'."
                },
                "to": {
                    "type": "string",
                    "enum": ["all", "session", "others"],
                    "description": "Recipient targeting. all = everyone, session = sender only, others = all except sender. Default: all."
                },
                "payload_path": {
                    "type": "string",
                    "description": "JSON pointer into the payload to extract the emit body. Empty = whole payload (or payload.payload if present)."
                },
                "room": {
                    "type": "string",
                    "description": "Static room id override. Required for server-initiated pipelines (AI agents, scheduled jobs, webhooks) without a WS trigger."
                }
            }
        }),
        dsl_flags: vec![
            DslFlag {
                flag: "--event".to_string(),
                config_key: "event".to_string(),
                description: "Event name clients receive. Examples: chat, player_shot. Default: event.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--to".to_string(),
                config_key: "to".to_string(),
                description: "Recipient: all (broadcast), session (sender only), others (all except sender). Default: all.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--payload-path".to_string(),
                config_key: "payload_path".to_string(),
                description: "JSON pointer into payload to extract the emit body. Empty = whole payload.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--room".to_string(),
                config_key: "room".to_string(),
                description: "Static room id for server-initiated pipelines without a WS trigger.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: {
            use crate::pipeline::model::{NodeFieldDef, NodeFieldType, SelectOptionDef};
            vec![
                NodeFieldDef { name: "title".to_string(), label: "Title".to_string(), field_type: NodeFieldType::Text, help: Some("Override display title for this node.".to_string()), ..Default::default() },
                NodeFieldDef { name: "event".to_string(), label: "Event".to_string(), field_type: NodeFieldType::Text, default_value: Some(serde_json::json!("message")), ..Default::default() },
                NodeFieldDef { name: "room".to_string(), label: "Room".to_string(), field_type: NodeFieldType::Text, ..Default::default() },
                NodeFieldDef { name: "to".to_string(), label: "To".to_string(), field_type: NodeFieldType::Select, options: vec![
                    SelectOptionDef { value: "room".to_string(), label: "Room — broadcast to all members".to_string() },
                    SelectOptionDef { value: "sender".to_string(), label: "Sender — reply to triggering socket only".to_string() },
                ], ..Default::default() },
                NodeFieldDef { name: "payload_path".to_string(), label: "Payload Path".to_string(), field_type: NodeFieldType::Text, ..Default::default() },
            ]
        },
        layout: vec![
            LayoutItem::Row { row: vec![LayoutItem::Field("title".to_string()), LayoutItem::Field("event".to_string())] },
            LayoutItem::Row { row: vec![LayoutItem::Field("room".to_string()), LayoutItem::Field("to".to_string())] },
            LayoutItem::Field("payload_path".to_string()),
        ],
        ai_tool: Default::default(),
    }
}

/// Configuration for `n.ws.emit`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Application-level event name sent to clients.
    ///
    /// Clients subscribe to specific event names on the client SDK.
    /// Examples: `"chat"`, `"player_shot"`, `"ai_narration"`, `"door_opened"`.
    /// Defaults to `"event"` if not set.
    #[serde(default)]
    pub event: String,

    /// Recipient selection.
    ///
    /// - `"all"` (default) — broadcast to all connected sessions.
    /// - `"session"` — send only to the session that triggered the pipeline.
    /// - `"others"` — send to all sessions except the triggering one.
    #[serde(default)]
    pub to: String,

    /// JSON pointer into the payload to extract the event body.
    ///
    /// Empty (default) — use the whole payload (or `payload.payload` if present).
    /// Example: `"/data"` — emit only `payload.data` to clients.
    #[serde(default)]
    pub payload_path: String,

    /// Static room id override.
    ///
    /// When set, the node targets `{owner}/{project}/{room}` regardless of
    /// `payload.room_id`.  Required for server-initiated pipelines (AI agents,
    /// scheduled jobs, webhooks) that have no WS trigger context.
    ///
    /// Example: `"lobby"`, `"places/hall"`.
    #[serde(default)]
    pub room: String,
}

/// `n.ws.emit` node instance.
pub struct Node {
    config: Config,
    ws_hub: Arc<WsHub>,
}

impl Node {
    pub fn new(config: Config, ws_hub: Arc<WsHub>) -> Result<Self, PipelineError> {
        Ok(Self { config, ws_hub })
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

        // Room resolution: static --room flag takes precedence over payload.room_id.
        let room_id = if !self.config.room.is_empty() {
            self.config.room.clone()
        } else {
            input
                .payload
                .get("room_id")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string()
        };

        if room_id.is_empty() {
            return Err(PipelineError::new(
                "FW_WS_EMIT_NO_ROOM",
                "n.ws.emit: room_id missing — set --room or ensure n.trigger.ws is upstream",
            ));
        }

        let session_id = input
            .payload
            .get("session_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();

        let room_key = format!("{}/{}/{}", owner, project, room_id);

        let target = match self.config.to.as_str() {
            "session" => EmitTarget::Session(session_id.clone()),
            "others" => EmitTarget::Others(session_id.clone()),
            _ => EmitTarget::All,
        };

        // Extract the emit body from the payload.
        let emit_payload = if self.config.payload_path.is_empty() {
            input
                .payload
                .get("payload")
                .cloned()
                .unwrap_or_else(|| input.payload.clone())
        } else {
            let ptr = if self.config.payload_path.starts_with('/') {
                self.config.payload_path.clone()
            } else {
                format!("/{}", self.config.payload_path)
            };
            input.payload.pointer(&ptr).cloned().unwrap_or(Value::Null)
        };

        let event = if self.config.event.is_empty() {
            "event".to_string()
        } else {
            self.config.event.clone()
        };

        if let Some(room) = self.ws_hub.get_room(&room_key) {
            room.send_cmd(RoomCmd::Emit {
                event: event.clone(),
                payload: emit_payload,
                to: target,
            });
        }

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: input.payload,
            trace: vec![format!(
                "n.ws.emit: event={} to={} room={}",
                event, self.config.to, room_id
            )],
        })
    }
}
