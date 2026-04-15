//! Zebtune autonomous agent.
//!
//! The full autonomous agent — strategic planning, tool use, synthesis.
//! Comparable in capability scope to Perplexity, Claude Code, or Cursor:
//! it receives a goal and works autonomously until the goal is achieved.
//!
//! # Architecture
//!
//! ```text
//! ZebtuneAgent::run(goal)
//!   │
//!   ├── Phase 1: Strategic Planning  [TODO M6]
//!   │     LLM decomposes goal → HierarchicalPlan (subgoals + steps)
//!   │
//!   ├── Phase 2: Execution Loop  ✅ native function calling (M7 done)
//!   │     LLM turn → tool_calls → execute via ToolRegistry → feed result → repeat
//!   │     Budget-bounded; emits ChainStep events for streaming UI
//!   │
//!   ├── Phase 3: Validation  [TODO M6]
//!   │     LLM checks if subgoal criteria met → pass/fail
//!   │
//!   ├── Phase 4: Adaptive Replanning  [TODO M6]
//!   │     On failure: LLM generates alternative plan for the failed subgoal
//!   │
//!   └── Phase 5: Synthesis
//!         Final answer assembled from chain + execution trace
//! ```

use std::sync::Arc;
use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::automaton::infra::llm_interface::{CallResult, LlmCall, ToolDef};
use crate::automaton::infra::shell_tools::ToolRegistry;

// ── Types ─────────────────────────────────────────────────────────────────────

/// Output mode for the agent run.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputMode {
    /// Final answer plus full chain (thinking, tool_call, tool_result) with timestamps.
    #[default]
    Full,
    /// Only the final answer — no chain details.
    FinalOnly,
}

/// One step in the execution chain, with elapsed time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainStep {
    pub step: String,
    pub description: String,
    /// Elapsed time from agent start: HH:MM:SS.
    pub at: String,
}

/// Callback invoked after each chain step is emitted. Used by pipeline nodes
/// to stream progress to clients without coupling the agent to pipeline types.
pub type StepCallback = Box<dyn Fn(&ChainStep) + Send + Sync>;

/// Configuration for ZebtuneAgent.
#[derive(Debug, Clone)]
pub struct ZebtuneConfig {
    /// Maximum LLM + tool iterations before stopping. Default: 10.
    pub step_budget: u32,
    /// Optional system prompt override.
    pub system_prompt: Option<String>,
    /// `Full` includes chain details; `FinalOnly` returns just the answer.
    pub output_mode: OutputMode,
}

impl Default for ZebtuneConfig {
    fn default() -> Self {
        Self {
            step_budget: 10,
            system_prompt: None,
            output_mode: OutputMode::Full,
        }
    }
}

/// Result of a Zebtune agent run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZebtuneResult {
    /// The final synthesized answer.
    pub final_content: String,
    /// Full execution chain (only populated when output_mode = Full).
    pub chain: Vec<ChainStep>,
    /// True when the step budget ran out before a natural finish.
    pub budget_exhausted: bool,
    /// Execution trace entries (for logging and audit).
    pub trace: Vec<String>,
}

// ── Agent ─────────────────────────────────────────────────────────────────────

/// Zebtune: full autonomous agent with planning, tool use, and synthesis.
pub struct ZebtuneAgent {
    pub config: ZebtuneConfig,
    pub llm: Option<Arc<dyn LlmCall>>,
}

impl ZebtuneAgent {
    pub fn new(config: ZebtuneConfig, llm: Option<Arc<dyn LlmCall>>) -> Self {
        let step_budget = if config.step_budget == 0 {
            10
        } else {
            config.step_budget
        };
        Self {
            config: ZebtuneConfig {
                step_budget,
                ..config
            },
            llm,
        }
    }

