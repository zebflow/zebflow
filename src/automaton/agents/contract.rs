//! Output contract checking for strategic agents (Phase A: the VERIFY gate).
//!
//! Dependency-free — built only on `serde_json`. Implements the subset of
//! JSON Schema that matters for validating tool-enabled agent output:
//! `type`, `required`, `properties`, `items`, `enum`, `minItems`, `minLength`.
//!
//! # Why this exists
//!
//! Without a contract, an agent loop exits the moment the model emits text —
//! it trusts the first "I'm done". This module is the cheap, deterministic
//! check that runs *before* accepting a final answer: it rejects malformed or
//! incomplete output without spending another model call, and turns failures
//! into concrete reasons that the loop feeds back for REPAIR.
//!
//! This is intentionally a small, stable, in-crate subset. A full JSON Schema
//! validator can later replace [`check_contract`] behind the same signature
//! without touching the agent loop.

use serde_json::Value;

/// Outcome of checking a candidate against a contract.
#[derive(Debug, Clone)]
pub struct Verdict {
    /// Whether the candidate satisfied every constraint.
    pub pass: bool,
    /// Human-readable failure reasons (empty when `pass` is true). These are
    /// fed back to the model verbatim to drive repair.
    pub reasons: Vec<String>,
}

impl Verdict {
    pub fn ok() -> Self {
        Self {
            pass: true,
            reasons: Vec::new(),
        }
    }
    pub fn fail(reasons: Vec<String>) -> Self {
        Self {
            pass: false,
            reasons,
        }
    }
}

/// Recover a JSON value from raw model text.
///
/// Tries, in order: whole-string parse, markdown-fence-stripped parse, then the
/// outermost `{...}` or `[...]` span (whichever starts earlier). Returns `None`
/// when no JSON can be recovered.
pub fn extract_json(raw: &str) -> Option<Value> {
    let t = raw.trim();
    if let Ok(v) = serde_json::from_str::<Value>(t) {
        return Some(v);
    }
    let stripped = t
        .trim_start_matches("```json")
        .trim_start_matches("```JSON")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    if let Ok(v) = serde_json::from_str::<Value>(stripped) {
        return Some(v);
    }
    let obj = span(stripped, '{', '}');
    let arr = span(stripped, '[', ']');
    let pick = match (obj, arr) {
        (Some(o), Some(a)) => {
            if o.0 <= a.0 {
                Some(o)
            } else {
                Some(a)
            }
        }
        (Some(o), None) => Some(o),
        (None, Some(a)) => Some(a),
        (None, None) => None,
    };
    pick.and_then(|(s, e)| serde_json::from_str::<Value>(&stripped[s..=e]).ok())
}

/// Byte span (start, end-inclusive) from the first `open` to the last `close`.
/// `open`/`close` must be ASCII so byte indexing is safe.
fn span(s: &str, open: char, close: char) -> Option<(usize, usize)> {
    let start = s.find(open)?;
    let end = s.rfind(close)?;
    if end > start {
        Some((start, end))
    } else {
        None
    }
}

/// Check `value` against a `schema` (JSON Schema subset). Always returns a
/// [`Verdict`]; an empty/`null` schema imposes no constraints (pass).
pub fn check_contract(value: &Value, schema: &Value) -> Verdict {
    if schema.is_null() {
        return Verdict::ok();
    }
    let mut reasons = Vec::new();
    check_node(value, schema, "$", &mut reasons);
    if reasons.is_empty() {
        Verdict::ok()
    } else {
        Verdict::fail(reasons)
    }
}

fn check_node(value: &Value, schema: &Value, path: &str, reasons: &mut Vec<String>) {
    let schema = match schema.as_object() {
        Some(o) => o,
        None => return, // non-object schema node => no constraints
    };

    // type
    if let Some(t) = schema.get("type").and_then(|v| v.as_str()) {
        if !type_matches(value, t) {
            reasons.push(format!(
                "{}: expected type '{}', got '{}'",
                path,
                t,
                type_name(value)
            ));
            return; // type mismatch — deeper checks would be noise
        }
    }

    // enum
    if let Some(en) = schema.get("enum").and_then(|v| v.as_array()) {
        if !en.iter().any(|e| e == value) {
            reasons.push(format!("{}: value is not one of the allowed enum values", path));
        }
    }

    // string constraints
    if let Some(s) = value.as_str() {
        if let Some(ml) = schema.get("minLength").and_then(|v| v.as_u64()) {
            if (s.chars().count() as u64) < ml {
                reasons.push(format!("{}: string is shorter than minLength {}", path, ml));
            }
        }
    }

    // object constraints
    if let Some(obj) = value.as_object() {
        if let Some(req) = schema.get("required").and_then(|v| v.as_array()) {
            for r in req {
                if let Some(key) = r.as_str() {
                    if !obj.contains_key(key) {
                        reasons.push(format!("{}: missing required key '{}'", path, key));
                    }
                }
            }
        }
        if let Some(props) = schema.get("properties").and_then(|v| v.as_object()) {
            for (k, subschema) in props {
                if let Some(child) = obj.get(k) {
                    check_node(child, subschema, &format!("{}.{}", path, k), reasons);
                }
            }
        }
    }

    // array constraints
    if let Some(arr) = value.as_array() {
        if let Some(mi) = schema.get("minItems").and_then(|v| v.as_u64()) {
            if (arr.len() as u64) < mi {
                reasons.push(format!(
                    "{}: array has {} item(s) but needs at least {}",
                    path,
                    arr.len(),
                    mi
                ));
            }
        }
        if let Some(items) = schema.get("items") {
            for (i, el) in arr.iter().enumerate() {
                check_node(el, items, &format!("{}[{}]", path, i), reasons);
            }
        }
    }
}

fn type_matches(v: &Value, t: &str) -> bool {
    match t {
        "object" => v.is_object(),
        "array" => v.is_array(),
        "string" => v.is_string(),
        "number" => v.is_number(),
        "integer" => v.is_i64() || v.is_u64(),
        "boolean" => v.is_boolean(),
        "null" => v.is_null(),
        _ => true, // unknown type keyword => don't constrain
    }
}

fn type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn passes_when_required_present() {
        let schema = json!({"type":"object","required":["data_queries"],
            "properties":{"data_queries":{"type":"array","minItems":1}}});
        let v = json!({"data_queries":[{"id":"q1"}]});
        assert!(check_contract(&v, &schema).pass);
    }

    #[test]
    fn fails_on_empty_required_array() {
        let schema = json!({"type":"object","required":["data_queries"],
            "properties":{"data_queries":{"type":"array","minItems":1}}});
        let v = json!({"data_queries":[]});
        let verdict = check_contract(&v, &schema);
        assert!(!verdict.pass);
        assert!(!verdict.reasons.is_empty());
    }

    #[test]
    fn fails_on_missing_key() {
        let schema = json!({"type":"object","required":["answer"]});
        let v = json!({"other": 1});
        assert!(!check_contract(&v, &schema).pass);
    }

    #[test]
    fn null_schema_passes() {
        assert!(check_contract(&json!({"x":1}), &Value::Null).pass);
    }

    #[test]
    fn extract_from_fenced_block() {
        let raw = "Here is the result:\n```json\n{\"answer\": 4}\n```\nDone.";
        let v = extract_json(raw).unwrap();
        assert_eq!(v.get("answer").and_then(|x| x.as_i64()), Some(4));
    }

    #[test]
    fn extract_bare_object_with_prose() {
        let raw = "Sure! {\"a\": 1, \"b\": [2,3]} hope that helps";
        assert!(extract_json(raw).is_some());
    }
}
