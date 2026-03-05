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
static CLIENT_TRANSPILE_CACHE: LazyLock<Mutex<HashMap<u64, String>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn prewarm(compiled: &CompiledTemplate) -> Result<(), EngineError> {
    let _ = transpile_client_cached(&compiled.client_module_source, compiled.deno_timeout_ms)?;
    let _ = deno_worker::render_ssr(&compiled.server_module_source, &json!({}), compiled.deno_timeout_ms)?;
    Ok(())
}

pub fn render(compiled: &CompiledTemplate, vars: &Value) -> Result<RenderOutput, EngineError> {
    let started = Instant::now();
    let ssr_html = deno_worker::render_ssr(
        &compiled.server_module_source,
        vars,
        compiled.deno_timeout_ms,
    )?;
    let transpiled_client =
        transpile_client_cached(&compiled.client_module_source, compiled.deno_timeout_ms)?;
    let ssr_ms = started.elapsed().as_millis();

    let payload_json = serde_json::to_string(vars).map_err(|e| {
        EngineError::new(
            "RWE_PAYLOAD_JSON",
            format!("failed serializing hydration payload: {e}"),
        )
    })?;

    let html = format!(
        "<div id=\"{ROOT_ID}\">{ssr_html}</div><script type=\"application/json\" id=\"{PAYLOAD_ID}\">{}</script>",
        escape_json_script(&payload_json)
    );

    let js = build_client_module(&transpiled_client);

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
            html_bytes: ssr_html.len(),
            js_bytes: js.len(),
            css_bytes: 0,
            ssr_ms,
        },
    })
}

fn build_client_module(client_source: &str) -> String {
    let runtime_ready_source = client_source
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
        "import {{ h, Fragment, hydrate, createContext }} from 'https://esm.sh/preact@10.28.4';\n\
         import {{ useContext, useEffect, useMemo, useRef, useState }} from 'https://esm.sh/preact@10.28.4/hooks';\n\
         const __RwePageStateContext = createContext(null);\n\
         function __rweUsePageState(initial={{}}) {{\n\
           const ctx = useContext(__RwePageStateContext);\n\
           if (ctx) return ctx;\n\
           const [state, setState] = useState(initial || {{}});\n\
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
         globalThis.usePageState = __rweUsePageState;\n\
         const __payloadEl = document.getElementById('{PAYLOAD_ID}');\n\
         const __input = __payloadEl ? JSON.parse(__payloadEl.textContent || '{{}}') : {{}};\n\
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
           return h(__RwePageStateContext.Provider, {{ value }}, h(__Page, props));\n\
         }}\n\
         const __root = document.getElementById('{ROOT_ID}');\n\
         if (__root && typeof __Page === 'function') {{\n\
           hydrate(h(__RweRoot, __input), __root);\n\
         }}\n"
    )
}

fn transpile_client_cached(source: &str, timeout_ms: u64) -> Result<String, EngineError> {
    let key = stable_hash_u64(source);
    if let Ok(cache) = CLIENT_TRANSPILE_CACHE.lock()
        && let Some(cached) = cache.get(&key)
    {
        return Ok(cached.clone());
    }

    let transpiled = deno_worker::transpile_client(source, timeout_ms)?;

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

fn escape_json_script(input: &str) -> String {
    input
        .replace("<", "\\u003c")
        .replace(">", "\\u003e")
        .replace("&", "\\u0026")
}
