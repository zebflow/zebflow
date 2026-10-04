//! Active pipeline runtime registry.
//!
//! This registry intentionally uses activated runtime snapshots, not mutable
//! working-tree pipeline files. Draft pipeline edits update metadata and local
//! validation, while production execution reads only from the active snapshot set.

use std::collections::HashMap;
use std::sync::Arc;

use arc_swap::ArcSwap;
use serde::{Deserialize, Serialize};

use crate::contracts::kinds::decode_pipeline_graph;
use crate::pipeline::PipelineGraph;
use crate::platform::error::PlatformError;
use crate::platform::model::PipelineMeta;
use crate::platform::services::{NodeRegistryService, ProjectService};

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
    /// A public route that knows who is signed in: auth failure means
    /// `webhook.auth` is null, not a 401.
    #[serde(default)]
    pub auth_optional: bool,
    /// `show` or `hide`: what this route's 5xx reveals, overriding the
    /// project's `errors` switch; empty means the project decides
    /// (`addressing.md` §2a).
    #[serde(default)]
    pub errors: String,
}

/// One extracted weberror trigger from an active compiled pipeline.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WebErrorTriggerSpec {
    pub node_id: String,
    /// Status pattern (`--status`): `"404"`, `"4xx"`, `"5xx"`, `"*"`, or `""` (catch-all).
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
    /// Topic (`--topic`, the publisher's channel) to subscribe to.
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
    /// First reconnect delay (`--delay`) in milliseconds; backoff doubles it.
    pub delay_ms: u64,
    /// Reconnects allowed (`--max-attempts`): `None` unlimited, `Some(0)` never.
    #[serde(default)]
    pub max_attempts: Option<u64>,
    /// Heartbeat ping interval (`--heartbeat`) in milliseconds.
    pub heartbeat_ms: u64,
    /// How a message is read (`--parse`): `json` or `text`.
    pub parse: String,
}

/// One extracted `trigger.mcp` from an active compiled pipeline: one tool of
/// the published MCP server on its route (`published-mcp.md`). Read only by
/// that route's server, never by the project's dev MCP.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct McpTriggerSpec {
    pub node_id: String,
    /// The route the server answers on, normalized (`/shop`), relative to
    /// the `mcp` surface (`/_mcp/shop`, `/mcp/{o}/{p}/shop`).
    pub route: String,
    /// The tool name in `tools/list` and `tools/call`, unique on its route.
    pub tool_name: String,
    /// What the agent reads to decide when to call the tool.
    pub tool_description: String,
    /// JSON Schema object of the tool's arguments (`--parameter`).
    pub input_schema: serde_json::Value,
    /// `none`, `jwt` or `api_key` — the same on every tool of a route.
    pub auth_type: String,
    #[serde(default)]
    pub auth_credential: String,
    #[serde(default)]
    pub auth_required_role: Vec<String>,
    /// `show` or `hide`; empty means the project decides.
    #[serde(default)]
    pub errors: String,
}

