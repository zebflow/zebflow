/// RWE SSR runtime — embedded V8 via deno_core (no external `deno` process).
///
/// One `JsRuntime` lives on a dedicated `std::thread`.  The thread runs its
/// own single-threaded Tokio executor so the async module-loading APIs work.
/// Communication uses:
///   • `tokio::sync::mpsc::UnboundedSender`  main → JS  (non-blocking send)
///   • `std::sync::mpsc::SyncSender`         JS → main  (blocking reply)
///
/// No V8 handle scopes are needed for the normal result handoff: the rendered
/// HTML is delivered from JS to Rust via a synchronous `#[op2]` that stores
/// into a thread-local slot. We do use short-lived V8 scopes to attach the
/// evaluated module namespace object for the current render so page identity is
/// local to that render, not ambient shared global state.
///
/// Public surface is identical to the old subprocess implementation.
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{LazyLock, Mutex};
use std::time::Duration;

use deno_core::v8;
use deno_core::{
    FastString, JsRuntime, ModuleId, ModuleLoadOptions, ModuleLoadReferrer, ModuleLoadResponse,
    ModuleLoader, ModuleSource, ModuleSourceCode, ModuleSpecifier, ModuleType,
    PollEventLoopOptions, ResolutionKind, RuntimeOptions,
};
use deno_error::JsErrorBox;
use serde_json::Value;

use super::error::EngineError;

// ---------------------------------------------------------------------------
// Embedded JS — installs preact SSR globals once at startup.
// ---------------------------------------------------------------------------
const PREACT_SSR_INIT: &str = include_str!("../runtime/preact_ssr_init.js");
const TOOL_INIT: &str = include_str!("../../language/runtime/tool_init.js");

// ---------------------------------------------------------------------------
// Thread-local result slot — JS op writes here; Rust reads after render.
// ---------------------------------------------------------------------------
thread_local! {
    static RENDER_RESULT: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Deno op: called by the render script to deliver the JSON result.
/// Runs synchronously within `execute_script`, so the slot is filled
/// before `execute_script` returns.
#[deno_core::op2(fast)]
fn op_rwe_store_result(#[string] json: String) {
    RENDER_RESULT.with(|r| *r.borrow_mut() = Some(json));
}

deno_core::extension!(rwe_ops, ops = [op_rwe_store_result],);

// ---------------------------------------------------------------------------
// Public result types (unchanged interface)
// ---------------------------------------------------------------------------
pub struct SsrResult {
    pub html: String,
    pub page_config: Option<Value>,
}

// ---------------------------------------------------------------------------
// Internal channel messages
// ---------------------------------------------------------------------------
enum JsOp {
    RenderSsr { source: String, ctx: Value },
}

struct JsRequest {
    op: JsOp,
    reply: std::sync::mpsc::SyncSender<Result<JsResponse, EngineError>>,
}

enum JsResponse {
    Rendered {
        html: String,
        page_config: Option<Value>,
    },
}

// ---------------------------------------------------------------------------
// Worker pool — N JS threads, round-robin dispatch, auto-respawn on death.
// ---------------------------------------------------------------------------
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);
/// Restart a V8 runtime after this many renders to prevent memory accumulation.
const RESTART_AFTER_RENDERS: u64 = 500;

struct WorkerPool {
    workers: Vec<Mutex<tokio::sync::mpsc::UnboundedSender<JsRequest>>>,
    next: AtomicUsize,
}

static WORKER_POOL: LazyLock<WorkerPool> = LazyLock::new(|| {
    let count = std::env::var("RWE_WORKER_COUNT")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(3)
        .max(1);
    eprintln!("rwe-js-runtime: starting {count} worker(s) (RWE_WORKER_COUNT={count})");
    let workers = (0..count).map(|i| Mutex::new(spawn_js_thread(i))).collect();
    WorkerPool {
        workers,
        next: AtomicUsize::new(0),
    }
});

