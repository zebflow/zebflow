//! Tailwind-like token compiler for Zebflow RWE.
//!
//! This implementation is kept as a dedicated
//! module so RWE engines can reuse style compilation without mixing concerns
//! into render orchestration code.

use serde_json::Value;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Mutex, OnceLock};

use super::variants::{collect_tw_variants, dynamic_runtime_css_for_patterns};
use crate::rwe::class_notation::extract_tailwind_tokens_from_class_value;

#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub(crate) struct ParsedTw {
    pub(crate) styles: HashMap<String, Value>,
    pub(crate) props: HashMap<String, Value>,
    pub(crate) total_tokens: usize,
    pub(crate) supported_tokens: usize,
    pub(crate) ignored_tokens: Vec<String>,
    pub(crate) applied_tokens: Vec<String>,
}

#[derive(Debug, Clone)]
struct UtilityRule {
    selector: String,
    declarations: String,
    prelude: Option<String>,
}

const TOKEN_RULE_CACHE_LIMIT: usize = 4096;
static TOKEN_RULE_CACHE: OnceLock<Mutex<TokenRuleCache>> = OnceLock::new();
const TAILWIND_PREFLIGHT_RAW: &str = include_str!("preflight.css");
static TAILWIND_PREFLIGHT_NORMALIZED: OnceLock<String> = OnceLock::new();

#[derive(Debug)]
struct TokenRuleCache {
    limit: usize,
    map: HashMap<String, Option<String>>,
    order: VecDeque<String>,
}

impl TokenRuleCache {
    fn new(limit: usize) -> Self {
        Self {
            limit,
            map: HashMap::new(),
            order: VecDeque::new(),
        }
    }
    fn get(&mut self, key: &str) -> Option<Option<String>> {
        let value = self.map.get(key).cloned();
        if value.is_some() {
            self.touch(key);
        }
        value
    }
    fn insert(&mut self, key: String, value: Option<String>) {
        if self.map.contains_key(&key) {
            self.map.insert(key.clone(), value);
            self.touch(&key);
            return;
        }
        self.map.insert(key.clone(), value);
        self.order.push_back(key);
        self.evict_if_needed();
    }
    fn touch(&mut self, key: &str) {
        if let Some(pos) = self.order.iter().position(|existing| existing == key) {
            self.order.remove(pos);
        }
        self.order.push_back(key.to_string());
    }
    fn evict_if_needed(&mut self) {
        while self.map.len() > self.limit {
            if let Some(oldest) = self.order.pop_front() {
                self.map.remove(&oldest);
            } else {
                break;
            }
        }
    }
}

pub(crate) fn token_css_rule(token: &str) -> Option<String> {
    let cache =
        TOKEN_RULE_CACHE.get_or_init(|| Mutex::new(TokenRuleCache::new(TOKEN_RULE_CACHE_LIMIT)));
    if let Ok(mut guard) = cache.lock() {
        if let Some(cached) = guard.get(token) {
            return cached;
        }
    }
    let computed = token_css_rule_uncached(token);
    if let Ok(mut guard) = cache.lock() {
        guard.insert(token.to_string(), computed.clone());
    }
    computed
}

fn token_css_rule_uncached(token: &str) -> Option<String> {
    let (variants, raw_utility) = split_variants(token)?;
    let selector = format!(".{}", escape_class_selector(token));
    let (important, utility) = if let Some(rest) = raw_utility.strip_prefix('!') {
        (true, rest)
    } else {
        (false, raw_utility.as_str())
    };
    let mut rule = utility_rule(utility, &selector, important)?;
    let mut medias = Vec::new();
    for variant in &variants {
        if let Some(media) = variant_media_query(variant) {
            medias.push(media);
            continue;
        }
        if apply_relational_variant(&mut rule.selector, variant) {
            continue;
        }
        if apply_attribute_variant(&mut rule.selector, variant) {
            continue;
        }
        match variant.as_str() {
            "rtl" => {
                rule.selector = format!("[dir=\"rtl\"] {}", rule.selector);
                continue;
            }
            "ltr" => {
                rule.selector = format!("[dir=\"ltr\"] {}", rule.selector);
                continue;
            }
            "*" => {
                rule.selector.push_str(" > *");
                continue;
            }
            "**" => {
                rule.selector.push_str(" *");
                continue;
            }
            _ => {}
        }
        if let Some(pseudo) = variant_pseudo(variant) {
            rule.selector.push_str(pseudo);
            continue;
        }
        return None;
    }
    let mut out = format!("{}{{{}}}", rule.selector, rule.declarations);
    for media in medias.into_iter().rev() {
        if media.starts_with("@supports ") {
            out = format!("{}{{{}}}", media, out);
        } else {
            out = format!("@media {}{{{}}}", media, out);
        }
    }
    if let Some(prelude) = rule.prelude.take() {
        return Some(format!("{}\n{}", prelude, out));
    }
    Some(out)
}

/// Compiles supported utility classes from HTML and injects generated CSS.
///
/// Behavior:
///
/// - scans static `class="..."` tokens
/// - converts each recognized token into CSS
/// - injects a consolidated `<style data-rwe-tw>` block into `<head>`
/// - leaves unsupported tokens untouched in markup
pub fn process_tailwind(html: &str, extra_tokens: &HashSet<String>) -> String {
    let mut css = tailwind_preflight_css().to_string();
    let variants = collect_tw_variants(html);

    let mut tokens = HashSet::new();
    let mut cursor = 0;
    while let Some(start) = html[cursor..].find("class=\"") {
        let actual_start = cursor + start + 7;
        if let Some(end) = html[actual_start..].find('"') {
            let class_value = &html[actual_start..actual_start + end];
            for token in extract_tailwind_tokens_from_class_value(class_value) {
                tokens.insert(token);
            }
            cursor = actual_start + end + 1;
        } else {
            break;
        }
    }
    for token in &variants.exact_tokens {
        tokens.insert(token.clone());
    }
    for token in extra_tokens {
        tokens.insert(token.clone());
    }

    let mut sorted_tokens: Vec<String> = tokens.into_iter().collect();
    sorted_tokens.sort_by(|a, b| compare_token_precedence(a, b));
    for token in sorted_tokens {
        if let Some(rule) = token_css_rule(&token) {
            css.push_str(&rule);
        }
    }
    css.push_str(&dynamic_runtime_css_for_patterns(
        &variants.wildcard_patterns,
    ));
    let css = minify_css_lossy(&css);
    if css.is_empty() {
        return html.to_string();
    }
    let style_block = format!("<style data-rwe-tw>{}</style>", css);
    if let Some(pos) = html.find("</head>") {
        let mut out = html.to_string();
        out.insert_str(pos, &style_block);
        out
    } else {
        format!("{}{}", style_block, html)
    }
}

/// Rebuilds the generated Tailwind-like style block from the provided HTML.
///
/// This removes any previous `<style data-rwe-tw>` block and recompiles styles
/// from the current markup snapshot.
pub fn rebuild_tailwind(html: &str) -> String {
    let stripped = strip_generated_tailwind_style_blocks(html);
    process_tailwind(&stripped, &HashSet::new())
}

fn strip_generated_tailwind_style_blocks(html: &str) -> String {
    const OPEN: &str = "<style data-rwe-tw>";
    const CLOSE: &str = "</style>";

    let mut out = String::with_capacity(html.len());
    let mut cursor = 0usize;

    while let Some(start_rel) = html[cursor..].find(OPEN) {
        let start = cursor + start_rel;
        out.push_str(&html[cursor..start]);
        let content_start = start + OPEN.len();
        if let Some(close_rel) = html[content_start..].find(CLOSE) {
            cursor = content_start + close_rel + CLOSE.len();
        } else {
            // Malformed style block: keep remaining content untouched.
            out.push_str(&html[start..]);
            return out;
        }
    }

    out.push_str(&html[cursor..]);
    out
}

/// Orders utility tokens so generated CSS respects Tailwind-like cascade:
///
/// - base utilities first (`w-20`, `ml-20`)
/// - non-media variants next (`hover:*`, `focus:*`)
/// - responsive variants last (`sm:*`, `md:*`, `lg:*`, ...)
///
/// This avoids responsive rules being accidentally overridden by later base
/// utilities when class tokens are sorted alphabetically.
fn compare_token_precedence(a: &str, b: &str) -> std::cmp::Ordering {
    let ak = token_precedence_key(a);
    let bk = token_precedence_key(b);
    ak.cmp(&bk).then_with(|| a.cmp(b))
}

fn token_precedence_key(token: &str) -> (u8, u8, u8) {
    let Some((variants, utility)) = split_variants(token) else {
        return (0, 0, 0);
    };
    let utility_rank = utility_precedence_rank(&utility);
    if variants.is_empty() {
        return (0, 0, utility_rank);
    }

    let mut has_non_media = false;
    let mut max_media_rank = 0u8;
    let mut has_media = false;

    for variant in variants {
        if let Some(rank) = variant_media_rank(&variant) {
            has_media = true;
            if rank > max_media_rank {
                max_media_rank = rank;
            }
        } else {
            has_non_media = true;
        }
    }

    if has_media {
        // Keep all responsive variants after base and pseudo variants.
        // Use breakpoint rank so `sm` rules emit before `md`, `lg`, ...
        return (2, max_media_rank, utility_rank);
    }
    if has_non_media {
        return (1, 0, utility_rank);
    }
    (0, 0, utility_rank)
}

fn utility_precedence_rank(utility: &str) -> u8 {
    let utility = utility.strip_prefix('!').unwrap_or(utility);
    if utility.starts_with("from-") {
        return 1;
    }
    if utility.starts_with("via-") {
        return 2;
    }
    if utility.starts_with("to-") {
        return 3;
    }
    0
}

fn tailwind_preflight_css() -> &'static str {
    TAILWIND_PREFLIGHT_NORMALIZED
        .get_or_init(|| minify_css_lossy(&normalize_theme_functions(TAILWIND_PREFLIGHT_RAW)))
        .as_str()
}

fn normalize_theme_functions(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut cursor = 0usize;

    while let Some(start_rel) = raw[cursor..].find("--theme(") {
        let start = cursor + start_rel;
        out.push_str(&raw[cursor..start]);
        let args_start = start + "--theme(".len();
        let Some((args, close_paren)) = extract_theme_args(raw, args_start) else {
            out.push_str(&raw[start..]);
            return out;
        };
        out.push_str(&theme_fallback_args(args));
        cursor = close_paren + 1;
    }

    out.push_str(&raw[cursor..]);
    out
}

fn minify_css_lossy(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    let mut in_string: Option<char> = None;
    let mut escaped = false;
    let mut pending_space = false;

    while let Some(ch) = chars.next() {
        if let Some(quote) = in_string {
            out.push(ch);
            if escaped {
                escaped = false;
                continue;
            }
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == quote {
                in_string = None;
            }
            continue;
        }

        if ch == '\'' || ch == '"' {
            if pending_space && should_emit_space(out.chars().last(), Some(ch)) {
                out.push(' ');
            }
            pending_space = false;
            in_string = Some(ch);
            out.push(ch);
            continue;
        }

        if ch == '/' && matches!(chars.peek(), Some('*')) {
            chars.next();
            let mut prev = '\0';
            for c in chars.by_ref() {
                if prev == '*' && c == '/' {
                    break;
                }
                prev = c;
            }
            pending_space = true;
            continue;
        }

        if ch.is_whitespace() {
            pending_space = true;
            continue;
        }

        if pending_space && should_emit_space(out.chars().last(), Some(ch)) {
            out.push(' ');
        }
        pending_space = false;

        if is_css_punct(ch) && out.ends_with(' ') {
            out.pop();
        }
        out.push(ch);
    }

    out.trim().to_string()
}

fn is_css_punct(ch: char) -> bool {
    matches!(
        ch,
        '{' | '}' | ':' | ';' | ',' | '>' | '+' | '~' | '(' | ')' | '[' | ']' | '='
    )
}

fn should_emit_space(prev: Option<char>, next: Option<char>) -> bool {
    let (Some(p), Some(n)) = (prev, next) else {
        return false;
    };
    if p.is_whitespace() || n.is_whitespace() {
        return false;
    }
    if is_css_punct(p) || is_css_punct(n) {
        return false;
    }
    true
}

fn extract_theme_args(input: &str, args_start: usize) -> Option<(&str, usize)> {
    let mut depth = 1usize;
    for (off, ch) in input[args_start..].char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let close = args_start + off;
                    return Some((&input[args_start..close], close));
                }
            }
            _ => {}
        }
    }
    None
}

fn theme_fallback_args(args: &str) -> String {
    let parts = split_top_level_commas(args);
    if parts.len() <= 1 {
        return "initial".to_string();
    }
    let fallback = parts[1..].join(",");
    let trimmed = fallback.trim();
    if trimmed.is_empty() {
        "initial".to_string()
    } else {
        trimmed.to_string()
    }
}

fn split_top_level_commas(input: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut buf = String::new();
    let mut depth = 0usize;
    let mut quote: Option<char> = None;
    let mut escaped = false;

    for ch in input.chars() {
        if let Some(q) = quote {
            buf.push(ch);
            if ch == q && !escaped {
                quote = None;
            }
            escaped = ch == '\\' && !escaped;
            continue;
        }

        match ch {
            '\'' | '"' => {
                quote = Some(ch);
                escaped = false;
                buf.push(ch);
            }
            '(' | '[' | '{' => {
                depth += 1;
                buf.push(ch);
            }
            ')' | ']' | '}' => {
                depth = depth.saturating_sub(1);
                buf.push(ch);
            }
            ',' if depth == 0 => {
                out.push(buf.trim().to_string());
                buf.clear();
            }
            _ => {
                buf.push(ch);
            }
        }
    }

    if !buf.trim().is_empty() {
        out.push(buf.trim().to_string());
    }
    out
}

fn escape_class_selector(token: &str) -> String {
    let mut out = String::new();
    for ch in token.chars() {
        if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
            out.push(ch);
        } else if ch == '\\' {
            out.push_str("\\\\");
        } else {
            out.push('\\');
            out.push(ch);
        }
    }
    out
}

fn split_variants(token: &str) -> Option<(Vec<String>, String)> {
    let mut parts: Vec<String> = Vec::new();
    let mut buf = String::new();
    let mut depth = 0usize;
    for ch in token.chars() {
        match ch {
            '[' => {
                depth += 1;
                buf.push(ch);
            }
            ']' => {
                depth = depth.saturating_sub(1);
                buf.push(ch);
            }
            ':' if depth == 0 => {
                if buf.is_empty() {
                    return None;
                }
                parts.push(buf.clone());
                buf.clear();
            }
            _ => buf.push(ch),
        }
    }
    if buf.is_empty() {
        return None;
    }
    parts.push(buf);
    if parts.is_empty() {
        return None;
    }
    let utility = parts.pop()?;
    Some((parts, utility))
}

fn variant_media_query(v: &str) -> Option<String> {
    let fixed = match v {
        "sm" => "(min-width: 640px)",
        "md" => "(min-width: 768px)",
        "lg" => "(min-width: 1024px)",
        "xl" => "(min-width: 1280px)",
        "2xl" => "(min-width: 1536px)",
        "motion-reduce" => "(prefers-reduced-motion: reduce)",
        "motion-safe" => "(prefers-reduced-motion: no-preference)",
        "dark" => "(prefers-color-scheme: dark)",
        "light" => "(prefers-color-scheme: light)",
        "portrait" => "(orientation: portrait)",
        "landscape" => "(orientation: landscape)",
        "contrast-more" => "(prefers-contrast: more)",
        "contrast-less" => "(prefers-contrast: less)",
        "forced-colors" => "(forced-colors: active)",
        "max-sm" => "(max-width: 639px)",
        "max-md" => "(max-width: 767px)",
        "max-lg" => "(max-width: 1023px)",
        "max-xl" => "(max-width: 1279px)",
        "max-2xl" => "(max-width: 1535px)",
        "print" => "print",
        _ => {
            let raw = v.strip_prefix("supports-[")?.strip_suffix(']')?;
            if !is_safe_css_fragment(raw) {
                return None;
            }
            return Some(format!("@supports ({})", raw.replace('_', " ")));
        }
    };
    Some(fixed.to_string())
}

fn variant_media_rank(v: &str) -> Option<u8> {
    match v {
        "sm" => Some(0),
        "md" => Some(1),
        "lg" => Some(2),
        "xl" => Some(3),
        "2xl" => Some(4),
        _ => None,
    }
}

fn variant_pseudo(v: &str) -> Option<&'static str> {
    match v {
        "hover" => Some(":hover"),
        "focus" => Some(":focus"),
        "focus-visible" => Some(":focus-visible"),
        "focus-within" => Some(":focus-within"),
        "active" => Some(":active"),
        "disabled" => Some(":disabled"),
        "last" => Some(":last-child"),
        "first" => Some(":first-child"),
        "only" => Some(":only-child"),
        "first-of-type" => Some(":first-of-type"),
        "last-of-type" => Some(":last-of-type"),
        "only-of-type" => Some(":only-of-type"),
        "odd" => Some(":nth-child(odd)"),
        "even" => Some(":nth-child(even)"),
        "checked" => Some(":checked"),
        "indeterminate" => Some(":indeterminate"),
        "default" => Some(":default"),
        "enabled" => Some(":enabled"),
        "optional" => Some(":optional"),
        "in-range" => Some(":in-range"),
        "out-of-range" => Some(":out-of-range"),
        "placeholder-shown" => Some(":placeholder-shown"),
        "autofill" => Some(":autofill"),
        "open" => Some("[open]"),
        "target" => Some(":target"),
        "visited" => Some(":visited"),
        "empty" => Some(":empty"),
        "required" => Some(":required"),
        "valid" => Some(":valid"),
        "invalid" => Some(":invalid"),
        "read-only" => Some(":read-only"),
        "not-last" => Some(":not(:last-child)"),
        "not-first" => Some(":not(:first-child)"),
        "before" => Some("::before"),
        "after" => Some("::after"),
        "placeholder" => Some("::placeholder"),
        "selection" => Some("::selection"),
        "marker" => Some("::marker"),
        "first-letter" => Some("::first-letter"),
        "first-line" => Some("::first-line"),
        "backdrop" => Some("::backdrop"),
        "file" => Some("::file-selector-button"),
        _ => None,
    }
}

