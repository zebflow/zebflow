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
//!   │     LLM turn → tool_calls → execute via caller-provided executor → feed result → repeat
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

/// Semantic verifier: `(candidate, goal) -> (pass, reason)`.
///
/// Where the JSON success contract checks *shape*, the verifier checks *truth*:
/// it may run real work (a query, a compile, an LLM judge) to decide whether the
/// answer is actually correct. Typically backed by a function pipeline supplied
/// by the host. `reason` is fed back verbatim to drive repair on failure.
///
/// Kept as an opaque closure so the agent stays decoupled from the platform —
/// the same design as the tool `executor`.
pub type VerifyFn = dyn Fn(&str, &str) -> (bool, String) + Send + Sync;

/// Configuration for ZebtuneAgent.
#[derive(Debug, Clone)]
pub struct ZebtuneConfig {
    /// Maximum LLM + tool iterations before stopping. Default: 10.
    pub step_budget: u32,
    /// Optional system prompt override.
    pub system_prompt: Option<String>,
    /// `Full` includes chain details; `FinalOnly` returns just the answer.
    pub output_mode: OutputMode,
    /// Optional success contract (JSON Schema subset, see
    /// [`crate::automaton::agents::contract`]). When set, the final answer must
    /// satisfy it; otherwise the agent does not break on the first text reply
    /// and instead feeds the failure back and retries (up to `max_repairs`).
    /// `None` preserves the original behaviour exactly.
    pub success_schema: Option<Value>,
    /// Maximum repair attempts when the success contract fails. `0` = no repair
    /// (the candidate is accepted but marked unverified). Repairs are also
    /// bounded by `step_budget`, whichever limit is reached first.
    pub max_repairs: u32,
}

impl Default for ZebtuneConfig {
    fn default() -> Self {
        Self {
            step_budget: 10,
            system_prompt: None,
            output_mode: OutputMode::Full,
            success_schema: None,
            max_repairs: 0,
        }
    }
}

