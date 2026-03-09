/// RWE SSR runtime — embedded V8 via deno_core (no external `deno` process).
///
/// One `JsRuntime` lives on a dedicated `std::thread`.  The thread runs its
/// own single-threaded Tokio executor so the async module-loading APIs work.
/// Communication uses:
///   • `tokio::sync::mpsc::UnboundedSender`  main → JS  (non-blocking send)
///   • `std::sync::mpsc::SyncSender`         JS → main  (blocking reply)
///
/// No V8 handle scopes are needed: the rendered HTML is delivered from JS to
/// Rust via a synchronous `#[op2]` that stores into a thread-local slot.
/// The page component function is injected into `globalThis` by post-processing
/// the transpiled JS before it is loaded as a module.
///
/// Public surface is identical to the old subprocess implementation.
use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use deno_core::{
    FastString, JsRuntime, ModuleId, ModuleLoadOptions, ModuleLoadReferrer,
    ModuleLoadResponse, ModuleLoader, ModuleSource, ModuleSourceCode,
    ModuleSpecifier, ModuleType, PollEventLoopOptions,
    ResolutionKind, RuntimeOptions,
};
use deno_error::JsErrorBox;
use serde_json::Value;

use super::error::EngineError;

// ---------------------------------------------------------------------------
// Embedded JS — installs preact SSR globals once at startup.
// ---------------------------------------------------------------------------
const PREACT_SSR_INIT: &str = include_str!("../runtime/preact_ssr_init.js");

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

deno_core::extension!(
    rwe_ops,
    ops = [op_rwe_store_result],
);

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
    Rendered { html: String, page_config: Option<Value> },
}

// ---------------------------------------------------------------------------
// Singleton JS thread, started lazily on first use.
// ---------------------------------------------------------------------------
static TEMP_COUNTER: AtomicU64 = AtomicU64::new(0);

static JS_CHANNEL: LazyLock<tokio::sync::mpsc::UnboundedSender<JsRequest>> =
    LazyLock::new(|| {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<JsRequest>();
        std::thread::Builder::new()
            .name("rwe-js-runtime".into())
            .spawn(move || run_js_thread(rx))
            .expect("failed spawning rwe-js-runtime thread");
        tx
    });

