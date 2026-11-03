//! A save recompiles exactly the pages built from the saved file, in place;
//! a page is one entry, named by its project and file, never by markup alone.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::*;
use crate::language::DenoSandboxEngine;
use crate::rwe::{TemplateOptions, resolve_engine_or_default};

const HOME: &str = r#"import Badge from "@/components/badge";
export default function Home(input) {
  return (<Page><main><Badge /></main></Page>);
}"#;

const ABOUT: &str = r#"export default function About(input) {
  return (<Page><main>about</main></Page>);
}"#;

fn badge(marker: &str) -> String {
    format!("export default function Badge() {{ return <span>{marker}</span>; }}")
}

/// One project's repository, with the cache a running process would share.
struct Project {
    root: PathBuf,
    cache: TemplateCache,
}

impl Project {
    fn new(label: &str, badge_marker: &str, cache: TemplateCache) -> Self {
        let root = std::env::temp_dir().join(format!(
            "zebflow-page-cache-{label}-{}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).expect("time").as_nanos()
        ));
        std::fs::create_dir_all(root.join("pages")).unwrap();
        std::fs::create_dir_all(root.join("components")).unwrap();
        std::fs::write(root.join("pages/home.tsx"), HOME).unwrap();
        std::fs::write(root.join("pages/about.tsx"), ABOUT).unwrap();
        std::fs::write(root.join("components/badge.tsx"), badge(badge_marker)).unwrap();
        let root = std::fs::canonicalize(&root).unwrap();
        Self { root, cache }
    }

    fn alone(label: &str) -> Self {
        Self::new(label, "BADGE_ONE", new_template_cache())
    }

    fn path(&self, rel: &str) -> String {
        self.root.join(rel).to_string_lossy().into_owned()
    }

    fn identity(&self, rel: &str) -> String {
        format!("page|{}|{}", self.root.display(), self.root.join(rel).display())
    }

    /// What a request does: read the entry page, trim it, compile through the cache.
    fn request(&self, rel: &str) -> Arc<web::response::CompiledPage> {
        let markup = std::fs::read_to_string(self.root.join(rel)).unwrap().trim().to_string();
        self.request_with(rel, markup)
    }

    /// A request whose markup was read earlier, before whatever happened since.
    fn request_with(&self, rel: &str, markup: String) -> Arc<web::response::CompiledPage> {
        let identity = self.identity(rel);
        let root = self.root.clone();
        compile_cached(Some(&self.cache), &identity, &markup.clone(), || recipe(&root, identity.clone(), rel, markup))
            .expect("compiles")
    }

    fn cached(&self, rel: &str) -> Vec<Arc<web::response::CompiledPage>> {
        let identity = self.identity(rel);
        let map = self.cache.read().unwrap();
        map.values().filter(|e| e.recipe.identity == identity).map(|e| e.page.clone()).collect()
    }

    fn len(&self) -> usize {
        self.cache.read().unwrap().len()
    }
}

