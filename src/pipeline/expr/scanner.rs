//! Scan a JSON config object for `{{ expr }}` template expressions.
//!
//! Used by the pipeline engine to resolve dynamic config values before
//! executing a node. See `resolver.rs` for the evaluation step.

use serde_json::Value;

/// Keys that are never scanned for expressions.
/// These hold code/markup that may legitimately contain `{{` for other purposes.
const SKIP_KEYS: &[&str] = &["markup", "source"];

/// A single segment inside a scanned string value.
#[derive(Debug, Clone)]
pub enum Segment {
    /// Plain text — emitted as-is during reconstruction.
    Literal(String),
    /// A `{{ expr }}` placeholder — evaluated at runtime.
    Expr(String),
}

/// A config field that contains one or more `{{ expr }}` expressions.
#[derive(Debug, Clone)]
pub struct ExprField {
    /// JSON Pointer to the field (e.g. `"/url"`, `"/params_expr"`).
    pub ptr: String,
    /// Parsed segments of the original string value.
    pub segments: Vec<Segment>,
    /// True when the entire string value is exactly one `{{ expr }}`.
    /// In this case the expression result preserves its native JS type.
    /// When false, all parts are stringified and concatenated.
    pub is_whole: bool,
}

/// Recursively scan `config` for `{{ expr }}` patterns.
///
/// Returns one `ExprField` per string field that contains at least one valid expression.
/// Keys listed in `SKIP_KEYS` are skipped.
pub fn scan(config: &Value) -> Vec<ExprField> {
    let mut out = Vec::new();
    scan_value(config, "", &mut out);
    out
}

fn escape_ptr_token(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

fn scan_value(val: &Value, ptr: &str, out: &mut Vec<ExprField>) {
    match val {
        Value::Object(map) => {
            for (k, v) in map {
                if SKIP_KEYS.contains(&k.as_str()) {
                    continue;
                }
                let child = format!("{}/{}", ptr, escape_ptr_token(k));
                scan_value(v, &child, out);
            }
        }
        Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                let child = format!("{}/{}", ptr, i);
                scan_value(v, &child, out);
            }
        }
        Value::String(s) => {
            if let Some(segments) = parse_template(s) {
                let is_whole =
                    segments.len() == 1 && matches!(&segments[0], Segment::Expr(_));
                out.push(ExprField { ptr: ptr.to_string(), segments, is_whole });
            }
        }
        _ => {}
    }
}

/// Parse a string value for `{{ expr }}` patterns.
/// Returns `None` if no valid expressions are found.
fn parse_template(s: &str) -> Option<Vec<Segment>> {
    if !s.contains("{{") {
        return None;
    }
    let mut segments: Vec<Segment> = Vec::new();
    let mut remaining = s;
    let mut found_any = false;

    while let Some(open) = remaining.find("{{") {
        let before = &remaining[..open];
        if !before.is_empty() {
            segments.push(Segment::Literal(before.to_string()));
        }
        let after_open = &remaining[open + 2..];
        if let Some(close) = after_open.find("}}") {
            let expr = after_open[..close].trim().to_string();
            if !expr.is_empty() {
                segments.push(Segment::Expr(expr));
                found_any = true;
            } else {
                // Empty {{ }} — emit as literal to avoid silent data loss.
                segments.push(Segment::Literal("{{}}".to_string()));
            }
            remaining = &after_open[close + 2..];
        } else {
            // Unclosed {{ — treat the rest as a literal.
            segments.push(Segment::Literal(remaining[open..].to_string()));
            remaining = "";
            break;
        }
    }
    if !remaining.is_empty() {
        segments.push(Segment::Literal(remaining.to_string()));
    }

    if found_any { Some(segments) } else { None }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn whole_expr() {
        let config = json!({ "url": "{{ $input.endpoint }}" });
        let fields = scan(&config);
        assert_eq!(fields.len(), 1);
        assert!(fields[0].is_whole);
        assert_eq!(fields[0].ptr, "/url");
    }

    #[test]
    fn interpolated_expr() {
        let config = json!({ "url": "https://api.example.com/{{ $input.id }}/details" });
        let fields = scan(&config);
        assert_eq!(fields.len(), 1);
        assert!(!fields[0].is_whole);
        assert_eq!(fields[0].segments.len(), 3);
    }

    #[test]
    fn no_exprs() {
        let config = json!({ "url": "https://static.example.com/path" });
        let fields = scan(&config);
        assert!(fields.is_empty());
    }

    #[test]
    fn skip_source_key() {
        let config = json!({ "source": "return {{ bad }};" });
        let fields = scan(&config);
        assert!(fields.is_empty(), "source key must be skipped");
    }

    #[test]
    fn nested_field() {
        let config = json!({ "body": { "name": "{{ $trigger.auth.sub }}" } });
        let fields = scan(&config);
        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].ptr, "/body/name");
    }
}
