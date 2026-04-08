//! Real framework engine with graph traversal and built-in node dispatch.

use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::pipeline::expr::{resolve_config_expressions, scanner::scan as scan_exprs};
use crate::pipeline::interface::PipelineEngine;
use crate::pipeline::model::{
    ExecuteOptions, NodeTraceEntry, PipelineContext, PipelineError, PipelineOutput, PipelineGraph, PipelineNode,
};
use crate::pipeline::nodes::basic::{
    agent, auth_token_create, browser_run, crypto, file_save, function_call, http_request,
    img_thumbnail, logic, mem_del, mem_exists, mem_expire, mem_get, mem_incr, mem_publish, mem_set, pg_query, script,
    sqlite_mutate, sqlite_query,
    trigger::{function as trigger_function, manual, memsubscribe, schedule, webhook, weberror},
    web_response, ws_emit, ws_sync_state, ws_trigger,
};
use crate::platform::services::PlatformService;
use crate::pipeline::nodes::{NodeHandler, NodeExecutionInput, NodeExecutionOutput};
use crate::language::{DenoSandboxEngine, LanguageEngine};
use crate::platform::services::CredentialService;
use crate::rwe::{ReactiveWebEngine, TemplateSource, resolve_engine_or_default};
use crate::infra::mem::MemHub;
use crate::infra::transport::ws::WsHub;

/// A single entry in the template compile cache.
/// Pairs the compiled page artifact with the set of component files it depends on,
/// enabling dependency-aware eviction when a component is edited.
pub struct CacheEntry {
    /// The compiled page artifact.
    pub page: Arc<web_response::CompiledPage>,
    /// Absolute filesystem paths of all component files inlined during compilation.
    /// Populated from `CompiledTemplate.dependency_paths` (the `visited` set of
    /// `collect_inlined_module`). When any of these paths change on disk,
    /// this entry is evicted so the next request recompiles with the updated component.
    pub dependencies: std::collections::HashSet<String>,
}

/// In-memory compile cache for template nodes.
///
/// Key: hash of the entry-page markup string.
/// Value: compiled page + dependency set for targeted eviction.
///
/// Entry-page changes → new hash → automatic cache miss.
/// Component changes → `evict_template_cache_by_path` removes affected entries.
///
/// Uses `RwLock` so concurrent reads (cache hits) never block each other;
/// only cache-miss writes take an exclusive lock.
pub type TemplateCache = Arc<RwLock<HashMap<u64, CacheEntry>>>;

/// Create a new empty template compile cache.
pub fn new_template_cache() -> TemplateCache {
    Arc::new(RwLock::new(HashMap::new()))
}

/// Evict all cache entries whose dependency set includes `abs_path`.
/// Called whenever any template or component file is written (UI, MCP, API).
/// Entry-page saves don't need this — they already cause a hash miss automatically.
pub fn evict_template_cache_by_path(cache: &TemplateCache, abs_path: &str) {
    cache
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .retain(|_, entry| !entry.dependencies.contains(abs_path));
}

fn hash_markup(s: &str) -> u64 {
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

fn take_private_tokens(payload: &mut Value, key: &str) -> Vec<String> {
    let Some(map) = payload.as_object_mut() else {
        return Vec::new();
    };
    let Some(Value::Array(items)) = map.remove(key) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in items {
        let Some(text) = item.as_str() else {
            continue;
        };
        let trimmed = text.trim();
        if trimmed.is_empty() || out.iter().any(|existing| existing == trimmed) {
            continue;
        }
        out.push(trimmed.to_string());
    }
    out
}

fn take_private_paths(payload: &mut Value, key: &str) -> Vec<Vec<String>> {
    let Some(map) = payload.as_object_mut() else {
        return Vec::new();
    };
    let Some(Value::Array(items)) = map.remove(key) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in items {
        let parts = match item {
            Value::String(path) => path
                .split('.')
                .map(str::trim)
                .filter(|segment| !segment.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>(),
            Value::Array(segments) => segments
                .into_iter()
                .filter_map(|segment| segment.as_str().map(str::trim).map(ToString::to_string))
                .filter(|segment| !segment.is_empty())
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        };
        if parts.is_empty() || out.iter().any(|existing| existing == &parts) {
            continue;
        }
        out.push(parts);
    }
    out
}

fn take_private_redact_tokens(payload: &mut Value) -> Vec<String> {
    take_private_tokens(payload, "__zf_private_redact")
}

fn take_private_trace_redact_tokens(payload: &mut Value) -> Vec<String> {
    take_private_tokens(payload, "__zf_private_trace_redact")
}

fn take_private_redact_except_paths(payload: &mut Value) -> Vec<Vec<String>> {
    take_private_paths(payload, "__zf_private_redact_except_paths")
}

fn take_private_trace_redact_except_paths(payload: &mut Value) -> Vec<Vec<String>> {
    take_private_paths(payload, "__zf_private_trace_redact_except_paths")
}

fn extend_unique_strings(target: &mut Vec<String>, extra: Vec<String>) {
    for item in extra {
        if target.iter().any(|existing| existing == &item) {
            continue;
        }
        target.push(item);
    }
}

fn extend_unique_paths(target: &mut Vec<Vec<String>>, extra: Vec<Vec<String>>) {
    for item in extra {
        if target.iter().any(|existing| existing == &item) {
            continue;
        }
        target.push(item);
    }
}

fn redact_string(value: &str, tokens: &[String]) -> String {
    let mut out = value.to_string();
    for token in tokens {
        if token.is_empty() {
            continue;
        }
        out = out.replace(token, "••••••");
    }
    out
}

fn redact_json_value(
    value: &Value,
    tokens: &[String],
    except_paths: &[Vec<String>],
    current_path: &[String],
) -> Value {
    if except_paths.iter().any(|path| path == current_path) {
        return value.clone();
    }
    match value {
        Value::String(text) => Value::String(redact_string(text, tokens)),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| redact_json_value(item, tokens, except_paths, current_path))
                .collect(),
        ),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, item)| {
                    let mut next_path = current_path.to_vec();
                    next_path.push(key.clone());
                    (
                        key.clone(),
                        redact_json_value(item, tokens, except_paths, &next_path),
                    )
                })
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Main framework engine used for real pipeline execution.
pub struct BasicPipelineEngine {
    language: Arc<dyn LanguageEngine>,
    rwe: Arc<dyn ReactiveWebEngine>,
    credentials: Option<Arc<CredentialService>>,
    template_cache: Option<TemplateCache>,
    ws_hub: Option<Arc<WsHub>>,
    mem_hub: Option<Arc<MemHub>>,
    platform: Option<Arc<PlatformService>>,
    /// Filesystem root for resolving `@/` alias imports in TSX templates.
    template_root: Option<std::path::PathBuf>,
    /// Platform data root — used by SQLite nodes to locate the project DB.
    data_root: Option<std::path::PathBuf>,
}

