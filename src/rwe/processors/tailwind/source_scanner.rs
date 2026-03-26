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