fn spawn_js_thread(worker_id: usize) -> tokio::sync::mpsc::UnboundedSender<JsRequest> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<JsRequest>();
    std::thread::Builder::new()
        .name(format!("rwe-js-runtime-{worker_id}"))
        .spawn(move || run_js_thread(worker_id, rx))
        .expect("failed spawning rwe-js-runtime thread");
    tx
}

/// Get a JS channel from the pool (round-robin), respawning any dead worker slot.
fn get_channel() -> tokio::sync::mpsc::UnboundedSender<JsRequest> {
    let pool = &*WORKER_POOL;
    let i = pool.next.fetch_add(1, Ordering::Relaxed) % pool.workers.len();
    let mut guard = pool.workers[i].lock().unwrap_or_else(|e| e.into_inner());
    if guard.is_closed() {
        eprintln!("rwe-js-runtime[{i}] died — respawning");
        *guard = spawn_js_thread(i);
    }
    guard.clone()
}

fn run_js_thread(worker_id: usize, mut rx: tokio::sync::mpsc::UnboundedReceiver<JsRequest>) {
    // Clean up leftover temp modules from previous runs.
    cleanup_temp_modules();

    // Multi-threaded Tokio runtime (1 worker thread) for this JS thread.
    // `block_in_place` — used inside `catch_unwind` to run async SSR — requires a
    // multi-threaded runtime.  `new_current_thread` always panics with
    // "can call blocking only when running on the multi-threaded runtime".
    // One worker thread keeps overhead the same as current_thread while allowing
    // `block_in_place`.  JsRuntime (!Send) stays pinned to this OS thread because
    // `block_in_place` never moves the current closure to a different thread.
    let tokio_rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(1)
        .enable_all()
        .build()
        .expect("rwe-js-runtime: failed building tokio runtime");

    tokio_rt.block_on(async move {
        let mut js_rt = JsRuntime::new(RuntimeOptions {
            module_loader: Some(Rc::new(RweModuleLoader)),
            extensions: vec![rwe_ops::init()],
            ..Default::default()
        });

        // Load the preact SSR globals once.
        if let Err(e) = js_rt.execute_script(
            "<preact_ssr_init>",
            FastString::from_static(PREACT_SSR_INIT),
        ) {
            eprintln!("rwe-js-runtime[{worker_id}]: preact_ssr_init failed: {e}");
            return;
        }
        if let Err(e) = js_rt.execute_script("<tool_init>", FastString::from_static(TOOL_INIT)) {
            eprintln!("rwe-js-runtime[{worker_id}]: tool_init failed: {e}");
            return;
        }

        let mut render_count: u64 = 0;

        // Process requests sequentially (one SSR at a time per worker).
        while let Some(req) = rx.recv().await {
            // Periodic restart: spawn a replacement worker to handle this request,
            // then exit cleanly so the pool slot is refreshed on the next request.
            // The user never sees a RWE_RESTART error.
            if render_count >= RESTART_AFTER_RENDERS {
                eprintln!(
                    "rwe-js-runtime[{worker_id}]: scheduled restart after {render_count} renders \
                     — forwarding to fresh worker"
                );
                let one_shot_tx = spawn_js_thread(worker_id);
                // Forward the request; one_shot_tx drop after this fn closes the channel
                // once the one-shot worker has received and processed the message.
                let _ = one_shot_tx.send(req);
                return; // Exit current thread; pool slot respawns on next get_channel() call.
            }

            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                // We need a way to run async in catch_unwind. Use block_on nested.
                tokio::task::block_in_place(|| {
                    tokio::runtime::Handle::current().block_on(do_render_ssr(
                        &mut js_rt,
                        match &req.op {
                            JsOp::RenderSsr { source, .. } => source,
                        },
                        match &req.op {
                            JsOp::RenderSsr { ctx, .. } => ctx,
                        },
                    ))
                })
            }));

            let result = match result {
                Ok(r) => r,
                Err(_panic) => {
                    eprintln!(
                        "rwe-js-runtime[{worker_id}]: panic during render — thread will respawn"
                    );
                    let _ = req.reply.send(Err(EngineError::new(
                        "RWE_PANIC",
                        "SSR render panicked — runtime will respawn",
                    )));
                    return; // Exit thread; get_channel() will respawn on next request.
                }
            };
            let _ = req.reply.send(result);
            render_count += 1;
        }
    });
}