impl McpTriggerSpec {
    /// The guard of the route, compared across its tools: `(auth, credential, roles)`.
    pub fn guard(&self) -> (String, String, Vec<String>) {
        let mut roles = self.auth_required_role.clone();
        roles.sort();
        (self.auth_type.clone(), self.auth_credential.clone(), roles)
    }
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
    /// Compiles one activated pipeline.
    ///
    /// `registry` resolves trigger roles for installed (`x.*`) nodes. Pass
    /// `None` when only compile validity is needed; installed trigger routes are
    /// then skipped because their role cannot be resolved.
    pub fn from_active_meta(
        meta: &PipelineMeta,
        source: &str,
        registry: Option<&crate::platform::services::NodeRegistryService>,
    ) -> Result<Self, PlatformError> {
        let graph = decode_pipeline_graph(source.as_bytes())
            .map_err(|err| {
                PlatformError::new(
                    "PLATFORM_PIPELINE_PARSE",
                    format!(
                        "failed parsing active pipeline '{}': {} ({})",
                        meta.file_rel_path,
                        err,
                        err.category()
                    ),
                )
            })?
            .spec;

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
                "trigger.webhook" => {
                    check_trigger_flags(meta, node)?;
                    let path = node
                        .config
                        .get("route")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("/")
                        .to_string();
                    // `/_…` is where the platform's surfaces answer on a
                    // project host (`addressing.md` §2); an app route there
                    // would be shadowed on every host but the neutral one.
                    if path.trim_start().starts_with(crate::platform::services::addressing::RESERVED_PREFIX) {
                        return Err(PlatformError::new(
                            "PIPELINE_ROUTE_RESERVED",
                            format!(
                                "pipeline '{}': webhook route '{}' starts with `/_`, which is reserved for the platform's surfaces on a project host (/_files, /_ws, …). Choose another route.",
                                meta.file_rel_path, path
                            ),
                        ));
                    }
                    let method = node
                        .config
                        .get("method")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or("POST")
                        .to_string();
                    let (auth_type, auth_credential, auth_required_role) = trigger_auth(&node.config);
                    let auth_optional = node
                        .config
                        .get("auth_optional")
                        .and_then(serde_json::Value::as_bool)
                        .unwrap_or(false);
                    let errors = node
                        .config
                        .get("errors")
                        .and_then(serde_json::Value::as_str)
                        .map(|s| s.trim().to_ascii_lowercase())
                        .unwrap_or_default();
                    webhook_triggers.push(WebhookTriggerSpec {
                        node_id: node.id.clone(),
                        path,
                        method,
                        auth_type,
                        auth_credential,
                        auth_required_role,
                        auth_optional,
                        errors,
                    });
                }
                "trigger.error" => {
                    // `--status 404` arrives as a number from the DSL; as a string
                    // it would have silently become the catch-all.
                    let code = match node.config.get("status") {
                        Some(serde_json::Value::String(s)) => s.clone(),
                        Some(serde_json::Value::Number(n)) => n.to_string(),
                        _ => String::new(),
                    };
                    weberror_triggers.push(WebErrorTriggerSpec {
                        node_id: node.id.clone(),
                        code,
                    });
                }
                "trigger.schedule" => {
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
                "trigger.room" => {
                    check_trigger_flags(meta, node)?;
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
                    let (auth_type, auth_credential, auth_required_role) = trigger_auth(&node.config);
                    ws_triggers.push(WsTriggerSpec {
                        node_id: node.id.clone(),
                        room,
                        event,
                        auth_type,
                        auth_credential,
                        auth_required_role,
                    });
                }
                "trigger.topic" => {
                    let channel = node
                        .config
                        .get("topic")
                        .and_then(serde_json::Value::as_str)
                        .unwrap_or_default()
                        .to_string();
                    kv_subscribe_triggers.push(KvSubscribeTriggerSpec {
                        node_id: node.id.clone(),
                        channel,
                    });
                }
                "trigger.socket" => {
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
                    let settings = crate::pipeline::nodes::basic::trigger::ws_client::Settings::of(&node.config)
                        .map_err(|err| trigger_refused(meta, node, err))?;
                    if !url.is_empty() {
                        ws_client_triggers.push(WsClientTriggerSpec {
                            node_id: node.id.clone(),
                            url,
                            credential_id,
                            delay_ms: settings.delay_ms,
                            max_attempts: settings.max_attempts,
                            heartbeat_ms: settings.heartbeat_ms,
                            parse: settings.parse,
                        });
                    }
                }
                "trigger.mcp" => {
                    mcp_triggers.push(mcp_trigger_spec(meta, node)?);
                }
                // Bundle-provided nodes declare their trigger role in the
                // package manifest; a kind no manifest names is not a trigger.
                other => {
                    let Some(trigger) = registry
                        .and_then(|registry| {
                            registry.get_manifest(&meta.owner, &meta.project, other)
                        })
                        .and_then(|manifest| manifest.trigger)
                        .filter(|trigger| trigger.trigger_type == "webhook")
                    else {
                        continue;
                    };
                    // An explicit `path` always wins. Otherwise the manifest's
                    // template is rendered against this node's configuration.
                    let path = node
                        .config
                        .get("path")
                        .and_then(serde_json::Value::as_str)
                        .map(|value| value.to_string())
                        .or_else(|| {
                            trigger
                                .path_template
                                .as_deref()
                                .map(|template| render_trigger_path(template, &node.config))
                        })
                        .unwrap_or_else(|| format!("/{}", node.id));
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
                        auth_optional: false,
                        errors: String::new(),
                    });
                }
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

/// `--auth`, `--credential`, `--role` of a webhook or room trigger.
fn trigger_auth(config: &serde_json::Value) -> (String, String, Vec<String>) {
    let text = |key: &str| {
        config
            .get(key)
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    let roles = config
        .get("role")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(ToString::to_string))
                .collect()
        })
        .unwrap_or_default();
    (text("auth"), text("credential_id"), roles)
}

