//! Browser module folders, built from files on disk.

use super::*;
use std::fs;

fn project(files: &[(&str, &str)]) -> tempfile::TempDir {
    let tmp = tempfile::Builder::new().prefix("rwe-modules-").tempdir().expect("tempdir");
    for (rel, text) in files {
        let path = tmp.path().join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }
    tmp
}

fn folder(entries: &[&str]) -> Vec<ModuleFolder> {
    vec![ModuleFolder { path: "modules/game".to_string(), entries: entries.iter().map(|e| e.to_string()).collect() }]
}

fn built(tmp: &tempfile::TempDir, entries: &[&str]) -> Result<Arc<Build>, EngineError> {
    build(tmp.path(), &folder(entries), &BTreeMap::new(), &SecurityPolicy::default())
}

fn script<'a>(build: &'a Build, entry: &str) -> &'a str {
    let hash = build.entries.get(entry).unwrap_or_else(|| panic!("no entry {entry}: {:?}", build.entries.keys()));
    &build.scripts[hash].content
}

/// Every script is a module oxc parses without an error.
fn parses(build: &Build) {
    for script in build.scripts.values() {
        let alloc = oxc_allocator::Allocator::default();
        let parsed = oxc_parser::Parser::new(&alloc, &script.content, oxc_span::SourceType::default().with_module(true)).parse();
        assert!(!parsed.panicked && parsed.errors.is_empty(), "{:?}\n{}", parsed.errors, script.content);
    }
}

#[test]
fn a_pattern_matches_inside_one_folder_or_across_with_two_stars() {
    assert!(glob_matches("islands/*.ts", "islands/north.ts"));
    assert!(!glob_matches("islands/*.ts", "islands/north/a.ts"));
    assert!(glob_matches("**/*.ts", "a/b/c.ts"));
    assert!(glob_matches("main.ts", "main.ts"));
}

#[test]
fn two_files_may_export_the_same_name() {
    let tmp = project(&[
        ("modules/game/main.ts", "import { build as tree } from \"./objects/tree\";\nimport { build as boat } from \"./objects/boat\";\nexport function start(n: number): string { return tree(n) + boat(n); }\n"),
        ("modules/game/objects/tree.ts", "const size = 2;\nexport function build(n: number): string { return \"tree\" + n * size; }\n"),
        ("modules/game/objects/boat.ts", "const size = 3;\nexport function build(n: number): string { return \"boat\" + n * size; }\n"),
    ]);
    let build = built(&tmp, &["main.ts"]).expect("builds");
    parses(&build);
    assert_eq!(build.scripts.len(), 1, "one entry, one script");
    let text = script(&build, "@/modules/game/main");
    // A name two files declare is renamed in both, so neither file's text
    // depends on which one the bundler happened to read first.
    assert!(text.contains("function build$1(") && text.contains("function build$2("), "{text}");
    assert!(text.contains("n * size$1") && text.contains("n * size$2"), "each file keeps its own constant: {text}");
    assert!(text.contains("return build$1(n) + build$2(n)") || text.contains("return build$2(n) + build$1(n)"), "imports become direct references: {text}");
    assert!(text.contains("function start("), "a name only one file uses stays: {text}");
    assert!(text.contains("export { start as start }"), "{text}");
    assert!(!text.contains(": number"), "compiled to JavaScript: {text}");
    assert!(!text.contains("import "), "nothing left to import: {text}");
    assert_eq!(build.entries.get("@/modules/game/main.ts"), build.entries.get("@/modules/game/main"));
}

