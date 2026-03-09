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

/// Emits compile-time CSS hooks for dynamic pattern families declared via `tw-variants`.
///
/// Each `<prop>-[*]` pattern produces a `.tw-<prop>-dyn` class that reads from
/// a CSS custom property `--tw-<prop>`. At runtime set the variable with inline
/// `style={{ "--tw-h": "26rem" }}` and apply the class `tw-h-dyn`.
pub fn dynamic_runtime_css_for_patterns(patterns: &BTreeSet<String>) -> String {
    /// Emit one hook rule if the pattern is present.
    macro_rules! hook {
        ($css:expr, $patterns:expr, $pat:expr, $cls:expr, $decl:expr) => {
            if $patterns.contains($pat) {
                $css.push_str(concat!(".", $cls, "{", $decl, "}"));
            }
        };
    }

    let mut css = String::new();

    // ── Colour / background ──────────────────────────────────────────────────
    hook!(css, patterns, "bg-[*]",   "tw-bg-dyn",   "background-color:var(--tw-bg);");
    hook!(css, patterns, "text-[*]", "tw-text-dyn",  "color:var(--tw-text);");
    hook!(css, patterns, "border-[*]", "tw-border-dyn", "border-color:var(--tw-border);");
    hook!(css, patterns, "fill-[*]", "tw-fill-dyn",  "fill:var(--tw-fill);");
    hook!(css, patterns, "stroke-[*]", "tw-stroke-dyn", "stroke:var(--tw-stroke);");
    hook!(css, patterns, "outline-[*]", "tw-outline-dyn", "outline-color:var(--tw-outline);");
    hook!(css, patterns, "shadow-[*]", "tw-shadow-dyn", "box-shadow:var(--tw-shadow);");
    hook!(css, patterns, "opacity-[*]", "tw-opacity-dyn", "opacity:var(--tw-opacity);");

    // ── Sizing ───────────────────────────────────────────────────────────────
    hook!(css, patterns, "h-[*]",     "tw-h-dyn",     "height:var(--tw-h);");
    hook!(css, patterns, "w-[*]",     "tw-w-dyn",     "width:var(--tw-w);");
    hook!(css, patterns, "min-h-[*]", "tw-min-h-dyn", "min-height:var(--tw-min-h);");
    hook!(css, patterns, "max-h-[*]", "tw-max-h-dyn", "max-height:var(--tw-max-h);");
    hook!(css, patterns, "min-w-[*]", "tw-min-w-dyn", "min-width:var(--tw-min-w);");
    hook!(css, patterns, "max-w-[*]", "tw-max-w-dyn", "max-width:var(--tw-max-w);");
    hook!(css, patterns, "size-[*]",  "tw-size-dyn",  "width:var(--tw-size);height:var(--tw-size);");

    // ── Spacing ──────────────────────────────────────────────────────────────
    hook!(css, patterns, "p-[*]",  "tw-p-dyn",  "padding:var(--tw-p);");
    hook!(css, patterns, "px-[*]", "tw-px-dyn", "padding-left:var(--tw-px);padding-right:var(--tw-px);");
    hook!(css, patterns, "py-[*]", "tw-py-dyn", "padding-top:var(--tw-py);padding-bottom:var(--tw-py);");
    hook!(css, patterns, "pt-[*]", "tw-pt-dyn", "padding-top:var(--tw-pt);");
    hook!(css, patterns, "pr-[*]", "tw-pr-dyn", "padding-right:var(--tw-pr);");
    hook!(css, patterns, "pb-[*]", "tw-pb-dyn", "padding-bottom:var(--tw-pb);");
    hook!(css, patterns, "pl-[*]", "tw-pl-dyn", "padding-left:var(--tw-pl);");
    hook!(css, patterns, "m-[*]",  "tw-m-dyn",  "margin:var(--tw-m);");
    hook!(css, patterns, "mx-[*]", "tw-mx-dyn", "margin-left:var(--tw-mx);margin-right:var(--tw-mx);");
    hook!(css, patterns, "my-[*]", "tw-my-dyn", "margin-top:var(--tw-my);margin-bottom:var(--tw-my);");
    hook!(css, patterns, "mt-[*]", "tw-mt-dyn", "margin-top:var(--tw-mt);");
    hook!(css, patterns, "mr-[*]", "tw-mr-dyn", "margin-right:var(--tw-mr);");
    hook!(css, patterns, "mb-[*]", "tw-mb-dyn", "margin-bottom:var(--tw-mb);");
    hook!(css, patterns, "ml-[*]", "tw-ml-dyn", "margin-left:var(--tw-ml);");
    hook!(css, patterns, "gap-[*]",   "tw-gap-dyn",   "gap:var(--tw-gap);");
    hook!(css, patterns, "gap-x-[*]", "tw-gap-x-dyn", "column-gap:var(--tw-gap-x);");
    hook!(css, patterns, "gap-y-[*]", "tw-gap-y-dyn", "row-gap:var(--tw-gap-y);");

    // ── Positioning ──────────────────────────────────────────────────────────
    hook!(css, patterns, "top-[*]",    "tw-top-dyn",    "top:var(--tw-top);");
    hook!(css, patterns, "right-[*]",  "tw-right-dyn",  "right:var(--tw-right);");
    hook!(css, patterns, "bottom-[*]", "tw-bottom-dyn", "bottom:var(--tw-bottom);");
    hook!(css, patterns, "left-[*]",   "tw-left-dyn",   "left:var(--tw-left);");
    hook!(css, patterns, "inset-[*]",  "tw-inset-dyn",  "inset:var(--tw-inset);");
    hook!(css, patterns, "z-[*]",      "tw-z-dyn",      "z-index:var(--tw-z);");

    // ── Typography ───────────────────────────────────────────────────────────
    hook!(css, patterns, "leading-[*]",  "tw-leading-dyn",  "line-height:var(--tw-leading);");
    hook!(css, patterns, "tracking-[*]", "tw-tracking-dyn", "letter-spacing:var(--tw-tracking);");
    hook!(css, patterns, "indent-[*]",   "tw-indent-dyn",   "text-indent:var(--tw-indent);");

    // ── Border ───────────────────────────────────────────────────────────────
    hook!(css, patterns, "rounded-[*]",   "tw-rounded-dyn",   "border-radius:var(--tw-rounded);");
    hook!(css, patterns, "border-t-[*]",  "tw-border-t-dyn",  "border-top-color:var(--tw-border-t);");
    hook!(css, patterns, "border-r-[*]",  "tw-border-r-dyn",  "border-right-color:var(--tw-border-r);");
    hook!(css, patterns, "border-b-[*]",  "tw-border-b-dyn",  "border-bottom-color:var(--tw-border-b);");
    hook!(css, patterns, "border-l-[*]",  "tw-border-l-dyn",  "border-left-color:var(--tw-border-l);");

    // ── Transform ────────────────────────────────────────────────────────────
    hook!(css, patterns, "rotate-[*]",      "tw-rotate-dyn",      "transform:rotate(var(--tw-rotate));");
    hook!(css, patterns, "scale-[*]",       "tw-scale-dyn",       "transform:scale(var(--tw-scale));");
    hook!(css, patterns, "translate-x-[*]", "tw-translate-x-dyn", "transform:translateX(var(--tw-translate-x));");
    hook!(css, patterns, "translate-y-[*]", "tw-translate-y-dyn", "transform:translateY(var(--tw-translate-y));");
    hook!(css, patterns, "skew-x-[*]",      "tw-skew-x-dyn",      "transform:skewX(var(--tw-skew-x));");
    hook!(css, patterns, "skew-y-[*]",      "tw-skew-y-dyn",      "transform:skewY(var(--tw-skew-y));");

    // ── Misc ─────────────────────────────────────────────────────────────────
    hook!(css, patterns, "blur-[*]",          "tw-blur-dyn",          "filter:blur(var(--tw-blur));");
    hook!(css, patterns, "brightness-[*]",    "tw-brightness-dyn",    "filter:brightness(var(--tw-brightness));");
    hook!(css, patterns, "duration-[*]",      "tw-duration-dyn",      "transition-duration:var(--tw-duration);");
    hook!(css, patterns, "delay-[*]",         "tw-delay-dyn",         "transition-delay:var(--tw-delay);");
    hook!(css, patterns, "grid-cols-[*]",     "tw-grid-cols-dyn",     "grid-template-columns:var(--tw-grid-cols);");
    hook!(css, patterns, "grid-rows-[*]",     "tw-grid-rows-dyn",     "grid-template-rows:var(--tw-grid-rows);");
    hook!(css, patterns, "col-span-[*]",      "tw-col-span-dyn",      "grid-column:span var(--tw-col-span)/span var(--tw-col-span);");
    hook!(css, patterns, "row-span-[*]",      "tw-row-span-dyn",      "grid-row:span var(--tw-row-span)/span var(--tw-row-span);");

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
