//! Project workers (`kinds/rwe-source` § Workers): a page's default import of
//! a `@/…*.worker.ts` file is the address of that file built as its own module
//! script — the file and the project files it imports, flat, plus project
//! library entries loaded from the same host. A page compile builds each
//! worker it meets once; the scripts travel with the compiled page.

use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap, HashSet};

use super::config::{CompileOptions, SecurityPolicy};
use super::error::EngineError;

/// What a resolved worker import is rewritten to: this prefix, then the
/// worker's address. Never a file, never a library, so nothing bundles it.
pub(super) const MARKER: &str = "zeb-worker:";

/// Where a worker's script is served before the page moves it onto the
/// page's own host (the same move the library addresses make).
pub const SCRIPT_BASE: &str = "/assets/rwe/scripts/";

/// One built worker.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct WorkerScript {
    /// The worker's source file, absolute.
    pub path: String,
    /// The module script, JavaScript.
    pub content: String,
    /// `sha256` hex of `content`: the script's address.
    pub hash: String,
}

/// Whether a resolved import names a worker.
pub fn is_worker_path(path: &str) -> bool {
    path.ends_with(".worker.ts")
}

struct Build {
    options: CompileOptions,
    scripts: Vec<WorkerScript>,
    by_path: HashMap<String, String>,
    deps: HashSet<String>,
    building: bool,
}

thread_local! {
    static BUILD: RefCell<Option<Build>> = const { RefCell::new(None) };
}

/// The workers a page compile built, removed from the thread when it ends.
pub(super) struct Session;

impl Session {
    /// Starts collecting the workers one page compile meets.
    pub(super) fn begin(options: &CompileOptions) -> Self {
        BUILD.with(|cell| {
            *cell.borrow_mut() = Some(Build {
                options: options.clone(),
                scripts: Vec::new(),
                by_path: HashMap::new(),
                deps: HashSet::new(),
                building: false,
            })
        });
        Session
    }

    /// The workers built, and every file they read.
    pub(super) fn finish(self) -> (Vec<WorkerScript>, HashSet<String>) {
        BUILD.with(|cell| cell.borrow_mut().take()).map(|build| (build.scripts, build.deps)).unwrap_or_default()
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        BUILD.with(|cell| cell.borrow_mut().take());
    }
}

/// The marker a worker import is rewritten to, building the worker the first
/// time it is met.
pub(super) fn marker_for(path: &str) -> Result<String, EngineError> {
    let known = BUILD.with(|cell| cell.borrow().as_ref().map(|build| (build.by_path.get(path).cloned(), build.building, build.options.clone())));
    let Some((existing, building, options)) = known else {
        return Err(EngineError::new("RWE_WORKER", format!("'{path}' is a worker; only a page can import it")));
    };
    if let Some(url) = existing {
        return Ok(format!("{MARKER}{url}"));
    }
    if building {
        return Err(EngineError::new("RWE_WORKER", format!("a worker may not start another project worker ('{path}')")));
    }
    BUILD.with(|cell| cell.borrow_mut().as_mut().map(|build| build.building = true));
    let built = build(path, &options);
    BUILD.with(|cell| cell.borrow_mut().as_mut().map(|build| build.building = false));
    let (script, deps) = built?;
    let url = format!("{SCRIPT_BASE}{}", script.hash);
    BUILD.with(|cell| {
        if let Some(build) = cell.borrow_mut().as_mut() {
            build.by_path.insert(path.to_string(), url.clone());
            build.deps.extend(deps);
            build.scripts.push(script);
        }
    });
    Ok(format!("{MARKER}{url}"))
}

/// The specifiers a module imports.
fn imports_of(source: &str) -> Vec<String> {
    use oxc_allocator::Allocator;
    use oxc_ast::ast::Statement;
    use oxc_parser::Parser;
    use oxc_span::SourceType;
    let alloc = Allocator::default();
    let parsed = Parser::new(&alloc, source, SourceType::default().with_module(true).with_typescript(true)).parse();
    parsed
        .program
        .body
        .iter()
        .filter_map(|stmt| match stmt {
            Statement::ImportDeclaration(import) => Some(import.source.value.to_string()),
            _ => None,
        })
        .collect()
}

