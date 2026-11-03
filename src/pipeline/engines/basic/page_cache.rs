//! The compiled-page cache: one entry per page, holding the compiled page,
//! the files its bundle inlined, and how to compile it again.
//!
//! A page is named by its identity — the project's template root, its entry
//! file and how it is compiled — and looked up by that identity together with
//! its markup. Two projects whose pages share markup are two pages: their
//! imports resolve to different files.
//!
//! A saved file recompiles the pages built from it before the save answers
//! ([`refresh_template_cache_by_paths`]), so the first request after a change
//! renders the new page instead of compiling it. A deleted or moved file, or
//! a write the cache cannot follow file by file, evicts instead
//! ([`evict_template_cache_by_path`]).

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use crate::language::LanguageEngine;
use crate::pipeline::PipelineError;
use crate::pipeline::nodes::basic::web;
use crate::rwe::{ReactiveWebEngine, ReactiveWebOptions, TemplateSource};

/// A single entry in the template compile cache.
/// Pairs the compiled page artifact with the set of component files it depends on,
/// enabling dependency-aware recompilation when a component is edited.
pub struct CacheEntry {
    /// The compiled page artifact.
    pub page: Arc<web::response::CompiledPage>,
    /// Absolute filesystem paths of all component files inlined during compilation.
    /// Populated from `CompiledTemplate.dependency_paths` (the `visited` set of
    /// `collect_inlined_module`). When any of these paths change on disk,
    /// this entry is recompiled.
    pub dependencies: HashSet<String>,
    /// How this page was compiled, and so how to compile it again.
    pub recipe: Arc<PageRecipe>,
}

/// In-memory compile cache for template nodes.
///
/// Key: hash of the page's identity and its entry markup.
/// Value: compiled page + dependency set for targeted recompilation.
///
/// Uses `RwLock` so concurrent reads (cache hits) never block each other;
/// only writes take an exclusive lock, and no compile runs under it.
pub type TemplateCache = Arc<PageCache>;

/// The pages, readable and writable as the map they are, and the count of
/// changes made to them from outside a request.
#[derive(Default)]
pub struct PageCache {
    pages: RwLock<HashMap<u64, CacheEntry>>,
    /// Bumped as each refresh or eviction starts, so a request's compile that
    /// began before it does not put its older page back afterwards.
    generation: AtomicU64,
}

impl std::ops::Deref for PageCache {
    type Target = RwLock<HashMap<u64, CacheEntry>>;

    fn deref(&self) -> &Self::Target {
        &self.pages
    }
}

/// Create a new empty template compile cache.
pub fn new_template_cache() -> TemplateCache {
    Arc::new(PageCache::default())
}

/// The cache key of one page with one markup.
fn page_key(identity: &str, markup: &str) -> u64 {
    let mut h = std::collections::hash_map::DefaultHasher::new();
    identity.hash(&mut h);
    markup.hash(&mut h);
    h.finish()
}

/// `path` as the compiler records dependencies: canonical when it exists.
fn canonical(path: &Path) -> String {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf()).to_string_lossy().into_owned()
}

/// Whether `path` is `target` or lies inside the folder `target`.
fn is_at_or_under(path: &str, target: &str) -> bool {
    path == target
        || path.strip_prefix(target).is_some_and(|rest| rest.starts_with(std::path::MAIN_SEPARATOR))
}

/// Everything one compile of a page read, so a save can repeat it.
pub struct PageRecipe {
    /// Which page this is. The cache holds at most one entry per identity.
    identity: String,
    /// Canonical path of the entry page, when its markup is that file's
    /// content: saving the file recompiles from what it now holds.
    entry_path: Option<String>,
    node_id: String,
    source: TemplateSource,
    options: ReactiveWebOptions,
    rwe: Arc<dyn ReactiveWebEngine>,
    language: Arc<dyn LanguageEngine>,
}

impl PageRecipe {
    pub fn new(
        identity: String,
        entry_path: Option<&Path>,
        node_id: &str,
        source: TemplateSource,
        options: ReactiveWebOptions,
        rwe: Arc<dyn ReactiveWebEngine>,
        language: Arc<dyn LanguageEngine>,
    ) -> Self {
        Self {
            identity,
            entry_path: entry_path.map(canonical),
            node_id: node_id.to_string(),
            source,
            options,
            rwe,
            language,
        }
    }

    fn key(&self) -> u64 {
        page_key(&self.identity, &self.source.markup)
    }

    fn compile(&self) -> Result<web::response::CompiledPage, PipelineError> {
        web::response::compile_page(
            &self.node_id,
            &self.source,
            &self.options,
            self.rwe.as_ref(),
            self.language.as_ref(),
        )
    }

    /// The same recipe over the entry page as it is on disk now. The request
    /// path trims the markup it reads before hashing it, so this does too.
    fn reread_entry(&self, path: &str) -> std::io::Result<Self> {
        let markup = std::fs::read_to_string(path)?.trim().to_string();
        Ok(Self {
            identity: self.identity.clone(),
            entry_path: self.entry_path.clone(),
            node_id: self.node_id.clone(),
            source: TemplateSource { markup, ..self.source.clone() },
            options: self.options.clone(),
            rwe: self.rwe.clone(),
            language: self.language.clone(),
        })
    }

    fn touches(&self, dependencies: &HashSet<String>, path: &str) -> bool {
        self.entry_path.as_deref().is_some_and(|e| is_at_or_under(e, path))
            || dependencies.iter().any(|d| is_at_or_under(d, path))
    }
}

