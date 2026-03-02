//! Agentic contract: types for LLM-in-the-loop with tools.
//!
//! Defines what is exposed to the agent (tool specs), what the agent returns
//! (content + optional tool calls), and what the executor feeds back (tool results).
//! No concrete tools here — hosts (Zebtune, pipeline) register tools and implement
//! the execution side.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Tool spec exposed to the LLM (name, description, parameters schema).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSpec {
    pub name: String,
    pub description: String,
    /// JSON Schema or simple object; e.g. {"type":"object","properties":{"path":{"type":"string"}}}
    #[serde(default)]
    pub parameters: Value,
}

/// Conversation message role.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

/// One message in the agentic conversation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    /// Set when role is Tool: which tool call this result belongs to.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// Set when role is Assistant and the model requested tool calls.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
}

/// A single tool call requested by the model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: Value,
}

/// Result of executing one tool call (fed back into the conversation).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    pub id: String,
    pub name: String,
    /// Output string or error message.
    pub content: String,
}

/// Input for one LLM turn: current messages and available tool specs.
#[derive(Debug, Clone)]
pub struct TurnInput {
    pub messages: Vec<Message>,
    pub tool_specs: Vec<ToolSpec>,
    pub step_budget_remaining: u32,
}

/// Output from one LLM turn: content (reasoning/plan/answer) and optional tool calls.
#[derive(Debug, Clone)]
pub struct TurnOutput {
    /// Text: chain-of-thought, plan, or final answer.
    pub content: String,
    /// If non-empty, executor runs these and appends results; then another turn.
    pub tool_calls: Vec<ToolCall>,
}

/// Result of the full agentic run (after loop exits).
#[derive(Debug, Clone)]
pub struct AgenticRunResult {
    /// Final assistant content when loop ended without tool calls.
    pub final_content: String,
    /// Whether we stopped due to budget (true) or natural finish (false).
    pub budget_exhausted: bool,
    /// Ordered trace entries for audit (e.g. "turn_1", "tool_ls", "turn_2").
    pub trace: Vec<String>,
}
