//! Whether a page or component would compile, without saving it: the check
//! Zebflow Studio's editor runs on every change, the MCP tool
//! `template_check` and a control door's `template_check` command. One check
//! for all three, with the options a page render uses, so it never passes or
//! refuses what the render would not.

use crate::platform::error::PlatformError;
use crate::platform::model::{TemplateCompileResponse, TemplateDiagnostic};
use crate::platform::services::PlatformService;
use crate::rwe::{ReactiveWebOptions, TemplateSource};

/// What a repository file is to the compiler, by its path.
pub fn template_kind(rel: &str) -> &'static str {
    if rel.ends_with(".css") {
        "style"
    } else if rel.ends_with(".ts") {
        "script"
    } else if rel.contains("/pages/") || rel.starts_with("pages/") {
        "page"
    } else {
        "component"
    }
}

/// Checks `content` as it would stand at `rel` in the project's repository.
/// A page is compiled with every file it imports as they are saved now; a
/// component is checked as a module. A script, a stylesheet and a library
/// recipe's own source are not page code, and pass.
pub fn check(platform: &PlatformService, owner: &str, project: &str, rel: &str, content: &str) -> Result<TemplateCompileResponse, PlatformError> {
    let rel = rel.trim();
    if rel.is_empty() {
        return Ok(TemplateCompileResponse {
            ok: false,
            diagnostics: vec![TemplateDiagnostic {
                code: "template_path_missing".to_string(),
                message: "template path must not be empty".to_string(),
                severity: "error".to_string(),
                from: Some(0),
                to: Some(1),
            }],
        });
    }

    let kind = template_kind(rel);
    // A library recipe's own source is compiled by zeb-bundler, not here.
    let recipe_source = crate::platform::services::project_library::is_recipe_source(platform, owner, project, rel);
    if kind == "script" || kind == "style" || recipe_source {
        return Ok(TemplateCompileResponse { ok: true, diagnostics: Vec::new() });
    }

    // A component file has no default export and is never compiled alone:
    // it is checked as a module, for what breaks it inside a page. A file
    // under `pages/` is a page whatever it holds, and its render needs the
    // default export, so it gets the full compile and that refusal.
    if kind != "page" && !crate::rwe::core::source_check::has_default_export(content) {
        return Ok(module_check(rel, content));
    }

    let template_root = platform.file.ensure_project_layout(owner, project)?.repo_source_dir();
    // The same options a page render uses: the check must not pass or refuse
    // what the render would not.
    let options = ReactiveWebOptions::for_project_page(Some(template_root.clone()), platform.library.source_roots())
        .with_runtime_libraries(crate::platform::services::project_library::runtime_libraries(platform, owner, project));
    let source = TemplateSource {
        id: format!("platform.editor.{}", rel.replace('/', ".")),
        source_path: Some(template_root.join(rel)),
        markup: content.to_string(),
    };
    let engine = crate::rwe::engines::resolve_engine_or_default(None);
    Ok(match engine.compile_template(&source, &crate::language::NoopLanguageEngine, &options) {
        Ok(compiled) => TemplateCompileResponse {
            ok: true,
            diagnostics: compiled
                .diagnostics
                .into_iter()
                .map(|diag| TemplateDiagnostic { code: diag.code, message: diag.message, severity: "warning".to_string(), from: None, to: None })
                .collect(),
        },
        Err(err) => TemplateCompileResponse {
            ok: false,
            diagnostics: vec![TemplateDiagnostic {
                code: err.code.to_string(),
                message: err.message,
                severity: "error".to_string(),
                from: Some(0),
                to: Some(1),
            }],
        },
    })
}

/// A component file checked as a module (`rwe::core::source_check`): it
/// parses, it declares or imports every name it uses, and a `.tsx` file has
/// no binding named `h`. Each problem is an error at its place in the buffer.
fn module_check(rel: &str, content: &str) -> TemplateCompileResponse {
    let diagnostics: Vec<TemplateDiagnostic> = crate::rwe::core::source_check::check_template_source(content, rel.ends_with(".tsx"))
        .into_iter()
        .map(|problem| {
            let from = char_offset(content, problem.line, problem.column);
            TemplateDiagnostic {
                code: "template_source_problem".to_string(),
                message: format!("{rel}:{problem}"),
                severity: "error".to_string(),
                from: Some(from),
                to: Some(from + 1),
            }
        })
        .collect();
    TemplateCompileResponse { ok: diagnostics.is_empty(), diagnostics }
}

/// The character offset of a 1-based line and column (counted in
/// characters, as `source_check` counts them).
fn char_offset(source: &str, line: usize, column: usize) -> usize {
    let before: usize = source.split('\n').take(line.saturating_sub(1)).map(|l| l.chars().count() + 1).sum();
    before + column.saturating_sub(1)
}
