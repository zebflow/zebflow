//! # Automaton
//!
//! Zebflow's autonomous agent infrastructure. An independent module with no
//! platform or pipeline dependencies — usable as a standalone library.
//!
//! Zebtune is the runtime instance: a goal-directed agent that can plan,
//! use tools, validate progress, replan, and synthesize answers.
//! Target capability: on par with Perplexity, Claude Code, or Cursor.
//!
//! ---
//!
//! ## Architecture: 5-Layer Model
//!
//! ```text
//! ┌──────────────────────────────────────────────────────────────┐
//! │  AGENTS           reasoning — how the automaton thinks       │
//! │  ├── zebtune      full autonomous: plan→act→validate→replan  │
//! │  └── tool_caller  single-pass structured tool sequence       │
//! ├──────────────────────────────────────────────────────────────┤
//! │  PLANNING         first-class goal decomposition layer       │
//! │  └── basic/       HierarchicalPlan, SubGoal, ValidationResult│
//! ├──────────────────────────────────────────────────────────────┤
//! │  MEMORY           first-class context retention layer        │
//! │  └── basic/       ConversationHistory, TokenUsage            │
//! ├──────────────────────────────────────────────────────────────┤
//! │  INTELLIGENCE     AI-native capabilities (not LLM reasoning) │
//! │  └── (planned: tts, stt, ocr, vectorize, classify, regress)  │
//! ├──────────────────────────────────────────────────────────────┤
//! │  INFRA            plumbing: LLM clients, REPL, shell tools    │
//! │  ├── llm_interface LlmCall — single LLM interface (call + call_with_tools) │
//! │  ├── http_client  OpenAiHttpClient + AnthropicClient + factories            │
//! │  ├── llm.rs       re-exports factories (backward-compat module path)        │
//! │  ├── model.rs     AutomatonContext, Objective, Plan, Error   │
//! │  ├── shell_tools  ToolRegistry: ls, pwd, python              │
//! │  └── repl.rs      interactive REPL mode                      │
//! └──────────────────────────────────────────────────────────────┘
//! ```
//!
//! ---
//!
//! ## Two Agents
//!
//! ### [`agents::zebtune::ZebtuneAgent`] — Full Autonomous Agent
//!
//! Goal → strategic plan → execution loop → validation → replanning → synthesis.
//!
//! ```text
//! Phase 1: Strategic Planning  [TODO M6]
//!   LLM decomposes goal → HierarchicalPlan (subgoals + validation criteria)
//!
//! Phase 2: Execution Loop
//!   LLM turn → detect tool request → execute → feed result back → repeat
//!   TODO M7: upgrade from text-parsing (RUN:) to native function calling
//!
//! Phase 3: Validation  [TODO M6]
//!   LLM checks if subgoal criteria met → pass/fail with confidence score
//!
//! Phase 4: Adaptive Replanning  [TODO M6]
//!   On failure: LLM generates alternative plan for the failed subgoal
//!
//! Phase 5: Synthesis
//!   Final answer assembled from chain + execution trace
//! ```
//!
//! Exposed as pipeline node `n.ai.agent.zebtune`.
//!
//! ### [`agents::tool_caller::ToolCallerAgent`] — Simple Tool Sequence Agent
//!
//! Goal + tool definitions → LLM native function calling → execute sequence → answer.
//!
//! ```text
//! - No strategic planning phase
//! - Uses OpenAI tools API (structured function calls, not text parsing)
//! - Suitable for deterministic task completion with known tools upfront
//! ```
//!
//! Exposed as pipeline node `n.ai.agent.tool_caller`.
//!
//! ---
//!
//! ## Infrastructure Note
//!
//! `llm.rs` is a thin re-export facade for `http_client` factory functions — NOT an agent.
//! `llm_interface.rs` defines `LlmCall` — the single LLM interface used by all agents.
//!
//! ---
//!
//! ## Milestone Roadmap
//!
//! | Milestone | Focus |
//! |-----------|-------|
//! | M6 | Strategic planning: goal decomposition, validation, replanning |
//! | M7 | Native function calling: replace RUN: text parsing in ZebtuneAgent ✅ DONE |
//! | M7 | Tool expansion: file ops, shell allowlist, web fetch, git, code |
//! | M8 | Memory: graph/, vector/, episodic/, semantic/ backends |
//! | M9 | Config: security policy, deployment profiles, user prefs |
//! | M10 | Cost optimization: model routing, batching, context management |
//!
//! ---
//!
//! ## Security Model
//!
//! Allowlist-only. No tool runs without explicit registration.
//! Shell: allowlisted commands only (no `rm -rf`, `dd`, etc.).
//! Web: domain allowlist.
//! Step budget: hard cap on LLM+tool iterations per run.

pub mod agents;
pub mod infra;
pub mod intelligence;
pub mod memory;
pub mod planning;

// ── Public API re-exports ────────────────────────────────────────────────────

// Core types (used by lib.rs, bin/zebtune.rs, platform)
pub use infra::interface::AutomatonEngine;
pub use infra::model::{
    AutomatonContext, AutomatonError, AutomatonExecutionOutput, AutomatonObjective, AutomatonPlan,
    AutomatonResult,
};
pub use infra::registry::AutomatonEngineRegistry;

// REPL utilities (used by bin/zebtune.rs)
pub use infra::repl::{
    check_llm, log_llm_status, print_running_mechanism, run_interactive,
    run_interactive_with_llm, run_one_turn, strip_thinking,
};

// Shell tool registry (used by zebtune pipeline node)
pub use infra::shell_tools::{
    LsTool, PwdTool, PythonTool, Tool, ToolRegistry, default_registry, enabled_auto_commands,
};

// Engine implementations
pub use agents::engines::NoopAutomatonEngine;

// Agents
pub use agents::zebtune::ZebtuneAgent;
pub use agents::tool_caller::ToolCallerAgent;

// Re-export llm module so `zebflow::automaton::llm::client_from_env()` keeps working
pub use infra::llm;
