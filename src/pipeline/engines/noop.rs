//! Minimal framework engine used as a reference implementation.
//!
//! `NoopPipelineEngine` validates node/pin wiring and returns a synthetic
//! payload instead of performing real graph traversal.

use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;

use crate::pipeline::interface::PipelineEngine;
use crate::pipeline::model::{PipelineContext, PipelineError, PipelineOutput, PipelineGraph};

/// Reference framework engine with strict pin validation and mock execution.
#[derive(Default)]
pub struct NoopPipelineEngine;

#[async_trait]
impl PipelineEngine for NoopPipelineEngine {
    fn id(&self) -> &'static str {
        "pipeline.noop"
    }

    fn validate_graph(&self, graph: &PipelineGraph) -> Result<(), PipelineError> {
        if graph.nodes.is_empty() {
            return Err(PipelineError::new(
                "FW_EMPTY_GRAPH",
                format!("pipeline '{}' has no nodes", graph.id),
            ));
        }
        let node_map: HashMap<&str, _> = graph.nodes.iter().map(|n| (n.id.as_str(), n)).collect();
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
                        "edge[{idx}] invalid from_pin '{}' for node '{}' (allowed: {:?})",
                        edge.from_pin, from.id, from.output_pins
                    ),
                ));
            }
            if !to.input_pins.iter().any(|p| p == &edge.to_pin) {
                return Err(PipelineError::new(
                    "FW_EDGE_TO_PIN",
                    format!(
                        "edge[{idx}] invalid to_pin '{}' for node '{}' (allowed: {:?})",
                        edge.to_pin, to.id, to.input_pins
                    ),
                ));
            }
        }
        Ok(())
    }

    async fn execute_with_options_async(
        &self,
        graph: &PipelineGraph,
        ctx: &PipelineContext,
        _options: &crate::pipeline::ExecuteOptions,
    ) -> Result<PipelineOutput, PipelineError> {
        self.validate_graph(graph)?;
        Ok(PipelineOutput {
            value: json!({
                "pipeline_id": graph.id,
                "node_count": graph.nodes.len(),
                "edge_count": graph.edges.len(),
                "input": ctx.input,
            }),
            trace: vec![
                format!("engine={}", self.id()),
                format!("owner={}", ctx.owner),
                format!("project={}", ctx.project),
                format!("pipeline={}", ctx.pipeline),
                format!("request_id={}", ctx.request_id),
            ],
            node_trace: vec![],
        })
    }
}
