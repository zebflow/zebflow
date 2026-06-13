//! Background service driving `n.trigger.ws.client` pipelines.
//!
//! When a pipeline with `n.trigger.ws.client` is activated, a dedicated
//! tokio task is spawned that connects to the external WS server and fires
//! the pipeline for every received message.  On deactivate the task is aborted.
//! Auto-reconnects with capped exponential backoff.

use std::collections::HashMap;
use std::sync::Arc;

use futures::stream::StreamExt;
use serde_json::json;
use tokio::sync::{Mutex, mpsc};
use tokio::task::JoinHandle;

use crate::pipeline::engines::BasicPipelineEngine;
use crate::pipeline::interface::PipelineEngine;
use crate::pipeline::model::PipelineContext;
use crate::platform::adapters::data::DataAdapter;
use crate::platform::model::PipelineInvocationEntry;
use crate::platform::services::pipeline_hits::PipelineHitsService;
use crate::platform::services::pipeline_runtime::PipelineRuntimeService;
use crate::platform::services::project_config::ZebflowJsonService;

/// Background task registry for WS client triggered pipelines.
pub struct WsClientManager {
    tasks: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    senders: Arc<Mutex<HashMap<String, mpsc::Sender<String>>>>,
    runtime: Arc<PipelineRuntimeService>,
    engine: Arc<BasicPipelineEngine>,
    hits: Arc<PipelineHitsService>,
    data: Arc<dyn DataAdapter>,
    zebflow_cfg: Arc<ZebflowJsonService>,
}

