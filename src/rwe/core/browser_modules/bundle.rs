//! The build itself: files → module graph → one script per entry (plus
//! shared scripts) → JavaScript text, each script named by its content hash.
//!
//! The graph is read from the entries. A file reached statically from the
//! same set of entries goes into the same script, so code two entries share
//! is stored once. A script's static imports only ever point at scripts
//! holding files reached from more entries, so they cannot form a cycle; a
//! lazy `import()` that would close one is refused by name.
//!
//! Inside a script every file keeps its own names: a top-level name is kept
//! when no other file of the script uses it in any scope, and is otherwise
//! renamed to one nobody uses (`build$1`). An import becomes a direct
//! reference to the binding it names, so nothing is copied.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use super::super::config::SecurityPolicy;
use super::super::error::EngineError;
use super::load::{Analysis, Edit, ExportTarget, Imported, Kind, DEFAULT_LOCAL};
use super::{Build, ModuleScript, glob_matches};

/// Stands in a script's text for the address of an entry it loads lazily,
/// until that entry's script has a hash.
const LAZY: &str = "@@zeb-module-entry:";

fn error(message: String) -> EngineError {
    EngineError::new("RWE_MODULE_BUILD", message)
}

/// Where a specifier leads.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Target {
    Module(usize),
    /// A library, by its address relative to the scripts (`../lib/…`).
    External(String),
}

/// What a name finally is.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum Binding {
    Local(usize, String),
    Namespace(usize),
    External(String, String),
    ExternalNamespace(String),
}

struct Node {
    path: String,
    rel: String,
    analysis: Arc<Analysis>,
    resolved: HashMap<String, Target>,
    statics: Vec<usize>,
    /// Per glob call of the file: `(key as written, module)`.
    globs: Vec<Vec<(String, usize)>>,
    entries: BTreeSet<usize>,
    chunk: usize,
}

struct Entry {
    module: usize,
    rel: String,
    folder: usize,
}

#[derive(Default)]
struct Chunk {
    modules: Vec<usize>,
    entries: Vec<usize>,
    /// All names any file of the chunk declares or reads.
    used: HashSet<String>,
    /// How many files of the chunk declare or read each name.
    count: HashMap<String, usize>,
    taken: HashSet<String>,
    externals: BTreeMap<(String, String), String>,
    external_ns: BTreeMap<String, String>,
    external_bare: BTreeSet<String>,
    cross: BTreeMap<(usize, String), String>,
    bare: BTreeSet<usize>,
    exports: BTreeMap<String, String>,
    export_of: HashMap<String, String>,
    namespaces: BTreeMap<usize, String>,
    namespace_text: BTreeMap<usize, String>,
    lazy: BTreeSet<usize>,
    bodies: Vec<String>,
    public: Vec<(String, String)>,
    public_stars: Vec<String>,
}

impl Chunk {
    fn fresh(&mut self, base: &str) -> String {
        let base: String = base.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' || c == '$' { c } else { '_' }).collect();
        let base = if base.is_empty() || base.starts_with(|c: char| c.is_ascii_digit()) { format!("_{base}") } else { base };
        for n in 1.. {
            let name = format!("{base}${n}");
            if !self.used.contains(&name) && !self.taken.contains(&name) {
                self.taken.insert(name.clone());
                return name;
            }
        }
        unreachable!()
    }
}

