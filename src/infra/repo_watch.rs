//! Background service driving `trigger.repo` pipelines.
//!
//! When a pipeline with `trigger.repo` is activated, a task listens to the
//! change record (`platform::services::change_log`) for its project. A change
//! under the watched path starts a burst; the burst ends when the repository
//! has been quiet for [`QUIET_MS`], or after [`MAX_WAIT_MS`], and the pipeline
//! runs once with every change in it. A change made by that pipeline's own run
//! is skipped, so a pipeline that writes under the path it watches cannot loop.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::Mutex;
use tokio::sync::broadcast::{Receiver, error::RecvError};
use tokio::task::JoinHandle;

use crate::pipeline::engines::BasicPipelineEngine;
use crate::pipeline::interface::PipelineEngine;
use crate::pipeline::model::PipelineContext;
use crate::pipeline::nodes::basic::trigger::repo::{self, MAX_WAIT_MS, NODE_KIND, QUIET_MS};
use crate::platform::adapters::data::DataAdapter;
use crate::platform::model::PipelineInvocationEntry;
use crate::platform::services::change_log::{self, ChangeActor, ChangeLog, RepoChange};
use crate::platform::services::pipeline_hits::PipelineHitsService;
use crate::platform::services::pipeline_runtime::{CompiledPipeline, PipelineRuntimeService};
use crate::platform::services::project_config::ProjectConfigurationService;

/// Background task registry for repository-triggered pipelines.
pub struct RepoWatcher {
    tasks: Arc<Mutex<HashMap<String, JoinHandle<()>>>>,
    changes: Arc<ChangeLog>,
    runner: Arc<Runner>,
}

/// What a task needs to run its pipeline.
struct Runner {
    runtime: Arc<PipelineRuntimeService>,
    engine: Arc<BasicPipelineEngine>,
    hits: Arc<PipelineHitsService>,
    data: Arc<dyn DataAdapter>,
    zebflow_cfg: Arc<ProjectConfigurationService>,
}

/// One watching task's subject.
struct Watch {
    owner: String,
    project: String,
    file: String,
    node_id: String,
    path: String,
}

impl Watch {
    /// The run origin its own pipeline's changes carry.
    fn own_run(&self) -> String {
        format!("{}/{}/{}", self.owner, self.project, self.file)
    }

    fn wants(&self, change: &RepoChange) -> bool {
        change.owner == self.owner
            && change.project == self.project
            && change.touches(&self.path)
            && change.run.as_deref() != Some(self.own_run().as_str())
    }
}

impl RepoWatcher {
    pub fn new(
        changes: Arc<ChangeLog>,
        runtime: Arc<PipelineRuntimeService>,
        engine: Arc<BasicPipelineEngine>,
        hits: Arc<PipelineHitsService>,
        data: Arc<dyn DataAdapter>,
        zebflow_cfg: Arc<ProjectConfigurationService>,
    ) -> Self {
        Self {
            tasks: Arc::new(Mutex::new(HashMap::new())),
            changes,
            runner: Arc::new(Runner { runtime, engine, hits, data, zebflow_cfg }),
        }
    }

    /// Starts a task for every active pipeline's `trigger.repo`.
    pub async fn register_all(&self) {
        for pipeline in self.runner.runtime.list_all() {
            self.register(&pipeline).await;
        }
    }

    /// Re-reads one pipeline's triggers (activate, deactivate, delete).
    pub async fn sync_pipeline(&self, owner: &str, project: &str, file_rel_path: &str) {
        let prefix = format!("{owner}/{project}/{file_rel_path}:");
        let mut tasks = self.tasks.lock().await;
        tasks.retain(|key, handle| {
            let stale = key.starts_with(&prefix);
            if stale {
                handle.abort();
            }
            !stale
        });
        drop(tasks);
        if let Some(pipeline) = self.runner.runtime.get(owner, project, file_rel_path) {
            self.register(&pipeline).await;
        }
    }

    async fn register(&self, pipeline: &CompiledPipeline) {
        for trigger in &pipeline.repo_triggers {
            let watch = Watch {
                owner: pipeline.owner.clone(),
                project: pipeline.project.clone(),
                file: pipeline.file_rel_path.clone(),
                node_id: trigger.node_id.clone(),
                path: trigger.path.clone(),
            };
            let key = format!("{}:{}", watch.own_run(), watch.node_id);
            let rx = self.changes.subscribe();
            let runner = self.runner.clone();
            let handle = tokio::spawn(async move { watch_loop(watch, rx, runner).await });
            if let Some(old) = self.tasks.lock().await.insert(key, handle) {
                old.abort();
            }
        }
    }
}

/// The next change the watch wants; none once the record is gone. A listener
/// that fell behind has missed changes it cannot name, so it answers one
/// `missed` change over the whole repository rather than nothing.
async fn next_wanted(watch: &Watch, rx: &mut Receiver<RepoChange>) -> Option<RepoChange> {
    loop {
        match rx.recv().await {
            Ok(change) if watch.wants(&change) => return Some(change),
            Ok(_) => {}
            Err(RecvError::Lagged(_)) => {
                return Some(RepoChange {
                    owner: watch.owner.clone(),
                    project: watch.project.clone(),
                    action: "missed".to_string(),
                    path: String::new(),
                    to_path: None,
                    person: String::new(),
                    via: "system".to_string(),
                    run: None,
                });
            }
            Err(RecvError::Closed) => return None,
        }
    }
}

