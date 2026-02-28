//! Real framework engine with graph traversal and built-in node dispatch.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use serde_json::{Value, json};

use crate::framework::interface::FrameworkEngine;
use crate::framework::model::{FrameworkContext, FrameworkError, FrameworkOutput, PipelineGraph, PipelineNode};
use crate::framework::nodes::{FrameworkNode, NodeExecutionInput};
use crate::framework::nodes::basic::{
    pg_query, script, sjtable_query,
    trigger::{schedule, webhook},
    web_render,
};
use crate::language::{DenoSandboxEngine, LanguageEngine};
use crate::platform::services::{CredentialService, SimpleTableService};
use crate::rwe::{NoopReactiveWebEngine, ReactiveWebEngine};

/// Main framework engine used for real pipeline execution.
pub struct BasicFrameworkEngine {
    language: Arc<dyn LanguageEngine>,
    rwe: Arc<dyn ReactiveWebEngine>,
    credentials: Option<Arc<CredentialService>>,
    simple_tables: Option<Arc<SimpleTableService>>,
}

impl Default for BasicFrameworkEngine {
    fn default() -> Self {
        Self {
            language: Arc::new(DenoSandboxEngine::default()),
            rwe: Arc::new(NoopReactiveWebEngine),
            credentials: None,
            simple_tables: None,
        }
    }
}

impl BasicFrameworkEngine {
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
        }
    }

    fn build_node(&self, node: &PipelineNode) -> Result<NodeDispatch, FrameworkError> {
        match node.kind.as_str() {
            webhook::NODE_KIND => Ok(NodeDispatch::Webhook(webhook::Node::new(
                serde_json::from_value(node.config.clone()).map_err(|err| {
                    FrameworkError::new("FW_NODE_WEBHOOK_CONFIG", err.to_string())
                })?,
            ))),
            schedule::NODE_KIND => Ok(NodeDispatch::Schedule(schedule::Node::new(
                serde_json::from_value(node.config.clone()).map_err(|err| {
                    FrameworkError::new("FW_NODE_SCHEDULE_CONFIG", err.to_string())
                })?,
            ))),
            script::NODE_KIND => Ok(NodeDispatch::Script(script::Node::new(
                &node.id,
                serde_json::from_value(node.config.clone()).map_err(|err| {
                    FrameworkError::new("FW_NODE_SCRIPT_CONFIG", err.to_string())
                })?,
                self.language.clone(),
            )?)),
            sjtable_query::NODE_KIND => {
                let Some(simple_tables) = &self.simple_tables else {
                    return Err(FrameworkError::new(
                        "FW_NODE_SJTABLE_UNAVAILABLE",
                        "simple table service is not configured on this framework engine",
                    ));
                };
                Ok(NodeDispatch::SimpleTable(sjtable_query::Node::new(
                    serde_json::from_value(node.config.clone()).map_err(|err| {
                        FrameworkError::new("FW_NODE_SJTABLE_CONFIG", err.to_string())
                    })?,
                    simple_tables.clone(),
                )?))
            }
            pg_query::NODE_KIND => {
                let Some(credentials) = &self.credentials else {
                    return Err(FrameworkError::new(
                        "FW_NODE_PG_UNAVAILABLE",
                        "credential service is not configured on this framework engine",
                    ));
                };
                Ok(NodeDispatch::Postgres(pg_query::Node::new(
                    serde_json::from_value(node.config.clone()).map_err(|err| {
                        FrameworkError::new("FW_NODE_PG_CONFIG", err.to_string())
                    })?,
                    credentials.clone(),
                )?))
            }
            web_render::NODE_KIND => {
                let config: web_render::Config =
                    serde_json::from_value(node.config.clone()).map_err(|err| {
                        FrameworkError::new("FW_NODE_WEB_RENDER_CONFIG", err.to_string())
                    })?;
                Ok(NodeDispatch::InlineWebRender {
                    node_id: node.id.clone(),
                    config,
                })
            }
            other => Err(FrameworkError::new(
                "FW_NODE_KIND_UNSUPPORTED",
                format!("unsupported node kind '{}'", other),
            )),
        }
    }
}

