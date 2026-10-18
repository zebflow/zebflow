//! The engine half of `web.site.generate`: render through the RWE engine and
//! the template cache this engine holds, then write the site into the
//! project store (`nodes/basic/web/site`). One function per mode.

use std::sync::Arc;

use serde_json::{Value, json};

use super::{BasicPipelineEngine, CacheEntry, hash_markup};
use crate::pipeline::PipelineError;
use crate::pipeline::model::PipelineContext;
use crate::pipeline::nodes::NodeExecutionOutput;
use crate::pipeline::nodes::basic::web::{self, site, static_site};
use crate::pipeline::nodes::shared::project_store::NodeStore;
use crate::pipeline::nodes::shared::util::with_answer;
use crate::rwe::{CompiledScript, TemplateSource};
use crate::zebfs::ZebFs;

const CODE_UNAVAILABLE: &str = "FW_NODE_WEB_SITE_GENERATE_UNAVAILABLE";
const CODE_TEMPLATE_ROOT: &str = "FW_NODE_WEB_SITE_GENERATE_TEMPLATE_ROOT";
const CODE_COMPILE: &str = "FW_NODE_WEB_SITE_GENERATE_COMPILE";
const CODE_RENDER: &str = "FW_NODE_WEB_SITE_GENERATE_RENDER";
const CODE_READ: &str = "FW_NODE_WEB_SITE_GENERATE_READ";
const CODE_TEMPLATE_WRITE: &str = "FW_NODE_WEB_SITE_GENERATE_TEMPLATE_WRITE";

/// What one render of a page produced, ready to be written.
struct RenderedPage {
    html: String,
    trace: Vec<String>,
}

impl BasicPipelineEngine {
    /// Runs `web.site.generate` in the mode its roles name.
    pub(super) fn run_site_generate(
        &self,
        node_id: &str,
        config: &site::Config,
        payload: &Value,
        metadata: &Value,
        ctx: &PipelineContext,
    ) -> Result<Vec<NodeExecutionOutput>, PipelineError> {
        match config.check()? {
            site::Mode::Page => self.run_site_page(node_id, config, payload, metadata, ctx),
            site::Mode::Docs => self.run_site_docs(node_id, config, payload, metadata, ctx),
        }
    }

    /// The store the node names, and its files; without a platform (tests,
    /// tools) the project's own files.
    fn site_store(&self, ctx: &PipelineContext, config: &site::Config) -> Result<(Option<NodeStore>, ZebFs), PipelineError> {
        let node_store = match self.platform.as_ref() {
            Some(platform) => Some(crate::pipeline::nodes::shared::project_store::open_store(
                platform,
                &ctx.owner,
                &ctx.project,
                config.store.as_deref(),
            )?),
            None => None,
        };
        let Some(zebfs) = node_store
            .as_ref()
            .map(|store| store.fs.clone())
            .or_else(|| self.repo_layout.as_ref().map(|layout| layout.open_files()))
        else {
            return Err(PipelineError::new(
                CODE_UNAVAILABLE,
                "the project's file store is not configured on this pipeline engine",
            ));
        };
        Ok((node_store, zebfs))
    }

    /// Writes a docs site's scaffolded template: through the repository API
    /// when the engine runs on a platform, else through the link-refusing
    /// repository writer (tests, tools).
    fn create_docs_template(
        &self,
        ctx: &PipelineContext,
        template_root: &std::path::Path,
        template_rel: &str,
        content: &str,
    ) -> Result<(), PipelineError> {
        let repo_rel = self
            .repo_layout
            .as_ref()
            .and_then(|layout| template_root.strip_prefix(&layout.repo_dir).ok())
            .map(|source| {
                let source = source.to_string_lossy().replace('\\', "/");
                if source.is_empty() { template_rel.to_string() } else { format!("{source}/{template_rel}") }
            });
        match (self.platform.as_ref(), repo_rel) {
            (Some(platform), Some(repo_rel)) => platform
                .projects
                .write_repo_file(&ctx.owner, &ctx.project, &repo_rel, content)
                .map(|_| ())
                .map_err(|err| PipelineError::new(CODE_TEMPLATE_WRITE, format!("'{repo_rel}': {}", err.message))),
            _ => crate::pipeline::nodes::shared::project_store::create_repo_file(
                template_root,
                template_rel,
                content.as_bytes(),
                CODE_TEMPLATE_WRITE,
            ),
        }
    }

