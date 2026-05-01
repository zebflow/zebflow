//! Real framework engine with graph traversal and built-in node dispatch.

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::infra::io::state::{DynStateBus, MemStateBus};
use crate::infra::mem::MemHub;
use crate::infra::transport::ws::WsHub;
use crate::language::{DenoSandboxEngine, LanguageEngine};
use crate::pipeline::expr::{resolve_config_expressions, scanner::scan as scan_exprs};
use crate::pipeline::interface::PipelineEngine;
use crate::pipeline::model::{
    ExecuteOptions, NodeTraceEntry, PipelineContext, PipelineError, PipelineGraph, PipelineNode,
    PipelineOutput,
};
use crate::pipeline::nodes::basic::{
    agent, ai_tts, auth_token_create, browser_run, crypto, file_compress, file_decompress,
    file_pdf_convert, file_save, function_call, http_request, img_thumbnail, logic, mem_del,
    mem_exists, mem_expire, mem_get, mem_incr, mem_publish, mem_set, pg_query, script,
    sekejap_query, sqlite_mutate, sqlite_query,
    trigger::{
        function as trigger_function, manual, mapserver, memsubscribe, schedule, weberror, webhook,
    },
    web_docs_generate, web_response, web_static_generate, web_static_site, ws_emit, ws_sync_state,
    ws_trigger,
};
use crate::pipeline::nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler};
use crate::platform::services::CredentialService;
use crate::platform::services::PlatformService;
use crate::rwe::{ReactiveWebEngine, TemplateSource, resolve_engine_or_default};

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

#[derive(Debug, Clone, Default)]
struct ReducePendingState {
    acc: Option<Value>,
    received: usize,
    expected: Option<usize>,
}

const RETRY_STATE_KEY: &str = "__zf_retry";

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

#[cfg(test)]
fn take_private_trace_redact_tokens(payload: &mut Value) -> Vec<String> {
    take_private_tokens(payload, "__zf_private_trace_redact")
}

fn take_private_redact_except_paths(payload: &mut Value) -> Vec<Vec<String>> {
    take_private_paths(payload, "__zf_private_redact_except_paths")
}

fn retry_attempt_from_payload(payload: &Value) -> usize {
    payload
        .get(RETRY_STATE_KEY)
        .and_then(|value| value.get("attempt"))
        .and_then(Value::as_u64)
        .map(|n| n as usize)
        .unwrap_or(0)
}

fn build_retry_error_payload(input_payload: &Value, error: &PipelineError) -> Value {
    let attempt = retry_attempt_from_payload(input_payload) + 1;
    json!({
        "input": input_payload,
        "error": {
            "code": error.code,
            "message": error.message,
            "node_id": error.node_id,
            "node_kind": error.node_kind,
        },
        RETRY_STATE_KEY: {
            "attempt": attempt,
            "failing_node_id": error.node_id,
            "failing_node_kind": error.node_kind,
        }
    })
}

fn is_sensitive_trace_config_key(key: &str) -> bool {
    let normalized = key
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "password"
            | "passwd"
            | "passphrase"
            | "secret"
            | "clientsecret"
            | "accesstoken"
            | "refreshtoken"
            | "idtoken"
            | "authorization"
            | "apikey"
            | "privatekey"
            | "signingkey"
            | "jwtsecret"
            | "cookiesecret"
            | "webhooksecret"
    )
}

fn sanitize_trace_config_value(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut out = serde_json::Map::with_capacity(map.len());
            for (key, value) in map {
                if key == "ui" {
                    continue;
                }
                let sanitized = if is_sensitive_trace_config_key(key) {
                    Value::String("••••••".to_string())
                } else {
                    sanitize_trace_config_value(value)
                };
                out.insert(key.clone(), sanitized);
            }
            Value::Object(out)
        }
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(sanitize_trace_config_value)
                .collect::<Vec<_>>(),
        ),
        _ => value.clone(),
    }
}

