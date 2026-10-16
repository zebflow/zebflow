//! `trigger.room` — declare a room and run a pipeline on its events
//! (`docs/contracts/rooms.md`).
//!
//! This node is a **declaration**, not an active processor. An exact
//! `--room lobby` is a fixed room: it exists while this trigger is active. A
//! pattern (`match-:id`, `zone-*`) declares opened rooms, which exist only
//! from `ws.room.open` until `ws.room.close` or their `--idle`. `--room` is
//! required and `*` alone is not a room. The node answers one key, `room`,
//! holding the event's envelope (`node-conventions.md` §6) — the same value
//! `$trigger` holds for the run.
//!
//! # The join rule
//!
//! `--auth` / `--credential` / `--role` are the room's join rule, read as on
//! `trigger.webhook` and checked once, when the socket connects; every active
//! `trigger.room` that can match the same room declares the same rule, or
//! activation refuses. A refused visitor is closed with `4401`, a room that
//! is not open with `4404`, a full one with `4429`, an expired login with
//! `4408`.
//!
//! # The answer: `room: { … }`
//!
//! | Field | Type | Description |
//! |---|---|---|
//! | `room_id` | string | The room from the WS URL (`/ws/{owner}/{project}/rooms/{room}`) |
//! | `session_id` | string | The connected client session |
//! | `event` | string | The event name sent by the client, or `$connect` / `$disconnect` |
//! | `payload` | object | The event body sent by the client |
//! | `auth` | object or null | What the join rule proved at connect (or `{ sub }` from a ticket) |
//!
//! Without `--room`, the room nodes downstream (`ws.state.*`,
//! `ws.message.send`) act on `$trigger.room_id`, and `ws.message.send
//! --recipient session|others` targets `$trigger.session_id`.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::pipeline::model::{DslFlag, DslFlagKind, LayoutItem, NodeFieldDef, NodeFieldType};
use crate::pipeline::nodes::basic::trigger::webhook::{AUTH_MODES, auth_fields, auth_flags};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "trigger.room";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";
/// The key this trigger answers under.
pub const ANSWER_KEY: &str = "room";

