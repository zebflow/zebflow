use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::sync::{LazyLock, Mutex};
use std::time::Instant;

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

pub fn prewarm(compiled: &CompiledTemplate) -> Result<(), EngineError> {
    let _ = transpile_client_cached(&compiled.client_module_source, compiled.deno_timeout_ms)?;
    let _ = deno_worker::render_ssr(&compiled.server_module_source, &json!({}), compiled.deno_timeout_ms)?;
    Ok(())
}

pub fn render(compiled: &CompiledTemplate, vars: &Value) -> Result<RenderOutput, EngineError> {
    let started = Instant::now();
    let ssr = deno_worker::render_ssr(
        &compiled.server_module_source,
        vars,
        compiled.deno_timeout_ms,
    )?;
    // Scan original source for zeb/* imports BEFORE stripping so the preamble
    // can be injected into the outer script even though the inner bundle strips them.
    let zeb_preamble = build_zeb_preamble(&compiled.client_module_source);
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

    let html = build_document_shell(&ssr.page_config, &body_content);

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
    // Join multi-line import statements into single logical lines before filtering.
    // A multi-line import starts with `import {` and ends on the line containing `from "..."`.
    let logical = join_import_lines(source);
    logical
        .lines()
        .filter(|line| {
            let t = line.trim();
            if !t.starts_with("import ") {
                return true;
            }
            // Strip "rwe" / "rwe-*" (hooks are globalThis globals) and
            // "zeb/*" (library exports are injected into globalThis by the outer script).
            !(t.contains("from \"zeb\"")
                || t.contains("from 'zeb'")
                || t.contains("from \"zeb/")
                || t.contains("from 'zeb/"))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Scan source for `import { … } from "zeb/<name>"` lines and return the
/// unique library specifiers used (e.g. `["zeb/threejs"]`).
fn collect_zeb_libraries(source: &str) -> Vec<String> {
    // Join multi-line import statements into single logical lines before scanning.
    let logical = join_import_lines(source);
    let mut libs: Vec<String> = Vec::new();
    for line in logical.lines() {
        let t = line.trim();
        if !t.starts_with("import ") {
            continue;
        }
        for quote in ['"', '\''] {
            let marker = format!("from {quote}zeb/");
            if t.contains(&marker) {
                let after_from = format!("from {quote}");
                if let Some(pos) = t.rfind(&after_from) {
                    let rest = &t[pos + after_from.len()..];
                    let end = rest.find(quote).unwrap_or(rest.len());
                    let lib = rest[..end].to_string();
                    if !libs.contains(&lib) {
                        libs.push(lib);
                    }
                }
                break;
            }
        }
    }
    libs
}

/// Collapse multi-line `import { … } from "…"` statements into a single line each.
///
/// ES import declarations can span multiple lines:
/// ```text
/// import {
///   WebGLRenderer,
///   Scene,
/// } from "zeb/threejs";
/// ```
/// Line-by-line scanners miss the `from "zeb/threejs"` part because it appears
/// on a line that does not start with `import`. This function joins continuation
/// lines (those between `import {` and the closing `} from "…"`) into one line
/// so that the scanners see a complete `import { … } from "…"` statement.
fn join_import_lines(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    let mut buf: Option<String> = None;

    for line in source.lines() {
        let t = line.trim();

        if let Some(ref mut acc) = buf {
            // Inside a multi-line import — append stripped content.
            acc.push(' ');
            acc.push_str(t);
            // The closing `} from "…"` ends the declaration.
            if t.contains("from \"") || t.contains("from '") {
                out.push_str(acc);
                out.push('\n');
                buf = None;
            }
        } else if t.starts_with("import ") && t.contains('{') && !t.contains("from ") {
            // Opening of a multi-line import: `import {` with no `from` yet.
            buf = Some(t.to_string());
        } else {
            out.push_str(line);
            out.push('\n');
        }
    }

    // Flush any unclosed buffer (shouldn't happen in valid source, but be safe).
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
        "zeb/threejs"     => Some("/assets/libraries/zeb/threejs/0.1/runtime/threejs.bundle.mjs"),
        "zeb/threejs-vrm" => Some("/assets/libraries/zeb/threejs-vrm/0.1/runtime/threejs-vrm.bundle.mjs"),
        "zeb/d3"          => Some("/assets/libraries/zeb/d3/0.1/runtime/d3.bundle.mjs"),
        "zeb/deckgl"      => Some("/assets/libraries/zeb/deckgl/0.1/runtime/deckgl.bundle.mjs"),
        "zeb/codemirror"  => Some("/assets/libraries/zeb/codemirror/0.1/runtime/codemirror.bundle.mjs"),
        "zeb/graphui"     => Some("/assets/libraries/zeb/graphui/0.1/runtime/graphui.bundle.mjs"),
        "zeb/markdown"    => Some("/assets/libraries/zeb/markdown/0.1/runtime/markdown.bundle.mjs"),
        "zeb/prosemirror" => Some("/assets/libraries/zeb/prosemirror/0.1/runtime/prosemirror.bundle.mjs"),
        "zeb/use"         => Some("/assets/libraries/zeb/use/0.1/runtime/use.bundle.mjs"),
        "zeb/icons"       => Some("/assets/libraries/zeb/icons/0.1/runtime/icons.bundle.mjs"),
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
fn build_zeb_preamble(source: &str) -> String {
    let libs = collect_zeb_libraries(source);
    if libs.is_empty() {
        return String::new();
    }
    // Use dynamic `await import(...)` — NOT static `import * as ...`.
    // Static imports are hoisted: the bundle would be evaluated before the outer
    // script body runs, so globalThis.createContext (etc.) wouldn't be set yet.
    // Dynamic imports run in-order during script body execution, after the preact
    // globals have been installed above.
    let mut out = String::new();
    for lib in &libs {
        if let Some(url) = zeb_bundle_url(lib) {
            let var = lib.replace('/', "_").replace('-', "_");
            out.push_str(&format!(
                "const __{var} = await import('{url}');\nObject.assign(globalThis, __{var});\n"
            ));
        }
    }
    out
}

fn build_client_module(client_source: &str, zeb_preamble: &str) -> String {
    let tool_js = TOOL_INIT;
    let runtime_ready_source = strip_rwe_client_imports(client_source)
        .replace(
            "from \"npm:preact/jsx-runtime\"",
            "from \"https://esm.sh/preact@10.28.4/jsx-runtime\"",
        )
        .replace(
            "from 'npm:preact/jsx-runtime'",
            "from 'https://esm.sh/preact@10.28.4/jsx-runtime'",
        )
        .replace(
            "from \"npm:preact\"",
            "from \"https://esm.sh/preact@10.28.4\"",
        )
        .replace(
            "from 'npm:preact'",
            "from 'https://esm.sh/preact@10.28.4'",
        )
        .replace(
            "from \"npm:preact/hooks\"",
            "from \"https://esm.sh/preact@10.28.4/hooks\"",
        )
        .replace(
            "from 'npm:preact/hooks'",
            "from 'https://esm.sh/preact@10.28.4/hooks'",
        );
    let encoded = STANDARD.encode(runtime_ready_source.as_bytes());
    format!(
        "{tool_js}\n\
         import {{ h, Fragment, hydrate, createContext }} from 'https://esm.sh/preact@10.28.4';\n\
         import {{ forwardRef, memo }} from 'https://esm.sh/preact@10.28.4/compat';\n\
         import {{ useCallback, useContext, useEffect, useId, useImperativeHandle, useLayoutEffect, useMemo, useReducer, useRef, useState }} from 'https://esm.sh/preact@10.28.4/hooks';\n\
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
         globalThis.createContext = createContext;\n\
         globalThis.forwardRef = forwardRef;\n\
         globalThis.memo = memo;\n\
         globalThis.usePageState = __rweUsePageState;\n\
         globalThis.useNavigate = function useNavigate() {{\n\
           return function(href) {{\n\
             if (typeof window.rweNavigate === 'function') {{\n\
               window.rweNavigate(href);\n\
             }} else {{\n\
               window.location.href = href;\n\
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
             window.rweNavigate = function(href) {{\n\
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
                     lRoot.innerHTML = nRoot.innerHTML;\n\
                     if (nPay && lPay) lPay.textContent = nPay.textContent;\n\
                   }}\n\
                   document.querySelectorAll('style[data-rwe-tw]').forEach(function(s) {{ s.remove(); }});\n\
                   doc.querySelectorAll('style[data-rwe-tw]').forEach(function(s) {{\n\
                     var nc = document.createElement('style');\n\
                     nc.setAttribute('data-rwe-tw', '');\n\
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
                   history.pushState(null, '', href);\n\
                   window.scrollTo(0, 0);\n\
                   Promise.all(__scriptPromises).then(function() {{\n\
                     window.dispatchEvent(new CustomEvent('rwe:nav', {{ detail: {{ url: href }} }}));\n\
                   }});\n\
                   __bDone();\n\
                 }})\n\
                 .catch(function() {{ __bFail(); window.location.href = href; }});\n\
             }};\n\
             window.addEventListener('popstate', function() {{\n\
               window.rweNavigate(window.location.pathname + window.location.search);\n\
             }});\n\
           }}\n\
         }})();\n\
         const __payloadEl = document.getElementById('{PAYLOAD_ID}');\n\
         const __input = __payloadEl ? JSON.parse(__payloadEl.textContent || '{{}}') : {{}};\n\
         globalThis.ctx = __input;\n\
         {zeb_preamble}\
         const __mod = await import('data:text/javascript;base64,{encoded}');\n\
         const __Page = __mod.default;\n\
         function __RweRoot(props) {{\n\
           const [state, setState] = useState({{}});\n\
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
            * the Preact page state. useState setter is stable so this ref is safe.\n\
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
/// Page config values (title, description, etc.) are already resolved by JS at module eval time
/// via `globalThis.ctx` — no Rust-side interpolation needed.
fn build_document_shell(page_config: &Option<Value>, body_content: &str) -> String {
    let pc = page_config.as_ref();

    let lang = pc
        .and_then(|p| p.get("html"))
        .and_then(|h| h.get("lang"))
        .and_then(Value::as_str)
        .unwrap_or("en");

    let title = pc
        .and_then(|p| p.get("head"))
        .and_then(|h| h.get("title"))
        .and_then(Value::as_str)
        .unwrap_or("");

    let description = pc
        .and_then(|p| p.get("head"))
        .and_then(|h| h.get("description"))
        .and_then(Value::as_str)
        .unwrap_or("");

    let body_class = pc
        .and_then(|p| p.get("body"))
        .and_then(|b| b.get("className"))
        .and_then(Value::as_str)
        .unwrap_or("");

    let mut head = String::new();
    head.push_str("<meta charset=\"utf-8\">");
    head.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">");
    if !title.is_empty() {
        head.push_str(&format!("<title>{}</title>", escape_html(&title)));
    }
    if !description.is_empty() {
        head.push_str(&format!(
            "<meta name=\"description\" content=\"{}\">",
            escape_attr(&description)
        ));
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
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

fn escape_attr(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn escape_json_script(input: &str) -> String {
    input
        .replace("<", "\\u003c")
        .replace(">", "\\u003e")
        .replace("&", "\\u0026")
}
