//! Pipeline cron scheduler — background job runner for `n.trigger.schedule` pipelines.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;
use tokio_cron_scheduler::{Job, JobScheduler};
use uuid::Uuid;

use crate::pipeline::engines::BasicPipelineEngine;
use crate::pipeline::interface::PipelineEngine;
use crate::pipeline::model::PipelineContext;
use crate::platform::adapters::data::DataAdapter;
use crate::platform::model::PipelineInvocationEntry;
use crate::platform::services::pipeline_hits::PipelineHitsService;
use crate::platform::services::pipeline_runtime::PipelineRuntimeService;
use crate::platform::services::project_config::ZebflowJsonService;

/// Background cron scheduler for activated pipelines with `n.trigger.schedule` triggers.
pub struct PipelineScheduler {
    sched: Arc<JobScheduler>,
    runtime: Arc<PipelineRuntimeService>,
    engine: Arc<BasicPipelineEngine>,
    hits: Arc<PipelineHitsService>,
    data: Arc<dyn DataAdapter>,
    zebflow_cfg: Arc<ZebflowJsonService>,
    jobs: Arc<RwLock<HashMap<String, Uuid>>>,
}

impl PipelineScheduler {
    pub async fn start(
        runtime: Arc<PipelineRuntimeService>,
        engine: Arc<BasicPipelineEngine>,
        hits: Arc<PipelineHitsService>,
        data: Arc<dyn DataAdapter>,
        zebflow_cfg: Arc<ZebflowJsonService>,
    ) -> Result<Arc<Self>, Box<dyn std::error::Error + Send + Sync>> {
        let sched = JobScheduler::new().await?;
        sched.start().await?;

        let scheduler = Arc::new(Self {
            sched: Arc::new(sched),
            runtime,
            engine,
            hits,
            data,
            zebflow_cfg,
            jobs: Arc::new(RwLock::new(HashMap::new())),
        });

        scheduler.register_all().await;
        Ok(scheduler)
    }

    pub async fn register_all(&self) {
        for pipeline in self.runtime.list_all() {
            for trigger in &pipeline.schedule_triggers {
                self.register_job(
                    &pipeline.owner,
                    &pipeline.project,
                    &pipeline.file_rel_path,
                    &pipeline.graph.id,
                    &trigger.node_id,
                    &trigger.cron,
                )
                .await;
            }
        }
    }

    pub async fn sync_pipeline(&self, owner: &str, project: &str, file_rel_path: &str) {
        let key_prefix = format!("{}/{}/{}", owner, project, file_rel_path);

        let stale: Vec<(String, Uuid)> = {
            let jobs = self.jobs.read().await;
            jobs.iter()
                .filter(|(k, _)| k.starts_with(&key_prefix))
                .map(|(k, v)| (k.clone(), *v))
                .collect()
        };

        for (key, uuid) in stale {
            let _ = self.sched.remove(&uuid).await;
            self.jobs.write().await.remove(&key);
            println!("Scheduler: removed job {}", key);
        }

        if let Some(pipeline) = self.runtime.get(owner, project, file_rel_path) {
            for trigger in &pipeline.schedule_triggers {
                self.register_job(
                    owner,
                    project,
                    file_rel_path,
                    &pipeline.graph.id,
                    &trigger.node_id,
                    &trigger.cron,
                )
                .await;
            }
        }
    }

