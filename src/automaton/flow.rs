//! Agentic flow: think → plan → act (tool calls) → observe → repeat until done or budget.
//!
//! Single loop that chains LLM turns with tool execution. The host supplies
//! the turn function (LLM) and the tool executor (allowlisted only).

use super::contract::{
    AgenticRunResult, Message, MessageRole, ToolCall, ToolResult, ToolSpec, TurnInput, TurnOutput,
};

/// Runs the agentic loop until the model returns no tool calls or the step budget is exhausted.
///
/// - **turn**: given current messages and tool specs, returns content (reasoning/plan/answer) and optional tool_calls.
/// - **execute**: runs the allowlisted tools and returns results (same order as tool_calls).
///
/// Each iteration: turn → if tool_calls then execute → append assistant message + tool result messages → repeat.
pub fn run_agentic_loop<E>(
    mut messages: Vec<Message>,
    tool_specs: Vec<ToolSpec>,
    step_budget: u32,
    mut turn: impl FnMut(TurnInput) -> Result<TurnOutput, E>,
    execute: impl Fn(&[ToolCall]) -> Vec<ToolResult>,
) -> Result<AgenticRunResult, E> {
    let mut trace = Vec::new();
    let mut budget = step_budget;

    loop {
        if budget == 0 {
            return Ok(AgenticRunResult {
                final_content: String::new(),
                budget_exhausted: true,
                trace,
            });
        }

        let input = TurnInput {
            messages: messages.clone(),
            tool_specs: tool_specs.clone(),
            step_budget_remaining: budget,
        };
        let output = turn(input)?;
        trace.push("turn".to_string());

        if output.tool_calls.is_empty() {
            return Ok(AgenticRunResult {
                final_content: output.content,
                budget_exhausted: false,
                trace,
            });
        }

        trace.push("tools".to_string());
        let results = execute(&output.tool_calls);
        budget = budget.saturating_sub(1);

        // Append assistant message (content + tool_calls) then one Tool message per result.
        messages.push(Message {
            role: MessageRole::Assistant,
            content: output.content,
            tool_call_id: None,
            tool_calls: Some(output.tool_calls.clone()),
        });
        for r in results {
            messages.push(Message {
                role: MessageRole::Tool,
                content: r.content,
                tool_call_id: Some(r.id),
                tool_calls: None,
            });
        }
    }
}
