//! `n.web.static.generate` — render a TSX page once and persist it to project file storage.
//!
//! This node is the file-producing counterpart to [`super::web_response`]:
//! it uses the same RWE compile/render path, but instead of returning the HTML
//! to the current HTTP request it writes the rendered document into
//! `files/{public|private}/...`.
//!
//! The generated HTML is self-contained for the first release:
//! - project `styles/main.css` is inlined when present
//! - Tailwind CSS extracted by the RWE engine is inlined
//! - compiled client scripts are inlined as `<script type="module">`
//!
//! That keeps the artifact directly openable via `/files/{owner}/{project}/...`
//! without depending on render-script cache plumbing.

use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::pipeline::PipelineError;
use crate::pipeline::model::{
    DslFlag, DslFlagKind, LayoutItem, NodeDefinition, NodeFieldDef, NodeFieldType, SelectOptionDef,
};
use crate::rwe::{CompiledScript, TemplateSource};

pub const NODE_KIND: &str = "n.web.static.generate";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

fn default_scope() -> String {
    "public".to_string()
}

fn default_on_conflict() -> String {
    "overwrite".to_string()
}

/// Typed configuration for `n.web.static.generate`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Relative TSX template path under `repo/pipelines`.
    ///
    /// `pages/song.tsx` is the preferred form. If no extension is supplied,
    /// `.tsx` is assumed.
    pub template: String,
    /// Inline template markup hydrated by higher layers.
    ///
    /// This is intentionally hidden from normal authors and only exists so
    /// compiled graphs can carry already-loaded markup if desired.
    #[serde(default)]
    pub markup: Option<String>,
    /// `public` or `private`.
    #[serde(default = "default_scope")]
    pub scope: String,
    /// Relative output path inside `files/{scope}/`.
    ///
    /// Config expressions are resolved before execution, so values like
    /// `artists/{{ $input.artist_slug }}/{{ $input.song_slug }}/lyric.html`
    /// are supported without node-specific syntax.
    pub output_path: String,
    /// Optional route injected into the RWE render context as `ctx.route`.
    ///
    /// Defaults to `/files/{owner}/{project}/{scope}/{output_path}`.
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
                "WEB_STATIC_TEMPLATE_ROOT",
                format!(
                    "node '{node_id}' requires template_root to load '{}'",
                    template_rel
                ),
            ));
        };
        let abs = root.join(&template_rel);
        if !abs.starts_with(root) || !abs.is_file() {
            return Err(PipelineError::new(
                "WEB_STATIC_TEMPLATE_MISSING",
                format!("node '{node_id}' template '{}' not found", template_rel),
            ));
        }
        std::fs::read_to_string(&abs).map_err(|err| {
            PipelineError::new(
                "WEB_STATIC_TEMPLATE_READ",
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

/// Returns an output path relative to `files/{scope}/...`.
pub fn normalize_output_rel_path(scope: &str, output_path: &str) -> Result<String, PipelineError> {
    let scope = match scope.trim() {
        "public" => "public",
        "private" => "private",
        other => {
            return Err(PipelineError::new(
                "WEB_STATIC_SCOPE",
                format!("unsupported scope '{other}' — expected public or private"),
            ));
        }
    };

    let mut parts = Vec::new();
    for part in output_path.trim().replace('\\', "/").split('/') {
        let part = part.trim();
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." || part.contains('\0') {
            return Err(PipelineError::new(
                "WEB_STATIC_OUTPUT_PATH",
                "output_path must stay inside the project files directory",
            ));
        }
        parts.push(part.to_string());
    }
    if parts.is_empty() {
        return Err(PipelineError::new(
            "WEB_STATIC_OUTPUT_PATH",
            "output_path must not be empty",
        ));
    }
    Ok(format!("{scope}/{}", parts.join("/")))
}

/// Compute the route seen by the template at render time.
pub fn default_route(owner: &str, project: &str, rel_path: &str) -> String {
    format!("/files/{owner}/{project}/{rel_path}")
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

    if let Some(css) = hydration_payload.get("css").and_then(Value::as_str)
        && !css.trim().is_empty()
    {
        html = inject_before_head_end(
            &html,
            &format!("<style data-rwe-tw>{}</style>", escape_style_block(css)),
        );
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

/// Persist generated HTML with simple conflict handling and atomic replace.
pub fn write_generated_html(
    abs_path: &Path,
    html: &str,
    on_conflict: &str,
) -> Result<&'static str, PipelineError> {
    let bytes = html.as_bytes();
    if let Ok(existing) = std::fs::read(abs_path) {
        if existing == bytes {
            return Ok("unchanged");
        }
        match on_conflict.trim() {
            "overwrite" | "" => {}
            "skip" => return Ok("skipped"),
            "error" => {
                return Err(PipelineError::new(
                    "WEB_STATIC_CONFLICT",
                    format!("destination '{}' already exists", abs_path.display()),
                ));
            }
            other => {
                return Err(PipelineError::new(
                    "WEB_STATIC_CONFLICT_MODE",
                    format!(
                        "unsupported on_conflict value '{other}' — expected overwrite, skip, or error"
                    ),
                ));
            }
        }
    }

    if let Some(parent) = abs_path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| {
            PipelineError::new(
                "WEB_STATIC_MKDIR",
                format!("failed creating '{}': {err}", parent.display()),
            )
        })?;
    }

    let tmp_path = abs_path.with_extension(format!(
        "{}.tmp",
        abs_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("html")
    ));
    std::fs::write(&tmp_path, bytes).map_err(|err| {
        PipelineError::new(
            "WEB_STATIC_WRITE",
            format!("failed writing '{}': {err}", tmp_path.display()),
        )
    })?;
    std::fs::rename(&tmp_path, abs_path).map_err(|err| {
        let _ = std::fs::remove_file(&tmp_path);
        PipelineError::new(
            "WEB_STATIC_RENAME",
            format!("failed finalizing '{}': {err}", abs_path.display()),
        )
    })?;
    Ok("written")
}

/// Kind-level contract for the pipeline editor, DSL, and MCP-facing node help.
pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Web Static Generate".to_string(),
        description: "Render an RWE TSX template once and persist the HTML into project file storage. \
            Use this for static page generation, cached exports, and regeneration pipelines. \
            Generated files are written under files/public or files/private and can be served later via /files."
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
                        "url": { "type": "string" },
                        "route": { "type": "string" },
                        "template": { "type": "string" },
                        "scope": { "type": "string" },
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
            "required": ["template", "output_path"],
            "properties": {
                "template": { "type": "string", "description": "TSX page template relative to repo/pipelines (e.g. pages/lyrics.tsx)." },
                "scope": { "type": "string", "enum": ["public", "private"], "description": "Output file scope under files/." },
                "output_path": { "type": "string", "description": "Relative path inside files/{scope}/. Supports config expressions." },
                "route": { "type": "string", "description": "Optional route exposed to the template as ctx.route." },
                "on_conflict": { "type": "string", "enum": ["overwrite", "skip", "error"], "description": "What to do when the destination exists and content differs." }
            }
        }),
        dsl_flags: vec![
            DslFlag {
                flag: "--template".to_string(),
                config_key: "template".to_string(),
                description: "TSX page file relative to repo/pipelines, e.g. pages/lyrics.tsx".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
            },
            DslFlag {
                flag: "--scope".to_string(),
                config_key: "scope".to_string(),
                description: "Output scope: public or private (default: public)".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--output-path".to_string(),
                config_key: "output_path".to_string(),
                description: "Relative path under files/{scope}/. Supports {{ expr }} interpolation.".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
            },
            DslFlag {
                flag: "--route".to_string(),
                config_key: "route".to_string(),
                description: "Optional ctx.route override seen by the template during generation".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--on-conflict".to_string(),
                config_key: "on_conflict".to_string(),
                description: "overwrite, skip, or error when destination exists (default: overwrite)".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: vec![
            NodeFieldDef {
                name: "template".to_string(),
                label: "Template".to_string(),
                field_type: NodeFieldType::Datalist,
                data_source: Some(crate::pipeline::model::NodeFieldDataSource::TemplatesPages),
                placeholder: Some("pages/lyrics.tsx".to_string()),
                help: Some("TSX page template used to render the generated static file.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "scope".to_string(),
                label: "Output Scope".to_string(),
                field_type: NodeFieldType::Select,
                options: vec![
                    SelectOptionDef { value: "public".to_string(), label: "Public".to_string() },
                    SelectOptionDef { value: "private".to_string(), label: "Private".to_string() },
                ],
                default_value: Some(json!("public")),
                help: Some("Public files are served without auth. Private files still require a platform session.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "output_path".to_string(),
                label: "Output Path".to_string(),
                field_type: NodeFieldType::Text,
                placeholder: Some("artists/{{ $input.artist_slug }}/{{ $input.song_slug }}/lyric.html".to_string()),
                help: Some("Relative path inside files/{scope}/. Config expressions are resolved before generation.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "route".to_string(),
                label: "Render Route".to_string(),
                field_type: NodeFieldType::Text,
                placeholder: Some("/lyrics/{{ $input.artist_slug }}/{{ $input.song_slug }}".to_string()),
                help: Some("Optional ctx.route override. Leave empty to use the generated /files URL.".to_string()),
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
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("scope".to_string()),
                    LayoutItem::Field("on_conflict".to_string()),
                ],
            },
            LayoutItem::Field("output_path".to_string()),
            LayoutItem::Field("route".to_string()),
        ],
        ai_tool: Default::default(),
    }
}

fn normalize_template_rel_path(raw: &str) -> Result<String, PipelineError> {
    let trimmed = raw.trim().trim_start_matches('/').replace('\\', "/");
    if trimmed.is_empty() {
        return Err(PipelineError::new(
            "WEB_STATIC_TEMPLATE_PATH",
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
                "WEB_STATIC_TEMPLATE_PATH",
                "template path must stay inside the project template root",
            ));
        }
        parts.push(part.to_string());
    }
    if parts.is_empty() {
        return Err(PipelineError::new(
            "WEB_STATIC_TEMPLATE_PATH",
            "template path must not be empty",
        ));
    }
    let last = parts.last_mut().expect("parts not empty");
    if !last.contains('.') {
        last.push_str(".tsx");
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

    use super::{NODE_KIND, build_static_html, default_route, normalize_output_rel_path};
    use serde_json::json;

    use crate::language::DenoSandboxEngine;
    use crate::pipeline::engines::basic::new_template_cache;
    use crate::pipeline::{
        BasicPipelineEngine, PipelineContext, PipelineEngine, PipelineGraph, PipelineNode,
    };
    use crate::platform::adapters::file::build_file_adapter;
    use crate::platform::model::FileAdapterKind;
    use crate::rwe::resolve_engine_or_default;

    #[test]
    fn output_path_stays_scoped() {
        let rel = normalize_output_rel_path("public", "artists/a/song.html").expect("path");
        assert_eq!(rel, "public/artists/a/song.html");
        assert!(normalize_output_rel_path("public", "../escape.html").is_err());
    }

    #[test]
    fn default_route_targets_served_file() {
        assert_eq!(
            default_route(
                "superadmin",
                "example-project",
                "public/collections/a/item.html"
            ),
            "/files/superadmin/example-project/public/collections/a/item.html"
        );
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
        let file = build_file_adapter(FileAdapterKind::Filesystem, root.clone());
        let layout = file
            .ensure_project_layout("superadmin", "example-project")
            .expect("layout");
        let template_dir = layout.repo_pipelines_dir.join("pages");
        std::fs::create_dir_all(&template_dir).expect("template dir");
        std::fs::write(
            template_dir.join("lyric.tsx"),
            r#"
export const page = {
  head: { title: "Lyric" }
};

export const app = {};

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
            kind: "zebflow.pipeline".to_string(),
            version: "0.1".to_string(),
            id: "generate-lyric".to_string(),
            description: None,
            entry_nodes: vec!["gen".to_string()],
            nodes: vec![PipelineNode {
                id: "gen".to_string(),
                kind: NODE_KIND.to_string(),
                input_pins: vec!["in".to_string()],
                output_pins: vec!["out".to_string()],
                config: json!({
                    "template": "pages/lyric",
                    "output_path": "artists/{{ $input.artist_slug }}/{{ $input.song_slug }}/lyric.html"
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
        };
        let engine = BasicPipelineEngine::new(
            Arc::new(DenoSandboxEngine::default()),
            resolve_engine_or_default(None),
            None,
        )
        .with_template_root(Some(layout.repo_pipelines_dir.clone()))
        .with_template_cache(new_template_cache())
        .with_data_root(root.clone());

        let first = engine
            .execute_async(&graph, &ctx)
            .await
            .expect("first generate");
        assert_eq!(first.value["generated"]["status"], "written");
        assert_eq!(
            first.value["generated"]["path"],
            "public/artists/iwan-fals/bento/lyric.html"
        );
        assert_eq!(
            first.value["generated"]["url"],
            "/files/superadmin/example-project/public/artists/iwan-fals/bento/lyric.html"
        );

        let generated_path = layout
            .files_dir
            .join("public")
            .join("artists")
            .join("iwan-fals")
            .join("bento")
            .join("lyric.html");
        let generated_html = std::fs::read_to_string(&generated_path).expect("generated html");
        assert!(generated_html.contains("Iwan Fals - Bento"));
        assert!(generated_html.contains("Namaku Bento."));
        assert!(generated_html.contains("data-rwe-tw"));

        let second = engine
            .execute_async(&graph, &ctx)
            .await
            .expect("second generate");
        assert_eq!(second.value["generated"]["status"], "unchanged");

        let _ = std::fs::remove_dir_all(&root);
    }
}