impl FrameworkEngine for BasicFrameworkEngine {
    fn id(&self) -> &'static str {
        "framework.basic"
    }

    fn validate_graph(&self, graph: &PipelineGraph) -> Result<(), FrameworkError> {
        if graph.nodes.is_empty() {
            return Err(FrameworkError::new(
                "FW_EMPTY_GRAPH",
                format!("pipeline '{}' has no nodes", graph.id),
            ));
        }
        let node_map: HashMap<&str, _> = graph.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
        for entry in &graph.entry_nodes {
            if !node_map.contains_key(entry.as_str()) {
                return Err(FrameworkError::new(
                    "FW_ENTRY_NODE",
                    format!("unknown entry node '{}'", entry),
                ));
            }
        }
        for (idx, edge) in graph.edges.iter().enumerate() {
            let from = node_map.get(edge.from_node.as_str()).ok_or_else(|| {
                FrameworkError::new(
                    "FW_EDGE_FROM_NODE",
                    format!("edge[{idx}] unknown from_node '{}'", edge.from_node),
                )
            })?;
            let to = node_map.get(edge.to_node.as_str()).ok_or_else(|| {
                FrameworkError::new(
                    "FW_EDGE_TO_NODE",
                    format!("edge[{idx}] unknown to_node '{}'", edge.to_node),
                )
            })?;
            if !from.output_pins.iter().any(|p| p == &edge.from_pin) {
                return Err(FrameworkError::new(
                    "FW_EDGE_FROM_PIN",
                    format!("edge[{idx}] invalid from_pin '{}' for node '{}'", edge.from_pin, from.id),
                ));
            }
            if !to.input_pins.iter().any(|p| p == &edge.to_pin) {
                return Err(FrameworkError::new(
                    "FW_EDGE_TO_PIN",
                    format!("edge[{idx}] invalid to_pin '{}' for node '{}'", edge.to_pin, to.id),
                ));
            }
        }
        for node in &graph.nodes {
            self.build_node(node)?;
        }
        Ok(())
    }

    fn execute(
        &self,
        graph: &PipelineGraph,
        ctx: &FrameworkContext,
    ) -> Result<FrameworkOutput, FrameworkError> {
        self.validate_graph(graph)?;

        let node_map: HashMap<&str, &PipelineNode> =
            graph.nodes.iter().map(|node| (node.id.as_str(), node)).collect();
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

        let mut queue = VecDeque::new();
        for node_id in start_nodes {
            let node = node_map
                .get(node_id.as_str())
                .ok_or_else(|| FrameworkError::new("FW_ENTRY_NODE", "entry node missing"))?;
            let first_pin = node
                .input_pins
                .first()
                .cloned()
                .unwrap_or_else(|| "in".to_string());
            queue.push_back(NodeExecutionInput {
                node_id: node.id.clone(),
                input_pin: first_pin,
                payload: ctx.input.clone(),
                metadata: json!({
                    "owner": ctx.owner,
                    "project": ctx.project,
                    "pipeline": ctx.pipeline,
                    "request_id": ctx.request_id,
                }),
            });
        }

        let mut trace = vec![format!("engine={}", self.id())];
        let mut last_value = Value::Null;

        while let Some(input) = queue.pop_front() {
            let node = node_map.get(input.node_id.as_str()).ok_or_else(|| {
                FrameworkError::new("FW_EXEC_NODE", format!("node '{}' missing", input.node_id))
            })?;
            let dispatch = self.build_node(node)?;
            let output = match dispatch {
                NodeDispatch::Webhook(node) => node.execute(input)?,
                NodeDispatch::Schedule(node) => node.execute(input)?,
                NodeDispatch::Script(node) => node.execute(input)?,
                NodeDispatch::SimpleTable(node) => node.execute(input)?,
                NodeDispatch::Postgres(node) => node.execute(input)?,
                NodeDispatch::InlineWebRender { node_id, config } => web_render::render_from_config(
                    &node_id,
                    &config,
                    input.payload,
                    input.metadata,
                    self.rwe.as_ref(),
                    self.language.as_ref(),
                    &ctx.request_id,
                )?,
            };
            trace.extend(output.trace.clone());
            last_value = output.payload.clone();
            if let Some(next_edges) =
                outgoing.get(&(node.id.as_str(), output.output_pin.as_str()))
            {
                for (to_node, to_pin) in next_edges {
                    queue.push_back(NodeExecutionInput {
                        node_id: (*to_node).to_string(),
                        input_pin: (*to_pin).to_string(),
                        payload: output.payload.clone(),
                        metadata: json!({
                            "owner": ctx.owner,
                            "project": ctx.project,
                            "pipeline": ctx.pipeline,
                            "request_id": ctx.request_id,
                        }),
                    });
                }
            }
        }

        Ok(FrameworkOutput { value: last_value, trace })
    }
}

enum NodeDispatch {
    Webhook(webhook::Node),
    Schedule(schedule::Node),
    Script(script::Node),
    SimpleTable(sjtable_query::Node),
    Postgres(pg_query::Node),
    InlineWebRender {
        node_id: String,
        config: web_render::Config,
    },
}
