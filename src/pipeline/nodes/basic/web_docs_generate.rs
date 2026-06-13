//! `n.web.docs.generate` — scan markdown docs and statically generate an SEO-first doc site.
//!
//! This node is the opinionated docs-site generator for Zebflow projects:
//! - source content lives under `repo/docs/<docs_root>/`
//! - the customizable render template lives at `repo/pipelines/<template_folder>/docs.template.tsx`
//! - if the template is missing, Zebflow scaffolds it automatically
//! - generated HTML is written under Zebflow FS

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::pipeline::PipelineError;
use crate::pipeline::model::{
    DslFlag, DslFlagKind, LayoutItem, NodeDefinition, NodeFieldDef, NodeFieldType,
};
use crate::pipeline::nodes::basic::web_static_site;
use crate::rwe::TemplateSource;

pub const NODE_KIND: &str = "n.web.docs.generate";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

fn default_meta_file() -> String {
    "_meta.yaml".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Relative folder under `repo/docs/`.
    pub docs_root: String,
    /// Zebflow FS output directory.
    pub output_dir: String,
    /// Optional static site root under Zebflow FS.
    ///
    /// Defaults to `output_dir`.
    #[serde(default)]
    pub site_root: Option<String>,
    /// Relative folder under `repo/pipelines/`.
    pub template_folder: String,
    #[serde(default)]
    pub site_title: Option<String>,
    #[serde(default, alias = "base_url")]
    pub deploy_base_url: Option<String>,
    #[serde(default, alias = "base_path")]
    pub deploy_base_path: Option<String>,
    #[serde(default = "default_meta_file")]
    pub meta_file: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PageFrontmatter {
    pub title: Option<String>,
    pub description: Option<String>,
    pub order: Option<i64>,
    pub keywords: Vec<String>,
    pub canonical: Option<String>,
    pub noindex: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FolderMeta {
    pub title: Option<String>,
    pub order: Option<i64>,
    pub collapsed: Option<bool>,
    pub nav: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocHeading {
    pub level: u8,
    pub id: String,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocPage {
    pub slug_segments: Vec<String>,
    pub title: String,
    pub description: String,
    pub keywords: Vec<String>,
    pub canonical: Option<String>,
    pub noindex: bool,
    pub markdown: String,
    pub headings: Vec<DocHeading>,
    pub source_rel_path: String,
    pub output_rel_path: String,
    pub route_path: String,
    pub order: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocSidebarItem {
    pub key: String,
    pub title: String,
    pub href: Option<String>,
    pub active: bool,
    pub expanded: bool,
    pub children: Vec<DocSidebarItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocCrumb {
    pub title: String,
    pub href: String,
}

#[derive(Debug, Clone)]
pub struct DocsSite {
    pub site_title: String,
    pub deploy_base_url: Option<String>,
    pub deploy_base_path: String,
    pub site_root_rel: String,
    pub template_rel_path: String,
    pub template_source: TemplateSource,
    pub pages: Vec<DocPage>,
    pub sidebar: Vec<DocSidebarItem>,
    pub sitemap_xml: String,
    pub search_index_json: String,
    pub search_index_route: String,
}

#[derive(Debug, Clone)]
struct FolderNode {
    segment: String,
    title: Option<String>,
    order: Option<i64>,
    collapsed: bool,
    nav: Vec<String>,
    page_index: Option<usize>,
    children: BTreeMap<String, FolderNode>,
}

impl FolderNode {
    fn new(segment: String) -> Self {
        Self {
            segment,
            title: None,
            order: None,
            collapsed: false,
            nav: Vec::new(),
            page_index: None,
            children: BTreeMap::new(),
        }
    }
}

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Web Docs Generate".to_string(),
        description: "Scan repo/docs markdown content, auto-scaffold docs.template.tsx when missing, and generate a fully static docs artifact tree under Zebflow FS. Treat output as publishable artifacts, not same-origin hosted pages.".to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "docs_generated": {
                    "type": "object",
                    "properties": {
                        "status": { "type": "string" },
                        "site_title": { "type": "string" },
                        "template": { "type": "string" },
                        "docs_root": { "type": "string" },
                        "output_dir": { "type": "string" },
                        "site_root": { "type": "string" },
                        "deploy_base_url": { "type": ["string", "null"] },
                        "deploy_base_path": { "type": "string" },
                        "manifest_path": { "type": "string" },
                        "asset_group": { "type": "string" },
                        "page_count": { "type": "integer" },
                        "generated_files": { "type": "integer" },
                        "skipped_files": { "type": "integer" },
                        "sitemap_path": { "type": "string" },
                        "search_index_path": { "type": "string" },
                        "urls": { "type": "array", "items": { "type": "string" } }
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
            "required": ["docs_root", "output_dir", "template_folder"],
            "properties": {
                "docs_root": { "type": "string", "description": "Folder under repo/docs containing the markdown doc tree." },
                "output_dir": { "type": "string", "description": "Zebflow FS folder where the generated site will be written." },
                "site_root": { "type": "string", "description": "Optional static site root under Zebflow FS. Defaults to output_dir." },
                "template_folder": { "type": "string", "description": "Folder under repo/pipelines/. docs.template.tsx is auto-created here when missing." },
                "site_title": { "type": "string" },
                "deploy_base_url": { "type": "string", "description": "Optional absolute deployed site origin used for canonical URLs and sitemap entries." },
                "deploy_base_path": { "type": "string", "description": "Optional deployed URL base path seen by generated pages. Defaults to /{output_dir}/." },
                "meta_file": { "type": "string", "description": "Folder metadata file name. Default: _meta.yaml" }
            }
        }),
        dsl_flags: vec![
            DslFlag {
                flag: "--docs-root".to_string(),
                config_key: "docs_root".to_string(),
                description: "Folder under repo/docs/ containing the documentation source tree.".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
            },
            DslFlag {
                flag: "--output-dir".to_string(),
                config_key: "output_dir".to_string(),
                description: "Zebflow FS folder where the generated site is written.".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
            },
            DslFlag {
                flag: "--site-root".to_string(),
                config_key: "site_root".to_string(),
                description: "Optional static site root under Zebflow FS. Defaults to output_dir.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--template-folder".to_string(),
                config_key: "template_folder".to_string(),
                description: "Folder under repo/pipelines/. docs.template.tsx is auto-created here when missing.".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
            },
            DslFlag {
                flag: "--site-title".to_string(),
                config_key: "site_title".to_string(),
                description: "Site title fallback used by the scaffold and generated payload.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--deploy-base-url".to_string(),
                config_key: "deploy_base_url".to_string(),
                description: "Optional absolute deployed site origin used for canonical URLs and sitemap.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--deploy-base-path".to_string(),
                config_key: "deploy_base_path".to_string(),
                description: "Optional deployed URL base path seen by generated pages. Defaults to /{output_dir}/.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--meta-file".to_string(),
                config_key: "meta_file".to_string(),
                description: "Folder metadata file name (default: _meta.yaml).".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: vec![
            NodeFieldDef {
                name: "docs_root".to_string(),
                label: "Docs Root".to_string(),
                field_type: NodeFieldType::Text,
                placeholder: Some("sekejap-docs".to_string()),
                help: Some("Folder under repo/docs/ that contains the markdown documentation tree.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "output_dir".to_string(),
                label: "Output Dir".to_string(),
                field_type: NodeFieldType::Text,
                placeholder: Some("docs".to_string()),
                help: Some("Zebflow FS folder where the generated site will be written.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "site_root".to_string(),
                label: "Site Root".to_string(),
                field_type: NodeFieldType::Text,
                placeholder: Some("static/sekejap-docs".to_string()),
                help: Some("Optional shared static site root under Zebflow FS. Defaults to Output Dir.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "template_folder".to_string(),
                label: "Template Folder".to_string(),
                field_type: NodeFieldType::Text,
                placeholder: Some("pages/docs".to_string()),
                help: Some("Folder under repo/pipelines/. docs.template.tsx is auto-created here when missing.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "site_title".to_string(),
                label: "Site Title".to_string(),
                field_type: NodeFieldType::Text,
                placeholder: Some("Sekejap Docs".to_string()),
                help: Some("Display title used in generated documentation pages and metadata.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "deploy_base_url".to_string(),
                label: "Deploy Base URL".to_string(),
                field_type: NodeFieldType::Text,
                placeholder: Some("https://db.sekejap.life".to_string()),
                help: Some("Optional absolute site origin for canonical URLs and sitemap entries.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "deploy_base_path".to_string(),
                label: "Deploy Base Path".to_string(),
                field_type: NodeFieldType::Text,
                placeholder: Some("/docs".to_string()),
                help: Some("URL base path for generated pages. Leave empty to derive from output_dir.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "meta_file".to_string(),
                label: "Meta File".to_string(),
                field_type: NodeFieldType::Text,
                placeholder: Some("_meta.yaml".to_string()),
                default_value: Some(json!("_meta.yaml")),
                help: Some("Folder metadata file name read from docs directories. Defaults to _meta.yaml.".to_string()),
                ..Default::default()
            },
        ],
        layout: vec![
            LayoutItem::Field("docs_root".to_string()),
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("output_dir".to_string()),
                    LayoutItem::Field("site_root".to_string()),
                ],
            },
            LayoutItem::Field("template_folder".to_string()),
            LayoutItem::Field("site_title".to_string()),
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("deploy_base_url".to_string()),
                    LayoutItem::Field("deploy_base_path".to_string()),
                ],
            },
            LayoutItem::Field("meta_file".to_string()),
        ],
        ai_tool: Default::default(),
        ..Default::default()
    }
}

pub fn load_site(config: &Config, template_root: &Path) -> Result<DocsSite, PipelineError> {
    let repo_root = template_root.parent().ok_or_else(|| {
        PipelineError::new(
            "WEB_DOCS_TEMPLATE_ROOT",
            "template_root must point to repo/pipelines so docs generation can resolve repo/docs",
        )
    })?;
    let docs_root_rel = normalize_rel_dir_path(&config.docs_root, "docs_root")?;
    let output_dir_rel = normalize_rel_dir_path(&config.output_dir, "output_dir")?;
    let site_root_rel = effective_site_root_rel_path(config, &output_dir_rel)?;
    let template_folder_rel = normalize_rel_dir_path(&config.template_folder, "template_folder")?;
    let meta_file = normalize_meta_file_name(&config.meta_file)?;

    let docs_root_abs = repo_root.join("docs").join(&docs_root_rel);
    if !docs_root_abs.is_dir() {
        return Err(PipelineError::new(
            "WEB_DOCS_ROOT_MISSING",
            format!("docs root '{}' not found", docs_root_abs.display()),
        ));
    }

    let (template_rel_path, template_source) =
        ensure_template_scaffold(template_root, &template_folder_rel, config)?;
    let deploy_base_path = web_static_site::normalize_deploy_base_path(
        config.deploy_base_path.as_deref(),
        &output_dir_rel,
    )?;

    let mut folder_meta = HashMap::new();
    let mut pages = Vec::new();
    collect_docs(
        &docs_root_abs,
        &docs_root_abs,
        &meta_file,
        &deploy_base_path,
        &mut folder_meta,
        &mut pages,
    )?;
    if pages.is_empty() {
        return Err(PipelineError::new(
            "WEB_DOCS_EMPTY",
            format!("docs root '{}' contains no markdown pages", docs_root_rel),
        ));
    }

    pages.sort_by(|a, b| a.source_rel_path.cmp(&b.source_rel_path));

    let site_title = config
        .site_title
        .clone()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            folder_meta
                .get("")
                .and_then(|m: &FolderMeta| m.title.clone())
        })
        .unwrap_or_else(|| titleize_segment(docs_root_rel.rsplit('/').next().unwrap_or("Docs")));

    let (sidebar, ordered_indices) = build_sidebar_and_order(&pages, &folder_meta);
    let ordered_pages = ordered_indices
        .into_iter()
        .map(|idx| pages[idx].clone())
        .collect::<Vec<_>>();
    let sitemap_xml = build_sitemap_xml(config.deploy_base_url.as_deref(), &ordered_pages);
    let search_index_route =
        web_static_site::route_path_for_output_path(&deploy_base_path, "search-index.json")?;
    let search_index_json = build_search_index_json(&ordered_pages);

    Ok(DocsSite {
        site_title,
        deploy_base_url: web_static_site::normalize_deploy_base_url(
            config.deploy_base_url.as_deref(),
        ),
        deploy_base_path,
        site_root_rel,
        template_rel_path,
        template_source,
        pages: ordered_pages,
        sidebar,
        sitemap_xml,
        search_index_json,
        search_index_route,
    })
}

pub fn page_payload(
    site: &DocsSite,
    page_index: usize,
    source_payload: Value,
) -> Result<Value, PipelineError> {
    let page = site.pages.get(page_index).ok_or_else(|| {
        PipelineError::new(
            "WEB_DOCS_PAGE_INDEX",
            format!("page index {page_index} out of range"),
        )
    })?;
    let breadcrumbs = breadcrumbs_for(&site.pages, &site.deploy_base_path, page);
    let (prev, next) = prev_next_for(&site.pages, page_index);

    Ok(json!({
        "site": {
            "title": site.site_title,
            "deploy_base_url": site.deploy_base_url,
            "deploy_base_path": site.deploy_base_path,
            "home_path": site
                .pages
                .first()
                .map(|root| root.route_path.clone())
                .unwrap_or_else(|| "/".to_string()),
            "search_index_href": site.search_index_route,
            "base_url": site.deploy_base_url,
            "base_path": site.deploy_base_path,
        },
        "page": {
            "title": page.title,
            "description": page.description,
            "keywords": page.keywords,
            "canonical": effective_canonical(site.deploy_base_url.as_deref(), &page.route_path, page.canonical.as_deref()),
            "noindex": page.noindex,
            "markdown": page.markdown,
            "headings": page.headings,
            "breadcrumbs": breadcrumbs,
            "prev": prev,
            "next": next,
            "route_path": page.route_path,
            "source_rel_path": page.source_rel_path,
        },
        "sidebar": mark_active_sidebar(&site.sidebar, &page.route_path),
        "source": source_payload,
    }))
}

pub fn effective_site_root_rel_path(
    config: &Config,
    output_dir_rel: &str,
) -> Result<String, PipelineError> {
    let raw = config
        .site_root
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(output_dir_rel);
    web_static_site::normalize_site_root_rel_path(raw)
}

pub fn sitemap_rel_path(site_root_rel: &str) -> String {
    format!("{}/sitemap.xml", site_root_rel.trim_end_matches('/'))
}

pub fn search_index_rel_path(site_root_rel: &str) -> String {
    format!("{}/search-index.json", site_root_rel.trim_end_matches('/'))
}

pub fn default_route(page: &DocPage) -> String {
    page.route_path.clone()
}

pub fn output_rel_path(page: &DocPage, site_root_rel: &str) -> Result<String, PipelineError> {
    web_static_site::page_rel_path_from_site_root(site_root_rel, &page.output_rel_path)
}

pub fn apply_page_seo(html: String, site: &DocsSite, page_index: usize) -> String {
    let Some(page) = site.pages.get(page_index) else {
        return html;
    };
    let canonical = effective_canonical(
        site.deploy_base_url.as_deref(),
        &page.route_path,
        page.canonical.as_deref(),
    );
    let mut out = html;
    let title = format!("{} | {}", page.title, site.site_title);
    out = replace_tag_content(&out, "title", &html_escape(&title));
    out = replace_or_insert_meta(&out, "name", "description", &html_escape(&page.description));
    if let Some(canonical) = canonical {
        out = upsert_link_rel(&out, "canonical", &canonical);
    }
    if page.noindex {
        out = replace_or_insert_meta(&out, "name", "robots", "noindex, nofollow");
    }
    out = replace_or_insert_meta(&out, "property", "og:title", &html_escape(&page.title));
    out = replace_or_insert_meta(
        &out,
        "property",
        "og:description",
        &html_escape(&page.description),
    );
    out
}

fn normalize_rel_dir_path(raw: &str, field: &str) -> Result<String, PipelineError> {
    let mut parts = Vec::new();
    for part in raw.trim().trim_matches('/').replace('\\', "/").split('/') {
        let part = part.trim();
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." || part.contains('\0') {
            return Err(PipelineError::new(
                "WEB_DOCS_PATH",
                format!("{field} must stay inside the project directory"),
            ));
        }
        parts.push(part.to_string());
    }
    if parts.is_empty() {
        return Err(PipelineError::new(
            "WEB_DOCS_PATH",
            format!("{field} must not be empty"),
        ));
    }
    Ok(parts.join("/"))
}

fn normalize_meta_file_name(raw: &str) -> Result<String, PipelineError> {
    let trimmed = raw.trim();
    if trimmed.is_empty()
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed == "."
        || trimmed == ".."
    {
        return Err(PipelineError::new(
            "WEB_DOCS_META_FILE",
            "meta_file must be a simple file name like _meta.yaml",
        ));
    }
    Ok(trimmed.to_string())
}

fn ensure_template_scaffold(
    template_root: &Path,
    template_folder_rel: &str,
    config: &Config,
) -> Result<(String, TemplateSource), PipelineError> {
    let template_rel_path = format!("{template_folder_rel}/docs.template.tsx");
    let template_abs_path = template_root.join(&template_rel_path);
    if !template_abs_path.exists() {
        if let Some(parent) = template_abs_path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| {
                PipelineError::new(
                    "WEB_DOCS_TEMPLATE_DIR",
                    format!("failed creating '{}': {err}", parent.display()),
                )
            })?;
        }
        std::fs::write(&template_abs_path, default_template_source(config)).map_err(|err| {
            PipelineError::new(
                "WEB_DOCS_TEMPLATE_WRITE",
                format!(
                    "failed writing docs scaffold '{}': {err}",
                    template_abs_path.display()
                ),
            )
        })?;
    }

    let markup = std::fs::read_to_string(&template_abs_path).map_err(|err| {
        PipelineError::new(
            "WEB_DOCS_TEMPLATE_READ",
            format!("failed reading '{}': {err}", template_abs_path.display()),
        )
    })?;

    Ok((
        template_rel_path.clone(),
        TemplateSource {
            id: template_rel_path,
            source_path: Some(template_abs_path),
            markup,
        },
    ))
}

fn default_template_source(config: &Config) -> String {
    let fallback_title = config
        .site_title
        .clone()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| "Docs".to_string());
    format!(
        r##"import Markdown from "zeb/markdown";

const DOCS_MARKDOWN_CSS = `
.docs-markdown {{
  color: #1f2937;
  font-size: 16px;
  line-height: 1.8;
}}
.docs-markdown h1,
.docs-markdown h2,
.docs-markdown h3,
.docs-markdown h4,
.docs-markdown h5,
.docs-markdown h6 {{
  color: #020617;
  font-weight: 800;
  letter-spacing: -0.02em;
  line-height: 1.2;
  scroll-margin-top: 96px;
}}
.docs-markdown h1 {{
  font-size: 2.4rem;
  margin: 0 0 1.25rem;
}}
.docs-markdown h2 {{
  font-size: 1.7rem;
  margin: 3rem 0 1rem;
  padding-top: 0.25rem;
  border-top: 1px solid #e2e8f0;
}}
.docs-markdown h3 {{
  font-size: 1.25rem;
  margin: 2rem 0 0.85rem;
}}
.docs-markdown p,
.docs-markdown ul,
.docs-markdown ol,
.docs-markdown pre,
.docs-markdown table,
.docs-markdown blockquote {{
  margin: 1rem 0;
}}
.docs-markdown ul,
.docs-markdown ol {{
  padding-left: 1.4rem;
}}
.docs-markdown ul {{
  list-style: disc;
}}
.docs-markdown ol {{
  list-style: decimal;
}}
.docs-markdown li + li {{
  margin-top: 0.35rem;
}}
.docs-markdown li > ul,
.docs-markdown li > ol {{
  margin-top: 0.5rem;
}}
.docs-markdown a {{
  color: #c2410c;
  text-decoration: underline;
  text-underline-offset: 0.18em;
}}
.docs-markdown a:hover {{
  color: #9a3412;
}}
.docs-markdown strong {{
  color: #020617;
  font-weight: 700;
}}
.docs-markdown code {{
  background: #fff7ed;
  color: #9a3412;
  border-radius: 0.375rem;
  padding: 0.12rem 0.38rem;
  font-size: 0.92em;
}}
.docs-markdown pre {{
  overflow-x: auto;
  background: #0f172a;
  color: #e2e8f0;
  border-radius: 0.75rem;
  padding: 1rem 1.1rem;
}}
.docs-markdown pre code {{
  background: transparent;
  color: inherit;
  padding: 0;
  border-radius: 0;
}}
.docs-markdown blockquote {{
  border-left: 3px solid #fdba74;
  padding-left: 1rem;
  color: #475569;
}}
.docs-markdown hr {{
  border: 0;
  border-top: 1px solid #e2e8f0;
  margin: 2rem 0;
}}
.docs-markdown table {{
  width: 100%;
  border-collapse: collapse;
  font-size: 0.95rem;
}}
.docs-markdown th,
.docs-markdown td {{
  border: 1px solid #e2e8f0;
  padding: 0.7rem 0.8rem;
  text-align: left;
  vertical-align: top;
}}
.docs-markdown th {{
  background: #f8fafc;
  color: #0f172a;
  font-weight: 700;
}}
.docs-markdown img {{
  border-radius: 0.75rem;
}}
`;

export const page = {{
  html: {{
    lang: "en",
  }},
}};

export function getPage(input) {{
  const page = input?.page || {{}};
  const site = input?.site || {{}};
  return {{
    head: {{
      title: site.title ? `${{page.title || "Untitled"}} | ${{site.title}}` : (page.title || "Untitled"),
      description: page.description || "",
      links: page.canonical ? [
        {{ rel: "canonical", href: page.canonical }}
      ] : [],
      meta: [
        {{ property: "og:title", content: page.title || "Untitled" }},
        {{ property: "og:description", content: page.description || "" }}
      ]
    }}
  }};
}}

function MarkdownBlock(props) {{
  return (
    <div class="docs-markdown">
      <Markdown
        content={{props.content || ""}}
        class="max-w-none"
      />
    </div>
  );
}}

function SidebarItem(props) {{
  const item = props.item || {{}};
  const kids = Array.isArray(item.children) ? item.children : [];
  if (kids.length === 0) {{
    return (
      <li>
        <a
          href={{item.href || "#"}}
          class={{item.active ? "font-semibold text-orange-600" : "text-slate-700 hover:text-slate-950"}}
        >
          {{item.title}}
        </a>
      </li>
    );
  }}
  return (
    <li>
      <details open={{item.expanded}}>
        <summary class="cursor-pointer font-semibold text-slate-900">
          {{item.href ? <a href={{item.href}} class="hover:text-orange-600">{{item.title}}</a> : item.title}}
        </summary>
        <ul class="mt-2 ml-3 space-y-2 border-l border-slate-200 pl-4">
          {{kids.map((child) => <SidebarItem item={{child}} />)}}
        </ul>
      </details>
    </li>
  );
}}

export default function DocsTemplate(input) {{
  const page = input.page || {{}};
  const sidebar = Array.isArray(input.sidebar) ? input.sidebar : [];
  const toc = Array.isArray(page.headings) ? page.headings.filter((item) => Number(item.level) >= 2 && Number(item.level) <= 3) : [];
  const breadcrumbs = Array.isArray(page.breadcrumbs) ? page.breadcrumbs : [];
  const [query, setQuery] = useState("");
  const [searchIndex, setSearchIndex] = useState([]);
  const [searchState, setSearchState] = useState("idle");
  const searchHref = input.site?.search_index_href || null;
  const jumpToHeading = useCallback((event, id) => {{
    if (!id) return;
    event?.preventDefault?.();
    const target = document.getElementById(id);
    if (!target) return;
    target.scrollIntoView({{ block: "start", behavior: "smooth" }});
    if (window?.history?.replaceState) {{
      window.history.replaceState(null, "", "#" + id);
    }} else {{
      window.location.hash = id;
    }}
  }}, []);

  useEffect(() => {{
    if (!searchHref || searchIndex.length > 0 || searchState === "loading") return;
    setSearchState("loading");
    fetch(searchHref)
      .then((response) => response.ok ? response.json() : [])
      .then((payload) => {{
        setSearchIndex(Array.isArray(payload) ? payload : []);
        setSearchState("ready");
      }})
      .catch(() => {{
        setSearchIndex([]);
        setSearchState("error");
      }});
  }}, [searchHref, searchIndex.length, searchState]);

  const searchResults = useMemo(() => {{
    const value = String(query || "").trim().toLowerCase();
    if (!value) return [];
    const terms = value.split(/\s+/).filter(Boolean);
    return (Array.isArray(searchIndex) ? searchIndex : [])
      .map((entry) => {{
        const title = String(entry.title || "");
        const description = String(entry.description || "");
        const keywords = Array.isArray(entry.keywords) ? entry.keywords.join(" ") : "";
        const headingsText = Array.isArray(entry.headings) ? entry.headings.join(" ") : "";
        const body = String(entry.excerpt || "");
        const haystack = (title + " " + description + " " + keywords + " " + headingsText + " " + body).toLowerCase();
        let score = 0;
        for (const term of terms) {{
          if (!haystack.includes(term)) return null;
          if (title.toLowerCase() === term) score += 120;
          else if (title.toLowerCase().startsWith(term)) score += 80;
          else if (title.toLowerCase().includes(term)) score += 50;
          if (keywords.toLowerCase().includes(term)) score += 25;
          if (headingsText.toLowerCase().includes(term)) score += 18;
          if (description.toLowerCase().includes(term)) score += 12;
          if (body.toLowerCase().includes(term)) score += 6;
        }}
        return {{ ...entry, score }};
      }})
      .filter(Boolean)
      .sort((a, b) => b.score - a.score || String(a.title).localeCompare(String(b.title)))
      .slice(0, 16);
  }}, [query, searchIndex]);

  return (
    <Page>
      <style>{{DOCS_MARKDOWN_CSS}}</style>
      <div class="min-h-screen bg-white text-slate-900 lg:h-screen lg:overflow-hidden">
        <div class="border-b border-slate-200 px-4 py-3 lg:hidden">
          <a href={{input.site?.home_path || input.site?.deploy_base_path || input.site?.base_path || "/"}} class="text-xs font-semibold uppercase tracking-[0.22em] text-orange-600">{fallback_title}</a>
          <div class="mt-1 text-lg font-bold tracking-tight">{{page.title || input.site?.title || "Docs"}}</div>
        </div>
        <div class="lg:grid lg:h-screen lg:grid-cols-12">
          <aside class="border-b border-slate-200 px-4 py-4 lg:col-span-3 lg:overflow-y-auto lg:border-b-0 lg:border-r lg:px-5 lg:py-5">
            <div class="text-xs font-semibold uppercase tracking-[0.22em] text-orange-600">{fallback_title}</div>
              <div class="mt-3">
                <input
                  type="search"
                  value={{query}}
                  onInput={{(event) => setQuery(event.currentTarget.value)}}
                  placeholder="Search the docs"
                  class="h-10 w-full rounded-md border border-slate-300 px-3 text-sm outline-none ring-0 focus:border-orange-500"
                />
              <div class="mt-2 text-[11px] text-slate-500">
                {{query.trim() ? `${{searchResults.length}} result${{searchResults.length === 1 ? "" : "s"}}` : "Type to search headings, keywords, and content"}}
              </div>
            </div>
            {{query.trim() ? (
              <div class="mt-4 border-b border-slate-200 pb-4">
                <ul class="space-y-3">
                  {{searchResults.length === 0 ? (
                    <li class="text-sm text-slate-500">
                      {{searchState === "loading" ? "Loading search index..." : "No matching pages."}}
                    </li>
                  ) : searchResults.map((entry) => (
                    <li>
                      <a href={{entry.href}} class="block">
                        <div class="text-sm font-semibold text-slate-900 hover:text-orange-600">{{entry.title}}</div>
                        {{entry.section ? <div class="mt-0.5 text-[11px] uppercase tracking-[0.16em] text-slate-400">{{entry.section}}</div> : null}}
                        {{entry.description ? <div class="mt-1 text-sm text-slate-600">{{entry.description}}</div> : null}}
                      </a>
                    </li>
                  ))}}
                </ul>
              </div>
            ) : null}}
            <nav class="mt-4 pb-8">
              <div class="mb-3 text-[11px] font-semibold uppercase tracking-[0.18em] text-slate-400">Contents</div>
              <ul class="space-y-2 text-[15px]">
                {{sidebar.map((item) => <SidebarItem item={{item}} />)}}
              </ul>
            </nav>
          </aside>
          <main class="min-w-0 px-4 py-5 lg:col-span-6 lg:overflow-y-auto lg:px-8 lg:py-8">
            <div class="mx-auto max-w-3xl">
              <nav class="mb-3 flex flex-wrap items-center gap-2 text-xs text-slate-500">
                {{breadcrumbs.map((crumb, index) => (
                  <span class="inline-flex items-center gap-2">
                    {{index > 0 ? <span>/</span> : null}}
                    <a href={{crumb.href}} class="hover:text-slate-800">{{crumb.title}}</a>
                  </span>
                ))}}
              </nav>
              <header class="mb-8 border-b border-slate-200 pb-5">
                <div class="text-xs font-semibold uppercase tracking-[0.22em] text-orange-600">{{input.site?.title || "Docs"}}</div>
                <h1 class="mt-2 text-3xl font-black tracking-tight text-slate-950">{{page.title || input.site?.title || "Docs"}}</h1>
                {{page.description ? <p class="mt-3 max-w-2xl text-base leading-7 text-slate-600">{{page.description}}</p> : null}}
              </header>
              <article class="pb-8">
                <MarkdownBlock content={{page.markdown || ""}} />
              </article>
              <div class="mt-8 grid gap-3 border-t border-slate-200 pt-5 md:grid-cols-2">
                {{page.prev?.href ? (
                  <a href={{page.prev.href}} class="block border border-slate-200 px-4 py-4 hover:border-orange-300">
                    <div class="text-[11px] uppercase tracking-[0.18em] text-slate-400">Previous</div>
                    <div class="mt-1 font-semibold">{{page.prev.title}}</div>
                  </a>
                ) : <div />}}
                {{page.next?.href ? (
                  <a href={{page.next.href}} class="block border border-slate-200 px-4 py-4 text-right hover:border-orange-300">
                    <div class="text-[11px] uppercase tracking-[0.18em] text-slate-400">Next</div>
                    <div class="mt-1 font-semibold">{{page.next.title}}</div>
                  </a>
                ) : <div />}}
              </div>
            </div>
          </main>
          <aside class="hidden border-l border-slate-200 px-5 py-8 lg:col-span-3 lg:block lg:overflow-y-auto">
            <div class="text-[11px] font-semibold uppercase tracking-[0.18em] text-slate-400">On this page</div>
            <ul class="mt-4 space-y-2 text-sm">
              {{toc.length === 0 ? <li class="text-slate-400">No sections</li> : toc.map((item) => (
                <li class={{Number(item.level) === 3 ? "ml-4" : ""}}>
                  <a
                    href={{"#" + item.id}}
                    onClick={{(event) => jumpToHeading(event, item.id)}}
                    class="text-slate-700 hover:text-slate-900"
                  >
                    {{item.text}}
                  </a>
                </li>
              ))}}
            </ul>
          </aside>
        </div>
      </div>
    </Page>
  );
}}
"##
    )
}

fn collect_docs(
    root_abs: &Path,
    dir_abs: &Path,
    meta_file: &str,
    deploy_base_path: &str,
    folder_meta: &mut HashMap<String, FolderMeta>,
    pages: &mut Vec<DocPage>,
) -> Result<(), PipelineError> {
    let dir_rel = rel_dir_string(root_abs, dir_abs)?;
    let meta_path = dir_abs.join(meta_file);
    if meta_path.is_file() {
        let content = std::fs::read_to_string(&meta_path).map_err(|err| {
            PipelineError::new(
                "WEB_DOCS_META_READ",
                format!("failed reading '{}': {err}", meta_path.display()),
            )
        })?;
        folder_meta.insert(dir_rel.clone(), parse_folder_meta(&content));
    }

    let mut entries = std::fs::read_dir(dir_abs)
        .map_err(|err| {
            PipelineError::new(
                "WEB_DOCS_READ_DIR",
                format!("failed reading '{}': {err}", dir_abs.display()),
            )
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| {
            PipelineError::new(
                "WEB_DOCS_READ_DIR",
                format!("failed reading '{}': {err}", dir_abs.display()),
            )
        })?;
    entries.sort_by_key(|entry| entry.path());

    for entry in entries {
        let path = entry.path();
        if path.is_dir() {
            collect_docs(
                root_abs,
                &path,
                meta_file,
                deploy_base_path,
                folder_meta,
                pages,
            )?;
            continue;
        }
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if name == meta_file || !name.ends_with(".md") {
            continue;
        }

        let rel = path.strip_prefix(root_abs).map_err(|_| {
            PipelineError::new(
                "WEB_DOCS_REL_PATH",
                format!("failed resolving relative path for '{}'", path.display()),
            )
        })?;
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        let raw = std::fs::read_to_string(&path).map_err(|err| {
            PipelineError::new(
                "WEB_DOCS_READ_PAGE",
                format!("failed reading '{}': {err}", path.display()),
            )
        })?;
        let (frontmatter, markdown) = split_frontmatter(&raw);
        let slug_segments = slug_segments_for_markdown(&rel_str);
        let output_rel_path = output_rel_path_for(&slug_segments);
        let route_path =
            web_static_site::route_path_for_output_path(deploy_base_path, &output_rel_path)?;
        let headings = extract_headings(&markdown);
        let title = frontmatter
            .title
            .clone()
            .filter(|s| !s.trim().is_empty())
            .or_else(|| headings.first().map(|h| h.text.clone()))
            .unwrap_or_else(|| {
                titleize_segment(
                    rel.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("docs")
                        .trim_end_matches(".md"),
                )
            });
        let description = frontmatter
            .description
            .clone()
            .unwrap_or_else(|| first_paragraph(&markdown));
        pages.push(DocPage {
            slug_segments,
            title,
            description,
            keywords: frontmatter.keywords.clone(),
            canonical: frontmatter.canonical.clone(),
            noindex: frontmatter.noindex,
            markdown,
            headings,
            source_rel_path: rel_str,
            output_rel_path,
            route_path,
            order: frontmatter.order.unwrap_or(0),
        });
    }

    Ok(())
}

fn build_sidebar_and_order(
    pages: &[DocPage],
    folder_meta: &HashMap<String, FolderMeta>,
) -> (Vec<DocSidebarItem>, Vec<usize>) {
    let mut root = FolderNode::new(String::new());
    apply_folder_meta(&mut root, folder_meta.get(""));
    for (idx, page) in pages.iter().enumerate() {
        let folder_segments =
            if page.source_rel_path.ends_with("/index.md") || page.source_rel_path == "index.md" {
                page.slug_segments.clone()
            } else if page.slug_segments.is_empty() {
                Vec::new()
            } else {
                page.slug_segments[..page.slug_segments.len() - 1].to_vec()
            };
        let is_index =
            page.source_rel_path.ends_with("/index.md") || page.source_rel_path == "index.md";
        if is_index {
            let dir_key = folder_segments.join("/");
            let current = node_mut_for_segments(&mut root, &folder_segments, folder_meta);
            apply_folder_meta(current, folder_meta.get(&dir_key));
            current.page_index = Some(idx);
            current.title.get_or_insert_with(|| page.title.clone());
            current.order.get_or_insert(page.order);
        } else {
            let current = node_mut_for_segments(&mut root, &folder_segments, folder_meta);
            let leaf_segment = page.slug_segments.last().cloned().unwrap_or_default();
            let leaf = current
                .children
                .entry(leaf_segment.clone())
                .or_insert_with(|| FolderNode::new(leaf_segment));
            leaf.page_index = Some(idx);
            leaf.title.get_or_insert_with(|| page.title.clone());
            leaf.order.get_or_insert(page.order);
        }
    }

    let mut ordered = Vec::new();
    let mut items = root
        .children
        .values()
        .map(|node| to_sidebar_item(node, pages, &mut ordered))
        .collect::<Vec<_>>();
    if let Some(root_idx) = root.page_index {
        ordered.insert(0, root_idx);
        let page = &pages[root_idx];
        items.insert(
            0,
            DocSidebarItem {
                key: page.route_path.clone(),
                title: page.title.clone(),
                href: Some(page.route_path.clone()),
                active: false,
                expanded: true,
                children: Vec::new(),
            },
        );
    }
    (sort_sidebar_items(items), ordered)
}

fn node_mut_for_segments<'a>(
    root: &'a mut FolderNode,
    segments: &[String],
    folder_meta: &HashMap<String, FolderMeta>,
) -> &'a mut FolderNode {
    if segments.is_empty() {
        return root;
    }
    let segment = segments[0].clone();
    let dir_key = segments[..1].join("/");
    let child = root
        .children
        .entry(segment.clone())
        .or_insert_with(|| FolderNode::new(segment));
    apply_folder_meta(child, folder_meta.get(&dir_key));
    node_mut_for_segments_inner(child, segments, 1, folder_meta)
}

fn node_mut_for_segments_inner<'a>(
    current: &'a mut FolderNode,
    segments: &[String],
    depth: usize,
    folder_meta: &HashMap<String, FolderMeta>,
) -> &'a mut FolderNode {
    if depth >= segments.len() {
        return current;
    }
    let segment = segments[depth].clone();
    let dir_key = segments[..=depth].join("/");
    let child = current
        .children
        .entry(segment.clone())
        .or_insert_with(|| FolderNode::new(segment));
    apply_folder_meta(child, folder_meta.get(&dir_key));
    node_mut_for_segments_inner(child, segments, depth + 1, folder_meta)
}