fn apply_relational_variant(selector: &mut String, variant: &str) -> bool {
    if let Some(state) = variant.strip_prefix("group-") {
        if let Some(pseudo) = variant_pseudo(state) {
            *selector = format!(".group{} {}", pseudo, selector);
            return true;
        }
    }
    if let Some(state) = variant.strip_prefix("peer-") {
        if let Some(pseudo) = variant_pseudo(state) {
            *selector = format!(".peer{} ~ {}", pseudo, selector);
            return true;
        }
    }
    if let Some(raw) = variant
        .strip_prefix("has-[")
        .and_then(|value| value.strip_suffix(']'))
    {
        if !is_safe_css_fragment(raw) {
            return false;
        }
        selector.push_str(&format!(":has({})", raw.replace('_', " ")));
        return true;
    }
    false
}

fn apply_attribute_variant(selector: &mut String, variant: &str) -> bool {
    if let Some(name) = variant.strip_prefix("aria-") {
        if let Some(raw) = name
            .strip_prefix('[')
            .and_then(|value| value.strip_suffix(']'))
        {
            if let Some((attribute, value)) = raw.split_once('=') {
                if is_safe_attribute_name(attribute) && is_safe_attribute_value(value) {
                    selector.push_str(&format!("[aria-{}=\"{}\"]", attribute, value));
                    return true;
                }
            }
        }
        if is_safe_attribute_name(name) {
            selector.push_str(&format!("[aria-{}=\"true\"]", name));
            return true;
        }
    }
    if let Some(raw) = variant
        .strip_prefix("data-[")
        .and_then(|value| value.strip_suffix(']'))
    {
        if let Some((name, value)) = raw.split_once('=') {
            if is_safe_attribute_name(name) && is_safe_attribute_value(value) {
                selector.push_str(&format!("[data-{}=\"{}\"]", name, value));
                return true;
            }
        } else if is_safe_attribute_name(raw) {
            selector.push_str(&format!("[data-{}]", raw));
            return true;
        }
    }
    false
}

fn is_safe_attribute_value(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.'))
}

fn is_safe_css_fragment(value: &str) -> bool {
    !value.is_empty()
        && !value
            .chars()
            .any(|ch| matches!(ch, '{' | '}' | ';' | '@' | '"' | '\'' | '\\'))
        && value.chars().all(|ch| {
            ch.is_ascii_alphanumeric()
                || matches!(
                    ch,
                    '-' | '_'
                        | ':'
                        | '.'
                        | '#'
                        | '['
                        | ']'
                        | '('
                        | ')'
                        | '='
                        | '>'
                        | '+'
                        | '~'
                        | '*'
                        | ' '
                )
        })
}

fn is_safe_attribute_name(value: &str) -> bool {
    !value.is_empty()
        && value
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
}

