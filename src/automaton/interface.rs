use super::model::{
    AutomatonContext, AutomatonError, AutomatonExecutionOutput, AutomatonObjective, AutomatonPlan,
};

/// Engine-agnostic automaton contract used by Zebflow runtime.
pub trait AutomatonEngine: Send + Sync {
    /// Stable engine id.
    fn id(&self) -> &'static str;

    /// Produces an executable plan from a high-level objective.
    fn plan(
        &self,
        objective: &AutomatonObjective,
        ctx: &AutomatonContext,
    ) -> Result<AutomatonPlan, AutomatonError>;

    /// Executes a previously generated plan.
    fn execute(
        &self,
        plan: &AutomatonPlan,
        ctx: &AutomatonContext,
    ) -> Result<AutomatonExecutionOutput, AutomatonError>;
}
