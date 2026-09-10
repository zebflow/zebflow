//! Adversarial regression suite for the Deno sandbox.
//!
//! Every test here encodes a defect that has been **empirically confirmed**
//! against the current engine. Each is `#[ignore]`d with the defect it
//! documents, so `cargo test` stays green while the work is outstanding and
//!
//!     cargo test --test language sandbox_adversarial -- --ignored --nocapture
//!
//! prints the live defect list. Drop the `#[ignore]` from a test as its fix
//! lands; a test that starts passing is a defect that is genuinely closed.
//!
//! Provenance: found by a mixed fleet — Claude subagents with repo access,
//! Grok 4.6, and Qwen2.5-Coder. Attribution is on each test, because it
//! records which lens caught what.
//!
//! METHOD NOTE, learned the hard way: dispatch is round-robin over up to 8
//! workers (`pool.rs` POOL_COUNTER). A two-run cross-run test proves nothing —
//! run B lands on a *different* worker. Every cross-run test below writes to
//! all 8 slots, then reads all 8.

use serde_json::json;
use zebflow::language::{DenoSandboxConfigPatch, DenoSandboxEngine};

const POOL: usize = 8;

fn probe(name: &str, source: &str) -> Result<serde_json::Value, String> {
    let engine = DenoSandboxEngine::default();
    match engine.run_script(source, &json!({}), None) {
        Ok(v) => {
            let s = v.to_string();
            println!("  [RETURNED] {name}: {}", &s[..s.len().min(150)]);
            Ok(v)
        }
        Err(e) => {
            println!("  [REFUSED ] {name}: {} — {}", e.code, e.message);
            Err(e.message)
        }
    }
}

// ---------------------------------------------------------------------------
// 1. Code execution — the dynamic-code lock is cosmetic.
// ---------------------------------------------------------------------------

/// `SANDBOX_INIT` replaces only the *global bindings* of `eval` and `Function`.
/// Every function object still carries `.constructor`, which is the real thing.
#[test]
fn function_constructor_routes_are_closed() {
    println!("\n=== routes to the Function constructor ===");
    let routes = [
        ("plain_fn", "(function(){}).constructor"),
        ("object_ctor_ctor", "({}).constructor.constructor"),
        ("arrow", "(()=>{}).constructor"),
        ("async_fn", "(async function(){}).constructor"),
        ("generator", "(function*(){}).constructor"),
        ("proto_of_fn", "Object.getPrototypeOf(function(){}).constructor"),
        ("array_map", "[].map.constructor"),
    ];
    let mut escaped = Vec::new();
    for (name, expr) in routes {
        let src = format!(
            "const F = {expr};\n\
             if (typeof F !== 'function') return {{ escaped: false }};\n\
             return {{ escaped: true, value: F('return 40 + 2')() }};"
        );
        if let Ok(v) = probe(name, &src) {
            if v.get("escaped").and_then(|b| b.as_bool()) == Some(true) {
                escaped.push(name);
            }
        }
    }
    assert!(escaped.is_empty(), "ESCAPE — reached Function via: {escaped:?}");
}

/// Found by Qwen: the blocked stub is *itself* a function, so the lock hands
/// back the constructor it exists to deny. `eval.constructor === Function`.
#[test]
fn blocked_stubs_do_not_leak_their_own_constructor() {
    println!("\n=== do the locks leak what they lock? ===");
    let mut leaked = Vec::new();
    for (name, stub) in [("eval", "globalThis.eval"), ("setTimeout", "globalThis.setTimeout")] {
        let src = format!(
            "const F = {stub}.constructor;\n\
             if (typeof F !== 'function') return {{ escaped: false }};\n\
             return {{ escaped: true, value: F('return 7*6')() }};"
        );
        if let Ok(v) = probe(name, &src) {
            if v.get("escaped").and_then(|b| b.as_bool()) == Some(true) {
                leaked.push(name);
            }
        }
    }
    assert!(leaked.is_empty(), "ESCAPE — locks leaked Function: {leaked:?}");
}

// ---------------------------------------------------------------------------
// 2. Cross-run contamination — the worker realm is reused across tenants.
// ---------------------------------------------------------------------------

