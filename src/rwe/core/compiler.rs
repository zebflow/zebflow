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
    // Wrap in catch_unwind — OXC parser can panic on pathological inputs.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        compile_inner(source, options)
    }));
    match result {
        Ok(r) => r,
        Err(_) => {
            let _ = std::fs::write("/tmp/rwe-parse-failed.tsx", source);
            Err(EngineError::new(
                "RWE_PARSE_PANIC",
                "oxc compiler panicked — source written to /tmp/rwe-parse-failed.tsx",
            ))
        }
    }
}

fn compile_inner(source: &str, options: CompileOptions) -> Result<CompiledTemplate, EngineError> {
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
        let msg = parsed
            .errors
            .iter()
            .map(|e| format!("{e:?}"))
            .collect::<Vec<_>>()
            .join("; ");
        eprintln!("[RWE] parse diagnostics (non-fatal): {msg}");
    }

    ensure_default_export(&parsed.program)?;

    let mut diagnostics = security::analyze(source, &options.security)?;

    let raw_imports = collect_imports(&parsed.program);
    validate_zeb_exclusive_symbols(&parsed.program)?;
    validate_import_allowlist(&raw_imports, &options)?;

    let (rewritten_source, imports) =
        rewrite_imports(source, &raw_imports, &options, &mut diagnostics)?;

    let normalized_page_source = rewrite_page_root_tag(&rewritten_source);
    let (bundled_server, _, server_deps, inline_styles) = bundle_for_client(
        &normalized_page_source,
        &imports,
        options.template_root.as_deref(),
    )?;
    let transformed_server = format!("{}{}", JSX_PRELUDE, bundled_server);
    let (bundled_client, detected_zeb_libs, client_deps, _) = bundle_for_client(
        &normalized_page_source,
        &imports,
        options.template_root.as_deref(),
    )?;
    validate_zeb_icons_requirements(&bundled_client, &detected_zeb_libs)?;
    let transformed_client = format!("{}{}", JSX_PRELUDE, bundled_client);
    let mut dependency_paths = server_deps;
    dependency_paths.extend(client_deps);
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
        detected_zeb_libs,
        inline_styles,
        dependency_paths,
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
        if import == "zeb" {
            continue;
        }
        if import.starts_with("zeb/") {
            continue;
        }
        if import.starts_with("@/") {
            continue;
        }
        if import.starts_with("./") || import.starts_with("../") {
            continue;
        }
        // Absolute paths are the resolved form of @/ imports written to disk by
        // prepare_template_root() before compile() is called, or the rewritten
        // resolved form of relative imports. Never user-authored.
        if import.starts_with('/') {
            continue;
        }
        return Err(EngineError::new(
            "RWE_IMPORT_NOT_ALLOWED",
            format!(
                "import '{import}' is not allowed; valid imports are \"zeb\", \"zeb/*\", \"@/…\", and boundary-checked relative imports"
            ),
        ));
    }
    Ok(())
}

const ZEB_EXCLUSIVE_SYMBOLS: &[&str] = &[
    "useState",
    "useEffect",
    "useRef",
    "useMemo",
    "useCallback",
    "useContext",
    "useReducer",
    "useId",
    "useLayoutEffect",
    "usePageState",
    "useNavigate",
    "cx",
    "Link",
    "forwardRef",
    "memo",
    "createContext",
    "Fragment",
];

