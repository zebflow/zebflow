//! `ws.room.close` — close rooms (`docs/contracts/rooms.md` § Leaving,
//! dropping and closing).
//!
//! ```text
//! ws.room.close [--room NAME-OR-PATTERN]
//! ```
//!
//! Every session of each room is closed with `4410` and raises
//! `$disconnect { reason: "room_closed" }`; the state is dropped. A fixed
//! room is back at once, empty, while its trigger is active. Without
//! `--room` the room is the one of the `trigger.room` that started the run.
//! An exact room that is not open is `FW_NODE_WS_ROOM_NOT_OPEN`; a pattern
//! reaching none is not an error. Answers `room: { room, closed, sessions }`.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{INPUT_PIN_IN, OUTPUT_PIN_OUT, answer, field, flag, project_of, refused, room_of, selection, text_of};
use crate::infra::transport::ws::WsHub;
use crate::pipeline::model::{LayoutItem, NodeExample};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "ws.room.close";
/// `--room` does not read.
pub const CONFIG_CODE: &str = "FW_NODE_WS_ROOM_CLOSE_CONFIG";
/// No `--room` and no `trigger.room` upstream.
pub const ROOM_CODE: &str = "FW_NODE_WS_ROOM_CLOSE_ROOM";
/// This engine has no room hub.
pub const UNAVAILABLE_CODE: &str = "FW_NODE_WS_ROOM_CLOSE_UNAVAILABLE";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "WS Room Close".to_string(),
        description: "Closes the room `--room` — an exact name, or a pattern closing every open room it matches. Each session \
            is closed with 4410 and raises `$disconnect { reason: \"room_closed\" }`; the room's state is dropped. A fixed room \
            is back at once, empty. Without `--room` the room is the one of the `trigger.room` that started the run. An exact \
            room that is not open is `FW_NODE_WS_ROOM_NOT_OPEN`. Adds `room: { room, closed, sessions }` — the rooms closed and \
            how many sessions they held — and keeps the rest of the payload."
            .to_string(),
        input_schema: json!({ "type": "object", "description": "Any payload; it is kept and `room` is added." }),
        output_schema: json!({ "type": "object", "properties": { "room": { "type": "object", "properties": {
            "room": { "type": "string", "description": "The --room given" },
            "closed": { "type": "array", "items": { "type": "string" }, "description": "The rooms closed" },
            "sessions": { "type": "integer", "description": "How many sessions they held" }
        } } } }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        dsl_flags: vec![flag(
            "--room",
            "room",
            "The room to close — an exact name, or a pattern closing every open room it matches; default: the room of the trigger.room that started the run.",
            "text",
            false,
        )],
        fields: vec![field("room", "Room", "An exact room or a pattern; empty: the room of the trigger.room that started the run.")],
        layout: vec![LayoutItem::Field("room".to_string())],
        examples: vec![
            NodeExample::dsl("End a match", r#"ws.room.close --room "match-{{ $trigger.params.id }}""#)
                .output(json!({ "room": { "room": "match-7", "closed": ["match-7"], "sessions": 4 } })),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub room: Value,
}

pub struct Node {
    room: String,
    hub: Arc<WsHub>,
}

impl Node {
    pub fn new(config: Config, hub: Arc<WsHub>) -> Result<Self, PipelineError> {
        let room = text_of(&config.room, "--room", CONFIG_CODE)?;
        if !room.is_empty() {
            selection(&room, "--room", CONFIG_CODE)?;
        }
        Ok(Self { room, hub })
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
        let closed = self.hub.close(owner, project, &room).map_err(|refusal| refused(refusal, CONFIG_CODE))?;
        let sessions: usize = closed.iter().map(|(_, n)| n).sum();
        let names: Vec<&str> = closed.iter().map(|(name, _)| name.as_str()).collect();
        Ok(answer(
            &input.payload,
            json!({ "room": { "room": room, "closed": names, "sessions": sessions } }),
            format!("node_kind={NODE_KIND} room={room} closed={} sessions={sessions}", names.len()),
        ))
    }
}
