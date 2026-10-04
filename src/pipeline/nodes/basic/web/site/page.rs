//! `web.site.generate` — render a TSX page once and persist it to Zebflow FS.
//!
//! This node is the file-producing counterpart to [`super::response`]:
//! it uses the same RWE compile/render path, but instead of returning the HTML
//! to the current HTTP request it writes the rendered document into
//! a Zebflow FS object path.
//!
//! The generated HTML is self-contained for the first release:
//! - project `styles/main.css` is inlined when present
//! - Tailwind CSS extracted by the RWE engine is inlined
//! - compiled client scripts are inlined as `<script type="module">`
//!
//! That keeps the artifact self-contained without depending on render-script
//! cache plumbing.

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
        let abs = root.join(&template_rel);
        if !abs.starts_with(root) || !abs.is_file() {
            return Err(PipelineError::new(
                "FW_NODE_WEB_SITE_GENERATE_TEMPLATE_MISSING",
                format!("node '{node_id}' template '{}' not found", template_rel),
            ));
        }
        std::fs::read_to_string(&abs).map_err(|err| {
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
        let project_css_path = root.join("styles").join("main.css");
        if let Ok(project_css) = std::fs::read_to_string(&project_css_path)
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

/// Persist one generated file into the project's store — its one active
/// backend — with simple conflict handling. The store writes atomically.
pub fn write_generated_object(
    store: &crate::zebfs::ZebFs,
    rel_path: &str,
    contents: &str,
    on_conflict: &str,
) -> Result<&'static str, PipelineError> {
    let bytes = contents.as_bytes();
    match crate::pipeline::nodes::shared::project_store::read_capped(store, rel_path, "FW_NODE_WEB_SITE_GENERATE_READ") {
        Ok(existing) => {
            if existing == bytes {
                return Ok("unchanged");
            }
            match on_conflict.trim() {
                "overwrite" | "" => {}
                "skip" => return Ok("skipped"),
                "error" => {
                    return Err(PipelineError::new(
                        "FW_NODE_WEB_SITE_GENERATE_CONFLICT",
                        format!("destination '{rel_path}' already exists"),
                    ));
                }
                other => {
                    return Err(PipelineError::new(
                        "FW_NODE_WEB_SITE_GENERATE_CONFLICT",
                        format!(
                            "unsupported on_conflict value '{other}' — expected overwrite, skip, or error"
                        ),
                    ));
                }
            }
        }
        Err(_) if store.head(rel_path).is_err() => {}
        Err(err) => {
            return Err(PipelineError::new(
                "FW_NODE_WEB_SITE_GENERATE_READ",
                format!("failed reading '{rel_path}': {err}"),
            ));
        }
    }
    store.put(rel_path, bytes).map_err(|err| {
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
        assert_eq!(page_key(&page("artists/a/song.html")).unwrap(), "site/artists/a/song.html");
        assert!(page_key(&page("../escape.html")).is_err());
        let named = Config { folder: Some("static/musicsite".to_string()), ..page("artists/a/song.html") };
        assert_eq!(page_key(&named).unwrap(), "static/musicsite/artists/a/song.html");
        let escaping = Config { folder: Some("../up".to_string()), ..page("a.html") };
        assert!(page_key(&escaping).is_err());
    }

    #[test]
    fn template_path_requires_explicit_tsx_extension() {
        assert_eq!(
            super::normalize_template_rel_path("pages/lyrics.tsx").expect("tsx template"),
            "pages/lyrics.tsx"
        );
        let err =
            super::normalize_template_rel_path("pages/lyrics").expect_err("missing extension");
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
            template_dir.join("lyric.tsx"),
            r#"
import { ErrorBoundary, useSyncExternalStore } from "zeb/react";
function LiveStatus() {
  const status = useSyncExternalStore(() => () => {}, () => 'online', () => 'snapshot');
  return <b>{status}</b>;
}
function BrokenPanel() { throw new Error('static recovery'); }
export const page = {
  head: {
    title: "Lyric",
    icons: [
      { rel: "icon", href: "/static/superadmin/example-project/icons/favicon.ico" }
    ]
  }
};

export const app = {};

export default function LyricPage(input) {
  return (
    <Page>
      <main className="min-h-screen bg-white text-slate-900 p-6">
        <img src="/assets/branding/logo.svg" alt="Zebflow" />
        <h1 className="text-3xl font-black">{input.artist_name} - {input.song_title}</h1>
        <p className="mt-4">{input.lyric_line}</p>
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
            id: "generate-lyric".to_string(),
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
                    "template": "pages/lyric.tsx",
                    "folder": "static/musicsite",
                    "path": "artists/{{ $input.artist_slug }}/{{ $input.song_slug }}/lyric.html"
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
                "artist_slug": "iwan-fals",
                "song_slug": "bento",
                "artist_name": "Iwan Fals",
                "song_title": "Bento",
                "lyric_line": "Namaku Bento."
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
            "static/musicsite/artists/iwan-fals/bento/lyric.html"
        );
        assert_eq!(first.value["site"]["folder"], "static/musicsite");
        assert_eq!(
            first.value["site"]["manifest_path"],
            "static/musicsite/.zebflow-static-site.json"
        );

        let generated_path = layout
            .files_dir
            .join("static")
            .join("musicsite")
            .join("artists")
            .join("iwan-fals")
            .join("bento")
            .join("lyric.html");
        let generated_html = std::fs::read_to_string(&generated_path).expect("generated html");
        assert!(generated_html.contains("Iwan Fals - Bento"));
        assert!(generated_html.contains("Namaku Bento."));
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
                .join("musicsite")
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
                .join("musicsite")
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
                .join("musicsite")
                .join("_assets")
                .join("branding")
                .join("logo.svg")
                .is_file()
        );
        let manifest = std::fs::read_to_string(
            layout
                .files_dir
                .join("static")
                .join("musicsite")
                .join(".zebflow-static-site.json"),
        )
        .expect("manifest");
        assert!(manifest.contains("\"site_root\": \"static/musicsite\""));
        assert!(manifest.contains("\"template\": \"pages/lyric.tsx\""));
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
            template_dir.join("lyric.tsx"),
            r#"
export const page = {
  html: {
    lang: "en",
  }
};

export function getPage(input) {
  return {
    head: {
      title: `${input.artist_name} — ${input.song_title} | Music Site`,
      description: `${input.song_title} lyrics by ${input.artist_name}.`
    }
  };
}

export default function LyricPage(input) {
  return (
    <Page>
      <main className="min-h-screen bg-white text-slate-900 p-6">
        <h1 className="text-3xl font-black">{input.artist_name} - {input.song_title}</h1>
        <p className="mt-4">{input.lyric_line}</p>
      </main>
    </Page>
  );
}
"#,
        )
        .expect("template write");

        let graph = PipelineGraph {
            id: "generate-one-lyric".to_string(),
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
                    "template": "pages/lyric.tsx",
                    "folder": "static/musicsite",
                    "path": "{{ $input.letter_slug }}/{{ $input.artist_slug }}/songs/{{ $input.song_slug }}/lyrics/index.html"
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

        let aurora_ctx = PipelineContext {
            owner: "superadmin".to_string(),
            project: "example-project".to_string(),
            pipeline: graph.id.clone(),
            request_id: "req-aurora-1".to_string(),
            route: String::new(),
            input: json!({
                "letter_slug": "a",
                "artist_slug": "aurora",
                "song_slug": "runaway",
                "artist_name": "Aurora",
                "song_title": "Runaway",
                "lyric_line": "I was listening to the ocean."
            }),
            trigger: None,
            placeholder: None,
        };
        let iwan_ctx = PipelineContext {
            owner: "superadmin".to_string(),
            project: "example-project".to_string(),
            pipeline: graph.id.clone(),
            request_id: "req-iwan-1".to_string(),
            route: String::new(),
            input: json!({
                "letter_slug": "i",
                "artist_slug": "iwan-fals",
                "song_slug": "bento",
                "artist_name": "Iwan Fals",
                "song_title": "Bento",
                "lyric_line": "Namaku Bento."
            }),
            trigger: None,
            placeholder: None,
        };

        engine
            .execute_async(&graph, &aurora_ctx)
            .await
            .expect("generate aurora page");
        engine
            .execute_async(&graph, &iwan_ctx)
            .await
            .expect("generate iwan page");

        let aurora_path = layout
            .files_dir
            .join("static")
            .join("musicsite")
            .join("a")
            .join("aurora")
            .join("songs")
            .join("runaway")
            .join("lyrics")
            .join("index.html");
        let iwan_path = layout
            .files_dir
            .join("static")
            .join("musicsite")
            .join("i")
            .join("iwan-fals")
            .join("songs")
            .join("bento")
            .join("lyrics")
            .join("index.html");

        let aurora_before = std::fs::read_to_string(&aurora_path).expect("aurora before");
        let iwan_before = std::fs::read_to_string(&iwan_path).expect("iwan before");
        assert!(aurora_before.contains("I was listening to the ocean."));
        assert!(iwan_before.contains("Namaku Bento."));

        let aurora_updated_ctx = PipelineContext {
            request_id: "req-aurora-2".to_string(),
            input: json!({
                "letter_slug": "a",
                "artist_slug": "aurora",
                "song_slug": "runaway",
                "artist_name": "Aurora",
                "song_title": "Runaway",
                "lyric_line": "I was listening to the ocean, again."
            }),
            ..aurora_ctx
        };

        let updated = engine
            .execute_async(&graph, &aurora_updated_ctx)
            .await
            .expect("update aurora page");
        assert_eq!(updated.value["site"]["status"], "written");
        assert_eq!(
            updated.value["site"]["path"],
            "static/musicsite/a/aurora/songs/runaway/lyrics/index.html"
        );

        let aurora_after = std::fs::read_to_string(&aurora_path).expect("aurora after");
        let iwan_after = std::fs::read_to_string(&iwan_path).expect("iwan after");
        assert!(aurora_after.contains("I was listening to the ocean, again."));
        assert_eq!(iwan_before, iwan_after);
    }
}
