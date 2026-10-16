//! `ws.session.tag` — give a session tags, or take them away
//! (`docs/contracts/rooms.md` § Views).
//!
//! ```text
//! ws.session.tag [--room ROOM] [--session ID] [--untag TAG]… [--tag TAG]…
//! ```
//!
//! A session's tags decide its view: a path written `--visible-to team:red`
//! is sent only to sessions holding `team:red`, and `ws.message.send
//! --recipient tag:team:red` reaches only them. Every `--untag` applies
//! before every `--tag`, so one call moves a session: `--untag "cell:*"
//! --tag cell:12:7` turns `[team:red, cell:11:7]` into `[cell:12:7,
//! team:red]`. An `--untag` ending in `*` removes every tag with that
//! prefix; `--untag "*"` removes them all. At least one of the two is given.
//!
//! Without `--room` / `--session` it is the room and the session of the
//! `trigger.room` that started the run — a `$connect` pipeline tags the
//! session that joined. The new view goes out with the room's next send: at
//! once, or in a tick room after the tick. Answers `session: { session_id,
//! tags }`, the tags sorted.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{INPUT_PIN_IN, OUTPUT_PIN_OUT, ROOM_NOT_OPEN_CODE, answer, field, flag, project_of, room_of, tags_of, text_of, trigger_field};
use crate::infra::transport::ws::{RoomPattern, WsHub};
use crate::pipeline::model::{DslFlag, DslFlagKind, LayoutItem, NodeExample};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "ws.session.tag";
/// A flag that does not read, a tag that is not one, or neither `--tag` nor `--untag`.
pub const CONFIG_CODE: &str = "FW_NODE_WS_SESSION_TAG_CONFIG";
/// No `--room` and no `trigger.room` upstream, or a pattern for `--room`.
pub const ROOM_CODE: &str = "FW_NODE_WS_SESSION_TAG_ROOM";
/// No `--session` and no triggering session, or one that holds no seat in the room.
pub const SESSION_CODE: &str = "FW_NODE_WS_SESSION_TAG_SESSION";
/// This engine has no room hub.
pub const UNAVAILABLE_CODE: &str = "FW_NODE_WS_SESSION_TAG_UNAVAILABLE";

/// The most tags one call adds or removes, each.
pub const MAX_TAGS: u32 = 64;

