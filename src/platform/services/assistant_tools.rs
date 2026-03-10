//! Platform-aware tools for the project assistant agentic loop.

use std::sync::Arc;

use serde_json::{Value, json};

use crate::automaton::llm_interface::ToolDef;
use crate::platform::services::PlatformService;

/// Result of executing a tool — text answer plus optional browser interaction sequence.
pub struct ToolRunResult {
    /// Human-readable result shown in the chat tool bubble.
    pub text: String,
    /// If present, emitted as `interaction_sequence` SSE event for browser automation.
    pub interaction: Option<Value>,
    /// If present, browser navigates to this URL after the tool call.
    pub navigate: Option<String>,
}

impl ToolRunResult {
    fn text(s: impl Into<String>) -> Self {
        Self { text: s.into(), interaction: None, navigate: None }
    }
}

/// Platform-aware tool runner for the project assistant.
pub struct AssistantPlatformTools {
    platform: Arc<PlatformService>,
    owner: String,
    project: String,
}

impl AssistantPlatformTools {
    pub fn new(platform: Arc<PlatformService>, owner: &str, project: &str) -> Self {
        Self {
            platform,
            owner: owner.to_string(),
            project: project.to_string(),
        }
    }

    /// Tool definitions in OpenAI function calling schema format.
    pub fn tool_defs() -> Vec<ToolDef> {
        vec![
            ToolDef {
                name: "execute_pipeline_dsl".to_string(),
                description: "Execute Pipeline DSL commands (get, describe, register, activate, deactivate, execute, run, patch, git, and more). Returns terminal output lines. Use && to chain multiple commands. Read the pipeline-dsl skill for full syntax reference.".to_string(),
                parameters: json!({
                    "type": "object",
                    "properties": {
                        "dsl": {
                            "type": "string",
                            "description": "DSL command string. Supports multiline with \\ continuation and && chaining. Examples: 'get pipelines', 'register my-pipe | trigger.webhook --path /api | pg.query --credential main-db', 'activate pipeline my-pipe && describe pipeline my-pipe'"
                        }
                    },
                    "required": ["dsl"]
                }),
            },
        ]
    }

    /// Execute a named tool that may require async.
    pub async fn run_async(&self, name: &str, args: &Value) -> ToolRunResult {
        match name {
            "execute_pipeline_dsl" => {
                let dsl = args["dsl"].as_str().unwrap_or("");
                let executor = crate::platform::shell::executor::DslExecutor::new(
                    self.platform.clone(),
                    &self.owner,
                    &self.project,
                );
                let output = executor.execute_dsl(dsl).await;
                let text = output
                    .lines
                    .iter()
                    .map(|l| l.text.clone())
                    .collect::<Vec<_>>()
                    .join("\n");
                let navigate = crate::platform::interaction::InteractionEngine::new(
                    &self.owner,
                    &self.project,
                )
                .match_dsl(dsl, output.ok);
                ToolRunResult { text, interaction: None, navigate }
            }
            _ => ToolRunResult::text(format!("Unknown tool: '{name}'")),
        }
    }
}
