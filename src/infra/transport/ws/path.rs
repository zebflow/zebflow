//! Dynamic path template interpolation for WebSocket state paths.
//!
//! State paths support `{key}` placeholders that are resolved against the
//! flowing pipeline payload at execution time. This is the **single source of
//! truth** for all dynamic path resolution in the WS engine — every node that
//! accepts a `--path` argument routes through [`interpolate_path`].
//!
//! # Supported patterns
//!
//! | Template | Payload key | Resolved path |
//! |---|---|---|
//! | `/places/hall` | — | `/places/hall` |
//! | `/players/{session_id}` | `session_id: "abc"` | `/players/abc` |
//! | `/places/house/{user_id}` | `user_id: "u42"` | `/places/house/u42` |
//! | `/rooms/{room_type}/players/{user_id}` | `room_type: "hall"`, `user_id: "u9"` | `/rooms/hall/players/u9` |
//!
//! # Key lookup rules
//!
//! - Placeholders are resolved from the **top-level string fields** of the payload
//!   JSON object.  Nested lookups are intentionally not supported — keep entity
//!   identifiers flat (`session_id`, `user_id`, `place_id`) rather than `user.id`.
//! - Numbers and booleans are written as text (`{seat}` with `3` → `3`).
//! - A placeholder that is missing, null, an object/array, or an empty string is
//!   an error ([`PathError`]): resolving it to `""` turned `/players/{session_id}`
//!   into `/players`, so a merge wrote into — and a delete wiped — the whole map.
//! - Malformed placeholders (unclosed `{`) are emitted verbatim so they are
//!   visible in trace logs.

use serde_json::Value;

/// A `{placeholder}` that resolved to nothing usable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathError {
    /// The placeholder name, without braces.
    pub placeholder: String,
}

impl std::fmt::Display for PathError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "path placeholder {{{}}} is missing, empty or not text/number in the payload",
            self.placeholder
        )
    }
}

/// Expand `{key}` placeholders in a path template using top-level string
/// fields from `payload`.
///
/// # Arguments
///
/// * `template` — A JSON-pointer-style path with optional `{key}` segments,
///   e.g. `"/players/{session_id}"` or `"/places/house/{user_id}"`.
/// * `payload`  — The flowing pipeline payload.  Only top-level string fields
///   are examined for substitution.
///
/// # Returns
///
/// A fully-resolved path string ready to be passed to [`crate::ws::room`]
/// state operations, or [`PathError`] naming the placeholder that resolved
/// to nothing.
///
/// # Examples
///
/// ```ignore
/// use serde_json::json;
/// use zebflow::ws::path::interpolate_path;
///
/// let p = json!({ "session_id": "abc123", "user_id": "u42" });
///
/// assert_eq!(interpolate_path("/places/hall", &p).unwrap(), "/places/hall");
/// assert_eq!(interpolate_path("/players/{session_id}", &p).unwrap(), "/players/abc123");
/// assert!(interpolate_path("/players/{missing}", &p).is_err());
/// ```
pub fn interpolate_path(template: &str, payload: &Value) -> Result<String, PathError> {
    // Fast path: no placeholders → return immediately without allocating.
    if !template.contains('{') {
        return Ok(template.to_string());
    }

    let mut result = String::with_capacity(template.len() + 32);
    let mut chars = template.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch != '{' {
            result.push(ch);
            continue;
        }

        // Collect the placeholder key up to the matching '}'.
        let mut key = String::new();
        let mut closed = false;
        for inner in chars.by_ref() {
            if inner == '}' {
                closed = true;
                break;
            }
            key.push(inner);
        }

        if closed && !key.is_empty() {
            // Look up a top-level field: text as is, numbers and booleans as text.
            let value = match payload.get(&key) {
                Some(Value::String(s)) => s.clone(),
                Some(Value::Number(n)) => n.to_string(),
                Some(Value::Bool(b)) => b.to_string(),
                _ => String::new(),
            };
            if value.is_empty() {
                return Err(PathError { placeholder: key });
            }
            result.push_str(&value);
        } else {
            // Malformed placeholder — emit verbatim so it shows up in traces.
            result.push('{');
            result.push_str(&key);
            // Note: no closing '}' emitted for unclosed placeholders.
        }
    }

    Ok(result)
}

// ---- Unit tests ------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn ok(t: &str, p: &Value) -> String {
        interpolate_path(t, p).expect("resolves")
    }

    #[test]
    fn static_path_returned_unchanged() {
        let p = json!({});
        assert_eq!(ok("/places/hall", &p), "/places/hall");
        assert_eq!(ok("/players", &p), "/players");
        assert_eq!(ok("/", &p), "/");
        assert_eq!(ok("", &p), "");
    }

    #[test]
    fn placeholders_resolved() {
        let p = json!({ "session_id": "abc123", "room_type": "arena", "user_id": "u9" });
        assert_eq!(ok("/players/{session_id}", &p), "/players/abc123");
        assert_eq!(ok("/rooms/{room_type}/players/{user_id}", &p), "/rooms/arena/players/u9");
    }

    #[test]
    fn numbers_and_booleans_are_written_as_text() {
        let p = json!({ "seat": 3, "ready": true });
        assert_eq!(ok("/seats/{seat}/{ready}", &p), "/seats/3/true");
    }

    #[test]
    fn a_missing_placeholder_is_refused_not_collapsed() {
        // `/players/` would have become `/players`: a delete wiped every player.
        let err = interpolate_path("/players/{session_id}", &json!({})).unwrap_err();
        assert_eq!(err.placeholder, "session_id");
    }

    #[test]
    fn empty_null_or_structured_values_are_refused() {
        for v in [json!(""), Value::Null, json!({ "a": 1 }), json!([1])] {
            let p = json!({ "session_id": v });
            assert!(interpolate_path("/players/{session_id}", &p).is_err());
        }
    }

    #[test]
    fn malformed_unclosed_placeholder_emitted_verbatim() {
        let p = json!({ "session_id": "abc" });
        assert_eq!(ok("/players/{session_id", &p), "/players/{session_id");
    }
}
