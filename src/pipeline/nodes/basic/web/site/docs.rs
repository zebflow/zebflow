//! `web.site.generate --from` — a docs site from a Markdown folder.
//!
//! - the Markdown lives under `repo/docs/<from>/`, each folder's `_meta.yaml`
//!   giving its title, order and nav;
//! - the page template is `--template` under the source root, scaffolded
//!   when missing (`docs_scaffold.rs`);
//! - every page, the search index and (with a serve origin) the sitemap are
//!   written under the site's folder in the store.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::docs_text::{
    excerpt_for_search, extract_headings, first_paragraph, html_escape, parse_folder_meta, replace_or_insert_meta,
    replace_tag_content, split_frontmatter, titleize_segment, upsert_link_rel, xml_escape,
};
pub use super::docs_text::{DocHeading, FolderMeta, PageFrontmatter};
use super::{Config, DEFAULT_DOCS_TEMPLATE, DOCS_META_FILE, Mode};
use crate::pipeline::PipelineError;
use crate::pipeline::nodes::basic::web::static_site;
use crate::rwe::TemplateSource;

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
    /// The Markdown folder under `docs/`, normalised.
    pub from_rel: String,
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

/// Loads the documentation site described by `config`.
///
/// `docs_root` is passed in rather than derived from `template_root`, which
/// used to be reached by a single `parent()` hop: that is only the repository
/// root when the source directory is exactly one segment deep, so a project
/// declaring `source: src/app` resolved its docs against `repo/src`.
pub fn load_site(
    config: &Config,
    template_root: &Path,
    docs_root: &Path,
) -> Result<DocsSite, PipelineError> {
    let docs_root_rel = normalize_rel_dir_path(config.from.as_deref().unwrap_or_default(), "--from")?;
    let site_root_rel = config.folder_rel(Mode::Docs)?;
    let template_rel_path = super::page::normalize_template_rel_path(
        config.template.as_deref().map(str::trim).filter(|s| !s.is_empty()).unwrap_or(DEFAULT_DOCS_TEMPLATE),
    )?;
    let meta_file = DOCS_META_FILE;

    let docs_root_abs = docs_root.join(&docs_root_rel);
    if !docs_root_abs.is_dir() {
        return Err(PipelineError::new(
            "FW_NODE_WEB_SITE_GENERATE_ROOT_MISSING",
            format!("docs root '{}' not found", docs_root_abs.display()),
        ));
    }

    let template_source = ensure_template_scaffold(template_root, &template_rel_path, config.name.as_deref())?;
    // A site runs at the root of its serve address (`node-conventions.md` §7).
    let deploy_base_path = "/".to_string();

    let mut folder_meta = HashMap::new();
    let mut pages = Vec::new();
    collect_docs(
        &docs_root_abs,
        &docs_root_abs,
        meta_file,
        &deploy_base_path,
        &mut folder_meta,
        &mut pages,
    )?;
    if pages.is_empty() {
        return Err(PipelineError::new(
            "FW_NODE_WEB_SITE_GENERATE_EMPTY",
            format!("docs root '{}' contains no markdown pages", docs_root_rel),
        ));
    }

    pages.sort_by(|a, b| a.source_rel_path.cmp(&b.source_rel_path));

    let site_title = config
        .name
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
    // The sitemap needs the site's origin, which the engine supplies from the
    // folder's serve rule (`DocsSite::set_origin`); without one there is none.
    let sitemap_xml = String::new();
    let search_index_route =
        static_site::route_path_for_output_path(&deploy_base_path, "search-index.json")?;
    let search_index_json = build_search_index_json(&ordered_pages);

    Ok(DocsSite {
        site_title,
        from_rel: docs_root_rel,
        deploy_base_url: None,
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
            "FW_NODE_WEB_SITE_GENERATE_PAGE_INDEX",
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
    static_site::page_rel_path_from_site_root(site_root_rel, &page.output_rel_path)
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
                "FW_NODE_WEB_SITE_GENERATE_PATH",
                format!("{field} must stay inside the project directory"),
            ));
        }
        parts.push(part.to_string());
    }
    if parts.is_empty() {
        return Err(PipelineError::new(
            "FW_NODE_WEB_SITE_GENERATE_PATH",
            format!("{field} must not be empty"),
        ));
    }
    Ok(parts.join("/"))
}

