//! Zebtune node: run automaton (LLM + tools) internally; output final answer and/or chain (thinking, tool use) with timestamps.
//!
//! Use after a one-time trigger (e.g. Telegram). Output mode: `full` (default) = final answer + chain
//! (each step: short description + time); `final_only` = final answer only.

use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::automaton::tools;
use crate::automaton::{parse_tool_request, strip_thinking};
use crate::pipeline::model::StepEvent;
use crate::pipeline::{
    PipelineError, NodeDefinition,
    nodes::{NodeHandler, NodeExecutionInput, NodeExecutionOutput},
};
use crate::automaton::llm::{LlmClient, LlmMessage, LlmRole};

pub const NODE_KIND: &str = "n.ai.zebtune";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

/// Output mode: full = final_content + chain (steps with time); final_only = final_content only.
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum OutputMode {
    /// Final answer plus chain (thinking, tool_call, tool_result) with timestamps. Default.
    #[default]
    Full,
    /// Only final_content (no chain).
    FinalOnly,
}

/// One step in the chain: short description + time (elapsed HH:MM:SS from node start).
#[derive(Debug, Clone, Serialize)]
pub struct ChainStep {
    pub step: String,
    pub description: String,
    pub at: String,
}

fn format_elapsed(instant: Instant) -> String {
    let s = instant.elapsed().as_secs();
    let h = s / 3600;
    let m = (s % 3600) / 60;
    let s = s % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}

/// Push step to chain and stream to step_tx when present (each step emitted as it happens).
fn emit_step(
    chain: &mut Vec<ChainStep>,
    step_tx: &Option<tokio::sync::mpsc::UnboundedSender<StepEvent>>,
    step: &str,
    description: &str,
    at: &str,
) {
    chain.push(ChainStep {
        step: step.to_string(),
        description: description.to_string(),
        at: at.to_string(),
    });
    if let Some(tx) = step_tx {
        let _ = tx.send(StepEvent {
            step: step.to_string(),
            description: description.to_string(),
            at: at.to_string(),
        });
    }
}

