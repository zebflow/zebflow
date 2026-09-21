//! `n.ai.agent` — one model loop: prompt in, answer out, tools when named.
//!
//! | Use | DSL |
//! |---|---|
//! | One call, text back | `\| ai.agent --credential openai_main --prompt "Summarise: {{ input.body.text }}"` |
//! | One call, JSON back | `\| ai.agent --credential openai_main --schema '{"type":"object","required":["sentiment"]}' -- Classify: {{ input.body.review }}` |
//! | Tools, until it answers | `\| ai.agent --credential openai_main --tools lookup-order,list-slots --budget 8 --prompt "{{ input.body.message }}"` |
//!
//! The loop is the one every coding agent runs: send the messages with the
//! tool definitions, run each tool call the model returns, append the results,
//! call again, stop when the model answers with text. With no tools named
//! there is nothing to call, so it is one round trip. `--budget` caps the
//! number of model calls; `--schema` and `--verify` gate the answer and feed a
//! failure back for repair.
//!
//! # Tools
//!
//! A tool is one of the project's **function pipelines** (`trigger.function`),
//! named by its slug in `--tools`. Nothing is offered unless named. There are
//! no shell tools and no built-in node tools: what the agent can do is what the
//! project wrote as a function pipeline, which is visible, reviewable and
//! guarded like any other route.
//!
//! # Credential
//!
//! `--credential` names a credential of kind `openai` or `openrouter`; its kind
//! picks the provider and its secret holds the key, base URL and default model.
//! That is the only source of an LLM secret.

use std::sync::Arc;

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::automaton::agents::zebtune::{
    ChainStep, OutputMode as ZebtuneOutputMode, ZebtuneAgent, ZebtuneConfig,
};
use crate::automaton::infra::http_client::client_from_provider_secret_with_model;
use crate::automaton::infra::llm_interface::{LlmCall, ToolDef};
use crate::pipeline::model::NodeCapability;
use crate::pipeline::model::{
    DslFlag, DslFlagKind, LayoutItem, NodeAiToolDefinition, NodeFieldDataSource, NodeFieldType,
    Signal,
};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::CredentialService;
use crate::platform::services::platform::PlatformService;

pub const NODE_KIND: &str = "n.ai.agent";
const INPUT_PIN: &str = "in";
const OUTPUT_PIN: &str = "out";
const DEFAULT_BUDGET: u32 = 10;
const DEFAULT_MAX_REPAIRS: u32 = 1;

/// Credential kinds whose secret builds an LLM client.
pub const LLM_CREDENTIAL_KINDS: &[&str] = &["openai", "openrouter"];