    async fn register_job(
        &self,
        owner: &str,
        project: &str,
        file_rel_path: &str,
        graph_id: &str,
        node_id: &str,
        cron: &str,
    ) {
        if cron.trim().is_empty() {
            eprintln!(
                "Scheduler: skipping {}/{}/{} — empty cron expression",
                owner, project, file_rel_path
            );
            return;
        }

        let job_key = format!("{}/{}/{}:{}", owner, project, file_rel_path, node_id);

        let runtime = self.runtime.clone();
        let engine = self.engine.clone();
        let hits = self.hits.clone();
        let data = self.data.clone();
        let zebflow_cfg_arc = self.zebflow_cfg.clone();
        let owner_s = owner.to_string();
        let project_s = project.to_string();
        let file_rel_path_s = file_rel_path.to_string();
        let graph_id_s = graph_id.to_string();
        let node_id_s = node_id.to_string();

        let job_result = Job::new_async(cron, move |_uuid, _sched| {
            let runtime = runtime.clone();
            let engine = engine.clone();
            let hits = hits.clone();
            let data = data.clone();
            let zebflow_cfg = zebflow_cfg_arc.clone();
            let owner = owner_s.clone();
            let project = project_s.clone();
            let file_rel_path = file_rel_path_s.clone();
            let graph_id = graph_id_s.clone();
            let node_id = node_id_s.clone();

            Box::pin(async move {
                println!("⏰ Schedule tick: {}/{}/{}", owner, project, file_rel_path);

                let Some(compiled) = runtime.get(&owner, &project, &file_rel_path) else {
                    eprintln!(
                        "Scheduler: pipeline no longer active — {}/{}/{}",
                        owner, project, file_rel_path
                    );
                    return;
                };

                let log_max_n = zebflow_cfg
                    .read_or_default(&owner, &project)
                    .logging
                    .effective_max_invocations();

                let fired_at = chrono::Utc::now();
                let ctx = PipelineContext {
                    owner: owner.clone(),
                    project: project.clone(),
                    pipeline: graph_id.clone(),
                    request_id: format!("schedule-{}", Uuid::new_v4()),
                    route: String::new(),
                    input: serde_json::json!({
                        "trigger": "schedule",
                        "fired_at": fired_at.to_rfc3339(),
                        "node_id": node_id,
                    }),
                };

                let exec_start = std::time::Instant::now();
                match engine.execute_async(&compiled.graph, &ctx).await {
                    Ok(output) => {
                        let duration_ms = exec_start.elapsed().as_millis() as u64;
                        hits.record_success(&owner, &project, &file_rel_path);
                        let _ = data.log_pipeline_invocation(
                            &owner,
                            &project,
                            &file_rel_path,
                            &PipelineInvocationEntry {
                                at: fired_at.timestamp(),
                                duration_ms,
                                status: "ok".to_string(),
                                trigger: "schedule".to_string(),
                                error: None,
                                trace: output.node_trace,
                            },
                            log_max_n,
                        );
                        println!("✅ Schedule OK: {}/{}/{}", owner, project, file_rel_path);
                    }
                    Err(e) => {
                        let duration_ms = exec_start.elapsed().as_millis() as u64;
                        hits.record_failure(
                            &owner,
                            &project,
                            &file_rel_path,
                            "schedule",
                            e.code,
                            &e.message,
                        );
                        let _ = data.log_pipeline_invocation(
                            &owner,
                            &project,
                            &file_rel_path,
                            &PipelineInvocationEntry {
                                at: fired_at.timestamp(),
                                duration_ms,
                                status: "error".to_string(),
                                trigger: "schedule".to_string(),
                                error: Some(e.message.clone()),
                                trace: vec![],
                            },
                            log_max_n,
                        );
                        eprintln!(
                            "❌ Schedule failed {}/{}/{}: {}",
                            owner, project, file_rel_path, e
                        );
                    }
                }
            })
        });

        match job_result {
            Ok(job) => match self.sched.add(job).await {
                Ok(uuid) => {
                    self.jobs.write().await.insert(job_key.clone(), uuid);
                    println!("✅ Scheduler: registered job {} cron={}", job_key, cron);
                }
                Err(e) => eprintln!("Scheduler: failed to add job {}: {}", job_key, e),
            },
            Err(e) => eprintln!(
                "Scheduler: invalid cron '{}' for {}: {}",
                cron, job_key, e
            ),
        }
    }
}
