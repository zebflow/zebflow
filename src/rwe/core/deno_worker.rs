use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{LazyLock, Mutex};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::error::EngineError;

static REQUEST_ID: AtomicU64 = AtomicU64::new(1);
static WORKER: LazyLock<Mutex<DenoWorker>> = LazyLock::new(|| Mutex::new(DenoWorker::spawn().expect("spawn rwe deno worker")));

#[derive(Debug, Serialize)]
struct WorkerRequest {
    id: u64,
    op: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    module_source: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    ctx: Option<Value>,
    timeout_ms: u64,
}

#[derive(Debug, Deserialize)]
struct WorkerResponse {
    id: u64,
    ok: bool,
    html: Option<String>,
    js: Option<String>,
    error: Option<String>,
}

struct DenoWorker {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl DenoWorker {
    fn spawn() -> Result<Self, EngineError> {
        let worker_script =
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/rwe/runtime/ssr_worker.mjs");
        let current_dir = std::env::current_dir().map_err(|e| {
            EngineError::new(
                "RWE_DENO_SPAWN",
                format!("failed reading current dir for deno allow-read: {e}"),
            )
        })?;
        let allow_read = format!(
            "/tmp,/private/tmp,/var/folders,/private/var/folders,{}",
            current_dir.display()
        );
        let allow_write = "/tmp,/private/tmp,/var/folders,/private/var/folders";
        let mut cmd = Command::new("deno");
        cmd.arg("run")
            .arg("--quiet")
            .arg("--no-prompt")
            .arg("--allow-env=DENO_DIR,DENO_AUTH_TOKENS")
            .arg(format!("--allow-read={allow_read}"))
            .arg(format!("--allow-write={allow_write}"))
            .arg("--allow-net=registry.npmjs.org,jsr.io,esm.sh")
            .arg(worker_script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        cmd.env("DENO_DIR", "/tmp/rwe-deno");

        let mut child = cmd.spawn().map_err(|e| {
            EngineError::new(
                "RWE_DENO_SPAWN",
                format!("failed spawning deno worker: {e}"),
            )
        })?;

        let stdin = child.stdin.take().ok_or_else(|| {
            EngineError::new("RWE_DENO_STDIN", "missing deno worker stdin")
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            EngineError::new("RWE_DENO_STDOUT", "missing deno worker stdout")
        })?;

        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        })
    }

    fn ensure_running(&mut self) -> Result<(), EngineError> {
        if self.child.try_wait().map_err(|e| {
            EngineError::new(
                "RWE_DENO_WAIT",
                format!("failed checking deno worker state: {e}"),
            )
        })?.is_none() {
            return Ok(());
        }

        *self = DenoWorker::spawn()?;
        Ok(())
    }

    fn request(&mut self, req: &WorkerRequest) -> Result<WorkerResponse, EngineError> {
        self.ensure_running()?;

        let line = serde_json::to_string(req).map_err(|e| {
            EngineError::new(
                "RWE_DENO_PROTOCOL",
                format!("failed encoding deno request: {e}"),
            )
        })?;
        self.stdin.write_all(line.as_bytes()).map_err(|e| {
            EngineError::new(
                "RWE_DENO_STDIN",
                format!("failed writing deno worker request: {e}"),
            )
        })?;
        self.stdin.write_all(b"\n").map_err(|e| {
            EngineError::new(
                "RWE_DENO_STDIN",
                format!("failed writing deno worker request newline: {e}"),
            )
        })?;
        self.stdin.flush().map_err(|e| {
            EngineError::new(
                "RWE_DENO_STDIN",
                format!("failed flushing deno worker stdin: {e}"),
            )
        })?;

        let mut out = String::new();
        let read = self.stdout.read_line(&mut out).map_err(|e| {
            EngineError::new(
                "RWE_DENO_STDOUT",
                format!("failed reading deno worker response: {e}"),
            )
        })?;
        if read == 0 {
            return Err(EngineError::new(
                "RWE_DENO_EOF",
                "deno worker closed stdout unexpectedly",
            ));
        }

        serde_json::from_str(out.trim()).map_err(|e| {
            EngineError::new(
                "RWE_DENO_PROTOCOL",
                format!("invalid deno worker response JSON: {e}"),
            )
        })
    }
}

pub fn render_ssr(module_source: &str, ctx: &Value, timeout_ms: u64) -> Result<String, EngineError> {
    let req = WorkerRequest {
        id: REQUEST_ID.fetch_add(1, Ordering::Relaxed),
        op: "render_ssr",
        module_source: Some(module_source.to_string()),
        ctx: Some(ctx.clone()),
        timeout_ms,
    };

    let mut worker = WORKER.lock().map_err(|_| {
        EngineError::new("RWE_DENO_LOCK", "failed locking deno worker mutex")
    })?;

    let mut response = worker.request(&req).or_else(|_| {
        // attempt one restart + retry when protocol fails
        *worker = DenoWorker::spawn()?;
        worker.request(&req)
    })?;

    if response.id != req.id {
        return Err(EngineError::new(
            "RWE_DENO_PROTOCOL",
            format!("mismatched response id {} for request {}", response.id, req.id),
        ));
    }

    if !response.ok {
        return Err(EngineError::new(
            "RWE_DENO_RENDER",
            response
                .error
                .take()
                .unwrap_or_else(|| "deno worker render failed".to_string()),
        ));
    }

    Ok(response.html.take().unwrap_or_default())
}

pub fn transpile_client(module_source: &str, timeout_ms: u64) -> Result<String, EngineError> {
    let req = WorkerRequest {
        id: REQUEST_ID.fetch_add(1, Ordering::Relaxed),
        op: "transpile_client",
        module_source: Some(module_source.to_string()),
        ctx: None,
        timeout_ms,
    };

    let mut worker = WORKER.lock().map_err(|_| {
        EngineError::new("RWE_DENO_LOCK", "failed locking deno worker mutex")
    })?;

    let mut response = worker.request(&req).or_else(|_| {
        // attempt one restart + retry when protocol fails
        *worker = DenoWorker::spawn()?;
        worker.request(&req)
    })?;

    if response.id != req.id {
        return Err(EngineError::new(
            "RWE_DENO_PROTOCOL",
            format!("mismatched response id {} for request {}", response.id, req.id),
        ));
    }

    if !response.ok {
        return Err(EngineError::new(
            "RWE_DENO_TRANSPILE",
            response
                .error
                .take()
                .unwrap_or_else(|| "deno worker transpile failed".to_string()),
        ));
    }

    Ok(response.js.take().unwrap_or_default())
}