/// Only `__tj_tick`, `__fetchConfig`, `__script_input`, `__script_n` and
/// `__script_ctx` are reset per run. Nothing clears `globalThis`.
#[test]
fn globals_do_not_persist_across_runs() {
    println!("\n=== cross-run globalThis persistence ===");
    let engine = DenoSandboxEngine::default();
    for _ in 0..POOL {
        let _ = engine.run_script(
            "globalThis.__leaked = 'tenant-a-secret'; return { w: true };",
            &json!({}),
            None,
        );
    }
    let mut hits = 0;
    for i in 0..POOL {
        if let Ok(v) = engine.run_script(
            "return { saw: globalThis.__leaked === undefined ? null : globalThis.__leaked };",
            &json!({}),
            None,
        ) {
            let saw = v.get("saw").and_then(|s| s.as_str());
            println!("  read {i}: {saw:?}");
            if saw.is_some() {
                hits += 1;
            }
        }
    }
    assert_eq!(hits, 0, "CROSS-TENANT LEAK — {hits}/{POOL} workers retained prior data");
}

/// The sharper form: a persistent `fetch` hook captures the *next* tenant's
/// reads, executed under that tenant's own `local_fetch_root`.
#[test]
fn fetch_hook_does_not_survive_into_later_runs() {
    println!("\n=== persistent fetch hook ===");
    let engine = DenoSandboxEngine::default();
    for _ in 0..POOL {
        let _ = engine.run_script(
            "if (!globalThis.__hooked) { globalThis.__hooked = true; \
             globalThis.__stash = []; var o = globalThis.fetch; \
             globalThis.fetch = function (u) { globalThis.__stash.push(String(u)); \
             return o(u); }; } return { hooked: true };",
            &json!({}),
            None,
        );
    }
    let mut survived = 0;
    for i in 0..POOL {
        if let Ok(v) = engine.run_script(
            "return { hooked: globalThis.__hooked === true };",
            &json!({}),
            None,
        ) {
            println!("  probe {i}: {v}");
            if v.get("hooked").and_then(|b| b.as_bool()) == Some(true) {
                survived += 1;
            }
        }
    }
    assert_eq!(survived, 0, "PERSISTENT HOOK — survived into {survived}/{POOL} runs");
}

/// Intrinsics are never frozen. The run wrapper calls `JSON.stringify` to
/// serialize the result, so poisoning it runs attacker code inside the *next*
/// tenant's serialization step.
#[test]
fn poisoned_intrinsics_do_not_persist_across_runs() {
    println!("\n=== cross-run intrinsic poisoning ===");
    let engine = DenoSandboxEngine::default();
    for _ in 0..POOL {
        let _ = engine.run_script(
            "Array.prototype.map = function(){ return ['poisoned']; }; return { p: true };",
            &json!({}),
            None,
        );
    }
    let mut poisoned = 0;
    for i in 0..POOL {
        if let Ok(v) = engine.run_script(
            "return { r: [1,2,3].map(function(x){ return x*2; }) };",
            &json!({}),
            None,
        ) {
            println!("  victim {i}: {v}");
            let hit = v
                .get("r")
                .and_then(|r| r.as_array())
                .map(|a| a.iter().any(|x| x.as_str() == Some("poisoned")))
                .unwrap_or(false);
            if hit {
                poisoned += 1;
            }
        }
    }
    assert_eq!(poisoned, 0, "POISONING — {poisoned}/{POOL} victims got a poisoned map");
}

// ---------------------------------------------------------------------------
// 3. Resource metering — the budget and the deadline both disarm.
// ---------------------------------------------------------------------------

/// `__tj_tick` is installed by plain assignment, not `defineProperty`. The
/// legacy `runtime/secure_js_runner.js:156` locked it; the embedded port lost
/// that. Both the op budget and the wall-clock deadline live inside it.
#[test]
fn tick_global_is_not_writable() {
    println!("\n=== overwrite the global tick ===");
    let src = "globalThis.__tj_tick = function(){}; let i=0,s=0; \
               while(i<2000000){ s+=i; i++; } return { done: true, s };";
    if let Ok(v) = probe("global_overwrite", src) {
        assert!(
            v.get("done").is_none(),
            "BUDGET DISARMED — 2M iterations completed after overwriting __tj_tick"
        );
    }
}

/// Found by Grok, and strictly stronger than the overwrite: the injected
/// guards call the *bare identifier*, so a function-scoped `var __tj_tick`
/// captures them. A `defineProperty` lock on the global does not help.
#[test]
fn tick_cannot_be_shadowed_by_a_local() {
    println!("\n=== shadow the tick with a local ===");
    let src = "var __tj_tick = function(){}; let i=0,s=0; \
               while(i<2000000){ s+=i; i++; } return { done: true, s };";
    if let Ok(v) = probe("local_shadow", src) {
        assert!(
            v.get("done").is_none(),
            "BUDGET DISARMED — a local __tj_tick captured the injected guards"
        );
    }
}