/// A project library entry's address, relative to where workers are served:
/// libraries are served from `<base>_rwe/lib/`, workers from
/// `<base>_rwe/scripts/`, on every host.
fn library_address(url: &str) -> String {
    format!("../lib/{}", url.trim_start_matches("/assets/libraries/"))
}

/// Builds one worker: its own source and the project files it imports, flat,
/// checked like page code, then compiled to JavaScript.
fn build(path: &str, options: &CompileOptions) -> Result<(WorkerScript, HashSet<String>), EngineError> {
    let refuse = |why: String| EngineError::new("RWE_WORKER", format!("worker '{path}': {why}"));
    let raw = std::fs::read_to_string(path).map_err(|err| refuse(format!("cannot be read: {err}")))?;
    let template_root = options.template_root.as_deref();
    let source = super::compiler::rewrite_at_imports(&raw, std::path::Path::new(template_root.unwrap_or("/")), &options.library_roots)?;

    // Project library entries stay real imports, loaded from the same host;
    // only the worker's own file may name them, so the flat bundle never
    // carries an import a module's renaming could touch.
    let runtime: &BTreeMap<String, String> = &options.runtime_libraries;
    let mut edges = Vec::new();
    let plain = CompileOptions { runtime_libraries: BTreeMap::new(), ..options.clone() };
    for import in imports_of(&source) {
        if import.starts_with("zeb/") {
            return Err(refuse(format!("imports '{import}'; zeb/* libraries live in a page, not a worker")));
        }
        if runtime.contains_key(&import) {
            continue;
        }
        if !super::is_file_specifier(&import) {
            return Err(refuse(format!("imports '{import}'; a worker imports project files (@/…) and project library entries only")));
        }
        let resolved = super::compiler::resolve_import(&import, &plain)?;
        edges.push(super::model::ImportEdge { source: import, resolved });
    }

    let (bundle, zeb_libs, visited, _) = super::compiler::bundle_for_client(&source, &edges, template_root, &options.library_roots, &BTreeMap::new())?;
    if let Some(lib) = zeb_libs.first() {
        return Err(refuse(format!("a file it imports uses '{lib}'; zeb/* libraries live in a page, not a worker")));
    }
    for file in visited.iter().filter(|file| file.as_str() != path) {
        let text = std::fs::read_to_string(file).unwrap_or_default();
        if let Some(bad) = imports_of(&text).into_iter().find(|import| import.starts_with("zeb/") || runtime.contains_key(import)) {
            return Err(refuse(format!("'{file}' imports '{bad}'; inside a worker only the worker's own file imports libraries, and never zeb/*")));
        }
    }
    let policy = SecurityPolicy::default();
    for text in std::iter::once(source.clone()).chain(visited.iter().filter(|file| file.as_str() != path).filter_map(|file| std::fs::read_to_string(file).ok())) {
        super::security::analyze(&text, &policy).map_err(|err| refuse(err.message))?;
        if text.contains("importScripts(") {
            return Err(refuse("uses importScripts; a module worker imports".to_string()));
        }
    }

    let bundle = super::compiler::rewrite_specifiers(&bundle, |specifier| runtime.get(specifier).map(|url| library_address(url)));
    let content = super::deno_worker::transpile_ts(&bundle)?;
    let content = format!("// zebflow worker: {}\n{content}", std::path::Path::new(path).file_name().and_then(|n| n.to_str()).unwrap_or("worker"));
    let hash = {
        use sha2::{Digest, Sha256};
        hex::encode(Sha256::digest(content.as_bytes()))
    };
    let mut deps: HashSet<String> = visited;
    deps.insert(path.to_string());
    Ok((WorkerScript { path: path.to_string(), content, hash }, deps))
}

#[cfg(test)]
mod tests {
    use super::super::config::CompileOptions;
    use std::fs;

    fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
        let tmp = tempfile::Builder::new().prefix("rwe-worker-").tempdir().expect("tempdir");
        for (rel, text) in files {
            let path = tmp.path().join(rel);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, text).unwrap();
        }
        tmp
    }

    fn compile_page(root: &std::path::Path, page: &str) -> Result<super::super::CompiledTemplate, super::super::EngineError> {
        let mut runtime = std::collections::BTreeMap::new();
        runtime.insert("site-a-lib/heat".to_string(), "/assets/libraries/site-a-lib/0.1.0/runtime/heat.mjs".to_string());
        super::super::compile(
            page,
            CompileOptions {
                template_root: Some(root.display().to_string()),
                file_path: Some(root.join("pages/p.tsx").display().to_string()),
                runtime_libraries: runtime,
                ..CompileOptions::default()
            },
        )
    }

    const PAGE: &str = "import { useEffect } from \"zeb/react\";\nimport heatWorker from \"@/modules/heat/heat.worker.ts\";\nexport default function Page() {\n  useEffect(() => { const w = new Worker(heatWorker, { type: \"module\" }); return () => w.terminate(); }, []);\n  return <main>heat</main>;\n}\n";

    #[test]
    fn a_page_gets_the_address_and_the_worker_is_built_on_its_own() {
        let tmp = project(&[
            ("modules/heat/heat.worker.ts", "import { warm } from \"@/modules/heat/model.ts\";\nimport { shade } from \"site-a-lib/heat\";\nself.onmessage = (e: MessageEvent<number>) => self.postMessage(shade(warm(e.data)));\n"),
            ("modules/heat/model.ts", "export function warm(x: number): number { return x * 2; }\n"),
        ]);
        let compiled = compile_page(tmp.path(), PAGE).expect("compiles");
        assert_eq!(compiled.worker_scripts.len(), 1);
        let worker = &compiled.worker_scripts[0];
        assert!(compiled.client_module_source.contains(&format!("globalThis.__zebWorker(\"{}\")", worker.hash)), "{}", compiled.client_module_source);
        assert!(!compiled.client_module_source.contains("onmessage"), "the worker is not in the page");
        assert!(worker.content.contains("x * 2"), "its project imports are bundled: {}", worker.content);
        assert!(worker.content.contains("\"../lib/site-a-lib/0.1.0/runtime/heat.mjs\""), "a library entry loads from the same host: {}", worker.content);
        assert!(!worker.content.contains(": number"), "compiled to JavaScript: {}", worker.content);
        for file in ["modules/heat/heat.worker.ts", "modules/heat/model.ts"] {
            let path = fs::canonicalize(tmp.path().join(file)).unwrap().display().to_string();
            assert!(compiled.dependency_paths.iter().any(|dep| dep == &path || dep.ends_with(file)), "a save of {file} recompiles the page: {:?}", compiled.dependency_paths);
        }
    }

    #[test]
    fn a_worker_stays_a_worker() {
        let refused = |worker: &str, page: &str, word: &str| {
            let tmp = project(&[("modules/heat/heat.worker.ts", worker), ("modules/heat/other.worker.ts", "self.onmessage = () => {};\n")]);
            let err = compile_page(tmp.path(), page).expect_err(word);
            assert!(err.message.contains(word), "{word}: {}", err.message);
        };
        refused("import { useState } from \"zeb/react\";\nself.onmessage = () => {};\n", PAGE, "zeb/*");
        refused("const f = eval(\"1\");\n", PAGE, "eval");
        refused("importScripts(\"/x.js\");\n", PAGE, "importScripts");
        refused("import o from \"@/modules/heat/other.worker.ts\";\nself.onmessage = () => o;\n", PAGE, "another project worker");
        refused("import x from \"npm:left-pad\";\n", PAGE, "project files");
        let named = PAGE.replace("import heatWorker from", "import { heatWorker } from");
        refused("self.onmessage = () => {};\n", &named, "imported by default only");
    }
}
