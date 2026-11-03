//! A worker's scheduled restart must not fail the renders queued behind it.
//!
//! Each runtime retires after a fixed number of renders. It used to forward
//! only the render in hand and drop its queue; every caller waiting in that
//! queue lost its reply channel, and its one retry could land on another
//! runtime retiring at the same moment. Under sustained load that surfaced as
//! a few HTTP 500s per few thousand pages. The queue is now handed to the
//! fresh runtime, so every render queued across a restart is served.
use serde_json::json;
use zebflow::rwe::core::deno_worker;

const PAGE: &str = r#"
export default function Page() { return <div>ok</div>; }
"#;

/// Enough concurrent renders that every runtime passes its restart point
/// (500 renders) while others wait in its queue.
const THREADS: usize = 16;
const EACH: usize = 120;

#[test]
fn renders_queued_across_a_restart_are_all_served() {
    let failures: Vec<String> = std::thread::scope(|scope| {
        let handles: Vec<_> = (0..THREADS)
            .map(|_| {
                scope.spawn(|| {
                    (0..EACH)
                        .filter_map(|_| deno_worker::render_ssr(PAGE, &json!({}), 5_000).err().map(|e| format!("{e:?}")))
                        .collect::<Vec<_>>()
                })
            })
            .collect();
        handles.into_iter().flat_map(|h| h.join().expect("render thread panicked")).collect()
    });
    assert!(failures.is_empty(), "{} of {} renders failed across restarts: {:?}", failures.len(), THREADS * EACH, failures.first());
}
