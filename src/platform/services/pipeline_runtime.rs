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
    /// A fixed room's name, or a pattern of opened rooms (`match-:id`, `zone-*`).
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
    /// `--event $tick --every … --max-inputs …`: the tick of the rooms it matches.
    #[serde(default)]
    pub tick: Option<crate::infra::transport::ws::TickSpec>,
    /// `--state`: the paths of the room's state each run gets.
    #[serde(default)]
    pub state: Vec<String>,
}

impl WsTriggerSpec {
    /// The join rule this trigger declares for its rooms, compared across
    /// every trigger that can match the same room: `(auth, credential, roles)`.
    pub fn join_rule(&self) -> (String, String, Vec<String>) {
        let auth = if self.auth_type.is_empty() { "none".to_string() } else { self.auth_type.clone() };
        let mut roles = self.auth_required_role.clone();
        roles.sort();
        (auth, self.auth_credential.clone(), roles)
    }

    /// The join rule as a refusal names it: `jwt (jwt_main, roles a b)`.
    pub fn describe_join_rule(&self) -> String {
        let (auth, credential, roles) = self.join_rule();
        match (credential.is_empty(), roles.is_empty()) {
            (true, _) => auth,
            (false, true) => format!("{auth} ({credential})"),
            (false, false) => format!("{auth} ({credential}, roles {})", roles.join(" ")),
        }
    }
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
    /// `none`, `jwt`, `api_key` or `oauth` — the same on every tool of a route.
    pub auth_type: String,
    #[serde(default)]
    pub auth_credential: String,
    #[serde(default)]
    pub auth_required_role: Vec<String>,
    /// With `oauth`: the app's login path (`--login`), on its pages.
    #[serde(default)]
    pub login: String,
    /// `show` or `hide`; empty means the project decides.
    #[serde(default)]
    pub errors: String,
}

impl McpTriggerSpec {
    /// The guard of the route, compared across its tools:
    /// `(auth, credential, roles, login)`.
    pub fn guard(&self) -> (String, String, Vec<String>, String) {
        let mut roles = self.auth_required_role.clone();
        roles.sort();
        (self.auth_type.clone(), self.auth_credential.clone(), roles, self.login.clone())
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
    /// The pipeline compiled for its runs (`engines/basic/compiled.rs`),
    /// made on its first run. A new activation is a new entry, so a new
    /// lane: nothing compiled from the old graph outlives it.
    #[serde(skip)]
    pub lane: Arc<crate::pipeline::engines::basic::CompiledLane>,
    /// The graph a webhook run takes with the project's RWE options applied,
    /// made once per configuration generation (`web::run_graph`). Like the
    /// lane, it goes with the entry.
    #[serde(skip)]
    pub run_graph: Arc<PreparedGraph>,
}

/// A graph prepared from an active entry's, and the configuration
/// generation it was prepared at.
#[derive(Default)]
pub struct PreparedGraph(arc_swap::ArcSwapOption<(u64, Arc<PipelineGraph>)>);

impl std::fmt::Debug for PreparedGraph {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(if self.0.load().is_some() { "prepared" } else { "not prepared" })
    }
}

