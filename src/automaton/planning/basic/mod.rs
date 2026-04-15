//! Basic planning strategy: hierarchical decomposition with validation and replanning.
//!
//! Used by ZebtuneAgent (TODO M6) for goal decomposition and adaptive replanning.
//! See `prompts` submodule for the LLM prompt templates that drive each phase.

pub mod prompts;

use serde::{Deserialize, Serialize};

/// One subgoal in a hierarchical plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubGoal {
    pub id: usize,
    pub description: String,
    pub validation_criteria: String,
    pub steps: Vec<String>,
    pub status: SubGoalStatus,
    pub attempts: u32,
    pub replans: u32,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SubGoalStatus {
    Pending,
    InProgress,
    Completed,
    Failed,
}

/// Hierarchical plan with subgoals.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HierarchicalPlan {
    pub objective: String,
    pub subgoals: Vec<SubGoal>,
    pub current_subgoal: usize,
    pub status: PlanStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlanStatus {
    Planning,
    Executing,
    Completed,
    Failed,
}

impl HierarchicalPlan {
    pub fn new(objective: String) -> Self {
        Self {
            objective,
            subgoals: Vec::new(),
            current_subgoal: 0,
            status: PlanStatus::Planning,
        }
    }

    pub fn current(&self) -> Option<&SubGoal> {
        self.subgoals.get(self.current_subgoal)
    }

    pub fn current_mut(&mut self) -> Option<&mut SubGoal> {
        self.subgoals.get_mut(self.current_subgoal)
    }

    /// Mark current subgoal complete and advance. Returns false when all subgoals done.
    pub fn complete_current(&mut self) -> bool {
        if let Some(sg) = self.current_mut() {
            sg.status = SubGoalStatus::Completed;
        }
        self.current_subgoal += 1;
        if self.current_subgoal >= self.subgoals.len() {
            self.status = PlanStatus::Completed;
            false
        } else {
            true
        }
    }

    pub fn fail_current(&mut self) {
        if let Some(sg) = self.current_mut() {
            sg.status = SubGoalStatus::Failed;
        }
        self.status = PlanStatus::Failed;
    }

    pub fn progress(&self) -> (usize, usize) {
        let completed = self
            .subgoals
            .iter()
            .filter(|sg| sg.status == SubGoalStatus::Completed)
            .count();
        (completed, self.subgoals.len())
    }
}

/// Validation result for subgoal completion check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub success: bool,
    pub confidence: f64,
    pub reason: String,
}
