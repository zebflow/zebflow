//! `trigger.room` — trigger a pipeline when a WebSocket event arrives.
//!
//! This node is a **routing declaration**, not an active processor.  At
//! runtime the WS route handler scans all active pipelines for their
//! [`WsTriggerSpec`](crate::platform::services::WsTriggerSpec) and fires
//! matching ones when a client sends an event.  The node answers one key,
//! `room`, holding the event's envelope (`node-conventions.md` §6) — the same
//! value `$trigger` holds for the run.
//!
//! # Config flags
//!
//! | Flag | Type | Default | Description |
//! |---|---|---|---|
//! | `--room` | string | `""` | Room id pattern to match; empty = any room |
//! | `--event` | string | `""` | Event name pattern to match; empty = any event |
//! | `--auth` | `none\|jwt\|hmac\|api_key` | `none` | How the connection is guarded |
//! | `--credential` | string | | The credential that verifies `--auth` |
//! | `--role` | string, repeated | | A role allowed in |
//!
//! # The answer: `room: { … }`
//!
//! The WS route handler builds the envelope before execution:
//!
//! | Field | Type | Description |
//! |---|---|---|
//! | `room_id` | string | The room id from the WS URL (`/ws/{owner}/{project}/rooms/{room_id}`) |
//! | `session_id` | string | Unique identifier for the connected client session |
//! | `event` | string | The event name sent by the client |
//! | `payload` | object | The event body sent by the client |
//! | `auth` | object | Verified claims, when `--auth jwt` admitted the connection |
//!
//! Without `--room`, the room nodes downstream (`ws.state.*`,
//! `ws.message.send`) act on `$trigger.room_id`, and `ws.message.send
//! --recipient session|others` targets `$trigger.session_id`; everything else
//! reads these fields through `{{ input.room.… }}` or `{{ $trigger.… }}`.
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
//! | trigger.room
//! | ws.message.send --event echo --recipient session --body "{{ input.room.payload }}"
//! ```
//!
//! **Multiplayer 3D position update (batched at 30 fps):**
//! ```text
//! | trigger.room --event move
//! | ws.state.update --key "/players/{{ input.room.session_id }}" --value "{{ input.room.payload }}" --batch
//! ```
//!
//! **Chat message in a specific room:**
//! ```text
//! | trigger.room --room lobby --event chat
//! | ws.message.send --event message --body "{{ input.room.payload }}"
//! ```
//!
//! **Classroom action (any room, specific event):**
//! ```text
//! | trigger.room --event classroom_action
//! | javascript.script.run -- "/* validate role, build response */"
//! | ws.state.update --key /classroom --value "{{ $trigger.payload }}"
//! | ws.message.send --event classroom_updated --body "{{ $trigger.payload }}"
//! ```

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
        description: "Runs when a browser connected to this project's WebSocket sends an event — the server half of a chat, a live \
            board, a multiplayer scene. `--room` scopes it to one room (empty = any), `--event` to one event name (empty = any); the \
            same `--auth` / `--credential` / `--role` flags as `trigger.webhook` guard the connection. Answers one key, `room`: \
            `{ room_id, session_id, event, payload, auth? }` — what the client sent is `input.room.payload` (`$trigger.payload`). Answer with `ws.message.send` (to the room or one session) or \
            `ws.state.put` / `ws.state.update` / `ws.state.delete` (shared state every client mirrors); a `web.response.send` here answers nobody. The server raises \
            `$connect` and `$disconnect` (payload `{ reason }`) on each connection's ordered queue — only `--event $connect` / \
            `--event $disconnect` receive them, and clients cannot send `$` events. The client is a plain `WebSocket` to \
            `/ws/{owner}/{project}/rooms/{room}` receiving `joined`, `state_patch`, `event` and `resync` \
            (help topic `pipeline/examples/realtime-game`)."
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
                    "description": "Room id to match. Empty string (default) matches any room. Exact string equality — no wildcards."
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
                description: "Room id to scope this trigger to. Omit to match any room."
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
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
        ].into_iter().chain(auth_flags("the connection")).collect(),
        fields: [
            NodeFieldDef {
                name: "room".to_string(),
                label: "Room".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Room name pattern to listen on.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "event".to_string(),
                label: "Event".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("WebSocket event name to listen for.".to_string()),
                ..Default::default()
            },
        ].into_iter().chain(auth_fields("On failure the event is silently dropped.")).collect(),
        layout: vec![
            LayoutItem::Field("room".to_string()),
            LayoutItem::Field("event".to_string()),
            LayoutItem::Field("auth".to_string()),
            LayoutItem::Row { row: vec![LayoutItem::Field("credential_id".to_string()), LayoutItem::Field("role".to_string())] },
        ],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Chat message in", r#"trigger.room --room lobby --event chat.send"#)
                .output(serde_json::json!({ "room": { "room_id": "lobby", "session_id": "s_8f2", "event": "chat.send", "payload": { "text": "hello" } } }))
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