impl Drop for Project {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

fn recipe(root: &Path, identity: String, rel: &str, markup: String) -> PageRecipe {
    let options = crate::rwe::ReactiveWebOptions {
        templates: TemplateOptions {
            template_root: Some(root.to_path_buf()),
            library_roots: Default::default(),
            style_entries: Vec::new(),
        },
        ..Default::default()
    };
    let entry = root.join(rel);
    PageRecipe::new(
        identity,
        Some(&entry),
        "page",
        TemplateSource { id: rel.to_string(), source_path: Some(entry.clone()), markup },
        options,
        resolve_engine_or_default(None),
        Arc::new(DenoSandboxEngine::default()),
    )
}

fn shows(page: &web::response::CompiledPage, marker: &str) -> bool {
    format!("{:?}", page.template.engine_payload).contains(marker)
}

#[tokio::test]
async fn a_component_save_recompiles_its_pages_in_place_and_no_others() {
    let p = Project::alone("component");
    let home = p.request("pages/home.tsx");
    let about = p.request("pages/about.tsx");
    assert!(shows(&home, "BADGE_ONE"));

    std::fs::write(p.root.join("components/badge.tsx"), badge("BADGE_TWO")).unwrap();
    let rebuilt = refresh_template_cache_by_path(&p.cache, &p.path("components/badge.tsx")).await;
    assert_eq!(rebuilt, 1, "only the page that imports the badge");

    let fresh = p.cached("pages/home.tsx");
    assert_eq!(fresh.len(), 1);
    assert!(shows(&fresh[0], "BADGE_TWO"), "the cached page is the new one");
    assert!(!Arc::ptr_eq(&fresh[0], &home));

    // The next request is a hit on the new page: it renders, never compiles.
    let served = p.request("pages/home.tsx");
    assert!(Arc::ptr_eq(&served, &fresh[0]));
    assert!(Arc::ptr_eq(&about, &p.request("pages/about.tsx")), "an unrelated page is untouched");
}

#[tokio::test]
async fn an_entry_page_save_replaces_its_page() {
    let p = Project::alone("entry");
    p.request("pages/about.tsx");

    std::fs::write(p.root.join("pages/about.tsx"), ABOUT.replace("about", "ABOUT_TWO")).unwrap();
    assert_eq!(refresh_template_cache_by_path(&p.cache, &p.path("pages/about.tsx")).await, 1);

    let cached = p.cached("pages/about.tsx");
    assert_eq!(cached.len(), 1, "the old markup's entry is gone, not left behind");
    assert!(shows(&cached[0], "ABOUT_TWO"));
    assert!(Arc::ptr_eq(&p.request("pages/about.tsx"), &cached[0]));
}

#[tokio::test]
async fn a_request_that_read_the_old_markup_cannot_leave_a_second_entry() {
    let p = Project::alone("late");
    p.request("pages/about.tsx");
    // The request read the page, then waited on an upstream node...
    let read_before_save = std::fs::read_to_string(p.root.join("pages/about.tsx")).unwrap().trim().to_string();
    std::fs::write(p.root.join("pages/about.tsx"), ABOUT.replace("about", "ABOUT_TWO")).unwrap();
    refresh_template_cache_by_path(&p.cache, &p.path("pages/about.tsx")).await;
    // ...and compiles only now, from the markup it read.
    p.request_with("pages/about.tsx", read_before_save);

    assert_eq!(p.cached("pages/about.tsx").len(), 1, "one entry per page, whatever the order");
    assert!(shows(&p.request("pages/about.tsx"), "ABOUT_TWO"), "the page on disk is what is served");
}

#[tokio::test]
async fn two_projects_with_the_same_page_are_two_pages() {
    let shared = new_template_cache();
    let a = Project::new("site-a", "BADGE_SITE_A", shared.clone());
    let b = Project::new("site-b", "BADGE_SITE_B", shared.clone());

    assert!(shows(&a.request("pages/home.tsx"), "BADGE_SITE_A"));
    assert!(shows(&b.request("pages/home.tsx"), "BADGE_SITE_B"), "not site-a's badge");

    std::fs::write(b.root.join("components/badge.tsx"), badge("BADGE_SITE_B2")).unwrap();
    assert_eq!(refresh_template_cache_by_path(&shared, &b.path("components/badge.tsx")).await, 1);
    assert!(shows(&a.request("pages/home.tsx"), "BADGE_SITE_A"));
    assert!(shows(&b.request("pages/home.tsx"), "BADGE_SITE_B2"));
}

#[tokio::test]
async fn one_batch_rebuilds_each_page_once() {
    let p = Project::alone("batch");
    p.request("pages/home.tsx");
    std::fs::write(p.root.join("components/badge.tsx"), badge("BADGE_TWO")).unwrap();
    std::fs::write(p.root.join("pages/home.tsx"), format!("{HOME}\n")).unwrap();

    let paths = vec![p.path("components/badge.tsx"), p.path("pages/home.tsx")];
    assert_eq!(refresh_template_cache_by_paths(&p.cache, paths).await, 1);
    assert!(shows(&p.request("pages/home.tsx"), "BADGE_TWO"));
}

#[tokio::test]
async fn a_page_that_no_longer_compiles_is_evicted() {
    let p = Project::alone("broken");
    p.request("pages/home.tsx");

    std::fs::write(p.root.join("components/badge.tsx"), "export default function Badge( {").unwrap();
    assert_eq!(refresh_template_cache_by_path(&p.cache, &p.path("components/badge.tsx")).await, 0);
    assert_eq!(p.len(), 0, "the next request compiles and reports the error");
}

#[test]
fn deleting_a_folder_evicts_every_page_built_from_it() {
    let p = Project::alone("folder");
    p.request("pages/home.tsx");
    p.request("pages/about.tsx");

    // Resolved before the delete, as the delete routes do.
    let folder = p.path("components");
    std::fs::remove_dir_all(p.root.join("components")).unwrap();
    evict_template_cache_by_path(&p.cache, &folder);
    assert_eq!(p.cached("pages/home.tsx").len(), 0, "home imported a file inside it");
    assert_eq!(p.cached("pages/about.tsx").len(), 1, "about did not");

    // A sibling whose name only starts the same is not inside it.
    evict_template_cache_by_path(&p.cache, &p.path("pages/abo"));
    assert_eq!(p.len(), 1);
    evict_template_cache_by_path(&p.cache, &p.path("pages/about.tsx"));
    assert_eq!(p.len(), 0, "an entry page is evicted by its own path");
}

#[test]
fn a_compile_that_began_before_a_save_is_not_cached() {
    let p = Project::alone("race");
    let markup = HOME.trim().to_string();
    let identity = p.identity("pages/home.tsx");
    let (root, cache) = (p.root.clone(), p.cache.clone());
    let served = compile_cached(Some(&p.cache), &identity, &markup.clone(), || {
        // A save starts its refresh while this request is compiling.
        cache.generation.fetch_add(1, Ordering::AcqRel);
        recipe(&root, identity.clone(), "pages/home.tsx", markup)
    })
    .expect("the request is still answered");
    assert!(shows(&served, "BADGE_ONE"));
    assert_eq!(p.len(), 0, "the refresh owns the cache, not the older compile");
}
