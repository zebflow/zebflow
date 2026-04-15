//! Embedded deno_core worker pool for sandboxed script execution.
//!
//! N pre-warmed `JsRuntime` threads replace the old external `deno` subprocess.
//! Each worker thread owns one `JsRuntime` and processes requests serially.
//! Work is dispatched round-robin across all N workers for parallelism.

use std::cell::RefCell;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicUsize, Ordering};

use deno_core::{FastString, JsRuntime, PollEventLoopOptions, RuntimeOptions};
use deno_error::JsErrorBox;
use serde_json::Value;

use super::config::DenoSandboxConfig;

// ---------------------------------------------------------------------------
// Thread-local result slot — JS op writes here; Rust reads after run.
// ---------------------------------------------------------------------------
thread_local! {
    static SCRIPT_RESULT: RefCell<Option<String>> = const { RefCell::new(None) };
}

/// Op: called by the IIFE to deliver the JSON result to Rust.
#[deno_core::op2(fast)]
fn op_script_result(#[string] json: String) {
    SCRIPT_RESULT.with(|r| *r.borrow_mut() = Some(json));
}

/// Op: synchronous file read used by the embedded local-fetch implementation.
#[deno_core::op2]
#[string]
fn op_read_local_file(#[string] path: String) -> Result<String, JsErrorBox> {
    std::fs::read_to_string(&path)
        .map_err(|e| JsErrorBox::generic(format!("local file read failed: {e}")))
}

deno_core::extension!(script_ops, ops = [op_script_result, op_read_local_file],);

// ---------------------------------------------------------------------------
// Embedded JS installed once per worker at startup.
// ---------------------------------------------------------------------------
const TOOL_INIT: &str = include_str!("../../../language/runtime/tool_init.js");

