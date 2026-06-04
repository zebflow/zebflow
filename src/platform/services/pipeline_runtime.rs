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
    /// Required roles — JWT claim `role` must match one. Empty = any authenticated user.
    #[serde(default)]
    pub auth_required_role: Vec<String>,
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
    /// Auth type: `"none"`, `"jwt"`, `"hmac"`, `"api_key"`.
    #[serde(default)]
    pub auth_type: String,
    /// Credential ID for auth verification.
    #[serde(default)]
    pub auth_credential: String,
    /// Required roles — JWT claim `roles` must match one. Empty = any authenticated user.
    #[serde(default)]
    pub auth_required_role: Vec<String>,
}

/// One extracted KV subscribe trigger from an active compiled pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KvSubscribeTriggerSpec {
    pub node_id: String,
    /// Channel name to subscribe to.
    pub channel: String,
}

/// One extracted WS client trigger from an active compiled pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WsClientTriggerSpec {
    pub node_id: String,
    /// WebSocket server URL to connect to (ws:// or wss://).
    pub url: String,
    /// Credential ID for injecting auth headers/tokens into the connection.
    #[serde(default)]
    pub credential_id: String,
    /// Whether to auto-reconnect on disconnect.
    #[serde(default = "default_true")]
    pub reconnect: bool,
    /// Base reconnect delay in milliseconds. Exponential backoff applied.
    #[serde(default = "default_reconnect_delay_ms")]
    pub reconnect_delay_ms: u64,
    /// Max reconnect attempts (0 = infinite).
    #[serde(default)]
    pub max_reconnect_attempts: u64,
    /// Heartbeat ping interval in milliseconds.
    #[serde(default = "default_heartbeat_interval_ms")]
    pub heartbeat_interval_ms: u64,
    /// Message format hint: "json" or "text".
    #[serde(default = "default_message_format")]
    pub message_format: String,
}

fn default_true() -> bool {
    true
}
fn default_reconnect_delay_ms() -> u64 {
    5000
}
fn default_heartbeat_interval_ms() -> u64 {
    30000
}
fn default_message_format() -> String {
    "json".to_string()
}

