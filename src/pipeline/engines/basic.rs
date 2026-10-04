//! Real framework engine with graph traversal and built-in node dispatch.
//!
//! # Engine-level common config
//!
//! The engine reads certain keys from each node's resolved config before
//! dispatching execution.  These are injected as DSL flags into every node
//! definition by [`builtin_node_definitions()`] so they are available in both
//! DSL and the UI pipeline editor.
//!
//! | Config key | DSL flag    | Description |
//! |------------|-------------|-------------|
//! | `timeout`  | `--timeout` | How long the node may run: a duration from 1s to 1h (`30s`, `2m`). Omitted: the project's node timeout. A value outside that is refused, not clamped. |

use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use base64::Engine as _;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use super::composite_host::execute_installed_node;
use crate::infra::io::state::{DynStateBus, MemStateBus};
use crate::infra::mem::MemHub;
use crate::infra::transport::ws::WsHub;
use crate::infra::ws_client::WsClientManager;
use crate::language::{DenoSandboxEngine, LanguageEngine};
use crate::pipeline::expr::{resolve_config_expressions, scanner::scan as scan_exprs};
use crate::pipeline::interface::PipelineEngine;
use crate::pipeline::model::{
    ExecuteOptions, ExecutionBus, NodeTraceEntry, PipelineContext, PipelineError, PipelineGraph,
    PipelineNode, PipelineOutput, Signal,
};
use crate::pipeline::nodes::shared::file_ref::{FILE_REF_TYPE, LIFECYCLE_DURABLE};
use crate::pipeline::nodes::basic::{
    ai, auth, browser, crypto, fs, function, geo, http, input, javascript, kv, logic, mail, mapserver, postgres,
    sekejap, sqlite, table, typescript,
    trigger::{
        function as trigger_function, kv_subscribe, manual, mcp_trigger, schedule, weberror,
        webhook, ws_client as trigger_ws_client,
    },
    web, ws,
};
use crate::pipeline::nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler};
use crate::pipeline::trace_capture::{TraceCapture, redact_literal_secrets};
#[cfg(test)]
use crate::pipeline::trace_capture::TraceCaptureSettings;
use crate::platform::services::CredentialService;
use crate::platform::services::PlatformService;
use crate::rwe::{ReactiveWebEngine, TemplateSource, resolve_engine_or_default};