/// A trigger's flags as the pipeline's refusal at activation.
fn trigger_refused(
    meta: &PipelineMeta,
    node: &crate::pipeline::PipelineNode,
    err: crate::pipeline::PipelineError,
) -> PlatformError {
    PlatformError::new(
        err.code,
        format!(
            "pipeline '{}': {} (node '{}'): {}",
            meta.file_rel_path, node.kind, node.id, err.message
        ),
    )
}

/// The closed words of a webhook or room trigger (`--method`, `--auth`,
/// `--errors`), checked when the pipeline is activated: an unknown word is
/// refused, never mapped to a default.
fn check_trigger_flags(
    meta: &PipelineMeta,
    node: &crate::pipeline::PipelineNode,
) -> Result<(), PlatformError> {
    use crate::pipeline::nodes::basic::trigger::webhook;
    use crate::pipeline::nodes::shared::limits::choice;
    let code = if node.kind == "trigger.room" {
        "FW_NODE_TRIGGER_ROOM_CONFIG"
    } else {
        webhook::CONFIG_CODE
    };
    let text = |key: &str| {
        node.config
            .get(key)
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string()
    };
    let refuse = |err| trigger_refused(meta, node, err);
    let auth = choice(&text("auth"), &webhook::AUTH_MODES, "none", "--auth", code).map_err(refuse)?;
    // A guard without its key would refuse every request at run time; say so
    // once, at activation, instead.
    if auth != "none" && text("credential_id").trim().is_empty() {
        return Err(refuse(crate::pipeline::PipelineError::new(
            code,
            format!("--auth {auth} needs --credential naming the credential that verifies it"),
        )));
    }
    let has_role = node.config.get("role").and_then(serde_json::Value::as_array).is_some_and(|roles| !roles.is_empty());
    if has_role && auth != "jwt" {
        return Err(refuse(crate::pipeline::PipelineError::new(code, "--role checks JWT claims: it needs --auth jwt")));
    }
    if node.kind == "trigger.webhook" {
        choice(&text("method"), &webhook::METHODS, "GET", "--method", code).map_err(refuse)?;
        choice(&text("errors"), &webhook::ERRORS_MODES, "show", "--errors", code).map_err(refuse)?;
    }
    Ok(())
}

