//! The `kv` family: the project's key-value store and its message bus.
//!
//! | Kind | File | Answer |
//! |---|---|---|
//! | `kv.entry.get` | `get.rs` | `entry: { key, value, found }` |
//! | `kv.entry.put` | `put.rs` | `entry: { key, ttl? }` |
//! | `kv.entry.delete` | `delete.rs` | `entry: { key, deleted }` |
//! | `kv.entry.head` | `head.rs` | `entry: { key, exists }` |
//! | `kv.entry.increment` | `increment.rs` | `entry: { key, value }` |
//! | `kv.entry.expire` | `expire.rs` | `entry: { key, ttl?, updated }` |
//! | `kv.message.publish` | `publish.rs` | `message: { topic, delivered }` |
//!
//! Every value comes from a flag (a literal or `{{ expr }}`, resolved before
//! the node is built), so the flags are checked when the node is built: an
//! empty `--key`, or one under the platform's own `zf.` prefix, is refused
//! with the kind's `_KEY` code, a `--ttl` that is
//! not a duration with its `_CONFIG` code. A `ttl` in an answer is the
//! duration it was given in seconds, e.g. `"600s"`.

pub mod delete;
pub mod expire;
pub mod get;
pub mod head;
pub mod increment;
pub mod publish;
pub mod put;

#[cfg(test)]
mod tests;

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::infra::io::state::DynStateBus;
use crate::pipeline::model::{DslFlag, DslFlagKind, NodeFieldDef, NodeFieldType};
use crate::pipeline::nodes::shared::util::with_answer;
use crate::pipeline::nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler};
use crate::pipeline::{NodeDefinition, PipelineError};

const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

/// Every kind of the family, in the order the node index shows them.
pub const KINDS: &[&str] = &[
    get::NODE_KIND,
    put::NODE_KIND,
    delete::NODE_KIND,
    head::NODE_KIND,
    increment::NODE_KIND,
    expire::NODE_KIND,
    publish::NODE_KIND,
];

/// The family list.
pub fn definitions() -> Vec<NodeDefinition> {
    vec![
        get::definition(),
        put::definition(),
        delete::definition(),
        head::definition(),
        increment::definition(),
        expire::definition(),
        publish::definition(),
    ]
}

/// The node for `kind`, built from its resolved config; `None` when `kind`
/// is not of this family. Every kind needs the state bus, which an engine
/// without one cannot offer.
pub fn build(kind: &str, config: &Value, bus: Option<DynStateBus>) -> Result<Option<Box<dyn NodeHandler>>, PipelineError> {
    let bus = |code: &'static str| bus.clone().ok_or_else(|| PipelineError::new(code, "the state bus is not configured on this pipeline engine"));
    let node: Box<dyn NodeHandler> = match kind {
        get::NODE_KIND => Box::new(get::Node::new(parse(config, get::CONFIG_CODE)?, bus(get::UNAVAILABLE_CODE)?)?),
        put::NODE_KIND => Box::new(put::Node::new(parse(config, put::CONFIG_CODE)?, bus(put::UNAVAILABLE_CODE)?)?),
        delete::NODE_KIND => Box::new(delete::Node::new(parse(config, delete::CONFIG_CODE)?, bus(delete::UNAVAILABLE_CODE)?)?),
        head::NODE_KIND => Box::new(head::Node::new(parse(config, head::CONFIG_CODE)?, bus(head::UNAVAILABLE_CODE)?)?),
        increment::NODE_KIND => Box::new(increment::Node::new(parse(config, increment::CONFIG_CODE)?, bus(increment::UNAVAILABLE_CODE)?)?),
        expire::NODE_KIND => Box::new(expire::Node::new(parse(config, expire::CONFIG_CODE)?, bus(expire::UNAVAILABLE_CODE)?)?),
        publish::NODE_KIND => Box::new(publish::Node::new(parse(config, publish::CONFIG_CODE)?, bus(publish::UNAVAILABLE_CODE)?)?),
        _ => return Ok(None),
    };
    Ok(Some(node))
}

fn parse<T: DeserializeOwned>(config: &Value, code: &'static str) -> Result<T, PipelineError> {
    serde_json::from_value(config.clone()).map_err(|err| PipelineError::new(code, err.to_string()))
}

// ── Shared by the kinds ──────────────────────────────────────────────────────