/// Unified node-definition metadata for `n.ai.zebtune`.
pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Zebtune".to_string(),
        description: "Run automaton (LLM + tools); output final answer and/or chain (thinking, tool use) with timestamps. Use after a one-time trigger (e.g. Telegram).".to_string(),
        input_schema: serde_json::json!({
            "type":"object",
            "description":"Trigger payload. Goal taken from message, body, text, or query."
        }),
        output_schema: serde_json::json!({
            "type":"object",
            "properties": {
                "final_content": { "type": "string" },
                "chain": { "type": "array", "items": { "type": "object", "properties": { "step": {}, "description": {}, "at": {} } } },
                "budget_exhausted": { "type": "boolean" },
                "trace": { "type": "array", "items": { "type": "string" } }
            }
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: Default::default(),
        ai_tool: Default::default(),
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// Max LLM+tool rounds. Default 10.
    #[serde(default)]
    pub step_budget: u32,
    /// Optional system prompt override.
    pub system_prompt: Option<String>,
    /// full = final + chain with timestamps (default); final_only = final answer only.
    #[serde(default)]
    pub output_mode: OutputMode,
}

pub struct Node {
    config: Config,
    llm: Option<Arc<dyn LlmClient>>,
}

impl Node {
    pub fn new(config: Config, llm: Option<Arc<dyn LlmClient>>) -> Self {
        let step_budget = if config.step_budget == 0 {
            10
        } else {
            config.step_budget
        };
        Self {
            config: Config {
                step_budget,
                ..config
            },
            llm,
        }
    }
}

/// Extract user goal from trigger payload (e.g. Telegram webhook sends message/text/body).
fn goal_from_payload(payload: &Value) -> Option<String> {
    let s = payload
        .get("message")
        .or_else(|| payload.get("body"))
        .or_else(|| payload.get("text"))
        .or_else(|| payload.get("query"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    if s.is_some() {
        return s;
    }
    if let Some(s) = payload.as_str() {
        return Some(s.to_string());
    }
    None
}

#[async_trait]
impl NodeHandler for Node {
    fn kind(&self) -> &'static str {
        NODE_KIND
    }
    fn input_pins(&self) -> &'static [&'static str] {
        &[INPUT_PIN_IN]
    }
    fn output_pins(&self) -> &'static [&'static str] {
        &[OUTPUT_PIN_OUT]
    }

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        if input.input_pin != INPUT_PIN_IN {
            return Err(PipelineError::new(
                "FW_NODE_ZEBTUNE_INPUT_PIN",
                format!("unsupported input pin '{}'", input.input_pin),
            ));
        }

        let goal = goal_from_payload(&input.payload).ok_or_else(|| {
            PipelineError::new(
                "FW_NODE_ZEBTUNE_GOAL",
                "payload must contain message, body, text, query (string), or be a string",
            )
        })?;

        let Some(ref llm) = self.llm else {
            let payload = if self.config.output_mode == OutputMode::Full {
                json!({
                    "final_content": "LLM not configured. Set ZEBTUNE_OPENAI_API_KEY or ZEBTUNE_ANTHROPIC_API_KEY.",
                    "chain": [],
                    "budget_exhausted": false,
                    "trace": ["zebtune_no_llm"]
                })
            } else {
                json!({
                    "final_content": "LLM not configured. Set ZEBTUNE_OPENAI_API_KEY or ZEBTUNE_ANTHROPIC_API_KEY.",
                    "budget_exhausted": false,
                    "trace": ["zebtune_no_llm"]
                })
            };
            return Ok(NodeExecutionOutput {
                output_pins: vec![OUTPUT_PIN_OUT.to_string()],
                payload,
                trace: vec![format!("node_kind={NODE_KIND}"), "no_llm".to_string()],
            });
        };

        let registry = tools::default_registry();
        let tool_list = registry.tool_names();
        let work_dir = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());

        let system_content = self.config.system_prompt.clone().unwrap_or_else(|| {
            format!(
                "You are Zebtune. Reply concisely. Do not output <think> blocks—only the final answer. \
                 You have tools: {}. To run one, output exactly one line: RUN: <name> or RUN: <name> <arg>. \
                 You will then receive the output and should give your final answer.",
                tool_list.join(", ")
            )
        });

        let start = Instant::now();
        let step_tx = input.step_tx.clone();
        let mut trace = vec![format!("node_kind={NODE_KIND}")];
        let mut chain: Vec<ChainStep> = Vec::new();
        let mut budget = self.config.step_budget;
        let mut messages: Vec<LlmMessage> = vec![
            LlmMessage {
                role: LlmRole::System,
                content: system_content.clone(),
            },
            LlmMessage {
                role: LlmRole::User,
                content: goal.clone(),
            },
        ];

        emit_step(
            &mut chain,
            &step_tx,
            "external",
            "Starting Zebtune",
            &format_elapsed(start),
        );

        let final_content = loop {
            if budget == 0 {
                emit_step(
                    &mut chain,
                    &step_tx,
                    "budget_exhausted",
                    "Step budget exhausted.",
                    &format_elapsed(start),
                );
                break "Step budget exhausted.".to_string();
            }
            let res = match llm.chat(&messages).await {
                Ok(r) => r,
                Err(e) => {
                    emit_step(
                        &mut chain,
                        &step_tx,
                        "error",
                        &format!("LLM error: {}", e),
                        &format_elapsed(start),
                    );
                    break format!("LLM error: {}", e);
                }
            };

            let short_thinking = res
                .content
                .trim()
                .lines()
                .next()
                .map(|l| {
                    let s = l.trim();
                    if s.len() > 80 {
                        format!("{}...", &s[..77])
                    } else {
                        s.to_string()
                    }
                })
                .unwrap_or_else(|| "(no content)".to_string());
            emit_step(
                &mut chain,
                &step_tx,
                "thinking",
                &short_thinking,
                &format_elapsed(start),
            );
            trace.push("turn".to_string());
            budget = budget.saturating_sub(1);

            if let Some((name, args)) = parse_tool_request(&res.content, &tool_list) {
                emit_step(
                    &mut chain,
                    &step_tx,
                    "tool_call",
                    &format!("RUN: {} {}", name, args),
                    &format_elapsed(start),
                );
                let tool_out = match registry.run_tool(&name, &args, &work_dir) {
                    Some(Ok(out)) => out,
                    Some(Err(e)) => format!("Tool error: {}", e),
                    None => "Unknown tool.".to_string(),
                };
                let short_out = if tool_out.len() > 120 {
                    format!(
                        "{}...",
                        tool_out.trim().chars().take(117).collect::<String>()
                    )
                } else {
                    tool_out.trim().to_string()
                };
                emit_step(
                    &mut chain,
                    &step_tx,
                    "tool_result",
                    &format!("{}: {}", name, short_out),
                    &format_elapsed(start),
                );
                trace.push(format!("tool:{}", name));
                messages.push(LlmMessage {
                    role: LlmRole::Assistant,
                    content: res.content.clone(),
                });
                messages.push(LlmMessage {
                    role: LlmRole::User,
                    content: format!(
                        "Tool output for {}:\n```\n{}\n```\n\nGive your final answer in 1–3 sentences.",
                        name, tool_out
                    ),
                });
                continue;
            }

            let final_text = strip_thinking(&res.content);
            let trimmed = final_text.trim();
            let short: String = trimmed.chars().take(120).collect();
            let description = if trimmed.chars().count() > 120 {
                format!("{}...", short)
            } else {
                short
            };
            emit_step(
                &mut chain,
                &step_tx,
                "final",
                &description,
                &format_elapsed(start),
            );
            break final_text;
        };

        let include_chain = self.config.output_mode == OutputMode::Full;
        let payload = if include_chain {
            json!({
                "final_content": final_content.trim(),
                "chain": chain,
                "budget_exhausted": budget == 0,
                "trace": trace
            })
        } else {
            json!({
                "final_content": final_content.trim(),
                "budget_exhausted": budget == 0,
                "trace": trace
            })
        };

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload,
            trace: trace.clone(),
        })
    }
}