impl PreparedGraph {
    /// The graph prepared at `generation`, made by `make` when there is
    /// none for it.
    pub fn get_or_make<E>(&self, generation: u64, make: impl FnOnce() -> Result<PipelineGraph, E>) -> Result<Arc<PipelineGraph>, E> {
        if let Some(found) = self.0.load().as_deref()
            && found.0 == generation
        {
            return Ok(found.1.clone());
        }
        let graph = Arc::new(make()?);
        self.0.store(Some(Arc::new((generation, graph.clone()))));
        Ok(graph)
    }
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
                    // The default the save check applies too (`webhook::DEFAULT_METHOD`).
                    let method = crate::pipeline::nodes::basic::trigger::webhook::method_of(&node.config);
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
                    if let Err(reason) = crate::infra::scheduler::schedule_of(&cron, &timezone) {
                        return Err(PlatformError::new(
                            "PIPELINE_SCHEDULE_INVALID",
                            format!("node '{}' in pipeline '{}': {reason}", node.id, meta.file_rel_path),
                        ));
                    }
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
                    let tick = crate::pipeline::nodes::basic::ws::trigger::tick_of(&node.config)
                        .map_err(|err| trigger_refused(meta, node, err))?;
                    let state = crate::pipeline::nodes::basic::ws::trigger::state_of(&node.config)
                        .map_err(|err| trigger_refused(meta, node, err))?;
                    ws_triggers.push(WsTriggerSpec {
                        node_id: node.id.clone(),
                        room,
                        event,
                        auth_type,
                        auth_credential,
                        auth_required_role,
                        tick,
                        state,
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
            lane: Arc::default(),
            run_graph: Arc::default(),
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
        crate::pipeline::nodes::basic::ws::trigger::CONFIG_CODE
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
    // `--room` is a fixed room or a pattern of opened rooms; `*` alone is
    // every room, which no trigger declares (`rooms.md`).
    if node.kind == "trigger.room"
        && let Err(why) = crate::infra::transport::ws::RoomPattern::parse(&text("room"))
    {
        return Err(refuse(crate::pipeline::PipelineError::new(code, why)));
    }
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
        choice(&text("method"), &webhook::METHODS, webhook::DEFAULT_METHOD, "--method", code).map_err(refuse)?;
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
    // `/_oauth/…` under a route is its sign-in; a `_` segment is the platform's.
    if raw_route.split('/').any(|segment| segment.starts_with('_')) {
        return Err(refuse(format!("--route '{raw_route}': a segment starting with `_` is reserved (a route's `/_oauth/…` is its sign-in)")));
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
    if !auth_required_role.is_empty() && !matches!(auth.as_str(), "jwt" | "oauth") {
        return Err(refuse("--role checks a token's roles: it needs --auth jwt or oauth".to_string()));
    }
    let login = text("login");
    match (auth.as_str(), login.as_str()) {
        ("oauth", "") => {
            return Err(refuse("--auth oauth needs --login: the app's own login path, e.g. --login /auth/login".to_string()));
        }
        ("oauth", path) if !valid_login_path(path) => {
            return Err(refuse(format!(
                "--login '{path}' must be a path of the app's pages: starting with / but not /_, without ? # : or .. — never a URL"
            )));
        }
        ("oauth", _) | (_, "") => {}
        (other, _) => return Err(refuse(format!("--login is where --auth oauth signs people in; --auth {other} takes none"))),
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
        login,
        errors: errors.to_string(),
    })
}

/// A `--login` path: on the app's pages, starting with `/` but not `/_`
/// (the platform's), no query, fragment, scheme or `..`.
fn valid_login_path(path: &str) -> bool {
    path.starts_with('/')
        && !path.starts_with("/_")
        && !path.starts_with("//")
        && !path.contains(['?', '#', ':', ' ', '\\'])
        && !path.contains("..")
}

/// Production runtime registry for activated pipelines.
pub struct PipelineRuntimeService {
    projects: Arc<ProjectService>,
    node_registry: Arc<NodeRegistryService>,
    /// Told which rooms each project's active `trigger.room` nodes declare
    /// whenever the active set changes (`rooms.md`: a fixed room exists
    /// while its trigger is active).
    ws_hub: Arc<crate::infra::transport::ws::WsHub>,
    /// Each entry behind its own `Arc`: a request matching routes takes the
    /// project's pipelines without copying a single graph.
    inner: ArcSwap<HashMap<ActivePipelineKey, Arc<CompiledPipeline>>>,
    /// The same entries by owner, then project, in path order, made when the
    /// active set changes: a request reads its own project's pipelines, never
    /// every project's.
    by_project: ArcSwap<ProjectIndex>,
}

/// [`PipelineRuntimeService::by_project`].
type ProjectIndex = HashMap<String, HashMap<String, Arc<ProjectActive>>>;

/// One project's active set, as its requests read it.
pub struct ProjectActive {
    /// Its active pipelines, in path order.
    pub pipelines: Arc<[Arc<CompiledPipeline>]>,
    /// Its webhook routes: a request finds its own among them without
    /// comparing its path with every route of the project.
    webhooks: WebhookRoutes,
}

impl ProjectActive {
    /// See [`PipelineRuntimeService::webhook_matches`].
    pub fn webhook_matches(&self, method: &str, path: &str) -> Vec<WebhookMatch> {
        let routes = &self.webhooks;
        let methods: &[&str] = if method == "HEAD" { &["HEAD", "GET"] } else { std::slice::from_ref(&method) };
        let key = webhook_segments(path).collect::<Vec<_>>().join("/");
        let exact: Vec<WebhookMatch> = methods
            .iter()
            .filter_map(|m| routes.exact.get(*m).and_then(|by_path| by_path.get(&key)))
            .flatten()
            .filter_map(|(compiled, i)| {
                let path = match_webhook_path(&compiled.webhook_triggers[*i].path, path)?;
                Some(WebhookMatch { compiled: compiled.clone(), trigger: *i, path })
            })
            .collect();
        if !exact.is_empty() {
            return exact;
        }
        routes
            .patterns
            .iter()
            .filter(|(compiled, i)| {
                let trigger_method = &compiled.webhook_triggers[*i].method;
                methods.iter().any(|m| trigger_method.eq_ignore_ascii_case(m))
            })
            .filter_map(|(compiled, i)| {
                let path = match_webhook_path(&compiled.webhook_triggers[*i].path, path)?;
                Some(WebhookMatch { compiled: compiled.clone(), trigger: *i, path })
            })
            .collect()
    }
}

/// One project's webhook routes, by what a request can find them by.
#[derive(Default)]
struct WebhookRoutes {
    /// Routes with no parameter, by method (upper case), then their
    /// segments joined by `/`.
    exact: HashMap<String, HashMap<String, Vec<(Arc<CompiledPipeline>, usize)>>>,
    /// Routes with a parameter, matched one by one.
    patterns: Vec<(Arc<CompiledPipeline>, usize)>,
}

/// A webhook trigger that answers a request: its pipeline, which of the
/// pipeline's webhook triggers, and how its path matched.
pub struct WebhookMatch {
    pub compiled: Arc<CompiledPipeline>,
    pub trigger: usize,
    pub path: WebhookPathMatch,
}

fn index_by_project(all: &HashMap<ActivePipelineKey, Arc<CompiledPipeline>>) -> ProjectIndex {
    let mut grouped: HashMap<String, HashMap<String, Vec<Arc<CompiledPipeline>>>> = HashMap::new();
    for compiled in all.values() {
        grouped.entry(compiled.owner.clone()).or_default().entry(compiled.project.clone()).or_default().push(compiled.clone());
    }
    grouped
        .into_iter()
        .map(|(owner, projects)| {
            let projects = projects
                .into_iter()
                .map(|(project, mut list)| {
                    list.sort_by(|a, b| a.file_rel_path.cmp(&b.file_rel_path));
                    let mut webhooks = WebhookRoutes::default();
                    for compiled in &list {
                        for (i, trigger) in compiled.webhook_triggers.iter().enumerate() {
                            if webhook_segments(&trigger.path).any(|segment| path_param_name(segment).is_some()) {
                                webhooks.patterns.push((compiled.clone(), i));
                            } else {
                                webhooks
                                    .exact
                                    .entry(trigger.method.to_ascii_uppercase())
                                    .or_default()
                                    .entry(webhook_segments(&trigger.path).collect::<Vec<_>>().join("/"))
                                    .or_default()
                                    .push((compiled.clone(), i));
                            }
                        }
                    }
                    (project, Arc::new(ProjectActive { pipelines: Arc::from(list), webhooks }))
                })
                .collect();
            (owner, projects)
        })
        .collect()
}

/// How a webhook route matched a request path: its parameters and how
/// specific it is, for ranking routes that both match.
#[derive(Debug)]
pub struct WebhookPathMatch {
    pub params: serde_json::Map<String, serde_json::Value>,
    pub static_segments: usize,
    pub dynamic_segments: usize,
    pub total_segments: usize,
}

pub fn match_webhook_path(pattern: &str, actual: &str) -> Option<WebhookPathMatch> {
    let pattern_segments = webhook_segments(pattern);
    let total_segments = pattern_segments.clone().count();
    if total_segments != webhook_segments(actual).count() {
        return None;
    }

    let mut params = serde_json::Map::new();
    let mut static_segments = 0usize;
    let mut dynamic_segments = 0usize;

    for (pattern_seg, actual_seg) in pattern_segments.zip(webhook_segments(actual)) {
        if let Some(name) = path_param_name(pattern_seg) {
            dynamic_segments += 1;
            params.insert(name.to_string(), serde_json::Value::String(actual_seg.to_string()));
            continue;
        }
        if pattern_seg == actual_seg {
            static_segments += 1;
            continue;
        }
        return None;
    }

    Some(WebhookPathMatch {
        params,
        static_segments,
        dynamic_segments,
        total_segments,
    })
}

/// A webhook path's segments, read where they sit: the pieces between
/// slashes, empty ones skipped. Every request matches every webhook route
/// of its project, so nothing is allocated for it.
fn webhook_segments(path: &str) -> impl Iterator<Item = &str> + Clone {
    path.trim().split('/').filter(|segment| !segment.is_empty())
}

#[cfg(test)]
mod webhook_segment_tests {
    /// The segments read in place are the ones the path once gave after
    /// being normalized (one leading slash, none trailing) and split.
    #[test]
    fn segments_read_in_place_are_the_normalized_ones() {
        let normalized = |raw: &str| -> Vec<String> {
            let raw = raw.trim();
            let path = if raw.is_empty() || raw == "/" { "/".to_string() } else { format!("/{}", raw.trim_matches('/')) };
            if path == "/" {
                return Vec::new();
            }
            path.trim_start_matches('/').split('/').filter(|s| !s.is_empty()).map(String::from).collect()
        };
        for raw in ["", "/", "//", " / ", "a", "/a", "a/", "/a/b", "a//b", "//a//b//", " /bench/one ", "/{id}/x", "/:id/", "/a b/c"] {
            let read: Vec<String> = super::webhook_segments(raw).map(String::from).collect();
            assert_eq!(read, normalized(raw), "'{raw}'");
        }
        let hit = super::match_webhook_path("/orders/{id}/items", "orders/42/items/").expect("matches");
        assert_eq!((hit.static_segments, hit.dynamic_segments, hit.total_segments), (2, 1, 3));
        assert_eq!(hit.params["id"], "42");
        assert!(super::match_webhook_path("/orders/{id}", "/orders/42/items").is_none());
    }
}

fn path_param_name(segment: &str) -> Option<&str> {
    // Support {name} style
    if let Some(inner) = segment.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
        let name = inner.trim();
        if !name.is_empty() && !name.contains('/') && !name.contains('{') && !name.contains('}') {
            return Some(name);
        }
    }
    // Support :name style
    if let Some(name) = segment.strip_prefix(':') {
        let name = name.trim();
        if !name.is_empty() && !name.contains('/') && !name.contains(':') {
            return Some(name);
        }
    }
    None
}



impl PipelineRuntimeService {
    pub fn new(
        projects: Arc<ProjectService>,
        node_registry: Arc<NodeRegistryService>,
        ws_hub: Arc<crate::infra::transport::ws::WsHub>,
    ) -> Self {
        Self {
            projects,
            node_registry,
            ws_hub,
            inner: ArcSwap::new(Arc::new(HashMap::new())),
            by_project: ArcSwap::new(Arc::new(HashMap::new())),
        }
    }