#[test]
fn code_two_entries_share_is_stored_once() {
    let tmp = project(&[
        ("modules/game/main.ts", "import { engine } from \"./engine\";\nexport const game = () => engine(\"game\");\n"),
        ("modules/game/tools.ts", "import { engine } from \"./engine\";\nexport const tools = () => engine(\"tools\");\n"),
        ("modules/game/engine.ts", "let frames = 0;\nexport function engine(who: string) { frames += 1; return who + frames; }\n"),
    ]);
    let build = built(&tmp, &["main.ts", "tools.ts"]).expect("builds");
    parses(&build);
    assert_eq!(build.scripts.len(), 3, "two entries and the engine they share");
    let shared: Vec<_> = build.scripts.values().filter(|s| s.content.contains("let frames")).collect();
    assert_eq!(shared.len(), 1, "the engine is in one script only");
    for entry in ["@/modules/game/main", "@/modules/game/tools"] {
        let text = script(&build, entry);
        assert!(!text.contains("let frames"), "{text}");
        assert!(text.contains(&format!("from \"./{}\"", shared[0].hash)), "imports the shared script by address: {text}");
        assert!(build.loads[&build.entries[entry]].contains(&shared[0].hash));
    }
    assert_eq!(build.scripts_for(&["@/modules/game/main".to_string()]).len(), 2);
}

#[test]
fn a_lazy_part_is_an_entry_of_its_own() {
    let tmp = project(&[
        ("modules/game/main.ts", "export async function enter() { const island = await import(\"./islands/north.ts\"); return island.name; }\n"),
        ("modules/game/islands/north.ts", "export const name = \"north\";\n"),
    ]);
    let build = built(&tmp, &["main.ts", "islands/*.ts"]).expect("builds");
    parses(&build);
    let north = &build.entries["@/modules/game/islands/north"];
    let main = script(&build, "@/modules/game/main");
    assert!(main.contains(&format!("import(\"./{north}\")")), "{main}");
    assert!(!main.contains("\"north\""), "the island is not in the main script: {main}");

    let refused = built(&tmp, &["main.ts"]).expect_err("not an entry");
    assert!(refused.message.contains("not an entry"), "{}", refused.message);
}

#[test]
fn a_glob_brings_in_every_file_of_a_folder() {
    let tmp = project(&[
        ("modules/game/objects.ts", "const all = import.meta.glob(\"./objects/*.ts\", { eager: true });\nexport const kinds = Object.keys(all);\nexport function make(kind: string) { return (all[\"./objects/\" + kind + \".ts\"] as any).build(); }\n"),
        ("modules/game/objects/tree.ts", "export function build() { return \"tree\"; }\n"),
        ("modules/game/objects/ship.ts", "export function build() { return \"ship\"; }\n"),
    ]);
    let build = built(&tmp, &["objects.ts"]).expect("builds");
    parses(&build);
    let text = script(&build, "@/modules/game/objects");
    assert!(text.contains("\"./objects/ship.ts\": ship_ns$1") || text.contains("\"./objects/ship.ts\":"), "{text}");
    assert!(text.contains("\"./objects/tree.ts\""), "{text}");
    assert!(text.contains("Object.freeze({ __proto__: null, get \"build\"()"), "each file is its namespace: {text}");
}

#[test]
fn data_and_text_import_as_values() {
    let tmp = project(&[
        ("modules/game/main.ts", "import zones from \"./data/north.json\";\nimport water from \"./shaders/water.glsl?raw\";\nexport const count = zones.length;\nexport const shader = water;\n"),
        ("modules/game/data/north.json", "[{\"id\": 1}, {\"id\": 2}]"),
        ("modules/game/shaders/water.glsl", "void main() { gl_FragColor = vec4(1.0); }\n"),
    ]);
    let build = built(&tmp, &["main.ts"]).expect("builds");
    parses(&build);
    let text = script(&build, "@/modules/game/main");
    assert!(text.contains("[{\"id\":1},{\"id\":2}]"), "{text}");
    assert!(text.contains("\"void main() { gl_FragColor = vec4(1.0); }\\n\""), "{text}");

    fs::write(tmp.path().join("modules/game/main.ts"), "import water from \"./shaders/water.glsl\";\nexport const shader = water;\n").unwrap();
    let refused = built(&tmp, &["main.ts"]).expect_err("needs ?raw");
    assert!(refused.message.contains("?raw"), "{}", refused.message);
}

#[test]
fn a_library_loads_from_beside_the_scripts() {
    let tmp = project(&[(
        "modules/game/main.ts",
        "import { Mesh, BoxGeometry } from \"zeb/threejs\";\nexport function box() { return new Mesh(new BoxGeometry(1, 1, 1)); }\n",
    )]);
    let build = built(&tmp, &["main.ts"]).expect("builds");
    parses(&build);
    let text = script(&build, "@/modules/game/main");
    assert!(text.contains("from \"../lib/zeb/threejs/0.1/runtime/threejs.bundle.mjs\""), "{text}");
    assert!(text.contains("BoxGeometry as BoxGeometry$1") && text.contains("new Mesh$1(new BoxGeometry$1(1, 1, 1))"), "{text}");
}

