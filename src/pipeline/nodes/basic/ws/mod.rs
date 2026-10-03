//! The `ws` family: WebSocket rooms and outgoing client sockets.
//!
//! | Kind | File | Answer |
//! |---|---|---|
//! | `ws.message.send` | `message_send.rs` | `message: { sent, room, event }` or `message: { sent, connection }` |
//! | `ws.state.put` | `state.rs` | `state: { key, room }` |
//! | `ws.state.update` | `state.rs` | `state: { key, room }` |
//! | `ws.state.delete` | `state.rs` | `state: { key, room }` |
//!
//! The trigger that starts a pipeline from a room event (`trigger.room`,
//! `trigger.rs`) lives with the family it serves.
//!
//! A room node without `--room` uses the room of the `trigger.room` that
//! started the run (`room_id` in its payload); a run started any other way
//! names its room.

pub mod message_send;
pub mod state;
pub mod trigger;

#[cfg(test)]
mod tests;

use serde_json::Value;

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
    ]
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

/// The room a node acts on: `--room`, else the room of the `trigger.room`
/// that started the run. Empty when there is neither.
fn room_of(configured: &str, payload: &Value) -> String {
    if !configured.is_empty() {
        return configured.to_string();
    }
    payload.get("room_id").and_then(Value::as_str).unwrap_or_default().to_string()
}

/// The hub's key for a room of this run's project.
fn room_key(metadata: &Value, room: &str) -> String {
    let owner = metadata.get("owner").and_then(Value::as_str).unwrap_or_default();
    let project = metadata.get("project").and_then(Value::as_str).unwrap_or_default();
    format!("{owner}/{project}/{room}")
}

/// The node's answer under its noun, the rest of the payload kept.
fn answer(payload: &Value, answer: Value, trace: String) -> NodeExecutionOutput {
    NodeExecutionOutput {
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        payload: with_answer(payload, answer),
        trace: vec![trace],
    }
}
