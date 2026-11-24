//! OXC-based static source scanner for Tailwind-like token discovery.
//!
//! Scans all string literals in the bundled JS/TSX source to find class tokens
//! that would be invisible to the HTML-only scanner (e.g. components that
//! return null during SSR, conditional branches not taken, dynamic ternaries).
//!
//! Uses the same `token_css_rule` validity gate as the HTML scanner — only
//! recognised utility tokens are included; arbitrary strings are silently dropped.

use std::collections::HashSet;

use oxc_allocator::Allocator;
use oxc_ast::ast::{StringLiteral, TemplateLiteral};
use oxc_parser::Parser;
use oxc_span::SourceType;

use crate::rwe::class_notation::extract_tailwind_tokens_from_class_value;

use super::compiler::token_css_rule;

struct StringLiteralCollector {
    candidates: Vec<String>,
}

impl<'a> oxc_ast_visit::Visit<'a> for StringLiteralCollector {
    fn visit_string_literal(&mut self, lit: &StringLiteral<'a>) {
        self.candidates.push(lit.value.to_string());
    }

    fn visit_template_literal(&mut self, lit: &TemplateLiteral<'a>) {
        for quasi in &lit.quasis {
            let s = quasi.value.raw.as_str();
            if !s.trim().is_empty() {
                self.candidates.push(s.to_string());
            }
        }
        // Recurse into expressions so nested string literals are visited too.
        oxc_ast_visit::walk::walk_template_literal(self, lit);
    }
}

/// Scan all string / template literals in the bundled JS source and return the
/// set of valid Tailwind-like tokens found.
///
/// Gracefully degrades: if OXC cannot parse the source an empty set is returned
/// and the HTML scanner result remains the sole source of tokens.
/// [`collect_source_tokens`], remembered by the source's hash.
///
/// The tokens depend on the compiled page alone, and a page renders many times
/// per compile: parsing a Studio page's 600 KB bundle on every request cost
/// more than rendering it. At most [`CACHED_SOURCES`] bundles are kept; past
/// that the cache starts over, so memory stays bounded however many pages a
/// server renders.
pub fn collect_source_tokens_cached(js_source: &str) -> std::sync::Arc<HashSet<String>> {
    use std::hash::{Hash, Hasher};
    static CACHE: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<(u64, usize), std::sync::Arc<HashSet<String>>>>> =
        std::sync::LazyLock::new(Default::default);
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    js_source.hash(&mut hasher);
    let key = (hasher.finish(), js_source.len());
    if let Some(found) = CACHE.lock().unwrap_or_else(|e| e.into_inner()).get(&key) {
        return found.clone();
    }
    let tokens = std::sync::Arc::new(collect_source_tokens(js_source));
    let mut cache = CACHE.lock().unwrap_or_else(|e| e.into_inner());
    if cache.len() >= CACHED_SOURCES {
        cache.clear();
    }
    cache.insert(key, tokens.clone());
    tokens
}

/// How many page bundles' tokens [`collect_source_tokens_cached`] keeps.
const CACHED_SOURCES: usize = 512;

pub fn collect_source_tokens(js_source: &str) -> HashSet<String> {
    let alloc = Allocator::default();
    let source_type = SourceType::default()
        .with_module(true)
        .with_jsx(true)
        .with_typescript(true);

    let parsed = Parser::new(&alloc, js_source, source_type).parse();
    if parsed.panicked {
        return HashSet::new();
    }

    let mut collector = StringLiteralCollector {
        candidates: Vec::new(),
    };
    oxc_ast_visit::walk::walk_program(&mut collector, &parsed.program);

    let mut tokens = HashSet::new();
    for candidate in collector.candidates {
        for token in extract_tailwind_tokens_from_class_value(&candidate) {
            if token_css_rule(&token).is_some() {
                tokens.insert(token);
            }
        }
    }
    tokens
}
