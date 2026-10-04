//! `logic.*` — control flow: `logic.if`, `logic.match`, `logic.collect`,
//! `logic.foreach`, `logic.reduce`, `logic.retry`, and `logic.concept`, a
//! step described but not built yet.

use crate::pipeline::NodeDefinition;

pub mod collect;
pub mod concept;
pub mod foreach_;
pub mod if_;
pub mod match_;
pub mod reduce;
pub mod retry;

/// Metadata key the engine sets on the `logic.reduce` or `logic.collect`
/// that closes a `logic.foreach` loop: the items the loop's runs delivered
/// into it, in item order (`node-conventions.md` §4, Loops). The close runs
/// once, over all of them; its payload is the one the foreach received.
pub const LOOP_ITEMS_METADATA_KEY: &str = "loop_items";

pub fn definitions() -> Vec<NodeDefinition> {
    vec![
        if_::definition(),
        match_::definition(),
        collect::definition(),
        foreach_::definition(),
        reduce::definition(),
        retry::definition(),
        concept::definition(),
    ]
}

/// An expression flag's text (`--when`, `--from`, `--initial`, `--step`).
/// The DSL reads a bare `true` or `42` as a JSON scalar; as an expression it
/// is that JavaScript literal, so it is taken back as its text.
pub(crate) fn expression_text<'de, D: serde::Deserializer<'de>>(de: D) -> Result<String, D::Error> {
    use serde::Deserialize;
    match serde_json::Value::deserialize(de)? {
        serde_json::Value::String(text) => Ok(text),
        serde_json::Value::Null => Ok(String::new()),
        other @ (serde_json::Value::Bool(_) | serde_json::Value::Number(_)) => Ok(other.to_string()),
        other => Err(serde::de::Error::custom(format!("an expression is text, got {other}"))),
    }
}

/// The text of a required expression flag, refused when it is empty
/// ("empty is not a value").
pub(crate) fn required_expression<'a>(
    text: &'a str,
    flag: &str,
    code: &'static str,
) -> Result<&'a str, crate::pipeline::PipelineError> {
    let text = text.trim();
    if text.is_empty() {
        return Err(crate::pipeline::PipelineError::new(code, format!("{flag} is empty; it needs an expression")));
    }
    Ok(text)
}