pub(super) fn build(
    root: &Path,
    declared: &[(String, PathBuf, Vec<String>)],
    files: &[PathBuf],
    runtime: &BTreeMap<String, String>,
    security: &SecurityPolicy,
) -> Result<Build, EngineError> {
    let folders: Vec<String> = declared.iter().map(|(_, abs, _)| format!("{}/", abs.display())).collect();
    let rel_of = |path: &Path| path.strip_prefix(root).map(|p| p.to_string_lossy().replace('\\', "/")).unwrap_or_else(|_| path.display().to_string());
    let mut graph = Graph { root: root.to_path_buf(), folders: folders.clone(), runtime, files, nodes: Vec::new(), index: HashMap::new(), security: SecurityPolicy { allow_dynamic_import: true, ..security.clone() } };

    // Entries, in the order the setting lists them.
    let mut entries: Vec<Entry> = Vec::new();
    for (folder_index, (rel, abs, patterns)) in declared.iter().enumerate() {
        for pattern in patterns {
            let mut matched: Vec<&PathBuf> = files
                .iter()
                .filter(|file| file.starts_with(abs) && is_script(file))
                .filter(|file| glob_matches(pattern, &file.strip_prefix(abs).map(|p| p.to_string_lossy().replace('\\', "/")).unwrap_or_default()))
                .collect();
            matched.sort();
            if matched.is_empty() {
                return Err(error(format!("entry '{pattern}' of browser module folder '{rel}' matches no .ts or .js file")));
            }
            for file in matched {
                let module = graph.load(file, Kind::Script)?;
                if !entries.iter().any(|entry| entry.module == module) {
                    entries.push(Entry { module, rel: rel_of(file), folder: folder_index });
                }
            }
        }
    }

    // The whole graph, from the entries.
    let mut queue: Vec<usize> = entries.iter().map(|entry| entry.module).collect();
    let mut done = HashSet::new();
    while let Some(module) = queue.pop() {
        if !done.insert(module) {
            continue;
        }
        let analysis = graph.nodes[module].analysis.clone();
        let mut specs: Vec<String> = analysis.imports.iter().map(|import| import.spec.clone()).collect();
        for target in analysis.exports.values() {
            if let ExportTarget::Reexport { spec, .. } | ExportTarget::Namespace { spec } = target {
                specs.push(spec.clone());
            }
        }
        specs.extend(analysis.stars.iter().cloned());
        specs.extend(analysis.dynamic.iter().map(|dynamic| dynamic.spec.clone()));
        specs.extend(analysis.workers.iter().map(|worker| worker.spec.clone()));
        let mut statics = Vec::new();
        for spec in &specs {
            let target = graph.resolve(module, spec)?;
            if let Target::Module(next) = target {
                queue.push(next);
                let loaded_apart = analysis.dynamic.iter().chain(&analysis.workers).any(|other| &other.spec == spec);
                if !loaded_apart || analysis.imports.iter().any(|import| &import.spec == spec) {
                    statics.push(next);
                }
            }
            graph.nodes[module].resolved.insert(spec.clone(), target);
        }
        let mut globs = Vec::new();
        for glob in &analysis.globs {
            let matched = graph.glob(module, &glob.patterns)?;
            for (_, next) in &matched {
                queue.push(*next);
                if glob.eager {
                    statics.push(*next);
                }
            }
            globs.push(matched);
        }
        statics.sort();
        statics.dedup();
        graph.nodes[module].statics = statics;
        graph.nodes[module].globs = globs;
    }

    // A lazy load names an entry: only entries are scripts a page or another
    // script can load by themselves.
    let entry_of: HashMap<usize, usize> = entries.iter().enumerate().map(|(index, entry)| (entry.module, index)).collect();
    let mut lazy: BTreeSet<String> = BTreeSet::new();
    for node in &graph.nodes {
        for dynamic in node.analysis.dynamic.iter().chain(&node.analysis.workers) {
            if let Some(Target::Module(target)) = node.resolved.get(&dynamic.spec) {
                lazy.insert(graph.nodes[*target].path.clone());
            }
        }
        for (glob, matched) in node.analysis.globs.iter().zip(&node.globs) {
            if !glob.eager {
                lazy.extend(matched.iter().map(|(_, target)| graph.nodes[*target].path.clone()));
            }
        }
    }
    for node in &graph.nodes {
        for dynamic in &node.analysis.dynamic {
            if let Some(Target::Module(target)) = node.resolved.get(&dynamic.spec)
                && !entry_of.contains_key(target)
            {
                return Err(error(format!(
                    "'{}' loads '{}' with import(), but it is not an entry; add it to the folder's entries so it becomes a script of its own",
                    node.rel, dynamic.spec
                )));
            }
        }
        for worker in &node.analysis.workers {
            if let Some(Target::Module(target)) = node.resolved.get(&worker.spec)
                && !entry_of.contains_key(target)
            {
                return Err(error(format!(
                    "'{}' starts a worker from '{}', but it is not an entry; add it to the folder's entries so it becomes a script of its own",
                    node.rel, worker.spec
                )));
            }
        }
        for (glob, matched) in node.analysis.globs.iter().zip(&node.globs) {
            if glob.eager {
                continue;
            }
            if let Some((key, _)) = matched.iter().find(|(_, target)| !entry_of.contains_key(target)) {
                return Err(error(format!(
                    "'{}' loads '{key}' lazily through import.meta.glob, but it is not an entry; add it to the entries, or pass {{ eager: true }}",
                    node.rel
                )));
            }
        }
    }

    // Which entries reach each file statically, then one chunk per set.
    for (index, entry) in entries.iter().enumerate() {
        let mut stack = vec![entry.module];
        while let Some(module) = stack.pop() {
            if graph.nodes[module].entries.insert(index) {
                stack.extend(graph.nodes[module].statics.iter().copied());
            }
        }
    }
    let mut chunks: Vec<Chunk> = Vec::new();
    let mut chunk_of_set: BTreeMap<BTreeSet<usize>, usize> = BTreeMap::new();
    let mut order: Vec<usize> = (0..graph.nodes.len()).collect();
    order.sort_by(|a, b| graph.nodes[*a].path.cmp(&graph.nodes[*b].path));
    for module in order {
        let set = graph.nodes[module].entries.clone();
        if set.is_empty() {
            continue;
        }
        let next = chunks.len();
        let chunk = *chunk_of_set.entry(set).or_insert(next);
        if chunk == chunks.len() {
            chunks.push(Chunk::default());
        }
        graph.nodes[module].chunk = chunk;
    }
    // Files in the order they run: what a file imports runs before it.
    for chunk in 0..chunks.len() {
        let mut members: Vec<usize> = (0..graph.nodes.len()).filter(|m| !graph.nodes[*m].entries.is_empty() && graph.nodes[*m].chunk == chunk).collect();
        members.sort_by(|a, b| graph.nodes[*a].path.cmp(&graph.nodes[*b].path));
        let mut seen = HashSet::new();
        let mut ordered = Vec::new();
        for module in &members {
            post_order(&graph, *module, chunk, &mut seen, &mut ordered);
        }
        chunks[chunk].modules = ordered;
    }
    for (index, entry) in entries.iter().enumerate() {
        chunks[graph.nodes[entry.module].chunk].entries.push(index);
    }

    // Names: a file's top-level name stays when it is unique in the chunk.
    let mut names: HashMap<(usize, String), String> = HashMap::new();
    for chunk in chunks.iter_mut() {
        for module in &chunk.modules {
            for name in &graph.nodes[*module].analysis.names {
                *chunk.count.entry(name.clone()).or_default() += 1;
                chunk.used.insert(name.clone());
            }
        }
        for module in chunk.modules.clone() {
            let node = &graph.nodes[module];
            for name in node.analysis.top.keys() {
                let kept = name != DEFAULT_LOCAL && chunk.count.get(name) == Some(&1) && !chunk.taken.contains(name);
                let chosen = if kept {
                    chunk.taken.insert(name.clone());
                    name.clone()
                } else if name == DEFAULT_LOCAL {
                    chunk.fresh(&format!("{}_default", stem(&node.path)))
                } else {
                    chunk.fresh(name)
                };
                names.insert((module, name.clone()), chosen);
            }
        }
    }

    let mut ctx = Emit { graph: &graph, names: &names, chunks, facades: Vec::new() };

    // Each entry's public names, then each file's text.
    let entry_count = entries.len();
    let mut publics: Vec<(Vec<(String, Binding)>, Vec<String>)> = Vec::new();
    for entry in &entries {
        let (exported, stars) = ctx.exported_names(entry.module, &mut HashSet::new())?;
        let mut bindings = Vec::new();
        for name in exported {
            bindings.push((name.clone(), ctx.resolve_export(entry.module, &name, 0)?));
        }
        publics.push((bindings, stars));
    }
    for chunk in 0..ctx.chunks.len() {
        for module in ctx.chunks[chunk].modules.clone() {
            let body = ctx.body(chunk, module, &entry_of)?;
            ctx.chunks[chunk].bodies.push(body);
        }
    }
    for (index, (bindings, stars)) in publics.into_iter().enumerate() {
        let chunk = graph.nodes[entries[index].module].chunk;
        let single = ctx.chunks[chunk].entries.len() == 1;
        let mut public = Vec::new();
        for (name, binding) in bindings {
            let local = ctx.local_for(chunk, &binding)?;
            let exported = if single { local } else { ctx.export_from(chunk, &local) };
            public.push((name, exported));
        }
        if single {
            ctx.chunks[chunk].public = public.clone();
            ctx.chunks[chunk].public_stars = stars.clone();
        }
        ctx.facades.push((chunk, public, stars, single));
    }
    // Namespace objects ask for more bindings, which may ask for more namespaces.
    loop {
        let mut pending = Vec::new();
        for (chunk, data) in ctx.chunks.iter().enumerate() {
            for module in data.namespaces.keys() {
                if !data.namespace_text.contains_key(module) {
                    pending.push((chunk, *module));
                }
            }
        }
        if pending.is_empty() {
            break;
        }
        for (chunk, module) in pending {
            let text = ctx.namespace_object(chunk, module)?;
            ctx.chunks[chunk].namespace_text.insert(module, text);
        }
    }

    let mut build = ctx.assemble(&entries, entry_count, &folders, files)?;
    for node in &graph.nodes {
        let holders: Vec<String> = node.entries.iter().map(|entry| entries[*entry].rel.clone()).collect();
        let file = node.path.trim_end_matches("?raw").to_string();
        let known = build.bundled_into.entry(file).or_default();
        for holder in holders {
            if !known.contains(&holder) {
                known.push(holder);
            }
        }
    }
    build.lazy = lazy;
    Ok(build)
}

