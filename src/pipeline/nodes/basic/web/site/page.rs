//! `web.site.generate --path` — one TSX page rendered once into the site.
//!
//! The file-producing counterpart to `web.response.send --template`: the same
//! RWE compile and render, but the document is written into the site folder
//! instead of answering a request. The page opens on its own:
//! - project `styles/main.css` is inlined when present
//! - Tailwind CSS extracted by the RWE engine is inlined
//! - compiled client scripts are inlined as `<script type="module">`

use std::path::Path;

use serde_json::Value;

use super::Config;
use crate::pipeline::PipelineError;
use crate::pipeline::nodes::basic::web::static_site;
use crate::rwe::{CompiledScript, TemplateSource};

/// Resolve the template file and load its markup from disk when `markup` was not
/// injected beforehand.
pub fn resolve_template_source(
    node_id: &str,
    config: &Config,
    template_root: Option<&Path>,
) -> Result<TemplateSource, PipelineError> {
    let template_rel = normalize_template_rel_path(config.template.as_deref().unwrap_or_default())?;
    let markup = if let Some(markup) = config
        .markup
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        markup.to_string()
    } else {
        let Some(root) = template_root else {
            return Err(PipelineError::new(
                "FW_NODE_WEB_SITE_GENERATE_TEMPLATE_ROOT",
                format!(
                    "node '{node_id}' requires template_root to load '{}'",
                    template_rel
                ),
            ));
        };
        // Through the repository reader: `..` and links refused, capped.
        let bytes = crate::pipeline::nodes::shared::project_store::read_repo_file(
            root,
            &template_rel,
            "FW_NODE_WEB_SITE_GENERATE_TEMPLATE_MISSING",
        )
        .map_err(|err| PipelineError::new(err.code, format!("node '{node_id}' template {}", err.message)))?;
        String::from_utf8(bytes).map_err(|err| {
            PipelineError::new(
                "FW_NODE_WEB_SITE_GENERATE_TEMPLATE_READ",
                format!("failed reading template '{}': {err}", template_rel),
            )
        })?
    };

    let source_path = template_root.map(|root| root.join(&template_rel));
    Ok(TemplateSource {
        id: template_rel.clone(),
        source_path,
        markup,
    })
}

/// The page's path inside the site folder, normalised.
pub fn page_path(config: &Config) -> Result<String, PipelineError> {
    static_site::normalize_page_output_path(config.path.as_deref().unwrap_or_default())
}

/// The page's key in the store: the site folder, then the page's path.
pub fn page_key(config: &Config) -> Result<String, PipelineError> {
    static_site::page_rel_path_from_site_root(&config.folder_rel(super::Mode::Page)?, &page_path(config)?)
}

/// The route a page has when its site is served at the root of an address.
pub fn default_route(config: &Config) -> Result<String, PipelineError> {
    static_site::route_path_for_output_path("/", &page_path(config)?)
}

/// Decorate a rendered RWE HTML document so the persisted file can open directly.
pub fn build_static_html(
    mut html: String,
    hydration_payload: &Value,
    compiled_scripts: &[CompiledScript],
    template_root: Option<&Path>,
) -> String {
    html = ensure_meta_charset(html);

    if let Some(root) = template_root {
        // Through the repository reader; a missing or linked file is no theme.
        let project_css = crate::pipeline::nodes::shared::project_store::read_repo_file(
            root,
            "styles/main.css",
            "FW_NODE_WEB_SITE_GENERATE_TEMPLATE_READ",
        )
        .ok()
        .and_then(|bytes| String::from_utf8(bytes).ok());
        if let Some(project_css) = project_css
            && !project_css.trim().is_empty()
        {
            html = inject_before_head_end(
                &html,
                &format!(
                    "<style data-project-theme>{}</style>",
                    escape_style_block(&project_css)
                ),
            );
        }
    }

    if let Some(css) = hydration_payload.get("css").and_then(Value::as_str) {
        html = crate::rwe::core::render::insert_engine_styles(&html, &escape_style_block(css));
    }

    if !compiled_scripts.is_empty() {
        let mut block = String::new();
        for script in compiled_scripts {
            block.push_str("<script type=\"module\" data-zf-static-rwe>");
            block.push_str(&escape_script_block(&script.content));
            block.push_str("</script>");
        }
        html = inject_before_body_end(&html, &block);
    }

    html
}

