//! The `ws` family: WebSocket rooms and outgoing client sockets.
//!
//! | Kind | File | Answer |
//! |---|---|---|
//! | `ws.message.send` | `message_send.rs` | `message: { sent, room, event }` or `message: { sent, connection }` |
//! | `ws.state.put` | `state.rs` | `state: { key, room }` |
//! | `ws.state.update` | `state.rs` | `state: { key, room }` |
//! | `ws.state.delete` | `state.rs` | `state: { key, room }` |
//! | `ws.room.open` | `room_open.rs` | `room: { room, kind, private, meta, sessions, … }` |
//! | `ws.room.close` | `room_close.rs` | `room: { room, closed, sessions }` |
//! | `ws.room.list` | `room_list.rs` | `room: { rooms: [ … ], total }` |
//! | `ws.ticket.create` | `ticket_create.rs` | `ticket: { room, ticket, expires_at }` |
//! | `ws.session.close` | `session_close.rs` | `session: { room, closed }` |
//! | `ws.session.tag` | `session_tag.rs` | `session: { session_id, tags }` |
//!
//! The trigger that starts a pipeline from a room event (`trigger.room`,
//! `trigger.rs`) lives with the family it serves.
//!
//! A room node without `--room` uses the room of the `trigger.room` that
//! started the run (`$trigger.room_id`, the envelope the trigger answers
//! under `room`); a run started any other way names its room.

pub mod message_send;
pub mod room_close;
pub mod room_list;
pub mod room_open;
pub mod session_close;
pub mod session_tag;
pub mod state;
pub mod ticket_create;
pub mod trigger;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod manage_tests;
#[cfg(test)]
mod view_tests;

use std::sync::Arc;
use std::time::Duration;

use serde_json::Value;

use crate::infra::transport::ws::{HubRefusal, RoomPattern, WsHub};
use crate::pipeline::nodes::NodeHandler;
use crate::pipeline::PipelineError;
use crate::pipeline::model::{DslFlag, DslFlagKind, NodeFieldDef, NodeFieldType};
use crate::pipeline::nodes::NodeExecutionOutput;
use crate::pipeline::nodes::shared::util::with_answer;
use crate::pipeline::NodeDefinition;

const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

pub fn definitions() -> Vec<NodeDefinition> {
    vec![
        trigger::definition(),
        message_send::definition(),
        state::definition(state::Verb::Put),
        state::definition(state::Verb::Update),
        state::definition(state::Verb::Delete),
        room_open::definition(),
        room_close::definition(),
        room_list::definition(),
        ticket_create::definition(),
        session_close::definition(),
        session_tag::definition(),
    ]
}

/// The room-management kinds, one builder for them (as `kv/mod.rs`); `None`
/// for any other kind.
pub fn build(kind: &str, config: &Value, hub: Option<Arc<WsHub>>) -> Result<Option<Box<dyn NodeHandler>>, PipelineError> {
    let hub = |code: &'static str| hub.clone().ok_or_else(|| PipelineError::new(code, "ws hub is not configured on this framework engine"));
    let node: Box<dyn NodeHandler> = match kind {
        room_open::NODE_KIND => Box::new(room_open::Node::new(parse(config, room_open::CONFIG_CODE)?, hub(room_open::UNAVAILABLE_CODE)?)?),
        room_close::NODE_KIND => Box::new(room_close::Node::new(parse(config, room_close::CONFIG_CODE)?, hub(room_close::UNAVAILABLE_CODE)?)?),
        room_list::NODE_KIND => Box::new(room_list::Node::new(parse(config, room_list::CONFIG_CODE)?, hub(room_list::UNAVAILABLE_CODE)?)?),
        ticket_create::NODE_KIND => {
            Box::new(ticket_create::Node::new(parse(config, ticket_create::CONFIG_CODE)?, hub(ticket_create::UNAVAILABLE_CODE)?)?)
        }
        session_close::NODE_KIND => {
            Box::new(session_close::Node::new(parse(config, session_close::CONFIG_CODE)?, hub(session_close::UNAVAILABLE_CODE)?)?)
        }
        session_tag::NODE_KIND => {
            Box::new(session_tag::Node::new(parse(config, session_tag::CONFIG_CODE)?, hub(session_tag::UNAVAILABLE_CODE)?)?)
        }
        _ => return Ok(None),
    };
    Ok(Some(node))
}

fn parse<T: serde::de::DeserializeOwned>(config: &Value, code: &'static str) -> Result<T, PipelineError> {
    serde_json::from_value(config.clone()).map_err(|err| PipelineError::new(code, err.to_string()))
}

// ── Shared by the kinds ──────────────────────────────────────────────────────

/// A flag with its 0.11 metadata.
fn flag(name: &str, key: &str, description: &str, value: &str, required: bool) -> DslFlag {
    DslFlag {
        flag: name.to_string(),
        config_key: key.to_string(),
        description: description.to_string(),
        kind: DslFlagKind::Scalar,
        required,
        value: value.to_string(),
        ..Default::default()
    }
}

/// A switch (`--private`, `--members`).
fn switch(name: &str, key: &str, description: &str) -> DslFlag {
    DslFlag { kind: DslFlagKind::Bool, ..flag(name, key, description, "", false) }
}

/// A checkbox for a switch.
fn checkbox(name: &str, label: &str, help: &str) -> NodeFieldDef {
    NodeFieldDef { field_type: NodeFieldType::Checkbox, ..field(name, label, help) }
}

fn field(name: &str, label: &str, help: &str) -> NodeFieldDef {
    NodeFieldDef {
        name: name.to_string(),
        label: label.to_string(),
        field_type: NodeFieldType::Text,
        help: Some(help.to_string()),
        ..Default::default()
    }
}