#[test]
fn the_border_is_kept() {
    let outside = project(&[
        ("modules/game/main.ts", "import { x } from \"@/shared/x\";\nexport const y = x;\n"),
        ("shared/x.ts", "export const x = 1;\n"),
    ]);
    let refused = built(&outside, &["main.ts"]).expect_err("outside");
    assert!(refused.message.contains("outside every browser module folder"), "{}", refused.message);

    let bare = project(&[("modules/game/main.ts", "import left from \"left-pad\";\nexport const y = left;\n")]);
    assert!(built(&bare, &["main.ts"]).expect_err("bare").message.contains("a browser module imports"));

    let computed = project(&[("modules/game/main.ts", "export const load = (n: string) => import(\"./\" + n);\n")]);
    assert!(built(&computed, &["main.ts"]).expect_err("computed").message.contains("fixed string"));

    let none = project(&[("modules/game/main.ts", "export const a = 1;\n")]);
    assert!(built(&none, &["missing.ts"]).expect_err("no match").message.contains("matches no"));
}

#[test]
fn a_lazy_load_that_closes_a_circle_is_refused() {
    // main loads north lazily; north imports main statically. main's script would
    // need north's address, and north's script main's: no order hashes both.
    let tmp = project(&[
        ("modules/game/main.ts", "export const name = \"main\";\nexport const go = () => import(\"./islands/north\");\n"),
        ("modules/game/islands/north.ts", "import { name } from \"../main\";\nexport const where = name + \"/north\";\n"),
    ]);
    let refused = built(&tmp, &["main.ts", "islands/*.ts"]).expect_err("circle");
    assert!(refused.message.contains("in a circle"), "{}", refused.message);
    assert!(refused.message.contains("modules/game/main.ts") && refused.message.contains("modules/game/islands/north.ts"), "names both: {}", refused.message);

    // The same lazy load without the way back is fine.
    fs::write(tmp.path().join("modules/game/islands/north.ts"), "export const where = \"north\";\n").unwrap();
    parses(&built(&tmp, &["main.ts", "islands/*.ts"]).expect("builds"));
}

#[test]
fn an_unchanged_folder_is_not_built_again() {
    let tmp = project(&[("modules/game/main.ts", "export const a = 1;\n")]);
    let first = built(&tmp, &["main.ts"]).expect("builds");
    let second = built(&tmp, &["main.ts"]).expect("builds");
    assert!(Arc::ptr_eq(&first, &second));
    std::thread::sleep(std::time::Duration::from_millis(10));
    fs::write(tmp.path().join("modules/game/main.ts"), "export const a = 2;\n").unwrap();
    let third = built(&tmp, &["main.ts"]).expect("builds");
    assert!(!Arc::ptr_eq(&first, &third));
    assert!(script(&third, "@/modules/game/main").contains("a = 2"));
}

#[test]
fn default_exports_and_shorthand_survive_renaming() {
    let tmp = project(&[
        ("modules/game/main.ts", "import one from \"./a\";\nimport two from \"./b\";\nexport default { one, two };\n"),
        ("modules/game/a.ts", "const value = 1;\nexport default value;\nexport const shape = { value };\n"),
        ("modules/game/b.ts", "const value = 2;\nexport default function () { return { value }; }\n"),
    ]);
    let build = built(&tmp, &["main.ts"]).expect("builds");
    parses(&build);
    let text = script(&build, "@/modules/game/main");
    assert!(text.contains("{ value: value$"), "a shorthand keeps its property name: {text}");
    assert!(text.contains("one: ") && text.contains("two: "), "{text}");
    assert!(text.contains("as default"), "{text}");
}