/// What a generated page renders from — and, since the RWE embeds its input
/// for hydration, what the page publishes: the payload without the request
/// that started the run. Every trigger envelope (`webhook`, `room`, `mcp`, …)
/// loses its `headers`, `cookies` and `auth`, so a page generated from a
/// request never carries the caller's address, browser, cookie, token or
/// claims (`addressing.md` §0: a page never holds the project's own address,
/// which `headers.host` is). A value the page should show is copied into a
/// key of its own by an upstream node, where the pipeline says so.
pub fn page_input(payload: &Value) -> Value {
    let mut input = payload.clone();
    if let Some(map) = input.as_object_mut() {
        for source in crate::pipeline::nodes::basic::trigger::SOURCE_KEYS {
            if let Some(Value::Object(envelope)) = map.get_mut(source) {
                for request_only in REQUEST_ONLY_KEYS {
                    envelope.remove(request_only);
                }
            }
        }
    }
    input
}

/// The parts of a trigger envelope that belong to the request, never to a page.
pub const REQUEST_ONLY_KEYS: [&str; 3] = ["headers", "cookies", "auth"];

/// What writing `contents` at `rel_path` would do under `on_conflict`, decided
/// before anything is written: `Some("unchanged")` or `Some("skipped")` leave
/// the object as it is, `None` writes it, and `error` on a changed object fails.
pub fn conflict_status(
    store: &crate::zebfs::ZebFs,
    rel_path: &str,
    contents: &str,
    on_conflict: &str,
) -> Result<Option<&'static str>, PipelineError> {
    let bytes = contents.as_bytes();
    match crate::pipeline::nodes::shared::project_store::read_capped(store, rel_path, "FW_NODE_WEB_SITE_GENERATE_READ") {
        Ok(existing) => {
            if existing == bytes {
                return Ok(Some("unchanged"));
            }
            match on_conflict.trim() {
                "overwrite" | "" => Ok(None),
                "skip" => Ok(Some("skipped")),
                "error" => Err(PipelineError::new(
                    super::CODE_CONFLICT,
                    format!("destination '{rel_path}' already exists with other content"),
                )),
                other => Err(PipelineError::new(
                    super::CODE_CONFLICT,
                    format!("unsupported on_conflict value '{other}' — expected overwrite, skip, or error"),
                )),
            }
        }
        Err(_) if store.head(rel_path).is_err() => Ok(None),
        Err(err) => Err(PipelineError::new(
            "FW_NODE_WEB_SITE_GENERATE_READ",
            format!("failed reading '{rel_path}': {err}"),
        )),
    }
}

/// Persist one generated file into the project's store — its one active
/// backend — after [`conflict_status`] allows it. The store writes atomically.
pub fn write_generated_object(
    store: &crate::zebfs::ZebFs,
    rel_path: &str,
    contents: &str,
    on_conflict: &str,
) -> Result<&'static str, PipelineError> {
    if let Some(status) = conflict_status(store, rel_path, contents, on_conflict)? {
        return Ok(status);
    }
    store.put(rel_path, contents.as_bytes()).map_err(|err| {
        PipelineError::new(
            "FW_NODE_WEB_SITE_GENERATE_WRITE",
            format!("failed writing '{rel_path}': {err}"),
        )
    })?;
    Ok("written")
}

/// A template path under the source root: no `..`, ending `.tsx`.
pub(super) fn normalize_template_rel_path(raw: &str) -> Result<String, PipelineError> {
    let trimmed = raw.trim().trim_start_matches('/').replace('\\', "/");
    if trimmed.is_empty() {
        return Err(PipelineError::new(
            "FW_NODE_WEB_SITE_GENERATE_TEMPLATE_PATH",
            "template path must not be empty",
        ));
    }

    let mut parts = Vec::new();
    for part in trimmed.split('/') {
        let part = part.trim();
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." || part.contains('\0') {
            return Err(PipelineError::new(
                "FW_NODE_WEB_SITE_GENERATE_TEMPLATE_PATH",
                "template path must stay inside the project template root",
            ));
        }
        parts.push(part.to_string());
    }
    if parts.is_empty() {
        return Err(PipelineError::new(
            "FW_NODE_WEB_SITE_GENERATE_TEMPLATE_PATH",
            "template path must not be empty",
        ));
    }
    let last = parts.last().expect("parts not empty");
    if !last.ends_with(".tsx") {
        return Err(PipelineError::new(
            "FW_NODE_WEB_SITE_GENERATE_TEMPLATE_PATH",
            "template path must end with .tsx",
        ));
    }
    Ok(parts.join("/"))
}

