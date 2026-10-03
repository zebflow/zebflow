//! The `crypto` family: one kind per task, the noun its answer key.
//!
//! | Kind | File | Answer |
//! |---|---|---|
//! | `crypto.password.hash` | `password.rs` | `password: { hash, algorithm }` |
//! | `crypto.password.verify` | `password.rs` | `password: { valid }` on `true` / `false` |
//! | `crypto.digest.create` | `digest.rs` | `digest: { algorithm, value }` (hex) |
//! | `crypto.signature.sign` | `signature.rs` | `signature: { algorithm, value }` (hex) |
//! | `crypto.signature.verify` | `signature.rs` | `signature: { valid }` on `true` / `false` |
//! | `crypto.base64.encode` | `base64.rs` | `base64: { value }` |
//! | `crypto.base64.decode` | `base64.rs` | `base64: { text }` |
//! | `crypto.random.generate` | `random.rs` | `random: { value }` |
//!
//! Every value a kind works on comes from a flag (a literal or `{{ expr }}`,
//! resolved before the node is built), and a needed value that is empty is
//! refused with the kind's `_EMPTY` code — a missing password is never hashed
//! as the empty string, and an unknown user's empty hash never verifies.

pub mod base64;
pub mod digest;
pub mod password;
pub mod random;
pub mod signature;

#[cfg(test)]
mod tests;

use std::sync::Arc;

use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::pipeline::model::{DslFlag, DslFlagKind, NodeFieldDef, NodeFieldType, SelectOptionDef};
use crate::pipeline::nodes::{NodeExecutionOutput, NodeHandler};
use crate::pipeline::nodes::shared::util::with_answer;
use crate::pipeline::{NodeDefinition, PipelineError};
use crate::platform::services::CredentialService;

const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";
const OUTPUT_PIN_TRUE: &str = "true";
const OUTPUT_PIN_FALSE: &str = "false";

/// Every kind of the family, in the order the node index shows them.
pub const KINDS: &[&str] = &[
    password::HASH_KIND,
    password::VERIFY_KIND,
    digest::NODE_KIND,
    signature::SIGN_KIND,
    signature::VERIFY_KIND,
    base64::ENCODE_KIND,
    base64::DECODE_KIND,
    random::NODE_KIND,
];

/// The family list.
pub fn definitions() -> Vec<NodeDefinition> {
    vec![
        password::hash_definition(),
        password::verify_definition(),
        digest::definition(),
        signature::sign_definition(),
        signature::verify_definition(),
        base64::encode_definition(),
        base64::decode_definition(),
        random::definition(),
    ]
}

/// The node for `kind`, built from its resolved config; `None` when `kind`
/// is not of this family. The signature kinds read their key from the
/// credential service, which an engine without one cannot offer.
pub fn build(
    kind: &str,
    config: &Value,
    credentials: Option<Arc<CredentialService>>,
) -> Result<Option<Box<dyn NodeHandler>>, PipelineError> {
    let signing = |code: &'static str| {
        credentials.clone().ok_or_else(|| PipelineError::new(code, "credential service is not configured on this framework engine"))
    };
    let node: Box<dyn NodeHandler> = match kind {
        password::HASH_KIND => Box::new(password::Hash::new(parse(config, password::HASH_CONFIG_CODE)?)?),
        password::VERIFY_KIND => Box::new(password::Verify::new(parse(config, password::VERIFY_CONFIG_CODE)?)?),
        digest::NODE_KIND => Box::new(digest::Node::new(parse(config, digest::CONFIG_CODE)?)?),
        signature::SIGN_KIND => Box::new(signature::Sign::new(
            parse(config, signature::SIGN_CONFIG_CODE)?,
            signing(signature::SIGN_CODE)?,
        )?),
        signature::VERIFY_KIND => Box::new(signature::Verify::new(
            parse(config, signature::VERIFY_CONFIG_CODE)?,
            signing(signature::VERIFY_CODE)?,
        )?),
        base64::ENCODE_KIND => Box::new(base64::Encode::new(parse(config, base64::ENCODE_CONFIG_CODE)?)?),
        base64::DECODE_KIND => Box::new(base64::Decode::new(parse(config, base64::DECODE_CONFIG_CODE)?)?),
        random::NODE_KIND => Box::new(random::Node::new(parse(config, random::CONFIG_CODE)?)?),
        _ => return Ok(None),
    };
    Ok(Some(node))
}

fn parse<T: DeserializeOwned>(config: &Value, code: &'static str) -> Result<T, PipelineError> {
    serde_json::from_value(config.clone()).map_err(|err| PipelineError::new(code, err.to_string()))
}

// ── Shared by the kinds ──────────────────────────────────────────────────────

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

/// A closed choice: its words listed, so the signature and the save-time
/// check both see them.
fn choice_flag(name: &str, key: &str, description: &str, words: &[&str]) -> DslFlag {
    DslFlag { choices: words.iter().map(|w| w.to_string()).collect(), ..flag(name, key, description, "") }
}

fn field(name: &str, label: &str, help: &str) -> NodeFieldDef {
    NodeFieldDef { name: name.to_string(), label: label.to_string(), field_type: NodeFieldType::Text, help: Some(help.to_string()), ..Default::default() }
}

fn select(name: &str, label: &str, help: &str, words: &[&str]) -> NodeFieldDef {
    NodeFieldDef {
        field_type: NodeFieldType::Select,
        options: words.iter().map(|w| SelectOptionDef { value: w.to_string(), label: w.to_string() }).collect(),
        default_value: words.first().map(|w| Value::String(w.to_string())),
        ..field(name, label, help)
    }
}

/// A flag's value as text: a string, or a number or boolean read as written.
/// `null` (unset) is empty; an object or a list is not text and is refused.
fn text_of(value: &Value, flag: &str, code: &'static str) -> Result<String, PipelineError> {
    match value {
        Value::Null => Ok(String::new()),
        Value::String(s) => Ok(s.clone()),
        Value::Number(n) => Ok(n.to_string()),
        Value::Bool(b) => Ok(b.to_string()),
        _ => Err(PipelineError::new(code, format!("{flag} must be text, not a JSON {}", if value.is_array() { "list" } else { "object" }))),
    }
}

/// A needed text flag: refused when empty ("empty is not a value").
fn required(value: &Value, flag: &str, config_code: &'static str, empty_code: &'static str) -> Result<String, PipelineError> {
    let text = text_of(value, flag, config_code)?;
    if text.is_empty() {
        return Err(PipelineError::new(empty_code, format!("{flag} is empty; it needs a value")));
    }
    Ok(text)
}

/// The node's answer under its noun, the rest of the payload kept.
fn answer(pin: &str, payload: &Value, answer: Value, trace: String) -> NodeExecutionOutput {
    NodeExecutionOutput { output_pins: vec![pin.to_string()], payload: with_answer(payload, answer), trace: vec![trace] }
}
