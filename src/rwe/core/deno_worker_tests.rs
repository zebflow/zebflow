//! A render must leave nothing of its `ctx` in its runtime's heap.
//!
//! Each render used to write its `ctx` as JSON into the source of two
//! scripts, `<rwe_ctx_init>` and `<rwe_render>`, so every render compiled two
//! scripts no other render shares, the way the script sandbox did before
//! `op_run_args`. Measured here on that code, over 2,000 renders: 7,113 bytes
//! kept per render with a small ctx and 57,381 with 16 KiB of ctx, about three
//! times the ctx, held until a collection forced by memory pressure.
//!
//! The page module itself is still loaded once per render (a runtime never
//! unloads one, which is why a worker retires); that cost is the same whatever
//! the ctx, so the test compares a small ctx with a large one and reads only
//! the difference.

use std::rc::Rc;

use serde_json::{Value, json};

use super::*;

/// Renders measured per ctx size.
const RENDERS: usize = 2_000;
const SAMPLE_EVERY: usize = 50;
/// The large ctx carries this much text unique to its render.
const LARGE_CTX_BYTES: usize = 16 * 1024;

const PAGE: &str = "export default function Page(input) { return <main>{input.title}</main>; }";

/// A runtime made as a pool worker makes one, without its heap limit.
fn runtime() -> JsRuntime {
    let mut js_rt = JsRuntime::new(RuntimeOptions {
        module_loader: Some(Rc::new(RweModuleLoader)),
        extensions: vec![rwe_ops::init()],
        ..Default::default()
    });
    js_rt.execute_script("<zeb_ssr_init>", FastString::from_static(ZEB_SSR_INIT)).expect("zeb_ssr_init");
    js_rt.execute_script("<tool_init>", FastString::from_static(TOOL_INIT)).expect("tool_init");
    js_rt
}

/// Render `i`'s ctx: a title, and `filler` bytes of text only it has.
fn ctx(i: usize, filler: usize) -> Value {
    json!({ "title": format!("page {i}"), "notes": format!("{i:08}").repeat(filler / 8) })
}

/// Heap bytes each render kept: the least heap seen over the last quarter of
/// the renders against the least seen over the first. The least of a stretch
/// is the heap just after a collection, as `pool_tests.rs` reads it; no
/// collection is forced, so what the compilation cache holds is counted.
async fn kept_per_render(filler: usize) -> f64 {
    let mut js_rt = runtime();
    let mut samples = Vec::new();
    for i in 0..RENDERS {
        let JsResponse::Rendered { html, .. } = do_render_ssr(&mut js_rt, PAGE, &ctx(i, filler)).await.expect("render");
        assert!(html.contains(&format!("page {i}")), "{html}");
        if i % SAMPLE_EVERY == SAMPLE_EVERY - 1 {
            samples.push(used_heap(&mut js_rt));
        }
    }
    let quarter = samples.len() / 4;
    let early = samples[..quarter].iter().min().copied().unwrap_or(0);
    let late = samples[samples.len() - quarter..].iter().min().copied().unwrap_or(0);
    late.saturating_sub(early) as f64 / ((samples.len() - quarter) * SAMPLE_EVERY) as f64
}

#[test]
fn a_render_keeps_nothing_of_its_ctx() {
    let tokio_rt = tokio::runtime::Builder::new_current_thread().enable_all().build().expect("tokio");
    tokio_rt.block_on(async {
        let small = kept_per_render(0).await;
        let large = kept_per_render(LARGE_CTX_BYTES).await;
        eprintln!("bytes kept per render: {small:.0} with a small ctx, {large:.0} with {LARGE_CTX_BYTES} bytes of ctx");
        assert!(
            large - small < (LARGE_CTX_BYTES / 8) as f64,
            "a render kept {:.0} more bytes with {LARGE_CTX_BYTES} bytes of ctx ({small:.0} against {large:.0})",
            large - small
        );
    });
}
