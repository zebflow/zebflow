use async_trait::async_trait;

use super::model::{
    ExecuteOptions, FrameworkContext, FrameworkError, FrameworkOutput, PipelineGraph,
};

/// Framework-level execution interface.
///
/// This layer owns orchestration semantics (graph traversal, branching,
/// merging, retries, observability envelopes) and delegates script/render work
/// into language and RWE engines.
#[async_trait]
pub trait FrameworkEngine: Send + Sync {
    /// Stable engine id used by registries.
    fn id(&self) -> &'static str;

    /// Validates structural constraints for a pipeline graph.
    fn validate_graph(&self, graph: &PipelineGraph) -> Result<(), FrameworkError>;

    /// Executes with optional step stream (e.g. for SSE).
    async fn execute_with_options_async(
        &self,
        graph: &PipelineGraph,
        ctx: &FrameworkContext,
        options: &ExecuteOptions,
    ) -> Result<FrameworkOutput, FrameworkError>;

    /// Executes a pipeline graph for a single request context asynchronously.
    async fn execute_async(
        &self,
        graph: &PipelineGraph,
        ctx: &FrameworkContext,
    ) -> Result<FrameworkOutput, FrameworkError> {
        self.execute_with_options_async(graph, ctx, &ExecuteOptions::default())
            .await
    }

    /// Blocking wrapper for non-async call sites.
    fn execute_with_options(
        &self,
        graph: &PipelineGraph,
        ctx: &FrameworkContext,
        options: &ExecuteOptions,
    ) -> Result<FrameworkOutput, FrameworkError> {
        if tokio::runtime::Handle::try_current().is_ok() {
            return Err(FrameworkError::new(
                "FW_ENGINE_SYNC_IN_ASYNC",
                "synchronous FrameworkEngine::execute_with_options used inside async runtime; call execute_with_options_async instead",
            ));
        }
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|err| FrameworkError::new("FW_ENGINE_RUNTIME", err.to_string()))?;
        runtime.block_on(self.execute_with_options_async(graph, ctx, options))
    }

    /// Blocking wrapper for non-async call sites.
    fn execute(
        &self,
        graph: &PipelineGraph,
        ctx: &FrameworkContext,
    ) -> Result<FrameworkOutput, FrameworkError> {
        self.execute_with_options(graph, ctx, &ExecuteOptions::default())
    }
}
