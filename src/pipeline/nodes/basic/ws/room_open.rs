//! `ws.room.open` — open a room under an active `trigger.room` pattern
//! (`docs/contracts/rooms.md` § Opening a room and letting people in).
//!
//! ```text
//! ws.room.open --room NAME [--private] [--max-sessions N] [--meta OBJECT]
//!              [--idle DURATION] [--hold DURATION] [--max-state SIZE]
//! ```
//!
//! Refused unless an active `trigger.room` pattern matches the name, for a
//! fixed room's name, and for a room already open. The room exists until
//! `ws.room.close`, or until it has sat empty for `--idle` (default 10m).
//! `--hold` is kept with the room for the seat-holding that comes later.
//! Answers `room` with the room as `ws.room.list` shows it.

use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{INPUT_PIN_IN, OUTPUT_PIN_OUT, answer, checkbox, duration_flag, exact_room, field, flag, project_of, switch, text_of};
use crate::infra::transport::ws::room::{DEFAULT_IDLE, DEFAULT_MAX_STATE};
use crate::infra::transport::ws::{HubRefusal, RoomSettings, WsHub};
use crate::pipeline::model::{LayoutItem, NodeExample, NodeFieldDef, NodeFieldType};
use crate::pipeline::nodes::shared::limits::{whole, within};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "ws.room.open";
/// A flag that does not read, or a name that is not a room name.
pub const CONFIG_CODE: &str = "FW_NODE_WS_ROOM_OPEN_CONFIG";
/// No active `trigger.room` pattern matches the name.
pub const PATTERN_CODE: &str = "FW_NODE_WS_ROOM_OPEN_PATTERN";
/// The name is a fixed room's (an exact `trigger.room --room`).
pub const FIXED_CODE: &str = "FW_NODE_WS_ROOM_OPEN_FIXED";
/// A room of that name is already open.
pub const TAKEN_CODE: &str = "FW_NODE_WS_ROOM_OPEN_TAKEN";
/// This engine has no room hub.
pub const UNAVAILABLE_CODE: &str = "FW_NODE_WS_ROOM_OPEN_UNAVAILABLE";

