//! Real framework engine with graph traversal and built-in node dispatch.

use std::collections::{HashMap, VecDeque};
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use std::sync::{Arc, Mutex};

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::pipeline::expr::{resolve_config_expressions, scanner::scan as scan_exprs};
use crate::pipeline::interface::PipelineEngine;
use crate::pipeline::model::{
    ExecuteOptions, NodeTraceEntry, PipelineContext, PipelineError, PipelineOutput, PipelineGraph, PipelineNode,
};
use crate::pipeline::nodes::basic::{
    agent, auth_token_create, browser_run, crypto, function_call, http_request, logic, pg_query,
    script, sjtable_mutate, sjtable_query,
    trigger::{function as trigger_function, manual, schedule, webhook, weberror},
    web_response, ws_emit, ws_sync_state, ws_trigger,
};
use crate::platform::services::PlatformService;
use crate::pipeline::nodes::{NodeHandler, NodeExecutionInput, NodeExecutionOutput};
use crate::language::{DenoSandboxEngine, LanguageEngine};
use crate::platform::services::{CredentialService, SimpleTableService};
use crate::rwe::{ReactiveWebEngine, TemplateSource, resolve_engine_or_default};
use crate::infra::transport::ws::WsHub;

/// In-memory compile cache for template nodes.
///
/// Key: hash of the template markup string.
/// Value: the compiled page artifact, shared via `Arc` for cheap clone on cache hit.
///
/// When a user saves a template the markup changes → new hash → cache miss → recompile.
/// Subsequent requests with the same markup → cache hit → skip compile, just re-render.
pub type TemplateCache = Arc<Mutex<HashMap<u64, Arc<web_response::CompiledPage>>>>;

/// Create a new empty template compile cache.
pub fn new_template_cache() -> TemplateCache {
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
    template_cache: Option<TemplateCache>,
    ws_hub: Option<Arc<WsHub>>,
    platform: Option<Arc<PlatformService>>,
    /// Filesystem root for resolving `@/` alias imports in TSX templates.
    template_root: Option<std::path::PathBuf>,
}

impl Default for BasicPipelineEngine {
    fn default() -> Self {
        let rwe_engine_id = std::env::var("ZEBFLOW_RWE_ENGINE_ID").ok();
        Self {
            language: Arc::new(DenoSandboxEngine::default()),
            rwe: resolve_engine_or_default(rwe_engine_id.as_deref()),
            credentials: None,
            simple_tables: None,
            template_cache: None,
            ws_hub: None,
            platform: None,
            template_root: None,
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
            template_cache: None,
            ws_hub: None,
            platform: None,
            template_root: None,
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

    /// Attach the platform service so n.function.call nodes can invoke sub-pipelines.
    pub fn with_platform(mut self, platform: Arc<PlatformService>) -> Self {
        self.platform = Some(platform);
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
            sjtable_query::NODE_KIND | sjtable_query::NODE_KIND_ALIAS => {
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
            sjtable_mutate::NODE_KIND => {
                let Some(simple_tables) = &self.simple_tables else {
                    return Err(PipelineError::new(
                        "FW_NODE_SJTABLE_UNAVAILABLE",
                        "simple table service is not configured on this framework engine",
                    ));
                };
                Ok(NodeDispatch::SimpleTableMutate(sjtable_mutate::Node::new(
                    serde_json::from_value(node.config.clone()).map_err(|err| {
                        PipelineError::new("FW_NODE_SJTABLE_MUTATE_CONFIG", err.to_string())
                    })?,
                    simple_tables.clone(),
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

            let exec_result: Result<NodeExecutionOutput, PipelineError> = match dispatch {
                NodeDispatch::Webhook(node) => node.execute_async(input).await,
                NodeDispatch::Schedule(node) => node.execute_async(input).await,
                NodeDispatch::Manual(node) => node.execute_async(input).await,
                NodeDispatch::Script(node) => node.execute_async(input).await,
                NodeDispatch::HttpRequest(node) => node.execute_async(input).await,
                NodeDispatch::BrowserRun(node) => node.execute_async(input).await,
                NodeDispatch::SimpleTable(node) => node.execute_async(input).await,
                NodeDispatch::SimpleTableMutate(node) => node.execute_async(input).await,
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
                            c.lock().unwrap_or_else(|e| e.into_inner()).get(&key).cloned()
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
                                    cache.lock().unwrap_or_else(|e| e.into_inner()).insert(key, fresh_arc.clone());
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
            };

            let output = match exec_result {
                Ok(out) => {
                    // Record this node's output so downstream nodes can access it via $nodes.
                    nodes_output.insert(trace_node_id.clone(), out.payload.clone());
                    node_trace.push(NodeTraceEntry {
                        node_id: trace_node_id,
                        node_kind: trace_node_kind,
                        duration_ms: node_start.elapsed().as_millis() as u64,
                        input: input_snapshot,
                        output: out.payload.clone(),
                        error: None,
                    });
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

enum NodeDispatch {
    Webhook(webhook::Node),
    Schedule(schedule::Node),
    Manual(manual::Node),
    Script(script::Node),
    HttpRequest(http_request::Node),
    BrowserRun(browser_run::Node),
    SimpleTable(sjtable_query::Node),
    SimpleTableMutate(sjtable_mutate::Node),
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
