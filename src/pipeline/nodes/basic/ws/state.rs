//! `ws.state.put` / `ws.state.update` / `ws.state.delete` — change the shared
//! state of a WebSocket room.
//!
//! ```text
//! ws.state.put     --key POINTER --value VALUE  [--room ROOM] [--batch]
//! ws.state.update  --key POINTER --value OBJECT [--room ROOM] [--batch]
//! ws.state.delete  --key POINTER                [--room ROOM] [--batch]
//! ```
//!
//! - `put` replaces the value at `--key`, creating the objects above it;
//!   `update` shallow-merges an object into it; `delete` removes it.
//! - `--key` is a JSON pointer (`/players/p1`). A part that comes from the
//!   payload is written with `{{ }}` — `/players/{{ input.room.session_id }}` — and
//!   a segment that resolves empty is refused: `/players/` would otherwise
//!   write into, or delete, the whole map.
//! - Without `--room` the room is the one of the `trigger.room` that started
//!   the run. A room nobody has joined holds no state, and is skipped.
//! - Every client in the room is sent the full state at once; `--batch`
//!   instead lets the room's 33 ms tick send it, at most once a tick — for
//!   streams of ten or more changes a second.
//!
//! The answer is `state: { key, room }`; the payload is kept.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::{INPUT_PIN_IN, OUTPUT_PIN_OUT, answer, field, flag, room_key, room_of, text_of};
use crate::infra::transport::ws::{RoomCmd, StateOp, WsHub};
use crate::pipeline::model::{DslFlag, DslFlagKind, LayoutItem, NodeExample, NodeFieldDef, NodeFieldType};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const PUT_KIND: &str = "ws.state.put";
pub const UPDATE_KIND: &str = "ws.state.update";
pub const DELETE_KIND: &str = "ws.state.delete";

/// `ws.state.update` was given a `--value` that is not an object.
pub const UPDATE_VALUE_CODE: &str = "FW_NODE_WS_STATE_UPDATE_VALUE";

/// The codes one verb raises.
#[derive(Debug, Clone, Copy)]
pub struct Codes {
    /// A flag that does not parse.
    pub config: &'static str,
    /// `--value` missing or resolved to nothing (`put`, `update`).
    pub empty: &'static str,
    /// No `--room` and no `trigger.room` upstream.
    pub room: &'static str,
    /// `--key` is not a pointer, or a segment of it is empty.
    pub key: &'static str,
    /// This engine has no room hub.
    pub unavailable: &'static str,
}

/// What happens at `--key`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verb {
    Put,
    Update,
    Delete,
}

impl Verb {
    pub fn kind(self) -> &'static str {
        match self {
            Verb::Put => PUT_KIND,
            Verb::Update => UPDATE_KIND,
            Verb::Delete => DELETE_KIND,
        }
    }

    pub fn of_kind(kind: &str) -> Option<Self> {
        [Verb::Put, Verb::Update, Verb::Delete].into_iter().find(|verb| verb.kind() == kind)
    }

    pub fn codes(self) -> Codes {
        match self {
            Verb::Put => Codes {
                config: "FW_NODE_WS_STATE_PUT_CONFIG",
                empty: "FW_NODE_WS_STATE_PUT_EMPTY",
                room: "FW_NODE_WS_STATE_PUT_ROOM",
                key: "FW_NODE_WS_STATE_PUT_KEY",
                unavailable: "FW_NODE_WS_STATE_PUT_UNAVAILABLE",
            },
            Verb::Update => Codes {
                config: "FW_NODE_WS_STATE_UPDATE_CONFIG",
                empty: "FW_NODE_WS_STATE_UPDATE_EMPTY",
                room: "FW_NODE_WS_STATE_UPDATE_ROOM",
                key: "FW_NODE_WS_STATE_UPDATE_KEY",
                unavailable: "FW_NODE_WS_STATE_UPDATE_UNAVAILABLE",
            },
            // A delete takes no value, so it has no `_EMPTY`.
            Verb::Delete => Codes {
                config: "FW_NODE_WS_STATE_DELETE_CONFIG",
                empty: "FW_NODE_WS_STATE_DELETE_CONFIG",
                room: "FW_NODE_WS_STATE_DELETE_ROOM",
                key: "FW_NODE_WS_STATE_DELETE_KEY",
                unavailable: "FW_NODE_WS_STATE_DELETE_UNAVAILABLE",
            },
        }
    }

    fn op(self) -> StateOp {
        match self {
            Verb::Put => StateOp::Set,
            Verb::Update => StateOp::Merge,
            Verb::Delete => StateOp::Delete,
        }
    }
}

