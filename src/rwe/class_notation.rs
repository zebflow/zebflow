//! Typed class-notation helpers shared by RWE compile and render stages.
//!
//! Contract:
//!
//! - `@{path|option|option|...}`
//! - `@{path|option|option|...|default=option}`
//!
//! Option forms:
//!
//! - bare token: `red`, `blue`, `md`, `2xl`
//! - bracketed class-group: `[bg-gray-900 text-white]`
//!
//! This keeps dynamic class usage explicit and finite so compile-time class
//! extraction remains predictable.

/// Parsed typed class slot.
#[derive(Debug, Clone)]
pub(crate) struct TypedClassSlot {
    /// Byte index right after the closing `}`.
    pub end: usize,
    /// Placeholder path.
    pub path: String,
    /// Allowed option values.
    pub options: Vec<String>,
    /// Optional default option value.
    pub default: Option<String>,
}

/// Parses a typed class slot starting at `start`.
pub(crate) fn parse_typed_class_slot(input: &str, start: usize) -> Option<TypedClassSlot> {
    if start >= input.len() || !input[start..].starts_with("@{") {
        return None;
    }

    let body_start = start + 2;
    let body_end = find_slot_body_end(input, body_start)?;
    let body = &input[body_start..body_end];
    let parts = split_slot_segments(body);
    if parts.len() < 2 {
        return None;
    }

    let path = parts[0].trim();
    if path.is_empty() {
        return None;
    }

    let mut options: Vec<String> = Vec::new();
    let mut default = None::<String>;
    for segment in parts.iter().skip(1) {
        let seg = segment.trim();
        if seg.is_empty() {
            return None;
        }
        if let Some(rhs) = seg.strip_prefix("default=") {
            let value = parse_slot_option(rhs.trim())?;
            default = Some(value.clone());
            if !options
                .iter()
                .any(|existing| normalize_value(existing) == normalize_value(&value))
            {
                options.push(value);
            }
            continue;
        }
        let value = parse_slot_option(seg)?;
        options.push(value);
    }
    if options.is_empty() {
        return None;
    }

    Some(TypedClassSlot {
        end: body_end + 1,
        path: path.to_string(),
        options,
        default,
    })
}

/// Extracts Tailwind-like tokens from a class attribute value.
///
/// This expands typed slots into all allowed options so all candidate utilities
/// are visible at compile time.
pub(crate) fn extract_tailwind_tokens_from_class_value(class_value: &str) -> Vec<String> {
    let mut out = Vec::new();
    for pattern in split_class_patterns(class_value) {
        let expanded = expand_class_pattern(&pattern);
        for candidate in expanded {
            for token in candidate.split_whitespace() {
                let token = token.trim();
                if !token.is_empty() {
                    out.push(token.to_string());
                }
            }
        }
    }
    out
}

/// Collects unresolved (untyped) `{{...}}` placeholders from class attributes.
pub(crate) fn collect_untyped_placeholders_from_class_value(class_value: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cursor = 0usize;
    while cursor < class_value.len() {
        if let Some(slot) = parse_typed_class_slot(class_value, cursor) {
            cursor = slot.end;
            continue;
        }
        if class_value[cursor..].starts_with("{{") {
            let expr_start = cursor + 2;
            let Some(end_rel) = class_value[expr_start..].find("}}") else {
                break;
            };
            let expr_end = expr_start + end_rel;
            let expr = class_value[expr_start..expr_end].trim();
            if expr.is_empty() {
                out.push("{{}}".to_string());
            } else {
                out.push(format!("{{{{{expr}}}}}"));
            }
            cursor = expr_end + 2;
            continue;
        }
        let Some(ch) = class_value[cursor..].chars().next() else {
            break;
        };
        cursor += ch.len_utf8();
    }
    out
}