/// A single entry in the template compile cache.
/// Pairs the compiled page artifact with the set of component files it depends on,
/// enabling dependency-aware eviction when a component is edited.
pub struct CacheEntry {
    /// The compiled page artifact.
    pub page: Arc<web::response::CompiledPage>,
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

mod flow;
mod scheduler;
mod web_site;

const RETRY_STATE_KEY: &str = "__zf_retry";
const MAX_NODE_OUTPUT_FILE_BYTES: usize = 256 * 1024 * 1024;

fn hash_markup(s: &str) -> u64 {
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

fn build_nodes_retention_plan(graph: &PipelineGraph) -> Result<NodesRetentionPlan, PipelineError> {
    let mut plan = NodesRetentionPlan::default();

    for node in &graph.nodes {
        let mut access = scan_node_nodes_access(node)?;
        // A retry node counts its own attempts from its last output
        // (`$nodes.<self>.__zf_retry`), so it is always retained and always
        // in its own scope. Without this, a loop whose payload is replaced
        // on the way round (`http.response.fetch` answers with a fresh body) would
        // sit at attempt 1 for ever — the poll loops needed a script whose
        // only job was to copy the count back.
        if node.kind == logic::retry::NODE_KIND {
            plan.retained_nodes.insert(node.id.clone());
            access = match access {
                NodesAccess::None => NodesAccess::Exact(HashSet::from([node.id.clone()])),
                NodesAccess::Exact(mut ids) => {
                    ids.insert(node.id.clone());
                    NodesAccess::Exact(ids)
                }
            };
        }
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

/// The flow checks of `node-conventions.md` §4 on their own: no cycle but a
/// `logic.retry` re-entry, every loop body entered only through its foreach
/// and left only into its close, every `$nodes` reference upstream.
/// Activation runs them; every run does too, through `validate_graph`.
pub fn validate_flow(graph: &PipelineGraph) -> Result<(), PipelineError> {
    let retention = build_nodes_retention_plan(graph)?;
    flow::FlowPlan::build(graph, &retention.consumer_access).map(|_| ())
}

/// The loops of a graph as the flow reads them (§4), by node id: the
/// `logic.foreach` each loop body node runs under (its own level), and the
/// foreach each `logic.reduce` / `logic.collect` closes. Refused like
/// [`validate_flow`].
pub fn flow_loops(graph: &PipelineGraph) -> Result<FlowLoops, PipelineError> {
    let retention = build_nodes_retention_plan(graph)?;
    let plan = flow::FlowPlan::build(graph, &retention.consumer_access)?;
    let id = |i: usize| graph.nodes[i].id.clone();
    let by_id = |list: &[Option<usize>]| -> HashMap<String, String> {
        list.iter().enumerate().filter_map(|(i, f)| f.map(|f| (id(i), id(f)))).collect()
    };
    Ok(FlowLoops { owner: by_id(&plan.owner), closes: by_id(&plan.closes) })
}

/// See [`flow_loops`].
#[derive(Debug, Clone, Default)]
pub struct FlowLoops {
    pub owner: HashMap<String, String>,
    pub closes: HashMap<String, String>,
}

/// `$trigger` for the run: the snapshot the ingress set, else the envelope
/// the run started with (`ctx.input`, what the trigger answers under its
/// source key) — so `$trigger` is the envelope for every trigger, not only
/// the ones whose ingress builds a snapshot of its own.
fn trigger_envelope(ctx: &PipelineContext) -> Value {
    ctx.trigger.clone().unwrap_or_else(|| ctx.input.clone())
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
        "trigger": trigger_envelope(ctx),
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

/// A retry node's `--max-attempts` as the graph declares it — the DSL coerces
/// the scalar to a number, an edited JSON may still hold a string.
fn retry_max_attempts(node: &PipelineNode) -> Option<u64> {
    let raw = node.config.get("max_attempts")?;
    raw.as_u64()
        .or_else(|| raw.as_str().and_then(|s| s.trim().parse().ok()))
}

/// The failure envelope an `:error` edge carries: the failing node's input
/// under `input`, the error, and the retry state. `last_attempt` is what the
/// consuming retry node counted to on the previous round (0 when there is
/// none, or it has not run yet); the payload's own count wins when higher.
fn build_retry_error_payload(
    input_payload: &Value,
    error: &PipelineError,
    last_attempt: usize,
) -> Value {
    let attempt = retry_attempt_from_payload(input_payload).max(last_attempt) + 1;
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

pub(crate) fn is_sensitive_trace_config_key(key: &str) -> bool {
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
            // `--header X-API-Key=…` typed on an http.response.fetch is a key in the
            // node's config, and the recorded config is the snapshot the
            // record keeps of it.
            | "xapikey"
            // A node outside this tree may name a field plainly. `accesstoken`
            // was matched and `token` was not, which is the gap a third-party
            // node would fall into.
            | "token"
            | "apitoken"
            | "authtoken"
            | "sessiontoken"
            | "bearertoken"
            | "privatekey"
            | "signingkey"
            | "jwtsecret"
            | "cookiesecret"
            | "webhooksecret"
    )
}

#[cfg(test)]
fn trace_config_snapshot(value: &Value) -> Option<Value> {
    let mut capture = TraceCapture::new(TraceCaptureSettings::default().resolve(None));
    capture.begin_node();
    capture.config(value)
}

fn redact_string(value: &str, tokens: &[String]) -> String {
    redact_literal_secrets(value, tokens)
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

#[cfg(test)]
const TRACE_SUMMARY_MAX_DEPTH: usize = 8;
#[cfg(test)]
const TRACE_SUMMARY_MAX_OBJECT_KEYS: usize = 48;
#[cfg(test)]
const TRACE_SUMMARY_MAX_ARRAY_ITEMS: usize = 16;
#[cfg(test)]
const TRACE_SUMMARY_PREVIEW_ITEMS: usize = 6;
#[cfg(test)]
const TRACE_SUMMARY_NUMERIC_ARRAY_THRESHOLD: usize = 64;
#[cfg(test)]
const TRACE_SUMMARY_MAX_STRING_CHARS: usize = 8192;
#[cfg(test)]
const TRACE_SUMMARY_STRING_PREVIEW_CHARS: usize = 512;

#[cfg(test)]
fn summarize_trace_value(value: &Value) -> Value {
    summarize_trace_value_inner(value, 0)
}

#[cfg(test)]
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

#[cfg(test)]
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

/// Blanks any value sitting under a key that names a secret.
///
/// The token mechanism above strikes out *values a node declared*; this strikes
/// out values *nobody declared* — the payload a cron, WS, or script-built run
/// carries, where no node publishes redaction tokens at all. The two are
/// complementary: tokens catch a secret that was copied somewhere else in the
/// trace, key names catch a secret nobody thought to declare.
///
/// `except_paths` is honoured here too, so a pipeline that deliberately traces
/// a field named `token` keeps the same one escape hatch it already had.
#[cfg(test)]
fn blank_sensitive_keys(value: &Value, except_paths: &[Vec<String>], path: &[String]) -> Value {
    if except_paths.iter().any(|candidate| candidate == path) {
        return value.clone();
    }
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, item)| {
                    let mut next = path.to_vec();
                    next.push(key.clone());
                    let sanitized = if except_paths.iter().any(|candidate| candidate == &next) {
                        item.clone()
                    } else if crate::pipeline::nodes::basic::trigger::webhook::is_sensitive_payload_key(key)
                    {
                        blanked_like(item)
                    } else {
                        blank_sensitive_keys(item, except_paths, &next)
                    };
                    (key.clone(), sanitized)
                })
                .collect(),
        ),
        Value::Array(items) => Value::Array(
            items
                .iter()
                .map(|item| blank_sensitive_keys(item, except_paths, path))
                .collect(),
        ),
        other => other.clone(),
    }
}

/// Replaces a secret with a mask of the same *shape*, so a trace still shows
/// whether the field was present, a string, or a list — just not what it said.
///
/// Booleans and nulls pass through: one bit carries no secret, and a masked
/// `"authorization": true` would cost a reader real information for nothing.
/// Numbers do not, because a `cvv`, `otp`, or `pin` is a number.
#[cfg(test)]
fn blanked_like(value: &Value) -> Value {
    match value {
        Value::Null | Value::Bool(_) => value.clone(),
        Value::Array(items) => Value::Array(items.iter().map(blanked_like).collect()),
        Value::Object(map) => Value::Object(
            map.iter()
                .map(|(key, item)| (key.clone(), blanked_like(item)))
                .collect(),
        ),
        Value::String(_) | Value::Number(_) => Value::String("••••••".to_string()),
    }
}

/// Lifts every redaction marker out of a payload, wherever it sits, and
/// returns the literal values they name.
///
/// The markers used to be read at the top level only. That is where a node
/// puts one, but not where it stays: `web.response.send` nests the whole upstream
/// payload under `__zf_response.json`, so by the time that node's *output* was
/// traced the marker was one level down -- unread, unremoved, and printed
/// verbatim into the run history with the secrets inside it.
#[cfg(test)]
fn sweep_private_markers(value: &mut Value, tokens: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for key in ["__zf_private_trace_redact", "__zf_private_redact"] {
                let Some(Value::Array(items)) = map.remove(key) else {
                    continue;
                };
                for item in items {
                    let Some(text) = item.as_str() else { continue };
                    let trimmed = text.trim();
                    if !trimmed.is_empty() && !tokens.iter().any(|seen| seen == trimmed) {
                        tokens.push(trimmed.to_string());
                    }
                }
            }
            map.remove("__zf_private_redact_except_paths");
            for item in map.values_mut() {
                sweep_private_markers(item, tokens);
            }
        }
        Value::Array(items) => {
            for item in items {
                sweep_private_markers(item, tokens);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
fn legacy_sanitized_trace_value(value: &Value) -> Value {
    let mut payload = value.clone();
    // Only a top-level exception list defines paths, because a path is written
    // from the payload root and a nested copy names nothing meaningful.
    let except_paths = take_private_redact_except_paths(&mut payload);
    let mut tokens = Vec::new();
    sweep_private_markers(&mut payload, &mut tokens);
    let redacted = if tokens.is_empty() {
        payload
    } else {
        redact_json_value(&payload, &tokens, &except_paths, &[])
    };
    let blanked = blank_sensitive_keys(&redacted, &except_paths, &[]);
    summarize_trace_value(&blanked)
}

#[cfg(test)]
fn sanitized_trace_value(value: &Value) -> Value {
    let mut capture = TraceCapture::new(TraceCaptureSettings::default().resolve(None));
    capture.begin_node();
    capture.capture(value)
}

fn materialize_node_output_files(
    platform: Option<&Arc<PlatformService>>,
    ctx: &PipelineContext,
    node_kind: &str,
    mut payload: Value,
) -> Result<Value, PipelineError> {
    let Some(files) = take_node_output_files(&mut payload) else {
        return Ok(payload);
    };
    if files.is_empty() {
        return Ok(payload);
    }
    let platform = platform.ok_or_else(|| {
        PipelineError::new(
            "FW_NODE_OUTPUT_FILE_CONTEXT",
            "node output files require platform storage context",
        )
    })?;
    let layout = platform
        .file
        .ensure_project_layout(&ctx.owner, &ctx.project)
        .map_err(|err| PipelineError::new("FW_NODE_OUTPUT_FILE", err.to_string()))?;
    let zebfs = layout.open_files();
    let request_id = sanitize_path_part(&ctx.request_id);
    let kind_part = sanitize_path_part(node_kind);
    let mut refs = Map::new();

    for (idx, file) in files.iter().enumerate() {
        let descriptor = file.as_object().ok_or_else(|| {
            PipelineError::new(
                "FW_NODE_OUTPUT_FILE",
                format!("node output file {idx} must be an object"),
            )
        })?;
        let name = descriptor
            .get("name")
            .and_then(Value::as_str)
            .map(sanitize_filename)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| format!("file-{idx}.json"));
        let content_type = descriptor
            .get("content_type")
            .or_else(|| descriptor.get("mime"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("application/octet-stream");
        let encoding = descriptor
            .get("encoding")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("json");
        let data = descriptor.get("data").ok_or_else(|| {
            PipelineError::new(
                "FW_NODE_OUTPUT_FILE",
                format!("node output file '{name}' is missing data"),
            )
        })?;
        let bytes = node_output_file_bytes(node_kind, &name, encoding, data)?;
        if bytes.len() > MAX_NODE_OUTPUT_FILE_BYTES {
            return Err(PipelineError::new(
                "FW_NODE_OUTPUT_FILE_TOO_LARGE",
                format!(
                    "node '{node_kind}' output file '{name}' is {} bytes; limit is {MAX_NODE_OUTPUT_FILE_BYTES}",
                    bytes.len()
                ),
            ));
        }
        let rel_path = format!("artifacts/nodes/{kind_part}/{request_id}/{name}");
        let stat = zebfs
            .put(&rel_path, &bytes)
            .map_err(|err| PipelineError::new("FW_NODE_OUTPUT_FILE_WRITE", err.to_string()))?;
        let file_ref = json!({
            "__zf_type": FILE_REF_TYPE,
            "backend": layout.file_backend().as_str(),
            "store": layout.store_id(),
            "ref": stat.path,
            "filename": name,
            "mime": content_type,
            "kind": crate::pipeline::nodes::shared::file_ref::infer_kind(content_type, &rel_path),
            "size": bytes.len(),
            "sha256": format!("sha256:{:x}", Sha256::digest(&bytes)),
            "lifecycle": LIFECYCLE_DURABLE,
            "origin": "node-output",
            "trust": "generated",
        });
        refs.insert(name, file_ref);
    }

    let refs_value = Value::Object(refs);
    replace_file_placeholders(&mut payload, &refs_value);
    if let Some(obj) = payload.as_object_mut() {
        obj.insert("file_refs".to_string(), refs_value);
    }
    Ok(payload)
}

fn take_node_output_files(payload: &mut Value) -> Option<Vec<Value>> {
    let map = payload.as_object_mut()?;
    let raw = map
        .remove("__zf_files")
        .or_else(|| map.remove("artifacts"))?;
    match raw {
        Value::Array(files) => Some(files),
        other => Some(vec![other]),
    }
}

fn node_output_file_bytes(
    node_kind: &str,
    name: &str,
    encoding: &str,
    data: &Value,
) -> Result<Vec<u8>, PipelineError> {
    match encoding {
        "json" => serde_json::to_vec(data).map_err(|err| {
            PipelineError::new(
                "FW_NODE_OUTPUT_FILE_JSON",
                format!("node '{node_kind}' output file '{name}' JSON serialization failed: {err}"),
            )
        }),
        "text" | "utf8" => Ok(data
            .as_str()
            .map(str::as_bytes)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| data.to_string().into_bytes())),
        "base64" => {
            let encoded = data.as_str().ok_or_else(|| {
                PipelineError::new(
                    "FW_NODE_OUTPUT_FILE_BASE64",
                    format!("node '{node_kind}' output file '{name}' base64 data must be a string"),
                )
            })?;
            base64::engine::general_purpose::STANDARD
                .decode(encoded.trim())
                .map_err(|err| {
                    PipelineError::new(
                        "FW_NODE_OUTPUT_FILE_BASE64",
                        format!(
                            "node '{node_kind}' output file '{name}' base64 decode failed: {err}"
                        ),
                    )
                })
        }
        other => Err(PipelineError::new(
            "FW_NODE_OUTPUT_FILE_ENCODING",
            format!("node '{node_kind}' output file '{name}' uses unsupported encoding '{other}'"),
        )),
    }
}

fn replace_file_placeholders(value: &mut Value, refs: &Value) {
    match value {
        Value::String(text) => {
            if let Some(name) = exact_file_placeholder(text)
                && let Some(file_ref) = refs.get(name)
            {
                *value = file_ref.clone();
                return;
            }
            let mut replaced = text.clone();
            if let Some(map) = refs.as_object() {
                for (name, file_ref) in map {
                    let url = file_ref
                        .get("url")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    replaced = replaced.replace(&format!("{{{{file:{name}}}}}"), url);
                    replaced = replaced.replace(&format!("{{{{artifact:{name}}}}}"), url);
                }
            }
            *text = replaced;
        }
        Value::Array(items) => {
            for item in items {
                replace_file_placeholders(item, refs);
            }
        }
        Value::Object(map) => {
            for child in map.values_mut() {
                replace_file_placeholders(child, refs);
            }
        }
        _ => {}
    }
}

fn exact_file_placeholder(value: &str) -> Option<&str> {
    value
        .strip_prefix("{{file:")
        .or_else(|| value.strip_prefix("{{artifact:"))
        .and_then(|rest| rest.strip_suffix("}}"))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn sanitize_filename(raw: &str) -> String {
    let name = raw.rsplit(['/', '\\']).next().unwrap_or(raw).trim();
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
            out.push(ch);
        } else if ch.is_whitespace() {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches(['.', '-']).to_string();
    if trimmed.is_empty() {
        "file.bin".to_string()
    } else {
        trimmed
    }
}

fn sanitize_path_part(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
            out.push(ch);
        } else {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        "run".to_string()
    } else {
        trimmed.to_string()
    }
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
    ///
    /// Derived from `repo_layout` and never set on its own, so the alias root
    /// and the layout that names it cannot disagree.
    template_root: Option<std::path::PathBuf>,
    /// The project's resolved repository layout, when the engine runs for one.
    ///
    /// Nodes that need a repository directory other than the source root read
    /// it from here instead of walking up from the template root, which only
    /// works for a source root exactly one segment deep.
    repo_layout: Option<crate::platform::model::ProjectFileLayout>,
    /// Platform data root — used by SQLite nodes to locate the project DB.
    data_root: Option<std::path::PathBuf>,
    /// The bundle policy governing this engine's nodes.
    ///
    /// Set only on an engine built to run a bundle's function pipeline, so it
    /// covers the whole inner subtree. A project's own pipeline runs with
    /// `None` and is unrestricted. `Some` means "inside a bundle" whether or
    /// not that bundle declared a host; the declared hosts are what it carries,
    /// not what makes it present.
    bundle_egress: Option<Arc<crate::pipeline::security::BundleEgress>>,
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
            repo_layout: None,
            data_root: None,
            bundle_egress: None,
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
            repo_layout: None,
            data_root: None,
            bundle_egress: None,
        }
    }

    /// Attach the project's repository layout.
    ///
    /// This also sets the template root, because the template root *is* the
    /// layout's source directory; there is no second way to set it.
    pub fn with_project_layout(
        mut self,
        layout: Option<crate::platform::model::ProjectFileLayout>,
    ) -> Self {
        self.template_root = layout.as_ref().map(|value| value.repo_source_dir());
        self.repo_layout = layout;
        self
    }

    /// Source libraries (`zeb/ui`, …) the compiler resolves by file. They ship
    /// with the platform, so an engine without one has none.
    fn library_roots(&self) -> std::collections::BTreeMap<String, std::path::PathBuf> {
        self.platform
            .as_ref()
            .map(|p| p.library.source_roots())
            .unwrap_or_default()
    }

    /// Attach a shared template compile cache to this engine.
    /// Same cache instance should be passed on every request so hits accumulate.
    pub fn with_template_cache(mut self, cache: TemplateCache) -> Self {
        self.template_cache = Some(cache);
        self
    }

    /// Attach the WS hub so `ws.message.send` and `ws.state.*` nodes can reach rooms.
    pub fn with_ws_hub(mut self, hub: Arc<WsHub>) -> Self {
        self.ws_hub = Some(hub);
        self
    }

    /// Attach the WS client manager so `ws.message.send --connection` can send through outbound connections.
    pub fn with_ws_client_manager(mut self, mgr: Arc<WsClientManager>) -> Self {
        self.ws_client_manager = Some(mgr);
        self
    }

    /// Attach the state bus so `kv.*` nodes can access the shared project-scoped KV/pubsub layer.
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

    /// Attach the platform service so function.result.call nodes can invoke sub-pipelines.
    pub fn with_platform(mut self, platform: Arc<PlatformService>) -> Self {
        self.platform = Some(platform);
        self
    }

    /// Attach the platform data root so SQLite nodes can locate the project DB.
    pub fn with_data_root(mut self, root: std::path::PathBuf) -> Self {
        self.data_root = Some(root);
        self
    }

    /// Confine this engine's nodes to the hosts a node bundle declared.
    ///
    /// Attached where a bundle's function pipeline is dispatched, which is what
    /// makes the declaration cover every node in that pipeline rather than only
    /// the bundle node the outer graph names.
    pub fn with_bundle_egress(
        mut self,
        egress: Option<Arc<crate::pipeline::security::BundleEgress>>,
    ) -> Self {
        self.bundle_egress = egress;
        self
    }

    fn build_node(&self, node: &PipelineNode) -> Result<NodeDispatch, PipelineError> {
        if let Some(egress) = &self.bundle_egress {
            refuse_uncheckable_egress_node(
                egress,
                &node.kind,
                &node.config,
                self.language.grants_network(),
            )?;
        }
        // One kind per task, one builder for the family (`crypto/mod.rs`).
        if let Some(handler) = crypto::build(&node.kind, &node.config, self.credentials.clone())? {
            return Ok(NodeDispatch::Crypto(handler));
        }
        if let Some(handler) = kv::build(&node.kind, &node.config, self.state_bus.clone())? {
            return Ok(NodeDispatch::Kv(handler));
        }
        match node.kind.as_str() {
            webhook::NODE_KIND => Ok(NodeDispatch::Webhook(webhook::Node::new(
                serde_json::from_value(node.config.clone())
                    .map_err(|err| PipelineError::new("FW_NODE_TRIGGER_WEBHOOK_CONFIG", err.to_string()))?,
            ))),
            schedule::NODE_KIND => Ok(NodeDispatch::Schedule(schedule::Node::new(
                serde_json::from_value(node.config.clone()).map_err(|err| {
                    PipelineError::new("FW_NODE_TRIGGER_SCHEDULE_CONFIG", err.to_string())
                })?,
            ))),
            manual::NODE_KIND => Ok(NodeDispatch::Manual(manual::Node::new(
                serde_json::from_value(node.config.clone())
                    .map_err(|err| PipelineError::new("FW_NODE_TRIGGER_MANUAL_CONFIG", err.to_string()))?,
            ))),
            javascript::NODE_KIND => Ok(NodeDispatch::Script(crate::pipeline::nodes::shared::script::Node::build(
                &node.id,
                &node.config,
                &javascript::LANGUAGE,
                self.language.clone(),
            )?)),
            typescript::NODE_KIND => Ok(NodeDispatch::Script(crate::pipeline::nodes::shared::script::Node::build(
                &node.id,
                &node.config,
                &typescript::LANGUAGE,
                self.language.clone(),
            )?)),
            http::request::NODE_KIND => Ok(NodeDispatch::HttpRequest(http::request::Node::new(
                serde_json::from_value(node.config.clone()).map_err(|err| {
                    PipelineError::new("FW_NODE_HTTP_RESPONSE_FETCH_CONFIG", err.to_string())
                })?,
                self.language.clone(),
                self.credentials.clone(),
                self.platform.clone(),
                self.bundle_egress.clone(),
            )?)),
            sqlite::query::NODE_KIND => {
                let Some(data_root) = &self.data_root else {
                    return Err(PipelineError::new(
                        "FW_NODE_SQLITE_QUERY_RUN_UNAVAILABLE",
                        "data_root is not configured on this pipeline engine",
                    ));
                };
                Ok(NodeDispatch::SqliteQuery(sqlite::query::Node::new(
                    serde_json::from_value(node.config.clone()).map_err(|err| {
                        PipelineError::new(sqlite::query::CONFIG_CODE, err.to_string())
                    })?,
                    data_root.clone(),
                )?))
            }
            sekejap::query::NODE_KIND => {
                let Some(data_root) = &self.data_root else {
                    return Err(PipelineError::new(
                        "FW_NODE_SEKEJAP_QUERY_RUN_UNAVAILABLE",
                        "data_root is not configured on this pipeline engine",
                    ));
                };
                Ok(NodeDispatch::SekejapQuery(sekejap::query::Node::new(
                    serde_json::from_value(node.config.clone()).map_err(|err| {
                        PipelineError::new(sekejap::query::CONFIG_CODE, err.to_string())
                    })?,
                    data_root.clone(),
                )?))
            }
            sekejap::record::NODE_KIND => {
                let Some(data_root) = &self.data_root else {
                    return Err(PipelineError::new(
                        "FW_NODE_SEKEJAP_RECORD_CREATE_UNAVAILABLE",
                        "data_root is not configured on this pipeline engine",
                    ));
                };
                Ok(NodeDispatch::SekejapRecord(sekejap::record::Node::new(
                    serde_json::from_value(node.config.clone()).map_err(|err| {
                        PipelineError::new(sekejap::record::CONFIG_CODE, err.to_string())
                    })?,
                    data_root.clone(),
                )?))
            }
            browser::run::NODE_KIND => {
                let Some(credentials) = &self.credentials else {
                    return Err(PipelineError::new(
                        "FW_NODE_BROWSER_PAGE_RUN_UNAVAILABLE",
                        "credential service is not configured on this framework engine",
                    ));
                };
                Ok(NodeDispatch::BrowserRun(browser::run::Node::new(
                    serde_json::from_value(node.config.clone()).map_err(|e| {
                        PipelineError::new("FW_NODE_BROWSER_PAGE_RUN_CONFIG", e.to_string())
                    })?,
                    credentials.clone(),
                    self.bundle_egress.clone(),
                )?))
            }
            postgres::query::NODE_KIND => {
                let Some(credentials) = &self.credentials else {
                    return Err(PipelineError::new(
                        "FW_NODE_POSTGRES_QUERY_RUN_UNAVAILABLE",
                        "credential service is not configured on this framework engine",
                    ));
                };
                Ok(NodeDispatch::Postgres(postgres::query::Node::new(
                    serde_json::from_value(node.config.clone())
                        .map_err(|err| PipelineError::new(postgres::query::CONFIG_CODE, err.to_string()))?,
                    credentials.clone(),
                )?))
            }
            table::query::NODE_KIND => {
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        table::query::UNAVAILABLE_CODE,
                        "platform service is not configured on this pipeline engine",
                    ));
                };
                Ok(NodeDispatch::TableQuery(table::query::Node::new(
                    serde_json::from_value(node.config.clone())
                        .map_err(|err| PipelineError::new(table::query::CONFIG_CODE, err.to_string()))?,
                    platform.clone(),
                    self.language.clone(),
                )?))
            }
            web::response::NODE_KIND => {
                let config: web::response::Config = serde_json::from_value(node.config.clone())
                    .map_err(|err| PipelineError::new(web::response::CODE_CONFIG, err.to_string()))?;
                config.check()?;
                if config.template.is_some() {
                    Ok(NodeDispatch::InlineWebResponse {
                        node_id: node.id.clone(),
                        config,
                    })
                } else {
                    Ok(NodeDispatch::WebResponse(web::response::Node::new(config, self.template_root.clone())))
                }
            }
            web::site::NODE_KIND => {
                if self.data_root.is_none() {
                    return Err(PipelineError::new(
                        "FW_NODE_WEB_SITE_GENERATE_UNAVAILABLE",
                        "data_root is not configured on this pipeline engine",
                    ));
                }
                let config: web::site::Config = serde_json::from_value(node.config.clone())
                    .map_err(|err| PipelineError::new(web::site::CODE_CONFIG, err.to_string()))?;
                config.check()?;
                Ok(NodeDispatch::InlineWebSiteGenerate {
                    node_id: node.id.clone(),
                    config,
                })
            }
            ai::agent::NODE_KIND => Ok(NodeDispatch::Agent(ai::agent::Node::build(
                &node.config,
                self.credentials.clone(),
                self.platform.clone(),
            )?)),
            ai::tts::NODE_KIND => Ok(NodeDispatch::AiTts(ai::tts::Node::build(
                &node.config,
                self.credentials.clone(),
                self.platform.clone(),
            )?)),
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
                &node.id,
                serde_json::from_value(node.config.clone())
                    .map_err(|e| PipelineError::new("FW_NODE_LOGIC_RETRY_CONFIG", e.to_string()))?,
                self.language.clone(),
            )?)),
            auth::token_create::NODE_KIND => {
                let Some(credentials) = &self.credentials else {
                    return Err(PipelineError::new(
                        "FW_NODE_AUTH_TOKEN_CREATE_UNAVAILABLE",
                        "credential service is not configured on this framework engine",
                    ));
                };
                Ok(NodeDispatch::AuthTokenCreate(auth::token_create::Node::new(
                    serde_json::from_value(node.config.clone()).map_err(|err| {
                        PipelineError::new("FW_NODE_AUTH_TOKEN_CREATE_CONFIG", err.to_string())
                    })?,
                    credentials.clone(),
                )?))
            }
            auth::token_verify::NODE_KIND => {
                let Some(credentials) = &self.credentials else {
                    return Err(PipelineError::new(
                        "FW_NODE_AUTH_TOKEN_VERIFY_UNAVAILABLE",
                        "credential service is not configured on this framework engine",
                    ));
                };
                Ok(NodeDispatch::AuthTokenVerify(auth::token_verify::Node::new(
                    serde_json::from_value(node.config.clone()).map_err(|err| {
                        PipelineError::new("FW_NODE_AUTH_TOKEN_VERIFY_CONFIG", err.to_string())
                    })?,
                    credentials.clone(),
                )?))
            }
            logic::concept::NODE_KIND => Ok(NodeDispatch::Concept(logic::concept::Node::new(
                serde_json::from_value(node.config.clone()).map_err(|err| {
                    PipelineError::new("FW_NODE_CONCEPT_CONFIG", err.to_string())
                })?,
            ))),
            mail::send::NODE_KIND => {
                let Some(credentials) = &self.credentials else {
                    return Err(PipelineError::new(
                        "FW_NODE_MAIL_MESSAGE_SEND_UNAVAILABLE",
                        "credential service is not configured on this framework engine",
                    ));
                };
                Ok(NodeDispatch::MailSend(mail::send::Node::new(
                    serde_json::from_value(node.config.clone()).map_err(|err| {
                        PipelineError::new("FW_NODE_MAIL_MESSAGE_SEND_CONFIG", err.to_string())
                    })?,
                    credentials.clone(),
                    self.platform.clone(),
                )?))
            }
            weberror::NODE_KIND => Ok(NodeDispatch::WebError(weberror::Node::new(
                serde_json::from_value(node.config.clone()).map_err(|err| {
                    PipelineError::new("FW_NODE_TRIGGER_ERROR_CONFIG", err.to_string())
                })?,
            ))),
            ws::trigger::NODE_KIND => Ok(NodeDispatch::WsTrigger(ws::trigger::Node::new(
                serde_json::from_value(node.config.clone()).map_err(|err| {
                    PipelineError::new("FW_NODE_TRIGGER_ROOM_CONFIG", err.to_string())
                })?,
            ))),
            ws::message_send::NODE_KIND => Ok(NodeDispatch::WsMessageSend(ws::message_send::Node::new(
                serde_json::from_value(node.config.clone()).map_err(|err| {
                    PipelineError::new(ws::message_send::CONFIG_CODE, err.to_string())
                })?,
                self.ws_hub.clone(),
                self.ws_client_manager.clone(),
            )?)),
            kind if ws::state::Verb::of_kind(kind).is_some() => {
                let verb = ws::state::Verb::of_kind(kind).expect("matched above");
                Ok(NodeDispatch::WsState(ws::state::Node::new(
                    verb,
                    serde_json::from_value(node.config.clone()).map_err(|err| {
                        PipelineError::new(verb.codes().config, err.to_string())
                    })?,
                    self.ws_hub.clone(),
                )?))
            }
            trigger_function::NODE_KIND => {
                let config: trigger_function::Config =
                    serde_json::from_value(node.config.clone()).unwrap_or_default();
                Ok(NodeDispatch::TriggerFunction(trigger_function::Node::new(
                    config,
                )))
            }
            function::call::NODE_KIND => {
                let config: function::call::Config =
                    serde_json::from_value(node.config.clone()).unwrap_or_default();
                Ok(NodeDispatch::FunctionCall(function::call::Node::new(
                    config,
                    self.platform.clone(),
                )))
            }
            fs::put::NODE_KIND => {
                let config: fs::put::Config = serde_json::from_value(node.config.clone())
                    .map_err(|err| PipelineError::new(fs::put::CONFIG_CODE, err.to_string()))?;
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        fs::put::CODE,
                        "platform service not available in this engine context",
                    ));
                };
                Ok(NodeDispatch::FilePut(fs::put::Node::new(config, platform.clone())?))
            }
            fs::object::LIST_NODE_KIND
            | fs::object::HEAD_NODE_KIND
            | fs::object::GET_NODE_KIND
            | fs::object::DELETE_NODE_KIND
            | fs::object::COPY_NODE_KIND
            | fs::object::MOVE_NODE_KIND
            | fs::object::MKDIR_NODE_KIND => {
                let operation = fs::object::Operation::for_kind(&node.kind).expect("an fs object kind");
                let config: fs::object::Config = serde_json::from_value(node.config.clone())
                    .map_err(|err| PipelineError::new(operation.config_code(), err.to_string()))?;
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        operation.code(),
                        "platform service not available in this engine context",
                    ));
                };
                Ok(NodeDispatch::FsObject(fs::object::Node::new(
                    config,
                    platform.clone(),
                    operation,
                )?))
            }
            mapserver::crud::PUBLISH_KIND
            | mapserver::crud::UNPUBLISH_KIND
            | mapserver::crud::GET_KIND
            | mapserver::crud::LIST_KIND => {
                let operation = mapserver::crud::Operation::from_kind(node.kind.as_str()).expect("an mapserver.layer kind");
                let config: mapserver::crud::Config = serde_json::from_value(node.config.clone())
                    .map_err(|err| PipelineError::new(operation.config_code(), err.to_string()))?;
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        operation.code(),
                        "platform service not available in this engine context",
                    ));
                };
                Ok(NodeDispatch::MapserverCrud(mapserver::crud::Node::new(
                    config,
                    platform.clone(),
                    operation,
                )?))
            }
            table::convert::NODE_KIND => {
                let config: table::convert::Config = serde_json::from_value(node.config.clone())
                    .map_err(|err| PipelineError::new(table::convert::CONFIG_CODE, err.to_string()))?;
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        table::convert::CODE,
                        "platform service not available in this engine context",
                    ));
                };
                Ok(NodeDispatch::TableConvert(table::convert::Node::new(
                    config,
                    platform.clone(),
                )?))
            }
            fs::compress::NODE_KIND => {
                let config: fs::compress::Config = serde_json::from_value(node.config.clone())
                    .map_err(|err| PipelineError::new(fs::compress::CONFIG_CODE, err.to_string()))?;
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        fs::compress::CODE,
                        "platform service not available in this engine context",
                    ));
                };
                Ok(NodeDispatch::FileCompress(fs::compress::Node::new(
                    config,
                    platform.clone(),
                )?))
            }
            fs::decompress::NODE_KIND => {
                let config: fs::decompress::Config = serde_json::from_value(node.config.clone())
                    .map_err(|err| PipelineError::new(fs::decompress::CONFIG_CODE, err.to_string()))?;
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        fs::decompress::CODE,
                        "platform service not available in this engine context",
                    ));
                };
                Ok(NodeDispatch::FileDecompress(fs::decompress::Node::new(
                    config,
                    platform.clone(),
                )?))
            }
            geo::inspect::NODE_KIND => {
                let config: geo::inspect::Config = serde_json::from_value(node.config.clone())
                    .map_err(|err| PipelineError::new(geo::inspect::CONFIG_CODE, err.to_string()))?;
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        geo::inspect::CODE,
                        "platform service not available in this engine context",
                    ));
                };
                Ok(NodeDispatch::GeoInspect(geo::inspect::Node::new(
                    config,
                    platform.clone(),
                )?))
            }
            geo::convert::NODE_KIND => {
                let config: geo::convert::Config = serde_json::from_value(node.config.clone())
                    .map_err(|err| PipelineError::new(geo::convert::CONFIG_CODE, err.to_string()))?;
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        geo::convert::CODE,
                        "platform service not available in this engine context",
                    ));
                };
                Ok(NodeDispatch::GeoConvert(geo::convert::Node::new(
                    config,
                    platform.clone(),
                )?))
            }
            fs::pdf::convert::NODE_KIND => {
                let config: fs::pdf::convert::Config = serde_json::from_value(node.config.clone())
                    .map_err(|err| PipelineError::new(fs::pdf::convert::CONFIG_CODE, err.to_string()))?;
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        fs::pdf::convert::CODE,
                        "platform service not available in this engine context",
                    ));
                };
                Ok(NodeDispatch::FilePdfConvert(fs::pdf::convert::Node::new(
                    config,
                    platform.clone(),
                )?))
            }
            fs::svg::convert::NODE_KIND => {
                let config: fs::svg::convert::Config = serde_json::from_value(node.config.clone())
                    .map_err(|e| PipelineError::new(fs::svg::convert::CONFIG_CODE, e.to_string()))?;
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        fs::svg::convert::CONFIG_CODE,
                        "platform service not available in this engine context",
                    ));
                };
                Ok(NodeDispatch::SvgConvert(fs::svg::convert::Node::new(config, platform.clone())?))
            }
            fs::image::chromakey::NODE_KIND => {
                let config: fs::image::chromakey::Config = serde_json::from_value(node.config.clone())
                    .map_err(|err| PipelineError::new(fs::image::chromakey::CONFIG_CODE, err.to_string()))?;
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        fs::image::chromakey::CONFIG_CODE,
                        "platform service not available in this engine context",
                    ));
                };
                Ok(NodeDispatch::ImgChromakey(fs::image::chromakey::Node::new(config, platform.clone())?))
            }
            fs::barcode::NODE_KIND => {
                let config: fs::barcode::Config = serde_json::from_value(node.config.clone())
                    .map_err(|err| PipelineError::new(fs::barcode::node::CONFIG_CODE, err.to_string()))?;
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        fs::barcode::node::CODE,
                        "platform service not available in this engine context",
                    ));
                };
                Ok(NodeDispatch::Barcode(fs::barcode::Node::new(config, platform.clone())?))
            }
            fs::image::thumbnail::NODE_KIND => {
                let config: fs::image::thumbnail::Config = serde_json::from_value(node.config.clone())
                    .map_err(|err| PipelineError::new(fs::image::thumbnail::CONFIG_CODE, err.to_string()))?;
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        fs::image::thumbnail::CODE,
                        "platform service not available in this engine context",
                    ));
                };
                Ok(NodeDispatch::ImgThumbnail(fs::image::thumbnail::Node::new(
                    config,
                    platform.clone(),
                )?))
            }
            // The whole `input.*` family builds through one door: the kind
            // picks the check, the config names the field.
            kind if input::is_input_kind(kind) => {
                let node = input::Node::for_kind(kind, &node.config)
                    .expect("guard says this is an input kind")?;
                Ok(NodeDispatch::Input(node))
            }
            mcp_trigger::NODE_KIND => Ok(NodeDispatch::McpTrigger(mcp_trigger::Node::new(
                serde_json::from_value(node.config.clone()).map_err(|err| {
                    PipelineError::new("FW_NODE_TRIGGER_MCP_CONFIG", err.to_string())
                })?,
            ))),
            kv_subscribe::NODE_KIND => Ok(NodeDispatch::KvSubscribe(kv_subscribe::Node::new(
                serde_json::from_value(node.config.clone()).map_err(|e| {
                    PipelineError::new("FW_NODE_TRIGGER_TOPIC_CONFIG", e.to_string())
                })?,
            ))),
            trigger_ws_client::NODE_KIND => Ok(NodeDispatch::WsClientTrigger(
                trigger_ws_client::Node::new(serde_json::from_value(node.config.clone()).map_err(
                    |e| PipelineError::new("FW_NODE_TRIGGER_SOCKET_CONFIG", e.to_string()),
                )?),
            )),
            // Anything the arms above did not claim is not a native node, so it
            // is provided by a bundle, curated or third-party (`x.*`); the
            // namespace cannot tell them apart. The manifest decides role and
            // implementation at execution time, and reports a missing package
            // if there is none. Without a platform there is no bundle to ask.
            other => {
                let Some(platform) = &self.platform else {
                    return Err(PipelineError::new(
                        "FW_NODE_KIND_UNSUPPORTED",
                        format!("unsupported node kind '{}'", other),
                    ));
                };
                Ok(NodeDispatch::InstalledNode {
                    kind: other.to_string(),
                    config: node.config.clone(),
                    platform: platform.clone(),
                    // A bundle composing another bundle's node stays governed by
                    // its own declaration, so the policy in force here descends
                    // with the dispatch rather than being replaced by it.
                    egress: self.bundle_egress.clone(),
                })
            }
        }
    }
}

