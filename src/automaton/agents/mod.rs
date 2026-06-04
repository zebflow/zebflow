//! Automaton agents — the reasoning brain layer.
//!
//! An agent is a goal-directed entity that uses infrastructure (LLM, tools, memory)
//! to accomplish objectives. Two agents are provided:
//!
//! ## [`zebtune::ZebtuneAgent`]
//!
//! Full autonomous agent on par with Perplexity, Claude Code, or Cursor.
//! Phases: strategic planning → execution loop → validation → replanning → synthesis.
//!
//! - Uses `crate::automaton::planning` for goal decomposition
//! - Uses `crate::automaton::memory` for context persistence (M8)
//! - Exposed via pipeline node `n.ai.agent.zebtune`
//!
//! ## [`tool_caller::ToolCallerAgent`]
//!
//! Simple single-pass tool sequence executor.
//! Receives goal + tool definitions → LLM generates structured tool calls →
//! executes them → feeds results back → final answer.
//!
//! - Uses native OpenAI function calling API (no text parsing)
//! - No planning or replanning phase
//! - Exposed via pipeline node `n.ai.agent.tool_caller`
//!
//! ## [`engines`]
//!
//! Implementations of the `AutomatonEngine` trait (plan/execute contract).
//! `NoopAutomatonEngine` is the reference implementation used in tests.

pub mod contract;
pub mod engines;
pub mod tool_caller;
pub mod zebtune;

pub use engines::NoopAutomatonEngine;
pub use tool_caller::ToolCallerAgent;
pub use zebtune::ZebtuneAgent;
