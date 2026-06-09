//! `n.ai.agent` pipeline node — unified AI agent with two modes.
//!
//! # Modes
//!
//! | Mode       | Behaviour |
//! |------------|-----------|
//! | `direct`   | Single-pass structured tool sequence via native function calling. Deterministic. |
//! | `strategic`| Full autonomous loop: execute → adapt → synthesize. Budget-bounded. |
//!
//! # Tool Discovery
//!
//! Available tools come from two sources, merged and filtered by `config.tools`:
//! 1. **Shell tools** — `default_registry()`: `ls`, `pwd`, `python`. Always executable.
//! 2. **Pipeline node tools** — nodes with `ai_tool.registered = true` in their definition
//!    (e.g. `http_request`, `database_query`). Exposed to the LLM via the catalog;
//!    inline execution requires pipeline context (TODO: future milestone).
//!
//! # Credential
//!
//! `credential_id` selects an OpenAI-compatible credential from the project store.
//! Secret shape: `{ "api_key": "...", "base_url": "...", "model": "..." }`.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::automaton::agents::tool_caller::{ToolCallerAgent, ToolCallerConfig};
use crate::automaton::agents::zebtune::{
    ChainStep, OutputMode as ZebtuneOutputMode, ZebtuneAgent, ZebtuneConfig,
};
use crate::automaton::infra::http_client::{client_from_env, client_from_secret_with_model};
use crate::automaton::infra::llm_interface::{LlmCall, ToolDef};
use crate::automaton::infra::shell_tools::default_registry;
use crate::pipeline::model::{
    DslFlag, DslFlagKind, LayoutItem, NodeAiToolDefinition, NodeFieldDataSource, NodeFieldType,
    Signal,
};
use crate::pipeline::nodes::basic::builtin_node_definitions;
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::CredentialService;
use crate::platform::services::platform::PlatformService;

pub const NODE_KIND: &str = "n.ai.agent";
const INPUT_PIN: &str = "in";
const OUTPUT_PIN: &str = "out";

