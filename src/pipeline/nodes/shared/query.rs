//! What the three database query nodes share — `pg.query.run`,
//! `sqlite.query.run` and `sekejap.query.run` take one grammar
//! (`node-conventions.md` §2 Selection, §6):
//!
//! ```text
//! <engine>.query.run (--query SQL | -- SQL) [--param K=V …] [--write] [--limit N] → query
//! ```
//!
//! - `--param` is a map: the key is the placeholder (`1` binds `$1` / `?1`, a
//!   name binds a named placeholder where the engine has them). Keys are all
//!   positional, `1..n` without a gap, or all named.
//! - `--write` is the only way to change data; each engine refuses a write
//!   without it with its own read-only mode, never by reading the SQL here.
//! - `--limit` caps the rows answered (default 200, at most 5000).
//! - The answer is one key, `query: { rows, columns, row_count, truncated }`,
//!   plus `rows_affected` when `--write` is set.

use serde_json::{Value, json};

use crate::pipeline::PipelineError;
use crate::pipeline::model::{DslFlag, DslFlagKind, NodeFieldDef, NodeFieldType};

/// Rows answered when `--limit` is not set.
pub const DEFAULT_LIMIT: usize = 200;
/// The most rows one run answers.
pub const MAX_LIMIT: usize = 5_000;

/// The bind values of `--param`, in the shape the engine binds them.
#[derive(Debug, Clone, PartialEq)]
pub enum Params {
    /// `1=…`, `2=…`: the value for `$1` / `?1` is first.
    Positional(Vec<Value>),
    /// `name=…`: a named placeholder each.
    Named(Vec<(String, Value)>),
}

/// `--param` read: an object of `key → value`, all positional (`1..n`, no
/// gap) or all named. `named` is whether the engine has named placeholders.
pub fn params(map: &Value, named: bool, code: &'static str) -> Result<Params, PipelineError> {
    let entries = match map {
        Value::Null => return Ok(Params::Positional(Vec::new())),
        Value::Object(entries) => entries,
        _ => {
            return Err(PipelineError::new(
                code,
                "--param is repeated key=value: `--param 1=…` binds $1; a list is no longer read",
            ));
        }
    };
    let mut positional: Vec<(usize, Value)> = Vec::new();
    let mut by_name: Vec<(String, Value)> = Vec::new();
    for (key, value) in entries {
        let key = key.trim();
        if let Some(index) = position(key) {
            positional.push((index, value.clone()));
        } else if is_name(key) {
            by_name.push((key.to_string(), value.clone()));
        } else {
            return Err(PipelineError::new(
                code,
                format!("--param key '{key}' is neither a position (1, 2, …) nor a name (letters, digits, _)"),
            ));
        }
    }
    if !positional.is_empty() && !by_name.is_empty() {
        return Err(PipelineError::new(
            code,
            "--param keys are all positions (1, 2, …) or all names, not both",
        ));
    }
    if !by_name.is_empty() {
        if !named {
            return Err(PipelineError::new(
                code,
                format!("this engine binds positions only: name the --param keys 1, 2, … (got '{}')", by_name[0].0),
            ));
        }
        return Ok(Params::Named(by_name));
    }
    positional.sort_by_key(|(index, _)| *index);
    for (expected, (index, _)) in positional.iter().enumerate() {
        if *index != expected + 1 {
            return Err(PipelineError::new(
                code,
                format!("--param keys run 1, 2, … without a gap: {} is missing", expected + 1),
            ));
        }
    }
    Ok(Params::Positional(positional.into_iter().map(|(_, value)| value).collect()))
}

/// `1`, `2`, … — a plain positive number, no sign or leading zero.
fn position(key: &str) -> Option<usize> {
    let plain = !key.is_empty() && !key.starts_with('0') && key.bytes().all(|b| b.is_ascii_digit());
    if plain { key.parse().ok() } else { None }
}

fn is_name(key: &str) -> bool {
    let mut chars = key.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// `--limit`: rows answered, `1..=MAX_LIMIT`, [`DEFAULT_LIMIT`] when unset.
pub fn limit(value: &Value, code: &'static str) -> Result<usize, PipelineError> {
    let n = match value {
        Value::Null => return Ok(DEFAULT_LIMIT),
        Value::String(text) if text.trim().is_empty() => return Ok(DEFAULT_LIMIT),
        Value::Number(n) => n.as_u64(),
        Value::String(text) => text.trim().parse::<u64>().ok(),
        _ => None,
    };
    match n {
        Some(n) if (1..=MAX_LIMIT as u64).contains(&n) => Ok(n as usize),
        _ => Err(PipelineError::new(
            code,
            format!("--limit {value} must be a whole number from 1 to {MAX_LIMIT}"),
        )),
    }
}

/// What a write refused for want of `--write` says.
pub fn write_refusal(kind: &str, detail: &str) -> String {
    format!("{kind} runs read-only: this statement writes ({detail}); add --write to let it change data")
}

/// The answer: one key, `query`. `rows_affected` is there when the node
/// ran with `--write`.
pub fn answer(columns: Vec<String>, rows: Vec<Value>, truncated: bool, rows_affected: Option<u64>) -> Value {
    let mut query = json!({
        "rows": rows,
        "columns": columns,
        "row_count": rows.len(),
        "truncated": truncated,
    });
    if let Some(affected) = rows_affected {
        query["rows_affected"] = json!(affected);
    }
    json!({ "query": query })
}

/// The answer's schema, shared by the three kinds.
pub fn output_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "query": {
                "type": "object",
                "properties": {
                    "rows": { "type": "array", "description": "One object per row, keyed by column name" },
                    "columns": { "type": "array", "items": { "type": "string" } },
                    "row_count": { "type": "integer" },
                    "truncated": { "type": "boolean", "description": "More rows matched than --limit answered" },
                    "rows_affected": { "type": "integer", "description": "Rows the statement changed; only with --write" }
                }
            }
        }
    })
}

