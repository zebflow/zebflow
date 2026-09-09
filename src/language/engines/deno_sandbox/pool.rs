//! Embedded deno_core worker pool for sandboxed script execution.
//!
//! N pre-warmed `JsRuntime` threads replace the old external `deno` subprocess.
//! Each worker thread owns one `JsRuntime` and processes requests serially.
//! Work is dispatched round-robin across all N workers for parallelism.

use std::cell::RefCell;
use std::path::{Component, Path, PathBuf};
use std::sync::LazyLock;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use deno_core::{FastString, JsRuntime, PollEventLoopOptions, RuntimeOptions};
use deno_error::JsErrorBox;
use serde_json::Value;

use super::config::DenoSandboxConfig;

// ---------------------------------------------------------------------------
// Thread-local result slot — JS op writes here; Rust reads after run.
// ---------------------------------------------------------------------------
thread_local! {
    static SCRIPT_RESULT: RefCell<Option<String>> = const { RefCell::new(None) };
    static LOCAL_FETCH_ROOT: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    /// External hosts this run may reach. Held in Rust, never in JS: a value
    /// on `globalThis` is writable by the script it is meant to constrain.
    static ALLOWED_FETCH_HOSTS: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    /// Byte ceiling for one local read. `max_output_bytes` was declared,
    /// defaulted, patched and clamped, and then read by nothing.
    static LOCAL_FETCH_MAX_BYTES: RefCell<usize> = const { RefCell::new(64 * 1024) };
    /// Set when a run's realm rewind reported it could not fully clean up.
    static REALM_UNCLEAN: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Op: is this host on the allow-list for the run in progress?
///
/// The decision is made in Rust against host-owned state. The previous design
/// kept the list in `globalThis.__fetchConfig`, where a script could simply
/// append to it — inert only while external fetch was unimplemented, and
/// exactly the thing `--allow-net` would have made load-bearing.
#[deno_core::op2(fast)]
fn op_fetch_host_allowed(#[string] host: String, #[string] host_port: String) -> bool {
    ALLOWED_FETCH_HOSTS.with(|hosts| {
        hosts
            .borrow()
            .iter()
            .any(|allowed| allowed == &host || (!host_port.is_empty() && allowed == &host_port))
    })
}

/// Op: called by the IIFE to deliver the JSON result to Rust.
#[deno_core::op2(fast)]
fn op_script_result(#[string] json: String) {
    SCRIPT_RESULT.with(|r| *r.borrow_mut() = Some(json));
}

/// Op: synchronous file read used only by the embedded local-fetch wrapper.
///
/// User code must not be able to call this op directly. The JS sandbox hides
/// `Deno.core` before user modules run, and this op also enforces a canonical
/// root boundary as a second line of defense.
#[deno_core::op2]
#[string]
fn op_read_local_file(#[string] rel_path: String) -> Result<String, JsErrorBox> {
    // A refusal is kept distinguishable from a miss. The fetch wrapper answers
    // 404 for a file that is simply not there, and rejects on DENIED_PREFIX, so
    // a boundary refusal reaches the script as an error rather than as a
    // missing file it might reasonably ignore.
    let rel = Path::new(&rel_path);
    if rel.is_absolute()
        || rel.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        return Err(denied(format!(
            "path '{rel_path}' escaped the sandbox root"
        )));
    }

    let root = LOCAL_FETCH_ROOT
        .with(|root| root.borrow().clone())
        .ok_or_else(|| {
            denied(format!(
                "no sandbox root is configured for this run, so '{rel_path}' resolves nowhere"
            ))
        })?;
    let canonical_root = root.canonicalize().map_err(|e| {
        denied(format!(
            "sandbox root '{}' is unusable: {e}",
            root.display()
        ))
    })?;
    let target = canonical_root.join(rel);
    let canonical_target = target
        .canonicalize()
        .map_err(|e| JsErrorBox::generic(format!("local file read failed: {e}")))?;
    if !canonical_target.starts_with(&canonical_root) {
        return Err(denied(format!(
            "path '{rel_path}' escaped the sandbox root {}",
            canonical_root.display()
        )));
    }

    // Only ever read a regular file.
    //
    // `read_to_string` on a FIFO with no writer blocks in the kernel, and the
    // op is synchronous: the host watchdog terminates *JavaScript*, so it has
    // no reach into a blocked syscall. A single `mkfifo` inside a project
    // therefore wedged its worker permanently, and eight of them took every
    // worker in the pool — the exact failure the watchdog exists to prevent,
    // arriving through the one door it cannot see. Confirmed 0/8 surviving
    // before this check.
    let meta = std::fs::metadata(&canonical_target)
        .map_err(|e| JsErrorBox::generic(format!("local file read failed: {e}")))?;
    if !meta.is_file() {
        return Err(denied(format!(
            "path '{rel_path}' is not a regular file"
        )));
    }

    let limit = LOCAL_FETCH_MAX_BYTES.with(|max| *max.borrow());
    if meta.len() as usize > limit {
        return Err(denied(format!(
            "file '{rel_path}' is {} bytes, over the {limit} byte read limit",
            meta.len()
        )));
    }

    // Bounded even so: length can change between the check and the read.
    use std::io::Read;
    let file = std::fs::File::open(&canonical_target)
        .map_err(|e| JsErrorBox::generic(format!("local file read failed: {e}")))?;
    let mut buf = String::new();
    file.take(limit as u64 + 1)
        .read_to_string(&mut buf)
        .map_err(|e| JsErrorBox::generic(format!("local file read failed: {e}")))?;
    if buf.len() > limit {
        return Err(denied(format!(
            "file '{rel_path}' exceeded the {limit} byte read limit while being read"
        )));
    }
    Ok(buf)
}