    /// The compiled page for `source`, from the cache when its markup was seen.
    fn compile_site_template(
        &self,
        node_id: &str,
        source: &TemplateSource,
        processors: Vec<String>,
    ) -> Result<Arc<web::response::CompiledPage>, PipelineError> {
        let options = crate::rwe::ReactiveWebOptions {
            templates: crate::rwe::TemplateOptions {
                template_root: self.template_root.clone(),
                library_roots: self.library_roots(),
                style_entries: Vec::new(),
            },
            processors,
            ..Default::default()
        };
        let key = hash_markup(&source.markup);
        let cached = self
            .template_cache
            .as_ref()
            .and_then(|c| c.read().unwrap_or_else(|e| e.into_inner()).get(&key).map(|e| e.page.clone()));
        if let Some(hit) = cached {
            return Ok(hit);
        }
        let fresh = web::response::compile_page(node_id, source, &options, self.rwe.as_ref(), self.language.as_ref())
            .map(Arc::new)
            .map_err(|err| PipelineError::new(CODE_COMPILE, err.message))?;
        if let Some(cache) = &self.template_cache {
            let deps = fresh.template.dependency_paths.clone();
            cache
                .write()
                .unwrap_or_else(|e| e.into_inner())
                .insert(key, CacheEntry { page: fresh.clone(), dependencies: deps });
        }
        Ok(fresh)
    }

    /// Renders one page and decorates it to open from a file.
    fn render_site_page(
        &self,
        node_id: &str,
        compiled: &web::response::CompiledPage,
        state: Value,
        metadata: Value,
        ctx: &PipelineContext,
    ) -> Result<RenderedPage, PipelineError> {
        // A generated page is not an answer to a request: the run's trigger
        // (its headers, auth, pathname) is never injected into its state.
        let mut metadata = metadata;
        if let Some(map) = metadata.as_object_mut() {
            map.remove("trigger");
        }
        let enabled_libraries: Vec<String> = self
            .platform
            .as_ref()
            .and_then(|p| p.zebflow_cfg.get_rwe_libraries(&ctx.owner, &ctx.project).ok())
            .map(|libs| libs.into_keys().collect())
            .unwrap_or_default();
        let render_out = web::response::render_compiled_page(
            compiled,
            state,
            metadata,
            self.rwe.as_ref(),
            self.language.as_ref(),
            &ctx.request_id,
            enabled_libraries,
        )
        .map_err(|err| PipelineError::new(CODE_RENDER, err.message))?;
        let html = render_out
            .payload
            .get("html")
            .and_then(Value::as_str)
            .ok_or_else(|| PipelineError::new(CODE_RENDER, format!("node '{node_id}' did not return rendered html")))?
            .to_string();
        let hydration_payload = render_out.payload.get("hydration_payload").cloned().unwrap_or(Value::Null);
        let compiled_scripts = render_out
            .payload
            .get("compiled_scripts")
            .cloned()
            .and_then(|value| serde_json::from_value::<Vec<CompiledScript>>(value).ok())
            .unwrap_or_default();
        let html = site::page::build_static_html(html, &hydration_payload, &compiled_scripts, self.template_root.as_deref());
        Ok(RenderedPage { html, trace: render_out.trace })
    }

