use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use serde_json::{Value, json};

use super::deno_worker;
use super::error::EngineError;
use super::model::{CompiledTemplate, RenderMeta, RenderOutput};

const ROOT_ID: &str = "__rwe_root";
const PAYLOAD_ID: &str = "__rwe_payload";
const TOOL_INIT: &str = include_str!("../../language/runtime/tool_init.js");
static CLIENT_TRANSPILE_CACHE: LazyLock<Mutex<HashMap<u64, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

// ---------------------------------------------------------------------------
// Phase 2a: SSR result cache
// ---------------------------------------------------------------------------

struct SsrCacheEntry {
    html: String,
    page_config: Option<Value>,
    expires_at: Instant,
}

struct SsrCache {
    entries: Mutex<HashMap<u64, SsrCacheEntry>>,
}

impl SsrCache {
    fn new() -> Self {
        Self {
            entries: Mutex::new(HashMap::new()),
        }
    }

    fn get(&self, key: u64) -> Option<(String, Option<Value>)> {
        let mut guard = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(entry) = guard.get(&key) {
            if entry.expires_at > Instant::now() {
                return Some((entry.html.clone(), entry.page_config.clone()));
            }
        }
        guard.remove(&key);
        None
    }

    fn insert(&self, key: u64, html: String, page_config: Option<Value>, ttl: Duration) {
        let mut guard = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        // Evict expired entries first; fall back to a full clear if still over capacity.
        if guard.len() >= 200 {
            let now = Instant::now();
            guard.retain(|_, v| v.expires_at > now);
            if guard.len() >= 200 {
                guard.clear();
            }
        }
        guard.insert(
            key,
            SsrCacheEntry {
                html,
                page_config,
                expires_at: Instant::now() + ttl,
            },
        );
    }
}

static SSR_CACHE: LazyLock<SsrCache> = LazyLock::new(SsrCache::new);

// ---------------------------------------------------------------------------
// Phase 2b: Circuit breaker per template
// ---------------------------------------------------------------------------

struct CircuitState {
    failures: u32,
    open_until: Option<Instant>,
}

static CIRCUIT: LazyLock<Mutex<HashMap<String, CircuitState>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

const CIRCUIT_FAILURE_THRESHOLD: u32 = 3;
const CIRCUIT_OPEN_SECS: u64 = 30;

/// Check if the circuit is open (i.e. this template is failing fast).
/// Returns Some(error_message) if the circuit is open, None if safe to proceed.
fn circuit_check(template_id: &str) -> Option<String> {
    let guard = CIRCUIT.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(state) = guard.get(template_id) {
        if let Some(open_until) = state.open_until {
            if Instant::now() < open_until {
                return Some(format!(
                    "template '{template_id}' circuit breaker open — render skipped for {}s",
                    CIRCUIT_OPEN_SECS,
                ));
            }
        }
    }
    None
}

/// Record a render failure for a template; open the circuit after the threshold.
fn circuit_record_failure(template_id: &str) {
    let mut guard = CIRCUIT.lock().unwrap_or_else(|e| e.into_inner());
    let state = guard
        .entry(template_id.to_string())
        .or_insert(CircuitState {
            failures: 0,
            open_until: None,
        });
    state.failures += 1;
    if state.failures >= CIRCUIT_FAILURE_THRESHOLD {
        let open_until = Instant::now() + Duration::from_secs(CIRCUIT_OPEN_SECS);
        eprintln!(
            "rwe: circuit breaker OPEN for '{}' after {} failures — cooling down for {}s",
            template_id, state.failures, CIRCUIT_OPEN_SECS,
        );
        state.open_until = Some(open_until);
        state.failures = 0; // reset counter so next window starts fresh
    }
}

/// Record a successful render — reset the failure counter and close the circuit.
fn circuit_record_success(template_id: &str) {
    let mut guard = CIRCUIT.lock().unwrap_or_else(|e| e.into_inner());
    guard.remove(template_id);
}

pub fn prewarm(compiled: &CompiledTemplate) -> Result<(), EngineError> {
    let _ = transpile_client_cached(&compiled.client_module_source, compiled.deno_timeout_ms)?;
    let _ = deno_worker::render_ssr(
        &compiled.server_module_source,
        &json!({}),
        compiled.deno_timeout_ms,
    )?;
    Ok(())
}