const ZEB_ICON_COMPONENTS: &[&str] = &[
    "ChevronLeft",
    "ChevronRight",
    "ChevronDown",
    "ChevronUp",
    "ChevronsLeft",
    "ChevronsRight",
    "ChevronsUpDown",
    "ArrowLeft",
    "ArrowRight",
    "ArrowUp",
    "ArrowDown",
    "Plus",
    "Minus",
    "X",
    "Check",
    "Search",
    "Filter",
    "RefreshCw",
    "Pencil",
    "Trash2",
    "Copy",
    "Clipboard",
    "Save",
    "Download",
    "Upload",
    "ExternalLink",
    "Undo2",
    "Redo2",
    "Eye",
    "EyeOff",
    "Lock",
    "Unlock",
    "Settings",
    "Menu",
    "MoreHorizontal",
    "MoreVertical",
    "Maximize2",
    "Minimize2",
    "PanelLeft",
    "PanelRight",
    "SidebarOpen",
    "SidebarClose",
    "AlertCircle",
    "AlertTriangle",
    "Info",
    "CheckCircle",
    "CheckCircle2",
    "XCircle",
    "Loader2",
    "Database",
    "TableIcon",
    "Columns2",
    "BarChart2",
    "PieChart",
    "TrendingUp",
    "TrendingDown",
    "File",
    "FileText",
    "Folder",
    "FolderOpen",
    "Code2",
    "Terminal",
    "User",
    "Users",
    "KeyRound",
    "LogIn",
    "LogOut",
    "Globe",
    "Package",
    "Zap",
    "Star",
    "Layers",
    "LayoutGrid",
    "ListIcon",
    "Cpu",
    "Cloud",
    "Wifi",
    "Bell",
    "BellOff",
    "Tag",
    "Bookmark",
    "Hash",
    "Slash",
    "Sparkles",
];

fn validate_zeb_exclusive_symbols(program: &oxc_ast::ast::Program<'_>) -> Result<(), EngineError> {
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
                                format!(
                                    "'{name}' must be imported from \"zeb\", not \"{specifier}\""
                                ),
                            ));
                        }
                    }
                }
            }
        }
    }
    Ok(())
}

fn validate_zeb_icons_requirements(
    source: &str,
    detected_zeb_libs: &[String],
) -> Result<(), EngineError> {
    if detected_zeb_libs.iter().any(|lib| lib == "zeb/icons") {
        return Ok(());
    }

    let used_icons = find_unbound_zeb_icon_components(source);
    if used_icons.is_empty() {
        return Ok(());
    }

    let sample = used_icons
        .iter()
        .take(3)
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    let suffix = if used_icons.len() > 3 { ", ..." } else { "" };
    let plural = if used_icons.len() > 1 {
        "icon components"
    } else {
        "icon component"
    };

    Err(EngineError::new(
        "RWE_IMPORT_ZEB_ICONS_REQUIRED",
        format!(
            "{plural} {sample}{suffix} require `import \"zeb/icons\"`; add `import \"zeb/icons\";` at module top-level"
        ),
    ))
}

fn find_unbound_zeb_icon_components(source: &str) -> Vec<String> {
    let alloc = Allocator::default();
    let source_type = SourceType::default()
        .with_module(true)
        .with_jsx(true)
        .with_typescript(true);
    let parsed = Parser::new(&alloc, source, source_type).parse();
    if parsed.panicked {
        return Vec::new();
    }

    let declared = collect_top_level_declared_names(&parsed.program);
    let (masked, _) = super::js_masker::mask(source);
    let used_names = collect_possible_jsx_component_names(&masked);

    let mut used = used_names
        .into_iter()
        .filter(|name| ZEB_ICON_COMPONENTS.contains(&name.as_str()) && !declared.contains(name))
        .collect::<Vec<_>>();
    used.sort();
    used.dedup();
    used
}