impl WsClientManager {
    pub fn new(
        runtime: Arc<PipelineRuntimeService>,
        engine: Arc<BasicPipelineEngine>,
        hits: Arc<PipelineHitsService>,
        data: Arc<dyn DataAdapter>,
        zebflow_cfg: Arc<ZebflowJsonService>,
    ) -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            senders: Arc::new(Mutex::new(HashMap::new())),
            runtime,
            engine,
            hits,
            data,
            zebflow_cfg,
        }
    }

    /// Register connection tasks for all currently active pipelines.
    pub async fn register_all(&self) {
        for pipeline in self.runtime.list_all() {
            for trigger in &pipeline.ws_client_triggers {
                self.register_task(
                    &pipeline.owner,
                    &pipeline.project,
                    &pipeline.file_rel_path,
                    &pipeline.graph.id,
                    &trigger.node_id,
                    &trigger.url,
                    &trigger.credential_id,
                    trigger.reconnect,
                    trigger.reconnect_delay_ms,
                    trigger.max_reconnect_attempts,
                    trigger.heartbeat_interval_ms,
                    &trigger.message_format,
                )
                .await;
            }
        }
    }

    /// Sync WS client connections for a single pipeline (called on activate / deactivate).
    pub async fn sync_pipeline(&self, owner: &str, project: &str, file_rel_path: &str) {
        let key_prefix = format!("{}/{}/{}", owner, project, file_rel_path);

        // Abort stale connection tasks for this pipeline.
        let stale: Vec<String> = {
            let tasks = self.tasks.lock().await;
            tasks
                .keys()
                .filter(|k| k.starts_with(&key_prefix))
                .cloned()
                .collect()
        };
        for key in &stale {
            if let Some(handle) = self.tasks.lock().await.remove(key) {
                handle.abort();
            }
            self.senders.lock().await.remove(key);
        }

        // Spawn fresh tasks if the pipeline is still active.
        if let Some(pipeline) = self.runtime.get(owner, project, file_rel_path) {
            for trigger in &pipeline.ws_client_triggers {
                self.register_task(
                    owner,
                    project,
                    file_rel_path,
                    &pipeline.graph.id,
                    &trigger.node_id,
                    &trigger.url,
                    &trigger.credential_id,
                    trigger.reconnect,
                    trigger.reconnect_delay_ms,
                    trigger.max_reconnect_attempts,
                    trigger.heartbeat_interval_ms,
                    &trigger.message_format,
                )
                .await;
            }
        }
    }

    /// Send a message through an active WS client connection.
    pub async fn send(&self, connection_key: &str, message: String) -> Result<(), String> {
        let sender = {
            let senders = self.senders.lock().await;
            senders.get(connection_key).cloned()
        };
        match sender {
            Some(tx) => tx
                .send(message)
                .await
                .map_err(|_| format!("WS client connection '{}' is closed", connection_key)),
            None => Err(format!(
                "no active WS client connection for '{}'",
                connection_key
            )),
        }
    }

    #[allow(clippy::too_many_arguments)]
    async fn register_task(
        &self,
        owner: &str,
        project: &str,
        file_rel_path: &str,
        graph_id: &str,
        node_id: &str,
        url: &str,
        credential_id: &str,
        reconnect: bool,
        reconnect_delay_ms: u64,
        max_reconnect_attempts: u64,
        heartbeat_interval_ms: u64,
        message_format: &str,
    ) {
        let task_key = format!("{}/{}/{}:{}", owner, project, file_rel_path, node_id);

        // Create send channel for n.ws.client.send to communicate with this connection.
        let (send_tx, send_rx) = mpsc::channel::<String>(256);
        self.senders.lock().await.insert(task_key.clone(), send_tx);

        // Resolve credential headers once at registration time.
        let extra_headers = if !credential_id.is_empty() {
            resolve_credential_headers(&self.data, owner, project, credential_id)
        } else {
            Vec::new()
        };

        let runtime = self.runtime.clone();
        let engine = self.engine.clone();
        let hits = self.hits.clone();
        let data = self.data.clone();
        let zebflow_cfg = self.zebflow_cfg.clone();
        let owner_s = owner.to_string();
        let project_s = project.to_string();
        let file_rel_path_s = file_rel_path.to_string();
        let graph_id_s = graph_id.to_string();
        let node_id_s = node_id.to_string();
        let url_s = url.to_string();
        let message_format_s = message_format.to_string();

        let handle = tokio::spawn(async move {
            let send_rx = Arc::new(Mutex::new(send_rx));
            let mut attempt: u64 = 0;

            loop {
                println!(
                    "🔌 WsClient: connecting to {} (pipeline {}/{})",
                    url_s, owner_s, file_rel_path_s
                );

                // Build request with auth headers from credential.
                let ws_request = build_ws_request(&url_s, &extra_headers);
                match tokio_tungstenite::connect_async(ws_request).await {
                    Ok((ws_stream, _response)) => {
                        println!("✅ WsClient: connected to {}", url_s);
                        attempt = 0; // Reset on successful connect.

                        let (mut ws_write, mut ws_read) = ws_stream.split();
                        let mut send_rx = send_rx.lock().await;
                        let mut heartbeat = tokio::time::interval(
                            std::time::Duration::from_millis(heartbeat_interval_ms),
                        );
                        heartbeat.tick().await; // Consume the initial immediate tick.

                        loop {
                            tokio::select! {
                                msg = ws_read.next() => {
                                    match msg {
                                        Some(Ok(ws_msg)) => {
                                            use tokio_tungstenite::tungstenite::Message;
                                            let text = match ws_msg {
                                                Message::Text(t) => Some(t.to_string()),
                                                Message::Binary(b) => {
                                                    String::from_utf8(b.to_vec()).ok()
                                                }
                                                Message::Ping(_) | Message::Pong(_) => None,
                                                Message::Close(_) => {
                                                    println!("🔌 WsClient: server closed connection to {}", url_s);
                                                    break;
                                                }
                                                _ => None,
                                            };

                                            if let Some(text) = text {
                                                // Parse message according to format.
                                                let message_value = if message_format_s == "json" {
                                                    serde_json::from_str::<serde_json::Value>(&text)
                                                        .unwrap_or_else(|_| serde_json::Value::String(text))
                                                } else {
                                                    serde_json::Value::String(text)
                                                };

                                                // Check pipeline still active.
                                                let Some(compiled) = runtime.get(&owner_s, &project_s, &file_rel_path_s) else {
                                                    eprintln!(
                                                        "WsClient: pipeline no longer active — stopping {}",
                                                        file_rel_path_s
                                                    );
                                                    return;
                                                };

                                                let project_cfg = zebflow_cfg.read_or_default(&owner_s, &project_s);
                                                let pipeline_retention =
                                                    compiled.graph.metadata.as_ref().and_then(|metadata| {
                                                        metadata.settings.invocation_retention.as_ref()
                                                    });
                                                let log_max_n = pipeline_retention
                                                    .and_then(|retention| retention.max_invocations)
                                                    .map(|value| value.max(1) as usize)
                                                    .unwrap_or_else(|| {
                                                        project_cfg
                                                            .configs
                                                            .pipelines
                                                            .logging
                                                            .effective_max_invocations()
                                                    });
                                                let max_age_secs = pipeline_retention
                                                    .and_then(|retention| retention.max_age_secs)
                                                    .map(|value| value.max(1) as i64);

                                                let fired_at = chrono::Utc::now();
                                                let ctx = PipelineContext {
                                                    owner: owner_s.clone(),
                                                    project: project_s.clone(),
                                                    pipeline: graph_id_s.clone(),
                                                    request_id: format!(
                                                        "wsc-{}",
                                                        std::time::SystemTime::now()
                                                            .duration_since(std::time::UNIX_EPOCH)
                                                            .unwrap_or_default()
                                                            .as_millis()
                                                    ),
                                                    route: String::new(),
                                                    input: json!({
                                                        "trigger": "ws_client",
                                                        "url": url_s,
                                                        "node_id": node_id_s,
                                                        "message": message_value,
                                                    }),
                                                    trigger: None,
                                                    placeholder: None,
                                                };

                                                let exec_start = std::time::Instant::now();
                                                match engine.execute_async(&compiled.graph, &ctx).await {
                                                    Ok(output) => {
                                                        let duration_ms = exec_start.elapsed().as_millis() as u64;
                                                        hits.record_success(&owner_s, &project_s, &file_rel_path_s);
                                                        let _ = data.log_pipeline_invocation(
                                                            &owner_s,
                                                            &project_s,
                                                            &file_rel_path_s,
                                                            &PipelineInvocationEntry {
                                                                run_id: ctx.request_id.clone(),
                                                                at: fired_at.timestamp(),
                                                                duration_ms,
                                                                status: "ok".to_string(),
                                                                trigger: "ws_client".to_string(),
                                                                error: None,
                                                                trace: output.node_trace,
                                                            },
                                                            log_max_n,
                                                            max_age_secs,
                                                        );
                                                    }
                                                    Err(e) => {
                                                        let duration_ms = exec_start.elapsed().as_millis() as u64;
                                                        hits.record_failure(
                                                            &owner_s,
                                                            &project_s,
                                                            &file_rel_path_s,
                                                            "ws_client",
                                                            e.code,
                                                            &e.message,
                                                        );
                                                        let _ = data.log_pipeline_invocation(
                                                            &owner_s,
                                                            &project_s,
                                                            &file_rel_path_s,
                                                            &PipelineInvocationEntry {
                                                                run_id: ctx.request_id.clone(),
                                                                at: fired_at.timestamp(),
                                                                duration_ms,
                                                                status: "error".to_string(),
                                                                trigger: "ws_client".to_string(),
                                                                error: Some(e.message.clone()),
                                                                trace: e.node_trace.clone(),
                                                            },
                                                            log_max_n,
                                                            max_age_secs,
                                                        );
                                                        eprintln!(
                                                            "❌ WsClient pipeline failed {}/{}/{}: {}",
                                                            owner_s, project_s, file_rel_path_s, e
                                                        );
                                                    }
                                                }
                                            }
                                        }
                                        Some(Err(e)) => {
                                            eprintln!("❌ WsClient: read error from {}: {}", url_s, e);
                                            break;
                                        }
                                        None => {
                                            println!("🔌 WsClient: stream ended for {}", url_s);
                                            break;
                                        }
                                    }
                                }
                                Some(outgoing) = send_rx.recv() => {
                                    use tokio_tungstenite::tungstenite::Message;
                                    use futures::SinkExt;
                                    if let Err(e) = ws_write.send(Message::Text(outgoing.into())).await {
                                        eprintln!("❌ WsClient: send error to {}: {}", url_s, e);
                                        break;
                                    }
                                }
                                _ = heartbeat.tick() => {
                                    use tokio_tungstenite::tungstenite::Message;
                                    use futures::SinkExt;
                                    if let Err(e) = ws_write.send(Message::Ping(vec![].into())).await {
                                        eprintln!("❌ WsClient: ping error to {}: {}", url_s, e);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("❌ WsClient: failed connecting to {}: {}", url_s, e);
                    }
                }

                // Check if pipeline is still active before reconnecting.
                if runtime
                    .get(&owner_s, &project_s, &file_rel_path_s)
                    .is_none()
                {
                    println!(
                        "🔌 WsClient: pipeline deactivated, stopping connection to {}",
                        url_s
                    );
                    return;
                }

                if !reconnect {
                    println!(
                        "🔌 WsClient: reconnect disabled, stopping connection to {}",
                        url_s
                    );
                    return;
                }

                attempt += 1;
                if max_reconnect_attempts > 0 && attempt > max_reconnect_attempts {
                    eprintln!(
                        "❌ WsClient: max reconnect attempts ({}) reached for {}",
                        max_reconnect_attempts, url_s
                    );
                    return;
                }

                // Exponential backoff: base * 2^(attempt-1), capped at 60s.
                let delay_ms = (reconnect_delay_ms * (1u64 << (attempt - 1).min(6))).min(60_000);
                println!(
                    "🔄 WsClient: reconnecting to {} in {}ms (attempt {})",
                    url_s, delay_ms, attempt
                );
                tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
            }
        });

        self.tasks.lock().await.insert(task_key, handle);
    }
}

// ---------------------------------------------------------------------------
// Credential → WS headers helpers
// ---------------------------------------------------------------------------

/// Resolve a credential into `(header_name, header_value)` pairs for the WS
/// upgrade request.  Supports credential kinds: `http` (Bearer token),
/// `api_key`, `custom` (looks for `headers` object), and any kind with a
/// `cookie` field.
fn resolve_credential_headers(
    data: &Arc<dyn DataAdapter>,
    owner: &str,
    project: &str,
    credential_id: &str,
) -> Vec<(String, String)> {
    let credential = match data.get_project_credential(owner, project, credential_id) {
        Ok(Some(c)) => c,
        Ok(None) => {
            eprintln!(
                "⚠ WsClient: credential '{}' not found in {}/{}",
                credential_id, owner, project
            );
            return Vec::new();
        }
        Err(e) => {
            eprintln!(
                "⚠ WsClient: error loading credential '{}': {}",
                credential_id, e
            );
            return Vec::new();
        }
    };

    let secret = &credential.secret;
    let mut headers: Vec<(String, String)> = Vec::new();

    match credential.kind.as_str() {
        "http" => {
            // Bearer token auth.
            if let Some(token) = secret.get("token").and_then(|v| v.as_str()) {
                if !token.is_empty() {
                    headers.push(("Authorization".into(), format!("Bearer {}", token)));
                }
            }
        }
        "api_key" => {
            if let Some(key) = secret.get("key").and_then(|v| v.as_str()) {
                if !key.is_empty() {
                    headers.push(("X-API-Key".into(), key.to_string()));
                }
            }
        }
        _ => {
            // Generic: look for explicit "headers" object in secret.
            if let Some(hdrs) = secret.get("headers").and_then(|v| v.as_object()) {
                for (k, v) in hdrs {
                    if let Some(val) = v.as_str() {
                        headers.push((k.clone(), val.to_string()));
                    }
                }
            }
            // Also support a top-level "token" for any kind.
            if let Some(token) = secret.get("token").and_then(|v| v.as_str()) {
                if !token.is_empty()
                    && !headers
                        .iter()
                        .any(|(k, _)| k.eq_ignore_ascii_case("authorization"))
                {
                    headers.push(("Authorization".into(), format!("Bearer {}", token)));
                }
            }
        }
    }

    // Any credential can carry a `cookie` field for session-based auth.
    if let Some(cookie) = secret.get("cookie").and_then(|v| v.as_str()) {
        if !cookie.is_empty() {
            headers.push(("Cookie".into(), cookie.to_string()));
        }
    }

    headers
}

/// Build a `tungstenite::http::Request` from a URL string plus extra headers.
/// Lets tungstenite generate the standard WS handshake headers (Host,
/// Connection, Upgrade, Sec-WebSocket-*) and only appends the extra ones.
fn build_ws_request(
    url: &str,
    extra_headers: &[(String, String)],
) -> tokio_tungstenite::tungstenite::http::Request<()> {
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;

    let mut request = url.into_client_request().expect("valid WS URL");

    let headers = request.headers_mut();
    for (name, value) in extra_headers {
        if let (Ok(hn), Ok(hv)) = (
            tokio_tungstenite::tungstenite::http::header::HeaderName::from_bytes(name.as_bytes()),
            tokio_tungstenite::tungstenite::http::header::HeaderValue::from_str(value),
        ) {
            headers.append(hn, hv);
        }
    }

    request
}