/// Found by Grok: the in-sandbox deadline check is `Date.now() > __deadline`,
/// so the clock it consults is one the script owns.
///
/// The work here is a catastrophic-backtracking regex, chosen deliberately: it
/// contains no loop, so it emits no ticks and the in-sandbox deadline can
/// never fire for it at all. If a poisoned clock could defeat the host too,
/// this would run to completion. Anything that stops it is therefore the
/// watchdog, and nothing else.
#[test]
fn poisoning_the_clock_cannot_defeat_the_host_deadline() {
    println!("\n=== poisoned clock vs the host watchdog ===");
    let engine = DenoSandboxEngine::default();
    let patch = DenoSandboxConfigPatch {
        timeout_ms: Some(200),
        ..Default::default()
    };
    let src = "Date.now = function(){ return 0; };\n\
               const re = /(a+)+$/;\n\
               return { m: re.test('a'.repeat(60) + '!') };";

    let started = std::time::Instant::now();
    let outcome = engine.run_script(src, &json!({}), Some(&patch));
    let elapsed = started.elapsed();
    println!("  outcome: {outcome:?} after {elapsed:?}");

    assert!(
        outcome.is_err(),
        "DEADLINE DEFEATED — a poisoned clock let unmeterable work finish"
    );
    assert!(
        elapsed < std::time::Duration::from_secs(10),
        "took {elapsed:?} — the watchdog did not fire promptly"
    );
}

/// Every loop *form* must be metered, not just the ones a text scanner
/// happened to anticipate. An AST transform sees all of them by construction.
#[test]
fn every_loop_form_is_metered() {
    println!("\n=== loop forms ===");
    let mut unmetered = Vec::new();
    for (name, src) in [
        ("for_of", "const a=new Array(2000000).fill(1); let s=0; \
                    for (const x of a){ s+=x; } return { done: true, s };"),
        ("for_in", "const o={}; for(let i=0;i<200000;i++){o['k'+i]=i;} let s=0; \
                    for (const k in o){ s+=o[k]; } return { done: true, s };"),
        ("for_ever", "let i=0,s=0; for(;;){ s+=i; i++; if(i>2000000) break; } \
                      return { done: true, s };"),
        ("braceless", "let i=0,s=0; while(i<2000000) i++; return { done: true, s };"),
    ] {
        if let Ok(v) = probe(name, src) {
            if v.get("done").is_some() {
                unmetered.push(name);
            }
        }
    }
    assert!(unmetered.is_empty(), "UNMETERED loop forms: {unmetered:?}");
}

/// The undecidable residue: work with no loop node at all, which no static
/// pass can meter. Catastrophic backtracking runs inside a single native regex
/// call — there is nothing to instrument. Only the host can stop it.
#[test]
fn unmeterable_work_is_stopped_by_the_host() {
    println!("\n=== work no static pass can meter ===");
    let engine = DenoSandboxEngine::default();
    let patch = DenoSandboxConfigPatch {
        timeout_ms: Some(200),
        ..Default::default()
    };
    // (a+)+$ against a non-matching string is exponential. No loop, no tick.
    let bomb = "const re = /(a+)+$/; return { m: re.test('a'.repeat(60) + '!') };";

    let started = std::time::Instant::now();
    let outcome = engine.run_script(bomb, &json!({}), Some(&patch));
    let elapsed = started.elapsed();
    println!("  regex bomb: {outcome:?} after {elapsed:?}");

    assert!(outcome.is_err(), "regex bomb completed — it should have been terminated");
    assert!(
        elapsed < std::time::Duration::from_secs(10),
        "took {elapsed:?} — the watchdog did not fire promptly"
    );
}

/// The availability property that matters most: a worker killed mid-run must
/// come back. Before the watchdog, a runaway occupied its worker for the life
/// of the process, and eight of them ended script execution platform-wide.
#[test]
fn the_pool_recovers_after_a_termination() {
    println!("\n=== pool recovery after kill ===");
    let engine = DenoSandboxEngine::default();
    let patch = DenoSandboxConfigPatch {
        timeout_ms: Some(150),
        ..Default::default()
    };
    let bomb = "const re = /(a+)+$/; return { m: re.test('a'.repeat(60) + '!') };";

    // Kill every worker in the pool, twice over.
    for i in 0..(POOL * 2) {
        let killed = engine.run_script(bomb, &json!({}), Some(&patch)).is_err();
        assert!(killed, "bomb {i} was not terminated");
    }

    // Every worker must now serve ordinary work again.
    for i in 0..(POOL * 2) {
        let v = engine
            .run_script("return { ok: 1 + 1 };", &json!({}), None)
            .unwrap_or_else(|e| panic!("worker still wedged after {i} recoveries: {e:?}"));
        assert_eq!(v.get("ok").and_then(|n| n.as_i64()), Some(2));
    }
    println!("  all {} workers served work after being killed", POOL * 2);
}

