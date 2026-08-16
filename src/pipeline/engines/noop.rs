//! Minimal framework engine used as a reference implementation.
//!
//! `NoopPipelineEngine` validates node/pin wiring and returns a synthetic
//! payload instead of performing real graph traversal.

use async_trait::async_trait;
use serde_json::json;

use crate::pipeline::interface::PipelineEngine;
use crate::pipeline::model::{PipelineContext, PipelineError, PipelineGraph, PipelineOutput};

/// Reference framework engine with strict pin validation and mock execution.
#[derive(Default)]
pub struct NoopPipelineEngine;

#[async_trait]
impl PipelineEngine for NoopPipelineEngine {
    fn id(&self) -> &'static str {
        "pipeline.noop"
    }

    fn validate_graph(&self, graph: &PipelineGraph) -> Result<(), PipelineError> {
        crate::contracts::kinds::validate_pipeline_activation(graph).map_err(|error| {
            PipelineError::new(
                error.violation_code().unwrap_or("FW_PIPELINE_CONTRACT"),
                error.to_string(),
            )
        })
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