/// The docs page template, written from the scaffold when the file is missing.
fn ensure_template_scaffold(
    template_root: &Path,
    template_rel_path: &str,
    name: Option<&str>,
) -> Result<TemplateSource, PipelineError> {
    let template_abs_path = template_root.join(template_rel_path);
    if !template_abs_path.exists() {
        if let Some(parent) = template_abs_path.parent() {
            std::fs::create_dir_all(parent).map_err(|err| {
                PipelineError::new(
                    "FW_NODE_WEB_SITE_GENERATE_TEMPLATE_DIR",
                    format!("failed creating '{}': {err}", parent.display()),
                )
            })?;
        }
        std::fs::write(&template_abs_path, super::docs_scaffold::default_template_source(name)).map_err(|err| {
            PipelineError::new(
                "FW_NODE_WEB_SITE_GENERATE_TEMPLATE_WRITE",
                format!(
                    "failed writing docs scaffold '{}': {err}",
                    template_abs_path.display()
                ),
            )
        })?;
    }

    let markup = std::fs::read_to_string(&template_abs_path).map_err(|err| {
        PipelineError::new(
            "FW_NODE_WEB_SITE_GENERATE_TEMPLATE_READ",
            format!("failed reading '{}': {err}", template_abs_path.display()),
        )
    })?;

    Ok(TemplateSource {
        id: template_rel_path.to_string(),
        source_path: Some(template_abs_path),
        markup,
    })
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
                "FW_NODE_WEB_SITE_GENERATE_META_READ",
                format!("failed reading '{}': {err}", meta_path.display()),
            )
        })?;
        folder_meta.insert(dir_rel.clone(), parse_folder_meta(&content));
    }

    let mut entries = std::fs::read_dir(dir_abs)
        .map_err(|err| {
            PipelineError::new(
                "FW_NODE_WEB_SITE_GENERATE_READ_DIR",
                format!("failed reading '{}': {err}", dir_abs.display()),
            )
        })?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| {
            PipelineError::new(
                "FW_NODE_WEB_SITE_GENERATE_READ_DIR",
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
                "FW_NODE_WEB_SITE_GENERATE_REL_PATH",
                format!("failed resolving relative path for '{}'", path.display()),
            )
        })?;
        let rel_str = rel.to_string_lossy().replace('\\', "/");
        let raw = std::fs::read_to_string(&path).map_err(|err| {
            PipelineError::new(
                "FW_NODE_WEB_SITE_GENERATE_READ_PAGE",
                format!("failed reading '{}': {err}", path.display()),
            )
        })?;
        let (frontmatter, markdown) = split_frontmatter(&raw);
        let slug_segments = slug_segments_for_markdown(&rel_str);
        let output_rel_path = output_rel_path_for(&slug_segments);
        let route_path =
            static_site::route_path_for_output_path(deploy_base_path, &output_rel_path)?;
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
        let route = static_site::route_path_for_output_path(
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
    static_site::absolute_deploy_url(base_url, route_path)
}