/// A `trigger.mcp` as its route's server reads it, refused at activation
/// when it cannot publish: a route that is not a path, a tool name MCP
/// clients refuse, `--auth` missing or outside its words (publishing is never
/// implicit), a guard without its credential, a role without `jwt`.
fn mcp_trigger_spec(
    meta: &PipelineMeta,
    node: &crate::pipeline::PipelineNode,
) -> Result<McpTriggerSpec, PlatformError> {
    use crate::pipeline::nodes::basic::trigger::{mcp_trigger, webhook};
    use crate::pipeline::nodes::shared::limits::choice;
    let code = mcp_trigger::CONFIG_CODE;
    let refuse = |message: String| trigger_refused(meta, node, crate::pipeline::PipelineError::new(code, message));
    let text = |key: &str| {
        node.config
            .get(key)
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string()
    };
    let raw_route = text("route");
    if !raw_route.starts_with('/') || raw_route.contains([':', '*', '?', '#', ' ']) || raw_route.contains("..") {
        return Err(refuse(format!(
            "--route '{raw_route}' must be a path starting with / and without parameters (`/shop`); it is served at /_mcp/… on the project's hosts"
        )));
    }
    let tool_name = text("name");
    if !mcp_trigger::valid_tool_name(&tool_name) {
        return Err(refuse(format!(
            "--name '{tool_name}' must be 1–128 letters, digits, `_`, `-` or `.` — the name MCP clients call"
        )));
    }
    let auth = text("auth");
    if auth.is_empty() {
        return Err(refuse("--auth is required: none (open), jwt or api_key — publishing is never implicit".to_string()));
    }
    let auth = choice(&auth, &mcp_trigger::AUTH_MODES, "none", "--auth", code)
        .map_err(|err| trigger_refused(meta, node, err))?
        .to_string();
    let (_, auth_credential, auth_required_role) = trigger_auth(&node.config);
    if auth != "none" && auth_credential.trim().is_empty() {
        return Err(refuse(format!("--auth {auth} needs --credential naming the credential that verifies it")));
    }
    if !auth_required_role.is_empty() && auth != "jwt" {
        return Err(refuse("--role checks JWT claims: it needs --auth jwt".to_string()));
    }
    let errors = choice(&text("errors"), &webhook::ERRORS_MODES, "", "--errors", code)
        .map_err(|err| trigger_refused(meta, node, err))?;
    Ok(McpTriggerSpec {
        node_id: node.id.clone(),
        route: mcp_trigger::normalize_route(&raw_route),
        tool_name,
        tool_description: text("description"),
        input_schema: mcp_trigger::input_schema_from_config(&node.config),
        auth_type: auth,
        auth_credential,
        auth_required_role,
        errors: errors.to_string(),
    })
}

/// Production runtime registry for activated pipelines.
pub struct PipelineRuntimeService {
    projects: Arc<ProjectService>,
    node_registry: Arc<NodeRegistryService>,
    inner: ArcSwap<HashMap<ActivePipelineKey, CompiledPipeline>>,
}