/// A flag's value as text: a string, or a number or boolean read as written.
/// `null` (unset) is empty; an object or a list is not text and is refused.
fn text_of(value: &Value, flag: &str, code: &'static str) -> Result<String, PipelineError> {
    match value {
        Value::Null => Ok(String::new()),
        Value::String(s) => Ok(s.trim().to_string()),
        Value::Number(n) => Ok(n.to_string()),
        Value::Bool(b) => Ok(b.to_string()),
        _ => Err(PipelineError::new(
            code,
            format!("{flag} must be text, not a JSON {}", if value.is_array() { "list" } else { "object" }),
        )),
    }
}

/// The longest tag.
pub const MAX_TAG_LEN: usize = 128;

/// A repeated tag flag (`--tag`, `--untag`, `--visible-to`): each
/// occurrence one tag, a `{{ [list] }}` flattened, empty ones dropped. A tag
/// is 1–128 characters without spaces or `*`; with `prefix`, a trailing `*`
/// makes it a prefix (`cell:*`, or `*` for every tag).
fn tags_of(value: &Value, flag: &str, code: &'static str, prefix: bool) -> Result<Vec<String>, PipelineError> {
    let mut tags = Vec::new();
    collect_tags(value, flag, code, &mut tags)?;
    for tag in &tags {
        let body = if prefix { tag.strip_suffix('*').unwrap_or(tag) } else { tag.as_str() };
        if tag.chars().count() > MAX_TAG_LEN || body.chars().any(|c| c.is_whitespace() || c == '*') {
            let star = if prefix { ", and * only at its end" } else { " or *" };
            return Err(PipelineError::new(code, format!("{flag} '{tag}': a tag is 1–{MAX_TAG_LEN} characters, no spaces{star}")));
        }
    }
    Ok(tags)
}

fn collect_tags(value: &Value, flag: &str, code: &'static str, out: &mut Vec<String>) -> Result<(), PipelineError> {
    match value {
        Value::Array(items) => items.iter().try_for_each(|item| collect_tags(item, flag, code, out)),
        other => {
            let text = text_of(other, flag, code)?;
            if !text.is_empty() {
                out.push(text);
            }
            Ok(())
        }
    }
}

/// One exact room: a name, never a pattern.
fn exact_room(room: &str, flag: &str, code: &'static str) -> Result<String, PipelineError> {
    match RoomPattern::parse(room) {
        Ok(RoomPattern::Exact(name)) => Ok(name),
        Ok(RoomPattern::Wild { .. }) => Err(PipelineError::new(code, format!("{flag} '{room}' is a pattern; this takes one exact room"))),
        Err(why) => Err(PipelineError::new(code, why)),
    }
}

/// A selection of rooms: an exact name, a pattern or `*` (every room).
fn selection(room: &str, flag: &str, code: &'static str) -> Result<String, PipelineError> {
    if room == "*" {
        return Ok(room.to_string());
    }
    RoomPattern::parse(room).map(|_| room.to_string()).map_err(|why| PipelineError::new(code, format!("{flag}: {why}")))
}

/// An optional duration flag inside `min..=max`.
fn duration_flag(value: &Value, flag: &str, code: &'static str, min: Duration, max: Duration) -> Result<Option<Duration>, PipelineError> {
    use crate::pipeline::nodes::shared::units::{describe_duration, duration};
    let text = text_of(value, flag, code)?;
    if text.is_empty() {
        return Ok(None);
    }
    let parsed = duration(&text, flag, code)?;
    if parsed < min || parsed > max {
        return Err(PipelineError::new(
            code,
            format!("{flag} {text} must be between {} and {}", describe_duration(min), describe_duration(max)),
        ));
    }
    Ok(Some(parsed))
}

/// A unix time as the answers write it.
fn rfc3339(secs: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp(secs, 0)
        .unwrap_or_default()
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// The room a node acts on: `--room`, else the room of the `trigger.room`
/// that started the run (`$trigger.room_id`). Empty when there is neither.
fn room_of(configured: &str, metadata: &Value) -> String {
    if !configured.is_empty() {
        return configured.to_string();
    }
    trigger_field(metadata, "room_id")
}

/// One text field of the run's trigger envelope (`$trigger.<name>`), empty
/// when the run has none.
fn trigger_field(metadata: &Value, name: &str) -> String {
    crate::pipeline::nodes::basic::trigger::envelope(metadata)
        .get(name)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

/// The run's project: `(owner, project)`.
fn project_of(metadata: &Value) -> (&str, &str) {
    let owner = metadata.get("owner").and_then(Value::as_str).unwrap_or_default();
    let project = metadata.get("project").and_then(Value::as_str).unwrap_or_default();
    (owner, project)
}

/// An exact room that is not open (`rooms.md` § Sending), whichever room
/// node met it.
pub const ROOM_NOT_OPEN_CODE: &str = "FW_NODE_WS_ROOM_NOT_OPEN";

/// A hub refusal as the node's error: a room not open is
/// [`ROOM_NOT_OPEN_CODE`], anything else the node's `code`.
fn refused(refusal: HubRefusal, code: &'static str) -> PipelineError {
    let code = match refusal {
        HubRefusal::NotOpen(_) => ROOM_NOT_OPEN_CODE,
        _ => code,
    };
    PipelineError::new(code, refusal.to_string())
}

/// The node's answer under its noun, the rest of the payload kept.
fn answer(payload: &Value, answer: Value, trace: String) -> NodeExecutionOutput {
    NodeExecutionOutput {
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        payload: with_answer(payload, answer),
        trace: vec![trace],
    }
}