/// Marker the fetch wrapper matches on to reject rather than answer 404.
///
/// `SANDBOX_INIT` is a JavaScript literal and cannot read this constant, so the
/// same string appears there; the two must change together.
const DENIED_PREFIX: &str = "local fetch denied: ";

fn denied(reason: String) -> JsErrorBox {
    JsErrorBox::generic(format!("{DENIED_PREFIX}{reason}"))
}

deno_core::extension!(script_ops, ops = [op_script_result, op_read_local_file, op_fetch_host_allowed],);

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
  var __zfOps = globalThis.Deno && globalThis.Deno.core && globalThis.Deno.core.ops;
  var __zfScriptResult = __zfOps && __zfOps.op_script_result;
  var __zfReadLocalFile = __zfOps && __zfOps.op_read_local_file;
  var __zfHostAllowed   = __zfOps && __zfOps.op_fetch_host_allowed;
  if (typeof __zfScriptResult !== "function" || typeof __zfReadLocalFile !== "function"
      || typeof __zfHostAllowed !== "function") {
    throw new Error("DenoSandboxError: host ops unavailable");
  }
  try {
    Object.defineProperty(globalThis, "__zebflow_script_result", {
      value: __zfScriptResult, writable: false, configurable: false
    });
  } catch (e) {}

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
      try {
        var content = __zfReadLocalFile(rel);
        var ct = rel.endsWith(".json") ? "application/json" : "text/plain";
        return Promise.resolve(new globalThis.Response(content, {
          status: 200, headers: { "content-type": ct }
        }));
      } catch (e) {
        // A refused path is an error, not an empty answer: only a genuine miss
        // becomes a 404.
        var msg = (e && e.message) ? String(e.message) : String(e);
        if (msg.indexOf("local fetch denied: ") !== -1) {
          return Promise.reject(new Error("DenoSandboxError: " + msg));
        }
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
      var allowed = __zfHostAllowed(host, hostPrt || "");
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

  // ----- Lockdown + per-run reset ----------------------------------------
  // The worker's realm is reused by runs belonging to different projects and
  // different customers. deno_core 0.390 exposes no public realm-creation API,
  // and rebuilding the runtime per run would cost far more than a run itself,
  // so the realm is instead hardened and rewound between runs.
  //
  // Two halves:
  //   * freeze the intrinsics, so one tenant cannot leave a poisoned
  //     `Array.prototype.map` (or `JSON.stringify`, which the run wrapper
  //     itself calls) behind for the next;
  //   * rewind the global object, so anything a tenant added is removed and
  //     anything it replaced — `fetch`, notably — is restored.
  //
  // The baseline lives in a closure and the reset is non-writable, so a script
  // can neither read it nor replace it. Every intrinsic the reset itself uses
  // is captured by reference first, so poisoning those afterwards cannot make
  // the reset misbehave.
  // ----- Close the back door to the Function constructor -----------------
  // Replacing the *global* bindings of `eval` and `Function` shut the front
  // door only. Every function object reaches the real constructor through its
  // prototype: `(function(){}).constructor`, `[].map.constructor`, and — best
  // of all — `eval.constructor`, so the lock handed back exactly what it
  // locked. There are four such intrinsics, one per function kind.
  //
  // This is what makes save-time analysis mean anything. A script that can
  // build code at run time can always generate what the parser would have
  // refused, so every static guarantee is best-effort until this is closed.
  // Script bodies live in the pipeline definition and are fixed at save, so
  // nothing legitimate needs to invent code while running. A node whose actual
  // purpose is evaluating supplied code is a separate node, granted this
  // explicitly.
  // Reached by prototype, never by name: the global `Function` binding was
  // already replaced above, so `Function.prototype` here would be the blocked
  // stub's own prototype — neutering the wrong object entirely.
  var _blockedCtor = _blocked("Function");
  var _fnProtos = [Object.getPrototypeOf(function () {})];
  try { _fnProtos.push(Object.getPrototypeOf(async function () {})); } catch (e) {}
  try { _fnProtos.push(Object.getPrototypeOf(function* () {})); } catch (e) {}
  try { _fnProtos.push(Object.getPrototypeOf(async function* () {})); } catch (e) {}
  _fnProtos.forEach(function (proto) {
    try {
      Object.defineProperty(proto, "constructor", {
        value: _blockedCtor, writable: false, configurable: false
      });
    } catch (e) {}
  });

  var _getOwn = Object.getOwnPropertyNames;
  var _freeze = Object.freeze;
  var _defineProp = Object.defineProperty;
  var _getDesc = Object.getOwnPropertyDescriptor;
  // `===` is the wrong comparison for a baseline sweep: the global `NaN` is
  // non-configurable and `NaN === NaN` is false, so every rewind concluded it
  // had been tampered with, declared the realm unclean, and retired the worker
  // — on every run, for a value nobody had touched.
  var _same = Object.is;

  // `Function` is already the blocked stub by now, so naming
  // `Function.prototype` here would freeze the stub's own prototype and leave
  // the real one writable — the same trap the constructor block above calls
  // out, and it was originally repeated here. That mattered: with the real
  // prototype unfrozen a script could replace `Function.prototype.call`, and
  // the realm reset below calls `indexOf.call(...)`, so a poisoned `call`
  // made the reset a no-op and cross-run isolation quietly failed.
  var _frozenTargets = [
    Object.prototype, Array.prototype, String.prototype,
    Number.prototype, Boolean.prototype, Symbol.prototype, Date.prototype,
    RegExp.prototype, Error.prototype, Promise.prototype, Map.prototype,
    Set.prototype, WeakMap.prototype, WeakSet.prototype,
    Object, Array, String, Number, Boolean, Date, RegExp, Promise,
    JSON, Math, Reflect
  ];
  for (var _fp = 0; _fp < _fnProtos.length; _fp++) {
    _frozenTargets[_frozenTargets.length] = _fnProtos[_fp];
  }
  _frozenTargets.forEach(function (target) {
    try { _freeze(target); } catch (e) {}
  });


  // Hide raw deno_core host capability access from user scripts. Supported
  // capabilities are exposed only through Tool.* and the `n` capability object.
  try {
    Object.defineProperty(globalThis, "Deno", {
      value: undefined, writable: false, configurable: false
    });
  } catch (e) {
    try { if (globalThis.Deno) Object.defineProperty(globalThis.Deno, "core", {
      value: undefined, writable: false, configurable: false
    }); } catch (_) {}
  }

  // Installed LAST, deliberately. The baseline must be taken after every
  // mutation the bootstrap itself makes — `Deno` is replaced with a
  // non-configurable `undefined` below, and a snapshot taken earlier captured
  // the real object, so every rewind saw `Deno` as changed and unrestorable
  // and reported the realm unclean. That retired a worker on every single run,
  // rebuilding a JsRuntime each time and costing 5x the run itself.
  try {
    _defineProp(globalThis, "__zfResetRealm", {
      value: (function () {
        // Captured before any script can run, and unreachable afterwards.
        // Descriptors, not values. An accessor property has no `value`, so a
        // value-based baseline saw every getter as "changed" on every run,
        // reported the realm unclean, and retired the worker each time —
        // rebuilding a JsRuntime per run and costing 5x the run itself.
        // Descriptors also let restoration go through defineProperty, which
        // never invokes a setter the previous tenant may have installed.
        // This baseline is computed *inside* the expression that defines
        // `__zfResetRealm`, so that property does not exist yet and cannot be
        // observed here. Left out, the rewind found a non-base,
        // non-configurable global — itself — and declared the realm unclean on
        // every single run.
        var baseNames = _getOwn(globalThis);
        baseNames[baseNames.length] = "__zfResetRealm";
        var baseDescs = [];
        for (var i = 0; i < baseNames.length; i++) {
          try { baseDescs[i] = _getDesc(globalThis, baseNames[i]); } catch (e) { baseDescs[i] = null; }
        }
        // Membership is decided with a plain loop rather than
        // `Array.prototype.indexOf.call`. Any method reached through a
        // prototype is something a script can replace, and a reset that calls
        // a poisoned method is a reset that does nothing — which is exactly
        // how this failed before the prototypes above were frozen. Frozen
        // intrinsics make that unreachable now; not depending on them at all
        // means a future gap in the freeze list cannot silently disarm this.
        var isBase = function (key) {
          for (var b = 0; b < baseNames.length; b++) {
            if (baseNames[b] === key) return true;
          }
          return false;
        };
        // Returns true when the realm was fully rewound. A false answer means
        // the worker is carrying state into the next tenant and the host must
        // retire it.
        return function () {
          var clean = true;
          var now = _getOwn(globalThis);
          for (var i = 0; i < now.length; i++) {
            var key = now[i];
            if (!isBase(key)) {
              var d = _getDesc(globalThis, key);
              if (d && !d.configurable) { clean = false; continue; }
              try { delete globalThis[key]; } catch (e) { clean = false; }
              if (_getDesc(globalThis, key)) clean = false;
            }
          }
          for (var j = 0; j < baseNames.length; j++) {
            var name = baseNames[j];
            var base = baseDescs[j];
            if (!base) continue;
            try {
              var desc = _getDesc(globalThis, name);
              var same;
              if (base.get || base.set) {
                same = !!desc && desc.get === base.get && desc.set === base.set;
              } else {
                same = !!desc && "value" in desc && _same(desc.value, base.value);
              }
              if (same) continue;
              if (desc && !desc.configurable) { clean = false; continue; }
              _defineProp(globalThis, name, base);
            } catch (e) { clean = false; }
          }
          return clean;
        };
      })(),
      writable: false,
      configurable: false
    });
  } catch (e) {}
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

/// Instruction to a worker's watchdog thread.
enum WatchCmd {
    /// A run has started; terminate the isolate if it is not disarmed in time.
    Arm(Duration),
    /// The run finished on its own.
    Disarm,
}

/// Spawns the watchdog that owns this generation's kill switch.
///
/// # Why the host has to hold this
///
/// Every limit that lives *inside* the sandbox shares a scope with the script
/// it is meant to constrain. The op budget was a global function, so a script
/// could shadow it with a local of the same name; the deadline consulted
/// `Date.now`, so a script could replace the clock. Both were confirmed
/// defeated. An `IsolateHandle` is unforgeable from JS: there is no expression
/// a script can write that reaches it.
///
/// One thread per worker generation, not per run — `{{ }}` expression
/// resolution goes through this pool on every node, and a thread spawn per
/// expression would be a latency cost paid by every pipeline.
type WatchChannels = (
    std::sync::mpsc::SyncSender<WatchCmd>,
    std::sync::mpsc::Receiver<bool>,
);

fn spawn_watchdog(handle: deno_core::v8::IsolateHandle) -> WatchChannels {
    let (cmd_tx, cmd_rx) = std::sync::mpsc::sync_channel::<WatchCmd>(2);
    let (ack_tx, ack_rx) = std::sync::mpsc::sync_channel::<bool>(1);
    std::thread::Builder::new()
        .name("deno-sandbox-watchdog".to_string())
        .spawn(move || {
            loop {
                match cmd_rx.recv() {
                    Ok(WatchCmd::Arm(budget)) => {
                        let fired = match cmd_rx.recv_timeout(budget) {
                            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {
                                handle.terminate_execution();
                                // The worker's Disarm is still in flight; take
                                // it here so it cannot be mistaken for the
                                // start of the next run.
                                if cmd_rx.recv().is_err() {
                                    return;
                                }
                                true
                            }
                            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return,
                            Ok(_) => false,
                        };
                        if ack_tx.send(fired).is_err() {
                            return;
                        }
                    }
                    Ok(WatchCmd::Disarm) => {}
                    Err(_) => return,
                }
            }
        })
        .expect("deno-sandbox: failed spawning watchdog");
    (cmd_tx, ack_rx)
}

fn build_runtime() -> JsRuntime {
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

    js_rt
}

fn run_worker_thread(rx: std::sync::mpsc::Receiver<WorkItem>) {
    // Each worker runs its own single-threaded Tokio executor.
    // JsRuntime is !Send — it must stay on this exact thread.
    let tokio_rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("deno-sandbox: failed building tokio runtime");

    tokio_rt.block_on(async move {
        // Each generation is one JsRuntime plus the watchdog holding its kill
        // switch. A terminated run retires the generation: V8 leaves the
        // isolate in a terminating state, and the realm may be half-mutated by
        // a script that was cut off mid-statement. Rebuilding is the only way
        // to hand the next tenant a runtime that means anything.
        loop {
            let mut js_rt = build_runtime();
            let handle = js_rt.v8_isolate().thread_safe_handle();
            let (watch_tx, watch_ack) = spawn_watchdog(handle);

            let mut retire = false;
            while let Ok(item) = rx.recv() {
                let budget = Duration::from_millis(item.work.config.timeout_ms.max(1));
                let _ = watch_tx.send(WatchCmd::Arm(budget));

                let result = execute_script(&mut js_rt, item.work).await;

                // Block until the watchdog has *decided*. A flag alone raced:
                // the watchdog could resolve to fire, be descheduled before
                // publishing that, and have the worker read "not fired", keep
                // the runtime, and take the kill on the following request.
                // After this handshake no termination can still be pending.
                let _ = watch_tx.send(WatchCmd::Disarm);
                let terminated = watch_ack.recv().unwrap_or(false);

                // Rewind now, while this run's mess is this run's problem. A
                // realm that will not come clean is carrying one tenant's
                // state toward the next, so the worker is retired instead.
                let unclean = !terminated && REALM_UNCLEAN.with(|flag| flag.get());


                let result = if terminated {
                    // Whatever error unwinding produced, the true cause is the
                    // watchdog, and the caller should be told that plainly.
                    Err("DenoSandboxError: timeout exceeded".to_string())
                } else {
                    result
                };
                let _ = item.reply.send(result);

                if terminated || unclean {
                    retire = true;
                    break;
                }
            }

            if !retire {
                // The dispatch channel closed: the pool is going away.
                return;
            }
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

    // An unset root stays unset: a local fetch is then refused by name rather
    // than resolved against whatever directory the server was started in.
    let fetch_root = if cfg.local_fetch_root.trim().is_empty() {
        None
    } else {
        Some(PathBuf::from(&cfg.local_fetch_root))
    };
    LOCAL_FETCH_ROOT.with(|root| {
        *root.borrow_mut() = fetch_root;
    });
    REALM_UNCLEAN.with(|flag| flag.set(false));
    LOCAL_FETCH_MAX_BYTES.with(|max| {
        *max.borrow_mut() = cfg.max_output_bytes;
    });
    REALM_UNCLEAN.with(|flag| flag.set(false));
    LOCAL_FETCH_MAX_BYTES.with(|max| {
        *max.borrow_mut() = cfg.max_output_bytes;
    });
    ALLOWED_FETCH_HOSTS.with(|hosts| {
        *hosts.borrow_mut() = cfg
            .allow_list
            .external_fetch_hosts
            .iter()
            .map(|h| h.trim().to_ascii_lowercase())
            .filter(|h| !h.is_empty())
            .collect();
    });

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
    // The realm rewind rides along inside this wrapper rather than in a
    // second `execute_script`. A separate call meant compiling a fresh script
    // on every run, which cost more than everything else put together —
    // 170µs became 981µs, paid by every `{{ }}` in every node. Here it is free.
    let run_code = format!(
        r#"(async function () {{
  var __payload;
  try {{
    var __fn = {fn_source};
    var __r  = await __fn(globalThis.__script_input, globalThis.__script_n, globalThis.__script_ctx);
    __payload = {{ ok: true, result: __r }};
  }} catch (e) {{
    __payload = {{ ok: false, error: String(e && e.message || e) }};
  }}
  var __clean = false;
  try {{ __clean = globalThis.__zfResetRealm() === true; }} catch (e) {{ __clean = false; }}
  __payload.clean = __clean;
  globalThis.__zebflow_script_result(JSON.stringify(__payload));
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

    // A realm that would not come clean marks the worker for retirement. The
    // signal travels with the result so it costs nothing extra to collect.
    if !parsed.get("clean").and_then(Value::as_bool).unwrap_or(false) {
        REALM_UNCLEAN.with(|flag| flag.set(true));
    }
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
    // ONE absolute deadline for the whole request. Queue wait and reply wait
    // previously each got the full budget, so a caller could wait twice what
    // it asked for before hearing anything.
    let budget = Duration::from_millis(work.config.timeout_ms.saturating_add(1_000));
    let deadline = std::time::Instant::now() + budget;
    let (reply_tx, reply_rx) = std::sync::mpsc::sync_channel::<Result<Value, String>>(1);
    // A blocking send would park this thread indefinitely once a worker's
    // queue fills — and `run_in_pool` is reached from async code, so that
    // stalls a Tokio worker rather than failing one request. Bounded instead.
    let mut item = WorkItem {
        work,
        reply: reply_tx,
    };

    loop {
        match pool[idx].try_send(item) {
            Ok(()) => break,
            Err(std::sync::mpsc::TrySendError::Disconnected(_)) => {
                return Err("DenoSandboxError: worker channel disconnected".to_string());
            }
            Err(std::sync::mpsc::TrySendError::Full(returned)) => {
                if std::time::Instant::now() >= deadline {
                    return Err("DenoSandboxError: sandbox queue full".to_string());
                }
                item = returned;
                std::thread::sleep(Duration::from_millis(1));
            }
        }
    }
    let remaining = deadline.saturating_duration_since(std::time::Instant::now());
    reply_rx.recv_timeout(remaining).map_err(|err| match err {
        std::sync::mpsc::RecvTimeoutError::Timeout => {
            "DenoSandboxError: worker reply timeout".to_string()
        }
        std::sync::mpsc::RecvTimeoutError::Disconnected => {
            "DenoSandboxError: worker reply disconnected".to_string()
        }
    })?
}
