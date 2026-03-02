//! Configuration for automaton execution strategy.

use serde::{Deserialize, Serialize};

/// Strategy for planning and execution.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlanningStrategy {
    /// Simple: plan once, execute, done (no replanning)
    PlanAndExecute,
    /// ReAct: reason → act → observe loop
    React,
    /// Hierarchical + Reflexion: decompose → plan → execute → validate → replan if needed
    HierarchicalReflexion,
}

impl Default for PlanningStrategy {
    fn default() -> Self {
        Self::HierarchicalReflexion
    }
}

/// Configuration for hierarchical + reflexion strategy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HierarchicalConfig {
    /// Max subgoals to decompose objective into (default: 5)
    pub max_subgoals: u32,
    /// Max attempts per subgoal before giving up (default: 3)
    pub max_attempts_per_subgoal: u32,
    /// Validate progress every N steps (default: 3)
    pub validation_frequency: u32,
    /// Enable replanning on validation failure (default: true)
    pub enable_replanning: bool,
    /// Max replanning attempts per subgoal (default: 2)
    pub max_replans_per_subgoal: u32,
}

impl Default for HierarchicalConfig {
    fn default() -> Self {
        Self {
            max_subgoals: 5,
            max_attempts_per_subgoal: 3,
            validation_frequency: 3,
            enable_replanning: true,
            max_replans_per_subgoal: 2,
        }
    }
}

/// Main automaton configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomatonConfig {
    /// Planning strategy
    pub strategy: PlanningStrategy,
    /// Config for hierarchical + reflexion
    pub hierarchical: HierarchicalConfig,
    /// Step budget (hard cap)
    pub step_budget: u32,
    /// Enable token counting for cost tracking
    pub track_tokens: bool,
    /// Enable memory/caching
    pub enable_memory: bool,
}

impl Default for AutomatonConfig {
    fn default() -> Self {
        Self {
            strategy: PlanningStrategy::HierarchicalReflexion,
            hierarchical: HierarchicalConfig::default(),
            step_budget: 50,
            track_tokens: true,
            enable_memory: true,
        }
    }
}
