//! Dynamic Tailwind-variant hint collector.
//!
//! `tw-variants` is a compile-time hint channel that declares possible dynamic
//! class tokens/patterns in a template subtree. Hints are aggregated as a page
//! union and consumed by the Tailwind-like processor.
//!
//! Supported attribute examples:
//!
//! - `tw-variants="bg-red-800 bg-orange-500 text-[*]"`
//! - `tw-variants="tw(bg-red-800 text-[*])"`
//! - `tw-variants="tw(bg-red-800 text-[*]); tw(border-[*])"`

use std::collections::{BTreeMap, BTreeSet};

/// Aggregated `tw-variants` manifest for a compiled page.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TwVariantManifest {
    /// Exact class tokens that should be compiled into static CSS.
    pub exact_tokens: BTreeSet<String>,
    /// Pattern tokens that imply dynamic runtime handling (for example `bg-[*]`).
    pub wildcard_patterns: BTreeSet<String>,
    /// Raw occurrence count by declared token/pattern.
    pub frequency: BTreeMap<String, usize>,
}

impl TwVariantManifest {
    /// Returns `true` when no hints are present.
    pub fn is_empty(&self) -> bool {
        self.exact_tokens.is_empty() && self.wildcard_patterns.is_empty()
    }

    /// Returns `true` when wildcard patterns were declared.
    pub fn has_wildcards(&self) -> bool {
        !self.wildcard_patterns.is_empty()
    }
}

/// Collects and aggregates all `tw-variants="..."` declarations from markup.
pub fn collect_tw_variants(html: &str) -> TwVariantManifest {
    let mut manifest = TwVariantManifest::default();
    let mut cursor = 0usize;

    while let Some(start_rel) = html[cursor..].find("tw-variants=\"") {
        let value_start = cursor + start_rel + "tw-variants=\"".len();
        let Some(end_rel) = html[value_start..].find('"') else {
            break;
        };
        let value_end = value_start + end_rel;
        let value = &html[value_start..value_end];

        for token in parse_tw_variants_value(value) {
            *manifest.frequency.entry(token.clone()).or_insert(0) += 1;
            if is_wildcard_pattern(&token) {
                manifest.wildcard_patterns.insert(token);
            } else {
                manifest.exact_tokens.insert(token);
            }
        }

        cursor = value_end + 1;
    }

    manifest
}

/// Emits compile-time CSS hooks for dynamic pattern families.
///
/// This keeps runtime handling lightweight:
///
/// - `bg-[*]` -> `.tw-bg-dyn { background-color: var(--tw-bg); }`
/// - `text-[*]` -> `.tw-text-dyn { color: var(--tw-text); }`
/// - `border-[*]` -> `.tw-border-dyn { border-color: var(--tw-border); }`
pub fn dynamic_runtime_css_for_patterns(patterns: &BTreeSet<String>) -> String {
    let mut css = String::new();
    if patterns.contains("bg-[*]") {
        css.push_str(".tw-bg-dyn{background-color:var(--tw-bg);}");
    }
    if patterns.contains("text-[*]") {
        css.push_str(".tw-text-dyn{color:var(--tw-text);}");
    }
    if patterns.contains("border-[*]") {
        css.push_str(".tw-border-dyn{border-color:var(--tw-border);}");
    }
    css
}

fn parse_tw_variants_value(value: &str) -> Vec<String> {
    let groups = extract_tw_groups(value);
    let mut out = Vec::new();
    for group in groups {
        for token in split_tokens(&group) {
            let normalized = normalize_token(&token);
            if !normalized.is_empty() {
                out.push(normalized);
            }
        }
    }
    out
}

fn extract_tw_groups(value: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cursor = 0usize;
    let mut found_group = false;

    while let Some(rel) = value[cursor..].find("tw(") {
        found_group = true;
        let start = cursor + rel + 3;
        let Some((group, next)) = take_balanced_group(value, start) else {
            break;
        };
        out.push(group);
        cursor = next;
    }

    if found_group {
        return out;
    }

    out.push(value.to_string());
    out
}

fn take_balanced_group(input: &str, start: usize) -> Option<(String, usize)> {
    let mut depth = 1usize;
    let mut bracket_depth = 0usize;
    let mut idx = start;
    while idx < input.len() {
        let ch = input[idx..].chars().next()?;
        match ch {
            '[' => bracket_depth += 1,
            ']' => bracket_depth = bracket_depth.saturating_sub(1),
            '(' if bracket_depth == 0 => depth += 1,
            ')' if bracket_depth == 0 => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some((input[start..idx].to_string(), idx + ch.len_utf8()));
                }
            }
            _ => {}
        }
        idx += ch.len_utf8();
    }
    None
}

fn split_tokens(raw: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut bracket_depth = 0usize;
    for ch in raw.chars() {
        match ch {
            '[' => {
                bracket_depth += 1;
                buf.push(ch);
            }
            ']' => {
                bracket_depth = bracket_depth.saturating_sub(1);
                buf.push(ch);
            }
            ',' | ';' if bracket_depth == 0 => {
                flush_token(&mut out, &mut buf);
            }
            c if c.is_whitespace() && bracket_depth == 0 => {
                flush_token(&mut out, &mut buf);
            }
            _ => buf.push(ch),
        }
    }
    flush_token(&mut out, &mut buf);
    out
}

fn flush_token(out: &mut Vec<String>, buf: &mut String) {
    let token = buf.trim();
    if !token.is_empty() {
        out.push(token.to_string());
    }
    buf.clear();
}

fn normalize_token(token: &str) -> String {
    token
        .trim()
        .trim_matches('"')
        .trim_matches('\'')
        .trim()
        .to_string()
}

fn is_wildcard_pattern(token: &str) -> bool {
    token.contains("[*]")
}
