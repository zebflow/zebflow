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
//! # Tick rooms
//!
//! `--event $tick --every <16ms..1s>` makes every room it matches a tick
//! room: client events wait for the next tick instead of running their own
//! triggers, and this trigger runs once per tick with every waiting input in
//! arrival order — at most `--max-inputs` (default 30) per session; the rest
//! are dropped and counted. `$connect` and `$disconnect` still run their own
//! triggers. Every `$tick` trigger that can match one room declares the same
//! `--every` and `--max-inputs`, or activation refuses.
//!
//! # The answer: `room: { … }`
//!
//! | Field | Type | Description |
//! |---|---|---|
//! | `room_id` | string | The room from the WS URL (`/ws/{owner}/{project}/rooms/{room}`) |
//! | `session_id` | string | The connected client session |
//! | `event` | string | The event name sent by the client, or one the server raises (`$connect`, `$disconnect`, `$away`, `$back`) |
//! | `payload` | object | The event body sent by the client |
//! | `auth` | object or null | What the join rule proved at connect (or `{ sub }` from a ticket) |
//!
//! A tick answers `room: { room_id, event: "$tick", tick, dt_ms, inputs: [ { session, event, payload, auth } ] }`.
//!
//! Without `--room`, the room nodes downstream (`ws.state.*`,
//! `ws.message.send`) act on `$trigger.room_id`, and `ws.message.send
//! --recipient session|others` targets `$trigger.session_id`.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::infra::transport::ws::TickSpec;
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
/// A flag of this trigger set wrong, refused at activation.
pub const CONFIG_CODE: &str = "FW_NODE_TRIGGER_ROOM_CONFIG";
/// The event a tick room's trigger names.
pub const TICK_EVENT: &str = "$tick";
/// `--every` bounds.
const EVERY_RANGE: (std::time::Duration, std::time::Duration) =
    (std::time::Duration::from_millis(16), std::time::Duration::from_secs(1));
/// `--max-inputs` when unset, and its bounds.
pub const DEFAULT_MAX_INPUTS: u32 = 30;
const MAX_INPUTS_RANGE: (u32, u32) = (1, 256);

