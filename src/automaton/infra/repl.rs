//! Interactive automaton REPL: one objective per message, plan → execute, optional LLM + tools.

use std::io::{self, Write};
use std::path::Path;
use std::sync::Arc;

use serde_json::json;

use super::interface::AutomatonEngine;
use super::model::{AutomatonContext, AutomatonExecutionOutput, AutomatonError, AutomatonObjective, AutomatonPlan};
use super::shell_tools;
use super::llm_interface::{LlmCall, Message, MessageRole, ToolDef};

const LOG_PREFIX: &str = "[Zebtune]";

/// Logs a line to stderr with prefix and newline (public for binary startup messages).
pub fn log_llm_status(line: &str) {
    let _ = io::stderr().write_all(LOG_PREFIX.as_bytes());
    let _ = io::stderr().write_all(b" ");
    let _ = io::stderr().write_all(line.as_bytes());
    let _ = io::stderr().write_all(b"\n");
    let _ = io::stderr().flush();
}

fn log(line: &str) {
    let _ = io::stderr().write_all(LOG_PREFIX.as_bytes());
    let _ = io::stderr().write_all(b" ");
    let _ = io::stderr().write_all(line.as_bytes());
    let _ = io::stderr().write_all(b"\n");
    let _ = io::stderr().flush();
}

/// Prints the running mechanism (what happens each turn).
pub fn print_running_mechanism() {
    let _ = io::stderr().write_all(b"\n");
    log("Running mechanism:");
    log("  1. You send a message → it becomes one objective (goal).");
    log("  2. I generate a plan (ordered steps) and stick to it until finish.");
    log("  3. I execute each step; this log shows current action and trace.");
    log("  4. I reply with the result (and you can keep chatting).");
    log("");
    log("Commands: /quit or /exit to stop. Empty line also exits.");
    log("----------------------------------------");
}

/// Runs one turn: objective → plan → execute with live log → return output.
pub fn run_one_turn(
    engine: &dyn AutomatonEngine,
    objective: &AutomatonObjective,
    ctx: &AutomatonContext,
) -> Result<AutomatonExecutionOutput, AutomatonError> {
    log("Planning...");
    let plan = engine.plan(objective, ctx)?;
    log_plan(&plan);

    log("Executing plan (stick until finish)...");
    for (i, step) in plan.steps.iter().enumerate() {
        let n = i + 1;
        let total = plan.steps.len();
        log(&format!("  Step {}/{}: {}", n, total, step));
    }

    let out = engine.execute(&plan, ctx)?;
    log_trace(&out.trace);
    log(&format!("Done. Result: {:?}", out.result));
    Ok(out)
}

fn log_plan(plan: &AutomatonPlan) {
    log(&format!("Plan ({} steps):", plan.steps.len()));
    for (i, step) in plan.steps.iter().enumerate() {
        log(&format!("  {}. {}", i + 1, step));
    }
}

fn log_trace(trace: &[String]) {
    if trace.is_empty() {
        return;
    }
    log("Trace:");
    for line in trace {
        log(&format!("  {}", line));
    }
}

/// Strip <think>...</think> and common chain-of-thought markers so the user sees only the answer.
pub fn strip_thinking(text: &str) -> String {
    let mut out = text.trim();
    if let Some(i) = out.find("</think>") {
        out = out[i + 7..].trim_start();
    }
    if let Some(i) = out.find("<think>") {
        out = out[i + 7..].trim_start();
    }
    for marker in [
        "We could say:",
        "Thus answer:",
        "So we could say:",
        "Thus final answer:",
    ] {
        if let Some(i) = out.find(marker) {
            out = out[i + marker.len()..].trim_start();
        }
    }
    out.to_string()
}

/// One-shot LLM connectivity check: sends a simple message and verifies a non-empty response.
pub async fn check_llm(client: &dyn LlmCall) -> Result<(), String> {
    let messages = vec![Message {
        role: MessageRole::User,
        content: "Reply with exactly: OK".into(),
    }];
    let text = client.call(messages).await?;
    if text.trim().is_empty() {
        return Err("LLM returned empty content".to_string());
    }
    Ok(())
}

/// Interactive REPL (sync): engine only, no LLM.
pub fn run_interactive(engine: Arc<dyn AutomatonEngine>) -> io::Result<()> {
    run_interactive_with_llm(engine, None)
}