fn apply_folder_meta(node: &mut FolderNode, meta: Option<&FolderMeta>) {
    if let Some(meta) = meta {
        if node.title.is_none() {
            node.title = meta.title.clone();
        }
        if node.order.is_none() {
            node.order = meta.order;
        }
        if meta.collapsed.unwrap_or(false) {
            node.collapsed = true;
        }
        if !meta.nav.is_empty() {
            node.nav = meta.nav.clone();
        }
    }
}

fn to_sidebar_item(
    node: &FolderNode,
    pages: &[DocPage],
    ordered: &mut Vec<usize>,
) -> DocSidebarItem {
    let mut children = node
        .children
        .values()
        .map(|child| to_sidebar_item(child, pages, ordered))
        .collect::<Vec<_>>();
    children = sort_sidebar_items(children);

    let (title, href, active) = if let Some(idx) = node.page_index {
        ordered.push(idx);
        let page = &pages[idx];
        (page.title.clone(), Some(page.route_path.clone()), false)
    } else {
        (
            node.title
                .clone()
                .unwrap_or_else(|| titleize_segment(&node.segment)),
            None,
            false,
        )
    };

    DocSidebarItem {
        key: if href.is_some() {
            href.clone().unwrap_or_default()
        } else {
            format!("group:{}", node.segment)
        },
        title,
        href,
        active,
        expanded: !node.collapsed,
        children,
    }
}