fn repeated(flag_: DslFlag) -> DslFlag {
    DslFlag { kind: DslFlagKind::RepeatedList, max_repeat: Some(MAX_TAGS), ..flag_ }
}

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "WS Session Tag".to_string(),
        description: "Gives a session of a room tags, or takes them away. A session's tags decide what it sees: a state path \
            written with `ws.state.put --visible-to team:red` is sent only to sessions holding `team:red` (fog of war, a private \
            hand, the grid cells near a player), and `ws.message.send --recipient tag:team:red` reaches only them. `--tag` adds \
            (repeatable); `--untag` removes (repeatable), and one ending in `*` removes every tag with that prefix (`--untag \
            \"cell:*\"`; `\"*\"` removes all). Every `--untag` applies before every `--tag`, so `--untag \"cell:*\" --tag cell:12:7` \
            moves a session to a new cell and keeps its other tags. Give at least one of the two. A tag is 1–128 characters, \
            no spaces or `*`. `--room` (an exact room) and `--session` default to the room and the session of the \
            `trigger.room` that started the run — tag on `$connect`. The changed view goes out with the room's next send (at once; \
            in a tick room, after the tick). A held session keeps its tags. An exact room that is not open is \
            `FW_NODE_WS_ROOM_NOT_OPEN`. Adds `session: { session_id, tags }` (all its tags now, sorted) and keeps the rest of the \
            payload."
            .to_string(),
        input_schema: json!({ "type": "object", "description": "Any payload; it is kept and `session` is added." }),
        output_schema: json!({ "type": "object", "properties": { "session": { "type": "object", "properties": {
            "session_id": { "type": "string" },
            "tags": { "type": "array", "items": { "type": "string" }, "description": "The session's tags now, sorted" }
        } } } }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        dsl_flags: vec![
            flag("--room", "room", "The exact room; default: the room of the trigger.room that started the run.", "text", false),
            flag("--session", "session", "The session (its id); default: the session of the trigger.room that started the run.", "text", false),
            repeated(flag("--tag", "tag", "A tag to add: team:red, seat:2, cell:12:7 (repeat for several).", "text", false)),
            repeated(flag(
                "--untag",
                "untag",
                "A tag to remove, or a prefix ending in * (cell:*; * for all), applied before every --tag (repeat for several).",
                "text",
                false,
            )),
        ],
        fields: vec![
            field("room", "Room", "The exact room; empty: the room of the trigger.room that started the run."),
            field("session", "Session", "The session's id; empty: the session of the trigger.room that started the run."),
            field("tag", "Tag", "A tag to add (team:red), or {{ [list] }}."),
            field("untag", "Untag", "A tag to remove, a prefix ending in * (cell:*), or {{ [list] }}; removed before Tag adds."),
        ],
        layout: vec![
            LayoutItem::Row { row: vec![LayoutItem::Field("room".to_string()), LayoutItem::Field("session".to_string())] },
            LayoutItem::Row { row: vec![LayoutItem::Field("untag".to_string()), LayoutItem::Field("tag".to_string())] },
        ],
        examples: vec![
            NodeExample::dsl("Put a joining player on a team", r#"ws.session.tag --tag "team:{{ input.room.auth.team }}""#)
                .output(json!({ "session": { "session_id": "ws-1f0c", "tags": ["team:red"] } }))
                .note("After `trigger.room --room \"match-:id\" --event $connect`; `ws.state.put --visible-to team:red` now reaches this session."),
            NodeExample::dsl(
                "Move a player to the cells around it",
                r#"ws.session.tag --session ws-1f0c --untag "cell:*" --tag cell:12:7 --tag cell:12:8"#,
            )
            .output(json!({ "session": { "session_id": "ws-1f0c", "tags": ["cell:12:7", "cell:12:8", "team:red"] } }))
            .note("Its old cells go, `team:red` stays: each entity written `--visible-to cell:…` now reaches it only near."),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub room: Value,
    #[serde(default)]
    pub session: Value,
    #[serde(default)]
    pub tag: Value,
    #[serde(default)]
    pub untag: Value,
}

pub struct Node {
    room: String,
    session: String,
    tag: Vec<String>,
    untag: Vec<String>,
    hub: Arc<WsHub>,
}

impl Node {
    pub fn new(config: Config, hub: Arc<WsHub>) -> Result<Self, PipelineError> {
        let room = text_of(&config.room, "--room", CONFIG_CODE)?;
        let session = text_of(&config.session, "--session", CONFIG_CODE)?;
        let tag = tags_of(&config.tag, "--tag", CONFIG_CODE, false)?;
        let untag = tags_of(&config.untag, "--untag", CONFIG_CODE, true)?;
        if tag.is_empty() && untag.is_empty() {
            return Err(PipelineError::new(CONFIG_CODE, "give --tag, --untag or both"));
        }
        if tag.len().max(untag.len()) > MAX_TAGS as usize {
            return Err(PipelineError::new(CONFIG_CODE, format!("one call adds or removes at most {MAX_TAGS} tags each")));
        }
        Ok(Self { room, session, tag, untag, hub })
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
        let room = room_of(&self.room, &input.metadata);
        if room.is_empty() {
            return Err(PipelineError::new(ROOM_CODE, "no room: give --room, or start the run from trigger.room"));
        }
        if !matches!(RoomPattern::parse(&room), Ok(RoomPattern::Exact(_))) {
            return Err(PipelineError::new(ROOM_CODE, format!("--room '{room}': a session is tagged in one exact room")));
        }
        let session = if self.session.is_empty() { trigger_field(&input.metadata, "session_id") } else { self.session.clone() };
        if session.is_empty() {
            return Err(PipelineError::new(SESSION_CODE, "no session: give --session, or start the run from trigger.room"));
        }
        let (owner, project) = project_of(&input.metadata);
        let handle = self
            .hub
            .room(owner, project, &room)
            .ok_or_else(|| PipelineError::new(ROOM_NOT_OPEN_CODE, format!("room '{room}' is not open")))?;
        let tags = handle
            .retag(&session, &self.untag, &self.tag)
            .ok_or_else(|| PipelineError::new(SESSION_CODE, format!("session '{session}' is not in room '{room}'")))?;
        Ok(answer(
            &input.payload,
            json!({ "session": { "session_id": session, "tags": tags } }),
            format!("node_kind={NODE_KIND} room={room} session={session} tags={}", tags.len()),
        ))
    }
}