pub fn render(
    compiled: &CompiledTemplate,
    vars: &Value,
    enabled_libraries: &[String],
) -> Result<RenderOutput, EngineError> {
    let started = Instant::now();

    // Template ID for circuit breaker and cache key derivation.
    let template_id = compiled.source_path.as_deref().unwrap_or("unknown");

    // Phase 2b: Check circuit breaker before touching V8.
    if let Some(cb_msg) = circuit_check(template_id) {
        return Err(EngineError::new("RWE_CIRCUIT_OPEN", cb_msg));
    }

    // Phase 2a: Check SSR cache.
    let ssr_cache_ttl_secs = std::env::var("RWE_SSR_CACHE_TTL_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(30);
    let vars_json = serde_json::to_string(vars).unwrap_or_default();
    let ssr_cache_key =
        stable_hash_u64(&compiled.server_module_source) ^ stable_hash_u64(&vars_json);

    let ssr = if let Some((cached_html, cached_config)) = SSR_CACHE.get(ssr_cache_key) {
        deno_worker::SsrResult {
            html: cached_html,
            page_config: cached_config,
        }
    } else {
        match deno_worker::render_ssr(
            &compiled.server_module_source,
            vars,
            compiled.deno_timeout_ms,
        ) {
            Ok(result) => {
                // Cache the successful result.
                SSR_CACHE.insert(
                    ssr_cache_key,
                    result.html.clone(),
                    result.page_config.clone(),
                    Duration::from_secs(ssr_cache_ttl_secs),
                );
                circuit_record_success(template_id);
                result
            }
            Err(e) => {
                circuit_record_failure(template_id);
                return Err(e);
            }
        }
    };
    // Use detected_zeb_libs collected at compile time (includes libs from all inlined components).
    let zeb_preamble = build_zeb_preamble(&compiled.detected_zeb_libs, enabled_libraries);
    let transpiled_client =
        transpile_client_cached(&compiled.client_module_source, compiled.deno_timeout_ms)?;
    let ssr_ms = started.elapsed().as_millis();

    let payload_json = serde_json::to_string(vars).map_err(|e| {
        EngineError::new(
            "RWE_PAYLOAD_JSON",
            format!("failed serializing hydration payload: {e}"),
        )
    })?;

    let body_content = format!(
        "<div id=\"{ROOT_ID}\">{}</div><script type=\"application/json\" id=\"{PAYLOAD_ID}\">{}</script>",
        ssr.html,
        escape_json_script(&payload_json)
    );

    let site = SiteContext::from_state(vars);
    let mut html = build_document_shell(&ssr.page_config, &body_content, &site);
    for (idx, css) in compiled.inline_styles.iter().enumerate() {
        if css.trim().is_empty() {
            continue;
        }
        let style_block = format!("<style data-rwe-style=\"{idx}\">{css}</style>");
        if let Some(pos) = html.find("</head>") {
            html.insert_str(pos, &style_block);
        } else {
            html = format!("{style_block}{html}");
        }
    }

    let js = build_client_module(&transpiled_client, &zeb_preamble);

    Ok(RenderOutput {
        html,
        js: js.clone(),
        css: String::new(),
        hydration_payload: json!({
            "engine": "rwe",
            "mode": format!("{:?}", compiled.hydrate_mode).to_lowercase(),
            "payloadId": PAYLOAD_ID,
            "rootId": ROOT_ID,
        }),
        meta: RenderMeta {
            html_bytes: ssr.html.len(),
            js_bytes: js.len(),
            css_bytes: 0,
            ssr_ms,
        },
    })
}

fn strip_rwe_client_imports(source: &str) -> String {
    // By AST span, not by line: the exports of a `zeb/*` runtime library are
    // globals the outer script installs, so the declaration goes; but a line
    // that merely *looks* like one — a code sample inside a template literal
    // — stays. The line filter used to eat the gallery's own examples.
    use oxc_allocator::Allocator;
    use oxc_ast::ast::Statement;
    use oxc_parser::Parser;
    use oxc_span::SourceType;

    let alloc = Allocator::default();
    let source_type = SourceType::default()
        .with_module(true)
        .with_jsx(true)
        .with_typescript(true);
    let parsed = Parser::new(&alloc, source, source_type).parse();
    if parsed.panicked {
        return strip_rwe_client_imports_by_line(source);
    }
    let mut ranges: Vec<(usize, usize)> = Vec::new();
    for stmt in &parsed.program.body {
        if let Statement::ImportDeclaration(import) = stmt {
            let spec = import.source.value.as_str();
            if spec == "zeb" || spec.starts_with("zeb/") {
                let start = import.span.start as usize;
                let mut end = import.span.end as usize;
                if end < source.len() && source.as_bytes()[end] == b'\n' {
                    end += 1;
                }
                ranges.push((start, end));
            }
        }
    }
    if ranges.is_empty() {
        return source.to_string();
    }
    let mut out = String::with_capacity(source.len());
    let mut cursor = 0;
    for (start, end) in ranges {
        out.push_str(&source[cursor..start]);
        cursor = end;
    }
    out.push_str(&source[cursor..]);
    out
}

/// The old line filter, kept only for a source oxc cannot parse.
fn strip_rwe_client_imports_by_line(source: &str) -> String {
    let logical = join_import_lines(source);
    logical
        .lines()
        .filter(|line| {
            let t = line.trim();
            if !t.starts_with("import ") {
                return true;
            }
            !(t.contains("from \"zeb/react\"")
                || t.contains("from 'zeb'")
                || t.contains("from \"zeb/")
                || t.contains("from 'zeb/"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Collapse multi-line `import { … } from "…"` statements into a single line each.
fn join_import_lines(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut buf: Option<String> = None;
    for line in source.lines() {
        let t = line.trim();
        if let Some(ref mut acc) = buf {
            acc.push(' ');
            acc.push_str(t);
            if t.contains("from \"") || t.contains("from '") {
                out.push_str(acc);
                out.push('\n');
                buf = None;
            }
        } else if t.starts_with("import ") && t.contains('{') && !t.contains("from ") {
            buf = Some(t.to_string());
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }
    if let Some(acc) = buf {
        out.push_str(&acc);
        out.push('\n');
    }
    out
}

/// Map a `zeb/*` library specifier to its versioned browser bundle URL.
/// TODO: resolve version from project `zeb.lock` instead of hardcoding.
fn zeb_bundle_url(lib: &str) -> Option<&'static str> {
    match lib {
        "zeb/threejs" => Some("/assets/libraries/zeb/threejs/0.1/runtime/threejs.bundle.mjs"),
        "zeb/threejs-vrm" => {
            Some("/assets/libraries/zeb/threejs-vrm/0.1/runtime/threejs-vrm.bundle.mjs")
        }
        "zeb/d3" => Some("/assets/libraries/zeb/d3/0.1/runtime/d3.bundle.mjs"),
        "zeb/deckgl" => Some("/assets/libraries/zeb/deckgl/0.1/runtime/deckgl.patched.mjs"),
        "zeb/codemirror" => Some("/assets/libraries/zeb/codemirror/0.1/runtime/entry.mjs"),
        "zeb/graphui" => Some("/assets/libraries/zeb/graphui/0.1/runtime/graphui.bundle.mjs"),
        "zeb/markdown" => Some("/assets/libraries/zeb/markdown/0.1/runtime/markdown.bundle.mjs"),
        "zeb/prosemirror" => {
            Some("/assets/libraries/zeb/prosemirror/0.1/runtime/prosemirror.bundle.mjs")
        }
        "zeb/use" => Some("/assets/libraries/zeb/use/0.1/runtime/use.bundle.mjs"),
        "zeb/livegeo" => Some("/assets/libraries/zeb/livegeo/0.1/runtime/livegeo.bundle.mjs"),
        "zeb/pdf" => Some("/assets/libraries/zeb/pdf/0.1/runtime/pdf.bundle.mjs"),
        _ => None,
    }
}

/// Build the outer-script preamble that imports each used `zeb/*` library
/// from its versioned bundle URL and assigns all exports onto `globalThis`.
///
/// This runs in the outer script (a real URL context), so absolute-path
/// imports like `/assets/libraries/…` resolve correctly. The inner user
/// bundle (loaded as a `data:` URL) then just uses the symbols as globals —
/// its `import { … } from "zeb/*"` lines are stripped by `strip_rwe_client_imports`.
fn build_zeb_preamble(detected_libs: &[String], enabled_libraries: &[String]) -> String {
    let libs = detected_libs;
    if libs.is_empty() {
        return String::new();
    }
    // Use dynamic `await import(...)` — NOT static `import * as ...`.
    // Static imports are hoisted: the bundle would be evaluated before the outer
    // script body runs, so globalThis.createContext (etc.) wouldn't be set yet.
    // Dynamic imports run in-order during script body execution, after the Zeb React
    // globals have been installed above.
    let mut out = String::new();
    for lib in libs {
        // Non-empty enabled list = strict mode: skip unlisted libraries.
        if !enabled_libraries.is_empty() && !enabled_libraries.contains(lib) {
            continue;
        }
        if let Some(url) = zeb_bundle_url(lib) {
            let var = lib.replace('/', "_").replace('-', "_");
            out.push_str(&format!(
                "const __{var} = await import('{url}');\nObject.assign(globalThis, __{var});\n"
            ));
            if lib == "zeb/markdown" {
                out.push_str(
                    "globalThis.Markdown = function Markdown(props) {\n\
                     const text = (props && props.content) || (typeof (props && props.children) === 'string' ? props.children : '') || '';\n\
                     const extra = (props && (props.className || props.class)) || '';\n\
                     const className = 'prose' + (extra ? ' ' + extra : '');\n\
                     return h('div', {\n\
                       className,\n\
                       dangerouslySetInnerHTML: { __html: globalThis.renderMarkdown ? globalThis.renderMarkdown(text) : '' }\n\
                     });\n\
                   };\n",
                );
            }
        }
    }
    out
}

fn build_client_module(client_source: &str, zeb_preamble: &str) -> String {
    let tool_js = TOOL_INIT;
    // No `npm:preact` rewriting. A template reaches the runtime through
    // "zeb/react" and nothing else; there is no second spelling to translate.
    let runtime_ready_source = strip_rwe_client_imports(client_source);
    let encoded = STANDARD.encode(runtime_ready_source.as_bytes());
    format!(
        "{tool_js}\n\
         import {{ h, Fragment, hydrate, render, createContext, createPortal, forwardRef, memo,\
           useCallback, useContext, useEffect, useId, useImperativeHandle,\
           useLayoutEffect, useMemo, useReducer, useRef, useState,\
           useSyncExternalStore }}\
           from '/assets/libraries/zeb/react/0.1/runtime/zeb_react.mjs';\n\
         const __RwePageStateContext = createContext(null);\n\
         function __rweUsePageState(keyOrInitial, defaultValue) {{\n\
           const isKeyed = typeof keyOrInitial === 'string';\n\
           const ctx = useContext(__RwePageStateContext);\n\
           const [state, setState] = useState(\n\
             isKeyed ? {{ [keyOrInitial]: defaultValue }} : (keyOrInitial || {{}})\n\
           );\n\
           if (isKeyed) {{\n\
             const key = keyOrInitial;\n\
             if (ctx) {{\n\
               const value = key in ctx ? ctx[key] : defaultValue;\n\
               const setter = (v) => ctx.setPageState({{ [key]: v }});\n\
               return [value, setter];\n\
             }}\n\
             const value = state[key] !== undefined ? state[key] : defaultValue;\n\
             const setter = (v) => setState((prev) => ({{ ...prev, [key]: v }}));\n\
             return [value, setter];\n\
           }}\n\
           if (ctx) return ctx;\n\
           const setPageState = (patch) => {{\n\
             if (typeof patch === 'function') {{\n\
               setState((prev) => ({{ ...(prev || {{}}), ...((patch(prev || {{}})) || {{}}) }}));\n\
               return;\n\
             }}\n\
             setState((prev) => ({{ ...(prev || {{}}), ...((patch) || {{}}) }}));\n\
           }};\n\
           return {{ ...(state || {{}}), setPageState }};\n\
         }}\n\
         globalThis.h = h;\n\
         globalThis.Fragment = Fragment;\n\
         globalThis.React = {{ createElement: h, Fragment }};\n\
         globalThis.useState = useState;\n\
         globalThis.useEffect = useEffect;\n\
         globalThis.useRef = useRef;\n\
         globalThis.useMemo = useMemo;\n\
         globalThis.useCallback = useCallback;\n\
         globalThis.useContext = useContext;\n\
         globalThis.useReducer = useReducer;\n\
         globalThis.useId = useId;\n\
         globalThis.useImperativeHandle = useImperativeHandle;\n\
         globalThis.useLayoutEffect = useLayoutEffect;\n\
         globalThis.useSyncExternalStore = useSyncExternalStore;\n\
         globalThis.createContext = createContext;\n\
         globalThis.createPortal = createPortal;\n\
         globalThis.forwardRef = forwardRef;\n\
         globalThis.memo = memo;\n\
         globalThis.usePageState = __rweUsePageState;\n\
         // Where we are. Both subscribe rather than reading location once,\n\
         // because Link swaps the page without a reload — a component that read\n\
         // window.location directly would keep rendering the previous URL.\n\
         // `rwe:nav` fires after a router navigation, `popstate` after Back.\n\
         (function() {{\n\
           var listeners = new Set();\n\
           var announce = function() {{ listeners.forEach(function(fn) {{ fn(); }}); }};\n\
           window.addEventListener('rwe:nav', announce);\n\
           window.addEventListener('popstate', announce);\n\
           var subscribe = function(fn) {{\n\
             listeners.add(fn);\n\
             return function() {{ listeners.delete(fn); }};\n\
           }};\n\
           globalThis.usePathname = function usePathname() {{\n\
             return useSyncExternalStore(\n\
               subscribe,\n\
               function() {{ return window.location.pathname; }},\n\
               function() {{ return (globalThis.ctx && globalThis.ctx.route) || '/'; }}\n\
             );\n\
           }};\n\
           // The snapshot is the raw query string, not a URLSearchParams: the\n\
           // hook compares snapshots by identity, and a fresh object every read\n\
           // would re-render forever.\n\
           globalThis.useSearchParams = function useSearchParams() {{\n\
             var search = useSyncExternalStore(\n\
               subscribe,\n\
               function() {{ return window.location.search; }},\n\
               function() {{\n\
                 if (globalThis.ctx && typeof globalThis.ctx.search === 'string') return globalThis.ctx.search;\n\
                 var q = (globalThis.ctx && globalThis.ctx.query) || {{}};\n\
                 var p = new URLSearchParams();\n\
                 for (var k in q) {{\n\
                   if (Object.prototype.hasOwnProperty.call(q, k)) p.set(k, String(q[k]));\n\
                 }}\n\
                 var s = p.toString();\n\
                 return s ? '?' + s : '';\n\
               }}\n\
             );\n\
             // Stable identity matters for effects that depend on params.\n\
             // Expose the same read-only contract as the embedded SSR view.\n\
             return useMemo(function() {{\n\
               var params = new URLSearchParams(search);\n\
               var readonly = function() {{ throw new TypeError('Search params are read-only; use router.push or router.replace.'); }};\n\
               Object.defineProperties(params, {{\n\
                 append: {{ value: readonly }}, delete: {{ value: readonly }},\n\
                 set: {{ value: readonly }}, sort: {{ value: readonly }}\n\
               }});\n\
               return Object.freeze(params);\n\
             }}, [search]);\n\
           }};\n\
         }})();\n\
         // Next.js App Router's shape: push/replace/back/forward/refresh/prefetch.\n\
         // Deliberately no pathname or query — those belong to the legacy Pages\n\
         // Router, and window.location already answers them without a hook.\n\
         globalThis.useRouter = function useRouter() {{\n\
           var go = function(href, mode) {{\n\
             if (typeof window.rweNavigate === 'function') {{\n\
               window.rweNavigate(href, mode);\n\
             }} else if (mode === 'replace') {{\n\
               window.location.replace(href);\n\
             }} else {{\n\
               window.location.href = href;\n\
             }}\n\
           }};\n\
           return {{\n\
             push: function(href) {{ go(href, 'push'); }},\n\
             replace: function(href) {{ go(href, 'replace'); }},\n\
             back: function() {{ history.back(); }},\n\
             forward: function() {{ history.forward(); }},\n\
             // Re-render where we already are, without recording a visit to it.\n\
             refresh: function() {{\n\
               go(window.location.pathname + window.location.search, 'none');\n\
             }},\n\
             // A hint, not a promise: warm the HTTP cache and ignore failures,\n\
             // exactly as a prefetch that cannot help should behave.\n\
             prefetch: function(href) {{\n\
               try {{ fetch(href, {{ credentials: 'same-origin' }}).catch(function() {{}}); }}\n\
               catch (e) {{}}\n\
             }}\n\
           }};\n\
         }};\n\
         globalThis.Link = function Link({{ href, children, className, ...props }}) {{\n\
           return h('a', {{\n\
             href,\n\
             className,\n\
             onClick: function(e) {{\n\
               e.preventDefault();\n\
               if (typeof window.rweNavigate === 'function') {{\n\
                 window.rweNavigate(href);\n\
               }} else {{\n\
                 window.location.href = href;\n\
               }}\n\
             }},\n\
             ...props\n\
           }}, children);\n\
         }};\n\
         globalThis.cx = function cx(...parts) {{ return parts.filter(Boolean).join(' '); }};\n\
         (function() {{\n\
           if (typeof window.rweNavigate !== 'function') {{\n\
             var __bar = document.createElement('div');\n\
             __bar.id = '__rwe_nav_bar';\n\
             __bar.style.cssText = 'position:fixed;top:0;left:0;height:3px;width:0%;background:var(--rwe-nav-color,#005b9a);z-index:99999;opacity:0;pointer-events:none;transition:none';\n\
             document.body.appendChild(__bar);\n\
             var __bt = null;\n\
             var __bStart = function() {{\n\
               clearTimeout(__bt);\n\
               __bar.style.transition = 'none';\n\
               __bar.style.width = '0%';\n\
               __bar.style.opacity = '1';\n\
               __bar.offsetWidth;\n\
               __bar.style.transition = 'width 0.25s ease';\n\
               __bar.style.width = '30%';\n\
               __bt = setTimeout(function() {{\n\
                 __bar.style.transition = 'width 1.5s ease';\n\
                 __bar.style.width = '70%';\n\
               }}, 250);\n\
             }};\n\
             var __bDone = function() {{\n\
               clearTimeout(__bt);\n\
               __bar.style.transition = 'width 0.1s ease';\n\
               __bar.style.width = '100%';\n\
               __bt = setTimeout(function() {{\n\
                 __bar.style.transition = 'opacity 0.2s ease';\n\
                 __bar.style.opacity = '0';\n\
               }}, 150);\n\
             }};\n\
             var __bFail = function() {{\n\
               clearTimeout(__bt);\n\
               __bar.style.transition = 'opacity 0.15s ease';\n\
               __bar.style.opacity = '0';\n\
             }};\n\
             window.rweNavigate = function(href, mode) {{\n\
               __bStart();\n\
               fetch(href, {{ credentials: 'same-origin' }})\n\
                 .then(function(r) {{\n\
                   if (!r.ok) {{ __bFail(); window.location.href = href; return null; }}\n\
                   return r.text();\n\
                 }})\n\
                 .then(function(html) {{\n\
                   if (!html) return;\n\
                   var doc = new DOMParser().parseFromString(html, 'text/html');\n\
                   var nRoot = doc.getElementById('{ROOT_ID}');\n\
                   var nPay = doc.getElementById('{PAYLOAD_ID}');\n\
                   var lRoot = document.getElementById('{ROOT_ID}');\n\
                   var lPay = document.getElementById('{PAYLOAD_ID}');\n\
                   if (nRoot && lRoot) {{\n\
                     render(null, lRoot);\n\
                     render(null, lRoot);\n\
                     lRoot.innerHTML = nRoot.innerHTML;\n\
                     if (nPay && lPay) lPay.textContent = nPay.textContent;\n\
                   }}\n\
                   document.querySelectorAll('style[data-rwe-tw], style[data-rwe-style]').forEach(function(s) {{ s.remove(); }});\n\
                   doc.querySelectorAll('style[data-rwe-tw]').forEach(function(s) {{\n\
                     var nc = document.createElement('style');\n\
                     nc.setAttribute('data-rwe-tw', '');\n\
                     nc.textContent = s.textContent;\n\
                     document.head.appendChild(nc);\n\
                   }});\n\
                   doc.querySelectorAll('style[data-rwe-style]').forEach(function(s) {{\n\
                     var nc = document.createElement('style');\n\
                     nc.setAttribute('data-rwe-style', s.getAttribute('data-rwe-style') || '');\n\
                     nc.textContent = s.textContent;\n\
                     document.head.appendChild(nc);\n\
                   }});\n\
                   var __newLinks = Array.from(doc.querySelectorAll('head link[rel=\"stylesheet\"]')).map(function(l) {{ return l.getAttribute('href'); }});\n\
                   Array.from(document.querySelectorAll('head link[rel=\"stylesheet\"]')).forEach(function(l) {{ if (__newLinks.indexOf(l.getAttribute('href')) === -1) l.parentNode.removeChild(l); }});\n\
                   var __curLinks = Array.from(document.querySelectorAll('head link[rel=\"stylesheet\"]')).map(function(l) {{ return l.getAttribute('href'); }});\n\
                   __newLinks.forEach(function(h) {{ if (!h || __curLinks.indexOf(h) !== -1) return; var el = document.createElement('link'); el.rel = 'stylesheet'; el.href = h; document.head.appendChild(el); }});\n\
                   document.body.className = doc.body.className;\n\
                   if (doc.documentElement.lang) document.documentElement.lang = doc.documentElement.lang;\n\
                   document.querySelectorAll('script[data-rwe-nav-script]').forEach(function(s) {{ s.remove(); }});\n\
                   var __scriptPromises = [];\n\
                   doc.querySelectorAll('script[type=\"module\"]').forEach(function(s) {{\n\
                     var src = s.getAttribute('src');\n\
                     if (src) {{\n\
                       __scriptPromises.push(\n\
                         fetch(src, {{ credentials: 'same-origin' }}).then(function(r) {{ return r.text(); }}).then(function(code) {{\n\
                           var n = document.createElement('script');\n\
                           n.type = 'module';\n\
                           n.setAttribute('data-rwe-nav-script', '');\n\
                           n.textContent = code;\n\
                           document.head.appendChild(n);\n\
                         }})\n\
                       );\n\
                     }} else if (s.textContent) {{\n\
                       var n = document.createElement('script');\n\
                       n.type = 'module';\n\
                       n.setAttribute('data-rwe-nav-script', '');\n\
                       n.textContent = s.textContent;\n\
                       document.head.appendChild(n);\n\
                     }}\n\
                   }});\n\
                   document.title = doc.title;\n\
                   if (mode === 'replace') {{\n\
                     history.replaceState(null, '', href);\n\
                   }} else if (mode !== 'none') {{\n\
                     history.pushState(null, '', href);\n\
                   }}\n\
                   window.scrollTo(0, 0);\n\
                   Promise.all(__scriptPromises).then(function() {{\n\
                     window.dispatchEvent(new CustomEvent('rwe:nav', {{ detail: {{ url: href }} }}));\n\
                   }});\n\
                   __bDone();\n\
                 }})\n\
                 .catch(function() {{ __bFail(); window.location.href = href; }});\n\
             }};\n\
             window.addEventListener('popstate', function() {{\n\
               // The browser already moved us. Render the destination, but do\n\
               // not push it — that turned every Back into a new entry and made\n\
               // the stack impossible to walk out of.\n\
               window.rweNavigate(window.location.pathname + window.location.search, 'none');\n\
             }});\n\
           }}\n\
         }})();\n\
         const __payloadEl = document.getElementById('{PAYLOAD_ID}');\n\
         const __input = __payloadEl ? JSON.parse(__payloadEl.textContent || '{{}}') : {{}};\n\
         globalThis.ctx = __input;\n\
         globalThis.input = globalThis.ctx;\n\
         {zeb_preamble}\
         let __islandCounter = 0;\n\
         const __IslandOff = memo(function({{ children }}) {{ return children; }}, () => true);\n\
         function __IslandOnView({{ id, children }}) {{\n\
           const [active, setActive] = useState(false);\n\
           const [ssrHtml] = useState(() => {{\n\
             if (typeof document === 'undefined') return '';\n\
             const el = document.querySelector('[data-island-id=\"' + id + '\"]');\n\
             return el ? el.outerHTML : '';\n\
           }});\n\
           const ref = useRef(null);\n\
           useEffect(() => {{\n\
             if (!ref.current || active) return;\n\
             const io = new IntersectionObserver(([e]) => {{\n\
               if (e.isIntersecting) {{ setActive(true); io.disconnect(); }}\n\
             }}, {{ threshold: 0.1 }});\n\
             io.observe(ref.current);\n\
             return () => io.disconnect();\n\
           }}, [active]);\n\
           if (active) return children;\n\
           return h('div', {{ ref, 'data-island-id': id, 'data-hydrate': 'onview',\n\
             dangerouslySetInnerHTML: {{ __html: ssrHtml }} }});\n\
         }}\n\
         function __IslandOnInteract({{ id, children }}) {{\n\
           const [active, setActive] = useState(false);\n\
           const [ssrHtml] = useState(() => {{\n\
             if (typeof document === 'undefined') return '';\n\
             const el = document.querySelector('[data-island-id=\"' + id + '\"]');\n\
             return el ? el.outerHTML : '';\n\
           }});\n\
           if (active) return children;\n\
           return h('div', {{\n\
             'data-island-id': id, 'data-hydrate': 'oninteract',\n\
             onClickCapture: () => setActive(true),\n\
             onFocusCapture: () => setActive(true),\n\
             dangerouslySetInnerHTML: {{ __html: ssrHtml }}\n\
           }});\n\
         }}\n\
         (function() {{\n\
           const __origH = globalThis.h;\n\
           globalThis.h = function(type, props, ...args) {{\n\
             if (props && props.hydrate && props.hydrate !== 'onload') {{\n\
               const mode = props.hydrate;\n\
               const id = 'island-' + (__islandCounter++);\n\
               const newProps = Object.assign({{}}, props);\n\
               delete newProps.hydrate;\n\
               newProps['data-island-id'] = id;\n\
               const el = __origH(type, newProps, ...args);\n\
               if (mode === 'off')        return __origH(__IslandOff, {{ id }}, el);\n\
               if (mode === 'onview')     return __origH(__IslandOnView, {{ id }}, el);\n\
               if (mode === 'oninteract') return __origH(__IslandOnInteract, {{ id }}, el);\n\
             }}\n\
             return __origH(type, props, ...args);\n\
           }};\n\
         }})();\n\
         __islandCounter = 0;\n\
         const __mod = await import('data:text/javascript;base64,{encoded}');\n\
         const __Page = __mod.default;\n\
         function __RweRoot(props) {{\n\
           // Start from the payload, exactly as the server's provider did\n\
           // (zeb_ssr_init.js wrapWithPageState): a keyed usePageState whose\n\
           // key exists in `input` must read the same value on both sides,\n\
           // or the first client render disagrees with the server HTML.\n\
           const [state, setState] = useState(() => ({{ ...__input }}));\n\
           const setPageState = (patch) => {{\n\
             if (typeof patch === 'function') {{\n\
               setState((prev) => ({{ ...(prev || {{}}), ...((patch(prev || {{}})) || {{}}) }}));\n\
               return;\n\
             }}\n\
             setState((prev) => ({{ ...(prev || {{}}), ...((patch) || {{}}) }}));\n\
           }};\n\
           const value = useMemo(() => ({{ ...(state || {{}}), setPageState }}), [state]);\n\
           /* Expose page-state bridge for external libraries (zeb/prosemirror, etc.).\n\
            * window.__rweSetPageState(patch) — call from any zeb/* bundle to patch\n\
            * the Zeb React page state. useState setter is stable so this ref is safe.\n\
            * window.__rwePageState — read-only snapshot; updated after every change.\n\
            * rwe:state:change event — dispatched on window after every state update;\n\
            * bundles listen here to react to page-driven content changes (e.g. swap\n\
            * a ProseEditor's content when the examiner navigates to the next answer). */\n\
           window.__rweSetPageState = setPageState;\n\
           useEffect(() => {{\n\
             window.__rwePageState = state;\n\
             window.dispatchEvent(new CustomEvent('rwe:state:change', {{ detail: state }}));\n\
           }}, [state]);\n\
           return h(__RwePageStateContext.Provider, {{ value }}, h(__Page, props));\n\
         }}\n\
         const __root = document.getElementById('{ROOT_ID}');\n\
         if (__root && typeof __Page === 'function') {{\n\
           hydrate(h(__RweRoot, __input), __root);\n\
         }}\n",
        zeb_preamble = zeb_preamble,
    )
}

fn transpile_client_cached(source: &str, timeout_ms: u64) -> Result<String, EngineError> {
    // Strip "rwe" imports BEFORE passing to the Deno bundler. If stripped after,
    // the bundler resolves "rwe" → absolute filesystem path which the browser can't load.
    let stripped = strip_rwe_client_imports(source);
    let key = stable_hash_u64(&stripped);
    if let Ok(cache) = CLIENT_TRANSPILE_CACHE.lock()
        && let Some(cached) = cache.get(&key)
    {
        return Ok(cached.clone());
    }

    let transpiled = deno_worker::transpile_client(&stripped, timeout_ms)?;

    if let Ok(mut cache) = CLIENT_TRANSPILE_CACHE.lock() {
        // keep cache lean by bounding entries; new entries overwrite oldest key eviction by clear.
        if cache.len() > 256 {
            cache.clear();
        }
        cache.insert(key, transpiled.clone());
    }

    Ok(transpiled)
}

fn stable_hash_u64(input: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    input.hash(&mut hasher);
    hasher.finish()
}

/// Build a complete HTML document from the resolved `export const page` config.
/// Page config values are already resolved by JS at module eval time via `globalThis.ctx`.
///
/// Supported `page.head` fields:
/// - `title`       → `<title>`
/// - `description` → `<meta name="description">`
/// - `themeColor`  → `<meta name="theme-color">`
/// - `canonical`   → `<link rel="canonical">`
/// - `robots`      → `<meta name="robots">`
/// - `manifest`    → `<link rel="manifest">`
/// - `icons`       → array of `{ rel, href, type?, sizes? }` link tags
///                   (favicon 32×32, 16×16, apple-touch-icon, etc.)
/// - `links`       → array of `{ rel, href, type?, sizes?, media?, crossorigin? }`
///                   link tags — how a page declares a stylesheet it needs
/// - `og`          → Open Graph `{ title, description, image, url, type, siteName, locale }`
/// - `twitter`     → Twitter Card `{ card, title, description, image, site, creator }`
/// - `extra`       → raw HTML string injected verbatim at end of `<head>` (trusted escape hatch)
/// - `titleSuffix` → appended to `<title>` (the site name, set once in the shell)
/// - `jsonld`      → an object or array → `<script type="application/ld+json">` per object
/// - `alternates`  → `{ en: "/x", id: "/id/x", "x-default": "/x" }` → `<link rel="alternate" hreflang>`
///
/// URLs in `canonical`, `og.url`, `og.image`, `twitter.image` and `alternates`
/// that start with `/` are made absolute with the request origin; `og:*` and
/// `twitter:card` are defaulted from title, description and image
/// (`docs/contracts/discoverability.md` §1).
fn build_document_shell(page_config: &Option<Value>, body_content: &str, site: &SiteContext) -> String {
    let pc = page_config.as_ref();
    let hd = pc.and_then(|p| p.get("head"));
    let abs = |v: &str| site.absolute(v);

    let lang = pc
        .and_then(|p| p.get("html"))
        .and_then(|h| h.get("lang"))
        .and_then(Value::as_str)
        .unwrap_or("en");

    let body_class = pc
        .and_then(|p| p.get("body"))
        .and_then(|b| b.get("className"))
        .and_then(Value::as_str)
        .unwrap_or("");

    let mut head = String::new();
    head.push_str("<meta charset=\"utf-8\">");
    head.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">");

    // title (+ the site's suffix, set once in the shell's page config)
    let title = hd.and_then(|h| h.get("title")).and_then(Value::as_str).unwrap_or("");
    let suffix = hd.and_then(|h| h.get("titleSuffix")).and_then(Value::as_str).unwrap_or("");
    if !title.is_empty() {
        // The suffix is the site name with its separator (" — RESEARCHSITE"); the
        // home page's title is the site name itself, so it takes none.
        let suffix_name = suffix.trim_start_matches(|c: char| c.is_whitespace() || matches!(c, '—' | '–' | '-' | '|' | '·' | ':')).trim();
        let already = suffix.is_empty() || title.trim() == suffix_name || title.ends_with(suffix.trim());
        let full = if already { title.to_string() } else { format!("{title}{suffix}") };
        head.push_str(&format!("<title>{}</title>", escape_html(&full)));
    }
    let description = hd.and_then(|h| h.get("description")).and_then(Value::as_str).unwrap_or("");

    // description
    if let Some(v) = hd
        .and_then(|h| h.get("description"))
        .and_then(Value::as_str)
    {
        if !v.is_empty() {
            head.push_str(&format!(
                "<meta name=\"description\" content=\"{}\">",
                escape_attr(v)
            ));
        }
    }

    // theme-color
    if let Some(v) = hd.and_then(|h| h.get("themeColor")).and_then(Value::as_str) {
        if !v.is_empty() {
            head.push_str(&format!(
                "<meta name=\"theme-color\" content=\"{}\">",
                escape_attr(v)
            ));
        }
    }

    // robots
    if let Some(v) = hd.and_then(|h| h.get("robots")).and_then(Value::as_str) {
        if !v.is_empty() {
            head.push_str(&format!(
                "<meta name=\"robots\" content=\"{}\">",
                escape_attr(v)
            ));
        }
    }

    // canonical — absolute, so the same page under three URLs claims one
    let canonical = hd
        .and_then(|h| h.get("canonical"))
        .and_then(Value::as_str)
        .filter(|v| !v.is_empty())
        .map(abs);
    if let Some(v) = &canonical {
        head.push_str(&format!("<link rel=\"canonical\" href=\"{}\">", escape_attr(v)));
    }

    // alternates — one link per language, plus x-default
    if let Some(alternates) = hd.and_then(|h| h.get("alternates")).and_then(Value::as_object) {
        for (lang, href) in alternates {
            if let Some(href) = href.as_str().filter(|h| !h.is_empty()) {
                head.push_str(&format!(
                    "<link rel=\"alternate\" hreflang=\"{}\" href=\"{}\">",
                    escape_attr(lang),
                    escape_attr(&abs(href))
                ));
            }
        }
    }

    // icons — [{ rel, href, type?, sizes? }]
    if let Some(icons) = hd.and_then(|h| h.get("icons")).and_then(Value::as_array) {
        for icon in icons {
            let href = icon.get("href").and_then(Value::as_str).unwrap_or_default();
            if href.is_empty() {
                continue;
            }
            let rel = icon.get("rel").and_then(Value::as_str).unwrap_or("icon");
            let mut tag = format!(
                "<link rel=\"{}\" href=\"{}\"",
                escape_attr(rel),
                escape_attr(href)
            );
            if let Some(t) = icon.get("type").and_then(Value::as_str) {
                if !t.is_empty() {
                    tag.push_str(&format!(" type=\"{}\"", escape_attr(t)));
                }
            }
            if let Some(s) = icon.get("sizes").and_then(Value::as_str) {
                if !s.is_empty() {
                    tag.push_str(&format!(" sizes=\"{}\"", escape_attr(s)));
                }
            }
            tag.push('>');
            head.push_str(&tag);
        }
    }

    // links — [{ rel, href, type?, sizes?, media?, crossorigin? }]
    //
    // A page saying which stylesheet it needs. This was declared by six pages
    // and read by nobody: the renderer knew `icons` and not `links`, so those
    // stylesheets only ever arrived when something else — a scan of the
    // rendered HTML for a marker class — happened to inject them. A page that
    // drew its icons after hydration got no stylesheet at all, because there
    // was nothing in the server output to scan for.
    if let Some(links) = hd.and_then(|h| h.get("links")).and_then(Value::as_array) {
        for link in links {
            let href = link.get("href").and_then(Value::as_str).unwrap_or_default();
            if href.is_empty() {
                continue;
            }
            let rel = link
                .get("rel")
                .and_then(Value::as_str)
                .unwrap_or("stylesheet");
            let mut tag = format!(
                "<link rel=\"{}\" href=\"{}\"",
                escape_attr(rel),
                escape_attr(href)
            );
            for attr in ["type", "sizes", "media", "crossorigin", "as"] {
                if let Some(v) = link.get(attr).and_then(Value::as_str) {
                    if !v.is_empty() {
                        tag.push_str(&format!(" {}=\"{}\"", attr, escape_attr(v)));
                    }
                }
            }
            tag.push('>');
            head.push_str(&tag);
        }
    }

    // A page saying which script it needs. Same omission as `links` had: a page
    // could declare a library and the tag never reached the document, so the
    // component that waited for it simply never upgraded — silently, because
    // nothing failed. `src` is required; a script with no source is not a
    // script.
    if let Some(scripts) = hd.and_then(|h| h.get("scripts")).and_then(Value::as_array) {
        for script in scripts {
            let src = script.get("src").and_then(Value::as_str).unwrap_or_default();
            if src.is_empty() {
                continue;
            }
            let mut tag = format!("<script src=\"{}\"", escape_attr(src));
            for flag in ["defer", "async", "nomodule"] {
                if script.get(flag).and_then(Value::as_bool).unwrap_or(false) {
                    tag.push(' ');
                    tag.push_str(flag);
                }
            }
            for attr in ["type", "crossorigin", "integrity", "referrerpolicy"] {
                if let Some(v) = script.get(attr).and_then(Value::as_str) {
                    if !v.is_empty() {
                        tag.push_str(&format!(" {}=\"{}\"", attr, escape_attr(v)));
                    }
                }
            }
            tag.push_str("></script>");
            head.push_str(&tag);
        }
    }

    // manifest
    if let Some(v) = hd.and_then(|h| h.get("manifest")).and_then(Value::as_str) {
        if !v.is_empty() {
            head.push_str(&format!(
                "<link rel=\"manifest\" href=\"{}\">",
                escape_attr(v)
            ));
        }
    }

    // Open Graph — explicit values win; the rest come from title,
    // description, canonical and the request. A page that set those three
    // gets a complete card without repeating itself.
    let og = hd.and_then(|h| h.get("og"));
    let og_str = |key: &str| og.and_then(|o| o.get(key)).and_then(Value::as_str).filter(|v| !v.is_empty()).map(str::to_string);
    let og_image = og_str("image").map(|v| abs(&v));
    let has_card_source = og.is_some() || !title.is_empty();
    if has_card_source {
        let og_title = og_str("title").unwrap_or_else(|| title.to_string());
        let og_description = og_str("description").unwrap_or_else(|| description.to_string());
        let og_url = og_str("url").map(|v| abs(&v)).or_else(|| canonical.clone()).or_else(|| site.page_url());
        let og_type = og_str("type").unwrap_or_else(|| "website".to_string());
        let pairs: [(&str, Option<String>); 7] = [
            ("og:title", (!og_title.is_empty()).then_some(og_title)),
            ("og:description", (!og_description.is_empty()).then_some(og_description)),
            ("og:image", og_image.clone()),
            ("og:url", og_url),
            ("og:type", Some(og_type)),
            ("og:site_name", og_str("siteName")),
            ("og:locale", og_str("locale")),
        ];
        for (prop, value) in pairs {
            if let Some(v) = value {
                head.push_str(&format!("<meta property=\"{}\" content=\"{}\">", prop, escape_attr(&v)));
            }
        }
    }

    // Twitter Card — defaults from Open Graph; `summary_large_image` when
    // there is an image to show.
    let tw = hd.and_then(|h| h.get("twitter"));
    let tw_str = |key: &str| tw.and_then(|t| t.get(key)).and_then(Value::as_str).filter(|v| !v.is_empty()).map(str::to_string);
    let tw_image = tw_str("image").map(|v| abs(&v)).or_else(|| og_image.clone());
    if tw.is_some() || tw_image.is_some() {
        let card = tw_str("card").unwrap_or_else(|| if tw_image.is_some() { "summary_large_image".into() } else { "summary".into() });
        let pairs: [(&str, Option<String>); 6] = [
            ("twitter:card", Some(card)),
            ("twitter:title", tw_str("title").or_else(|| og_str("title")).or_else(|| (!title.is_empty()).then(|| title.to_string()))),
            ("twitter:description", tw_str("description").or_else(|| og_str("description")).or_else(|| (!description.is_empty()).then(|| description.to_string()))),
            ("twitter:image", tw_image),
            ("twitter:site", tw_str("site")),
            ("twitter:creator", tw_str("creator")),
        ];
        for (name, value) in pairs {
            if let Some(v) = value {
                head.push_str(&format!("<meta name=\"{}\" content=\"{}\">", name, escape_attr(&v)));
            }
        }
    }

    // JSON-LD — an object or an array of objects, one block each. The
    // renderer serialises and escapes; a page never builds this as a string.
    if let Some(jsonld) = hd.and_then(|h| h.get("jsonld")) {
        let blocks: Vec<&Value> = match jsonld {
            Value::Array(items) => items.iter().filter(|v| v.is_object()).collect(),
            Value::Object(_) => vec![jsonld],
            _ => Vec::new(),
        };
        for block in blocks {
            if let Ok(text) = serde_json::to_string(block) {
                head.push_str(&format!(
                    "<script type=\"application/ld+json\">{}</script>",
                    escape_json_script(&text)
                ));
            }
        }
    }

    // extra — raw HTML, injected verbatim (author-trusted content, no escaping)
    if let Some(v) = hd.and_then(|h| h.get("extra")).and_then(Value::as_str) {
        if !v.is_empty() {
            head.push_str(v);
        }
    }

    // stylesheets — array of href strings or objects with href/rel/media
    if let Some(stylesheets) = hd
        .and_then(|h| h.get("stylesheets"))
        .and_then(Value::as_array)
    {
        for stylesheet in stylesheets {
            match stylesheet {
                Value::String(href) if !href.is_empty() => {
                    head.push_str(&format!(
                        "<link rel=\"stylesheet\" href=\"{}\">",
                        escape_attr(href)
                    ));
                }
                Value::Object(obj) => {
                    let href = obj.get("href").and_then(Value::as_str).unwrap_or_default();
                    if href.is_empty() {
                        continue;
                    }
                    let rel = obj
                        .get("rel")
                        .and_then(Value::as_str)
                        .unwrap_or("stylesheet");
                    let mut tag = format!(
                        "<link rel=\"{}\" href=\"{}\"",
                        escape_attr(rel),
                        escape_attr(href)
                    );
                    if let Some(media) = obj.get("media").and_then(Value::as_str)
                        && !media.is_empty()
                    {
                        tag.push_str(&format!(" media=\"{}\"", escape_attr(media)));
                    }
                    tag.push('>');
                    head.push_str(&tag);
                }
                _ => {}
            }
        }
    }

    let body_attr = if body_class.is_empty() {
        String::new()
    } else {
        format!(" class=\"{}\"", escape_attr(body_class))
    };

    format!(
        "<!DOCTYPE html><html lang=\"{lang}\"><head>{head}</head><body{body_attr}>{body_content}</body></html>"
    )
}

fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Puts the engine's Tailwind block — preflight first, then the utilities —
/// into the head **before** the project's own stylesheets.
///
/// Order is the whole point. The preflight resets `border: 0 solid`, which
/// resets border *colour* to `currentColor`; shadcn's base rule
/// `* { border-color: var(--border) }` lives in the project's `globals.css`
/// and has to come after that reset to win. Every project stylesheet is a
/// `<style data-rwe-style>` block written by [`render`], so the engine block
/// goes in front of the first one, or before `</head>` when there is none.
/// The same order a Tailwind build produces: preflight, utilities, your CSS.
pub fn insert_engine_styles(html: &str, css: &str) -> String {
    if css.trim().is_empty() {
        return html.to_string();
    }
    let block = format!("<style data-rwe-tw>{css}</style>");
    let mut html = html.to_string();
    if let Some(pos) = html.find("<style data-rwe-style") {
        html.insert_str(pos, &block);
    } else if let Some(pos) = html.find("</head>") {
        html.insert_str(pos, &block);
    } else {
        html = format!("{block}{html}");
    }
    html
}

fn escape_json_script(input: &str) -> String {
    input
        .replace("<", "\\u003c")
        .replace(">", "\\u003e")
        .replace("&", "\\u0026")
}

/// Where this render is being served from, read off the page state the
/// pipeline injected (`headers.host`, `x-forwarded-*`, `route`). The project
/// never writes its own address (`addressing.md` §0); the renderer knows it
/// from the request and makes the head's URLs absolute with it.
#[derive(Debug, Clone, Default)]
pub struct SiteContext {
    /// `https://research.example`, or `None` when no host header reached the render
    /// (previews, tests) — relative URLs are then left as they are.
    pub origin: Option<String>,
    /// The browser path being rendered, for `og:url` when no canonical is set.
    pub route: Option<String>,
}

impl SiteContext {
    pub fn from_state(state: &Value) -> Self {
        let headers = state.get("headers");
        let header = |name: &str| {
            headers
                .and_then(|h| h.get(name))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .map(str::to_string)
        };
        let host = header("x-forwarded-host").or_else(|| header("host"));
        let proto = header("x-forwarded-proto").unwrap_or_else(|| "http".to_string());
        let origin = host.map(|h| format!("{}://{}", proto.split(',').next().unwrap_or("http").trim(), h.split(',').next().unwrap_or("").trim()));
        let route = state
            .get("route")
            .and_then(Value::as_str)
            .filter(|r| r.starts_with('/'))
            .map(str::to_string);
        Self { origin, route }
    }

    /// A root-relative URL made absolute; anything else unchanged.
    pub fn absolute(&self, url: &str) -> String {
        match (&self.origin, url.starts_with('/') && !url.starts_with("//")) {
            (Some(origin), true) => format!("{origin}{url}"),
            _ => url.to_string(),
        }
    }

    pub fn page_url(&self) -> Option<String> {
        match (&self.origin, &self.route) {
            (Some(origin), Some(route)) => Some(format!("{origin}{route}")),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rwe::core::model::HydrateMode;
    use serde_json::json;

    fn site() -> SiteContext {
        SiteContext::from_state(&json!({
            "headers": { "host": "research.example", "x-forwarded-proto": "https" },
            "route": "/researchers/jane-doe"
        }))
    }

    /// `discoverability.md` §1: the page declares a few values; the renderer
    /// writes the tags, resolves the URLs and fills the defaults.
    #[test]
    fn engine_styles_go_before_the_project_stylesheet() {
        let html = "<html><head><title>x</title><style data-rwe-style=\"0\">*{border-color:var(--border)}</style></head><body></body></html>";
        let out = insert_engine_styles(html, "*{border:0 solid}");
        let tw = out.find("<style data-rwe-tw>").expect("engine block");
        let mine = out.find("<style data-rwe-style").expect("project block");
        assert!(tw < mine, "preflight first, the project's base rules after it: {out}");
        let bare = insert_engine_styles("<html><head></head></html>", ".a{}");
        assert!(bare.contains("<style data-rwe-tw>.a{}</style></head>"));
        assert_eq!(insert_engine_styles("<p>", "  "), "<p>");
    }

    #[test]
    fn head_urls_become_absolute_and_cards_are_defaulted() {
        let page = json!({ "head": {
            "title": "Jane Doe", "titleSuffix": " — RESEARCHSITE", "description": "Maternal health in eastern Indonesia.",
            "canonical": "/researchers/jane-doe",
            "og": { "image": "/files/photos/jane.webp" }
        }});
        let html = build_document_shell(&Some(page), "<div></div>", &site());
        assert!(html.contains("<title>Jane Doe — RESEARCHSITE</title>"), "{html}");
        assert!(html.contains(r#"<link rel="canonical" href="https://research.example/researchers/jane-doe">"#), "{html}");
        assert!(html.contains(r#"<meta property="og:title" content="Jane Doe">"#), "og:title from title");
        assert!(html.contains(r#"<meta property="og:description" content="Maternal health in eastern Indonesia.">"#));
        assert!(html.contains(r#"<meta property="og:image" content="https://research.example/files/photos/jane.webp">"#), "absolute image");
        assert!(html.contains(r#"<meta property="og:url" content="https://research.example/researchers/jane-doe">"#), "og:url from canonical");
        assert!(html.contains(r#"<meta property="og:type" content="website">"#));
        assert!(html.contains(r#"<meta name="twitter:card" content="summary_large_image">"#), "card from image");
        assert!(html.contains(r#"<meta name="twitter:image" content="https://research.example/files/photos/jane.webp">"#));
    }

    #[test]
    fn explicit_card_values_win_and_a_suffix_is_never_doubled() {
        let page = json!({ "head": {
            "title": "RESEARCHSITE", "titleSuffix": " — RESEARCHSITE",
            "og": { "title": "Custom", "type": "profile", "url": "https://elsewhere.example/x" },
            "twitter": { "card": "summary" }
        }});
        let html = build_document_shell(&Some(page), "<div></div>", &site());
        assert!(html.contains("<title>RESEARCHSITE</title>"), "{html}");
        assert!(html.contains(r#"<meta property="og:title" content="Custom">"#));
        assert!(html.contains(r#"<meta property="og:type" content="profile">"#));
        assert!(html.contains(r#"<meta property="og:url" content="https://elsewhere.example/x">"#));
        assert!(html.contains(r#"<meta name="twitter:card" content="summary">"#));
    }

    #[test]
    fn jsonld_and_alternates_are_emitted_and_escaped() {
        let page = json!({ "head": {
            "title": "x",
            "jsonld": [
                { "@context": "https://schema.org", "@type": "Person", "name": "Jane </script> Doe" },
                { "@context": "https://schema.org", "@type": "BreadcrumbList" }
            ],
            "alternates": { "en": "/researchers/jane-doe", "id": "/id/researchers/jane-doe", "x-default": "/researchers/jane-doe" }
        }});
        let html = build_document_shell(&Some(page), "<div></div>", &site());
        assert_eq!(html.matches(r#"<script type="application/ld+json">"#).count(), 2);
        assert!(!html.contains("</script> Sari"), "the closing tag inside the JSON is escaped: {html}");
        assert!(html.contains(r#""@type":"Person""#));
        assert!(html.contains(r#"<link rel="alternate" hreflang="id" href="https://research.example/id/researchers/jane-doe">"#), "{html}");
        assert_eq!(html.matches(r#"rel="alternate" hreflang="#).count(), 3);
        let single = json!({ "head": { "jsonld": { "@type": "Organization", "name": "RESEARCHSITE" } } });
        let html = build_document_shell(&Some(single), "<div></div>", &SiteContext::default());
        assert_eq!(html.matches("application/ld+json").count(), 1);
    }

    #[test]
    fn without_a_host_relative_urls_are_left_alone() {
        let page = json!({ "head": { "title": "x", "canonical": "/p", "og": { "image": "/i.png" } } });
        let html = build_document_shell(&Some(page), "<div></div>", &SiteContext::default());
        assert!(html.contains(r#"href="/p""#) && html.contains(r#"content="/i.png""#), "{html}");
        assert!(html.contains(r#"<meta property="og:url" content="/p">"#), "canonical still feeds og:url, relative as given");
        let bare = build_document_shell(&Some(json!({ "head": { "title": "x" } })), "<div></div>", &SiteContext::default());
        assert!(!bare.contains("og:url"), "no canonical, no origin, no route → no og:url guessed: {bare}");
        let forwarded = SiteContext::from_state(&json!({ "headers": { "host": "127.0.0.1:10611", "x-forwarded-host": "research.example", "x-forwarded-proto": "https" } }));
        assert_eq!(forwarded.absolute("/p"), "https://research.example/p");
    }

    /// A page saying which stylesheet it needs must get it.
    ///
    /// Six pages declared `head.links` and the renderer read only `head.icons`,
    /// so every one of those stylesheets was dropped. They appeared anyway when
    /// a separate scan of the rendered HTML spotted a marker class — which
    /// meant a page drawing its icons after hydration got nothing, because
    /// there was no marker in the server output to find.
    /// A page that declares a library and never gets the tag has no way to
    /// know: the component waiting for the global simply stays in its
    /// not-loaded branch forever.
    #[test]
    fn a_declared_script_reaches_the_head() {
        let page = json!({
            "head": {
                "scripts": [
                    { "src": "https://unpkg.com/leaflet@1.9.4/dist/leaflet.js", "defer": true },
                    { "src": "/local.js", "type": "module" },
                    { "src": "" },
                    { "defer": true },
                ]
            }
        });
        let html = build_document_shell(&Some(page), "<div></div>", &SiteContext::default());

        assert!(
            html.contains(
                r#"<script src="https://unpkg.com/leaflet@1.9.4/dist/leaflet.js" defer></script>"#
            ),
            "{html}"
        );
        assert!(
            html.contains(r#"<script src="/local.js" type="module"></script>"#),
            "{html}"
        );
        assert_eq!(
            html.matches("<script src=").count(),
            2,
            "a script with no src is not a script: {html}"
        );
    }

    #[test]
    fn a_declared_stylesheet_reaches_the_head() {
        let page = json!({
            "head": {
                "links": [
                    { "rel": "stylesheet", "href": "/assets/platform/db-suite.css" },
                    { "rel": "stylesheet", "href": "/icons.css", "media": "screen" },
                    { "href": "/implicitly-a-stylesheet.css" },
                    { "rel": "stylesheet", "href": "" },
                ]
            }
        });
        let html = build_document_shell(&Some(page), "<div></div>", &SiteContext::default());

        assert!(
            html.contains(r#"<link rel="stylesheet" href="/assets/platform/db-suite.css">"#),
            "{html}"
        );
        assert!(
            html.contains(r#"<link rel="stylesheet" href="/icons.css" media="screen">"#),
            "an optional attribute travels with it: {html}"
        );
        assert!(
            html.contains(r#"<link rel="stylesheet" href="/implicitly-a-stylesheet.css">"#),
            "a link without a rel is a stylesheet, the only kind a page declares: {html}"
        );
        assert_eq!(
            html.matches("<link rel=\"stylesheet\"").count(),
            3,
            "an entry with no href is not a tag: {html}"
        );
    }

    /// Declaring nothing adds nothing.
    #[test]
    fn a_page_without_links_gets_no_stylesheet_tags() {
        let html = build_document_shell(&Some(json!({ "head": { "title": "x" } })), "<div></div>", &SiteContext::default());
        assert!(!html.contains("rel=\"stylesheet\""), "{html}");
    }

    /// A runtime-library import is removed by its span; the same text inside
    /// a template literal — a code sample — survives.
    #[test]
    fn stripping_runtime_imports_leaves_code_samples_alone() {
        let source = "import { useDebounce } from \"zeb/use\";\nconst sample = `\nimport { Button } from \"zeb/ui/button\";\n`;\nexport default function P() { return null; }\n";
        let out = strip_rwe_client_imports(source);
        assert!(!out.contains("from \"zeb/use\""), "{out}");
        assert!(out.contains("import { Button } from \"zeb/ui/button\";"), "{out}");
    }

    #[test]
    fn zeb_preamble_uses_codemirror_entry_module() {
        let preamble = build_zeb_preamble(&["zeb/codemirror".to_string()], &[]);

        assert!(
            preamble.contains("/assets/libraries/zeb/codemirror/0.1/runtime/entry.mjs"),
            "expected codemirror preamble to load entry.mjs, got {preamble}"
        );
        assert!(
            preamble.contains("Object.assign(globalThis"),
            "expected zeb preamble to expose library exports on globalThis, got {preamble}"
        );
    }

    #[test]
    fn build_client_module_sets_global_input_alias() {
        let module = build_client_module("export default function Page(){ return null; }", "");
        assert!(
            module.contains("globalThis.input = globalThis.ctx;"),
            "expected client bootstrap to mirror input alias, got {module}"
        );
    }

    #[test]
    fn render_injects_collected_inline_styles_into_document_head() {
        let compiled = CompiledTemplate {
            engine: "rwe".to_string(),
            source_path: Some("inline-style-test.tsx".to_string()),
            runtime_mode: super::super::config::RuntimeMode::Inline,
            deno_timeout_ms: 10_000,
            server_module_source:
                "export default function Page(){ return <main>INLINE_STYLE_TEST</main>; }"
                    .to_string(),
            client_module_source:
                "export default function Page(){ return <main>INLINE_STYLE_TEST</main>; }"
                    .to_string(),
            imports: Vec::new(),
            diagnostics: Vec::new(),
            hydrate_mode: HydrateMode::Onload,
            compile_options: super::super::config::CompileOptions::default(),
            detected_zeb_libs: Vec::new(),
            inline_styles: vec![".editor-shell { color: red; }".to_string()],
            dependency_paths: Default::default(),
        };

        let output = render(&compiled, &json!({}), &[]).expect("render");
        let style_pos = output
            .html
            .find("data-rwe-style=\"0\"")
            .expect("expected inline style block");
        let root_pos = output
            .html
            .find("__rwe_root")
            .expect("expected rwe root in html");

        assert!(output.html.contains(".editor-shell { color: red; }"));
        assert!(
            style_pos < root_pos,
            "expected collected inline styles in head before body content, got {}",
            output.html
        );
    }

    #[test]
    fn client_root_page_state_starts_from_the_payload() {
        // The server renders `usePageState("k", d)` from `{...input}`; the
        // browser must hydrate from the same object or keys present in the
        // payload tear on first render.
        let module = build_client_module("export default function P(){return null}", "");
        assert!(
            module.contains("useState(() => ({ ...__input }))"),
            "expected the client root to seed page state from __input, got {module}"
        );
    }

    #[test]
    fn build_client_module_syncs_imported_styles_during_navigation() {
        let module = build_client_module("export default function Page(){ return null; }", "");
        assert!(
            module.contains("style[data-rwe-tw], style[data-rwe-style]"),
            "expected navigation cleanup to remove imported styles, got {module}"
        );
        assert!(
            module.contains("doc.querySelectorAll('style[data-rwe-style]')"),
            "expected navigation sync to clone imported style blocks, got {module}"
        );
        assert!(
            module.contains("setAttribute('data-rwe-style'"),
            "expected imported style blocks to preserve identity during navigation, got {module}"
        );
    }
}

#[cfg(test)]
mod global_installation_tests {
    /// Every `globalThis.<name> = ...` in a source, in the order written.
    ///
    /// Deliberately textual rather than parsed: these files are the runtime
    /// preamble, assembled from string fragments and `format!`, so there is no
    /// single AST to walk. The pattern is uniform enough that reading the text
    /// is honest, and a name spelled some other way is a name this test does
    /// not claim to cover.
    fn installed_globals(source: &str) -> Vec<String> {
        let mut names = Vec::new();
        for (index, _) in source.match_indices("globalThis.") {
            let rest = &source[index + "globalThis.".len()..];
            let name: String = rest
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '$')
                .collect();
            if name.is_empty() {
                continue;
            }
            // An assignment, not a read. `globalThis.ctx.route` and
            // `typeof globalThis.L` are uses; only `= ` on the right declares.
            let after = rest[name.len()..].trim_start();
            if after.starts_with('=') && !after.starts_with("==") {
                names.push(name);
            }
        }
        names
    }

    /// Two assignments to one name is not an error anywhere — the later one
    /// simply wins, silently, from wherever it happens to sit in the file.
    ///
    /// This is not hypothetical: `useSearchParams` was installed twice in the
    /// SSR preamble with two different shapes, `URLSearchParams` and
    /// `[params, setter]`. The one twenty lines further down won, and nothing
    /// reported it. A component calling the hook got whichever the file
    /// happened to end with.
    fn assert_no_name_installed_twice(label: &str, source: &str) {
        let names = installed_globals(source);
        let mut seen = std::collections::HashMap::<String, usize>::new();
        for name in &names {
            *seen.entry(name.clone()).or_insert(0) += 1;
        }
        // Being assigned twice is only a bug when the second assignment throws
        // the first away. `h` is deliberately wrapped — the original is saved
        // (`var __orig = globalThis.h`) and the replacement calls through — and
        // that is decoration, not a name collision. A capture of the previous
        // value anywhere in the source is what separates the two.
        let mut duplicates: Vec<_> = seen
            .into_iter()
            .filter(|(_, count)| *count > 1)
            .filter(|(name, _)| !source.contains(&format!("= globalThis.{name};")))
            .map(|(name, count)| format!("{name} ({count}x)"))
            .collect();
        duplicates.sort();

        assert!(
            duplicates.is_empty(),
            "{label}: these names are installed onto globalThis more than once, \
             so the last assignment silently wins: {}",
            duplicates.join(", ")
        );
        assert!(
            names.len() > 20,
            "{label}: only {} globals found — the scan stopped matching, which \
             would make this test pass by seeing nothing",
            names.len()
        );
    }

    #[test]
    fn no_server_global_is_installed_twice() {
        assert_no_name_installed_twice(
            "zeb_ssr_init.js",
            include_str!("../runtime/zeb_ssr_init.js"),
        );
    }

    #[test]
    fn no_browser_global_is_installed_twice() {
        let module = super::build_client_module("export default function P() { return null; }", "");
        assert_no_name_installed_twice("build_client_module", &module);
    }
}