pub fn definition(verb: Verb) -> NodeDefinition {
    let (title, does, value_help) = match verb {
        Verb::Put => ("WS State Put", "Replaces the value at `--key` with `--value`, creating the objects above it.", "What to write — a literal or {{ expr }}."),
        Verb::Update => ("WS State Update", "Shallow-merges the object `--value` into the object at `--key`, creating it if absent.", "An object to merge — {{ expr }}."),
        Verb::Delete => ("WS State Delete", "Removes the value at `--key`.", ""),
    };
    let mut dsl_flags = vec![flag(
        "--key",
        "key",
        "A JSON pointer into the room's state: /players/{{ input.room.session_id }}. A segment that resolves empty is refused.",
        "text",
        true,
    )];
    let mut fields = vec![field("key", "Key", "A JSON pointer: /players/{{ input.room.session_id }}.")];
    if verb != Verb::Delete {
        dsl_flags.push(flag("--value", "value", value_help, "json", true));
        fields.push(field("value", "Value", value_help));
    }
    dsl_flags.push(flag("--room", "room", "The room; default: the room of the trigger.room that started the run.", "text", false));
    dsl_flags.push(DslFlag {
        kind: DslFlagKind::Bool,
        ..flag("--batch", "batch", "Let the room's 33 ms tick send the state, at most once a tick — for 10+ changes a second.", "", false)
    });
    fields.push(field("room", "Room", "The room; empty: the room of the trigger.room that started the run."));
    fields.push(NodeFieldDef {
        field_type: NodeFieldType::Checkbox,
        ..field("batch", "Batch", "Send the state on the room's 33 ms tick instead of at once.")
    });
    let mut layout = vec![LayoutItem::Field("key".to_string())];
    if verb != Verb::Delete {
        layout.push(LayoutItem::Field("value".to_string()));
    }
    layout.push(LayoutItem::Row { row: vec![LayoutItem::Field("room".to_string()), LayoutItem::Field("batch".to_string())] });
    let examples = match verb {
        Verb::Put => vec![
            NodeExample::dsl("Keep the last move", r#"ws.state.put --key /last_move --value "{{ input.room.payload }}""#)
                .output(json!({ "state": { "key": "/last_move", "room": "lobby" } })),
        ],
        Verb::Update => vec![
            NodeExample::dsl(
                "Move a player",
                r#"ws.state.update --key "/players/{{ input.room.session_id }}" --value "{{ { x: input.room.payload.x, y: input.room.payload.y } }}" --batch"#,
            )
            .output(json!({ "state": { "key": "/players/s_8f2", "room": "lobby" } }))
            .note("Every client in the room sees the same `state.players`; `--batch` sends it at most 30 times a second."),
        ],
        Verb::Delete => vec![
            NodeExample::dsl("Remove a player who left", r#"ws.state.delete --key "/players/{{ input.room.session_id }}""#)
                .output(json!({ "state": { "key": "/players/s_8f2", "room": "lobby" } })),
        ],
    };
    NodeDefinition {
        kind: verb.kind().to_string(),
        title: title.to_string(),
        description: format!(
            "{does} The room's shared state is what every client of the room mirrors. `--key` is a JSON pointer; a part from \
             the payload is written with {{{{ }}}}, and a segment that resolves empty is refused. Without `--room` the room \
             is the one of the `trigger.room` that started the run. Clients get the full state at once, or with `--batch` \
             on the room's 33 ms tick. Adds `state: {{ key, room }}` and keeps the rest of the payload. Sending an event \
             without changing state is `ws.message.send`."
        ),
        input_schema: json!({ "type": "object", "description": "Any payload; it is kept and `state` is added." }),
        output_schema: json!({ "type": "object", "properties": { "state": { "type": "object", "properties": {
            "key": { "type": "string", "description": "The pointer, resolved" },
            "room": { "type": "string", "description": "The room" }
        } } } }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        dsl_flags,
        fields,
        layout,
        examples,
        ..Default::default()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// A JSON pointer into the room's state, resolved.
    #[serde(default)]
    pub key: Value,
    /// What to write (`put`) or merge (`update`).
    #[serde(default)]
    pub value: Value,
    /// The room; empty: the room of the `trigger.room` that started the run.
    #[serde(default)]
    pub room: Value,
    /// Send on the room's tick instead of at once.
    #[serde(default)]
    pub batch: bool,
}

pub struct Node {
    verb: Verb,
    key: String,
    value: Option<Value>,
    room: String,
    batch: bool,
    hub: Arc<WsHub>,
}

impl Node {
    pub fn new(verb: Verb, config: Config, hub: Option<Arc<WsHub>>) -> Result<Self, PipelineError> {
        let codes = verb.codes();
        let key = pointer(&text_of(&config.key, "--key", codes.config)?, codes.key)?;
        let room = text_of(&config.room, "--room", codes.config)?;
        let value = match verb {
            Verb::Delete => None,
            _ if config.value.is_null() => {
                return Err(PipelineError::new(codes.empty, "--value is empty; it needs a value"));
            }
            Verb::Update if !config.value.is_object() => {
                return Err(PipelineError::new(UPDATE_VALUE_CODE, "--value must be an object to merge; ws.state.put replaces"));
            }
            _ => Some(config.value),
        };
        let hub = hub.ok_or_else(|| PipelineError::new(codes.unavailable, "ws hub is not configured on this framework engine"))?;
        Ok(Self { verb, key, value, room, batch: config.batch, hub })
    }
}

/// `--key` as a JSON pointer whose every segment says something. The
/// `{name}` placeholders that once read the payload are gone; a brace left
/// in a segment is one, and is refused rather than written as a name.
fn pointer(key: &str, code: &'static str) -> Result<String, PipelineError> {
    if key.is_empty() {
        return Err(PipelineError::new(code, "--key is empty; it needs a JSON pointer such as /players/p1"));
    }
    let Some(rest) = key.strip_prefix('/') else {
        return Err(PipelineError::new(code, format!("--key {key} is not a JSON pointer: it starts with /")));
    };
    for (at, segment) in rest.split('/').enumerate() {
        if segment.is_empty() {
            return Err(PipelineError::new(code, format!("--key {key}: segment {} is empty", at + 1)));
        }
        if segment.contains('{') || segment.contains('}') {
            return Err(PipelineError::new(
                code,
                format!("--key {key}: a part from the payload is written with {{{{ }}}}, e.g. /players/{{{{ input.room.session_id }}}}"),
            ));
        }
    }
    Ok(key.to_string())
}

#[async_trait]
impl NodeHandler for Node {
    fn kind(&self) -> &'static str {
        self.verb.kind()
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
            return Err(PipelineError::new(
                self.verb.codes().room,
                "no room: give --room, or start the run from trigger.room",
            ));
        }
        // A room nobody has joined holds no state to change.
        if let Some(handle) = self.hub.get_room(&room_key(&input.metadata, &room)) {
            let (op, path, value) = (self.verb.op(), self.key.clone(), self.value.clone());
            handle.send_cmd(if self.batch {
                RoomCmd::PatchStateSilent { op, path, value }
            } else {
                RoomCmd::PatchState { op, path, value }
            });
        }
        Ok(answer(
            &input.payload,
            json!({ "state": { "key": self.key, "room": room } }),
            format!("node_kind={} key={} room={room} batch={}", self.verb.kind(), self.key, self.batch),
        ))
    }
}