/// Resolves typed class slots using caller-provided path lookup.
pub(crate) fn resolve_typed_class_macros<F>(input: &str, mut resolve_path: F) -> String
where
    F: FnMut(&str) -> Option<String>,
{
    let mut out = String::with_capacity(input.len());
    let mut cursor = 0usize;
    while cursor < input.len() {
        if let Some(slot) = parse_typed_class_slot(input, cursor) {
            let selected = resolve_path(&slot.path)
                .and_then(|value| select_slot_option(&slot, &value))
                .or_else(|| slot.default.clone())
                .unwrap_or_default();
            out.push_str(&selected);
            cursor = slot.end;
            continue;
        }
        let Some(ch) = input[cursor..].chars().next() else {
            break;
        };
        let next = cursor + ch.len_utf8();
        out.push_str(&input[cursor..next]);
        cursor = next;
    }
    out
}

fn select_slot_option(slot: &TypedClassSlot, value: &str) -> Option<String> {
    let wanted = normalize_value(value);
    slot.options
        .iter()
        .find(|option| normalize_value(option) == wanted)
        .cloned()
}

fn expand_class_pattern(pattern: &str) -> Vec<String> {
    let Some(start) = pattern.find("@{") else {
        return vec![pattern.to_string()];
    };
    let Some(slot) = parse_typed_class_slot(pattern, start) else {
        return vec![pattern.to_string()];
    };
    let prefix = &pattern[..start];
    let suffix = &pattern[slot.end..];
    let suffix_expanded = expand_class_pattern(suffix);
    let mut out = Vec::new();
    for option in &slot.options {
        for tail in &suffix_expanded {
            out.push(format!("{prefix}{option}{tail}"));
        }
    }
    out
}

fn split_class_patterns(class_value: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut cursor = 0usize;
    while cursor < class_value.len() {
        if let Some(slot) = parse_typed_class_slot(class_value, cursor) {
            buf.push_str(&class_value[cursor..slot.end]);
            cursor = slot.end;
            continue;
        }
        let Some(ch) = class_value[cursor..].chars().next() else {
            break;
        };
        if ch.is_whitespace() {
            if !buf.trim().is_empty() {
                out.push(buf.trim().to_string());
            }
            buf.clear();
            cursor += ch.len_utf8();
            continue;
        }
        let next = cursor + ch.len_utf8();
        buf.push_str(&class_value[cursor..next]);
        cursor = next;
    }
    if !buf.trim().is_empty() {
        out.push(buf.trim().to_string());
    }
    out
}

fn find_slot_body_end(input: &str, body_start: usize) -> Option<usize> {
    let mut idx = body_start;
    let mut bracket_depth = 0usize;
    while idx < input.len() {
        let ch = input[idx..].chars().next()?;
        match ch {
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '}' if bracket_depth == 0 => return Some(idx),
            _ => {}
        }
        idx += ch.len_utf8();
    }
    None
}

fn split_slot_segments(body: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut bracket_depth = 0usize;
    let mut idx = 0usize;
    while idx < body.len() {
        let Some(ch) = body[idx..].chars().next() else {
            break;
        };
        match ch {
            '[' => {
                bracket_depth += 1;
                buf.push(ch);
            }
            ']' => {
                bracket_depth = bracket_depth.saturating_sub(1);
                buf.push(ch);
            }
            '|' if bracket_depth == 0 => {
                out.push(buf.trim().to_string());
                buf.clear();
            }
            _ => buf.push(ch),
        }
        idx += ch.len_utf8();
    }
    if !buf.trim().is_empty() {
        out.push(buf.trim().to_string());
    }
    out
}

fn parse_slot_option(raw: &str) -> Option<String> {
    if raw.starts_with('[') && raw.ends_with(']') && raw.len() >= 2 {
        let inner = raw[1..raw.len() - 1].trim();
        if inner.is_empty() {
            return None;
        }
        return Some(inner.to_string());
    }
    let token = raw.trim();
    if token.is_empty() {
        return None;
    }
    Some(token.to_string())
}

fn normalize_value(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}