    /// Run the agent: goal → execute → synthesize → result.
    ///
    /// Uses native function calling (OpenAI tools API).
    /// `step_callback` is called synchronously after each chain step.
    pub async fn run(
        &self,
        goal: &str,
        tool_registry: &ToolRegistry,
        step_callback: Option<&StepCallback>,
    ) -> ZebtuneResult {
        let Some(ref llm) = self.llm else {
            return ZebtuneResult {
                final_content:
                    "LLM not configured. Set ZEBTUNE_OPENAI_API_KEY or ZEBTUNE_ANTHROPIC_API_KEY."
                        .to_string(),
                chain: vec![],
                budget_exhausted: false,
                trace: vec!["zebtune_no_llm".to_string()],
            };
        };

        // Build ToolDef list from registry for the LLM
        let tool_defs: Vec<ToolDef> = tool_registry
            .tool_names()
            .into_iter()
            .map(|name| {
                let desc = tool_registry
                    .get(&name)
                    .map(|t| t.description().to_string())
                    .unwrap_or_default();
                ToolDef {
                    name,
                    description: desc,
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "path": { "type": "string", "description": "Optional path argument" }
                        }
                    }),
                }
            })
            .collect();

        let system_content = self.config.system_prompt.clone().unwrap_or_else(|| {
            "You are Zebtune, an autonomous assistant. Use the available tools when helpful. \
             Provide a clear, concise final answer."
                .to_string()
        });

        let work_dir =
            std::env::current_dir().unwrap_or_else(|_| std::path::Path::new(".").to_path_buf());

        let start = Instant::now();
        let mut trace = vec!["agent=zebtune".to_string()];
        let mut chain: Vec<ChainStep> = Vec::new();
        let mut budget = self.config.step_budget;

        // TODO M6: Phase 1 — Strategic Planning
        // let plan = crate::automaton::planning::basic::ZebtunePlanner::decompose(llm, goal).await;

        let mut messages: Vec<Value> = vec![
            json!({ "role": "system", "content": system_content }),
            json!({ "role": "user",   "content": goal }),
        ];

        self.emit(
            &mut chain,
            step_callback,
            "start",
            "Starting Zebtune",
            &format_elapsed(start),
        );

        let final_content = loop {
            if budget == 0 {
                self.emit(
                    &mut chain,
                    step_callback,
                    "budget_exhausted",
                    "Step budget exhausted.",
                    &format_elapsed(start),
                );
                break "Step budget exhausted.".to_string();
            }

            let result = match llm.call_with_tools(messages.clone(), &tool_defs).await {
                Ok(r) => r,
                Err(e) => {
                    let msg = format!("LLM error: {}", e);
                    self.emit(
                        &mut chain,
                        step_callback,
                        "error",
                        &msg,
                        &format_elapsed(start),
                    );
                    break msg;
                }
            };

            trace.push("turn".to_string());
            budget = budget.saturating_sub(1);

            match result {
                CallResult::Text(text) => {
                    let trimmed = crate::automaton::infra::repl::strip_thinking(&text);
                    let short: String = trimmed.chars().take(120).collect();
                    let desc = if trimmed.chars().count() > 120 {
                        format!("{}...", short)
                    } else {
                        short
                    };
                    self.emit(
                        &mut chain,
                        step_callback,
                        "final",
                        &desc,
                        &format_elapsed(start),
                    );
                    break trimmed;
                }

                CallResult::ToolCalls(calls) => {
                    // Emit a "thinking" step summarising what the LLM is doing
                    let thinking_desc = calls
                        .first()
                        .map(|c| format!("calling {}", c.name))
                        .unwrap_or_else(|| "tool use".to_string());
                    self.emit(
                        &mut chain,
                        step_callback,
                        "thinking",
                        &thinking_desc,
                        &format_elapsed(start),
                    );

                    // Append assistant tool_calls message
                    let tool_calls_json: Vec<Value> = calls
                        .iter()
                        .map(|tc| {
                            json!({
                                "id": tc.id,
                                "type": "function",
                                "function": { "name": tc.name, "arguments": tc.arguments }
                            })
                        })
                        .collect();
                    messages.push(json!({
                        "role": "assistant",
                        "content": null,
                        "tool_calls": tool_calls_json,
                    }));

                    // Execute each tool and append results
                    for tc in &calls {
                        let args: Value = serde_json::from_str(&tc.arguments).unwrap_or(json!({}));

                        self.emit(
                            &mut chain,
                            step_callback,
                            "tool_call",
                            &format!("RUN: {} {:?}", tc.name, args),
                            &format_elapsed(start),
                        );
                        trace.push(format!("tool:{}", tc.name));

                        let tool_out = match tool_registry.run_tool(&tc.name, &args, &work_dir) {
                            Some(Ok(out)) => out,
                            Some(Err(e)) => format!("Tool error: {}", e),
                            None => format!("Unknown tool: {}", tc.name),
                        };

                        let short_out = if tool_out.len() > 200 {
                            format!(
                                "{}...",
                                tool_out.trim().chars().take(197).collect::<String>()
                            )
                        } else {
                            tool_out.trim().to_string()
                        };
                        self.emit(
                            &mut chain,
                            step_callback,
                            "tool_result",
                            &format!("{}: {}", tc.name, short_out),
                            &format_elapsed(start),
                        );

                        messages.push(json!({
                            "role": "tool",
                            "tool_call_id": tc.id,
                            "content": tool_out,
                        }));
                    }
                }
            }
        };

        // TODO M6: Phase 3+4 — Validation + Replanning

        ZebtuneResult {
            final_content: final_content.trim().to_string(),
            chain: if self.config.output_mode == OutputMode::FinalOnly {
                vec![]
            } else {
                chain
            },
            budget_exhausted: budget == 0,
            trace,
        }
    }

    fn emit(
        &self,
        chain: &mut Vec<ChainStep>,
        callback: Option<&StepCallback>,
        step: &str,
        description: &str,
        at: &str,
    ) {
        let s = ChainStep {
            step: step.to_string(),
            description: description.to_string(),
            at: at.to_string(),
        };
        if let Some(cb) = callback {
            cb(&s);
        }
        chain.push(s);
    }
}

fn format_elapsed(instant: Instant) -> String {
    let s = instant.elapsed().as_secs();
    let h = s / 3600;
    let m = (s % 3600) / 60;
    let s = s % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}