/// Return the [`NodeDefinition`] for `trigger.room`.
pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "WebSocket Trigger".to_string(),
        description: "Declares a room and runs when a client joined to it sends an event — the server half of a chat, a live board, \
            a match. `--room lobby` (an exact name) is a fixed room, there while this trigger is active; `--room \"match-:id\"` or \
            `\"zone-*\"` (a pattern) declares opened rooms, which exist only after `ws.room.open` and until `ws.room.close` or their \
            `--idle`; joining one that is not open closes with 4404. `--room` is required and `*` alone is not a room. `--event` \
            picks one event name (empty = any client event). `--auth` / `--credential` / `--role` are the room's join rule, read \
            as on `trigger.webhook` and checked once, at connect (refused: 4401); every active trigger.room that can match the same \
            room must declare the same rule, or activation refuses. Answers one key, `room`: `{ room_id, session_id, event, payload, \
            auth }` — `auth` is what the rule proved at connect (or `{ sub }` from a ticket). Answer with `ws.message.send` or \
            `ws.state.put` / `ws.state.update` / `ws.state.delete`; a `web.response.send` here answers nobody. The server raises \
            `$connect` and `$disconnect` (payload `{ reason }`: left, dropped, closed, room_closed, expired, flood) on the \
            room's one queue: a room handles one thing at a time, in arrival order across all its sessions, so a pipeline that \
            reads and writes back needs no lock — only `--event $connect` / `--event $disconnect` receive them, and clients cannot send `$` \
            events. The client is a plain `WebSocket` to `/ws/{owner}/{project}/rooms/{room}` (`?ticket=` for a private room) \
            receiving `joined`, `state_patch`, `event` and `resync` (help topic `pipeline/examples/realtime-game`)."
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
                },
                "auth": {
                    "description": "What the join rule proved at connect, or { sub } from a ticket; null when nothing was proved."
                }
            }
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "room": {
                    "type": "object",
                    "properties": {
                        "room_id":    { "type": "string" },
                        "session_id": { "type": "string" },
                        "event":      { "type": "string" },
                        "payload":    { "type": "object" },
                        "auth":       {}
                    }
                }
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
                    "description": "A fixed room (lobby) or a pattern of opened rooms (match-:id, zone-*). Required; * alone is not a room."
                },
                "event": {
                    "type": "string",
                    "description": "Event name to match. Empty string (default) matches any event sent by the client. Exact string equality."
                },
                "auth": {
                    "type": "string",
                    "enum": AUTH_MODES,
                    "description": "Authentication mode. none = open (default). jwt/hmac/api_key require credential_id."
                },
                "credential_id": {
                    "type": "string",
                    "description": "Credential ID used for auth verification. Required when auth is not none."
                },
                "role": {
                    "type": "array",
                    "items": { "type": "string" },
                    "description": "Required roles for this trigger. JWT claim 'roles' must match one. Empty = any authenticated user."
                }
            }
        }),
        dsl_flags: [
            DslFlag {
                flag: "--room".to_string(),
                config_key: "room".to_string(),
                description: "A fixed room (lobby) or a pattern of opened rooms (match-:id, zone-*). Required; * alone is not a room."
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
                value: "text".to_string(),
                ..Default::default()
            },
            DslFlag {
                flag: "--event".to_string(),
                config_key: "event".to_string(),
                description: "Event name to match. Omit to match any event from connected clients."
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
                value: "text".to_string(),
                ..Default::default()
            },
        ].into_iter().chain(auth_flags("joining the room")).collect(),
        fields: [
            NodeFieldDef {
                name: "room".to_string(),
                label: "Room".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("A fixed room (lobby) or a pattern of opened rooms (match-:id, zone-*). Required.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "event".to_string(),
                label: "Event".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("WebSocket event name to listen for.".to_string()),
                ..Default::default()
            },
        ].into_iter().chain(auth_fields("The room's join rule, checked once at connect; a refused visitor is closed with 4401.")).collect(),
        layout: vec![
            LayoutItem::Field("room".to_string()),
            LayoutItem::Field("event".to_string()),
            LayoutItem::Field("auth".to_string()),
            LayoutItem::Row { row: vec![LayoutItem::Field("credential_id".to_string()), LayoutItem::Field("role".to_string())] },
        ],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Chat message in", r#"trigger.room --room lobby --event chat.send"#)
                .output(serde_json::json!({ "room": { "room_id": "lobby", "session_id": "s_8f2", "event": "chat.send", "payload": { "text": "hello" }, "auth": null } }))
                .note("Then `| ws.message.send --event chat.message --body \"{{ { from: input.room.session_id, text: input.room.payload.text } }}\"`."),
        ],
        ..Default::default()
    }
}

/// Configuration for `trigger.room`.
///
/// Both fields are used only for **route matching** at pipeline dispatch time;
/// they have no effect during node execution.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// A fixed room's name, or a pattern of opened rooms.
    #[serde(default)]
    pub room: String,

    /// Event name pattern to match.  Empty string (default) matches any event.
    ///
    /// Matched against the `"event"` field in the client's JSON message.
    #[serde(default)]
    pub event: String,

    /// `"none"` (default), `"jwt"`, `"hmac"`, `"api_key"`.
    #[serde(default)]
    pub auth: String,

    /// Credential ID to use for auth verification (required when `auth != "none"`).
    #[serde(default)]
    pub credential_id: String,

    /// Roles allowed in; the JWT `roles` claim must hold one of these.
    /// Empty = any authenticated user may access.
    #[serde(default)]
    pub role: Vec<String>,
}

/// `trigger.room` node instance.
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
        // The WS route handler built the envelope (room_id, session_id,
        // event, payload, auth) before dispatch.
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: crate::pipeline::nodes::basic::trigger::answer_under(ANSWER_KEY, input.payload),
            trace: vec!["trigger.room: passthrough".to_string()],
        })
    }
}
