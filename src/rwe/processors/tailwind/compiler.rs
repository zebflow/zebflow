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
        if variant == "peer-disabled" {
            rule.selector = format!(".peer:disabled ~ {}", rule.selector);
            continue;
        }
        if let Some(pseudo) = variant_pseudo(variant) {
            rule.selector.push_str(pseudo);
            continue;
        }
        return None;
    }
    let mut out = format!("{}{{{}}}", rule.selector, rule.declarations);
    for media in medias.into_iter().rev() {
        out = format!("@media {}{{{}}}", media, out);
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
pub fn process_tailwind(html: &str) -> String {
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
    process_tailwind(&stripped)
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

fn token_precedence_key(token: &str) -> (u8, u8) {
    let Some((variants, _)) = split_variants(token) else {
        return (0, 0);
    };
    if variants.is_empty() {
        return (0, 0);
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
        return (2, max_media_rank);
    }
    if has_non_media {
        return (1, 0);
    }
    (0, 0)
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

fn variant_media_query(v: &str) -> Option<&'static str> {
    match v {
        "sm" => Some("(min-width: 640px)"),
        "md" => Some("(min-width: 768px)"),
        "lg" => Some("(min-width: 1024px)"),
        "xl" => Some("(min-width: 1280px)"),
        "2xl" => Some("(min-width: 1536px)"),
        _ => None,
    }
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
        _ => None,
    }
}

fn utility_rule(utility: &str, base_selector: &str, important: bool) -> Option<UtilityRule> {
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
    if let Some(v) = utility.strip_prefix("space-y-") {
        let value = spacing_value(v)?;
        return Some(UtilityRule {
            selector: format!("{} > :not([hidden]) ~ :not([hidden])", base_selector),
            declarations: maybe_important(&format!("margin-top:{};", value), important),
            prelude: None,
        });
    }
    if let Some(v) = utility.strip_prefix("space-x-") {
        let value = spacing_value(v)?;
        return Some(UtilityRule {
            selector: format!("{} > :not([hidden]) ~ :not([hidden])", base_selector),
            declarations: maybe_important(&format!("margin-left:{};", value), important),
            prelude: None,
        });
    }
    if let Some(v) = utility.strip_prefix("grid-cols-") {
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
        if !matches!(v, "left" | "center" | "right" | "justify") {
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
                _ => (v, v),
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
                &format!("font-family:var(--zebflow-font-{}, {});", slug, fallback),
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
                &format!("--tw-gradient-from:{};--tw-gradient-to:transparent;", color),
                important,
            ));
        }
    }
    if let Some(v) = utility.strip_prefix("to-") {
        if let Some(color) = color_value(v) {
            return Some(simple_rule(
                base_selector,
                &format!("--tw-gradient-to:{};", color),
                important,
            ));
        }
    }
    if let Some(v) = utility.strip_prefix("via-") {
        if let Some(color) = color_value(v) {
            return Some(simple_rule(
                base_selector,
                &format!("--tw-gradient-via:{};", color),
                important,
            ));
        }
    }
    if let Some(v) = utility.strip_prefix("border-b-") {
        let color = color_value(v)?;
        return Some(simple_rule(
            base_selector,
            &format!("border-bottom-color:{};", color),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("border-t-") {
        let color = color_value(v)?;
        return Some(simple_rule(
            base_selector,
            &format!("border-top-color:{};", color),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("border-l-") {
        let color = color_value(v)?;
        return Some(simple_rule(
            base_selector,
            &format!("border-left-color:{};", color),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("border-r-") {
        let color = color_value(v)?;
        return Some(simple_rule(
            base_selector,
            &format!("border-right-color:{};", color),
            important,
        ));
    }
    if let Some(v) = utility.strip_prefix("border-") {
        if let Some(decl) = border_rule(v) {
            return Some(simple_rule(base_selector, &decl, important));
        }
    }
    if let Some(v) = utility.strip_prefix("outline-") {
        if let Some(decl) = outline_rule(v) {
            return Some(simple_rule(base_selector, &decl, important));
        }
    }
    if let Some(v) = utility.strip_prefix("ring-") {
        if let Some(decl) = ring_rule(v) {
            return Some(simple_rule(base_selector, &decl, important));
        }
    }
    if let Some(v) = utility.strip_prefix("rounded-") {
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
    if let Some(v) = utility.strip_prefix("order-") {
        let order = v.parse::<i32>().ok()?;
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
    match utility {
        "flex" => Some(simple_rule(base_selector, "display:flex;", important)), "grid" => Some(simple_rule(base_selector, "display:grid;", important)), "block" => Some(simple_rule(base_selector, "display:block;", important)), "inline" => Some(simple_rule(base_selector, "display:inline;", important)), "inline-block" => Some(simple_rule(base_selector, "display:inline-block;", important)), "hidden" => Some(simple_rule(base_selector, "display:none;", important)), "flex-col" => Some(simple_rule(base_selector, "flex-direction:column;", important)), "flex-row" => Some(simple_rule(base_selector, "flex-direction:row;", important)), "flex-wrap" => Some(simple_rule(base_selector, "flex-wrap:wrap;", important)), "flex-1" => Some(simple_rule(base_selector, "flex:1 1 0%;", important)), "flex-0" => Some(simple_rule(base_selector, "flex:0 0 auto;", important)), "flex-none" => Some(simple_rule(base_selector, "flex:none;", important)), "shrink-0" => Some(simple_rule(base_selector, "flex-shrink:0;", important)), "basis-0" => Some(simple_rule(base_selector, "flex-basis:0;", important)), "items-start" => Some(simple_rule(base_selector, "align-items:flex-start;", important)), "items-center" => Some(simple_rule(base_selector, "align-items:center;", important)), "items-end" => Some(simple_rule(base_selector, "align-items:flex-end;", important)), "items-stretch" => Some(simple_rule(base_selector, "align-items:stretch;", important)), "items-baseline" => Some(simple_rule(base_selector, "align-items:baseline;", important)), "align-start" => Some(simple_rule(base_selector, "align-items:flex-start;", important)), "justify-start" => Some(simple_rule(base_selector, "justify-content:flex-start;", important)), "justify-center" => Some(simple_rule(base_selector, "justify-content:center;", important)), "justify-end" => Some(simple_rule(base_selector, "justify-content:flex-end;", important)), "justify-between" => Some(simple_rule(base_selector, "justify-content:space-between;", important)), "justify-around" => Some(simple_rule(base_selector, "justify-content:space-around;", important)), "justify-evenly" => Some(simple_rule(base_selector, "justify-content:space-evenly;", important)), "justify-stretch" => Some(simple_rule(base_selector, "justify-content:stretch;", important)), "rounded" => Some(simple_rule(base_selector, "border-radius:0.25rem;", important)), "rounded-sm" => Some(simple_rule(base_selector, "border-radius:0.25rem;", important)), "rounded-md" => Some(simple_rule(base_selector, "border-radius:0.375rem;", important)), "rounded-lg" => Some(simple_rule(base_selector, "border-radius:0.5rem;", important)), "rounded-xl" => Some(simple_rule(base_selector, "border-radius:0.75rem;", important)), "rounded-2xl" => Some(simple_rule(base_selector, "border-radius:1rem;", important)), "rounded-3xl" => Some(simple_rule(base_selector, "border-radius:1.5rem;", important)), "rounded-4xl" => Some(simple_rule(base_selector, "border-radius:2rem;", important)), "rounded-full" => Some(simple_rule(base_selector, "border-radius:9999px;", important)), "rounded-none" => Some(simple_rule(base_selector, "border-radius:0;", important)), "rounded-xs" => Some(simple_rule(base_selector, "border-radius:0.125rem;", important)), "shadow" | "shadow-sm" => Some(simple_rule(base_selector, "box-shadow:0 1px 2px rgba(0,0,0,0.05);", important)), "shadow-md" => Some(simple_rule(base_selector, "box-shadow:0 4px 12px rgba(0,0,0,0.08);", important)), "shadow-lg" => Some(simple_rule(base_selector, "box-shadow:0 12px 32px rgba(0,0,0,0.12);", important)), "shadow-2xl" => Some(simple_rule(base_selector, "box-shadow:0 20px 48px rgba(0,0,0,0.2);", important)), "shadow-xs" => Some(simple_rule(base_selector, "box-shadow:0 1px 1px rgba(0,0,0,0.04);", important)), "font-thin" => Some(simple_rule(base_selector, "font-weight:100;", important)), "font-extralight" => Some(simple_rule(base_selector, "font-weight:200;", important)), "font-light" => Some(simple_rule(base_selector, "font-weight:300;", important)), "font-normal" => Some(simple_rule(base_selector, "font-weight:400;", important)), "font-medium" => Some(simple_rule(base_selector, "font-weight:500;", important)), "font-semibold" => Some(simple_rule(base_selector, "font-weight:600;", important)), "font-bold" => Some(simple_rule(base_selector, "font-weight:700;", important)), "font-extrabold" => Some(simple_rule(base_selector, "font-weight:800;", important)), "font-black" => Some(simple_rule(base_selector, "font-weight:900;", important)), "italic" => Some(simple_rule(base_selector, "font-style:italic;", important)), "text-left" => Some(simple_rule(base_selector, "text-align:left;", important)), "text-center" => Some(simple_rule(base_selector, "text-align:center;", important)), "text-right" => Some(simple_rule(base_selector, "text-align:right;", important)), "text-justify" => Some(simple_rule(base_selector, "text-align:justify;", important)), "leading-none" => Some(simple_rule(base_selector, "line-height:1;", important)), "leading-tight" => Some(simple_rule(base_selector, "line-height:1.25;", important)), "leading-snug" => Some(simple_rule(base_selector, "line-height:1.375;", important)), "leading-normal" => Some(simple_rule(base_selector, "line-height:1.5;", important)), "leading-relaxed" => Some(simple_rule(base_selector, "line-height:1.625;", important)), "tracking-tight" => Some(simple_rule(base_selector, "letter-spacing:-0.025em;", important)), "tracking-normal" => Some(simple_rule(base_selector, "letter-spacing:0;", important)), "tracking-wide" => Some(simple_rule(base_selector, "letter-spacing:0.025em;", important)), "tracking-wider" => Some(simple_rule(base_selector, "letter-spacing:0.05em;", important)), "tracking-widest" => Some(simple_rule(base_selector, "letter-spacing:0.1em;", important)), "border" => Some(simple_rule(base_selector, "border-width:1px;border-style:solid;", important)), "border-0" => Some(simple_rule(base_selector, "border-width:0;", important)), "border-2" => Some(simple_rule(base_selector, "border-width:2px;border-style:solid;", important)), "border-4" => Some(simple_rule(base_selector, "border-width:4px;border-style:solid;", important)), "border-t" => Some(simple_rule(base_selector, "border-top-width:1px;border-top-style:solid;", important)), "border-r" => Some(simple_rule(base_selector, "border-right-width:1px;border-right-style:solid;", important)), "border-b" => Some(simple_rule(base_selector, "border-bottom-width:1px;border-bottom-style:solid;", important)), "border-l" => Some(simple_rule(base_selector, "border-left-width:1px;border-left-style:solid;", important)), "border-x" => Some(simple_rule(base_selector, "border-left-width:1px;border-right-width:1px;border-left-style:solid;border-right-style:solid;", important)), "border-y" => Some(simple_rule(base_selector, "border-top-width:1px;border-bottom-width:1px;border-top-style:solid;border-bottom-style:solid;", important)), "border-dashed" => Some(simple_rule(base_selector, "border-style:dashed;", important)), "border-solid" => Some(simple_rule(base_selector, "border-style:solid;", important)), "relative" => Some(simple_rule(base_selector, "position:relative;", important)), "absolute" => Some(simple_rule(base_selector, "position:absolute;", important)), "fixed" => Some(simple_rule(base_selector, "position:fixed;", important)), "sticky" => Some(simple_rule(base_selector, "position:sticky;", important)), "min-h-screen" => Some(simple_rule(base_selector, "min-height:100vh;", important)), "h-full" => Some(simple_rule(base_selector, "height:100%;", important)), "w-full" => Some(simple_rule(base_selector, "width:100%;", important)), "w-auto" => Some(simple_rule(base_selector, "width:auto;", important)), "h-auto" => Some(simple_rule(base_selector, "height:auto;", important)), "overflow-hidden" => Some(simple_rule(base_selector, "overflow:hidden;", important)), "overflow-auto" => Some(simple_rule(base_selector, "overflow:auto;", important)), "overflow-scroll" => Some(simple_rule(base_selector, "overflow:scroll;", important)), "overflow-visible" => Some(simple_rule(base_selector, "overflow:visible;", important)), "overflow-x-auto" => Some(simple_rule(base_selector, "overflow-x-auto;", important)), "overflow-y-auto" => Some(simple_rule(base_selector, "overflow-y-auto;", important)), "overflow-x-hidden" => Some(simple_rule(base_selector, "overflow-x:hidden;", important)), "overflow-y-hidden" => Some(simple_rule(base_selector, "overflow-y:hidden;", important)), "whitespace-normal" => Some(simple_rule(base_selector, "white-space:normal;", important)), "whitespace-nowrap" => Some(simple_rule(base_selector, "white-space:nowrap;", important)), "transition" => Some(simple_rule(base_selector, "transition-property:all;transition-duration:150ms;transition-timing-function:cubic-bezier(0.4,0,0.2,1);", important)), "transition-all" => Some(simple_rule(base_selector, "transition-property:all;transition-duration:150ms;transition-timing-function:cubic-bezier(0.4,0,0.2,1);", important)), "transition-colors" => Some(simple_rule(base_selector, "transition-property:background-color,border-color,color,fill,stroke;transition-duration:150ms;transition-timing-function:cubic-bezier(0.4,0,0.2,1);", important)), "transition-none" => Some(simple_rule(base_selector, "transition-property:none;", important)), "transition-opacity" => Some(simple_rule(base_selector, "transition-property:opacity;transition-duration:150ms;transition-timing-function:cubic-bezier(0.4,0,0.2,1);", important)), "transition-transform" => Some(simple_rule(base_selector, "transition-property:transform;transition-duration:150ms;transition-timing-function:cubic-bezier(0.4,0,0.2,1);", important)), "cursor-pointer" => Some(simple_rule(base_selector, "cursor:pointer;", important)), "cursor-default" => Some(simple_rule(base_selector, "cursor:default;", important)), "uppercase" => Some(simple_rule(base_selector, "text-transform:uppercase;", important)), "lowercase" => Some(simple_rule(base_selector, "text-transform:lowercase;", important)), "capitalize" => Some(simple_rule(base_selector, "text-transform:capitalize;", important)), "underline" => Some(simple_rule(base_selector, "text-decoration:underline;", important)), "inline-flex" => Some(simple_rule(base_selector, "display:inline-flex;", important)), "list-disc" => Some(simple_rule(base_selector, "list-style-type:disc;", important)), "list-inside" => Some(simple_rule(base_selector, "list-style-position:inside;", important)), "break-words" => Some(simple_rule(base_selector, "overflow-wrap:break-word;", important)), "appearance-none" => Some(simple_rule(base_selector, "appearance:none;", important)), "backdrop-blur-sm" => Some(simple_rule(base_selector, "backdrop-filter:blur(4px);", important)), "antialiased" => Some(simple_rule(base_selector, "-webkit-font-smoothing:antialiased;-moz-osx-font-smoothing:grayscale;", important)), "pointer-events-none" => Some(simple_rule(base_selector, "pointer-events:none;", important)), "pointer-events-auto" => Some(simple_rule(base_selector, "pointer-events:auto;", important)), "select-none" => Some(simple_rule(base_selector, "user-select:none;", important)), "fill-current" => Some(simple_rule(base_selector, "fill:currentColor;", important)), "align-top" => Some(simple_rule(base_selector, "vertical-align:top;", important)), "align-middle" => Some(simple_rule(base_selector, "vertical-align:middle;", important)), "resize-y" => Some(simple_rule(base_selector, "resize:vertical;", important)), "touch-pan-y" => Some(simple_rule(base_selector, "touch-action:pan-y;", important)), "tabular-nums" => Some(simple_rule(base_selector, "font-variant-numeric:tabular-nums;", important)), "sr-only" => Some(simple_rule(base_selector, "position:absolute;width:1px;height:1px;padding:0;margin:-1px;overflow:hidden;clip:rect(0,0,0,0);white-space:nowrap;border-width:0;", important)), "prose-sm" => Some(simple_rule(base_selector, "font-size:0.875rem;line-height:1.7142857;", important)), "bg-center" => Some(simple_rule(base_selector, "background-position:center;", important)), "bg-bottom" => Some(simple_rule(base_selector, "background-position:bottom;", important)), "bg-repeat" => Some(simple_rule(base_selector, "background-repeat:repeat;", important)), "bg-repeat-x" => Some(simple_rule(base_selector, "background-repeat:repeat-x;", important)), "bg-repeat-y" => Some(simple_rule(base_selector, "background-repeat:repeat-y;", important)), "bg-no-repeat" => Some(simple_rule(base_selector, "background-repeat:no-repeat;", important)), "bg-cover" => Some(simple_rule(base_selector, "background-size:cover;", important)), "bg-contain" => Some(simple_rule(base_selector, "background-size:contain;", important)), "bg-gradient-to-r" => Some(simple_rule(base_selector, "background-image:linear-gradient(to right,var(--tw-gradient-from),var(--tw-gradient-to));", important)), "bg-gradient-to-b" => Some(simple_rule(base_selector, "background-image:linear-gradient(to bottom,var(--tw-gradient-from),var(--tw-gradient-to));", important)), "bg-gradient-to-br" => Some(simple_rule(base_selector, "background-image:linear-gradient(to bottom right,var(--tw-gradient-from),var(--tw-gradient-to));", important)), "outline-none" => Some(simple_rule(base_selector, "outline:2px solid transparent;outline-offset:2px;", important)), "outline-hidden" => Some(simple_rule(base_selector, "outline:none;", important)), "ring" => Some(simple_rule(base_selector, "box-shadow:0 0 0 1px rgba(59,130,246,0.5);", important)), "ring-1" => Some(simple_rule(base_selector, "box-shadow:0 0 0 1px rgba(59,130,246,0.5);", important)), "ring-2" => Some(simple_rule(base_selector, "box-shadow:0 0 0 2px rgba(59,130,246,0.5);", important)), "ring-4" => Some(simple_rule(base_selector, "box-shadow:0 0 0 4px rgba(59,130,246,0.5);", important)), "max-w-none" => Some(simple_rule(base_selector, "max-width:none;", important)), "w-px" => Some(simple_rule(base_selector, "width:1px;", important)), "size-full" => Some(simple_rule(base_selector, "width:100%;height:100%;", important)), "list-decimal" => Some(simple_rule(base_selector, "list-style-type:decimal;", important)), "animate-spin" => Some(UtilityRule { selector: base_selector.to_string(), declarations: maybe_important("animation:zebflow-spin 1s linear infinite;", important), prelude: Some("@keyframes zebflow-spin{to{transform:rotate(360deg);}}".to_string()) }), "animate-ping" => Some(UtilityRule { selector: base_selector.to_string(), declarations: maybe_important("animation:zebflow-ping 1s cubic-bezier(0,0,0.2,1) infinite;", important), prelude: Some("@keyframes zebflow-ping{75%,100%{transform:scale(2);opacity:0;}}".to_string()) }), "animate-pulse" => Some(UtilityRule { selector: base_selector.to_string(), declarations: maybe_important("animation:zebflow-pulse 2s cubic-bezier(0.4,0,0.6,1) infinite;", important), prelude: Some("@keyframes zebflow-pulse{0%,100%{opacity:1;}50%{opacity:.5;}}".to_string()) }), "animate-bounce" => Some(UtilityRule { selector: base_selector.to_string(), declarations: maybe_important("animation:zebflow-bounce 1s infinite;", important), prelude: Some("@keyframes zebflow-bounce{0%,100%{transform:translateY(-25%);animation-timing-function:cubic-bezier(.8,0,1,1);}50%{transform:none;animation-timing-function:cubic-bezier(0,0,.2,1);}}".to_string()) }), _ => None }
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
        "sm" => Some("24rem".to_string()),
        "md" => Some("28rem".to_string()),
        "lg" => Some("32rem".to_string()),
        "xl" => Some("36rem".to_string()),
        "2xl" => Some("42rem".to_string()),
        "3xl" => Some("48rem".to_string()),
        "4xl" => Some("56rem".to_string()),
        "5xl" => Some("64rem".to_string()),
        "6xl" => Some("72rem".to_string()),
        "screen-md" => Some("768px".to_string()),
        "screen-lg" => Some("1024px".to_string()),
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
        _ => None,
    }
}
fn border_rule(v: &str) -> Option<String> {
    match v { "0" => Some("border-width:0;".to_string()), "2" => Some("border-width:2px;border-style:solid;".to_string()), "4" => Some("border-width:4px;border-style:solid;".to_string()), "t" => Some("border-top-width:1px;border-top-style:solid;".to_string()), "r" => Some("border-right-width:1px;border-right-style:solid;".to_string()), "b" => Some("border-bottom-width:1px;border-bottom-style:solid;".to_string()), "l" => Some("border-left-width:1px;border-left-style:solid;".to_string()), "x" => Some("border-left-width:1px;border-right-width:1px;border-left-style:solid;border-right-style:solid;".to_string()), "y" => Some("border-top-width:1px;border-bottom-width:1px;border-top-style:solid;border-bottom-style:solid;".to_string()), "dashed" => Some("border-style:dashed;".to_string()), "solid" => Some("border-style:solid;".to_string()), _ => color_value(v).map(|c| format!("border-color:{};", c)) }
}
fn outline_rule(v: &str) -> Option<String> {
    match v {
        "none" => Some("outline:2px solid transparent;outline-offset:2px;".to_string()),
        "hidden" => Some("outline:none;".to_string()),
        _ => {
            if let Ok(px) = v.parse::<u32>() {
                Some(format!("outline-width:{}px;outline-style:solid;", px))
            } else {
                color_value(v).map(|c| format!("outline-color:{};outline-style:solid;", c))
            }
        }
    }
}
fn ring_rule(v: &str) -> Option<String> {
    if let Some(raw) = arbitrary_value(v) {
        if raw.ends_with("px") || raw.ends_with("rem") || raw.ends_with("em") {
            return Some(format!("box-shadow:0 0 0 {} rgba(59,130,246,0.5);", raw));
        }
        if let Ok(px) = raw.parse::<u32>() {
            return Some(format!("box-shadow:0 0 0 {}px rgba(59,130,246,0.5);", px));
        }
    }
    if let Ok(px) = v.parse::<u32>() {
        return Some(format!("box-shadow:0 0 0 {}px rgba(59,130,246,0.5);", px));
    }
    color_value(v).map(|c| format!("box-shadow:0 0 0 3px {};", c))
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
        format!("var(--zebflow-color-{},{})", k, k)
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
    if v.is_empty() {
        return false;
    }
    v.chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
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
        "green-100" => Some("#dcfce7"),
        "green-200" => Some("#bbf7d0"),
        "green-300" => Some("#86efac"),
        "green-400" => Some("#4ade80"),
        "blue-400" => Some("#60a5fa"),
        "blue-500" => Some("#3b82f6"),
        "blue-600" => Some("#2563eb"),
        "blue-700" => Some("#1d4ed8"),
        "blue-800" => Some("#1e40af"),
        "blue-900" => Some("#1e3a8a"),
        "blue-950" => Some("#172554"),
        "green-50" => Some("#f0fdf4"),
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
        "yellow-50" => Some("#fefce8"),
        "yellow-500" => Some("#eab308"),
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
        "purple-500" => Some("#a855f7"),
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
    use super::{minify_css_lossy, process_tailwind};

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
        let out = process_tailwind(html);
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
}