fn sort_sidebar_items(items: Vec<DocSidebarItem>) -> Vec<DocSidebarItem> {
    let mut items = items;
    items.sort_by(|a, b| {
        let a_group = !a.children.is_empty();
        let b_group = !b.children.is_empty();
        match (a_group, b_group) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.title.to_lowercase().cmp(&b.title.to_lowercase()),
        }
    });
    items
}

fn mark_active_sidebar(items: &[DocSidebarItem], current: &str) -> Vec<DocSidebarItem> {
    items
        .iter()
        .map(|item| mark_active_sidebar_item(item, current))
        .collect()
}

fn mark_active_sidebar_item(item: &DocSidebarItem, current: &str) -> DocSidebarItem {
    let children = item
        .children
        .iter()
        .map(|child| mark_active_sidebar_item(child, current))
        .collect::<Vec<_>>();
    let child_active = children.iter().any(|child| child.active || child.expanded);
    let self_active = item.href.as_deref() == Some(current);
    DocSidebarItem {
        key: item.key.clone(),
        title: item.title.clone(),
        href: item.href.clone(),
        active: self_active,
        expanded: item.expanded || child_active || self_active,
        children,
    }
}

fn breadcrumbs_for(pages: &[DocPage], deploy_base_path: &str, page: &DocPage) -> Vec<DocCrumb> {
    let mut out = Vec::new();
    for depth in 0..page.slug_segments.len() {
        let current_segments = page.slug_segments[..=depth].to_vec();
        let route = web_static_site::route_path_for_output_path(
            deploy_base_path,
            &output_rel_path_for(&current_segments),
        )
        .unwrap_or_else(|_| page.route_path.clone());
        if let Some(found) = pages.iter().find(|candidate| candidate.route_path == route) {
            out.push(DocCrumb {
                title: found.title.clone(),
                href: found.route_path.clone(),
            });
        } else {
            out.push(DocCrumb {
                title: titleize_segment(&page.slug_segments[depth]),
                href: route,
            });
        }
    }
    if out.is_empty() {
        out.push(DocCrumb {
            title: page.title.clone(),
            href: page.route_path.clone(),
        });
    }
    out
}

