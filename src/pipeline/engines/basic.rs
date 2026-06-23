//! Real framework engine with graph traversal and built-in node dispatch.
//!
//! # Engine-level common config
//!
//! The engine reads certain keys from each node's resolved config before
//! dispatching execution.  These are injected as DSL flags into every node
//! definition by [`builtin_node_definitions()`] so they are available in both
//! DSL and the UI pipeline editor.
//!
//! | Config key      | DSL flag      | Description |
//! |-----------------|---------------|-------------|
//! | `timeout_secs`  | `--timeout`   | Per-node execution timeout in seconds (5–3600). Overrides project-level `pipeline_node_timeout_secs`. |

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::infra::io::state::{DynStateBus, MemStateBus};
use crate::infra::mem::MemHub;
use crate::infra::transport::ws::WsHub;
use crate::infra::ws_client::WsClientManager;
use crate::language::{DenoSandboxEngine, LanguageEngine};
use crate::pipeline::expr::{resolve_config_expressions, scanner::scan as scan_exprs};
use crate::pipeline::interface::PipelineEngine;
use crate::pipeline::model::{
    ExecuteOptions, NodeTraceEntry, PipelineContext, PipelineError, PipelineGraph, PipelineNode,
    PipelineOutput, Signal,
};
use crate::pipeline::nodes::basic::{
    agent, ai_tts, auth_token_create, browser_run, crypto, fs_compress, fs_decompress, fs_object,
    fs_pdf_convert, fs_save, fs_thumbnail, function_call, geo_convert, geo_inspect, http_request,
    kv_del, kv_exists, kv_expire, kv_get, kv_incr, kv_publish, kv_set, logic, mapserver_crud,
    pg_query, script, sekejap_insert, sekejap_query, sqlite_mutate, sqlite_query, table_convert,
    table_query,
    trigger::{
        function as trigger_function, kv_subscribe, manual, mcp_trigger, schedule, weberror,
        webhook, ws_client as trigger_ws_client,
    },
    web_docs_generate, web_response, web_static_generate, web_static_site, ws_client_send, ws_emit,
    ws_sync_state, ws_trigger,
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

#[derive(Debug, Clone, PartialEq, Eq)]
enum NodesAccess {
    None,
    Exact(HashSet<String>),
}

impl Default for NodesAccess {
    fn default() -> Self {
        Self::None
    }
}

#[derive(Debug, Clone, Default)]
struct NodesRetentionPlan {
    consumer_access: HashMap<String, NodesAccess>,
    retained_nodes: HashSet<String>,
}

const RETRY_STATE_KEY: &str = "__zf_retry";

fn hash_markup(s: &str) -> u64 {
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

fn build_nodes_retention_plan(graph: &PipelineGraph) -> Result<NodesRetentionPlan, PipelineError> {
    let mut plan = NodesRetentionPlan::default();

    for node in &graph.nodes {
        let access = scan_node_nodes_access(node)?;
        match &access {
            NodesAccess::None => {}
            NodesAccess::Exact(ids) => {
                plan.retained_nodes.extend(ids.iter().cloned());
            }
        }
        plan.consumer_access.insert(node.id.clone(), access);
    }

    Ok(plan)
}

fn scan_node_nodes_access(node: &PipelineNode) -> Result<NodesAccess, PipelineError> {
    let mut access = NodesAccessAccumulator::default();
    scan_value_nodes_access(node, &node.config, &mut access)?;
    Ok(access.finish())
}

#[derive(Debug, Default)]
struct NodesAccessAccumulator {
    refs: HashSet<String>,
}

impl NodesAccessAccumulator {
    fn mark_ref(&mut self, id: String) {
        self.refs.insert(id);
    }

    fn finish(self) -> NodesAccess {
        if self.refs.is_empty() {
            NodesAccess::None
        } else {
            NodesAccess::Exact(self.refs)
        }
    }
}

fn scan_value_nodes_access(
    node: &PipelineNode,
    value: &Value,
    access: &mut NodesAccessAccumulator,
) -> Result<(), PipelineError> {
    match value {
        Value::String(text) => scan_text_nodes_access(node, text, access)?,
        Value::Array(items) => {
            for item in items {
                scan_value_nodes_access(node, item, access)?;
            }
        }
        Value::Object(map) => {
            for item in map.values() {
                scan_value_nodes_access(node, item, access)?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn scan_text_nodes_access(
    node: &PipelineNode,
    text: &str,
    access: &mut NodesAccessAccumulator,
) -> Result<(), PipelineError> {
    scan_nodes_marker(node, text, "$nodes", access)?;
    scan_nodes_marker(node, text, "ctx.nodes", access)?;
    Ok(())
}

fn scan_nodes_marker(
    node: &PipelineNode,
    text: &str,
    marker: &str,
    access: &mut NodesAccessAccumulator,
) -> Result<(), PipelineError> {
    let mut offset = 0;
    while let Some(found) = text[offset..].find(marker) {
        let marker_start = offset + found;
        let marker_end = marker_start + marker.len();
        if !marker_boundary_ok(text, marker_start, marker_end) {
            offset = marker_end;
            continue;
        }

        match parse_nodes_reference_after_marker(text, marker_end) {
            NodesReferenceScan::Exact(id, next) => {
                access.mark_ref(id);
                offset = next;
            }
            NodesReferenceScan::Dynamic(_) => {
                return Err(PipelineError::new(
                    "FW_NODES_SCOPE_DYNAMIC",
                    format!(
                        "node '{}' uses dynamic {} access; use a literal node id like {}.foo or {}[\"foo\"]",
                        node.id, marker, marker, marker
                    ),
                ));
            }
            NodesReferenceScan::NoReference(next) => {
                offset = next;
            }
        }
    }
    Ok(())
}

fn marker_boundary_ok(text: &str, start: usize, end: usize) -> bool {
    let before_ok = text[..start]
        .chars()
        .next_back()
        .is_none_or(|ch| !is_js_ident_continue(ch));
    let after_ok = text[end..]
        .chars()
        .next()
        .is_none_or(|ch| !is_js_ident_continue(ch));
    before_ok && after_ok
}

#[derive(Debug, PartialEq, Eq)]
enum NodesReferenceScan {
    Exact(String, usize),
    Dynamic(usize),
    NoReference(usize),
}

fn parse_nodes_reference_after_marker(text: &str, marker_end: usize) -> NodesReferenceScan {
    let mut idx = skip_ascii_ws(text, marker_end);
    let Some(ch) = text[idx..].chars().next() else {
        return NodesReferenceScan::NoReference(idx);
    };

    if ch == '.' {
        idx += ch.len_utf8();
        let start = idx;
        while let Some(next) = text[idx..].chars().next() {
            if is_js_ident_continue(next) || next == '-' {
                idx += next.len_utf8();
            } else {
                break;
            }
        }
        if idx == start {
            return NodesReferenceScan::Dynamic(idx);
        }
        return NodesReferenceScan::Exact(text[start..idx].to_string(), idx);
    }

    if ch == '[' {
        return parse_bracket_nodes_reference(text, idx);
    }

    NodesReferenceScan::Dynamic(idx)
}

fn parse_bracket_nodes_reference(text: &str, bracket_start: usize) -> NodesReferenceScan {
    let mut idx = skip_ascii_ws(text, bracket_start + 1);
    let Some(quote) = text[idx..].chars().next() else {
        return NodesReferenceScan::Dynamic(idx);
    };
    if quote != '\'' && quote != '"' {
        return NodesReferenceScan::Dynamic(idx);
    }
    idx += quote.len_utf8();
    let value_start = idx;
    let mut escaped = false;
    let mut out = String::new();
    while let Some(ch) = text[idx..].chars().next() {
        idx += ch.len_utf8();
        if escaped {
            out.push(ch);
            escaped = false;
            continue;
        }
        if ch == '\\' {
            escaped = true;
            continue;
        }
        if ch == quote {
            let after_quote = skip_ascii_ws(text, idx);
            if text[after_quote..].starts_with(']') {
                let id = if out.is_empty() {
                    text[value_start..idx - quote.len_utf8()].to_string()
                } else {
                    out
                };
                if id.trim().is_empty() {
                    return NodesReferenceScan::Dynamic(after_quote + 1);
                }
                return NodesReferenceScan::Exact(id, after_quote + 1);
            }
            return NodesReferenceScan::Dynamic(after_quote);
        }
        out.push(ch);
    }
    NodesReferenceScan::Dynamic(idx)
}

fn skip_ascii_ws(text: &str, mut idx: usize) -> usize {
    while let Some(ch) = text[idx..].chars().next() {
        if ch.is_ascii_whitespace() {
            idx += ch.len_utf8();
        } else {
            break;
        }
    }
    idx
}

fn is_js_ident_continue(ch: char) -> bool {
    ch == '_' || ch == '$' || ch.is_ascii_alphanumeric()
}

fn nodes_scope_for_target(
    target_node_id: &str,
    nodes_output: &serde_json::Map<String, Value>,
    retention: &NodesRetentionPlan,
) -> Value {
    match retention
        .consumer_access
        .get(target_node_id)
        .unwrap_or(&NodesAccess::None)
    {
        NodesAccess::None => json!({}),
        NodesAccess::Exact(ids) => {
            let mut scoped = serde_json::Map::new();
            for id in ids {
                if let Some(value) = nodes_output.get(id) {
                    scoped.insert(id.clone(), value.clone());
                }
            }
            Value::Object(scoped)
        }
    }
}

fn should_retain_node_output(node_id: &str, retention: &NodesRetentionPlan) -> bool {
    retention.retained_nodes.contains(node_id)
}

fn execution_metadata(
    ctx: &PipelineContext,
    nodes_scope: Value,
    placeholder: Option<Value>,
) -> Value {
    json!({
        "owner": ctx.owner,
        "project": ctx.project,
        "pipeline": ctx.pipeline,
        "request_id": ctx.request_id,
        "route": ctx.route,
        "trigger": ctx.trigger,
        "nodes": nodes_scope,
        "placeholder": placeholder,
    })
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

const TRACE_SUMMARY_MAX_DEPTH: usize = 8;
const TRACE_SUMMARY_MAX_OBJECT_KEYS: usize = 48;
const TRACE_SUMMARY_MAX_ARRAY_ITEMS: usize = 16;
const TRACE_SUMMARY_PREVIEW_ITEMS: usize = 6;
const TRACE_SUMMARY_NUMERIC_ARRAY_THRESHOLD: usize = 64;
const TRACE_SUMMARY_MAX_STRING_CHARS: usize = 8192;
const TRACE_SUMMARY_STRING_PREVIEW_CHARS: usize = 512;

fn summarize_trace_value(value: &Value) -> Value {
    summarize_trace_value_inner(value, 0)
}

fn summarize_trace_value_inner(value: &Value, depth: usize) -> Value {
    if depth >= TRACE_SUMMARY_MAX_DEPTH {
        return match value {
            Value::Object(map) => json!({
                "__zf_trace_summary": "object",
                "keys": map.len(),
            }),
            Value::Array(items) => json!({
                "__zf_trace_summary": "array",
                "len": items.len(),
            }),
            Value::String(text) => summarize_trace_string(text),
            other => other.clone(),
        };
    }

    match value {
        Value::String(text) if text.chars().count() > TRACE_SUMMARY_MAX_STRING_CHARS => {
            summarize_trace_string(text)
        }
        Value::Array(items)
            if items.len() >= TRACE_SUMMARY_NUMERIC_ARRAY_THRESHOLD
                && items.iter().all(Value::is_number) =>
        {
            let preview = items
                .iter()
                .take(TRACE_SUMMARY_PREVIEW_ITEMS)
                .cloned()
                .collect::<Vec<_>>();
            let tail = items
                .last()
                .cloned()
                .map(|item| vec![item])
                .unwrap_or_default();
            json!({
                "__zf_trace_summary": "numeric_array",
                "len": items.len(),
                "preview": preview,
                "tail": tail,
            })
        }
        Value::Array(items) if items.len() > TRACE_SUMMARY_MAX_ARRAY_ITEMS => {
            let preview = items
                .iter()
                .take(TRACE_SUMMARY_PREVIEW_ITEMS)
                .map(|item| summarize_trace_value_inner(item, depth + 1))
                .collect::<Vec<_>>();
            json!({
                "__zf_trace_summary": "array",
                "len": items.len(),
                "preview": preview,
            })
        }
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| summarize_trace_value_inner(item, depth + 1))
                .collect(),
        ),
        Value::Object(map) if map.len() > TRACE_SUMMARY_MAX_OBJECT_KEYS => {
            let preview = map
                .iter()
                .take(TRACE_SUMMARY_MAX_OBJECT_KEYS)
                .map(|(key, item)| (key.clone(), summarize_trace_value_inner(item, depth + 1)))
                .collect::<serde_json::Map<_, _>>();
            json!({
                "__zf_trace_summary": "object",
                "keys": map.len(),
                "preview": Value::Object(preview),
            })
        }
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, item)| (key.clone(), summarize_trace_value_inner(item, depth + 1)))
                .collect(),
        ),
        other => other.clone(),
    }
}

fn summarize_trace_string(text: &str) -> Value {
    let preview = text
        .chars()
        .take(TRACE_SUMMARY_STRING_PREVIEW_CHARS)
        .collect::<String>();
    json!({
        "__zf_trace_summary": "string",
        "chars": text.chars().count(),
        "preview": preview,
    })
}

fn sanitized_trace_value(value: &Value) -> Value {
    let mut payload = value.clone();
    let mut tokens = take_private_trace_redact_tokens(&mut payload);
    tokens.extend(take_private_redact_tokens(&mut payload));
    let except_paths = take_private_redact_except_paths(&mut payload);
    let redacted = if tokens.is_empty() {
        payload
    } else {
        redact_json_value(&payload, &tokens, &except_paths, &[])
    };
    summarize_trace_value(&redacted)
}

/// Main framework engine used for real pipeline execution.
pub struct BasicPipelineEngine {
    language: Arc<dyn LanguageEngine>,
    rwe: Arc<dyn ReactiveWebEngine>,
    credentials: Option<Arc<CredentialService>>,
    template_cache: Option<TemplateCache>,
    ws_hub: Option<Arc<WsHub>>,
    ws_client_manager: Option<Arc<WsClientManager>>,
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
            ws_client_manager: None,
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
            ws_client_manager: None,
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

    /// Attach the WS client manager so n.ws.client.send nodes can send through outbound connections.
    pub fn with_ws_client_manager(mut self, mgr: Arc<WsClientManager>) -> Self {
        self.ws_client_manager = Some(mgr);
        self
    }

    /// Attach the state bus so `n.kv.*` nodes can access the shared project-scoped KV/pubsub layer.
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
                self.platform.clone(),
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
                    self.language.clone(),
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
                    self.language.clone(),
                )?))
            }
            sekejap_insert::NODE_KIND => {
                let Some(data_root) = &self.data_root else {
                    return Err(PipelineError::new(
                        "FW_NODE_SEKEJAP_UNAVAILABLE",
                        "data_root is not configured on this pipeline engine",
                    ));
                };
                Ok(NodeDispatch::SekejapInsert(sekejap_insert::Node::new(
                    serde_json::from_value(node.config.clone()).map_err(|err| {
                        PipelineError::new("FW_NODE_SEKEJAP_INSERT_CONFIG", err.to_string())
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
                    self.language.clone(),
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
            table_query::NODE_KIND => {
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        "FW_NODE_TABLE_QUERY_UNAVAILABLE",
                        "platform service is not configured on this pipeline engine",
                    ));
                };
                Ok(NodeDispatch::TableQuery(table_query::Node::new(
                    serde_json::from_value(node.config.clone()).map_err(|err| {
                        PipelineError::new("FW_NODE_TABLE_QUERY_CONFIG", err.to_string())
                    })?,
                    platform.clone(),
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
                let config: agent::Config = serde_json::from_value(node.config.clone())
                    .map_err(|err| PipelineError::new("FW_NODE_AGENT_CONFIG", err.to_string()))?;
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
            logic::foreach_::NODE_KIND => {
                Ok(NodeDispatch::LogicForeach(logic::foreach_::Node::new(
                    serde_json::from_value(node.config.clone()).map_err(|e| {
                        PipelineError::new("FW_NODE_LOGIC_FOREACH_CONFIG", e.to_string())
                    })?,
                    self.language.clone(),
                )?))
            }
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
            fs_save::NODE_KIND => {
                let config: fs_save::Config =
                    serde_json::from_value(node.config.clone()).unwrap_or_default();
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        "FW_NODE_FILE_SAVE",
                        "platform service not available in this engine context",
                    ));
                };
                Ok(NodeDispatch::FileSave(fs_save::Node::new(
                    config,
                    platform.clone(),
                )?))
            }
            fs_object::LIST_NODE_KIND
            | fs_object::HEAD_NODE_KIND
            | fs_object::GET_NODE_KIND
            | fs_object::PUT_NODE_KIND
            | fs_object::DELETE_NODE_KIND
            | fs_object::COPY_NODE_KIND
            | fs_object::MOVE_NODE_KIND
            | fs_object::MKDIR_NODE_KIND => {
                let config: fs_object::Config =
                    serde_json::from_value(node.config.clone()).unwrap_or_default();
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        "FW_NODE_FS_OBJECT",
                        "platform service not available in this engine context",
                    ));
                };
                let operation = match node.kind.as_str() {
                    fs_object::LIST_NODE_KIND => fs_object::Operation::List,
                    fs_object::HEAD_NODE_KIND => fs_object::Operation::Head,
                    fs_object::GET_NODE_KIND => fs_object::Operation::Get,
                    fs_object::PUT_NODE_KIND => fs_object::Operation::Put,
                    fs_object::DELETE_NODE_KIND => fs_object::Operation::Delete,
                    fs_object::COPY_NODE_KIND => fs_object::Operation::Copy,
                    fs_object::MOVE_NODE_KIND => fs_object::Operation::Move,
                    fs_object::MKDIR_NODE_KIND => fs_object::Operation::Mkdir,
                    _ => unreachable!(),
                };
                Ok(NodeDispatch::FsObject(fs_object::Node::new(
                    config,
                    platform.clone(),
                    operation,
                )?))
            }
            mapserver_crud::PUBLISH_KIND
            | mapserver_crud::UNPUBLISH_KIND
            | mapserver_crud::GET_KIND
            | mapserver_crud::LIST_KIND => {
                let config: mapserver_crud::Config =
                    serde_json::from_value(node.config.clone()).unwrap_or_default();
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        "FW_NODE_MS",
                        "platform service not available in this engine context",
                    ));
                };
                let operation = match node.kind.as_str() {
                    mapserver_crud::PUBLISH_KIND => mapserver_crud::Operation::Publish,
                    mapserver_crud::UNPUBLISH_KIND => mapserver_crud::Operation::Unpublish,
                    mapserver_crud::GET_KIND => mapserver_crud::Operation::Get,
                    mapserver_crud::LIST_KIND => mapserver_crud::Operation::List,
                    _ => unreachable!(),
                };
                Ok(NodeDispatch::MapserverCrud(mapserver_crud::Node::new(
                    config,
                    platform.clone(),
                    operation,
                )?))
            }
            table_convert::NODE_KIND => {
                let config: table_convert::Config =
                    serde_json::from_value(node.config.clone()).unwrap_or_default();
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        "FW_NODE_TABLE_CONVERT",
                        "platform service not available in this engine context",
                    ));
                };
                Ok(NodeDispatch::TableConvert(table_convert::Node::new(
                    config,
                    platform.clone(),
                    self.language.clone(),
                )?))
            }
            fs_compress::NODE_KIND => {
                let config: fs_compress::Config =
                    serde_json::from_value(node.config.clone()).unwrap_or_default();
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        "FW_NODE_FILE_COMPRESS",
                        "platform service not available in this engine context",
                    ));
                };
                Ok(NodeDispatch::FileCompress(fs_compress::Node::new(
                    config,
                    platform.clone(),
                )?))
            }
            fs_decompress::NODE_KIND => {
                let config: fs_decompress::Config =
                    serde_json::from_value(node.config.clone()).unwrap_or_default();
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        "FW_NODE_FILE_DECOMPRESS",
                        "platform service not available in this engine context",
                    ));
                };
                Ok(NodeDispatch::FileDecompress(fs_decompress::Node::new(
                    config,
                    platform.clone(),
                )?))
            }
            geo_inspect::NODE_KIND => {
                let config: geo_inspect::Config =
                    serde_json::from_value(node.config.clone()).unwrap_or_default();
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        "FW_NODE_GEO_INSPECT",
                        "platform service not available in this engine context",
                    ));
                };
                Ok(NodeDispatch::GeoInspect(geo_inspect::Node::new(
                    config,
                    platform.clone(),
                )?))
            }
            geo_convert::NODE_KIND => {
                let config: geo_convert::Config =
                    serde_json::from_value(node.config.clone()).unwrap_or_default();
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        "FW_NODE_GEO_CONVERT",
                        "platform service not available in this engine context",
                    ));
                };
                Ok(NodeDispatch::GeoConvert(geo_convert::Node::new(
                    config,
                    platform.clone(),
                )?))
            }
            fs_pdf_convert::NODE_KIND => {
                let config: fs_pdf_convert::Config =
                    serde_json::from_value(node.config.clone()).unwrap_or_default();
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        "FW_NODE_PDF_CONVERT",
                        "platform service not available in this engine context",
                    ));
                };
                Ok(NodeDispatch::FilePdfConvert(fs_pdf_convert::Node::new(
                    config,
                    platform.clone(),
                )?))
            }
            fs_thumbnail::NODE_KIND => {
                let config: fs_thumbnail::Config =
                    serde_json::from_value(node.config.clone()).unwrap_or_default();
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        "IMG_THUMBNAIL",
                        "platform service not available in this engine context",
                    ));
                };
                Ok(NodeDispatch::ImgThumbnail(fs_thumbnail::Node::new(
                    config,
                    platform.clone(),
                )?))
            }
            kv_set::NODE_KIND => {
                let Some(state_bus) = &self.state_bus else {
                    return Err(PipelineError::new(
                        "FW_NODE_MEM_UNAVAILABLE",
                        "state bus is not configured on this pipeline engine",
                    ));
                };
                Ok(NodeDispatch::KvSet(kv_set::Node::new(
                    serde_json::from_value(node.config.clone())
                        .map_err(|e| PipelineError::new("FW_NODE_KV_SET_CONFIG", e.to_string()))?,
                    state_bus.clone(),
                )))
            }
            kv_get::NODE_KIND => {
                let Some(state_bus) = &self.state_bus else {
                    return Err(PipelineError::new(
                        "FW_NODE_MEM_UNAVAILABLE",
                        "state bus is not configured on this pipeline engine",
                    ));
                };
                Ok(NodeDispatch::KvGet(kv_get::Node::new(
                    serde_json::from_value(node.config.clone())
                        .map_err(|e| PipelineError::new("FW_NODE_KV_GET_CONFIG", e.to_string()))?,
                    state_bus.clone(),
                )))
            }
            kv_del::NODE_KIND => {
                let Some(state_bus) = &self.state_bus else {
                    return Err(PipelineError::new(
                        "FW_NODE_MEM_UNAVAILABLE",
                        "state bus is not configured on this pipeline engine",
                    ));
                };
                Ok(NodeDispatch::KvDel(kv_del::Node::new(
                    serde_json::from_value(node.config.clone())
                        .map_err(|e| PipelineError::new("FW_NODE_KV_DEL_CONFIG", e.to_string()))?,
                    state_bus.clone(),
                )))
            }
            kv_incr::NODE_KIND => {
                let Some(state_bus) = &self.state_bus else {
                    return Err(PipelineError::new(
                        "FW_NODE_MEM_UNAVAILABLE",
                        "state bus is not configured on this pipeline engine",
                    ));
                };
                Ok(NodeDispatch::KvIncr(kv_incr::Node::new(
                    serde_json::from_value(node.config.clone())
                        .map_err(|e| PipelineError::new("FW_NODE_KV_INCR_CONFIG", e.to_string()))?,
                    state_bus.clone(),
                )))
            }
            kv_publish::NODE_KIND => {
                let Some(state_bus) = &self.state_bus else {
                    return Err(PipelineError::new(
                        "FW_NODE_MEM_UNAVAILABLE",
                        "state bus is not configured on this pipeline engine",
                    ));
                };
                Ok(NodeDispatch::KvPublish(kv_publish::Node::new(
                    serde_json::from_value(node.config.clone()).map_err(|e| {
                        PipelineError::new("FW_NODE_KV_PUBLISH_CONFIG", e.to_string())
                    })?,
                    state_bus.clone(),
                )))
            }
            kv_exists::NODE_KIND => {
                let Some(state_bus) = &self.state_bus else {
                    return Err(PipelineError::new(
                        "FW_NODE_MEM_UNAVAILABLE",
                        "state bus is not configured on this pipeline engine",
                    ));
                };
                Ok(NodeDispatch::KvExists(kv_exists::Node::new(
                    serde_json::from_value(node.config.clone()).map_err(|e| {
                        PipelineError::new("FW_NODE_KV_EXISTS_CONFIG", e.to_string())
                    })?,
                    state_bus.clone(),
                )))
            }
            kv_expire::NODE_KIND => {
                let Some(state_bus) = &self.state_bus else {
                    return Err(PipelineError::new(
                        "FW_NODE_MEM_UNAVAILABLE",
                        "state bus is not configured on this pipeline engine",
                    ));
                };
                Ok(NodeDispatch::KvExpire(kv_expire::Node::new(
                    serde_json::from_value(node.config.clone()).map_err(|e| {
                        PipelineError::new("FW_NODE_KV_EXPIRE_CONFIG", e.to_string())
                    })?,
                    state_bus.clone(),
                )))
            }
            mcp_trigger::NODE_KIND => Ok(NodeDispatch::McpTrigger(mcp_trigger::Node::new(
                serde_json::from_value(node.config.clone()).map_err(|err| {
                    PipelineError::new("FW_NODE_MCP_TRIGGER_CONFIG", err.to_string())
                })?,
            ))),
            kv_subscribe::NODE_KIND => Ok(NodeDispatch::KvSubscribe(kv_subscribe::Node::new(
                serde_json::from_value(node.config.clone()).map_err(|e| {
                    PipelineError::new("FW_NODE_KV_SUBSCRIBE_CONFIG", e.to_string())
                })?,
            ))),
            trigger_ws_client::NODE_KIND => Ok(NodeDispatch::WsClientTrigger(
                trigger_ws_client::Node::new(serde_json::from_value(node.config.clone()).map_err(
                    |e| PipelineError::new("FW_NODE_WS_CLIENT_TRIGGER_CONFIG", e.to_string()),
                )?),
            )),
            ws_client_send::NODE_KIND => {
                let Some(ws_client_manager) = &self.ws_client_manager else {
                    return Err(PipelineError::new(
                        "FW_NODE_WS_CLIENT_SEND_UNAVAILABLE",
                        "ws client manager is not configured on this framework engine",
                    ));
                };
                Ok(NodeDispatch::WsClientSend(ws_client_send::Node::new(
                    serde_json::from_value(node.config.clone()).map_err(|e| {
                        PipelineError::new("FW_NODE_WS_CLIENT_SEND_CONFIG", e.to_string())
                    })?,
                    ws_client_manager.clone(),
                )?))
            }
            // Composite trigger nodes act as pipeline entry points.
            // They pass through the webhook payload, optionally running the
            // package's `on_message` transform function.
            other if other.starts_with("n.c.trigger.") => {
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        "FW_NODE_COMPOSITE_NO_PLATFORM",
                        format!(
                            "composite trigger '{}': platform service not injected into engine",
                            other
                        ),
                    ));
                };
                Ok(NodeDispatch::CompositeTrigger {
                    kind: other.to_string(),
                    config: node.config.clone(),
                    platform: platform.clone(),
                })
            }
            other if other.starts_with("n.c.") => {
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        "FW_NODE_COMPOSITE_NO_PLATFORM",
                        format!(
                            "composite node '{}': platform service not injected into engine",
                            other
                        ),
                    ));
                };
                Ok(NodeDispatch::CompositeNode {
                    kind: other.to_string(),
                    config: node.config.clone(),
                    platform: platform.clone(),
                })
            }
            other if other.starts_with("n.wasm.") => Ok(NodeDispatch::WasmNodeStub {
                kind: other.to_string(),
            }),
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
        build_nodes_retention_plan(graph)?;
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

        let bus = options.bus.clone();
        // nodes_output: accumulates only completed node outputs required by the
        // graph's `$nodes`/`ctx.nodes` retention plan.
        // Declared here (before the initial queue push) so entry-node metadata can include it.
        let nodes_retention = build_nodes_retention_plan(graph)?;
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
                metadata: execution_metadata(
                    ctx,
                    nodes_scope_for_target(&node.id, &nodes_output, &nodes_retention),
                    ctx.placeholder.clone(),
                ),
                bus: bus.clone(),
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
            // Priority: node config `timeout_secs` → project config → env var → default(30s).
            let project_timeout_secs: u64 = self
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
            let node_timeout_secs: u64 = effective_config
                .get("timeout_secs")
                .and_then(|v| {
                    v.as_u64()
                        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                })
                .map(|v| v.clamp(5, 3600))
                .unwrap_or(project_timeout_secs);
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
                    NodeDispatch::SekejapInsert(node) => {
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
                            let url = format!("/fs/{}/{}/{}", ctx.owner, ctx.project, rel_path);
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
                    NodeDispatch::FsObject(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::MapserverCrud(node) => {
                        node.execute_many_async(input_for_exec).await
                    }
                    NodeDispatch::TableConvert(node) => {
                        node.execute_many_async(input_for_exec).await
                    }
                    NodeDispatch::TableQuery(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::FileCompress(node) => {
                        node.execute_many_async(input_for_exec).await
                    }
                    NodeDispatch::FileDecompress(node) => {
                        node.execute_many_async(input_for_exec).await
                    }
                    NodeDispatch::GeoInspect(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::GeoConvert(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::FilePdfConvert(node) => {
                        node.execute_many_async(input_for_exec).await
                    }
                    NodeDispatch::ImgThumbnail(node) => {
                        node.execute_many_async(input_for_exec).await
                    }
                    NodeDispatch::KvSet(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::KvGet(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::KvDel(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::KvExists(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::KvExpire(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::KvIncr(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::KvPublish(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::McpTrigger(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::KvSubscribe(node) => {
                        node.execute_many_async(input_for_exec).await
                    }
                    NodeDispatch::WsClientTrigger(node) => {
                        node.execute_many_async(input_for_exec).await
                    }
                    NodeDispatch::WsClientSend(node) => {
                        node.execute_many_async(input_for_exec).await
                    }
                    NodeDispatch::CompositeTrigger {
                        kind,
                        config,
                        platform,
                    } => {
                        execute_composite_trigger(&kind, &config, &platform, vec![input_for_exec])
                            .await
                    }
                    NodeDispatch::CompositeNode {
                        kind,
                        config,
                        platform,
                    } => {
                        execute_composite_node(&kind, &config, &platform, vec![input_for_exec])
                            .await
                    }
                    NodeDispatch::WasmNodeStub { kind } => Err(PipelineError::new(
                        "FW_NODE_WASM_UNAVAILABLE",
                        format!(
                            "WASM runtime not available. Node '{}' requires Extism, \
                             planned for a future release.",
                            kind
                        ),
                    )),
                }
            }; // end exec_fut
            let timeout_node_id = trace_node_id.clone();
            let timeout_is_per_node = effective_config.get("timeout_secs").is_some();
            let exec_result: Result<Vec<NodeExecutionOutput>, PipelineError> =
                tokio::time::timeout(std::time::Duration::from_secs(node_timeout_secs), exec_fut)
                    .await
                    .unwrap_or_else(|_| {
                        let source = if timeout_is_per_node {
                            "per-node"
                        } else {
                            "project-level"
                        };
                        Err(PipelineError::new(
                            "FW_NODE_TIMEOUT",
                            format!(
                                "node '{}' timed out after {node_timeout_secs}s ({source} timeout)",
                                timeout_node_id
                            ),
                        ))
                    });

            let outputs = match exec_result {
                Ok(mut outs) => {
                    let mut processed_payloads: Vec<Value> = Vec::new();
                    let mut redacted_outputs: Vec<Value> = Vec::new();
                    let trace_input = sanitized_trace_value(&input_snapshot);

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
                        redacted_outputs.push(sanitized_trace_value(&payload_output));
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
                    if should_retain_node_output(&trace_node_id, &nodes_retention) {
                        nodes_output.insert(trace_node_id.clone(), nodes_output_value);
                    }
                    node_trace.push(NodeTraceEntry {
                        node_id: trace_node_id.clone(),
                        node_kind: trace_node_kind.clone(),
                        config: redacted_config,
                        duration_ms: node_start.elapsed().as_millis() as u64,
                        input: trace_input,
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
                        input: sanitized_trace_value(&input_snapshot),
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
                                metadata: execution_metadata(
                                    ctx,
                                    nodes_scope_for_target(
                                        to_node,
                                        &nodes_output,
                                        &nodes_retention,
                                    ),
                                    ctx.placeholder.clone(),
                                ),
                                bus: bus.clone(),
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

            for mut output in outputs {
                // ── __signal extraction ──────────────────────────────────────
                // Strip `__signal` from node output and route through the bus.
                // Supports: string, object {kind,message,data}, or array of either.
                if let Some(raw_signal) = output
                    .payload
                    .as_object_mut()
                    .and_then(|m| m.remove("__signal"))
                {
                    if let Some(ref bus) = bus {
                        let emit = |s: &Value| {
                            let signal = match s {
                                Value::String(msg) => Signal {
                                    kind: "signal".to_string(),
                                    message: msg.clone(),
                                    node_id: trace_node_id.clone(),
                                    node_kind: trace_node_kind.clone(),
                                    data: None,
                                    at: format!("{}ms", node_start.elapsed().as_millis()),
                                },
                                Value::Object(obj) => Signal {
                                    kind: obj
                                        .get("kind")
                                        .and_then(Value::as_str)
                                        .unwrap_or("signal")
                                        .to_string(),
                                    message: obj
                                        .get("message")
                                        .and_then(Value::as_str)
                                        .unwrap_or("")
                                        .to_string(),
                                    node_id: trace_node_id.clone(),
                                    node_kind: trace_node_kind.clone(),
                                    data: obj.get("data").cloned(),
                                    at: format!("{}ms", node_start.elapsed().as_millis()),
                                },
                                _ => return,
                            };
                            bus.emit(signal);
                        };
                        if let Value::Array(items) = &raw_signal {
                            for item in items {
                                emit(item);
                            }
                        } else {
                            emit(&raw_signal);
                        }
                    }
                }
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
                                        metadata: execution_metadata(
                                            ctx,
                                            nodes_scope_for_target(
                                                to_node,
                                                &nodes_output,
                                                &nodes_retention,
                                            ),
                                            ctx.placeholder.clone(),
                                        ),
                                        bus: bus.clone(),
                                    });
                                }
                            } else {
                                queue.push_back(NodeExecutionInput {
                                    node_id: (*to_node).to_string(),
                                    input_pin: (*to_pin).to_string(),
                                    payload: output.payload.clone(),
                                    metadata: execution_metadata(
                                        ctx,
                                        nodes_scope_for_target(
                                            to_node,
                                            &nodes_output,
                                            &nodes_retention,
                                        ),
                                        ctx.placeholder.clone(),
                                    ),
                                    bus: bus.clone(),
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
    use std::sync::Arc;

    use serde_json::json;

    use super::{
        BasicPipelineEngine, NodesAccess, NodesAccessAccumulator, PipelineContext,
        redact_json_value, sanitized_trace_value, scan_text_nodes_access,
        take_private_redact_except_paths, take_private_redact_tokens,
        take_private_trace_redact_tokens, trace_config_snapshot,
    };
    use crate::pipeline::interface::PipelineEngine;
    use crate::pipeline::model::{PipelineEdge, PipelineGraph, PipelineNode};
    use crate::pipeline::nodes::basic::{
        script,
        table_convert::{TableFormat, collect_columns, encode_rows},
    };
    use crate::platform::model::PlatformConfig;
    use crate::platform::services::PlatformService;
    use crate::platform::shell::parser::build_pipeline_graph;
    use crate::zebfs::LocalZebFs;

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

    #[test]
    fn trace_summary_summarizes_large_numeric_arrays_generically() {
        let vector = (0..128).map(|i| json!(i as f64 / 10.0)).collect::<Vec<_>>();
        let summary = sanitized_trace_value(&json!({
            "id": "row-1",
            "values": vector,
            "nested": {
                "items": [
                    { "payload": (0..80).map(|i| json!(i)).collect::<Vec<_>>() }
                ]
            }
        }));

        assert_eq!(summary["id"], "row-1");
        assert_eq!(summary["values"]["__zf_trace_summary"], "numeric_array");
        assert_eq!(summary["values"]["len"], 128);
        assert_eq!(
            summary["nested"]["items"][0]["payload"]["__zf_trace_summary"],
            "numeric_array"
        );
        assert_eq!(summary["nested"]["items"][0]["payload"]["len"], 80);
    }

    #[test]
    fn nodes_access_scanner_detects_literal_references() {
        let mut access = NodesAccessAccumulator::default();
        scan_text_nodes_access(
            &PipelineNode {
                id: "reader".to_string(),
                kind: script::NODE_KIND.to_string(),
                input_pins: vec!["in".to_string()],
                output_pins: vec!["out".to_string()],
                config: json!({}),
            },
            "return ctx.nodes.prepare.id + $nodes['geo-convert'].path + $nodes.match.value;",
            &mut access,
        )
        .expect("scan exact refs");
        let result = access.finish();
        let NodesAccess::Exact(ids) = result else {
            panic!("expected exact node references");
        };
        assert!(ids.contains("prepare"));
        assert!(ids.contains("geo-convert"));
        assert!(ids.contains("match"));
    }

    #[test]
    fn nodes_access_scanner_rejects_dynamic_references() {
        let mut dynamic = NodesAccessAccumulator::default();
        let err = scan_text_nodes_access(
            &PipelineNode {
                id: "reader".to_string(),
                kind: script::NODE_KIND.to_string(),
                input_pins: vec!["in".to_string()],
                output_pins: vec!["out".to_string()],
                config: json!({}),
            },
            "const key = input.key; return ctx.nodes[key];",
            &mut dynamic,
        )
        .expect_err("dynamic node access should be rejected");
        assert_eq!(err.code, "FW_NODES_SCOPE_DYNAMIC");
    }

    #[tokio::test]
    async fn nodes_scope_only_carries_referenced_upstream_outputs() {
        let dsl = r#"
[a] trigger.manual
[b] script -- "return { big: 'huge-marker-that-should-not-leak' };"
[c] script -- "return { small: 7 };"
[d] script -- "return { small: ctx.nodes.c.small, leaked: JSON.stringify(ctx).includes('huge-marker-that-should-not-leak') };"

[a] -> [b]
[b] -> [c]
[c] -> [d]
"#;

        let graph = build_pipeline_graph("nodes-retention-exact-test", dsl).expect("graph");
        let engine = BasicPipelineEngine::default();
        let out = engine
            .execute_async(
                &graph,
                &PipelineContext {
                    owner: "test".to_string(),
                    project: "test".to_string(),
                    pipeline: "nodes-retention-exact-test".to_string(),
                    request_id: "req-nodes-retention-exact".to_string(),
                    route: String::new(),
                    input: json!({}),
                    trigger: None,
                    placeholder: None,
                },
            )
            .await
            .expect("execute");

        assert_eq!(out.value["small"], 7);
        assert_eq!(out.value["leaked"], false);
    }

    #[tokio::test]
    async fn nodes_scope_rejects_dynamic_script_access() {
        let dsl = r#"
[a] trigger.manual
[b] script -- "return { big: 'dynamic-access-should-not-work' };"
[c] script -- "return { small: 7 };"
[d] script -- "const key = 'b'; return { big: ctx.nodes[key].big, small: ctx.nodes.c.small };"

[a] -> [b]
[b] -> [c]
[c] -> [d]
"#;

        let graph = build_pipeline_graph("nodes-retention-dynamic-test", dsl).expect("graph");
        let engine = BasicPipelineEngine::default();
        let err = engine
            .execute_async(
                &graph,
                &PipelineContext {
                    owner: "test".to_string(),
                    project: "test".to_string(),
                    pipeline: "nodes-retention-dynamic-test".to_string(),
                    request_id: "req-nodes-retention-dynamic".to_string(),
                    route: String::new(),
                    input: json!({}),
                    trigger: None,
                    placeholder: None,
                },
            )
            .await
            .expect_err("dynamic nodes access should fail validation");

        assert_eq!(err.code, "FW_NODES_SCOPE_DYNAMIC");
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
                    placeholder: None,
                },
            )
            .await
            .expect("execute");

        assert_eq!(out.value["b"]["user"]["id"], "u_42");
        assert_eq!(out.value["c"]["orders"][0]["id"], "o_1");
    }

    #[tokio::test]
    async fn fs_object_nodes_cover_s3_like_crud_flow() {
        let data_root = tempfile::tempdir().expect("temp data root");
        let owner = "superadmin";
        let project = "fs_object_e2e";
        let platform = Arc::new(
            PlatformService::from_config(PlatformConfig {
                data_root: data_root.path().to_path_buf(),
                default_password: "secret".to_string(),
                default_project: project.to_string(),
                ..Default::default()
            })
            .expect("platform"),
        );

        let dsl = r#"
[a] trigger.manual
[b] fs.put --path qa/fs/hello.txt --text "hello fs"
[c] fs.get --path qa/fs/hello.txt
[d] fs.copy --from qa/fs/hello.txt --to qa/fs/copy.txt
[e] fs.move --from qa/fs/copy.txt --to qa/fs/moved.txt
[f] fs.list --path qa/fs
[g] fs.delete --path qa/fs/hello.txt
[h] fs.mkdir --path qa/fs/prefix

[a] -> [b]
[b] -> [c]
[c] -> [d]
[d] -> [e]
[e] -> [f]
[f] -> [g]
[g] -> [h]
"#;

        let graph = build_pipeline_graph("fs-object-e2e", dsl).expect("graph");
        let engine = BasicPipelineEngine::default().with_platform(platform.clone());
        let out = engine
            .execute_async(
                &graph,
                &PipelineContext {
                    owner: owner.to_string(),
                    project: project.to_string(),
                    pipeline: "fs-object-e2e".to_string(),
                    request_id: "req-fs-object".to_string(),
                    route: String::new(),
                    input: json!({}),
                    trigger: None,
                    placeholder: None,
                },
            )
            .await
            .expect("execute");

        assert_eq!(out.value["fs"]["operation"], "mkdir");
        assert_eq!(out.value["fs"]["object"]["kind"], "prefix");

        let layout = platform
            .file
            .ensure_project_layout(owner, project)
            .expect("project layout");
        let zebfs = LocalZebFs::new(layout.files_dir);
        let moved = zebfs.get("qa/fs/moved.txt").expect("moved object");
        assert_eq!(String::from_utf8(moved.bytes).expect("utf8"), "hello fs");
        assert!(zebfs.head("qa/fs/prefix").is_ok());
        assert!(zebfs.head("qa/fs/hello.txt").is_err());
        assert!(zebfs.head("qa/fs/copy.txt").is_err());
    }

    #[tokio::test]
    async fn table_convert_parquet_roundtrips_through_pipeline_and_zebfs() {
        let data_root = tempfile::tempdir().expect("temp data root");
        let owner = "superadmin";
        let project = "table_convert_e2e";
        let platform = Arc::new(
            PlatformService::from_config(PlatformConfig {
                data_root: data_root.path().to_path_buf(),
                default_password: "secret".to_string(),
                default_project: project.to_string(),
                ..Default::default()
            })
            .expect("platform"),
        );

        let dsl = r#"
[a] trigger.manual
[b] table.convert --from-expr "$input.rows" --to datasets/posts.parquet
[c] table.convert --from datasets/posts.parquet --to-json --preview 2

[a] -> [b]
[b] -> [c]
"#;

        let graph = build_pipeline_graph("table-convert-parquet-e2e", dsl).expect("graph");
        let engine = BasicPipelineEngine::default().with_platform(platform.clone());
        let out = engine
            .execute_async(
                &graph,
                &PipelineContext {
                    owner: owner.to_string(),
                    project: project.to_string(),
                    pipeline: "table-convert-parquet-e2e".to_string(),
                    request_id: "req-table-convert".to_string(),
                    route: String::new(),
                    input: json!({
                        "rows": [
                            { "id": 1, "title": "First", "score": 1.5, "active": true },
                            { "id": 2, "title": "Second", "score": 2.0, "active": false }
                        ]
                    }),
                    trigger: None,
                    placeholder: None,
                },
            )
            .await
            .expect("execute");

        assert_eq!(out.value["table"]["from_format"], "parquet");
        assert_eq!(out.value["table"]["data"][0]["id"], 1);
        assert_eq!(out.value["table"]["data"][0]["title"], "First");
        assert_eq!(out.value["table"]["data"][0]["score"], 1.5);
        assert_eq!(out.value["table"]["data"][0]["active"], true);
        assert_eq!(out.value["table"]["rows"], 2);

        let layout = platform
            .file
            .ensure_project_layout(owner, project)
            .expect("project layout");
        let object = LocalZebFs::new(layout.files_dir)
            .get("datasets/posts.parquet")
            .expect("parquet object");
        assert!(!object.bytes.is_empty());
    }

    #[tokio::test]
    async fn table_query_joins_multiple_zebfs_sources_with_params() {
        let data_root = tempfile::tempdir().expect("temp data root");
        let owner = "superadmin";
        let project = "table_query_e2e";
        let platform = Arc::new(
            PlatformService::from_config(PlatformConfig {
                data_root: data_root.path().to_path_buf(),
                default_password: "secret".to_string(),
                default_project: project.to_string(),
                ..Default::default()
            })
            .expect("platform"),
        );
        let layout = platform
            .file
            .ensure_project_layout(owner, project)
            .expect("project layout");
        let zebfs = LocalZebFs::new(layout.files_dir);
        zebfs
            .put(
                "datasets/posts.csv",
                b"id,author_id,title\n1,10,First\n2,20,Second\n",
            )
            .expect("posts csv");
        zebfs
            .put("datasets/authors.csv", b"id,name\n10,Ada\n20,Bob\n")
            .expect("authors csv");

        let dsl = r#"
[a] trigger.manual
[b] table.query --from "datasets/posts.csv as posts" --from "datasets/authors.csv as authors" --params-expr "[$input.post_id]" --to-json --preview 1 --sql "select p.id, p.title, a.name from posts p join authors a on p.author_id = a.id where p.id = $1"

[a] -> [b]
"#;

        let graph = build_pipeline_graph("table-query-e2e", dsl).expect("graph");
        let engine = BasicPipelineEngine::default().with_platform(platform.clone());
        let out = engine
            .execute_async(
                &graph,
                &PipelineContext {
                    owner: owner.to_string(),
                    project: project.to_string(),
                    pipeline: "table-query-e2e".to_string(),
                    request_id: "req-table-query".to_string(),
                    route: String::new(),
                    input: json!({ "post_id": 1 }),
                    trigger: None,
                    placeholder: None,
                },
            )
            .await
            .expect("execute");

        assert_eq!(out.value["table"]["engine"], "geodatafusion");
        assert_eq!(out.value["table"]["rows"], 1);
        assert_eq!(out.value["table"]["data"][0]["id"], 1);
        assert_eq!(out.value["table"]["data"][0]["title"], "First");
        assert_eq!(out.value["table"]["data"][0]["name"], "Ada");
        assert_eq!(out.value["table"]["preview"][0]["title"], "First");
    }

    #[tokio::test]
    async fn table_query_joins_multiple_parquet_sources() {
        let data_root = tempfile::tempdir().expect("temp data root");
        let owner = "superadmin";
        let project = "table_query_parquet_join";
        let platform = Arc::new(
            PlatformService::from_config(PlatformConfig {
                data_root: data_root.path().to_path_buf(),
                default_password: "secret".to_string(),
                default_project: project.to_string(),
                ..Default::default()
            })
            .expect("platform"),
        );
        let layout = platform
            .file
            .ensure_project_layout(owner, project)
            .expect("project layout");
        let zebfs = LocalZebFs::new(layout.files_dir);

        let posts = vec![
            json!({ "id": 1, "author_id": 10, "title": "First", "score": 7.5 }),
            json!({ "id": 2, "author_id": 20, "title": "Second", "score": 3.0 }),
            json!({ "id": 3, "author_id": 30, "title": "Third", "score": 9.0 }),
        ];
        let posts_bytes = encode_rows(&posts, &collect_columns(&posts), TableFormat::Parquet)
            .expect("posts parquet");
        zebfs
            .put("datasets/posts.parquet", &posts_bytes)
            .expect("posts parquet put");

        let authors = vec![
            json!({ "id": 10, "name": "Ada", "active": true }),
            json!({ "id": 20, "name": "Bob", "active": false }),
            json!({ "id": 30, "name": "Cora", "active": true }),
        ];
        let authors_bytes = encode_rows(&authors, &collect_columns(&authors), TableFormat::Parquet)
            .expect("authors parquet");
        zebfs
            .put("datasets/authors.parquet", &authors_bytes)
            .expect("authors parquet put");

        let dsl = r#"
[a] trigger.manual
[b] table.query --from "datasets/posts.parquet as posts" --from "datasets/authors.parquet as authors" --to-json --preview 2 --sql "select p.id, p.title, a.name from posts p join authors a on p.author_id = a.id where a.active = true order by p.id"

[a] -> [b]
"#;

        let graph = build_pipeline_graph("table-query-parquet-join", dsl).expect("graph");
        let engine = BasicPipelineEngine::default().with_platform(platform);
        let out = engine
            .execute_async(
                &graph,
                &PipelineContext {
                    owner: owner.to_string(),
                    project: project.to_string(),
                    pipeline: "table-query-parquet-join".to_string(),
                    request_id: "req-table-query-parquet-join".to_string(),
                    route: String::new(),
                    input: json!({}),
                    trigger: None,
                    placeholder: None,
                },
            )
            .await
            .expect("execute");

        assert_eq!(out.value["table"]["engine"], "geodatafusion");
        assert_eq!(out.value["table"]["rows"], 2);
        assert_eq!(out.value["table"]["data"][0]["id"], 1);
        assert_eq!(out.value["table"]["data"][0]["title"], "First");
        assert_eq!(out.value["table"]["data"][0]["name"], "Ada");
        assert_eq!(out.value["table"]["data"][1]["id"], 3);
        assert_eq!(out.value["table"]["data"][1]["title"], "Third");
        assert_eq!(out.value["table"]["data"][1]["name"], "Cora");
    }

    #[tokio::test]
    async fn table_query_accepts_ui_source_binding_rows() {
        let data_root = tempfile::tempdir().expect("temp data root");
        let owner = "superadmin";
        let project = "table_query_ui_rows";
        let platform = Arc::new(
            PlatformService::from_config(PlatformConfig {
                data_root: data_root.path().to_path_buf(),
                default_password: "secret".to_string(),
                default_project: project.to_string(),
                ..Default::default()
            })
            .expect("platform"),
        );
        let layout = platform
            .file
            .ensure_project_layout(owner, project)
            .expect("project layout");
        LocalZebFs::new(layout.files_dir)
            .put("datasets/posts.csv", b"id,title\n1,First\n2,Second\n")
            .expect("posts csv");

        let graph = PipelineGraph {
            kind: "zebflow.pipeline".to_string(),
            version: "0.1".to_string(),
            id: "table-query-ui-rows".to_string(),
            description: None,
            metadata: None,
            entry_nodes: Vec::new(),
            nodes: vec![
                PipelineNode {
                    id: "a".to_string(),
                    kind: "n.trigger.manual".to_string(),
                    input_pins: vec![],
                    output_pins: vec!["out".to_string()],
                    config: json!({}),
                },
                PipelineNode {
                    id: "b".to_string(),
                    kind: "n.table.query".to_string(),
                    input_pins: vec!["in".to_string()],
                    output_pins: vec!["out".to_string()],
                    config: json!({
                        "sources": [
                            { "source": "datasets/posts.csv", "alias": "posts" }
                        ],
                        "query": "select * from posts where id = $1",
                        "params_expr": "[$input.post_id]",
                        "to_json": true,
                        "preview": 1
                    }),
                },
            ],
            edges: vec![PipelineEdge {
                from_node: "a".to_string(),
                from_pin: "out".to_string(),
                to_node: "b".to_string(),
                to_pin: "in".to_string(),
            }],
        };

        let engine = BasicPipelineEngine::default().with_platform(platform);
        let out = engine
            .execute_async(
                &graph,
                &PipelineContext {
                    owner: owner.to_string(),
                    project: project.to_string(),
                    pipeline: "table-query-ui-rows".to_string(),
                    request_id: "req-table-query-ui".to_string(),
                    route: String::new(),
                    input: json!({ "post_id": 2 }),
                    trigger: None,
                    placeholder: None,
                },
            )
            .await
            .expect("execute");

        assert_eq!(out.value["table"]["rows"], 1);
        assert_eq!(out.value["table"]["data"][0]["title"], "Second");
        assert_eq!(out.value["table"]["sources"][0]["alias"], "posts");
    }

    #[tokio::test]
    async fn table_query_geodatafusion_runs_geospatial_sql() {
        let data_root = tempfile::tempdir().expect("temp data root");
        let owner = "superadmin";
        let project = "table_query_geodatafusion";
        let platform = Arc::new(
            PlatformService::from_config(PlatformConfig {
                data_root: data_root.path().to_path_buf(),
                default_password: "secret".to_string(),
                default_project: project.to_string(),
                ..Default::default()
            })
            .expect("platform"),
        );

        let dsl = r#"
[a] trigger.manual
[b] table.query --engine geodatafusion --from "$input.rows as points" --to-json --preview 1 --sql "select id, ST_AsText(ST_Point(x, y)) as geom from points where id = 1"

[a] -> [b]
"#;

        let graph = build_pipeline_graph("table-query-geodatafusion", dsl).expect("graph");
        let engine = BasicPipelineEngine::default().with_platform(platform);
        let out = engine
            .execute_async(
                &graph,
                &PipelineContext {
                    owner: owner.to_string(),
                    project: project.to_string(),
                    pipeline: "table-query-geodatafusion".to_string(),
                    request_id: "req-table-query-geodatafusion".to_string(),
                    route: String::new(),
                    input: json!({
                        "rows": [
                            { "id": 1, "x": 30.0, "y": 10.0 },
                            { "id": 2, "x": 40.0, "y": 20.0 }
                        ],
                    }),
                    trigger: None,
                    placeholder: None,
                },
            )
            .await
            .expect("execute");

        assert_eq!(out.value["table"]["engine"], "geodatafusion");
        assert_eq!(out.value["table"]["rows"], 1);
        assert_eq!(out.value["table"]["data"][0]["id"], 1);
        assert_eq!(out.value["table"]["data"][0]["geom"], "POINT(30 10)");
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
                    placeholder: None,
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
                    placeholder: None,
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
[b] logic.foreach --items-expr "$input.rows"

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
                    placeholder: None,
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
        assert!(out.value.get("rows").is_none());
    }

    #[tokio::test]
    async fn logic_foreach_keep_input_preserves_parent_payload_when_requested() {
        let dsl = r#"
[a] trigger.manual
[b] logic.foreach --items-expr "$input.rows" --keep-input

[a] -> [b]
"#;

        let graph = build_pipeline_graph("logic-foreach-keep-input-test", dsl).expect("graph");
        let engine = BasicPipelineEngine::default();
        let out = engine
            .execute_async(
                &graph,
                &PipelineContext {
                    owner: "test".to_string(),
                    project: "test".to_string(),
                    pipeline: "logic-foreach-keep-input-test".to_string(),
                    request_id: "req-foreach-keep-input".to_string(),
                    route: String::new(),
                    input: json!({
                        "rows": [
                            { "id": "r1" },
                            { "id": "r2" }
                        ],
                        "batch_marker": "kept-only-when-requested"
                    }),
                    trigger: None,
                    placeholder: None,
                },
            )
            .await
            .expect("execute");

        assert_eq!(out.value["item"]["id"], "r2");
        assert_eq!(out.value["batch_marker"], "kept-only-when-requested");
        assert_eq!(out.value["rows"][0]["id"], "r1");
    }

    #[tokio::test]
    async fn logic_reduce_accumulates_foreach_series() {
        let dsl = r#"
[a] trigger.manual
[b] logic.foreach --items-expr "$input.rows"
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
                    placeholder: None,
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
                    placeholder: None,
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
                    placeholder: None,
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
    Script(script::Node),
    HttpRequest(http_request::Node),
    BrowserRun(browser_run::Node),
    SqliteQuery(sqlite_query::Node),
    SekejapQuery(sekejap_query::Node),
    SekejapInsert(sekejap_insert::Node),
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
    FileSave(fs_save::Node),
    FsObject(fs_object::Node),
    MapserverCrud(mapserver_crud::Node),
    TableConvert(table_convert::Node),
    TableQuery(table_query::Node),
    FileCompress(fs_compress::Node),
    FileDecompress(fs_decompress::Node),
    GeoInspect(geo_inspect::Node),
    GeoConvert(geo_convert::Node),
    FilePdfConvert(fs_pdf_convert::Node),
    ImgThumbnail(fs_thumbnail::Node),
    KvSet(kv_set::Node),
    KvGet(kv_get::Node),
    KvDel(kv_del::Node),
    KvExists(kv_exists::Node),
    KvExpire(kv_expire::Node),
    KvIncr(kv_incr::Node),
    KvPublish(kv_publish::Node),
    KvSubscribe(kv_subscribe::Node),
    WsClientTrigger(trigger_ws_client::Node),
    WsClientSend(ws_client_send::Node),
    McpTrigger(mcp_trigger::Node),
    /// Composite trigger node: acts as pipeline entry point, runs `on_message`
    /// transform function from the package manifest on the incoming payload.
    CompositeTrigger {
        kind: String,
        config: serde_json::Value,
        platform: std::sync::Arc<crate::platform::services::PlatformService>,
    },
    /// Composite node: executes an inner function pipeline with placeholder injection.
    CompositeNode {
        kind: String,
        config: serde_json::Value,
        platform: std::sync::Arc<crate::platform::services::PlatformService>,
    },
    /// WASM node stub: returns a clear error until WASM runtime is available.
    WasmNodeStub {
        kind: String,
    },
}

fn is_logic_collect(node: &PipelineNode) -> bool {
    node.kind == logic::collect::NODE_KIND
}

/// Execute a composite node by loading its inner function pipeline and running it.
///
/// Reuses the same pattern as `PlatformService::execute_function_pipeline`:
/// loads the inner pipeline graph, creates a sub-engine, and executes it.
///
/// Executes a composite trigger node (`n.c.trigger.*`).
///
/// Composite triggers act as pipeline entry points. If the package manifest declares
/// an `on_message` function, the raw inbound payload is transformed through that
/// function pipeline. Otherwise the payload passes through unchanged (like
/// `n.trigger.webhook`).
async fn execute_composite_trigger(
    kind: &str,
    config: &Value,
    platform: &Arc<crate::platform::services::PlatformService>,
    inputs: Vec<crate::pipeline::nodes::NodeExecutionInput>,
) -> Result<Vec<crate::pipeline::nodes::NodeExecutionOutput>, PipelineError> {
    use crate::pipeline::nodes::NodeExecutionOutput;

    let mut results = Vec::with_capacity(inputs.len());

    for input in inputs {
        let owner = input
            .metadata
            .get("owner")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let project = input
            .metadata
            .get("project")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        // Check manifest for on_message transform function.
        let on_message_fn = platform
            .node_registry
            .get_manifest(&owner, &project, kind)
            .and_then(|m| m.trigger.and_then(|t| t.on_message));

        if let Some(fn_name) = on_message_fn {
            // Load and execute the on_message transform pipeline.
            let graph = match platform.node_registry.load_composite_function(
                &owner,
                &project,
                kind,
                Some(&fn_name),
            ) {
                Ok(g) => g,
                Err(e) => {
                    // If transform function fails to load, pass through raw payload.
                    eprintln!(
                        "composite_trigger: on_message '{}' load failed for '{}': {}, passing through raw",
                        fn_name, kind, e.message
                    );
                    results.push(NodeExecutionOutput {
                        output_pins: vec!["out".to_string()],
                        payload: input.payload.clone(),
                        trace: vec![format!(
                            "composite trigger '{}' on_message load error, raw passthrough",
                            kind
                        )],
                    });
                    continue;
                }
            };

            let placeholder_map =
                build_composite_placeholder_map(kind, config, &owner, &project, platform);

            // The input to the transform is the webhook body.
            let transform_input = if let Some(body) = input.payload.get("body") {
                body.clone()
            } else {
                input.payload.clone()
            };

            let ctx = PipelineContext {
                owner: owner.clone(),
                project: project.clone(),
                pipeline: format!("composite_trigger::{}::on_message", kind),
                request_id: format!(
                    "ct-{}-{}",
                    kind,
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis()
                ),
                route: Default::default(),
                input: transform_input,
                trigger: None,
                placeholder: if placeholder_map.is_empty() {
                    None
                } else {
                    Some(json!(placeholder_map))
                },
            };

            let engine = BasicPipelineEngine::new(
                Arc::new(DenoSandboxEngine::default()),
                crate::rwe::resolve_engine_or_default(None),
                Some(platform.credentials.clone()),
            )
            .with_platform(platform.clone())
            .with_ws_hub(platform.ws_hub.clone())
            .with_state_bus(platform.state_bus.clone())
            .with_data_root(platform.config.data_root.clone());

            match engine.execute_async(&graph, &ctx).await {
                Ok(output) => {
                    results.push(NodeExecutionOutput {
                        output_pins: vec!["out".to_string()],
                        payload: output.value,
                        trace: vec![format!("composite trigger '{}' on_message ok", kind)],
                    });
                }
                Err(e) => {
                    // Transform failed — pass through raw payload so pipeline can still work.
                    eprintln!(
                        "composite_trigger: on_message '{}' exec failed for '{}': {}, passing through raw",
                        fn_name, kind, e.message
                    );
                    results.push(NodeExecutionOutput {
                        output_pins: vec!["out".to_string()],
                        payload: input.payload.clone(),
                        trace: vec![format!(
                            "composite trigger '{}' on_message error: {}, raw passthrough",
                            kind, e.message
                        )],
                    });
                }
            }
        } else {
            // No on_message — pass through like n.trigger.webhook.
            results.push(NodeExecutionOutput {
                output_pins: vec!["out".to_string()],
                payload: input.payload.clone(),
                trace: vec![format!("composite trigger '{}' passthrough", kind)],
            });
        }
    }

    Ok(results)
}

/// Builds a `$placeholder` map from the composite manifest's credential declarations,
/// resolving each credential's secret fields into named placeholders. This map is
/// injected into the inner pipeline's execution context so `{{ $placeholder.X }}`
/// expressions resolve to actual credential values.
async fn execute_composite_node(
    kind: &str,
    config: &Value,
    platform: &Arc<crate::platform::services::PlatformService>,
    inputs: Vec<crate::pipeline::nodes::NodeExecutionInput>,
) -> Result<Vec<crate::pipeline::nodes::NodeExecutionOutput>, PipelineError> {
    use crate::pipeline::nodes::NodeExecutionOutput;

    let mut results = Vec::with_capacity(inputs.len());

    for input in inputs {
        let owner = input
            .metadata
            .get("owner")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let project = input
            .metadata
            .get("project")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        // Load the inner pipeline graph from the installed package.
        let graph = match platform
            .node_registry
            .load_composite_pipeline(&owner, &project, kind)
        {
            Ok(g) => g,
            Err(e) => {
                results.push(NodeExecutionOutput {
                    output_pins: vec!["error".to_string()],
                    payload: json!({"error": format!("{}: {}", e.code, e.message)}),
                    trace: vec![format!("composite '{}' load error: {}", kind, e.message)],
                });
                continue;
            }
        };

        // Build $placeholder map from credential declarations in the manifest.
        let mut placeholder_map =
            build_composite_placeholder_map(kind, config, &owner, &project, platform);

        // Inject non-credential config fields as CONFIG_<KEY> placeholders so
        // inner pipelines can access node config (e.g. model override).
        if let Some(obj) = config.as_object() {
            for (key, val) in obj {
                // Skip credential ID fields — those are already resolved.
                if key.ends_with("_credential_id") || key == "credential_id" {
                    continue;
                }
                let placeholder_key = format!("CONFIG_{}", key.to_uppercase());
                if !placeholder_map.contains_key(&placeholder_key) {
                    placeholder_map.insert(placeholder_key, val.clone());
                }
            }
        }

        // Allow runtime input `__config` to override CONFIG_<KEY> placeholders,
        // enabling dynamic per-run config (e.g. model selection like n.ai.agent).
        if let Some(runtime_cfg) = input.payload.get("__config").and_then(|v| v.as_object()) {
            for (key, val) in runtime_cfg {
                if key.ends_with("_credential_id") || key == "credential_id" {
                    continue;
                }
                let placeholder_key = format!("CONFIG_{}", key.to_uppercase());
                placeholder_map.insert(placeholder_key, val.clone());
            }
        }

        // Build execution context — same pattern as execute_function_pipeline.
        let ctx = PipelineContext {
            owner: owner.clone(),
            project: project.clone(),
            pipeline: format!("composite::{}", kind),
            request_id: format!(
                "composite-{}-{}",
                kind,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            ),
            route: Default::default(),
            input: input.payload.clone(),
            trigger: None,
            placeholder: if placeholder_map.is_empty() {
                None
            } else {
                Some(json!(placeholder_map))
            },
        };

        let engine = BasicPipelineEngine::new(
            Arc::new(DenoSandboxEngine::default()),
            crate::rwe::resolve_engine_or_default(None),
            Some(platform.credentials.clone()),
        )
        .with_platform(platform.clone())
        .with_ws_hub(platform.ws_hub.clone())
        .with_state_bus(platform.state_bus.clone())
        .with_data_root(platform.config.data_root.clone());

        match engine.execute_async(&graph, &ctx).await {
            Ok(output) => {
                results.push(NodeExecutionOutput {
                    output_pins: vec!["out".to_string()],
                    payload: output.value,
                    trace: vec![format!("composite '{}' ok", kind)],
                });
            }
            Err(e) => {
                results.push(NodeExecutionOutput {
                    output_pins: vec!["error".to_string()],
                    payload: json!({"error": format!("{}: {}", e.code, e.message)}),
                    trace: vec![format!(
                        "composite '{}' error: {} — {}",
                        kind, e.code, e.message
                    )],
                });
            }
        }
    }

    Ok(results)
}

/// Builds a placeholder name → resolved value map from a composite node's
/// credential declarations and the user's config.
///
/// For each credential declaration in the manifest:
/// 1. Read `config_key` to find which config field holds the credential ID.
/// 2. Fetch the credential from `CredentialService`.
/// 3. For each `placeholders` entry (e.g. `"BOT_TOKEN" -> "bot_token"`),
///    read the secret field and map the placeholder name to the actual value.
pub fn build_composite_placeholder_map(
    kind: &str,
    config: &Value,
    owner: &str,
    project: &str,
    platform: &Arc<crate::platform::services::PlatformService>,
) -> serde_json::Map<String, Value> {
    let mut placeholder_map = serde_json::Map::new();

    let manifest = match platform.node_registry.get_manifest(owner, project, kind) {
        Some(m) => m,
        None => return placeholder_map,
    };

    for cred_decl in &manifest.credentials {
        if cred_decl.config_key.is_empty() || cred_decl.placeholders.is_empty() {
            continue;
        }

        // Read the credential ID from the composite node's config.
        let credential_id = match config.get(&cred_decl.config_key).and_then(|v| v.as_str()) {
            Some(id) if !id.is_empty() => id.to_string(),
            _ => {
                eprintln!(
                    "composite '{}': config key '{}' is empty or missing",
                    kind, cred_decl.config_key
                );
                continue;
            }
        };

        // Fetch the credential from the credential service.
        let credential =
            match platform
                .credentials
                .get_project_credential(owner, project, &credential_id)
            {
                Ok(Some(c)) => c,
                Ok(None) => {
                    eprintln!(
                        "composite '{}': credential '{}' not found",
                        kind, credential_id
                    );
                    continue;
                }
                Err(e) => {
                    eprintln!(
                        "composite '{}': failed to fetch credential '{}': {}",
                        kind, credential_id, e.message
                    );
                    continue;
                }
            };

        // Build placeholder entries from the manifest's placeholder map.
        for (placeholder_name, secret_field) in &cred_decl.placeholders {
            if let Some(value) = credential.secret.get(secret_field) {
                placeholder_map.insert(placeholder_name.clone(), value.clone());
            }
        }
    }

    placeholder_map
}
