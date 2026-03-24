//! Automaton planning subsystem.
//!
//! Planning is a first-class concern: lean, effective planning is critical
//! to any autonomous agent that operates in complex or ambiguous environments.
//! By treating planning as a dedicated layer, Zebflow can explore and swap
//! different planning strategies independently of agent execution logic.
//!
//! ## Current
//!
//! - [`basic`] — hierarchical decomposition with validation and replanning.
//!   Goal → subgoals → steps. Each subgoal validated; failed subgoals replanned.
//!   Used by ZebtuneAgent (TODO M6 integration).
//!
//! ## Planned
//!
//! - `hierarchical/` — multi-level decomposition with dependency tracking.
//! - `mcts/` — Monte Carlo Tree Search for stochastic environments.
//! - `reflexion/` — Reflexion-style self-critique and verbal reinforcement learning.

pub mod basic;

pub use basic::{HierarchicalPlan, PlanStatus, SubGoal, SubGoalStatus, ValidationResult};