/// Permanent sandbox security hooks installed once per worker thread.
///
/// Blocks eval/Function forever. Installs URL + Response polyfills if the
/// bare deno_core runtime does not provide them. Installs a `fetch` wrapper
/// that reads per-run `__fetchConfig` for allow-list enforcement.
const SANDBOX_INIT: &str = r#"
(function () {
  "use strict";

  // ----- URL polyfill (bare deno_core has no web APIs) --------------------
  if (typeof URL === "undefined") {
    globalThis.URL = function URL(url) {
      var m = String(url).match(
        /^(https?):\/\/([^\/:\?#]+)(?::(\d+))?(\/[^\?#]*)?(\?[^#]*)?(#.*)?/i
      );
      if (!m) { var e = new TypeError("Invalid URL: " + url); e.name = "TypeError"; throw e; }
      this.protocol = m[1].toLowerCase() + ":";
      this.hostname = m[2].toLowerCase();
      this.port     = m[3] || "";
      this.pathname = m[4] || "/";
      this.search   = m[5] || "";
      this.hash     = m[6] || "";
      this.host     = this.hostname + (this.port ? ":" + this.port : "");
      this.origin   = this.protocol + "//" + this.host;
      this.href     = url;
    };
  }

  // ----- Response polyfill -----------------------------------------------
  if (typeof Response === "undefined") {
    globalThis.Response = function Response(body, init) {
      this._body  = String(body == null ? "" : body);
      this.status = (init && init.status) || 200;
      this.ok     = this.status >= 200 && this.status < 300;
      this.headers = (init && init.headers) || {};
    };
    globalThis.Response.prototype.text = function () {
      return Promise.resolve(this._body);
    };
    globalThis.Response.prototype.json = function () {
      try { return Promise.resolve(JSON.parse(this._body)); }
      catch (e) { return Promise.reject(e); }
    };
    globalThis.Response.prototype.arrayBuffer = function () {
      var b = this._body, u = new Uint8Array(b.length);
      for (var i = 0; i < b.length; i++) u[i] = b.charCodeAt(i);
      return Promise.resolve(u.buffer);
    };
  }

  // ----- Permanent security locks ----------------------------------------
  var _blocked = function (name) {
    return function () { throw new Error("DenoSandboxError: " + name + " is disabled"); };
  };
  // eval and Function are V8 built-ins — block them once forever.
  try {
    Object.defineProperty(globalThis, "eval", {
      value: _blocked("eval"), writable: false, configurable: false
    });
  } catch (e) {}
  try {
    Object.defineProperty(globalThis, "Function", {
      value: _blocked("Function"), writable: false, configurable: false
    });
  } catch (e) {}
  // setTimeout/setInterval do not exist in bare deno_core; define as blocked
  // in case they appear via some extension.
  try {
    Object.defineProperty(globalThis, "setTimeout", {
      value: _blocked("setTimeout"), writable: false, configurable: false
    });
  } catch (e) {}
  try {
    Object.defineProperty(globalThis, "setInterval", {
      value: _blocked("setInterval"), writable: false, configurable: false
    });
  } catch (e) {}

  // ----- Permanent fetch wrapper -----------------------------------------
  // Reads __fetchConfig (set per run) so allow-list is enforced correctly.
  globalThis.__fetchConfig = { allowedHosts: [], localFetchRoot: "." };
  globalThis.__tj_tick     = function () {};
  globalThis.__script_input = null;

  globalThis.fetch = function secureFetch(input) {
    if (typeof globalThis.__tj_tick === "function") globalThis.__tj_tick();

    var raw;
    if (typeof input === "string")                  raw = input.trim();
    else if (input && typeof input.url === "string") raw = input.url.trim();
    else return Promise.reject(new Error("DenoSandboxError: unsupported fetch input"));

    // Local path (e.g. "/payload.json") — read via Rust op
    if (raw.startsWith("/")) {
      var rel = decodeURIComponent(raw).replace(/^\/+/, "");
      if (!rel || rel.indexOf("..") !== -1) {
        return Promise.reject(new Error("DenoSandboxError: local fetch path invalid"));
      }
      var localRoot = (globalThis.__fetchConfig && globalThis.__fetchConfig.localFetchRoot) || ".";
      var fullPath  = localRoot + "/" + rel;
      try {
        var content = Deno.core.ops.op_read_local_file(fullPath);
        var ct = fullPath.endsWith(".json") ? "application/json" : "text/plain";
        return Promise.resolve(new globalThis.Response(content, {
          status: 200, headers: { "content-type": ct }
        }));
      } catch (e) {
        return Promise.resolve(new globalThis.Response("Not Found", { status: 404 }));
      }
    }

    // External URL — enforce allow-list
    var parsed;
    try { parsed = new URL(raw); } catch (e) {
      return Promise.reject(new Error("DenoSandboxError: unsupported fetch url '" + raw + "'"));
    }
    if (parsed.protocol === "http:" || parsed.protocol === "https:") {
      var host    = parsed.hostname.toLowerCase();
      var hostPrt = parsed.port ? (host + ":" + parsed.port) : "";
      var allowed = false;
      var hosts   = (globalThis.__fetchConfig && globalThis.__fetchConfig.allowedHosts) || [];
      for (var i = 0; i < hosts.length; i++) {
        if (hosts[i] === host || (hostPrt && hosts[i] === hostPrt)) { allowed = true; break; }
      }
      if (!allowed) {
        return Promise.reject(new Error(
          "DenoSandboxError: external fetch denied for " + host +
          ". add it to allowList.externalFetchHosts"
        ));
      }
      return Promise.reject(new Error(
        "DenoSandboxError: external HTTP fetch not implemented in embedded mode"
      ));
    }
    return Promise.reject(new Error(
      "DenoSandboxError: fetch protocol denied (" + parsed.protocol + ")"
    ));
  };
})();
"#;

// ---------------------------------------------------------------------------
// Work item types
// ---------------------------------------------------------------------------

/// Script work dispatched to a pool worker.
pub(crate) struct ScriptWork {
    /// Async function expression (no `export default`, no TOOL_INIT prefix).
    /// Format: `async function(input, n, ctx) { <body> }`
    pub fn_source: String,
    /// Resolved sandbox configuration for this run.
    pub config: DenoSandboxConfig,
    /// JSON-serializable input value passed as first argument.
    pub input: Value,
    /// Execution context passed as `ctx` to the script.
    /// Contains `pipeline`, `request_id`, `trigger`, and `metadata`.
    pub ctx: Value,
}

struct WorkItem {
    work: ScriptWork,
    reply: std::sync::mpsc::SyncSender<Result<Value, String>>,
}

// ---------------------------------------------------------------------------
// Worker pool — N threads, each owning one JsRuntime.
// ---------------------------------------------------------------------------

const MAX_POOL_SIZE: usize = 8;

static POOL_COUNTER: AtomicUsize = AtomicUsize::new(0);

static POOL: LazyLock<Vec<std::sync::mpsc::SyncSender<WorkItem>>> = LazyLock::new(|| {
    let n = std::thread::available_parallelism()
        .map(|p| p.get())
        .unwrap_or(2)
        .min(MAX_POOL_SIZE);

    (0..n)
        .map(|i| {
            let (tx, rx) = std::sync::mpsc::sync_channel::<WorkItem>(64);
            std::thread::Builder::new()
                .name(format!("deno-sandbox-{i}"))
                .spawn(move || run_worker_thread(rx))
                .expect("failed to spawn deno sandbox worker");
            tx
        })
        .collect()
});

fn run_worker_thread(rx: std::sync::mpsc::Receiver<WorkItem>) {
    // Each worker runs its own single-threaded Tokio executor.
    // JsRuntime is !Send — it must stay on this exact thread.
    let tokio_rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("deno-sandbox: failed building tokio runtime");

    tokio_rt.block_on(async move {
        let mut js_rt = JsRuntime::new(RuntimeOptions {
            extensions: vec![script_ops::init()],
            ..Default::default()
        });

        // Install Tool.* globals once.
        js_rt
            .execute_script("<tool_init>", FastString::from_static(TOOL_INIT))
            .expect("deno-sandbox: tool_init failed");

        // Install permanent security locks + polyfills + fetch wrapper.
        js_rt
            .execute_script("<sandbox_init>", FastString::from_static(SANDBOX_INIT))
            .expect("deno-sandbox: sandbox_init failed");

        // Process requests serially (one at a time per worker).
        while let Ok(item) = rx.recv() {
            let result = execute_script(&mut js_rt, item.work).await;
            let _ = item.reply.send(result);
        }
    });
}

// ---------------------------------------------------------------------------
// Per-execution logic
// ---------------------------------------------------------------------------

async fn execute_script(js_rt: &mut JsRuntime, work: ScriptWork) -> Result<Value, String> {
    // Reset result slot from any previous run.
    SCRIPT_RESULT.with(|r| *r.borrow_mut() = None);

    let cfg = &work.config;

    let input_json = serde_json::to_string(&work.input)
        .map_err(|e| format!("DenoSandboxError: serialize input: {e}"))?;

    let fetch_cfg_json = serde_json::json!({
        "allowedHosts": cfg.allow_list.external_fetch_hosts,
        "localFetchRoot": cfg.local_fetch_root,
    })
    .to_string();

    let timeout_ms = cfg.timeout_ms;
    let max_ops = cfg.max_ops;
    let caps_expr = build_capabilities_expr(cfg);

    let ctx_json = serde_json::to_string(&work.ctx)
        .map_err(|e| format!("DenoSandboxError: serialize ctx: {e}"))?;

    // Per-run setup: fresh budget, fetch policy, input, capabilities, ctx.
    let setup = format!(
        r#"(function () {{
  "use strict";
  var __deadline = Date.now() + {timeout_ms};
  var __opsLeft  = {max_ops};
  globalThis.__tj_tick = function () {{
    __opsLeft -= 1;
    if (__opsLeft < 0) throw new Error("DenoSandboxError: op budget exceeded");
    if (Date.now() > __deadline) throw new Error("DenoSandboxError: timeout exceeded");
  }};
  globalThis.__fetchConfig  = {fetch_cfg_json};
  globalThis.__script_input = {input_json};
  globalThis.__script_n     = {caps_expr};
  globalThis.__script_ctx   = {ctx_json};
}})();"#
    );

    js_rt
        .execute_script("<per_run_setup>", setup)
        .map_err(|e| format!("DenoSandboxError: per-run setup: {e}"))?;

    // Execute user script as async IIFE.
    // fn_source = `async function(input, n, ctx) { <user body> }`
    let run_code = format!(
        r#"(async function () {{
  try {{
    var __fn = {fn_source};
    var __r  = await __fn(globalThis.__script_input, globalThis.__script_n, globalThis.__script_ctx);
    Deno.core.ops.op_script_result(JSON.stringify({{ ok: true, result: __r }}));
  }} catch (e) {{
    Deno.core.ops.op_script_result(JSON.stringify({{ ok: false, error: String(e && e.message || e) }}));
  }}
}})();"#,
        fn_source = work.fn_source,
    );

    js_rt
        .execute_script("<script_run>", run_code)
        .map_err(|e| format!("DenoSandboxError: script kick: {e}"))?;

    // Drive event loop until the async IIFE completes.
    js_rt
        .run_event_loop(PollEventLoopOptions::default())
        .await
        .map_err(|e| format!("DenoSandboxError: event loop: {e}"))?;

    // Read result stored by op_script_result.
    let result_str = SCRIPT_RESULT
        .with(|r| r.borrow_mut().take())
        .ok_or_else(|| "DenoSandboxError: script op was not called".to_string())?;

    let parsed: Value = serde_json::from_str(&result_str)
        .map_err(|e| format!("DenoSandboxError: result parse: {e}"))?;

    if parsed.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        Ok(parsed.get("result").cloned().unwrap_or(Value::Null))
    } else {
        Err(format!(
            "DenoSandboxError: {}",
            parsed
                .get("error")
                .and_then(Value::as_str)
                .unwrap_or("script execution failed")
        ))
    }
}