fn compile_page(tmp: &tempfile::TempDir, entries: &[&str], page: &str) -> Result<super::super::CompiledTemplate, EngineError> {
    super::super::compile(
        page,
        super::super::CompileOptions {
            template_root: Some(tmp.path().display().to_string()),
            file_path: Some(tmp.path().join("pages/game.tsx").display().to_string()),
            browser_modules: if entries.is_empty() { Vec::new() } else { folder(entries) },
            ..super::super::CompileOptions::default()
        },
    )
}

const GAME_PAGE: &str = "import { useEffect, useRef } from \"zeb/react\";\nimport { startGame } from \"@/modules/game/main\";\nimport Hud from \"@/components/hud\";\nexport default function Page() {\n  const canvas = useRef(null);\n  useEffect(() => { const game = startGame(canvas.current); return () => game.stop(); }, []);\n  return <main><Hud /><canvas ref={canvas} /></main>;\n}\n";

fn game_project() -> tempfile::TempDir {
    project(&[
        ("modules/game/main.ts", "import { engine } from \"./engine\";\nexport function startGame(canvas: unknown) { const e = engine(canvas); return { stop() { e.running = false; } }; }\n"),
        ("modules/game/engine.ts", "const built = new Map<string, number>();\nexport function engine(canvas: unknown) { built.set(\"frame\", 1); return { canvas, running: true }; }\n"),
        ("components/hud.tsx", "import { startGame } from \"@/modules/game/main\";\nexport default function Hud() { return <p>{typeof startGame}</p>; }\n"),
    ])
}

#[test]
fn a_page_loads_an_entry_and_never_inlines_it() {
    let tmp = game_project();
    let compiled = compile_page(&tmp, &["main.ts"], GAME_PAGE).expect("compiles");
    assert_eq!(compiled.module_scripts.len(), 1, "{:?}", compiled.module_scripts.iter().map(|s| &s.label).collect::<Vec<_>>());
    let module = &compiled.module_scripts[0];
    assert!(module.content.contains("const built = new Map()"), "the game is in its own script: {}", module.content);
    for source in [&compiled.client_module_source, &compiled.server_module_source] {
        assert!(!source.contains("new Map"), "the game code is in neither page bundle: {source}");
    }
    assert!(compiled.client_module_source.contains("globalThis.__zebLibs[\"@/modules/game/main\"][\"startGame\"]"), "{}", compiled.client_module_source);
    assert!(compiled.server_module_source.contains("__inert"), "an inert stand-in on the server: {}", compiled.server_module_source);
    assert!(compiled.detected_zeb_libs.iter().any(|lib| lib == "@/modules/game/main"));
    let folder_path = fs::canonicalize(tmp.path().join("modules/game")).unwrap().display().to_string();
    assert!(compiled.dependency_paths.contains(&folder_path), "a save anywhere in the folder recompiles the page: {:?}", compiled.dependency_paths);
}

#[test]
fn page_code_imports_entries_only() {
    let tmp = game_project();
    let page = GAME_PAGE.replace("@/modules/game/main", "@/modules/game/engine").replace("startGame", "engine");
    let refused = compile_page(&tmp, &["main.ts"], &page).expect_err("internal");
    assert_eq!(refused.code, "RWE_IMPORT_MODULE_INTERNAL");
    assert!(refused.message.contains("@/modules/game/main"), "names the entry to use instead: {}", refused.message);
}

#[test]
fn a_project_without_folders_compiles_as_before() {
    let tmp = game_project();
    let page = "export default function Page() { return <main>plain</main>; }\n";
    let without = compile_page(&tmp, &[], page);
    let options = super::super::CompileOptions {
        template_root: Some(tmp.path().display().to_string()),
        file_path: Some(tmp.path().join("pages/game.tsx").display().to_string()),
        ..super::super::CompileOptions::default()
    };
    let before = super::super::compile(page, options).expect("compiles");
    let after = without.expect("compiles");
    assert_eq!(before.client_module_source, after.client_module_source);
    assert_eq!(before.server_module_source, after.server_module_source);
    assert!(after.module_scripts.is_empty());
}

