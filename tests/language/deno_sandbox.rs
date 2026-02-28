use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::json;
use zebflow::language::{DenoSandboxConfigPatch, DenoSandboxDangerZonePatch, DenoSandboxEngine};

#[test]
fn deno_sandbox_blocks_dynamic_code_by_default() {
    let engine = DenoSandboxEngine::default();
    let source = "return eval('1 + 1');";
    let err = engine
        .compile_script(source, None)
        .expect_err("compile should reject eval by default");
    assert_eq!(err.code, "LANG_DENO_POLICY");
}

#[test]
fn deno_sandbox_can_allow_dynamic_code_via_patch() {
    let engine = DenoSandboxEngine::default();
    let source = "return eval('1 + 1');";
    let patch = DenoSandboxConfigPatch {
        danger_zone: Some(DenoSandboxDangerZonePatch {
            allow_dynamic_code: Some(true),
            ..Default::default()
        }),
        ..Default::default()
    };
    let compiled = engine
        .compile_script(source, Some(&patch))
        .expect("compile should pass when allow_dynamic_code=true");
    assert!(compiled.module_source.contains("eval"));
}

#[test]
fn deno_sandbox_clamps_limits_in_strict_mode() {
    let engine = DenoSandboxEngine::default();
    let patch = DenoSandboxConfigPatch {
        max_ops: Some(999_999_999),
        timeout_ms: Some(999_999),
        ..Default::default()
    };
    let compiled = engine
        .compile_script("return 1;", Some(&patch))
        .expect("compile should succeed");
    assert_eq!(compiled.resolved_config.max_ops, 50_000_000);
    assert_eq!(compiled.resolved_config.timeout_ms, 60_000);
}

#[test]
fn deno_sandbox_runtime_supports_local_fetch_with_root() {
    if !deno_available() {
        eprintln!("skipping: deno executable not found");
        return;
    }

    let dir = make_temp_dir("zebflow_deno_fetch");
    let file_path = dir.join("payload.json");
    fs::write(&file_path, br#"{"value": 42}"#).expect("write payload file");

    let engine = DenoSandboxEngine::default();
    let source = r#"
const data = await fetch("/payload.json").then((r) => r.json());
return { value: data.value, hasTime: n.time.now() > 0 };
"#;
    let patch = DenoSandboxConfigPatch {
        local_fetch_root: Some(dir.display().to_string()),
        ..Default::default()
    };

    let out = engine
        .run_script(source, &json!({}), Some(&patch))
        .expect("run_script should allow local fetch under local_fetch_root");

    assert_eq!(out.get("value").and_then(|v| v.as_i64()), Some(42));
    assert_eq!(out.get("hasTime").and_then(|v| v.as_bool()), Some(true));

    let _ = fs::remove_file(file_path);
    let _ = fs::remove_dir(dir);
}

#[test]
fn deno_sandbox_runtime_denies_external_fetch_without_allow_list() {
    if !deno_available() {
        eprintln!("skipping: deno executable not found");
        return;
    }

    let engine = DenoSandboxEngine::default();
    let source = r#"
await fetch("https://example.com/");
return { ok: true };
"#;
    let err = engine
        .run_script(source, &json!({}), None)
        .expect_err("run_script should deny external fetch by default");

    assert_eq!(err.code, "LANG_DENO_RUN");
    assert!(
        err.message.contains("external fetch denied"),
        "unexpected error: {}",
        err.message
    );
}

#[test]
fn deno_sandbox_runtime_runs_proper_script() {
    if !deno_available() {
        eprintln!("skipping: deno executable not found");
        return;
    }

    let engine = DenoSandboxEngine::default();
    let source = r#"
let sum = 0;
for (let i = 0; i < 10; i++) {
  sum += i;
}
return {
  sum,
  mul: n.math.imul(6, 7),
  nowOk: n.time.now() > 0
};
"#;

    let out = engine
        .run_script(source, &json!({}), None)
        .expect("proper script should execute successfully");

    assert_eq!(out.get("sum").and_then(|v| v.as_i64()), Some(45));
    assert_eq!(out.get("mul").and_then(|v| v.as_i64()), Some(42));
    assert_eq!(out.get("nowOk").and_then(|v| v.as_bool()), Some(true));
}

#[test]
fn deno_sandbox_runtime_blocks_indirect_eval_access() {
    if !deno_available() {
        eprintln!("skipping: deno executable not found");
        return;
    }

    let engine = DenoSandboxEngine::default();
    let source = r#"
const dynEval = globalThis["eval"];
return dynEval("1 + 1");
"#;

    let err = engine
        .run_script(source, &json!({}), None)
        .expect_err("indirect eval path must still be blocked");

    assert!(
        err.code == "LANG_DENO_POLICY" || err.code == "LANG_DENO_RUN",
        "unexpected error code: {} ({})",
        err.code,
        err.message
    );
    assert!(
        err.message.contains("eval")
            || err.message.contains("dynamic code is disabled")
            || err.message.contains("policy violation"),
        "unexpected error: {}",
        err.message
    );
}

fn deno_available() -> bool {
    Command::new("deno").arg("--version").output().is_ok()
}

fn make_temp_dir(prefix: &str) -> PathBuf {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("{prefix}_{nonce}"));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}