fn stem(path: &str) -> String {
    Path::new(path.trim_end_matches("?raw")).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "module".to_string())
}

fn is_script(path: &Path) -> bool {
    let name = path.to_string_lossy();
    (name.ends_with(".ts") || name.ends_with(".mts") || name.ends_with(".js") || name.ends_with(".mjs")) && !name.ends_with(".d.ts")
}

fn post_order(graph: &Graph, module: usize, chunk: usize, seen: &mut HashSet<usize>, out: &mut Vec<usize>) {
    if graph.nodes[module].chunk != chunk || graph.nodes[module].entries.is_empty() || !seen.insert(module) {
        return;
    }
    for next in &graph.nodes[module].statics {
        post_order(graph, *next, chunk, seen, out);
    }
    out.push(module);
}

fn normalize(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other),
        }
    }
    out
}

/// `to` as written from `from_dir`: `./objects/tree.ts`, `../shared/x.ts`.
fn relative(from_dir: &Path, to: &Path) -> String {
    let from: Vec<_> = from_dir.components().collect();
    let target: Vec<_> = to.components().collect();
    let common = from.iter().zip(&target).take_while(|(a, b)| a == b).count();
    let mut parts: Vec<String> = Vec::new();
    for _ in common..from.len() {
        parts.push("..".to_string());
    }
    for component in &target[common..] {
        parts.push(component.as_os_str().to_string_lossy().to_string());
    }
    let joined = parts.join("/");
    if joined.starts_with("..") { joined } else { format!("./{joined}") }
}

struct Graph<'a> {
    root: PathBuf,
    folders: Vec<String>,
    runtime: &'a BTreeMap<String, String>,
    files: &'a [PathBuf],
    nodes: Vec<Node>,
    index: HashMap<String, usize>,
    security: SecurityPolicy,
}