    fn asset_sources<'a>(&'a self, ctx: &'a PipelineContext, root: Option<&'a std::path::Path>) -> static_site::StaticAssetSources<'a> {
        static_site::StaticAssetSources {
            owner: Some(&ctx.owner),
            project: Some(&ctx.project),
            project_asset_root_abs: root,
        }
    }

    /// `--path`: one page into the site folder.
    fn run_site_page(
        &self,
        node_id: &str,
        config: &site::Config,
        payload: &Value,
        metadata: &Value,
        ctx: &PipelineContext,
    ) -> Result<Vec<NodeExecutionOutput>, PipelineError> {
        let (node_store, zebfs) = self.site_store(ctx, config)?;
        let template_source = site::page::resolve_template_source(node_id, config, self.template_root.as_deref())?;
        let compiled = self.compile_site_template(node_id, &template_source, Vec::new())?;

        let folder = config.folder_rel(site::Mode::Page)?;
        let page_path = site::page::page_path(config)?;
        let rel_path = site::page::page_key(config)?;
        let route = match config.route.clone().filter(|s| !s.trim().is_empty()) {
            Some(explicit) => explicit,
            None => static_site::route_path_for_output_path("/", &page_path)?,
        };
        let mut metadata = metadata.clone();
        if let Some(map) = metadata.as_object_mut() {
            map.insert("route".to_string(), Value::String(route.clone()));
        }
        let rendered = self.render_site_page(node_id, &compiled, site::page::page_input(payload), metadata, ctx)?;

        let asset_group = static_site::asset_group_id(&template_source.id, &template_source.markup);
        let project_asset_root = self.repo_layout.as_ref().map(|layout| layout.repo_static_dir());
        let site_store = static_site::SiteStore { store: &zebfs, root_rel: folder.clone() };
        let sources = self.asset_sources(ctx, project_asset_root.as_deref());
        // Decided before anything is written: `skip` and `error` leave the
        // site exactly as it was — no asset, no manifest entry.
        let relinked = static_site::relink_static_html(&page_path, &rendered.html, &sources)?;
        let on_conflict = config.on_conflict()?;
        let skipped = site::page::conflict_status(&zebfs, &rel_path, &relinked, on_conflict.as_str())? == Some("skipped");
        // The site's origin is its folder's serve origin, never a flag.
        let origin = static_site::site_origin(self.platform.as_ref(), &ctx.owner, &ctx.project, &folder);
        let status = if skipped {
            "skipped"
        } else {
            let localized = static_site::localize_static_html_assets(&site_store, &page_path, &rendered.html, sources, &asset_group)?;
            let status = site::page::write_generated_object(&zebfs, &rel_path, &localized.html, on_conflict.as_str())?;
            let page_record = static_site::StaticPageRecord {
                path: page_path.clone(),
                route: route.clone(),
                template: template_source.id.clone(),
                asset_group: asset_group.clone(),
                generator: site::NODE_KIND.to_string(),
            };
            static_site::update_site_manifest(
                &site_store,
                &folder,
                origin.as_deref(),
                "/",
                site::NODE_KIND,
                &template_source.id,
                &asset_group,
                &[page_record],
                &localized.assets,
                false,
            )?;
            status
        };
        let bytes = relinked.len() as u64;
        let file = match node_store.as_ref() {
            Some(store) => {
                let written = crate::pipeline::nodes::shared::project_store::read_capped(&zebfs, &rel_path, CODE_READ)?;
                let leaf = rel_path.rsplit('/').next().unwrap_or(&rel_path).to_string();
                store.file_ref(&rel_path, &leaf, "text/html", &written, site::NODE_KIND, "generated")
            }
            None => Value::Null,
        };

        let mut trace = rendered.trace;
        trace.push(format!("node_kind={}", site::NODE_KIND));
        trace.push(format!("generated_path={rel_path}"));
        trace.push(format!("generated_status={status}"));
        Ok(vec![NodeExecutionOutput {
            output_pins: vec![site::OUTPUT_PIN_OUT.to_string()],
            payload: with_answer(payload.clone(), json!({
                "site": {
                    "mode": site::Mode::Page.as_str(),
                    "status": status,
                    "path": rel_path,
                    "route": route,
                    "store": node_store.as_ref().map(|store| store.id.clone()),
                    "file": file,
                    "origin": origin,
                    "template": template_source.id,
                    "folder": folder,
                    "manifest_path": static_site::site_manifest_rel_path(&folder),
                    "asset_group": asset_group,
                    "bytes": bytes,
                }
            })),
            trace,
        }])
    }

    /// `--from`: every Markdown page of a docs folder, its sitemap and search
    /// index. With `--version` the build is written under `<folder>/<version>/`
    /// (pinned, not indexed) and, with `--latest`, again at `<folder>/`; the
    /// versions list is updated once every copy is written.
    fn run_site_docs(
        &self,
        node_id: &str,
        config: &site::Config,
        payload: &Value,
        metadata: &Value,
        ctx: &PipelineContext,
    ) -> Result<Vec<NodeExecutionOutput>, PipelineError> {
        let (node_store, zebfs) = self.site_store(ctx, config)?;
        let Some(template_root) = &self.template_root else {
            return Err(PipelineError::new(CODE_TEMPLATE_ROOT, "template_root is not configured on this pipeline engine"));
        };
        let Some(docs_root) = self.repo_layout.as_ref().map(|layout| layout.repo_docs_dir()) else {
            return Err(PipelineError::new(CODE_TEMPLATE_ROOT, "project layout is not configured on this pipeline engine"));
        };
        let folder = config.folder_rel(site::Mode::Docs)?;
        // The unversioned address of the site: its folder's place under the served folder.
        let root_path = static_site::site_base_path(self.platform.as_ref(), &ctx.owner, &ctx.project, &folder);
        let label = config.version_label()?.map(str::to_string);
        let mut targets = Vec::new();
        match &label {
            None => targets.push(DocsTarget { site_root_rel: folder.clone(), base_path: root_path.clone(), version: None }),
            Some(label) => {
                let version = |pinned| site::docs::DocsVersion { label: label.clone(), root_path: root_path.clone(), pinned };
                targets.push(DocsTarget {
                    site_root_rel: format!("{}/{label}", folder.trim_end_matches('/')),
                    base_path: format!("{root_path}{label}/"),
                    version: Some(version(true)),
                });
                if config.latest {
                    targets.push(DocsTarget { site_root_rel: folder.clone(), base_path: root_path.clone(), version: Some(version(false)) });
                }
            }
        }

        let mut built = Vec::with_capacity(targets.len());
        for target in &targets {
            built.push(self.build_docs_target(node_id, config, payload, metadata, ctx, &zebfs, template_root, &docs_root, target)?);
        }
        let versions_path = match &label {
            Some(label) => Some(update_versions_list(&zebfs, &folder, label, &format!("{root_path}{label}/"), config.latest)?),
            None => None,
        };

        // The answer describes the copy readers are sent to: the root when
        // there is one, else the only build.
        let main = built.last().expect("at least one target");
        let generated: usize = built.iter().map(|b| b.written.generated).sum();
        let skipped: usize = built.iter().map(|b| b.written.skipped).sum();
        Ok(vec![NodeExecutionOutput {
            output_pins: vec![site::OUTPUT_PIN_OUT.to_string()],
            payload: with_answer(payload.clone(), json!({
                "site": {
                    "mode": site::Mode::Docs.as_str(),
                    "status": "ok",
                    "name": main.site_title,
                    "template": main.template_rel_path,
                    "from": main.from_rel,
                    "folder": main.site_root_rel,
                    "store": node_store.as_ref().map(|store| store.id.clone()),
                    "origin": main.origin,
                    "manifest_path": static_site::site_manifest_rel_path(&main.site_root_rel),
                    "asset_group": main.asset_group,
                    "page_count": main.page_count,
                    "generated_files": generated,
                    "skipped_files": skipped,
                    "sitemap_path": main.sitemap_path,
                    "robots_path": main.sitemap_path.as_ref().map(|_| site::docs::robots_rel_path(&main.site_root_rel)),
                    "llms_path": main.sitemap_path.as_ref().map(|_| site::docs::llms_rel_path(&main.site_root_rel)),
                    "llms_full_path": main.sitemap_path.as_ref().map(|_| site::docs::llms_full_rel_path(&main.site_root_rel)),
                    "search_manifest_path": site::docs::search_file_rel_path(&main.site_root_rel, &site::docs_search::manifest_rel_path()),
                    "search_files": main.search_files,
                    "routes": main.routes,
                    "version": label,
                    "latest": config.latest,
                    "versions_path": versions_path,
                }
            })),
            trace: vec![
                format!("node={node_id}"),
                format!("node_kind={}", site::NODE_KIND),
                format!("pages={}", main.page_count),
                format!("copies={}", built.len()),
            ],
        }])
    }

    /// One copy of a docs site: every page, its Markdown twin, the search
    /// index and, unless it is a pinned version, the sitemap, robots and llms
    /// files — all decided before the first is written.
    #[allow(clippy::too_many_arguments)]
    fn build_docs_target(
        &self,
        node_id: &str,
        config: &site::Config,
        payload: &Value,
        metadata: &Value,
        ctx: &PipelineContext,
        zebfs: &ZebFs,
        template_root: &std::path::Path,
        docs_root: &std::path::Path,
        target: &DocsTarget,
    ) -> Result<DocsBuilt, PipelineError> {
        let on_conflict = config.on_conflict()?;
        let create_template = |rel: &str, content: &str| self.create_docs_template(ctx, template_root, rel, content);
        let mut docs = site::docs::load_site(config, &target.site_root_rel, &target.base_path, template_root, docs_root, &create_template)?;
        docs.set_origin(static_site::site_origin(self.platform.as_ref(), &ctx.owner, &ctx.project, &docs.site_root_rel));
        if let Some(version) = &target.version {
            if let Some(page) = docs.pages.iter().find(|page| page.slug_segments.first().is_some_and(|first| *first == version.label)) {
                return Err(PipelineError::new(
                    site::CODE_VERSION,
                    format!("--version {} would hide the page {}: a top-level page or folder may not be named like a version", version.label, page.source_rel_path),
                ));
            }
            docs.set_version(version.clone());
        }
        docs.set_asset_base(&ctx.owner, &ctx.project);
        let compiled = self.compile_site_template(
            node_id,
            &docs.template_source,
            vec!["tailwind".to_string(), "markdown".to_string()],
        )?;

        let asset_group = static_site::asset_group_id(&docs.template_rel_path, &docs.template_source.markup);
        let site_store = static_site::SiteStore { store: zebfs, root_rel: docs.site_root_rel.clone() };
        let project_asset_root = self.repo_layout.as_ref().map(|layout| layout.repo_static_dir());
        let source = site::page::page_input(payload);

        // Every file of the build, rendered and decided before the first is
        // written: `--on-conflict error` on one page leaves the whole site as
        // it was.
        let mut files = Vec::new();
        let mut rendered_pages = Vec::new();
        for (page_index, page) in docs.pages.iter().enumerate() {
            let mut page_metadata = metadata.clone();
            if let Some(map) = page_metadata.as_object_mut() {
                map.insert("route".to_string(), Value::String(site::docs::default_route(page)));
            }
            let state = site::docs::page_payload(&docs, page_index, source.clone())?;
            let rendered = self.render_site_page(node_id, &compiled, state, page_metadata, ctx)?;
            let relinked = static_site::relink_static_html(
                &page.output_rel_path,
                &rendered.html,
                &self.asset_sources(ctx, project_asset_root.as_deref()),
            )?;
            let html = site::docs::apply_page_seo(relinked, &docs, page_index);
            files.push((site::docs::output_rel_path(page, &docs.site_root_rel)?, html));
            rendered_pages.push(rendered.html);
        }
        // Every page also answers as Markdown, beside its HTML: the source is
        // already in hand, and it is what `llms-full.txt` and an agent read.
        for page in docs.pages.iter() {
            files.push((
                site::docs::markdown_twin_rel_path(page, &docs.site_root_rel)?,
                site::docs::markdown_twin_body(page),
            ));
        }
        // The machine-readable surface (`discoverability.md` §3), written on
        // every build whether or not the folder has an address yet.
        // A pinned version is never indexed: these belong to the unversioned root.
        let indexed = docs.version.as_ref().is_none_or(|version| !version.pinned);
        let sitemap_path = indexed.then(|| site::docs::sitemap_rel_path(&docs.site_root_rel));
        if indexed {
            files.push((site::docs::sitemap_rel_path(&docs.site_root_rel), docs.sitemap_xml.clone()));
            files.push((site::docs::robots_rel_path(&docs.site_root_rel), docs.robots_txt.clone()));
            files.push((site::docs::llms_rel_path(&docs.site_root_rel), docs.llms_txt.clone()));
            files.push((site::docs::llms_full_rel_path(&docs.site_root_rel), docs.llms_full_txt.clone()));
        }
        // The search index, in chunks: the page fetches the ones its query
        // names and nothing else (`docs_search.rs`).
        for file in &docs.search_files {
            files.push((site::docs::search_file_rel_path(&docs.site_root_rel, &file.rel_path), file.contents.clone()));
        }
        let mut statuses = Vec::with_capacity(files.len());
        for (rel_path, contents) in &files {
            statuses.push(site::page::conflict_status(zebfs, rel_path, contents, on_conflict.as_str())?);
        }

        let mut written = Written::default();
        let mut asset_records = Vec::new();
        for (page, html) in docs.pages.iter().zip(&rendered_pages) {
            let localized = static_site::localize_static_html_assets(
                &site_store,
                &page.output_rel_path,
                html,
                self.asset_sources(ctx, project_asset_root.as_deref()),
                &asset_group,
            )?;
            asset_records.extend(localized.assets);
        }
        for ((rel_path, contents), status) in files.iter().zip(statuses) {
            written.count(match status {
                Some(kept) => kept,
                None => site::page::write_generated_object(zebfs, rel_path, contents, on_conflict.as_str())?,
            });
        }
        let routes = docs.pages.iter().map(|page| page.route_path.clone()).collect::<Vec<_>>();
        let page_records = docs
            .pages
            .iter()
            .map(|page| static_site::StaticPageRecord {
                path: page.output_rel_path.clone(),
                route: page.route_path.clone(),
                template: docs.template_rel_path.clone(),
                asset_group: asset_group.clone(),
                generator: site::DOCS_MANIFEST_GENERATOR.to_string(),
            })
            .collect::<Vec<_>>();

        static_site::update_site_manifest(
            &site_store,
            &docs.site_root_rel,
            docs.deploy_base_url.as_deref(),
            &docs.deploy_base_path,
            site::DOCS_MANIFEST_GENERATOR,
            &docs.template_rel_path,
            &asset_group,
            &page_records,
            &asset_records,
            true,
        )?;

        Ok(DocsBuilt {
            site_title: docs.site_title.clone(),
            template_rel_path: docs.template_rel_path.clone(),
            from_rel: docs.from_rel.clone(),
            site_root_rel: docs.site_root_rel.clone(),
            origin: docs.deploy_base_url.clone(),
            asset_group,
            page_count: docs.pages.len(),
            written,
            sitemap_path,
            search_files: docs.search_files.len(),
            routes,
        })
    }
}

