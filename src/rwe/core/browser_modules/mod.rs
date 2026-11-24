//! Browser modules (`kinds/rwe-source` § Browser modules): a project folder
//! declared in `zebflow.yaml` whose files are real ES modules in the browser
//! instead of being pasted into each page's script.
//!
//! ```yaml
//! rwe:
//!   browser_modules:
//!     - path: modules/game
//!       entries: [main.ts, objects.ts, "islands/*.ts"]
//! ```
//!
//! Each entry is built, with every file it imports, into one script; code two
//! entries share goes into one shared script; colliding names are renamed.
//! Files that are not entries are internal: a page imports entries only. The
//! scripts are addressed by content hash and served where the page's own
//! scripts are (`_rwe/scripts/<hash>`), and they import each other and the
//! `zeb/*` libraries by relative address, so they are the same bytes on every
//! host. They never run on the server: a page's import of an entry is inert
//! there, as a project library's is.
//!
//! A project that declares no folder never reaches this module.

mod bundle;
mod load;

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex};

use serde::{Deserialize, Serialize};

use super::config::SecurityPolicy;
use super::error::EngineError;

/// One declared folder.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct ModuleFolder {
    /// Relative to the project's source root, e.g. `modules/game`.
    pub path: String,
    /// Patterns relative to the folder: `main.ts`, `islands/*.ts`.
    pub entries: Vec<String>,
}

/// What a page's runtime-library map carries for an entry: this prefix, then
/// the script's hash. `render` turns it into an import of `./<hash>`.
pub const URL_PREFIX: &str = "zeb-module:";

/// One built script.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModuleScript {
    /// What it holds, for logs: `modules/game/main.ts`, or `shared` for a shared script.
    pub label: String,
    /// JavaScript.
    pub content: String,
    /// `sha256` hex of `content`: its address.
    pub hash: String,
}

/// The built folders of one project.
#[derive(Debug, Default)]
pub struct Build {
    /// Every way a page may name an entry (`@/modules/game/main`,
    /// `@/modules/game/main.ts`) → the hash of the script a page imports.
    pub entries: BTreeMap<String, String>,
    /// Every script, by hash.
    pub scripts: BTreeMap<String, ModuleScript>,
    /// The scripts each script loads, statically or lazily.
    pub loads: BTreeMap<String, BTreeSet<String>>,
    /// Every file of every declared folder, absolute: a page that imports an
    /// entry is compiled again when one of them is saved.
    pub files: Vec<String>,
    /// The declared folders, absolute, with a trailing `/`.
    pub folders: Vec<String>,
    /// Each folder's entries as a page imports them (`@/modules/game/main`).
    pub folder_entries: BTreeMap<String, Vec<String>>,
    /// Each file (absolute) → the entries whose scripts hold it, relative to
    /// the source root (`modules/game/objects.ts`).
    pub bundled_into: BTreeMap<String, Vec<String>>,
    /// Entries (absolute) another file loads with `import()` or a lazy glob.
    pub lazy: BTreeSet<String>,
}

/// What a row of a file listing is to the browser modules of its project:
/// the field Studio colours by and an agent reads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "role", rename_all = "snake_case")]
pub enum ModuleRole {
    /// The declared folder itself.
    Folder { entries: Vec<String> },
    /// A file a page may import; one script of its own.
    Entry {
        folder: String,
        /// Loaded on demand by another file of the folder.
        #[serde(default)]
        lazy: bool,
    },
    /// Any other file or folder inside: bundled into the scripts of the entries
    /// that import it (known once the folder has been built).
    Internal {
        folder: String,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        bundled_into: Vec<String>,
    },
}

/// The role of `rel` (relative to the source root) among `folders`, with the
/// folder's last build for what only a build knows. `None` outside every folder.
pub fn role_of(folders: &[ModuleFolder], source_root: &Path, rel: &str, is_dir: bool) -> Option<ModuleRole> {
    let rel = rel.trim_matches('/');
    let folder = folders.iter().find(|folder| {
        let path = folder.path.trim_matches('/');
        rel == path || rel.starts_with(&format!("{path}/"))
    })?;
    let path = folder.path.trim_matches('/').to_string();
    if rel == path {
        return Some(ModuleRole::Folder { entries: folder.entries.clone() });
    }
    let inside = &rel[path.len() + 1..];
    let latest = latest(source_root, folders);
    let abs = std::fs::canonicalize(source_root.join(rel)).map(|p| p.display().to_string()).unwrap_or_default();
    if !is_dir && folder.entries.iter().any(|pattern| glob_matches(pattern, inside)) {
        let lazy = latest.as_ref().is_some_and(|build| build.lazy.contains(&abs));
        return Some(ModuleRole::Entry { folder: path, lazy });
    }
    let bundled_into = latest.as_ref().and_then(|build| build.bundled_into.get(&abs).cloned()).unwrap_or_default();
    Some(ModuleRole::Internal { folder: path, bundled_into })
}