fn prev_next_for(pages: &[DocPage], page_index: usize) -> (Value, Value) {
    let prev = page_index
        .checked_sub(1)
        .and_then(|idx| pages.get(idx))
        .map(|page| json!({ "title": page.title, "href": page.route_path }))
        .unwrap_or(Value::Null);
    let next = pages
        .get(page_index + 1)
        .map(|page| json!({ "title": page.title, "href": page.route_path }))
        .unwrap_or(Value::Null);
    (prev, next)
}

fn effective_canonical(
    base_url: Option<&str>,
    route_path: &str,
    override_value: Option<&str>,
) -> Option<String> {
    if let Some(value) = override_value.filter(|s| !s.trim().is_empty()) {
        return Some(value.to_string());
    }
    web_static_site::absolute_deploy_url(base_url, route_path)
}

fn build_sitemap_xml(base_url: Option<&str>, pages: &[DocPage]) -> String {
    let Some(base_url) = base_url.filter(|s| !s.trim().is_empty()) else {
        return String::new();
    };
    let mut out = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<urlset xmlns=\"http://www.sitemaps.org/schemas/sitemap/0.9\">\n",
    );
    for page in pages {
        out.push_str("  <url><loc>");
        out.push_str(&xml_escape(&format!(
            "{}",
            web_static_site::absolute_deploy_url(Some(base_url), &page.route_path)
                .unwrap_or_else(|| page.route_path.clone())
        )));
        out.push_str("</loc></url>\n");
    }
    out.push_str("</urlset>\n");
    out
}