/// Puts `page` in as the one entry for its identity.
fn insert_page(map: &mut HashMap<u64, CacheEntry>, recipe: Arc<PageRecipe>, page: Arc<web::response::CompiledPage>) {
    map.retain(|_, entry| entry.recipe.identity != recipe.identity);
    map.insert(
        recipe.key(),
        CacheEntry { dependencies: page.template.dependency_paths.clone(), page, recipe },
    );
}

/// The compiled page `identity` names with `markup`: from the cache, or
/// compiled from the recipe `make` builds (only on a miss, so a hit costs one
/// lookup) and cached in place of any older entry for that page. `make` must
/// build a recipe for the same identity and markup.
pub(super) fn compile_cached(
    cache: Option<&TemplateCache>,
    identity: &str,
    markup: &str,
    make: impl FnOnce() -> PageRecipe,
) -> Result<Arc<web::response::CompiledPage>, PipelineError> {
    // Read before anything this compile depends on is read.
    let seen = cache.map(|c| c.generation.load(Ordering::Acquire));
    let key = page_key(identity, markup);
    if let Some(hit) = cache.and_then(|c| {
        c.read().unwrap_or_else(|e| e.into_inner()).get(&key).map(|e| e.page.clone())
    }) {
        return Ok(hit);
    }
    let recipe = make();
    debug_assert_eq!(recipe.key(), key, "the recipe names another page");
    let fresh = Arc::new(recipe.compile()?);
    if let Some(cache) = cache {
        let mut map = cache.write().unwrap_or_else(|e| e.into_inner());
        // A save landed while this compiled: it may have read the old files.
        // Serve it to this request, but leave the cache to the save.
        if Some(cache.generation.load(Ordering::Acquire)) != seen {
            return Ok(fresh);
        }
        insert_page(&mut map, Arc::new(recipe), fresh.clone());
    }
    Ok(fresh)
}

/// Evict every page built from `abs_path`, or from anything inside it when
/// it is a folder: as its entry page or as a file its bundle inlined. For a
/// deleted or moved file, and for writes the cache cannot follow one file at
/// a time (an installed package, a migration); the next request compiles.
///
/// A deleted file no longer canonicalises, so resolve `abs_path` before the
/// delete.
pub fn evict_template_cache_by_path(cache: &TemplateCache, abs_path: &str) {
    let mut map = cache.write().unwrap_or_else(|e| e.into_inner());
    cache.generation.fetch_add(1, Ordering::AcqRel);
    map.retain(|_, entry| !entry.recipe.touches(&entry.dependencies, abs_path));
}

/// Saves recompile one at a time: each compile reads the files as they are
/// when it starts, so the last refresh to run has read the last save.
static REFRESH: Mutex<()> = Mutex::new(());

/// Recompile every cached page built from any of `abs_paths`, as its entry
/// page or as a file its bundle inlined, and swap each in place: once, however
/// many of the paths it uses. Called whenever template or component files are
/// written (UI, MCP, API); it answers once the pages are live, with how many
/// were rebuilt.
///
/// A page that no longer compiles is evicted, so the next request reports
/// the error as before.
pub async fn refresh_template_cache_by_paths(cache: &TemplateCache, abs_paths: Vec<String>) -> usize {
    let shared = cache.clone();
    let paths = abs_paths.clone();
    match tokio::task::spawn_blocking(move || refresh_now(&shared, &paths)).await {
        Ok(rebuilt) => rebuilt,
        // A compile that panicked rebuilt nothing it can vouch for.
        Err(_) => {
            for path in &abs_paths {
                evict_template_cache_by_path(cache, path);
            }
            0
        }
    }
}

/// [`refresh_template_cache_by_paths`] for one file.
pub async fn refresh_template_cache_by_path(cache: &TemplateCache, abs_path: &str) -> usize {
    refresh_template_cache_by_paths(cache, vec![abs_path.to_string()]).await
}

fn refresh_now(cache: &TemplateCache, abs_paths: &[String]) -> usize {
    let _serial = REFRESH.lock().unwrap_or_else(|e| e.into_inner());
    cache.generation.fetch_add(1, Ordering::AcqRel);
    let stale: Vec<(u64, Arc<PageRecipe>)> = cache
        .read()
        .unwrap_or_else(|e| e.into_inner())
        .iter()
        .filter(|(_, entry)| abs_paths.iter().any(|p| entry.recipe.touches(&entry.dependencies, p)))
        .map(|(key, entry)| (*key, entry.recipe.clone()))
        .collect();

    let mut rebuilt = 0;
    for (old_key, recipe) in stale {
        let recipe = match recipe.entry_path.as_deref() {
            Some(entry) if abs_paths.iter().any(|p| p == entry) => match recipe.reread_entry(entry) {
                Ok(fresh) => Arc::new(fresh),
                Err(_) => {
                    cache.write().unwrap_or_else(|e| e.into_inner()).remove(&old_key);
                    continue;
                }
            },
            _ => recipe,
        };
        let compiled = recipe.compile();
        let mut map = cache.write().unwrap_or_else(|e| e.into_inner());
        map.remove(&old_key);
        if let Ok(page) = compiled {
            insert_page(&mut map, recipe, Arc::new(page));
            rebuilt += 1;
        }
    }
    rebuilt
}

/// A project's repository root as the cache records paths, for evicting
/// every page of the project.
pub fn project_pages_root(repo_dir: &Path) -> String {
    canonical(repo_dir)
}

#[cfg(test)]
#[path = "page_cache_tests.rs"]
mod tests;