// ---------------------------------------------------------------------------
// SSR execution
// ---------------------------------------------------------------------------
async fn do_render_ssr(
    js_rt: &mut JsRuntime,
    tsx_source: &str,
    ctx: &Value,
) -> Result<JsResponse, EngineError> {
    // Reset result slot from any previous render.
    RENDER_RESULT.with(|r| *r.borrow_mut() = None);
    reset_render_globals(js_rt)?;

    // 1. Transpile TSX → plain JS (strip TypeScript, JSX → h() calls).
    let js = transpile_tsx(tsx_source)?;

    // 1b. Strip "rwe" and "rwe-*" runtime import lines.
    //     The compiler's rewrite_imports() leaves `import { ... } from 'rwe'` (and
    //     convention aliases like 'rwe-sky') verbatim because they are valid in the
    //     source. But our deno_core module loader only handles file:// URLs — it would
    //     try to resolve "rwe" as a relative path and fail with "no such file".
    //     All symbols from those imports are already on globalThis via preact_ssr_init.js,
    //     so stripping the lines is safe and correct.
    let js = strip_rwe_imports(&js);

    // 2. Inject render vars as globalThis.ctx (primary) and globalThis.input (compat).
    //     `ctx` is the canonical name — used in `export const page` config and top-level code.
    //     `input` is kept for backward compat (component function parameter convention).
    //     Must run before the module loads so variables are present during top-level eval.
    let ctx_json =
        serde_json::to_string(ctx).map_err(|e| EngineError::new("RWE_CTX_JSON", e.to_string()))?;
    let input_init_code = format!(
        "globalThis.ctx = {ctx_json}; globalThis.input = globalThis.ctx; globalThis.__islandCounter = 0;",
    );
    js_rt
        .execute_script("<rwe_ctx_init>", input_init_code)
        .map_err(|e| EngineError::new("RWE_CTX_INIT", e.to_string()))?;

    // 3. Write to a temp file so deno_core can load it as a file:// module.
    //    All component imports inside have already been resolved to absolute
    //    file paths by prepare_template_root / compiler.rs.
    let temp_path = write_temp_module(&js)?;
    let specifier = path_to_file_url(&temp_path)?;

    // 4. Load the module as a *side* module so the singleton JsRuntime can
    //    render multiple pages across its lifetime (load_main_es_module can only
    //    be called once — subsequent calls fail with "main module already exists").
    let module_id: ModuleId = js_rt
        .load_side_es_module(&specifier)
        .await
        .map_err(|e| EngineError::new("RWE_MODULE_LOAD", e.to_string()))?;

    // 5. Start module evaluation.
    let eval_fut = js_rt.mod_evaluate(module_id);

    // 6. Drive the event loop until all promises/imports resolve.
    js_rt
        .run_event_loop(PollEventLoopOptions::default())
        .await
        .map_err(|e| EngineError::new("RWE_EVENT_LOOP", e.to_string()))?;

    // 7. Await the module evaluation result.
    let _ = eval_fut.await;

    // Resolve the evaluated module's namespace and pin it only for this render.
    let module_namespace = js_rt
        .get_module_namespace(module_id)
        .map_err(|e| EngineError::new("RWE_MODULE_NAMESPACE", e.to_string()))?;
    set_render_module_namespace(js_rt, &module_namespace)?;

    // Clean up temp file (best-effort, before render script runs).
    let _ = std::fs::remove_file(&temp_path);

    // 8. Execute render script.
    //    It calls Deno.core.ops.op_rwe_store_result(json) synchronously,
    //    which stores the HTML+config JSON in our thread-local slot.
    let ctx_json =
        serde_json::to_string(ctx).map_err(|e| EngineError::new("RWE_CTX_JSON", e.to_string()))?;
    let render_code = format!(
        "(function(){{ \
           var __ns = globalThis.__rwe_module_ns || null; \
           if (!__ns || typeof __ns.default !== 'function') {{ \
             throw new Error('RWE render target missing default export'); \
           }} \
           function __mergePageConfig(base, dynamic) {{ \
             if (!base || typeof base !== 'object') return dynamic; \
             if (!dynamic || typeof dynamic !== 'object') return base; \
             var out = {{}}; \
             var key; \
             for (key in base) out[key] = base[key]; \
             for (key in dynamic) out[key] = dynamic[key]; \
             if (base.head || dynamic.head) out.head = Object.assign({{}}, base.head || {{}}, dynamic.head || {{}}); \
             if (base.html || dynamic.html) out.html = Object.assign({{}}, base.html || {{}}, dynamic.html || {{}}); \
             if (base.body || dynamic.body) out.body = Object.assign({{}}, base.body || {{}}, dynamic.body || {{}}); \
             return out; \
           }} \
           var __html = globalThis.__rweRenderToString(\
             globalThis.__rweWrapWithPageState(__ns.default, {ctx_json})\
           ); \
           var __baseCfg = (typeof __ns.page !== 'undefined' && __ns.page !== null) \
             ? __ns.page : null; \
           var __dynCfg = (typeof __ns.getPage === 'function') \
             ? __ns.getPage(globalThis.input) : null; \
           var __cfg = __mergePageConfig(__baseCfg, __dynCfg); \
           Deno.core.ops.op_rwe_store_result(JSON.stringify({{html: __html, page_config: __cfg}})); \
         }})()"
    );

    let render_result = js_rt
        .execute_script("<rwe_render>", render_code)
        .map_err(|e| EngineError::new("RWE_RENDER_EXEC", e.to_string()));
    reset_render_globals(js_rt)?;
    render_result?;

    // 9. Read the result from the thread-local slot (filled by the op above).
    let result_str = RENDER_RESULT
        .with(|r| r.borrow_mut().take())
        .ok_or_else(|| {
            EngineError::new(
                "RWE_RENDER_EXEC",
                "render op was not called — page may have no default export",
            )
        })?;

    let result_json: Value = serde_json::from_str(&result_str)
        .map_err(|e| EngineError::new("RWE_RESULT_JSON", e.to_string()))?;

    let html = result_json["html"].as_str().unwrap_or("").to_string();
    let raw_cfg = result_json["page_config"].clone();
    let page_config = if raw_cfg.is_null() {
        None
    } else {
        Some(raw_cfg)
    };

    Ok(JsResponse::Rendered { html, page_config })
}

