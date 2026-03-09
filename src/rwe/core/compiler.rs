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
    validate_import_allowlist(&raw_imports, &options)?;

    let (rewritten_source, imports) =
        rewrite_imports(source, &raw_imports, &options, &mut diagnostics)?;

    let normalized_page_source = rewrite_page_root_tag(&rewritten_source);
    let transformed_server = format!("{}{}", JSX_PRELUDE, normalized_page_source);
    let transformed_client = transformed_server.clone();
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
    options: &CompileOptions,
) -> Result<(), EngineError> {
    for import in imports {
        if import == "rwe" {
            continue;
        }
        if import.starts_with("npm:") || import.starts_with("node:") || import.starts_with("jsr:") {
            continue;
        }
        if import.starts_with('/') {
            continue;
        }
        if options
            .security
            .import_allowlist
            .iter()
            .any(|prefix| import.starts_with(prefix))
        {
            continue;
        }
        return Err(EngineError::new(
            "RWE_IMPORT_ALLOWLIST",
            format!("import '{import}' is not allowed by security policy"),
        ));
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
        if import == "rwe" {
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
    if base.exists() {
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