fn build_search_index_json(pages: &[DocPage]) -> String {
    let entries = pages
        .iter()
        .map(|page| {
            let section = if page.slug_segments.len() > 1 {
                page.slug_segments[..page.slug_segments.len() - 1]
                    .iter()
                    .map(|segment| titleize_segment(segment))
                    .collect::<Vec<_>>()
                    .join(" / ")
            } else {
                String::new()
            };
            json!({
                "title": page.title,
                "href": page.route_path,
                "description": page.description,
                "keywords": page.keywords,
                "headings": page.headings.iter().map(|heading| heading.text.clone()).collect::<Vec<_>>(),
                "excerpt": excerpt_for_search(&page.markdown),
                "section": section,
                "source_rel_path": page.source_rel_path,
            })
        })
        .collect::<Vec<_>>();
    serde_json::to_string_pretty(&entries).unwrap_or_else(|_| "[]".to_string())
}

fn xml_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn html_escape(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn replace_tag_content(input: &str, tag: &str, value: &str) -> String {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    if let Some(start) = input.find(&open)
        && let Some(end_rel) = input[start + open.len()..].find(&close)
    {
        let end = start + open.len() + end_rel;
        let mut out = String::with_capacity(input.len() + value.len());
        out.push_str(&input[..start + open.len()]);
        out.push_str(value);
        out.push_str(&input[end..]);
        return out;
    }
    insert_before_head_end(input, &format!("{open}{value}{close}"))
}

fn replace_or_insert_meta(input: &str, attr_name: &str, attr_value: &str, content: &str) -> String {
    let needle = format!("<meta {attr_name}=\"{attr_value}\"");
    if let Some(start) = input.find(&needle)
        && let Some(end_rel) = input[start..].find('>')
    {
        let end = start + end_rel + 1;
        let replacement = format!("<meta {attr_name}=\"{attr_value}\" content=\"{content}\">");
        let mut out = String::with_capacity(input.len() + replacement.len());
        out.push_str(&input[..start]);
        out.push_str(&replacement);
        out.push_str(&input[end..]);
        return out;
    }
    insert_before_head_end(
        input,
        &format!("<meta {attr_name}=\"{attr_value}\" content=\"{content}\">"),
    )
}

fn upsert_link_rel(input: &str, rel: &str, href: &str) -> String {
    let needle = format!("<link rel=\"{rel}\"");
    if let Some(start) = input.find(&needle)
        && let Some(end_rel) = input[start..].find('>')
    {
        let end = start + end_rel + 1;
        let replacement = format!("<link rel=\"{rel}\" href=\"{}\">", html_escape(href));
        let mut out = String::with_capacity(input.len() + replacement.len());
        out.push_str(&input[..start]);
        out.push_str(&replacement);
        out.push_str(&input[end..]);
        return out;
    }
    insert_before_head_end(
        input,
        &format!("<link rel=\"{rel}\" href=\"{}\">", html_escape(href)),
    )
}

fn insert_before_head_end(input: &str, snippet: &str) -> String {
    if let Some(pos) = input.find("</head>") {
        let mut out = String::with_capacity(input.len() + snippet.len());
        out.push_str(&input[..pos]);
        out.push_str(snippet);
        out.push_str(&input[pos..]);
        out
    } else {
        format!("{snippet}{input}")
    }
}

fn rel_dir_string(root_abs: &Path, dir_abs: &Path) -> Result<String, PipelineError> {
    if root_abs == dir_abs {
        return Ok(String::new());
    }
    dir_abs
        .strip_prefix(root_abs)
        .map(|rel| rel.to_string_lossy().replace('\\', "/"))
        .map_err(|_| {
            PipelineError::new(
                "WEB_DOCS_REL_DIR",
                format!(
                    "failed resolving '{}' relative to '{}'",
                    dir_abs.display(),
                    root_abs.display()
                ),
            )
        })
}

fn output_rel_path_for(slug_segments: &[String]) -> String {
    if slug_segments.is_empty() {
        "index.html".to_string()
    } else {
        format!("{}/index.html", slug_segments.join("/"))
    }
}

fn slug_segments_for_markdown(rel_path: &str) -> Vec<String> {
    let trimmed = rel_path.trim_end_matches(".md");
    let mut parts = trimmed
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    if parts.last().map(|part| part == "index").unwrap_or(false) {
        parts.pop();
    }
    parts
}

fn split_frontmatter(raw: &str) -> (PageFrontmatter, String) {
    let Some(rest) = raw.strip_prefix("---\n") else {
        return (PageFrontmatter::default(), raw.to_string());
    };
    let Some(end) = rest.find("\n---\n") else {
        return (PageFrontmatter::default(), raw.to_string());
    };
    let frontmatter_raw = &rest[..end];
    let body = rest[end + "\n---\n".len()..].to_string();
    (parse_page_frontmatter(frontmatter_raw), body)
}

fn parse_page_frontmatter(raw: &str) -> PageFrontmatter {
    let map = parse_simple_yaml_map(raw);
    PageFrontmatter {
        title: map_string(&map, "title"),
        description: map_string(&map, "description"),
        order: map_i64(&map, "order"),
        keywords: map_string_list(&map, "keywords"),
        canonical: map_string(&map, "canonical"),
        noindex: map_bool(&map, "noindex"),
    }
}

fn parse_folder_meta(raw: &str) -> FolderMeta {
    let map = parse_simple_yaml_map(raw);
    FolderMeta {
        title: map_string(&map, "title"),
        order: map_i64(&map, "order"),
        collapsed: map_optional_bool(&map, "collapsed"),
        nav: map_string_list(&map, "nav"),
    }
}

fn parse_simple_yaml_map(raw: &str) -> Map<String, Value> {
    let mut out = Map::new();
    let mut current_list_key: Option<String> = None;
    let mut current_list = Vec::new();

    let flush_list =
        |out: &mut Map<String, Value>, key: &mut Option<String>, list: &mut Vec<Value>| {
            if let Some(current_key) = key.take() {
                out.insert(current_key, Value::Array(std::mem::take(list)));
            }
        };

    for line in raw.lines() {
        let trimmed = line.trim_end();
        if trimmed.trim().is_empty() || trimmed.trim_start().starts_with('#') {
            continue;
        }
        let start_trimmed = trimmed.trim_start();
        if let Some(item) = start_trimmed.strip_prefix("- ") {
            if current_list_key.is_some() {
                current_list.push(parse_scalar(item.trim()));
            }
            continue;
        }
        flush_list(&mut out, &mut current_list_key, &mut current_list);
        let Some((key, value)) = trimmed.split_once(':') else {
            continue;
        };
        let key = key.trim().to_string();
        let value = value.trim();
        if value.is_empty() {
            current_list_key = Some(key);
            current_list = Vec::new();
        } else {
            out.insert(key, parse_scalar(value));
        }
    }
    flush_list(&mut out, &mut current_list_key, &mut current_list);
    out
}

fn parse_scalar(raw: &str) -> Value {
    let trimmed = raw.trim().trim_matches('"').trim_matches('\'');
    if trimmed.eq_ignore_ascii_case("true") {
        Value::Bool(true)
    } else if trimmed.eq_ignore_ascii_case("false") {
        Value::Bool(false)
    } else if let Ok(number) = trimmed.parse::<i64>() {
        Value::Number(number.into())
    } else {
        Value::String(trimmed.to_string())
    }
}

fn map_string(map: &Map<String, Value>, key: &str) -> Option<String> {
    map.get(key)
        .and_then(Value::as_str)
        .map(ToString::to_string)
}

fn map_i64(map: &Map<String, Value>, key: &str) -> Option<i64> {
    map.get(key).and_then(Value::as_i64)
}

fn map_bool(map: &Map<String, Value>, key: &str) -> bool {
    map.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn map_optional_bool(map: &Map<String, Value>, key: &str) -> Option<bool> {
    map.get(key).and_then(Value::as_bool)
}

fn map_string_list(map: &Map<String, Value>, key: &str) -> Vec<String> {
    match map.get(key) {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(Value::as_str)
            .map(ToString::to_string)
            .collect(),
        Some(Value::String(value)) => vec![value.clone()],
        _ => Vec::new(),
    }
}

fn extract_headings(markdown: &str) -> Vec<DocHeading> {
    let mut out = Vec::new();
    let mut seen = HashMap::new();
    for line in markdown.lines() {
        let trimmed = line.trim_start();
        let level = trimmed.chars().take_while(|ch| *ch == '#').count();
        if !(1..=6).contains(&level) {
            continue;
        }
        let Some(rest) = trimmed.get(level..) else {
            continue;
        };
        let text = rest.trim();
        if text.is_empty() {
            continue;
        }
        let base = slugify(text);
        let id = unique_slug(base, &mut seen);
        out.push(DocHeading {
            level: level as u8,
            id,
            text: strip_inline_markdown(text),
        });
    }
    out
}

fn strip_inline_markdown(input: &str) -> String {
    input
        .chars()
        .filter(|ch| !matches!(ch, '*' | '_' | '`' | '[' | ']' | '(' | ')' | '#' | '!'))
        .collect::<String>()
        .trim()
        .to_string()
}

fn first_paragraph(markdown: &str) -> String {
    let mut lines = Vec::new();
    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if !lines.is_empty() {
                break;
            }
            continue;
        }
        if trimmed.starts_with('#') {
            continue;
        }
        lines.push(trimmed);
    }
    lines.join(" ")
}

