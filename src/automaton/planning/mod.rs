//! Strategic planning: decomposition, validation, replanning.

pub mod prompts;

use serde::{Deserialize, Serialize};

/// One subgoal in hierarchical plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubGoal {
    /// Subgoal ID (1-indexed)
    pub id: usize,
    /// Description
    pub description: String,
    /// Success criteria (how to validate completion)
    pub validation_criteria: String,
    /// Planned steps for this subgoal
    pub steps: Vec<String>,
    /// Status
    pub status: SubGoalStatus,
    /// Attempts so far
    pub attempts: u32,
    /// Replanning count
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
    /// Original objective
    pub objective: String,
    /// Decomposed subgoals
    pub subgoals: Vec<SubGoal>,
    /// Current subgoal index
    pub current_subgoal: usize,
    /// Overall status
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
    /// Create new plan from objective.
    pub fn new(objective: String) -> Self {
        Self {
            objective,
            subgoals: Vec::new(),
            current_subgoal: 0,
            status: PlanStatus::Planning,
        }
    }

    /// Get current active subgoal.
    pub fn current(&self) -> Option<&SubGoal> {
        self.subgoals.get(self.current_subgoal)
    }

    /// Get current active subgoal (mutable).
    pub fn current_mut(&mut self) -> Option<&mut SubGoal> {
        self.subgoals.get_mut(self.current_subgoal)
    }

    /// Mark current subgoal complete and move to next.
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

    /// Mark current subgoal failed.
    pub fn fail_current(&mut self) {
        if let Some(sg) = self.current_mut() {
            sg.status = SubGoalStatus::Failed;
        }
        self.status = PlanStatus::Failed;
    }

    /// Get progress (completed/total).
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
    /// Did subgoal complete successfully?
    pub success: bool,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
    /// Explanation
    pub reason: String,
}
