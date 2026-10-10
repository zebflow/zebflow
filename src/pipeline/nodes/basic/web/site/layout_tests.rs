//! What a site writes into the store, key by key, in both modes.
//!
//! Live sites are regenerated in place, and their manifest is read back on
//! every run: the keys, the `_assets/` layout and the manifest's labels are a
//! format, not a detail. These tests fix them.

use std::path::Path;
use std::sync::Arc;

use serde_json::{Value, json};

use super::NODE_KIND;
use crate::language::DenoSandboxEngine;
use crate::pipeline::engines::basic::new_template_cache;
use crate::pipeline::{BasicPipelineEngine, PipelineContext, PipelineEngine, PipelineGraph, PipelineNode};
use crate::platform::adapters::file::build_file_adapter;
use crate::platform::model::{FileAdapterKind, ProjectFileLayout};
use crate::platform::services::project_config::ProjectConfigurationService;
use crate::rwe::resolve_engine_or_default;

const PAGE_TSX: &str = r#"
export const page = { head: { icons: [ { rel: "icon", href: "/static/demo/site-a/icons/favicon.ico" } ] } };
export default function SongPage(input) {
  return (
    <Page>
      <main className="p-6">
        <img src="/assets/branding/logo.svg" alt="Logo" />
        <h1 className="text-3xl">{input.title}</h1>
      </main>
    </Page>
  );
}
"#;

struct Site {
    root: std::path::PathBuf,
    layout: ProjectFileLayout,
    engine: BasicPipelineEngine,
}

impl Site {
    fn new(label: &str) -> Self {
        let root = std::env::temp_dir().join(format!(
            "zebflow-site-layout-{label}-{}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("time").as_nanos()
        ));
        let file = build_file_adapter(
            FileAdapterKind::Filesystem,
            root.clone(),
            Arc::new(ProjectConfigurationService::new(root.join("users"))),
            None,
        );
        let layout = file.ensure_project_layout("demo", "site-a").expect("layout");
        let source = layout.repo_source_dir();
        std::fs::create_dir_all(source.join("pages")).unwrap();
        std::fs::write(source.join("pages/song.tsx"), PAGE_TSX).unwrap();
        std::fs::create_dir_all(layout.repo_static_dir().join("icons")).unwrap();
        std::fs::write(layout.repo_static_dir().join("icons/favicon.ico"), b"ico").unwrap();
        let docs = layout.repo_docs_dir().join("guide");
        std::fs::create_dir_all(docs.join("basic")).unwrap();
        std::fs::write(docs.join("_meta.yaml"), "title: Guide Docs\n").unwrap();
        std::fs::write(docs.join("index.md"), "# Home\n\nWelcome.").unwrap();
        std::fs::write(docs.join("basic/query.md"), "# Query\n\n## Select\n\nUse select.\n").unwrap();
        let engine = BasicPipelineEngine::new(Arc::new(DenoSandboxEngine::default()), resolve_engine_or_default(None), None)
            .with_project_layout(Some(layout.clone()))
            .with_template_cache(new_template_cache())
            .with_data_root(root.clone());
        Self { root, layout, engine }
    }

    async fn run(&self, config: Value, input: Value) -> Value {
        let graph = PipelineGraph {
            id: "gen".to_string(),
            description: None,
            metadata: None,
            notes: Vec::new(),
            entry_nodes: vec!["gen".to_string()],
            nodes: vec![PipelineNode {
                id: "gen".to_string(),
                kind: NODE_KIND.to_string(),
                input_pins: vec!["in".to_string()],
                output_pins: vec!["out".to_string()],
                config,
            }],
            edges: vec![],
        };
        let ctx = PipelineContext {
            owner: "demo".to_string(),
            project: "site-a".to_string(),
            pipeline: graph.id.clone(),
            request_id: "req-1".to_string(),
            route: String::new(),
            input,
            trigger: None,
            placeholder: None,
        };
        self.engine.execute_async(&graph, &ctx).await.expect("generate").value
    }

    /// Every key under `folder`, sorted.
    fn keys(&self, folder: &str) -> Vec<String> {
        fn walk(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
                let path = entry.path();
                if path.is_dir() { walk(&path, out) } else { out.push(path) }
            }
        }
        let base = self.layout.files_dir.join(folder);
        let mut files = Vec::new();
        walk(&base, &mut files);
        let mut keys: Vec<String> =
            files.iter().map(|f| f.strip_prefix(&base).unwrap().to_string_lossy().replace('\\', "/")).collect();
        keys.sort();
        keys
    }

    fn manifest(&self, folder: &str) -> Value {
        let raw = std::fs::read_to_string(self.layout.files_dir.join(folder).join(".zebflow-static-site.json")).expect("manifest");
        serde_json::from_str(&raw).expect("manifest json")
    }
}

