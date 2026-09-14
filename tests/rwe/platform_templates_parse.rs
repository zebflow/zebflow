//! Every platform template must parse, and must declare what it uses.
//!
//! The server compiles all of them at startup and panics on the first one that
//! does not, which costs a full rebuild to discover and names only the page,
//! not the line. This test reads the same files straight from the source tree
//! and reports every broken one at once, in about a second.

use std::path::{Path, PathBuf};

use oxc_allocator::Allocator;
use oxc_parser::Parser;
use oxc_span::SourceType;

fn template_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "node_modules") {
                continue;
            }
            template_files(&path, out);
        } else if path
            .extension()
            .is_some_and(|e| e == "tsx" || e == "ts")
        {
            out.push(path);
        }
    }
}

#[test]
fn every_platform_template_parses() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/platform/web/templates");
    let mut files = Vec::new();
    template_files(&root, &mut files);
    assert!(!files.is_empty(), "no templates found under {}", root.display());
    files.sort();

    let mut broken: Vec<String> = Vec::new();
    for path in &files {
        let source = std::fs::read_to_string(path).expect("read template");
        let allocator = Allocator::default();
        let source_type = SourceType::default()
            .with_module(true)
            .with_jsx(true)
            .with_typescript(true);
        let parsed = Parser::new(&allocator, &source, source_type).parse();

        let rel = path.strip_prefix(&root).unwrap_or(path).display().to_string();
        // A panic still leaves the errors it collected, and those carry the
        // line the reader has to go and look at — worth printing either way.
        let mut reported = 0;
        for error in &parsed.errors {
            broken.push(format!("{rel}: {error}"));
            reported += 1;
            // One broken construct cascades; the first few name the cause.
            if reported >= 3 {
                break;
            }
        }
        if parsed.panicked && reported == 0 {
            broken.push(format!("{rel}: parser panicked with no diagnostic"));
        }
    }

    assert!(
        broken.is_empty(),
        "{} template(s) do not parse:\n{}",
        broken.len(),
        broken.join("\n")
    );
}


/// Names a template file may use without declaring them: the browser and
/// runtime provide these.
const AMBIENT: &[&str] = &[
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

/// Every name a template file uses must be declared in it or imported into it.
///
/// The RWE compiler inlines component files into one flat bundle, so a
/// component that uses a binding it never imported still runs — it borrows the
/// entry page's. That works until the page stops holding that binding, at
/// which point the component throws `X is not defined` in the browser, with
/// nothing failing at build time. This is the check that turns that into a
/// test failure.
#[test]
fn no_template_borrows_a_binding_it_never_declared() {
    use oxc_semantic::SemanticBuilder;

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/platform/web/templates");
    let mut files = Vec::new();
    template_files(&root, &mut files);
    files.sort();

    let ambient: std::collections::HashSet<&str> = AMBIENT.iter().copied().collect();
    let mut borrowed: Vec<String> = Vec::new();

    for path in &files {
        let source = std::fs::read_to_string(path).expect("read template");
        let allocator = Allocator::default();
        let source_type = SourceType::default()
            .with_module(true)
            .with_jsx(true)
            .with_typescript(true);
        let parsed = Parser::new(&allocator, &source, source_type).parse();
        if parsed.panicked {
            continue; // the parse test above already reports this file
        }

        let semantic = SemanticBuilder::new().build(&parsed.program).semantic;
        let rel = path.strip_prefix(&root).unwrap_or(path).display().to_string();

        let mut names: Vec<String> = semantic
            .scoping()
            .root_unresolved_references()
            .iter()
            .map(|(name, _)| name.to_string())
            .filter(|name| !ambient.contains(name.as_str()))
            // `as const` reads to the analyzer as a reference to `const`.
            .filter(|name| name != "const")
            // A capitalised name is a type or a component from a d.ts-style
            // declaration; only lowercase value bindings are the hazard here.
            .filter(|name| name.starts_with(|c: char| c.is_lowercase()))
            .collect();
        names.sort();

        if !names.is_empty() {
            borrowed.push(format!("{rel}: {}", names.join(", ")));
        }
    }

    assert!(
        borrowed.is_empty(),
        "{} template(s) use a binding they never declared:\n{}",
        borrowed.len(),
        borrowed.join("\n")
    );
}

/// No template may declare a binding named `h` where JSX appears.
///
/// The compiler lowers every JSX element to a call of `h(...)`. A local named
/// `h` — `hosts.map((h) => <option>{h}</option>)` — shadows the factory inside
/// its own scope, the server markup shows `<!-- RWE component error: TypeError:
/// h is not a function -->`, and the response is still 200. The Addressing
/// tab shipped exactly that; this refuses it at test time.
#[test]
fn no_template_declares_a_binding_named_h() {
    use oxc_semantic::SemanticBuilder;

    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/platform/web/templates");
    let mut files = Vec::new();
    template_files(&root, &mut files);
    files.sort();

    let mut offenders: Vec<String> = Vec::new();
    for path in &files {
        // Only a .tsx file can contain JSX; a `.ts` helper may call its
        // hours `h` in peace.
        if path.extension().and_then(|e| e.to_str()) != Some("tsx") {
            continue;
        }
        let source = std::fs::read_to_string(path).expect("read template");
        let allocator = Allocator::default();
        let source_type = SourceType::default()
            .with_module(true)
            .with_jsx(true)
            .with_typescript(true);
        let parsed = Parser::new(&allocator, &source, source_type).parse();
        if parsed.panicked {
            continue;
        }
        let semantic = SemanticBuilder::new().build(&parsed.program).semantic;
        let scoping = semantic.scoping();
        let shadows = scoping
            .symbol_ids()
            .any(|symbol| scoping.symbol_name(symbol) == "h");
        if shadows {
            let rel = path.strip_prefix(&root).unwrap_or(path).display().to_string();
            offenders.push(rel);
        }
    }
    assert!(
        offenders.is_empty(),
        "{} template(s) declare a binding named `h`, the JSX factory:\n{}",
        offenders.len(),
        offenders.join("\n")
    );
}
