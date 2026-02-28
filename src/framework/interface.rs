use super::model::{FrameworkContext, FrameworkError, FrameworkOutput, PipelineGraph};

/// Framework-level execution interface.
///
/// This layer owns orchestration semantics (graph traversal, branching,
/// merging, retries, observability envelopes) and delegates script/render work
/// into language and RWE engines.
pub trait FrameworkEngine: Send + Sync {
    /// Stable engine id used by registries.
    fn id(&self) -> &'static str;

    /// Validates structural constraints for a pipeline graph.
    ///
    /// Typical checks:
    ///
    /// - node existence
    /// - pin compatibility
    /// - graph-level invariants
    fn validate_graph(&self, graph: &PipelineGraph) -> Result<(), FrameworkError>;

    /// Executes a pipeline graph for a single request context.
    ///
    /// Implementations should return deterministic traces suitable for
    /// observability and debugging.
    fn execute(
        &self,
        graph: &PipelineGraph,
        ctx: &FrameworkContext,
    ) -> Result<FrameworkOutput, FrameworkError>;
}