    /// Replace the active set and tell the room hub what one project's
    /// `trigger.room` nodes now declare, ticks included.
    fn store(&self, next: HashMap<ActivePipelineKey, Arc<CompiledPipeline>>, owner: &str, project: &str) {
        use crate::infra::transport::ws::RoomDecl;
        let mut rooms: Vec<RoomDecl> = next
            .values()
            .filter(|compiled| compiled.owner == owner && compiled.project == project)
            .flat_map(|compiled| {
                compiled.ws_triggers.iter().map(|t| RoomDecl { tick: t.tick, ..RoomDecl::new(&t.room, &compiled.file_rel_path) })
            })
            .collect();
        // By room and path, so a fixed room names the same pipeline each time.
        rooms.sort_by(|a, b| (&a.room, &a.pipeline).cmp(&(&b.room, &b.pipeline)));
        self.by_project.store(Arc::new(index_by_project(&next)));
        self.inner.store(Arc::new(next));
        self.ws_hub.declare(owner, project, &rooms);
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
            next.insert(compiled.key.clone(), Arc::new(compiled));
        }

        self.store(next, &owner, &project);
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
            next.insert(compiled.key.clone(), Arc::new(compiled));
        }

        self.store(next, &owner, &project);
        Ok(())
    }

    pub fn get(&self, owner: &str, project: &str, file_rel_path: &str) -> Option<Arc<CompiledPipeline>> {
        let key = active_pipeline_key(owner, project, file_rel_path);
        self.inner.load().get(&key).cloned()
    }

    /// Removes one pipeline from the active runtime registry without refreshing from disk.
    pub fn evict(&self, owner: &str, project: &str, file_rel_path: &str) {
        let key = active_pipeline_key(owner, project, file_rel_path);
        let mut next = (*self.inner.load_full()).clone();
        next.remove(&key);
        let owner = crate::platform::model::slug_segment(owner);
        let project = crate::platform::model::slug_segment(project);
        self.store(next, &owner, &project);
    }

    /// One project's active set.
    fn project_active(&self, owner: &str, project: &str) -> Option<Arc<ProjectActive>> {
        let index = self.by_project.load();
        let of = |owner: &str, project: &str| index.get(owner).and_then(|projects| projects.get(project)).cloned();
        // Names arrive as slugs already; only one that is not is made one.
        of(owner, project).or_else(|| of(&crate::platform::model::slug_segment(owner), &crate::platform::model::slug_segment(project)))
    }

    /// One project's active pipelines, in path order.
    pub fn list_project(&self, owner: &str, project: &str) -> Arc<[Arc<CompiledPipeline>]> {
        self.project_active(owner, project).map(|active| active.pipelines.clone()).unwrap_or_else(|| Arc::from(Vec::new()))
    }

    /// The webhook triggers of one project that answer `method` (upper
    /// case) on `path`. A route with no parameter whose path is the request's
    /// outranks every route with one (it has every segment static), so when
    /// there is such a route only those are answered; else every route with
    /// a parameter that matches. A `GET` route answers `HEAD` too.
    pub fn webhook_matches(&self, owner: &str, project: &str, method: &str, path: &str) -> Vec<WebhookMatch> {
        self.project_active(owner, project).map(|active| active.webhook_matches(method, path)).unwrap_or_default()
    }

    /// Returns all active compiled pipelines across every owner/project.
    pub fn list_all(&self) -> Vec<Arc<CompiledPipeline>> {
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

    /// The route index picks the route that matching every route and
    /// ranking them picks: most static segments, then fewest parameters,
    /// then the pipeline's path; a GET route answers HEAD.
    #[test]
    fn the_route_index_picks_what_ranking_every_route_picks() {
        use std::collections::HashMap;
        use std::sync::Arc;

        use super::{active_pipeline_key, index_by_project, match_webhook_path};

        let routes = [
            ("pipelines/a.zf.json", "/orders/{id} --method POST"),
            ("pipelines/b.zf.json", "/orders/today --method POST"),
            ("pipelines/c.zf.json", "/orders/today --method POST"),
            ("pipelines/d.zf.json", "/:kind/today --method POST"),
            ("pipelines/e.zf.json", "/orders/{id}/items --method GET"),
            ("pipelines/f.zf.json", "/orders/7/items --method GET"),
            ("pipelines/g.zf.json", "/ --method GET"),
            ("pipelines/h.zf.json", "/health"),
        ];
        let mut all = HashMap::new();
        for (file, route) in routes {
            let mut compiled = compile(&format!("| trigger.webhook --route {route}")).expect("compiles");
            compiled.file_rel_path = file.to_string();
            compiled.key = active_pipeline_key("demo", "site-a", file);
            all.insert(compiled.key.clone(), Arc::new(compiled));
        }
        let index = index_by_project(&all);
        let active = &index["demo"]["site-a"];
        let rank = |found: Vec<(String, usize, usize, usize)>| {
            let mut found = found;
            found.sort_by(|a, b| b.1.cmp(&a.1).then(a.2.cmp(&b.2)).then(b.3.cmp(&a.3)).then(a.0.cmp(&b.0)));
            found.into_iter().next().map(|f| f.0)
        };
        for (method, path) in [
            ("POST", "/orders/today"),
            ("POST", "orders/42/"),
            ("POST", "/things/today"),
            ("GET", "/orders/7/items"),
            ("HEAD", "/orders/7/items"),
            ("HEAD", "/orders/8/items"),
            ("GET", "/"),
            ("HEAD", "/health"),
            ("POST", "/health"),
            ("DELETE", "/orders/today"),
        ] {
            let mut every = Vec::new();
            for compiled in all.values() {
                for trigger in &compiled.webhook_triggers {
                    let answers = trigger.method.eq_ignore_ascii_case(method) || (method == "HEAD" && trigger.method.eq_ignore_ascii_case("GET"));
                    if let (true, Some(m)) = (answers, match_webhook_path(&trigger.path, path)) {
                        every.push((compiled.file_rel_path.clone(), m.static_segments, m.dynamic_segments, m.total_segments));
                    }
                }
            }
            let indexed = active
                .webhook_matches(method, path)
                .into_iter()
                .map(|f| (f.compiled.file_rel_path.clone(), f.path.static_segments, f.path.dynamic_segments, f.path.total_segments))
                .collect();
            assert_eq!(rank(indexed), rank(every), "{method} {path}");
        }
    }

    /// No `--method` is GET — the word the save check applies, not a second
    /// default of the route table's own.
    #[test]
    fn a_webhook_without_a_method_serves_get() {
        let compiled = compile("| trigger.webhook --route /no-method").expect("compiles");
        assert_eq!(compiled.webhook_triggers[0].method, "GET");
        assert_eq!(crate::platform::services::project::canonical_webhook_method(None), "GET");
        let lower = compile("| trigger.webhook --route /lower --method post").expect("compiles");
        assert_eq!(lower.webhook_triggers[0].method, "POST");
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
        let err = compile("| trigger.room --room lobby --auth sometimes").err().expect("refused");
        assert_eq!(err.code, "FW_NODE_TRIGGER_ROOM_CONFIG");
        for dsl in ["| trigger.room --event move", r#"| trigger.room --room "   " --event move"#] {
            let missing_room = compile(dsl).err().expect("missing --room is refused");
            assert_eq!(missing_room.code, "FW_NODE_TRIGGER_ROOM_CONFIG", "{dsl}");
            assert!(missing_room.message.contains("--room is required"), "{dsl}: {}", missing_room.message);
        }
        // `*` alone is every room, not a room; a pattern with a literal part is.
        for dsl in [r#"| trigger.room --room "*""#, r#"| trigger.room --room ":id""#, r#"| trigger.room --room "a b""#] {
            let err = compile(dsl).err().unwrap_or_else(|| panic!("accepted: {dsl}"));
            assert_eq!(err.code, "FW_NODE_TRIGGER_ROOM_CONFIG", "{dsl}");
        }
        for dsl in [r#"| trigger.room --room "zone-*""#, r#"| trigger.room --room "match-:id""#, "| trigger.room --room lobby"] {
            assert!(compile(dsl).is_ok(), "{dsl}");
        }
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

    /// `oauth` names its credential and the app's login path; `--login` is a
    /// path of the app's pages and belongs to `oauth` alone; `_` segments of a
    /// route are reserved for its `/_oauth/…`.
    #[test]
    fn an_oauth_route_names_its_login_and_reserves_underscore_segments() {
        let spec = compile("| trigger.mcp --route /library --name search --auth oauth --credential library-jwt --login /auth/login --role reader")
            .expect("compiles")
            .mcp_triggers[0]
            .clone();
        assert_eq!((spec.auth_type.as_str(), spec.login.as_str()), ("oauth", "/auth/login"));
        assert_eq!(spec.auth_required_role, vec!["reader".to_string()]);
        for dsl in [
            "| trigger.mcp --route /library --name search --auth oauth --credential library-jwt",
            "| trigger.mcp --route /library --name search --auth oauth --login /auth/login",
            "| trigger.mcp --route /library --name search --auth oauth --credential k --login https://evil.example/login",
            "| trigger.mcp --route /library --name search --auth oauth --credential k --login /_mcp/x",
            "| trigger.mcp --route /library --name search --auth oauth --credential k --login //evil.example",
            "| trigger.mcp --route /library --name search --auth oauth --credential k --login /a/../b",
            "| trigger.mcp --route /library --name search --auth oauth --credential k --login /login?next=x",
            "| trigger.mcp --route /library --name search --auth api_key --credential k --login /auth/login",
            "| trigger.mcp --route /library/_oauth --name search --auth none",
            "| trigger.mcp --route /_private --name search --auth none",
        ] {
            let err = compile(dsl).err().unwrap_or_else(|| panic!("accepted: {dsl}"));
            assert_eq!(err.code, "FW_NODE_TRIGGER_MCP_CONFIG", "{dsl}");
        }
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
