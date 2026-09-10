//! A non-terminating render must not take its worker with it.
//!
//! `render_ssr` used to wait on a reply channel with a 10s floor and hold no
//! V8 termination handle. The caller stopped waiting; the worker did not stop
//! rendering. The pool's respawn-on-death never fired, because nothing died.
//! Measured before the fix: one runaway render, and every later render on that
//! worker timed out — while `is_pool_ready()` still answered `true`, so a
//! readiness probe reported a dead instance as healthy.
//!
//! Both tests are `#[ignore]`d because they are destructive to a process-wide
//! resource: they terminate every worker in a pool the whole `rwe` test binary
//! shares, so anything rendering in parallel meets a retiring worker. That is
//! an artefact of the test, not the product — in production one worker retires
//! at a time and round-robin puts the retry on a healthy one. Run them with:
//!
//!     cargo test --test rwe worker_wedge -- --ignored --test-threads=1
//!
//! Deliberately does not pin `RWE_WORKER_COUNT`: the pool is a process-wide
//! `LazyLock`, so setting it here would either be ignored (if another test
//! rendered first) or impose one worker on every other test in this binary.
//! Instead it wedges more workers than the default pool has.

use serde_json::json;
use zebflow::rwe::core::deno_worker;

/// Comfortably above the default pool size of 3.
const ATTEMPTS: usize = 5;

const RUNAWAY: &str = r#"
while (true) {}
export default function Page() { return <div>never reached</div>; }
"#;

const ORDINARY: &str = r#"
export default function Page() { return <div>ok</div>; }
"#;

#[test]
#[ignore = "destroys the shared worker pool; run with --ignored --test-threads=1"]
fn a_runaway_render_does_not_wedge_the_pool() {
    // Occupy every worker with a render that never returns.
    for i in 0..ATTEMPTS {
        let started = std::time::Instant::now();
        let out = deno_worker::render_ssr(RUNAWAY, &json!({}), 400);
        let took = started.elapsed();
        assert!(out.is_err(), "runaway {i} was not stopped");
        assert!(
            took < std::time::Duration::from_secs(5),
            "runaway {i} took {took:?} — the caller's 400ms allowance was ignored"
        );
    }

    // Every worker must serve ordinary work again.
    for i in 0..ATTEMPTS {
        let out = deno_worker::render_ssr(ORDINARY, &json!({}), 5_000)
            .unwrap_or_else(|e| panic!("POOL WEDGED — render {i} after runaways failed: {e:?}"));
        assert!(out.html.contains("ok"), "unexpected html: {}", out.html);
    }

    assert!(
        deno_worker::is_pool_ready(),
        "pool reports not ready after recovering"
    );
}

/// The caller's allowance is the caller's. A 10s floor meant a page asking for
/// 500ms waited ten seconds to be told it had timed out.
#[test]
#[ignore = "destroys the shared worker pool; run with --ignored --test-threads=1"]
fn a_render_deadline_is_the_callers_not_a_floor() {
    let started = std::time::Instant::now();
    let out = deno_worker::render_ssr(RUNAWAY, &json!({}), 300);
    let took = started.elapsed();
    assert!(out.is_err(), "runaway render returned a result");
    assert!(
        took < std::time::Duration::from_secs(3),
        "asked for 300ms, waited {took:?}"
    );
}