fn utility_rule(utility: &str, base_selector: &str, important: bool) -> Option<UtilityRule> {
    if utility == "container" {
        let prelude = [
            ("640px", "640px"),
            ("768px", "768px"),
            ("1024px", "1024px"),
            ("1280px", "1280px"),
            ("1536px", "1536px"),
        ]
        .into_iter()
        .map(|(screen, width)| {
            format!(
                "@media (min-width: {}){{{}{{max-width:{};}}}}",
                screen, base_selector, width
            )
        })
        .collect::<String>();
        return Some(UtilityRule {
            selector: base_selector.to_string(),
            declarations: maybe_important("width:100%;", important),
            prelude: Some(prelude),
        });
    }
    if let Some(v) = utility.strip_prefix("aspect-") {
        let value = match v {
            "auto" => "auto".to_string(),
            "square" => "1 / 1".to_string(),
            "video" => "16 / 9".to_string(),
            _ => arbitrary_value(v)?,
        };
        return Some(simple_rule(
            base_selector,
            &format!("aspect-ratio:{};", value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("columns-") {
        let value = match v {
            "auto" => "auto".to_string(),
            "3xs" => "16rem".to_string(),
            "2xs" => "18rem".to_string(),
            "xs" => "20rem".to_string(),
            "sm" => "24rem".to_string(),
            "md" => "28rem".to_string(),
            "lg" => "32rem".to_string(),
            "xl" => "36rem".to_string(),
            "2xl" => "42rem".to_string(),
            _ => v
                .parse::<u32>()
                .ok()
                .filter(|n| *n > 0)
                .map(|n| n.to_string())
                .or_else(|| arbitrary_value(v))?,
        };
        return Some(simple_rule(
            base_selector,
            &format!("columns:{};", value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("basis-") {
        let value = size_value(v, SizeAxis::Width)?;
        return Some(simple_rule(
            base_selector,
            &format!("flex-basis:{};", value),
            important,
        ));
    }
    if let Some(declarations) = fragmentation_rule(utility) {
        return Some(simple_rule(base_selector, &declarations, important));
    }
    if let Some(v) = utility.strip_prefix("gap-x-") {
        let value = spacing_value(v)?;
        return Some(simple_rule(
            base_selector,
            &format!("column-gap:{};", value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("gap-y-") {
        let value = spacing_value(v)?;
        return Some(simple_rule(
            base_selector,
            &format!("row-gap:{};", value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("object-") {
        let position = match v {
            "bottom" => "bottom",
            "center" => "center",
            "left" => "left",
            "left-bottom" => "left bottom",
            "left-top" => "left top",
            "right" => "right",
            "right-bottom" => "right bottom",
            "right-top" => "right top",
            "top" => "top",
            _ => "",
        };
        if !position.is_empty() {
            return Some(simple_rule(
                base_selector,
                &format!("object-position:{};", position),
                important,
            ));
        }
    }
    if let Some(declarations) = background_layout_rule(utility) {
        return Some(simple_rule(base_selector, &declarations, important));
    }
    if let Some(declarations) = table_layout_rule(utility) {
        return Some(simple_rule(base_selector, &declarations, important));
    }
    if let Some(declarations) = svg_rule(utility) {
        return Some(simple_rule(base_selector, &declarations, important));
    }
    if let Some(declarations) = scroll_rule(utility) {
        return Some(simple_rule(base_selector, &declarations, important));
    }
    if let Some(declarations) = common_misc_rule(utility) {
        return Some(simple_rule(base_selector, &declarations, important));
    }
    if let Some(declarations) = transform_rule(utility) {
        return Some(simple_rule(base_selector, &declarations, important));
    }
    if let Some(declarations) = filter_rule(utility, false) {
        return Some(simple_rule(base_selector, &declarations, important));
    }
    if let Some(declarations) = filter_rule(utility, true) {
        return Some(simple_rule(base_selector, &declarations, important));
    }
    if let Some(declarations) = shadow_rule(utility) {
        return Some(simple_rule(base_selector, &declarations, important));
    }
    if let Some(v) = utility.strip_prefix("brightness-") {
        let amount = v.parse::<f64>().ok()?;
        return Some(simple_rule(
            base_selector,
            &format!("filter:brightness({:.3});", amount / 100.0),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("line-clamp-") {
        let lines = v.parse::<u32>().ok()?;
        if lines == 0 {
            return None;
        }
        return Some(simple_rule(
            base_selector,
            &format!(
                "overflow:hidden;display:-webkit-box;-webkit-box-orient:vertical;-webkit-line-clamp:{};",
                lines
            ),
            important,
        ));
    }
    if utility == "space-y-reverse" {
        return Some(UtilityRule {
            selector: format!("{} > :not([hidden]) ~ :not([hidden])", base_selector),
            declarations: maybe_important("--tw-space-y-reverse:1;", important),
            prelude: None,
        });
    }
    if utility == "space-x-reverse" {
        return Some(UtilityRule {
            selector: format!("{} > :not([hidden]) ~ :not([hidden])", base_selector),
            declarations: maybe_important("--tw-space-x-reverse:1;", important),
            prelude: None,
        });
    }
    if let Some(v) = utility.strip_prefix("space-y-") {
        let value = spacing_value(v)?;
        return Some(UtilityRule {
            selector: format!("{} > :not([hidden]) ~ :not([hidden])", base_selector),
            declarations: maybe_important(
                &format!(
                    "--tw-space-y-reverse:0;margin-top:calc({0} * calc(1 - var(--tw-space-y-reverse)));margin-bottom:calc({0} * var(--tw-space-y-reverse));",
                    value
                ),
                important,
            ),
            prelude: None,
        });
    }
    if let Some(v) = utility.strip_prefix("space-x-") {
        let value = spacing_value(v)?;
        return Some(UtilityRule {
            selector: format!("{} > :not([hidden]) ~ :not([hidden])", base_selector),
            declarations: maybe_important(
                &format!(
                    "--tw-space-x-reverse:0;margin-right:calc({0} * var(--tw-space-x-reverse));margin-left:calc({0} * calc(1 - var(--tw-space-x-reverse)));",
                    value
                ),
                important,
            ),
            prelude: None,
        });
    }
    if let Some(v) = utility.strip_prefix("grid-cols-") {
        if v == "subgrid" {
            return Some(simple_rule(
                base_selector,
                "grid-template-columns:subgrid;",
                important,
            ));
        }
        if v == "none" {
            return Some(simple_rule(
                base_selector,
                "grid-template-columns:none;",
                important,
            ));
        }
        if let Some(raw) = arbitrary_value(v) {
            return Some(simple_rule(
                base_selector,
                &format!("grid-template-columns:{};", raw),
                important,
            ));
        }
        let n = v.parse::<u32>().ok()?;
        if n == 0 {
            return None;
        }
        return Some(simple_rule(
            base_selector,
            &format!("grid-template-columns:repeat({}, minmax(0, 1fr));", n),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("col-span-") {
        if v == "full" {
            return Some(simple_rule(base_selector, "grid-column:1 / -1;", important));
        }
        let n = v.parse::<u32>().ok()?;
        if n == 0 {
            return None;
        }
        return Some(simple_rule(
            base_selector,
            &format!("grid-column:span {} / span {};", n, n),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("w-") {
        let value = size_value(v, SizeAxis::Width)?;
        return Some(simple_rule(
            base_selector,
            &format!("width:{};", value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("h-") {
        let value = size_value(v, SizeAxis::Height)?;
        return Some(simple_rule(
            base_selector,
            &format!("height:{};", value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("min-w-") {
        let value = minmax_size_value(v, SizeAxis::Width)?;
        return Some(simple_rule(
            base_selector,
            &format!("min-width:{};", value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("max-w-") {
        let value = max_width_value(v)?;
        return Some(simple_rule(
            base_selector,
            &format!("max-width:{};", value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("min-h-") {
        let value = minmax_size_value(v, SizeAxis::Height)?;
        return Some(simple_rule(
            base_selector,
            &format!("min-height:{};", value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("max-h-") {
        let value = minmax_size_value(v, SizeAxis::Height)?;
        return Some(simple_rule(
            base_selector,
            &format!("max-height:{};", value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("size-") {
        let value = size_value(v, SizeAxis::Width)?;
        return Some(simple_rule(
            base_selector,
            &format!("width:{};height:{};", value, value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("translate-x-") {
        let value = inset_value(v)?;
        return Some(simple_rule(
            base_selector,
            &format!("transform:translateX({});", value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("-translate-x-") {
        let value = inset_value(v)?;
        let neg = negate_css_value(&value)?;
        return Some(simple_rule(
            base_selector,
            &format!("transform:translateX({});", neg),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("translate-y-") {
        let value = inset_value(v)?;
        return Some(simple_rule(
            base_selector,
            &format!("transform:translateY({});", value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("-translate-y-") {
        let value = inset_value(v)?;
        let neg = negate_css_value(&value)?;
        return Some(simple_rule(
            base_selector,
            &format!("transform:translateY({});", neg),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("-m-") {
        let value = spacing_value(v)?;
        let neg = negate_css_value(&value)?;
        return Some(simple_rule(
            base_selector,
            &format!("margin:{};", neg),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("p-") {
        let value = spacing_value(v)?;
        return Some(simple_rule(
            base_selector,
            &format!("padding:{};", value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("px-") {
        let value = spacing_value(v)?;
        return Some(simple_rule(
            base_selector,
            &format!("padding-left:{};padding-right:{};", value, value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("py-") {
        let value = spacing_value(v)?;
        return Some(simple_rule(
            base_selector,
            &format!("padding-top:{};padding-bottom:{};", value, value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("pt-") {
        let value = spacing_value(v)?;
        return Some(simple_rule(
            base_selector,
            &format!("padding-top:{};", value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("pr-") {
        let value = spacing_value(v)?;
        return Some(simple_rule(
            base_selector,
            &format!("padding-right:{};", value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("pb-") {
        let value = spacing_value(v)?;
        return Some(simple_rule(
            base_selector,
            &format!("padding-bottom:{};", value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("pl-") {
        let value = spacing_value(v)?;
        return Some(simple_rule(
            base_selector,
            &format!("padding-left:{};", value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("m-") {
        if v == "auto" {
            return Some(simple_rule(base_selector, "margin:auto;", important));
        }
        let value = spacing_value(v)?;
        return Some(simple_rule(
            base_selector,
            &format!("margin:{};", value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("-mx-") {
        let value = spacing_value(v)?;
        let neg = negate_css_value(&value)?;
        return Some(simple_rule(
            base_selector,
            &format!("margin-left:{};margin-right:{};", neg, neg),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("mx-") {
        if v == "auto" {
            return Some(simple_rule(
                base_selector,
                "margin-left:auto;margin-right:auto;",
                important,
            ));
        }
        let value = spacing_value(v)?;
        return Some(simple_rule(
            base_selector,
            &format!("margin-left:{};margin-right:{};", value, value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("-my-") {
        let value = spacing_value(v)?;
        let neg = negate_css_value(&value)?;
        return Some(simple_rule(
            base_selector,
            &format!("margin-top:{};margin-bottom:{};", neg, neg),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("my-") {
        if v == "auto" {
            return Some(simple_rule(
                base_selector,
                "margin-top:auto;margin-bottom:auto;",
                important,
            ));
        }
        let value = spacing_value(v)?;
        return Some(simple_rule(
            base_selector,
            &format!("margin-top:{};margin-bottom:{};", value, value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("-mt-") {
        let value = spacing_value(v)?;
        let neg = negate_css_value(&value)?;
        return Some(simple_rule(
            base_selector,
            &format!("margin-top:{};", neg),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("mt-") {
        if v == "auto" {
            return Some(simple_rule(base_selector, "margin-top:auto;", important));
        }
        let value = spacing_value(v)?;
        return Some(simple_rule(
            base_selector,
            &format!("margin-top:{};", value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("-mr-") {
        let value = spacing_value(v)?;
        let neg = negate_css_value(&value)?;
        return Some(simple_rule(
            base_selector,
            &format!("margin-right:{};", neg),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("mr-") {
        if v == "auto" {
            return Some(simple_rule(base_selector, "margin-right:auto;", important));
        }
        let value = spacing_value(v)?;
        return Some(simple_rule(
            base_selector,
            &format!("margin-right:{};", value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("-mb-") {
        let value = spacing_value(v)?;
        let neg = negate_css_value(&value)?;
        return Some(simple_rule(
            base_selector,
            &format!("margin-bottom:{};", neg),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("mb-") {
        if v == "auto" {
            return Some(simple_rule(base_selector, "margin-bottom:auto;", important));
        }
        let value = spacing_value(v)?;
        return Some(simple_rule(
            base_selector,
            &format!("margin-bottom:{};", value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("-ml-") {
        let value = spacing_value(v)?;
        let neg = negate_css_value(&value)?;
        return Some(simple_rule(
            base_selector,
            &format!("margin-left:{};", neg),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("ml-") {
        if v == "auto" {
            return Some(simple_rule(base_selector, "margin-left:auto;", important));
        }
        let value = spacing_value(v)?;
        return Some(simple_rule(
            base_selector,
            &format!("margin-left:{};", value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("gap-") {
        let value = spacing_value(v)?;
        return Some(simple_rule(
            base_selector,
            &format!("gap:{};", value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("-top-") {
        let value = inset_value(v)?;
        let neg = negate_css_value(&value)?;
        return Some(simple_rule(
            base_selector,
            &format!("top:{};", neg),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("top-") {
        let value = inset_value(v)?;
        return Some(simple_rule(
            base_selector,
            &format!("top:{};", value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("-right-") {
        let value = inset_value(v)?;
        let neg = negate_css_value(&value)?;
        return Some(simple_rule(
            base_selector,
            &format!("right:{};", neg),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("right-") {
        let value = inset_value(v)?;
        return Some(simple_rule(
            base_selector,
            &format!("right:{};", value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("-bottom-") {
        let value = inset_value(v)?;
        let neg = negate_css_value(&value)?;
        return Some(simple_rule(
            base_selector,
            &format!("bottom:{};", neg),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("bottom-") {
        let value = inset_value(v)?;
        return Some(simple_rule(
            base_selector,
            &format!("bottom:{};", value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("-left-") {
        let value = inset_value(v)?;
        let neg = negate_css_value(&value)?;
        return Some(simple_rule(
            base_selector,
            &format!("left:{};", neg),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("left-") {
        let value = inset_value(v)?;
        return Some(simple_rule(
            base_selector,
            &format!("left:{};", value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("-inset-") {
        let value = inset_value(v)?;
        let neg = negate_css_value(&value)?;
        return Some(simple_rule(
            base_selector,
            &format!("inset:{};", neg),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("inset-y-") {
        let value = inset_value(v)?;
        return Some(simple_rule(
            base_selector,
            &format!("top:{};bottom:{};", value, value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("inset-x-") {
        let value = inset_value(v)?;
        return Some(simple_rule(
            base_selector,
            &format!("left:{};right:{};", value, value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("inset-") {
        let value = inset_value(v)?;
        return Some(simple_rule(
            base_selector,
            &format!("inset:{};", value),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("-z-") {
        if let Some(raw) = arbitrary_value(v) {
            let neg = negate_css_value(&raw)?;
            return Some(simple_rule(
                base_selector,
                &format!("z-index:{};", neg),
                important,
            ));
        }
        let z = v.parse::<i64>().ok()?;
        return Some(simple_rule(
            base_selector,
            &format!("z-index:{};", -z),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("z-") {
        if v == "auto" {
            return Some(simple_rule(base_selector, "z-index:auto;", important));
        }
        if let Some(raw) = arbitrary_value(v) {
            return Some(simple_rule(
                base_selector,
                &format!("z-index:{};", raw),
                important,
            ));
        }
        let z = v.parse::<i64>().ok()?;
        return Some(simple_rule(
            base_selector,
            &format!("z-index:{};", z),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("opacity-") {
        let num = v.parse::<u32>().ok()?.min(100);
        let alpha = (num as f64) / 100.0;
        return Some(simple_rule(
            base_selector,
            &format!("opacity:{:.3};", alpha),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("duration-") {
        let ms = v.parse::<u32>().ok()?;
        return Some(simple_rule(
            base_selector,
            &format!("transition-duration:{}ms;", ms),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("text-") {
        if !matches!(v, "left" | "center" | "right" | "start" | "end" | "justify") {
            if let Some(raw) = arbitrary_value(v) {
                if is_size_like(&raw) {
                    return Some(simple_rule(
                        base_selector,
                        &format!("font-size:{};", raw),
                        important,
                    ));
                }
            }
            if let Some(size) = text_size_value(v) {
                return Some(simple_rule(
                    base_selector,
                    &format!("font-size:{};", size),
                    important,
                ));
            }
            if let Some(color) = color_value(v) {
                return Some(simple_rule(
                    base_selector,
                    &format!("color:{};", color),
                    important,
                ));
            }
        }
    }
    if let Some(v) = utility.strip_prefix("placeholder-") {
        if let Some(color) = color_value(v) {
            return Some(UtilityRule {
                selector: format!("{}::placeholder", base_selector),
                declarations: maybe_important(&format!("color:{};opacity:1;", color), important),
                prelude: None,
            });
        }
    }
    if let Some(v) = utility.strip_prefix("font-") {
        if !matches!(
            v,
            "thin"
                | "extralight"
                | "light"
                | "normal"
                | "medium"
                | "semibold"
                | "bold"
                | "extrabold"
                | "black"
        ) {
            let (var_name, fallback) = match v {
                "sans" => ("sans", "ui-sans-serif, system-ui, sans-serif"),
                "mono" => (
                    "mono",
                    "ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace",
                ),
                "serif" => ("serif", "ui-serif, Georgia, Cambria, serif"),
                _ => (v, "ui-sans-serif, system-ui, sans-serif"),
            };
            let slug = var_name
                .chars()
                .map(|c| {
                    if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                        c.to_ascii_lowercase()
                    } else {
                        '-'
                    }
                })
                .collect::<String>();
            return Some(simple_rule(
                base_selector,
                &format!("font-family:var(--font-{}, {});", slug, fallback),
                important,
            ));
        }
    }
    if let Some(v) = utility.strip_prefix("bg-") {
        if !matches!(
            v,
            "center"
                | "bottom"
                | "repeat"
                | "repeat-x"
                | "repeat-y"
                | "no-repeat"
                | "cover"
                | "contain"
                | "gradient-to-r"
                | "gradient-to-b"
                | "gradient-to-br"
        ) {
            if let Some(bg) = background_value(v) {
                let decl = background_declaration(&bg);
                return Some(simple_rule(base_selector, &decl, important));
            }
            if let Some(color) = color_value(v) {
                return Some(simple_rule(
                    base_selector,
                    &format!("background-color:{};", color),
                    important,
                ));
            }
        }
    }
    if let Some(v) = utility.strip_prefix("from-") {
        if let Some(color) = color_value(v) {
            return Some(simple_rule(
                base_selector,
                &format!(
                    "--tw-gradient-from:{} var(--tw-gradient-from-position,);--tw-gradient-to:transparent var(--tw-gradient-to-position,);--tw-gradient-stops:var(--tw-gradient-from),var(--tw-gradient-to);",
                    color
                ),
                important,
            ));
        }
    }
    if let Some(v) = utility.strip_prefix("to-") {
        if let Some(color) = color_value(v) {
            return Some(simple_rule(
                base_selector,
                &format!(
                    "--tw-gradient-to:{} var(--tw-gradient-to-position,);",
                    color
                ),
                important,
            ));
        }
    }
    if let Some(v) = utility.strip_prefix("via-") {
        if let Some(color) = color_value(v) {
            return Some(simple_rule(
                base_selector,
                &format!(
                    "--tw-gradient-to:transparent var(--tw-gradient-to-position,);--tw-gradient-stops:var(--tw-gradient-from),{} var(--tw-gradient-via-position,),var(--tw-gradient-to);",
                    color
                ),
                important,
            ));
        }
    }
    if let Some(v) = utility.strip_prefix("border-t-") {
        if let Ok(px) = v.parse::<u32>() {
            return Some(simple_rule(
                base_selector,
                &format!("border-top-width:{px}px;border-top-style:solid;"),
                important,
            ));
        }
        if let Some(raw) = v.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            return Some(simple_rule(
                base_selector,
                &format!("border-top-width:{raw};border-top-style:solid;"),
                important,
            ));
        }
        if let Some(color) = color_value(v) {
            return Some(simple_rule(
                base_selector,
                &format!("border-top-color:{};", color),
                important,
            ));
        }
        return None;
    }
    if let Some(v) = utility.strip_prefix("border-r-") {
        if let Ok(px) = v.parse::<u32>() {
            return Some(simple_rule(
                base_selector,
                &format!("border-right-width:{px}px;border-right-style:solid;"),
                important,
            ));
        }
        if let Some(raw) = v.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            return Some(simple_rule(
                base_selector,
                &format!("border-right-width:{raw};border-right-style:solid;"),
                important,
            ));
        }
        if let Some(color) = color_value(v) {
            return Some(simple_rule(
                base_selector,
                &format!("border-right-color:{};", color),
                important,
            ));
        }
        return None;
    }
    if let Some(v) = utility.strip_prefix("border-b-") {
        if let Ok(px) = v.parse::<u32>() {
            return Some(simple_rule(
                base_selector,
                &format!("border-bottom-width:{px}px;border-bottom-style:solid;"),
                important,
            ));
        }
        if let Some(raw) = v.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            return Some(simple_rule(
                base_selector,
                &format!("border-bottom-width:{raw};border-bottom-style:solid;"),
                important,
            ));
        }
        if let Some(color) = color_value(v) {
            return Some(simple_rule(
                base_selector,
                &format!("border-bottom-color:{};", color),
                important,
            ));
        }
        return None;
    }
    if let Some(v) = utility.strip_prefix("border-l-") {
        if let Ok(px) = v.parse::<u32>() {
            return Some(simple_rule(
                base_selector,
                &format!("border-left-width:{px}px;border-left-style:solid;"),
                important,
            ));
        }
        if let Some(raw) = v.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
            return Some(simple_rule(
                base_selector,
                &format!("border-left-width:{raw};border-left-style:solid;"),
                important,
            ));
        }
        if let Some(color) = color_value(v) {
            return Some(simple_rule(
                base_selector,
                &format!("border-left-color:{};", color),
                important,
            ));
        }
        return None;
    }
    for (prefix, first, second) in [
        ("border-x-", "border-left", "border-right"),
        ("border-y-", "border-top", "border-bottom"),
    ] {
        if let Some(v) = utility.strip_prefix(prefix) {
            if let Some(width) = border_width_value(v) {
                return Some(simple_rule(
                    base_selector,
                    &format!(
                        "{first}-width:{width};{second}-width:{width};{first}-style:solid;{second}-style:solid;"
                    ),
                    important,
                ));
            }
            if let Some(color) = color_value(v) {
                return Some(simple_rule(
                    base_selector,
                    &format!("{first}-color:{color};{second}-color:{color};"),
                    important,
                ));
            }
            return None;
        }
    }
    if let Some(v) = utility.strip_prefix("border-") {
        if let Some(decl) = border_rule(v) {
            return Some(simple_rule(base_selector, &decl, important));
        }
    }
    if let Some(v) = utility.strip_prefix("outline-") {
        if let Some(offset) = v.strip_prefix("offset-") {
            let value = if let Ok(px) = offset.parse::<u32>() {
                format!("{}px", px)
            } else {
                arbitrary_value(offset)?
            };
            return Some(simple_rule(
                base_selector,
                &format!("outline-offset:{};", value),
                important,
            ));
        }
        if let Some(decl) = outline_rule(v) {
            return Some(simple_rule(base_selector, &decl, important));
        }
    }
    if let Some(v) = utility.strip_prefix("ring-") {
        if let Some(decl) = composable_ring_rule(v) {
            return Some(simple_rule(base_selector, &decl, important));
        }
    }
    if utility == "ring" {
        return Some(simple_rule(
            base_selector,
            &ring_width_declaration("3px"),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("rounded-") {
        if let Some(declarations) = directional_radius_rule(v) {
            return Some(simple_rule(base_selector, &declarations, important));
        }
        if v == "none" {
            return Some(simple_rule(base_selector, "border-radius:0;", important));
        }
        if v == "xs" {
            return Some(simple_rule(
                base_selector,
                "border-radius:0.125rem;",
                important,
            ));
        }
        if v == "sm" {
            return Some(simple_rule(
                base_selector,
                "border-radius:0.25rem;",
                important,
            ));
        }
        if v == "md" {
            return Some(simple_rule(
                base_selector,
                "border-radius:0.375rem;",
                important,
            ));
        }
        if v == "lg" {
            return Some(simple_rule(
                base_selector,
                "border-radius:0.5rem;",
                important,
            ));
        }
        if v == "xl" {
            return Some(simple_rule(
                base_selector,
                "border-radius:0.75rem;",
                important,
            ));
        }
        if v == "2xl" {
            return Some(simple_rule(base_selector, "border-radius:1rem;", important));
        }
        if v == "3xl" {
            return Some(simple_rule(
                base_selector,
                "border-radius:1.5rem;",
                important,
            ));
        }
        if v == "4xl" {
            return Some(simple_rule(base_selector, "border-radius:2rem;", important));
        }
        if let Some(raw) = arbitrary_value(v) {
            return Some(simple_rule(
                base_selector,
                &format!("border-radius:{};", raw),
                important,
            ));
        }
    }
    if let Some(v) = utility.strip_prefix("divide-x-") {
        let value = border_width_value(v)?;
        return Some(UtilityRule {
            selector: format!("{} > :not([hidden]) ~ :not([hidden])", base_selector),
            declarations: maybe_important(
                &format!("border-left-width:{};border-left-style:solid;", value),
                important,
            ),
            prelude: None,
        });
    }
    if utility == "divide-x" {
        return Some(UtilityRule {
            selector: format!("{} > :not([hidden]) ~ :not([hidden])", base_selector),
            declarations: maybe_important(
                "border-left-width:1px;border-left-style:solid;",
                important,
            ),
            prelude: None,
        });
    }
    if let Some(v) = utility.strip_prefix("divide-y-") {
        let value = border_width_value(v)?;
        return Some(UtilityRule {
            selector: format!("{} > :not([hidden]) ~ :not([hidden])", base_selector),
            declarations: maybe_important(
                &format!("border-top-width:{};border-top-style:solid;", value),
                important,
            ),
            prelude: None,
        });
    }
    if utility == "divide-y" {
        return Some(UtilityRule {
            selector: format!("{} > :not([hidden]) ~ :not([hidden])", base_selector),
            declarations: maybe_important(
                "border-top-width:1px;border-top-style:solid;",
                important,
            ),
            prelude: None,
        });
    }
    if let Some(v) = utility.strip_prefix("divide-") {
        if matches!(v, "solid" | "dashed" | "dotted" | "double" | "none") {
            return Some(UtilityRule {
                selector: format!("{} > :not([hidden]) ~ :not([hidden])", base_selector),
                declarations: maybe_important(&format!("border-style:{};", v), important),
                prelude: None,
            });
        }
        if let Some(color) = color_value(v) {
            return Some(UtilityRule {
                selector: format!("{} > :not([hidden]) ~ :not([hidden])", base_selector),
                declarations: maybe_important(&format!("border-color:{};", color), important),
                prelude: None,
            });
        }
    }
    if let Some(v) = utility.strip_prefix("order-") {
        let order = match v {
            "first" => i32::MIN,
            "last" => i32::MAX,
            "none" => 0,
            _ => v.parse::<i32>().ok()?,
        };
        return Some(simple_rule(
            base_selector,
            &format!("order:{};", order),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("shrink-") {
        let shrink = v.parse::<i32>().ok()?;
        return Some(simple_rule(
            base_selector,
            &format!("flex-shrink:{};", shrink),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("flex-shrink-") {
        let shrink = v.parse::<i32>().ok()?;
        return Some(simple_rule(
            base_selector,
            &format!("flex-shrink:{};", shrink),
            important,
        ));
    }
    // leading-N: numeric scale (N * 0.25rem), e.g. leading-6 → 1.5rem
    if let Some(v) = utility.strip_prefix("leading-") {
        if let Some(value) = match v {
            "none" => Some("1"),
            "tight" => Some("1.25"),
            "snug" => Some("1.375"),
            "normal" => Some("1.5"),
            "relaxed" => Some("1.625"),
            "loose" => Some("2"),
            _ => None,
        } {
            return Some(simple_rule(
                base_selector,
                &format!("line-height:{};", value),
                important,
            ));
        }
        if let Ok(n) = v.parse::<u64>() {
            let hundredths = n * 25;
            let whole = hundredths / 100;
            let frac = hundredths % 100;
            let val = if frac == 0 {
                format!("{}rem", whole)
            } else if frac % 10 == 0 {
                format!("{}.{}rem", whole, frac / 10)
            } else {
                format!("{}.{:02}rem", whole, frac)
            };
            return Some(simple_rule(
                base_selector,
                &format!("line-height:{};", val),
                important,
            ));
        }
        if let Some(raw) = arbitrary_value(v) {
            return Some(simple_rule(
                base_selector,
                &format!("line-height:{};", raw),
                important,
            ));
        }
    }
    // cursor-{value}
    if let Some(v) = utility.strip_prefix("cursor-") {
        let valid = matches!(
            v,
            "auto"
                | "default"
                | "pointer"
                | "wait"
                | "text"
                | "move"
                | "help"
                | "not-allowed"
                | "none"
                | "context-menu"
                | "progress"
                | "cell"
                | "crosshair"
                | "vertical-text"
                | "alias"
                | "copy"
                | "no-drop"
                | "grab"
                | "grabbing"
                | "all-scroll"
                | "col-resize"
                | "row-resize"
                | "n-resize"
                | "e-resize"
                | "s-resize"
                | "w-resize"
                | "ne-resize"
                | "nw-resize"
                | "se-resize"
                | "sw-resize"
                | "ew-resize"
                | "ns-resize"
                | "nesw-resize"
                | "nwse-resize"
                | "zoom-in"
                | "zoom-out"
        );
        if valid {
            return Some(simple_rule(
                base_selector,
                &format!("cursor:{};", v),
                important,
            ));
        }
    }
    // accent-{color}
    if let Some(v) = utility.strip_prefix("accent-") {
        if v == "auto" {
            return Some(simple_rule(base_selector, "accent-color:auto;", important));
        }
        if let Some(c) = color_value(v) {
            return Some(simple_rule(
                base_selector,
                &format!("accent-color:{};", c),
                important,
            ));
        }
    }
    // caret-{color}
    if let Some(v) = utility.strip_prefix("caret-") {
        if let Some(c) = color_value(v) {
            return Some(simple_rule(
                base_selector,
                &format!("caret-color:{};", c),
                important,
            ));
        }
        if let Some(raw) = arbitrary_value(v) {
            return Some(simple_rule(
                base_selector,
                &format!("caret-color:{};", raw),
                important,
            ));
        }
    }
    // underline-offset-{n|auto|arbitrary}
    if let Some(v) = utility.strip_prefix("underline-offset-") {
        if v == "auto" {
            return Some(simple_rule(
                base_selector,
                "text-underline-offset:auto;",
                important,
            ));
        }
        if let Ok(n) = v.parse::<u32>() {
            return Some(simple_rule(
                base_selector,
                &format!("text-underline-offset:{}px;", n),
                important,
            ));
        }
        if let Some(raw) = arbitrary_value(v) {
            return Some(simple_rule(
                base_selector,
                &format!("text-underline-offset:{};", raw),
                important,
            ));
        }
    }
    // decoration-{style|thickness|color}
    if let Some(v) = utility.strip_prefix("decoration-") {
        match v {
            "solid" => {
                return Some(simple_rule(
                    base_selector,
                    "text-decoration-style:solid;",
                    important,
                ));
            }
            "double" => {
                return Some(simple_rule(
                    base_selector,
                    "text-decoration-style:double;",
                    important,
                ));
            }
            "dotted" => {
                return Some(simple_rule(
                    base_selector,
                    "text-decoration-style:dotted;",
                    important,
                ));
            }
            "dashed" => {
                return Some(simple_rule(
                    base_selector,
                    "text-decoration-style:dashed;",
                    important,
                ));
            }
            "wavy" => {
                return Some(simple_rule(
                    base_selector,
                    "text-decoration-style:wavy;",
                    important,
                ));
            }
            "auto" => {
                return Some(simple_rule(
                    base_selector,
                    "text-decoration-thickness:auto;",
                    important,
                ));
            }
            "from-font" => {
                return Some(simple_rule(
                    base_selector,
                    "text-decoration-thickness:from-font;",
                    important,
                ));
            }
            _ => {}
        }
        if let Ok(n) = v.parse::<u32>() {
            return Some(simple_rule(
                base_selector,
                &format!("text-decoration-thickness:{}px;", n),
                important,
            ));
        }
        if let Some(c) = color_value(v) {
            return Some(simple_rule(
                base_selector,
                &format!("text-decoration-color:{};", c),
                important,
            ));
        }
        if let Some(raw) = arbitrary_value(v) {
            return Some(simple_rule(
                base_selector,
                &format!("text-decoration-color:{};", raw),
                important,
            ));
        }
    }
    // content-{value}
    if let Some(v) = utility.strip_prefix("content-") {
        if let Some(raw) = arbitrary_value(v) {
            let inner = raw.trim_matches('\'');
            return Some(simple_rule(
                base_selector,
                &format!("content:'{}';", inner),
                important,
            ));
        }
        match v {
            "none" => return Some(simple_rule(base_selector, "content:none;", important)),
            "normal" => {
                return Some(simple_rule(
                    base_selector,
                    "align-content:normal;",
                    important,
                ));
            }
            "start" => {
                return Some(simple_rule(
                    base_selector,
                    "align-content:start;",
                    important,
                ));
            }
            "end" => return Some(simple_rule(base_selector, "align-content:end;", important)),
            "center" => {
                return Some(simple_rule(
                    base_selector,
                    "align-content:center;",
                    important,
                ));
            }
            "between" => {
                return Some(simple_rule(
                    base_selector,
                    "align-content:space-between;",
                    important,
                ));
            }
            "around" => {
                return Some(simple_rule(
                    base_selector,
                    "align-content:space-around;",
                    important,
                ));
            }
            "evenly" => {
                return Some(simple_rule(
                    base_selector,
                    "align-content:space-evenly;",
                    important,
                ));
            }
            "stretch" => {
                return Some(simple_rule(
                    base_selector,
                    "align-content:stretch;",
                    important,
                ));
            }
            "baseline" => {
                return Some(simple_rule(
                    base_selector,
                    "align-content:baseline;",
                    important,
                ));
            }
            _ => {}
        }
    }
    // scrollbar-{value}
    if let Some(v) = utility.strip_prefix("scrollbar-") {
        match v {
            "none" => {
                return Some(simple_rule(
                    base_selector,
                    "scrollbar-width:none;",
                    important,
                ));
            }
            "thin" => {
                return Some(simple_rule(
                    base_selector,
                    "scrollbar-width:thin;",
                    important,
                ));
            }
            "auto" => {
                return Some(simple_rule(
                    base_selector,
                    "scrollbar-width:auto;",
                    important,
                ));
            }
            "stable" => {
                return Some(simple_rule(
                    base_selector,
                    "scrollbar-gutter:stable;",
                    important,
                ));
            }
            "stable-both" => {
                return Some(simple_rule(
                    base_selector,
                    "scrollbar-gutter:stable both-edges;",
                    important,
                ));
            }
            _ => {}
        }
    }
    // grid-rows-{n|arbitrary}
    if let Some(v) = utility.strip_prefix("grid-rows-") {
        if v == "subgrid" {
            return Some(simple_rule(
                base_selector,
                "grid-template-rows:subgrid;",
                important,
            ));
        }
        if v == "none" {
            return Some(simple_rule(
                base_selector,
                "grid-template-rows:none;",
                important,
            ));
        }
        if let Ok(n) = v.parse::<u32>() {
            if n > 0 {
                return Some(simple_rule(
                    base_selector,
                    &format!("grid-template-rows:repeat({}, minmax(0, 1fr));", n),
                    important,
                ));
            }
        }
        if let Some(raw) = arbitrary_value(v) {
            return Some(simple_rule(
                base_selector,
                &format!("grid-template-rows:{};", raw),
                important,
            ));
        }
    }
    // row-span-{n}
    if let Some(v) = utility.strip_prefix("row-span-") {
        if let Ok(n) = v.parse::<u32>() {
            if n > 0 {
                return Some(simple_rule(
                    base_selector,
                    &format!("grid-row:span {} / span {};", n, n),
                    important,
                ));
            }
        }
    }
    // col-start-{n|auto}, col-end-{n|auto}, row-start-{n|auto}, row-end-{n|auto}
    if let Some(v) = utility.strip_prefix("col-start-") {
        if v == "auto" {
            return Some(simple_rule(
                base_selector,
                "grid-column-start:auto;",
                important,
            ));
        }
        if let Ok(n) = v.parse::<i32>() {
            return Some(simple_rule(
                base_selector,
                &format!("grid-column-start:{};", n),
                important,
            ));
        }
    }
    if let Some(v) = utility.strip_prefix("col-end-") {
        if v == "auto" {
            return Some(simple_rule(
                base_selector,
                "grid-column-end:auto;",
                important,
            ));
        }
        if let Ok(n) = v.parse::<i32>() {
            return Some(simple_rule(
                base_selector,
                &format!("grid-column-end:{};", n),
                important,
            ));
        }
    }
    if let Some(v) = utility.strip_prefix("row-start-") {
        if v == "auto" {
            return Some(simple_rule(
                base_selector,
                "grid-row-start:auto;",
                important,
            ));
        }
        if let Ok(n) = v.parse::<i32>() {
            return Some(simple_rule(
                base_selector,
                &format!("grid-row-start:{};", n),
                important,
            ));
        }
    }
    if let Some(v) = utility.strip_prefix("row-end-") {
        if v == "auto" {
            return Some(simple_rule(base_selector, "grid-row-end:auto;", important));
        }
        if let Ok(n) = v.parse::<i32>() {
            return Some(simple_rule(
                base_selector,
                &format!("grid-row-end:{};", n),
                important,
            ));
        }
    }
    // ease-{value}
    if let Some(v) = utility.strip_prefix("ease-") {
        let timing = match v {
            "linear" => "linear",
            "in" => "cubic-bezier(0.4,0,1,1)",
            "out" => "cubic-bezier(0,0,0.2,1)",
            "in-out" => "cubic-bezier(0.4,0,0.2,1)",
            _ => return None,
        };
        return Some(simple_rule(
            base_selector,
            &format!("transition-timing-function:{};", timing),
            important,
        ));
    }
    // duration-{n}
    if let Some(v) = utility.strip_prefix("duration-") {
        if let Ok(n) = v.parse::<u32>() {
            return Some(simple_rule(
                base_selector,
                &format!("transition-duration:{}ms;", n),
                important,
            ));
        }
        if let Some(raw) = arbitrary_value(v) {
            return Some(simple_rule(
                base_selector,
                &format!("transition-duration:{};", raw),
                important,
            ));
        }
    }
    // delay-{n}
    if let Some(v) = utility.strip_prefix("delay-") {
        if let Ok(n) = v.parse::<u32>() {
            return Some(simple_rule(
                base_selector,
                &format!("transition-delay:{}ms;", n),
                important,
            ));
        }
        if let Some(raw) = arbitrary_value(v) {
            return Some(simple_rule(
                base_selector,
                &format!("transition-delay:{};", raw),
                important,
            ));
        }
    }
    // indent-{n}
    if let Some(v) = utility.strip_prefix("indent-") {
        if let Some(val) = spacing_value(v) {
            return Some(simple_rule(
                base_selector,
                &format!("text-indent:{};", val),
                important,
            ));
        }
        if let Some(raw) = arbitrary_value(v) {
            return Some(simple_rule(
                base_selector,
                &format!("text-indent:{};", raw),
                important,
            ));
        }
    }
    // will-change-{value}
    if let Some(v) = utility.strip_prefix("will-change-") {
        let val = match v {
            "auto" => "auto",
            "scroll" => "scroll-position",
            "contents" => "contents",
            "transform" => "transform",
            _ => return None,
        };
        return Some(simple_rule(
            base_selector,
            &format!("will-change:{};", val),
            important,
        ));
    }
    match utility {
        "flex" => Some(simple_rule(base_selector, "display:flex;", important)), "grid" => Some(simple_rule(base_selector, "display:grid;", important)), "block" => Some(simple_rule(base_selector, "display:block;", important)), "inline" => Some(simple_rule(base_selector, "display:inline;", important)), "inline-block" => Some(simple_rule(base_selector, "display:inline-block;", important)), "hidden" => Some(simple_rule(base_selector, "display:none;", important)), "flex-col" => Some(simple_rule(base_selector, "flex-direction:column;", important)), "flex-row" => Some(simple_rule(base_selector, "flex-direction:row;", important)), "flex-wrap" => Some(simple_rule(base_selector, "flex-wrap:wrap;", important)), "flex-1" => Some(simple_rule(base_selector, "flex:1 1 0%;", important)), "flex-0" => Some(simple_rule(base_selector, "flex:0 0 auto;", important)), "flex-none" => Some(simple_rule(base_selector, "flex:none;", important)), "shrink-0" => Some(simple_rule(base_selector, "flex-shrink:0;", important)), "basis-0" => Some(simple_rule(base_selector, "flex-basis:0;", important)), "items-start" => Some(simple_rule(base_selector, "align-items:flex-start;", important)), "items-center" => Some(simple_rule(base_selector, "align-items:center;", important)), "items-end" => Some(simple_rule(base_selector, "align-items:flex-end;", important)), "items-stretch" => Some(simple_rule(base_selector, "align-items:stretch;", important)), "items-baseline" => Some(simple_rule(base_selector, "align-items:baseline;", important)), "align-start" => Some(simple_rule(base_selector, "align-items:flex-start;", important)), "justify-start" => Some(simple_rule(base_selector, "justify-content:flex-start;", important)), "justify-center" => Some(simple_rule(base_selector, "justify-content:center;", important)), "justify-end" => Some(simple_rule(base_selector, "justify-content:flex-end;", important)), "justify-between" => Some(simple_rule(base_selector, "justify-content:space-between;", important)), "justify-around" => Some(simple_rule(base_selector, "justify-content:space-around;", important)), "justify-evenly" => Some(simple_rule(base_selector, "justify-content:space-evenly;", important)), "justify-stretch" => Some(simple_rule(base_selector, "justify-content:stretch;", important)), "rounded" => Some(simple_rule(base_selector, "border-radius:0.25rem;", important)), "rounded-sm" => Some(simple_rule(base_selector, "border-radius:0.25rem;", important)), "rounded-md" => Some(simple_rule(base_selector, "border-radius:0.375rem;", important)), "rounded-lg" => Some(simple_rule(base_selector, "border-radius:0.5rem;", important)), "rounded-xl" => Some(simple_rule(base_selector, "border-radius:0.75rem;", important)), "rounded-2xl" => Some(simple_rule(base_selector, "border-radius:1rem;", important)), "rounded-3xl" => Some(simple_rule(base_selector, "border-radius:1.5rem;", important)), "rounded-4xl" => Some(simple_rule(base_selector, "border-radius:2rem;", important)), "rounded-full" => Some(simple_rule(base_selector, "border-radius:9999px;", important)), "rounded-none" => Some(simple_rule(base_selector, "border-radius:0;", important)), "rounded-xs" => Some(simple_rule(base_selector, "border-radius:0.125rem;", important)), "shadow" | "shadow-sm" => Some(simple_rule(base_selector, "box-shadow:0 1px 2px rgba(0,0,0,0.05);", important)), "shadow-md" => Some(simple_rule(base_selector, "box-shadow:0 4px 12px rgba(0,0,0,0.08);", important)), "shadow-lg" => Some(simple_rule(base_selector, "box-shadow:0 12px 32px rgba(0,0,0,0.12);", important)), "shadow-2xl" => Some(simple_rule(base_selector, "box-shadow:0 20px 48px rgba(0,0,0,0.2);", important)), "shadow-xs" => Some(simple_rule(base_selector, "box-shadow:0 1px 1px rgba(0,0,0,0.04);", important)), "font-thin" => Some(simple_rule(base_selector, "font-weight:100;", important)), "font-extralight" => Some(simple_rule(base_selector, "font-weight:200;", important)), "font-light" => Some(simple_rule(base_selector, "font-weight:300;", important)), "font-normal" => Some(simple_rule(base_selector, "font-weight:400;", important)), "font-medium" => Some(simple_rule(base_selector, "font-weight:500;", important)), "font-semibold" => Some(simple_rule(base_selector, "font-weight:600;", important)), "font-bold" => Some(simple_rule(base_selector, "font-weight:700;", important)), "font-extrabold" => Some(simple_rule(base_selector, "font-weight:800;", important)), "font-black" => Some(simple_rule(base_selector, "font-weight:900;", important)), "font-mono" => Some(simple_rule(base_selector, "font-family:ui-monospace,SFMono-Regular,Menlo,Monaco,Consolas,'Liberation Mono','Courier New',monospace;", important)), "font-sans" => Some(simple_rule(base_selector, "font-family:ui-sans-serif,system-ui,-apple-system,BlinkMacSystemFont,'Segoe UI',Roboto,'Helvetica Neue',Arial,sans-serif;", important)), "font-serif" => Some(simple_rule(base_selector, "font-family:ui-serif,Georgia,Cambria,'Times New Roman',Times,serif;", important)), "italic" => Some(simple_rule(base_selector, "font-style:italic;", important)), "text-left" => Some(simple_rule(base_selector, "text-align:left;", important)), "text-center" => Some(simple_rule(base_selector, "text-align:center;", important)), "text-right" => Some(simple_rule(base_selector, "text-align:right;", important)), "text-justify" => Some(simple_rule(base_selector, "text-align:justify;", important)), "leading-none" => Some(simple_rule(base_selector, "line-height:1;", important)), "leading-tight" => Some(simple_rule(base_selector, "line-height:1.25;", important)), "leading-snug" => Some(simple_rule(base_selector, "line-height:1.375;", important)), "leading-normal" => Some(simple_rule(base_selector, "line-height:1.5;", important)), "leading-relaxed" => Some(simple_rule(base_selector, "line-height:1.625;", important)), "tracking-tight" => Some(simple_rule(base_selector, "letter-spacing:-0.025em;", important)), "tracking-normal" => Some(simple_rule(base_selector, "letter-spacing:0;", important)), "tracking-wide" => Some(simple_rule(base_selector, "letter-spacing:0.025em;", important)), "tracking-wider" => Some(simple_rule(base_selector, "letter-spacing:0.05em;", important)), "tracking-widest" => Some(simple_rule(base_selector, "letter-spacing:0.1em;", important)), "border" => Some(simple_rule(base_selector, "border-width:1px;border-style:solid;", important)), "border-0" => Some(simple_rule(base_selector, "border-width:0;", important)), "border-2" => Some(simple_rule(base_selector, "border-width:2px;border-style:solid;", important)), "border-4" => Some(simple_rule(base_selector, "border-width:4px;border-style:solid;", important)), "border-t" => Some(simple_rule(base_selector, "border-top-width:1px;border-top-style:solid;", important)), "border-r" => Some(simple_rule(base_selector, "border-right-width:1px;border-right-style:solid;", important)), "border-b" => Some(simple_rule(base_selector, "border-bottom-width:1px;border-bottom-style:solid;", important)), "border-l" => Some(simple_rule(base_selector, "border-left-width:1px;border-left-style:solid;", important)), "border-x" => Some(simple_rule(base_selector, "border-left-width:1px;border-right-width:1px;border-left-style:solid;border-right-style:solid;", important)), "border-y" => Some(simple_rule(base_selector, "border-top-width:1px;border-bottom-width:1px;border-top-style:solid;border-bottom-style:solid;", important)), "border-dashed" => Some(simple_rule(base_selector, "border-style:dashed;", important)), "border-solid" => Some(simple_rule(base_selector, "border-style:solid;", important)), "relative" => Some(simple_rule(base_selector, "position:relative;", important)), "absolute" => Some(simple_rule(base_selector, "position:absolute;", important)), "fixed" => Some(simple_rule(base_selector, "position:fixed;", important)), "sticky" => Some(simple_rule(base_selector, "position:sticky;", important)), "min-h-screen" => Some(simple_rule(base_selector, "min-height:100vh;", important)), "h-full" => Some(simple_rule(base_selector, "height:100%;", important)), "w-full" => Some(simple_rule(base_selector, "width:100%;", important)), "w-auto" => Some(simple_rule(base_selector, "width:auto;", important)), "h-auto" => Some(simple_rule(base_selector, "height:auto;", important)), "overflow-hidden" => Some(simple_rule(base_selector, "overflow:hidden;", important)), "overflow-auto" => Some(simple_rule(base_selector, "overflow:auto;", important)), "overflow-scroll" => Some(simple_rule(base_selector, "overflow:scroll;", important)), "overflow-visible" => Some(simple_rule(base_selector, "overflow:visible;", important)), "overflow-x-auto" => Some(simple_rule(base_selector, "overflow-x:auto;", important)), "overflow-y-auto" => Some(simple_rule(base_selector, "overflow-y:auto;", important)), "overflow-x-hidden" => Some(simple_rule(base_selector, "overflow-x:hidden;", important)), "overflow-y-hidden" => Some(simple_rule(base_selector, "overflow-y:hidden;", important)), "whitespace-normal" => Some(simple_rule(base_selector, "white-space:normal;", important)), "whitespace-nowrap" => Some(simple_rule(base_selector, "white-space:nowrap;", important)), "transition" => Some(simple_rule(base_selector, "transition-property:all;transition-duration:150ms;transition-timing-function:cubic-bezier(0.4,0,0.2,1);", important)), "transition-all" => Some(simple_rule(base_selector, "transition-property:all;transition-duration:150ms;transition-timing-function:cubic-bezier(0.4,0,0.2,1);", important)), "transition-colors" => Some(simple_rule(base_selector, "transition-property:background-color,border-color,color,fill,stroke;transition-duration:150ms;transition-timing-function:cubic-bezier(0.4,0,0.2,1);", important)), "transition-none" => Some(simple_rule(base_selector, "transition-property:none;", important)), "transition-opacity" => Some(simple_rule(base_selector, "transition-property:opacity;transition-duration:150ms;transition-timing-function:cubic-bezier(0.4,0,0.2,1);", important)), "transition-transform" => Some(simple_rule(base_selector, "transition-property:transform;transition-duration:150ms;transition-timing-function:cubic-bezier(0.4,0,0.2,1);", important)), "cursor-pointer" => Some(simple_rule(base_selector, "cursor:pointer;", important)), "cursor-default" => Some(simple_rule(base_selector, "cursor:default;", important)), "uppercase" => Some(simple_rule(base_selector, "text-transform:uppercase;", important)), "lowercase" => Some(simple_rule(base_selector, "text-transform:lowercase;", important)), "capitalize" => Some(simple_rule(base_selector, "text-transform:capitalize;", important)), "underline" => Some(simple_rule(base_selector, "text-decoration:underline;", important)), "inline-flex" => Some(simple_rule(base_selector, "display:inline-flex;", important)), "list-disc" => Some(simple_rule(base_selector, "list-style-type:disc;", important)), "list-inside" => Some(simple_rule(base_selector, "list-style-position:inside;", important)), "break-words" => Some(simple_rule(base_selector, "overflow-wrap:break-word;", important)), "appearance-none" => Some(simple_rule(base_selector, "appearance:none;", important)), "backdrop-blur-sm" => Some(simple_rule(base_selector, "backdrop-filter:blur(4px);", important)), "backdrop-blur" => Some(simple_rule(base_selector, "backdrop-filter:blur(8px);", important)), "backdrop-blur-md" => Some(simple_rule(base_selector, "backdrop-filter:blur(12px);", important)), "backdrop-blur-lg" => Some(simple_rule(base_selector, "backdrop-filter:blur(16px);", important)), "backdrop-blur-xl" => Some(simple_rule(base_selector, "backdrop-filter:blur(24px);", important)), "backdrop-blur-none" => Some(simple_rule(base_selector, "backdrop-filter:none;", important)), "antialiased" => Some(simple_rule(base_selector, "-webkit-font-smoothing:antialiased;-moz-osx-font-smoothing:grayscale;", important)), "pointer-events-none" => Some(simple_rule(base_selector, "pointer-events:none;", important)), "pointer-events-auto" => Some(simple_rule(base_selector, "pointer-events:auto;", important)), "select-none" => Some(simple_rule(base_selector, "user-select:none;", important)), "fill-current" => Some(simple_rule(base_selector, "fill:currentColor;", important)), "align-top" => Some(simple_rule(base_selector, "vertical-align:top;", important)), "align-middle" => Some(simple_rule(base_selector, "vertical-align:middle;", important)), "resize-y" => Some(simple_rule(base_selector, "resize:vertical;", important)), "touch-pan-y" => Some(simple_rule(base_selector, "touch-action:pan-y;", important)), "tabular-nums" => Some(simple_rule(base_selector, "font-variant-numeric:tabular-nums;", important)), "sr-only" => Some(simple_rule(base_selector, "position:absolute;width:1px;height:1px;padding:0;margin:-1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border-width:0;", important)), "prose-sm" => Some(simple_rule(base_selector, "font-size:0.875rem;line-height:1.7142857;", important)), "bg-center" => Some(simple_rule(base_selector, "background-position:center;", important)), "bg-bottom" => Some(simple_rule(base_selector, "background-position:bottom;", important)), "bg-repeat" => Some(simple_rule(base_selector, "background-repeat:repeat;", important)), "bg-repeat-x" => Some(simple_rule(base_selector, "background-repeat:repeat-x;", important)), "bg-repeat-y" => Some(simple_rule(base_selector, "background-repeat:repeat-y;", important)), "bg-no-repeat" => Some(simple_rule(base_selector, "background-repeat:no-repeat;", important)), "bg-cover" => Some(simple_rule(base_selector, "background-size:cover;", important)), "bg-contain" => Some(simple_rule(base_selector, "background-size:contain;", important)), "bg-gradient-to-r" => Some(simple_rule(base_selector, "background-image:linear-gradient(to right,var(--tw-gradient-from),var(--tw-gradient-to));", important)), "bg-gradient-to-b" => Some(simple_rule(base_selector, "background-image:linear-gradient(to bottom,var(--tw-gradient-from),var(--tw-gradient-to));", important)), "bg-gradient-to-br" => Some(simple_rule(base_selector, "background-image:linear-gradient(to bottom right,var(--tw-gradient-from),var(--tw-gradient-to));", important)), "outline-none" => Some(simple_rule(base_selector, "outline:2px solid transparent;outline-offset:2px;", important)), "outline-hidden" => Some(simple_rule(base_selector, "outline:none;", important)), "ring" => Some(simple_rule(base_selector, "box-shadow:0 0 0 1px rgba(59,130,246,0.5);", important)), "ring-1" => Some(simple_rule(base_selector, "box-shadow:0 0 0 1px rgba(59,130,246,0.5);", important)), "ring-2" => Some(simple_rule(base_selector, "box-shadow:0 0 0 2px rgba(59,130,246,0.5);", important)), "ring-4" => Some(simple_rule(base_selector, "box-shadow:0 0 0 4px rgba(59,130,246,0.5);", important)), "max-w-none" => Some(simple_rule(base_selector, "max-width:none;", important)), "w-px" => Some(simple_rule(base_selector, "width:1px;", important)), "size-full" => Some(simple_rule(base_selector, "width:100%;height:100%;", important)), "list-decimal" => Some(simple_rule(base_selector, "list-style-type:decimal;", important)), "animate-spin" => Some(UtilityRule { selector: base_selector.to_string(), declarations: maybe_important("animation:zebflow-spin 1s linear infinite;", important), prelude: Some("@keyframes zebflow-spin{to{transform:rotate(360deg);}}".to_string()) }), "animate-ping" => Some(UtilityRule { selector: base_selector.to_string(), declarations: maybe_important("animation:zebflow-ping 1s cubic-bezier(0,0,0.2,1) infinite;", important), prelude: Some("@keyframes zebflow-ping{75%,100%{transform:scale(2);opacity:0;}}".to_string()) }), "animate-pulse" => Some(UtilityRule { selector: base_selector.to_string(), declarations: maybe_important("animation:zebflow-pulse 2s cubic-bezier(0.4,0,0.6,1) infinite;", important), prelude: Some("@keyframes zebflow-pulse{0%,100%{opacity:1;}50%{opacity:.5;}}".to_string()) }), "animate-bounce" => Some(UtilityRule { selector: base_selector.to_string(), declarations: maybe_important("animation:zebflow-bounce 1s infinite;", important), prelude: Some("@keyframes zebflow-bounce{0%,100%{transform:translateY(-25%);animation-timing-function:cubic-bezier(.8,0,1,1);}50%{transform:none;animation-timing-function:cubic-bezier(0,0,.2,1);}}".to_string()) }),
        // Overflow
        "overflow-x-scroll"   => Some(simple_rule(base_selector, "overflow-x:scroll;", important)),
        "overflow-y-scroll"   => Some(simple_rule(base_selector, "overflow-y:scroll;", important)),
        "overflow-x-visible"  => Some(simple_rule(base_selector, "overflow-x:visible;", important)),
        "overflow-y-visible"  => Some(simple_rule(base_selector, "overflow-y:visible;", important)),
        "overflow-clip"       => Some(simple_rule(base_selector, "overflow:clip;", important)),
        "overflow-x-clip"     => Some(simple_rule(base_selector, "overflow-x:clip;", important)),
        "overflow-y-clip"     => Some(simple_rule(base_selector, "overflow-y:clip;", important)),
        // Display
        "contents"    => Some(simple_rule(base_selector, "display:contents;", important)),
        "table"       => Some(simple_rule(base_selector, "display:table;", important)),
        "table-cell"  => Some(simple_rule(base_selector, "display:table-cell;", important)),
        "table-row"   => Some(simple_rule(base_selector, "display:table-row;", important)),
        "flow-root"   => Some(simple_rule(base_selector, "display:flow-root;", important)),
        "list-item"   => Some(simple_rule(base_selector, "display:list-item;", important)),
        "inline-grid" => Some(simple_rule(base_selector, "display:inline-grid;", important)),
        // Whitespace
        "whitespace-pre"          => Some(simple_rule(base_selector, "white-space:pre;", important)),
        "whitespace-pre-wrap"     => Some(simple_rule(base_selector, "white-space:pre-wrap;", important)),
        "whitespace-pre-line"     => Some(simple_rule(base_selector, "white-space:pre-line;", important)),
        "whitespace-break-spaces" => Some(simple_rule(base_selector, "white-space:break-spaces;", important)),
        // Text overflow
        "text-ellipsis" => Some(simple_rule(base_selector, "text-overflow:ellipsis;", important)),
        "text-clip"     => Some(simple_rule(base_selector, "text-overflow:clip;", important)),
        "truncate"      => Some(simple_rule(base_selector, "overflow:hidden;text-overflow:ellipsis;white-space:nowrap;", important)),
        // Text wrap
        "text-wrap"    => Some(simple_rule(base_selector, "text-wrap:wrap;", important)),
        "text-nowrap"  => Some(simple_rule(base_selector, "text-wrap:nowrap;", important)),
        "text-balance" => Some(simple_rule(base_selector, "text-wrap:balance;", important)),
        "text-pretty"  => Some(simple_rule(base_selector, "text-wrap:pretty;", important)),
        // Box sizing
        "box-border"  => Some(simple_rule(base_selector, "box-sizing:border-box;", important)),
        "box-content" => Some(simple_rule(base_selector, "box-sizing:content-box;", important)),
        // Box decoration break
        "box-decoration-clone" => Some(simple_rule(base_selector, "box-decoration-break:clone;-webkit-box-decoration-break:clone;", important)),
        "box-decoration-slice" => Some(simple_rule(base_selector, "box-decoration-break:slice;-webkit-box-decoration-break:slice;", important)),
        // Isolation
        "isolate"        => Some(simple_rule(base_selector, "isolation:isolate;", important)),
        "isolation-auto" => Some(simple_rule(base_selector, "isolation:auto;", important)),
        // Object fit
        "object-contain"    => Some(simple_rule(base_selector, "object-fit:contain;", important)),
        "object-cover"      => Some(simple_rule(base_selector, "object-fit:cover;", important)),
        "object-fill"       => Some(simple_rule(base_selector, "object-fit:fill;", important)),
        "object-none"       => Some(simple_rule(base_selector, "object-fit:none;", important)),
        "object-scale-down" => Some(simple_rule(base_selector, "object-fit:scale-down;", important)),
        // Visibility
        "visible"   => Some(simple_rule(base_selector, "visibility:visible;", important)),
        "invisible" => Some(simple_rule(base_selector, "visibility:hidden;", important)),
        "collapse"  => Some(simple_rule(base_selector, "visibility:collapse;", important)),
        // Typography
        "not-italic"      => Some(simple_rule(base_selector, "font-style:normal;", important)),
        "no-underline"    => Some(simple_rule(base_selector, "text-decoration:none;", important)),
        "line-through"    => Some(simple_rule(base_selector, "text-decoration:line-through;", important)),
        "overline"        => Some(simple_rule(base_selector, "text-decoration:overline;", important)),
        "normal-nums"     => Some(simple_rule(base_selector, "font-variant-numeric:normal;", important)),
        "appearance-auto" => Some(simple_rule(base_selector, "appearance:auto;", important)),
        // User select
        "select-all"  => Some(simple_rule(base_selector, "user-select:all;", important)),
        "select-text" => Some(simple_rule(base_selector, "user-select:text;", important)),
        "select-auto" => Some(simple_rule(base_selector, "user-select:auto;", important)),
        // Touch action
        "touch-none"         => Some(simple_rule(base_selector, "touch-action:none;", important)),
        "touch-auto"         => Some(simple_rule(base_selector, "touch-action:auto;", important)),
        "touch-pan-x"        => Some(simple_rule(base_selector, "touch-action:pan-x;", important)),
        "touch-manipulation" => Some(simple_rule(base_selector, "touch-action:manipulation;", important)),
        "touch-pinch-zoom"   => Some(simple_rule(base_selector, "touch-action:pinch-zoom;", important)),
        // Resize
        "resize-none" => Some(simple_rule(base_selector, "resize:none;", important)),
        "resize-x"    => Some(simple_rule(base_selector, "resize:horizontal;", important)),
        "resize"      => Some(simple_rule(base_selector, "resize:both;", important)),
        // Vertical align
        "align-bottom"      => Some(simple_rule(base_selector, "vertical-align:bottom;", important)),
        "align-text-top"    => Some(simple_rule(base_selector, "vertical-align:text-top;", important)),
        "align-text-bottom" => Some(simple_rule(base_selector, "vertical-align:text-bottom;", important)),
        "align-sub"         => Some(simple_rule(base_selector, "vertical-align:sub;", important)),
        "align-super"       => Some(simple_rule(base_selector, "vertical-align:super;", important)),
        "align-baseline"    => Some(simple_rule(base_selector, "vertical-align:baseline;", important)),
        // Grid
        "col-span-full"       => Some(simple_rule(base_selector, "grid-column:1 / -1;", important)),
        "row-span-full"       => Some(simple_rule(base_selector, "grid-row:1 / -1;", important)),
        "col-auto"            => Some(simple_rule(base_selector, "grid-column:auto;", important)),
        "row-auto"            => Some(simple_rule(base_selector, "grid-row:auto;", important)),
        "auto-rows-auto"      => Some(simple_rule(base_selector, "grid-auto-rows:auto;", important)),
        "auto-rows-min"       => Some(simple_rule(base_selector, "grid-auto-rows:min-content;", important)),
        "auto-rows-max"       => Some(simple_rule(base_selector, "grid-auto-rows:max-content;", important)),
        "auto-rows-fr"        => Some(simple_rule(base_selector, "grid-auto-rows:minmax(0,1fr);", important)),
        "auto-cols-auto"      => Some(simple_rule(base_selector, "grid-auto-columns:auto;", important)),
        "auto-cols-min"       => Some(simple_rule(base_selector, "grid-auto-columns:min-content;", important)),
        "auto-cols-max"       => Some(simple_rule(base_selector, "grid-auto-columns:max-content;", important)),
        "auto-cols-fr"        => Some(simple_rule(base_selector, "grid-auto-columns:minmax(0,1fr);", important)),
        "grid-flow-row"       => Some(simple_rule(base_selector, "grid-auto-flow:row;", important)),
        "grid-flow-col"       => Some(simple_rule(base_selector, "grid-auto-flow:column;", important)),
        "grid-flow-dense"     => Some(simple_rule(base_selector, "grid-auto-flow:dense;", important)),
        "grid-flow-row-dense" => Some(simple_rule(base_selector, "grid-auto-flow:row dense;", important)),
        "grid-flow-col-dense" => Some(simple_rule(base_selector, "grid-auto-flow:column dense;", important)),
        // Flex additions
        "flex-auto"         => Some(simple_rule(base_selector, "flex:1 1 auto;", important)),
        "flex-row-reverse"  => Some(simple_rule(base_selector, "flex-direction:row-reverse;", important)),
        "flex-col-reverse"  => Some(simple_rule(base_selector, "flex-direction:column-reverse;", important)),
        "flex-wrap-reverse" => Some(simple_rule(base_selector, "flex-wrap:wrap-reverse;", important)),
        "flex-nowrap"       => Some(simple_rule(base_selector, "flex-wrap:nowrap;", important)),
        "grow"              => Some(simple_rule(base_selector, "flex-grow:1;", important)),
        "grow-0"            => Some(simple_rule(base_selector, "flex-grow:0;", important)),
        "shrink"            => Some(simple_rule(base_selector, "flex-shrink:1;", important)),
        // Justify / align self
        "justify-items-start"   => Some(simple_rule(base_selector, "justify-items:start;", important)),
        "justify-items-end"     => Some(simple_rule(base_selector, "justify-items:end;", important)),
        "justify-items-center"  => Some(simple_rule(base_selector, "justify-items:center;", important)),
        "justify-items-stretch" => Some(simple_rule(base_selector, "justify-items:stretch;", important)),
        "justify-self-auto"     => Some(simple_rule(base_selector, "justify-self:auto;", important)),
        "justify-self-start"    => Some(simple_rule(base_selector, "justify-self:start;", important)),
        "justify-self-end"      => Some(simple_rule(base_selector, "justify-self:end;", important)),
        "justify-self-center"   => Some(simple_rule(base_selector, "justify-self:center;", important)),
        "justify-self-stretch"  => Some(simple_rule(base_selector, "justify-self:stretch;", important)),
        "self-auto"     => Some(simple_rule(base_selector, "align-self:auto;", important)),
        "self-start"    => Some(simple_rule(base_selector, "align-self:flex-start;", important)),
        "self-end"      => Some(simple_rule(base_selector, "align-self:flex-end;", important)),
        "self-center"   => Some(simple_rule(base_selector, "align-self:center;", important)),
        "self-stretch"  => Some(simple_rule(base_selector, "align-self:stretch;", important)),
        "self-baseline" => Some(simple_rule(base_selector, "align-self:baseline;", important)),
        // Place utilities
        "place-items-start"   => Some(simple_rule(base_selector, "place-items:start;", important)),
        "place-items-end"     => Some(simple_rule(base_selector, "place-items:end;", important)),
        "place-items-center"  => Some(simple_rule(base_selector, "place-items:center;", important)),
        "place-items-stretch" => Some(simple_rule(base_selector, "place-items:stretch;", important)),
        "place-self-auto"     => Some(simple_rule(base_selector, "place-self:auto;", important)),
        "place-self-start"    => Some(simple_rule(base_selector, "place-self:start;", important)),
        "place-self-end"      => Some(simple_rule(base_selector, "place-self:end;", important)),
        "place-self-center"   => Some(simple_rule(base_selector, "place-self:center;", important)),
        "place-self-stretch"  => Some(simple_rule(base_selector, "place-self:stretch;", important)),
        // Word break / overflow
        "break-all"          => Some(simple_rule(base_selector, "word-break:break-all;", important)),
        "break-keep"         => Some(simple_rule(base_selector, "word-break:keep-all;", important)),
        "break-normal"       => Some(simple_rule(base_selector, "overflow-wrap:normal;word-break:normal;", important)),
        "overscroll-auto"    => Some(simple_rule(base_selector, "overscroll-behavior:auto;", important)),
        "overscroll-contain" => Some(simple_rule(base_selector, "overscroll-behavior:contain;", important)),
        "overscroll-none"    => Some(simple_rule(base_selector, "overscroll-behavior:none;", important)),
        // Float / clear
        "float-right" => Some(simple_rule(base_selector, "float:right;", important)),
        "float-left"  => Some(simple_rule(base_selector, "float:left;", important)),
        "float-none"  => Some(simple_rule(base_selector, "float:none;", important)),
        "clear-left"  => Some(simple_rule(base_selector, "clear:left;", important)),
        "clear-right" => Some(simple_rule(base_selector, "clear:right;", important)),
        "clear-both"  => Some(simple_rule(base_selector, "clear:both;", important)),
        "clear-none"  => Some(simple_rule(base_selector, "clear:none;", important)),
        // Misc
        "inset-auto"   => Some(simple_rule(base_selector, "inset:auto;", important)),
        "leading-wide" => Some(simple_rule(base_selector, "line-height:1.75;", important)),
        _ => None }
}

fn background_layout_rule(utility: &str) -> Option<String> {
    let declaration = match utility {
        "bg-fixed" => "background-attachment:fixed;",
        "bg-local" => "background-attachment:local;",
        "bg-scroll" => "background-attachment:scroll;",
        "bg-clip-border" => "background-clip:border-box;",
        "bg-clip-padding" => "background-clip:padding-box;",
        "bg-clip-content" => "background-clip:content-box;",
        "bg-clip-text" => "background-clip:text;-webkit-background-clip:text;",
        "bg-origin-border" => "background-origin:border-box;",
        "bg-origin-padding" => "background-origin:padding-box;",
        "bg-origin-content" => "background-origin:content-box;",
        "bg-top" => "background-position:top;",
        "bg-top-right" => "background-position:top right;",
        "bg-right" => "background-position:right;",
        "bg-right-bottom" => "background-position:right bottom;",
        "bg-left" => "background-position:left;",
        "bg-left-top" => "background-position:left top;",
        "bg-left-bottom" => "background-position:left bottom;",
        "bg-repeat-round" => "background-repeat:round;",
        "bg-repeat-space" => "background-repeat:space;",
        "bg-none" => "background-image:none;",
        "bg-gradient-to-t" => "background-image:linear-gradient(to top,var(--tw-gradient-stops));",
        "bg-gradient-to-tr" => {
            "background-image:linear-gradient(to top right,var(--tw-gradient-stops));"
        }
        "bg-gradient-to-r" => {
            "background-image:linear-gradient(to right,var(--tw-gradient-stops));"
        }
        "bg-gradient-to-br" => {
            "background-image:linear-gradient(to bottom right,var(--tw-gradient-stops));"
        }
        "bg-gradient-to-b" => {
            "background-image:linear-gradient(to bottom,var(--tw-gradient-stops));"
        }
        "bg-gradient-to-bl" => {
            "background-image:linear-gradient(to bottom left,var(--tw-gradient-stops));"
        }
        "bg-gradient-to-l" => "background-image:linear-gradient(to left,var(--tw-gradient-stops));",
        "bg-gradient-to-tl" => {
            "background-image:linear-gradient(to top left,var(--tw-gradient-stops));"
        }
        _ => return None,
    };
    Some(declaration.to_string())
}

fn table_layout_rule(utility: &str) -> Option<String> {
    let declaration = match utility {
        "border-collapse" => "border-collapse:collapse;",
        "border-separate" => "border-collapse:separate;",
        "table-auto" => "table-layout:auto;",
        "table-fixed" => "table-layout:fixed;",
        "caption-top" => "caption-side:top;",
        "caption-bottom" => "caption-side:bottom;",
        _ => {
            if let Some(v) = utility.strip_prefix("border-spacing-x-") {
                return Some(format!(
                    "--tw-border-spacing-x:{};border-spacing:var(--tw-border-spacing-x) var(--tw-border-spacing-y,0);",
                    spacing_value(v)?
                ));
            }
            if let Some(v) = utility.strip_prefix("border-spacing-y-") {
                return Some(format!(
                    "--tw-border-spacing-y:{};border-spacing:var(--tw-border-spacing-x,0) var(--tw-border-spacing-y);",
                    spacing_value(v)?
                ));
            }
            if let Some(v) = utility.strip_prefix("border-spacing-") {
                let value = spacing_value(v)?;
                return Some(format!(
                    "--tw-border-spacing-x:{0};--tw-border-spacing-y:{0};border-spacing:var(--tw-border-spacing-x) var(--tw-border-spacing-y);",
                    value
                ));
            }
            return None;
        }
    };
    Some(declaration.to_string())
}

fn fragmentation_rule(utility: &str) -> Option<String> {
    for (prefix, property) in [
        ("break-after-", "break-after"),
        ("break-before-", "break-before"),
        ("break-inside-", "break-inside"),
    ] {
        if let Some(value) = utility.strip_prefix(prefix) {
            let valid = match value {
                "auto" | "avoid" | "avoid-page" | "avoid-column" => value,
                "page" | "left" | "right" | "column" if property != "break-inside" => value,
                _ => return None,
            };
            return Some(format!("{}:{};", property, valid));
        }
    }
    None
}

fn common_misc_rule(utility: &str) -> Option<String> {
    let declaration = match utility {
        "static" => "position:static;",
        "not-sr-only" => {
            "position:static;width:auto;height:auto;padding:0;margin:0;overflow:visible;clip:auto;white-space:normal;"
        }
        "ordinal" => "font-variant-numeric:ordinal;",
        "tracking-tighter" => "letter-spacing:-0.05em;",
        "list-square" => "list-style-type:square;",
        "text-start" => "text-align:start;",
        "text-end" => "text-align:end;",
        "normal-case" => "text-transform:none;",
        "transition-shadow" => {
            "transition-property:box-shadow;transition-duration:150ms;transition-timing-function:cubic-bezier(0.4,0,0.2,1);"
        }
        "list-inside" => "list-style-position:inside;",
        "list-outside" => "list-style-position:outside;",
        "list-image-none" => "list-style-image:none;",
        "hyphens-none" => "hyphens:none;",
        "hyphens-manual" => "hyphens:manual;",
        "hyphens-auto" => "hyphens:auto;",
        "place-content-center" => "place-content:center;",
        "place-content-start" => "place-content:start;",
        "place-content-end" => "place-content:end;",
        "place-content-between" => "place-content:space-between;",
        "place-content-around" => "place-content:space-around;",
        "place-content-evenly" => "place-content:space-evenly;",
        "place-content-baseline" => "place-content:baseline;",
        "place-content-stretch" => "place-content:stretch;",
        "forced-color-adjust-auto" => "forced-color-adjust:auto;",
        "forced-color-adjust-none" => "forced-color-adjust:none;",
        _ => {
            for (prefix, property) in [
                ("overscroll-x-", "overscroll-behavior-x"),
                ("overscroll-y-", "overscroll-behavior-y"),
            ] {
                if let Some(value) = utility.strip_prefix(prefix) {
                    if matches!(value, "auto" | "contain" | "none") {
                        return Some(format!("{}:{};", property, value));
                    }
                }
            }
            if let Some(mode) = utility.strip_prefix("mix-blend-") {
                if valid_blend_mode(mode) {
                    return Some(format!("mix-blend-mode:{};", mode));
                }
            }
            if let Some(mode) = utility.strip_prefix("bg-blend-") {
                if valid_blend_mode(mode) {
                    return Some(format!("background-blend-mode:{};", mode));
                }
            }
            if let Some(raw) = utility
                .strip_prefix("list-image-[")
                .and_then(|value| value.strip_suffix(']'))
            {
                return Some(format!("list-style-image:{};", raw.replace('_', " ")));
            }
            return None;
        }
    };
    Some(declaration.to_string())
}

fn valid_blend_mode(value: &str) -> bool {
    matches!(
        value,
        "normal"
            | "multiply"
            | "screen"
            | "overlay"
            | "darken"
            | "lighten"
            | "color-dodge"
            | "color-burn"
            | "hard-light"
            | "soft-light"
            | "difference"
            | "exclusion"
            | "hue"
            | "saturation"
            | "color"
            | "luminosity"
            | "plus-lighter"
    )
}

fn svg_rule(utility: &str) -> Option<String> {
    if let Some(v) = utility.strip_prefix("fill-") {
        return color_value(v).map(|color| format!("fill:{};", color));
    }
    if let Some(v) = utility.strip_prefix("stroke-") {
        if let Ok(width) = v.parse::<u32>() {
            return Some(format!("stroke-width:{};", width));
        }
        return color_value(v).map(|color| format!("stroke:{};", color));
    }
    None
}

fn scroll_rule(utility: &str) -> Option<String> {
    let declaration = match utility {
        "scroll-auto" => "scroll-behavior:auto;",
        "scroll-smooth" => "scroll-behavior:smooth;",
        "snap-start" => "scroll-snap-align:start;",
        "snap-end" => "scroll-snap-align:end;",
        "snap-center" => "scroll-snap-align:center;",
        "snap-align-none" => "scroll-snap-align:none;",
        "snap-normal" => "scroll-snap-stop:normal;",
        "snap-always" => "scroll-snap-stop:always;",
        "snap-none" => "scroll-snap-type:none;",
        "snap-x" => "scroll-snap-type:x var(--tw-scroll-snap-strictness,proximity);",
        "snap-y" => "scroll-snap-type:y var(--tw-scroll-snap-strictness,proximity);",
        "snap-both" => "scroll-snap-type:both var(--tw-scroll-snap-strictness,proximity);",
        "snap-mandatory" => "--tw-scroll-snap-strictness:mandatory;",
        "snap-proximity" => "--tw-scroll-snap-strictness:proximity;",
        _ => {
            for (prefix, property) in [
                ("scroll-m-", "scroll-margin"),
                ("scroll-mx-", "scroll-margin-left|scroll-margin-right"),
                ("scroll-my-", "scroll-margin-top|scroll-margin-bottom"),
                ("scroll-mt-", "scroll-margin-top"),
                ("scroll-mr-", "scroll-margin-right"),
                ("scroll-mb-", "scroll-margin-bottom"),
                ("scroll-ml-", "scroll-margin-left"),
                ("scroll-p-", "scroll-padding"),
                ("scroll-px-", "scroll-padding-left|scroll-padding-right"),
                ("scroll-py-", "scroll-padding-top|scroll-padding-bottom"),
                ("scroll-pt-", "scroll-padding-top"),
                ("scroll-pr-", "scroll-padding-right"),
                ("scroll-pb-", "scroll-padding-bottom"),
                ("scroll-pl-", "scroll-padding-left"),
            ] {
                if let Some(v) = utility.strip_prefix(prefix) {
                    let value = spacing_value(v)?;
                    return Some(
                        property
                            .split('|')
                            .map(|name| format!("{}:{};", name, value))
                            .collect(),
                    );
                }
            }
            return None;
        }
    };
    Some(declaration.to_string())
}

fn transform_expression() -> &'static str {
    "translate(var(--tw-translate-x,0),var(--tw-translate-y,0)) rotate(var(--tw-rotate,0)) skewX(var(--tw-skew-x,0)) skewY(var(--tw-skew-y,0)) scaleX(var(--tw-scale-x,1)) scaleY(var(--tw-scale-y,1))"
}

fn transform_rule(utility: &str) -> Option<String> {
    if utility == "transform" {
        return Some(format!("transform:{};", transform_expression()));
    }
    if utility == "transform-none" {
        return Some("transform:none;".to_string());
    }
    for (prefix, variable) in [
        ("translate-x-", "--tw-translate-x"),
        ("translate-y-", "--tw-translate-y"),
    ] {
        if let Some(v) = utility.strip_prefix(prefix) {
            return Some(format!(
                "{}:{};transform:{};",
                variable,
                inset_value(v)?,
                transform_expression()
            ));
        }
        let negative_prefix = format!("-{}", prefix);
        if let Some(v) = utility.strip_prefix(&negative_prefix) {
            return Some(format!(
                "{}:{};transform:{};",
                variable,
                negate_css_value(&inset_value(v)?)?,
                transform_expression()
            ));
        }
    }
    for (prefix, variable) in [("skew-x-", "--tw-skew-x"), ("skew-y-", "--tw-skew-y")] {
        if let Some(v) = utility.strip_prefix(prefix) {
            let degrees = angle_value(v)?;
            return Some(format!(
                "{}:{};transform:{};",
                variable,
                degrees,
                transform_expression()
            ));
        }
        let negative_prefix = format!("-{}", prefix);
        if let Some(v) = utility.strip_prefix(&negative_prefix) {
            let degrees = negate_css_value(&angle_value(v)?)?;
            return Some(format!(
                "{}:{};transform:{};",
                variable,
                degrees,
                transform_expression()
            ));
        }
    }
    if let Some(v) = utility.strip_prefix("rotate-") {
        return Some(format!(
            "--tw-rotate:{};transform:{};",
            angle_value(v)?,
            transform_expression()
        ));
    }
    if let Some(v) = utility.strip_prefix("-rotate-") {
        return Some(format!(
            "--tw-rotate:{};transform:{};",
            negate_css_value(&angle_value(v)?)?,
            transform_expression()
        ));
    }
    if let Some(v) = utility.strip_prefix("scale-x-") {
        return Some(format!(
            "--tw-scale-x:{};transform:{};",
            scale_value(v)?,
            transform_expression()
        ));
    }
    if let Some(v) = utility.strip_prefix("scale-y-") {
        return Some(format!(
            "--tw-scale-y:{};transform:{};",
            scale_value(v)?,
            transform_expression()
        ));
    }
    if let Some(v) = utility.strip_prefix("scale-") {
        let value = scale_value(v)?;
        return Some(format!(
            "--tw-scale-x:{0};--tw-scale-y:{0};transform:{1};",
            value,
            transform_expression()
        ));
    }
    if let Some(v) = utility.strip_prefix("origin-") {
        let value = match v {
            "center" => "center",
            "top" => "top",
            "top-right" => "top right",
            "right" => "right",
            "bottom-right" => "bottom right",
            "bottom" => "bottom",
            "bottom-left" => "bottom left",
            "left" => "left",
            "top-left" => "top left",
            _ => return arbitrary_value(v).map(|raw| format!("transform-origin:{};", raw)),
        };
        return Some(format!("transform-origin:{};", value));
    }
    None
}

fn angle_value(v: &str) -> Option<String> {
    if let Some(raw) = arbitrary_value(v) {
        return Some(raw);
    }
    Some(format!("{}deg", v.parse::<f64>().ok()?))
}

fn scale_value(v: &str) -> Option<String> {
    if let Some(raw) = arbitrary_value(v) {
        return Some(raw);
    }
    let value = v.parse::<f64>().ok()? / 100.0;
    Some(format_number(value))
}

fn filter_expression(prefix: &str) -> String {
    [
        "blur",
        "brightness",
        "contrast",
        "grayscale",
        "hue-rotate",
        "invert",
        "opacity",
        "saturate",
        "sepia",
        "drop-shadow",
    ]
    .into_iter()
    .map(|name| format!("var(--tw-{}{},)", prefix, name))
    .collect::<Vec<_>>()
    .join(" ")
}

fn filter_rule(utility: &str, backdrop: bool) -> Option<String> {
    let (name, variable_prefix, property) = if backdrop {
        (
            utility.strip_prefix("backdrop-")?,
            "backdrop-",
            "backdrop-filter",
        )
    } else {
        if utility.starts_with("backdrop-") {
            return None;
        }
        (utility, "", "filter")
    };
    let (variable, function) = if name == "blur" {
        ("blur", "blur(8px)".to_string())
    } else if let Some(v) = name.strip_prefix("blur-") {
        let value = match v {
            "none" => "0".to_string(),
            "sm" => "4px".to_string(),
            "md" => "12px".to_string(),
            "lg" => "16px".to_string(),
            "xl" => "24px".to_string(),
            "2xl" => "40px".to_string(),
            "3xl" => "64px".to_string(),
            _ => arbitrary_value(v)?,
        };
        ("blur", format!("blur({})", value))
    } else if let Some(v) = name.strip_prefix("brightness-") {
        ("brightness", format!("brightness({})", percent_ratio(v)?))
    } else if let Some(v) = name.strip_prefix("contrast-") {
        ("contrast", format!("contrast({})", percent_ratio(v)?))
    } else if name == "grayscale" {
        ("grayscale", "grayscale(100%)".to_string())
    } else if let Some(v) = name.strip_prefix("grayscale-") {
        (
            "grayscale",
            format!("grayscale({}%)", v.parse::<u32>().ok()?),
        )
    } else if let Some(v) = name.strip_prefix("hue-rotate-") {
        ("hue-rotate", format!("hue-rotate({})", angle_value(v)?))
    } else if name == "invert" {
        ("invert", "invert(100%)".to_string())
    } else if let Some(v) = name.strip_prefix("invert-") {
        ("invert", format!("invert({}%)", v.parse::<u32>().ok()?))
    } else if backdrop && name.starts_with("opacity-") {
        let v = name.strip_prefix("opacity-")?;
        ("opacity", format!("opacity({})", percent_ratio(v)?))
    } else if let Some(v) = name.strip_prefix("saturate-") {
        ("saturate", format!("saturate({})", percent_ratio(v)?))
    } else if name == "sepia" {
        ("sepia", "sepia(100%)".to_string())
    } else if let Some(v) = name.strip_prefix("sepia-") {
        ("sepia", format!("sepia({}%)", v.parse::<u32>().ok()?))
    } else if let Some(v) = name.strip_prefix("drop-shadow-") {
        let value = match v {
            "sm" => "0 1px 1px rgb(0 0 0 / 0.05)",
            "md" => "0 4px 3px rgb(0 0 0 / 0.07)",
            "lg" => "0 10px 8px rgb(0 0 0 / 0.04)",
            "xl" => "0 20px 13px rgb(0 0 0 / 0.03)",
            "2xl" => "0 25px 25px rgb(0 0 0 / 0.15)",
            "none" => "0 0 #0000",
            _ => return None,
        };
        ("drop-shadow", format!("drop-shadow({})", value))
    } else {
        return None;
    };
    Some(format!(
        "--tw-{}{}:{};{}:{};",
        variable_prefix,
        variable,
        function,
        property,
        filter_expression(variable_prefix)
    ))
}

fn percent_ratio(v: &str) -> Option<String> {
    Some(format_number(v.parse::<f64>().ok()? / 100.0))
}

fn format_number(value: f64) -> String {
    let mut out = format!("{:.4}", value);
    while out.ends_with('0') {
        out.pop();
    }
    if out.ends_with('.') {
        out.pop();
    }
    if out.starts_with("0.") {
        out.remove(0);
    }
    if out.starts_with("-0.") {
        out.remove(1);
    }
    out
}

fn box_shadow_expression() -> &'static str {
    "var(--tw-ring-offset-shadow,0 0 #0000),var(--tw-ring-shadow,0 0 #0000),var(--tw-shadow,0 0 #0000)"
}

fn shadow_rule(utility: &str) -> Option<String> {
    if utility == "shadow-none" {
        return Some(format!(
            "--tw-shadow:0 0 #0000;box-shadow:{};",
            box_shadow_expression()
        ));
    }
    let (shadow, colored) = match utility {
        "shadow-sm" => (
            "0 1px 2px 0 rgb(0 0 0 / 0.05)",
            "0 1px 2px 0 var(--tw-shadow-color)",
        ),
        "shadow" => (
            "0 1px 3px 0 rgb(0 0 0 / 0.1),0 1px 2px -1px rgb(0 0 0 / 0.1)",
            "0 1px 3px 0 var(--tw-shadow-color),0 1px 2px -1px var(--tw-shadow-color)",
        ),
        "shadow-md" => (
            "0 4px 6px -1px rgb(0 0 0 / 0.1),0 2px 4px -2px rgb(0 0 0 / 0.1)",
            "0 4px 6px -1px var(--tw-shadow-color),0 2px 4px -2px var(--tw-shadow-color)",
        ),
        "shadow-lg" => (
            "0 10px 15px -3px rgb(0 0 0 / 0.1),0 4px 6px -4px rgb(0 0 0 / 0.1)",
            "0 10px 15px -3px var(--tw-shadow-color),0 4px 6px -4px var(--tw-shadow-color)",
        ),
        "shadow-xl" => (
            "0 20px 25px -5px rgb(0 0 0 / 0.1),0 8px 10px -6px rgb(0 0 0 / 0.1)",
            "0 20px 25px -5px var(--tw-shadow-color),0 8px 10px -6px var(--tw-shadow-color)",
        ),
        "shadow-2xl" => (
            "0 25px 50px -12px rgb(0 0 0 / 0.25)",
            "0 25px 50px -12px var(--tw-shadow-color)",
        ),
        _ => {
            let color = color_value(utility.strip_prefix("shadow-")?)?;
            return Some(format!(
                "--tw-shadow-color:{};--tw-shadow:var(--tw-shadow-colored);box-shadow:{};",
                color,
                box_shadow_expression()
            ));
        }
    };
    Some(format!(
        "--tw-shadow:{};--tw-shadow-colored:{};box-shadow:{};",
        shadow,
        colored,
        box_shadow_expression()
    ))
}

fn ring_width_declaration(width: &str) -> String {
    format!(
        "--tw-ring-offset-shadow:var(--tw-ring-inset,) 0 0 0 var(--tw-ring-offset-width,0px) var(--tw-ring-offset-color,#fff);--tw-ring-shadow:var(--tw-ring-inset,) 0 0 0 calc({} + var(--tw-ring-offset-width,0px)) var(--tw-ring-color,rgb(59 130 246 / 0.5));box-shadow:{};",
        width,
        box_shadow_expression()
    )
}

fn composable_ring_rule(v: &str) -> Option<String> {
    if v == "inset" {
        return Some("--tw-ring-inset:inset;".to_string());
    }
    if let Some(offset) = v.strip_prefix("offset-") {
        if let Ok(px) = offset.parse::<u32>() {
            return Some(format!("--tw-ring-offset-width:{}px;", px));
        }
        return color_value(offset).map(|color| format!("--tw-ring-offset-color:{};", color));
    }
    if let Ok(px) = v.parse::<u32>() {
        return Some(ring_width_declaration(&format!("{}px", px)));
    }
    if let Some(raw) = arbitrary_value(v) {
        if is_size_like(&raw) {
            return Some(ring_width_declaration(&raw));
        }
    }
    color_value(v).map(|color| {
        format!(
            "--tw-ring-color:{};box-shadow:{};",
            color,
            box_shadow_expression()
        )
    })
}

fn directional_radius_rule(v: &str) -> Option<String> {
    let (side, size) = v.split_once('-')?;
    let value = radius_value(size)?;
    let properties: &[&str] = match side {
        "t" => &["border-top-left-radius", "border-top-right-radius"],
        "r" => &["border-top-right-radius", "border-bottom-right-radius"],
        "b" => &["border-bottom-right-radius", "border-bottom-left-radius"],
        "l" => &["border-top-left-radius", "border-bottom-left-radius"],
        "tl" => &["border-top-left-radius"],
        "tr" => &["border-top-right-radius"],
        "br" => &["border-bottom-right-radius"],
        "bl" => &["border-bottom-left-radius"],
        _ => return None,
    };
    Some(
        properties
            .iter()
            .map(|property| format!("{}:{};", property, value))
            .collect(),
    )
}

fn radius_value(v: &str) -> Option<String> {
    let value = match v {
        "none" => "0",
        "sm" => "0.125rem",
        "" | "DEFAULT" => "0.25rem",
        "md" => "0.375rem",
        "lg" => "0.5rem",
        "xl" => "0.75rem",
        "2xl" => "1rem",
        "3xl" => "1.5rem",
        "full" => "9999px",
        _ => return arbitrary_value(v),
    };
    Some(value.to_string())
}

fn border_width_value(v: &str) -> Option<String> {
    if v == "px" {
        return Some("1px".to_string());
    }
    if let Ok(px) = v.parse::<u32>() {
        return Some(format!("{}px", px));
    }
    arbitrary_value(v)
}

fn simple_rule(s: &str, d: &str, i: bool) -> UtilityRule {
    UtilityRule {
        selector: s.to_string(),
        declarations: maybe_important(d, i),
        prelude: None,
    }
}
fn maybe_important(d: &str, i: bool) -> String {
    if !i {
        return d.to_string();
    }
    let mut o = String::new();
    for p in d.split(';') {
        let t = p.trim();
        if !t.is_empty() {
            if let Some((k, v)) = t.split_once(':') {
                o.push_str(k.trim());
                o.push(':');
                o.push_str(v.trim());
                o.push_str(" !important;");
            }
        }
    }
    o
}
fn spacing_value(v: &str) -> Option<String> {
    if v == "px" {
        return Some("1px".to_string());
    }
    if let Some(r) = arbitrary_value(v) {
        return Some(r);
    }
    let p = v.parse::<f64>().ok()?;
    if p.abs() < f64::EPSILON {
        return Some("0".to_string());
    }
    Some(format_rem(p * 0.25))
}
fn inset_value(v: &str) -> Option<String> {
    if v == "0" {
        return Some("0".to_string());
    }
    if v == "auto" {
        return Some("auto".to_string());
    }
    if v == "full" {
        return Some("100%".to_string());
    }
    if let Some(r) = arbitrary_value(v) {
        return Some(r);
    }
    if let Some(f) = fraction_to_percent(v) {
        return Some(f);
    }
    spacing_value(v)
}
fn negate_css_value(v: &str) -> Option<String> {
    let t = v.trim();
    if t.is_empty() || t == "auto" {
        return None;
    }
    if t == "0" || t == "0px" || t == "0rem" || t == "0%" {
        return Some("0".to_string());
    }
    if t.starts_with('-') {
        return Some(t.to_string());
    }
    if t.starts_with("var(")
        || t.starts_with("calc(")
        || t.starts_with("min(")
        || t.starts_with("max(")
        || t.starts_with("clamp(")
    {
        return Some(format!("calc({} * -1)", t));
    }
    Some(format!("-{}", t))
}
#[derive(Debug, Clone, Copy)]
enum SizeAxis {
    Width,
    Height,
}
fn size_value(v: &str, a: SizeAxis) -> Option<String> {
    match v {
        "full" => Some("100%".to_string()),
        "auto" => Some("auto".to_string()),
        "min" => Some("min-content".to_string()),
        "max" => Some("max-content".to_string()),
        "fit" => Some("fit-content".to_string()),
        "screen" => Some(
            match a {
                SizeAxis::Width => "100vw",
                SizeAxis::Height => "100vh",
            }
            .to_string(),
        ),
        "svw" if matches!(a, SizeAxis::Width) => Some("100svw".to_string()),
        "lvw" if matches!(a, SizeAxis::Width) => Some("100lvw".to_string()),
        "dvw" if matches!(a, SizeAxis::Width) => Some("100dvw".to_string()),
        "svh" if matches!(a, SizeAxis::Height) => Some("100svh".to_string()),
        "lvh" if matches!(a, SizeAxis::Height) => Some("100lvh".to_string()),
        "dvh" if matches!(a, SizeAxis::Height) => Some("100dvh".to_string()),
        "px" => Some("1px".to_string()),
        _ => {
            if let Some(r) = arbitrary_value(v) {
                return Some(r);
            }
            if let Some(f) = fraction_to_percent(v) {
                return Some(f);
            }
            if has_direct_css_unit(v) {
                return Some(v.to_string());
            }
            spacing_value(v)
        }
    }
}
fn minmax_size_value(v: &str, a: SizeAxis) -> Option<String> {
    if let Some(m) = size_value(v, a) {
        return Some(m);
    }
    if v == "none" {
        return Some("none".to_string());
    }
    None
}
fn max_width_value(v: &str) -> Option<String> {
    match v {
        "xs" => Some("20rem".to_string()),
        "sm" => Some("24rem".to_string()),
        "md" => Some("28rem".to_string()),
        "lg" => Some("32rem".to_string()),
        "xl" => Some("36rem".to_string()),
        "2xl" => Some("42rem".to_string()),
        "3xl" => Some("48rem".to_string()),
        "4xl" => Some("56rem".to_string()),
        "5xl" => Some("64rem".to_string()),
        "6xl" => Some("72rem".to_string()),
        "7xl" => Some("80rem".to_string()),
        "screen-md" => Some("768px".to_string()),
        "screen-lg" => Some("1024px".to_string()),
        "screen-sm" => Some("640px".to_string()),
        "screen-xl" => Some("1280px".to_string()),
        "screen-2xl" => Some("1536px".to_string()),
        "prose" => Some("65ch".to_string()),
        _ => minmax_size_value(v, SizeAxis::Width),
    }
}
fn text_size_value(v: &str) -> Option<&'static str> {
    match v {
        "xs" => Some("0.75rem"),
        "sm" => Some("0.875rem"),
        "base" => Some("1rem"),
        "lg" => Some("1.125rem"),
        "xl" => Some("1.25rem"),
        "2xl" => Some("1.5rem"),
        "3xl" => Some("1.875rem"),
        "4xl" => Some("2.25rem"),
        "5xl" => Some("3rem"),
        "6xl" => Some("3.75rem"),
        "7xl" => Some("4.5rem"),
        "8xl" => Some("6rem"),
        "9xl" => Some("8rem"),
        _ => None,
    }
}
fn border_rule(v: &str) -> Option<String> {
    match v { "0" => Some("border-width:0;".to_string()), "2" => Some("border-width:2px;border-style:solid;".to_string()), "4" => Some("border-width:4px;border-style:solid;".to_string()), "8" => Some("border-width:8px;border-style:solid;".to_string()), "t" => Some("border-top-width:1px;border-top-style:solid;".to_string()), "r" => Some("border-right-width:1px;border-right-style:solid;".to_string()), "b" => Some("border-bottom-width:1px;border-bottom-style:solid;".to_string()), "l" => Some("border-left-width:1px;border-left-style:solid;".to_string()), "x" => Some("border-left-width:1px;border-right-width:1px;border-left-style:solid;border-right-style:solid;".to_string()), "y" => Some("border-top-width:1px;border-bottom-width:1px;border-top-style:solid;border-bottom-style:solid;".to_string()), "dashed" => Some("border-style:dashed;".to_string()), "dotted" => Some("border-style:dotted;".to_string()), "double" => Some("border-style:double;".to_string()), "hidden" => Some("border-style:hidden;".to_string()), "none" => Some("border-style:none;".to_string()), "solid" => Some("border-style:solid;".to_string()), _ => color_value(v).map(|c| format!("border-color:{};", c)) }
}
fn outline_rule(v: &str) -> Option<String> {
    match v {
        "none" => Some("outline:2px solid transparent;outline-offset:2px;".to_string()),
        "hidden" => Some("outline:none;".to_string()),
        "solid" | "dashed" | "dotted" | "double" => Some(format!("outline-style:{};", v)),
        _ => {
            if let Ok(px) = v.parse::<u32>() {
                Some(format!("outline-width:{}px;outline-style:solid;", px))
            } else {
                color_value(v).map(|c| format!("outline-color:{};outline-style:solid;", c))
            }
        }
    }
}
fn background_value(v: &str) -> Option<String> {
    arbitrary_value(v)
}
fn background_declaration(v: &str) -> String {
    let t = v.trim();
    if t.starts_with("url(")
        || t.starts_with("linear-gradient(")
        || t.starts_with("radial-gradient(")
        || t.starts_with("conic-gradient(")
        || t.starts_with("image(")
    {
        format!("background-image:{};", t)
    } else {
        format!("background:{};", t)
    }
}
fn color_value(v: &str) -> Option<String> {
    if let Some(r) = arbitrary_value(v) {
        return Some(r);
    }
    let (b, a) = split_color_alpha(v);
    let c = if b == "current" {
        "currentColor".to_string()
    } else if b == "transparent" {
        "transparent".to_string()
    } else if b == "black" {
        "#000000".to_string()
    } else if b == "white" {
        "#ffffff".to_string()
    } else if let Some(h) = tw_color_hex(b) {
        h.to_string()
    } else if is_semantic_color_token(b) {
        let k = b.replace('_', "-");
        format!("var(--color-{})", k)
    } else {
        return None;
    };
    if let Some(alpha) = a {
        return Some(apply_alpha(&c, alpha));
    }
    Some(c)
}
fn split_color_alpha(v: &str) -> (&str, Option<f64>) {
    if let Some((b, a)) = v.split_once('/') {
        if let Some(alpha) = parse_alpha(a) {
            return (b, Some(alpha));
        }
        (b, None)
    } else {
        (v, None)
    }
}
fn parse_alpha(v: &str) -> Option<f64> {
    if let Some(r) = v.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
        r.parse::<f64>().ok().map(|n| n.clamp(0.0, 1.0))
    } else if let Ok(n) = v.parse::<u32>() {
        Some((n.min(100) as f64) / 100.0)
    } else {
        v.parse::<f64>().ok().map(|n| n.clamp(0.0, 1.0))
    }
}
fn apply_alpha(c: &str, a: f64) -> String {
    if c == "transparent" {
        return c.to_string();
    }
    if let Some((r, g, b)) = hex_to_rgb(c) {
        return format!("rgba({}, {}, {}, {:.3})", r, g, b, a);
    }
    let p = (a * 100.0).clamp(0.0, 100.0);
    format!("color-mix(in srgb, {} {:.1}%, transparent)", c, p)
}
fn hex_to_rgb(v: &str) -> Option<(u8, u8, u8)> {
    let h = v.strip_prefix('#')?;
    match h.len() {
        6 => Some((
            u8::from_str_radix(&h[0..2], 16).ok()?,
            u8::from_str_radix(&h[2..4], 16).ok()?,
            u8::from_str_radix(&h[4..6], 16).ok()?,
        )),
        3 => Some((
            u8::from_str_radix(&h[0..1].repeat(2), 16).ok()?,
            u8::from_str_radix(&h[1..2].repeat(2), 16).ok()?,
            u8::from_str_radix(&h[2..3].repeat(2), 16).ok()?,
        )),
        _ => None,
    }
}
fn is_semantic_color_token(v: &str) -> bool {
    matches!(
        v,
        "bg" | "surface"
            | "surface-2"
            | "surface-3"
            | "body"
            | "body-soft"
            | "body-muted"
            | "accent"
            | "accent-strong"
            | "accent-alt"
            | "accent-alt-strong"
            | "border"
            | "border-soft"
            | "warning"
            | "destructive"
            | "destructive-foreground"
            | "brand-blue"
            | "brand-blue-ink"
            | "brand-orange"
            | "brand-orange-ink"
            | "dark-background"
            | "dark-border"
            | "dark-menus"
            | "dark-text1"
            | "dark-accent1"
            | "dark-accent2"
            | "dark-accent3"
            | "dark-accent4"
            | "dark-accent5"
            | "ui-bg"
            | "ui-bg-muted"
            | "ui-bg-subtle"
            | "ui-border"
            | "ui-border-subtle"
            | "ui-text"
            | "ui-text-muted"
            | "ui-text-soft"
    )
}
fn is_size_like(v: &str) -> bool {
    if v == "0" || v == "auto" {
        return true;
    }
    has_direct_css_unit(v)
}
fn tw_color_hex(v: &str) -> Option<&'static str> {
    match v {
        "slate-50" => Some("#f8fafc"),
        "slate-100" => Some("#f1f5f9"),
        "slate-200" => Some("#e2e8f0"),
        "slate-300" => Some("#cbd5e1"),
        "slate-400" => Some("#94a3b8"),
        "slate-500" => Some("#64748b"),
        "slate-600" => Some("#475569"),
        "slate-700" => Some("#334155"),
        "slate-800" => Some("#1e293b"),
        "slate-900" => Some("#0f172a"),
        "slate-950" => Some("#020617"),
        "gray-50" => Some("#f9fafb"),
        "gray-100" => Some("#f3f4f6"),
        "gray-200" => Some("#e5e7eb"),
        "gray-300" => Some("#d1d5db"),
        "gray-400" => Some("#9ca3af"),
        "gray-500" => Some("#6b7280"),
        "gray-600" => Some("#4b5563"),
        "gray-700" => Some("#374151"),
        "gray-800" => Some("#1f2937"),
        "gray-900" => Some("#111827"),
        "gray-950" => Some("#030712"),
        "zinc-50" => Some("#fafafa"),
        "zinc-100" => Some("#f4f4f5"),
        "zinc-200" => Some("#e4e4e7"),
        "zinc-300" => Some("#d4d4d8"),
        "zinc-400" => Some("#a1a1aa"),
        "zinc-500" => Some("#71717a"),
        "zinc-600" => Some("#52525b"),
        "zinc-700" => Some("#3f3f46"),
        "zinc-800" => Some("#27272a"),
        "zinc-900" => Some("#18181b"),
        "zinc-950" => Some("#09090b"),
        "neutral-50" => Some("#fafafa"),
        "neutral-100" => Some("#f5f5f5"),
        "neutral-200" => Some("#e5e5e5"),
        "neutral-300" => Some("#d4d4d4"),
        "neutral-400" => Some("#a3a3a3"),
        "neutral-500" => Some("#737373"),
        "neutral-600" => Some("#525252"),
        "neutral-700" => Some("#404040"),
        "neutral-800" => Some("#262626"),
        "neutral-900" => Some("#171717"),
        "neutral-950" => Some("#0a0a0a"),
        "stone-50" => Some("#fafaf9"),
        "stone-100" => Some("#f5f5f4"),
        "stone-200" => Some("#e7e5e4"),
        "stone-300" => Some("#d6d3d1"),
        "stone-400" => Some("#a8a29e"),
        "stone-500" => Some("#78716c"),
        "stone-600" => Some("#57534e"),
        "stone-700" => Some("#44403c"),
        "stone-800" => Some("#292524"),
        "stone-900" => Some("#1c1917"),
        "stone-950" => Some("#0c0a09"),
        "orange-50" => Some("#fff7ed"),
        "orange-100" => Some("#ffedd5"),
        "orange-200" => Some("#fed7aa"),
        "orange-300" => Some("#fdba74"),
        "orange-400" => Some("#fb923c"),
        "orange-500" => Some("#f97316"),
        "orange-600" => Some("#ea580c"),
        "orange-700" => Some("#c2410c"),
        "orange-800" => Some("#9a3412"),
        "orange-900" => Some("#7c2d12"),
        "orange-950" => Some("#431407"),
        "yellow-50" => Some("#fefce8"),
        "yellow-100" => Some("#fef9c3"),
        "yellow-200" => Some("#fef08a"),
        "yellow-300" => Some("#fde047"),
        "yellow-400" => Some("#facc15"),
        "yellow-500" => Some("#eab308"),
        "yellow-600" => Some("#ca8a04"),
        "yellow-700" => Some("#a16207"),
        "yellow-800" => Some("#854d0e"),
        "yellow-900" => Some("#713f12"),
        "yellow-950" => Some("#422006"),
        "lime-50" => Some("#f7fee7"),
        "lime-100" => Some("#ecfccb"),
        "lime-200" => Some("#d9f99d"),
        "lime-300" => Some("#bef264"),
        "lime-400" => Some("#a3e635"),
        "lime-500" => Some("#84cc16"),
        "lime-600" => Some("#65a30d"),
        "lime-700" => Some("#4d7c0f"),
        "lime-800" => Some("#3f6212"),
        "lime-900" => Some("#365314"),
        "lime-950" => Some("#1a2e05"),
        "green-50" => Some("#f0fdf4"),
        "green-100" => Some("#dcfce7"),
        "green-200" => Some("#bbf7d0"),
        "green-300" => Some("#86efac"),
        "green-400" => Some("#4ade80"),
        "green-500" => Some("#22c55e"),
        "green-600" => Some("#16a34a"),
        "green-700" => Some("#15803d"),
        "green-800" => Some("#166534"),
        "green-900" => Some("#14532d"),
        "green-950" => Some("#052e16"),
        "emerald-50" => Some("#ecfdf5"),
        "emerald-100" => Some("#d1fae5"),
        "emerald-200" => Some("#a7f3d0"),
        "emerald-300" => Some("#6ee7b7"),
        "emerald-400" => Some("#34d399"),
        "emerald-500" => Some("#10b981"),
        "emerald-600" => Some("#059669"),
        "emerald-700" => Some("#047857"),
        "emerald-800" => Some("#065f46"),
        "emerald-900" => Some("#064e3b"),
        "emerald-950" => Some("#022c22"),
        "teal-50" => Some("#f0fdfa"),
        "teal-100" => Some("#ccfbf1"),
        "teal-200" => Some("#99f6e4"),
        "teal-300" => Some("#5eead4"),
        "teal-400" => Some("#2dd4bf"),
        "teal-500" => Some("#14b8a6"),
        "teal-600" => Some("#0d9488"),
        "teal-700" => Some("#0f766e"),
        "teal-800" => Some("#115e59"),
        "teal-900" => Some("#134e4a"),
        "teal-950" => Some("#042f2e"),
        "cyan-50" => Some("#ecfeff"),
        "cyan-100" => Some("#cffafe"),
        "cyan-200" => Some("#a5f3fc"),
        "cyan-300" => Some("#67e8f9"),
        "cyan-400" => Some("#22d3ee"),
        "cyan-500" => Some("#06b6d4"),
        "cyan-600" => Some("#0891b2"),
        "cyan-700" => Some("#0e7490"),
        "cyan-800" => Some("#155e75"),
        "cyan-900" => Some("#164e63"),
        "cyan-950" => Some("#083344"),
        "sky-50" => Some("#f0f9ff"),
        "sky-100" => Some("#e0f2fe"),
        "sky-200" => Some("#bae6fd"),
        "sky-300" => Some("#7dd3fc"),
        "sky-400" => Some("#38bdf8"),
        "sky-500" => Some("#0ea5e9"),
        "sky-600" => Some("#0284c7"),
        "sky-700" => Some("#0369a1"),
        "sky-800" => Some("#075985"),
        "sky-900" => Some("#0c4a6e"),
        "sky-950" => Some("#082f49"),
        "blue-50" => Some("#eff6ff"),
        "blue-100" => Some("#dbeafe"),
        "blue-200" => Some("#bfdbfe"),
        "blue-300" => Some("#93c5fd"),
        "blue-400" => Some("#60a5fa"),
        "blue-500" => Some("#3b82f6"),
        "blue-600" => Some("#2563eb"),
        "blue-700" => Some("#1d4ed8"),
        "blue-800" => Some("#1e40af"),
        "blue-900" => Some("#1e3a8a"),
        "blue-950" => Some("#172554"),
        "indigo-50" => Some("#eef2ff"),
        "indigo-100" => Some("#e0e7ff"),
        "indigo-200" => Some("#c7d2fe"),
        "indigo-300" => Some("#a5b4fc"),
        "indigo-400" => Some("#818cf8"),
        "indigo-500" => Some("#6366f1"),
        "indigo-600" => Some("#4f46e5"),
        "indigo-700" => Some("#4338ca"),
        "indigo-800" => Some("#3730a3"),
        "indigo-900" => Some("#312e81"),
        "indigo-950" => Some("#1e1b4b"),
        "violet-50" => Some("#f5f3ff"),
        "violet-100" => Some("#ede9fe"),
        "violet-200" => Some("#ddd6fe"),
        "violet-300" => Some("#c4b5fd"),
        "violet-400" => Some("#a78bfa"),
        "violet-500" => Some("#8b5cf6"),
        "violet-600" => Some("#7c3aed"),
        "violet-700" => Some("#6d28d9"),
        "violet-800" => Some("#5b21b6"),
        "violet-900" => Some("#4c1d95"),
        "violet-950" => Some("#2e1065"),
        "purple-50" => Some("#faf5ff"),
        "purple-100" => Some("#f3e8ff"),
        "purple-200" => Some("#e9d5ff"),
        "purple-300" => Some("#d8b4fe"),
        "purple-400" => Some("#c084fc"),
        "purple-500" => Some("#a855f7"),
        "purple-600" => Some("#9333ea"),
        "purple-700" => Some("#7e22ce"),
        "purple-800" => Some("#6b21a8"),
        "purple-900" => Some("#581c87"),
        "purple-950" => Some("#3b0764"),
        "fuchsia-50" => Some("#fdf4ff"),
        "fuchsia-100" => Some("#fae8ff"),
        "fuchsia-200" => Some("#f5d0fe"),
        "fuchsia-300" => Some("#f0abfc"),
        "fuchsia-400" => Some("#e879f9"),
        "fuchsia-500" => Some("#d946ef"),
        "fuchsia-600" => Some("#c026d3"),
        "fuchsia-700" => Some("#a21caf"),
        "fuchsia-800" => Some("#86198f"),
        "fuchsia-900" => Some("#701a75"),
        "fuchsia-950" => Some("#4a044e"),
        "pink-50" => Some("#fdf2f8"),
        "pink-100" => Some("#fce7f3"),
        "pink-200" => Some("#fbcfe8"),
        "pink-300" => Some("#f9a8d4"),
        "pink-400" => Some("#f472b6"),
        "pink-500" => Some("#ec4899"),
        "pink-600" => Some("#db2777"),
        "pink-700" => Some("#be185d"),
        "pink-800" => Some("#9d174d"),
        "pink-900" => Some("#831843"),
        "pink-950" => Some("#500724"),
        "rose-50" => Some("#fff1f2"),
        "rose-100" => Some("#ffe4e6"),
        "rose-200" => Some("#fecdd3"),
        "rose-300" => Some("#fda4af"),
        "rose-400" => Some("#fb7185"),
        "rose-500" => Some("#f43f5e"),
        "rose-600" => Some("#e11d48"),
        "rose-700" => Some("#be123c"),
        "rose-800" => Some("#9f1239"),
        "rose-900" => Some("#881337"),
        "rose-950" => Some("#4c0519"),
        "red-50" => Some("#fef2f2"),
        "red-100" => Some("#fee2e2"),
        "red-200" => Some("#fecaca"),
        "red-300" => Some("#fca5a5"),
        "red-400" => Some("#f87171"),
        "red-500" => Some("#ef4444"),
        "red-600" => Some("#dc2626"),
        "red-700" => Some("#b91c1c"),
        "red-800" => Some("#991b1b"),
        "red-900" => Some("#7f1d1d"),
        "red-950" => Some("#450a0a"),
        "amber-50" => Some("#fffbeb"),
        "amber-100" => Some("#fef3c7"),
        "amber-200" => Some("#fde68a"),
        "amber-300" => Some("#fcd34d"),
        "amber-400" => Some("#fbbf24"),
        "amber-500" => Some("#f59e0b"),
        "amber-600" => Some("#d97706"),
        "amber-700" => Some("#b45309"),
        "amber-800" => Some("#92400e"),
        "amber-900" => Some("#78350f"),
        "amber-950" => Some("#451a03"),
        _ => None,
    }
}

fn arbitrary_value(v: &str) -> Option<String> {
    let r = v.strip_prefix('[')?.strip_suffix(']')?;
    if r.is_empty() {
        return None;
    }
    let s = r.chars().all(|c| {
        c.is_ascii_alphanumeric()
            || matches!(
                c,
                '#' | '.' | ',' | '%' | '/' | '_' | '-' | '(' | ')' | ':' | '\'' | '"' | ' '
            )
    });
    if !s || r.contains(';') || r.contains('{') || r.contains('}') {
        return None;
    }
    Some(r.replace('_', " "))
}
fn fraction_to_percent(v: &str) -> Option<String> {
    let (a, b) = v.split_once('/')?;
    let n = a.parse::<f64>().ok()?;
    let d = b.parse::<f64>().ok()?;
    if d == 0.0 {
        return None;
    }
    let p = (n / d) * 100.0;
    let mut s = format!("{:.6}", p);
    while s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
    if s.is_empty() {
        s.push('0');
    }
    Some(format!("{}%", s))
}
fn has_direct_css_unit(v: &str) -> bool {
    let l = v.to_ascii_lowercase();
    ["px", "rem", "em", "%", "vh", "vw", "svh", "dvh", "ch"]
        .iter()
        .any(|u| l.ends_with(u))
}
fn format_rem(v: f64) -> String {
    let mut s = format!("{:.6}", v);
    while s.ends_with('0') {
        s.pop();
    }
    if s.ends_with('.') {
        s.pop();
    }
    if s.is_empty() {
        "0rem".to_string()
    } else {
        format!("{}rem", s)
    }
}

#[cfg(test)]
mod tests {
    use super::{minify_css_lossy, process_tailwind, token_css_rule};

    #[test]
    fn css_minifier_removes_comments_and_compacts_whitespace() {
        let raw = r#"
/* comment */
.a { color: red; }
.b    { margin : 0 ; padding : 4px ; }
"#;
        let minified = minify_css_lossy(raw);
        assert_eq!(minified, ".a{color:red;}.b{margin:0;padding:4px;}");
    }

    #[test]
    fn process_tailwind_injects_compacted_style_block() {
        let html =
            "<html><head></head><body><div class=\"p-4 text-slate-100\"></div></body></html>";
        let out = process_tailwind(html, &std::collections::HashSet::new());
        let start = out.find("<style data-rwe-tw>").expect("style open");
        let content_start = start + "<style data-rwe-tw>".len();
        let end = out[content_start..].find("</style>").expect("style close") + content_start;
        let css = &out[content_start..end];
        assert!(
            !css.contains("/*"),
            "css should not include preflight comments"
        );
        assert!(css.contains(".p-4{padding:1rem;}"));
        assert!(css.contains(".text-slate-100{color:#f1f5f9;}"));
    }

    #[test]
    fn font_display_utility_uses_global_font_token() {
        let css = token_css_rule("font-display").expect("font-display rule");
        assert!(css.contains(
            ".font-display{font-family:var(--font-display, ui-sans-serif, system-ui, sans-serif);}"
        ));
        assert!(!css.contains("--zebflow-font-display"));
        assert!(!css.contains(", display)"));
    }

    #[test]
    fn semantic_color_tokens_map_to_main_css_variable_contract() {
        let text_css = token_css_rule("text-brand-blue").expect("text-brand-blue rule");
        let bg_css = token_css_rule("bg-surface").expect("bg-surface rule");
        let border_css = token_css_rule("border-body-soft").expect("border-body-soft rule");

        assert!(text_css.contains("var(--color-brand-blue)"));
        assert!(bg_css.contains("var(--color-surface)"));
        assert!(border_css.contains("var(--color-body-soft)"));
    }

    #[test]
    fn semantic_color_tokens_align_with_platform_main_css() {
        let main_css = include_str!("../../../platform/web/templates/styles/main.css");
        assert!(
            main_css.contains("--color-brand-blue:"),
            "expected platform main.css to define --color-brand-blue"
        );
        assert!(
            main_css.contains("--color-ui-text-soft:"),
            "expected platform main.css to define --color-ui-text-soft"
        );

        let text_css = token_css_rule("text-brand-blue").expect("text-brand-blue rule");
        let soft_text_css = token_css_rule("text-ui-text-soft").expect("text-ui-text-soft rule");
        assert!(text_css.contains("var(--color-brand-blue)"));
        assert!(soft_text_css.contains("var(--color-ui-text-soft)"));
    }
}