pub fn definition() -> NodeDefinition {
    use crate::pipeline::model::{NodeFieldDef, SelectOptionDef};
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Network, NodeCapability::Credential],
        title: "AI Agent".to_string(),
        description: "One model loop: the prompt goes to the model with the named tools; each tool call the model \
            makes is run and fed back; it stops when the model answers with text or the budget is spent. \
            With no --tools it is one call — classify, extract, summarise, draft. With --schema the answer is \
            parsed as JSON, checked, repaired once on failure and returned as `data`. Tools are the project's \
            function pipelines, named by slug; nothing is offered unless named. Step signals (thinking, \
            tool_call, tool_result, final) stream over SSE when the webhook is called with Accept: text/event-stream."
            .to_string(),
        input_schema: json!({
            "type": "object",
            "description": "Payload the {{ expr }} in --prompt and --system-prompt resolve against. Without --prompt, the goal is read from message, body, text or query."
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "response":         { "type": "string",  "description": "The model's final answer as text." },
                "data":             { "description": "With --schema: the answer parsed as JSON (present only when it passed the check)." },
                "verified":         { "type": "boolean", "description": "True when no contract was set, or the answer passed --schema and --verify." },
                "tools_called":     { "type": "array",   "description": "Function pipeline slugs the model called, in order." },
                "iterations":       { "type": "number",  "description": "Model calls made." },
                "budget_exhausted": { "type": "boolean", "description": "True when the budget ended the loop before an answer." },
                "chain":            { "type": "array",   "description": "Step log (full output only)." },
                "tool_events":      { "type": "array",   "description": "Each tool call with arguments and result (full output only)." },
                "metrics":          { "type": "object",  "description": "llm_calls, tool_calls, prompt_tokens, completion_tokens, wall_ms, stop_reason (full output only)." }
            }
        }),
        input_pins: vec![INPUT_PIN.to_string()],
        output_pins: vec![OUTPUT_PIN.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: json!({
            "type": "object",
            "properties": {
                "credential_id":   { "type": "string", "description": "Credential of kind openai or openrouter." },
                "prompt":          { "type": "string", "description": "The prompt — literal or {{ expr }}. Without it the goal is read from the payload's message, body, text or query." },
                "system_prompt":   { "type": "string", "description": "Standing instructions — literal or {{ expr }}." },
                "tools":           { "type": "array", "items": { "type": "string" }, "description": "Function pipeline slugs the model may call. Empty = no tools." },
                "budget":          { "type": "integer", "description": "Maximum model calls (default 10)." },
                "schema":          { "type": "string", "description": "JSON Schema (subset) the answer must satisfy; the parsed answer is returned as `data`." },
                "max_repairs":     { "type": "integer", "description": "Repair rounds when --schema or --verify fails (default 1)." },
                "verify_function": { "type": "string", "description": "Slug of a function pipeline that checks the answer: receives {candidate, goal}, returns {pass, reason}." },
                "model":           { "type": "string", "description": "Model id, overriding the credential's default." },
                "output_mode":     { "type": "string", "enum": ["full", "final_only"], "description": "full (default) adds chain, tool_events and metrics." }
            },
            "required": ["credential_id"]
        }),
        dsl_flags: vec![
            DslFlag { flag: "--credential".into(), config_key: "credential_id".into(), description: "Credential of kind openai or openrouter — the only source of the LLM key; its kind picks the provider.".into(), kind: DslFlagKind::Scalar, required: true },
            DslFlag { flag: "--prompt".into(), config_key: "prompt".into(), description: "The prompt — literal or {{ expr }}; or write it after `--`. Without it the goal is read from the payload's message, body, text or query.".into(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--system-prompt".into(), config_key: "system_prompt".into(), description: "Standing instructions for every call — literal or {{ expr }}.".into(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--tools".into(), config_key: "tools".into(), description: "Comma-separated function pipeline slugs the model may call. Nothing is offered unless named; with no tools the node is one call.".into(), kind: DslFlagKind::CommaSeparatedList, required: false },
            DslFlag { flag: "--budget".into(), config_key: "budget".into(), description: "Maximum model calls before the loop stops (default 10). Repairs count.".into(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--schema".into(), config_key: "schema".into(), description: "JSON Schema (type, required, properties, items, enum, minItems, minLength) the answer must satisfy; the parsed answer is returned as `data`. Quote with single quotes in the DSL.".into(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--max-repairs".into(), config_key: "max_repairs".into(), description: "How many times a failed --schema/--verify is fed back for another attempt (default 1).".into(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--verify".into(), config_key: "verify_function".into(), description: "Slug of a function pipeline that checks the answer: receives {candidate, goal}, returns {pass, reason}. Runs after the schema check.".into(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--model".into(), config_key: "model".into(), description: "Model id, overriding the credential's default.".into(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--output-mode".into(), config_key: "output_mode".into(), description: "full (default) or final_only — final_only drops chain, tool_events and metrics.".into(), kind: DslFlagKind::Scalar, required: false },
        ],
        fields: vec![
            NodeFieldDef {
                name: "credential_id".to_string(),
                label: "Credential".to_string(),
                field_type: NodeFieldType::Select,
                data_source: Some(NodeFieldDataSource::CredentialsOpenAi),
                help: Some("Credential of kind openai or openrouter; its kind picks the provider and its secret holds the key.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "model".to_string(),
                label: "Model".to_string(),
                field_type: NodeFieldType::Text,
                placeholder: Some("e.g. gpt-4o-mini".to_string()),
                help: Some("Overrides the model set in the credential.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "system_prompt".to_string(),
                label: "System Prompt".to_string(),
                field_type: NodeFieldType::Textarea,
                rows: Some(4),
                help: Some("Standing instructions for every call — literal or {{ expr }}.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "prompt".to_string(),
                label: "Prompt".to_string(),
                field_type: NodeFieldType::Textarea,
                rows: Some(4),
                help: Some("Literal or {{ expr }}. Empty: the goal is read from the payload's message, body, text or query.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "tools".to_string(),
                label: "Tools (function pipelines)".to_string(),
                field_type: NodeFieldType::MultiCheckbox,
                data_source: Some(NodeFieldDataSource::AiTools),
                help: Some("Function pipelines the model may call. None checked = one call, no tools.".to_string()),
                span: Some("full".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "budget".to_string(),
                label: "Budget (model calls)".to_string(),
                field_type: NodeFieldType::Number,
                help: Some("Maximum model calls before the loop stops. Default 10.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "output_mode".to_string(),
                label: "Output".to_string(),
                field_type: NodeFieldType::Select,
                options: vec![
                    SelectOptionDef { value: "full".to_string(), label: "Full (with chain and metrics)".to_string() },
                    SelectOptionDef { value: "final_only".to_string(), label: "Final only".to_string() },
                ],
                default_value: Some(json!("full")),
                help: Some("Full adds the step log, tool events and metrics to the answer.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "schema".to_string(),
                label: "Answer Schema (JSON)".to_string(),
                field_type: NodeFieldType::CodeEditor,
                language: Some("json".to_string()),
                help: Some("When set, the answer must be JSON matching this schema (type, required, properties, items, enum, minItems, minLength) and is returned as `data`.".to_string()),
                span: Some("full".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "max_repairs".to_string(),
                label: "Max Repairs".to_string(),
                field_type: NodeFieldType::Number,
                help: Some("How many times a failed schema or verifier check is fed back for another attempt. Default 1.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "verify_function".to_string(),
                label: "Verifier (function pipeline)".to_string(),
                field_type: NodeFieldType::Select,
                data_source: Some(NodeFieldDataSource::FunctionPipelines),
                help: Some("Receives {candidate, goal}, returns {pass, reason}. Runs after the schema check; a fail is fed back as a repair.".to_string()),
                ..Default::default()
            },
        ],
        layout: vec![
            LayoutItem::Row { row: vec![LayoutItem::Field("credential_id".to_string()), LayoutItem::Field("model".to_string())] },
            LayoutItem::Field("system_prompt".to_string()),
            LayoutItem::Field("prompt".to_string()),
            LayoutItem::Field("tools".to_string()),
            LayoutItem::Row { row: vec![LayoutItem::Field("budget".to_string()), LayoutItem::Field("output_mode".to_string())] },
            LayoutItem::Field("schema".to_string()),
            LayoutItem::Row { row: vec![LayoutItem::Field("max_repairs".to_string()), LayoutItem::Field("verify_function".to_string())] },
        ],
        ai_tool: NodeAiToolDefinition::default(),
        failure_semantics: vec![
            crate::pipeline::model::NodeFailureSemantic { code: "FW_NODE_AGENT_CREDENTIAL".into(), description: "`--credential` is missing, names no credential, or names one that is not openai/openrouter.".into(), ..Default::default() },
            crate::pipeline::model::NodeFailureSemantic { code: "FW_NODE_AGENT_QUERY".into(), description: "No `--prompt` and the payload has no message, body, text or query string.".into(), ..Default::default() },
            crate::pipeline::model::NodeFailureSemantic { code: "FW_NODE_AGENT_BAD_SCHEMA".into(), description: "`--schema` is set but is not valid JSON (quote it with single quotes in the DSL).".into(), ..Default::default() },
            crate::pipeline::model::NodeFailureSemantic { code: "FW_NODE_AGENT_CALL".into(), description: "The provider refused or the request failed; the message carries the provider's text.".into(), retryable: true, retry_hint: "429 and 5xx recover on retry; a 401 needs a new key.".into() },
        ],
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("One call: summarise a submission", r#"ai.agent --credential openai_main --system-prompt "You write one plain sentence." --prompt "Summarise: {{ input.body.text }}""#)
                .input(json!({ "body": { "text": "Our clinic moved to 12 High St and now opens Saturdays 9–1." } }))
                .output(json!({ "response": "The clinic has moved to 12 High St and now opens on Saturday mornings.", "verified": true, "tools_called": [], "iterations": 1, "budget_exhausted": false }))
                .note("No --tools, so one round trip. Replaces the payload; keep what later nodes need in `$nodes.<id>`. The credential is created by the owner in Studio → Credentials."),
            crate::pipeline::model::NodeExample::dsl("One call: classify to JSON", r#"ai.agent --credential openai_main --output-mode final_only --schema '{"type":"object","required":["sentiment"],"properties":{"sentiment":{"enum":["positive","neutral","negative"]}}}' -- Classify this review: {{ input.body.review }}"#)
                .input(json!({ "body": { "review": "Booking was easy but the wait was long." } }))
                .output(json!({ "response": "{\"sentiment\":\"neutral\"}", "data": { "sentiment": "neutral" }, "verified": true }))
                .note("`data` is the parsed, checked answer; branch on it with `logic.if --expr \"$nodes.a.data.sentiment == 'negative'\"`. A failed check is fed back once (--max-repairs)."),
            crate::pipeline::model::NodeExample::dsl("Tools: answer from the project's data", r#"ai.agent --credential openai_main --tools lookup-order --budget 6 --system-prompt "You answer questions about orders. Use the tools; never guess." --prompt "{{ input.body.message }}""#)
                .input(json!({ "body": { "message": "Where is order o_91?" } }))
                .output(json!({ "response": "Order o_91 shipped yesterday and arrives Friday.", "verified": true, "tools_called": ["lookup-order"], "iterations": 2, "budget_exhausted": false }))
                .note("`lookup-order` is a function pipeline (trigger.function) in this project; the model calls it with the arguments its input schema declares."),
        ],
        ..Default::default()
    }
}

// ── Config ──────────────────────────────────────────────────────────────────

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
    /// Credential of kind openai or openrouter — the only source of the key.
    #[serde(default)]
    pub credential_id: Option<String>,
    /// Model id overriding the credential's default.
    #[serde(default)]
    pub model: Option<String>,
    /// The prompt, literal or `{{ expr }}` (resolved by the engine). Empty:
    /// the goal is read from the payload's message/body/text/query.
    #[serde(default)]
    pub prompt: Option<String>,
    /// Standing instructions for every call.
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// Function pipeline slugs the model may call. Empty = no tools.
    #[serde(default)]
    pub tools: Vec<String>,
    /// Maximum model calls (default 10).
    #[serde(default)]
    pub budget: u32,
    /// Output verbosity.
    #[serde(default)]
    pub output_mode: OutputMode,
    /// JSON Schema (subset) the answer must satisfy. Stored as text (DSL flag
    /// / UI editor); parsed at run time, refused when malformed.
    #[serde(default)]
    pub schema: Option<String>,
    /// Repair rounds when the schema or verifier fails (default 1).
    #[serde(default)]
    pub max_repairs: Option<u32>,
    /// Slug of a function pipeline that checks the answer: receives
    /// `{candidate, goal}`, returns `{pass, reason}`.
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

    /// Resolve the LLM client from the node's credential — the only source of
    /// an LLM secret. A missing or unusable credential is a refusal, never a
    /// silent fallback.
    fn build_llm(&self, owner: &str, project: &str) -> Result<Arc<dyn LlmCall>, PipelineError> {
        let model_override = self.config.model.as_deref().map(str::trim).filter(|m| !m.is_empty());
        let cred_id = self
            .config
            .credential_id
            .as_deref()
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .ok_or_else(|| {
                PipelineError::new(
                    "FW_NODE_AGENT_CREDENTIAL",
                    format!(
                        "ai.agent needs --credential naming a credential of kind {}",
                        LLM_CREDENTIAL_KINDS.join(" or ")
                    ),
                )
            })?;
        let creds = self.credentials.as_ref().ok_or_else(|| {
            PipelineError::new(
                "FW_NODE_AGENT_CREDENTIAL",
                "credential service is not configured on this engine",
            )
        })?;
        let cred = creds
            .get_project_credential(owner, project, cred_id)
            .map_err(|err| PipelineError::new("FW_NODE_AGENT_CREDENTIAL", err.to_string()))?
            .ok_or_else(|| {
                PipelineError::new(
                    "FW_NODE_AGENT_CREDENTIAL",
                    format!("credential '{cred_id}' not found"),
                )
            })?;
        if !LLM_CREDENTIAL_KINDS.contains(&cred.kind.as_str()) {
            return Err(PipelineError::new(
                "FW_NODE_AGENT_CREDENTIAL",
                format!(
                    "credential '{cred_id}' is kind '{}'; ai.agent needs {}",
                    cred.kind,
                    LLM_CREDENTIAL_KINDS.join(" or ")
                ),
            ));
        }
        client_from_provider_secret_with_model(&cred.kind, &cred.secret, model_override).ok_or_else(|| {
            PipelineError::new(
                "FW_NODE_AGENT_CREDENTIAL",
                format!("credential '{cred_id}' ({}) has no api_key", cred.kind),
            )
        })
    }

    /// The tool definitions offered to the model: the project's active
    /// function pipelines whose slug is named in `config.tools`. Nothing else.
    fn build_tool_defs(&self, owner: &str, project: &str) -> Vec<ToolDef> {
        let wanted: Vec<&str> = self
            .config
            .tools
            .iter()
            .map(|t| t.trim())
            .filter(|t| !t.is_empty())
            .collect();
        if wanted.is_empty() {
            return Vec::new();
        }
        let Some(platform) = &self.platform else {
            return Vec::new();
        };
        use crate::platform::services::project::name_from_file_rel_path;
        const FN_TRIGGER: &str = "n.trigger.function";

        let mut defs = Vec::new();
        for compiled in platform.pipeline_runtime.list_project(owner, project) {
            let Some(trigger) = compiled.graph.nodes.iter().find(|n| n.kind == FN_TRIGGER) else {
                continue;
            };
            let slug = name_from_file_rel_path(&compiled.file_rel_path);
            if !wanted.contains(&slug.as_str()) {
                continue;
            }
            let trigger_config = trigger.config.clone();
            defs.push(ToolDef {
                name: slug.clone(),
                description: crate::pipeline::nodes::basic::trigger::function::tool_description_from_config(
                    &slug,
                    &trigger_config,
                ),
                parameters: crate::pipeline::nodes::basic::trigger::function::input_schema_from_config(
                    &trigger_config,
                ),
            });
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

    /// The prompt: `--prompt` when given, else the payload's message/body/text/query.
    fn goal(&self, payload: &Value) -> Result<String, PipelineError> {
        if let Some(prompt) = self.config.prompt.as_deref().map(str::trim).filter(|p| !p.is_empty()) {
            return Ok(prompt.to_string());
        }
        goal_from_payload(payload).ok_or_else(|| {
            PipelineError::new(
                "FW_NODE_AGENT_QUERY",
                "no --prompt, and the payload has no message, body, text or query string",
            )
        })
    }

    /// The answer schema, parsed. An absent or empty schema is no contract; a
    /// non-empty one that does not parse is refused — running unverified while
    /// the author believes verification is on would be a fail-open hole.
    fn schema(&self) -> Result<Option<Value>, PipelineError> {
        match self.config.schema.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            Some(raw) => serde_json::from_str::<Value>(raw).map(Some).map_err(|err| {
                PipelineError::new(
                    "FW_NODE_AGENT_BAD_SCHEMA",
                    format!(
                        "--schema is set but is not valid JSON: {err}. In the DSL wrap it in single quotes: --schema '{{...}}'."
                    ),
                )
            }),
            None => Ok(None),
        }
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
        let owner = input.metadata.get("owner").and_then(|v| v.as_str()).unwrap_or("");
        let project = input.metadata.get("project").and_then(|v| v.as_str()).unwrap_or("");

        let goal = self.goal(&input.payload)?;
        let schema = self.schema()?;
        let llm = self.build_llm(owner, project)?;
        let tools = self.build_tool_defs(owner, project);
        let has_contract = schema.is_some()
            || self.config.verify_function.as_deref().map(str::trim).filter(|s| !s.is_empty()).is_some();

        let agent = ZebtuneAgent::new(
            ZebtuneConfig {
                step_budget: if self.config.budget == 0 { DEFAULT_BUDGET } else { self.config.budget },
                system_prompt: self.config.system_prompt.clone(),
                output_mode: match &self.config.output_mode {
                    OutputMode::Full => ZebtuneOutputMode::Full,
                    OutputMode::FinalOnly => ZebtuneOutputMode::FinalOnly,
                },
                success_schema: schema.clone(),
                max_repairs: self.config.max_repairs.unwrap_or(DEFAULT_MAX_REPAIRS),
            },
            Some(llm),
        );

        // Tool executor: a function pipeline by slug. The async → sync bridge
        // is what the agent's synchronous executor signature needs.
        let platform_for_tools = self.platform.clone();
        let (owner_s, project_s) = (owner.to_string(), project.to_string());
        let executor = move |tool_name: &str, args_json: &str| -> Result<String, String> {
            let Some(platform) = &platform_for_tools else {
                return Err(format!("tool '{tool_name}' not found"));
            };
            let args: Value = serde_json::from_str(args_json).unwrap_or(json!({}));
            let platform = Arc::clone(platform);
            let (slug, owner_s, project_s) = (tool_name.to_string(), owner_s.clone(), project_s.clone());
            let result = tokio::task::block_in_place(|| {
                tokio::runtime::Handle::current().block_on(async move {
                    platform
                        .execute_function_pipeline(&owner_s, &project_s, &slug, args)
                        .await
                })
            });
            function_tool_result_for_agent(result)
        };

        // Step events → pipeline signals on the execution bus (SSE to the client).
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

        // The verifier: a function pipeline receiving {candidate, goal} and
        // answering {pass, reason}. Runs after the schema check passes.
        let verify_slug = self
            .config
            .verify_function
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        let verify_fn: Option<Box<crate::automaton::agents::zebtune::VerifyFn>> =
            match (verify_slug, &self.platform) {
                (Some(slug), Some(platform_arc)) => {
                    let platform = Arc::clone(platform_arc);
                    let (owner_v, project_v) = (owner.to_string(), project.to_string());
                    Some(Box::new(move |candidate: &str, goal: &str| -> (bool, String) {
                        let platform = Arc::clone(&platform);
                        let (slug, owner_v, project_v) = (slug.clone(), owner_v.clone(), project_v.clone());
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
                                Some(pass) => (
                                    pass,
                                    v.get("reason").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                                ),
                                None => (false, format!("verifier did not return a boolean 'pass' field; got: {v}")),
                            },
                            Err(e) => (false, format!("verifier pipeline error: {e}")),
                        }
                    }))
                }
                _ => None,
            };

        let result = agent
            .run(&goal, tools, executor, verify_fn.as_deref(), callback.as_ref())
            .await;

        // A provider failure is a node failure, not an answer: the error groups
        // and the request id must see it.
        if result.metrics.stop_reason == "error" {
            return Err(PipelineError::new("FW_NODE_AGENT_CALL", result.final_content));
        }

        let tools_called: Vec<String> = result
            .tool_events
            .iter()
            .filter_map(|e| e.get("name").and_then(|n| n.as_str()).map(str::to_string))
            .collect();
        let data = match (&schema, result.verified || !has_contract) {
            (Some(_), true) => crate::automaton::agents::contract::extract_json(&result.final_content),
            _ => None,
        };

        let mut payload = json!({
            "response": result.final_content,
            "verified": result.verified,
        });
        if let Some(data) = data {
            payload["data"] = data;
        }
        if matches!(self.config.output_mode, OutputMode::Full) {
            payload["tools_called"] = json!(tools_called);
            payload["iterations"] = json!(result.metrics.llm_calls);
            payload["budget_exhausted"] = json!(result.budget_exhausted);
            payload["chain"] = json!(result.chain);
            payload["tool_events"] = json!(result.tool_events);
            payload["metrics"] = json!(result.metrics);
        }

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN.to_string()],
            payload,
            trace: result.trace,
        })
    }
}

fn function_tool_result_for_agent(result: Result<Value, PipelineError>) -> Result<String, String> {
    match result {
        Ok(v) => Ok(serde_json::to_string_pretty(&v).unwrap_or_else(|_| v.to_string())),
        Err(e) if e.code == "FW_FUNCTION_INPUT_INVALID" => {
            let parsed = serde_json::from_str::<Value>(&e.message).unwrap_or_else(|_| {
                json!({
                    "ok": false,
                    "code": e.code,
                    "message": e.message
                })
            });
            Ok(serde_json::to_string_pretty(&parsed).unwrap_or_else(|_| parsed.to_string()))
        }
        Err(e) => Err(e.to_string()),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn node(config: Config) -> Node {
        Node::new(config, None, None)
    }

    #[test]
    fn the_prompt_flag_wins_and_the_payload_is_the_fallback() {
        let n = node(Config { prompt: Some("Summarise: hello".into()), ..Default::default() });
        assert_eq!(n.goal(&json!({ "message": "ignored" })).unwrap(), "Summarise: hello");

        let n = node(Config::default());
        assert_eq!(n.goal(&json!({ "message": "from message" })).unwrap(), "from message");
        assert_eq!(n.goal(&json!({ "body": "from body" })).unwrap(), "from body");
        assert_eq!(n.goal(&json!("bare string")).unwrap(), "bare string");
        let err = n.goal(&json!({ "other": 1 })).unwrap_err();
        assert_eq!(err.code, "FW_NODE_AGENT_QUERY");
    }

    #[test]
    fn a_malformed_schema_is_refused_not_ignored() {
        let n = node(Config { schema: Some("{not json".into()), ..Default::default() });
        let err = n.schema().unwrap_err();
        assert_eq!(err.code, "FW_NODE_AGENT_BAD_SCHEMA");
        assert!(err.message.contains("single quotes"));
        let n = node(Config { schema: Some("   ".into()), ..Default::default() });
        assert!(n.schema().unwrap().is_none());
    }

    #[test]
    fn no_credential_is_a_refusal_naming_the_kinds() {
        let n = node(Config::default());
        let err = match n.build_llm("o", "p") {
            Ok(_) => panic!("no credential must be refused"),
            Err(e) => e,
        };
        assert_eq!(err.code, "FW_NODE_AGENT_CREDENTIAL");
        assert!(err.message.contains("openai or openrouter"), "{}", err.message);
    }

    #[test]
    fn no_tools_are_offered_unless_named() {
        let n = node(Config::default());
        assert!(n.build_tool_defs("o", "p").is_empty());
        let n = node(Config { tools: vec!["lookup-order".into()], ..Default::default() });
        // No platform → no function pipelines to resolve against, and nothing else exists.
        assert!(n.build_tool_defs("o", "p").is_empty());
    }

    #[test]
    fn the_definition_documents_the_single_loop() {
        let def = definition();
        let flags: Vec<&str> = def.dsl_flags.iter().map(|f| f.flag.as_str()).collect();
        assert!(flags.contains(&"--prompt"));
        assert!(flags.contains(&"--schema"));
        assert!(flags.contains(&"--budget"));
        assert!(!flags.contains(&"--mode"));
        assert!(!flags.contains(&"--max-iterations"));
        assert!(!def.description.contains("plan"));
        let tools = def.dsl_flags.iter().find(|f| f.flag == "--tools").unwrap();
        assert!(tools.description.contains("Nothing is offered unless named"));
    }
}