fn excerpt_for_search(markdown: &str) -> String {
    plain_text_from_markdown(markdown)
        .split_whitespace()
        .take(48)
        .collect::<Vec<_>>()
        .join(" ")
}

fn plain_text_from_markdown(markdown: &str) -> String {
    markdown
        .lines()
        .map(|line| {
            line.trim()
                .trim_start_matches('#')
                .trim_start_matches('-')
                .trim_start_matches('*')
                .trim_start_matches('>')
                .trim()
                .replace('`', "")
                .replace('[', "")
                .replace(']', "")
                .replace('(', "")
                .replace(')', "")
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" ")
}

fn titleize_segment(raw: &str) -> String {
    raw.split(['-', '_', ' '])
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut chars = segment.chars();
            match chars.next() {
                Some(first) => format!(
                    "{}{}",
                    first.to_uppercase().collect::<String>(),
                    chars.as_str().to_lowercase()
                ),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn slugify(raw: &str) -> String {
    let mut out = String::new();
    let mut last_dash = true;
    for ch in raw.chars().flat_map(|ch| ch.to_lowercase()) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "section".to_string()
    } else {
        trimmed
    }
}

fn unique_slug(base: String, seen: &mut HashMap<String, usize>) -> String {
    match seen.get_mut(&base) {
        Some(count) => {
            *count += 1;
            format!("{base}-{}", *count)
        }
        None => {
            seen.insert(base.clone(), 0);
            base
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::json;

    use super::{NODE_KIND, extract_headings, parse_folder_meta, split_frontmatter};
    use crate::language::DenoSandboxEngine;
    use crate::pipeline::engines::basic::new_template_cache;
    use crate::pipeline::{
        BasicPipelineEngine, PipelineContext, PipelineEngine, PipelineGraph, PipelineNode,
    };
    use crate::platform::adapters::file::build_file_adapter;
    use crate::platform::model::FileAdapterKind;
    use crate::rwe::resolve_engine_or_default;

    #[test]
    fn parses_frontmatter_and_markdown_body() {
        let raw = "---\ntitle: Query Basics\ndescription: Learn query flow\norder: 20\nkeywords:\n  - sekejap\n  - query\nnoindex: true\n---\n# Heading\n\nHello";
        let (fm, body) = split_frontmatter(raw);
        assert_eq!(fm.title.as_deref(), Some("Query Basics"));
        assert_eq!(fm.description.as_deref(), Some("Learn query flow"));
        assert_eq!(fm.order, Some(20));
        assert_eq!(
            fm.keywords,
            vec!["sekejap".to_string(), "query".to_string()]
        );
        assert!(fm.noindex);
        assert!(body.contains("# Heading"));
    }

    #[test]
    fn parses_folder_meta_lists() {
        let meta = parse_folder_meta("title: Basic\ncollapsed: true\nnav:\n  - index\n  - query\n");
        assert_eq!(meta.title.as_deref(), Some("Basic"));
        assert_eq!(meta.collapsed, Some(true));
        assert_eq!(meta.nav, vec!["index".to_string(), "query".to_string()]);
    }

    #[test]
    fn extracts_heading_ids() {
        let headings = extract_headings("# Intro\n## Query\n## Query\n");
        assert_eq!(headings.len(), 3);
        assert_eq!(headings[1].id, "query");
        assert_eq!(headings[2].id, "query-1");
    }

    #[tokio::test]
    async fn engine_generates_static_docs_site_and_scaffolds_template() {
        let root = std::env::temp_dir().join(format!(
            "zebflow-docsgen-test-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("unix time")
                .as_nanos()
        ));
        let file = build_file_adapter(FileAdapterKind::Filesystem, root.clone());
        let layout = file
            .ensure_project_layout("superadmin", "docs-project")
            .expect("layout");

        let docs_root = layout.repo_docs_dir.join("sekejap-docs");
        std::fs::create_dir_all(docs_root.join("basic")).expect("docs dir");
        std::fs::write(
            docs_root.join("_meta.yaml"),
            "title: Sekejap Docs\ncollapsed: false\n",
        )
        .expect("meta");
        std::fs::write(
            docs_root.join("index.md"),
            "---\ntitle: Home\ndescription: Home page\n---\n# Home\n\nWelcome to Sekejap.",
        )
        .expect("index");
        std::fs::write(
            docs_root.join("basic").join("query.md"),
            "---\ntitle: Query Basics\ndescription: Query guide\nkeywords:\n  - query\n  - basics\n---\n# Query Basics\n\n## Select\n\nUse select.\n",
        )
        .expect("query");

        let graph = PipelineGraph {
            kind: "zebflow.pipeline".to_string(),
            version: "0.1".to_string(),
            id: "generate-docs".to_string(),
            description: None,
            metadata: None,
            entry_nodes: vec!["gen".to_string()],
            nodes: vec![PipelineNode {
                id: "gen".to_string(),
                kind: NODE_KIND.to_string(),
                input_pins: vec!["in".to_string()],
                output_pins: vec!["out".to_string()],
                config: json!({
                    "docs_root": "sekejap-docs",
                    "output_dir": "docs",
                    "template_folder": "pages/docs",
                    "deploy_base_url": "https://db.sekejap.life",
                    "site_title": "Sekejap Docs"
                }),
            }],
            edges: vec![],
        };

        let ctx = PipelineContext {
            owner: "superadmin".to_string(),
            project: "docs-project".to_string(),
            pipeline: graph.id.clone(),
            request_id: "req-docs-1".to_string(),
            route: String::new(),
            input: json!({}),
            trigger: None,
            placeholder: None,
        };

        let engine = BasicPipelineEngine::new(
            Arc::new(DenoSandboxEngine::default()),
            resolve_engine_or_default(None),
            None,
        )
        .with_template_root(Some(layout.repo_pipelines_dir.clone()))
        .with_template_cache(new_template_cache())
        .with_data_root(root.clone());

        let result = engine
            .execute_async(&graph, &ctx)
            .await
            .expect("docs generate");
        assert_eq!(result.value["docs_generated"]["status"], "ok");
        assert_eq!(result.value["docs_generated"]["page_count"], 2);
        assert_eq!(result.value["docs_generated"]["site_root"], "docs");
        assert_eq!(
            result.value["docs_generated"]["manifest_path"],
            "docs/.zebflow-static-site.json"
        );
        assert_eq!(
            result.value["docs_generated"]["deploy_base_url"],
            "https://db.sekejap.life"
        );
        assert_eq!(result.value["docs_generated"]["deploy_base_path"], "/docs");
        assert_eq!(
            result.value["docs_generated"]["search_index_path"],
            "docs/search-index.json"
        );

        let template_path = layout
            .repo_pipelines_dir
            .join("pages")
            .join("docs")
            .join("docs.template.tsx");
        assert!(template_path.is_file());

        let home_path = layout.files_dir.join("docs").join("index.html");
        let query_path = layout
            .files_dir
            .join("docs")
            .join("basic")
            .join("query")
            .join("index.html");
        let sitemap_path = layout.files_dir.join("docs").join("sitemap.xml");
        let search_index_path = layout.files_dir.join("docs").join("search-index.json");
        assert!(home_path.is_file());
        assert!(query_path.is_file());
        assert!(sitemap_path.is_file());
        assert!(search_index_path.is_file());
        let manifest_path = layout
            .files_dir
            .join("docs")
            .join(".zebflow-static-site.json");
        assert!(manifest_path.is_file());

        let home_html = std::fs::read_to_string(home_path).expect("home html");
        let query_html = std::fs::read_to_string(query_path).expect("query html");
        let sitemap = std::fs::read_to_string(sitemap_path).expect("sitemap");
        let search_index = std::fs::read_to_string(search_index_path).expect("search index");
        let manifest = std::fs::read_to_string(manifest_path).expect("manifest");

        assert!(home_html.contains("Welcome to Sekejap."));
        assert!(query_html.contains("Query Basics"));
        assert!(query_html.contains("id=\"select\""));
        assert!(home_html.contains("Search the docs"));
        assert!(home_html.contains("_assets/libraries/zeb/preact/0.1/runtime/preact.bundle.mjs"));
        assert!(
            query_html.contains("../../_assets/libraries/zeb/preact/0.1/runtime/preact.bundle.mjs")
        );
        assert!(
            layout
                .files_dir
                .join("docs")
                .join("_assets")
                .join("libraries")
                .join("zeb")
                .join("preact")
                .join("0.1")
                .join("runtime")
                .join("preact.bundle.mjs")
                .is_file()
        );
        assert!(sitemap.contains("https://db.sekejap.life/docs/"));
        assert!(sitemap.contains("https://db.sekejap.life/docs/basic/query/"));
        assert!(search_index.contains("\"href\": \"/docs/basic/query/\""));
        assert!(search_index.contains("\"Query Basics\""));
        assert!(search_index.contains("\"Select\""));
        assert!(manifest.contains("\"site_root\": \"docs\""));
        assert!(manifest.contains("\"deploy_base_path\": \"/docs\""));
        assert!(manifest.contains("\"template\": \"pages/docs/docs.template.tsx\""));

        if std::env::var("ZEBFLOW_KEEP_DOCSGEN_TEST").ok().as_deref() != Some("1") {
            let _ = std::fs::remove_dir_all(root);
        }
    }
}