async fn watch_loop(watch: Watch, mut rx: Receiver<RepoChange>, runner: Arc<Runner>) {
    while let Some(first) = next_wanted(&watch, &mut rx).await {
        let mut burst = vec![first];
        let started = Instant::now();
        let limit = Duration::from_millis(MAX_WAIT_MS);
        while let Some(left) = limit.checked_sub(started.elapsed()) {
            match tokio::time::timeout(left.min(Duration::from_millis(QUIET_MS)), next_wanted(&watch, &mut rx)).await {
                Ok(Some(change)) => burst.push(change),
                Ok(None) => return,
                Err(_quiet) => break,
            }
        }
        if !runner.run(&watch, &burst).await {
            return;
        }
    }
}

impl Runner {
    /// Runs the pipeline once for a burst and records the run. False when the
    /// pipeline is no longer active, which ends the task.
    async fn run(&self, watch: &Watch, burst: &[RepoChange]) -> bool {
        let Some(compiled) = self.runtime.get(&watch.owner, &watch.project, &watch.file) else {
            return false;
        };
        let (log_max_n, max_age_secs) = match self.retention(&compiled, watch) {
            Some(retention) => retention,
            None => return false,
        };
        let fired_at = chrono::Utc::now();
        let ctx = PipelineContext {
            owner: watch.owner.clone(),
            project: watch.project.clone(),
            pipeline: compiled.graph.id.clone(),
            request_id: format!("repo-{}", uuid::Uuid::new_v4()),
            route: String::new(),
            input: repo::envelope(&watch.path, &watch.node_id, burst),
            trigger: None,
            placeholder: None,
        };
        let actor = ChangeActor::pipeline(&watch.owner, &watch.project, &watch.file);
        let started = Instant::now();
        let result = change_log::scope(actor, self.engine.execute_async(&compiled.graph, &ctx)).await;
        let duration_ms = started.elapsed().as_millis() as u64;
        let (status, error, trace) = match result {
            Ok(output) => {
                self.hits.record_success(&watch.owner, &watch.project, &watch.file);
                ("ok", None, output.node_trace)
            }
            Err(err) => {
                self.hits.record_failure(&watch.owner, &watch.project, &watch.file, NODE_KIND, err.code, &err.message);
                eprintln!("trigger.repo failed {}: {}", watch.own_run(), err);
                ("error", Some(err.message.clone()), err.node_trace.clone())
            }
        };
        let _ = self.data.log_pipeline_invocation(
            &watch.owner,
            &watch.project,
            &watch.file,
            PipelineInvocationEntry {
                run_id: ctx.request_id.clone(),
                at: fired_at.timestamp(),
                duration_ms,
                status: status.to_string(),
                trigger: NODE_KIND.to_string(),
                error,
                trace,
            },
            log_max_n,
            max_age_secs,
        );
        true
    }

    /// How many runs to keep, and for how long: the pipeline's own setting,
    /// else the project's.
    fn retention(&self, compiled: &CompiledPipeline, watch: &Watch) -> Option<(usize, Option<i64>)> {
        let project_cfg = match self.zebflow_cfg.read_or_default(&watch.owner, &watch.project) {
            Ok(config) => config,
            Err(err) => {
                eprintln!("trigger.repo: invalid project configuration for {}/{}: {}", watch.owner, watch.project, err);
                return None;
            }
        };
        let own = compiled.graph.metadata.as_ref().and_then(|metadata| metadata.settings.invocation_retention.as_ref());
        let log_max_n = own
            .and_then(|retention| retention.max_invocations)
            .map(|value| value.max(1) as usize)
            .unwrap_or_else(|| project_cfg.configs.pipelines.logging.effective_max_invocations());
        let max_age_secs = own.and_then(|retention| retention.max_age_secs).map(|value| value.max(1) as i64);
        Some((log_max_n, max_age_secs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn change(project: &str, path: &str, run: Option<&str>) -> RepoChange {
        RepoChange {
            owner: "superadmin".into(),
            project: project.into(),
            action: "edit".into(),
            path: path.into(),
            to_path: None,
            person: String::new(),
            via: "studio".into(),
            run: run.map(str::to_string),
        }
    }

    #[test]
    fn a_watch_skips_its_own_runs_and_other_projects() {
        let watch = Watch {
            owner: "superadmin".into(),
            project: "site-a".into(),
            file: "pipelines/docs.zf.json".into(),
            node_id: "n0".into(),
            path: "docs".into(),
        };
        assert!(watch.wants(&change("site-a", "docs/a.md", None)));
        assert!(watch.wants(&change("site-a", "docs/a.md", Some("superadmin/site-a/pipelines/other.zf.json"))), "another pipeline's write counts");
        assert!(!watch.wants(&change("site-a", "docs/a.md", Some("superadmin/site-a/pipelines/docs.zf.json"))), "its own write never starts it again");
        assert!(!watch.wants(&change("site-b", "docs/a.md", None)), "another project");
        assert!(!watch.wants(&change("site-a", "docs-old/a.md", None)));
    }

    /// A pipeline run's writes carry the run, and a door that names its own
    /// actor inside the run keeps it.
    #[tokio::test]
    async fn a_run_is_carried_to_the_writer() {
        let actor = ChangeActor::pipeline("superadmin", "site-a", "pipelines/docs.zf.json");
        let seen = change_log::scope(actor, async { change_log::current_actor() }).await;
        assert_eq!(seen.run.as_deref(), Some("superadmin/site-a/pipelines/docs.zf.json"));
        assert_eq!(seen.via, "pipeline · pipelines/docs.zf.json");
        assert_eq!(change_log::current_actor().run, None, "outside a run there is none");
    }
}