impl Default for BasicPipelineEngine {
    fn default() -> Self {
        let rwe_engine_id = std::env::var("ZEBFLOW_RWE_ENGINE_ID").ok();
        Self {
            language: Arc::new(DenoSandboxEngine::default()),
            rwe: resolve_engine_or_default(rwe_engine_id.as_deref()),
            credentials: None,
            template_cache: None,
            ws_hub: None,
            mem_hub: None,
            platform: None,
            template_root: None,
            data_root: None,
        }
    }
}

impl BasicPipelineEngine {
    pub fn new(
        language: Arc<dyn LanguageEngine>,
        rwe: Arc<dyn ReactiveWebEngine>,
        credentials: Option<Arc<CredentialService>>,
    ) -> Self {
        Self {
            language,
            rwe,
            credentials,
            template_cache: None,
            ws_hub: None,
            mem_hub: None,
            platform: None,
            template_root: None,
            data_root: None,
        }
    }

    /// Set the template root so `@/` alias imports resolve correctly in TSX templates.
    pub fn with_template_root(mut self, root: Option<std::path::PathBuf>) -> Self {
        self.template_root = root;
        self
    }

    /// Attach a shared template compile cache to this engine.
    /// Same cache instance should be passed on every request so hits accumulate.
    pub fn with_template_cache(mut self, cache: TemplateCache) -> Self {
        self.template_cache = Some(cache);
        self
    }

    /// Attach the WS hub so ws_sync_state and ws_emit nodes can access rooms.
    pub fn with_ws_hub(mut self, hub: Arc<WsHub>) -> Self {
        self.ws_hub = Some(hub);
        self
    }

    /// Attach the mem hub so n.mem.* nodes can access the per-project KV store.
    pub fn with_mem_hub(mut self, hub: Arc<MemHub>) -> Self {
        self.mem_hub = Some(hub);
        self
    }

    /// Attach the platform service so n.function.call nodes can invoke sub-pipelines.
    pub fn with_platform(mut self, platform: Arc<PlatformService>) -> Self {
        self.platform = Some(platform);
        self
    }

    /// Attach the platform data root so SQLite nodes can locate the project DB.
    pub fn with_data_root(mut self, root: std::path::PathBuf) -> Self {
        self.data_root = Some(root);
        self
    }

