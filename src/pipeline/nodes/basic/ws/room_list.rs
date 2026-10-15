//! `ws.room.list` — this project's open rooms (`docs/contracts/rooms.md`
//! § Listing).
//!
//! ```text
//! ws.room.list [--room PATTERN] [--members]
//! ```
//!
//! `--room` is a pattern or an exact name (default `*`, every room). Answers
//! `room: { rooms: [ … ], total: { rooms, sessions } }`, each room with its
//! settings, sessions and counters that only rise; `--members` adds
//! `members: [sub, …]` — without it no identity is listed.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{INPUT_PIN_IN, OUTPUT_PIN_OUT, answer, checkbox, field, flag, project_of, refused, selection, switch, text_of};
use crate::infra::transport::ws::WsHub;
use crate::pipeline::model::{LayoutItem, NodeExample};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "ws.room.list";
/// `--room` does not read.
pub const CONFIG_CODE: &str = "FW_NODE_WS_ROOM_LIST_CONFIG";
/// This engine has no room hub.
pub const UNAVAILABLE_CODE: &str = "FW_NODE_WS_ROOM_LIST_UNAVAILABLE";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "WS Room List".to_string(),
        description: "Lists this project's open rooms that `--room` matches (a pattern or an exact name; default `*`, every \
            room). Adds `room: { rooms: [ { room, kind, private, mode, meta, sessions, max_sessions, opened_at, opened_by, \
            last_activity_at, state_bytes, counters } ], total: { rooms, sessions } }` and keeps the rest of the payload. \
            Counters only rise from the moment the room opened (`messages_in`, `messages_out`, `pipeline_runs`, \
            `pipeline_errors`, `resyncs`, `flood_closes`; `ticks`, `tick_overruns` and `inputs_dropped` stay 0 until tick rooms \
            come); a rate is the difference between two readings. `--members` adds each room's `members: [sub, …]`; without it \
            no identity is listed. Matchmaking reads `meta` and the free seats here, then `ws.ticket.create` or `ws.room.open`."
            .to_string(),
        input_schema: json!({ "type": "object", "description": "Any payload; it is kept and `room` is added." }),
        output_schema: json!({ "type": "object", "properties": { "room": { "type": "object", "properties": {
            "rooms": { "type": "array", "items": { "type": "object" } },
            "total": { "type": "object", "properties": { "rooms": { "type": "integer" }, "sessions": { "type": "integer" } } }
        } } } }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        dsl_flags: vec![
            flag("--room", "room", "Which rooms: a pattern or an exact name (default *, every room).", "text", false),
            switch("--members", "members", "Also list each room's members (their sub)."),
        ],
        fields: vec![
            field("room", "Room", "A pattern or an exact name; empty: every room."),
            checkbox("members", "Members", "List each room's members."),
        ],
        layout: vec![LayoutItem::Row { row: vec![LayoutItem::Field("room".to_string()), LayoutItem::Field("members".to_string())] }],
        examples: vec![
            NodeExample::dsl("Find a match with a free seat", r#"ws.room.list --room "match-*""#)
                .output(json!({ "room": {
                    "rooms": [ {
                        "room": "match-7", "kind": "opened", "private": true, "mode": "event", "meta": { "mode": "ranked" },
                        "sessions": 3, "max_sessions": 10, "opened_at": "2026-10-16T08:00:00Z", "opened_by": "pipelines/matches/open",
                        "last_activity_at": "2026-10-16T08:04:10Z", "state_bytes": 512,
                        "counters": { "messages_in": 140, "messages_out": 410, "ticks": 0, "tick_overruns": 0, "inputs_dropped": 0,
                                      "pipeline_runs": 140, "pipeline_errors": 0, "resyncs": 0, "flood_closes": 0 }
                    } ],
                    "total": { "rooms": 1, "sessions": 3 }
                } }))
                .note("Then pick a room whose `sessions` is under `max_sessions` and `| ws.ticket.create --room …`."),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub room: Value,
    #[serde(default)]
    pub members: bool,
}

pub struct Node {
    room: String,
    members: bool,
    hub: Arc<WsHub>,
}

impl Node {
    pub fn new(config: Config, hub: Arc<WsHub>) -> Result<Self, PipelineError> {
        let room = match text_of(&config.room, "--room", CONFIG_CODE)? {
            text if text.is_empty() => "*".to_string(),
            text => selection(&text, "--room", CONFIG_CODE)?,
        };
        Ok(Self { room, members: config.members, hub })
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
        let list = self.hub.list(owner, project, &self.room, self.members).map_err(|refusal| refused(refusal, CONFIG_CODE))?;
        let total = list["total"]["rooms"].clone();
        Ok(answer(&input.payload, json!({ "room": list }), format!("node_kind={NODE_KIND} room={} rooms={total}", self.room)))
    }
}
