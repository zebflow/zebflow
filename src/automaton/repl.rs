//! Interactive automaton REPL: one objective per message, plan → execute, optional LLM + tools.

use std::io::{self, Write};
use std::path::Path;
use std::sync::Arc;

use super::model::{AutomatonContext, AutomatonExecutionOutput, AutomatonObjective, AutomatonPlan};
use super::tools;
use super::{AutomatonEngine, AutomatonError};
use crate::llm::{LlmClient, LlmError, LlmMessage, LlmRole};

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

/// Parse first "RUN: name [args]" line from LLM response. Returns (name, args) if allowlisted.
pub fn parse_tool_request(text: &str, allowed: &[String]) -> Option<(String, serde_json::Value)> {
    for line in text.lines() {
        if let Some(pair) = tools::parse_run_line(line, allowed) {
            return Some(pair);
        }
    }
    None
}

/// Strip <think>...</think> and content after "We could say:" so the user sees only the final answer.
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

/// One-shot LLM check: send "Reply with exactly: OK" and verify non-empty response.
pub async fn check_llm(client: &dyn LlmClient) -> Result<(), LlmError> {
    let messages = [LlmMessage {
        role: LlmRole::User,
        content: "Reply with exactly: OK".into(),
    }];
    let res = client.chat(&messages).await?;
    if res.content.trim().is_empty() {
        return Err(LlmError {
            message: "LLM returned empty content".into(),
        });
    }
    Ok(())
}

/// Interactive REPL (sync): engine only, no LLM.
pub fn run_interactive(engine: Arc<dyn AutomatonEngine>) -> io::Result<()> {
    run_interactive_with_llm(engine, None)
}

/// Interactive REPL: read line from stdin; if LLM set, chat and reply; else engine plan+execute.
pub fn run_interactive_with_llm(
    engine: Arc<dyn AutomatonEngine>,
    llm: Option<Arc<dyn LlmClient>>,
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
            let registry = tools::default_registry();
            let enabled = tools::enabled_auto_commands();
            let work_dir = std::env::current_dir().unwrap_or_else(|_| Path::new(".").to_path_buf());
            let user_content = if enabled.is_empty() {
                line.to_string()
            } else {
                log("Running auto-commands (context for LLM)...");
                let ctx = registry.run_auto(&enabled, &work_dir);
                for name in &enabled {
                    log(&format!("  ran: {}", name));
                }
                format!(
                    "Context from your environment (output of allowed commands):\n```\n{}\n```\n\nUser message: {}",
                    ctx.trim(),
                    line
                )
            };
            let tool_list: Vec<String> = registry.tool_names();
            let system_content = format!(
                "You are Zebtune. Reply in 2–4 short sentences. Do not output <think> blocks or 'We could say:'—only the final answer. \
                 You have tools: {}. To run one, output exactly one line: RUN: <name> or RUN: <name> <arg> (e.g. RUN: ls or RUN: ls /tmp). You will then receive the output and should give your final answer.",
                tool_list.join(", ")
            );
            log("Calling LLM...");
            let messages = [
                LlmMessage {
                    role: LlmRole::System,
                    content: system_content.clone(),
                },
                LlmMessage {
                    role: LlmRole::User,
                    content: user_content.clone(),
                },
            ];
            match rt.block_on(llm.chat(&messages)) {
                Ok(res) => {
                    log("Response received.");
                    let final_text = if let Some((name, args)) =
                        parse_tool_request(&res.content, &tool_list)
                    {
                        match registry.run_tool(&name, &args, &work_dir) {
                            Some(Ok(output)) => {
                                log(&format!("Tool run: {} (with args)", name));
                                let follow_up = [
                                    LlmMessage {
                                        role: LlmRole::System,
                                        content: system_content,
                                    },
                                    LlmMessage {
                                        role: LlmRole::User,
                                        content: user_content,
                                    },
                                    LlmMessage {
                                        role: LlmRole::Assistant,
                                        content: res.content.clone(),
                                    },
                                    LlmMessage {
                                        role: LlmRole::User,
                                        content: format!(
                                            "Tool output for {}:\n```\n{}\n```\n\nGive your final answer to the user in 1–3 sentences.",
                                            name, output
                                        ),
                                    },
                                ];
                                match rt.block_on(llm.chat(&follow_up)) {
                                    Ok(r) => strip_thinking(&r.content),
                                    Err(_) => strip_thinking(&res.content),
                                }
                            }
                            Some(Err(e)) => format!("Tool error: {}", e),
                            None => strip_thinking(&res.content),
                        }
                    } else {
                        strip_thinking(&res.content)
                    };
                    let _ = io::stdout().write_all(b"\nZebtune> ");
                    let _ = io::stdout().write_all(final_text.trim().as_bytes());
                    let _ = io::stdout().write_all(b"\n");
                    let _ = io::stdout().flush();
                }
                Err(e) => {
                    log(&format!("LLM error: {}", e));
                    let _ = io::stdout().write_all(b"\nZebtune> ");
                    let _ = io::stdout().write_all(format!("Error: {}", e).as_bytes());
                    let _ = io::stdout().write_all(b"\n");
                    let _ = io::stdout().flush();
                }
            }
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