/// One extracted MCP trigger from an active compiled pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpTriggerSpec {
    pub node_id: String,
    /// MCP tool name — the identifier used in `tools/list` and `tools/call`.
    pub tool_name: String,
    /// Human-readable tool description shown to AI agents.
    pub tool_description: String,
    /// JSON Schema object describing the tool input parameters.
    pub input_schema: serde_json::Value,
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
    pub kv_subscribe_triggers: Vec<KvSubscribeTriggerSpec>,
    pub ws_client_triggers: Vec<WsClientTriggerSpec>,
    pub mcp_triggers: Vec<McpTriggerSpec>,
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
                let present = node
                    .config
                    .get(field)
                    .map(|v| !v.is_null())
                    .unwrap_or(false);
                let non_empty = node
                    .config
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
        let mut kv_subscribe_triggers = Vec::new();
        let mut ws_client_triggers = Vec::new();
        let mut mcp_triggers = Vec::new();
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
                    let auth_required_role = node
                        .config
                        .get("auth_required_role")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(ToString::to_string))
                                .collect()
                        })
                        .unwrap_or_default();
                    webhook_triggers.push(WebhookTriggerSpec {
                        node_id: node.id.clone(),
                        path,
                        method,
                        auth_type,
                        auth_credential,
                        auth_required_role,
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
                    let auth_required_role = node
                        .config
                        .get("auth_required_role")
                        .and_then(|v| v.as_array())
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(ToString::to_string))
                                .collect()
                        })
                        .unwrap_or_default();
                    ws_triggers.push(WsTriggerSpec {
                        node_id: node.id.clone(),
                        room,
                        event,
                        auth_type,
                        auth_credential,
                        auth_required_role,
                    });
                }
                "n.trigger.kv.subscribe" => {
                    let channel = node
                        .config
                        .get("channel")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    kv_subscribe_triggers.push(KvSubscribeTriggerSpec {
                        node_id: node.id.clone(),
                        channel,
                    });
                }
                "n.trigger.ws.client" => {
                    let url = node
                        .config
                        .get("url")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    let credential_id = node
                        .config
                        .get("credential_id")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    let reconnect = node
                        .config
                        .get("reconnect")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(true);
                    let reconnect_delay_ms = node
                        .config
                        .get("reconnect_delay_ms")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(5000);
                    let max_reconnect_attempts = node
                        .config
                        .get("max_reconnect_attempts")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(0);
                    let heartbeat_interval_ms = node
                        .config
                        .get("heartbeat_interval_ms")
                        .and_then(serde_json::Value::as_u64)
                        .unwrap_or(30000);
                    let message_format = node
                        .config
                        .get("message_format")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("json")
                        .to_string();
                    if !url.is_empty() {
                        ws_client_triggers.push(WsClientTriggerSpec {
                            node_id: node.id.clone(),
                            url,
                            credential_id,
                            reconnect,
                            reconnect_delay_ms,
                            max_reconnect_attempts,
                            heartbeat_interval_ms,
                            message_format,
                        });
                    }
                }
                "n.trigger.mcp" => {
                    let tool_name = node
                        .config
                        .get("tool_name")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    let tool_description = node
                        .config
                        .get("tool_description")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    let params_str = node
                        .config
                        .get("parameters")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default();
                    let input_schema =
                        crate::pipeline::nodes::basic::trigger::mcp_trigger::params_to_json_schema(
                            params_str,
                        );
                    if !tool_name.is_empty() {
                        mcp_triggers.push(McpTriggerSpec {
                            node_id: node.id.clone(),
                            tool_name,
                            tool_description,
                            input_schema,
                        });
                    }
                }
                // Composite trigger nodes (n.c.trigger.*): extract webhook trigger
                // from the node's trigger metadata in the package manifest.
                other if other.starts_with("n.c.trigger.") => {
                    // Composite triggers embed their trigger type in the package
                    // manifest.  At compile-time we don't have the node registry,
                    // so we derive the webhook path from a `path` config key on
                    // the node (set by DSL / UI).  If no explicit path, fall back
                    // to a conventional `/tg/{credential_id}` style path built
                    // from `path_template` or node kind suffix + credential id.
                    let path = node
                        .config
                        .get("path")
                        .and_then(serde_json::Value::as_str)
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| {
                            // Convention: /{kind_suffix}/{credential_id}
                            // e.g. n.c.trigger.tg → /tg/{bot_credential_id}
                            let suffix = other.strip_prefix("n.c.trigger.").unwrap_or("hook");
                            let cred_id = node
                                .config
                                .get("bot_credential_id")
                                .or_else(|| node.config.get("credential_id"))
                                .and_then(serde_json::Value::as_str)
                                .unwrap_or("default");
                            format!("/{}/{}", suffix, cred_id)
                        });
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
                        auth_type: String::new(),
                        auth_credential: String::new(),
                        auth_required_role: Vec::new(),
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
            kv_subscribe_triggers,
            ws_client_triggers,
            mcp_triggers,
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

        for meta in &active_rows {
            let source = match self
                .projects
                .read_active_pipeline_source(&owner, &project, meta)
            {
                Ok(s) => s,
                Err(e) => {
                    eprintln!(
                        "⚠ pipeline {}/{}/{}: skipping — {}",
                        owner, project, meta.name, e.message
                    );
                    continue;
                }
            };
            let compiled = match CompiledPipeline::from_active_meta(meta, &source) {
                Ok(c) => c,
                Err(e) => {
                    eprintln!(
                        "⚠ pipeline {}/{}/{}: compile error — {}",
                        owner, project, meta.name, e.message
                    );
                    continue;
                }
            };
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
        let Some(meta) =
            self.projects
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

    /// Returns all active compiled pipelines across every owner/project.
    pub fn list_all(&self) -> Vec<CompiledPipeline> {
        self.inner.load().values().cloned().collect()
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
