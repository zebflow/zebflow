use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use oxc_allocator::Allocator;
use oxc_ast::ast::Statement;
use oxc_parser::Parser;
use oxc_span::SourceType;

use super::config::CompileOptions;
use super::error::EngineError;
use super::model::{CompiledTemplate, HydrateMode, ImportEdge};
use super::security;

const JSX_PRELUDE: &str = "/** @jsxImportSource npm:preact */\n";

pub fn compile(source: &str, options: CompileOptions) -> Result<CompiledTemplate, EngineError> {
    let alloc = Allocator::default();
    let source_type = source_type_from_options(&options);
    let parsed = Parser::new(&alloc, source, source_type).parse();

    if parsed.panicked {
        let _ = std::fs::write("/tmp/rwe-parse-failed.tsx", source);
        return Err(EngineError::new(
            "RWE_PARSE",
            "oxc parser panicked while parsing TSX",
        ));
    }
    if !parsed.errors.is_empty() {
        let _ = std::fs::write("/tmp/rwe-parse-failed.tsx", source);
        let msg = parsed
            .errors
            .iter()
            .map(|e| format!("{e:?}"))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(EngineError::new(
            "RWE_PARSE",
            format!("tsx parse errors: {msg}"),
        ));
    }

    ensure_default_export(&parsed.program)?;

    let mut diagnostics = security::analyze(source, &options.security)?;

    let raw_imports = collect_imports(&parsed.program);
    validate_zeb_exclusive_symbols(&parsed.program)?;
    validate_import_allowlist(&raw_imports, &options)?;

    let (rewritten_source, imports) =
        rewrite_imports(source, &raw_imports, &options, &mut diagnostics)?;

    let normalized_page_source = rewrite_page_root_tag(&rewritten_source);
    let bundled_server = bundle_for_client(&normalized_page_source, &imports)?;
    let transformed_server = format!("{}{}", JSX_PRELUDE, bundled_server);
    let bundled_client = bundle_for_client(&normalized_page_source, &imports)?;
    let transformed_client = format!("{}{}", JSX_PRELUDE, bundled_client);
    let hydrate_mode = detect_hydrate_mode(source);

    Ok(CompiledTemplate {
        engine: "rwe".to_string(),
        source_path: options.file_path.clone(),
        runtime_mode: options.runtime_mode.clone(),
        deno_timeout_ms: options.deno_timeout_ms,
        server_module_source: transformed_server,
        client_module_source: transformed_client,
        imports,
        diagnostics,
        hydrate_mode,
        compile_options: options,
    })
}


fn source_type_from_options(options: &CompileOptions) -> SourceType {
    if let Some(path) = &options.file_path {
        SourceType::from_path(path)
            .unwrap_or_default()
            .with_module(true)
            .with_jsx(true)
            .with_typescript(true)
    } else {
        SourceType::default()
            .with_module(true)
            .with_jsx(true)
            .with_typescript(true)
    }
}

fn ensure_default_export(program: &oxc_ast::ast::Program<'_>) -> Result<(), EngineError> {
    let has_default = program
        .body
        .iter()
        .any(|stmt| matches!(stmt, Statement::ExportDefaultDeclaration(_)));
    if has_default {
        Ok(())
    } else {
        Err(EngineError::new(
            "RWE_EXPORT_DEFAULT",
            "template must have one default export component",
        ))
    }
}

fn collect_imports(program: &oxc_ast::ast::Program<'_>) -> Vec<String> {
    let mut imports = Vec::new();
    for stmt in &program.body {
        if let Statement::ImportDeclaration(import) = stmt {
            imports.push(import.source.value.as_str().to_string());
        }
    }
    imports
}

fn validate_import_allowlist(
    imports: &[String],
    _options: &CompileOptions,
) -> Result<(), EngineError> {
    for import in imports {
        if import == "zeb" { continue; }
        if import.starts_with("zeb/") { continue; }
        if import.starts_with("@/") { continue; }
        // Absolute paths are the resolved form of @/ imports written to disk by
        // prepare_template_root() before compile() is called. Never user-authored.
        if import.starts_with('/') { continue; }
        return Err(EngineError::new(
            "RWE_IMPORT_NOT_ALLOWED",
            format!("import '{import}' is not allowed; only \"zeb\", \"zeb/*\", and \"@/\" imports are valid"),
        ));
    }
    Ok(())
}