impl DocsSite {
    /// The origin the site is served on — the `serve` origin of its folder —
    /// for canonical links and the sitemap; `None` writes neither.
    pub fn set_origin(&mut self, origin: Option<String>) {
        self.sitemap_xml = build_sitemap_xml(origin.as_deref(), &self.pages);
        self.deploy_base_url = origin;
    }
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
            static_site::absolute_deploy_url(Some(base_url), &page.route_path)
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

fn rel_dir_string(root_abs: &Path, dir_abs: &Path) -> Result<String, PipelineError> {
    if root_abs == dir_abs {
        return Ok(String::new());
    }
    dir_abs
        .strip_prefix(root_abs)
        .map(|rel| rel.to_string_lossy().replace('\\', "/"))
        .map_err(|_| {
            PipelineError::new(
                "FW_NODE_WEB_SITE_GENERATE_REL_DIR",
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::json;

    use crate::pipeline::nodes::basic::web::site::NODE_KIND;
    use crate::language::DenoSandboxEngine;
    use crate::pipeline::engines::basic::new_template_cache;
    use crate::pipeline::{
        BasicPipelineEngine, PipelineContext, PipelineEngine, PipelineGraph, PipelineNode,
    };
    use crate::platform::adapters::file::build_file_adapter;
    use crate::platform::model::FileAdapterKind;
    use crate::platform::services::project_config::ProjectConfigurationService;
    use crate::rwe::resolve_engine_or_default;

    #[tokio::test]
    async fn engine_generates_static_docs_site_and_scaffolds_template() {
        let root = std::env::temp_dir().join(format!(
            "zebflow-docsgen-test-{}",
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
            .ensure_project_layout("superadmin", "docs-project")
            .expect("layout");

        let docs_root = layout.repo_docs_dir().join("sekejap-docs");
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
            id: "generate-docs".to_string(),
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
                    "from": "sekejap-docs",
                    "folder": "docs",
                    "template": "pages/docs/docs.template.tsx",
                    "name": "Sekejap Docs"
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
        .with_project_layout(Some(layout.clone()))
        .with_template_cache(new_template_cache())
        .with_data_root(root.clone());

        let result = engine
            .execute_async(&graph, &ctx)
            .await
            .expect("docs generate");
        assert_eq!(result.value["site"]["status"], "ok");
        assert_eq!(result.value["site"]["page_count"], 2);
        assert_eq!(result.value["site"]["folder"], "docs");
        assert_eq!(
            result.value["site"]["manifest_path"],
            "docs/.zebflow-static-site.json"
        );
        // No serve rule names the folder, so the site has no origin and no
        // sitemap; it never takes its address from a flag.
        assert!(result.value["site"]["origin"].is_null());
        assert_eq!(
            result.value["site"]["search_index_path"],
            "docs/search-index.json"
        );

        let template_path = layout
            .repo_source_dir()
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
        assert!(!sitemap_path.exists());
        assert!(search_index_path.is_file());
        let manifest_path = layout
            .files_dir
            .join("docs")
            .join(".zebflow-static-site.json");
        assert!(manifest_path.is_file());

        let home_html = std::fs::read_to_string(home_path).expect("home html");
        let query_html = std::fs::read_to_string(query_path).expect("query html");
        let search_index = std::fs::read_to_string(search_index_path).expect("search index");
        let manifest = std::fs::read_to_string(manifest_path).expect("manifest");

        assert!(home_html.contains("Welcome to Sekejap."));
        assert!(query_html.contains("Query Basics"));
        assert!(query_html.contains("id=\"select\""));
        assert!(home_html.contains("Search the docs"));
        assert!(home_html.contains("_assets/libraries/zeb/react/0.1/runtime/zeb_react.mjs"));
        assert!(query_html.contains("../../_assets/libraries/zeb/react/0.1/runtime/zeb_react.mjs"));
        assert!(
            layout
                .files_dir
                .join("docs")
                .join("_assets")
                .join("libraries")
                .join("zeb")
                .join("react")
                .join("0.1")
                .join("runtime")
                .join("zeb_react.mjs")
                .is_file()
        );
        assert!(search_index.contains("\"href\": \"/basic/query/\""));
        assert!(search_index.contains("\"Query Basics\""));
        assert!(search_index.contains("\"Select\""));
        assert!(manifest.contains("\"site_root\": \"docs\""));
        assert!(manifest.contains("\"deploy_base_path\": \"/\""));
        assert!(manifest.contains("\"template\": \"pages/docs/docs.template.tsx\""));

        if std::env::var("ZEBFLOW_KEEP_DOCSGEN_TEST").ok().as_deref() != Some("1") {
            let _ = std::fs::remove_dir_all(root);
        }
    }
}