/// Found by Qwen. The scanner matches the keyword adjacent to its paren, so a
/// comment between them hides the loop from instrumentation entirely.
#[test]
fn comment_between_keyword_and_paren_is_still_guarded() {
    println!("\n=== while/**/(...) ===");
    let src = "let i=0,s=0; while/**/(i<2000000){ s+=i; i++; } return { done: true, s };";
    if let Ok(v) = probe("comment_gap", src) {
        assert!(
            v.get("done").is_none(),
            "UNMETERED — a comment between `while` and `(` skipped the guard"
        );
    }
}

// ---------------------------------------------------------------------------
// 4. The rewriter and the scanner corrupt or refuse valid programs.
// ---------------------------------------------------------------------------

/// `inject_loop_guards` edits source as text, so it rewrites inside regex
/// literals. One case silently changes a program's meaning; the other turns a
/// valid program into a SyntaxError.
#[test]
fn loop_rewriter_leaves_regex_literals_alone() {
    println!("\n=== regex literals vs the text rewriter ===");
    let mut broken = Vec::new();

    // Meaning changed: /for (a;b;c)/ has source length 11, not 30.
    if let Ok(v) = probe("regex_source_len", "const re = /for (a;b;c)/; return { len: re.source.length };") {
        let len = v.get("len").and_then(|l| l.as_u64());
        println!("    expected 11, got {len:?}");
        if len != Some(11) {
            broken.push("source rewritten inside regex literal");
        }
    }
    // Valid program turned into a syntax error.
    let src = "let i=0,s=0; while (/(a)(b)/.test('x') === false && i<10){ s+=i; i++; } \
               return { done: true, s };";
    if probe("regex_in_condition", src).is_err() {
        broken.push("valid program became a SyntaxError");
    }

    assert!(broken.is_empty(), "REWRITER CORRUPTION: {broken:?}");
}

/// `forbid_patterns` substring-scans the lowercased source, so ordinary prose
/// and data trip it. This breaks honest users, not attackers.
#[test]
fn innocent_scripts_are_not_refused() {
    println!("\n=== false positives ===");
    let innocent = [
        ("prose_import", "return { doc: 'import the CSV before you begin' };"),
        ("markdown_eval", "return { md: 'Never call eval() in user code.' };"),
    ];
    let mut refused = Vec::new();
    for (name, src) in innocent {
        if probe(name, src).is_err() {
            refused.push(name);
        }
    }
    assert!(refused.is_empty(), "FALSE POSITIVES — harmless scripts refused: {refused:?}");
}

// ---------------------------------------------------------------------------
// 5. The fetch allow-list is writable by the script it constrains.
// ---------------------------------------------------------------------------

/// Layer one: the reserved name is refused before the script ever runs.
#[test]
fn assigning_the_fetch_config_is_refused_at_compile() {
    println!("\n=== allow-list tamper, static layer ===");
    let err = probe(
        "assign_fetchconfig",
        "globalThis.__fetchConfig = { allowedHosts: ['evil.example.com'] }; return 1;",
    )
    .expect_err("assigning a reserved runtime name must be refused");
    assert!(err.contains("__fetchConfig"), "{err}");
}

/// Layer two, verified independently: a computed key is deliberately invisible
/// to the static pass, so this reaches the runtime. The allow-list now lives in
/// Rust rather than on `globalThis`, so there is nothing for the script to
/// widen — asserted by consequence, not by inspection. A host that was never
/// granted must still come back *denied*, not merely unimplemented.
#[test]
fn a_script_cannot_widen_its_own_fetch_allowlist_at_run_time() {
    println!("\n=== allow-list tamper, runtime layer ===");
    let src = "const k = '__fetch' + 'Config';\n\
       let planted = false;\n\
       try { globalThis[k] = { allowedHosts: ['evil.example.com'] }; planted = true; } catch (e) {}\n\
       let verdict;\n\
       try { await fetch('https://evil.example.com/x'); verdict = 'FETCHED'; }\n\
       catch (e) { verdict = String(e.message || e); }\n\
       return { planted, verdict };";

    let v = probe("widen_at_runtime", src).expect("the computed form should reach the runtime");
    let verdict = v.get("verdict").and_then(|s| s.as_str()).unwrap_or("");
    assert!(
        verdict.contains("denied"),
        "ALLOW-LIST WIDENED — after planting a config, the fetch reported \
         {verdict:?} instead of denied"
    );
    assert!(
        !verdict.contains("not implemented"),
        "the planted host was treated as granted: {verdict:?}"
    );
}