// ---------------------------------------------------------------------------
// Public API (synchronous — same signatures as old subprocess wrapper)
// ---------------------------------------------------------------------------

pub fn render_ssr(
    module_source: &str,
    ctx: &Value,
    timeout_ms: u64,
) -> Result<SsrResult, EngineError> {
    let (reply_tx, reply_rx) = std::sync::mpsc::sync_channel(1);
    get_channel()
        .send(JsRequest {
            op: JsOp::RenderSsr {
                source: module_source.to_string(),
                ctx: ctx.clone(),
            },
            reply: reply_tx,
        })
        .map_err(|_| EngineError::new("RWE_CHANNEL", "js runtime channel disconnected"))?;

    let timeout = Duration::from_millis(timeout_ms.max(10_000));
    let response = reply_rx
        .recv_timeout(timeout)
        .map_err(|_| EngineError::new("RWE_TIMEOUT", "js render timed out"))?;

    match response? {
        JsResponse::Rendered { html, page_config } => Ok(SsrResult { html, page_config }),
    }
}

/// Non-blocking check: returns true if at least one worker slot is alive.
/// Used by the /ready health endpoint.
pub fn is_pool_ready() -> bool {
    let pool = &*WORKER_POOL;
    pool.workers
        .iter()
        .any(|slot| !slot.lock().unwrap_or_else(|e| e.into_inner()).is_closed())
}

