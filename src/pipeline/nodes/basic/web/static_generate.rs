//! `n.web.static.generate` — render a TSX page once and persist it to Zebflow FS.
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

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::pipeline::PipelineError;
use crate::pipeline::model::NodeCapability;
use crate::pipeline::model::{
    DslFlag, DslFlagKind, LayoutItem, NodeDefinition, NodeFieldDef, NodeFieldType, SelectOptionDef,
};
use super::static_site;
use crate::rwe::{CompiledScript, TemplateSource};

pub const NODE_KIND: &str = "n.web.static.generate";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

fn default_on_conflict() -> String {
    "overwrite".to_string()
}

/// Typed configuration for `n.web.static.generate`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Relative TSX template path under `repo/pipelines`.
    /// Example: `pages/song.tsx`.
    pub template: String,
    /// Inline template markup hydrated by higher layers.
    ///
    /// This is intentionally hidden from normal authors and only exists so
    /// compiled graphs can carry already-loaded markup if desired.
    #[serde(default)]
    pub markup: Option<String>,
    /// The page's path inside the site root.
    ///
    /// Config expressions are resolved before execution, so values like
    /// `artists/{{ $input.artist_slug }}/{{ $input.song_slug }}/lyric.html`
    /// are supported without node-specific syntax.
    pub path: String,
    /// The site's root folder in the store (default: `site`); every page of
    /// one site shares it, with one manifest and one `_assets/`.
    #[serde(default)]
    pub site_root: Option<String>,
    /// The store to write to; saved explicitly at registration.
    #[serde(default)]
    pub store: Option<String>,
    /// Optional route injected into the RWE render context as `ctx.route`.
    ///
    /// Defaults to the page's address on the site: `/` for `index.html`,
    /// `/blog/` for `blog/index.html`, `/about.html` for `about.html`.
    #[serde(default)]
    pub route: Option<String>,
    /// `overwrite`, `skip`, or `error` when the file already exists and content differs.
    #[serde(default = "default_on_conflict")]
    pub on_conflict: String,
}

