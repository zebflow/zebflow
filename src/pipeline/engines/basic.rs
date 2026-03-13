//! Real framework engine with graph traversal and built-in node dispatch.

use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::pipeline::interface::PipelineEngine;
use crate::pipeline::model::{
    ExecuteOptions, PipelineContext, PipelineError, PipelineOutput, PipelineGraph, PipelineNode,
};
use crate::pipeline::nodes::basic::{
    auth_token_create, crypto, http_request, logic, pg_query, script, sjtable_query,
    trigger::{manual, schedule, webhook, weberror},
    web_render, ws_emit, ws_sync_state, ws_trigger, zebtune,
};
use crate::pipeline::nodes::{NodeHandler, NodeExecutionInput};
use crate::language::{DenoSandboxEngine, LanguageEngine};
use crate::platform::services::{CredentialService, SimpleTableService};
use crate::rwe::{ReactiveWebEngine, TemplateSource, resolve_engine_or_default};
use crate::infra::transport::ws::WsHub;

/// In-memory compile cache for `n.web.render` nodes.
///
/// Key: hash of the template markup string.
/// Value: the compiled node artifact, shared via `Arc` for cheap clone on cache hit.
///
/// When a user saves a template the markup changes → new hash → cache miss → recompile.
/// Subsequent requests with the same markup → cache hit → skip compile, just re-render.
pub type WebRenderCache = Arc<Mutex<HashMap<u64, Arc<web_render::Compiled>>>>;

/// Create a new empty web render cache.
pub fn new_web_render_cache() -> WebRenderCache {
    Arc::new(Mutex::new(HashMap::new()))
}