impl PipelineRuntimeService {
    pub fn new(projects: Arc<ProjectService>, node_registry: Arc<NodeRegistryService>) -> Self {
        Self {
            projects,
            node_registry,
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
            let compiled = match CompiledPipeline::from_active_meta(
                meta,
                &source,
                Some(&self.node_registry),
            ) {
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
            let compiled =
                CompiledPipeline::from_active_meta(&meta, &source, Some(&self.node_registry))?;
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

/// Renders a manifest trigger path template against a node's configuration.
///
/// `{{ key }}` is replaced by the string value of that configuration key. An
/// unset key renders as `default`, which keeps a route stable instead of
/// producing an unroutable path.
fn render_trigger_path(template: &str, config: &serde_json::Value) -> String {
    let mut out = String::with_capacity(template.len());
    let mut rest = template;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let Some(end) = after.find("}}") else {
            out.push_str(&rest[start..]);
            return out;
        };
        let key = after[..end].trim();
        let value = config
            .get(key)
            .and_then(serde_json::Value::as_str)
            .unwrap_or("default");
        out.push_str(value);
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    out
}

#[cfg(test)]
mod trigger_tests {
    use super::CompiledPipeline;
    use crate::contracts::kinds::encode_pipeline_graph;
    use crate::platform::model::PipelineMeta;
    use crate::platform::shell::parser::build_pipeline_graph;

    fn compile(dsl: &str) -> Result<CompiledPipeline, crate::platform::error::PlatformError> {
        let graph = build_pipeline_graph("triggers", dsl).expect("graph");
        let source = String::from_utf8(encode_pipeline_graph(graph).expect("encode")).expect("utf8");
        let meta = PipelineMeta {
            owner: "demo".into(),
            project: "site-a".into(),
            name: "triggers".into(),
            title: String::new(),
            virtual_path: String::new(),
            file_rel_path: "pipelines/triggers.zf.json".into(),
            description: String::new(),
            trigger_kind: String::new(),
            hash: "h".into(),
            active_hash: Some("h".into()),
            activated_at: None,
            created_at: 0,
            updated_at: 0,
        };
        CompiledPipeline::from_active_meta(&meta, &source, None)
    }

    #[test]
    fn a_webhook_route_is_registered_with_its_guard_and_roles() {
        let compiled = compile(
            "| trigger.webhook --route /admin/posts/:id --method POST --auth jwt --credential jwt_main --role editor --role admin --auth-optional --errors hide",
        )
        .expect("compiles");
        let spec = &compiled.webhook_triggers[0];
        assert_eq!((spec.path.as_str(), spec.method.as_str()), ("/admin/posts/:id", "POST"));
        assert_eq!((spec.auth_type.as_str(), spec.auth_credential.as_str()), ("jwt", "jwt_main"));
        assert_eq!(spec.auth_required_role, vec!["editor".to_string(), "admin".to_string()]);
        assert!(spec.auth_optional);
        assert_eq!(spec.errors, "hide");
    }

    /// A guard that cannot verify anything is refused when the pipeline is
    /// activated, not on every request afterwards.
    #[test]
    fn a_guard_without_its_credential_or_a_role_without_jwt_is_refused_at_activation() {
        for (dsl, says) in [
            ("| trigger.webhook --route /x --auth jwt", "needs --credential"),
            ("| trigger.webhook --route /x --role editor", "needs --auth jwt"),
            ("| trigger.webhook --route /x --auth hmac --credential hook_key --role editor", "needs --auth jwt"),
        ] {
            let err = compile(dsl).err().unwrap_or_else(|| panic!("accepted: {dsl}"));
            assert_eq!(err.code, "FW_NODE_TRIGGER_WEBHOOK_CONFIG", "{dsl}");
            assert!(err.message.contains(says), "{dsl}: {}", err.message);
        }
    }

    #[test]
    fn an_unknown_word_in_a_closed_choice_is_refused_at_activation() {
        for dsl in [
            "| trigger.webhook --route /x --method FETCH",
            "| trigger.webhook --route /x --auth oauth",
            "| trigger.webhook --route /x --errors loud",
        ] {
            let err = compile(dsl).err().unwrap_or_else(|| panic!("accepted: {dsl}"));
            assert_eq!(err.code, "FW_NODE_TRIGGER_WEBHOOK_CONFIG", "{dsl}");
        }
        let err = compile("| trigger.room --auth sometimes").err().expect("refused");
        assert_eq!(err.code, "FW_NODE_TRIGGER_ROOM_CONFIG");
    }

    #[test]
    fn a_room_reads_the_same_guard_words() {
        let compiled = compile("| trigger.room --room lobby --event move --auth jwt --credential jwt_main --role player").expect("compiles");
        let spec = &compiled.ws_triggers[0];
        assert_eq!((spec.room.as_str(), spec.event.as_str()), ("lobby", "move"));
        assert_eq!((spec.auth_type.as_str(), spec.auth_credential.as_str()), ("jwt", "jwt_main"));
        assert_eq!(spec.auth_required_role, vec!["player".to_string()]);
    }

    #[test]
    fn a_socket_reads_durations_and_attempts() {
        let compiled = compile("| trigger.socket --url wss://feed.example.com/ticks --delay 2s --heartbeat 15s --max-attempts 4 --parse text").expect("compiles");
        let spec = &compiled.ws_client_triggers[0];
        assert_eq!((spec.delay_ms, spec.heartbeat_ms, spec.max_attempts), (2000, 15000, Some(4)));
        assert_eq!(spec.parse, "text");
        let spec = compile("| trigger.socket --url wss://feed.example.com/ticks").expect("compiles").ws_client_triggers[0].clone();
        assert_eq!((spec.delay_ms, spec.heartbeat_ms, spec.max_attempts), (5000, 30000, None), "omitted: unlimited");
        let err = compile("| trigger.socket --url wss://feed.example.com/ticks --delay soon").err().expect("refused");
        assert_eq!(err.code, "FW_NODE_TRIGGER_SOCKET_CONFIG");
    }

    #[test]
    fn mcp_topic_and_error_triggers_register_under_their_new_words() {
        let compiled = compile("| trigger.mcp --route /shop/ --name stock_lookup --description \"Stock for one SKU.\" --parameter sku:string! \"The SKU.\" --auth api_key --credential shop-key").expect("compiles");
        let spec = &compiled.mcp_triggers[0];
        assert_eq!((spec.route.as_str(), spec.tool_name.as_str(), spec.tool_description.as_str()), ("/shop", "stock_lookup", "Stock for one SKU."));
        assert_eq!(spec.input_schema["properties"]["sku"]["type"], "string");
        assert_eq!(spec.input_schema["required"], serde_json::json!(["sku"]));
        assert_eq!((spec.auth_type.as_str(), spec.auth_credential.as_str()), ("api_key", "shop-key"));

        let compiled = compile("| trigger.topic --topic order.placed").expect("compiles");
        assert_eq!(compiled.kv_subscribe_triggers[0].channel, "order.placed");

        let compiled = compile("| trigger.error --status 404").expect("compiles");
        assert_eq!(compiled.weberror_triggers[0].code, "404");
        let compiled = compile("| trigger.error --status 5xx").expect("compiles");
        assert_eq!(compiled.weberror_triggers[0].code, "5xx");
    }

    /// Publishing is never implicit: `--auth` is required, a guard needs its
    /// credential, a role needs jwt, and a route is a plain path.
    #[test]
    fn a_published_tool_is_refused_at_activation_without_a_clear_guard() {
        for dsl in [
            "| trigger.mcp --route /shop --name search",
            "| trigger.mcp --route /shop --name search --auth api_key",
            "| trigger.mcp --route /shop --name search --auth none --role admin",
            "| trigger.mcp --route /shop/:id --name search --auth none",
            "| trigger.mcp --route /shop --name \"two words\" --auth none",
        ] {
            assert!(compile(dsl).is_err(), "{dsl} must be refused");
        }
        let err = compile("| trigger.mcp --route /shop --name search --auth hmac --credential k").err().expect("refused");
        assert_eq!(err.code, "FW_NODE_TRIGGER_MCP_CONFIG", "hmac is not a word a published route takes");
        let open = compile("| trigger.mcp --route /shop --name search --auth none").expect("none is written, so it publishes");
        assert_eq!(open.mcp_triggers[0].auth_type, "none");
        assert!(open.webhook_triggers.is_empty(), "a published tool is never a webhook route");
    }

    #[test]
    fn a_function_declares_its_parameters_and_result_under_the_new_keys() {
        let graph = build_pipeline_graph(
            "fn",
            "| trigger.function --description \"Find one user.\" --parameter email:string! \"Address.\" --result user:object",
        )
        .expect("graph");
        let config = &graph.nodes[0].config;
        let schema = crate::pipeline::nodes::basic::trigger::function::input_schema_from_config(config);
        assert_eq!(schema["required"], serde_json::json!(["email"]));
        let result = crate::pipeline::nodes::basic::trigger::function::output_schema_from_config(config);
        assert!(result["properties"]["user"].is_object(), "{result}");
    }
}