/// Transpile a TSX/TSX source to plain browser JS (no JsRuntime needed).
pub fn transpile_client(module_source: &str, _timeout_ms: u64) -> Result<String, EngineError> {
    transpile_tsx(module_source)
}

// ---------------------------------------------------------------------------
// Custom module loader — resolves and transpiles file:// imports.
// ---------------------------------------------------------------------------
struct RweModuleLoader;

impl ModuleLoader for RweModuleLoader {
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        _kind: ResolutionKind,
    ) -> Result<ModuleSpecifier, JsErrorBox> {
        // Absolute paths written by prepare_template_root.
        if specifier.starts_with('/') {
            return ModuleSpecifier::parse(&format!("file://{specifier}"))
                .map_err(JsErrorBox::from_err);
        }
        // Relative or other specifiers — resolve against referrer URL.
        if let Ok(base) = ModuleSpecifier::parse(referrer) {
            if let Ok(resolved) = base.join(specifier) {
                return Ok(resolved);
            }
        }
        ModuleSpecifier::parse(specifier).map_err(JsErrorBox::from_err)
    }

    fn load(
        &self,
        module_specifier: &ModuleSpecifier,
        _maybe_referrer: Option<&ModuleLoadReferrer>,
        _options: ModuleLoadOptions,
    ) -> ModuleLoadResponse {
        let specifier = module_specifier.clone();
        ModuleLoadResponse::Sync(load_sync(&specifier))
    }
}

fn load_sync(specifier: &ModuleSpecifier) -> Result<ModuleSource, JsErrorBox> {
    if specifier.scheme() != "file" {
        return Err(JsErrorBox::generic(format!(
            "RWE module loader only handles file:// URLs; got: {specifier}"
        )));
    }

    let path = specifier
        .to_file_path()
        .map_err(|_| JsErrorBox::generic(format!("invalid file URL: {specifier}")))?;

    let source = std::fs::read_to_string(&path)
        .map_err(|e| JsErrorBox::generic(format!("failed reading {}: {e}", path.display())))?;

    // Transpile TypeScript / TSX → JS.
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let js = if matches!(ext, "tsx" | "ts" | "jsx") {
        transpile_tsx(&source).map_err(|e| {
            JsErrorBox::generic(format!("transpile {}: {}", path.display(), e.message))
        })?
    } else {
        source
    };

    Ok(ModuleSource::new(
        ModuleType::JavaScript,
        ModuleSourceCode::String(FastString::from(js)),
        specifier,
        None,
    ))
}

// ---------------------------------------------------------------------------
// TSX → JS transpilation via oxc
//
// Strips TypeScript types and converts JSX to `h(…)` calls (classic runtime)
// compatible with the globals installed by preact_ssr_init.js and by
// build_client_module() in render.rs.
// ---------------------------------------------------------------------------
pub fn transpile_tsx(source: &str) -> Result<String, EngineError> {
    use oxc_allocator::Allocator;
    use oxc_codegen::Codegen;
    use oxc_parser::Parser;
    use oxc_semantic::SemanticBuilder;
    use oxc_span::SourceType;
    use oxc_transformer::{JsxOptions, JsxRuntime, TransformOptions, Transformer};

    let alloc = Allocator::default();
    let source_type = SourceType::default()
        .with_module(true)
        .with_jsx(true)
        .with_typescript(true);

    let parsed = Parser::new(&alloc, source, source_type).parse();
    if parsed.panicked {
        let _ = std::fs::write("/tmp/rwe-transpile-failed.tsx", source);
        return Err(EngineError::new("RWE_TRANSPILE", "oxc parser panicked"));
    }

    let mut program = parsed.program;

    // Semantic analysis is required before transformation.
    let sem_ret = SemanticBuilder::new()
        .with_excess_capacity(2.0)
        .build(&program);

    let scoping = sem_ret.semantic.into_scoping();

    let options = TransformOptions {
        jsx: JsxOptions {
            runtime: JsxRuntime::Classic,
            pragma: Some("h".to_string()),
            pragma_frag: Some("Fragment".to_string()),
            ..JsxOptions::enable()
        },
        ..Default::default()
    };

    let source_path = Path::new("module.tsx");
    let transform_ret = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        Transformer::new(&alloc, source_path, &options).build_with_scoping(scoping, &mut program)
    }))
    .map_err(|_| EngineError::new("RWE_TRANSFORM", "oxc transformer panicked"))?;

    if !transform_ret.errors.is_empty() {
        let msg = transform_ret
            .errors
            .iter()
            .map(|e| format!("{e:?}"))
            .collect::<Vec<_>>()
            .join("; ");
        eprintln!("[RWE] transform diagnostics (non-fatal): {msg}");
    }

    Ok(Codegen::new().build(&program).code)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Strip `import { … } from "rwe"`, `import { … } from "rwe-*"`, and
