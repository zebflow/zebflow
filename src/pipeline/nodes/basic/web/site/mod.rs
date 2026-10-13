//! `web.site.generate` — a static site in the project store.
//!
//! One kind, two modes, chosen by the role given:
//!
//! - `--path` renders one TSX page (`--template`) into the site (`page.rs`);
//! - `--from` builds a whole docs site from a Markdown folder under `docs/`
//!   (`docs.rs`), its page template scaffolded when missing.
//!
//! Both write under `--folder` with one manifest and one `_assets/`
//! (`../static_site.rs`), and answer one key, `site`.

pub mod docs;
pub mod docs_blocks;
pub mod docs_links;
pub mod docs_math;
mod docs_scaffold;
pub mod docs_search;
mod docs_text;
#[cfg(test)]
mod layout_tests;
pub mod page;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::pipeline::PipelineError;
use crate::pipeline::model::{
    DslFlag, DslFlagKind, LayoutItem, NodeCapability, NodeDefinition, NodeExample, NodeFieldDataSource, NodeFieldDef,
    NodeFieldType,
};
use crate::pipeline::nodes::shared::project_store::{OnConflict, on_conflict_flag, store_fields, store_flag};

pub const NODE_KIND: &str = "web.site.generate";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

/// The site's folder in the store when `--folder` is not given.
pub const DEFAULT_PAGE_FOLDER: &str = "site";
pub const DEFAULT_DOCS_FOLDER: &str = "docs";
/// The docs page template, under the source root, when `--template` is not given.
pub const DEFAULT_DOCS_TEMPLATE: &str = "docs.template.tsx";
/// The folder metadata file a docs folder may hold.
pub const DOCS_META_FILE: &str = "_meta.yaml";
/// What a docs-built page records as its generator in the site manifest: the
/// kind that built it, as the template mode records. A kind is written as
/// itself; the retired `n.web.docs.generate` label is gone.
pub const DOCS_MANIFEST_GENERATOR: &str = NODE_KIND;

pub const CODE_CONFIG: &str = "FW_NODE_WEB_SITE_GENERATE_CONFIG";
pub const CODE_MODE: &str = "FW_NODE_WEB_SITE_GENERATE_MODE";
pub const CODE_CONFLICT: &str = "FW_NODE_WEB_SITE_GENERATE_CONFLICT";
/// A docs build refused because the Markdown links somewhere it does not write.
pub const CODE_LINKS: &str = "FW_NODE_WEB_SITE_GENERATE_LINKS";
/// A `--version` that is not a version label, or one that collides with a page.
pub const CODE_VERSION: &str = "FW_NODE_WEB_SITE_GENERATE_VERSION";

/// Typed configuration for `web.site.generate`.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// The page template under the source root, ending `.tsx`. Required for
    /// a page; for a docs site, the template scaffolded when missing
    /// (default [`DEFAULT_DOCS_TEMPLATE`]).
    #[serde(default)]
    pub template: Option<String>,
    /// Template markup a higher layer already loaded (page mode); never
    /// written by an author.
    #[serde(default)]
    pub markup: Option<String>,
    /// Page mode: the page's path inside the site folder, e.g.
    /// `posts/{{ input.slug }}.html`.
    #[serde(default)]
    pub path: Option<String>,
    /// Docs mode: the Markdown folder under `docs/`.
    #[serde(default)]
    pub from: Option<String>,
    /// The site's folder in the store (default `site` for a page, `docs` for
    /// a docs site); every page of one site shares it.
    #[serde(default)]
    pub folder: Option<String>,
    /// The store to write to; saved explicitly at registration.
    #[serde(default)]
    pub store: Option<String>,
    /// Page mode: `ctx.route` while rendering (default: the page's address).
    #[serde(default)]
    pub route: Option<String>,
    /// Docs mode: the site's name (default: the docs folder's `_meta.yaml`
    /// title, else its name).
    #[serde(default)]
    pub name: Option<String>,
    /// `overwrite` (default), `skip` or `error` when a page exists with other content.
    #[serde(default)]
    pub on_conflict: Option<String>,
    /// Docs mode: this build is one version of the site (`0.11`), written
    /// under `<folder>/<version>/` and listed in `<folder>/versions.json`.
    #[serde(default)]
    pub version: Option<String>,
    /// Docs mode, with `--version`: the latest version, written again at
    /// `<folder>/` — the unversioned address, the only one indexed.
    #[serde(default)]
    pub latest: bool,
}