impl Graph<'_> {
    fn load(&mut self, path: &Path, kind: Kind) -> Result<usize, EngineError> {
        let mut key = path.display().to_string();
        if kind == Kind::Raw {
            key.push_str("?raw");
        }
        if let Some(found) = self.index.get(&key) {
            return Ok(*found);
        }
        let rel = path.strip_prefix(&self.root).map(|p| p.to_string_lossy().replace('\\', "/")).unwrap_or_else(|_| key.clone());
        let source = std::fs::read_to_string(path).map_err(|err| error(format!("'{rel}' cannot be read: {err}")))?;
        let analysis = super::analysis_cache(&key, kind.clone(), &source, &self.security)
            .map_err(|err| error(err.message.replace(&key, &rel)))?;
        let index = self.nodes.len();
        self.nodes.push(Node { path: key.clone(), rel, analysis, resolved: HashMap::new(), statics: Vec::new(), globs: Vec::new(), entries: BTreeSet::new(), chunk: 0 });
        self.index.insert(key, index);
        Ok(index)
    }

    fn rel(&self, module: usize) -> &str {
        &self.nodes[module].rel
    }

    fn inside_folders(&self, path: &Path) -> bool {
        let text = format!("{}", path.display());
        self.folders.iter().any(|folder| text.starts_with(folder.as_str()))
    }

    fn resolve(&mut self, module: usize, spec: &str) -> Result<Target, EngineError> {
        let from = self.rel(module).to_string();
        if let Some(url) = library_address(spec, self.runtime) {
            return Ok(Target::External(url));
        }
        if spec.starts_with("zeb/") {
            return Err(error(format!("'{from}' imports '{spec}', which is not a zeb/* library a browser module can load")));
        }
        let (bare, raw) = match spec.strip_suffix("?raw") {
            Some(bare) => (bare, true),
            None => (spec, false),
        };
        let base = if let Some(rest) = bare.strip_prefix("@/") {
            self.root.join(rest)
        } else if bare.starts_with("./") || bare.starts_with("../") {
            let file = PathBuf::from(self.nodes[module].path.trim_end_matches("?raw"));
            file.parent().unwrap_or(Path::new("/")).join(bare)
        } else {
            return Err(error(format!(
                "'{from}' imports '{spec}'; a browser module imports files of browser module folders (./…, ../…, @/…), zeb/* libraries and project library entries"
            )));
        };
        let base = normalize(&base);
        let found = if raw { base.is_file().then(|| base.clone()) } else { find_file(&base) };
        let Some(found) = found else {
            return Err(error(format!("'{from}' imports '{spec}', which does not exist")));
        };
        let found = std::fs::canonicalize(&found).unwrap_or(found);
        if !self.inside_folders(&found) {
            return Err(error(format!(
                "'{from}' imports '{spec}', which is outside every browser module folder; a browser module only imports files of browser module folders"
            )));
        }
        let kind = if raw {
            Kind::Raw
        } else if found.extension().is_some_and(|ext| ext == "json") {
            Kind::Json
        } else if is_script(&found) {
            Kind::Script
        } else {
            return Err(error(format!("'{from}' imports '{spec}'; to import a file's text, add ?raw (\"{spec}?raw\")")));
        };
        Ok(Target::Module(self.load(&found, kind)?))
    }

    fn glob(&mut self, module: usize, patterns: &[String]) -> Result<Vec<(String, usize)>, EngineError> {
        let file = PathBuf::from(self.nodes[module].path.trim_end_matches("?raw"));
        let dir = file.parent().unwrap_or(Path::new("/")).to_path_buf();
        let mut out = Vec::new();
        for pattern in patterns {
            let absolute = if let Some(rest) = pattern.strip_prefix("@/") {
                self.root.join(rest)
            } else if pattern.starts_with("./") || pattern.starts_with("../") {
                dir.join(pattern)
            } else {
                return Err(error(format!("'{}': import.meta.glob pattern '{pattern}' starts with ./, ../ or @/", self.rel(module))));
            };
            let absolute = normalize(&absolute);
            let Ok(pattern_rel) = absolute.strip_prefix(&self.root) else { continue };
            let pattern_rel = pattern_rel.to_string_lossy().replace('\\', "/");
            let mut matched: Vec<PathBuf> = self
                .files
                .iter()
                .filter(|candidate| is_script(candidate) || candidate.extension().is_some_and(|ext| ext == "json"))
                .filter(|candidate| candidate.as_path() != file.as_path())
                .filter(|candidate| {
                    candidate.strip_prefix(&self.root).is_ok_and(|rel| glob_matches(&pattern_rel, &rel.to_string_lossy().replace('\\', "/")))
                })
                .cloned()
                .collect();
            matched.sort();
            for found in matched {
                let key = if pattern.starts_with("@/") {
                    format!("@/{}", found.strip_prefix(&self.root).map(|p| p.to_string_lossy().replace('\\', "/")).unwrap_or_default())
                } else {
                    relative(&dir, &found)
                };
                let kind = if found.extension().is_some_and(|ext| ext == "json") { Kind::Json } else { Kind::Script };
                let target = self.load(&found, kind)?;
                if !out.iter().any(|(known, _)| known == &key) {
                    out.push((key, target));
                }
            }
        }
        Ok(out)
    }
}

