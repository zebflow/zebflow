//! Dedicated health server for process liveness.
//!
//! The main platform router can be busy with pipelines, DB writes, map rendering,
//! or template work. Kubernetes liveness should not share that same router path:
//! if the main runtime is overloaded, readiness can fail and remove traffic, but
//! liveness should only kill the process when this minimal health listener itself
//! cannot answer.

use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::Router;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use serde_json::json;

use crate::version::APP_VERSION;

const DEFAULT_RUNTIME_STALE_AFTER_MS: i64 = 15_000;

/// Minimal process health state shared with the dedicated health listener.
#[derive(Debug)]
pub struct HealthState {
    started_at_ms: i64,
    last_main_runtime_tick_ms: AtomicI64,
    shutdown_requested: AtomicBool,
}

impl HealthState {
    /// Create health state and mark the main runtime as alive at startup.
    pub fn new() -> Arc<Self> {
        let now = now_ms();
        Arc::new(Self {
            started_at_ms: now,
            last_main_runtime_tick_ms: AtomicI64::new(now),
            shutdown_requested: AtomicBool::new(false),
        })
    }

    /// Record that the main runtime can still schedule lightweight tasks.
    pub fn mark_main_runtime_tick(&self) {
        self.last_main_runtime_tick_ms
            .store(now_ms(), Ordering::Relaxed);
    }

    /// Mark process shutdown intent for diagnostic responses.
    pub fn mark_shutdown_requested(&self) {
        self.shutdown_requested.store(true, Ordering::Relaxed);
    }

    fn started_at_ms(&self) -> i64 {
        self.started_at_ms
    }

    fn last_main_runtime_tick_ms(&self) -> i64 {
        self.last_main_runtime_tick_ms.load(Ordering::Relaxed)
    }

    fn shutdown_requested(&self) -> bool {
        self.shutdown_requested.load(Ordering::Relaxed)
    }
}

/// Spawn a tiny heartbeat task on the main runtime.
///
/// `/health/runtime` uses this to distinguish "dedicated health listener is
/// alive" from "the main Tokio runtime is scheduling tasks normally".
pub fn spawn_main_runtime_heartbeat(state: Arc<HealthState>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;
            state.mark_main_runtime_tick();
        }
    });
}

/// Start the dedicated health server on its own OS thread and Tokio runtime.
///
/// Dropping the returned join handle detaches the thread; process exit will
/// terminate it. The server intentionally has no platform, DB, template, or
/// pipeline dependencies.
pub fn start_dedicated_health_server(
    addr: SocketAddr,
    state: Arc<HealthState>,
) -> io::Result<thread::JoinHandle<()>> {
    let listener = std::net::TcpListener::bind(addr)?;
    listener.set_nonblocking(true)?;

    let handle = thread::Builder::new()
        .name("zebflow-health".to_string())
        .spawn(move || {
            let runtime = match tokio::runtime::Builder::new_current_thread()
                .enable_io()
                .enable_time()
                .build()
            {
                Ok(runtime) => runtime,
                Err(err) => {
                    eprintln!("Zebflow health server failed creating runtime: {err}");
                    return;
                }
            };

            runtime.block_on(async move {
                let listener = match tokio::net::TcpListener::from_std(listener) {
                    Ok(listener) => listener,
                    Err(err) => {
                        eprintln!("Zebflow health server failed adopting listener: {err}");
                        return;
                    }
                };
                let app = health_router(state);
                if let Err(err) = axum::serve(listener, app).await {
                    eprintln!("Zebflow health server stopped: {err}");
                }
            });
        })?;

    Ok(handle)
}

fn health_router(state: Arc<HealthState>) -> Router {
    Router::new()
        .route("/health", get(live_handler))
        .route("/health/live", get(live_handler))
        .route("/health/runtime", get(runtime_handler))
        .with_state(state)
}

async fn live_handler(State(state): State<Arc<HealthState>>) -> impl IntoResponse {
    (
        StatusCode::OK,
        axum::Json(json!({
            "status": "ok",
            "kind": "live",
            "version": APP_VERSION,
            "started_at_ms": state.started_at_ms(),
            "now_ms": now_ms(),
            "shutdown_requested": state.shutdown_requested()
        })),
    )
}

async fn runtime_handler(State(state): State<Arc<HealthState>>) -> impl IntoResponse {
    let now = now_ms();
    let last_tick = state.last_main_runtime_tick_ms();
    let lag_ms = now.saturating_sub(last_tick);
    let is_ready = lag_ms <= DEFAULT_RUNTIME_STALE_AFTER_MS;
    let status = if is_ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (
        status,
        axum::Json(json!({
            "status": if is_ready { "ok" } else { "stale" },
            "kind": "runtime",
            "version": APP_VERSION,
            "last_main_runtime_tick_ms": last_tick,
            "main_runtime_lag_ms": lag_ms,
            "stale_after_ms": DEFAULT_RUNTIME_STALE_AFTER_MS,
            "shutdown_requested": state.shutdown_requested()
        })),
    )
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(i64::MAX as u128) as i64)
        .unwrap_or(0)
}