/// Which site a configuration builds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Page,
    Docs,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Page => "page",
            Self::Docs => "docs",
        }
    }
}

fn given(value: &Option<String>) -> Option<&str> {
    value.as_deref().map(str::trim).filter(|s| !s.is_empty())
}

impl Config {
    /// The mode the roles name: `--path` a page, `--from` a docs site; both
    /// or neither is refused. Also refuses a flag the mode does not read.
    pub fn mode(&self) -> Result<Mode, PipelineError> {
        let mode = match (given(&self.path), given(&self.from)) {
            (Some(_), None) => Mode::Page,
            (None, Some(_)) => Mode::Docs,
            (Some(_), Some(_)) => {
                return Err(PipelineError::new(
                    CODE_MODE,
                    "web.site.generate takes --path (one page) or --from (a docs site), not both",
                ));
            }
            (None, None) => {
                return Err(PipelineError::new(
                    CODE_MODE,
                    "web.site.generate needs --path PAGE (one page) or --from DOCS_FOLDER (a docs site)",
                ));
            }
        };
        match mode {
            Mode::Page if given(&self.template).is_none() => Err(PipelineError::new(
                CODE_MODE,
                "a page (--path) needs --template PAGE.tsx",
            )),
            Mode::Page if given(&self.name).is_some() => Err(PipelineError::new(
                CODE_MODE,
                "--name names a docs site (--from); a page takes its title from its template",
            )),
            Mode::Docs if given(&self.route).is_some() => Err(PipelineError::new(
                CODE_MODE,
                "--route sets one page's route (--path); a docs site routes every page from its file",
            )),
            Mode::Page if given(&self.version).is_some() || self.latest => Err(PipelineError::new(
                CODE_MODE,
                "--version and --latest version a docs site (--from); a page has no versions",
            )),
            Mode::Docs if self.latest && given(&self.version).is_none() => Err(PipelineError::new(
                CODE_VERSION,
                "--latest marks a version as the latest: give it --version, e.g. --version 0.11 --latest",
            )),
            Mode::Docs => {
                self.version_label()?;
                Ok(mode)
            }
            _ => Ok(mode),
        }
    }

    /// The `--version` label, checked: a letter or digit, then letters,
    /// digits, `.`, `-` or `_`, at most 32 — one path segment of the site.
    pub fn version_label(&self) -> Result<Option<&str>, PipelineError> {
        let Some(label) = given(&self.version) else { return Ok(None) };
        let ok = label.len() <= 32
            && label.chars().next().is_some_and(|c| c.is_ascii_alphanumeric())
            && label.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'));
        if !ok {
            return Err(PipelineError::new(
                CODE_VERSION,
                format!("--version '{label}' is not a version label: a letter or digit, then letters, digits, '.', '-' or '_' (e.g. 0.11, 2.x, next)"),
            ));
        }
        Ok(Some(label))
    }

    /// `--on-conflict`, overwrite by default (`node-conventions.md` §8).
    pub fn on_conflict(&self) -> Result<OnConflict, PipelineError> {
        OnConflict::parse(self.on_conflict.as_deref(), OnConflict::Overwrite, CODE_CONFLICT)
    }

    /// Everything that can be refused before rendering.
    pub fn check(&self) -> Result<Mode, PipelineError> {
        self.on_conflict()?;
        self.mode()
    }

