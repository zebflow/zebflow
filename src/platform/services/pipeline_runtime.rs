//! Active pipeline runtime registry.
//!
//! This registry intentionally uses activated runtime snapshots, not mutable
//! working-tree pipeline files. Draft pipeline edits update metadata and local
//! validation, while production execution reads only from the active snapshot set.

use std::collections::HashMap;
use std::sync::Arc;

use arc_swap::ArcSwap;
use serde::{Deserialize, Serialize};

use crate::framework::PipelineGraph;
use crate::platform::error::PlatformError;
use crate::platform::model::PipelineMeta;
use crate::platform::services::ProjectService;

/// Stable active pipeline key.
pub type ActivePipelineKey = String;

/// One extracted webhook trigger from an active compiled pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WebhookTriggerSpec {
    pub node_id: String,
    pub path: String,
    pub method: String,
}

/// One extracted schedule trigger from an active compiled pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScheduleTriggerSpec {
    pub node_id: String,
    pub cron: String,
    pub timezone: String,
}

/// Execution-ready active pipeline entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompiledPipeline {
    pub key: ActivePipelineKey,
    pub owner: String,
    pub project: String,
    pub file_rel_path: String,
    pub current_hash: String,
    pub active_hash: String,
    pub graph: PipelineGraph,
    pub webhook_triggers: Vec<WebhookTriggerSpec>,
    pub schedule_triggers: Vec<ScheduleTriggerSpec>,
}

impl CompiledPipeline {
    /// Builds one compiled runtime entry from active metadata and snapshot source.
    pub fn from_active_meta(meta: &PipelineMeta, source: &str) -> Result<Self, PlatformError> {
        let graph: PipelineGraph = serde_json::from_str(source).map_err(|err| {
            PlatformError::new(
                "PLATFORM_PIPELINE_PARSE",
                format!("failed parsing active pipeline '{}': {}", meta.file_rel_path, err),
            )
        })?;
        let mut webhook_triggers = Vec::new();
        let mut schedule_triggers = Vec::new();
        for node in &graph.nodes {
            match node.kind.as_str() {
                "x.n.trigger.webhook" => {
                    let path = node
                        .config
                        .get("path")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("/")
                        .to_string();
                    let method = node
                        .config
                        .get("method")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("POST")
                        .to_string();
                    webhook_triggers.push(WebhookTriggerSpec {
                        node_id: node.id.clone(),
                        path,
                        method,
                    });
                }
                "x.n.trigger.schedule" => {
                    let cron = node
                        .config
                        .get("cron")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    let timezone = node
                        .config
                        .get("timezone")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    schedule_triggers.push(ScheduleTriggerSpec {
                        node_id: node.id.clone(),
                        cron,
                        timezone,
                    });
                }
                _ => {}
            }
        }

        Ok(Self {
            key: active_pipeline_key(&meta.owner, &meta.project, &meta.file_rel_path),
            owner: meta.owner.clone(),
            project: meta.project.clone(),
            file_rel_path: meta.file_rel_path.clone(),
            current_hash: meta.hash.clone(),
            active_hash: meta.active_hash.clone().unwrap_or_default(),
            graph,
            webhook_triggers,
            schedule_triggers,
        })
    }
}

/// Production runtime registry for activated pipelines.
pub struct PipelineRuntimeService {
    projects: Arc<ProjectService>,
    inner: ArcSwap<HashMap<ActivePipelineKey, CompiledPipeline>>,
}

impl PipelineRuntimeService {
    pub fn new(projects: Arc<ProjectService>) -> Self {
        Self {
            projects,
            inner: ArcSwap::new(Arc::new(HashMap::new())),
        }
    }

    /// Rebuilds one project's active runtime snapshot.
    pub fn refresh_project(&self, owner: &str, project: &str) -> Result<(), PlatformError> {
        let owner = crate::platform::model::slug_segment(owner);
        let project = crate::platform::model::slug_segment(project);
        let active_rows = self.projects.list_active_pipeline_meta(&owner, &project)?;
        let mut next = (*self.inner.load_full()).clone();
        next.retain(|_, compiled| !(compiled.owner == owner && compiled.project == project));

        for meta in active_rows {
            let source = self.projects.read_active_pipeline_source(&owner, &project, &meta)?;
            let compiled = CompiledPipeline::from_active_meta(&meta, &source)?;
            next.insert(compiled.key.clone(), compiled);
        }

        self.inner.store(Arc::new(next));
        Ok(())
    }

    /// Refreshes one active pipeline entry only.
    pub fn refresh_pipeline(
        &self,
        owner: &str,
        project: &str,
        virtual_path: &str,
        name: &str,
    ) -> Result<(), PlatformError> {
        let owner = crate::platform::model::slug_segment(owner);
        let project = crate::platform::model::slug_segment(project);
        let Some(meta) = self
            .projects
            .get_pipeline_meta(&owner, &project, virtual_path, name)?
        else {
            return Err(PlatformError::new(
                "PLATFORM_PIPELINE_MISSING",
                "pipeline not found",
            ));
        };

        let key = active_pipeline_key(&owner, &project, &meta.file_rel_path);
        let mut next = (*self.inner.load_full()).clone();
        next.remove(&key);

        if meta.active_hash.is_some() {
            let source = self.projects.read_active_pipeline_source(&owner, &project, &meta)?;
            let compiled = CompiledPipeline::from_active_meta(&meta, &source)?;
            next.insert(compiled.key.clone(), compiled);
        }

        self.inner.store(Arc::new(next));
        Ok(())
    }

    pub fn get(&self, owner: &str, project: &str, file_rel_path: &str) -> Option<CompiledPipeline> {
        let key = active_pipeline_key(owner, project, file_rel_path);
        self.inner.load().get(&key).cloned()
    }

    pub fn list_project(&self, owner: &str, project: &str) -> Vec<CompiledPipeline> {
        let owner = crate::platform::model::slug_segment(owner);
        let project = crate::platform::model::slug_segment(project);
        self.inner
            .load()
            .values()
            .filter(|compiled| compiled.owner == owner && compiled.project == project)
            .cloned()
            .collect()
    }
}

pub fn active_pipeline_key(owner: &str, project: &str, file_rel_path: &str) -> ActivePipelineKey {
    format!(
        "{}/{}/{}",
        crate::platform::model::slug_segment(owner),
        crate::platform::model::slug_segment(project),
        file_rel_path.trim().replace('\\', "/")
    )
}