/// `--query`: the statement, or the body after `--`.
pub fn query_flag(language: &str) -> DslFlag {
    DslFlag {
        flag: "--query".to_string(),
        config_key: "query".to_string(),
        description: format!("The {language} statement (or the body after `--`). A literal or {{{{ expr }}}}."),
        kind: DslFlagKind::Scalar,
        value: "text".to_string(),
        ..Default::default()
    }
}

/// `--param KEY=VALUE`, repeated.
pub fn param_flag(placeholders: &str) -> DslFlag {
    DslFlag {
        flag: "--param".to_string(),
        config_key: "param".to_string(),
        description: format!(
            "One bind value, repeated: {placeholders}. A literal is text; a whole {{{{ expr }}}} keeps its JSON type, so a number stays a number."
        ),
        kind: DslFlagKind::KeyValuePairs,
        value: "expression".to_string(),
        ..Default::default()
    }
}

/// `--write`: let the statement change data.
pub fn write_flag() -> DslFlag {
    DslFlag {
        flag: "--write".to_string(),
        config_key: "write".to_string(),
        description: "Let the statement change data. Without it the query runs read-only and a write is refused. Literal; never {{ }}.".to_string(),
        kind: DslFlagKind::Bool,
        ..Default::default()
    }
}

/// `--limit N`: rows answered.
pub fn limit_flag() -> DslFlag {
    DslFlag {
        flag: "--limit".to_string(),
        config_key: "limit".to_string(),
        description: format!("Rows answered (default {DEFAULT_LIMIT}, at most {MAX_LIMIT}); `truncated` says more matched."),
        kind: DslFlagKind::Scalar,
        value: "number".to_string(),
        ..Default::default()
    }
}

/// The editor fields for `--param`, `--write` and `--limit`.
pub fn fields(placeholders: &str) -> Vec<NodeFieldDef> {
    vec![
        NodeFieldDef {
            name: "param".to_string(),
            label: "Params".to_string(),
            field_type: NodeFieldType::KeyValuePairs,
            help: Some(format!("One row per bind value: {placeholders}. A value is a literal (text) or {{{{ expr }}}}.")),
            ..Default::default()
        },
        NodeFieldDef {
            name: "write".to_string(),
            label: "Write".to_string(),
            field_type: NodeFieldType::Checkbox,
            help: Some("Let the statement change data. Off: read-only, and a write is refused.".to_string()),
            default_value: Some(json!(false)),
            ..Default::default()
        },
        NodeFieldDef {
            name: "limit".to_string(),
            label: "Limit".to_string(),
            field_type: NodeFieldType::Number,
            help: Some(format!("Rows answered (default {DEFAULT_LIMIT}, at most {MAX_LIMIT}).")),
            default_value: Some(json!(DEFAULT_LIMIT)),
            ..Default::default()
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    const CODE: &str = "T";

    #[test]
    fn positional_keys_bind_in_order_and_keep_their_type() {
        let got = params(&json!({ "2": "Ana", "1": 7 }), false, CODE).unwrap();
        assert_eq!(got, Params::Positional(vec![json!(7), json!("Ana")]));
        assert_eq!(params(&Value::Null, false, CODE).unwrap(), Params::Positional(vec![]));
    }

    #[test]
    fn a_gap_a_mix_or_a_list_is_refused() {
        let gap = params(&json!({ "1": 1, "3": 3 }), true, CODE).unwrap_err();
        assert!(gap.message.contains("2 is missing"), "{}", gap.message);
        let mixed = params(&json!({ "1": 1, "name": "x" }), true, CODE).unwrap_err();
        assert!(mixed.message.contains("not both"), "{}", mixed.message);
        assert!(params(&json!(["a"]), true, CODE).is_err(), "the list form is gone");
        assert!(params(&json!({ "01": 1 }), true, CODE).is_err(), "a padded position is not a position");
        assert!(params(&json!({ "a-b": 1 }), true, CODE).is_err());
    }

    #[test]
    fn names_bind_only_where_the_engine_has_them() {
        assert_eq!(
            params(&json!({ "email": "a@example.com" }), true, CODE).unwrap(),
            Params::Named(vec![("email".to_string(), json!("a@example.com"))])
        );
        let refused = params(&json!({ "email": "a@example.com" }), false, CODE).unwrap_err();
        assert!(refused.message.contains("positions only"), "{}", refused.message);
    }

    #[test]
    fn the_limit_has_a_default_and_a_ceiling() {
        assert_eq!(limit(&Value::Null, CODE).unwrap(), DEFAULT_LIMIT);
        assert_eq!(limit(&json!(50), CODE).unwrap(), 50);
        assert_eq!(limit(&json!("5000"), CODE).unwrap(), MAX_LIMIT);
        assert!(limit(&json!(0), CODE).is_err());
        assert!(limit(&json!(5001), CODE).is_err());
        assert!(limit(&json!(2.5), CODE).is_err());
    }

    #[test]
    fn the_answer_is_one_query_key() {
        let read = answer(vec!["id".into()], vec![json!({ "id": 1 })], false, None);
        assert_eq!(read, json!({ "query": { "rows": [{ "id": 1 }], "columns": ["id"], "row_count": 1, "truncated": false } }));
        let write = answer(vec![], vec![], false, Some(3));
        assert_eq!(write["query"]["rows_affected"], json!(3));
    }
}