pub fn definition() -> NodeDefinition {
    use crate::pipeline::model::{NodeFieldDef, SelectOptionDef};
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "AI Agent".to_string(),
        description: "Autonomous agent with tool use. Direct mode: single-pass structured tool sequence. \
            Strategic mode: plan → act → adapt → synthesize. \
            Signals: this node emits real-time signals through the ExecutionBus during execution \
            (thinking, tool_call, tool_result, etc.). When the pipeline is triggered via a webhook \
            with Accept: text/event-stream, these signals are streamed to the client as SSE \
            event: signal messages.".to_string(),
        input_schema: json!({
            "type": "object",
            "description": "Trigger payload. Goal/query extracted from message, body, text, or query field."
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "response":        { "type": "string",  "description": "Final answer (direct mode)" },
                "final_content":   { "type": "string",  "description": "Final answer (strategic mode)" },
                "tools_called":    { "type": "array",   "description": "Tool names called (direct, full output)" },
                "iterations":      { "type": "number",  "description": "Iterations used (direct, full output)" },
                "chain":           { "type": "array",   "description": "Execution chain (strategic, full output)" },
                "budget_exhausted":{ "type": "boolean", "description": "Whether step budget was reached (strategic)" }
            }
        }),
        input_pins: vec![INPUT_PIN.to_string()],
        output_pins: vec![OUTPUT_PIN.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![
            DslFlag {
                flag: "--mode".to_string(),
                config_key: "mode".to_string(),
                description: "Agent mode: direct (default) or strategic.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--credential".to_string(),
                config_key: "credential_id".to_string(),
                description: "OpenAI-compatible credential id.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--max-iterations".to_string(),
                config_key: "max_iterations".to_string(),
                description: "Max LLM iterations for direct mode (default 5).".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--budget".to_string(),
                config_key: "step_budget".to_string(),
                description: "Step budget for strategic mode (default 10).".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--output-mode".to_string(),
                config_key: "output_mode".to_string(),
                description: "Output verbosity: full (default) or final_only.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--tools".to_string(),
                config_key: "tools".to_string(),
                description: "Comma-separated tool names to enable. Empty = all available.".to_string(),
                kind: DslFlagKind::CommaSeparatedList,
                required: false,
            },
            DslFlag {
                flag: "--system-prompt".to_string(),
                config_key: "system_prompt".to_string(),
                description: "System prompt override for the agent.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--model".to_string(),
                config_key: "model".to_string(),
                description: "Model name override (overrides model set in credential).".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--success-schema".to_string(),
                config_key: "success_schema".to_string(),
                description: "Strategic mode: JSON Schema (subset: type, required, properties, \
                    items, enum, minItems, minLength) the final output must satisfy. Enables \
                    verify-and-repair."
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--max-repairs".to_string(),
                config_key: "max_repairs".to_string(),
                description: "Strategic mode: max repair attempts when the success contract fails \
                    (default 0)."
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--verify".to_string(),
                config_key: "verify_function".to_string(),
                description: "Strategic mode: slug of a function pipeline that verifies the final \
                    answer. Receives {candidate, goal}, returns {pass, reason}. Runs after the \
                    JSON schema check; failure triggers repair."
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: vec![
            NodeFieldDef {
                name: "mode".to_string(),
                label: "Mode".to_string(),
                field_type: NodeFieldType::MethodButtons,
                options: vec![
                    SelectOptionDef { value: "direct".to_string(),    label: "Direct".to_string() },
                    SelectOptionDef { value: "strategic".to_string(), label: "Strategic".to_string() },
                ],
                default_value: Some(json!("direct")),
                span: Some("full".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "credential_id".to_string(),
                label: "Credential".to_string(),
                field_type: NodeFieldType::Select,
                data_source: Some(NodeFieldDataSource::CredentialsOpenAi),
                help: Some("OpenAI-compatible credential (api_key, base_url, model).".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "output_mode".to_string(),
                label: "Output Mode".to_string(),
                field_type: NodeFieldType::Select,
                options: vec![
                    SelectOptionDef { value: "full".to_string(),       label: "Full".to_string() },
                    SelectOptionDef { value: "final_only".to_string(), label: "Final Only".to_string() },
                ],
                default_value: Some(json!("full")),
                ..Default::default()
            },
            NodeFieldDef {
                name: "model".to_string(),
                label: "Model".to_string(),
                field_type: NodeFieldType::Text,
                placeholder: Some("e.g. MiniMax-M2.5, gpt-4o-mini".to_string()),
                help: Some("Model name to use. Overrides the model set in the credential.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "max_iterations".to_string(),
                label: "Max Iterations / Budget".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Direct: max LLM calls (default 5). Strategic: step budget (default 10).".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "system_prompt".to_string(),
                label: "System Prompt".to_string(),
                field_type: NodeFieldType::Textarea,
                rows: Some(6),
                ..Default::default()
            },
            NodeFieldDef {
                name: "tools".to_string(),
                label: "Available Tools".to_string(),
                field_type: NodeFieldType::MultiCheckbox,
                data_source: Some(NodeFieldDataSource::AiTools),
                help: Some("Tools the agent can invoke. Unchecked = disabled. All checked (default) = all available.".to_string()),
                span: Some("full".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "success_schema".to_string(),
                label: "Success Contract (JSON Schema)".to_string(),
                field_type: NodeFieldType::Textarea,
                rows: Some(6),
                help: Some("Strategic mode only. Optional JSON Schema (subset: type, required, \
                    properties, items, enum, minItems, minLength). The agent must produce output \
                    satisfying this contract; failures trigger up to 'Max Repairs' retries. Leave \
                    empty to disable verification.".to_string()),
                span: Some("full".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "max_repairs".to_string(),
                label: "Max Repairs".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Strategic mode only. How many times to retry when the success contract \
                    fails. Default 0 (accept best effort, marked unverified).".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "verify_function".to_string(),
                label: "Verifier (function pipeline)".to_string(),
                field_type: NodeFieldType::Select,
                data_source: Some(NodeFieldDataSource::FunctionPipelines),
                help: Some("Strategic mode only. Optional function pipeline that semantically \
                    verifies the final answer. It receives {candidate, goal} and must return \
                    {pass: bool, reason: string}. Runs after the JSON schema check; a failure \
                    triggers repair. Leave empty to disable.".to_string()),
                ..Default::default()
            },
        ],
        layout: vec![
            LayoutItem::Field("mode".to_string()),
            LayoutItem::Row { row: vec![
                LayoutItem::Field("credential_id".to_string()),
                LayoutItem::Field("output_mode".to_string()),
            ]},
            LayoutItem::Row { row: vec![
                LayoutItem::Field("model".to_string()),
                LayoutItem::Field("max_iterations".to_string()),
            ]},
            LayoutItem::Field("system_prompt".to_string()),
            LayoutItem::Field("tools".to_string()),
            LayoutItem::Field("success_schema".to_string()),
            LayoutItem::Row { row: vec![
                LayoutItem::Field("max_repairs".to_string()),
                LayoutItem::Field("verify_function".to_string()),
            ]},
        ],
        ai_tool: NodeAiToolDefinition::default(),
        ..Default::default()
    }
}

// ── Config ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AgentMode {
    #[default]
    Direct,
    Strategic,
}

impl<'de> serde::Deserialize<'de> for AgentMode {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        match s.to_ascii_lowercase().as_str() {
            "strategic" => Ok(Self::Strategic),
            _ => Ok(Self::Direct),
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputMode {
    #[default]
    Full,
    FinalOnly,
}

impl<'de> serde::Deserialize<'de> for OutputMode {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let s = String::deserialize(d)?;
        match s.to_ascii_lowercase().as_str() {
            "final_only" | "finalonly" => Ok(Self::FinalOnly),
            _ => Ok(Self::Full),
        }
    }
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Config {
    /// Agent mode: direct (single-pass tool sequence) or strategic (autonomous loop).
    #[serde(default)]
    pub mode: AgentMode,
    /// OpenAI-compatible credential id from project credential store.
    pub credential_id: Option<String>,
    /// Model name override. If set, overrides the model in the credential secret.
    pub model: Option<String>,
    /// Tool names to expose to the agent. Empty = all available.
    #[serde(default)]
    pub tools: Vec<String>,
    /// System prompt override. Supports {{ expr }} interpolation.
    pub system_prompt: Option<String>,
    /// Direct mode: max LLM iterations (default 5). Strategic mode: step budget (default 10).
    /// Stored in a single field; the UI label adapts based on mode.
    #[serde(default)]
    pub max_iterations: u32,
    /// Strategic mode: step budget alias — reads from max_iterations if step_budget is 0.
    #[serde(default)]
    pub step_budget: u32,
    /// Output verbosity.
    #[serde(default)]
    pub output_mode: OutputMode,
    /// Strategic mode: optional success contract as a JSON Schema (subset:
    /// type, required, properties, items, enum, minItems, minLength). Stored as
    /// a string (DSL flag / UI textarea); parsed to JSON at run time. When set,
    /// the agent verifies its final output and repairs on failure.
    #[serde(default)]
    pub success_schema: Option<String>,
    /// Strategic mode: max repair attempts when the success contract fails.
    /// Default 0 (no repair).
    #[serde(default)]
    pub max_repairs: u32,
    /// Strategic mode: optional semantic verifier (Phase B). The slug of a
    /// function pipeline that receives `{candidate, goal}` and returns
    /// `{pass: bool, reason: string}`. Runs after the JSON schema check; a
    /// `pass=false` triggers repair just like a schema failure. `None`/empty
    /// disables it.
    #[serde(default)]
    pub verify_function: Option<String>,
}

// ── Node ─────────────────────────────────────────────────────────────────────

pub struct Node {
    config: Config,
    credentials: Option<Arc<CredentialService>>,
    platform: Option<Arc<PlatformService>>,
}

impl Node {
    pub fn new(
        config: Config,
        credentials: Option<Arc<CredentialService>>,
        platform: Option<Arc<PlatformService>>,
    ) -> Self {
        Self {
            config,
            credentials,
            platform,
        }
    }

    /// Resolve LlmCall from credential then env — used by both direct and strategic modes.
    fn build_llm(&self, owner: &str, project: &str) -> Option<Arc<dyn LlmCall>> {
        let model_override = self.config.model.as_deref().filter(|m| !m.is_empty());
        if let (Some(cred_id), Some(creds)) = (&self.config.credential_id, &self.credentials) {
            if !cred_id.is_empty() {
                if let Ok(Some(cred)) = creds.get_project_credential(owner, project, cred_id) {
                    if let Some(client) =
                        client_from_secret_with_model(&cred.secret, model_override)
                    {
                        return Some(client);
                    }
                }
            }
        }
        client_from_env()
    }

    /// Build the merged ToolDef list from shell tools + registered pipeline node tools
    /// + active function pipelines, filtered by config.tools (empty = none).
    fn build_tool_defs(&self, owner: &str, project: &str) -> Vec<ToolDef> {
        let filter = &self.config.tools;
        if filter.is_empty() {
            return Vec::new();
        }
        let enabled = |name: &str| filter.iter().any(|t| t == name);

        let registry = default_registry();

        // 1. Shell tools
        let mut defs: Vec<ToolDef> = registry
            .tool_names()
            .into_iter()
            .filter(|n| enabled(n))
            .map(|name| {
                let desc = registry
                    .get(&name)
                    .map(|t| t.description().to_string())
                    .unwrap_or_default();
                ToolDef {
                    name: name.clone(),
                    description: desc,
                    parameters: json!({
                        "type": "object",
                        "properties": {
                            "path": { "type": "string", "description": "Path argument (optional)" }
                        }
                    }),
                }
            })
            .collect();

        // 2. Pipeline node tools (static, registered in node definitions)
        for def in builtin_node_definitions() {
            if def.ai_tool.registered && enabled(&def.ai_tool.tool_name) {
                defs.push(ToolDef {
                    name: def.ai_tool.tool_name,
                    description: def.ai_tool.tool_description,
                    parameters: def.ai_tool.tool_input_schema,
                });
            }
        }

        // 3. Function pipeline tools (dynamic, active n.trigger.function pipelines)
        if let Some(platform) = &self.platform {
            use crate::platform::services::project::name_from_file_rel_path;
            const FN_TRIGGER: &str = "n.trigger.function";

            let pipelines = platform.pipeline_runtime.list_project(owner, project);
            for compiled in pipelines {
                let is_fn = compiled.graph.nodes.iter().any(|n| n.kind == FN_TRIGGER);
                if !is_fn {
                    continue;
                }

                let slug = name_from_file_rel_path(&compiled.file_rel_path);
                if !enabled(&slug) {
                    continue;
                }

                // Extract params schema from the trigger node's config.
                // The `params` value may be stored as a JSON string (from DSL --params flag)
                // or as a parsed object (from UI editor). Either way, wrap it as a full
                // JSON Schema: {"type":"object","properties": <params>}.
                let params_schema = compiled
                    .graph
                    .nodes
                    .iter()
                    .find(|n| n.kind == FN_TRIGGER)
                    .and_then(|n| n.config.get("params"))
                    .and_then(|v| {
                        let props = if let Some(s) = v.as_str() {
                            serde_json::from_str::<Value>(s).ok()
                        } else if v.is_object() {
                            Some(v.clone())
                        } else {
                            None
                        };
                        props.map(|p| json!({ "type": "object", "properties": p }))
                    })
                    .unwrap_or_else(|| {
                        json!({
                            "type": "object",
                            "additionalProperties": true
                        })
                    });

                let description = format!(
                    "Call the '{}' function pipeline. {}",
                    slug,
                    compiled
                        .graph
                        .nodes
                        .iter()
                        .find(|n| n.kind == FN_TRIGGER)
                        .and_then(|n| n.config.get("description"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                )
                .trim_end()
                .to_string();

                defs.push(ToolDef {
                    name: slug,
                    description,
                    parameters: params_schema,
                });
            }
        }

        defs
    }

    /// Returns the slugs of all active function pipelines — used by the UI data source.
    pub fn list_function_pipeline_tool_names(
        platform: &PlatformService,
        owner: &str,
        project: &str,
    ) -> Vec<String> {
        const FN_TRIGGER: &str = "n.trigger.function";
        use crate::platform::services::project::name_from_file_rel_path;
        platform
            .pipeline_runtime
            .list_project(owner, project)
            .into_iter()
            .filter(|c| c.graph.nodes.iter().any(|n| n.kind == FN_TRIGGER))
            .map(|c| name_from_file_rel_path(&c.file_rel_path))
            .collect()
    }
}

#[async_trait]
impl NodeHandler for Node {
    fn kind(&self) -> &'static str {
        NODE_KIND
    }
    fn input_pins(&self) -> &'static [&'static str] {
        &[INPUT_PIN]
    }
    fn output_pins(&self) -> &'static [&'static str] {
        &[OUTPUT_PIN]
    }

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        if input.input_pin != INPUT_PIN {
            return Err(PipelineError::new(
                "FW_NODE_AGENT_INPUT_PIN",
                format!("unsupported input pin '{}'", input.input_pin),
            ));
        }

        let query = goal_from_payload(&input.payload).ok_or_else(|| {
            PipelineError::new(
                "FW_NODE_AGENT_QUERY",
                "payload must contain message, body, text, query (string), or be a string",
            )
        })?;

        let owner = input
            .metadata
            .get("owner")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let project = input
            .metadata
            .get("project")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        match &self.config.mode {
            AgentMode::Direct => self.run_direct(&query, owner, project, &input).await,
            AgentMode::Strategic => self.run_strategic(&query, owner, project, &input).await,
        }
    }
}

impl Node {
    async fn run_direct(
        &self,
        query: &str,
        owner: &str,
        project: &str,
        _input: &NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        let Some(llm) = self.build_llm(owner, project) else {
            return Ok(NodeExecutionOutput {
                output_pins: vec![OUTPUT_PIN.to_string()],
                payload: json!({
                    "response": "LLM not configured. Set a credential or ZEBTUNE_OPENAI_API_KEY.",
                    "tools_called": [],
                    "iterations": 0
                }),
                trace: vec!["agent_direct_no_llm".to_string()],
            });
        };

        let tools = self.build_tool_defs(owner, project);
        let registry = default_registry();
        let work_dir =
            std::env::current_dir().unwrap_or_else(|_| std::path::Path::new(".").to_path_buf());

        let max_iter = if self.config.max_iterations == 0 {
            5
        } else {
            self.config.max_iterations
        };

        let agent = ToolCallerAgent::new(
            ToolCallerConfig {
                system_prompt: self.config.system_prompt.clone(),
                max_iterations: max_iter,
            },
            llm,
        );

        // Capture values needed inside the sync executor closure.
        let platform_clone = self.platform.clone();
        let owner_s = owner.to_string();
        let project_s = project.to_string();

        let result = agent
            .run(query, tools, move |tool_name, args_json| {
                let args: Value = serde_json::from_str(args_json).unwrap_or(json!({}));

                // 1. Try shell tools first.
                if let Some(out) = registry.run_tool(tool_name, &args, &work_dir) {
                    return out;
                }

                // 2. Try function pipeline tools (async → sync bridge).
                if let Some(platform) = &platform_clone {
                    let platform = Arc::clone(platform);
                    let slug = tool_name.to_string();
                    let input = args.clone();
                    let owner_s = owner_s.clone();
                    let project_s = project_s.clone();

                    let result = tokio::task::block_in_place(|| {
                        tokio::runtime::Handle::current().block_on(async move {
                            platform
                                .execute_function_pipeline(&owner_s, &project_s, &slug, input)
                                .await
                        })
                    });

                    return match result {
                        Ok(v) => {
                            Ok(serde_json::to_string_pretty(&v).unwrap_or_else(|_| v.to_string()))
                        }
                        Err(e) => Err(e.to_string()),
                    };
                }

                Err(format!("tool '{}' not found", tool_name))
            })
            .await
            .map_err(|e| PipelineError::new("FW_NODE_AGENT_DIRECT_RUN", e))?;

        let payload = match &self.config.output_mode {
            OutputMode::Full => json!({
                "response":     result.response,
                "tools_called": result.tools_called,
                "iterations":   result.iterations,
            }),
            OutputMode::FinalOnly => json!({ "response": result.response }),
        };

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN.to_string()],
            payload,
            trace: vec![
                "agent=direct".to_string(),
                format!("iterations={}", result.iterations),
            ],
        })
    }

    async fn run_strategic(
        &self,
        query: &str,
        owner: &str,
        project: &str,
        input: &NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        let llm = self.build_llm(owner, project);

        let budget = match (self.config.step_budget, self.config.max_iterations) {
            (0, 0) => 10,
            (0, m) => m,
            (b, _) => b,
        };

        let zebtune_output_mode = match &self.config.output_mode {
            OutputMode::Full => ZebtuneOutputMode::Full,
            OutputMode::FinalOnly => ZebtuneOutputMode::FinalOnly,
        };

        // Parse the optional success contract.
        //
        // Fail-loud rule: an absent or empty schema means "no contract" (fine).
        // But a NON-EMPTY schema that fails to parse is a configuration error and
        // must NOT be silently downgraded to "no contract" — doing so would run
        // the agent unverified while the operator believes verification is on
        // (a fail-open security hole; also masks the DSL double-quote pitfall
        // where `--success-schema "{...}"` strips inner quotes). Surface it.
        let success_schema = match self
            .config
            .success_schema
            .as_ref()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
        {
            Some(s) => match serde_json::from_str::<Value>(s) {
                Ok(v) => Some(v),
                Err(e) => {
                    return Err(PipelineError::new(
                        "FW_NODE_AGENT_BAD_SCHEMA",
                        format!(
                            "success_schema is set but is not valid JSON: {}. \
                             A malformed contract is refused rather than silently ignored \
                             (which would run the agent unverified). If authoring via DSL, \
                             wrap the schema in single quotes: --success-schema '{{...}}'.",
                            e
                        ),
                    ));
                }
            },
            None => None,
        };

        let agent = ZebtuneAgent::new(
            ZebtuneConfig {
                step_budget: budget,
                system_prompt: self.config.system_prompt.clone(),
                output_mode: zebtune_output_mode,
                success_schema,
                max_repairs: self.config.max_repairs,
            },
            llm,
        );

        // Use the same tool discovery as direct mode: shell + node AI tools + function pipelines.
        let tools = self.build_tool_defs(owner, project);
        let registry = default_registry();
        let work_dir =
            std::env::current_dir().unwrap_or_else(|_| std::path::Path::new(".").to_path_buf());

        let platform_clone = self.platform.clone();
        let owner_s = owner.to_string();
        let project_s = project.to_string();

        // Bridge step events to pipeline Signal via ExecutionBus
        let bus = input.bus.clone();
        let agent_node_id = input.node_id.clone();
        let callback: Option<crate::automaton::agents::zebtune::StepCallback> =
            bus.map(|b| -> crate::automaton::agents::zebtune::StepCallback {
                Box::new(move |s: &ChainStep| {
                    b.emit(Signal {
                        kind: s.step.clone(),
                        message: s.description.clone(),
                        node_id: agent_node_id.clone(),
                        node_kind: NODE_KIND.to_string(),
                        data: None,
                        at: s.at.clone(),
                    });
                })
            });

        // Build the optional semantic verifier (Phase B) from `verify_function`.
        // Mirrors the tool executor's async→sync pipeline bridge so the agent
        // stays decoupled. The verifier pipeline receives `{candidate, goal}`
        // and must return `{pass: bool, reason: string}`.
        let verify_slug = self
            .config
            .verify_function
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let verify_fn: Option<Box<crate::automaton::agents::zebtune::VerifyFn>> =
            match (verify_slug, &self.platform) {
                (Some(slug), Some(platform_arc)) => {
                    let platform = Arc::clone(platform_arc);
                    let owner_v = owner.to_string();
                    let project_v = project.to_string();
                    Some(Box::new(move |candidate: &str, goal: &str| -> (bool, String) {
                        let platform = Arc::clone(&platform);
                        let slug = slug.clone();
                        let owner_v = owner_v.clone();
                        let project_v = project_v.clone();
                        let input = json!({ "candidate": candidate, "goal": goal });
                        let res = tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async move {
                                platform
                                    .execute_function_pipeline(&owner_v, &project_v, &slug, input)
                                    .await
                            })
                        });
                        match res {
                            Ok(v) => match v.get("pass").and_then(|x| x.as_bool()) {
                                Some(pass) => {
                                    let reason = v
                                        .get("reason")
                                        .and_then(|x| x.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    (pass, reason)
                                }
                                None => (
                                    false,
                                    format!(
                                        "verifier did not return a boolean 'pass' field; got: {}",
                                        v
                                    ),
                                ),
                            },
                            Err(e) => (false, format!("verifier pipeline error: {}", e)),
                        }
                    }))
                }
                _ => None,
            };

        let result = agent
            .run(
                query,
                tools,
                move |tool_name, args_json| {
                    let args: Value = serde_json::from_str(args_json).unwrap_or(json!({}));

                    // 1. Try shell tools first.
                    if let Some(out) = registry.run_tool(tool_name, &args, &work_dir) {
                        return out;
                    }

                    // 2. Try function pipeline tools (async → sync bridge).
                    if let Some(platform) = &platform_clone {
                        let platform = Arc::clone(platform);
                        let slug = tool_name.to_string();
                        let input = args.clone();
                        let owner_s = owner_s.clone();
                        let project_s = project_s.clone();

                        let result = tokio::task::block_in_place(|| {
                            tokio::runtime::Handle::current().block_on(async move {
                                platform
                                    .execute_function_pipeline(&owner_s, &project_s, &slug, input)
                                    .await
                            })
                        });

                        return match result {
                            Ok(v) => {
                                Ok(serde_json::to_string_pretty(&v)
                                    .unwrap_or_else(|_| v.to_string()))
                            }
                            Err(e) => Err(e.to_string()),
                        };
                    }

                    Err(format!("tool '{}' not found", tool_name))
                },
                verify_fn.as_deref(),
                callback.as_ref(),
            )
            .await;

        let payload = match &self.config.output_mode {
            OutputMode::Full => {
                if result.chain.is_empty() {
                    json!({
                        "final_content":    result.final_content,
                        "budget_exhausted": result.budget_exhausted,
                        "verified":         result.verified,
                        "repairs_used":     result.repairs_used,
                        "metrics":          result.metrics,
                        "trace":            result.trace,
                    })
                } else {
                    json!({
                        "final_content":    result.final_content,
                        "chain":            result.chain,
                        "budget_exhausted": result.budget_exhausted,
                        "verified":         result.verified,
                        "repairs_used":     result.repairs_used,
                        "metrics":          result.metrics,
                        "trace":            result.trace,
                    })
                }
            }
            // Even in final_only we include metrics — it's the benchmark meter,
            // cheap, and harmless to consumers that ignore it.
            OutputMode::FinalOnly => json!({
                "final_content": result.final_content,
                "metrics":       result.metrics,
            }),
        };

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN.to_string()],
            payload,
            trace: result.trace,
        })
    }
}

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
    payload.as_str().map(|s| s.to_string())
}