/// The tick a `trigger.room` configuration declares: `Some` for
/// `--event $tick`, which needs `--every`; `--every` and `--max-inputs` on
/// any other event are refused.
pub fn tick_of(config: &serde_json::Value) -> Result<Option<TickSpec>, PipelineError> {
    let null = serde_json::Value::Null;
    let event = config.get("event").and_then(serde_json::Value::as_str).unwrap_or_default().trim();
    let every = super::duration_flag(config.get("every").unwrap_or(&null), "--every", CONFIG_CODE, EVERY_RANGE.0, EVERY_RANGE.1)?;
    let max_inputs = super::text_of(config.get("max_inputs").unwrap_or(&null), "--max-inputs", CONFIG_CODE)?;
    if event != TICK_EVENT {
        if every.is_some() || !max_inputs.is_empty() {
            return Err(PipelineError::new(CONFIG_CODE, "--every and --max-inputs belong to --event $tick, the trigger of a tick room"));
        }
        return Ok(None);
    }
    let every = every.ok_or_else(|| PipelineError::new(CONFIG_CODE, "--event $tick needs --every: how often the room ticks, 16ms to 1s"))?;
    let max_inputs = if max_inputs.is_empty() {
        DEFAULT_MAX_INPUTS
    } else {
        max_inputs
            .parse::<u32>()
            .ok()
            .filter(|n| (MAX_INPUTS_RANGE.0..=MAX_INPUTS_RANGE.1).contains(n))
            .ok_or_else(|| PipelineError::new(CONFIG_CODE, format!("--max-inputs {max_inputs} must be a whole number from 1 to 256")))?
    };
    Ok(Some(TickSpec { every_ms: every.as_millis() as u64, max_inputs }))
}

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
            `$connect` and `$disconnect` (payload `{ reason }`: left, dropped, closed, room_closed, expired, flood, timeout) — \
            and in a room opened with `--hold`, `$away` when a connection drops and `$back` when it resumes — on the \
            room's one queue: a room handles one thing at a time, in arrival order across all its sessions, so a pipeline that \
            reads and writes back needs no lock — only a trigger naming one (`--event $connect`) receives it, and clients cannot send `$` \
            events. `--event $tick --every 50ms` (16ms–1s) makes a tick room: every client event waits for the next tick, and \
            this trigger runs once per tick with `room: { tick, dt_ms, inputs: [ { session, event, payload, auth } ] }` in arrival \
            order — at most `--max-inputs` (default 30) per session, the rest dropped and counted; ticks never overlap (one that \
            outlasts `--every` delays the next and counts a tick_overrun), an empty tick still runs, and state changes go out \
            once, after the tick. The client is a plain `WebSocket` to `/ws/{owner}/{project}/rooms/{room}` (`?ticket=` for a private room) \
            naming the protocol `zebflow.rooms.v1`, receiving `joined`, `patch`, `event` and `resync` (help topic \
            `pipeline/rooms-protocol`)."
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
                },
                "every": {
                    "type": "string",
                    "description": "With --event $tick: how often the room ticks, 16ms to 1s."
                },
                "max_inputs": {
                    "description": "With --event $tick: per session, the most inputs that wait for one tick (default 30, 1-256)."
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
                description: "Event name to match. Omit to match any event from connected clients; $tick makes a tick room."
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
                value: "text".to_string(),
                ..Default::default()
            },
            super::flag("--every", "every", "With --event $tick: how often the room ticks, 16ms to 1s.", "duration", false),
            super::flag(
                "--max-inputs",
                "max_inputs",
                "With --event $tick: per session, the most inputs that wait for one tick (default 30); extras are dropped and counted.",
                "number",
                false,
            ),
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
                help: Some("WebSocket event name to listen for; $tick runs once per tick of a tick room.".to_string()),
                ..Default::default()
            },
            super::field("every", "Every", "With $tick: how often the room ticks, 16ms to 1s."),
            super::field("max_inputs", "Max inputs", "With $tick: per session, the most inputs per tick (default 30)."),
        ].into_iter().chain(auth_fields("The room's join rule, checked once at connect; a refused visitor is closed with 4401.")).collect(),
        layout: vec![
            LayoutItem::Field("room".to_string()),
            LayoutItem::Field("event".to_string()),
            LayoutItem::Row { row: vec![LayoutItem::Field("every".to_string()), LayoutItem::Field("max_inputs".to_string())] },
            LayoutItem::Field("auth".to_string()),
            LayoutItem::Row { row: vec![LayoutItem::Field("credential_id".to_string()), LayoutItem::Field("role".to_string())] },
        ],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Chat message in", r#"trigger.room --room lobby --event chat.send"#)
                .output(serde_json::json!({ "room": { "room_id": "lobby", "session_id": "s_8f2", "event": "chat.send", "payload": { "text": "hello" }, "auth": null } }))
                .note("Then `| ws.message.send --event chat.message --body \"{{ { from: input.room.session_id, text: input.room.payload.text } }}\"`."),
            crate::pipeline::model::NodeExample::dsl("A match ticking 20 times a second", r#"trigger.room --room "match-:id" --event $tick --every 50ms"#)
                .output(serde_json::json!({ "room": { "room_id": "match-1", "event": "$tick", "tick": 412, "dt_ms": 50, "inputs": [
                    { "session": "s_8f2", "event": "move", "payload": { "dx": 1 }, "auth": { "sub": "u-17" } }
                ] } }))
                .note("Every client event of the room waits for the tick; resolve them in one run, in order, and every client gets one result."),
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

    /// With `--event $tick`: how often the room ticks ([`tick_of`]).
    #[serde(default)]
    pub every: serde_json::Value,

    /// With `--event $tick`: per session, the most inputs per tick.
    #[serde(default)]
    pub max_inputs: serde_json::Value,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tick_needs_every_in_range_and_only_a_tick_takes_it() {
        let tick = tick_of(&json!({ "event": "$tick", "every": "50ms" })).expect("reads").expect("a tick");
        assert_eq!(tick, TickSpec { every_ms: 50, max_inputs: DEFAULT_MAX_INPUTS });
        let tick = tick_of(&json!({ "event": "$tick", "every": "1s", "max_inputs": 4 })).unwrap().unwrap();
        assert_eq!(tick, TickSpec { every_ms: 1000, max_inputs: 4 });
        assert_eq!(tick_of(&json!({ "event": "move" })).unwrap(), None);
        for refused in [
            json!({ "event": "$tick" }),
            json!({ "event": "$tick", "every": "10ms" }),
            json!({ "event": "$tick", "every": "2s" }),
            json!({ "event": "$tick", "every": "50ms", "max_inputs": 0 }),
            json!({ "event": "$tick", "every": "50ms", "max_inputs": "lots" }),
            json!({ "event": "move", "every": "50ms" }),
            json!({ "event": "move", "max_inputs": 3 }),
        ] {
            assert_eq!(tick_of(&refused).expect_err(&refused.to_string()).code, CONFIG_CODE, "{refused}");
        }
    }
}