/// Resolve the template file and load its markup from disk when `markup` was not
/// injected beforehand.
pub fn resolve_template_source(
    node_id: &str,
    config: &Config,
    template_root: Option<&Path>,
) -> Result<TemplateSource, PipelineError> {
    let template_rel = normalize_template_rel_path(&config.template)?;
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
                "FW_NODE_WEB_STATIC_GENERATE_TEMPLATE_ROOT",
                format!(
                    "node '{node_id}' requires template_root to load '{}'",
                    template_rel
                ),
            ));
        };
        let abs = root.join(&template_rel);
        if !abs.starts_with(root) || !abs.is_file() {
            return Err(PipelineError::new(
                "FW_NODE_WEB_STATIC_GENERATE_TEMPLATE_MISSING",
                format!("node '{node_id}' template '{}' not found", template_rel),
            ));
        }
        std::fs::read_to_string(&abs).map_err(|err| {
            PipelineError::new(
                "FW_NODE_WEB_STATIC_GENERATE_TEMPLATE_READ",
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

/// Returns an output path relative to the project Zebflow FS root.
pub fn normalize_output_rel_path(output_path: &str) -> Result<String, PipelineError> {
    let mut parts = Vec::new();
    for part in output_path.trim().replace('\\', "/").split('/') {
        let part = part.trim();
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." || part.contains('\0') {
            return Err(PipelineError::new(
                "FW_NODE_WEB_STATIC_GENERATE_OUTPUT_PATH",
                "output_path must stay inside the project files directory",
            ));
        }
        parts.push(part.to_string());
    }
    if parts.is_empty() {
        return Err(PipelineError::new(
            "FW_NODE_WEB_STATIC_GENERATE_OUTPUT_PATH",
            "output_path must not be empty",
        ));
    }
    Ok(parts.join("/"))
}

pub fn effective_site_root_rel_path(config: &Config) -> Result<Option<String>, PipelineError> {
    let raw = config
        .site_root
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("site");
    static_site::normalize_site_root_rel_path(raw).map(Some)
}

pub fn effective_output_rel_path(config: &Config) -> Result<String, PipelineError> {
    let site_root_rel = effective_site_root_rel_path(config)?.unwrap_or_else(|| "site".to_string());
    static_site::page_rel_path_from_site_root(&site_root_rel, &config.path)
}

pub fn effective_page_output_path(config: &Config) -> Result<String, PipelineError> {
    static_site::normalize_page_output_path(&config.path)
}

/// The route a page has when its site is served at the root of an address.
pub fn default_route(config: &Config) -> Result<String, PipelineError> {
    static_site::route_path_for_output_path("/", &effective_page_output_path(config)?)
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
    match crate::pipeline::nodes::shared::project_store::read_capped(store, rel_path, "FW_NODE_WEB_STATIC_GENERATE_READ") {
        Ok(existing) => {
            if existing == bytes {
                return Ok("unchanged");
            }
            match on_conflict.trim() {
                "overwrite" | "" => {}
                "skip" => return Ok("skipped"),
                "error" => {
                    return Err(PipelineError::new(
                        "FW_NODE_WEB_STATIC_GENERATE_CONFLICT",
                        format!("destination '{rel_path}' already exists"),
                    ));
                }
                other => {
                    return Err(PipelineError::new(
                        "FW_NODE_WEB_STATIC_GENERATE_CONFLICT_MODE",
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
                "FW_NODE_WEB_STATIC_GENERATE_READ",
                format!("failed reading '{rel_path}': {err}"),
            ));
        }
    }
    store.put(rel_path, bytes).map_err(|err| {
        PipelineError::new(
            "FW_NODE_WEB_STATIC_GENERATE_WRITE",
            format!("failed writing '{rel_path}': {err}"),
        )
    })?;
    Ok("written")
}

/// Kind-level contract for the pipeline editor, DSL, and MCP-facing node help.
pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Filesystem, NodeCapability::Process],
        title: "Web Static Generate".to_string(),
        description: "Render an RWE TSX template once and persist the HTML into project file storage. \
            Use this for static page generation, cached exports, and regeneration pipelines. \
            Generated files are written as Zebflow FS objects and should be treated as static artifacts, not same-origin hosted pages."
            .to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "generated": {
                    "type": "object",
                    "properties": {
                        "status": { "type": "string" },
                        "path": { "type": "string" },
                        "route": { "type": "string" },
                        "store": { "type": ["string", "null"] },
                        "file": { "description": "A durable FileRef for the page" },
                        "origin": { "type": ["string", "null"], "description": "The site's serve origin from Studio → Files, if it has one" },
                        "template": { "type": "string" },
                        "site_root": { "type": ["string", "null"] },
                        "manifest_path": { "type": ["string", "null"] },
                        "asset_group": { "type": "string" },
                        "bytes": { "type": "integer" }
                    }
                }
            }
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: json!({
            "type": "object",
            "required": ["template", "path"],
            "properties": {
                "template": { "type": "string", "description": "TSX page template relative to repo/pipelines. Must end with .tsx (e.g. pages/lyrics.tsx)." },
                "site_root": { "type": "string", "description": "The site's root folder in the store (default: site)." },
                "path": { "type": "string", "description": "The page's path inside the site root. Supports config expressions." },
                "store": { "type": "string", "description": "The store to write to; saved explicitly at registration." },
                "route": { "type": "string", "description": "Optional route exposed to the template as ctx.route." },
                "on_conflict": { "type": "string", "enum": ["overwrite", "skip", "error"], "description": "What to do when the destination exists and content differs." }
            }
        }),
        dsl_flags: vec![
            DslFlag {
                flag: "--template".to_string(),
                config_key: "template".to_string(),
                description: "TSX page file relative to the source root. Must end with .tsx, e.g. pages/post.tsx".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
                ..Default::default()
            },
            DslFlag {
                flag: "--path".to_string(),
                config_key: "path".to_string(),
                description: "The page's path inside the site root, e.g. posts/{{ input.slug }}.html. Supports {{ expr }} interpolation.".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
                ..Default::default()
            },
            DslFlag {
                flag: "--site-root".to_string(),
                config_key: "site_root".to_string(),
                description: "The site's root folder in the store (default: site). Pages of one site share it, with one manifest and one _assets/.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
                ..Default::default()
            },
            crate::pipeline::nodes::shared::project_store::store_flag(),
            DslFlag {
                flag: "--route".to_string(),
                config_key: "route".to_string(),
                description: "Optional ctx.route override seen by the template during generation".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
                ..Default::default()
            },
            DslFlag {
                flag: "--on-conflict".to_string(),
                config_key: "on_conflict".to_string(),
                description: "overwrite, skip, or error when destination exists (default: overwrite)".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
                ..Default::default()
            },
        ],
        fields: vec![
            NodeFieldDef {
                name: "template".to_string(),
                label: "Template".to_string(),
                field_type: NodeFieldType::Datalist,
                data_source: Some(crate::pipeline::model::NodeFieldDataSource::TemplatesPages),
                placeholder: Some("pages/lyrics.tsx".to_string()),
                help: Some("TSX page template used to render the generated static file. Must end with .tsx.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "site_root".to_string(),
                label: "Site Root".to_string(),
                field_type: NodeFieldType::Text,
                placeholder: Some("static/musicsite".to_string()),
                help: Some("The site's root folder in the store (default: site).".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "path".to_string(),
                label: "Path".to_string(),
                field_type: NodeFieldType::Text,
                placeholder: Some("artists/{{ $input.artist_slug }}/{{ $input.song_slug }}/lyric.html".to_string()),
                help: Some("The page's path inside the site root. Config expressions are resolved before generation.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "store".to_string(),
                label: "Store".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("`local`, or the id of an s3 credential. Empty: the project's default, saved when the pipeline is registered.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "route".to_string(),
                label: "Render Route".to_string(),
                field_type: NodeFieldType::Text,
                placeholder: Some("/lyrics/{{ $input.artist_slug }}/{{ $input.song_slug }}".to_string()),
                help: Some("Optional ctx.route override. Leave empty to use the generated /fs URL.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "on_conflict".to_string(),
                label: "On Conflict".to_string(),
                field_type: NodeFieldType::Select,
                options: vec![
                    SelectOptionDef { value: "overwrite".to_string(), label: "Overwrite".to_string() },
                    SelectOptionDef { value: "skip".to_string(), label: "Skip".to_string() },
                    SelectOptionDef { value: "error".to_string(), label: "Error".to_string() },
                ],
                default_value: Some(json!("overwrite")),
                help: Some("Skip keeps the old file, overwrite replaces it, error stops the pipeline.".to_string()),
                ..Default::default()
            },
        ],
        layout: vec![
            LayoutItem::Field("template".to_string()),
            LayoutItem::Field("on_conflict".to_string()),
            LayoutItem::Field("site_root".to_string()),
            LayoutItem::Field("path".to_string()),
            LayoutItem::Field("store".to_string()),
            LayoutItem::Field("route".to_string()),
        ],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Render one post to a static file", r#"web.static.generate --template pages/post.tsx --path "posts/{{ input.rows[0].slug }}.html" --route "/posts/{{ input.rows[0].slug }}""#)
                .output(serde_json::json!({ "generated": { "status": "written", "path": "site/posts/hello.html", "route": "/posts/hello", "template": "pages/post.tsx", "site_root": "site", "store": "local", "file": { "__zf_type": "file_ref", "backend": "zebfs", "store": "local", "ref": "site/posts/hello.html", "filename": "hello.html", "mime": "text/html", "kind": "binary", "size": 5120, "sha256": "sha256:…", "lifecycle": "durable", "origin": "web.static.generate", "trust": "generated" } } })),
        ],
        ..Default::default()
    }
}

fn normalize_template_rel_path(raw: &str) -> Result<String, PipelineError> {
    let trimmed = raw.trim().trim_start_matches('/').replace('\\', "/");
    if trimmed.is_empty() {
        return Err(PipelineError::new(
            "FW_NODE_WEB_STATIC_GENERATE_TEMPLATE_PATH",
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
                "FW_NODE_WEB_STATIC_GENERATE_TEMPLATE_PATH",
                "template path must stay inside the project template root",
            ));
        }
        parts.push(part.to_string());
    }
    if parts.is_empty() {
        return Err(PipelineError::new(
            "FW_NODE_WEB_STATIC_GENERATE_TEMPLATE_PATH",
            "template path must not be empty",
        ));
    }
    let last = parts.last().expect("parts not empty");
    if !last.ends_with(".tsx") {
        return Err(PipelineError::new(
            "FW_NODE_WEB_STATIC_GENERATE_TEMPLATE_PATH",
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
    use std::sync::Arc;

    use super::{
        NODE_KIND, build_static_html, default_route, effective_output_rel_path,
        normalize_output_rel_path,
    };
    use serde_json::json;

    use crate::language::DenoSandboxEngine;
    use crate::pipeline::engines::basic::new_template_cache;
    use crate::pipeline::{
        BasicPipelineEngine, PipelineContext, PipelineEngine, PipelineGraph, PipelineNode,
    };
    use crate::platform::adapters::file::build_file_adapter;
    use crate::platform::model::FileAdapterKind;
    use crate::platform::services::project_config::ProjectConfigurationService;
    use crate::rwe::resolve_engine_or_default;

    #[test]
    fn output_path_stays_scoped() {
        let rel = normalize_output_rel_path("artists/a/song.html").expect("path");
        assert_eq!(rel, "artists/a/song.html");
        assert!(normalize_output_rel_path("../escape.html").is_err());
        let rel = effective_output_rel_path(&super::Config {
            path: "artists/a/song.html".to_string(),
            site_root: Some("static/musicsite".to_string()),
            ..Default::default()
        })
        .expect("site root path");
        assert_eq!(rel, "static/musicsite/artists/a/song.html");
    }

    #[test]
    fn template_path_requires_explicit_tsx_extension() {
        assert_eq!(
            super::normalize_template_rel_path("pages/lyrics.tsx").expect("tsx template"),
            "pages/lyrics.tsx"
        );
        let err =
            super::normalize_template_rel_path("pages/lyrics").expect_err("missing extension");
        assert_eq!(err.code, "FW_NODE_WEB_STATIC_GENERATE_TEMPLATE_PATH");
        assert!(err.message.contains(".tsx"));
    }

    #[test]
    fn default_route_is_the_page_address_on_its_site() {
        let config = |path: &str| super::Config { path: path.to_string(), ..Default::default() };
        assert_eq!(default_route(&config("index.html")).unwrap(), "/");
        assert_eq!(default_route(&config("blog/index.html")).unwrap(), "/blog/");
        assert_eq!(default_route(&config("collections/a/item.html")).unwrap(), "/collections/a/item.html");
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
        let root = std::env::temp_dir().join(format!(
            "zebflow-staticgen-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix time")
                .as_nanos()
        ));
        let file = build_file_adapter(
            FileAdapterKind::Filesystem,
            root.clone(),
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
                    "site_root": "static/musicsite",
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
        .with_data_root(root.clone());

        let first = engine
            .execute_async(&graph, &ctx)
            .await
            .expect("first generate");
        assert_eq!(first.value["generated"]["status"], "written");
        assert_eq!(
            first.value["generated"]["path"],
            "static/musicsite/artists/iwan-fals/bento/lyric.html"
        );
        assert_eq!(first.value["generated"]["site_root"], "static/musicsite");
        assert_eq!(
            first.value["generated"]["manifest_path"],
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
        assert_eq!(second.value["generated"]["status"], "unchanged");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn regenerating_one_static_page_only_updates_that_page() {
        let root = std::env::temp_dir().join(format!(
            "zebflow-staticgen-single-page-update-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix time")
                .as_nanos()
        ));
        let file = build_file_adapter(
            FileAdapterKind::Filesystem,
            root.clone(),
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
                    "site_root": "static/musicsite",
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
        .with_data_root(root.clone());

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
        assert_eq!(updated.value["generated"]["status"], "written");
        assert_eq!(
            updated.value["generated"]["path"],
            "static/musicsite/a/aurora/songs/runaway/lyrics/index.html"
        );

        let aurora_after = std::fs::read_to_string(&aurora_path).expect("aurora after");
        let iwan_after = std::fs::read_to_string(&iwan_path).expect("iwan after");
        assert!(aurora_after.contains("I was listening to the ocean, again."));
        assert_eq!(iwan_before, iwan_after);

        let _ = std::fs::remove_dir_all(&root);
    }
}