    /// The site's folder in the store, normalised.
    pub fn folder_rel(&self, mode: Mode) -> Result<String, PipelineError> {
        let default = match mode {
            Mode::Page => DEFAULT_PAGE_FOLDER,
            Mode::Docs => DEFAULT_DOCS_FOLDER,
        };
        super::static_site::normalize_site_root_rel_path(given(&self.folder).unwrap_or(default))
    }
}

fn flag(name: &str, key: &str, value: &str, description: &str) -> DslFlag {
    DslFlag {
        flag: name.to_string(),
        config_key: key.to_string(),
        description: description.to_string(),
        kind: DslFlagKind::Scalar,
        value: value.to_string(),
        ..Default::default()
    }
}

fn field(name: &str, label: &str, placeholder: &str, help: &str) -> NodeFieldDef {
    NodeFieldDef {
        name: name.to_string(),
        label: label.to_string(),
        field_type: NodeFieldType::Text,
        placeholder: Some(placeholder.to_string()).filter(|p| !p.is_empty()),
        help: Some(help.to_string()),
        ..Default::default()
    }
}

/// Kind-level contract for the pipeline editor, DSL, and MCP-facing node help.
pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Filesystem, NodeCapability::Process],
        title: "Web Site Generate".to_string(),
        description: "Write a static site into the project store. One page: `--template pages/post.tsx --path posts/{{ input.slug }}.html` \
            renders the TSX page with the payload as its `input` (`--route` sets `ctx.route`; default: the page's address). \
            A docs site: `--from handbook` turns the Markdown under `docs/handbook/` into pages, a sidebar, a search index and, \
            when the folder is served, a sitemap; `--template` is the page template, created from a scaffold when missing \
            (default `docs.template.tsx`); `--name` is the site's name; each folder's `_meta.yaml` sets its title, `order` and `nav` \
            (a page's front matter its `order`), and that one order drives the sidebar, prev/next and the search index; \
            a page whose Markdown is gone is removed from the site on the next build. \
            A link to another page is written as that page's address. The site answers at its folder's place under the served folder \
            (served `site`, docs written to `site/docs`: pages at `/docs/…`), at `/` until the folder is served. \
            `--version 0.11` makes the build one version: written under `<folder>/0.11/` with its own assets and search index, never indexed, \
            listed in `<folder>/versions.json`; with `--latest` it is also written at `<folder>/` — the unversioned address, the only one indexed, \
            with the sitemap. A version once built is never rebuilt by a newer one: the template's switcher reads versions.json when the page loads. \
            Both write under `--folder` (default `site` for a page, `docs` for a docs site), sharing one manifest and one `_assets/`; \
            a page whose content changed is overwritten unless `--on-conflict` says skip or error, decided before anything is written. \
            The page renders from the payload, which it also embeds for hydration — so it publishes what the payload holds — \
            except the request: the trigger's `headers`, `cookies` and `auth` are never part of it; copy a value into a key of its own upstream to show it. \
            Adds `site` and keeps the payload. The files stay private until the owner serves the folder in Studio → Files."
            .to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "site": {
                    "type": "object",
                    "description": "Page mode: mode, status, path, route, store, file, origin, template, folder, manifest_path, asset_group, bytes. \
                        Docs mode: mode, status, name, template, from, folder, store, origin, manifest_path, asset_group, page_count, \
                        generated_files, skipped_files, sitemap_path, search_index_path, routes; with --version also version, latest, versions_path.",
                    "properties": {
                        "mode": { "type": "string", "enum": ["page", "docs"] },
                        "status": { "type": "string", "description": "page: written, unchanged or skipped; docs: ok" },
                        "path": { "type": "string" },
                        "route": { "type": "string" },
                        "routes": { "type": "array", "items": { "type": "string" } },
                        "store": { "type": ["string", "null"] },
                        "file": { "description": "A durable FileRef for the page" },
                        "origin": { "type": ["string", "null"], "description": "The site's serve origin from Studio → Files, if it has one" },
                        "template": { "type": "string" },
                        "from": { "type": "string" },
                        "name": { "type": "string" },
                        "folder": { "type": "string" },
                        "manifest_path": { "type": "string" },
                        "asset_group": { "type": "string" },
                        "bytes": { "type": "integer" },
                        "page_count": { "type": "integer" },
                        "generated_files": { "type": "integer" },
                        "skipped_files": { "type": "integer" },
                        "sitemap_path": { "type": ["string", "null"] },
                        "search_index_path": { "type": "string" },
                        "version": { "type": ["string", "null"], "description": "The --version label" },
                        "latest": { "type": "boolean" },
                        "versions_path": { "type": ["string", "null"], "description": "The versions list, <folder>/versions.json" }
                    }
                }
            }
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![
            flag("--template", "template", "text", "The page template under the source root, ending .tsx, e.g. pages/post.tsx. Required for a page; for a docs site, created from a scaffold when missing (default: docs.template.tsx)."),
            flag("--path", "path", "text", "One page: its path inside the site folder, e.g. posts/{{ input.slug }}.html."),
            flag("--from", "from", "text", "A docs site: the Markdown folder under docs/, e.g. handbook."),
            flag("--route", "route", "text", "One page: ctx.route while rendering (default: the page's address on the site)."),
            flag("--name", "name", "text", "A docs site: its name (default: the folder's _meta.yaml title, else the folder name)."),
            DslFlag { value: "text".to_string(), ..store_flag() },
            flag("--folder", "folder", "text", "The site's folder in the store (default: site for a page, docs for a docs site). Every page of one site shares it, with one manifest and one _assets/."),
            flag("--version", "version", "text", "A docs site: this build is one version, e.g. 0.11 — written under <folder>/0.11/ and listed in <folder>/versions.json."),
            DslFlag { kind: DslFlagKind::Bool, ..flag("--latest", "latest", "", "A docs site, with --version: the latest version, written again at <folder>/ — the unversioned address, the only one indexed.") },
            DslFlag {
                choices: vec!["overwrite".to_string(), "skip".to_string(), "error".to_string()],
                ..on_conflict_flag(OnConflict::Overwrite)
            },
        ],
        fields: vec![
            NodeFieldDef {
                field_type: NodeFieldType::Datalist,
                data_source: Some(NodeFieldDataSource::TemplatesPages),
                ..field("template", "Template", "pages/post.tsx", "The page template, ending .tsx. Required for a page; a docs site's is created when missing.")
            },
            field("path", "Path", "posts/{{ input.slug }}.html", "One page: its path inside the site folder. Set Path or From."),
            field("from", "From", "handbook", "A docs site: the Markdown folder under docs/. Set Path or From."),
            field("route", "Route", "/posts/{{ input.slug }}", "One page: ctx.route while rendering (default: the page's address)."),
            field("name", "Name", "Handbook", "A docs site: its name."),
            field("folder", "Folder", "site", "The site's folder in the store (default: site for a page, docs for a docs site)."),
            field("version", "Version", "0.11", "A docs site: this build is one version, under <folder>/<version>/."),
            NodeFieldDef { field_type: NodeFieldType::Checkbox, default_value: Some(json!(false)), ..field("latest", "Latest", "", "A docs site, with Version: also written at the unversioned address.") },
        ]
        .into_iter()
        .chain(store_fields(OnConflict::Overwrite))
        .collect(),
        layout: ["template", "path", "from", "route", "name", "folder", "version", "latest", "store", "on_conflict"]
            .iter()
            .map(|name| LayoutItem::Field(name.to_string()))
            .collect(),
        ai_tool: Default::default(),
        examples: vec![
            NodeExample::dsl("Render one post to a static page", r#"web.site.generate --template pages/post.tsx --path "posts/{{ input.query.rows[0].slug }}.html" --route "/posts/{{ input.query.rows[0].slug }}""#)
                .output(json!({ "site": { "mode": "page", "status": "written", "path": "site/posts/hello.html", "route": "/posts/hello", "template": "pages/post.tsx", "folder": "site", "store": "local", "manifest_path": "site/.zebflow-static-site.json", "file": { "__zf_type": "file_ref", "backend": "zebfs", "store": "local", "ref": "site/posts/hello.html", "filename": "hello.html", "mime": "text/html", "kind": "binary", "size": 5120, "sha256": "sha256:…", "lifecycle": "durable", "origin": "web.site.generate", "trust": "generated" } } })),
            NodeExample::dsl("Build the docs site nightly", r#"web.site.generate --from handbook --folder handbook-site --name "Example Handbook""#)
                .output(json!({ "site": { "mode": "docs", "status": "ok", "name": "Example Handbook", "template": "docs.template.tsx", "from": "handbook", "folder": "handbook-site", "store": "local", "page_count": 12, "search_index_path": "handbook-site/search-index.json" } }))
                .note("Markdown under `docs/handbook/` becomes HTML under `handbook-site/`, private until the owner serves that folder as a site in Studio → Files."),
            NodeExample::dsl("Publish the 0.11 docs as the latest version", r#"web.site.generate --from handbook --folder site/docs --version 0.11 --latest"#)
                .output(json!({ "site": { "mode": "docs", "status": "ok", "name": "Handbook", "template": "docs.template.tsx", "from": "handbook", "folder": "site/docs", "store": "local", "page_count": 12, "version": "0.11", "latest": true, "versions_path": "site/docs/versions.json" } }))
                .note("With `site` served, the pages answer at `/docs/…` (latest) and `/docs/0.11/…` (pinned, not indexed); a later `--version 0.12 --latest` adds `/docs/0.12/` and leaves 0.11 as it was."),
        ],
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::{Config, Mode};

    fn config(path: Option<&str>, from: Option<&str>) -> Config {
        Config {
            template: Some("pages/a.tsx".to_string()),
            path: path.map(str::to_string),
            from: from.map(str::to_string),
            ..Default::default()
        }
    }

    #[test]
    fn the_role_given_chooses_the_mode() {
        assert_eq!(config(Some("index.html"), None).check().unwrap(), Mode::Page);
        assert_eq!(config(None, Some("handbook")).check().unwrap(), Mode::Docs);
        for (path, from) in [(Some("index.html"), Some("handbook")), (None, None), (Some("  "), None)] {
            let err = config(path, from).check().unwrap_err();
            assert_eq!(err.code, super::CODE_MODE, "{path:?} {from:?}");
        }
    }

    #[test]
    fn a_page_needs_its_template_and_each_mode_refuses_the_others_flag() {
        let mut page = config(Some("index.html"), None);
        page.template = None;
        assert_eq!(page.check().unwrap_err().code, super::CODE_MODE);
        let mut named_page = config(Some("index.html"), None);
        named_page.name = Some("Site".to_string());
        assert_eq!(named_page.check().unwrap_err().code, super::CODE_MODE);
        let mut routed_docs = config(None, Some("handbook"));
        routed_docs.route = Some("/x".to_string());
        assert_eq!(routed_docs.check().unwrap_err().code, super::CODE_MODE);
        let mut docs = config(None, Some("handbook"));
        docs.template = None;
        assert_eq!(docs.check().unwrap(), Mode::Docs, "a docs site has a default template");
    }

    #[test]
    fn on_conflict_is_closed_and_overwrites_by_default() {
        let mut page = config(Some("index.html"), None);
        assert_eq!(page.on_conflict().unwrap().as_str(), "overwrite");
        page.on_conflict = Some("replace".to_string());
        assert_eq!(page.check().unwrap_err().code, super::CODE_CONFLICT);
    }

    #[test]
    fn each_mode_has_its_default_folder() {
        let page = config(Some("index.html"), None);
        assert_eq!(page.folder_rel(Mode::Page).unwrap(), "site");
        assert_eq!(page.folder_rel(Mode::Docs).unwrap(), "docs");
        let named = Config { folder: Some("/static/site-a/".to_string()), ..page };
        assert_eq!(named.folder_rel(Mode::Page).unwrap(), "static/site-a");
    }
}
