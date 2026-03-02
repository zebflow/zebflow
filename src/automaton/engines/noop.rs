//! Minimal automaton engine used for testing and scaffolding.

use serde_json::json;

use crate::automaton::interface::AutomatonEngine;
use crate::automaton::model::{
    AutomatonContext, AutomatonError, AutomatonExecutionOutput, AutomatonObjective, AutomatonPlan,
    AutomatonResult,
};

/// Reference automaton engine that returns deterministic plan/execute outputs.
#[derive(Default)]
pub struct NoopAutomatonEngine;

impl AutomatonEngine for NoopAutomatonEngine {
    fn id(&self) -> &'static str {
        "automaton.noop"
    }

    fn plan(
        &self,
        objective: &AutomatonObjective,
        _ctx: &AutomatonContext,
    ) -> Result<AutomatonPlan, AutomatonError> {
        if objective.goal.trim().is_empty() {
            return Err(AutomatonError::new(
                "AUTOMATON_EMPTY_GOAL",
                "objective goal is required",
            ));
        }

        Ok(AutomatonPlan {
            objective_id: objective.objective_id.clone(),
            steps: vec![format!("interpret objective: {}", objective.goal)],
            metadata: json!({
                "engine": self.id(),
            }),
        })
    }

    fn execute(
        &self,
        plan: &AutomatonPlan,
        ctx: &AutomatonContext,
    ) -> Result<AutomatonExecutionOutput, AutomatonError> {
        if plan.steps.is_empty() {
            return Err(AutomatonError::new(
                "AUTOMATON_EMPTY_PLAN",
                "plan has no steps to execute",
            ));
        }

        Ok(AutomatonExecutionOutput {
            result: AutomatonResult::Succeeded,
            output: json!({
                "objective_id": plan.objective_id,
                "executed_steps": plan.steps.len(),
            }),
            trace: vec![
                format!("engine={}", self.id()),
                format!("owner={}", ctx.owner),
                format!("project={}", ctx.project),
                format!("run_id={}", ctx.run_id),
                format!("step_budget={}", ctx.step_budget),
            ],
        })
    }
}