fn trace_config_snapshot(value: &Value) -> Option<Value> {
    let sanitized = sanitize_trace_config_value(value);
    match &sanitized {
        Value::Null => None,
        Value::Object(map) if map.is_empty() => None,
        Value::Array(items) if items.is_empty() => None,
        _ => Some(sanitized),
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
    state_bus: Option<DynStateBus>,
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
            state_bus: None,
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
            state_bus: None,
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

    /// Attach the state bus so `n.mem.*` nodes can access the shared project-scoped KV/pubsub layer.
    pub fn with_state_bus(mut self, bus: DynStateBus) -> Self {
        self.state_bus = Some(bus);
        self
    }

    /// Attach the legacy mem hub convenience wrapper.
    ///
    /// This keeps current call sites simple while routing the live mem node surface through the
    /// stronger `StateBus` abstraction.
    pub fn with_mem_hub(mut self, hub: Arc<MemHub>) -> Self {
        self.state_bus = Some(Arc::new(MemStateBus::from_hub((*hub).clone())));
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
                serde_json::from_value(node.config.clone())
                    .map_err(|err| PipelineError::new("FW_NODE_WEBHOOK_CONFIG", err.to_string()))?,
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
            mapserver::NODE_KIND => Ok(NodeDispatch::Mapserver(mapserver::Node::new(
                serde_json::from_value(node.config.clone()).map_err(|err| {
                    PipelineError::new("FW_NODE_MAPSERVER_CONFIG", err.to_string())
                })?,
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
            sekejap_query::NODE_KIND => {
                let Some(data_root) = &self.data_root else {
                    return Err(PipelineError::new(
                        "FW_NODE_SEKEJAP_UNAVAILABLE",
                        "data_root is not configured on this pipeline engine",
                    ));
                };
                Ok(NodeDispatch::SekejapQuery(sekejap_query::Node::new(
                    serde_json::from_value(node.config.clone()).map_err(|err| {
                        PipelineError::new("FW_NODE_SEKEJAP_QUERY_CONFIG", err.to_string())
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
                    serde_json::from_value(node.config.clone()).map_err(|e| {
                        PipelineError::new("FW_NODE_BROWSER_RUN_CONFIG", e.to_string())
                    })?,
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
            web_static_generate::NODE_KIND => {
                if self.data_root.is_none() {
                    return Err(PipelineError::new(
                        "FW_NODE_WEB_STATIC_UNAVAILABLE",
                        "data_root is not configured on this pipeline engine",
                    ));
                }
                let config: web_static_generate::Config =
                    serde_json::from_value(node.config.clone()).map_err(|err| {
                        PipelineError::new("FW_NODE_WEB_STATIC_CONFIG", err.to_string())
                    })?;
                Ok(NodeDispatch::InlineWebStaticGenerate {
                    node_id: node.id.clone(),
                    config,
                })
            }
            web_docs_generate::NODE_KIND => {
                if self.data_root.is_none() {
                    return Err(PipelineError::new(
                        "FW_NODE_WEB_DOCS_UNAVAILABLE",
                        "data_root is not configured on this pipeline engine",
                    ));
                }
                let config: web_docs_generate::Config = serde_json::from_value(node.config.clone())
                    .map_err(|err| {
                        PipelineError::new("FW_NODE_WEB_DOCS_CONFIG", err.to_string())
                    })?;
                Ok(NodeDispatch::InlineWebDocsGenerate {
                    node_id: node.id.clone(),
                    config,
                })
            }
            agent::NODE_KIND => {
                let config: agent::Config =
                    serde_json::from_value(node.config.clone()).unwrap_or_default();
                Ok(NodeDispatch::Agent(agent::Node::new(
                    config,
                    self.credentials.clone(),
                    self.platform.clone(),
                )))
            }
            ai_tts::NODE_KIND => {
                let config: ai_tts::Config = serde_json::from_value(node.config.clone())
                    .map_err(|err| PipelineError::new("FW_NODE_AI_TTS_CONFIG", err.to_string()))?;
                Ok(NodeDispatch::AiTts(ai_tts::Node::new(
                    config,
                    self.credentials.clone(),
                    self.platform.clone(),
                    self.language.clone(),
                )))
            }
            logic::if_::NODE_KIND => Ok(NodeDispatch::LogicIf(logic::if_::Node::new(
                &node.id,
                serde_json::from_value(node.config.clone())
                    .map_err(|e| PipelineError::new("FW_NODE_LOGIC_IF_CONFIG", e.to_string()))?,
                self.language.clone(),
            )?)),
            logic::match_::NODE_KIND => Ok(NodeDispatch::LogicMatch(logic::match_::Node::new(
                &node.id,
                serde_json::from_value(node.config.clone())
                    .map_err(|e| PipelineError::new("FW_NODE_LOGIC_MATCH_CONFIG", e.to_string()))?,
                self.language.clone(),
            )?)),
            logic::collect::NODE_KIND => Ok(NodeDispatch::LogicCollect(logic::collect::Node::new(
                serde_json::from_value(node.config.clone()).map_err(|e| {
                    PipelineError::new("FW_NODE_LOGIC_COLLECT_CONFIG", e.to_string())
                })?,
            ))),
            logic::foreach_::NODE_KIND => Ok(NodeDispatch::LogicForeach(
                logic::foreach_::Node::new(serde_json::from_value(node.config.clone()).map_err(
                    |e| PipelineError::new("FW_NODE_LOGIC_FOREACH_CONFIG", e.to_string()),
                )?),
            )),
            logic::reduce::NODE_KIND => Ok(NodeDispatch::LogicReduce(logic::reduce::Node::new(
                &node.id,
                serde_json::from_value(node.config.clone()).map_err(|e| {
                    PipelineError::new("FW_NODE_LOGIC_REDUCE_CONFIG", e.to_string())
                })?,
                self.language.clone(),
            )?)),
            logic::retry::NODE_KIND => Ok(NodeDispatch::LogicRetry(logic::retry::Node::new(
                serde_json::from_value(node.config.clone())
                    .map_err(|e| PipelineError::new("FW_NODE_LOGIC_RETRY_CONFIG", e.to_string()))?,
            )?)),
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
                Ok(NodeDispatch::TriggerFunction(trigger_function::Node::new(
                    config,
                )))
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
                Ok(NodeDispatch::FileSave(file_save::Node::new(
                    config,
                    platform.clone(),
                )?))
            }
            file_compress::NODE_KIND => {
                let config: file_compress::Config =
                    serde_json::from_value(node.config.clone()).unwrap_or_default();
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        "FW_NODE_FILE_COMPRESS",
                        "platform service not available in this engine context",
                    ));
                };
                Ok(NodeDispatch::FileCompress(file_compress::Node::new(
                    config,
                    platform.clone(),
                )?))
            }
            file_decompress::NODE_KIND => {
                let config: file_decompress::Config =
                    serde_json::from_value(node.config.clone()).unwrap_or_default();
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        "FW_NODE_FILE_DECOMPRESS",
                        "platform service not available in this engine context",
                    ));
                };
                Ok(NodeDispatch::FileDecompress(file_decompress::Node::new(
                    config,
                    platform.clone(),
                )?))
            }
            file_pdf_convert::NODE_KIND => {
                let config: file_pdf_convert::Config =
                    serde_json::from_value(node.config.clone()).unwrap_or_default();
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        "FW_NODE_PDF_CONVERT",
                        "platform service not available in this engine context",
                    ));
                };
                Ok(NodeDispatch::FilePdfConvert(file_pdf_convert::Node::new(
                    config,
                    platform.clone(),
                )?))
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
                Ok(NodeDispatch::ImgThumbnail(img_thumbnail::Node::new(
                    config,
                    platform.clone(),
                )?))
            }
            mem_set::NODE_KIND => {
                let Some(state_bus) = &self.state_bus else {
                    return Err(PipelineError::new(
                        "FW_NODE_MEM_UNAVAILABLE",
                        "state bus is not configured on this pipeline engine",
                    ));
                };
                Ok(NodeDispatch::MemSet(mem_set::Node::new(
                    serde_json::from_value(node.config.clone())
                        .map_err(|e| PipelineError::new("FW_NODE_MEM_SET_CONFIG", e.to_string()))?,
                    state_bus.clone(),
                )))
            }
            mem_get::NODE_KIND => {
                let Some(state_bus) = &self.state_bus else {
                    return Err(PipelineError::new(
                        "FW_NODE_MEM_UNAVAILABLE",
                        "state bus is not configured on this pipeline engine",
                    ));
                };
                Ok(NodeDispatch::MemGet(mem_get::Node::new(
                    serde_json::from_value(node.config.clone())
                        .map_err(|e| PipelineError::new("FW_NODE_MEM_GET_CONFIG", e.to_string()))?,
                    state_bus.clone(),
                )))
            }
            mem_del::NODE_KIND => {
                let Some(state_bus) = &self.state_bus else {
                    return Err(PipelineError::new(
                        "FW_NODE_MEM_UNAVAILABLE",
                        "state bus is not configured on this pipeline engine",
                    ));
                };
                Ok(NodeDispatch::MemDel(mem_del::Node::new(
                    serde_json::from_value(node.config.clone())
                        .map_err(|e| PipelineError::new("FW_NODE_MEM_DEL_CONFIG", e.to_string()))?,
                    state_bus.clone(),
                )))
            }
            mem_incr::NODE_KIND => {
                let Some(state_bus) = &self.state_bus else {
                    return Err(PipelineError::new(
                        "FW_NODE_MEM_UNAVAILABLE",
                        "state bus is not configured on this pipeline engine",
                    ));
                };
                Ok(NodeDispatch::MemIncr(mem_incr::Node::new(
                    serde_json::from_value(node.config.clone()).map_err(|e| {
                        PipelineError::new("FW_NODE_MEM_INCR_CONFIG", e.to_string())
                    })?,
                    state_bus.clone(),
                )))
            }
            mem_publish::NODE_KIND => {
                let Some(state_bus) = &self.state_bus else {
                    return Err(PipelineError::new(
                        "FW_NODE_MEM_UNAVAILABLE",
                        "state bus is not configured on this pipeline engine",
                    ));
                };
                Ok(NodeDispatch::MemPublish(mem_publish::Node::new(
                    serde_json::from_value(node.config.clone()).map_err(|e| {
                        PipelineError::new("FW_NODE_MEM_PUBLISH_CONFIG", e.to_string())
                    })?,
                    state_bus.clone(),
                )))
            }
            mem_exists::NODE_KIND => {
                let Some(state_bus) = &self.state_bus else {
                    return Err(PipelineError::new(
                        "FW_NODE_MEM_UNAVAILABLE",
                        "state bus is not configured on this pipeline engine",
                    ));
                };
                Ok(NodeDispatch::MemExists(mem_exists::Node::new(
                    serde_json::from_value(node.config.clone()).map_err(|e| {
                        PipelineError::new("FW_NODE_MEM_EXISTS_CONFIG", e.to_string())
                    })?,
                    state_bus.clone(),
                )))
            }
            mem_expire::NODE_KIND => {
                let Some(state_bus) = &self.state_bus else {
                    return Err(PipelineError::new(
                        "FW_NODE_MEM_UNAVAILABLE",
                        "state bus is not configured on this pipeline engine",
                    ));
                };
                Ok(NodeDispatch::MemExpire(mem_expire::Node::new(
                    serde_json::from_value(node.config.clone()).map_err(|e| {
                        PipelineError::new("FW_NODE_MEM_EXPIRE_CONFIG", e.to_string())
                    })?,
                    state_bus.clone(),
                )))
            }
            memsubscribe::NODE_KIND => Ok(NodeDispatch::MemSubscribe(memsubscribe::Node::new(
                serde_json::from_value(node.config.clone()).map_err(|e| {
                    PipelineError::new("FW_NODE_MEM_SUBSCRIBE_CONFIG", e.to_string())
                })?,
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
            if !from.output_pins.iter().any(|p| p == &edge.from_pin) && edge.from_pin != "error" {
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
        let mut incoming_counts: HashMap<&str, usize> = HashMap::new();
        for edge in &graph.edges {
            outgoing
                .entry((edge.from_node.as_str(), edge.from_pin.as_str()))
                .or_default()
                .push((edge.to_node.as_str(), edge.to_pin.as_str()));
            *incoming_counts.entry(edge.to_node.as_str()).or_default() += 1;
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
        let mut collect_pending: HashMap<String, HashMap<String, Value>> = HashMap::new();
        let mut reduce_pending: HashMap<String, ReducePendingState> = HashMap::new();

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
            let base_trace_config = trace_config_snapshot(&effective_config);
            let dispatch = if effective_config == node.config {
                // No expressions resolved — use original node directly (common fast path).
                self.build_node(node)?
            } else {
                self.build_node(&PipelineNode {
                    config: effective_config.clone(),
                    ..(*node).clone()
                })?
            };

            // Capture context for per-node trace before consuming `input`.
            let trace_node_id = node.id.clone();
            let trace_node_kind = node.kind.clone();
            let node_start = std::time::Instant::now();
            let input_snapshot = input.payload.clone();

            // Per-node timeout: prevents slow HTTP/DB nodes from hanging pipelines.
            let node_timeout_secs: u64 = self
                .platform
                .as_ref()
                .map(|platform| {
                    platform
                        .zebflow_cfg
                        .read_or_default(&ctx.owner, &ctx.project)
                        .configs
                        .pipelines
                        .effective_node_timeout_secs()
                })
                .or_else(|| {
                    std::env::var("PIPELINE_NODE_TIMEOUT_SECS")
                        .ok()
                        .and_then(|s| s.parse().ok())
                })
                .unwrap_or(crate::platform::model::default_pipeline_node_timeout_secs());
            let mut input_for_exec = input.clone();
            if node.kind == logic::reduce::NODE_KIND
                && let Some(acc) = reduce_pending
                    .get(node.id.as_str())
                    .and_then(|state| state.acc.clone())
                && let Some(map) = input_for_exec.metadata.as_object_mut()
            {
                map.insert("reduce_acc".to_string(), acc);
            }
            let exec_fut = async {
                match dispatch {
                    NodeDispatch::Webhook(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::Schedule(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::Manual(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::Mapserver(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::Script(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::HttpRequest(node) => {
                        node.execute_many_async(input_for_exec).await
                    }
                    NodeDispatch::BrowserRun(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::SqliteQuery(node) => {
                        node.execute_many_async(input_for_exec).await
                    }
                    NodeDispatch::SekejapQuery(node) => {
                        node.execute_many_async(input_for_exec).await
                    }
                    NodeDispatch::SqliteMutate(node) => {
                        node.execute_many_async(input_for_exec).await
                    }
                    NodeDispatch::Postgres(node) => node.execute_many_async(input_for_exec).await,
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
                            let status = config
                                .status
                                .or_else(|| if location.is_some() { Some(302) } else { None });
                            let cookie = config
                                .set_cookie
                                .as_deref()
                                .and_then(|s| web_response::parse_cookie_spec(s, &input.payload));
                            let headers = config.headers.clone();

                            let template_id = config.template.clone().unwrap_or_default();
                            let source_path = self
                                .template_root
                                .as_ref()
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
                                c.read()
                                    .unwrap_or_else(|e| e.into_inner())
                                    .get(&key)
                                    .map(|e| e.page.clone())
                            });
                            let compiled_result: Result<Arc<_>, PipelineError> = if let Some(hit) =
                                cached
                            {
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
                                        cache.write().unwrap_or_else(|e| e.into_inner()).insert(
                                            key,
                                            CacheEntry {
                                                page: fresh_arc.clone(),
                                                dependencies: deps,
                                            },
                                        );
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
                            Ok(vec![NodeExecutionOutput {
                                output_pins: render_out.output_pins,
                                payload: serde_json::json!({ "__zf_response": envelope }),
                                trace: render_out.trace,
                            }])
                        })
                        }
                    }
                    NodeDispatch::WebResponse(node) => {
                        node.execute_many_async(input_for_exec).await
                    }
                    NodeDispatch::InlineWebDocsGenerate { node_id, config } => {
                        let Some(data_root) = &self.data_root else {
                            return Err(PipelineError::new(
                                "FW_NODE_WEB_DOCS_UNAVAILABLE",
                                "data_root is not configured on this pipeline engine",
                            ));
                        };
                        let Some(template_root) = &self.template_root else {
                            return Err(PipelineError::new(
                                "FW_NODE_WEB_DOCS_TEMPLATE_ROOT",
                                "template_root is not configured on this pipeline engine",
                            ));
                        };

                        let site = web_docs_generate::load_site(&config, template_root)?;
                        let options = crate::rwe::ReactiveWebOptions {
                            templates: crate::rwe::TemplateOptions {
                                template_root: self.template_root.clone(),
                                style_entries: Vec::new(),
                            },
                            processors: vec!["tailwind".to_string(), "markdown".to_string()],
                            ..Default::default()
                        };
                        let key = hash_markup(&site.template_source.markup);
                        let cached = self.template_cache.as_ref().and_then(|c| {
                            c.read()
                                .unwrap_or_else(|e| e.into_inner())
                                .get(&key)
                                .map(|e| e.page.clone())
                        });
                        let compiled_result: Result<Arc<_>, PipelineError> =
                            if let Some(hit) = cached {
                                Ok(hit)
                            } else {
                                let fresh = web_response::compile_page(
                                    &node_id,
                                    &site.template_source,
                                    &options,
                                    self.rwe.as_ref(),
                                    self.language.as_ref(),
                                )
                                .map(Arc::new);
                                if let Ok(ref fresh_arc) = fresh
                                    && let Some(cache) = &self.template_cache
                                {
                                    let deps = fresh_arc.template.dependency_paths.clone();
                                    cache.write().unwrap_or_else(|e| e.into_inner()).insert(
                                        key,
                                        CacheEntry {
                                            page: fresh_arc.clone(),
                                            dependencies: deps,
                                        },
                                    );
                                }
                                fresh
                            };

                        compiled_result.and_then(|compiled| {
                            let mut generated_files = 0usize;
                            let mut skipped_files = 0usize;
                            let mut urls = Vec::new();
                            let asset_group = web_static_site::asset_group_id(
                                &site.template_rel_path,
                                &site.template_source.markup,
                            );
                            let mut page_records = Vec::new();
                            let mut asset_records = Vec::new();
                            let site_root_abs = data_root
                                .join("users")
                                .join(&ctx.owner)
                                .join(&ctx.project)
                                .join("files")
                                .join(&site.site_root_rel);
                            let project_asset_root = self
                                .template_root
                                .as_deref()
                                .map(|root| root.join("assets"));

                            let enabled_libraries: Vec<String> = self
                                .platform
                                .as_ref()
                                .and_then(|p| {
                                    p.zebflow_cfg
                                        .get_rwe_libraries(&ctx.owner, &ctx.project)
                                        .ok()
                                })
                                .map(|libs| libs.into_keys().collect())
                                .unwrap_or_default();

                            for (page_index, page) in site.pages.iter().enumerate() {
                                let mut metadata = input.metadata.clone();
                                if let Some(map) = metadata.as_object_mut() {
                                    map.insert(
                                        "route".to_string(),
                                        Value::String(web_docs_generate::default_route(page)),
                                    );
                                }
                                let payload = web_docs_generate::page_payload(
                                    &site,
                                    page_index,
                                    input.payload.clone(),
                                )?;
                                let render_out = web_response::render_compiled_page(
                                    &compiled,
                                    payload,
                                    metadata,
                                    self.rwe.as_ref(),
                                    self.language.as_ref(),
                                    &ctx.request_id,
                                    enabled_libraries.clone(),
                                )?;
                                let html = render_out
                                    .payload
                                    .get("html")
                                    .and_then(Value::as_str)
                                    .ok_or_else(|| {
                                        PipelineError::new(
                                            "FW_NODE_WEB_DOCS_RENDER",
                                            format!("node '{node_id}' did not return rendered html"),
                                        )
                                    })?
                                    .to_string();
                                let hydration_payload = render_out
                                    .payload
                                    .get("hydration_payload")
                                    .cloned()
                                    .unwrap_or(Value::Null);
                                let compiled_scripts = render_out
                                    .payload
                                    .get("compiled_scripts")
                                    .cloned()
                                    .and_then(|value| serde_json::from_value::<Vec<crate::rwe::CompiledScript>>(value).ok())
                                    .unwrap_or_default();
                                let final_html = web_static_generate::build_static_html(
                                    html,
                                    &hydration_payload,
                                    &compiled_scripts,
                                    self.template_root.as_deref(),
                                );
                                let localized = web_static_site::localize_static_html_assets(
                                    &site_root_abs,
                                    &page.output_rel_path,
                                    &final_html,
                                    web_static_site::StaticAssetSources {
                                        owner: Some(&ctx.owner),
                                        project: Some(&ctx.project),
                                        project_asset_root_abs: project_asset_root.as_deref(),
                                    },
                                    &asset_group,
                                )?;
                                asset_records.extend(localized.assets.iter().cloned());
                                let final_html = web_docs_generate::apply_page_seo(
                                    localized.html,
                                    &site,
                                    page_index,
                                );
                                let rel_path =
                                    web_docs_generate::output_rel_path(page, &site.site_root_rel)?;
                                let abs_path = data_root
                                    .join("users")
                                    .join(&ctx.owner)
                                    .join(&ctx.project)
                                    .join("files")
                                    .join(&rel_path);
                                let status = web_static_generate::write_generated_html(
                                    &abs_path,
                                    &final_html,
                                    "overwrite",
                                )?;
                                if status == "skipped" || status == "unchanged" {
                                    skipped_files += 1;
                                } else {
                                    generated_files += 1;
                                }
                                urls.push(page.route_path.clone());
                                page_records.push(web_static_site::StaticPageRecord {
                                    path: page.output_rel_path.clone(),
                                    route: page.route_path.clone(),
                                    template: site.template_rel_path.clone(),
                                    asset_group: asset_group.clone(),
                                    generator: web_docs_generate::NODE_KIND.to_string(),
                                });
                            }

                            if !site.sitemap_xml.trim().is_empty() {
                                let sitemap_rel =
                                    web_docs_generate::sitemap_rel_path(&site.site_root_rel);
                                let sitemap_abs = data_root
                                    .join("users")
                                    .join(&ctx.owner)
                                    .join(&ctx.project)
                                    .join("files")
                                    .join(&sitemap_rel);
                                let status = web_static_generate::write_generated_html(
                                    &sitemap_abs,
                                    &site.sitemap_xml,
                                    "overwrite",
                                )?;
                                if status == "skipped" || status == "unchanged" {
                                    skipped_files += 1;
                                } else {
                                    generated_files += 1;
                                }
                            }

                            let search_index_rel =
                                web_docs_generate::search_index_rel_path(&site.site_root_rel);
                            let search_index_abs = data_root
                                .join("users")
                                .join(&ctx.owner)
                                .join(&ctx.project)
                                .join("files")
                                .join(&search_index_rel);
                            let status = web_static_generate::write_generated_html(
                                &search_index_abs,
                                &site.search_index_json,
                                "overwrite",
                            )?;
                            if status == "skipped" || status == "unchanged" {
                                skipped_files += 1;
                            } else {
                                generated_files += 1;
                            }

                            let manifest_rel =
                                web_static_site::site_manifest_rel_path(&site.site_root_rel);
                            let manifest_abs = data_root
                                .join("users")
                                .join(&ctx.owner)
                                .join(&ctx.project)
                                .join("files")
                                .join(&manifest_rel);
                            let _manifest = web_static_site::update_site_manifest(
                                &manifest_abs,
                                &site.site_root_rel,
                                site.deploy_base_url.as_deref(),
                                &site.deploy_base_path,
                                web_docs_generate::NODE_KIND,
                                &site.template_rel_path,
                                &asset_group,
                                &page_records,
                                &asset_records,
                                true,
                            )?;

                            Ok(vec![NodeExecutionOutput {
                                output_pins: vec![web_docs_generate::OUTPUT_PIN_OUT.to_string()],
                                payload: json!({
                                    "docs_generated": {
                                        "status": "ok",
                                        "site_title": site.site_title,
                                        "template": site.template_rel_path,
                                        "docs_root": config.docs_root,
                                        "output_dir": config.output_dir,
                                        "site_root": site.site_root_rel,
                                        "deploy_base_url": site.deploy_base_url,
                                        "deploy_base_path": site.deploy_base_path,
                                        "manifest_path": manifest_rel,
                                        "asset_group": asset_group,
                                        "page_count": site.pages.len(),
                                        "generated_files": generated_files,
                                        "skipped_files": skipped_files,
                                        "sitemap_path": if site.sitemap_xml.trim().is_empty() { Value::Null } else { Value::String(web_docs_generate::sitemap_rel_path(&site.site_root_rel)) },
                                        "search_index_path": search_index_rel,
                                        "urls": urls,
                                    }
                                }),
                                trace: vec![
                                    format!("node={node_id}"),
                                    format!("node_kind={}", web_docs_generate::NODE_KIND),
                                    format!("pages={}", site.pages.len()),
                                ],
                            }])
                        })
                    }
                    NodeDispatch::InlineWebStaticGenerate { node_id, config } => {
                        let Some(data_root) = &self.data_root else {
                            return Err(PipelineError::new(
                                "FW_NODE_WEB_STATIC_UNAVAILABLE",
                                "data_root is not configured on this pipeline engine",
                            ));
                        };

                        let template_source = web_static_generate::resolve_template_source(
                            &node_id,
                            &config,
                            self.template_root.as_deref(),
                        )?;

                        let options = crate::rwe::ReactiveWebOptions {
                            templates: crate::rwe::TemplateOptions {
                                template_root: self.template_root.clone(),
                                style_entries: Vec::new(),
                            },
                            ..Default::default()
                        };

                        let key = hash_markup(&template_source.markup);
                        let cached = self.template_cache.as_ref().and_then(|c| {
                            c.read()
                                .unwrap_or_else(|e| e.into_inner())
                                .get(&key)
                                .map(|e| e.page.clone())
                        });
                        let compiled_result: Result<Arc<_>, PipelineError> =
                            if let Some(hit) = cached {
                                Ok(hit)
                            } else {
                                let fresh = web_response::compile_page(
                                    &node_id,
                                    &template_source,
                                    &options,
                                    self.rwe.as_ref(),
                                    self.language.as_ref(),
                                )
                                .map(Arc::new);
                                if let Ok(ref fresh_arc) = fresh
                                    && let Some(cache) = &self.template_cache
                                {
                                    let deps = fresh_arc.template.dependency_paths.clone();
                                    cache.write().unwrap_or_else(|e| e.into_inner()).insert(
                                        key,
                                        CacheEntry {
                                            page: fresh_arc.clone(),
                                            dependencies: deps,
                                        },
                                    );
                                }
                                fresh
                            };

                        compiled_result.and_then(|compiled| {
                            let rel_path = web_static_generate::effective_output_rel_path(&config)?;
                            let abs_path = data_root
                                .join("users")
                                .join(&ctx.owner)
                                .join(&ctx.project)
                                .join("files")
                                .join(&rel_path);
                            let route =
                                if let Some(explicit_route) =
                                    config.route.clone().filter(|s| !s.trim().is_empty())
                                {
                                    explicit_route
                                } else if let Some(deploy_base_path) =
                                    web_static_generate::effective_deploy_base_path(&config)?
                                {
                                    let page_output_path =
                                        web_static_generate::effective_page_output_path(&config)?;
                                    web_static_site::route_path_for_output_path(
                                        &deploy_base_path,
                                        &page_output_path,
                                    )?
                                } else {
                                    web_static_generate::default_route(
                                        &ctx.owner,
                                        &ctx.project,
                                        &rel_path,
                                    )
                                };

                            let mut metadata = input.metadata.clone();
                            if let Some(map) = metadata.as_object_mut() {
                                map.insert("route".to_string(), Value::String(route.clone()));
                            }

                            let enabled_libraries: Vec<String> = self
                                .platform
                                .as_ref()
                                .and_then(|p| {
                                    p.zebflow_cfg
                                        .get_rwe_libraries(&ctx.owner, &ctx.project)
                                        .ok()
                                })
                                .map(|libs| libs.into_keys().collect())
                                .unwrap_or_default();

                            let render_out = web_response::render_compiled_page(
                                &compiled,
                                input.payload,
                                metadata,
                                self.rwe.as_ref(),
                                self.language.as_ref(),
                                &ctx.request_id,
                                enabled_libraries,
                            )?;

                            let html = render_out
                                .payload
                                .get("html")
                                .and_then(Value::as_str)
                                .ok_or_else(|| {
                                    PipelineError::new(
                                        "FW_NODE_WEB_STATIC_RENDER",
                                        format!("node '{node_id}' did not return rendered html"),
                                    )
                                })?
                                .to_string();

                            let hydration_payload = render_out
                                .payload
                                .get("hydration_payload")
                                .cloned()
                                .unwrap_or(Value::Null);
                            let compiled_scripts = render_out
                                .payload
                                .get("compiled_scripts")
                                .cloned()
                                .and_then(|value| {
                                    serde_json::from_value::<Vec<crate::rwe::CompiledScript>>(value)
                                        .ok()
                                })
                                .unwrap_or_default();

                            let final_html = web_static_generate::build_static_html(
                                html,
                                &hydration_payload,
                                &compiled_scripts,
                                self.template_root.as_deref(),
                            );
                            let asset_group = web_static_site::asset_group_id(
                                &template_source.id,
                                &template_source.markup,
                            );
                            let project_asset_root = self
                                .template_root
                                .as_deref()
                                .map(|root| root.join("assets"));
                            let localized = if let Some(site_root_rel) =
                                web_static_generate::effective_site_root_rel_path(&config)?
                            {
                                let site_root_abs = data_root
                                    .join("users")
                                    .join(&ctx.owner)
                                    .join(&ctx.project)
                                    .join("files")
                                    .join(&site_root_rel);
                                web_static_site::localize_static_html_assets(
                                    &site_root_abs,
                                    &web_static_site::normalize_page_output_path(
                                        &config.output_path,
                                    )?,
                                    &final_html,
                                    web_static_site::StaticAssetSources {
                                        owner: Some(&ctx.owner),
                                        project: Some(&ctx.project),
                                        project_asset_root_abs: project_asset_root.as_deref(),
                                    },
                                    &asset_group,
                                )?
                            } else {
                                let page_dir_abs =
                                    abs_path.parent().unwrap_or(abs_path.as_path()).to_path_buf();
                                let page_file_name = abs_path
                                    .file_name()
                                    .and_then(|name| name.to_str())
                                    .ok_or_else(|| {
                                        PipelineError::new(
                                            "FW_NODE_WEB_STATIC_OUTPUT_NAME",
                                            format!(
                                                "node '{node_id}' produced an invalid output path '{}'",
                                                abs_path.display()
                                            ),
                                        )
                                    })?
                                    .to_string();
                                web_static_site::localize_static_html_assets(
                                    &page_dir_abs,
                                    &page_file_name,
                                    &final_html,
                                    web_static_site::StaticAssetSources {
                                        owner: Some(&ctx.owner),
                                        project: Some(&ctx.project),
                                        project_asset_root_abs: project_asset_root.as_deref(),
                                    },
                                    &asset_group,
                                )?
                            };
                            let status = web_static_generate::write_generated_html(
                                &abs_path,
                                &localized.html,
                                &config.on_conflict,
                            )?;
                            let bytes = localized.html.as_bytes().len() as u64;
                            let url = format!("/files/{}/{}/{}", ctx.owner, ctx.project, rel_path);
                            let site_root_rel =
                                web_static_generate::effective_site_root_rel_path(&config)?;
                            let manifest_rel = if let Some(site_root_rel) = site_root_rel.as_deref()
                            {
                                let manifest_rel =
                                    web_static_site::site_manifest_rel_path(site_root_rel);
                                let manifest_abs = data_root
                                    .join("users")
                                    .join(&ctx.owner)
                                    .join(&ctx.project)
                                    .join("files")
                                    .join(&manifest_rel);
                                let page_path = web_static_site::normalize_page_output_path(
                                    &config.output_path,
                                )?;
                                let page_record = web_static_site::StaticPageRecord {
                                    path: page_path,
                                    route: route.clone(),
                                    template: template_source.id.clone(),
                                    asset_group: asset_group.clone(),
                                    generator: web_static_generate::NODE_KIND.to_string(),
                                };
                                let _manifest = web_static_site::update_site_manifest(
                                    &manifest_abs,
                                    site_root_rel,
                                    web_static_generate::effective_deploy_base_url(&config)
                                        .as_deref(),
                                    web_static_generate::effective_deploy_base_path(&config)?
                                        .as_deref()
                                        .unwrap_or("/"),
                                    web_static_generate::NODE_KIND,
                                    &template_source.id,
                                    &asset_group,
                                    &[page_record],
                                    &localized.assets,
                                    false,
                                )?;
                                Some(manifest_rel)
                            } else {
                                None
                            };

                            let mut trace = render_out.trace;
                            trace.push(format!("node_kind={}", web_static_generate::NODE_KIND));
                            trace.push(format!("generated_path={rel_path}"));
                            trace.push(format!("generated_status={status}"));

                            Ok(vec![NodeExecutionOutput {
                                output_pins: vec![web_static_generate::OUTPUT_PIN_OUT.to_string()],
                                payload: json!({
                                    "generated": {
                                        "status": status,
                                        "path": rel_path,
                                        "url": url,
                                        "route": route,
                                        "deploy_base_url": web_static_generate::effective_deploy_base_url(&config),
                                        "deploy_base_path": web_static_generate::effective_deploy_base_path(&config)?,
                                        "template": template_source.id,
                                        "site_root": site_root_rel,
                                        "manifest_path": manifest_rel,
                                        "asset_group": asset_group,
                                        "scope": config.scope,
                                        "bytes": bytes,
                                    }
                                }),
                                trace,
                            }])
                        })
                    }
                    NodeDispatch::Agent(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::AiTts(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::LogicIf(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::LogicMatch(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::LogicCollect(node) => {
                        node.execute_many_async(input_for_exec).await
                    }
                    NodeDispatch::LogicForeach(node) => {
                        node.execute_many_async(input_for_exec).await
                    }
                    NodeDispatch::LogicReduce(node) => {
                        node.execute_many_async(input_for_exec).await
                    }
                    NodeDispatch::LogicRetry(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::AuthTokenCreate(node) => {
                        node.execute_many_async(input_for_exec).await
                    }
                    NodeDispatch::WebError(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::WsTrigger(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::WsSyncState(node) => {
                        node.execute_many_async(input_for_exec).await
                    }
                    NodeDispatch::WsEmit(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::Crypto(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::TriggerFunction(node) => {
                        node.execute_many_async(input_for_exec).await
                    }
                    NodeDispatch::FunctionCall(node) => {
                        node.execute_many_async(input_for_exec).await
                    }
                    NodeDispatch::FileSave(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::FileCompress(node) => {
                        node.execute_many_async(input_for_exec).await
                    }
                    NodeDispatch::FileDecompress(node) => {
                        node.execute_many_async(input_for_exec).await
                    }
                    NodeDispatch::FilePdfConvert(node) => {
                        node.execute_many_async(input_for_exec).await
                    }
                    NodeDispatch::ImgThumbnail(node) => {
                        node.execute_many_async(input_for_exec).await
                    }
                    NodeDispatch::MemSet(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::MemGet(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::MemDel(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::MemExists(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::MemExpire(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::MemIncr(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::MemPublish(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::MemSubscribe(node) => {
                        node.execute_many_async(input_for_exec).await
                    }
                }
            }; // end exec_fut
            let timeout_node_id = trace_node_id.clone();
            let exec_result: Result<Vec<NodeExecutionOutput>, PipelineError> =
                tokio::time::timeout(std::time::Duration::from_secs(node_timeout_secs), exec_fut)
                    .await
                    .unwrap_or_else(|_| {
                        Err(PipelineError::new(
                            "FW_NODE_TIMEOUT",
                            format!(
                                "node '{}' timed out after {node_timeout_secs}s",
                                timeout_node_id
                            ),
                        ))
                    });

            let outputs = match exec_result {
                Ok(mut outs) => {
                    let mut processed_payloads: Vec<Value> = Vec::new();
                    let mut redacted_outputs: Vec<Value> = Vec::new();
                    let redacted_input = input_snapshot.clone();

                    for out in &mut outs {
                        let mut output_payload = out.payload.clone();
                        let payload_redact_tokens = take_private_redact_tokens(&mut output_payload);
                        let payload_redact_except_paths =
                            take_private_redact_except_paths(&mut output_payload);
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
                        processed_payloads.push(payload_output.clone());
                        redacted_outputs.push(payload_output.clone());
                        out.payload = payload_output;
                    }

                    let redacted_config = base_trace_config.clone();
                    let node_output_value = if redacted_outputs.len() == 1 {
                        redacted_outputs[0].clone()
                    } else {
                        json!({
                            "count": redacted_outputs.len(),
                            "emissions": redacted_outputs,
                        })
                    };
                    let nodes_output_value = if processed_payloads.len() == 1 {
                        processed_payloads[0].clone()
                    } else {
                        Value::Array(processed_payloads)
                    };
                    nodes_output.insert(trace_node_id.clone(), nodes_output_value);
                    node_trace.push(NodeTraceEntry {
                        node_id: trace_node_id,
                        node_kind: trace_node_kind,
                        config: redacted_config,
                        duration_ms: node_start.elapsed().as_millis() as u64,
                        input: redacted_input,
                        output: node_output_value,
                        error: None,
                    });
                    outs
                }
                Err(mut e) => {
                    node_trace.push(NodeTraceEntry {
                        node_id: trace_node_id.clone(),
                        node_kind: trace_node_kind.clone(),
                        config: base_trace_config,
                        duration_ms: node_start.elapsed().as_millis() as u64,
                        input: input_snapshot.clone(),
                        output: Value::Null,
                        error: Some(e.message.clone()),
                    });
                    // Attribute error to the failing node if not already set.
                    if e.node_id.is_none() {
                        e.node_id = Some(trace_node_id);
                        e.node_kind = Some(trace_node_kind);
                    }
                    if let Some(next_edges) = outgoing.get(&(node.id.as_str(), "error")) {
                        let error_payload = build_retry_error_payload(&input_snapshot, &e);
                        for (to_node, to_pin) in next_edges {
                            queue.push_back(NodeExecutionInput {
                                node_id: (*to_node).to_string(),
                                input_pin: (*to_pin).to_string(),
                                payload: error_payload.clone(),
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
                        continue;
                    }
                    e.node_trace = node_trace.clone();
                    return Err(e);
                }
            };
            let outputs = if node.kind == logic::reduce::NODE_KIND {
                let mut final_outputs = Vec::new();
                if let Some(last_output) = outputs.last() {
                    let state = reduce_pending.entry(node.id.clone()).or_default();
                    state.acc = Some(last_output.payload.clone());
                    state.received += 1;
                    if state.expected.is_none() {
                        state.expected = input_snapshot
                            .get("count")
                            .and_then(Value::as_u64)
                            .map(|n| n as usize);
                    }
                    let expected = state.expected.unwrap_or(1);
                    if state.received >= expected {
                        final_outputs.push(last_output.clone());
                        reduce_pending.remove(node.id.as_str());
                    }
                }
                final_outputs
            } else {
                outputs
            };

            for output in outputs {
                trace.extend(output.trace.clone());
                last_value = output.payload.clone();

                for emitted_pin in &output.output_pins {
                    if let Some(next_edges) =
                        outgoing.get(&(node.id.as_str(), emitted_pin.as_str()))
                    {
                        for (to_node, to_pin) in next_edges {
                            let target = node_map.get(to_node).ok_or_else(|| {
                                PipelineError::new(
                                    "FW_EXEC_EDGE",
                                    format!("target node '{}' missing", to_node),
                                )
                            })?;
                            if is_logic_collect(target) {
                                let pending =
                                    collect_pending.entry((*to_node).to_string()).or_default();
                                let key = if emitted_pin == "out" {
                                    node.id.clone()
                                } else {
                                    format!("{}:{}", node.id, emitted_pin)
                                };
                                pending.insert(key, output.payload.clone());
                                let expected = incoming_counts.get(*to_node).copied().unwrap_or(1);
                                if pending.len() >= expected {
                                    let combined = Value::Object(
                                        pending
                                            .iter()
                                            .map(|(k, v)| (k.clone(), v.clone()))
                                            .collect(),
                                    );
                                    collect_pending.remove(*to_node);
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
                            } else {
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
        BasicPipelineEngine, PipelineContext, redact_json_value, take_private_redact_except_paths,
        take_private_redact_tokens, take_private_trace_redact_tokens, trace_config_snapshot,
    };
    use crate::pipeline::interface::PipelineEngine;
    use crate::platform::shell::parser::build_pipeline_graph;

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

    #[test]
    fn trace_config_snapshot_redacts_sensitive_keys_but_keeps_query_text() {
        let snapshot = trace_config_snapshot(&json!({
            "query": "SELECT * FROM posts LIMIT 10",
            "limit": 20,
            "ui": { "x": 100, "y": 120 },
            "password": "super-secret",
            "headers": {
                "authorization": "Bearer abc123"
            }
        }))
        .expect("config snapshot");

        assert_eq!(snapshot["query"], "SELECT * FROM posts LIMIT 10");
        assert_eq!(snapshot["limit"], 20);
        assert!(snapshot.get("ui").is_none());
        assert_eq!(snapshot["password"], "••••••");
        assert_eq!(snapshot["headers"]["authorization"], "••••••");
    }

    #[tokio::test]
    async fn logic_collect_groups_multiple_upstreams_before_continuing() {
        let dsl = r#"
[a] trigger.manual
[b] script -- "return { user: { id: 'u_42' } };"
[c] script -- "return { orders: [{ id: 'o_1' }] };"
[d] logic.collect
[e] script -- "return input;"

[a] -> [b]
[a] -> [c]
[b] -> [d]
[c] -> [d]
[d] -> [e]
"#;

        let graph = build_pipeline_graph("logic-collect-test", dsl).expect("graph");
        let engine = BasicPipelineEngine::default();
        let out = engine
            .execute_async(
                &graph,
                &PipelineContext {
                    owner: "test".to_string(),
                    project: "test".to_string(),
                    pipeline: "logic-collect-test".to_string(),
                    request_id: "req-1".to_string(),
                    route: String::new(),
                    input: json!({}),
                    trigger: None,
                },
            )
            .await
            .expect("execute");

        assert_eq!(out.value["b"]["user"]["id"], "u_42");
        assert_eq!(out.value["c"]["orders"][0]["id"], "o_1");
    }

    #[tokio::test]
    async fn logic_if_supports_dsl_input_scope() {
        let dsl = r#"
[a] trigger.manual
[b] script -- "return { type: 'billing' };"
[c] logic.if --expr "$input.type == 'billing'"
[d] script -- "return { branch: 'true' };"
[e] script -- "return { branch: 'false' };"

[a] -> [b]
[b] -> [c]
[c]:true -> [d]
[c]:false -> [e]
"#;

        let graph = build_pipeline_graph("logic-if-scope-test", dsl).expect("graph");
        let engine = BasicPipelineEngine::default();
        let out = engine
            .execute_async(
                &graph,
                &PipelineContext {
                    owner: "test".to_string(),
                    project: "test".to_string(),
                    pipeline: "logic-if-scope-test".to_string(),
                    request_id: "req-if".to_string(),
                    route: String::new(),
                    input: json!({}),
                    trigger: None,
                },
            )
            .await
            .expect("execute");

        assert_eq!(out.value["branch"], "true");
    }

    #[tokio::test]
    async fn logic_match_supports_dsl_nodes_scope() {
        let dsl = r#"
[a] trigger.manual
[b] script -- "return { kind: 'billing' };"
[c] logic.match --expr "$nodes.b.kind" --cases billing,technical --default default
[d] script -- "return { lane: 'billing' };"
[e] script -- "return { lane: 'technical' };"
[f] script -- "return { lane: 'default' };"

[a] -> [b]
[b] -> [c]
[c]:billing -> [d]
[c]:technical -> [e]
[c]:default -> [f]
"#;

        let graph = build_pipeline_graph("logic-match-scope-test", dsl).expect("graph");
        let engine = BasicPipelineEngine::default();
        let out = engine
            .execute_async(
                &graph,
                &PipelineContext {
                    owner: "test".to_string(),
                    project: "test".to_string(),
                    pipeline: "logic-match-scope-test".to_string(),
                    request_id: "req-match".to_string(),
                    route: String::new(),
                    input: json!({}),
                    trigger: None,
                },
            )
            .await
            .expect("execute");

        assert_eq!(out.value["lane"], "billing");
    }

    #[tokio::test]
    async fn logic_foreach_emits_one_run_per_item() {
        let dsl = r#"
[a] trigger.manual
[b] logic.foreach --items-path /rows

[a] -> [b]
"#;

        let graph = build_pipeline_graph("logic-foreach-test", dsl).expect("graph");
        let engine = BasicPipelineEngine::default();
        let out = engine
            .execute_async(
                &graph,
                &PipelineContext {
                    owner: "test".to_string(),
                    project: "test".to_string(),
                    pipeline: "logic-foreach-test".to_string(),
                    request_id: "req-2".to_string(),
                    route: String::new(),
                    input: json!({
                        "rows": [
                            { "id": "r1" },
                            { "id": "r2" }
                        ]
                    }),
                    trigger: None,
                },
            )
            .await
            .expect("execute");

        let foreach_trace = out
            .node_trace
            .iter()
            .find(|entry| entry.node_kind == "n.logic.foreach")
            .expect("foreach trace");
        assert_eq!(foreach_trace.output["count"], 2);
        assert_eq!(foreach_trace.output["emissions"][0]["item"]["id"], "r1");
        assert_eq!(foreach_trace.output["emissions"][1]["item"]["id"], "r2");
        assert_eq!(out.value["item"]["id"], "r2");
        assert_eq!(out.value["index"], 1);
        assert_eq!(out.value["count"], 2);
    }

    #[tokio::test]
    async fn logic_reduce_accumulates_foreach_series() {
        let dsl = r#"
[a] trigger.manual
[b] logic.foreach --items-path /rows
[c] logic.reduce --init-expr "{ total: 0 }" --step-expr "{ total: $acc.total + $input.item.amount }"

[a] -> [b]
[b]:item -> [c]
"#;

        let graph = build_pipeline_graph("logic-reduce-test", dsl).expect("graph");
        let engine = BasicPipelineEngine::default();
        let out = engine
            .execute_async(
                &graph,
                &PipelineContext {
                    owner: "test".to_string(),
                    project: "test".to_string(),
                    pipeline: "logic-reduce-test".to_string(),
                    request_id: "req-3".to_string(),
                    route: String::new(),
                    input: json!({
                        "rows": [
                            { "amount": 10 },
                            { "amount": 15 },
                            { "amount": 7 }
                        ]
                    }),
                    trigger: None,
                },
            )
            .await
            .expect("execute");

        assert_eq!(out.value["total"], 32);
    }

    #[tokio::test]
    async fn logic_retry_retries_until_success() {
        let dsl = r#"
[a] trigger.manual
[b] script -- "const attempt = input.__zf_retry?.attempt ?? 0; if (attempt < 2) { throw new Error('retry me'); } return { ok: true, attempt };"
[r] logic.retry --max-attempts 3
[c] script -- "return input;"
[d] script -- "return input;"

[a] -> [b]
[b]:error -> [r]
[r]:retry -> [b]
[b] -> [c]
[r]:failed -> [d]
"#;

        let graph = build_pipeline_graph("logic-retry-success-test", dsl).expect("graph");
        let engine = BasicPipelineEngine::default();
        let out = engine
            .execute_async(
                &graph,
                &PipelineContext {
                    owner: "test".to_string(),
                    project: "test".to_string(),
                    pipeline: "logic-retry-success-test".to_string(),
                    request_id: "req-4".to_string(),
                    route: String::new(),
                    input: json!({}),
                    trigger: None,
                },
            )
            .await
            .expect("execute");

        assert_eq!(out.value["ok"], true);
        assert_eq!(out.value["attempt"], 2);
    }

    #[tokio::test]
    async fn logic_retry_routes_to_failed_after_budget() {
        let dsl = r#"
[a] trigger.manual
[b] script -- "throw new Error('always fail');"
[r] logic.retry --max-attempts 2
[c] script -- "return input;"

[a] -> [b]
[b]:error -> [r]
[r]:retry -> [b]
[r]:failed -> [c]
"#;

        let graph = build_pipeline_graph("logic-retry-failed-test", dsl).expect("graph");
        let engine = BasicPipelineEngine::default();
        let out = engine
            .execute_async(
                &graph,
                &PipelineContext {
                    owner: "test".to_string(),
                    project: "test".to_string(),
                    pipeline: "logic-retry-failed-test".to_string(),
                    request_id: "req-5".to_string(),
                    route: String::new(),
                    input: json!({}),
                    trigger: None,
                },
            )
            .await
            .expect("execute");

        assert!(
            out.value["error"]["message"]
                .as_str()
                .expect("error message")
                .contains("always fail")
        );
        assert_eq!(out.value["__zf_retry"]["attempt"], 2);
    }
}

enum NodeDispatch {
    Webhook(webhook::Node),
    Schedule(schedule::Node),
    Manual(manual::Node),
    Mapserver(mapserver::Node),
    Script(script::Node),
    HttpRequest(http_request::Node),
    BrowserRun(browser_run::Node),
    SqliteQuery(sqlite_query::Node),
    SekejapQuery(sekejap_query::Node),
    SqliteMutate(sqlite_mutate::Node),
    Postgres(pg_query::Node),
    InlineWebResponse {
        node_id: String,
        config: web_response::Config,
    },
    InlineWebStaticGenerate {
        node_id: String,
        config: web_static_generate::Config,
    },
    InlineWebDocsGenerate {
        node_id: String,
        config: web_docs_generate::Config,
    },
    WebResponse(web_response::Node),
    Agent(agent::Node),
    AiTts(ai_tts::Node),
    LogicIf(logic::if_::Node),
    LogicMatch(logic::match_::Node),
    LogicCollect(logic::collect::Node),
    LogicForeach(logic::foreach_::Node),
    LogicReduce(logic::reduce::Node),
    LogicRetry(logic::retry::Node),
    AuthTokenCreate(auth_token_create::Node),
    WebError(weberror::Node),
    WsTrigger(ws_trigger::Node),
    WsSyncState(ws_sync_state::Node),
    WsEmit(ws_emit::Node),
    Crypto(crypto::Node),
    TriggerFunction(trigger_function::Node),
    FunctionCall(function_call::Node),
    FileSave(file_save::Node),
    FileCompress(file_compress::Node),
    FileDecompress(file_decompress::Node),
    FilePdfConvert(file_pdf_convert::Node),
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

fn is_logic_collect(node: &PipelineNode) -> bool {
    node.kind == logic::collect::NODE_KIND
}