fn ensure_meta_charset(mut html: String) -> String {
    if html.contains("<meta charset") || html.contains("<meta http-equiv=\"Content-Type\"") {
        return html;
    }
    let tag = "<meta charset=\"utf-8\">";
    if let Some(pos) = html.find("<head>") {
        html.insert_str(pos + "<head>".len(), tag);
    } else if let Some(pos) = html.find("</head>") {
        html.insert_str(pos, tag);
    } else {
        html = format!("{tag}{html}");
    }
    html
}

fn inject_before_head_end(html: &str, snippet: &str) -> String {
    if let Some(pos) = html.find("</head>") {
        let mut out = String::with_capacity(html.len() + snippet.len());
        out.push_str(&html[..pos]);
        out.push_str(snippet);
        out.push_str(&html[pos..]);
        out
    } else {
        format!("{snippet}{html}")
    }
}

fn inject_before_body_end(html: &str, snippet: &str) -> String {
    if let Some(pos) = html.find("</body>") {
        let mut out = String::with_capacity(html.len() + snippet.len());
        out.push_str(&html[..pos]);
        out.push_str(snippet);
        out.push_str(&html[pos..]);
        out
    } else {
        format!("{html}{snippet}")
    }
}

fn escape_script_block(content: &str) -> String {
    content.replace("</script>", "<\\/script>")
}

