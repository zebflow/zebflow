//! Simple tool-calling agent.
//!
//! Receives a goal and a list of tool definitions, calls the LLM with native
//! function calling (OpenAI tools API), executes each returned tool call,
//! feeds results back, and repeats until the model produces a final answer.
//!
//! # Key distinction from ZebtuneAgent
//!
//! - **No strategic planning phase** — single-pass tool sequence execution
//! - **No replanning** — if a tool fails, the result is fed back as-is
//! - **Uses native function calling** (ToolDef → LLM tool_calls) rather than text parsing
//! - Suitable for: deterministic task completion where the tools are known upfront
//!
//! # Usage (pipeline node)
//!
//! The `n.ai.agent.tool_caller` pipeline node constructs this agent.
//! See `src/pipeline/nodes/basic/tool_caller_agent.rs`.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::automaton::infra::llm_interface::{CallResult, LlmCall, ToolDef};

// ── Types ─────────────────────────────────────────────────────────────────────

/// Configuration for ToolCallerAgent.
#[derive(Debug, Clone)]
pub struct ToolCallerConfig {
    /// System instructions for the LLM. Default: generic helpful assistant.
    pub system_prompt: Option<String>,
    /// Maximum agentic iterations before stopping. Default: 5.
    pub max_iterations: u32,
}

impl Default for ToolCallerConfig {
    fn default() -> Self {
        Self {
            system_prompt: None,
            max_iterations: 5,
        }
    }
}

/// Result of a ToolCallerAgent run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallerResult {
    /// Final text response from the LLM.
    pub response: String,
    /// Names of tools that were called during execution.
    pub tools_called: Vec<String>,
    /// Number of LLM iterations used.
    pub iterations: u32,
}

// ── Agent ─────────────────────────────────────────────────────────────────────

/// Simple tool-calling agent: single-pass structured tool sequence executor.
///
/// The caller supplies:
/// - `tools`: `Vec<ToolDef>` describing available tools (name, description, JSON schema)
/// - `executor`: a closure `(tool_name, arguments_json) -> Result<String, String>`
///   that executes the tool and returns its output
///
/// The agent handles the LLM → tool_calls → execute → feed back loop.
pub struct ToolCallerAgent {
    config: ToolCallerConfig,
    llm: Arc<dyn LlmCall>,
}

impl ToolCallerAgent {
    pub fn new(config: ToolCallerConfig, llm: Arc<dyn LlmCall>) -> Self {
        let max_iterations = if config.max_iterations == 0 {
            5
        } else {
            config.max_iterations
        };
        Self {
            config: ToolCallerConfig {
                max_iterations,
                ..config
            },
            llm,
        }
    }

    /// Run the agent: query → structured tool calls → execute → final answer.
    ///
    /// `executor` is called for each tool call the LLM makes.
    /// It receives `(tool_name, arguments_json_str)` and returns `Ok(output)` or `Err(msg)`.
    pub async fn run(
        &self,
        query: &str,
        tools: Vec<ToolDef>,
        executor: impl Fn(&str, &str) -> Result<String, String>,
    ) -> Result<ToolCallerResult, String> {
        let system = self
            .config
            .system_prompt
            .clone()
            .unwrap_or_else(|| "You are a helpful AI assistant.".to_string());

        let mut messages: Vec<Value> = vec![
            json!({ "role": "system", "content": system }),
            json!({ "role": "user",   "content": query }),
        ];

        let mut tools_called: Vec<String> = Vec::new();
        let mut iteration = 0u32;
        let mut final_response = String::new();

        loop {
            iteration += 1;
            if iteration > self.config.max_iterations {
                return Err(format!(
                    "ToolCallerAgent exceeded max_iterations ({})",
                    self.config.max_iterations
                ));
            }

            let result = self.llm.call_with_tools(messages.clone(), &tools).await?;

            match result {
                CallResult::Text(text) => {
                    final_response = text;
                    break;
                }
                CallResult::ToolCalls(calls) => {
                    if calls.is_empty() {
                        break;
                    }

                    // Append assistant message with tool_calls
                    let tool_calls_json: Vec<Value> = calls
                        .iter()
                        .map(|tc| {
                            json!({
                                "id": tc.id,
                                "type": "function",
                                "function": {
                                    "name": tc.name,
                                    "arguments": tc.arguments,
                                }
                            })
                        })
                        .collect();
                    messages.push(json!({
                        "role": "assistant",
                        "content": null,
                        "tool_calls": tool_calls_json,
                    }));

                    // Execute each tool and append its result
                    for call in &calls {
                        tools_called.push(call.name.clone());
                        let tool_result = match executor(&call.name, &call.arguments) {
                            Ok(out) => out,
                            Err(e) => format!("Tool error: {}", e),
                        };
                        messages.push(json!({
                            "role": "tool",
                            "tool_call_id": call.id,
                            "content": tool_result,
                        }));
                    }
                }
            }
        }

        Ok(ToolCallerResult {
            response: final_response,
            tools_called,
            iterations: iteration,
        })
    }
}