fn find_file(base: &Path) -> Option<PathBuf> {
    if base.is_file() {
        return Some(base.to_path_buf());
    }
    for ext in ["ts", "mts", "js", "mjs", "json"] {
        let candidate = PathBuf::from(format!("{}.{ext}", base.display()));
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    for index in ["index.ts", "index.js", "index.mjs"] {
        let candidate = base.join(index);
        if candidate.is_file() {
            return Some(candidate);
        }
    }
    None
}

/// A library a browser module loads, by its address relative to the module
/// scripts. Libraries are served from `_rwe/lib/`, module scripts from
/// `_rwe/scripts/`, beside each other on every host, so this is the address
/// the page's own import of the same library resolves to: one instance.
fn library_address(spec: &str, runtime: &BTreeMap<String, String>) -> Option<String> {
    let url = runtime
        .get(spec)
        .filter(|url| !url.starts_with(super::URL_PREFIX))
        .map(String::as_str)
        .or_else(|| super::super::render::zeb_bundle_url(spec))?;
    Some(format!("../lib/{}", url.trim_start_matches("/assets/libraries/")))
}

struct Emit<'a> {
    graph: &'a Graph<'a>,
    names: &'a HashMap<(usize, String), String>,
    chunks: Vec<Chunk>,
    facades: Vec<Facade>,
}

/// An entry's public names: `(chunk, (public name, name in the chunk), external stars, whether the chunk is the entry's own)`.
type Facade = (usize, Vec<(String, String)>, Vec<String>, bool);

