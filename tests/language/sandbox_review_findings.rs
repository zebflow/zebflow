//! Regressions for defects found by an independent model review of the first
//! round of sandbox hardening.
//!
//! Provenance matters here: every one of these is a hole in a fix that had
//! already been written, tested, and reported as complete. They are the case
//! for having something other than the author check the author's work.
//!
//! Two claims from that review did NOT reproduce against this runtime —
//! replacing `globalThis` wholesale, and symbol-keyed or `Tool.*` mutation
//! surviving a rewind. They were demonstrated in Node VM contexts, which are
//! not bare `deno_core`. They are left unasserted rather than asserted as
//! safe: not reproducing is not the same as being impossible.

use serde_json::json;
use zebflow::language::DenoSandboxEngine;

const POOL: usize = 8;

fn leaks_across_runs(attack: &str, check: &str) -> bool {
    let engine = DenoSandboxEngine::default();
    for _ in 0..POOL {
        let _ = engine.run_script(attack, &json!({}), None);
    }
    for _ in 0..POOL {
        if let Ok(v) = engine.run_script(check, &json!({}), None) {
            if v.get("leaked").and_then(|b| b.as_bool()) == Some(true) {
                return true;
            }
        }
    }
    false
}

/// `with` rebinds every free name in its body against an object the script
/// controls, which captures the injected `__tj_tick()` calls. The body is not
/// strict-mode, so the engine accepts it. The host deadline still fired, but
/// the op budget was silently void.
#[test]
fn with_statement_cannot_capture_the_injected_tick() {
    let engine = DenoSandboxEngine::default();
    let src = "let i = 0;\n\
               with ({ __tj_tick: function(){} }) { while (i < 2000000) i++; }\n\
               return { done: true, i };";
    let err = engine
        .run_script(src, &json!({}), None)
        .expect_err("`with` must be refused");
    assert!(err.message.contains("with"), "{}", err.message);
}

/// Shorthand object destructuring reaches its target through a node that
/// neither the identifier nor the member-expression check ever visits, so a
/// reserved name could be reassigned with none of the guards firing.
#[test]
fn shorthand_destructuring_cannot_assign_a_reserved_name() {
    let engine = DenoSandboxEngine::default();
    let src = "let i = 0;\n\
               ({ __tj_tick } = { __tj_tick: function(){} });\n\
               while (i < 2000000) i++;\n\
               return { done: true, i };";
    let err = engine
        .run_script(src, &json!({}), None)
        .expect_err("shorthand destructuring of a reserved name must be refused");
    assert!(err.message.contains("__tj_tick"), "{}", err.message);
}

/// The freeze list named `Function.prototype` — but by then the global
/// `Function` binding was already the blocked stub, so it froze the stub's
/// prototype and left the real one writable. The same trap the constructor
/// block two lines above explicitly warns about.
#[test]
fn the_real_function_prototype_is_frozen() {
    let engine = DenoSandboxEngine::default();
    let src = "const realProto = Object.getPrototypeOf(function(){});\n\
               let replaced = false;\n\
               try { realProto.call = function(){ return 0; };\n\
                     replaced = realProto.call() === 0; } catch (e) {}\n\
               return { frozen: Object.isFrozen(realProto), replaced };";
    let v = engine.run_script(src, &json!({}), None).expect("should run");
    assert_eq!(v.get("frozen").and_then(|b| b.as_bool()), Some(true));
    assert_eq!(v.get("replaced").and_then(|b| b.as_bool()), Some(false));
}

/// The consequence of the above, and why it mattered: the realm rewind used
/// `indexOf.call(...)`, so a poisoned `Function.prototype.call` made the
/// rewind a silent no-op and cross-tenant isolation failed with no signal.
#[test]
fn poisoning_call_cannot_disarm_the_realm_rewind() {
    let leaked = leaks_across_runs(
        "try { Object.getPrototypeOf(function(){}).call = function(){ return 0; }; } catch(e){}\n\
         globalThis.__leftover = 42; return { a: 1 };",
        "return { leaked: globalThis.__leftover === 42 };",
    );
    assert!(!leaked, "a poisoned `call` disarmed the rewind");
}

/// A non-configurable global cannot be deleted, so the rewind could not undo
/// it. The realm is now checked for cleanliness and the worker retired when it
/// will not come clean.
#[test]
fn a_non_configurable_global_cannot_outlive_its_run() {
    let leaked = leaks_across_runs(
        "try { Object.defineProperty(globalThis, '__locked', { value: 42 }); } catch(e){}\n\
         return { a: 1 };",
        "return { leaked: globalThis.__locked === 42 };",
    );
    assert!(!leaked, "a non-configurable global survived into a later run");
}

/// The tenant who leaves an unrewindable realm behind is the one whose worker
/// is retired. The next run must land on a healthy worker and simply work.
#[test]
fn an_unclean_realm_does_not_fail_the_following_run() {
    let engine = DenoSandboxEngine::default();
    for _ in 0..POOL {
        let _ = engine.run_script(
            "try { Object.defineProperty(globalThis, '__locked2', { value: 1 }); } catch(e){}\n\
             return { a: 1 };",
            &json!({}),
            None,
        );
    }
    for i in 0..POOL {
        let v = engine
            .run_script("return { ok: 2 + 2 };", &json!({}), None)
            .unwrap_or_else(|e| panic!("innocent run {i} paid for a previous tenant: {e:?}"));
        assert_eq!(v.get("ok").and_then(|n| n.as_i64()), Some(4));
    }
}
