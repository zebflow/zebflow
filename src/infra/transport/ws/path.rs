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
//! - Non-string values (numbers, booleans) are **not** coerced; the placeholder
//!   resolves to `""`.  This produces a path like `/players/` which is valid JSON
//!   pointer but may write to the wrong bucket — verify at the pipeline level.
//! - Missing keys resolve to `""` with the same caveat.
//! - Malformed placeholders (unclosed `{`) are emitted verbatim so they are
//!   visible in trace logs.

use serde_json::Value;

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
/// state operations.
///
/// # Examples
///
/// ```ignore
/// use serde_json::json;
/// use zebflow::ws::path::interpolate_path;
///
/// let p = json!({ "session_id": "abc123", "user_id": "u42" });
///
/// // Static path — returned unchanged, zero allocations avoided by early return.
/// assert_eq!(interpolate_path("/places/hall", &p), "/places/hall");
///
/// // Single placeholder.
/// assert_eq!(interpolate_path("/players/{session_id}", &p), "/players/abc123");
///
/// // Nested dynamic path.
/// assert_eq!(interpolate_path("/places/house/{user_id}", &p), "/places/house/u42");
///
/// // Multiple placeholders in one path.
/// let p2 = json!({ "room_type": "arena", "user_id": "u9" });
/// assert_eq!(
///     interpolate_path("/rooms/{room_type}/players/{user_id}", &p2),
///     "/rooms/arena/players/u9"
/// );
/// ```
pub fn interpolate_path(template: &str, payload: &Value) -> String {
    // Fast path: no placeholders → return immediately without allocating.
    if !template.contains('{') {
        return template.to_string();
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
            // Look up a top-level string field in the payload.
            let value = payload
                .get(&key)
                .and_then(|v| v.as_str())
                .unwrap_or("");
            result.push_str(value);
        } else {
            // Malformed placeholder — emit verbatim so it shows up in traces.
            result.push('{');
            result.push_str(&key);
            // Note: no closing '}' emitted for unclosed placeholders.
        }
    }

    result
}

// ---- Unit tests ------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn static_path_returned_unchanged() {
        let p = json!({});
        assert_eq!(interpolate_path("/places/hall", &p), "/places/hall");
        assert_eq!(interpolate_path("/players", &p), "/players");
        assert_eq!(interpolate_path("/", &p), "/");
        assert_eq!(interpolate_path("", &p), "");
    }

    #[test]
    fn single_placeholder_resolved() {
        let p = json!({ "session_id": "abc123" });
        assert_eq!(
            interpolate_path("/players/{session_id}", &p),
            "/players/abc123"
        );
    }

    #[test]
    fn single_placeholder_at_suffix() {
        let p = json!({ "user_id": "u42" });
        assert_eq!(
            interpolate_path("/places/house/{user_id}", &p),
            "/places/house/u42"
        );
    }

    #[test]
    fn multiple_placeholders() {
        let p = json!({ "room_type": "arena", "user_id": "u9" });
        assert_eq!(
            interpolate_path("/rooms/{room_type}/players/{user_id}", &p),
            "/rooms/arena/players/u9"
        );
    }

    #[test]
    fn missing_key_produces_empty_segment() {
        let p = json!({});
        assert_eq!(interpolate_path("/players/{session_id}", &p), "/players/");
    }

    #[test]
    fn non_string_value_produces_empty_segment() {
        let p = json!({ "session_id": 42 });
        assert_eq!(interpolate_path("/players/{session_id}", &p), "/players/");
    }

    #[test]
    fn malformed_unclosed_placeholder_emitted_verbatim() {
        let p = json!({ "session_id": "abc" });
        assert_eq!(
            interpolate_path("/players/{session_id", &p),
            "/players/{session_id"
        );
    }

    #[test]
    fn placeholder_at_root() {
        let p = json!({ "entity": "ship" });
        assert_eq!(interpolate_path("/{entity}", &p), "/ship");
    }
}
