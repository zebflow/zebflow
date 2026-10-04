//! A template's source checked on its own, before anything compiles it.
//!
//! Problems a 200 hides: a file that does not parse (the page answers
//! `<!-- RWE component error -->`); a binding a file uses without declaring
//! or importing it — the compiler inlines component files into one flat
//! bundle, so it still runs, borrowing the entry page's, until the page stops
//! holding that binding and the browser throws `X is not defined`, with
//! nothing failing at build time; and a local named `h`, which shadows the
//! JSX factory.
//!
//! The platform's own templates are held to this by
//! `tests/rwe/platform_templates_parse.rs`; a project file is checked when an
//! agent writes it (`file_write`), with the line of each problem.

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::{GetSpan, SourceType};

/// Names a template file may use without declaring them: the browser and
/// the runtime provide these.
pub const AMBIENT: &[&str] = &[
    // Browser
    "window", "document", "console", "fetch", "navigator", "location", "history", "localStorage",
    "sessionStorage", "setTimeout", "clearTimeout", "setInterval", "clearInterval",
    "requestAnimationFrame", "cancelAnimationFrame", "queueMicrotask", "structuredClone",
    "alert", "confirm", "prompt", "getComputedStyle", "matchMedia", "IntersectionObserver",
    "MutationObserver", "ResizeObserver", "CustomEvent", "Event", "AbortController", "Image",
    "FileReader", "WebSocket", "crypto", "btoa", "atob", "URL", "URLSearchParams", "FormData",
    "Blob", "File", "Response", "Request", "Headers", "DOMParser", "HTMLElement", "Node", "Element",
    // Language
    "globalThis", "undefined", "NaN", "Infinity", "Object", "Array", "String", "Number", "Boolean",
    "Symbol", "BigInt", "Math", "JSON", "Date", "RegExp", "Error", "TypeError", "RangeError",
    "Promise", "Map", "Set", "WeakMap", "WeakSet", "Proxy", "Reflect", "Intl", "parseInt",
    "parseFloat", "isNaN", "isFinite", "encodeURIComponent", "decodeURIComponent", "encodeURI",
    "decodeURI", "performance", "unescape", "escape", "Uint8Array", "Int8Array", "Uint16Array", "Uint32Array", "Float32Array",
    "Float64Array", "ArrayBuffer", "DataView", "TextEncoder", "TextDecoder", "process",
    // Platform runtime
    "h", "Fragment", "input",
];

/// One problem in a template's source, at a 1-based line and column.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct SourceProblem {
    pub line: usize,
    pub column: usize,
    pub message: String,
}

impl std::fmt::Display for SourceProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.column, self.message)
    }
}

/// The files this check reads: TypeScript, with JSX in `.tsx`.
pub fn is_template_path(path: &str) -> bool {
    path.ends_with(".tsx") || path.ends_with(".ts")
}

fn line_col(source: &str, offset: usize) -> (usize, usize) {
    let offset = offset.min(source.len());
    let before = &source[..offset];
    let line = before.matches('\n').count() + 1;
    let column = before.rsplit('\n').next().map(|l| l.chars().count()).unwrap_or(0) + 1;
    (line, column)
}