fn collect_possible_jsx_component_names(source: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut i = 0;

    while i < source.len() {
        let ch = source[i..]
            .chars()
            .next()
            .expect("source slice should contain one char");
        if ch == '<' {
            let start = i + ch.len_utf8();
            if start < source.len() {
                let next = source[start..]
                    .chars()
                    .next()
                    .expect("jsx candidate slice should contain one char");
                if next.is_ascii_uppercase() {
                    let mut end = start + next.len_utf8();
                    while end < source.len() {
                        let tail = source[end..]
                            .chars()
                            .next()
                            .expect("identifier tail slice should contain one char");
                        if tail.is_ascii_alphanumeric() || tail == '_' {
                            end += tail.len_utf8();
                        } else {
                            break;
                        }
                    }
                    names.push(source[start..end].to_string());
                }
            }
        }
        i += ch.len_utf8();
    }

    names
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
        if import == "zeb" || import.starts_with("zeb/") {
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
        let resolved = if import.ends_with(".css") {
            resolve_style_path(&joined)?
        } else {
            resolve_module_path(&joined)?
        };
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
        let resolved = if import.ends_with(".css") {
            resolve_style_path(&joined)?
        } else {
            resolve_module_path(&joined)?
        };
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
                format!(
                    "failed reading current_dir while resolving '{}': {e}",
                    path.display()
                ),
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

fn resolve_style_path(base: &Path) -> Result<PathBuf, EngineError> {
    if base.is_file() {
        return Ok(base.to_path_buf());
    }

    if base.extension().and_then(|v| v.to_str()) == Some("css") {
        return Ok(base.to_path_buf());
    }

    let candidate = PathBuf::from(format!("{}.css", base.display()));
    if candidate.exists() {
        return Ok(candidate);
    }

    let index = base.join("index.css");
    if index.exists() {
        return Ok(index);
    }

    Ok(base.to_path_buf())
}

fn ensure_within_root(path: &Path, root: &Path) -> Result<(), EngineError> {
    if path.starts_with(root) {
        Ok(())
    } else {
        Err(EngineError::new(
            "RWE_IMPORT_BOUNDARY",
            format!(
                "resolved import '{}' escapes template_root '{}'",
                path.display(),
                root.display()
            ),
        ))
    }
}

fn detect_hydrate_mode(source: &str) -> HydrateMode {
    if source.contains("hydrate=\"off\"") || source.contains("hydrate={'off'}") {
        HydrateMode::Off
    } else if source.contains("hydrate=\"onview\"") || source.contains("hydrate={'onview'}") {
        HydrateMode::Onview
    } else if source.contains("hydrate=\"oninteract\"") || source.contains("hydrate={'oninteract'}")
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

/// Rewrite `@/` alias imports in a component source to absolute filesystem paths.
///
/// Component files written via the template API may not have been pre-processed
/// by `prepare_template_root`. When `collect_inlined_module` reads such a file,
/// its `@/` imports are still in short-form and invisible to the
/// `extract_filesystem_import_paths` filter (which only matches paths starting
/// with `/`). Resolving them here makes transitive inlining work correctly
/// regardless of whether `prepare_template_root` has been called.
fn rewrite_at_imports(source: &str, template_root: &Path) -> String {
    let alloc = Allocator::default();
    let source_type = SourceType::default()
        .with_module(true)
        .with_jsx(true)
        .with_typescript(true);
    let parsed = Parser::new(&alloc, source, source_type).parse();
    if parsed.panicked {
        return source.to_string();
    }

    let mut out = source.to_string();
    for stmt in &parsed.program.body {
        if let Statement::ImportDeclaration(import) = stmt {
            let spec = import.source.value.as_str();
            if !spec.starts_with("@/") {
                continue;
            }
            let rel = spec.trim_start_matches("@/");
            let joined = template_root.join(rel);
            let resolved = if spec.ends_with(".css") {
                resolve_style_path(&joined)
            } else {
                resolve_module_path(&joined)
            };
            if let Ok(resolved) = resolved {
                if let Ok(canonical) = canonical_or_fallback(&resolved) {
                    let abs = normalize_path(&canonical);
                    let abs_str = abs.to_string_lossy();
                    out = out.replace(&format!("\"{spec}\""), &format!("\"{}\"", abs_str));
                    out = out.replace(&format!("'{spec}'"), &format!("'{}'", abs_str));
                }
            }
        }
    }
    out
}

/// At compile time, inline all filesystem-path imports into one self-contained
/// module. The result has zero filesystem imports — only npm:/jsr:/https: imports
/// (handled later by build_client_module in render.rs) and pure code.
fn bundle_for_client(
    page_source: &str,
    imports: &[ImportEdge],
    template_root: Option<&str>,
) -> Result<(String, Vec<String>, HashSet<String>, Vec<String>), EngineError> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut inlined_parts: Vec<String> = Vec::new();
    let mut inline_styles: Vec<String> = Vec::new();
    let mut counter: usize = 0;
    let mut rwe_names: HashSet<String> = HashSet::new();
    let mut zeb_libs: HashSet<String> = HashSet::new();

    // Collect rwe + zeb imports from the main page itself
    rwe_names.extend(extract_rwe_import_names(page_source));
    zeb_libs.extend(extract_zeb_lib_specifiers(page_source));

    // Depth-first: inline all filesystem imports from the page.
    for edge in imports {
        let path = edge.resolved.as_deref().unwrap_or(&edge.source);
        if path.starts_with('/') && path.ends_with(".css") {
            collect_inline_style(path, &mut inline_styles, &mut visited)?;
        } else if path.starts_with('/') && !is_rwe_runtime_path(path) {
            collect_inlined_module(
                path,
                &mut inlined_parts,
                &mut visited,
                &mut inline_styles,
                &mut counter,
                &mut rwe_names,
                &mut zeb_libs,
                template_root,
            )?;
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
    let mut libs: Vec<String> = zeb_libs.into_iter().collect();
    libs.sort();
    Ok((result, libs, visited, inline_styles))
}

fn collect_inlined_module(
    path: &str,
    parts: &mut Vec<String>,
    visited: &mut HashSet<String>,
    inline_styles: &mut Vec<String>,
    counter: &mut usize,
    rwe_names: &mut HashSet<String>,
    zeb_libs: &mut HashSet<String>,
    template_root: Option<&str>,
) -> Result<(), EngineError> {
    let canonical_path = canonical_module_identity(path)?;
    let visit_key = canonical_path.to_string_lossy().to_string();

    if visited.contains(&visit_key) {
        return Ok(());
    }
    visited.insert(visit_key);

    let raw = fs::read_to_string(&canonical_path).map_err(|e| {
        EngineError::new(
            "RWE_BUNDLE_READ",
            format!("cannot read '{}': {e}", canonical_path.display()),
        )
    })?;

    // Resolve any remaining @/ alias imports to absolute paths.
    // Component files written via the API may not have been pre-processed by
    // prepare_template_root, so @/ imports are still present as-is on disk.
    // Without rewriting them here, extract_filesystem_import_paths (which only
    // matches paths starting with '/') would miss them, leaving unresolved
    // imports in the inlined bundle.
    let content = if let Some(root) = template_root {
        rewrite_at_imports(&raw, Path::new(root))
    } else {
        raw
    };

    // Collect rwe + zeb imports from this file before stripping them
    rwe_names.extend(extract_rwe_import_names(&content));
    zeb_libs.extend(extract_zeb_lib_specifiers(&content));

    // Recursively inline this file's own filesystem imports first (depth-first)
    let sub_paths = extract_filesystem_import_paths(&content);
    for sub_path in &sub_paths {
        if sub_path.ends_with(".css") {
            collect_inline_style(sub_path, inline_styles, visited)?;
        } else {
            collect_inlined_module(
                sub_path,
                parts,
                visited,
                inline_styles,
                counter,
                rwe_names,
                zeb_libs,
                template_root,
            )?;
        }
    }

    // Collect exported names BEFORE localize_exports strips the export keywords.
    let exported = collect_top_level_exported_names(&content);

    // Strip import lines on original content (import paths must be visible to the filter).
    let stripped = strip_local_imports(&content);

    // Mask string/template literal contents before line-based transforms.
    // This prevents code-like text inside strings (e.g. `import x from 'y'` inside a
    // template literal, or `const FOO = ...` inside a string) from confusing
    // export localization and constant prefixing.
    let (masked, masks) = super::js_masker::mask(&stripped);

    // Localize exports: "export default function X" → "function X" etc.
    let localized = localize_exports(&masked);

    // Auto-prefix all module-scope non-exported declarations to avoid name collisions
    // in the flat inlined bundle.
    let prefix = format!("__c{counter}_");
    *counter += 1;
    let prefixed = prefix_module_locals(&localized, &prefix, &exported);

    // Restore original string contents.
    let processed = super::js_masker::unmask(&prefixed, &masks);

    parts.push(processed);
    Ok(())
}

fn collect_inline_style(
    path: &str,
    inline_styles: &mut Vec<String>,
    visited: &mut HashSet<String>,
) -> Result<(), EngineError> {
    let canonical_path = canonical_module_identity(path)?;
    let visit_key = canonical_path.to_string_lossy().to_string();
    if visited.contains(&visit_key) {
        return Ok(());
    }
    visited.insert(visit_key);
    let css = fs::read_to_string(&canonical_path).map_err(|e| {
        EngineError::new(
            "RWE_STYLE_READ",
            format!("cannot read stylesheet '{}': {e}", canonical_path.display()),
        )
    })?;
    inline_styles.push(css);
    Ok(())
}

fn canonical_module_identity(path: &str) -> Result<PathBuf, EngineError> {
    let canonical = canonical_or_fallback(Path::new(path))?;
    Ok(normalize_path(&canonical))
}

/// Collect all top-level exported binding names from a module source (before export stripping).
/// These names must NOT be prefixed — other files import them by exact name.
fn collect_top_level_exported_names(source: &str) -> HashSet<String> {
    use oxc_ast::ast::{Declaration, ExportDefaultDeclarationKind, ModuleExportName};

    let alloc = Allocator::default();
    let source_type = SourceType::default()
        .with_module(true)
        .with_jsx(true)
        .with_typescript(true);
    let parsed = Parser::new(&alloc, source, source_type).parse();
    if parsed.panicked {
        return HashSet::new();
    }

    let mut exported: HashSet<String> = HashSet::new();

    for stmt in &parsed.program.body {
        match stmt {
            Statement::ExportNamedDeclaration(ed) => {
                // `export function Foo`, `export const X`, `export class Bar`
                if let Some(decl) = &ed.declaration {
                    match decl {
                        Declaration::FunctionDeclaration(f) => {
                            if let Some(id) = &f.id {
                                exported.insert(id.name.as_str().to_string());
                            }
                        }
                        Declaration::ClassDeclaration(c) => {
                            if let Some(id) = &c.id {
                                exported.insert(id.name.as_str().to_string());
                            }
                        }
                        Declaration::VariableDeclaration(vd) => {
                            for d in &vd.declarations {
                                if let Some(name) = binding_ident_name(&d.id) {
                                    exported.insert(name);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                // `export { foo, bar as baz }` — the exported name is `bar`/`baz`
                for spec in &ed.specifiers {
                    let name = match &spec.exported {
                        ModuleExportName::IdentifierName(n) => n.name.as_str().to_string(),
                        ModuleExportName::IdentifierReference(r) => r.name.as_str().to_string(),
                        ModuleExportName::StringLiteral(s) => s.value.as_str().to_string(),
                    };
                    exported.insert(name);
                }
            }
            Statement::ExportDefaultDeclaration(edd) => match &edd.declaration {
                ExportDefaultDeclarationKind::FunctionDeclaration(f) => {
                    if let Some(id) = &f.id {
                        exported.insert(id.name.as_str().to_string());
                    }
                }
                ExportDefaultDeclarationKind::ClassDeclaration(c) => {
                    if let Some(id) = &c.id {
                        exported.insert(id.name.as_str().to_string());
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    exported
}

fn collect_top_level_declared_names(program: &oxc_ast::ast::Program<'_>) -> HashSet<String> {
    use oxc_ast::ast::Declaration;

    let mut declared = HashSet::new();

    for stmt in &program.body {
        match stmt {
            Statement::FunctionDeclaration(f) => {
                if let Some(id) = &f.id {
                    declared.insert(id.name.as_str().to_string());
                }
            }
            Statement::ClassDeclaration(c) => {
                if let Some(id) = &c.id {
                    declared.insert(id.name.as_str().to_string());
                }
            }
            Statement::VariableDeclaration(vd) => {
                for d in &vd.declarations {
                    if let Some(name) = binding_ident_name(&d.id) {
                        declared.insert(name);
                    }
                }
            }
            Statement::ExportNamedDeclaration(ed) => {
                if let Some(decl) = &ed.declaration {
                    match decl {
                        Declaration::FunctionDeclaration(f) => {
                            if let Some(id) = &f.id {
                                declared.insert(id.name.as_str().to_string());
                            }
                        }
                        Declaration::ClassDeclaration(c) => {
                            if let Some(id) = &c.id {
                                declared.insert(id.name.as_str().to_string());
                            }
                        }
                        Declaration::VariableDeclaration(vd) => {
                            for d in &vd.declarations {
                                if let Some(name) = binding_ident_name(&d.id) {
                                    declared.insert(name);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            Statement::ExportDefaultDeclaration(ed) => match &ed.declaration {
                oxc_ast::ast::ExportDefaultDeclarationKind::FunctionDeclaration(f) => {
                    if let Some(id) = &f.id {
                        declared.insert(id.name.as_str().to_string());
                    }
                }
                oxc_ast::ast::ExportDefaultDeclarationKind::ClassDeclaration(c) => {
                    if let Some(id) = &c.id {
                        declared.insert(id.name.as_str().to_string());
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    declared
}

/// Get the simple identifier name from a binding pattern, or `None` for destructuring patterns.
fn binding_ident_name(pattern: &oxc_ast::ast::BindingPattern) -> Option<String> {
    if let oxc_ast::ast::BindingPattern::BindingIdentifier(id) = pattern {
        Some(id.name.as_str().to_string())
    } else {
        None
    }
}

/// Auto-prefix all module-scope non-exported declarations with a unique per-component
/// prefix so they don't collide when inlined into a flat bundle.
/// Uses OXC AST — catches all declaration forms, not just UPPER_SNAKE_CASE.
fn prefix_module_locals(source: &str, prefix: &str, exported: &HashSet<String>) -> String {
    use oxc_ast::ast::Declaration;

    let alloc = Allocator::default();
    let source_type = SourceType::default()
        .with_module(true)
        .with_jsx(true)
        .with_typescript(true);
    let parsed = Parser::new(&alloc, source, source_type).parse();
    if parsed.panicked {
        return source.to_string();
    }

    let mut local_names: Vec<String> = Vec::new();

    for stmt in &parsed.program.body {
        match stmt {
            Statement::FunctionDeclaration(f) => {
                if let Some(id) = &f.id {
                    let name = id.name.as_str().to_string();
                    if !exported.contains(&name) {
                        local_names.push(name);
                    }
                }
            }
            Statement::ClassDeclaration(c) => {
                if let Some(id) = &c.id {
                    let name = id.name.as_str().to_string();
                    if !exported.contains(&name) {
                        local_names.push(name);
                    }
                }
            }
            Statement::VariableDeclaration(vd) => {
                for d in &vd.declarations {
                    if let Some(name) = binding_ident_name(&d.id) {
                        if !exported.contains(&name) {
                            local_names.push(name);
                        }
                    }
                }
            }
            // ExportNamedDeclaration/ExportDefaultDeclaration appear here when localize_exports
            // leaves a remnant (shouldn't happen, but be safe).
            Statement::ExportNamedDeclaration(ed) => {
                if let Some(decl) = &ed.declaration {
                    match decl {
                        Declaration::FunctionDeclaration(f) => {
                            if let Some(id) = &f.id {
                                let name = id.name.as_str().to_string();
                                if !exported.contains(&name) {
                                    local_names.push(name);
                                }
                            }
                        }
                        Declaration::ClassDeclaration(c) => {
                            if let Some(id) = &c.id {
                                let name = id.name.as_str().to_string();
                                if !exported.contains(&name) {
                                    local_names.push(name);
                                }
                            }
                        }
                        Declaration::VariableDeclaration(vd) => {
                            for d in &vd.declarations {
                                if let Some(name) = binding_ident_name(&d.id) {
                                    if !exported.contains(&name) {
                                        local_names.push(name);
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

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
            if specifier == "zeb" || specifier.starts_with("zeb/") {
                if let Some(ref specifiers) = import.specifiers {
                    for s in specifiers {
                        match s {
                            oxc_ast::ast::ImportDeclarationSpecifier::ImportSpecifier(named) => {
                                names.push(named.local.name.as_str().to_string());
                            }
                            oxc_ast::ast::ImportDeclarationSpecifier::ImportDefaultSpecifier(
                                def,
                            ) => {
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

/// Extract all `zeb/*` library specifiers (e.g. `"zeb/use"`, `"zeb/icons"`) from import declarations.
fn extract_zeb_lib_specifiers(source: &str) -> Vec<String> {
    let alloc = Allocator::default();
    let source_type = SourceType::default()
        .with_module(true)
        .with_jsx(true)
        .with_typescript(true);
    let parsed = Parser::new(&alloc, source, source_type).parse();
    if parsed.panicked {
        return Vec::new();
    }
    let mut libs = Vec::new();
    for stmt in &parsed.program.body {
        if let Statement::ImportDeclaration(import) = stmt {
            let specifier = import.source.value.as_str();
            if specifier.starts_with("zeb/") {
                let s = specifier.to_string();
                if !libs.contains(&s) {
                    libs.push(s);
                }
            }
        }
    }
    libs
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
            let should_strip =
                specifier == "zeb" || specifier.starts_with("zeb/") || specifier.starts_with('/');
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[cfg(unix)]
    use std::os::unix::fs::symlink;

    #[test]
    fn compile_detects_codemirror_library_imports() {
        let source = r#"
import { presets } from "zeb/codemirror";

export default function DemoPage() {
  return <Page><div data-kind={typeof presets}>ok</div></Page>;
}
"#;

        let compiled = compile(source, CompileOptions::default()).expect("compile should succeed");

        assert!(
            compiled
                .detected_zeb_libs
                .iter()
                .any(|lib| lib == "zeb/codemirror"),
            "expected zeb/codemirror to be detected, got {:?}",
            compiled.detected_zeb_libs
        );
    }

    #[test]
    fn compile_rejects_icons_without_explicit_import() {
        let source = r#"
export default function DemoPage() {
  return <Page><CheckCircle className="w-4 h-4" /></Page>;
}
"#;

        let err = compile(source, CompileOptions::default()).expect_err("compile should fail");
        assert_eq!(err.code, "RWE_IMPORT_ZEB_ICONS_REQUIRED");
        assert!(
            err.message.contains("import \"zeb/icons\""),
            "expected explicit import guidance, got {:?}",
            err.message
        );
    }

    #[test]
    fn compile_allows_icons_with_explicit_import() {
        let source = r#"
import "zeb/icons";

export default function DemoPage() {
  return <Page><CheckCircle className="w-4 h-4" /></Page>;
}
"#;

        let compiled = compile(source, CompileOptions::default()).expect("compile should succeed");
        assert!(
            compiled
                .detected_zeb_libs
                .iter()
                .any(|lib| lib == "zeb/icons"),
            "expected zeb/icons to be detected, got {:?}",
            compiled.detected_zeb_libs
        );
    }

    #[test]
    fn prepare_path_rewrite_is_not_required_for_zeb_imports() {
        let root =
            std::env::temp_dir().join(format!("rwe-zeb-rewrite-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create temp root");

        let file = root.join("page.tsx");
        let source = r#"import { useState } from "zeb";
export default function Page() { return <div />; }"#;
        fs::write(&file, source).expect("write source");

        crate::rwe::core::prepare_template_root(&root).expect("prepare template root");
        let rewritten = fs::read_to_string(&file).expect("read rewritten source");
        assert!(
            rewritten.contains(r#"from "zeb""#),
            "zeb imports must stay logical, got: {rewritten}"
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn compile_collects_side_effect_css_imports() {
        let root = std::env::temp_dir().join(format!("rwe-css-import-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("styles")).expect("create styles dir");
        fs::create_dir_all(root.join("pages")).expect("create pages dir");
        fs::write(
            root.join("styles/editor.css"),
            ".editor-shell { color: rgb(1, 2, 3); }",
        )
        .expect("write css");

        let file = root.join("pages/page.tsx");
        fs::write(
            &file,
            r#"
import "@/styles/editor.css";

export default function Page() {
  return <div className="editor-shell">ok</div>;
}
"#,
        )
        .expect("write page");

        let compiled = compile(
            &fs::read_to_string(&file).expect("read page"),
            CompileOptions {
                template_root: Some(root.display().to_string()),
                file_path: Some(file.display().to_string()),
                ..Default::default()
            },
        )
        .expect("compile should succeed");

        assert_eq!(compiled.inline_styles.len(), 1);
        assert!(
            compiled.inline_styles[0].contains(".editor-shell"),
            "expected collected inline CSS, got {:?}",
            compiled.inline_styles
        );
        assert!(
            compiled
                .dependency_paths
                .iter()
                .any(|path| path.ends_with("styles/editor.css")),
            "expected stylesheet dependency path, got {:?}",
            compiled.dependency_paths
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn compile_allows_relative_component_imports() {
        let root =
            std::env::temp_dir().join(format!("rwe-relative-import-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("pages")).expect("create pages dir");
        fs::create_dir_all(root.join("components")).expect("create components dir");
        fs::write(
            root.join("components/badge.tsx"),
            r#"
export function Badge() {
  return <span>badge</span>;
}
"#,
        )
        .expect("write badge");

        let file = root.join("pages/page.tsx");
        fs::write(
            &file,
            r#"
import { Badge } from "../components/badge";

export default function Page() {
  return <div><Badge /></div>;
}
"#,
        )
        .expect("write page");

        let compiled = compile(
            &fs::read_to_string(&file).expect("read page"),
            CompileOptions {
                template_root: Some(root.display().to_string()),
                file_path: Some(file.display().to_string()),
                ..Default::default()
            },
        )
        .expect("compile should succeed");

        assert!(
            compiled
                .dependency_paths
                .iter()
                .any(|path| path.ends_with("components/badge.tsx")),
            "expected relative component dependency path, got {:?}",
            compiled.dependency_paths
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn compile_collects_relative_css_imports() {
        let root =
            std::env::temp_dir().join(format!("rwe-relative-css-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(root.join("pages")).expect("create pages dir");
        fs::write(
            root.join("pages/editor.css"),
            ".editor-pane { background: rgb(10, 20, 30); }",
        )
        .expect("write css");

        let file = root.join("pages/page.tsx");
        fs::write(
            &file,
            r#"
import "./editor.css";

export default function Page() {
  return <section className="editor-pane">ok</section>;
}
"#,
        )
        .expect("write page");

        let compiled = compile(
            &fs::read_to_string(&file).expect("read page"),
            CompileOptions {
                template_root: Some(root.display().to_string()),
                file_path: Some(file.display().to_string()),
                ..Default::default()
            },
        )
        .expect("compile should succeed");

        assert_eq!(compiled.inline_styles.len(), 1);
        assert!(
            compiled.inline_styles[0].contains(".editor-pane"),
            "expected collected relative CSS, got {:?}",
            compiled.inline_styles
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[cfg(unix)]
    #[test]
    fn canonical_module_identity_collapses_symlink_aliases() {
        let root =
            std::env::temp_dir().join(format!("rwe-canonical-module-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("create temp root");

        let target = root.join("real.tsx");
        let alias = root.join("alias.tsx");
        fs::write(
            &target,
            "export default function Real() { return <div />; }",
        )
        .expect("write real file");
        symlink(&target, &alias).expect("create symlink");

        let real = canonical_module_identity(target.to_str().expect("real path utf8"))
            .expect("canonical real");
        let aliased = canonical_module_identity(alias.to_str().expect("alias path utf8"))
            .expect("canonical alias");
        assert_eq!(real, aliased);

        let _ = fs::remove_dir_all(&root);
    }
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
                let Some(decl) = &ed.declaration else {
                    continue;
                };
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
                    ExportDefaultDeclarationKind::FunctionDeclaration(f) => f.span.start as usize,
                    ExportDefaultDeclarationKind::ClassDeclaration(c) => c.span.start as usize,
                    _ => {
                        // Covers: expressions, `export default interface X {}`, etc.
                        // Scan past the `export default ` keyword in raw bytes.
                        let mut i = export_start;
                        while i < src_bytes.len() && src_bytes[i] != b' ' && src_bytes[i] != b'\t' {
                            i += 1;
                        }
                        while i < src_bytes.len() && (src_bytes[i] == b' ' || src_bytes[i] == b'\t')
                        {
                            i += 1;
                        }
                        while i < src_bytes.len()
                            && src_bytes[i] != b' '
                            && src_bytes[i] != b'\t'
                            && src_bytes[i] != b'\n'
                        {
                            i += 1;
                        }
                        while i < src_bytes.len() && (src_bytes[i] == b' ' || src_bytes[i] == b'\t')
                        {
                            i += 1;
                        }
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