/// `--max-sessions` when none is given, and its ceiling.
pub const DEFAULT_MAX_SESSIONS: u32 = 1000;
pub const MAX_MAX_SESSIONS: u32 = 10_000;
/// `--meta` is a small object: at most this many bytes of JSON.
pub const MAX_META_BYTES: usize = 4096;
/// The ceiling of `--max-state`.
pub const MAX_MAX_STATE: u64 = 16_000_000;
/// `--idle` and `--hold` bounds.
const IDLE_RANGE: (Duration, Duration) = (Duration::from_secs(1), Duration::from_secs(24 * 3600));
const HOLD_RANGE: (Duration, Duration) = (Duration::from_secs(1), Duration::from_secs(3600));

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "WS Room Open".to_string(),
        description: "Opens the room `--room` (an exact name, e.g. `match-{{ Date.now() }}`) under an active `trigger.room` pattern \
            (`match-:id`) — no visitor can make a room by naming one. Refused for a name no active pattern matches, for a fixed \
            room's name and for a room already open. `--private` admits only holders of a `ws.ticket.create` ticket; \
            `--max-sessions` (default 1000, at most 10000) caps the room, a join past it closing with 4429; `--meta` is a small \
            JSON object `ws.room.list` shows (what matchmaking filters on); `--idle` (default 10m) closes the room after it has \
            sat empty that long; `--hold` keeps a dropped session's seat (stored now, honoured later); `--max-state` (default \
            1MB, at most 16MB) caps the room's state, a write past it refused. Adds `room` — the room as `ws.room.list` shows it \
            — and keeps the rest of the payload."
            .to_string(),
        input_schema: json!({ "type": "object", "description": "Any payload; it is kept and `room` is added." }),
        output_schema: json!({ "type": "object", "properties": { "room": { "type": "object", "properties": {
            "room": { "type": "string" }, "kind": { "type": "string" }, "private": { "type": "boolean" },
            "meta": { "type": "object" }, "sessions": { "type": "integer" }, "max_sessions": { "type": "integer" },
            "opened_at": { "type": "string" }, "opened_by": { "type": "string" }, "state_bytes": { "type": "integer" },
            "counters": { "type": "object" }
        } } } }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        dsl_flags: vec![
            flag("--room", "room", "The room's name: 1–128 of letters, digits, '-', '_', '.', matched by an active trigger.room pattern.", "text", true),
            switch("--private", "private", "Admit only holders of a ws.ticket.create ticket."),
            flag("--max-sessions", "max_sessions", "The most sessions at once (default 1000, at most 10000); a join past it closes with 4429.", "number", false),
            flag("--meta", "meta", "A small JSON object ws.room.list shows: mode, map, level band.", "json", false),
            flag("--idle", "idle", "Close the room after it has sat empty this long (default 10m, 1s–24h).", "duration", false),
            flag("--hold", "hold", "Keep a dropped session's seat this long (1s–1h); stored now, honoured in a later release.", "duration", false),
            flag("--max-state", "max_state", "The most bytes of room state (default 1MB, at most 16MB); a write past it is refused.", "size", false),
        ],
        fields: vec![
            field("room", "Room", "The room's name, matched by an active trigger.room pattern: match-{{ Date.now() }}."),
            checkbox("private", "Private", "Only ticket holders may join."),
            NodeFieldDef { field_type: NodeFieldType::Number, ..field("max_sessions", "Max sessions", "Default 1000.") },
            field("meta", "Meta", "A small JSON object ws.room.list shows."),
            field("idle", "Idle", "Close after sitting empty this long; default 10m."),
            field("hold", "Hold", "Keep a dropped seat this long."),
            field("max_state", "Max state", "State cap; default 1MB."),
        ],
        layout: vec![
            LayoutItem::Row { row: vec![LayoutItem::Field("room".to_string()), LayoutItem::Field("private".to_string())] },
            LayoutItem::Field("meta".to_string()),
            LayoutItem::Row { row: vec![LayoutItem::Field("max_sessions".to_string()), LayoutItem::Field("max_state".to_string())] },
            LayoutItem::Row { row: vec![LayoutItem::Field("idle".to_string()), LayoutItem::Field("hold".to_string())] },
        ],
        examples: vec![
            NodeExample::dsl(
                "Open a private match for ten",
                r#"ws.room.open --room "match-{{ Date.now() }}" --private --max-sessions 10 --meta "{{ { mode: 'ranked', map: 'valley' } }}""#,
            )
            .output(json!({ "room": {
                "room": "match-1760601600000", "kind": "opened", "private": true, "mode": "event", "meta": { "mode": "ranked", "map": "valley" },
                "sessions": 0, "max_sessions": 10, "opened_at": "2026-10-16T08:00:00Z", "opened_by": "pipelines/matches/open",
                "last_activity_at": "2026-10-16T08:00:00Z", "state_bytes": 2,
                "counters": { "messages_in": 0, "messages_out": 0, "ticks": 0, "tick_overruns": 0, "inputs_dropped": 0,
                              "pipeline_runs": 0, "pipeline_errors": 0, "resyncs": 0, "flood_closes": 0 }
            } }))
            .note("Needs an active `trigger.room --room \"match-:id\"`. Then `| ws.ticket.create --room \"{{ input.room.room }}\" --user \"{{ $trigger.auth.sub }}\"` lets a player in."),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub room: Value,
    #[serde(default)]
    pub private: bool,
    #[serde(default)]
    pub max_sessions: Value,
    #[serde(default)]
    pub meta: Value,
    #[serde(default)]
    pub idle: Value,
    #[serde(default)]
    pub hold: Value,
    #[serde(default)]
    pub max_state: Value,
}

pub struct Node {
    room: String,
    settings: RoomSettings,
    hub: Arc<WsHub>,
}

impl Node {
    pub fn new(config: Config, hub: Arc<WsHub>) -> Result<Self, PipelineError> {
        let room = exact_room(&text_of(&config.room, "--room", CONFIG_CODE)?, "--room", CONFIG_CODE)?;
        let max_sessions = match whole(&config.max_sessions, "--max-sessions", CONFIG_CODE)? {
            Some(n) => within(n, 1, MAX_MAX_SESSIONS, "--max-sessions", CONFIG_CODE)?,
            None => DEFAULT_MAX_SESSIONS,
        };
        let meta = match config.meta {
            Value::Null => json!({}),
            Value::String(ref s) if s.trim().is_empty() => json!({}),
            Value::Object(_) => config.meta,
            _ => return Err(PipelineError::new(CONFIG_CODE, "--meta must be a JSON object, e.g. {{ { mode: 'ranked' } }}")),
        };
        if meta.to_string().len() > MAX_META_BYTES {
            return Err(PipelineError::new(CONFIG_CODE, format!("--meta is over {MAX_META_BYTES} bytes of JSON; it is a small label, not data")));
        }
        let idle = duration_flag(&config.idle, "--idle", CONFIG_CODE, IDLE_RANGE.0, IDLE_RANGE.1)?.unwrap_or(DEFAULT_IDLE);
        let hold = duration_flag(&config.hold, "--hold", CONFIG_CODE, HOLD_RANGE.0, HOLD_RANGE.1)?;
        let max_state = match text_of(&config.max_state, "--max-state", CONFIG_CODE)? {
            text if text.is_empty() => DEFAULT_MAX_STATE,
            text => {
                let bytes = crate::pipeline::nodes::shared::units::size(&text, "--max-state", CONFIG_CODE)?;
                within(bytes, 2, MAX_MAX_STATE, "--max-state", CONFIG_CODE)?
            }
        };
        let settings = RoomSettings {
            private: config.private,
            max_sessions: Some(max_sessions),
            meta,
            idle,
            hold,
            max_state,
            opened_by: String::new(),
        };
        Ok(Self { room, settings, hub })
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
        let (owner, project) = project_of(&input.metadata);
        let opened_by = input.metadata.get("pipeline").and_then(Value::as_str).unwrap_or_default().to_string();
        let settings = RoomSettings { opened_by, ..self.settings.clone() };
        let room = self.hub.open(owner, project, &self.room, settings).map_err(|refusal| {
            let code = match refusal {
                HubRefusal::Undeclared(_) => PATTERN_CODE,
                HubRefusal::Fixed(_) => FIXED_CODE,
                HubRefusal::Taken(_) => TAKEN_CODE,
                _ => CONFIG_CODE,
            };
            PipelineError::new(code, refusal.to_string())
        })?;
        Ok(answer(
            &input.payload,
            json!({ "room": room.describe(false) }),
            format!("node_kind={NODE_KIND} room={} private={}", self.room, self.settings.private),
        ))
    }
}