/// Where one copy of a docs site is written and the address it answers at.
struct DocsTarget {
    site_root_rel: String,
    base_path: String,
    version: Option<site::docs::DocsVersion>,
}

/// What one copy wrote, for the node's answer.
struct DocsBuilt {
    site_title: String,
    template_rel_path: String,
    from_rel: String,
    site_root_rel: String,
    origin: Option<String>,
    asset_group: String,
    page_count: usize,
    written: Written,
    sitemap_path: Option<String>,
    search_files: usize,
    routes: Vec<String>,
}

/// `<folder>/versions.json`, with this version in it: `{ "versions": [{ "version",
/// "path", "latest" }] }`, the latest first, then newest to oldest. Every
/// build of the site reads it when its page loads, so a version is listed
/// here once and never rebuilt. Answers the file's path.
fn update_versions_list(zebfs: &ZebFs, folder: &str, label: &str, path: &str, latest: bool) -> Result<String, PipelineError> {
    let rel = format!("{}/{}", folder.trim_end_matches('/'), site::docs::VERSIONS_FILE);
    let previous = crate::pipeline::nodes::shared::project_store::read_capped(zebfs, &rel, CODE_READ)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
    let mut versions: Vec<Value> = previous
        .as_ref()
        .and_then(|doc| doc.get("versions"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|entry| entry.get("version").and_then(Value::as_str).is_some_and(|v| v != label))
        .collect();
    if latest {
        for entry in versions.iter_mut() {
            if let Some(map) = entry.as_object_mut() {
                map.insert("latest".to_string(), Value::Bool(false));
            }
        }
    }
    versions.push(json!({ "version": label, "path": path, "latest": latest }));
    versions.sort_by(|a, b| {
        let latest = |v: &Value| v.get("latest").and_then(Value::as_bool).unwrap_or(false);
        let key = |v: &Value| v.get("version").and_then(Value::as_str).unwrap_or("").to_string();
        latest(b).cmp(&latest(a)).then_with(|| natural_cmp(&key(b), &key(a)))
    });
    let body = serde_json::to_string_pretty(&json!({ "versions": versions })).unwrap_or_default() + "\n";
    site::page::write_generated_object(zebfs, &rel, &body, "overwrite")?;
    Ok(rel)
}

/// `0.9` before `0.11`: digit runs compare as numbers.
fn natural_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    fn parts(s: &str) -> Vec<(bool, String)> {
        let mut out: Vec<(bool, String)> = Vec::new();
        for c in s.chars() {
            let digit = c.is_ascii_digit();
            match out.last_mut() {
                Some((d, run)) if *d == digit => run.push(c),
                _ => out.push((digit, c.to_string())),
            }
        }
        out
    }
    let (pa, pb) = (parts(a), parts(b));
    for (x, y) in pa.iter().zip(pb.iter()) {
        let ord = if x.0 && y.0 {
            x.1.parse::<u128>().unwrap_or(0).cmp(&y.1.parse::<u128>().unwrap_or(0))
        } else {
            x.1.cmp(&y.1)
        };
        if ord != std::cmp::Ordering::Equal {
            return ord;
        }
    }
    pa.len().cmp(&pb.len())
}

/// How many files a docs build wrote, and how many it left as they were.
#[derive(Default)]
struct Written {
    generated: usize,
    skipped: usize,
}

impl Written {
    fn count(&mut self, status: &str) {
        if status == "skipped" || status == "unchanged" {
            self.skipped += 1;
        } else {
            self.generated += 1;
        }
    }
}