#[test]
fn a_worker_is_an_entry_started_by_its_address() {
    let tmp = project(&[
        ("modules/game/main.ts", "export function start() { return new Worker(new URL(\"./terrain.worker.ts\", import.meta.url), { type: \"module\" }); }\n"),
        ("modules/game/terrain.worker.ts", "import { height } from \"./noise\";\nself.onmessage = (e: MessageEvent<number>) => self.postMessage(height(e.data));\n"),
        ("modules/game/noise.ts", "export function height(x: number): number { return x * 2; }\n"),
    ]);
    let build = built(&tmp, &["main.ts", "*.worker.ts"]).expect("builds");
    parses(&build);
    let worker = &build.entries["@/modules/game/terrain.worker"];
    let main = script(&build, "@/modules/game/main");
    assert!(main.contains(&format!("new URL(\"./{worker}\", import.meta.url)")), "{main}");
    let worker_text = &build.scripts[worker].content;
    assert!(worker_text.contains("x * 2") && worker_text.contains("onmessage"), "the worker carries what it imports: {worker_text}");
    assert!(build.scripts_for(&["@/modules/game/main".to_string()]).iter().any(|s| &s.hash == worker), "a page that imports main gets the worker's script");

    let refused = built(&tmp, &["main.ts"]).expect_err("not an entry");
    assert!(refused.message.contains("starts a worker") && refused.message.contains("not an entry"), "{}", refused.message);
}

/// How long a game-sized folder takes to build: 300 files of about 200 lines
/// (60,000 lines), every one exporting `build` and `update`, four entries.
/// Ignored in the normal run; `cargo test --lib -- --ignored a_large_folder`.
#[test]
#[ignore]
fn a_large_folder_builds_in_reasonable_time() {
    let mut files: Vec<(String, String)> = Vec::new();
    for i in 0..300 {
        let mut text = format!("import {{ shared }} from \"../core/shared\";\nconst size = {i};\n");
        for j in 0..48 {
            text.push_str(&format!(
                "function step{j}(x: number): number {{\n  const k = x * {j} + size;\n  return shared(k) + {j};\n}}\n"
            ));
        }
        text.push_str("export function build(): number { return step0(1) + step47(2); }\nexport function update(dt: number): number { return dt * size; }\n");
        files.push((format!("modules/game/objects/o{i:03}.ts"), text));
    }
    files.push(("modules/game/core/shared.ts".to_string(), "export function shared(x: number): number { return x + 1; }\n".to_string()));
    for (name, from, to) in [("main", 0, 100), ("tools", 100, 200), ("world", 150, 250), ("admin", 250, 300)] {
        let mut text = String::new();
        for i in from..to {
            text.push_str(&format!("import {{ build as b{i} }} from \"./objects/o{i:03}\";\n"));
        }
        text.push_str(&format!("export function run(): number {{ return {}; }}\n", (from..to).map(|i| format!("b{i}()")).collect::<Vec<_>>().join(" + ")));
        files.push((format!("modules/game/{name}.ts"), text));
    }
    let refs: Vec<(&str, &str)> = files.iter().map(|(a, b)| (a.as_str(), b.as_str())).collect();
    let tmp = project(&refs);
    let lines: usize = files.iter().map(|(_, text)| text.lines().count()).sum();

    let started = std::time::Instant::now();
    let build = built(&tmp, &["main.ts", "tools.ts", "world.ts", "admin.ts"]).expect("builds");
    let cold = started.elapsed();
    let started = std::time::Instant::now();
    built(&tmp, &["main.ts", "tools.ts", "world.ts", "admin.ts"]).expect("builds");
    let unchanged = started.elapsed();
    std::thread::sleep(std::time::Duration::from_millis(10));
    fs::write(tmp.path().join("modules/game/objects/o042.ts"), files[42].1.replace("return dt * size", "return dt * size * 2")).unwrap();
    let started = std::time::Instant::now();
    built(&tmp, &["main.ts", "tools.ts", "world.ts", "admin.ts"]).expect("builds");
    let one_file = started.elapsed();
    let bytes: usize = build.scripts.values().map(|s| s.content.len()).sum();
    eprintln!(
        "{} files, {lines} lines → {} scripts, {bytes} bytes: cold {cold:?}, unchanged {unchanged:?}, one file changed {one_file:?}",
        files.len(),
        build.scripts.len()
    );
    parses(&build);
}