/// Interactive REPL: read line from stdin; if LLM set, chat with native tool calling; else engine plan+execute.
pub fn run_interactive_with_llm(
    engine: Arc<dyn AutomatonEngine>,
    llm: Option<Arc<dyn LlmCall>>,
) -> io::Result<()> {
    let mut run_counter: u64 = 0;
    let owner = "user".to_string();
    let project = "repl".to_string();
    let step_budget = 100u32;

    let rt = tokio::runtime::Runtime::new()
        .map_err(|e| io::Error::new(io::ErrorKind::Other, format!("tokio runtime: {}", e)))?;

    loop {
        let _ = io::stdout().write_all(b"\nYou> ");
        let _ = io::stdout().flush();
        let mut line = String::new();
        if io::stdin().read_line(&mut line)? == 0 {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            log("Empty input. Exiting.");
            break;
        }
        if line == "/quit" || line == "/exit" {
            log("Bye.");
            break;
        }

        run_counter += 1;
        let run_id = format!("run-{}", run_counter);
        log("---");

        if let Some(ref llm) = llm {
            let registry = shell_tools::default_registry();
            let enabled = shell_tools::enabled_auto_commands();
            let work_dir = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());

            // Build auto-context if enabled
            let user_content = if enabled.is_empty() {
                line.to_string()
            } else {
                log("Running auto-commands (context for LLM)...");
                let ctx = registry.run_auto(&enabled, &work_dir);
                for name in &enabled {
                    log(&format!("  ran: {}", name));
                }
                format!(
                    "Context from your environment:\n```\n{}\n```\n\nUser message: {}",
                    ctx.trim(),
                    line
                )
            };

            // Build tool definitions from registry
            let tool_defs: Vec<ToolDef> = registry.tool_names().into_iter().map(|name| {
                let desc = registry.get(&name)
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
            }).collect();

            let system = "You are Zebtune. Reply concisely. Use tools when helpful.".to_string();
            let mut messages: Vec<serde_json::Value> = vec![
                json!({ "role": "system", "content": system }),
                json!({ "role": "user", "content": user_content }),
            ];

            log("Calling LLM...");
            let max_steps = 5u32;
            let mut final_text = String::new();

            'turn: for _ in 0..max_steps {
                match rt.block_on(llm.call_with_tools(messages.clone(), &tool_defs)) {
                    Ok(crate::automaton::infra::llm_interface::CallResult::Text(text)) => {
                        final_text = strip_thinking(&text);
                        break 'turn;
                    }
                    Ok(crate::automaton::infra::llm_interface::CallResult::ToolCalls(calls)) => {
                        let tool_calls_json: Vec<serde_json::Value> = calls.iter().map(|tc| json!({
                            "id": tc.id,
                            "type": "function",
                            "function": { "name": tc.name, "arguments": tc.arguments }
                        })).collect();
                        messages.push(json!({
                            "role": "assistant",
                            "content": null,
                            "tool_calls": tool_calls_json,
                        }));
                        for tc in &calls {
                            let args: serde_json::Value =
                                serde_json::from_str(&tc.arguments).unwrap_or(json!({}));
                            log(&format!("Tool call: {}", tc.name));
                            let result = match registry.run_tool(&tc.name, &args, &work_dir) {
                                Some(Ok(out)) => out,
                                Some(Err(e)) => format!("Tool error: {}", e),
                                None => format!("Unknown tool: {}", tc.name),
                            };
                            messages.push(json!({
                                "role": "tool",
                                "tool_call_id": tc.id,
                                "content": result,
                            }));
                        }
                    }
                    Err(e) => {
                        final_text = format!("LLM error: {}", e);
                        break 'turn;
                    }
                }
            }

            let _ = io::stdout().write_all(b"\nZebtune> ");
            let _ = io::stdout().write_all(final_text.trim().as_bytes());
            let _ = io::stdout().write_all(b"\n");
            let _ = io::stdout().flush();
        } else {
            let objective = AutomatonObjective {
                objective_id: run_id.clone(),
                goal: line.to_string(),
                input: serde_json::Value::Null,
            };
            let ctx = AutomatonContext {
                owner: owner.clone(),
                project: project.clone(),
                run_id: run_id.clone(),
                step_budget,
                metadata: serde_json::Value::Null,
            };
            match run_one_turn(engine.as_ref(), &objective, &ctx) {
                Ok(out) => {
                    let _ = io::stdout().write_all(b"\nZebtune> ");
                    let reply = out
                        .output
                        .get("executed_steps")
                        .and_then(serde_json::Value::as_u64)
                        .map(|n| format!("Completed {} step(s).", n))
                        .unwrap_or_else(|| serde_json::to_string(&out.output).unwrap_or_default());
                    let _ = io::stdout().write_all(reply.as_bytes());
                    let _ = io::stdout().write_all(b"\n");
                    let _ = io::stdout().flush();
                }
                Err(e) => {
                    log(&format!("Error: {}", e));
                    let _ = io::stdout().write_all(b"\nZebtune> ");
                    let _ = io::stdout().write_all(format!("Error: {}", e).as_bytes());
                    let _ = io::stdout().write_all(b"\n");
                    let _ = io::stdout().flush();
                }
            }
        }
    }

    Ok(())
}