fn run_js_thread(mut rx: tokio::sync::mpsc::UnboundedReceiver<JsRequest>) {
    // Dedicated single-threaded Tokio runtime for this thread.
    // JsRuntime is !Send so it must stay on this exact thread.
    let tokio_rt = tokio::runtime::Builder::new_current_thread()
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
        js_rt
            .execute_script("<preact_ssr_init>", FastString::from_static(PREACT_SSR_INIT))
            .expect("rwe-js-runtime: preact_ssr_init failed");

        // Process requests sequentially (one SSR at a time).
        while let Some(req) = rx.recv().await {
            let result = match req.op {
                JsOp::RenderSsr { source, ctx } => {
                    do_render_ssr(&mut js_rt, &source, &ctx).await
                }
            };
            let _ = req.reply.send(result);
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

    // 2. Inject globalThis assignments so we can read exports without handle_scope.
    //    `globalThis.__rwe_page` ← the default-exported component function.
    //    `globalThis.__rwe_page_config` ← the optional `export const page = { … }`.
    let js = inject_page_globals(&js);

    // 2b. Inject ctx as globalThis.input so top-level IIFE code in templates can
    //     reference `input` (e.g. `const seed = (input?.shared?.seed ?? 0)`).
    //     This must run before the module is loaded so the variable is present
    //     when the module's top-level statements execute.
    let input_init_code = format!(
        "globalThis.input = {};",
        serde_json::to_string(ctx).map_err(|e| EngineError::new("RWE_CTX_JSON", e.to_string()))?
    );
    js_rt
        .execute_script("<rwe_input_init>", input_init_code)
        .map_err(|e| EngineError::new("RWE_INPUT_INIT", e.to_string()))?;

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
    //    The module's top-level code runs here, setting globalThis.__rwe_page.
    js_rt
        .run_event_loop(PollEventLoopOptions::default())
        .await
        .map_err(|e| EngineError::new("RWE_EVENT_LOOP", e.to_string()))?;

    // 7. Await the module evaluation result.
    let _ = eval_fut.await;

    // Clean up temp file (best-effort, before render script runs).
    let _ = std::fs::remove_file(&temp_path);

    // 8. Execute render script.
    //    It calls Deno.core.ops.op_rwe_store_result(json) synchronously,
    //    which stores the HTML+config JSON in our thread-local slot.
    let ctx_json = serde_json::to_string(ctx)
        .map_err(|e| EngineError::new("RWE_CTX_JSON", e.to_string()))?;
    let render_code = format!(
        "(function(){{ \
           var __html = globalThis.__rweRenderToString(\
             globalThis.__rweWrapWithPageState(globalThis.__rwe_page, {ctx_json})\
           ); \
           var __cfg = (typeof globalThis.__rwe_page_config !== 'undefined' \
                        && globalThis.__rwe_page_config !== null) \
             ? globalThis.__rwe_page_config : null; \
           Deno.core.ops.op_rwe_store_result(JSON.stringify({{html: __html, page_config: __cfg}})); \
         }})()"
    );

    js_rt
        .execute_script("<rwe_render>", render_code)
        .map_err(|e| EngineError::new("RWE_RENDER_EXEC", e.to_string()))?;

    // 9. Read the result from the thread-local slot (filled by the op above).
    let result_str = RENDER_RESULT.with(|r| r.borrow_mut().take())
        .ok_or_else(|| EngineError::new("RWE_RENDER_EXEC", "render op was not called — page may have no default export"))?;

    let result_json: Value = serde_json::from_str(&result_str)
        .map_err(|e| EngineError::new("RWE_RESULT_JSON", e.to_string()))?;

    let html = result_json["html"].as_str().unwrap_or("").to_string();
    let raw_cfg = result_json["page_config"].clone();
    let page_config = if raw_cfg.is_null() { None } else { Some(raw_cfg) };

    Ok(JsResponse::Rendered { html, page_config })
}

// ---------------------------------------------------------------------------
// inject_page_globals — post-process transpiled JS so the page component
// and optional page-config object are exposed on globalThis without needing
// a V8 handle scope.
//
// After oxc TypeScript + JSX transformation the entry module contains:
//   export default function MyPage(props) { … }
//   export const page = { title: "…" };          ← optional
//
// We scan for the default-export function/class name and append:
//   globalThis.__rwe_page = MyPage;
//   globalThis.__rwe_page_config = typeof page !== 'undefined' ? page : undefined;
//
// The ES module's `export` statements are left intact so the module is still
// valid for the loader; the appended assignments are simply side-effects that
// execute during module evaluation.
// ---------------------------------------------------------------------------
fn extract_default_export_name(js: &str) -> Option<String> {
    for line in js.lines() {
        let t = line.trim();

        // export default function Name(
        // export default function Name<T>(
        if let Some(rest) = t.strip_prefix("export default function ") {
            let end = rest
                .find(|c: char| c == '(' || c == '<' || c == '{' || c == ' ')
                .unwrap_or(rest.len());
            let name = rest[..end].trim().to_string();
            if !name.is_empty() {
                return Some(name);
            }
        }

        // export default class Name
        if let Some(rest) = t.strip_prefix("export default class ") {
            let end = rest
                .find(|c: char| !c.is_alphanumeric() && c != '_')
                .unwrap_or(rest.len());
            let name = rest[..end].trim().to_string();
            if !name.is_empty() {
                return Some(name);
            }
        }

        // export { Name as default } or export { Name as default, … }
        if t.starts_with("export {") && t.contains(" as default") {
            if let Some(pos) = t.find(" as default") {
                let before = &t[..pos];
                // Last identifier token before " as default"
                if let Some(name) = before.split(|c: char| !c.is_alphanumeric() && c != '_').last()
                {
                    let name = name.trim().to_string();
                    if !name.is_empty() {
                        return Some(name);
                    }
                }
            }
        }
    }
    None
}

fn inject_page_globals(js: &str) -> String {
    let name = extract_default_export_name(js);
    let mut out = js.to_string();
    if !out.ends_with('\n') {
        out.push('\n');
    }
    if let Some(n) = name {
        out.push_str(&format!("globalThis.__rwe_page = {n};\n"));
    }
    // Inject page_config regardless — JS `typeof` guard handles the absent case.
    out.push_str(
        "globalThis.__rwe_page_config = \
         (typeof page !== 'undefined') ? page : undefined;\n",
    );
    out
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
    JS_CHANNEL
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
        transpile_tsx(&source)
            .map_err(|e| JsErrorBox::generic(format!("transpile {}: {}", path.display(), e.message)))?
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
    let transform_ret = Transformer::new(&alloc, source_path, &options)
        .build_with_scoping(scoping, &mut program);

    if !transform_ret.errors.is_empty() {
        let msg = transform_ret
            .errors
            .iter()
            .map(|e| format!("{e:?}"))
            .collect::<Vec<_>>()
            .join("; ");
        return Err(EngineError::new("RWE_TRANSFORM", format!("transform errors: {msg}")));
    }

    Ok(Codegen::new().build(&program).code)
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Strip `import { … } from "rwe"` and `import { … } from "rwe-*"` lines.
///
/// The compiler leaves these verbatim (they are valid specifiers in TSX source).
/// In the embedded deno_core runtime all "rwe" symbols are already installed as
/// `globalThis.*` by `preact_ssr_init.js`, so we just drop the import lines.
/// Without stripping, deno_core's module loader tries to open a file at
/// `<temp_dir>/rwe` (or `<temp_dir>/rwe-sky`, etc.) and fails.
fn strip_rwe_imports(js: &str) -> String {
    js.lines()
        .filter(|line| {
            let t = line.trim();
            if !t.starts_with("import ") {
                return true;
            }
            // Keep every import that is NOT from "rwe" / 'rwe' / "rwe-anything" / 'rwe-anything'
            !(t.contains("from \"rwe\"")
                || t.contains("from 'rwe'")
                || t.contains("from \"rwe-")
                || t.contains("from 'rwe-"))
        })
        .collect::<Vec<_>>()
        .join("\n")
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