/// Per-run metrics — the benchmark meter. All counts are observed locally
/// during the run; token counts are summed from the provider's `usage` blocks
/// and are `0` when the provider does not report them (never estimated).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunMetrics {
    /// Number of model turns (LLM calls).
    pub llm_calls: u32,
    /// Total tool invocations across all turns.
    pub tool_calls: u32,
    /// Number of verifier-pipeline invocations (Phase B).
    pub verify_calls: u32,
    /// Repair attempts performed (mirrors `ZebtuneResult::repairs_used`).
    pub repairs_used: u32,
    /// Summed prompt tokens (provider-reported; 0 if unavailable).
    pub prompt_tokens: u64,
    /// Summed completion tokens (provider-reported; 0 if unavailable).
    pub completion_tokens: u64,
    /// Wall-clock duration of the run, milliseconds.
    pub wall_ms: u64,
    /// Whether the step budget was exhausted.
    pub budget_exhausted: bool,
    /// Why the run ended: "ok" | "verified" | "repairs_exhausted" |
    /// "budget_exhausted" | "error" | "no_llm".
    pub stop_reason: String,
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
    /// True when the final answer satisfied the success contract (or no
    /// contract was set). False when a contract was set but never satisfied
    /// (e.g. repair attempts were exhausted, or the budget ran out first).
    pub verified: bool,
    /// Number of repair attempts performed because the contract was not yet met.
    pub repairs_used: u32,
    /// Per-run metrics (cost/timing/stop reason) for benchmarking.
    pub metrics: RunMetrics,
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

    /// Run the agent: goal → execute → verify → (repair) → synthesize → result.
    ///
    /// Uses native function calling (OpenAI tools API).
    /// `tool_defs` describes which tools the LLM may call.
    /// `executor` resolves each tool call: `(name, args_json) -> Result<output, error>`.
    /// `verifier` is an optional semantic check applied to the final answer
    /// (Phase B). It runs *after* the cheap JSON `success_schema` check passes,
    /// so a malformed answer never wastes a verifier call. `None` disables it.
    /// `step_callback` is called synchronously after each chain step.
    pub async fn run(
        &self,
        goal: &str,
        tool_defs: Vec<ToolDef>,
        executor: impl Fn(&str, &str) -> Result<String, String>,
        verifier: Option<&VerifyFn>,
        step_callback: Option<&StepCallback>,
    ) -> ZebtuneResult {
        let Some(ref llm) = self.llm else {
            return ZebtuneResult {
                final_content:
                    "LLM not configured. Set ZEBTUNE_OPENAI_API_KEY or ZEBTUNE_ANTHROPIC_API_KEY."
                        .to_string(),
                chain: vec![],
                budget_exhausted: false,
                verified: false,
                repairs_used: 0,
                metrics: RunMetrics {
                    stop_reason: "no_llm".to_string(),
                    ..Default::default()
                },
                trace: vec!["zebtune_no_llm".to_string()],
            };
        };

        let system_content = self.config.system_prompt.clone().unwrap_or_else(|| {
            "You are Zebtune, an autonomous assistant. Use the available tools when helpful. \
             Provide a clear, concise final answer."
                .to_string()
        });

        let start = Instant::now();
        let mut trace = vec!["agent=zebtune".to_string()];
        let mut chain: Vec<ChainStep> = Vec::new();
        let mut budget = self.config.step_budget;
        // VERIFY/REPAIR state. `verified` starts true only when there is no
        // contract to satisfy — neither a JSON schema (shape) nor a verifier
        // (truth) — so the no-contract path is byte-for-byte the original
        // behaviour.
        let has_contract = self.config.success_schema.is_some() || verifier.is_some();
        let mut repairs_used: u32 = 0;
        let mut verified: bool = !has_contract;

        // Metric counters (the benchmark meter).
        let mut llm_calls: u32 = 0;
        let mut tool_calls: u32 = 0;
        let mut verify_calls: u32 = 0;
        let mut prompt_tokens: u64 = 0;
        let mut completion_tokens: u64 = 0;
        let mut stop_reason = String::from("ok");

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
                stop_reason = "budget_exhausted".to_string();
                self.emit(
                    &mut chain,
                    step_callback,
                    "budget_exhausted",
                    "Step budget exhausted.",
                    &format_elapsed(start),
                );
                break "Step budget exhausted.".to_string();
            }

            let (result, usage) = match llm.call_with_tools(messages.clone(), &tool_defs).await {
                Ok(r) => r,
                Err(e) => {
                    stop_reason = "error".to_string();
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

            llm_calls += 1;
            prompt_tokens += usage.prompt_tokens;
            completion_tokens += usage.completion_tokens;
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

                    // VERIFY gate. When a contract is configured (a JSON schema,
                    // a verifier, or both), the model's "I'm done" is not trusted
                    // until it passes. Checks run cheap-first: the deterministic
                    // JSON schema (shape) before the verifier (truth), so a
                    // malformed answer never wastes a verifier call. On failure we
                    // REPAIR (feed reasons back, loop) until repairs run out, then
                    // accept best effort marked unverified.
                    if has_contract {
                        let mut reasons: Vec<String> = Vec::new();

                        // Layer 1 — shape (deterministic, free).
                        if let Some(schema) = &self.config.success_schema {
                            match crate::automaton::agents::contract::extract_json(&trimmed) {
                                Some(candidate) => {
                                    let verdict =
                                        crate::automaton::agents::contract::check_contract(
                                            &candidate, schema,
                                        );
                                    if !verdict.pass {
                                        reasons.extend(verdict.reasons);
                                    }
                                }
                                None => reasons.push(
                                    "output is not valid JSON and a JSON success contract is \
                                     required"
                                        .to_string(),
                                ),
                            }
                        }

                        // Layer 2 — truth (verifier pipeline). Only run when the
                        // shape is already valid, to avoid spending a verifier
                        // call on output that is obviously malformed.
                        if reasons.is_empty() {
                            if let Some(vf) = verifier {
                                verify_calls += 1;
                                self.emit(
                                    &mut chain,
                                    step_callback,
                                    "verify",
                                    "running verifier",
                                    &format_elapsed(start),
                                );
                                let (pass, reason) = vf(&trimmed, goal);
                                trace.push(format!("verify:{}", if pass { "pass" } else { "fail" }));
                                if !pass {
                                    reasons.push(if reason.trim().is_empty() {
                                        "verifier rejected the answer".to_string()
                                    } else {
                                        reason
                                    });
                                }
                            }
                        }

                        if reasons.is_empty() {
                            verified = true;
                            stop_reason = "verified".to_string();
                            self.emit(
                                &mut chain,
                                step_callback,
                                "final",
                                &desc,
                                &format_elapsed(start),
                            );
                            break trimmed;
                        } else if repairs_used < self.config.max_repairs {
                            repairs_used += 1;
                            let joined = reasons.join("; ");
                            self.emit(
                                &mut chain,
                                step_callback,
                                "repair",
                                &format!("contract failed, repair {}: {}", repairs_used, joined),
                                &format_elapsed(start),
                            );
                            trace.push(format!("repair:{}", repairs_used));
                            // Feed the rejected candidate and the concrete
                            // reasons back so the next turn can correct itself.
                            messages.push(json!({
                                "role": "assistant",
                                "content": trimmed,
                            }));
                            messages.push(json!({
                                "role": "user",
                                "content": format!(
                                    "Your previous answer did not satisfy the required output \
                                     contract. Problems: {}. Return a corrected answer that \
                                     resolves every problem. Output only the required structure.",
                                    joined
                                ),
                            }));
                            continue;
                        } else {
                            // Out of repairs: accept best effort, mark unverified.
                            verified = false;
                            stop_reason = "repairs_exhausted".to_string();
                            self.emit(
                                &mut chain,
                                step_callback,
                                "final_unverified",
                                &desc,
                                &format_elapsed(start),
                            );
                            trace.push("contract_unsatisfied".to_string());
                            break trimmed;
                        }
                    }

                    // No contract — original behaviour.
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
                    tool_calls += calls.len() as u32;
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
                        self.emit(
                            &mut chain,
                            step_callback,
                            "tool_call",
                            &format!("RUN: {} {}", tc.name, tc.arguments),
                            &format_elapsed(start),
                        );
                        trace.push(format!("tool:{}", tc.name));

                        let tool_out = match executor(&tc.name, &tc.arguments) {
                            Ok(out) => out,
                            Err(e) => format!("Tool error: {}", e),
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

        let budget_exhausted = budget == 0;
        let metrics = RunMetrics {
            llm_calls,
            tool_calls,
            verify_calls,
            repairs_used,
            prompt_tokens,
            completion_tokens,
            wall_ms: start.elapsed().as_millis() as u64,
            budget_exhausted,
            stop_reason,
        };

        ZebtuneResult {
            final_content: final_content.trim().to_string(),
            chain: if self.config.output_mode == OutputMode::FinalOnly {
                vec![]
            } else {
                chain
            },
            budget_exhausted,
            verified,
            repairs_used,
            metrics,
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