/// Every parse error (the first three: one broken construct cascades),
/// every lowercase name used but never declared or imported (a capitalised
/// name is a type or a component from a declaration file; only value
/// bindings are the hazard), and in a `.tsx` file (`tsx`) a binding named
/// `h`, which shadows the JSX factory.
pub fn check_template_source(source: &str, tsx: bool) -> Vec<SourceProblem> {
    let allocator = Allocator::default();
    let source_type = SourceType::default().with_module(true).with_jsx(true).with_typescript(true);
    let parsed = Parser::new(&allocator, source, source_type).parse();
    let mut problems = Vec::new();
    for error in parsed.errors.iter().take(3) {
        let offset = error.labels.as_ref().and_then(|l| l.first().map(|s| s.offset())).unwrap_or(0);
        let (line, column) = line_col(source, offset);
        problems.push(SourceProblem { line, column, message: format!("does not parse: {}", error.message) });
    }
    if parsed.panicked {
        if problems.is_empty() {
            problems.push(SourceProblem { line: 1, column: 1, message: "does not parse (the parser gave up without a diagnostic)".to_string() });
        }
        return problems;
    }
    if !problems.is_empty() {
        return problems;
    }

    let semantic = oxc_semantic::SemanticBuilder::new().build(&parsed.program).semantic;
    let scoping = semantic.scoping();
    let mut borrowed: Vec<SourceProblem> = scoping
        .root_unresolved_references()
        .iter()
        .filter(|(name, _)| {
            let name: &str = name;
            // `as const` reads to the analyzer as a reference to `const`.
            !AMBIENT.contains(&name) && name != "const" && name.starts_with(|c: char| c.is_lowercase())
        })
        .map(|(name, references)| {
            let offset = references
                .first()
                .map(|&id| semantic.nodes().get_node(scoping.get_reference(id).node_id()).kind().span().start as usize)
                .unwrap_or(0);
            let (line, column) = line_col(source, offset);
            SourceProblem {
                line,
                column,
                message: format!("`{name}` is used but never declared or imported — import it (or pass it as a prop); a component never borrows the page's"),
            }
        })
        .collect();
    // Every JSX element lowers to a call of `h(...)`: a local `h`
    // (`hosts.map((h) => <option>{h}</option>)`) shadows it and the page
    // answers `TypeError: h is not a function` with a 200.
    if tsx {
        for symbol in scoping.symbol_ids().filter(|&symbol| scoping.symbol_name(symbol) == "h") {
            let (line, column) = line_col(source, scoping.symbol_span(symbol).start as usize);
            borrowed.push(SourceProblem {
                line,
                column,
                message: "declares a binding named `h`, the JSX factory every element calls — rename it".to_string(),
            });
        }
    }
    borrowed.sort_by_key(|p| (p.line, p.column));
    problems.extend(borrowed);
    problems
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_broken_file_reports_its_line() {
        let found = check_template_source("export default function Page() {\n  return <div>\n}\n", true);
        assert!(!found.is_empty());
        assert!(found[0].message.starts_with("does not parse"), "{found:?}");
        assert!(found[0].line >= 2, "{found:?}");
    }

    #[test]
    fn a_borrowed_binding_is_named_with_its_line() {
        let source = "import { useState } from \"zeb/react\";\n\nexport default function Card() {\n  const [open] = useState(false);\n  return <div class={cx(\"a\", open && \"b\")}>{input.title}</div>;\n}\n";
        let found = check_template_source(source, true);
        assert_eq!(found.len(), 1, "{found:?}");
        assert_eq!((found[0].line, found[0].column), (5, 22), "{found:?}");
        assert!(found[0].message.starts_with("`cx` is used but never declared"), "{found:?}");
    }

    #[test]
    fn a_clean_file_has_nothing_to_report() {
        let source = "import { useState } from \"zeb/react\";\nexport default function Page() {\n  const [n] = useState(0);\n  return <p>{n}</p>;\n}\n";
        assert!(check_template_source(source, true).is_empty());
    }

    #[test]
    fn a_local_named_h_shadows_the_jsx_factory() {
        let source = "export default function Hosts(props) {\n  return <select>{props.hosts.map((h) => <option>{h}</option>)}</select>;\n}\n";
        let found = check_template_source(source, true);
        assert_eq!(found.len(), 1, "{found:?}");
        assert!(found[0].message.contains("binding named `h`"), "{found:?}");
        assert_eq!(found[0].line, 2);
        // A `.ts` helper may call its hours `h`.
        assert!(check_template_source("export const hours = (h: number) => h * 60;\n", false).is_empty());
    }
}
