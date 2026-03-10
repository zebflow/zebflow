//! JavaScript/TypeScript string literal masker.
//!
//! # Why this exists
//!
//! The RWE bundler inlines multiple component files into a single flat module.
//! Line-based transforms (`localize_exports`, `prefix_module_locals`) operate
//! on the raw source text. Without masking, code-looking text inside string
//! literals can confuse those transforms — e.g. a string containing
//! `"export default function"` would be misidentified as a real export.
//!
//! # How it works
//!
//! We use OXC's own parser to identify the **exact byte spans** of every
//! `StringLiteral` and `TemplateLiteral` in the source. Only the CONTENT
//! of those literals (between the delimiters) is replaced with placeholder
//! tokens `__RWE_MASK_n__`. Everything else — comments, regex, JSX text
//! content, code — is left untouched.
//!
//! This is correct by construction: OXC already knows all JavaScript lexical
//! contexts (comments, regex, JSX text, strings, template literals). We
//! piggyback on that knowledge instead of reimplementing a half-baked lexer.
//!
//! # Fallback
//!
//! If OXC fails to parse the source (e.g. because `strip_local_imports`
//! removed an import creating an unresolved reference that somehow causes
//! a parse panic), we fall back to a simple heuristic masker that handles
//! the common cases. This should rarely happen in practice.

use oxc_allocator::Allocator;
use oxc_ast::ast::{StringLiteral, TemplateLiteral};
use oxc_parser::Parser;
use oxc_span::SourceType;

/// Mask the contents of string and template literals in a JavaScript/TypeScript
/// source file. Returns the masked source and a list of original contents
/// that can be restored with [`unmask`].
pub fn mask(source: &str) -> (String, Vec<String>) {
    // Try OXC-based masking first (correct for all JS/TS/JSX contexts).
    if let Some(result) = mask_with_oxc(source) {
        return result;
    }
    // Fallback: simple heuristic masker for cases where OXC can't parse.
    mask_heuristic(source)
}

/// Restore masked string contents after line-based transforms.
pub fn unmask(source: &str, masks: &[String]) -> String {
    let mut result = source.to_string();
    for (idx, content) in masks.iter().enumerate() {
        result = result.replace(&format!("__RWE_MASK_{idx}__"), content);
    }
    result
}

// ── OXC-based masking ────────────────────────────────────────────────────────

/// Collect byte-offset spans of all string/template literal contents using
/// OXC's parser. Returns None if parsing fails.
fn mask_with_oxc(source: &str) -> Option<(String, Vec<String>)> {
    let alloc = Allocator::default();
    let source_type = SourceType::default()
        .with_module(true)
        .with_jsx(true)
        .with_typescript(true);

    let parsed = Parser::new(&alloc, source, source_type).parse();
    if parsed.panicked {
        return None;
    }
    // We allow parse errors (e.g. undefined references after import stripping)
    // — the AST is still usable for span collection.

    let mut collector = SpanCollector { spans: Vec::new() };
    oxc_ast_visit::walk::walk_program(&mut collector, &parsed.program);

    // Sort spans by start position and deduplicate.
    collector.spans.sort_by_key(|s| s.0);
    collector.spans.dedup();

    // Build the masked source by replacing literal contents with placeholders.
    let mut masks: Vec<String> = Vec::new();
    let mut result = String::with_capacity(source.len());
    let mut cursor: usize = 0;

    for (start, end) in &collector.spans {
        let s = *start as usize;
        let e = *end as usize;

        if s > source.len() || e > source.len() || s >= e {
            continue;
        }

        // Push everything before this span verbatim.
        if s > cursor {
            result.push_str(&source[cursor..s]);
        }

        // The span includes delimiters. We want to keep delimiters and mask
        // only the content between them.
        let span_text = &source[s..e];

        if span_text.starts_with('`') {
            // Template literal: `content`
            // Keep opening `, mask content, keep closing `
            result.push('`');
            let inner = &span_text[1..span_text.len().saturating_sub(1)];
            let idx = masks.len();
            masks.push(inner.to_string());
            result.push_str(&format!("__RWE_MASK_{idx}__"));
            if span_text.ends_with('`') {
                result.push('`');
            }
        } else if span_text.starts_with('"') || span_text.starts_with('\'') {
            // String literal: "content" or 'content'
            let quote = span_text.chars().next().unwrap();
            result.push(quote);
            let inner = &span_text[1..span_text.len().saturating_sub(1)];
            let idx = masks.len();
            masks.push(inner.to_string());
            result.push_str(&format!("__RWE_MASK_{idx}__"));
            if span_text.ends_with(quote) {
                result.push(quote);
            }
        } else {
            // Unknown delimiter — push as-is (shouldn't happen).
            result.push_str(span_text);
        }

        cursor = e;
    }

    // Push remaining source after the last span.
    if cursor < source.len() {
        result.push_str(&source[cursor..]);
    }

    Some((result, masks))
}