impl Drop for Site {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[tokio::test]
async fn a_page_site_keeps_its_keys_and_manifest() {
    let site = Site::new("page");
    let config = json!({ "template": "pages/song.tsx", "folder": "static/music", "path": "songs/{{ $input.slug }}/index.html" });
    let first = site.run(config.clone(), json!({ "slug": "one", "title": "One" })).await;
    site.run(config.clone(), json!({ "slug": "two", "title": "Two" })).await;
    assert_eq!(first["slug"], "one", "the payload is kept");
    assert_eq!(first["site"]["path"], "static/music/songs/one/index.html");
    assert_eq!(first["site"]["folder"], "static/music");
    assert_eq!(first["site"]["route"], "/songs/one/");
    assert_eq!(
        site.keys("static/music"),
        [
            ".zebflow-static-site.json",
            "_assets/branding/logo.svg",
            "_assets/libraries/zeb/react/0.1/runtime/zeb_react.js",
            "_assets/libraries/zeb/react/0.1/runtime/zeb_react.mjs",
            "_assets/project/icons/favicon.ico",
            "songs/one/index.html",
            "songs/two/index.html",
        ]
    );
    let manifest = site.manifest("static/music");
    assert_eq!(manifest["site_root"], "static/music");
    assert_eq!(manifest["deploy_base_path"], "/");
    assert_eq!(manifest["generators"], json!(["web.site.generate"]));
    assert_eq!(manifest["pages"][0]["path"], "songs/one/index.html");
    assert_eq!(manifest["pages"][0]["route"], "/songs/one/");
    assert_eq!(manifest["pages"][0]["generator"], "web.site.generate");
    let again = site.run(config, json!({ "slug": "two", "title": "Two" })).await;
    assert_eq!(again["site"]["status"], "unchanged");
}

#[tokio::test]
async fn a_docs_site_keeps_its_keys_manifest_and_scaffold() {
    let site = Site::new("docs");
    let config = json!({ "from": "guide", "template": "docsite/docs.template.tsx", "name": "Guide" });
    let first = site.run(config.clone(), json!({})).await;
    assert_eq!(first["site"]["mode"], "docs");
    assert_eq!(first["site"]["folder"], "docs");
    assert_eq!(first["site"]["from"], "guide");
    // Two pages, each with its Markdown twin, the chunked search index (one
    // metadata block, one chunk per first letter in use, the manifest) and the
    // machine-readable surface (`discoverability.md` §3): sitemap, robots and
    // the two llms files, written whether or not the folder has an address.
    assert_eq!(first["site"]["generated_files"], 17);
    assert_eq!(
        site.keys("docs"),
        [
            ".zebflow-static-site.json",
            "_assets/libraries/zeb/markdown/0.1/runtime/markdown.bundle.mjs",
            "_assets/libraries/zeb/react/0.1/runtime/zeb_react.js",
            "_assets/libraries/zeb/react/0.1/runtime/zeb_react.mjs",
            "basic/query/index.html",
            "basic/query/index.md",
            "index.html",
            "index.md",
            "llms-full.txt",
            "llms.txt",
            "robots.txt",
            "search/m-0.json",
            "search/manifest.json",
            "search/t-h.json",
            "search/t-q.json",
            "search/t-s.json",
            "search/t-u.json",
            "search/t-w.json",
            "search/x-0.json",
            "search/x-1.json",
            "sitemap.xml",
        ]
    );
    // Docs-built pages keep the generator label live manifests already carry.
    let manifest = site.manifest("docs");
    assert_eq!(manifest["generators"], json!([super::DOCS_MANIFEST_GENERATOR]));
    assert_eq!(manifest["pages"][0]["generator"], super::DOCS_MANIFEST_GENERATOR);
    assert_eq!(manifest["templates"][0]["template"], "docsite/docs.template.tsx");
    let scaffold = std::fs::read_to_string(site.layout.repo_source_dir().join("docsite/docs.template.tsx")).expect("scaffold");
    assert_eq!(scaffold, super::docs_scaffold::default_template_source(Some("Guide")));

    let again = site.run(config, json!({})).await;
    // A second build writes nothing: every one of the nine files — pages,
    // twins, index and surface — is byte-identical, so each is skipped.
    assert_eq!((again["site"]["generated_files"].as_u64(), again["site"]["skipped_files"].as_u64()), (Some(0), Some(17)));
}

#[tokio::test]
async fn a_docs_site_without_a_template_flag_scaffolds_the_default() {
    let site = Site::new("docs-default");
    let out = site.run(json!({ "from": "guide", "folder": "handbook-site" }), json!({})).await;
    assert_eq!(out["site"]["template"], super::DEFAULT_DOCS_TEMPLATE);
    assert_eq!(out["site"]["name"], "Guide Docs", "the folder's _meta.yaml title");
    assert!(site.layout.repo_source_dir().join(super::DEFAULT_DOCS_TEMPLATE).is_file());
    assert!(site.keys("handbook-site").contains(&"basic/query/index.html".to_string()));
}

#[tokio::test]
async fn both_roles_or_neither_is_refused_before_anything_is_written() {
    let site = Site::new("mode");
    for config in [
        json!({ "template": "pages/song.tsx", "path": "index.html", "from": "guide" }),
        json!({ "template": "pages/song.tsx" }),
    ] {
        let graph = PipelineGraph {
            id: "gen".to_string(),
            description: None,
            metadata: None,
            notes: Vec::new(),
            entry_nodes: vec!["gen".to_string()],
            nodes: vec![PipelineNode {
                id: "gen".to_string(),
                kind: NODE_KIND.to_string(),
                input_pins: vec!["in".to_string()],
                output_pins: vec!["out".to_string()],
                config: config.clone(),
            }],
            edges: vec![],
        };
        let err = site.engine.validate_graph(&graph).unwrap_err();
        assert_eq!(err.code, super::CODE_MODE, "{config}");
    }
    assert!(site.keys("site").is_empty() && site.keys("docs").is_empty());
}