/// The newest build of these folders under this root, whatever libraries it
/// was built with; `None` before any page has imported an entry.
fn latest(source_root: &Path, folders: &[ModuleFolder]) -> Option<Arc<Build>> {
    let root = std::fs::canonicalize(source_root).ok()?;
    let prefix = format!("{}|{:?}|", root.display(), folders);
    BUILDS.lock().unwrap_or_else(|e| e.into_inner()).iter().find(|(key, _)| key.starts_with(&prefix)).map(|(_, (_, build))| build.clone())
}

impl Build {
    /// The scripts a page needs for the entries it imports: each entry's
    /// script and everything it loads, transitively.
    pub fn scripts_for(&self, entries: &[String]) -> Vec<ModuleScript> {
        let mut seen = BTreeSet::new();
        let mut stack: Vec<String> = entries.iter().filter_map(|entry| self.entries.get(entry).cloned()).collect();
        while let Some(hash) = stack.pop() {
            if !seen.insert(hash.clone()) {
                continue;
            }
            stack.extend(self.loads.get(&hash).into_iter().flatten().cloned());
        }
        seen.into_iter().filter_map(|hash| self.scripts.get(&hash).cloned()).collect()
    }

    /// The declared folder an absolute path is inside, if any.
    pub fn folder_of(&self, path: &str) -> Option<&str> {
        self.folders.iter().find(|folder| path.starts_with(folder.as_str())).map(String::as_str)
    }

}

thread_local! {
    /// The build the page compile running on this thread uses: how a
    /// component's import of an entry is left alone and an import of an
    /// internal file is refused, wherever in the page it is written.
    static ACTIVE: std::cell::RefCell<Option<Arc<Build>>> = const { std::cell::RefCell::new(None) };
}

/// Holds a build as the active one for one page compile; cleared on drop.
pub(crate) struct Guard;

impl Guard {
    pub(crate) fn begin(build: Arc<Build>) -> Self {
        ACTIVE.with(|cell| *cell.borrow_mut() = Some(build));
        Guard
    }
}

impl Drop for Guard {
    fn drop(&mut self) {
        ACTIVE.with(|cell| *cell.borrow_mut() = None);
    }
}

/// Whether `spec` names an entry of the active build.
pub(crate) fn is_entry(spec: &str) -> bool {
    ACTIVE.with(|cell| cell.borrow().as_ref().is_some_and(|build| build.entries.contains_key(spec)))
}

/// Refuses page code importing a file inside a browser module folder that is
/// not one of its entries: that file is internal to the folder.
pub(crate) fn refuse_internal(spec: &str, resolved: &str) -> Result<(), EngineError> {
    ACTIVE.with(|cell| {
        let active = cell.borrow();
        let Some(build) = active.as_ref() else { return Ok(()) };
        let resolved = std::fs::canonicalize(resolved).map(|p| p.display().to_string()).unwrap_or_else(|_| resolved.to_string());
        let Some(folder) = build.folder_of(&resolved) else { return Ok(()) };
        let named = build.folder_entries.get(folder).cloned().unwrap_or_default();
        Err(EngineError::new(
            "RWE_IMPORT_MODULE_INTERNAL",
            format!(
                "'{spec}' is inside a browser module folder and is not one of its entries, so page code cannot import it; \
                 import an entry ({}), or add this file to the folder's entries in zebflow.yaml",
                named.join(", ")
            ),
        ))
    })
}

/// What a page's cache identity gains from the folders: nothing when there
/// are none, so every other project's pages keep the identity they had; a
/// digest of the setting otherwise, so changing it compiles the page again.
pub fn identity(folders: &[ModuleFolder]) -> String {
    if folders.is_empty() {
        return String::new();
    }
    use sha2::{Digest, Sha256};
    let text = serde_json::to_string(folders).unwrap_or_default();
    format!("|modules:{}", &hex::encode(Sha256::digest(text.as_bytes()))[..16])
}