/// Build a frozen JS capabilities object expression from config.
fn build_capabilities_expr(cfg: &DenoSandboxConfig) -> String {
    let caps: std::collections::HashSet<&str> =
        cfg.capabilities.iter().map(String::as_str).collect();
    let mut parts: Vec<&str> = vec![];

    let time_part = r#"time: Object.freeze({ now: function () { return Date.now(); } })"#;
    let math_part = r#"math: Object.freeze({ imul: function (a, b) { return Math.imul(a|0, b|0); }, u32: function (v) { return Number(v) >>> 0; } })"#;

    if caps.contains("time.now") {
        parts.push(time_part);
    }
    if caps.contains("math.imul") || caps.contains("math.u32") {
        parts.push(math_part);
    }
    format!("Object.freeze({{ {} }})", parts.join(", "))
}

// ---------------------------------------------------------------------------
// Public dispatch entry-point
// ---------------------------------------------------------------------------

/// Send a script to the pool and block until the result arrives.
pub(crate) fn run_in_pool(work: ScriptWork) -> Result<Value, String> {
    let pool = &*POOL;
    if pool.is_empty() {
        return Err("DenoSandboxError: worker pool is empty".into());
    }
    let idx = POOL_COUNTER.fetch_add(1, Ordering::Relaxed) % pool.len();
    let (reply_tx, reply_rx) = std::sync::mpsc::sync_channel::<Result<Value, String>>(1);
    pool[idx]
        .send(WorkItem {
            work,
            reply: reply_tx,
        })
        .map_err(|_| "DenoSandboxError: worker channel disconnected".to_string())?;
    reply_rx
        .recv()
        .map_err(|_| "DenoSandboxError: worker reply disconnected".to_string())?
}
