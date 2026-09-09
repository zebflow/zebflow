use serde_json::json;
use zebflow::language::{DenoSandboxConfigPatch, DenoSandboxEngine};

/// Does the watchdog actually cover a block inside a Rust op?
///
/// `terminate_execution()` interrupts JavaScript. `op_read_local_file` is a
/// synchronous Rust op calling `read_to_string`, and reading a FIFO with no
/// writer blocks in the kernel — where V8 has no say. If that is true, the
/// watchdog frees the *caller* while the worker thread stays gone, which is
/// exactly the wedge the watchdog was supposed to end.
///
/// Bounded: every call returns once the caller's own recv timeout expires.
#[test]
fn a_blocking_op_does_not_wedge_the_pool() {
    let dir = std::env::temp_dir().join(format!(
        "zf_fifo_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let fifo = dir.join("hang.fifo");
    let made = std::process::Command::new("mkfifo")
        .arg(&fifo)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !made {
        println!("  mkfifo unavailable — skipping");
        return;
    }

    let engine = DenoSandboxEngine::default();
    let patch = DenoSandboxConfigPatch {
        local_fetch_root: Some(dir.display().to_string()),
        timeout_ms: Some(150),
        ..Default::default()
    };
    let src = "const r = await fetch('/hang.fifo'); return { len: (await r.text()).length };";

    // Try to wedge every worker.
    for i in 0..8 {
        let started = std::time::Instant::now();
        let out = engine.run_script(src, &json!({}), Some(&patch));
        println!("  fifo read {i}: {:?} after {:?}", out.is_ok(), started.elapsed());
    }

    // Can the pool still serve ordinary work?
    let mut healthy = 0;
    for _ in 0..8 {
        if engine
            .run_script("return { ok: 1 };", &json!({}), None)
            .is_ok()
        {
            healthy += 1;
        }
    }
    println!("  workers still serving afterwards: {healthy}/8");

    let _ = std::fs::remove_file(&fifo);
    let _ = std::fs::remove_dir(&dir);

    assert_eq!(
        healthy, 8,
        "POOL WEDGED — only {healthy}/8 workers survived a blocking op; \
         terminate_execution() cannot interrupt a blocked syscall in Rust"
    );
}