/// Why the codegen route had to close, stated as a test.
///
/// Save-time analysis can only be load-bearing if a script cannot build, at
/// run time, the code the parser would have refused at save. This pair proves
/// the two halves meet: the direct form is refused by the analyzer, and the
/// generated form can no longer be constructed at all. Before the
/// `.constructor` route was shut, the second case returned cleanly and every
/// static guarantee was decorative.
#[test]
fn generated_code_cannot_evade_the_save_time_analyzer() {
    println!("\n=== codegen vs the static layer ===");

    let direct = "globalThis.__fetchConfig = { allowedHosts: ['evil'] }; return { planted: true };";
    let refused = probe("written_directly", direct)
        .expect_err("the analyzer must refuse the literal form");
    assert!(refused.contains("__fetchConfig"), "{refused}");

    let generated = "const F = (function(){}).constructor;\n\
                     F(\"globalThis['__fetch'+'Config'] = { allowedHosts: ['evil'] }\")();\n\
                     return { planted: typeof globalThis.__fetchConfig };";
    match probe("generated_at_run", generated) {
        Err(_) => {} // codegen refused — the static layer holds
        Ok(v) => panic!(
            "STATIC LAYER BYPASSED — generated code ran and planted {v}; \
             every save-time guarantee is best-effort while this works"
        ),
    }
}

/// `atob` / `btoa` — the WinterTC base64 pair.
///
/// Pure computation: no host reach, nothing granted, the sandbox's guarantees
/// unchanged. Added because reading an identity out of an OAuth `id_token`
/// means decoding a JWT payload, and without these a pipeline author has to
/// hand-roll base64 — which is how subtly wrong decoders get written.
#[test]
fn base64_round_trips_including_the_jwt_shape() {
    let engine = DenoSandboxEngine::default();

    let v = engine
        .run_script(
            "const s = 'zebflow ✓'; const b = btoa(unescape(encodeURIComponent(s))); \
             return { b, back: decodeURIComponent(escape(atob(b))) };",
            &json!({}),
            None,
        )
        .expect("round trip");
    assert_eq!(v.get("back").and_then(|s| s.as_str()), Some("zebflow ✓"));

    // A JWT payload segment: base64url, and unpadded.
    let v = engine
        .run_script(
            "const seg = 'eyJlbWFpbCI6ImFAYi5jIiwibmFtZSI6IkEifQ'; \
             const json = JSON.parse(atob(seg.replace(/-/g,'+').replace(/_/g,'/'))); \
             return { email: json.email };",
            &json!({}),
            None,
        )
        .expect("jwt segment decodes");
    assert_eq!(v.get("email").and_then(|s| s.as_str()), Some("a@b.c"));

    // Garbage is refused rather than silently producing wrong bytes.
    let err = engine
        .run_script("return { v: atob('not valid base64!!') };", &json!({}), None)
        .expect_err("invalid base64 must throw");
    assert!(err.message.contains("base64"), "{}", err.message);
}

/// A script gets the same `$` scope a `{{ }}` expression gets.
///
/// `$nodes` worked in an expression and not in the script beside it — one
/// concept with two answers, and the failure was `$nodes is not defined` at
/// run time rather than anything a reader would notice. `ctx` still works.
#[test]
fn a_script_sees_the_same_scope_an_expression_does() {
    let engine = DenoSandboxEngine::default();
    let out = engine
        .run_script(
            "return { nodes: typeof $nodes, trigger: typeof $trigger, \
             placeholder: typeof $placeholder, run: typeof $run, ctx: typeof ctx };",
            &json!({}),
            None,
        )
        .expect("the $ scope must exist");
    for key in ["nodes", "placeholder", "run"] {
        assert_ne!(
            out.get(key).and_then(|v| v.as_str()),
            Some("undefined"),
            "${key} must be defined in a script: {out}"
        );
    }
    // ctx is not taken away.
    assert_eq!(out.get("ctx").and_then(|v| v.as_str()), Some("object"));
}