#[async_trait]
impl PipelineEngine for BasicPipelineEngine {
    fn id(&self) -> &'static str {
        "pipeline.basic"
    }

    fn validate_graph(&self, graph: &PipelineGraph) -> Result<(), PipelineError> {
        crate::contracts::kinds::validate_pipeline_activation(graph).map_err(|error| {
            PipelineError::new(
                error.violation_code().unwrap_or("FW_PIPELINE_CONTRACT"),
                error.to_string(),
            )
        })?;
        for node in &graph.nodes {
            // Skip upfront validation for nodes whose config contains {{ expr }} placeholders —
            // those are resolved per-input at runtime, so type validation must happen there.
            if scan_exprs(&node.config).is_empty() {
                self.build_node(node)?;
            } else {
                // A config still holding `{{ }}` is built per run, but its
                // provider is literal and its profile's closed words are
                // known now (`node-conventions.md` §11).
                ai::check_profile_of(&node.kind, &node.config)?;
            }
        }
        self.flow_plan(graph)?;
        Ok(())
    }

    async fn execute_with_options_async(
        &self,
        graph: &PipelineGraph,
        ctx: &PipelineContext,
        options: &ExecuteOptions,
    ) -> Result<PipelineOutput, PipelineError> {
        // The run is bracketed on the bus: `run_start` before the first node,
        // `run_done` after the last, whatever the outcome — the n8n-style
        // canvas badges read these, and so may any SSE client.
        let started = std::time::Instant::now();
        emit_lifecycle(
            &options.bus,
            "run_start",
            format!("run {} start", ctx.request_id),
            None,
            Some(json!({ "run_id": ctx.request_id })),
            &started,
        );
        let result = self.run_graph(graph, ctx, options).await;
        let status = if result.is_ok() { "ok" } else { "error" };
        let duration_ms = started.elapsed().as_millis() as u64;
        emit_lifecycle(
            &options.bus,
            "run_done",
            format!("run {} {} {} ms", ctx.request_id, status, duration_ms),
            None,
            Some(json!({ "run_id": ctx.request_id, "status": status, "duration_ms": duration_ms })),
            &started,
        );
        result
    }
}

/// One engine lifecycle signal on the run's bus, when the run has one.
///
/// The engine announces `run_start`, then per node `node_start` followed by
/// exactly one of `node_ok` / `node_empty` / `node_fail` / `node_retry` /
/// `node_error_routed`, then `run_done`. A node that never ran (every edge
/// into it skipped) gets one `node_skipped` and no `node_start`.
/// `node_retry` and `node_error_routed` are a failure an `:error` edge
/// consumed: `node_retry` when the edge reaches `logic.retry`
/// (`data: { attempt, max_attempts, duration_ms, error_code, message }`),
/// `node_error_routed` for any other consumer (`{ duration_ms, error_code,
/// message, to_node }`); `node_fail` is the unrouted failure only. Every
/// consumer of the bus forwards every signal, so a client that only cared
/// for node-emitted ones (an agent's steps) now sees these too — the SSE
/// help says so.
fn emit_lifecycle(
    bus: &Option<Arc<ExecutionBus>>,
    kind: &str,
    message: String,
    node: Option<(&str, &str)>,
    data: Option<Value>,
    run_started: &std::time::Instant,
) {
    let Some(bus) = bus else { return };
    let (node_id, node_kind) = node.unwrap_or(("", ""));
    bus.emit(Signal {
        kind: kind.to_string(),
        message,
        node_id: node_id.to_string(),
        node_kind: node_kind.to_string(),
        data,
        at: format!("{}ms", run_started.elapsed().as_millis()),
    });
}

/// `node_fail` for one node, whether the failure came from the node itself
/// or from resolving its config or building it.
fn emit_node_fail(
    bus: &Option<Arc<ExecutionBus>>,
    node_id: &str,
    node_kind: &str,
    error: &PipelineError,
    node_start: &std::time::Instant,
    run_started: &std::time::Instant,
) {
    let duration_ms = node_start.elapsed().as_millis() as u64;
    emit_lifecycle(
        bus,
        "node_fail",
        format!("{node_id} {} fail {duration_ms} ms", node_kind),
        Some((node_id, node_kind)),
        Some(json!({
            "duration_ms": duration_ms,
            "error_code": error.code,
            "error_class": crate::pipeline::error_class::class_of_error(error.code, &error.message).as_status_word(),
            "error": error.message,
        })),
        run_started,
    );
}


impl BasicPipelineEngine {
    /// The `$nodes` retention plan and the flow plan of a graph, refused
    /// here exactly as at activation (`node-conventions.md` §4).
    fn flow_plan(
        &self,
        graph: &PipelineGraph,
    ) -> Result<(NodesRetentionPlan, flow::FlowPlan), PipelineError> {
        let retention = build_nodes_retention_plan(graph)?;
        let plan = flow::FlowPlan::build(graph, &retention.consumer_access)?;
        Ok((retention, plan))
    }

    /// The run itself. `execute_with_options_async` brackets it with
    /// `run_start` / `run_done` on the bus; [`scheduler`] decides which node
    /// runs when.
    async fn run_graph(
        &self,
        graph: &PipelineGraph,
        ctx: &PipelineContext,
        options: &ExecuteOptions,
    ) -> Result<PipelineOutput, PipelineError> {
        self.validate_graph(graph)?;
        let (retention, plan) = self.flow_plan(graph)?;
        let run_started = std::time::Instant::now();
        let bus = options.bus.clone();
        // Project configuration is immutable for this invocation. Snapshot the
        // timeout once instead of reparsing zebflow.yaml for every node.
        let project_config = self
            .platform
            .as_ref()
            .map(|platform| {
                platform
                    .zebflow_cfg
                    .read_or_default(&ctx.owner, &ctx.project)
            })
            .transpose()
            .map_err(|err| PipelineError::new(err.code, err.message))?;
        let project_capture = project_config
            .as_ref()
            .and_then(|config| config.configs.pipelines.logging.trace_capture.as_ref())
            .cloned()
            .unwrap_or_default();
        let pipeline_capture = graph
            .metadata
            .as_ref()
            .and_then(|metadata| metadata.settings.trace_capture.as_ref());
        project_capture
            .validate()
            .map_err(|message| PipelineError::new("FW_TRACE_CONFIG", message))?;
        if let Some(settings) = pipeline_capture {
            settings
                .validate()
                .map_err(|message| PipelineError::new("FW_TRACE_CONFIG", message))?;
        }
        // Rule 3: whatever this project has taken out of the credential store is
        // masked in every payload, at every level, without any node declaring
        // it. Scoped to this owner/project — never another tenant's secrets.
        let trace_capture = TraceCapture::new(project_capture.resolve(pipeline_capture))
            .with_confidential(crate::platform::services::credential::confidential_values(
                &ctx.owner,
                &ctx.project,
            ));
        let project_timeout_secs = project_config
            .as_ref()
            .map(|config| config.configs.pipelines.effective_node_timeout_secs())
            .or_else(|| {
                std::env::var("PIPELINE_NODE_TIMEOUT_SECS")
                    .ok()
                    .and_then(|value| value.parse().ok())
            })
            .unwrap_or(crate::platform::model::default_pipeline_node_timeout_secs());
        let mut run = scheduler::RunState {
            ctx,
            graph,
            bus,
            run_started,
            project_timeout_secs,
            trace_capture,
            retention,
            responder: options.responder.clone(),
            trace: vec![format!("engine={}", self.id())],
            node_trace: Vec::new(),
            response: None,
            frames: vec![HashMap::new()],
            last_answer: vec![None; graph.nodes.len()],
            last_in_time: None,
        };
        let mut top = scheduler::ScopeRun::top(&plan, graph, &ctx.input);
        self.run_scope(graph, &plan, &mut top, &mut run).await?;
        Ok(run.finish(&plan))
    }