/// `import { … } from "zeb/*"` lines before SSR.
///
/// SSR stubs for all `zeb/*` library symbols are installed once at JsRuntime
/// startup via `preact_ssr_init.js` as `globalThis.*` assignments — no
/// per-render injection needed here.
fn strip_rwe_imports(js: &str) -> String {
    js.lines()
        .filter(|line| {
            let t = line.trim();
            if !t.starts_with("import ") {
                return true;
            }
            !(t.contains("from \"zeb\"")
                || t.contains("from 'zeb'")
                || t.contains("from \"zeb/")
                || t.contains("from 'zeb/"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn reset_render_globals(js_rt: &mut JsRuntime) -> Result<(), EngineError> {
    js_rt
        .execute_script(
            "<rwe_reset_render_globals>",
            "delete globalThis.__rwe_module_ns; \
             delete globalThis.__rwe_page; \
             delete globalThis.__rwe_page_config; \
             delete globalThis.ctx; \
             delete globalThis.input; \
             globalThis.__islandCounter = 0;",
        )
        .map(|_| ())
        .map_err(|e| EngineError::new("RWE_RESET_GLOBALS", e.to_string()))
}

fn set_render_module_namespace(
    js_rt: &mut JsRuntime,
    module_namespace: &v8::Global<v8::Object>,
) -> Result<(), EngineError> {
    deno_core::scope!(scope, js_rt);
    let global = scope.get_current_context().global(scope);
    let key = v8::String::new(scope, "__rwe_module_ns")
        .ok_or_else(|| EngineError::new("RWE_MODULE_NAMESPACE", "failed creating v8 key"))?;
    let module_namespace = v8::Local::<v8::Object>::new(scope, module_namespace);
    global
        .set(scope, key.into(), module_namespace.into())
        .ok_or_else(|| {
            EngineError::new(
                "RWE_MODULE_NAMESPACE",
                "failed binding module namespace for render",
            )
        })?;
    Ok(())
}

/// Delete `/tmp/rwe-module-*.js` files older than 5 minutes.
/// Called at worker startup and can be triggered periodically to avoid disk exhaustion.
pub fn cleanup_temp_modules() {
    let tmp_dir = std::env::temp_dir();
    let cutoff = std::time::SystemTime::now()
        .checked_sub(std::time::Duration::from_secs(300))
        .unwrap_or(std::time::UNIX_EPOCH);
    if let Ok(entries) = std::fs::read_dir(&tmp_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str.starts_with("rwe-module-") && name_str.ends_with(".js") {
                if let Ok(meta) = entry.metadata() {
                    if let Ok(modified) = meta.modified() {
                        if modified < cutoff {
                            let _ = std::fs::remove_file(entry.path());
                        }
                    }
                }
            }
        }
    }
}

fn write_temp_module(js: &str) -> Result<std::path::PathBuf, EngineError> {
    use std::io::Write;
    let n = TEMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let mut path = std::env::temp_dir();
    path.push(format!("rwe-module-{n}.js"));
    let mut f = std::fs::File::create(&path)
        .map_err(|e| EngineError::new("RWE_TEMP_CREATE", e.to_string()))?;
    f.write_all(js.as_bytes())
        .map_err(|e| EngineError::new("RWE_TEMP_WRITE", e.to_string()))?;
    Ok(path)
}

fn path_to_file_url(path: &std::path::Path) -> Result<ModuleSpecifier, EngineError> {
    let path_str = path.to_string_lossy();
    let url_str = if path_str.starts_with('/') {
        format!("file://{path_str}")
    } else {
        format!("file:///{path_str}")
    };
    ModuleSpecifier::parse(&url_str)
        .map_err(|e| EngineError::new("RWE_URL", format!("invalid module URL: {e}")))
}

#[cfg(test)]
mod tests {
    use super::{render_ssr, transpile_client};
    use serde_json::json;

    #[test]
    fn render_ssr_does_not_leak_previous_page_identity_between_renders() {
        let first = r#"
            export const page = { title: "First" };
            export default function FirstPage() {
                return <section>FIRST_PAGE_ONLY</section>;
            }
        "#;
        let second = r#"
            export const page = { title: "Second" };
            export default function SecondPage() {
                return <main>SECOND_PAGE_ONLY</main>;
            }
        "#;

        let first_render = render_ssr(first, &json!({}), 10_000).expect("first render");
        assert!(first_render.html.contains("FIRST_PAGE_ONLY"));
        assert_eq!(first_render.page_config, Some(json!({ "title": "First" })));

        let second_render = render_ssr(second, &json!({}), 10_000).expect("second render");
        assert!(second_render.html.contains("SECOND_PAGE_ONLY"));
        assert!(!second_render.html.contains("FIRST_PAGE_ONLY"));
        assert_eq!(
            second_render.page_config,
            Some(json!({ "title": "Second" }))
        );
    }

    #[test]
    fn render_ssr_merges_static_page_with_get_page_input() {
        let source = r#"
            export const page = {
                html: { lang: "en" },
                body: { className: "base-body" },
                navigation: "history",
            };
            export function getPage(input) {
                return {
                    head: {
                        title: `${input.artist} — ${input.song}`,
                        description: `Lyrics for ${input.song}`,
                    },
                    body: { className: "dynamic-body" },
                };
            }
            export default function Page(input) {
                return <main>{input.song}</main>;
            }
        "#;

        let render = render_ssr(
            source,
            &json!({ "artist": "Aurora", "song": "Runaway" }),
            10_000,
        )
        .expect("render");
        assert!(render.html.contains("Runaway"));
        assert_eq!(
            render.page_config,
            Some(json!({
                "head": {
                    "title": "Aurora — Runaway",
                    "description": "Lyrics for Runaway"
                },
                "html": { "lang": "en" },
                "body": { "className": "dynamic-body" },
                "navigation": "history"
            }))
        );
    }

    #[test]
    fn render_ssr_serializes_style_object_props() {
        let source = r##"
            export default function Page() {
                return (
                    <div
                        style={{
                            width: "100%",
                            backgroundColor: "#fff",
                            "--track-size": "12px"
                        }}
                    >
                        inline-style-ok
                    </div>
                );
            }
        "##;

        let render = render_ssr(source, &json!({}), 10_000).expect("render");
        assert!(render.html.contains("inline-style-ok"));
        assert!(
            render
                .html
                .contains(r#"style="width:100%;background-color:#fff;--track-size:12px""#)
                || render
                    .html
                    .contains(r#"style="width:100%;background-color:#fff;--track-size:12px;""#),
            "expected SSR style object serialization, got {}",
            render.html
        );
    }

    #[test]
    fn transpile_client_preserves_style_object_for_hydration() {
        let source = r##"
            export default function Page() {
                return (
                    <div style={{ width: "100%", backgroundColor: "#fff", "--track-size": "12px" }}>
                        hydrated-style-ok
                    </div>
                );
            }
        "##;

        let js = transpile_client(source, 10_000).expect("transpile client");
        assert!(js.contains("style: {"));
        assert!(js.contains("width: \"100%\""));
        assert!(js.contains("backgroundColor: \"#fff\""));
        assert!(js.contains("\"--track-size\": \"12px\""));
    }
}