const ZEB_EXCLUSIVE_SYMBOLS: &[&str] = &[
    "useState", "useEffect", "useRef", "useMemo", "useCallback",
    "useContext", "useReducer", "useId", "useLayoutEffect",
    "usePageState", "useNavigate",
    "cx", "Link", "forwardRef", "memo", "createContext", "Fragment",
];

fn validate_zeb_exclusive_symbols(
    program: &oxc_ast::ast::Program<'_>,
) -> Result<(), EngineError> {
    use oxc_ast::ast::ImportDeclarationSpecifier;
    for stmt in &program.body {
        if let Statement::ImportDeclaration(import) = stmt {
            let specifier = import.source.value.as_str();
            // "zeb", "zeb/*", and absolute paths (resolved form of @/ and "zeb" after
            // prepare_template_root rewrites them on disk) are all trusted.
            if specifier == "zeb" || specifier.starts_with("zeb/") || specifier.starts_with('/') {
                continue;
            }
            if let Some(specifiers) = &import.specifiers {
                for s in specifiers.iter() {
                    if let ImportDeclarationSpecifier::ImportSpecifier(named) = s {
                        let name = named.imported.name().as_str();
                        if ZEB_EXCLUSIVE_SYMBOLS.contains(&name) {
                            return Err(EngineError::new(
                                "RWE_IMPORT_ZEB_ONLY",
                                format!("'{name}' must be imported from \"zeb\", not \"{specifier}\""),
                            ));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

#[allow(dead_code)]
fn strip_runtime_imports(source: &str) -> String {
    source
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            if !trimmed.starts_with("import ") {
                return true;
            }
            !trimmed.contains("from 'rwe'")
                && !trimmed.contains("from \"rwe\"")
                && !trimmed.contains("from 'zeb'")
                && !trimmed.contains("from \"zeb\"")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn rewrite_imports(
    source: &str,
    imports: &[String],
    options: &CompileOptions,
    diagnostics: &mut Vec<super::model::Diagnostic>,
) -> Result<(String, Vec<ImportEdge>), EngineError> {
    let mut rewritten = source.to_string();
    let mut out = Vec::new();

    for import in imports {
        if import == "zeb" {
            continue;
        }

        let resolved = resolve_import(import, options)?;
        if let Some(path) = &resolved {
            rewritten = rewritten.replace(&format!("\"{import}\""), &format!("\"{path}\""));
            rewritten = rewritten.replace(&format!("'{import}'"), &format!("'{path}'"));
        }

        if import.starts_with("@/") && resolved.is_none() {
            diagnostics.push(super::model::Diagnostic {
                code: "RWE_IMPORT_UNRESOLVED".to_string(),
                message: format!("could not resolve alias import '{import}'"),
                line: None,
            });
        }

        out.push(ImportEdge {
            source: import.clone(),
            resolved,
        });
    }

    Ok((rewritten, out))
}

fn resolve_import(import: &str, options: &CompileOptions) -> Result<Option<String>, EngineError> {
    if import.starts_with("npm:")
        || import.starts_with("node:")
        || import.starts_with("jsr:")
        || import.starts_with("http://")
        || import.starts_with("https://")
    {
        return Ok(None);
    }

    if import.starts_with("@/") {
        let root = options.template_root.as_ref().ok_or_else(|| {
            EngineError::new(
                "RWE_TEMPLATE_ROOT",
                format!("template_root is required for alias import '{import}'"),
            )
        })?;
        let root_path = canonical_or_current(Path::new(root))?;
        let joined = root_path.join(import.trim_start_matches("@/"));
        let resolved = resolve_module_path(&joined)?;
        let final_path = normalize_path(&canonical_or_fallback(&resolved)?);
        ensure_within_root(&final_path, &root_path)?;
        return Ok(Some(final_path.to_string_lossy().to_string()));
    }

    if import.starts_with("./") || import.starts_with("../") {
        let file_path = options.file_path.as_ref().ok_or_else(|| {
            EngineError::new(
                "RWE_FILE_PATH",
                format!("file_path is required for relative import '{import}'"),
            )
        })?;
        let base = Path::new(file_path)
            .parent()
            .ok_or_else(|| EngineError::new("RWE_FILE_PATH", "invalid file_path"))?;
        let base = canonical_or_current(base)?;
        let joined = base.join(import);
        let resolved = resolve_module_path(&joined)?;
        let final_path = normalize_path(&canonical_or_fallback(&resolved)?);
        if let Some(root) = &options.template_root {
            let root_path = canonical_or_current(Path::new(root))?;
            ensure_within_root(&final_path, &root_path)?;
        }
        return Ok(Some(final_path.to_string_lossy().to_string()));
    }

    Ok(None)
}


fn normalize_path(path: &Path) -> PathBuf {
    use std::path::Component;

    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn canonical_or_current(path: &Path) -> Result<PathBuf, EngineError> {
    fs::canonicalize(path).or_else(|_| {
        let cwd = std::env::current_dir().map_err(|e| {
            EngineError::new(
                "RWE_PATH",
                format!("failed reading current_dir while resolving '{}': {e}", path.display()),
            )
        })?;
        Ok(cwd.join(path))
    })
}

fn canonical_or_fallback(path: &Path) -> Result<PathBuf, EngineError> {
    if path.exists() {
        fs::canonicalize(path).map_err(|e| {
            EngineError::new(
                "RWE_IMPORT_RESOLVE",
                format!("failed canonicalizing '{}': {e}", path.display()),
            )
        })
    } else {
        Ok(path.to_path_buf())
    }
}

fn resolve_module_path(base: &Path) -> Result<PathBuf, EngineError> {
    if base.is_file() {
        return Ok(base.to_path_buf());
    }

    // Try common TSX/TS module suffixes used by platform templates.
    const FILE_EXTS: [&str; 4] = [".tsx", ".ts", ".jsx", ".js"];
    for ext in FILE_EXTS {
        let candidate = PathBuf::from(format!("{}{}", base.display(), ext));
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    // Try index files for directory-style imports.
    const INDEX_FILES: [&str; 4] = ["index.tsx", "index.ts", "index.jsx", "index.js"];
    for index in INDEX_FILES {
        let candidate = base.join(index);
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Ok(base.to_path_buf())
}

fn ensure_within_root(path: &Path, root: &Path) -> Result<(), EngineError> {
    if path.starts_with(root) {
        Ok(())
    } else {
        Err(EngineError::new(
            "RWE_IMPORT_BOUNDARY",
            format!("resolved import '{}' escapes template_root '{}'", path.display(), root.display()),
        ))
    }
}

fn detect_hydrate_mode(source: &str) -> HydrateMode {
    if source.contains("hydrate=\"off\"") || source.contains("hydrate={'off'}") {
        HydrateMode::Off
    } else if source.contains("hydrate=\"onview\"") || source.contains("hydrate={'onview'}") {
        HydrateMode::Onview
    } else if source.contains("hydrate=\"oninteract\"")
        || source.contains("hydrate={'oninteract'}")
    {
        HydrateMode::Oninteract
    } else {
        HydrateMode::Onload
    }
}

fn rewrite_page_root_tag(source: &str) -> String {
    source
        .replace("<Page>", "<Fragment>")
        .replace("</Page>", "</Fragment>")
        .replace("<Page />", "<Fragment />")
        .replace("<Page/>", "<Fragment/>")
        .replace("<Page ", "<Fragment ")
}

/// At compile time, inline all filesystem-path imports into one self-contained
/// module. The result has zero filesystem imports — only npm:/jsr:/https: imports
/// (handled later by build_client_module in render.rs) and pure code.
fn bundle_for_client(
    page_source: &str,
    imports: &[ImportEdge],
) -> Result<String, EngineError> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut inlined_parts: Vec<String> = Vec::new();
    let mut counter: usize = 0;
    let mut rwe_names: HashSet<String> = HashSet::new();

    // Collect rwe imports from the main page itself
    rwe_names.extend(extract_rwe_import_names(page_source));

    // Depth-first: inline all filesystem imports from the page.
    // The resolved path is preferred, but when files on disk are already rewritten
    // with absolute paths (by prepare_template_root), edge.source IS the absolute
    // path and edge.resolved is None — so fall back to edge.source.
    for edge in imports {
        let path = edge
            .resolved
            .as_deref()
            .unwrap_or(&edge.source);
        if path.starts_with('/') && !is_rwe_runtime_path(path) {
            collect_inlined_module(path, &mut inlined_parts, &mut visited, &mut counter, &mut rwe_names)?;
        }
    }

    // Strip filesystem imports + rwe imports from the main page source
    let clean_main = strip_local_imports(page_source);

    // Build: inlined components first, then main page
    let mut result = inlined_parts.join("\n\n");
    if !result.is_empty() {
        result.push('\n');
    }
    result.push_str(&clean_main);
    Ok(result)
}

fn collect_inlined_module(
    path: &str,
    parts: &mut Vec<String>,
    visited: &mut HashSet<String>,
    counter: &mut usize,
    rwe_names: &mut HashSet<String>,
) -> Result<(), EngineError> {
    if visited.contains(path) {
        return Ok(());
    }
    visited.insert(path.to_string());

    let content = fs::read_to_string(path).map_err(|e| {
        EngineError::new(
            "RWE_BUNDLE_READ",
            format!("cannot read '{path}': {e}"),
        )
    })?;

    // Collect rwe imports from this file before stripping them
    rwe_names.extend(extract_rwe_import_names(&content));

    // Recursively inline this file's own filesystem imports first (depth-first)
    let sub_paths = extract_filesystem_import_paths(&content);
    for sub_path in &sub_paths {
        collect_inlined_module(sub_path, parts, visited, counter, rwe_names)?;
    }

    // Strip import lines on original content (import paths must be visible to the filter).
    let stripped = strip_local_imports(&content);

    // Mask string/template literal contents before line-based transforms.
    // This prevents code-like text inside strings (e.g. `import x from 'y'` inside a
    // template literal, or `const FOO = ...` inside a string) from confusing
    // export localization and constant prefixing.
    let (masked, masks) = super::js_masker::mask(&stripped);

    // Localize exports: "export default function X" → "function X" etc.
    let localized = localize_exports(&masked);

    // Auto-prefix module-scope UPPER_SNAKE_CASE constants to avoid name collisions
    // in the flat inlined bundle.
    let prefix = format!("__c{counter}_");
    *counter += 1;
    let prefixed = prefix_module_locals(&localized, &prefix);

    // Restore original string contents.
    let processed = super::js_masker::unmask(&prefixed, &masks);

    parts.push(processed);
    Ok(())
}

/// Auto-prefix all module-scope UPPER_SNAKE_CASE constants in a component
/// with a unique per-component prefix so they don't collide when inlined
/// into a single flat bundle. Developers write clean code; the bundler
/// owns the scoping.
fn prefix_module_locals(source: &str, prefix: &str) -> String {
    // Collect UPPER_SNAKE_CASE names from top-level const/let/var declarations.
    let local_names: Vec<String> = source
        .lines()
        .filter_map(|line| {
            let t = line.trim();
            for kw in &["const ", "let ", "var "] {
                if let Some(rest) = t.strip_prefix(kw) {
                    let name: String = rest
                        .chars()
                        .take_while(|c| c.is_alphanumeric() || *c == '_')
                        .collect();
                    if !name.is_empty()
                        && name.chars().next().map(|c| c.is_uppercase()).unwrap_or(false)
                        && name.chars().all(|c| c.is_uppercase() || c == '_' || c.is_ascii_digit())
                    {
                        return Some(name);
                    }
                }
            }
            None
        })
        .collect();

    if local_names.is_empty() {
        return source.to_string();
    }

    let mut result = source.to_string();
    for name in &local_names {
        result = replace_whole_word(&result, name, &format!("{prefix}{name}"));
    }
    result
}

/// Word-boundary string replacement — replaces `old` with `new` only when
/// not adjacent to an identifier character (`[a-zA-Z0-9_$]`).
fn replace_whole_word(source: &str, old: &str, new: &str) -> String {
    let mut result = String::with_capacity(source.len() + new.len());
    let mut i = 0;
    while i < source.len() {
        if source[i..].starts_with(old) {
            let before_ok = i == 0
                || !source[..i]
                    .chars()
                    .next_back()
                    .map(is_ident_char)
                    .unwrap_or(false);
            let after_pos = i + old.len();
            let after_ok = after_pos >= source.len()
                || !source[after_pos..]
                    .chars()
                    .next()
                    .map(is_ident_char)
                    .unwrap_or(false);
            if before_ok && after_ok {
                result.push_str(new);
                i += old.len();
                continue;
            }
        }
        let ch = source[i..].chars().next().unwrap();
        result.push(ch);
        i += ch.len_utf8();
    }
    result
}

fn is_ident_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}

/// Check if a path points to the RWE runtime shim (e.g. ".../rwe.ts").
/// These should NOT be inlined — they are runtime globals.
fn is_rwe_runtime_path(path: &str) -> bool {
    let fname = path.rsplit('/').next().unwrap_or("");
    fname == "rwe.ts" || fname == "rwe.js" || fname == "rwe.tsx"
}

/// Collect all named imports from `"rwe"` across a source file using OXC AST.
/// e.g. `import { useState, cx } from "rwe"` → ["useState", "cx"]
/// Handles multi-line imports correctly.
fn extract_rwe_import_names(source: &str) -> Vec<String> {
    let alloc = Allocator::default();
    let source_type = SourceType::default()
        .with_module(true)
        .with_jsx(true)
        .with_typescript(true);
    let parsed = Parser::new(&alloc, source, source_type).parse();
    if parsed.panicked {
        return Vec::new();
    }
    let mut names = Vec::new();
    for stmt in &parsed.program.body {
        if let Statement::ImportDeclaration(import) = stmt {
            let specifier = import.source.value.as_str();
            if specifier == "zeb" {
                if let Some(ref specifiers) = import.specifiers {
                    for s in specifiers {
                        match s {
                            oxc_ast::ast::ImportDeclarationSpecifier::ImportSpecifier(named) => {
                                names.push(named.local.name.as_str().to_string());
                            }
                            oxc_ast::ast::ImportDeclarationSpecifier::ImportDefaultSpecifier(def) => {
                                names.push(def.local.name.as_str().to_string());
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }
    names
}

/// Extract absolute filesystem paths from import declarations using OXC AST.
/// Handles multi-line imports correctly.
fn extract_filesystem_import_paths(source: &str) -> Vec<String> {
    let alloc = Allocator::default();
    let source_type = SourceType::default()
        .with_module(true)
        .with_jsx(true)
        .with_typescript(true);
    let parsed = Parser::new(&alloc, source, source_type).parse();
    if parsed.panicked {
        return Vec::new();
    }
    parsed
        .program
        .body
        .iter()
        .filter_map(|stmt| {
            if let Statement::ImportDeclaration(import) = stmt {
                let path = import.source.value.as_str();
                if path.starts_with('/') {
                    return Some(path.to_string());
                }
            }
            None
        })
        .collect()
}


/// Remove all filesystem-path imports AND rwe imports from source using OXC AST.
/// Handles multi-line imports correctly (OXC knows exact byte spans).
/// Keeps: npm:, node:, jsr:, https: imports (handled by render.rs later).
fn strip_local_imports(source: &str) -> String {
    let alloc = Allocator::default();
    let source_type = SourceType::default()
        .with_module(true)
        .with_jsx(true)
        .with_typescript(true);
    let parsed = Parser::new(&alloc, source, source_type).parse();
    if parsed.panicked {
        return source.to_string();
    }

    // Collect byte ranges of import declarations to remove.
    let mut remove_ranges: Vec<(usize, usize)> = Vec::new();
    for stmt in &parsed.program.body {
        if let Statement::ImportDeclaration(import) = stmt {
            let specifier = import.source.value.as_str();
            let should_strip = specifier == "zeb"
                || specifier.starts_with('/');
            if should_strip {
                let start = import.span.start as usize;
                let mut end = import.span.end as usize;
                // Consume trailing newline so we don't leave blank lines.
                if end < source.len() && source.as_bytes()[end] == b'\n' {
                    end += 1;
                }
                remove_ranges.push((start, end));
            }
        }
    }

    if remove_ranges.is_empty() {
        return source.to_string();
    }

    // Build result by copying everything except the removed ranges.
    let mut result = String::with_capacity(source.len());
    let mut cursor = 0;
    for (start, end) in &remove_ranges {
        if *start > cursor {
            result.push_str(&source[cursor..*start]);
        }
        cursor = *end;
    }
    if cursor < source.len() {
        result.push_str(&source[cursor..]);
    }
    result
}

/// Convert exported declarations to local ones for inlined modules.
/// Uses OXC AST — handles all valid TypeScript syntax regardless of formatting or comments.
///
/// - `export type X = ...`, `export interface X {}` → stripped entirely (type-only, no runtime)
/// - `export function/class/const/let/var X` → `export ` prefix removed
/// - `export default function/class X` → `export default ` prefix removed
/// - `export default expression` → `export default ` prefix removed
fn localize_exports(source: &str) -> String {
    use oxc_ast::ast::{Declaration, ExportDefaultDeclarationKind};

    let alloc = Allocator::default();
    let source_type = SourceType::default()
        .with_module(true)
        .with_jsx(true)
        .with_typescript(true);
    let parsed = Parser::new(&alloc, source, source_type).parse();
    if parsed.panicked {
        // Cannot parse — return source unchanged rather than corrupting it.
        return source.to_string();
    }

    let src_bytes = source.as_bytes();
    let mut ops: Vec<(usize, usize)> = Vec::new(); // byte ranges to delete

    for stmt in &parsed.program.body {
        match stmt {
            Statement::ExportNamedDeclaration(ed) => {
                let Some(decl) = &ed.declaration else { continue };
                let export_start = ed.span.start as usize;
                match decl {
                    // Type-only: strip the entire statement.
                    Declaration::TSTypeAliasDeclaration(_)
                    | Declaration::TSInterfaceDeclaration(_) => {
                        let mut end = ed.span.end as usize;
                        if end < source.len() && src_bytes[end] == b'\n' {
                            end += 1;
                        }
                        ops.push((export_start, end));
                    }
                    // Runtime: strip only the `export ` prefix so the declaration stays.
                    Declaration::FunctionDeclaration(f) => {
                        ops.push((export_start, f.span.start as usize));
                    }
                    Declaration::ClassDeclaration(c) => {
                        ops.push((export_start, c.span.start as usize));
                    }
                    Declaration::VariableDeclaration(v) => {
                        ops.push((export_start, v.span.start as usize));
                    }
                    _ => {}
                }
            }
            Statement::ExportDefaultDeclaration(ed) => {
                let export_start = ed.span.start as usize;
                let inner_start = match &ed.declaration {
                    ExportDefaultDeclarationKind::FunctionDeclaration(f) => {
                        f.span.start as usize
                    }
                    ExportDefaultDeclarationKind::ClassDeclaration(c) => {
                        c.span.start as usize
                    }
                    _ => {
                        // Covers: expressions, `export default interface X {}`, etc.
                        // Scan past the `export default ` keyword in raw bytes.
                        let mut i = export_start;
                        while i < src_bytes.len() && src_bytes[i] != b' ' && src_bytes[i] != b'\t' { i += 1; }
                        while i < src_bytes.len() && (src_bytes[i] == b' ' || src_bytes[i] == b'\t') { i += 1; }
                        while i < src_bytes.len() && src_bytes[i] != b' ' && src_bytes[i] != b'\t' && src_bytes[i] != b'\n' { i += 1; }
                        while i < src_bytes.len() && (src_bytes[i] == b' ' || src_bytes[i] == b'\t') { i += 1; }
                        i
                    }
                };
                if inner_start > export_start {
                    ops.push((export_start, inner_start));
                }
            }
            _ => {}
        }
    }

    if ops.is_empty() {
        return source.to_string();
    }

    ops.sort_by_key(|r| r.0);

    let mut result = String::with_capacity(source.len());
    let mut cursor = 0;
    for (start, end) in &ops {
        if *start > cursor {
            result.push_str(&source[cursor..*start]);
        }
        cursor = *end;
    }
    if cursor < source.len() {
        result.push_str(&source[cursor..]);
    }
    result
}