/// The owner and project a run belongs to.
fn scope(input: &NodeExecutionInput) -> (String, String) {
    let read = |key: &str| input.metadata.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
    (read("owner"), read("project"))
}

/// A scalar flag with its 0.11 metadata.
fn flag(name: &str, key: &str, description: &str, value: &str) -> DslFlag {
    DslFlag {
        flag: name.to_string(),
        config_key: key.to_string(),
        description: description.to_string(),
        kind: DslFlagKind::Scalar,
        value: value.to_string(),
        ..Default::default()
    }
}

/// `--key`, the entry every `kv.entry.*` kind names.
fn key_flag(description: &str) -> DslFlag {
    DslFlag { required: true, ..flag("--key", "key", description, "text") }
}

/// `--durable`: the disk-backed store instead of the in-memory one.
fn durable_flag() -> DslFlag {
    DslFlag {
        kind: DslFlagKind::Bool,
        ..flag("--durable", "durable", "Use the disk-backed store, which survives a restart. Default: the in-memory one.", "")
    }
}

fn field(name: &str, label: &str, help: &str) -> NodeFieldDef {
    NodeFieldDef { name: name.to_string(), label: label.to_string(), field_type: NodeFieldType::Text, help: Some(help.to_string()), ..Default::default() }
}

fn durable_field() -> NodeFieldDef {
    NodeFieldDef { field_type: NodeFieldType::Checkbox, ..field("durable", "Durable", "Use the disk-backed store, which survives a restart. Default: the in-memory one.") }
}

/// A flag's value as text: a string, or a number or boolean read as written.
/// `null` (unset) is empty; an object or a list is not text and is refused.
fn text_of(value: &Value, flag: &str, code: &'static str) -> Result<String, PipelineError> {
    match value {
        Value::Null => Ok(String::new()),
        Value::String(s) => Ok(s.trim().to_string()),
        Value::Number(n) => Ok(n.to_string()),
        Value::Bool(b) => Ok(b.to_string()),
        _ => Err(PipelineError::new(code, format!("{flag} must be text, not a JSON {}", if value.is_array() { "list" } else { "object" }))),
    }
}

/// `--key`, refused when it resolves empty ("empty is not a value").
/// Keys the platform keeps in the project's store for itself — a published
/// MCP route's OAuth state (`zf.oauth/…`). No `kv.*` node reads or writes one.
pub const RESERVED_KEY_PREFIX: &str = "zf.";

fn required_key(value: &Value, config_code: &'static str, key_code: &'static str) -> Result<String, PipelineError> {
    let key = text_of(value, "--key", config_code)?;
    if key.is_empty() {
        return Err(PipelineError::new(key_code, "--key is empty; it needs a value"));
    }
    if key.starts_with(RESERVED_KEY_PREFIX) {
        return Err(PipelineError::new(
            key_code,
            format!("--key '{key}' starts with `{RESERVED_KEY_PREFIX}`, which the platform keeps for itself (a published route's sign-ins); choose another key"),
        ));
    }
    Ok(key)
}

/// `--ttl` as whole seconds: a duration of at least one second
/// (`600s`, `10m`, `1d`); unset is `None`, no expiry. A bare number names no
/// unit and `0` is not a lifetime, so both are refused.
fn ttl_seconds(value: &Value, code: &'static str) -> Result<Option<u64>, PipelineError> {
    let text = text_of(value, "--ttl", code)?;
    if text.is_empty() {
        return Ok(None);
    }
    let duration = crate::pipeline::nodes::shared::units::duration(&text, "--ttl", code)?;
    if duration.as_millis() % 1000 != 0 || duration.as_secs() == 0 {
        return Err(PipelineError::new(code, format!("--ttl '{text}' must be whole seconds, at least 1s")));
    }
    Ok(Some(duration.as_secs()))
}

/// A lifetime as the answer says it: `"600s"`.
fn ttl_text(seconds: u64) -> String {
    format!("{seconds}s")
}

/// The node's answer under its noun, the rest of the payload kept.
fn answer(payload: &Value, answer: Value, trace: String) -> NodeExecutionOutput {
    NodeExecutionOutput { output_pins: vec![OUTPUT_PIN_OUT.to_string()], payload: with_answer(payload, answer), trace: vec![trace] }
}

/// A state-bus failure under the calling kind's code: the world's side.
fn bus_error(code: &'static str) -> impl Fn(crate::infra::io::state::StateBusError) -> PipelineError {
    move |err| PipelineError::new(code, err.to_string())
}