fn hash_markup(s: &str) -> u64 {
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

/// Main framework engine used for real pipeline execution.
pub struct BasicPipelineEngine {
    language: Arc<dyn LanguageEngine>,
    rwe: Arc<dyn ReactiveWebEngine>,
    credentials: Option<Arc<CredentialService>>,
    simple_tables: Option<Arc<SimpleTableService>>,
    web_render_cache: Option<WebRenderCache>,
    ws_hub: Option<Arc<WsHub>>,
}

impl Default for BasicPipelineEngine {
    fn default() -> Self {
        let rwe_engine_id = std::env::var("ZEBFLOW_RWE_ENGINE_ID").ok();
        Self {
            language: Arc::new(DenoSandboxEngine::default()),
            rwe: resolve_engine_or_default(rwe_engine_id.as_deref()),
            credentials: None,
            simple_tables: None,
            web_render_cache: None,
            ws_hub: None,
        }
    }
}

impl BasicPipelineEngine {
    pub fn new(
        language: Arc<dyn LanguageEngine>,
        rwe: Arc<dyn ReactiveWebEngine>,
        credentials: Option<Arc<CredentialService>>,
        simple_tables: Option<Arc<SimpleTableService>>,
    ) -> Self {
        Self {
            language,
            rwe,
            credentials,
            simple_tables,
            web_render_cache: None,
            ws_hub: None,
        }
    }

    /// Attach a shared web render compile cache to this engine.
    /// Same cache instance should be passed on every request so hits accumulate.
    pub fn with_web_render_cache(mut self, cache: WebRenderCache) -> Self {
        self.web_render_cache = Some(cache);
        self
    }

    /// Attach the WS hub so ws_sync_state and ws_emit nodes can access rooms.
    pub fn with_ws_hub(mut self, hub: Arc<WsHub>) -> Self {
        self.ws_hub = Some(hub);
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
            )?)),
            sjtable_query::NODE_KIND => {
                let Some(simple_tables) = &self.simple_tables else {
                    return Err(PipelineError::new(
                        "FW_NODE_SJTABLE_UNAVAILABLE",
                        "simple table service is not configured on this framework engine",
                    ));
                };
                Ok(NodeDispatch::SimpleTable(sjtable_query::Node::new(
                    serde_json::from_value(node.config.clone()).map_err(|err| {
                        PipelineError::new("FW_NODE_SJTABLE_CONFIG", err.to_string())
                    })?,
                    simple_tables.clone(),
                    self.language.clone(),
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
            web_render::NODE_KIND => {
                let mut config: web_render::Config = serde_json::from_value(node.config.clone())
                    .map_err(|err| {
                        PipelineError::new("FW_NODE_WEB_RENDER_CONFIG", err.to_string())
                    })?;
                // Derive template_id from template_path when not explicitly set.
                if config.template_id.is_empty() && !config.template_path.is_empty() {
                    config.template_id = config.template_path.clone();
                }
                Ok(NodeDispatch::InlineWebRender {
                    node_id: node.id.clone(),
                    config,
                })
            }
            zebtune::NODE_KIND | "n.zebtune" => {
                let config: zebtune::Config =
                    serde_json::from_value(node.config.clone()).unwrap_or_default();
                let llm = crate::automaton::llm::client_from_env();
                Ok(NodeDispatch::Zebtune(zebtune::Node::new(config, llm)))
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
            self.build_node(node)?;
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
                }),
                step_tx: step_tx.clone(),
            });
        }

        let mut trace = vec![format!("engine={}", self.id())];
        let mut last_value = Value::Null;
        // merge_pending: node_id -> { pin_name -> payload }
        let mut merge_pending: HashMap<String, HashMap<String, Value>> = HashMap::new();
        // first_fired: tracks merge nodes that already fired (first_completed strategy)
        let mut first_fired: std::collections::HashSet<String> = std::collections::HashSet::new();

        while let Some(input) = queue.pop_front() {
            let node = node_map.get(input.node_id.as_str()).ok_or_else(|| {
                PipelineError::new("FW_EXEC_NODE", format!("node '{}' missing", input.node_id))
            })?;
            let dispatch = self.build_node(node)?;
            let output = match dispatch {
                NodeDispatch::Webhook(node) => node.execute_async(input).await?,
                NodeDispatch::Schedule(node) => node.execute_async(input).await?,
                NodeDispatch::Manual(node) => node.execute_async(input).await?,
                NodeDispatch::Script(node) => node.execute_async(input).await?,
                NodeDispatch::HttpRequest(node) => node.execute_async(input).await?,
                NodeDispatch::SimpleTable(node) => node.execute_async(input).await?,
                NodeDispatch::Postgres(node) => node.execute_async(input).await?,
                NodeDispatch::InlineWebRender { node_id, config } => {
                    // Require markup — same contract as render_from_config.
                    let markup = config.markup.as_deref().unwrap_or("").trim();
                    if markup.is_empty() {
                        return Err(PipelineError::new(
                            "FW_NODE_WEB_RENDER_CONFIG",
                            format!("node '{node_id}' requires config.markup for inline execution"),
                        ));
                    }

                    // Check compile cache: same markup hash → reuse CompiledTemplate.
                    // Cache miss (first request or user saved new content) → compile + store.
                    let key = hash_markup(markup);
                    let cached = self.web_render_cache.as_ref().and_then(|c| {
                        c.lock().unwrap_or_else(|e| e.into_inner()).get(&key).cloned()
                    });

                    let compiled = if let Some(hit) = cached {
                        hit
                    } else {
                        let fresh = Arc::new(
                            web_render::Node::compile(
                                &node_id,
                                &config,
                                &TemplateSource {
                                    id: config.template_id.clone(),
                                    source_path: None,
                                    markup: markup.to_string(),
                                },
                                self.rwe.as_ref(),
                                self.language.as_ref(),
                            )
                            .map_err(|e| {
                                PipelineError::new(
                                    "FW_NODE_WEB_RENDER_COMPILE",
                                    e.to_string(),
                                )
                            })?,
                        );
                        if let Some(cache) = &self.web_render_cache {
                            cache
                                .lock()
                                .unwrap_or_else(|e| e.into_inner())
                                .insert(key, fresh.clone());
                        }
                        fresh
                    };

                    web_render::render_with_engines(
                        &compiled,
                        input.payload,
                        input.metadata,
                        self.rwe.as_ref(),
                        self.language.as_ref(),
                        &ctx.request_id,
                    )?
                }
                NodeDispatch::Zebtune(node) => node.execute_async(input).await?,
                NodeDispatch::LogicIf(node) => node.execute_async(input).await?,
                NodeDispatch::LogicSwitch(node) => node.execute_async(input).await?,
                NodeDispatch::LogicBranch(node) => node.execute_async(input).await?,
                NodeDispatch::LogicMerge(node) => node.execute_async(input).await?,
                NodeDispatch::AuthTokenCreate(node) => node.execute_async(input).await?,
                NodeDispatch::WebError(node) => node.execute_async(input).await?,
                NodeDispatch::WsTrigger(node) => node.execute_async(input).await?,
                NodeDispatch::WsSyncState(node) => node.execute_async(input).await?,
                NodeDispatch::WsEmit(node) => node.execute_async(input).await?,
                NodeDispatch::Crypto(node) => node.execute_async(input).await?,
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
        })
    }
}

enum NodeDispatch {
    Webhook(webhook::Node),
    Schedule(schedule::Node),
    Manual(manual::Node),
    Script(script::Node),
    HttpRequest(http_request::Node),
    SimpleTable(sjtable_query::Node),
    Postgres(pg_query::Node),
    InlineWebRender {
        node_id: String,
        config: web_render::Config,
    },
    Zebtune(zebtune::Node),
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
