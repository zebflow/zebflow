//! Latency guard for the sandbox pool.
//!
//! `{{ }}` expression resolution runs through this pool on every pipeline
//! node, so per-run cost is paid platform-wide, not just by `n.script`. The
//! per-run realm rewind that closes cross-tenant contamination costs roughly
//! 60µs of the figure below; that trade was made deliberately.
//!
//! # Why the minimum, not the mean
//!
//! The pool is a process-global static and the rest of this suite runs in
//! parallel — including tests that kill every worker and force runtime
//! rebuilds. A mean measured against that neighbourhood is measuring the
//! neighbours: it read 188µs alone and 11ms beside them. The minimum across
//! several batches is the cost when the pool is actually free, which is the
//! number this guard is about. The bound stays loose: it exists to catch an
//! order-of-magnitude regression — someone rebuilding the runtime per run —
//! not to police microseconds on a shared machine.

use serde_json::json;
use zebflow::language::DenoSandboxEngine;

#[test]
fn a_warm_run_stays_well_under_a_millisecond() {
    let engine = DenoSandboxEngine::default();
    let src = "return { ok: 1 };";
    for _ in 0..16 {
        let _ = engine.run_script(src, &json!({}), None);
    }

    const BATCHES: u32 = 12;
    const PER_BATCH: u32 = 20;
    let mut best = std::time::Duration::MAX;
    for _ in 0..BATCHES {
        let started = std::time::Instant::now();
        for _ in 0..PER_BATCH {
            engine
                .run_script(src, &json!({}), None)
                .expect("warm run must succeed");
        }
        best = best.min(started.elapsed() / PER_BATCH);
    }

    println!("\n  warm run: {best:?} per script (best of {BATCHES} batches)");
    assert!(
        best < std::time::Duration::from_micros(700),
        "warm run cost {best:?} even at its best — healthy is ~190µs; a cost \
         near 1ms means the worker is being retired and rebuilt on every run"
    );
}