fn escape_style_block(content: &str) -> String {
    content.replace("</style>", "<\\/style>")
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::sync::Arc;

    use super::{build_static_html, default_route, page_key};
    use serde_json::json;

    use crate::language::DenoSandboxEngine;
    use crate::pipeline::engines::basic::new_template_cache;
    use crate::pipeline::nodes::basic::web::site::{Config, NODE_KIND};
    use crate::pipeline::{
        BasicPipelineEngine, PipelineContext, PipelineEngine, PipelineGraph, PipelineNode,
    };
    use crate::platform::adapters::file::build_file_adapter;
    use crate::platform::model::FileAdapterKind;
    use crate::platform::services::project_config::ProjectConfigurationService;
    use crate::rwe::resolve_engine_or_default;

    fn page(path: &str) -> Config {
        Config { template: Some("pages/a.tsx".to_string()), path: Some(path.to_string()), ..Default::default() }
    }

    /// A temp root removed when the returned `TempDir` is dropped; the
    /// caller must keep it bound (not `_`) for as long as the path is used.
    fn temp_root(name: &str) -> (tempfile::TempDir, PathBuf) {
        let tmp = tempfile::Builder::new()
            .prefix(&format!("{name}-"))
            .tempdir()
            .expect("tempdir");
        let root = tmp.path().to_path_buf();
        (tmp, root)
    }

    #[test]
    fn a_page_key_stays_inside_its_folder() {
        assert_eq!(page_key(&page("authors/a/note.html")).unwrap(), "site/authors/a/note.html");
        assert!(page_key(&page("../escape.html")).is_err());
        let named = Config { folder: Some("static/site-a".to_string()), ..page("authors/a/note.html") };
        assert_eq!(page_key(&named).unwrap(), "static/site-a/authors/a/note.html");
        let escaping = Config { folder: Some("../up".to_string()), ..page("a.html") };
        assert!(page_key(&escaping).is_err());
    }

    /// The request never reaches a page: every trigger envelope loses its
    /// headers, cookies and auth; the rest of the payload is the page's.
    #[test]
    fn a_page_input_drops_the_request_and_keeps_the_rest() {
        let payload = json!({
            "webhook": { "body": { "title": "Demo" }, "query": { "q": "1" }, "headers": { "user-agent": "demo-agent" }, "cookies": { "s": "x" }, "auth": { "sub": "user-1" } },
            "room": { "message": "hi", "auth": { "sub": "user-2" } },
            "query": { "rows": [{ "headers": "a column, not a request" }] },
        });
        let input = super::page_input(&payload);
        assert_eq!(input["webhook"], json!({ "body": { "title": "Demo" }, "query": { "q": "1" } }));
        assert_eq!(input["room"], json!({ "message": "hi" }));
        assert_eq!(input["query"], payload["query"], "a node's answer is the author's to render");
    }

    #[test]
    fn template_path_requires_explicit_tsx_extension() {
        assert_eq!(
            super::normalize_template_rel_path("pages/note.tsx").expect("tsx template"),
            "pages/note.tsx"
        );
        let err =
            super::normalize_template_rel_path("pages/note").expect_err("missing extension");
        assert_eq!(err.code, "FW_NODE_WEB_SITE_GENERATE_TEMPLATE_PATH");
        assert!(err.message.contains(".tsx"));
    }

    #[test]
    fn default_route_is_the_page_address_on_its_site() {
        assert_eq!(default_route(&page("index.html")).unwrap(), "/");
        assert_eq!(default_route(&page("blog/index.html")).unwrap(), "/blog/");
        assert_eq!(default_route(&page("collections/a/item.html")).unwrap(), "/collections/a/item.html");
    }

    #[test]
    fn build_static_html_inlines_css_and_scripts() {
        let html = build_static_html(
            "<html><head></head><body><main>ok</main></body></html>".to_string(),
            &json!({ "css": ".x{color:red;}" }),
            &[crate::rwe::CompiledScript {
                id: "page".to_string(),
                scope: crate::rwe::CompiledScriptScope::Page,
                content_type: "text/javascript".to_string(),
                content: "console.log('ok')".to_string(),
                content_hash: "abc".to_string(),
                suggested_file_name: "rwe-abc.mjs".to_string(),
            }],
            None,
        );
        assert!(html.contains("data-rwe-tw"));
        assert!(html.contains("data-zf-static-rwe"));
        assert!(html.contains("console.log('ok')"));
    }

    #[tokio::test]
    async fn engine_generates_static_file_and_detects_unchanged_content() {
        let (_tmp, root) = temp_root("zebflow-staticgen-test");
        let file = build_file_adapter(
            FileAdapterKind::Filesystem,
            root.to_path_buf(),
            std::sync::Arc::new(ProjectConfigurationService::new(root.join("users"))),
            None,
        );
        let layout = file
            .ensure_project_layout("superadmin", "example-project")
            .expect("layout");
        let template_dir = layout.repo_source_dir().join("pages");
        std::fs::create_dir_all(&template_dir).expect("template dir");
        let asset_dir = layout.repo_static_dir().join("icons");
        std::fs::create_dir_all(&asset_dir).expect("asset dir");
        std::fs::write(asset_dir.join("favicon.ico"), b"ico").expect("favicon");
        std::fs::write(
            template_dir.join("note.tsx"),
            r#"
import { ErrorBoundary, useSyncExternalStore } from "zeb/react";
function LiveStatus() {
  const status = useSyncExternalStore(() => () => {}, () => 'online', () => 'snapshot');
  return <b>{status}</b>;
}
function BrokenPanel() { throw new Error('static recovery'); }
export const page = {
  head: {
    title: "Note",
    icons: [
      { rel: "icon", href: "/static/superadmin/example-project/icons/favicon.ico" }
    ]
  }
};

export const app = {};

export default function NotePage(input) {
  return (
    <Page>
      <main className="min-h-screen bg-white text-slate-900 p-6">
        <img src="/assets/branding/logo.svg" alt="Zebflow" />
        <h1 className="text-3xl font-black">{input.author_name} - {input.note_title}</h1>
        <p className="mt-4">{input.note_line}</p>
        <LiveStatus />
        <ErrorBoundary fallback={<i>panel unavailable</i>}><BrokenPanel /></ErrorBoundary>
      </main>
    </Page>
  );
}
"#,
        )
        .expect("template write");

        let graph = PipelineGraph {
            id: "generate-note".to_string(),
            description: None,
            metadata: None,
            notes: Vec::new(),
            entry_nodes: vec!["gen".to_string()],
            nodes: vec![PipelineNode {
                id: "gen".to_string(),
                kind: NODE_KIND.to_string(),
                input_pins: vec!["in".to_string()],
                output_pins: vec!["out".to_string()],
                config: json!({
                    "template": "pages/note.tsx",
                    "folder": "static/site-a",
                    "path": "authors/{{ $input.author_slug }}/{{ $input.note_slug }}/note.html"
                }),
            }],
            edges: vec![],
        };
        let ctx = PipelineContext {
            owner: "superadmin".to_string(),
            project: "example-project".to_string(),
            pipeline: graph.id.clone(),
            request_id: "req-1".to_string(),
            route: String::new(),
            input: json!({
                "author_slug": "demo-author",
                "note_slug": "first-note",
                "author_name": "Demo Author",
                "note_title": "First Note",
                "note_line": "A first demo line."
            }),
            trigger: None,
            placeholder: None,
        };
        let engine = BasicPipelineEngine::new(
            Arc::new(DenoSandboxEngine::default()),
            resolve_engine_or_default(None),
            None,
        )
        .with_project_layout(Some(layout.clone()))
        .with_template_cache(new_template_cache())
        .with_data_root(root.to_path_buf());

        let first = engine
            .execute_async(&graph, &ctx)
            .await
            .expect("first generate");
        assert_eq!(first.value["site"]["status"], "written");
        assert_eq!(
            first.value["site"]["path"],
            "static/site-a/authors/demo-author/first-note/note.html"
        );
        assert_eq!(first.value["site"]["folder"], "static/site-a");
        assert_eq!(
            first.value["site"]["manifest_path"],
            "static/site-a/.zebflow-static-site.json"
        );

        let generated_path = layout
            .files_dir
            .join("static")
            .join("site-a")
            .join("authors")
            .join("demo-author")
            .join("first-note")
            .join("note.html");
        let generated_html = std::fs::read_to_string(&generated_path).expect("generated html");
        assert!(generated_html.contains("Demo Author - First Note"));
        assert!(generated_html.contains("A first demo line."));
        assert!(generated_html.contains("<b>snapshot</b>"));
        assert!(generated_html.contains("<i>panel unavailable</i>"));
        assert!(!generated_html.contains("RWE component error"));
        assert!(generated_html.contains("data-rwe-tw"));
        assert!(
            generated_html
                .contains("../../../_assets/libraries/zeb/react/0.1/runtime/zeb_react.mjs")
        );
        assert!(generated_html.contains("../../../_assets/project/icons/favicon.ico"));
        assert!(generated_html.contains("../../../_assets/branding/logo.svg"));
        assert!(
            layout
                .files_dir
                .join("static")
                .join("site-a")
                .join("_assets")
                .join("libraries")
                .join("zeb")
                .join("react")
                .join("0.1")
                .join("runtime")
                .join("zeb_react.mjs")
                .is_file()
        );
        assert!(
            layout
                .files_dir
                .join("static")
                .join("site-a")
                .join("_assets")
                .join("project")
                .join("icons")
                .join("favicon.ico")
                .is_file()
        );
        assert!(
            layout
                .files_dir
                .join("static")
                .join("site-a")
                .join("_assets")
                .join("branding")
                .join("logo.svg")
                .is_file()
        );
        let manifest = std::fs::read_to_string(
            layout
                .files_dir
                .join("static")
                .join("site-a")
                .join(".zebflow-static-site.json"),
        )
        .expect("manifest");
        assert!(manifest.contains("\"site_root\": \"static/site-a\""));
        assert!(manifest.contains("\"template\": \"pages/note.tsx\""));
        assert!(manifest.contains("\"path\": \"_assets/project/icons/favicon.ico\""));
        assert!(manifest.contains("\"path\": \"_assets/branding/logo.svg\""));

        let second = engine
            .execute_async(&graph, &ctx)
            .await
            .expect("second generate");
        assert_eq!(second.value["site"]["status"], "unchanged");
    }

    #[tokio::test]
    async fn regenerating_one_static_page_only_updates_that_page() {
        let (_tmp, root) = temp_root("zebflow-staticgen-single-page-update");
        let file = build_file_adapter(
            FileAdapterKind::Filesystem,
            root.to_path_buf(),
            std::sync::Arc::new(ProjectConfigurationService::new(root.join("users"))),
            None,
        );
        let layout = file
            .ensure_project_layout("superadmin", "example-project")
            .expect("layout");
        let template_dir = layout.repo_source_dir().join("pages");
        std::fs::create_dir_all(&template_dir).expect("template dir");
        std::fs::write(
            template_dir.join("note.tsx"),
            r#"
export const page = {
  html: {
    lang: "en",
  }
};

export function getPage(input) {
  return {
    head: {
      title: `${input.author_name} — ${input.note_title} | Demo Site`,
      description: `${input.note_title} notes by ${input.author_name}.`
    }
  };
}

export default function NotePage(input) {
  return (
    <Page>
      <main className="min-h-screen bg-white text-slate-900 p-6">
        <h1 className="text-3xl font-black">{input.author_name} - {input.note_title}</h1>
        <p className="mt-4">{input.note_line}</p>
      </main>
    </Page>
  );
}
"#,
        )
        .expect("template write");

        let graph = PipelineGraph {
            id: "generate-one-note".to_string(),
            description: None,
            metadata: None,
            notes: Vec::new(),
            entry_nodes: vec!["gen".to_string()],
            nodes: vec![PipelineNode {
                id: "gen".to_string(),
                kind: NODE_KIND.to_string(),
                input_pins: vec!["in".to_string()],
                output_pins: vec!["out".to_string()],
                config: json!({
                    "template": "pages/note.tsx",
                    "folder": "static/site-a",
                    "path": "{{ $input.letter_slug }}/{{ $input.author_slug }}/notes/{{ $input.note_slug }}/text/index.html"
                }),
            }],
            edges: vec![],
        };

        let engine = BasicPipelineEngine::new(
            Arc::new(DenoSandboxEngine::default()),
            resolve_engine_or_default(None),
            None,
        )
        .with_project_layout(Some(layout.clone()))
        .with_template_cache(new_template_cache())
        .with_data_root(root.to_path_buf());

        let sample_ctx = PipelineContext {
            owner: "superadmin".to_string(),
            project: "example-project".to_string(),
            pipeline: graph.id.clone(),
            request_id: "req-sample-1".to_string(),
            route: String::new(),
            input: json!({
                "letter_slug": "s",
                "author_slug": "sample-author",
                "note_slug": "second-note",
                "author_name": "Sample Author",
                "note_title": "Second Note",
                "note_line": "A second demo line."
            }),
            trigger: None,
            placeholder: None,
        };
        let demo_ctx = PipelineContext {
            owner: "superadmin".to_string(),
            project: "example-project".to_string(),
            pipeline: graph.id.clone(),
            request_id: "req-demo-1".to_string(),
            route: String::new(),
            input: json!({
                "letter_slug": "d",
                "author_slug": "demo-author",
                "note_slug": "first-note",
                "author_name": "Demo Author",
                "note_title": "First Note",
                "note_line": "A first demo line."
            }),
            trigger: None,
            placeholder: None,
        };

        engine
            .execute_async(&graph, &sample_ctx)
            .await
            .expect("generate sample page");
        engine
            .execute_async(&graph, &demo_ctx)
            .await
            .expect("generate demo page");

        let sample_path = layout
            .files_dir
            .join("static")
            .join("site-a")
            .join("s")
            .join("sample-author")
            .join("notes")
            .join("second-note")
            .join("text")
            .join("index.html");
        let demo_path = layout
            .files_dir
            .join("static")
            .join("site-a")
            .join("d")
            .join("demo-author")
            .join("notes")
            .join("first-note")
            .join("text")
            .join("index.html");

        let sample_before = std::fs::read_to_string(&sample_path).expect("sample before");
        let demo_before = std::fs::read_to_string(&demo_path).expect("demo before");
        assert!(sample_before.contains("A second demo line."));
        assert!(demo_before.contains("A first demo line."));

        let sample_updated_ctx = PipelineContext {
            request_id: "req-sample-2".to_string(),
            input: json!({
                "letter_slug": "s",
                "author_slug": "sample-author",
                "note_slug": "second-note",
                "author_name": "Sample Author",
                "note_title": "Second Note",
                "note_line": "A second demo line, edited."
            }),
            ..sample_ctx
        };

        let updated = engine
            .execute_async(&graph, &sample_updated_ctx)
            .await
            .expect("update sample page");
        assert_eq!(updated.value["site"]["status"], "written");
        assert_eq!(
            updated.value["site"]["path"],
            "static/site-a/s/sample-author/notes/second-note/text/index.html"
        );

        let sample_after = std::fs::read_to_string(&sample_path).expect("sample after");
        let demo_after = std::fs::read_to_string(&demo_path).expect("demo after");
        assert!(sample_after.contains("A second demo line, edited."));
        assert_eq!(demo_before, demo_after);
    }
}