/// Visitor that collects byte spans of StringLiteral and TemplateLiteral nodes.
struct SpanCollector {
    spans: Vec<(u32, u32)>,
}

impl<'a> oxc_ast_visit::Visit<'a> for SpanCollector {
    fn visit_string_literal(&mut self, lit: &StringLiteral<'a>) {
        self.spans.push((lit.span.start, lit.span.end));
    }

    fn visit_template_literal(&mut self, lit: &TemplateLiteral<'a>) {
        // Mask the entire template literal (including expressions).
        // Expressions inside ${...} are unlikely to contain patterns that
        // confuse localize_exports/prefix_module_locals, and masking the
        // whole thing is safer than trying to mask only quasis.
        self.spans.push((lit.span.start, lit.span.end));

        // Do NOT recurse into children — we've masked the entire thing.
    }
}

// ── Heuristic fallback masker ────────────────────────────────────────────────

/// Simple character-by-character masker that handles common cases.
/// Used as a fallback when OXC can't parse the source.
fn mask_heuristic(source: &str) -> (String, Vec<String>) {
    let mut masks: Vec<String> = Vec::new();
    let mut result = String::with_capacity(source.len());
    let chars: Vec<char> = source.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Skip single-line comments: // ...
        if chars[i] == '/' && i + 1 < len && chars[i + 1] == '/' {
            result.push(chars[i]);
            i += 1;
            result.push(chars[i]);
            i += 1;
            while i < len && chars[i] != '\n' {
                result.push(chars[i]);
                i += 1;
            }
            if i < len {
                result.push(chars[i]); // push the \n
            }
            i += 1;
            continue;
        }
        // Skip block comments: /* ... */
        if chars[i] == '/' && i + 1 < len && chars[i + 1] == '*' {
            result.push(chars[i]);
            i += 1;
            result.push(chars[i]);
            i += 1;
            while i < len {
                if chars[i] == '*' && i + 1 < len && chars[i + 1] == '/' {
                    result.push(chars[i]);
                    i += 1;
                    result.push(chars[i]);
                    i += 1;
                    break;
                }
                result.push(chars[i]);
                i += 1;
            }
            continue;
        }
        // Skip regex literals: /pattern/flags
        if chars[i] == '/' && i + 1 < len && chars[i + 1] != '/' && chars[i + 1] != '*' {
            let prev_significant = result.chars().rev()
                .find(|c| !c.is_ascii_whitespace());
            let is_regex = match prev_significant {
                None => true,
                Some(c) => matches!(c,
                    '(' | '=' | '[' | '!' | '&' | '|' | '?' | ':' | ','
                    | ';' | '{' | '}' | '~' | '^' | '%' | '<' | '>' | '+'
                    | '-' | '*' | '/' | '\n'
                ),
            };
            if is_regex {
                result.push(chars[i]);
                i += 1;
                while i < len {
                    if chars[i] == '\\' {
                        result.push(chars[i]);
                        i += 1;
                        if i < len {
                            result.push(chars[i]);
                            i += 1;
                        }
                    } else if chars[i] == '/' {
                        result.push(chars[i]);
                        i += 1;
                        while i < len && chars[i].is_ascii_alphabetic() {
                            result.push(chars[i]);
                            i += 1;
                        }
                        break;
                    } else if chars[i] == '\n' {
                        break;
                    } else {
                        result.push(chars[i]);
                        i += 1;
                    }
                }
                continue;
            }
        }
        match chars[i] {
            '"' | '\'' => {
                let quote = chars[i];
                let mut content = String::new();
                i += 1;
                while i < len {
                    let ch = chars[i];
                    if ch == '\\' {
                        content.push(ch);
                        i += 1;
                        if i < len {
                            content.push(chars[i]);
                        }
                    } else if ch == quote {
                        break;
                    } else if ch == '\n' {
                        content.push(ch);
                        break;
                    } else {
                        content.push(ch);
                    }
                    i += 1;
                }
                let idx = masks.len();
                masks.push(content);
                result.push(quote);
                result.push_str(&format!("__RWE_MASK_{idx}__"));
                result.push(quote);
            }
            '`' => {
                let mut content = String::new();
                i += 1;
                let mut depth = 0i32;
                while i < len {
                    let ch = chars[i];
                    if ch == '\\' {
                        content.push(ch);
                        i += 1;
                        if i < len {
                            content.push(chars[i]);
                        }
                    } else if ch == '$' && i + 1 < len && chars[i + 1] == '{' {
                        depth += 1;
                        content.push(ch);
                        i += 1;
                        content.push(chars[i]);
                    } else if ch == '}' {
                        if depth > 0 {
                            depth -= 1;
                        }
                        content.push(ch);
                    } else if ch == '`' && depth == 0 {
                        break;
                    } else {
                        content.push(ch);
                    }
                    i += 1;
                }
                let idx = masks.len();
                masks.push(content);
                result.push('`');
                result.push_str(&format!("__RWE_MASK_{idx}__"));
                result.push('`');
            }
            c => result.push(c),
        }
        i += 1;
    }

    (result, masks)
}