    fn build_node(&self, node: &PipelineNode) -> Result<NodeDispatch, PipelineError> {
        match node.kind.as_str() {
            webhook::NODE_KIND => Ok(NodeDispatch::Webhook(webhook::Node::new(
                serde_json::from_value(node.config.clone()).map_err(|err| {
                    PipelineError::new("FW_NODE_WEBHOOK_CONFIG", err.to_string())
                })?,
            ))),
            schedule::NODE_KIND => Ok(NodeDispatch::Schedule(schedule::Node::new(
                serde_json::from_value(node.config.clone()).map_err(|err| {
                    PipelineError::new("FW_NODE_SCHEDULE_CONFIG", err.to_string())
                })?,
            ))),
            manual::NODE_KIND => Ok(NodeDispatch::Manual(manual::Node::new(
                serde_json::from_value(node.config.clone())
                    .map_err(|err| PipelineError::new("FW_NODE_MANUAL_CONFIG", err.to_string()))?,
            ))),
            script::NODE_KIND => Ok(NodeDispatch::Script(script::Node::new(
                &node.id,
                serde_json::from_value(node.config.clone())
                    .map_err(|err| PipelineError::new("FW_NODE_SCRIPT_CONFIG", err.to_string()))?,
                self.language.clone(),
            )?)),
            http_request::NODE_KIND => Ok(NodeDispatch::HttpRequest(http_request::Node::new(
                serde_json::from_value(node.config.clone()).map_err(|err| {
                    PipelineError::new("FW_NODE_HTTP_REQUEST_CONFIG", err.to_string())
                })?,
                self.language.clone(),
                self.credentials.clone(),
            )?)),
            sqlite_query::NODE_KIND => {
                let Some(data_root) = &self.data_root else {
                    return Err(PipelineError::new(
                        "FW_NODE_SQLITE_UNAVAILABLE",
                        "data_root is not configured on this pipeline engine",
                    ));
                };
                Ok(NodeDispatch::SqliteQuery(sqlite_query::Node::new(
                    serde_json::from_value(node.config.clone()).map_err(|err| {
                        PipelineError::new("FW_NODE_SQLITE_QUERY_CONFIG", err.to_string())
                    })?,
                    data_root.clone(),
                )?))
            }
            sqlite_mutate::NODE_KIND => {
                let Some(data_root) = &self.data_root else {
                    return Err(PipelineError::new(
                        "FW_NODE_SQLITE_UNAVAILABLE",
                        "data_root is not configured on this pipeline engine",
                    ));
                };
                Ok(NodeDispatch::SqliteMutate(sqlite_mutate::Node::new(
                    serde_json::from_value(node.config.clone()).map_err(|err| {
                        PipelineError::new("FW_NODE_SQLITE_MUTATE_CONFIG", err.to_string())
                    })?,
                    data_root.clone(),
                )?))
            }
            browser_run::NODE_KIND => {
                let Some(credentials) = &self.credentials else {
                    return Err(PipelineError::new(
                        "FW_NODE_BROWSER_RUN_UNAVAILABLE",
                        "credential service is not configured on this framework engine",
                    ));
                };
                Ok(NodeDispatch::BrowserRun(browser_run::Node::new(
                    serde_json::from_value(node.config.clone())
                        .map_err(|e| PipelineError::new("FW_NODE_BROWSER_RUN_CONFIG", e.to_string()))?,
                    credentials.clone(),
                )?))
            }
            pg_query::NODE_KIND => {
                let Some(credentials) = &self.credentials else {
                    return Err(PipelineError::new(
                        "FW_NODE_PG_UNAVAILABLE",
                        "credential service is not configured on this framework engine",
                    ));
                };
                Ok(NodeDispatch::Postgres(pg_query::Node::new(
                    serde_json::from_value(node.config.clone())
                        .map_err(|err| PipelineError::new("FW_NODE_PG_CONFIG", err.to_string()))?,
                    credentials.clone(),
                    self.language.clone(),
                )?))
            }
            web_response::NODE_KIND => {
                let config: web_response::Config = serde_json::from_value(node.config.clone())
                    .map_err(|err| {
                        PipelineError::new("FW_NODE_WEB_RESPONSE_CONFIG", err.to_string())
                    })?;
                if config.template.is_some() {
                    Ok(NodeDispatch::InlineWebResponse {
                        node_id: node.id.clone(),
                        config,
                    })
                } else {
                    Ok(NodeDispatch::WebResponse(web_response::Node::new(config)))
                }
            }
            agent::NODE_KIND => {
                let config: agent::Config =
                    serde_json::from_value(node.config.clone()).unwrap_or_default();
                Ok(NodeDispatch::Agent(agent::Node::new(config, self.credentials.clone(), self.platform.clone())))
            }
            logic::if_::NODE_KIND => Ok(NodeDispatch::LogicIf(logic::if_::Node::new(
                &node.id,
                serde_json::from_value(node.config.clone())
                    .map_err(|e| PipelineError::new("FW_NODE_LOGIC_IF_CONFIG", e.to_string()))?,
                self.language.clone(),
            )?)),
            logic::switch::NODE_KIND => Ok(NodeDispatch::LogicSwitch(logic::switch::Node::new(
                &node.id,
                serde_json::from_value(node.config.clone())
                    .map_err(|e| PipelineError::new("FW_NODE_LOGIC_SWITCH_CONFIG", e.to_string()))?,
                self.language.clone(),
            )?)),
            logic::branch::NODE_KIND => Ok(NodeDispatch::LogicBranch(logic::branch::Node::new(
                &node.id,
                serde_json::from_value(node.config.clone())
                    .map_err(|e| PipelineError::new("FW_NODE_LOGIC_BRANCH_CONFIG", e.to_string()))?,
                self.language.clone(),
            )?)),
            logic::merge::NODE_KIND => Ok(NodeDispatch::LogicMerge(logic::merge::Node::new(
                serde_json::from_value(node.config.clone())
                    .map_err(|e| PipelineError::new("FW_NODE_LOGIC_MERGE_CONFIG", e.to_string()))?,
            ))),
            auth_token_create::NODE_KIND => {
                let Some(credentials) = &self.credentials else {
                    return Err(PipelineError::new(
                        "FW_NODE_AUTH_TOKEN_UNAVAILABLE",
                        "credential service is not configured on this framework engine",
                    ));
                };
                Ok(NodeDispatch::AuthTokenCreate(auth_token_create::Node::new(
                    serde_json::from_value(node.config.clone()).map_err(|err| {
                        PipelineError::new("FW_NODE_AUTH_TOKEN_CONFIG", err.to_string())
                    })?,
                    credentials.clone(),
                )?))
            }
            weberror::NODE_KIND => Ok(NodeDispatch::WebError(weberror::Node::new(
                serde_json::from_value(node.config.clone()).map_err(|err| {
                    PipelineError::new("FW_NODE_WEBERROR_CONFIG", err.to_string())
                })?,
            ))),
            ws_trigger::NODE_KIND => Ok(NodeDispatch::WsTrigger(ws_trigger::Node::new(
                serde_json::from_value(node.config.clone()).map_err(|err| {
                    PipelineError::new("FW_NODE_WS_TRIGGER_CONFIG", err.to_string())
                })?,
            ))),
            ws_sync_state::NODE_KIND => {
                let Some(ws_hub) = &self.ws_hub else {
                    return Err(PipelineError::new(
                        "FW_NODE_WS_SYNC_STATE_UNAVAILABLE",
                        "ws hub is not configured on this framework engine",
                    ));
                };
                Ok(NodeDispatch::WsSyncState(ws_sync_state::Node::new(
                    serde_json::from_value(node.config.clone()).map_err(|err| {
                        PipelineError::new("FW_NODE_WS_SYNC_STATE_CONFIG", err.to_string())
                    })?,
                    ws_hub.clone(),
                )?))
            }
            ws_emit::NODE_KIND => {
                let Some(ws_hub) = &self.ws_hub else {
                    return Err(PipelineError::new(
                        "FW_NODE_WS_EMIT_UNAVAILABLE",
                        "ws hub is not configured on this framework engine",
                    ));
                };
                Ok(NodeDispatch::WsEmit(ws_emit::Node::new(
                    serde_json::from_value(node.config.clone()).map_err(|err| {
                        PipelineError::new("FW_NODE_WS_EMIT_CONFIG", err.to_string())
                    })?,
                    ws_hub.clone(),
                )?))
            }
            crypto::NODE_KIND => Ok(NodeDispatch::Crypto(crypto::Node::new(
                serde_json::from_value(node.config.clone())
                    .map_err(|err| PipelineError::new("FW_NODE_CRYPTO_CONFIG", err.to_string()))?,
            )?)),
            trigger_function::NODE_KIND => {
                let config: trigger_function::Config =
                    serde_json::from_value(node.config.clone()).unwrap_or_default();
                Ok(NodeDispatch::TriggerFunction(trigger_function::Node::new(config)))
            }
            function_call::NODE_KIND => {
                let config: function_call::Config =
                    serde_json::from_value(node.config.clone()).unwrap_or_default();
                Ok(NodeDispatch::FunctionCall(function_call::Node::new(
                    config,
                    self.platform.clone(),
                )))
            }
            file_save::NODE_KIND => {
                let config: file_save::Config =
                    serde_json::from_value(node.config.clone()).unwrap_or_default();
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        "FW_NODE_FILE_SAVE",
                        "platform service not available in this engine context",
                    ));
                };
                Ok(NodeDispatch::FileSave(file_save::Node::new(config, platform.clone())?))
            }
            img_thumbnail::NODE_KIND => {
                let config: img_thumbnail::Config =
                    serde_json::from_value(node.config.clone()).unwrap_or_default();
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        "IMG_THUMBNAIL",
                        "platform service not available in this engine context",
                    ));
                };
                Ok(NodeDispatch::ImgThumbnail(img_thumbnail::Node::new(config, platform.clone())?))
            }
            mem_set::NODE_KIND => {
                let Some(mem_hub) = &self.mem_hub else {
                    return Err(PipelineError::new(
                        "FW_NODE_MEM_UNAVAILABLE",
                        "mem hub is not configured on this pipeline engine",
                    ));
                };
                Ok(NodeDispatch::MemSet(mem_set::Node::new(
                    serde_json::from_value(node.config.clone())
                        .map_err(|e| PipelineError::new("FW_NODE_MEM_SET_CONFIG", e.to_string()))?,
                    mem_hub.clone(),
                )))
            }
            mem_get::NODE_KIND => {
                let Some(mem_hub) = &self.mem_hub else {
                    return Err(PipelineError::new(
                        "FW_NODE_MEM_UNAVAILABLE",
                        "mem hub is not configured on this pipeline engine",
                    ));
                };
                Ok(NodeDispatch::MemGet(mem_get::Node::new(
                    serde_json::from_value(node.config.clone())
                        .map_err(|e| PipelineError::new("FW_NODE_MEM_GET_CONFIG", e.to_string()))?,
                    mem_hub.clone(),
                )))
            }
            mem_del::NODE_KIND => {
                let Some(mem_hub) = &self.mem_hub else {
                    return Err(PipelineError::new(
                        "FW_NODE_MEM_UNAVAILABLE",
                        "mem hub is not configured on this pipeline engine",
                    ));
                };
                Ok(NodeDispatch::MemDel(mem_del::Node::new(
                    serde_json::from_value(node.config.clone())
                        .map_err(|e| PipelineError::new("FW_NODE_MEM_DEL_CONFIG", e.to_string()))?,
                    mem_hub.clone(),
                )))
            }
            mem_incr::NODE_KIND => {
                let Some(mem_hub) = &self.mem_hub else {
                    return Err(PipelineError::new(
                        "FW_NODE_MEM_UNAVAILABLE",
                        "mem hub is not configured on this pipeline engine",
                    ));
                };
                Ok(NodeDispatch::MemIncr(mem_incr::Node::new(
                    serde_json::from_value(node.config.clone())
                        .map_err(|e| PipelineError::new("FW_NODE_MEM_INCR_CONFIG", e.to_string()))?,
                    mem_hub.clone(),
                )))
            }
            mem_publish::NODE_KIND => {
                let Some(mem_hub) = &self.mem_hub else {
                    return Err(PipelineError::new(
                        "FW_NODE_MEM_UNAVAILABLE",
                        "mem hub is not configured on this pipeline engine",
                    ));
                };
                Ok(NodeDispatch::MemPublish(mem_publish::Node::new(
                    serde_json::from_value(node.config.clone())
                        .map_err(|e| PipelineError::new("FW_NODE_MEM_PUBLISH_CONFIG", e.to_string()))?,
                    mem_hub.clone(),
                )))
            }
            mem_exists::NODE_KIND => {
                let Some(mem_hub) = &self.mem_hub else {
                    return Err(PipelineError::new(
                        "FW_NODE_MEM_UNAVAILABLE",
                        "mem hub is not configured on this pipeline engine",
                    ));
                };
                Ok(NodeDispatch::MemExists(mem_exists::Node::new(
                    serde_json::from_value(node.config.clone())
                        .map_err(|e| PipelineError::new("FW_NODE_MEM_EXISTS_CONFIG", e.to_string()))?,
                    mem_hub.clone(),
                )))
            }
            mem_expire::NODE_KIND => {
                let Some(mem_hub) = &self.mem_hub else {
                    return Err(PipelineError::new(
                        "FW_NODE_MEM_UNAVAILABLE",
                        "mem hub is not configured on this pipeline engine",
                    ));
                };
                Ok(NodeDispatch::MemExpire(mem_expire::Node::new(
                    serde_json::from_value(node.config.clone())
                        .map_err(|e| PipelineError::new("FW_NODE_MEM_EXPIRE_CONFIG", e.to_string()))?,
                    mem_hub.clone(),
                )))
            }
            memsubscribe::NODE_KIND => Ok(NodeDispatch::MemSubscribe(memsubscribe::Node::new(
                serde_json::from_value(node.config.clone())
                    .map_err(|e| PipelineError::new("FW_NODE_MEM_SUBSCRIBE_CONFIG", e.to_string()))?,
            ))),
            other => Err(PipelineError::new(
                "FW_NODE_KIND_UNSUPPORTED",
                format!("unsupported node kind '{}'", other),
            )),
        }
    }
}

