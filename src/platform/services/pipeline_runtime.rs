//! Active pipeline runtime registry.
//!
//! This registry intentionally uses activated runtime snapshots, not mutable
//! working-tree pipeline files. Draft pipeline edits update metadata and local
//! validation, while production execution reads only from the active snapshot set.

use std::collections::HashMap;
use std::sync::Arc;

use arc_swap::ArcSwap;
use serde::{Deserialize, Serialize};

use crate::pipeline::PipelineGraph;
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
    /// Auth type: `"none"`, `"jwt"`, `"hmac"`, `"api_key"`.
    #[serde(default)]
    pub auth_type: String,
    /// Credential ID for auth verification.
    #[serde(default)]
    pub auth_credential: String,
}

/// One extracted weberror trigger from an active compiled pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WebErrorTriggerSpec {
    pub node_id: String,
    /// Code pattern: `"404"`, `"4xx"`, `"5xx"`, `"*"`, or `""` (catch-all).
    pub code: String,
}

/// One extracted schedule trigger from an active compiled pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScheduleTriggerSpec {
    pub node_id: String,
    pub cron: String,
    pub timezone: String,
}

/// One extracted WS trigger from an active compiled pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WsTriggerSpec {
    pub node_id: String,
    /// Room pattern — empty matches any room.
    pub room: String,
    /// Event pattern — empty matches any event.
    pub event: String,
}

/// Describes one webhook path conflict found during pipeline registration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebhookPathConflict {
    pub path: String,
    pub method: String,
    pub pipeline_name: String,
    pub file_rel_path: String,
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
    pub ws_triggers: Vec<WsTriggerSpec>,
    pub weberror_triggers: Vec<WebErrorTriggerSpec>,
}

impl CompiledPipeline {
    /// Builds one compiled runtime entry from active metadata and snapshot source.
    pub fn from_active_meta(meta: &PipelineMeta, source: &str) -> Result<Self, PlatformError> {
        let graph: PipelineGraph = serde_json::from_str(source).map_err(|err| {
            PlatformError::new(
                "PLATFORM_PIPELINE_PARSE",
                format!(
                    "failed parsing active pipeline '{}': {}",
                    meta.file_rel_path, err
                ),
            )
        })?;

        // Guard: reject pipelines with node configs that violate their definition.
        let definitions = crate::pipeline::nodes::builtin_node_definitions();
        for node in &graph.nodes {
            let Some(def) = definitions.iter().find(|d| d.kind == node.kind) else {
                continue;
            };
            let required = def
                .config_schema
                .get("required")
                .and_then(|v| v.as_array())
                .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>())
                .unwrap_or_default();
            for field in required {
                let present = node.config.get(field).map(|v| !v.is_null()).unwrap_or(false);
                let non_empty = node.config
                    .get(field)
                    .and_then(|v| v.as_str())
                    .map(|s| !s.trim().is_empty())
                    .unwrap_or(true);
                if !present || !non_empty {
                    return Err(PlatformError::new(
                        "PIPELINE_NODE_CONFIG_VIOLATION",
                        format!(
                            "node '{}' (id: '{}') in pipeline '{}' is missing required config field '{}' \
                            as defined by its node definition. \
                            Pipeline rejected.",
                            node.kind, node.id, meta.file_rel_path, field
                        ),
                    ));
                }
            }
        }
        let mut webhook_triggers = Vec::new();
        let mut schedule_triggers = Vec::new();
        let mut ws_triggers = Vec::new();
        let mut weberror_triggers = Vec::new();
        for node in &graph.nodes {
            match node.kind.as_str() {
                "n.trigger.webhook" => {
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
                    let auth_type = node
                        .config
                        .get("auth_type")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    let auth_credential = node
                        .config
                        .get("auth_credential")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    webhook_triggers.push(WebhookTriggerSpec {
                        node_id: node.id.clone(),
                        path,
                        method,
                        auth_type,
                        auth_credential,
                    });
                }
                "n.trigger.weberror" => {
                    let code = node
                        .config
                        .get("code")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    weberror_triggers.push(WebErrorTriggerSpec {
                        node_id: node.id.clone(),
                        code,
                    });
                }
                "n.trigger.schedule" => {
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
                "n.trigger.ws" => {
                    let room = node
                        .config
                        .get("room")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    let event = node
                        .config
                        .get("event")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    ws_triggers.push(WsTriggerSpec {
                        node_id: node.id.clone(),
                        room,
                        event,
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
            ws_triggers,
            weberror_triggers,
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
            let source = self
                .projects
                .read_active_pipeline_source(&owner, &project, &meta)?;
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
        file_rel_path: &str,
    ) -> Result<(), PlatformError> {
        let owner = crate::platform::model::slug_segment(owner);
        let project = crate::platform::model::slug_segment(project);
        let Some(meta) = self
            .projects
            .get_pipeline_meta_by_file_id(&owner, &project, file_rel_path)?
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
            let source = self
                .projects
                .read_active_pipeline_source(&owner, &project, &meta)?;
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

    /// Removes one pipeline from the active runtime registry without refreshing from disk.
    pub fn evict(&self, owner: &str, project: &str, file_rel_path: &str) {
        let key = active_pipeline_key(owner, project, file_rel_path);
        let mut next = (*self.inner.load_full()).clone();
        next.remove(&key);
        self.inner.store(Arc::new(next));
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

    /// Returns any active pipelines that already claim the same `{method, path}` webhook
    /// combination as the incoming graph.  Pass `self_file_rel_path` so a pipeline updating
    /// itself is not reported as a conflict.
    pub fn check_webhook_path_conflict(
        &self,
        owner: &str,
        project: &str,
        graph: &crate::pipeline::PipelineGraph,
        self_file_rel_path: &str,
    ) -> Vec<WebhookPathConflict> {
        use crate::pipeline::nodes::basic::trigger::webhook::NODE_KIND as WEBHOOK_KIND;

        let new_triggers: Vec<(String, String)> = graph
            .nodes
            .iter()
            .filter(|n| n.kind == WEBHOOK_KIND)
            .map(|n| {
                let path = n
                    .config
                    .get("path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("/")
                    .to_string();
                let method = n
                    .config
                    .get("method")
                    .and_then(|v| v.as_str())
                    .unwrap_or("POST")
                    .to_uppercase();
                (method, path)
            })
            .collect();

        if new_triggers.is_empty() {
            return vec![];
        }

        let mut conflicts = vec![];
        for compiled in self.list_project(owner, project) {
            if compiled.file_rel_path == self_file_rel_path {
                continue;
            }
            for trigger in &compiled.webhook_triggers {
                let existing = (trigger.method.to_uppercase(), trigger.path.clone());
                for new in &new_triggers {
                    if *new == existing {
                        conflicts.push(WebhookPathConflict {
                            path: trigger.path.clone(),
                            method: trigger.method.to_uppercase(),
                            pipeline_name: compiled
                                .file_rel_path
                                .trim_end_matches(".zf.json")
                                .split('/')
                                .last()
                                .unwrap_or(&compiled.file_rel_path)
                                .to_string(),
                            file_rel_path: compiled.file_rel_path.clone(),
                        });
                    }
                }
            }
        }
        conflicts
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
