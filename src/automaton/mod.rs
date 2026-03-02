//! Automaton: autonomous goal execution + interactive REPL (Zebtune).
//!
//! - Objective → plan → execute; contract + flow for LLM-in-the-loop (tools, chain-of-thought).
//! - REPL: one message = one objective; optional LLM + allowlisted tools.

pub mod assistant_config;
pub mod config;
pub mod contract;
pub mod engines;
pub mod flow;
pub mod http_client;
pub mod interface;
pub mod llm_interface;
pub mod memory;
pub mod model;
pub mod planning;
pub mod registry;
pub mod repl;
pub mod tools;

pub use contract::{
    AgenticRunResult, Message, MessageRole, ToolCall, ToolResult, ToolSpec, TurnInput, TurnOutput,
};
pub use engines::NoopAutomatonEngine;
pub use flow::run_agentic_loop;
pub use interface::AutomatonEngine;
pub use model::{
    AutomatonContext, AutomatonError, AutomatonExecutionOutput, AutomatonObjective, AutomatonPlan,
    AutomatonResult,
};
pub use registry::AutomatonEngineRegistry;
pub use repl::{
    check_llm, log_llm_status, parse_tool_request, print_running_mechanism, run_interactive,
    run_interactive_with_llm, run_one_turn, strip_thinking,
};
pub use tools::{
    LsTool, PwdTool, PythonTool, Tool, ToolRegistry, default_registry, enabled_auto_commands,
    parse_run_line,
};
