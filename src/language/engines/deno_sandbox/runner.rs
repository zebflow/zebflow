//! Subprocess runner boundary for Deno sandbox execution.
//!
//! This module is intentionally process-based:
//!
//! - script execution happens in an isolated Deno process
//! - host can enforce hard kill timeout
//! - temporary files are cleaned after each run

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use serde_json::Value;

use super::config::DenoSandboxConfig;
use super::engine::CompiledDenoSandboxScript;

/// Executes a compiled sandbox script using the configured Deno runner.
pub(crate) fn run_compiled_script(
    runner_path: &Path,
    compiled: &CompiledDenoSandboxScript,
    input: &Value,
) -> Result<Value, String> {
    let tmp_dir = std::env::temp_dir();
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::from_millis(0))
        .as_millis();

    let script_path = tmp_dir.join(format!("{}_{}.mjs", compiled.script_id, nonce));
    let cfg_path = tmp_dir.join(format!("{}_{}.config.json", compiled.script_id, nonce));
    let input_path = tmp_dir.join(format!("{}_{}.input.json", compiled.script_id, nonce));

    fs::write(&script_path, compiled.module_source.as_bytes())
        .map_err(|e| format!("DenoSandboxError: write script failed: {e}"))?;
    let cfg_json = serde_json::to_vec(&compiled.resolved_config)
        .map_err(|e| format!("DenoSandboxError: encode config failed: {e}"))?;
    fs::write(&cfg_path, cfg_json)
        .map_err(|e| format!("DenoSandboxError: write config failed: {e}"))?;
    let input_json = serde_json::to_vec(input)
        .map_err(|e| format!("DenoSandboxError: encode input failed: {e}"))?;
    fs::write(&input_path, input_json)
        .map_err(|e| format!("DenoSandboxError: write input failed: {e}"))?;

    let run_out = run_deno_runner(
        runner_path,
        &compiled.resolved_config,
        &script_path,
        &cfg_path,
        &input_path,
    );

    let _ = fs::remove_file(&script_path);
    let _ = fs::remove_file(&cfg_path);
    let _ = fs::remove_file(&input_path);

    run_out
}

fn run_deno_runner(
    runner_path: &Path,
    cfg: &DenoSandboxConfig,
    script_path: &Path,
    cfg_path: &Path,
    input_path: &Path,
) -> Result<Value, String> {
    let mut args: Vec<String> = vec!["run".into(), "--quiet".into(), "--no-prompt".into()];

    if !cfg.danger_zone.allow_dynamic_code {
        args.push("--v8-flags=--disallow-code-generation-from-strings".into());
    }

    if !cfg.allow_list.external_fetch_hosts.is_empty() {
        args.push(format!(
            "--allow-net={}",
            cfg.allow_list.external_fetch_hosts.join(",")
        ));
    } else if cfg.danger_zone.allow_net {
        if cfg.danger_zone.allow_net_hosts.is_empty() {
            args.push("--allow-net".into());
        } else {
            args.push(format!(
                "--allow-net={}",
                cfg.danger_zone.allow_net_hosts.join(",")
            ));
        }
    }

    let mut allow_read = vec![
        display_path(runner_path),
        display_path(script_path),
        display_path(cfg_path),
        display_path(input_path),
    ];
    if !cfg.local_fetch_root.trim().is_empty() {
        allow_read.push(cfg.local_fetch_root.clone());
    }
    allow_read.extend(cfg.danger_zone.allow_read_paths.clone());
    args.push(format!("--allow-read={}", allow_read.join(",")));

    if !cfg.danger_zone.allow_write_paths.is_empty() {
        args.push(format!(
            "--allow-write={}",
            cfg.danger_zone.allow_write_paths.join(",")
        ));
    }

    if !cfg.danger_zone.allow_env_keys.is_empty() {
        args.push(format!(
            "--allow-env={}",
            cfg.danger_zone.allow_env_keys.join(",")
        ));
    }

    if cfg.danger_zone.allow_run {
        args.push("--allow-run".into());
    }

    args.push(display_path(runner_path));
    args.push(display_path(script_path));
    args.push(display_path(cfg_path));
    args.push(display_path(input_path));

    let mut cmd = Command::new("deno");
    cmd.args(args).stdout(Stdio::piped()).stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("DenoSandboxError: failed to start deno: {e}"))?;

    let kill_after = Duration::from_millis(cfg.host_kill_timeout_ms.max(cfg.timeout_ms + 5));
    let started = Instant::now();
    let mut timed_out = false;

    loop {
        if child
            .try_wait()
            .map_err(|e| format!("DenoSandboxError: process wait failed: {e}"))?
            .is_some()
        {
            break;
        }
        if started.elapsed() > kill_after {
            timed_out = true;
            let _ = child.kill();
            break;
        }
        thread::sleep(Duration::from_millis(2));
    }

    let out = child
        .wait_with_output()
        .map_err(|e| format!("DenoSandboxError: process output failed: {e}"))?;

    if timed_out {
        return Err("DenoSandboxError: host timeout kill triggered".into());
    }

    if out.stdout.len() > cfg.max_output_bytes {
        return Err(format!(
            "DenoSandboxError: output too large ({} > {} bytes)",
            out.stdout.len(),
            cfg.max_output_bytes
        ));
    }

    let stderr = String::from_utf8_lossy(&out.stderr);
    if !stderr.trim().is_empty() {
        return Err(format!("DenoSandboxError: deno stderr: {}", stderr.trim()));
    }

    let stdout = String::from_utf8_lossy(&out.stdout);
    let line = stdout
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .ok_or_else(|| "DenoSandboxError: missing runner output".to_string())?;

    let parsed: Value = serde_json::from_str(line)
        .map_err(|e| format!("DenoSandboxError: invalid runner json: {e}"))?;

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

fn display_path(path: &Path) -> String {
    PathBuf::from(path).display().to_string()
}
