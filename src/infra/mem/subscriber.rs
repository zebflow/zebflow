//! Background service driving `n.trigger.memsubscribe` pipelines.
//!
//! When a pipeline with `n.trigger.memsubscribe` is activated, a dedicated
//! tokio task is spawned that listens on the named channel and fires the
//! pipeline for every received message.  On deactivate the task is aborted.

use std::collections::HashMap;
use std::sync::Arc;

use serde_json::json;
use tokio::sync::Mutex;
use tokio::task::JoinHandle;

use crate::infra::mem::MemHub;
use crate::pipeline::engines::BasicPipelineEngine;
use crate::pipeline::interface::PipelineEngine;
use crate::pipeline::model::PipelineContext;
use crate::platform::adapters::data::DataAdapter;
use crate::platform::model::PipelineInvocationEntry;
use crate::platform::services::pipeline_hits::PipelineHitsService;
use crate::platform::services::pipeline_runtime::PipelineRuntimeService;
use crate::platform::services::project_config::ZebflowJsonService;

/// Background task registry for mem-subscribe triggered pipelines.
pub struct MemSubscriber {
    tasks: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    mem_hub: Arc<MemHub>,
    runtime: Arc<PipelineRuntimeService>,
    engine: Arc<BasicPipelineEngine>,
    hits: Arc<PipelineHitsService>,
    data: Arc<dyn DataAdapter>,
    zebflow_cfg: Arc<ZebflowJsonService>,
}

impl MemSubscriber {
    pub fn new(
        mem_hub: Arc<MemHub>,
        runtime: Arc<PipelineRuntimeService>,
        engine: Arc<BasicPipelineEngine>,
        hits: Arc<PipelineHitsService>,
        data: Arc<dyn DataAdapter>,
        zebflow_cfg: Arc<ZebflowJsonService>,
    ) -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            mem_hub,
            runtime,
            engine,
            hits,
            data,
            zebflow_cfg,
        }
    }

    /// Register listener tasks for all currently active pipelines.
    pub async fn register_all(&self) {
        for pipeline in self.runtime.list_all() {
            for trigger in &pipeline.mem_subscribe_triggers {
                self.register_task(
                    &pipeline.owner,
                    &pipeline.project,
                    &pipeline.file_rel_path,
                    &pipeline.graph.id,
                    &trigger.node_id,
                    &trigger.channel,
                )
                .await;
            }
        }
    }

    /// Sync mem subscriptions for a single pipeline (called on activate / deactivate).
    pub async fn sync_pipeline(&self, owner: &str, project: &str, file_rel_path: &str) {
        let key_prefix = format!("{}/{}/{}", owner, project, file_rel_path);

        // Abort stale listener tasks for this pipeline.
        let stale: Vec<String> = {
            let tasks = self.tasks.lock().await;
            tasks
                .keys()
                .filter(|k| k.starts_with(&key_prefix))
                .cloned()
                .collect()
        };
        for key in stale {
            if let Some(handle) = self.tasks.lock().await.remove(&key) {
                handle.abort();
            }
        }

        // Spawn fresh tasks if the pipeline is still active.
        if let Some(pipeline) = self.runtime.get(owner, project, file_rel_path) {
            for trigger in &pipeline.mem_subscribe_triggers {
                self.register_task(
                    owner,
                    project,
                    file_rel_path,
                    &pipeline.graph.id,
                    &trigger.node_id,
                    &trigger.channel,
                )
                .await;
            }
        }
    }

    async fn register_task(
        &self,
        owner: &str,
        project: &str,
        file_rel_path: &str,
        graph_id: &str,
        node_id: &str,
        channel: &str,
    ) {
        let task_key = format!("{}/{}/{}:{}", owner, project, file_rel_path, node_id);

        let mut rx = self.mem_hub.subscribe(owner, project, channel);

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
        let channel_s = channel.to_string();

        let handle = tokio::spawn(async move {
            loop {
                match rx.recv().await {
                    Ok(message) => {
                        let Some(compiled) = runtime.get(&owner_s, &project_s, &file_rel_path_s)
                        else {
                            eprintln!(
                                "MemSubscriber: pipeline no longer active — stopping listener {}",
                                file_rel_path_s
                            );
                            break;
                        };

                        let log_max_n = zebflow_cfg
                            .read_or_default(&owner_s, &project_s)
                            .logging
                            .effective_max_invocations();

                        let fired_at = chrono::Utc::now();
                        let ctx = PipelineContext {
                            owner: owner_s.clone(),
                            project: project_s.clone(),
                            pipeline: graph_id_s.clone(),
                            request_id: format!(
                                "memsub-{}",
                                std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis()
                            ),
                            route: String::new(),
                            input: json!({
                                "trigger": "memsubscribe",
                                "channel": channel_s,
                                "node_id": node_id_s,
                                "message": message,
                            }),
                            trigger: None,
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
                                        at: fired_at.timestamp(),
                                        duration_ms,
                                        status: "ok".to_string(),
                                        trigger: "memsubscribe".to_string(),
                                        error: None,
                                        trace: output.node_trace,
                                    },
                                    log_max_n,
                                );
                            }
                            Err(e) => {
                                let duration_ms = exec_start.elapsed().as_millis() as u64;
                                hits.record_failure(
                                    &owner_s,
                                    &project_s,
                                    &file_rel_path_s,
                                    "memsubscribe",
                                    e.code,
                                    &e.message,
                                );
                                let _ = data.log_pipeline_invocation(
                                    &owner_s,
                                    &project_s,
                                    &file_rel_path_s,
                                    &PipelineInvocationEntry {
                                        at: fired_at.timestamp(),
                                        duration_ms,
                                        status: "error".to_string(),
                                        trigger: "memsubscribe".to_string(),
                                        error: Some(e.message.clone()),
                                        trace: vec![],
                                    },
                                    log_max_n,
                                );
                                eprintln!(
                                    "❌ MemSubscriber failed {}/{}/{}: {}",
                                    owner_s, project_s, file_rel_path_s, e
                                );
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        eprintln!(
                            "MemSubscriber: channel '{}' lagged by {} messages — some events dropped",
                            channel_s, n
                        );
                        // Continue — lag is recoverable
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
        });

        self.tasks.lock().await.insert(task_key, handle);
    }
}