impl Emit<'_> {
    fn target(&self, module: usize, spec: &str) -> Result<Target, EngineError> {
        self.graph.nodes[module]
            .resolved
            .get(spec)
            .cloned()
            .ok_or_else(|| error(format!("'{}': '{spec}' was not resolved", self.graph.rel(module))))
    }

    fn resolve_export(&self, module: usize, name: &str, depth: usize) -> Result<Binding, EngineError> {
        if depth > 64 {
            return Err(error(format!("'{}': export '{name}' goes round in a circle", self.graph.rel(module))));
        }
        let analysis = &self.graph.nodes[module].analysis;
        if let Some(target) = analysis.exports.get(name) {
            return match target {
                ExportTarget::Local(local) => {
                    if analysis.import_refs.contains_key(local) || analysis.imports.iter().any(|i| i.bindings.iter().any(|(l, _)| l == local)) {
                        self.resolve_import(module, local, depth + 1)
                    } else {
                        Ok(Binding::Local(module, local.clone()))
                    }
                }
                ExportTarget::Reexport { spec, imported } => self.resolve_imported(module, spec, imported, depth + 1),
                ExportTarget::Namespace { spec } => match self.target(module, spec)? {
                    Target::Module(next) => Ok(Binding::Namespace(next)),
                    Target::External(url) => Ok(Binding::ExternalNamespace(url)),
                },
            };
        }
        if name != "default" {
            let mut external = None;
            for spec in &analysis.stars {
                match self.target(module, spec)? {
                    Target::Module(next) => {
                        if let Ok(found) = self.resolve_export(next, name, depth + 1) {
                            return Ok(found);
                        }
                    }
                    Target::External(url) => external = Some(url),
                }
            }
            if let Some(url) = external {
                return Ok(Binding::External(url, name.to_string()));
            }
        }
        Err(error(format!("'{}' has no export named '{name}'", self.graph.rel(module))))
    }

    fn resolve_imported(&self, module: usize, spec: &str, imported: &Imported, depth: usize) -> Result<Binding, EngineError> {
        match (self.target(module, spec)?, imported) {
            (Target::Module(next), Imported::Namespace) => Ok(Binding::Namespace(next)),
            (Target::Module(next), Imported::Default) => self.resolve_export(next, "default", depth),
            (Target::Module(next), Imported::Named(name)) => self.resolve_export(next, name, depth),
            (Target::External(url), Imported::Namespace) => Ok(Binding::ExternalNamespace(url)),
            (Target::External(url), Imported::Default) => Ok(Binding::External(url, "default".to_string())),
            (Target::External(url), Imported::Named(name)) => Ok(Binding::External(url, name.clone())),
        }
    }

    fn resolve_import(&self, module: usize, local: &str, depth: usize) -> Result<Binding, EngineError> {
        let analysis = &self.graph.nodes[module].analysis;
        for import in &analysis.imports {
            if let Some((_, imported)) = import.bindings.iter().find(|(name, _)| name == local) {
                return self.resolve_imported(module, &import.spec, imported, depth);
            }
        }
        Err(error(format!("'{}': '{local}' is not imported", self.graph.rel(module))))
    }

    /// Every name `module` exports, and the libraries it re-exports wholesale.
    fn exported_names(&self, module: usize, seen: &mut HashSet<usize>) -> Result<(BTreeSet<String>, Vec<String>), EngineError> {
        let mut names = BTreeSet::new();
        let mut stars = Vec::new();
        if !seen.insert(module) {
            return Ok((names, stars));
        }
        let analysis = &self.graph.nodes[module].analysis;
        names.extend(analysis.exports.keys().cloned());
        for spec in &analysis.stars {
            match self.target(module, spec)? {
                Target::Module(next) => {
                    let (more, more_stars) = self.exported_names(next, seen)?;
                    names.extend(more.into_iter().filter(|name| name != "default"));
                    stars.extend(more_stars);
                }
                Target::External(url) => stars.push(url),
            }
        }
        Ok((names, stars))
    }

    /// The name `binding` has inside `chunk`, importing it when it lives elsewhere.
    fn local_for(&mut self, chunk: usize, binding: &Binding) -> Result<String, EngineError> {
        match binding {
            Binding::Local(module, local) => {
                let name = self.names.get(&(*module, local.clone())).cloned().ok_or_else(|| {
                    error(format!("'{}': '{local}' is not declared at the top of the file", self.graph.rel(*module)))
                })?;
                let home = self.graph.nodes[*module].chunk;
                if home == chunk {
                    return Ok(name);
                }
                let exported = self.export_from(home, &name);
                Ok(self.import_from(chunk, home, &exported))
            }
            Binding::Namespace(module) => {
                let home = self.graph.nodes[*module].chunk;
                let name = self.namespace_in(home, *module);
                if home == chunk {
                    return Ok(name);
                }
                let exported = self.export_from(home, &name);
                Ok(self.import_from(chunk, home, &exported))
            }
            Binding::External(url, name) => {
                let key = (url.clone(), name.clone());
                if let Some(found) = self.chunks[chunk].externals.get(&key) {
                    return Ok(found.clone());
                }
                let base = if name == "default" { stem(url) } else { name.clone() };
                let local = self.chunks[chunk].fresh(&base);
                self.chunks[chunk].externals.insert(key, local.clone());
                Ok(local)
            }
            Binding::ExternalNamespace(url) => {
                if let Some(found) = self.chunks[chunk].external_ns.get(url) {
                    return Ok(found.clone());
                }
                let local = self.chunks[chunk].fresh(&stem(url));
                self.chunks[chunk].external_ns.insert(url.clone(), local.clone());
                Ok(local)
            }
        }
    }

    fn namespace_in(&mut self, chunk: usize, module: usize) -> String {
        if let Some(found) = self.chunks[chunk].namespaces.get(&module) {
            return found.clone();
        }
        let name = self.chunks[chunk].fresh(&format!("{}_ns", stem(&self.graph.nodes[module].path)));
        self.chunks[chunk].namespaces.insert(module, name.clone());
        name
    }

    /// The name `chunk` exports `local` under, for another script to import.
    fn export_from(&mut self, chunk: usize, local: &str) -> String {
        if let Some(found) = self.chunks[chunk].export_of.get(local) {
            return found.clone();
        }
        let exported = format!("__z{}", self.chunks[chunk].exports.len());
        self.chunks[chunk].exports.insert(exported.clone(), local.to_string());
        self.chunks[chunk].export_of.insert(local.to_string(), exported.clone());
        exported
    }

    fn import_from(&mut self, chunk: usize, home: usize, exported: &str) -> String {
        let key = (home, exported.to_string());
        if let Some(found) = self.chunks[chunk].cross.get(&key) {
            return found.clone();
        }
        let local = self.chunks[chunk].fresh("z");
        self.chunks[chunk].cross.insert(key, local.clone());
        local
    }

    fn namespace_object(&mut self, chunk: usize, module: usize) -> Result<String, EngineError> {
        let (names, _) = self.exported_names(module, &mut HashSet::new())?;
        let name = self.chunks[chunk].namespaces[&module].clone();
        let mut members = Vec::new();
        for exported in names {
            let binding = self.resolve_export(module, &exported, 0)?;
            let local = self.local_for(chunk, &binding)?;
            members.push(format!("get {}() {{ return {local}; }}", quote(&exported)));
        }
        Ok(format!("const {name} = Object.freeze({{ __proto__: null, {} }});", members.join(", ")))
    }

    /// One file's text inside `chunk`.
    fn body(&mut self, chunk: usize, module: usize, entry_of: &HashMap<usize, usize>) -> Result<String, EngineError> {
        let node = &self.graph.nodes[module];
        let analysis = node.analysis.clone();
        let header = format!("// {}\n", node.rel);
        if analysis.kind != Kind::Script {
            let name = &self.names[&(module, DEFAULT_LOCAL.to_string())];
            return Ok(format!("{header}const {name} = {};\n", analysis.js));
        }

        let mut edits: Vec<(u32, u32, String)> = Vec::new();
        let mut removed: Vec<(u32, u32)> = Vec::new();
        for edit in &analysis.edits {
            match edit {
                Edit::Remove(start, end) => {
                    removed.push((*start, *end));
                    edits.push((*start, *end, String::new()));
                }
                Edit::Strip(start, end) => edits.push((*start, *end, String::new())),
                Edit::DefaultValue { start, end, stmt_end } => {
                    let name = &self.names[&(module, DEFAULT_LOCAL.to_string())];
                    edits.push((*start, *end, format!("const {name} = ")));
                    edits.push((*stmt_end, *stmt_end, ";".to_string()));
                }
            }
        }
        let inside = |start: u32, end: u32| removed.iter().any(|(s, e)| start >= *s && end <= *e);
        for (name, sites) in &analysis.top {
            let renamed = &self.names[&(module, name.clone())];
            if renamed == name {
                continue;
            }
            for site in sites.iter().filter(|site| !inside(site.start, site.end)) {
                let text = if site.shorthand { format!("{name}: {renamed}") } else { renamed.clone() };
                edits.push((site.start, site.end, text));
            }
        }
        for (local, sites) in &analysis.import_refs {
            let live: Vec<_> = sites.iter().filter(|site| !inside(site.start, site.end)).collect();
            if live.is_empty() {
                continue;
            }
            let binding = self.resolve_import(module, local, 0)?;
            let name = self.local_for(chunk, &binding)?;
            for site in live {
                let text = if site.shorthand { format!("{local}: {name}") } else { name.clone() };
                edits.push((site.start, site.end, text));
            }
        }
        // An import whose names are not used still runs its file first.
        for import in &analysis.imports {
            match self.target(module, &import.spec)? {
                Target::Module(next) => {
                    let home = self.graph.nodes[next].chunk;
                    if home != chunk {
                        self.chunks[chunk].bare.insert(home);
                    }
                }
                Target::External(url) => {
                    self.chunks[chunk].external_bare.insert(url);
                }
            }
        }
        for dynamic in &analysis.dynamic {
            let Target::Module(next) = self.target(module, &dynamic.spec)? else {
                // A library loaded lazily keeps its relative address.
                let Target::External(url) = self.target(module, &dynamic.spec)? else { unreachable!() };
                edits.push((dynamic.start, dynamic.end, quote(&url)));
                continue;
            };
            let entry = entry_of[&next];
            self.chunks[chunk].lazy.insert(entry);
            edits.push((dynamic.start, dynamic.end, quote(&format!("./{LAZY}{entry}@@"))));
        }
        // A worker's script, by its address beside this one.
        for worker in &analysis.workers {
            let Target::Module(next) = self.target(module, &worker.spec)? else {
                return Err(error(format!("'{}' starts a worker from '{}', which is not a file of a browser module folder", self.graph.rel(module), worker.spec)));
            };
            let entry = entry_of[&next];
            self.chunks[chunk].lazy.insert(entry);
            edits.push((worker.start, worker.end, quote(&format!("./{LAZY}{entry}@@"))));
        }
        for (glob, matched) in analysis.globs.iter().zip(&self.graph.nodes[module].globs) {
            let mut members = Vec::new();
            for (key, next) in matched {
                let value = if glob.eager {
                    let binding = match &glob.import {
                        None => Binding::Namespace(*next),
                        Some(name) => self.resolve_export(*next, name, 0)?,
                    };
                    self.local_for(chunk, &binding)?
                } else {
                    let entry = entry_of[next];
                    self.chunks[chunk].lazy.insert(entry);
                    let load = format!("import({})", quote(&format!("./{LAZY}{entry}@@")));
                    match &glob.import {
                        None => format!("() => {load}"),
                        Some(name) => format!("() => {load}.then((m) => m[{}])", quote(name)),
                    }
                };
                members.push(format!("{}: {value}", quote(key)));
            }
            edits.push((glob.start, glob.end, format!("({{ {} }})", members.join(", "))));
        }

        edits.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
        let mut text = analysis.js.clone();
        let mut floor = u32::MAX;
        for (start, end, replacement) in edits {
            if end > floor && start != end {
                continue;
            }
            text.replace_range(start as usize..end as usize, &replacement);
            floor = start;
        }
        Ok(format!("{header}{text}\n"))
    }

    fn assemble(mut self, entries: &[Entry], entry_count: usize, folders: &[String], files: &[PathBuf]) -> Result<Build, EngineError> {
        // Scripts: one per chunk, then one facade per entry that shares its chunk.
        let facades = std::mem::take(&mut self.facades);
        let chunk_count = self.chunks.len();
        let mut entry_script: Vec<usize> = Vec::new();
        let mut facade_nodes: Vec<(usize, usize)> = Vec::new();
        for (entry, (chunk, _, _, single)) in facades.iter().enumerate() {
            if *single {
                entry_script.push(*chunk);
            } else {
                entry_script.push(chunk_count + facade_nodes.len());
                facade_nodes.push((entry, *chunk));
            }
        }
        let total = chunk_count + facade_nodes.len();
        let mut deps: Vec<BTreeSet<usize>> = vec![BTreeSet::new(); total];
        for (chunk, data) in self.chunks.iter().enumerate() {
            for (home, _) in data.cross.keys() {
                deps[chunk].insert(*home);
            }
            deps[chunk].extend(data.bare.iter().copied());
            for entry in &data.lazy {
                deps[chunk].insert(entry_script[*entry]);
            }
            deps[chunk].remove(&chunk);
        }
        for (index, (_, chunk)) in facade_nodes.iter().enumerate() {
            deps[chunk_count + index].insert(*chunk);
        }

        // Hash in dependency order; a cycle is refused by name.
        let label_of = |script: usize, this: &Self| -> String {
            if script < chunk_count {
                let data = &this.chunks[script];
                match data.entries.as_slice() {
                    [only] => entries[*only].rel.clone(),
                    _ => format!("shared by {}", data.entries.iter().map(|e| entries[*e].rel.clone()).collect::<Vec<_>>().join(", ")),
                }
            } else {
                entries[facade_nodes[script - chunk_count].0].rel.clone()
            }
        };
        let mut hashes: Vec<Option<String>> = vec![None; total];
        let mut state = vec![0u8; total];
        let mut order = Vec::new();
        fn visit(node: usize, deps: &[BTreeSet<usize>], state: &mut [u8], order: &mut Vec<usize>, path: &mut Vec<usize>) -> Result<(), Vec<usize>> {
            match state[node] {
                2 => return Ok(()),
                1 => {
                    let at = path.iter().position(|n| *n == node).unwrap_or(0);
                    return Err(path[at..].to_vec());
                }
                _ => {}
            }
            state[node] = 1;
            path.push(node);
            for next in &deps[node] {
                visit(*next, deps, state, order, path)?;
            }
            path.pop();
            state[node] = 2;
            order.push(node);
            Ok(())
        }
        for node in 0..total {
            if let Err(cycle) = visit(node, &deps, &mut state, &mut order, &mut Vec::new()) {
                let named: Vec<String> = cycle.iter().map(|n| label_of(*n, &self)).collect();
                return Err(error(format!(
                    "these scripts load each other in a circle: {}; a file loaded lazily with import() must not import, directly or through others, the file that loads it",
                    named.join(" → ")
                )));
            }
        }

        let mut scripts = BTreeMap::new();
        let mut loads = BTreeMap::new();
        for node in order {
            let content = if node < chunk_count {
                self.chunk_text(node, &hashes, &entry_script)
            } else {
                let (entry, chunk) = facade_nodes[node - chunk_count];
                let (_, public, stars, _) = &facades[entry];
                let mut text = format!("// zebflow browser module: {}\n", entries[entry].rel);
                if !public.is_empty() {
                    let list: Vec<String> = public.iter().map(|(name, local)| format!("{local} as {}", export_name(name))).collect();
                    text.push_str(&format!("export {{ {} }} from \"./{}\";\n", list.join(", "), hashes[chunk].as_deref().unwrap_or("")));
                }
                for url in stars {
                    text.push_str(&format!("export * from {};\n", quote(url)));
                }
                text
            };
            let hash = {
                use sha2::{Digest, Sha256};
                hex::encode(Sha256::digest(content.as_bytes()))
            };
            let loaded: BTreeSet<String> = deps[node].iter().filter_map(|d| hashes[*d].clone()).collect();
            loads.insert(hash.clone(), loaded);
            scripts.insert(hash.clone(), ModuleScript { label: label_of(node, &self), content, hash: hash.clone() });
            hashes[node] = Some(hash);
        }

        let mut by_entry = BTreeMap::new();
        let mut folder_entries: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for (index, entry) in entries.iter().enumerate().take(entry_count) {
            let hash = hashes[entry_script[index]].clone().unwrap_or_default();
            let without_ext = entry.rel.rsplit_once('.').map(|(stem, _)| stem.to_string()).unwrap_or_else(|| entry.rel.clone());
            folder_entries.entry(folders[entry.folder].clone()).or_default().push(format!("@/{without_ext}"));
            by_entry.insert(format!("@/{}", entry.rel), hash.clone());
            by_entry.insert(format!("@/{without_ext}"), hash.clone());
            if let Some(folder) = without_ext.strip_suffix("/index") {
                by_entry.insert(format!("@/{folder}"), hash);
            }
        }
        Ok(Build {
            entries: by_entry,
            scripts,
            loads,
            files: files.iter().map(|f| f.display().to_string()).collect(),
            folders: folders.to_vec(),
            folder_entries,
            bundled_into: BTreeMap::new(),
            lazy: BTreeSet::new(),
        })
    }

    fn chunk_text(&self, chunk: usize, hashes: &[Option<String>], entry_script: &[usize]) -> String {
        let data = &self.chunks[chunk];
        let hash_of = |script: usize| hashes[script].clone().unwrap_or_default();
        let mut text = String::from("// zebflow browser module\n");
        // Libraries.
        let mut by_url: BTreeMap<&String, Vec<String>> = BTreeMap::new();
        for ((url, name), local) in &data.externals {
            by_url.entry(url).or_default().push(format!("{} as {local}", export_name(name)));
        }
        for (url, list) in &by_url {
            text.push_str(&format!("import {{ {} }} from {};\n", list.join(", "), quote(url)));
        }
        for (url, local) in &data.external_ns {
            text.push_str(&format!("import * as {local} from {};\n", quote(url)));
        }
        for url in &data.external_bare {
            if !by_url.contains_key(url) && !data.external_ns.contains_key(url) {
                text.push_str(&format!("import {};\n", quote(url)));
            }
        }
        // Other scripts of the folder.
        let mut by_chunk: BTreeMap<usize, Vec<String>> = BTreeMap::new();
        for ((home, exported), local) in &data.cross {
            by_chunk.entry(*home).or_default().push(format!("{exported} as {local}"));
        }
        for home in &data.bare {
            by_chunk.entry(*home).or_default();
        }
        for (home, list) in &by_chunk {
            if list.is_empty() {
                text.push_str(&format!("import \"./{}\";\n", hash_of(*home)));
            } else {
                text.push_str(&format!("import {{ {} }} from \"./{}\";\n", list.join(", "), hash_of(*home)));
            }
        }
        for namespace in data.namespace_text.values() {
            text.push_str(namespace);
            text.push('\n');
        }
        for body in &data.bodies {
            text.push_str(body);
        }
        let mut exported: Vec<String> = data.exports.iter().map(|(name, local)| format!("{local} as {name}")).collect();
        exported.extend(data.public.iter().map(|(name, local)| format!("{local} as {}", export_name(name))));
        if !exported.is_empty() {
            text.push_str(&format!("export {{ {} }};\n", exported.join(", ")));
        }
        for url in &data.public_stars {
            text.push_str(&format!("export * from {};\n", quote(url)));
        }
        // Lazy loads, now that every target has its address.
        let mut out = text;
        for entry in &data.lazy {
            out = out.replace(&format!("{LAZY}{entry}@@"), &hash_of(entry_script[*entry]));
        }
        out
    }
}

fn quote(text: &str) -> String {
    serde_json::to_string(text).unwrap_or_else(|_| format!("\"{text}\""))
}

/// A name as an export clause writes it: bare when it is an identifier, quoted otherwise.
fn export_name(name: &str) -> String {
    let identifier = name.chars().next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_' || c == '$')
        && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '$');
    if identifier { name.to_string() } else { quote(name) }
}
