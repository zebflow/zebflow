//! `ws.session.close` — close one session, or every session of one user
//! (`docs/contracts/rooms.md` § Leaving, dropping and closing).
//!
//! ```text
//! ws.session.close [--room NAME-OR-PATTERN] (--session ID | --user SUB)
//! ```
//!
//! Each session is closed with `4409` and raises `$disconnect { reason:
//! "closed" }`. Without `--room` the room is the one of the `trigger.room`
//! that started the run. Kicking is a webhook the app guards with its own
//! auth, ending here — never an event a client sends. Answers
//! `session: { room, closed }`.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{INPUT_PIN_IN, OUTPUT_PIN_OUT, answer, field, flag, project_of, refused, room_of, selection, text_of};
use crate::infra::transport::ws::{EmitTarget, WsHub};
use crate::pipeline::model::{LayoutItem, NodeExample};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "ws.session.close";
/// A flag that does not read, or both or neither of `--session` and `--user`.
pub const CONFIG_CODE: &str = "FW_NODE_WS_SESSION_CLOSE_CONFIG";
/// No `--room` and no `trigger.room` upstream.
pub const ROOM_CODE: &str = "FW_NODE_WS_SESSION_CLOSE_ROOM";
/// This engine has no room hub.
pub const UNAVAILABLE_CODE: &str = "FW_NODE_WS_SESSION_CLOSE_UNAVAILABLE";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "WS Session Close".to_string(),
        description: "Closes the session `--session` (its id), or every session of the identity `--user` (its sub), in the \
            room `--room` — an exact name, or a pattern reaching every open room it matches; without `--room`, the room of the \
            `trigger.room` that started the run. Each is closed with 4409 and raises `$disconnect { reason: \"closed\" }`. Give \
            one of `--session` and `--user`. Kicking someone is a webhook the app guards with its own auth, ending here — never \
            an event a client sends. An exact room that is not open is `FW_NODE_WS_ROOM_NOT_OPEN`. Adds \
            `session: { room, closed }` (how many were closed) and keeps the rest of the payload."
            .to_string(),
        input_schema: json!({ "type": "object", "description": "Any payload; it is kept and `session` is added." }),
        output_schema: json!({ "type": "object", "properties": { "session": { "type": "object", "properties": {
            "room": { "type": "string" },
            "closed": { "type": "integer", "description": "How many sessions were closed" }
        } } } }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        dsl_flags: vec![
            flag("--room", "room", "The room — an exact name or a pattern; default: the room of the trigger.room that started the run.", "text", false),
            flag("--session", "session", "The session to close (its id, input.room.session_id where it joined).", "text", false),
            flag("--user", "user", "Close every session of this identity (its sub).", "text", false),
        ],
        fields: vec![
            field("room", "Room", "An exact room or a pattern; empty: the room of the trigger.room that started the run."),
            field("session", "Session", "One session's id."),
            field("user", "User", "Or every session of this sub."),
        ],
        layout: vec![
            LayoutItem::Field("room".to_string()),
            LayoutItem::Row { row: vec![LayoutItem::Field("session".to_string()), LayoutItem::Field("user".to_string())] },
        ],
        examples: vec![
            NodeExample::dsl("Ban a player from every zone", r#"ws.session.close --room "zone-*" --user "{{ $trigger.body.player }}""#)
                .output(json!({ "session": { "room": "zone-*", "closed": 2 } }))
                .note("After `trigger.webhook --route /api/ban --method POST --auth jwt --credential admin-key --role moderator`."),
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
    pub user: Value,
}

pub struct Node {
    room: String,
    which: EmitTarget,
    hub: Arc<WsHub>,
}

impl Node {
    pub fn new(config: Config, hub: Arc<WsHub>) -> Result<Self, PipelineError> {
        let room = text_of(&config.room, "--room", CONFIG_CODE)?;
        if !room.is_empty() {
            selection(&room, "--room", CONFIG_CODE)?;
        }
        let session = text_of(&config.session, "--session", CONFIG_CODE)?;
        let user = text_of(&config.user, "--user", CONFIG_CODE)?;
        let which = match (session.is_empty(), user.is_empty()) {
            (false, true) => EmitTarget::Session(session),
            (true, false) => EmitTarget::User(user),
            _ => return Err(PipelineError::new(CONFIG_CODE, "give one of --session and --user")),
        };
        Ok(Self { room, which, hub })
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
        let (owner, project) = project_of(&input.metadata);
        let (_, closed) = self.hub.close_sessions(owner, project, &room, &self.which).map_err(|refusal| refused(refusal, CONFIG_CODE))?;
        Ok(answer(
            &input.payload,
            json!({ "session": { "room": room, "closed": closed } }),
            format!("node_kind={NODE_KIND} room={room} closed={closed}"),
        ))
    }
}