/// Builds the declared folders under `template_root`, or answers the last
/// build when no file and no setting changed.
pub fn build(
    template_root: &Path,
    folders: &[ModuleFolder],
    runtime_libraries: &BTreeMap<String, String>,
    security: &SecurityPolicy,
) -> Result<Arc<Build>, EngineError> {
    let root = std::fs::canonicalize(template_root)
        .map_err(|err| EngineError::new("RWE_MODULE_FOLDER", format!("the source root cannot be read: {err}")))?;
    let mut declared = Vec::new();
    for folder in folders {
        let rel = folder.path.trim_matches('/');
        if rel.is_empty() || rel.split('/').any(|part| part.is_empty() || part == "." || part == "..") {
            return Err(EngineError::new("RWE_MODULE_FOLDER", format!("browser module folder '{}' must be a plain path inside the project", folder.path)));
        }
        let abs = std::fs::canonicalize(root.join(rel)).map_err(|_| {
            EngineError::new("RWE_MODULE_FOLDER", format!("browser module folder '{rel}' does not exist"))
        })?;
        if !abs.starts_with(&root) || !abs.is_dir() {
            return Err(EngineError::new("RWE_MODULE_FOLDER", format!("browser module folder '{rel}' is not a folder inside the project")));
        }
        if folder.entries.is_empty() {
            return Err(EngineError::new("RWE_MODULE_FOLDER", format!("browser module folder '{rel}' names no entries")));
        }
        declared.push((rel.to_string(), abs, folder.entries.clone()));
    }

    // Every file of every folder, with what identifies its bytes on disk. A
    // build is answered from the cache while this list is the same.
    let mut files = Vec::new();
    for (_, abs, _) in &declared {
        walk(abs, &mut files);
    }
    files.sort();
    files.dedup();
    let key = format!("{}|{:?}|{:?}", root.display(), folders, runtime_libraries);
    let stamp = stamp_of(&files);
    if let Some((known, build)) = BUILDS.lock().unwrap_or_else(|e| e.into_inner()).get(&key)
        && *known == stamp
    {
        return Ok(build.clone());
    }

    let built = Arc::new(bundle::build(&root, &declared, &files, runtime_libraries, security)?);
    BUILDS.lock().unwrap_or_else(|e| e.into_inner()).insert(key, (stamp, built.clone()));
    Ok(built)
}

/// The last build of each project's folders, with the stamp of the files it read.
static BUILDS: LazyLock<Mutex<HashMap<String, (u64, Arc<Build>)>>> = LazyLock::new(Default::default);

/// Analyses by file bytes: an unchanged file is never compiled twice.
static ANALYSES: LazyLock<Mutex<HashMap<(String, String), Arc<load::Analysis>>>> = LazyLock::new(Default::default);

/// One file's analysis, checked against `security`, from the cache when its
/// bytes and the policy are unchanged: a rebuild after one save reads every
/// file again but compiles and checks only the one that changed.
fn analysis_cache(path: &str, kind: load::Kind, source: &str, security: &SecurityPolicy) -> Result<Arc<load::Analysis>, EngineError> {
    use sha2::{Digest, Sha256};
    let policy = serde_json::to_string(security).unwrap_or_default();
    let digest = hex::encode(Sha256::digest(format!("{policy}\u{0}{source}").as_bytes()));
    let key = (format!("{path}|{kind:?}"), digest);
    if let Some(found) = ANALYSES.lock().unwrap_or_else(|e| e.into_inner()).get(&key) {
        return Ok(found.clone());
    }
    let analysis = load::analyze(path, kind, source)?;
    if analysis.kind == load::Kind::Script {
        super::security::analyze(&analysis.js, security).map_err(|err| EngineError::new("RWE_MODULE_BUILD", format!("'{path}': {}", err.message)))?;
    }
    let analysis = Arc::new(analysis);
    let mut cache = ANALYSES.lock().unwrap_or_else(|e| e.into_inner());
    // Old versions of a file are dropped when it changes: one entry per file and kind.
    cache.retain(|(known, _), _| known != &key.0);
    cache.insert(key, analysis.clone());
    Ok(analysis)
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        if path.is_dir() {
            walk(&path, out);
        } else {
            out.push(path);
        }
    }
}

/// The size and modification time of every file, hashed.
fn stamp_of(files: &[PathBuf]) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    for file in files {
        file.hash(&mut hasher);
        if let Ok(meta) = std::fs::metadata(file) {
            meta.len().hash(&mut hasher);
            meta.modified().ok().hash(&mut hasher);
        }
    }
    hasher.finish()
}

/// Whether `path` (relative, `/`-separated) matches `pattern`: `*` is any
/// run of characters inside one folder, `?` one character, `**` any number
/// of folders.
pub(crate) fn glob_matches(pattern: &str, path: &str) -> bool {
    fn segments(pattern: &[&str], path: &[&str]) -> bool {
        match (pattern.first(), path.first()) {
            (None, None) => true,
            (Some(&"**"), _) => segments(&pattern[1..], path) || (!path.is_empty() && segments(pattern, &path[1..])),
            (Some(p), Some(s)) => segment(p.as_bytes(), s.as_bytes()) && segments(&pattern[1..], &path[1..]),
            _ => false,
        }
    }
    fn segment(pattern: &[u8], text: &[u8]) -> bool {
        match (pattern.first(), text.first()) {
            (None, None) => true,
            (Some(b'*'), _) => segment(&pattern[1..], text) || (!text.is_empty() && segment(pattern, &text[1..])),
            (Some(b'?'), Some(_)) => segment(&pattern[1..], &text[1..]),
            (Some(p), Some(t)) if p == t => segment(&pattern[1..], &text[1..]),
            _ => false,
        }
    }
    let pattern: Vec<&str> = pattern.split('/').filter(|s| !s.is_empty() && *s != ".").collect();
    let path: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    segments(&pattern, &path)
}

#[cfg(test)]
mod tests;