#[async_trait]
impl PipelineEngine for BasicPipelineEngine {
    fn id(&self) -> &'static str {
        "pipeline.basic"
    }

    fn validate_graph(&self, graph: &PipelineGraph) -> Result<(), PipelineError> {
        if graph.nodes.is_empty() {
            return Err(PipelineError::new(
                "FW_EMPTY_GRAPH",
                format!("pipeline '{}' has no nodes", graph.id),
            ));
        }
        let node_map: HashMap<&str, _> = graph.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
        for entry in &graph.entry_nodes {
            if !node_map.contains_key(entry.as_str()) {
                return Err(PipelineError::new(
                    "FW_ENTRY_NODE",
                    format!("unknown entry node '{}'", entry),
                ));
            }
        }
        for (idx, edge) in graph.edges.iter().enumerate() {
            let from = node_map.get(edge.from_node.as_str()).ok_or_else(|| {
                PipelineError::new(
                    "FW_EDGE_FROM_NODE",
                    format!("edge[{idx}] unknown from_node '{}'", edge.from_node),
                )
            })?;
            let to = node_map.get(edge.to_node.as_str()).ok_or_else(|| {
                PipelineError::new(
                    "FW_EDGE_TO_NODE",
                    format!("edge[{idx}] unknown to_node '{}'", edge.to_node),
                )
            })?;
            if !from.output_pins.iter().any(|p| p == &edge.from_pin) {
                return Err(PipelineError::new(
                    "FW_EDGE_FROM_PIN",
                    format!(
                        "edge[{idx}] invalid from_pin '{}' for node '{}'",
                        edge.from_pin, from.id
                    ),
                ));
            }
            if !to.input_pins.iter().any(|p| p == &edge.to_pin) {
                return Err(PipelineError::new(
                    "FW_EDGE_TO_PIN",
                    format!(
                        "edge[{idx}] invalid to_pin '{}' for node '{}'",
                        edge.to_pin, to.id
                    ),
                ));
            }
        }
        for node in &graph.nodes {
            // Skip upfront validation for nodes whose config contains {{ expr }} placeholders —
            // those are resolved per-input at runtime, so type validation must happen there.
            if scan_exprs(&node.config).is_empty() {
                self.build_node(node)?;
            }
        }
        Ok(())
    }

    async fn execute_with_options_async(
        &self,
        graph: &PipelineGraph,
        ctx: &PipelineContext,
        options: &ExecuteOptions,
    ) -> Result<PipelineOutput, PipelineError> {
        self.validate_graph(graph)?;

        let node_map: HashMap<&str, &PipelineNode> = graph
            .nodes
            .iter()
            .map(|node| (node.id.as_str(), node))
            .collect();
        let mut outgoing: HashMap<(&str, &str), Vec<(&str, &str)>> = HashMap::new();
        for edge in &graph.edges {
            outgoing
                .entry((edge.from_node.as_str(), edge.from_pin.as_str()))
                .or_default()
                .push((edge.to_node.as_str(), edge.to_pin.as_str()));
        }

        let start_nodes = if graph.entry_nodes.is_empty() {
            vec![graph.nodes[0].id.clone()]
        } else {
            graph.entry_nodes.clone()
        };

        let step_tx = options.step_tx.clone();
        // nodes_output: accumulates each completed node's output payload for $nodes access.
        // Declared here (before the initial queue push) so entry-node metadata can include it.
        let mut nodes_output = serde_json::Map::<String, Value>::new();
        let mut queue = VecDeque::new();
        for node_id in start_nodes {
            let node = node_map
                .get(node_id.as_str())
                .ok_or_else(|| PipelineError::new("FW_ENTRY_NODE", "entry node missing"))?;
            let first_pin = node.input_pins.first().cloned().unwrap_or_default();
            queue.push_back(NodeExecutionInput {
                node_id: node.id.clone(),
                input_pin: first_pin,
                payload: ctx.input.clone(),
                metadata: json!({
                    "owner": ctx.owner,
                    "project": ctx.project,
                    "pipeline": ctx.pipeline,
                    "request_id": ctx.request_id,
                    "route": ctx.route,
                    "trigger": ctx.trigger,
                    "nodes": Value::Object(nodes_output.clone()),
                }),
                step_tx: step_tx.clone(),
            });
        }

        let mut trace = vec![format!("engine={}", self.id())];
        let mut last_value = Value::Null;
        let mut node_trace: Vec<NodeTraceEntry> = Vec::new();
        // merge_pending: node_id -> { pin_name -> payload }
        let mut merge_pending: HashMap<String, HashMap<String, Value>> = HashMap::new();
        // first_fired: tracks merge nodes that already fired (first_completed strategy)
        let mut first_fired: std::collections::HashSet<String> = std::collections::HashSet::new();

        while let Some(input) = queue.pop_front() {
            let node = node_map.get(input.node_id.as_str()).ok_or_else(|| {
                PipelineError::new("FW_EXEC_NODE", format!("node '{}' missing", input.node_id))
            })?;

            // Resolve {{ expr }} placeholders in the node's config before building.
            // Uses input.metadata["nodes"] (snapshot at queue-time) for $nodes scope,
            // so each node only sees outputs of its transitive predecessors.
            let effective_config = resolve_config_expressions(
                node.config.clone(),
                &input.payload,
                &input.metadata,
                &self.language,
            )?;
            let dispatch = if effective_config == node.config {
                // No expressions resolved — use original node directly (common fast path).
                self.build_node(node)?
            } else {
                self.build_node(&PipelineNode { config: effective_config, ..(*node).clone() })?
            };

            // Capture context for per-node trace before consuming `input`.
            let trace_node_id = node.id.clone();
            let trace_node_kind = node.kind.clone();
            let node_start = std::time::Instant::now();
            let input_snapshot = input.payload.clone();

            // Per-node timeout: prevents slow HTTP/DB nodes from hanging pipelines.
            let node_timeout_secs: u64 = std::env::var("PIPELINE_NODE_TIMEOUT_SECS")
                .ok()
                .and_then(|s| s.parse().ok())
                .unwrap_or(30);
            let exec_fut = async {
            match dispatch {
                NodeDispatch::Webhook(node) => node.execute_async(input).await,
                NodeDispatch::Schedule(node) => node.execute_async(input).await,
                NodeDispatch::Manual(node) => node.execute_async(input).await,
                NodeDispatch::Script(node) => node.execute_async(input).await,
                NodeDispatch::HttpRequest(node) => node.execute_async(input).await,
                NodeDispatch::BrowserRun(node) => node.execute_async(input).await,
                NodeDispatch::SqliteQuery(node) => node.execute_async(input).await,
                NodeDispatch::SqliteMutate(node) => node.execute_async(input).await,
                NodeDispatch::Postgres(node) => node.execute_async(input).await,
                NodeDispatch::InlineWebResponse { node_id, config } => {
                    let markup = config.markup.as_deref().unwrap_or("").trim();
                    if markup.is_empty() {
                        Err(PipelineError::new(
                            "FW_NODE_WEB_RESPONSE_CONFIG",
                            format!("node '{node_id}' --template set but markup not loaded"),
                        ))
                    } else {
                        // Resolve response envelope from config + input
                        let location = config.location.as_deref().map(|loc| {
                            if loc.starts_with("$.") || loc == "$" {
                                web_response::resolve_json_path_string(&input.payload, loc)
                                    .unwrap_or_else(|| loc.to_string())
                            } else {
                                loc.to_string()
                            }
                        });
                        let status = config.status
                            .or_else(|| if location.is_some() { Some(302) } else { None });
                        let cookie = config.set_cookie.as_deref()
                            .and_then(|s| web_response::parse_cookie_spec(s, &input.payload));
                        let headers = config.headers.clone();

                        let template_id = config.template.clone().unwrap_or_default();
                        let source_path = self.template_root.as_ref()
                            .and_then(|r| config.template.as_deref().map(|t| r.join(t)));
                        let options = crate::rwe::ReactiveWebOptions {
                            templates: crate::rwe::TemplateOptions {
                                template_root: self.template_root.clone(),
                                style_entries: Vec::new(),
                            },
                            ..Default::default()
                        };

                        let key = hash_markup(markup);
                        let cached = self.template_cache.as_ref().and_then(|c| {
                            c.read().unwrap_or_else(|e| e.into_inner()).get(&key).map(|e| e.page.clone())
                        });
                        let compiled_result: Result<Arc<_>, PipelineError> = if let Some(hit) = cached {
                            Ok(hit)
                        } else {
                            let fresh = web_response::compile_page(
                                &node_id,
                                &TemplateSource {
                                    id: template_id,
                                    source_path,
                                    markup: markup.to_string(),
                                },
                                &options,
                                self.rwe.as_ref(),
                                self.language.as_ref(),
                            )
                            .map(Arc::new);
                            if let Ok(ref fresh_arc) = fresh {
                                if let Some(cache) = &self.template_cache {
                                    let deps = fresh_arc.template.dependency_paths.clone();
                                    cache.write().unwrap_or_else(|e| e.into_inner()).insert(key, CacheEntry {
                                        page: fresh_arc.clone(),
                                        dependencies: deps,
                                    });
                                }
                            }
                            fresh
                        };

                        compiled_result.and_then(|compiled| {
                            let enabled_libraries: Vec<String> = self.platform
                                .as_ref()
                                .and_then(|p| {
                                    p.zebflow_cfg.get_rwe_libraries(&ctx.owner, &ctx.project).ok()
                                })
                                .map(|libs| libs.into_keys().collect())
                                .unwrap_or_default();

                            let render_out = web_response::render_compiled_page(
                                &compiled,
                                input.payload,
                                input.metadata,
                                self.rwe.as_ref(),
                                self.language.as_ref(),
                                &ctx.request_id,
                                enabled_libraries,
                            )?;
                            let envelope = serde_json::json!({
                                "status": status,
                                "location": location,
                                "set_cookie": cookie,
                                "headers": headers,
                                "html": render_out.payload.get("html"),
                                "compiled_scripts": render_out.payload.get("compiled_scripts"),
                                "hydration_payload": render_out.payload.get("hydration_payload"),
                            });
                            Ok(NodeExecutionOutput {
                                output_pins: render_out.output_pins,
                                payload: serde_json::json!({ "__zf_response": envelope }),
                                trace: render_out.trace,
                            })
                        })
                    }
                }
                NodeDispatch::WebResponse(node) => node.execute_async(input).await,
                NodeDispatch::Agent(node) => node.execute_async(input).await,
                NodeDispatch::LogicIf(node) => node.execute_async(input).await,
                NodeDispatch::LogicSwitch(node) => node.execute_async(input).await,
                NodeDispatch::LogicBranch(node) => node.execute_async(input).await,
                NodeDispatch::LogicMerge(node) => node.execute_async(input).await,
                NodeDispatch::AuthTokenCreate(node) => node.execute_async(input).await,
                NodeDispatch::WebError(node) => node.execute_async(input).await,
                NodeDispatch::WsTrigger(node) => node.execute_async(input).await,
                NodeDispatch::WsSyncState(node) => node.execute_async(input).await,
                NodeDispatch::WsEmit(node) => node.execute_async(input).await,
                NodeDispatch::Crypto(node) => node.execute_async(input).await,
                NodeDispatch::TriggerFunction(node) => node.execute_async(input).await,
                NodeDispatch::FunctionCall(node) => node.execute_async(input).await,
                NodeDispatch::FileSave(node) => node.execute_async(input).await,
                NodeDispatch::ImgThumbnail(node) => node.execute_async(input).await,
                NodeDispatch::MemSet(node) => node.execute_async(input).await,
                NodeDispatch::MemGet(node) => node.execute_async(input).await,
                NodeDispatch::MemDel(node) => node.execute_async(input).await,
                NodeDispatch::MemExists(node) => node.execute_async(input).await,
                NodeDispatch::MemExpire(node) => node.execute_async(input).await,
                NodeDispatch::MemIncr(node) => node.execute_async(input).await,
                NodeDispatch::MemPublish(node) => node.execute_async(input).await,
                NodeDispatch::MemSubscribe(node) => node.execute_async(input).await,
            }
            }; // end exec_fut
            let timeout_node_id = trace_node_id.clone();
            let exec_result: Result<NodeExecutionOutput, PipelineError> =
                tokio::time::timeout(
                    std::time::Duration::from_secs(node_timeout_secs),
                    exec_fut,
                )
                .await
                .unwrap_or_else(|_| Err(PipelineError::new(
                    "FW_NODE_TIMEOUT",
                    format!("node '{}' timed out after {node_timeout_secs}s", timeout_node_id),
                )));

            let output = match exec_result {
                Ok(mut out) => {
                    let mut output_payload = out.payload.clone();
                    let payload_redact_tokens = take_private_redact_tokens(&mut output_payload);
                    let payload_redact_except_paths =
                        take_private_redact_except_paths(&mut output_payload);
                    let mut trace_redact_tokens = payload_redact_tokens.clone();
                    extend_unique_strings(
                        &mut trace_redact_tokens,
                        take_private_trace_redact_tokens(&mut output_payload),
                    );
                    let mut trace_redact_except_paths = payload_redact_except_paths.clone();
                    extend_unique_paths(
                        &mut trace_redact_except_paths,
                        take_private_trace_redact_except_paths(&mut output_payload),
                    );
                    let payload_output = if payload_redact_tokens.is_empty() {
                        output_payload
                    } else {
                        redact_json_value(
                            &output_payload,
                            &payload_redact_tokens,
                            &payload_redact_except_paths,
                            &[],
                        )
                    };
                    let redacted_input = if trace_redact_tokens.is_empty() {
                        input_snapshot
                    } else {
                        redact_json_value(
                            &input_snapshot,
                            &trace_redact_tokens,
                            &trace_redact_except_paths,
                            &[],
                        )
                    };
                    let redacted_output = if trace_redact_tokens.is_empty() {
                        payload_output.clone()
                    } else {
                        redact_json_value(
                            &payload_output,
                            &trace_redact_tokens,
                            &trace_redact_except_paths,
                            &[],
                        )
                    };
                    // Record this node's output so downstream nodes can access it via $nodes.
                    nodes_output.insert(trace_node_id.clone(), payload_output.clone());
                    node_trace.push(NodeTraceEntry {
                        node_id: trace_node_id,
                        node_kind: trace_node_kind,
                        duration_ms: node_start.elapsed().as_millis() as u64,
                        input: redacted_input,
                        output: redacted_output.clone(),
                        error: None,
                    });
                    out.payload = payload_output;
                    out
                }
                Err(mut e) => {
                    node_trace.push(NodeTraceEntry {
                        node_id: trace_node_id.clone(),
                        node_kind: trace_node_kind.clone(),
                        duration_ms: node_start.elapsed().as_millis() as u64,
                        input: input_snapshot,
                        output: Value::Null,
                        error: Some(e.message.clone()),
                    });
                    // Attribute error to the failing node if not already set.
                    if e.node_id.is_none() {
                        e.node_id = Some(trace_node_id);
                        e.node_kind = Some(trace_node_kind);
                    }
                    return Err(e);
                }
            };
            trace.extend(output.trace.clone());
            last_value = output.payload.clone();

            for emitted_pin in &output.output_pins {
                if let Some(next_edges) = outgoing.get(&(node.id.as_str(), emitted_pin.as_str())) {
                    for (to_node, to_pin) in next_edges {
                        let target = node_map.get(to_node).ok_or_else(|| {
                            PipelineError::new("FW_EXEC_EDGE", format!("target node '{}' missing", to_node))
                        })?;
                        let merge_strategy = logic_merge_strategy(target);
                        match merge_strategy {
                            Some(MergeStrategy::WaitAll) => {
                                let pending = merge_pending.entry((*to_node).to_string()).or_default();
                                pending.insert((*to_pin).to_string(), output.payload.clone());
                                let expected = target.input_pins.len();
                                if pending.len() >= expected {
                                    let combined = Value::Object(
                                        pending.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
                                    );
                                    merge_pending.remove(*to_node);
                                    queue.push_back(NodeExecutionInput {
                                        node_id: (*to_node).to_string(),
                                        input_pin: (*to_pin).to_string(),
                                        payload: combined,
                                        metadata: json!({
                                            "owner": ctx.owner,
                                            "project": ctx.project,
                                            "pipeline": ctx.pipeline,
                                            "request_id": ctx.request_id,
                                            "route": ctx.route,
                                            "trigger": ctx.trigger,
                                            "nodes": Value::Object(nodes_output.clone()),
                                        }),
                                        step_tx: step_tx.clone(),
                                    });
                                }
                            }
                            Some(MergeStrategy::FirstCompleted) => {
                                if first_fired.insert((*to_node).to_string()) {
                                    queue.push_back(NodeExecutionInput {
                                        node_id: (*to_node).to_string(),
                                        input_pin: (*to_pin).to_string(),
                                        payload: output.payload.clone(),
                                        metadata: json!({
                                            "owner": ctx.owner,
                                            "project": ctx.project,
                                            "pipeline": ctx.pipeline,
                                            "request_id": ctx.request_id,
                                            "route": ctx.route,
                                            "trigger": ctx.trigger,
                                            "nodes": Value::Object(nodes_output.clone()),
                                        }),
                                        step_tx: step_tx.clone(),
                                    });
                                }
                            }
                            None | Some(MergeStrategy::PassThrough) => {
                                queue.push_back(NodeExecutionInput {
                                    node_id: (*to_node).to_string(),
                                    input_pin: (*to_pin).to_string(),
                                    payload: output.payload.clone(),
                                    metadata: json!({
                                        "owner": ctx.owner,
                                        "project": ctx.project,
                                        "pipeline": ctx.pipeline,
                                        "request_id": ctx.request_id,
                                        "route": ctx.route,
                                        "trigger": ctx.trigger,
                                        "nodes": Value::Object(nodes_output.clone()),
                                    }),
                                    step_tx: step_tx.clone(),
                                });
                            }
                        }
                    }
                }
            }
        }

        Ok(PipelineOutput {
            value: last_value,
            trace,
            node_trace,
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        redact_json_value, take_private_redact_except_paths, take_private_redact_tokens,
        take_private_trace_redact_tokens,
    };

    #[test]
    fn private_redact_tokens_are_removed_and_applied_recursively() {
        let mut payload = json!({
            "__zf_private_redact": ["abc123", "secret-value"],
            "password": "abc123",
            "nested": {
                "preview": "token=secret-value",
                "array": ["abc123", "ok"]
            }
        });

        let tokens = take_private_redact_tokens(&mut payload);
        assert_eq!(tokens, vec!["abc123", "secret-value"]);
        assert!(payload.get("__zf_private_redact").is_none());

        let redacted = redact_json_value(&payload, &tokens, &[], &[]);
        assert_eq!(redacted["password"], "••••••");
        assert_eq!(redacted["nested"]["preview"], "token=••••••");
        assert_eq!(redacted["nested"]["array"][0], "••••••");
        assert_eq!(redacted["nested"]["array"][1], "ok");
    }

    #[test]
    fn private_redact_can_preserve_response_body_subtree() {
        let mut payload = json!({
            "__zf_private_redact": ["https://secret.example/api/login", "token-123"],
            "__zf_private_redact_except_paths": ["response.body"],
            "request": {
                "url": "https://secret.example/api/login",
                "summary": "token-123"
            },
            "response": {
                "body": {
                    "echoed_url": "https://secret.example/api/login",
                    "echoed_token": "token-123"
                }
            }
        });

        let tokens = take_private_redact_tokens(&mut payload);
        let except_paths = take_private_redact_except_paths(&mut payload);
        let redacted = redact_json_value(&payload, &tokens, &except_paths, &[]);

        assert_eq!(redacted["request"]["url"], "••••••");
        assert_eq!(redacted["request"]["summary"], "••••••");
        assert_eq!(
            redacted["response"]["body"]["echoed_url"],
            "https://secret.example/api/login"
        );
        assert_eq!(redacted["response"]["body"]["echoed_token"], "token-123");
    }

    #[test]
    fn private_trace_redact_tokens_are_removed_without_touching_payload_redact_keys() {
        let mut payload = json!({
            "__zf_private_trace_redact": ["abc123"],
            "token": "abc123"
        });

        let tokens = take_private_trace_redact_tokens(&mut payload);
        assert_eq!(tokens, vec!["abc123"]);
        assert!(payload.get("__zf_private_trace_redact").is_none());
        assert_eq!(payload["token"], "abc123");
    }
}

enum NodeDispatch {
    Webhook(webhook::Node),
    Schedule(schedule::Node),
    Manual(manual::Node),
    Script(script::Node),
    HttpRequest(http_request::Node),
    BrowserRun(browser_run::Node),
    SqliteQuery(sqlite_query::Node),
    SqliteMutate(sqlite_mutate::Node),
    Postgres(pg_query::Node),
    InlineWebResponse {
        node_id: String,
        config: web_response::Config,
    },
    WebResponse(web_response::Node),
    Agent(agent::Node),
    LogicIf(logic::if_::Node),
    LogicSwitch(logic::switch::Node),
    LogicBranch(logic::branch::Node),
    LogicMerge(logic::merge::Node),
    AuthTokenCreate(auth_token_create::Node),
    WebError(weberror::Node),
    WsTrigger(ws_trigger::Node),
    WsSyncState(ws_sync_state::Node),
    WsEmit(ws_emit::Node),
    Crypto(crypto::Node),
    TriggerFunction(trigger_function::Node),
    FunctionCall(function_call::Node),
    FileSave(file_save::Node),
    ImgThumbnail(img_thumbnail::Node),
    MemSet(mem_set::Node),
    MemGet(mem_get::Node),
    MemDel(mem_del::Node),
    MemExists(mem_exists::Node),
    MemExpire(mem_expire::Node),
    MemIncr(mem_incr::Node),
    MemPublish(mem_publish::Node),
    MemSubscribe(memsubscribe::Node),
}

enum MergeStrategy {
    WaitAll,
    FirstCompleted,
    PassThrough,
}

fn logic_merge_strategy(node: &PipelineNode) -> Option<MergeStrategy> {
    if node.kind != logic::merge::NODE_KIND {
        return None;
    }
    let strategy = node.config.get("strategy").and_then(|v| v.as_str()).unwrap_or("pass_through");
    Some(match strategy {
        "wait_all" => MergeStrategy::WaitAll,
        "first_completed" => MergeStrategy::FirstCompleted,
        _ => MergeStrategy::PassThrough,
    })
}