    /// One node, once: resolve its config, build it, run it under its
    /// timeout, and record it. `Ran::Answered` carries its outputs with the
    /// response envelope and `__signal` taken out; `Ran::Routed` is a failure
    /// its wired `:error` pin takes, with the payload that pin delivers —
    /// whether it failed resolving its config, building, running or timing
    /// out. `Err` is the unrouted failure: the run fails there.
    async fn run_node(
        &self,
        graph: &PipelineGraph,
        plan: &flow::FlowPlan,
        idx: usize,
        delivery: scheduler::Delivery,
        run: &mut scheduler::RunState<'_>,
    ) -> Result<scheduler::Ran, PipelineError> {
        let node = &graph.nodes[idx];
        let ctx = run.ctx;
        let bus = run.bus.clone();
        let run_started = run.run_started;
        let mut metadata = execution_metadata(ctx, run.nodes_scope(plan, node), ctx.placeholder.clone());
        if let Some(map) = metadata.as_object_mut() {
            if let Some(series) = delivery.series {
                map.insert(crate::pipeline::expr::FOREACH_METADATA_KEY.to_string(), series);
            }
            if let Some(items) = delivery.loop_items {
                map.insert(logic::LOOP_ITEMS_METADATA_KEY.to_string(), Value::Array(items));
            }
        }
        let input = NodeExecutionInput {
            node_id: node.id.clone(),
            input_pin: delivery.input_pin,
            payload: delivery.payload,
            metadata,
            bus: bus.clone(),
        };

        // The node's clock starts here, before its config is resolved and
        // it is built, so a failure in either is announced as this node's
        // and the badge never stays "running".
        let node_start = std::time::Instant::now();
        emit_lifecycle(
            &bus,
            "node_start",
            format!("{} {} start", node.id, node.kind),
            Some((&node.id, &node.kind)),
            None,
            &run_started,
        );

        // Resolve {{ expr }} placeholders before building, against the
        // `$nodes` of this node's ancestors as they stand now. A failure here
        // or in the build is the node's failure and routes like any other.
        let resolved = resolve_config_expressions(
            node.config.clone(),
            &input.payload,
            &input.metadata,
            &self.language,
        );
        let effective_config = resolved.as_ref().map_or_else(|_| node.config.clone(), Clone::clone);
        run.trace_capture.begin_node();
        // A declared canvas preview asks for that payload in the record —
        // see `TraceCapture::records_payload_for`. Read off the stored
        // config: `preview` is presentation, never expression-resolved.
        // An input node's widget is its input view — the Run form shows
        // what went in from the record — so the family asks for its input
        // payload the way a declared `--preview-in` does.
        let preview_in =
            node.config["preview"]["in"].is_object() || input::is_input_kind(&node.kind);
        let preview_out = node.config["preview"]["out"].is_object();
        // Captured before the node runs, because the error path needs it and
        // the outcome is not known yet. At `on-error` a successful node
        // drops it below; at `none` it is never taken at all.
        let base_trace_config = if run.trace_capture.records_any_payload() {
            run.trace_capture
                .config(&effective_config)
                .map(|config| mask_secret_config(&node.kind, config))
        } else {
            None
        };
        let built = resolved.and_then(|config| {
            if config == node.config {
                // No expressions resolved — use the node as stored.
                self.build_node(node)
            } else {
                self.build_node(&PipelineNode { config, ..node.clone() })
            }
        });

        let trace_node_id = node.id.clone();
        let trace_node_kind = node.kind.clone();
        let input_snapshot = input.payload.clone();

        // Per-node timeout: the node's `--timeout` → project config → env
        // var → default. A `--timeout` that is not a duration from 1s to 1h
        // fails the node here rather than being clamped into another limit.
        let per_node_timeout = crate::pipeline::model::node_timeout(&effective_config);
        let node_timeout = match &per_node_timeout {
            Ok(Some(duration)) => *duration,
            _ => std::time::Duration::from_secs(run.project_timeout_secs),
        };
        let input_for_exec = input.clone();
        let exec_result: Result<Vec<NodeExecutionOutput>, PipelineError> = match built {
            Err(e) => Err(e),
            Ok(dispatch) => {
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
                    NodeDispatch::SekejapRecord(node) => {
                        node.execute_many_async(input_for_exec).await
                    }
                    NodeDispatch::Postgres(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::InlineWebResponse { node_id, config } => {
                        let markup = config.markup.as_deref().unwrap_or("").trim();
                        if markup.is_empty() {
                            Err(PipelineError::new(
                                web::response::CODE_CONFIG,
                                format!("node '{node_id}' --template set but markup not loaded"),
                            ))
                        } else {
                            // Every value arrives final — {{ }} resolved above.
                            let head = config.check()?;

                            let template_id = config.template.clone().unwrap_or_default();
                            let source_path = self
                                .template_root
                                .as_ref()
                                .and_then(|r| config.template.as_deref().map(|t| r.join(t)));
                            let options = crate::rwe::ReactiveWebOptions {
                                templates: crate::rwe::TemplateOptions {
                                    template_root: self.template_root.clone(),
                                    library_roots: self.library_roots(),
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
                                let fresh = web::response::compile_page(
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

                            let render_out = web::response::render_compiled_page(
                                &compiled,
                                input.payload.clone(),
                                input.metadata,
                                self.rwe.as_ref(),
                                self.language.as_ref(),
                                &ctx.request_id,
                                enabled_libraries,
                            )?;
                            let mut envelope = head.envelope();
                            for key in ["html", "compiled_scripts", "hydration_payload"] {
                                envelope.insert(
                                    key.to_string(),
                                    render_out.payload.get(key).cloned().unwrap_or(Value::Null),
                                );
                            }
                            Ok(vec![NodeExecutionOutput {
                                output_pins: render_out.output_pins,
                                payload: web::response::with_envelope(&input.payload, Value::Object(envelope)),
                                trace: render_out.trace,
                            }])
                        })
                        }
                    }
                    NodeDispatch::WebResponse(node) => {
                        node.execute_many_async(input_for_exec).await
                    }
                    NodeDispatch::InlineWebSiteGenerate { node_id, config } => {
                        self.run_site_generate(&node_id, &config, &input.payload, &input.metadata, ctx)
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
                    NodeDispatch::AuthTokenVerify(node) => {
                        node.execute_many_async(input_for_exec).await
                    }
                    NodeDispatch::MailSend(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::Concept(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::WebError(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::WsTrigger(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::WsMessageSend(node) => {
                        node.execute_many_async(input_for_exec).await
                    }
                    NodeDispatch::WsState(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::Crypto(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::Kv(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::TriggerFunction(node) => {
                        node.execute_many_async(input_for_exec).await
                    }
                    NodeDispatch::FunctionCall(node) => {
                        node.execute_many_async(input_for_exec).await
                    }
                    NodeDispatch::FilePut(node) => node.execute_many_async(input_for_exec).await,
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
                    NodeDispatch::Barcode(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::SvgConvert(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::ImgChromakey(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::Input(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::McpTrigger(node) => node.execute_many_async(input_for_exec).await,
                    NodeDispatch::KvSubscribe(node) => {
                        node.execute_many_async(input_for_exec).await
                    }
                    NodeDispatch::WsClientTrigger(node) => {
                        node.execute_many_async(input_for_exec).await
                    }
                    NodeDispatch::InstalledNode {
                        kind,
                        config,
                        platform,
                        egress,
                    } => {
                        execute_installed_node(kind, config, platform, egress, input_for_exec).await
                    }
                }
            }; // end exec_fut
                let timeout_node_id = trace_node_id.clone();
                let timeout_is_per_node = matches!(per_node_timeout, Ok(Some(_)));
                match per_node_timeout {
                    Err(refused) => Err(refused),
                    Ok(_) => tokio::time::timeout(node_timeout, exec_fut).await.unwrap_or_else(|_| {
                        let source = if timeout_is_per_node { "--timeout" } else { "the project's node timeout" };
                        Err(PipelineError::new(
                            "FW_NODE_TIMEOUT",
                            format!(
                                "node '{}' timed out after {} ({source})",
                                timeout_node_id,
                                crate::pipeline::nodes::shared::units::describe_duration(node_timeout)
                            ),
                        ))
                    }),
                }
            }
        };

        let mut outs = match exec_result {
            Ok(outs) => outs,
            Err(e) => {
                return self.node_failed(plan, idx, node, e, &input_snapshot, base_trace_config, &node_start, run);
            }
        };
        // The first `web.response.send` of a run answers the caller; a
        // second one answers nobody, so it is recorded as having emitted
        // nothing (`node-conventions.md` §4).
        let second_response = node.kind == web::response::NODE_KIND && run.response.is_some();
        if second_response {
            outs.clear();
        }
        let mut processed_payloads: Vec<Value> = Vec::new();
        // Skipped entirely when not recorded: not capturing is cheaper than
        // capturing and discarding, which is the point on a device.
        let record_input = run.trace_capture.records_payload_for(true, preview_in);
        let record_output = run.trace_capture.records_payload_for(true, preview_out);
        let trace_input = if record_input {
            run.trace_capture.capture(&input_snapshot)
        } else {
            Value::Null
        };
        for out in &mut outs {
            // Writing the files a node answered with is part of the node: a
            // failure here routes like any other.
            out.payload = match materialize_node_output_files(
                self.platform.as_ref(),
                ctx,
                &trace_node_kind,
                out.payload.clone(),
            ) {
                Ok(payload) => payload,
                Err(e) => {
                    return self.node_failed(plan, idx, node, e, &input_snapshot, base_trace_config, &node_start, run);
                }
            };
            let mut output_payload = out.payload.clone();
            let payload_redact_tokens = take_private_redact_tokens(&mut output_payload);
            let payload_redact_except_paths = take_private_redact_except_paths(&mut output_payload);
            let payload_output = if payload_redact_tokens.is_empty() {
                output_payload
            } else {
                redact_json_value(&output_payload, &payload_redact_tokens, &payload_redact_except_paths, &[])
            };
            processed_payloads.push(payload_output.clone());
            out.payload = payload_output;
        }

        // Rule 3 by position: the node kind declares where a secret sits in
        // its own output, and only the record is masked. `outs` — what the
        // next node receives — is untouched, because a level shapes the
        // record, never the run.
        let node_output_value = if !record_output {
            Value::Null
        } else {
            match declared_secret_paths(&trace_node_kind) {
                Some(paths) => crate::pipeline::trace_capture::mask_secret_paths(&run.trace_capture.outputs(&outs), paths),
                None => run.trace_capture.outputs(&outs),
            }
        };
        let nodes_output_value = if processed_payloads.len() == 1 {
            processed_payloads[0].clone()
        } else {
            Value::Array(processed_payloads)
        };
        // A declared image preview of a temporary file keeps a small copy in
        // the record, since the file itself is deleted with the run.
        let preview_snapshot = if preview_in || preview_out {
            crate::pipeline::trace_capture::preview_snapshot::snapshot_for(
                self.platform.as_ref(),
                &ctx.owner,
                &ctx.project,
                &node.config,
                &input_snapshot,
                &nodes_output_value,
            )
        } else {
            None
        };
        if !second_response {
            run.store_answer(node, idx, nodes_output_value);
        }
        // A `logic.retry` that sent a verdict round again is a wait, and the
        // canvas should say so: `retry` in the record, `node_retry` on the
        // bus, with the count the node stamped in `__zf_retry`. On the error
        // road the failing node has already told this story, so the retry
        // node stays `ok` there.
        let verdict_retry = trace_node_kind == logic::retry::NODE_KIND
            && outs.iter().any(|o| o.output_pins.iter().any(|p| p == logic::retry::OUTPUT_PIN_RETRY))
            && input_snapshot
                .get(RETRY_STATE_KEY)
                .and_then(|s| s.get("failing_node_id"))
                .is_none();
        let status_word = if verdict_retry {
            "retry"
        } else if outs.is_empty() {
            // Ran and emitted nothing — a match with no case, a filter that
            // filtered everything, a loop over an empty list. Not an error;
            // grey in a run view where a green dot would lie.
            "empty"
        } else {
            "ok"
        };
        run.node_trace.push(NodeTraceEntry {
            node_id: trace_node_id.clone(),
            node_kind: trace_node_kind.clone(),
            config: if run.trace_capture.records_successful_payloads() { base_trace_config } else { None },
            duration_ms: node_start.elapsed().as_millis() as u64,
            input: trace_input,
            output: node_output_value,
            error: second_response.then(|| {
                "a response was already sent to the caller in this run; this one was not sent".to_string()
            }),
            status: status_word.to_string(),
            error_code: None,
            preview_snapshot,
        });
        let duration_ms = node_start.elapsed().as_millis() as u64;
        if verdict_retry {
            let state = outs
                .first()
                .and_then(|o| o.payload.get(RETRY_STATE_KEY))
                .cloned()
                .unwrap_or(Value::Null);
            let attempt = state.get("attempt").and_then(Value::as_u64).unwrap_or(1);
            let max_attempts = state.get("max_attempts").and_then(Value::as_u64);
            // The reason a wait is a wait: the pause the node took before the
            // next attempt, as it stamped it (`--delay` grown by `--backoff`).
            let message = state
                .get("next_delay_ms")
                .and_then(Value::as_u64)
                .filter(|d| *d > 0)
                .map(|d| format!("next attempt in {d} ms"));
            emit_lifecycle(
                &bus,
                "node_retry",
                match max_attempts {
                    Some(max) => format!("{trace_node_id} {trace_node_kind} waiting {attempt}/{max} {duration_ms} ms"),
                    None => format!("{trace_node_id} {trace_node_kind} waiting {attempt} {duration_ms} ms"),
                },
                Some((&trace_node_id, &trace_node_kind)),
                Some(json!({
                    "attempt": attempt,
                    "max_attempts": max_attempts,
                    "duration_ms": duration_ms,
                    "message": message,
                })),
                &run_started,
            );
        } else {
            let outcome = if outs.is_empty() { "node_empty" } else { "node_ok" };
            emit_lifecycle(
                &bus,
                outcome,
                format!("{trace_node_id} {trace_node_kind} {duration_ms} ms"),
                Some((&trace_node_id, &trace_node_kind)),
                Some(json!({ "duration_ms": duration_ms })),
                &run_started,
            );
        }

        for output in &mut outs {
            // ── __signal extraction ──────────────────────────────────────
            // Strip `__signal` from node output and route through the bus.
            // Supports: string, object {kind,message,data}, or array of either.
            if let Some(raw_signal) = output.payload.as_object_mut().and_then(|m| m.remove("__signal")) {
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
                                kind: obj.get("kind").and_then(Value::as_str).unwrap_or("signal").to_string(),
                                message: obj.get("message").and_then(Value::as_str).unwrap_or("").to_string(),
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
            // ── the answer to the caller ─────────────────────────────────
            // `web.response.send` hands what it answers beside its payload;
            // it leaves the payload here, so the next node receives what the
            // response node received. The first answer goes to the caller at
            // once, through the responder the HTTP ingress waits on.
            if node.kind == web::response::NODE_KIND
                && let Some(envelope) = output
                    .payload
                    .as_object_mut()
                    .and_then(|m| m.remove(web::response::ENVELOPE_KEY))
                && run.response.is_none()
            {
                if let Some(responder) = &run.responder {
                    responder.send(strip_private_markers(envelope.clone()));
                }
                run.response = Some(envelope);
            }
            run.trace.extend(output.trace.clone());
        }
        let last = outs.last().map(|o| o.payload.clone());
        if last.is_some() {
            run.last_in_time = last.clone();
        }
        run.last_answer[idx] = last;
        Ok(scheduler::Ran::Answered(outs))
    }

    /// A node that failed: routed when its `:error` pin is wired (a retry
    /// when that edge reaches `logic.retry`), else the run's failure.
    #[allow(clippy::too_many_arguments)]
    fn node_failed(
        &self,
        plan: &flow::FlowPlan,
        idx: usize,
        node: &PipelineNode,
        mut e: PipelineError,
        input_snapshot: &Value,
        base_trace_config: Option<Value>,
        node_start: &std::time::Instant,
        run: &mut scheduler::RunState<'_>,
    ) -> Result<scheduler::Ran, PipelineError> {
        let (trace_node_id, trace_node_kind) = (&node.id, &node.kind);
        let bus = run.bus.clone();
        let run_started = run.run_started;
        // Attribute error to the failing node if not already set.
        if e.node_id.is_none() {
            e.node_id = Some(trace_node_id.clone());
            e.node_kind = Some(trace_node_kind.clone());
        }
        // A failure an `:error` edge consumes is not the run failing: it is a
        // retry (the consumer is `logic.retry`) or an error handled by
        // whatever the edge reaches. The record and the bus say which, so a
        // poll loop that waited eight times and then succeeded does not show
        // eight red crosses. `node_fail` is for the unrouted failure that
        // ends the run.
        let routed: Vec<usize> = plan.outgoing[idx]
            .iter()
            .copied()
            .filter(|&edge| plan.edges[edge].from_pin == flow::ERROR_PIN)
            .collect();
        let retry_consumer = routed
            .iter()
            .map(|&edge| plan.edges[edge].to)
            .find(|&to| run.graph.nodes[to].kind == logic::retry::NODE_KIND);
        let error_payload = (!routed.is_empty()).then(|| {
            let last_attempt = retry_consumer
                .and_then(|retry| run.answer_of(retry))
                .map(retry_attempt_from_payload)
                .unwrap_or(0);
            build_retry_error_payload(input_snapshot, &e, last_attempt)
        });
        let duration_ms = node_start.elapsed().as_millis() as u64;
        let routed_status = match (routed.first(), retry_consumer) {
            (None, _) => None,
            (Some(_), Some(retry)) => {
                let attempt = error_payload.as_ref().map(retry_attempt_from_payload).unwrap_or(1);
                let max_attempts = retry_max_attempts(&run.graph.nodes[retry]);
                emit_lifecycle(
                    &bus,
                    "node_retry",
                    match max_attempts {
                        Some(max) => format!("{trace_node_id} {trace_node_kind} retry {attempt}/{max} {duration_ms} ms"),
                        None => format!("{trace_node_id} {trace_node_kind} retry {attempt} {duration_ms} ms"),
                    },
                    Some((trace_node_id, trace_node_kind)),
                    Some(json!({
                        "attempt": attempt,
                        "max_attempts": max_attempts,
                        "duration_ms": duration_ms,
                        "error_code": e.code,
                        "message": e.message,
                    })),
                    &run_started,
                );
                Some("retry")
            }
            (Some(&first), None) => {
                let to_node = run.graph.nodes[plan.edges[first].to].id.clone();
                emit_lifecycle(
                    &bus,
                    "node_error_routed",
                    format!("{trace_node_id} {trace_node_kind} error → {to_node} {duration_ms} ms"),
                    Some((trace_node_id, trace_node_kind)),
                    Some(json!({
                        "duration_ms": duration_ms,
                        "error_code": e.code,
                        "message": e.message,
                        "to_node": to_node,
                    })),
                    &run_started,
                );
                Some("error_routed")
            }
        };
        if routed_status.is_none() {
            emit_node_fail(&bus, trace_node_id, trace_node_kind, &e, node_start, &run_started);
        }
        let input = if run.trace_capture.records_any_payload() {
            run.trace_capture.capture(input_snapshot)
        } else {
            Value::Null
        };
        run.node_trace.push(NodeTraceEntry {
            node_id: trace_node_id.clone(),
            node_kind: trace_node_kind.clone(),
            config: base_trace_config,
            duration_ms,
            input,
            output: Value::Null,
            // The error text stays so the Logs panel says why, whichever way
            // the failure went.
            error: Some(e.message.clone()),
            status: match routed_status {
                Some(word) => word.to_string(),
                None => crate::pipeline::error_class::class_of_error(e.code, &e.message).as_status_word().to_string(),
            },
            error_code: Some(e.code.to_string()),
            preview_snapshot: None,
        });
        run.last_answer[idx] = None;
        match error_payload {
            Some(payload) => Ok(scheduler::Ran::Routed(payload)),
            None => {
                e.node_trace = run.node_trace.clone();
                Err(e)
            }
        }
    }
}

/// Removes the engine's private redaction markers from a pipeline's result.
///
/// `__zf_private_trace_redact` and its siblings are how a node tells the trace
/// sanitizer which literal values to strike out; they travel in the payload
/// because that is the only channel between nodes. They are not part of
/// anybody's API, and the trace marker's contents are the secrets themselves,
/// so none of them may ride out to a caller in the response body.
///
/// The walk is recursive because a marker does not stay at the top: an
/// `web.response.send` in the chain nests the whole upstream payload under
/// `__zf_response.json`, and a top-level-only sweep would leave it there.
fn strip_private_markers(value: Value) -> Value {
    match value {
        Value::Object(map) => Value::Object(
            map.into_iter()
                .filter(|(key, _)| {
                    !matches!(
                        key.as_str(),
                        "__zf_private_trace_redact"
                            | "__zf_private_redact"
                            | "__zf_private_redact_except_paths"
                    )
                })
                .map(|(key, item)| (key, strip_private_markers(item)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.into_iter().map(strip_private_markers).collect()),
        other => other,
    }
}

/// Declared secret positions per node kind — `NodeDefinition::secret_paths`.
///
/// Built once. Looking these up per node execution by rebuilding the whole
/// definition list would cost more than the masking it enables.
static SECRET_PATHS_BY_KIND: std::sync::LazyLock<
    std::collections::HashMap<String, Vec<String>>,
> = std::sync::LazyLock::new(|| {
    crate::pipeline::nodes::builtin_node_definitions()
        .into_iter()
        .filter(|def| !def.secret_paths.is_empty())
        .map(|def| (def.kind, def.secret_paths))
        .collect()
});

/// Declared secret positions for a node kind, if it declares any.
fn declared_secret_paths(kind: &str) -> Option<&'static [String]> {
    SECRET_PATHS_BY_KIND.get(kind).map(Vec::as_slice)
}

/// The config keys of each kind's flags declared `secret`, built once.
static SECRET_CONFIG_KEYS_BY_KIND: std::sync::LazyLock<
    std::collections::HashMap<String, Vec<String>>,
> = std::sync::LazyLock::new(|| {
    crate::pipeline::nodes::builtin_node_definitions()
        .into_iter()
        .filter_map(|def| {
            let keys: Vec<String> = def
                .dsl_flags
                .iter()
                .filter(|flag| flag.secret)
                .map(|flag| format!("/{}", flag.config_key))
                .collect();
            (!keys.is_empty()).then_some((def.kind, keys))
        })
        .collect()
});

/// The recorded config with every value a flag declares `secret` masked: the
/// resolved config holds what the expression gave, a plaintext password
/// included.
fn mask_secret_config(kind: &str, config: Value) -> Value {
    match SECRET_CONFIG_KEYS_BY_KIND.get(kind) {
        Some(keys) => crate::pipeline::trace_capture::mask_secret_paths(&config, keys),
        None => config,
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    /// A flag declared `secret` is masked in the recorded config; the rest
    /// of the config is recorded as it was.
    #[test]
    fn a_secret_flag_is_masked_in_the_recorded_config() {
        let config = serde_json::json!({ "from": "a plaintext password", "hash": "$argon2id$v=19$stored" });
        let recorded = super::mask_secret_config("crypto.password.verify", config.clone());
        assert_ne!(recorded["from"], config["from"]);
        assert_ne!(recorded["hash"], config["hash"]);
        assert_eq!(super::mask_secret_config("logic.if", config.clone()), config);
    }

    use serde_json::json;

    use super::{
        BasicPipelineEngine, NodesAccess, NodesAccessAccumulator, PipelineContext,
        redact_json_value, sanitized_trace_value, scan_text_nodes_access,
        take_private_redact_except_paths, take_private_redact_tokens, trace_config_snapshot,
    };
    use crate::pipeline::interface::PipelineEngine;
    use crate::pipeline::model::{PipelineEdge, PipelineGraph, PipelineNode};
    use crate::pipeline::nodes::basic::{
        javascript,
        table::convert::{TableFormat, collect_columns, encode_rows},
    };
    use crate::platform::model::PlatformConfig;
    use crate::platform::services::PlatformService;
    use crate::platform::shell::parser::build_pipeline_graph;

    /// A swappable task's provider is literal and its profile closed
    /// (`node-conventions.md` §11): a config still holding `{{ }}` is not
    /// built until it runs, but its provider and its profile's closed words
    /// are checked when the graph is validated.
    #[test]
    fn a_provider_profile_is_checked_when_the_graph_is_validated() {
        let graph = |line: &str| {
            build_pipeline_graph("profile-check", &format!("[a] trigger.manual\n[b] {line}\n[a] -> [b]")).expect("graph")
        };
        let engine = BasicPipelineEngine::default();
        engine
            .validate_graph(&graph(r#"ai.text.generate --provider openai --credential c --prompt "{{ input.manual.q }}""#))
            .expect("a literal provider with a profile");
        for (line, says) in [
            (r#"ai.text.generate --provider "{{ input.manual.who }}" --credential c --prompt "{{ input.manual.q }}""#, "literal"),
            (r#"ai.text.generate --provider mistral --credential c --prompt "{{ input.manual.q }}""#, "'mistral' is not one of openai, openrouter"),
            (r#"ai.audio.generate --provider piper --credential c --text "{{ input.manual.q }}" --option pitch=2"#, "'pitch' is not a setting of --provider piper"),
            (r#"ai.audio.generate --provider piper --credential c --text hi --option volume=loud"#, "not a number"),
        ] {
            let err = engine.validate_graph(&graph(line)).expect_err("refused");
            assert!(matches!(err.code, "FW_NODE_AI_TEXT_GENERATE_CONFIG" | "FW_NODE_AI_AUDIO_GENERATE_CONFIG"), "{}", err.code);
            assert!(err.message.contains(says), "{}", err.message);
        }
    }

    /// Compaction affects logs only, including across intermediate nodes. A
    /// zero override must preserve all array items while other caps still apply.
    #[tokio::test]
    async fn trace_capture_preserves_execution_data_and_pipeline_overrides() {
        use crate::pipeline::model::{PipelineGraphMetadata, PipelineGraphSettings};
        use crate::pipeline::trace_capture::TraceCaptureSettings;
        let mut graph = build_pipeline_graph(
            "trace-capture",
            r#"
[a] trigger.manual
[b] logic.if --when "1 == 1"
[c] javascript.script.run -- "return { count: input.manual.rows.length, last: input.manual.rows[input.manual.rows.length - 1] };"
[a] -> [b]
[b]:true -> [c]
"#,
        )
        .expect("graph");
        graph.metadata = Some(PipelineGraphMetadata {
            settings: PipelineGraphSettings {
                trace_capture: Some(TraceCaptureSettings {
                    // This test is about what capture *does* to a payload, so
                    // it asks for the level that records one. The default is
                    // on-error, which deliberately records nothing for a node
                    // that succeeded.
                    level: Some(crate::pipeline::trace_capture::CaptureLevel::Full),
                    array_sample_count: Some(1),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        });
        let ctx = PipelineContext {
            owner: "test".into(),
            project: "test".into(),
            pipeline: "trace-capture".into(),
            request_id: "trace-test".into(),
            route: String::new(),
            input: json!({"rows": (0..100).collect::<Vec<_>>()}),
            trigger: None,
            placeholder: None,
        };
        let engine = BasicPipelineEngine::default();
        let sampled = engine
            .execute_async(&graph, &ctx)
            .await
            .expect("sampled execution");
        assert_eq!(sampled.value["script"], json!({"count":100,"last":99}));
        for trace in &sampled.node_trace {
            // The trigger records the envelope it was given; every node
            // after it reads that envelope under `manual`.
            let rows = trace.input.get("manual").map_or(&trace.input["rows"], |m| &m["rows"]);
            assert_eq!(rows["len"], 100);
            assert_eq!(rows["preview"], json!([0]));
        }
        graph
            .metadata
            .as_mut()
            .unwrap()
            .settings
            .trace_capture
            .as_mut()
            .unwrap()
            .array_sample_count = Some(0);
        let full = engine
            .execute_async(&graph, &ctx)
            .await
            .expect("unsampled execution");
        assert_eq!(full.value, sampled.value);
        assert_eq!(
            full.node_trace[0].input["rows"].as_array().unwrap().len(),
            100
        );
    }

    /// Capture levels decide what a run records — `kinds/invocation-record`
    /// § Capture Level. Asserted against a canary that would be plainly visible
    /// in a payload if the level kept it.
    #[tokio::test]
    async fn capture_levels_decide_what_a_run_records() {
        use crate::pipeline::model::{PipelineGraphMetadata, PipelineGraphSettings};
        use crate::pipeline::trace_capture::{CaptureLevel, TraceCaptureSettings};

        async fn run_at(level: CaptureLevel, body: &str) -> crate::pipeline::model::PipelineOutput {
            let mut graph = build_pipeline_graph(
                "levels",
                &format!("[a] trigger.manual\n[b] javascript.script.run -- \"{body}\"\n[a] -> [b]\n"),
            )
            .expect("graph");
            graph.metadata = Some(PipelineGraphMetadata {
                settings: PipelineGraphSettings {
                    trace_capture: Some(TraceCaptureSettings {
                        level: Some(level),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                ..Default::default()
            });
            let ctx = PipelineContext {
                owner: "test".into(),
                project: "test".into(),
                pipeline: "levels".into(),
                request_id: "levels-test".into(),
                route: String::new(),
                input: json!({ "canary": "CANARY-IN-INPUT" }),
                trigger: None,
                placeholder: None,
            };
            BasicPipelineEngine::default()
                .execute_async(&graph, &ctx)
                .await
                .expect("execution")
        }

        let ok_body = "return { seen: input.manual.canary };";

        // full — everything is there.
        let out = run_at(CaptureLevel::Full, ok_body).await;
        let text = serde_json::to_string(&out.node_trace).unwrap();
        assert!(text.contains("CANARY-IN-INPUT"), "full must record payloads");

        // on-error — a node that succeeded records nothing, but the run still
        // returns its real value: a level shapes the record, never the run.
        let out = run_at(CaptureLevel::OnError, ok_body).await;
        let text = serde_json::to_string(&out.node_trace).unwrap();
        assert!(
            !text.contains("CANARY-IN-INPUT"),
            "on-error must not record a successful node's payload: {text}"
        );
        assert_eq!(out.value["script"], json!({ "seen": "CANARY-IN-INPUT" }));
        assert!(
            out.node_trace.iter().all(|t| t.status == "ok"),
            "status must survive at on-error"
        );

        // none — nothing, even when a node fails.
        let out = run_at(CaptureLevel::None, ok_body).await;
        let text = serde_json::to_string(&out.node_trace).unwrap();
        assert!(!text.contains("CANARY-IN-INPUT"), "none must record nothing");
        assert!(
            !out.node_trace.is_empty(),
            "none still records that the nodes ran"
        );
    }

    /// The point of `on-error`: the run that fails is the one you can read.
    #[tokio::test]
    async fn on_error_records_the_node_that_failed() {
        use crate::pipeline::model::{PipelineGraphMetadata, PipelineGraphSettings};
        use crate::pipeline::trace_capture::{CaptureLevel, TraceCaptureSettings};

        let mut graph = build_pipeline_graph(
            "levels-fail",
            "[a] trigger.manual\n[b] javascript.script.run -- \"throw new Error('boom');\"\n[a] -> [b]\n",
        )
        .expect("graph");
        graph.metadata = Some(PipelineGraphMetadata {
            settings: PipelineGraphSettings {
                trace_capture: Some(TraceCaptureSettings {
                    level: Some(CaptureLevel::OnError),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        });
        let ctx = PipelineContext {
            owner: "test".into(),
            project: "test".into(),
            pipeline: "levels-fail".into(),
            request_id: "levels-fail-test".into(),
            route: String::new(),
            input: json!({ "canary": "CANARY-IN-INPUT" }),
            trigger: None,
            placeholder: None,
        };
        let result = BasicPipelineEngine::default().execute_async(&graph, &ctx).await;
        let trace = match &result {
            Ok(out) => out.node_trace.clone(),
            Err(e) => e.node_trace.clone(),
        };
        let text = serde_json::to_string(&trace).unwrap();
        assert!(
            text.contains("CANARY-IN-INPUT"),
            "on-error must record the failing node's input: {text}"
        );
    }

    /// A declared preview records its own payload at `on-error` — and only
    /// its own: the node beside it, and its other half, stay unrecorded. At
    /// `none` the preview changes nothing.
    #[tokio::test]
    async fn a_declared_preview_records_its_payload_at_on_error() {
        use crate::pipeline::model::{PipelineGraphMetadata, PipelineGraphSettings};
        use crate::pipeline::trace_capture::{CaptureLevel, TraceCaptureSettings};

        async fn run_at(level: CaptureLevel) -> crate::pipeline::model::PipelineOutput {
            let mut graph = build_pipeline_graph(
                "preview-levels",
                "[a] trigger.manual\n[b] javascript.script.run --preview json -- \"return { seen: input.manual.canary };\"\n[a] -> [b]\n",
            )
            .expect("graph");
            assert_eq!(graph.nodes[1].config["preview"]["out"], json!({ "as": "json" }));
            graph.metadata = Some(PipelineGraphMetadata {
                settings: PipelineGraphSettings {
                    trace_capture: Some(TraceCaptureSettings {
                        level: Some(level),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                ..Default::default()
            });
            let ctx = PipelineContext {
                owner: "test".into(),
                project: "test".into(),
                pipeline: "preview-levels".into(),
                request_id: "preview-levels-test".into(),
                route: String::new(),
                input: json!({ "canary": "CANARY-IN-INPUT" }),
                trigger: None,
                placeholder: None,
            };
            BasicPipelineEngine::default()
                .execute_async(&graph, &ctx)
                .await
                .expect("execution")
        }

        let out = run_at(CaptureLevel::OnError).await;
        let a = out.node_trace.iter().find(|t| t.node_id == "a").expect("node a");
        let b = out.node_trace.iter().find(|t| t.node_id == "b").expect("node b");
        assert_eq!(a.output, serde_json::Value::Null, "no preview on a: nothing recorded");
        assert_eq!(a.input, serde_json::Value::Null);
        assert_eq!(b.input, serde_json::Value::Null, "b previews its output, not its input");
        assert_eq!(
            b.output["script"],
            json!({ "seen": "CANARY-IN-INPUT" }),
            "b previews its output, so on-error records it"
        );
        assert!(b.config.is_none(), "config capture is unchanged by a preview");

        let out = run_at(CaptureLevel::None).await;
        let text = serde_json::to_string(&out.node_trace).unwrap();
        assert!(!text.contains("CANARY-IN-INPUT"), "none records nothing, preview or not: {text}");
    }

    /// The engine announces the run on the bus: `run_start`, then per node
    /// `node_start` and exactly one outcome, then `run_done` — in execution
    /// order, with the node named and its duration in `data`. A failing
    /// node gets `node_fail` with its code, and the run still gets its
    /// `run_done` (status `error`).
    #[tokio::test]
    async fn the_engine_announces_run_and_node_lifecycle_on_the_bus() {
        use crate::pipeline::model::{
            ExecuteOptions, ExecutionBus, PipelineError, PipelineOutput, Signal,
        };

        async fn signals_of(dsl: &str) -> (Result<PipelineOutput, PipelineError>, Vec<Signal>) {
            let graph = build_pipeline_graph("lifecycle", dsl).expect("graph");
            let ctx = PipelineContext {
                owner: "test".into(),
                project: "test".into(),
                pipeline: "lifecycle".into(),
                request_id: "lifecycle-run-1".into(),
                route: String::new(),
                input: json!({ "n": 1 }),
                trigger: None,
                placeholder: None,
            };
            let bus = Arc::new(ExecutionBus::new(64));
            let mut rx = bus.subscribe();
            let options = ExecuteOptions { bus: Some(bus), ..Default::default() };
            let result = BasicPipelineEngine::default()
                .execute_with_options_async(&graph, &ctx, &options)
                .await;
            let mut signals = Vec::new();
            while let Ok(signal) = rx.try_recv() {
                signals.push(signal);
            }
            (result, signals)
        }

        let (result, signals) = signals_of(
            "[a] trigger.manual\n[b] javascript.script.run -- \"return { n: input.n + 1 };\"\n[a] -> [b]\n",
        )
        .await;
        result.expect("the run succeeds");
        let seen: Vec<(String, String)> = signals
            .iter()
            .map(|s| (s.kind.clone(), s.node_id.clone()))
            .collect();
        assert_eq!(
            seen,
            [
                ("run_start", ""),
                ("node_start", "a"),
                ("node_ok", "a"),
                ("node_start", "b"),
                ("node_ok", "b"),
                ("run_done", ""),
            ]
            .map(|(k, n)| (k.to_string(), n.to_string()))
        );
        let node_ok = &signals[4];
        assert_eq!(node_ok.node_kind, "javascript.script.run");
        assert!(node_ok.data.as_ref().unwrap()["duration_ms"].is_u64());
        assert!(node_ok.message.starts_with("b javascript.script.run "), "{}", node_ok.message);
        let run_done = signals.last().unwrap();
        let data = run_done.data.as_ref().unwrap();
        assert_eq!(data["run_id"], "lifecycle-run-1");
        assert_eq!(data["status"], "ok");
        assert!(data["duration_ms"].is_u64());

        let (result, signals) = signals_of(
            "[a] trigger.manual\n[b] javascript.script.run -- \"throw new Error('boom');\"\n[a] -> [b]\n",
        )
        .await;
        assert!(result.is_err());
        let kinds: Vec<&str> = signals.iter().map(|s| s.kind.as_str()).collect();
        assert_eq!(
            kinds,
            ["run_start", "node_start", "node_ok", "node_start", "node_fail", "run_done"]
        );
        let fail = &signals[4];
        assert_eq!(fail.node_id, "b");
        let data = fail.data.as_ref().unwrap();
        assert!(data["error_code"].as_str().is_some_and(|c| !c.is_empty()));
        assert!(data["error_class"].as_str().is_some_and(|c| c == "refused" || c == "failed"));
        assert!(data["error"].as_str().unwrap().contains("boom"), "{data}");
        assert_eq!(signals.last().unwrap().data.as_ref().unwrap()["status"], "error");
    }

    /// Rule 3 of `kinds/invocation-record` under a preview: a declared
    /// `--preview json` makes `on-error` record a successful node's payload,
    /// and that record goes through the same capture as every other, so a
    /// value the project took out of its credential store is masked in it.
    #[tokio::test]
    async fn a_previewed_payload_never_shows_a_credential_value() {
        use crate::pipeline::model::{PipelineGraphMetadata, PipelineGraphSettings};
        use crate::pipeline::trace_capture::{CaptureLevel, TraceCaptureSettings};

        const SECRET: &str = "sk-live-CREDENTIAL-VALUE-9f2c8d";
        crate::platform::services::credential::register_confidential_for_test(
            "preview-rule3",
            "preview-rule3",
            &json!({ "api_key": SECRET }),
        );
        let mut graph = build_pipeline_graph(
            "preview-rule3",
            "[a] trigger.manual\n[b] javascript.script.run --preview json -- \"return { token: input.manual.secret, note: 'ok' };\"\n[a] -> [b]\n",
        )
        .expect("graph");
        graph.metadata = Some(PipelineGraphMetadata {
            settings: PipelineGraphSettings {
                trace_capture: Some(TraceCaptureSettings {
                    level: Some(CaptureLevel::OnError),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        });
        let ctx = PipelineContext {
            owner: "preview-rule3".into(),
            project: "preview-rule3".into(),
            pipeline: "preview-rule3".into(),
            request_id: "preview-rule3-test".into(),
            route: String::new(),
            input: json!({ "secret": SECRET }),
            trigger: None,
            placeholder: None,
        };
        let out = BasicPipelineEngine::default()
            .execute_async(&graph, &ctx)
            .await
            .expect("execution");
        assert_eq!(out.value["script"]["token"], json!(SECRET), "the run itself is never masked");
        let b = out.node_trace.iter().find(|t| t.node_id == "b").expect("node b");
        assert_eq!(b.output["script"]["note"], json!("ok"), "the preview recorded the payload");
        let text = serde_json::to_string(&out.node_trace).unwrap();
        assert!(!text.contains(SECRET), "the record shows no credential value: {text}");
    }

    /// Reproducible capture-only comparison, without external I/O. This is an
    /// ignored diagnostic, not a timing assertion or a production throughput
    /// promise. The legacy implementation is compiled only into tests.
    #[test]
    #[ignore = "manual capture performance measurement"]
    fn trace_capture_benchmark() {
        use crate::pipeline::trace_capture::{TraceCapture, TraceCaptureSettings};
        use std::hint::black_box;
        use std::time::Instant;
        for (name, rows, repeats, nodes) in [
            ("small", 4, 1000, 1),
            ("large", 10000, 5, 1),
            ("long-chain", 10000, 1, 30),
        ] {
            let payload = json!({"rows": (0..rows).map(|i| json!({"id": i, "name": "example-row", "nested": {"values": [1,2,3,4]}, "body": "x".repeat(256)})).collect::<Vec<_>>()});
            for mode in ["disabled", "legacy", "compact"] {
                let start = Instant::now();
                let mut bytes = 0;
                for _ in 0..repeats {
                    let mut capture = TraceCapture::new(
                        TraceCaptureSettings {
                            array_sample_count: Some(1),
                            ..Default::default()
                        }
                        .resolve(None),
                    );
                    for _ in 0..nodes {
                        if mode == "disabled" {
                            black_box(&payload);
                            continue;
                        }
                        capture.begin_node();
                        let log = if mode == "legacy" {
                            super::legacy_sanitized_trace_value(black_box(&payload))
                        } else {
                            capture.capture(black_box(&payload))
                        };
                        bytes += serde_json::to_vec(black_box(&log)).unwrap().len();
                    }
                }
                eprintln!(
                    "capture-bench {name} {mode}: {:.3} ms/capture, {} bytes/capture ({} captures, debug_assertions={})",
                    start.elapsed().as_secs_f64() * 1000.0 / (repeats * nodes) as f64,
                    bytes / (repeats * nodes),
                    repeats * nodes,
                    cfg!(debug_assertions)
                );
            }
        }
    }

    /// A curated bundle ships in the binary, so a declaration it cannot run
    /// under is a startup-shaped bug rather than a user's problem.
    ///
    /// Checks both halves of the guard against every embedded bundle: no inner
    /// node is refused as uncheckable, and every host its functions name
    /// outright is one the bundle declared. A URL assembled at run time is not
    /// readable here, which is exactly why the check also exists at the moment
    /// of the request.
    ///
    /// The uncheckable half runs for bundles that declare nothing too, because
    /// that is now when it runs in production.
    #[test]
    fn embedded_bundles_can_run_under_their_own_declared_hosts() {
        use crate::contracts::decode_contract;
        use crate::contracts::kinds::{NodeBundleContract, decode_pipeline_graph};
        use crate::pipeline::security::BundleEgress;
        use crate::platform::web::embedded::{
            PLATFORM_COMPOSITE_NODE_ASSETS, platform_composite_node_asset,
        };

        fn literal_host(url: &str) -> Option<String> {
            let rest = url
                .strip_prefix("https://")
                .or_else(|| url.strip_prefix("http://"))?;
            let host = rest.split(['/', '?', '#']).next().unwrap_or_default();
            let host = host.split('@').next_back().unwrap_or_default();
            let host = host.split(':').next().unwrap_or_default();
            (!host.is_empty() && !host.contains("{{")).then(|| host.to_string())
        }

        let mut declaring_bundles = 0;
        for asset in PLATFORM_COMPOSITE_NODE_ASSETS
            .iter()
            .filter(|asset| asset.path.ends_with("/definition.json"))
        {
            let slug = asset.path.trim_end_matches("/definition.json");
            let spec = decode_contract::<NodeBundleContract>(asset.bytes)
                .unwrap_or_else(|error| panic!("embedded bundle '{slug}': {error}"))
                .spec;
            let egress = BundleEgress::extend(None, &spec.package, &spec.hosts);
            if egress.is_active() {
                declaring_bundles += 1;
            }

            for path in spec.functions.values() {
                let bytes = platform_composite_node_asset(&format!("{slug}/{path}"))
                    .unwrap_or_else(|| panic!("embedded bundle '{slug}' is missing '{path}'"));
                let graph = decode_pipeline_graph(bytes)
                    .unwrap_or_else(|error| panic!("embedded bundle '{slug}'/{path}: {error}"))
                    .spec;
                for node in &graph.nodes {
                    // The sandbox denies fetch as shipped, which is the state a
                    // curated bundle must run in; what happens when an operator
                    // widens it is asserted separately.
                    super::refuse_uncheckable_egress_node(&egress, &node.kind, &node.config, false)
                        .unwrap_or_else(|error| {
                            panic!("embedded bundle '{slug}'/{path}: {}", error.message)
                        });
                    let Some(url) = node.config.get("url").and_then(|value| value.as_str()) else {
                        continue;
                    };
                    let Some(host) = literal_host(url) else {
                        continue;
                    };
                    egress
                        .check_host(&host, &node.kind)
                        .unwrap_or_else(|error| {
                            panic!("embedded bundle '{slug}'/{path}: {}", error.message)
                        });
                }
            }
        }
        assert!(
            declaring_bundles > 0,
            "no embedded bundle declares hosts, so this proved nothing"
        );
    }

    /// The incentive this guard exists to remove: if refusing an unreadable
    /// destination only happened inside a bundle that declared hosts, an author
    /// who wanted `postgres.query.run` would be better off declaring nothing, and only
    /// the honest author would be constrained.
    #[test]
    fn a_bundle_that_declares_no_hosts_still_cannot_use_an_uncheckable_node() {
        use crate::pipeline::security::BundleEgress;

        let silent = BundleEgress::extend(None, "silent", &[]);
        assert!(!silent.is_active(), "the bundle declared nothing");

        for (kind, config) in [
            ("ai.text.generate", json!({})),
            ("postgres.query.run", json!({})),
            ("table.query.run", json!({})),
            ("ws.message.send", json!({ "connection": "n0" })),
            ("trigger.socket", json!({})),
        ] {
            let error = super::refuse_uncheckable_egress_node(&silent, kind, &config, false)
                .expect_err("an unreadable destination is refused whatever was declared");
            assert_eq!(error.code, "FW_EGRESS_UNCHECKED_NODE");
            assert!(
                error.message.contains(kind) && error.message.contains("'silent'"),
                "{}",
                error.message
            );
        }

        // What a declaration does buy is still bought: a readable destination is
        // checked against the list rather than refused outright.
        super::refuse_uncheckable_egress_node(&silent, super::http::request::NODE_KIND, &json!({}), false)
            .expect("a host-checked node is not an unreadable destination");
        // A room send never leaves the platform; only `--connection` does.
        super::refuse_uncheckable_egress_node(&silent, "ws.message.send", &json!({ "room": "lobby" }), false)
            .expect("a room send is not an egress path");
    }

    /// `javascript.script.run` is a network node only when an operator has made it one, so
    /// the guard reads the sandbox in force rather than the sandbox as shipped.
    #[test]
    fn a_script_is_refused_inside_a_bundle_only_when_its_sandbox_reaches_the_network() {
        use crate::language::{
            DenoSandboxConfigPatch, DenoSandboxDangerZonePatch, DenoSandboxEngine, LanguageEngine,
        };
        use crate::pipeline::security::BundleEgress;

        let egress = BundleEgress::extend(None, "telegram", &["api.telegram.org".to_string()]);

        super::refuse_uncheckable_egress_node(&egress, super::javascript::NODE_KIND, &json!({}), false)
            .expect("a sandbox that denies fetch reaches nothing to check");

        let error = super::refuse_uncheckable_egress_node(&egress, super::javascript::NODE_KIND, &json!({}), true)
            .expect_err("a sandbox that may fetch is an egress path the guard cannot read");
        assert_eq!(error.code, "FW_EGRESS_UNCHECKED_NODE");
        assert!(
            error.message.contains("javascript.script.run") && error.message.contains("'telegram'"),
            "{}",
            error.message
        );

        // And the boolean is not hypothetical: it is what the engine answers.
        assert!(
            !DenoSandboxEngine::default().grants_network(),
            "the shipped sandbox denies fetch"
        );
        let widened = DenoSandboxEngine::new(
            DenoSandboxConfigPatch {
                danger_zone: Some(DenoSandboxDangerZonePatch {
                    allow_net: Some(true),
                    ..Default::default()
                }),
                ..Default::default()
            },
            DenoSandboxConfigPatch::default(),
        );
        assert!(
            widened.grants_network(),
            "an operator's platform patch is what the guard must read"
        );
    }

    #[test]
    fn private_redact_masks_overlaps_without_applying_log_limits_to_execution() {
        let tail = "retained execution text ".repeat(1024);
        let mut payload = json!({
            "__zf_private_redact": ["", "abc", "bcdef"],
            "__zf_private_redact_except_paths": ["response.body"],
            "message": format!("abcdef {tail}"),
            "nested": ["abcdef"],
            "response": {"body": "abcdef"}
        });
        let tokens = take_private_redact_tokens(&mut payload);
        let exceptions = take_private_redact_except_paths(&mut payload);
        let redacted = redact_json_value(&payload, &tokens, &exceptions, &[]);
        assert_eq!(redacted["message"], format!("•••••• {tail}"));
        assert_eq!(redacted["nested"][0], "••••••");
        assert_eq!(redacted["response"]["body"], "abcdef");
        assert!(redacted["message"].as_str().unwrap().len() > 8192);
    }

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
            "token": "abc123",
            "nested": { "__zf_private_trace_redact": ["deep-secret"] }
        });

        let mut tokens = Vec::new();
        super::sweep_private_markers(&mut payload, &mut tokens);
        assert_eq!(
            tokens,
            vec!["abc123".to_string(), "deep-secret".to_string()]
        );
        assert!(payload.get("__zf_private_trace_redact").is_none());
        assert!(payload["nested"].get("__zf_private_trace_redact").is_none());
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

    /// The fallback exists for the node this tree does not own. `access_token`
    /// was matched and a plainly-named `token` was not, which is the gap a
    /// third-party node would fall into.
    #[test]
    fn trace_config_snapshot_redacts_a_plainly_named_token() {
        let snapshot = trace_config_snapshot(&json!({
            "endpoint": "https://api.example.com/v1",
            "token": "t-abc123",
            "api_token": "t-def456",
            "auth_token": "t-ghi789",
            "session_token": "t-jkl012",
            "bearer_token": "t-mno345",
        }))
        .expect("config snapshot");

        assert_eq!(snapshot["endpoint"], "https://api.example.com/v1");
        for key in [
            "token",
            "api_token",
            "auth_token",
            "session_token",
            "bearer_token",
        ] {
            assert_eq!(snapshot[key], "••••••", "{key} must be redacted");
        }
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
        assert_eq!(summary["values"]["__zf_trace_summary"], "array");
        assert_eq!(summary["values"]["len"], 128);
        assert_eq!(
            summary["nested"]["items"][0]["payload"]["__zf_trace_summary"],
            "array"
        );
        assert_eq!(summary["nested"]["items"][0]["payload"]["len"], 80);
    }

    /// S4 regression, engine half. Every trigger's payload passes through the
    /// trace sanitizer, and only the webhook node ever publishes redaction
    /// tokens -- so a cron, WS, or script-built run carrying a secret had
    /// nothing standing between it and `trace.input` on disk. Remove the
    /// `blank_sensitive_keys` call in `sanitized_trace_value` and the password
    /// comes back in clear.
    #[test]
    fn a_secret_under_a_secret_key_never_reaches_a_trace() {
        let trace = sanitized_trace_value(&json!({
            "body": {
                "username": "wawan",
                "password": "toryoto",
                "profile": { "apiKey": "abc123", "city": "Jakarta" },
                "tokens": ["t-one", "t-two"]
            },
            "method": "POST"
        }));

        assert_eq!(
            trace["body"]["password"],
            "\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}"
        );
        assert_eq!(
            trace["body"]["profile"]["apiKey"],
            "\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}"
        );
        assert_eq!(
            trace["body"]["tokens"][0],
            "\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}"
        );
        assert_eq!(
            trace["body"]["tokens"][1],
            "\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}\u{2022}"
        );

        // What is not a secret is still legible, or the trace stops being useful.
        assert_eq!(trace["body"]["username"], "wawan");
        assert_eq!(
            sanitized_trace_value(&json!({ "authorization": true }))["authorization"],
            true,
            "a flag carries no secret and must stay readable"
        );
        assert_eq!(trace["body"]["profile"]["city"], "Jakarta");
        assert_eq!(trace["method"], "POST");

        let serialized = trace.to_string();
        for secret in ["toryoto", "abc123", "t-one", "t-two"] {
            assert!(
                !serialized.contains(secret),
                "'{secret}' survived: {serialized}"
            );
        }
    }

    /// The redaction markers are an engine-internal channel between nodes, and
    /// the trace one carries the secrets verbatim. A pipeline that returns its
    /// payload must not return them with it.
    #[test]
    fn private_markers_never_ride_out_in_a_pipeline_result() {
        let result = super::strip_private_markers(json!({
            "body": { "username": "wawan" },
            "__zf_private_trace_redact": ["toryoto"],
            "__zf_private_redact": ["abc123"],
            "__zf_private_redact_except_paths": ["response.body"]
        }));
        assert_eq!(result, json!({ "body": { "username": "wawan" } }));

        // `web.response.send` nests the whole upstream payload, so the sweep has
        // to reach down into it.
        let nested = super::strip_private_markers(json!({
            "__zf_response": {
                "status": 200,
                "json": {
                    "username": "wawan",
                    "__zf_private_trace_redact": ["toryoto"]
                }
            }
        }));
        assert!(
            !nested.to_string().contains("__zf_private"),
            "a nested marker survived: {nested}"
        );
        assert_eq!(nested["__zf_response"]["json"]["username"], "wawan");
    }

    /// A marker that has been nested by a downstream node is still read, still
    /// removed, and its contents still redacted -- the shape `web.response.send`
    /// produces, where the whole upstream payload becomes `__zf_response.json`.
    #[test]
    fn a_nested_redaction_marker_is_not_printed_into_the_run_history() {
        let trace = sanitized_trace_value(&json!({
            "__zf_response": {
                "status": 200,
                "json": {
                    "body": { "username": "wawan", "password": "toryoto" },
                    "audit": "login attempt with toryoto",
                    "__zf_private_trace_redact": ["toryoto"]
                }
            }
        }));

        let serialized = trace.to_string();
        assert!(
            !serialized.contains("__zf_private"),
            "an engine marker reached the trace: {serialized}"
        );
        assert!(
            !serialized.contains("toryoto"),
            "the secret reached the trace: {serialized}"
        );
        assert_eq!(trace["__zf_response"]["json"]["body"]["username"], "wawan");
    }

    /// The one escape hatch a pipeline already had keeps working: a field it
    /// deliberately traces is not blanked just because of what it is called.
    #[test]
    fn an_excepted_path_is_still_traced_verbatim() {
        let trace = sanitized_trace_value(&json!({
            "__zf_private_redact_except_paths": ["response.token"],
            "response": { "token": "deliberately-visible" },
            "request": { "token": "hidden" }
        }));

        assert_eq!(trace["response"]["token"], "deliberately-visible");
        assert_ne!(trace["request"]["token"], "hidden");
    }

    /// S4 regression, ingress half. The webhook collector published the tokens
    /// that let the engine strike a secret out of *every* node's trace, not
    /// just the trigger's -- and it read top-level keys only, so on the nested
    /// shape the ingress actually produces it published nothing.
    #[test]
    fn a_webhook_secret_is_struck_out_of_a_later_nodes_trace_too() {
        let ingress = json!({
            "body": { "password": "toryoto" },
            "method": "POST"
        });
        let tokens =
            crate::pipeline::nodes::basic::trigger::webhook::collect_trace_private_tokens_for_test(
                &ingress,
            );
        assert_eq!(tokens, vec!["toryoto".to_string()]);

        // A downstream node copied the secret into a differently-named field;
        // the token list still reaches it.
        let downstream = json!({
            "__zf_private_trace_redact": tokens,
            "audit_line": "login attempt for wawan with toryoto"
        });
        let trace = sanitized_trace_value(&downstream);
        assert!(
            !trace.to_string().contains("toryoto"),
            "a copy of the secret survived: {trace}"
        );
    }

    #[test]
    fn nodes_access_scanner_detects_literal_references() {
        let mut access = NodesAccessAccumulator::default();
        scan_text_nodes_access(
            &PipelineNode {
                id: "reader".to_string(),
                kind: javascript::NODE_KIND.to_string(),
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
                kind: javascript::NODE_KIND.to_string(),
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
[b] javascript.script.run -- "return { big: 'huge-marker-that-should-not-leak' };"
[c] javascript.script.run -- "return { small: 7 };"
[d] javascript.script.run -- "return { small: ctx.nodes.c.script.small, leaked: JSON.stringify(ctx).includes('huge-marker-that-should-not-leak') };"

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

        assert_eq!(out.value["script"]["small"], 7);
        // `b`'s answer was replaced by `c`'s under the same key, and `d`
        // references only `c`: the big marker reaches no node's scope.
        assert_eq!(out.value["script"]["leaked"], false);
    }

    #[tokio::test]
    async fn nodes_scope_rejects_dynamic_script_access() {
        let dsl = r#"
[a] trigger.manual
[b] javascript.script.run -- "return { big: 'dynamic-access-should-not-work' };"
[c] javascript.script.run -- "return { small: 7 };"
[d] javascript.script.run -- "const key = 'b'; return { big: ctx.nodes[key].big, small: ctx.nodes.c.small };"

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

    /// `kinds/pipeline/README.md` Frozen Defaults: an omitted
    /// `spec.entry_nodes` means "Runtime derives roots from graph
    /// connectivity". Two independent roots are two roots — taking the first
    /// node in source order runs half the pipeline and silently drops the
    /// rest.
    #[tokio::test]
    async fn every_root_of_a_multi_root_graph_runs() {
        let dsl = r#"
[a] trigger.manual
[b] javascript.script.run -- "return { left: true };"
[c] trigger.manual
[d] javascript.script.run -- "return { right: true };"

[a] -> [b]
[c] -> [d]
"#;

        // The DSL parser fills `entry_nodes` in; a graph registered as raw
        // contract JSON does not have to, and the frozen default is what is
        // under test.
        let mut graph = build_pipeline_graph("multi-root-test", dsl).expect("graph");
        graph.entry_nodes.clear();
        assert_eq!(graph.entry_node_ids(), vec!["a", "c"]);

        let engine = BasicPipelineEngine::default();
        let out = engine
            .execute_async(
                &graph,
                &PipelineContext {
                    owner: "test".to_string(),
                    project: "test".to_string(),
                    pipeline: "multi-root-test".to_string(),
                    request_id: "req-multi-root".to_string(),
                    route: String::new(),
                    input: json!({}),
                    trigger: None,
                    placeholder: None,
                },
            )
            .await
            .expect("execute");

        let executed: Vec<&str> = out
            .node_trace
            .iter()
            .map(|entry| entry.node_id.as_str())
            .collect();
        for node in ["a", "b", "c", "d"] {
            assert!(
                executed.contains(&node),
                "node '{node}' never ran; executed {executed:?}"
            );
        }
    }

    /// A node runs once, when everything wired into it has answered, so a
    /// cycle would wait for itself: one that is not a `logic.retry`
    /// re-entry is refused at activation, naming it (`node-conventions.md`
    /// §4). The parser still reads it, so the Studio can show it.
    #[test]
    fn a_cycle_that_is_not_a_retry_re_entry_is_refused_at_activation() {
        let dsl = r#"
[a] javascript.script.run -- "return input;"
[b] javascript.script.run -- "return input;"

[a] -> [b]
[b] -> [a]
"#;
        let graph = build_pipeline_graph("cyclic-root-test", dsl).expect("graph");
        let err = BasicPipelineEngine::default().validate_graph(&graph).expect_err("refused");
        assert_eq!(err.code, "FW_PIPELINE_CYCLE");
        assert!(err.message.contains("a → b → a"), "{}", err.message);
    }

    #[tokio::test]
    async fn logic_collect_groups_multiple_upstreams_before_continuing() {
        let dsl = r#"
[a] trigger.manual
[b] javascript.script.run -- "return { user: { id: 'u_42' } };"
[c] javascript.script.run -- "return { orders: [{ id: 'o_1' }] };"
[d] logic.collect
[e] javascript.script.run -- "return input;"

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

        // `collect: { items, count }`, the deliveries in DSL text order (b
        // is declared before c), over those payloads merged.
        assert_eq!(out.value["script"]["collect"]["count"], 2, "{}", out.value);
        assert_eq!(out.value["script"]["collect"]["items"][0]["script"]["user"]["id"], "u_42");
        assert_eq!(out.value["script"]["collect"]["items"][1]["script"]["orders"][0]["id"], "o_1");
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
[b] fs.file.put --path qa/fs/hello.txt --text "hello fs"
[c] fs.file.get --from qa/fs/hello.txt
[d] fs.file.copy --from qa/fs/hello.txt --filename copy.txt
[e] fs.file.move --from qa/fs/copy.txt --filename moved.txt
[f] fs.folder.list --from qa/fs
[g] fs.file.delete --from qa/fs/hello.txt
[h] fs.folder.create --folder qa/fs/prefix

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

        // Each node adds its own key and keeps the rest: the last `file` is
        // the delete's answer, the last `folder` the create's.
        assert_eq!(out.value["folder"], json!({ "path": "qa/fs/prefix", "created": true }));
        assert_eq!(out.value["file"], json!({ "ref": "qa/fs/hello.txt", "deleted": true }));

        let layout = platform
            .file
            .ensure_project_layout(owner, project)
            .expect("project layout");
        let zebfs = layout.open_files();
        let moved = zebfs.get("qa/fs/moved.txt").expect("moved object");
        assert_eq!(String::from_utf8(moved.bytes).expect("utf8"), "hello fs");
        assert!(zebfs.head("qa/fs/prefix").is_ok());
        assert!(zebfs.head("qa/fs/hello.txt").is_err());
        assert!(zebfs.head("qa/fs/copy.txt").is_err());
    }

    #[tokio::test]
    async fn node_output_files_materialize_for_any_node_output() {
        let data_root = tempfile::tempdir().expect("temp data root");
        let owner = "superadmin";
        let project = "node_output_files_e2e";
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
[b] javascript.script.run -- "return { ok: true, state_sequence: '{{file:state-sequence.json}}', __zf_files: [{ name: 'state-sequence.json', content_type: 'application/json', encoding: 'json', data: { frames: [1, 2, 3] } }] };"
[c] javascript.script.run -- "return input;"

[a] -> [b]
[b] -> [c]
"#;

        let graph = build_pipeline_graph("node-output-files-e2e", dsl).expect("graph");
        let engine = BasicPipelineEngine::default().with_platform(platform.clone());
        let out = engine
            .execute_async(
                &graph,
                &PipelineContext {
                    owner: owner.to_string(),
                    project: project.to_string(),
                    pipeline: "node-output-files-e2e".to_string(),
                    request_id: "req-node-output-files".to_string(),
                    route: String::new(),
                    input: json!({}),
                    trigger: None,
                    placeholder: None,
                },
            )
            .await
            .expect("execute");

        let file_ref = &out.value["script"]["script"]["state_sequence"];
        assert_eq!(file_ref["__zf_type"], "file_ref");
        // The eleven contract fields, and nothing that was dropped
        // (`kinds/file-ref/README.md`).
        crate::pipeline::nodes::shared::file_ref::validate_file_ref(file_ref)
            .expect("valid FileRef");
        assert_eq!(file_ref["mime"], "application/json");
        assert_eq!(file_ref["kind"], "json");
        for dropped in ["path", "name", "content_type", "url"] {
            assert!(
                file_ref.get(dropped).is_none(),
                "FileRef must not carry '{dropped}'"
            );
        }
        assert!(out.value.get("__zf_files").is_none());
        assert_eq!(
            out.value["file_refs"]["state-sequence.json"]["ref"],
            file_ref["ref"]
        );

        let rel_path = file_ref["ref"].as_str().expect("file ref path");
        let layout = platform
            .file
            .ensure_project_layout(owner, project)
            .expect("project layout");
        let zebfs = layout.open_files();
        let object = zebfs.get(rel_path).expect("materialized object");
        let stored: serde_json::Value = serde_json::from_slice(&object.bytes).expect("json file");
        assert_eq!(stored, json!({ "frames": [1, 2, 3] }));
    }

    #[tokio::test]
    async fn table_convert_parquet_roundtrips_through_pipeline_and_zebfs() {
        let data_root = tempfile::tempdir().expect("temp data root");
        let owner = "superadmin";
        let project = "table_convert_e2e";
        let platform = Arc::new(
            PlatformService::from_config(PlatformConfig {
                data_root: data_root.path().to_path_buf(),
                default_password: uuid::Uuid::new_v4().to_string(),
                default_project: project.to_string(),
                ..Default::default()
            })
            .expect("platform"),
        );

        let dsl = r#"
[a] trigger.manual
[b] table.data.convert --from "{{ input.manual.rows }}" --path datasets/posts.parquet
[c] table.data.convert --from datasets/posts.parquet

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

        // [c] read the file [b] wrote; without a destination its rows are the answer.
        assert_eq!(out.value["data"]["parse"], "parquet");
        assert_eq!(out.value["data"]["rows"][0]["id"], 1);
        assert_eq!(out.value["data"]["rows"][0]["title"], "First");
        assert_eq!(out.value["data"]["rows"][0]["score"], 1.5);
        assert_eq!(out.value["data"]["rows"][0]["active"], true);
        assert_eq!(out.value["data"]["row_count"], 2);
        assert!(out.value["data"].get("ref").is_none(), "[c] wrote no file");
        assert_eq!(out.value["manual"]["rows"][1]["id"], 2, "the payload is kept");

        let layout = platform
            .file
            .ensure_project_layout(owner, project)
            .expect("project layout");
        let object = layout
            .open_files()
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
                default_password: uuid::Uuid::new_v4().to_string(),
                default_project: project.to_string(),
                ..Default::default()
            })
            .expect("platform"),
        );
        let layout = platform
            .file
            .ensure_project_layout(owner, project)
            .expect("project layout");
        let zebfs = layout.open_files();
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
[b] table.query.run --from "datasets/posts.csv as posts" --from "datasets/authors.csv as authors" --param "1={{ input.manual.post_id }}" --query "select p.id, p.title, a.name from posts p join authors a on p.author_id = a.id where p.id = $1"

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

        assert_eq!(out.value["query"]["row_count"], 1);
        assert_eq!(out.value["query"]["truncated"], false);
        assert_eq!(out.value["query"]["rows"][0]["id"], 1);
        assert_eq!(out.value["query"]["rows"][0]["title"], "First");
        assert_eq!(out.value["query"]["rows"][0]["name"], "Ada");
    }

    #[tokio::test]
    async fn table_query_joins_multiple_parquet_sources() {
        let data_root = tempfile::tempdir().expect("temp data root");
        let owner = "superadmin";
        let project = "table_query_parquet_join";
        let platform = Arc::new(
            PlatformService::from_config(PlatformConfig {
                data_root: data_root.path().to_path_buf(),
                default_password: uuid::Uuid::new_v4().to_string(),
                default_project: project.to_string(),
                ..Default::default()
            })
            .expect("platform"),
        );
        let layout = platform
            .file
            .ensure_project_layout(owner, project)
            .expect("project layout");
        let zebfs = layout.open_files();

        let posts = vec![
            json!({ "id": 1, "author_id": 10, "title": "First", "score": 7.5 }),
            json!({ "id": 2, "author_id": 20, "title": "Second", "score": 3.0 }),
            json!({ "id": 3, "author_id": 30, "title": "Third", "score": 9.0 }),
        ];
        let posts_bytes = encode_rows(&posts, &collect_columns(&posts), TableFormat::Parquet, crate::pipeline::nodes::basic::table::convert::CODE)
            .expect("posts parquet");
        zebfs
            .put("datasets/posts.parquet", &posts_bytes)
            .expect("posts parquet put");

        let authors = vec![
            json!({ "id": 10, "name": "Ada", "active": true }),
            json!({ "id": 20, "name": "Bob", "active": false }),
            json!({ "id": 30, "name": "Cora", "active": true }),
        ];
        let authors_bytes = encode_rows(&authors, &collect_columns(&authors), TableFormat::Parquet, crate::pipeline::nodes::basic::table::convert::CODE)
            .expect("authors parquet");
        zebfs
            .put("datasets/authors.parquet", &authors_bytes)
            .expect("authors parquet put");

        let dsl = r#"
[a] trigger.manual
[b] table.query.run --from "datasets/posts.parquet as posts" --from "datasets/authors.parquet as authors" --query "select p.id, p.title, a.name from posts p join authors a on p.author_id = a.id where a.active = true order by p.id"

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

        assert_eq!(out.value["query"]["row_count"], 2);
        assert_eq!(out.value["query"]["columns"], json!(["id", "title", "name"]));
        assert_eq!(out.value["query"]["rows"][0]["id"], 1);
        assert_eq!(out.value["query"]["rows"][0]["title"], "First");
        assert_eq!(out.value["query"]["rows"][0]["name"], "Ada");
        assert_eq!(out.value["query"]["rows"][1]["id"], 3);
        assert_eq!(out.value["query"]["rows"][1]["title"], "Third");
        assert_eq!(out.value["query"]["rows"][1]["name"], "Cora");
    }

    #[tokio::test]
    async fn table_query_accepts_ui_source_binding_rows() {
        let data_root = tempfile::tempdir().expect("temp data root");
        let owner = "superadmin";
        let project = "table_query_ui_rows";
        let platform = Arc::new(
            PlatformService::from_config(PlatformConfig {
                data_root: data_root.path().to_path_buf(),
                default_password: uuid::Uuid::new_v4().to_string(),
                default_project: project.to_string(),
                ..Default::default()
            })
            .expect("platform"),
        );
        let layout = platform
            .file
            .ensure_project_layout(owner, project)
            .expect("project layout");
        layout
            .open_files()
            .put("datasets/posts.csv", b"id,title\n1,First\n2,Second\n")
            .expect("posts csv");

        let graph = PipelineGraph {
            id: "table-query-ui-rows".to_string(),
            description: None,
            metadata: None,
            notes: Vec::new(),
            entry_nodes: Vec::new(),
            nodes: vec![
                PipelineNode {
                    id: "a".to_string(),
                    kind: "trigger.manual".to_string(),
                    input_pins: vec![],
                    output_pins: vec!["out".to_string()],
                    config: json!({}),
                },
                PipelineNode {
                    id: "b".to_string(),
                    kind: "table.query.run".to_string(),
                    input_pins: vec!["in".to_string()],
                    output_pins: vec!["out".to_string()],
                    config: json!({
                        "from": [
                            { "source": "datasets/posts.csv", "alias": "posts" }
                        ],
                        "query": "select * from posts where id = $1",
                        "param": { "1": "{{ input.manual.post_id }}" }
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

        assert_eq!(out.value["query"]["row_count"], 1);
        assert_eq!(out.value["query"]["rows"][0]["title"], "Second");
    }

    #[tokio::test]
    async fn table_query_geodatafusion_runs_geospatial_sql() {
        let data_root = tempfile::tempdir().expect("temp data root");
        let owner = "superadmin";
        let project = "table_query_geodatafusion";
        let platform = Arc::new(
            PlatformService::from_config(PlatformConfig {
                data_root: data_root.path().to_path_buf(),
                default_password: uuid::Uuid::new_v4().to_string(),
                default_project: project.to_string(),
                ..Default::default()
            })
            .expect("platform"),
        );

        let dsl = r#"
[a] trigger.manual
[b] table.query.run --from "$input.manual.rows as points" --query "select id, ST_AsText(ST_Point(x, y)) as geom from points where id = 1"

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

        assert_eq!(out.value["query"]["row_count"], 1);
        assert_eq!(out.value["query"]["rows"][0]["id"], 1);
        assert_eq!(out.value["query"]["rows"][0]["geom"], "POINT(30 10)");
    }

    #[tokio::test]
    async fn table_query_writes_the_whole_result_and_answers_its_file() {
        let data_root = tempfile::tempdir().expect("temp data root");
        let owner = "superadmin";
        let project = "table_query_file";
        let platform = Arc::new(
            PlatformService::from_config(PlatformConfig {
                data_root: data_root.path().to_path_buf(),
                default_password: uuid::Uuid::new_v4().to_string(),
                default_project: project.to_string(),
                ..Default::default()
            })
            .expect("platform"),
        );
        let layout = platform.file.ensure_project_layout(owner, project).expect("project layout");
        layout
            .open_files()
            .put("datasets/posts.csv", b"id,title\n1,First\n2,Second\n3,Third\n")
            .expect("posts csv");

        let run = |dsl: &'static str| {
            let platform = platform.clone();
            async move {
                let graph = build_pipeline_graph("table-query-file", dsl).expect("graph");
                BasicPipelineEngine::default()
                    .with_platform(platform)
                    .execute_async(
                        &graph,
                        &PipelineContext {
                            owner: owner.to_string(),
                            project: project.to_string(),
                            pipeline: "table-query-file".to_string(),
                            request_id: "req-table-query-file".to_string(),
                            route: String::new(),
                            input: json!({}),
                            trigger: None,
                            placeholder: None,
                        },
                    )
                    .await
                    .expect("execute")
            }
        };

        let out = run("[a] trigger.manual\n[b] table.query.run --from \"datasets/posts.csv as posts\" --path exports/posts.ndjson --rows --limit 1 -- \"SELECT id, title FROM posts ORDER BY id\"\n[a] -> [b]\n").await;
        let query = &out.value["query"];
        assert_eq!(query["__zf_type"], "file_ref");
        assert_eq!(query["ref"], "exports/posts.ndjson");
        assert_eq!(query["format"], "ndjson");
        assert_eq!(query["row_count"], 3, "the file holds the whole result");
        assert_eq!(query["rows"].as_array().map(Vec::len), Some(1), "--rows answers the first --limit rows");
        assert_eq!(query["truncated"], true);
        let written = layout.open_files().get("exports/posts.ndjson").expect("written");
        assert_eq!(String::from_utf8_lossy(&written.bytes).lines().count(), 3);

        let only_file = run("[a] trigger.manual\n[b] table.query.run --from \"datasets/posts.csv as posts\" --folder exports --filename all.csv -- \"SELECT * FROM posts\"\n[a] -> [b]\n").await;
        assert_eq!(only_file.value["query"]["ref"], "exports/all.csv");
        assert!(only_file.value["query"].get("rows").is_none(), "no --rows: the file is the answer");

        let truncated = run("[a] trigger.manual\n[b] table.query.run --from \"datasets/posts.csv as posts\" --limit 2 -- \"SELECT * FROM posts\"\n[a] -> [b]\n").await;
        assert_eq!(truncated.value["query"]["row_count"], 2);
        assert_eq!(truncated.value["query"]["truncated"], true);
    }

    #[tokio::test]
    async fn logic_if_supports_dsl_input_scope() {
        let dsl = r#"
[a] trigger.manual
[b] javascript.script.run -- "return { type: 'billing' };"
[c] logic.if --when "$input.script.type == 'billing'"
[d] javascript.script.run -- "return { branch: 'true' };"
[e] javascript.script.run -- "return { branch: 'false' };"

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

        assert_eq!(out.value["script"]["branch"], "true");
    }

    #[tokio::test]
    async fn logic_match_supports_dsl_nodes_scope() {
        let dsl = r#"
[a] trigger.manual
[b] javascript.script.run -- "return { kind: 'billing' };"
[c] logic.match --from "$nodes.b.script.kind" --case billing --case technical --default default
[d] javascript.script.run -- "return { lane: 'billing' };"
[e] javascript.script.run -- "return { lane: 'technical' };"
[f] javascript.script.run -- "return { lane: 'default' };"

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

        assert_eq!(out.value["script"]["lane"], "billing");
    }

    /// An input node reads the envelope (`$trigger`) and answers the value
    /// it checked at its `--name`, the rest of the payload kept; a required
    /// field that was not sent refuses the run naming the field.
    #[tokio::test]
    async fn input_nodes_answer_the_checked_value_at_their_name() {
        let dsl = r#"
[a] trigger.manual
[b] input.text prompt --label "Caption"
[c] input.number count --default 3
[d] javascript.script.run -- "return { echoed: input.manual.body.prompt, prompt: $nodes.b.prompt, count: input.count, keys: Object.keys(input).sort() };"

[a] -> [b]
[b] -> [c]
[c] -> [d]
"#;
        let graph = build_pipeline_graph("input-nodes-test", dsl).expect("graph");
        let engine = BasicPipelineEngine::default();
        let ctx = |input: serde_json::Value| PipelineContext {
            owner: "test".to_string(),
            project: "test".to_string(),
            pipeline: "input-nodes-test".to_string(),
            request_id: "req-input".to_string(),
            route: String::new(),
            input,
            trigger: None,
            placeholder: None,
        };
        let out = engine
            .execute_async(&graph, &ctx(json!({ "body": { "prompt": "hello", "extra": true } })))
            .await
            .expect("execute");
        assert_eq!(out.value["script"]["echoed"], "hello");
        assert_eq!(out.value["script"]["prompt"], "hello");
        assert_eq!(out.value["script"]["count"], 3.0);
        assert_eq!(out.value["script"]["keys"], json!(["count", "manual", "prompt"]));

        let err = engine
            .execute_async(&graph, &ctx(json!({ "body": {} })))
            .await
            .expect_err("a required input that was not sent refuses the run");
        assert_eq!(err.code, "FW_NODE_INPUT_MISSING");
        assert!(err.message.contains("'prompt'"), "{}", err.message);
        assert_eq!(err.node_id.as_deref(), Some("b"));
    }

    #[tokio::test]
    async fn logic_foreach_emits_one_run_per_item() {
        let dsl = r#"
[a] trigger.manual
[b] logic.foreach --from "$input.manual.rows"

[a] -> [b]
"#;

        let mut graph = build_pipeline_graph("logic-foreach-test", dsl).expect("graph");
        // Asserts on the foreach node's recorded output, so it needs the level
        // that records a successful node. Default is on-error.
        graph.metadata = Some(crate::pipeline::model::PipelineGraphMetadata {
            settings: crate::pipeline::model::PipelineGraphSettings {
                trace_capture: Some(crate::pipeline::trace_capture::TraceCaptureSettings {
                    level: Some(crate::pipeline::trace_capture::CaptureLevel::Full),
                    ..Default::default()
                }),
                ..Default::default()
            },
            ..Default::default()
        });
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
            .find(|entry| entry.node_kind == "logic.foreach")
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
[b] logic.foreach --from "$input.manual.rows" --keep-input

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
        assert_eq!(out.value["manual"]["batch_marker"], "kept-only-when-requested");
        assert_eq!(out.value["manual"]["rows"][0]["id"], "r1");
    }

    #[tokio::test]
    async fn item_still_names_the_loop_element_after_a_node_replaced_the_payload() {
        // [c] answers over the payload; [d]'s {{ $item }} must still be the
        // element this run started from, not null and not another run's.
        let dsl = r#"
[a] trigger.manual
[b] logic.foreach --from "$input.manual.rows"
[c] javascript.script.run -- "return { unrelated: true };"
[d] crypto.base64.encode --text "{{ $item.key }}"
[e] logic.reduce --initial "{ keys: [] }" --step "{ keys: $acc.keys.concat([$input.base64.value]) }"

[a] -> [b]
[b]:item -> [c]
[c] -> [d]
[d] -> [e]
"#;
        let graph = build_pipeline_graph("foreach-item-scope-test", dsl).expect("graph");
        let out = BasicPipelineEngine::default()
            .execute_async(
                &graph,
                &PipelineContext {
                    owner: "test".to_string(),
                    project: "test".to_string(),
                    pipeline: "foreach-item-scope-test".to_string(),
                    request_id: "req-item-scope".to_string(),
                    route: String::new(),
                    input: json!({ "rows": [{ "key": "a" }, { "key": "b" }, { "key": "c" }] }),
                    trigger: None,
                    placeholder: None,
                },
            )
            .await
            .expect("execute");

        // base64 of "a", "b", "c".
        assert_eq!(out.value["reduce"]["keys"], json!(["YQ==", "Yg==", "Yw=="]), "{}", out.value);
    }

    #[tokio::test]
    async fn logic_reduce_waits_for_the_whole_series_when_a_node_sits_between() {
        // A node sits between the loop and the reduce; the reduce must still
        // fold all three runs, not fire after each one.
        let dsl = r#"
[a] trigger.manual
[b] logic.foreach --from "$input.manual.rows"
[c] javascript.script.run -- "return { v: input.item.amount * 10 };"
[d] logic.reduce --initial "{ vs: [] }" --step "{ vs: $acc.vs.concat([$input.script.v]) }"

[a] -> [b]
[b]:item -> [c]
[c] -> [d]
"#;
        let graph = build_pipeline_graph("logic-reduce-between-test", dsl).expect("graph");
        let out = BasicPipelineEngine::default()
            .execute_async(
                &graph,
                &PipelineContext {
                    owner: "test".to_string(),
                    project: "test".to_string(),
                    pipeline: "logic-reduce-between-test".to_string(),
                    request_id: "req-reduce-between".to_string(),
                    route: String::new(),
                    input: json!({ "rows": [{ "amount": 1 }, { "amount": 2 }, { "amount": 3 }] }),
                    trigger: None,
                    placeholder: None,
                },
            )
            .await
            .expect("execute");

        assert_eq!(out.value["reduce"]["vs"], json!([10, 20, 30]));
    }

    #[tokio::test]
    async fn logic_reduce_accumulates_foreach_series() {
        let dsl = r#"
[a] trigger.manual
[b] logic.foreach --from "$input.manual.rows"
[c] logic.reduce --initial "{ total: 0 }" --step "{ total: $acc.total + $input.item.amount }"

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

        assert_eq!(out.value["reduce"]["total"], 32);
    }


    /// The one resolution mechanism must reach the response node like any
    /// other: a whole {{ }} in --body or --header is the typed value.
    #[tokio::test]
    async fn a_web_response_config_is_resolved_like_any_other_node() {
        let dsl = r#"
[a] trigger.manual
[b] javascript.script.run -- "return { u: 'hello-from-expr' };"
[c] web.response.send --body "{{ input.script.u }}"

[a] -> [b]
[b] -> [c]
"#;
        let graph = build_pipeline_graph("web-response-expr-test", dsl).expect("graph");
        let engine = BasicPipelineEngine::default();
        let out = engine
            .execute_async(
                &graph,
                &PipelineContext {
                    owner: "test".to_string(),
                    project: "test".to_string(),
                    pipeline: "web-response-expr-test".to_string(),
                    request_id: "req-wr".to_string(),
                    route: String::new(),
                    input: json!({}),
                    trigger: None,
                    placeholder: None,
                },
            )
            .await
            .expect("execute");
        let response = out.response.expect("the response node answered");
        assert_eq!(
            response["text"], "hello-from-expr",
            "resolved body did not reach the envelope: {response}"
        );
        // It answers the caller and passes its payload on unchanged.
        assert_eq!(out.value["script"]["u"], "hello-from-expr", "{}", out.value);
        assert!(out.value.get("__zf_response").is_none(), "{}", out.value);
    }

    /// Two `Set-Cookie` headers are two cookies: the repeat survives the
    /// DSL, the stored config and the envelope the HTTP layer sends, and each
    /// value is sent as written — the bare `theme=dark` gains nothing.
    #[tokio::test]
    async fn a_repeated_header_reaches_the_answer_twice() {
        let dsl = r#"
[a] trigger.manual
[b] javascript.script.run -- "return { sid: 'demo-session' };"
[c] web.response.send --status 303 --header "Location=/home" --header "Set-Cookie=session={{ input.script.sid }}; Path=/; Max-Age=86400; SameSite=Lax; HttpOnly" --header "Set-Cookie=theme=dark"

[a] -> [b]
[b] -> [c]
"#;
        let graph = build_pipeline_graph("web-response-cookies", dsl).expect("graph");
        let node = graph.nodes.iter().find(|n| n.id == "c").expect("node c");
        assert_eq!(
            node.config["headers"]["Set-Cookie"],
            json!(["session={{ input.script.sid }}; Path=/; Max-Age=86400; SameSite=Lax; HttpOnly", "theme=dark"]),
            "the stored config keeps both"
        );
        let out = BasicPipelineEngine::default()
            .execute_async(
                &graph,
                &PipelineContext {
                    owner: "test".to_string(),
                    project: "test".to_string(),
                    pipeline: "web-response-cookies".to_string(),
                    request_id: "req-wc".to_string(),
                    route: String::new(),
                    input: json!({}),
                    trigger: None,
                    placeholder: None,
                },
            )
            .await
            .expect("execute");
        let response = out.response.expect("answered");
        assert_eq!(response["status"], 303);
        assert_eq!(
            response["headers"],
            json!([
                ["Location", "/home"],
                ["Set-Cookie", "session=demo-session; Path=/; Max-Age=86400; SameSite=Lax; HttpOnly"],
                ["Set-Cookie", "theme=dark"],
            ])
        );
    }

    #[tokio::test]
    async fn logic_retry_retries_until_success() {
        let dsl = r#"
[a] trigger.manual
[b] javascript.script.run -- "const attempt = input.__zf_retry?.attempt ?? 0; if (attempt < 2) { throw new Error('retry me'); } return { ok: true, attempt };"
[r] logic.retry --max-attempts 3
[c] javascript.script.run -- "return input;"
[d] javascript.script.run -- "return input;"

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

        // `c` answers its input (which holds `b`'s answer) as its own result.
        assert_eq!(out.value["script"]["script"]["ok"], true, "{}", out.value);
        assert_eq!(out.value["script"]["script"]["attempt"], 2);
    }

    /// `$trigger` is the envelope the trigger answers under its source key:
    /// the ingress snapshot when one is set, else the run's input — in a
    /// flag's `{{ }}` and in a script alike, after a node replaced the payload.
    #[tokio::test]
    async fn trigger_still_resolves_to_the_envelope() {
        let dsl = r#"
[t] trigger.webhook --route /hello --method POST
[a] javascript.script.run -- "return { replaced: true };"
[b] logic.if --when "$trigger.body.name === 'Ana'"
[c] javascript.script.run -- "return { name: $trigger.body.name, sub: $trigger.auth ? $trigger.auth.sub : null, kept: input.script.replaced, answer: ctx.nodes.t.webhook.body.name };"
[t] -> [a]
[a] -> [b]
[b]:true -> [c]
"#;
        let graph = build_pipeline_graph("trigger-scope-test", dsl).expect("graph");
        let envelope = json!({ "body": { "name": "Ana" }, "query": {}, "params": {}, "path": "/hello", "method": "POST", "auth": { "sub": "u_1" } });
        let ctx = |trigger: Option<serde_json::Value>| PipelineContext {
            owner: "test".to_string(),
            project: "test".to_string(),
            pipeline: "trigger-scope-test".to_string(),
            request_id: "req-trigger".to_string(),
            route: "/hello".to_string(),
            input: envelope.clone(),
            trigger,
            placeholder: None,
        };
        for trigger in [Some(envelope.clone()), None] {
            let out = BasicPipelineEngine::default()
                .execute_async(&graph, &ctx(trigger.clone()))
                .await
                .expect("execute");
            assert_eq!(
                out.value["script"],
                json!({ "name": "Ana", "sub": "u_1", "kept": true, "answer": "Ana" }),
                "snapshot set: {}",
                trigger.is_some()
            );
        }
    }

    /// `input.* --default` answers on every path: an execute that omits the
    /// field finds the default at `input.<name>` downstream, the same as
    /// `$nodes.<id>.<name>`; a sent value wins.
    #[tokio::test]
    async fn an_input_default_answers_when_the_field_is_omitted() {
        let dsl = r#"
[t] trigger.manual
[who] input.text who --optional --default world
[s] javascript.script.run -- "return { hi: input.who, own: ctx.nodes.who.who, sent: input.manual.body ? input.manual.body.who : null };"
[t] -> [who]
[who] -> [s]
"#;
        let graph = build_pipeline_graph("input-default-test", dsl).expect("graph");
        let engine = BasicPipelineEngine::default();
        let run = |input: serde_json::Value| {
            let graph = &graph;
            let engine = &engine;
            async move {
                engine
                    .execute_async(
                        graph,
                        &PipelineContext {
                            owner: "test".to_string(),
                            project: "test".to_string(),
                            pipeline: "input-default-test".to_string(),
                            request_id: "req-default".to_string(),
                            route: String::new(),
                            input,
                            trigger: None,
                            placeholder: None,
                        },
                    )
                    .await
                    .expect("execute")
            }
        };
        let omitted = run(json!({})).await;
        assert_eq!(omitted.value["script"]["hi"], "world");
        assert_eq!(omitted.value["script"]["own"], "world");
        let empty_body = run(json!({ "body": {} })).await;
        assert_eq!(empty_body.value["script"]["hi"], "world");
        assert_eq!(empty_body.value["script"]["sent"], serde_json::Value::Null, "the envelope is not written");
        let sent = run(json!({ "body": { "who": "x" } })).await;
        assert_eq!(sent.value["script"]["hi"], "x");
        assert_eq!(sent.value["script"]["own"], "x");
    }

    #[tokio::test]
    async fn logic_retry_routes_to_failed_after_budget() {
        let dsl = r#"
[a] trigger.manual
[b] javascript.script.run -- "throw new Error('always fail');"
[r] logic.retry --max-attempts 2
[c] javascript.script.run -- "return input;"

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

    async fn run_with_bus(
        id: &str,
        dsl: &str,
    ) -> (
        Result<crate::pipeline::model::PipelineOutput, crate::pipeline::model::PipelineError>,
        Vec<crate::pipeline::model::Signal>,
    ) {
        use crate::pipeline::model::{ExecuteOptions, ExecutionBus};
        let graph = build_pipeline_graph(id, dsl).expect("graph");
        let ctx = PipelineContext {
            owner: "test".into(),
            project: "test".into(),
            pipeline: id.into(),
            request_id: format!("{id}-run"),
            route: String::new(),
            input: json!({}),
            trigger: None,
            placeholder: None,
        };
        let bus = Arc::new(ExecutionBus::new(128));
        let mut rx = bus.subscribe();
        let options = ExecuteOptions { bus: Some(bus), ..Default::default() };
        let result = BasicPipelineEngine::default()
            .execute_with_options_async(&graph, &ctx, &options)
            .await;
        let mut signals = Vec::new();
        while let Ok(signal) = rx.try_recv() {
            signals.push(signal);
        }
        (result, signals)
    }

    /// A failure an `:error` edge hands to `logic.retry` is a retry, not a
    /// failure: the record says `retry`, the bus says `node_retry` with the
    /// attempt counted, `node_fail` is never announced, and the run is `ok`.
    /// The error text stays in the entry so the log still says why.
    #[tokio::test]
    async fn a_routed_failure_is_a_retry_in_the_record_and_on_the_bus() {
        let dsl = r#"
[a] trigger.manual
[b] javascript.script.run -- "const attempt = input.__zf_retry?.attempt ?? 0; if (attempt < 2) { throw new Error('not yet'); } return { ok: true, attempt };"
[r] logic.retry --max-attempts 5
[c] javascript.script.run -- "return input;"
[d] javascript.script.run -- "return { gaveup: true };"
[a] -> [b]
[b]:error -> [r]
[r]:retry -> [b]
[b] -> [c]
[r]:failed -> [d]
"#;
        let (result, signals) = run_with_bus("routed-retry", dsl).await;
        let out = result.expect("the run succeeds");
        assert_eq!(out.value["script"]["script"]["ok"], true, "{}", out.value);

        let b_entries: Vec<&crate::pipeline::model::NodeTraceEntry> =
            out.node_trace.iter().filter(|t| t.node_id == "b").collect();
        let statuses: Vec<&str> = b_entries.iter().map(|t| t.status.as_str()).collect();
        assert_eq!(statuses, ["retry", "retry", "ok"]);
        assert!(b_entries[0].error.as_deref().is_some_and(|e| e.contains("not yet")));
        assert!(b_entries[0].error_code.is_some());
        assert!(b_entries[2].error.is_none());

        let retries: Vec<&crate::pipeline::model::Signal> =
            signals.iter().filter(|s| s.kind == "node_retry").collect();
        assert_eq!(retries.len(), 2, "{:?}", signals.iter().map(|s| &s.kind).collect::<Vec<_>>());
        for (i, signal) in retries.iter().enumerate() {
            assert_eq!(signal.node_id, "b");
            let data = signal.data.as_ref().unwrap();
            assert_eq!(data["attempt"], (i + 1) as u64, "{data}");
            assert_eq!(data["max_attempts"], 5);
            assert!(data["duration_ms"].is_u64());
            assert!(data["error_code"].as_str().is_some_and(|c| !c.is_empty()));
            assert!(data["message"].as_str().unwrap().contains("not yet"));
        }
        assert!(retries[0].message.contains("retry 1/5"), "{}", retries[0].message);
        assert!(signals.iter().all(|s| s.kind != "node_fail"), "a routed failure is never node_fail");
        assert_eq!(signals.last().unwrap().kind, "run_done");
        assert_eq!(signals.last().unwrap().data.as_ref().unwrap()["status"], "ok");

        // The record's key for an error group skips routed entries: this run
        // has no failing entry at all, so it could never name a group.
        let entry = crate::platform::model::PipelineInvocationEntry {
            run_id: "routed-retry-run".into(),
            at: 0,
            duration_ms: 1,
            status: "ok".into(),
            trigger: "manual".into(),
            error: None,
            trace: out.node_trace.clone(),
        };
        assert!(crate::platform::model::failing_trace_entry(&entry.trace).is_none());
    }

    /// A failure routed to something other than `logic.retry` is
    /// `error_routed`, naming where it went; an unrouted one is still
    /// `node_fail` and fails the run.
    #[tokio::test]
    async fn a_failure_routed_elsewhere_is_error_routed_and_an_unrouted_one_still_fails() {
        let dsl = r#"
[a] trigger.manual
[b] javascript.script.run -- "throw new Error('handled here');"
[h] javascript.script.run -- "return { handled: input.error.message };"
[a] -> [b]
[b]:error -> [h]
"#;
        let (result, signals) = run_with_bus("routed-elsewhere", dsl).await;
        let out = result.expect("the run succeeds");
        assert!(out.value["script"]["handled"].as_str().unwrap().contains("handled here"), "{}", out.value);
        let b = out.node_trace.iter().find(|t| t.node_id == "b").unwrap();
        assert_eq!(b.status, "error_routed");
        let routed = signals.iter().find(|s| s.kind == "node_error_routed").expect("node_error_routed");
        assert_eq!(routed.node_id, "b");
        assert_eq!(routed.data.as_ref().unwrap()["to_node"], "h");
        assert!(signals.iter().all(|s| s.kind != "node_fail"));

        let (result, signals) = run_with_bus(
            "unrouted",
            "[a] trigger.manual\n[b] javascript.script.run -- \"throw new Error('boom');\"\n[a] -> [b]\n",
        )
        .await;
        let err = result.expect_err("the run fails");
        assert_eq!(err.node_trace.last().unwrap().status, "failed");
        assert!(signals.iter().any(|s| s.kind == "node_fail"));
        assert!(
            crate::platform::model::failing_trace_entry(&err.node_trace)
                .is_some_and(|t| t.node_id == "b")
        );
    }

    /// The verdict road: a check that never throws answers `retry: true`
    /// while waiting; `logic.retry` counts its own attempts from `$nodes`
    /// even though `poll` replaces the payload every round, then `done`
    /// carries the payload on. No failure anywhere, so every badge is green.
    #[tokio::test]
    async fn logic_retry_takes_a_verdict_and_counts_for_itself() {
        let dsl = r#"
[a] trigger.manual
[poll] javascript.script.run -- "return { polled: true };"
[check] javascript.script.run -- "const seen = (ctx.nodes.wait && ctx.nodes.wait.__zf_retry && ctx.nodes.wait.__zf_retry.attempt) || 0; return { retry: seen < 2, seen };"
[wait] logic.retry --max-attempts 5 --when "input.script.retry"
[done] javascript.script.run -- "return { done: true, attempts: input.__zf_retry.attempt, seen: input.script.seen };"
[gaveup] javascript.script.run -- "return { gaveup: true };"
[a] -> [poll]
[poll] -> [check]
[check] -> [wait]
[wait]:retry -> [poll]
[wait]:done -> [done]
[wait]:failed -> [gaveup]
"#;
        let (result, signals) = run_with_bus("verdict-done", dsl).await;
        let out = result.expect("the run succeeds");
        assert_eq!(out.value["script"]["done"], true);
        assert_eq!(out.value["script"]["attempts"], 2, "{}", out.value);
        assert_eq!(out.value["script"]["seen"], 2);
        assert_eq!(out.node_trace.iter().filter(|t| t.node_id == "poll").count(), 3);
        // The wait is told on the retry node — `retry` twice, then `ok` when
        // `done` fires — and nowhere else: every other entry is `ok`, but
        // `gaveup`, on the pin not taken, which is recorded `skipped`
        // (`node-conventions.md` §4).
        let wait_statuses: Vec<&str> = out
            .node_trace
            .iter()
            .filter(|t| t.node_id == "wait")
            .map(|t| t.status.as_str())
            .collect();
        assert_eq!(wait_statuses, ["retry", "retry", "ok"]);
        assert!(
            out.node_trace
                .iter()
                .filter(|t| t.node_id != "wait")
                .all(|t| t.status == if t.node_id == "gaveup" { "skipped" } else { "ok" }),
            "{:?}",
            out.node_trace.iter().map(|t| (&t.node_id, &t.status)).collect::<Vec<_>>()
        );
        assert!(signals.iter().all(|s| s.kind != "node_fail"));
        let waits: Vec<&crate::pipeline::model::Signal> =
            signals.iter().filter(|s| s.kind == "node_retry").collect();
        assert_eq!(waits.len(), 2);
        assert!(waits.iter().all(|s| s.node_id == "wait"));
        assert_eq!(waits[0].data.as_ref().unwrap()["attempt"], 1);
        assert_eq!(waits[1].data.as_ref().unwrap()["attempt"], 2);
        assert_eq!(waits[1].data.as_ref().unwrap()["max_attempts"], 5);

        // The budget spent on a verdict that never turns false is `failed`.
        let dsl = r#"
[a] trigger.manual
[poll] javascript.script.run -- "return { polled: true };"
[check] javascript.script.run -- "return { retry: true };"
[wait] logic.retry --max-attempts 2 --when "input.script.retry"
[done] javascript.script.run -- "return { done: true };"
[gaveup] javascript.script.run -- "return { gaveup: true, attempts: input.__zf_retry.attempt };"
[a] -> [poll]
[poll] -> [check]
[check] -> [wait]
[wait]:retry -> [poll]
[wait]:done -> [done]
[wait]:failed -> [gaveup]
"#;
        let (result, _) = run_with_bus("verdict-failed", dsl).await;
        let out = result.expect("the run succeeds through the failed pin");
        assert_eq!(out.value["script"]["gaveup"], true);
        assert_eq!(out.value["script"]["attempts"], 2);
    }

    // ── node-conventions.md §4, Flow ─────────────────────────────────────

    fn flow_ctx(id: &str, input: serde_json::Value) -> PipelineContext {
        PipelineContext {
            owner: "test".into(),
            project: "test".into(),
            pipeline: id.into(),
            request_id: format!("{id}-run"),
            route: String::new(),
            input,
            trigger: None,
            placeholder: None,
        }
    }

    async fn run_flow(
        id: &str,
        dsl: &str,
        input: serde_json::Value,
    ) -> Result<crate::pipeline::model::PipelineOutput, crate::pipeline::model::PipelineError> {
        let graph = build_pipeline_graph(id, dsl).expect("graph");
        BasicPipelineEngine::default().execute_async(&graph, &flow_ctx(id, input)).await
    }

    fn statuses_of(trace: &[crate::pipeline::model::NodeTraceEntry], node: &str) -> Vec<String> {
        trace.iter().filter(|t| t.node_id == node).map(|t| t.status.clone()).collect()
    }

    /// Two branches meeting is a join: the node runs once, its input the
    /// delivered payloads merged in DSL text order — `right` is declared
    /// after `left`, so its key wins although it answered first in time.
    #[tokio::test]
    async fn a_join_runs_once_with_its_inputs_merged_in_text_order() {
        let dsl = r#"
[a] trigger.manual
[left] javascript.script.run -- "return { side: 'left', left: true };"
[right] javascript.script.run -- "return { side: 'right', right: true };"
[join] javascript.script.run -- "return { side: input.script.side, keys: Object.keys(input).sort() };"
[a] -> [right]
[a] -> [left]
[left] -> [join]
[right] -> [join]
"#;
        let out = run_flow("join-once", dsl, json!({})).await.expect("run");
        assert_eq!(statuses_of(&out.node_trace, "join"), ["ok"], "the join runs once");
        let order: Vec<&str> = out.node_trace.iter().map(|t| t.node_id.as_str()).collect();
        assert_eq!(order, ["a", "right", "left", "join"], "right answered first in time");
        assert_eq!(out.value["script"]["side"], "right", "a later line wins: {}", out.value);
        assert_eq!(out.value["script"]["keys"], json!(["manual", "script"]));
    }

    /// A branch not taken is skipped, and so is everything only it feeds;
    /// a join with one live edge still runs. Skipped nodes are recorded
    /// `skipped` and announced `node_skipped`.
    #[tokio::test]
    async fn a_skip_propagates_down_a_chain_and_a_join_with_one_live_edge_runs() {
        let dsl = r#"
[a] trigger.manual
[gate] logic.if --when "false"
[b] javascript.script.run -- "return { b: 1 };"
[c] javascript.script.run -- "return { c: 1 };"
[d] javascript.script.run -- "return { d: 1, from_c: input.script ? input.script.c : null };"
[e] javascript.script.run -- "return { e: 1 };"
[a] -> [gate]
[gate]:true -> [b]
[b] -> [c]
[c] -> [d]
[gate]:false -> [d]
[c] -> [e]
"#;
        let (result, signals) = run_with_bus("skip-chain", dsl).await;
        let out = result.expect("run");
        for node in ["b", "c", "e"] {
            assert_eq!(statuses_of(&out.node_trace, node), ["skipped"], "{node}");
        }
        assert_eq!(statuses_of(&out.node_trace, "d"), ["ok"]);
        assert_eq!(out.value["script"], json!({ "d": 1, "from_c": null }), "{}", out.value);
        let skipped: Vec<&str> = signals.iter().filter(|s| s.kind == "node_skipped").map(|s| s.node_id.as_str()).collect();
        assert_eq!(skipped, ["b", "c", "e"]);
        assert!(signals.iter().all(|s| !(s.kind == "node_start" && ["b", "c", "e"].contains(&s.node_id.as_str()))));
    }

    /// `{{ $nodes.big.text ?? $nodes.small.text }}` joins two branches: the
    /// skipped one is `null`, and the path through it is `null`, not an error.
    #[tokio::test]
    async fn a_nullish_join_reads_whichever_branch_ran() {
        let dsl = r#"
[a] trigger.manual
[gate] logic.if --when "input.manual.big"
[big] javascript.script.run -- "return { text: 'BIG' };"
[small] javascript.script.run -- "return { text: 'small' };"
[out] web.response.send --body "{{ $nodes.big.script.text ?? $nodes.small.script.text }}"
[a] -> [gate]
[gate]:true -> [big]
[gate]:false -> [small]
[big] -> [out]
[small] -> [out]
"#;
        for (big, said) in [(false, "small"), (true, "BIG")] {
            let out = run_flow("nullish-join", dsl, json!({ "big": big })).await.expect("run");
            assert_eq!(out.response.expect("answered")["text"], said);
        }
    }

    /// A reference to a node on the branch not taken is `null`, and a path
    /// through it is `null` — never `FW_EXPR_EVAL`.
    #[tokio::test]
    async fn a_reference_to_an_untaken_branch_is_null_not_an_error() {
        let dsl = r#"
[a] trigger.manual
[gate] logic.if --when "false"
[taken] javascript.script.run -- "return { text: 'never' };"
[other] javascript.script.run -- "return { ok: true };"
[out] web.response.send --body "{{ String($nodes.taken) + '/' + String($nodes.taken.script.text) + '/' + String($nodes.taken['script'].text) }}"
[a] -> [gate]
[gate]:true -> [taken]
[gate]:false -> [other]
[taken] -> [out]
[other] -> [out]
"#;
        let out = run_flow("untaken-null", dsl, json!({})).await.expect("no error");
        assert_eq!(out.response.expect("answered")["text"], "null/undefined/undefined");
    }

    /// References point upstream: a node no edge path leads from is refused
    /// at activation, with "did you mean" when a near name exists.
    #[test]
    fn a_reference_to_a_node_not_upstream_is_refused_at_activation() {
        let engine = BasicPipelineEngine::default();
        let graph = |dsl: &str| build_pipeline_graph("not-upstream", dsl).expect("graph");
        let sibling = graph("[a] trigger.manual\n[b] javascript.script.run -- \"return { x: 1 };\"\n[c] crypto.base64.encode --text \"{{ $nodes.b.script.x }}\"\n[a] -> [b]\n[a] -> [c]\n");
        let err = engine.validate_graph(&sibling).expect_err("a sibling branch is not upstream");
        assert_eq!(err.code, "FW_NODES_SCOPE_UPSTREAM");
        assert!(err.message.contains("'c'") && err.message.contains("$nodes.b"), "{}", err.message);

        let typo = graph("[a] trigger.manual\n[fetch] javascript.script.run -- \"return { x: 1 };\"\n[c] crypto.base64.encode --text \"{{ $nodes.fecth.script.x }}\"\n[a] -> [fetch]\n[fetch] -> [c]\n");
        let err = engine.validate_graph(&typo).expect_err("no such node");
        assert!(err.message.contains("did you mean $nodes.fetch"), "{}", err.message);
    }

    /// A failure resolving a node's config, or building it from the
    /// resolved config, routes to `:error` like any execution failure.
    #[tokio::test]
    async fn a_config_expression_or_build_failure_routes_to_error() {
        let dsl = r#"
[a] trigger.manual
[b] crypto.base64.encode --text "{{ input.manual.missing.deep }}"
[h] javascript.script.run -- "return { handled: input.error.code };"
[a] -> [b]
[b]:error -> [h]
"#;
        let out = run_flow("expr-routed", dsl, json!({})).await.expect("routed, not failed");
        assert_eq!(out.value["script"]["handled"], "FW_EXPR_EVAL", "{}", out.value);
        assert_eq!(statuses_of(&out.node_trace, "b"), ["error_routed"]);

        let dsl = r#"
[a] trigger.manual
[b] logic.foreach --from "input.manual.rows" --batch-size "{{ input.manual.size }}"
[h] javascript.script.run -- "return { handled: input.error.code };"
[a] -> [b]
[b]:error -> [h]
"#;
        let out = run_flow("build-routed", dsl, json!({ "rows": [1], "size": 0 })).await.expect("routed");
        assert_eq!(out.value["script"]["handled"], "FW_NODE_LOGIC_FOREACH_CONFIG", "{}", out.value);

        // Unrouted, the same failure still fails the run there.
        let err = run_flow("expr-unrouted", "[a] trigger.manual\n[b] crypto.base64.encode --text \"{{ input.manual.missing.deep }}\"\n[a] -> [b]\n", json!({}))
            .await
            .expect_err("fails");
        assert_eq!(err.code, "FW_EXPR_EVAL");
        assert_eq!(err.node_id.as_deref(), Some("b"));
    }

    /// The body runs once per item, each in its own frame; the reduce that
    /// closes the loop fires once, after every frame, folding only the items
    /// that reached it — and an empty or all-filtered list closes too.
    #[tokio::test]
    async fn a_loop_close_fires_once_over_the_items_that_reached_it() {
        let dsl = r#"
[a] trigger.manual
[f] logic.foreach --from "input.manual.rows"
[keep] logic.if --when "input.item > 1"
[x] javascript.script.run -- "return { v: input.item * 10 };"
[r] logic.reduce --initial "{ vs: [] }" --step "{ vs: $acc.vs.concat([$input.script.v]) }"
[a] -> [f]
[f]:item -> [keep]
[keep]:true -> [x]
[x] -> [r]
"#;
        let out = run_flow("loop-close", dsl, json!({ "rows": [1, 2, 3] })).await.expect("run");
        assert_eq!(out.value["reduce"]["vs"], json!([20, 30]), "{}", out.value);
        assert_eq!(out.value["manual"]["rows"], json!([1, 2, 3]), "the close answers on the loop's own payload");
        assert!(out.value.get("item").is_none(), "{}", out.value);
        assert_eq!(statuses_of(&out.node_trace, "r"), ["ok"], "one entry for the close");
        assert_eq!(statuses_of(&out.node_trace, "keep"), ["ok", "ok", "ok"]);
        assert_eq!(statuses_of(&out.node_trace, "x"), ["skipped", "ok", "ok"]);

        for rows in [json!([]), json!([1])] {
            let out = run_flow("loop-close-empty", dsl, json!({ "rows": rows })).await.expect("run");
            assert_eq!(out.value["reduce"]["vs"], json!([]), "{rows}: answers --initial");
            assert_eq!(statuses_of(&out.node_trace, "r"), ["ok"]);
        }

        // `logic.collect` closes a loop the same way.
        let dsl = "[a] trigger.manual\n[f] logic.foreach --from \"input.manual.rows\"\n[x] javascript.script.run -- \"return { v: input.item };\"\n[c] logic.collect\n[a] -> [f]\n[f]:item -> [x]\n[x] -> [c]\n";
        let out = run_flow("loop-collect", dsl, json!({ "rows": ["p", "q"] })).await.expect("run");
        assert_eq!(out.value["collect"]["count"], 2);
        let vs: Vec<&serde_json::Value> = out.value["collect"]["items"].as_array().unwrap().iter().map(|i| &i["script"]["v"]).collect();
        assert_eq!(vs, [&json!("p"), &json!("q")]);
        let out = run_flow("loop-collect-empty", dsl, json!({ "rows": [] })).await.expect("run");
        assert_eq!(out.value["collect"], json!({ "items": [], "count": 0 }));
    }

    /// Frames are keyed by the loop path: an inner loop runs whole inside
    /// each outer item, and its close fires once per outer item.
    #[tokio::test]
    async fn nested_loops_keep_their_frames_apart() {
        let dsl = r#"
[a] trigger.manual
[outer] logic.foreach --from "input.manual.groups"
[inner] logic.foreach --from "input.item.values"
[double] javascript.script.run -- "return { v: input.item * 2 };"
[sum] logic.reduce --initial "0" --step "$acc + $input.script.v"
[all] logic.collect
[a] -> [outer]
[outer]:item -> [inner]
[inner]:item -> [double]
[double] -> [sum]
[sum] -> [all]
"#;
        let input = json!({ "groups": [{ "values": [1, 2] }, { "values": [] }, { "values": [3] }] });
        let out = run_flow("nested-loops", dsl, input).await.expect("run");
        let sums: Vec<&serde_json::Value> = out.value["collect"]["items"].as_array().unwrap().iter().map(|i| &i["reduce"]).collect();
        assert_eq!(sums, [&json!(6), &json!(0), &json!(6)], "{}", out.value);
        assert_eq!(statuses_of(&out.node_trace, "sum").len(), 3);
        assert_eq!(statuses_of(&out.node_trace, "all"), ["ok"]);
        assert_eq!(statuses_of(&out.node_trace, "double").len(), 3);
    }

    /// Inside a frame a body node reads that item's answers; after the close
    /// the body is gone from `$nodes` and only the close's answer remains.
    #[tokio::test]
    async fn body_nodes_are_hidden_from_nodes_after_the_close() {
        let dsl = r#"
[a] trigger.manual
[f] logic.foreach --from "input.manual.rows"
[x] javascript.script.run -- "return { v: input.item };"
[y] javascript.script.run -- "return { seen: ctx.nodes.x.script.v };"
[r] logic.reduce --initial "0" --step "$acc + $input.script.seen"
[after] web.response.send --body "{{ String($nodes.x) + ':' + $nodes.r.reduce }}"
[a] -> [f]
[f]:item -> [x]
[x] -> [y]
[y] -> [r]
[r] -> [after]
"#;
        let out = run_flow("body-hidden", dsl, json!({ "rows": [1, 2, 3] })).await.expect("run");
        assert_eq!(out.response.expect("answered")["text"], "null:6");
    }

    /// `web.response.send` answers the caller the moment it runs; the run
    /// keeps going, and a later failure fails the run without touching what
    /// the caller already has. A second response in one run is not sent.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_response_reaches_the_caller_before_a_slow_later_node_and_survives_its_failure() {
        use crate::pipeline::model::{ExecuteOptions, Responder};
        let dsl = r#"
[a] trigger.manual
[resp] web.response.send --body "answered early"
[slow] logic.retry --max-attempts 3 --delay 1s --when "true"
[boom] javascript.script.run -- "throw new Error('late failure');"
[a] -> [resp]
[resp] -> [slow]
[slow]:retry -> [boom]
"#;
        let graph = build_pipeline_graph("early-response", dsl).expect("graph");
        let (responder, early) = Responder::channel();
        let options = ExecuteOptions { responder: Some(responder), ..Default::default() };
        let started = std::time::Instant::now();
        let run = tokio::spawn(async move {
            BasicPipelineEngine::default()
                .execute_with_options_async(&graph, &flow_ctx("early-response", json!({})), &options)
                .await
        });
        let envelope = tokio::time::timeout(std::time::Duration::from_millis(900), early)
            .await
            .expect("the response arrives before the slow node finishes")
            .expect("sent");
        assert!(!run.is_finished(), "the run is still going");
        assert_eq!(envelope["text"], "answered early");
        let err = run.await.expect("joined").expect_err("the later failure fails the run");
        assert!(started.elapsed() >= std::time::Duration::from_secs(1));
        assert!(err.message.contains("late failure"), "{}", err.message);
        assert_eq!(envelope["text"], "answered early", "what the caller has is unchanged");

        let dsl = "[a] trigger.manual\n[one] web.response.send --body first\n[two] web.response.send --body second\n[a] -> [one]\n[one] -> [two]\n";
        let out = run_flow("two-responses", dsl, json!({})).await.expect("run");
        assert_eq!(out.response.expect("answered")["text"], "first", "the first response wins");
        let two = out.node_trace.iter().find(|t| t.node_id == "two").expect("two");
        assert_eq!(two.status, "empty");
        assert!(two.error.as_deref().is_some_and(|note| note.contains("already sent")), "{:?}", two.error);
    }

    /// `value` is the answer of the last sink that ran, in DSL text order —
    /// never "whichever finished last".
    #[tokio::test]
    async fn the_run_value_is_the_last_sink_in_text_order() {
        let dsl = r#"
[a] trigger.manual
[step] javascript.script.run -- "return { step: 1 };"
[first] javascript.script.run -- "return { who: 'first' };"
[second] javascript.script.run -- "return { who: 'second' };"
[third] javascript.script.run -- "return { who: 'third' };"
[gate] logic.if --when "false"
[a] -> [second]
[a] -> [step]
[step] -> [first]
[a] -> [gate]
[gate]:true -> [third]
"#;
        let out = run_flow("value-sink", dsl, json!({})).await.expect("run");
        let order: Vec<&str> = out.node_trace.iter().filter(|t| t.status == "ok").map(|t| t.node_id.as_str()).collect();
        assert_eq!(order.last(), Some(&"first"), "first answered last in time: {order:?}");
        assert_eq!(statuses_of(&out.node_trace, "third"), ["skipped"]);
        assert_eq!(out.value["script"]["who"], "second", "{}", out.value);
    }
}

enum NodeDispatch {
    Webhook(webhook::Node),
    Schedule(schedule::Node),
    Manual(manual::Node),
    Script(crate::pipeline::nodes::shared::script::Node),
    HttpRequest(http::request::Node),
    BrowserRun(browser::run::Node),
    SqliteQuery(sqlite::query::Node),
    SekejapQuery(sekejap::query::Node),
    SekejapRecord(sekejap::record::Node),
    Postgres(postgres::query::Node),
    InlineWebResponse {
        node_id: String,
        config: web::response::Config,
    },
    InlineWebSiteGenerate {
        node_id: String,
        config: web::site::Config,
    },
    WebResponse(web::response::Node),
    Agent(ai::agent::Node),
    AiTts(ai::tts::Node),
    LogicIf(logic::if_::Node),
    LogicMatch(logic::match_::Node),
    LogicCollect(logic::collect::Node),
    LogicForeach(logic::foreach_::Node),
    LogicReduce(logic::reduce::Node),
    LogicRetry(logic::retry::Node),
    AuthTokenCreate(auth::token_create::Node),
    AuthTokenVerify(auth::token_verify::Node),
    MailSend(mail::send::Node),
    Concept(logic::concept::Node),
    WebError(weberror::Node),
    WsTrigger(ws::trigger::Node),
    WsMessageSend(ws::message_send::Node),
    WsState(ws::state::Node),
    /// Any `crypto.*` kind.
    Crypto(Box<dyn crate::pipeline::nodes::NodeHandler>),
    Kv(Box<dyn crate::pipeline::nodes::NodeHandler>),
    TriggerFunction(trigger_function::Node),
    FunctionCall(function::call::Node),
    FilePut(fs::put::Node),
    FsObject(fs::object::Node),
    MapserverCrud(mapserver::crud::Node),
    TableConvert(table::convert::Node),
    TableQuery(table::query::Node),
    FileCompress(fs::compress::Node),
    FileDecompress(fs::decompress::Node),
    GeoInspect(geo::inspect::Node),
    GeoConvert(geo::convert::Node),
    FilePdfConvert(fs::pdf::convert::Node),
    ImgThumbnail(fs::image::thumbnail::Node),
    Barcode(fs::barcode::Node),
    SvgConvert(fs::svg::convert::Node),
    ImgChromakey(fs::image::chromakey::Node),
    /// Any `input.*` kind — a pass-through validator of one envelope field.
    Input(input::Node),
    KvSubscribe(kv_subscribe::Node),
    WsClientTrigger(trigger_ws_client::Node),
    McpTrigger(mcp_trigger::Node),
    /// A node provided by an installed bundle (`x.*`).
    ///
    /// Role and implementation come from the package manifest at execution
    /// time, not from the node kind, so a node may move between composite and
    /// WASM without changing its kind.
    InstalledNode {
        kind: String,
        config: serde_json::Value,
        platform: std::sync::Arc<crate::platform::services::PlatformService>,
        /// Declared-host policy already in force, when this dispatch is itself
        /// inside another bundle's function pipeline.
        egress: Option<Arc<crate::pipeline::security::BundleEgress>>,
    },
}

/// Node kinds whose outbound destination reaches an egress guard as a URL, so a
/// bundle's declared hosts can be checked against what they actually contact.
const HOST_CHECKED_NETWORK_NODES: &[&str] = &[http::request::NODE_KIND, browser::run::NODE_KIND];

/// Refuses a network-capable node whose destination cannot be host-checked.
///
/// Capability comes from the same compiled-in table the package review reads, so
/// a network node added later is refused until it is given a guard rather than
/// silently becoming a way out of a bundle's declaration. Kinds absent from that
/// table are bundle-provided, and are governed by the dispatch they recurse into.
///
/// This runs for every bundle-provided node, whether or not that bundle declared
/// a host. Asking only inside a declaring bundle would have made silence the
/// cheapest way to reach an unreadable destination, so an author who declared
/// truthfully would be the only one constrained.
///
/// A script kind (`javascript.script.run`, `typescript.script.run`) is not a network node in the capability table, and with the
/// sandbox denying `fetch` it is not one in fact either. When an operator has
/// granted the sandbox network access, it becomes an egress path this guard
/// cannot read, and is refused on the same grounds as the rest.
fn refuse_uncheckable_egress_node(
    egress: &crate::pipeline::security::BundleEgress,
    kind: &str,
    config: &Value,
    sandbox_reaches_network: bool,
) -> Result<(), PipelineError> {
    // A room send stays inside the platform; only `--connection` leaves it.
    if kind == ws::message_send::NODE_KIND
        && config.get("connection").is_none_or(|connection| connection.is_null() || connection == "")
    {
        return Ok(());
    }
    if kind == javascript::NODE_KIND || kind == typescript::NODE_KIND {
        if sandbox_reaches_network {
            return Err(egress.refuse_uncheckable(kind));
        }
        return Ok(());
    }
    if HOST_CHECKED_NETWORK_NODES.contains(&kind) {
        return Ok(());
    }
    let reaches_network = crate::pipeline::nodes::native_node_capabilities()
        .get(kind)
        .is_some_and(|capabilities| {
            capabilities.contains(&crate::pipeline::model::NodeCapability::Network)
        });
    if reaches_network {
        return Err(egress.refuse_uncheckable(kind));
    }
    Ok(())
}


