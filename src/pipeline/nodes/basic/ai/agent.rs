//! `ai.text.generate` — one model loop: prompt in, text out, tools when named.
//!
//! | Use | DSL |
//! |---|---|
//! | One call, text back | `\| ai.text.generate --provider openai --credential openai_main --prompt "Summarise: {{ $trigger.body.text }}"` |
//! | One call, JSON back | `\| ai.text.generate --provider openai --credential openai_main --schema '{"type":"object","required":["sentiment"]}' -- Classify: {{ $trigger.body.review }}` |
//! | Tools, until it answers | `\| ai.text.generate --provider openrouter --credential router_main --tool lookup-order --tool list-slots --budget 8 --prompt "{{ $trigger.body.message }}"` |
//!
//! The loop is the one every coding agent runs: send the messages with the
//! tool definitions, run each tool call the model returns, append the results,
//! call again, stop when the model answers with text. With no tools named
//! there is nothing to call, so it is one round trip. `--budget` caps the
//! number of model calls; `--schema` and `--verify` gate the answer and feed a
//! failure back for repair.
//!
//! # Provider and credential (`node-conventions.md` §11)
//!
//! `--provider` (literal, `openai` or `openrouter`) picks the API; each
//! provider's profile names the credential kind that holds its key, so a
//! `--credential` of another provider's kind is refused. The credential is
//! the only source of an LLM secret; its secret holds the key, base URL and
//! default model.
//!
//! # Tools
//!
//! A tool is one of the project's **function pipelines** (`trigger.function`),
//! named by `--tool` (repeat). Nothing is offered unless named. There are no
//! shell tools and no built-in node tools: what the agent can do is what the
//! project wrote as a function pipeline, which is visible, reviewable and
//! guarded like any other route.
//!
//! # Answer
//!
//! `text: { value, verified, data?, tools_called, iterations,
//! budget_exhausted, chain, tool_events, metrics }`, the payload kept;
//! `--answer-only` keeps `value`, `verified` and `data`.

use std::collections::BTreeMap;
use std::sync::{Arc, LazyLock};

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
    ProviderProfile, Signal,
};
use crate::pipeline::nodes::shared::profile::{check_credential, check_profile, option_flag, provider_flag};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::CredentialService;
use crate::platform::services::platform::PlatformService;

pub const NODE_KIND: &str = "ai.text.generate";
const INPUT_PIN: &str = "in";
const OUTPUT_PIN: &str = "out";
const DEFAULT_BUDGET: u32 = 10;
const DEFAULT_MAX_REPAIRS: u32 = 1;
/// The most function pipelines one node offers the model.
pub const MAX_TOOLS: u32 = 32;

const CONFIG_CODE: &str = "FW_NODE_AI_TEXT_GENERATE_CONFIG";
const CREDENTIAL_CODE: &str = "FW_NODE_AI_TEXT_GENERATE_CREDENTIAL";

/// The providers, each with the credential kind that holds its key. Both
/// speak the OpenAI wire format; `openai` the Responses API, `openrouter`
/// Chat Completions.
pub const PROVIDERS: &[&str] = &["openai", "openrouter"];

fn profiles() -> Vec<ProviderProfile> {
    PROVIDERS
        .iter()
        .map(|provider| ProviderProfile {
            provider: provider.to_string(),
            credential_kinds: vec![provider.to_string()],
            ..Default::default()
        })
        .collect()
}

/// The definition, built once: every build of the node checks its config
/// against the profiles here.
fn checked_definition() -> &'static NodeDefinition {
    static DEFINITION: LazyLock<NodeDefinition> = LazyLock::new(definition);
    &DEFINITION
}

/// The config against the chosen provider's profile.
pub fn check_config_profile(config: &Value) -> Result<(), PipelineError> {
    check_profile(checked_definition(), config).map_err(|message| PipelineError::new(CONFIG_CODE, message))
}

fn flag(name: &str, key: &str, description: &str, value: &str) -> DslFlag {
    DslFlag {
        flag: name.to_string(),
        config_key: key.to_string(),
        description: description.to_string(),
        kind: DslFlagKind::Scalar,
        value: value.to_string(),
        ..Default::default()
    }
}

pub fn definition() -> NodeDefinition {
    use crate::pipeline::model::{NodeFieldDef, SelectOptionDef};
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Network, NodeCapability::Credential],
        title: "AI Text".to_string(),
        description: "Asks a language model for text, optionally letting it call the project's function pipelines as tools. One model loop: the prompt goes to the model with the named tools; each tool call the model \
            makes is run and fed back; it stops when the model answers with text or the budget is spent. \
            With no --tool it is one call — classify, extract, summarise, draft. With --schema the answer is \
            parsed as JSON, checked, repaired once on failure and returned as `text.data`. Tools are the project's \
            function pipelines, named by --tool; nothing is offered unless named. Adds `text: { value, verified, data, … }` \
            and keeps the payload. Step signals (thinking, tool_call, tool_result, final) stream over SSE when the \
            webhook is called with Accept: text/event-stream."
            .to_string(),
        input_schema: json!({
            "type": "object",
            "description": "Payload the {{ expr }} in --prompt and --system-prompt resolve against. The node reads nothing else of it."
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "object",
                    "properties": {
                        "value":            { "type": "string",  "description": "The model's final answer as text." },
                        "verified":         { "type": "boolean", "description": "True when no contract was set, or the answer passed --schema and --verify." },
                        "data":             { "description": "With --schema: the answer parsed as JSON (present only when it passed the check)." },
                        "tools_called":     { "type": "array",   "description": "Function pipelines the model called, in order (not with --answer-only)." },
                        "iterations":       { "type": "number",  "description": "Model calls made (not with --answer-only)." },
                        "budget_exhausted": { "type": "boolean", "description": "True when the budget ended the loop before an answer (not with --answer-only)." },
                        "chain":            { "type": "array",   "description": "Step log (not with --answer-only)." },
                        "tool_events":      { "type": "array",   "description": "Each tool call with arguments and result (not with --answer-only)." },
                        "metrics":          { "type": "object",  "description": "llm_calls, tool_calls, prompt_tokens, completion_tokens, wall_ms, stop_reason (not with --answer-only)." }
                    }
                }
            }
        }),
        input_pins: vec![INPUT_PIN.to_string()],
        output_pins: vec![OUTPUT_PIN.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: json!({
            "type": "object",
            "properties": {
                "provider":      { "type": "string", "enum": PROVIDERS, "description": "Who answers: openai or openrouter. Literal." },
                "credential_id": { "type": "string", "description": "Credential of the provider's kind." },
                "prompt":        { "type": "string", "description": "The prompt — literal or {{ expr }}." },
                "system_prompt": { "type": "string", "description": "Standing instructions — literal or {{ expr }}." },
                "tool":          { "type": "array", "items": { "type": "string" }, "description": "Function pipelines the model may call. Empty = no tools." },
                "budget":        { "type": "integer", "description": "Maximum model calls (default 10)." },
                "schema":        { "type": "string", "description": "JSON Schema (subset) the answer must satisfy; the parsed answer is returned as `data`." },
                "max_repairs":   { "type": "integer", "description": "Repair rounds when --schema or --verify fails (default 1)." },
                "verify":        { "type": "string", "description": "A function pipeline that checks the answer: receives {candidate, goal}, returns {pass, reason}." },
                "model":         { "type": "string", "description": "Model id, overriding the credential's default." },
                "answer_only":   { "type": "boolean", "description": "Answer value, verified and data only." },
                "option":        { "type": "object", "description": "Provider settings, closed by the provider's profile." }
            },
            "required": ["provider", "credential_id", "prompt"]
        }),
        dsl_flags: vec![
            provider_flag(PROVIDERS, "Who answers: openai (Responses API) or openrouter (Chat Completions). Literal, never {{ }}."),
            DslFlag { required: true, ..flag("--credential", "credential_id", "Credential holding the provider's key: kind openai for openai, openrouter for openrouter. The only source of the LLM key.", "text") },
            DslFlag { required: true, ..flag("--prompt", "prompt", "The prompt — literal or {{ expr }}; or write it after `--`.", "text") },
            flag("--system-prompt", "system_prompt", "Standing instructions for every call — literal or {{ expr }}.", "text"),
            DslFlag {
                kind: DslFlagKind::RepeatedList,
                max_repeat: Some(MAX_TOOLS),
                ..flag("--tool", "tool", "A function pipeline the model may call, by name — repeat for several. Nothing is offered unless named; with no tool the node is one call.", "text")
            },
            flag("--budget", "budget", "Maximum model calls before the loop stops (default 10). Repairs count.", "number"),
            flag("--schema", "schema", "JSON Schema (type, required, properties, items, enum, minItems, minLength) the answer must satisfy; the parsed answer is `text.data`. Quote with single quotes in the DSL.", "json"),
            flag("--max-repairs", "max_repairs", "How many times a failed --schema/--verify is fed back for another attempt (default 1).", "number"),
            flag("--verify", "verify", "A function pipeline that checks the answer: receives {candidate, goal}, returns {pass, reason}. Runs after the schema check.", "text"),
            flag("--model", "model", "Model id, overriding the credential's default.", "text"),
            DslFlag { kind: DslFlagKind::Bool, ..flag("--answer-only", "answer_only", "Answer only value, verified and data: no step log, tool events or metrics.", "") },
            option_flag(),
        ],
        profiles: profiles(),
        fields: vec![
            NodeFieldDef {
                name: "provider".to_string(),
                label: "Provider".to_string(),
                field_type: NodeFieldType::Select,
                options: PROVIDERS.iter().map(|p| SelectOptionDef { value: p.to_string(), label: p.to_string() }).collect(),
                help: Some("Who answers. The credential must be of this provider's kind.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "credential_id".to_string(),
                label: "Credential".to_string(),
                field_type: NodeFieldType::Select,
                data_source: Some(NodeFieldDataSource::CredentialsOpenAi),
                help: Some("Credential of the provider's kind; its secret holds the key.".to_string()),
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
                help: Some("Literal or {{ expr }}. Required.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "tool".to_string(),
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
                name: "answer_only".to_string(),
                label: "Answer only".to_string(),
                field_type: NodeFieldType::Checkbox,
                help: Some("Answer value, verified and data only — no step log, tool events or metrics.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "schema".to_string(),
                label: "Answer Schema (JSON)".to_string(),
                field_type: NodeFieldType::CodeEditor,
                language: Some("json".to_string()),
                help: Some("When set, the answer must be JSON matching this schema (type, required, properties, items, enum, minItems, minLength) and is returned as `text.data`.".to_string()),
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
                name: "verify".to_string(),
                label: "Verifier (function pipeline)".to_string(),
                field_type: NodeFieldType::Select,
                data_source: Some(NodeFieldDataSource::FunctionPipelines),
                help: Some("Receives {candidate, goal}, returns {pass, reason}. Runs after the schema check; a fail is fed back as a repair.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "option".to_string(),
                label: "Provider options".to_string(),
                field_type: NodeFieldType::KeyValuePairs,
                help: Some("Settings of the chosen provider, key=value; only the keys its profile names.".to_string()),
                ..Default::default()
            },
        ],
        layout: vec![
            LayoutItem::Row { row: vec![LayoutItem::Field("provider".to_string()), LayoutItem::Field("credential_id".to_string()), LayoutItem::Field("model".to_string())] },
            LayoutItem::Field("system_prompt".to_string()),
            LayoutItem::Field("prompt".to_string()),
            LayoutItem::Field("tool".to_string()),
            LayoutItem::Row { row: vec![LayoutItem::Field("budget".to_string()), LayoutItem::Field("answer_only".to_string())] },
            LayoutItem::Field("schema".to_string()),
            LayoutItem::Row { row: vec![LayoutItem::Field("max_repairs".to_string()), LayoutItem::Field("verify".to_string())] },
            LayoutItem::Field("option".to_string()),
        ],
        ai_tool: NodeAiToolDefinition::default(),
        failure_semantics: vec![
            crate::pipeline::model::NodeFailureSemantic { code: CONFIG_CODE.into(), description: "The config breaks the provider's profile (an unknown or {{ }} --provider, an --option it does not take), or a value has the wrong type.".into(), ..Default::default() },
            crate::pipeline::model::NodeFailureSemantic { code: CREDENTIAL_CODE.into(), description: "`--credential` is missing, names no credential, or names one whose kind is not the provider's.".into(), ..Default::default() },
            crate::pipeline::model::NodeFailureSemantic { code: "FW_NODE_AI_TEXT_GENERATE_PROMPT".into(), description: "`--prompt` (or the `--` body) resolved empty.".into(), ..Default::default() },
            crate::pipeline::model::NodeFailureSemantic { code: "FW_NODE_AI_TEXT_GENERATE_SCHEMA".into(), description: "`--schema` is set but is not valid JSON (quote it with single quotes in the DSL).".into(), ..Default::default() },
            crate::pipeline::model::NodeFailureSemantic { code: "FW_NODE_AI_TEXT_GENERATE_CALL".into(), description: "The provider refused or the request failed; the message carries the provider's text.".into(), retryable: true, retry_hint: "429 and 5xx recover on retry; a 401 needs a new key.".into() },
        ],
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("One call: summarise a submission", r#"ai.text.generate --provider openai --credential openai_main --system-prompt "You write one plain sentence." --prompt "Summarise: {{ $trigger.body.text }}""#)
                .input(json!({ "webhook": { "body": { "text": "Our clinic moved to 12 High St and now opens Saturdays 9–1." } } }))
                .output(json!({ "webhook": { "body": { "text": "Our clinic moved to 12 High St and now opens Saturdays 9–1." } }, "text": { "value": "The clinic has moved to 12 High St and now opens on Saturday mornings.", "verified": true, "tools_called": [], "iterations": 1, "budget_exhausted": false } }))
                .note("No --tool, so one round trip. The answer is added under `text` and the rest of the payload is kept. The credential is created by the owner in Studio → Credentials."),
            crate::pipeline::model::NodeExample::dsl("One call: classify to JSON", r#"ai.text.generate --provider openai --credential openai_main --answer-only --schema '{"type":"object","required":["sentiment"],"properties":{"sentiment":{"enum":["positive","neutral","negative"]}}}' -- Classify this review: {{ $trigger.body.review }}"#)
                .input(json!({ "webhook": { "body": { "review": "Booking was easy but the wait was long." } } }))
                .output(json!({ "webhook": { "body": { "review": "Booking was easy but the wait was long." } }, "text": { "value": "{\"sentiment\":\"neutral\"}", "data": { "sentiment": "neutral" }, "verified": true } }))
                .note("`text.data` is the parsed, checked answer; branch on it with `logic.if --when \"$nodes.a.text.data.sentiment == 'negative'\"`. A failed check is fed back once (--max-repairs)."),
            crate::pipeline::model::NodeExample::dsl("Tools: answer from the project's data", r#"ai.text.generate --provider openrouter --credential router_main --tool lookup-order --budget 6 --system-prompt "You answer questions about orders. Use the tools; never guess." --prompt "{{ $trigger.body.message }}""#)
                .input(json!({ "webhook": { "body": { "message": "Where is order o_91?" } } }))
                .output(json!({ "webhook": { "body": { "message": "Where is order o_91?" } }, "text": { "value": "Order o_91 shipped yesterday and arrives Friday.", "verified": true, "tools_called": ["lookup-order"], "iterations": 2, "budget_exhausted": false } }))
                .note("`lookup-order` is a function pipeline (trigger.function) in this project; the model calls it with the arguments its input schema declares."),
        ],
        ..Default::default()
    }
}

// ── Config ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Config {
    /// `openai` or `openrouter`; checked against the profiles before this
    /// is read.
    #[serde(default)]
    pub provider: String,
    /// Credential of the provider's kind — the only source of the key.
    #[serde(default)]
    pub credential_id: Option<String>,
    /// Model id overriding the credential's default.
    #[serde(default)]
    pub model: Option<String>,
    /// The prompt, literal or `{{ expr }}` (resolved by the engine).
    #[serde(default)]
    pub prompt: Option<String>,
    /// Standing instructions for every call.
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// Function pipelines the model may call: repeated, or one list.
    #[serde(default)]
    pub tool: Value,
    /// Maximum model calls (default 10).
    #[serde(default)]
    pub budget: u32,
    /// Answer `value`, `verified` and `data` only.
    #[serde(default)]
    pub answer_only: bool,
    /// JSON Schema (subset) the answer must satisfy. Stored as text (DSL flag
    /// / UI editor); parsed at run time, refused when malformed.
    #[serde(default)]
    pub schema: Option<String>,
    /// Repair rounds when the schema or verifier fails (default 1).
    #[serde(default)]
    pub max_repairs: Option<u32>,
    /// A function pipeline that checks the answer: receives
    /// `{candidate, goal}`, returns `{pass, reason}`.
    #[serde(default)]
    pub verify: Option<String>,
    /// Provider settings; the profile has already closed the keys.
    #[serde(default)]
    pub option: BTreeMap<String, Value>,
}

/// A list flag's values, a list inside the list flattened, empties dropped.
fn listed(value: &Value) -> Vec<String> {
    match value {
        Value::Null => Vec::new(),
        Value::Array(items) => items.iter().flat_map(listed).collect(),
        Value::String(s) if s.trim().is_empty() => Vec::new(),
        Value::String(s) => vec![s.trim().to_string()],
        other => vec![other.to_string()],
    }
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

    /// The node from its stored (or resolved) config: held to the chosen
    /// provider's profile first, then read.
    pub fn build(
        config: &Value,
        credentials: Option<Arc<CredentialService>>,
        platform: Option<Arc<PlatformService>>,
    ) -> Result<Self, PipelineError> {
        check_config_profile(config)?;
        let config: Config =
            serde_json::from_value(config.clone()).map_err(|err| PipelineError::new(CONFIG_CODE, err.to_string()))?;
        let tools = listed(&config.tool);
        if tools.len() > MAX_TOOLS as usize {
            return Err(PipelineError::new(
                CONFIG_CODE,
                format!("--tool names {} function pipelines; one node offers at most {MAX_TOOLS}", tools.len()),
            ));
        }
        Ok(Self::new(config, credentials, platform))
    }

    /// Resolve the LLM client from the node's credential — the only source of
    /// an LLM secret. A missing credential, or one whose kind is not the
    /// provider's, is a refusal, never a silent fallback.
    fn build_llm(&self, owner: &str, project: &str) -> Result<Arc<dyn LlmCall>, PipelineError> {
        let provider = self.config.provider.trim();
        let model_override = self.config.model.as_deref().map(str::trim).filter(|m| !m.is_empty());
        let cred_id = self
            .config
            .credential_id
            .as_deref()
            .map(str::trim)
            .filter(|c| !c.is_empty())
            .ok_or_else(|| {
                PipelineError::new(
                    CREDENTIAL_CODE,
                    format!("{NODE_KIND} needs --credential naming a credential of kind {provider}"),
                )
            })?;
        let creds = self.credentials.as_ref().ok_or_else(|| {
            PipelineError::new(CREDENTIAL_CODE, "credential service is not configured on this engine")
        })?;
        let cred = creds
            .get_project_credential(owner, project, cred_id)
            .map_err(|err| PipelineError::new(CREDENTIAL_CODE, err.to_string()))?
            .ok_or_else(|| PipelineError::new(CREDENTIAL_CODE, format!("credential '{cred_id}' not found")))?;
        check_credential(checked_definition(), provider, cred_id, &cred.kind)
            .map_err(|message| PipelineError::new(CREDENTIAL_CODE, message))?;
        client_from_provider_secret_with_model(provider, &cred.secret, model_override).ok_or_else(|| {
            PipelineError::new(CREDENTIAL_CODE, format!("credential '{cred_id}' ({}) has no api_key", cred.kind))
        })
    }

    /// The tool definitions offered to the model: the project's active
    /// function pipelines named by `--tool`. Nothing else.
    fn build_tool_defs(&self, owner: &str, project: &str) -> Vec<ToolDef> {
        let wanted = listed(&self.config.tool);
        if wanted.is_empty() {
            return Vec::new();
        }
        let Some(platform) = &self.platform else {
            return Vec::new();
        };
        use crate::platform::services::project::name_from_file_rel_path;
        const FN_TRIGGER: &str = "trigger.function";

        let mut defs = Vec::new();
        for compiled in platform.pipeline_runtime.list_project(owner, project) {
            let Some(trigger) = compiled.graph.nodes.iter().find(|n| n.kind == FN_TRIGGER) else {
                continue;
            };
            let slug = name_from_file_rel_path(&compiled.file_rel_path);
            if !wanted.contains(&slug) {
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
        const FN_TRIGGER: &str = "trigger.function";
        use crate::platform::services::project::name_from_file_rel_path;
        platform
            .pipeline_runtime
            .list_project(owner, project)
            .into_iter()
            .filter(|c| c.graph.nodes.iter().any(|n| n.kind == FN_TRIGGER))
            .map(|c| name_from_file_rel_path(&c.file_rel_path))
            .collect()
    }

    /// The prompt: `--prompt` or the `--` body, resolved. Every source is
    /// explicit, so an empty one is refused rather than read off the payload.
    fn prompt(&self) -> Result<String, PipelineError> {
        self.config
            .prompt
            .as_deref()
            .map(str::trim)
            .filter(|p| !p.is_empty())
            .map(str::to_string)
            .ok_or_else(|| {
                PipelineError::new("FW_NODE_AI_TEXT_GENERATE_PROMPT", "--prompt is empty; it needs the prompt (or write it after `--`)")
            })
    }

    /// The answer schema, parsed. An absent or empty schema is no contract; a
    /// non-empty one that does not parse is refused — running unverified while
    /// the author believes verification is on would be a fail-open hole.
    fn schema(&self) -> Result<Option<Value>, PipelineError> {
        match self.config.schema.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            Some(raw) => serde_json::from_str::<Value>(raw).map(Some).map_err(|err| {
                PipelineError::new(
                    "FW_NODE_AI_TEXT_GENERATE_SCHEMA",
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
                "FW_NODE_AI_TEXT_GENERATE_INPUT_PIN",
                format!("unsupported input pin '{}'", input.input_pin),
            ));
        }
        let owner = input.metadata.get("owner").and_then(|v| v.as_str()).unwrap_or("");
        let project = input.metadata.get("project").and_then(|v| v.as_str()).unwrap_or("");

        let goal = self.prompt()?;
        let schema = self.schema()?;
        let llm = self.build_llm(owner, project)?;
        let tools = self.build_tool_defs(owner, project);
        let has_contract = schema.is_some()
            || self.config.verify.as_deref().map(str::trim).filter(|s| !s.is_empty()).is_some();

        let agent = ZebtuneAgent::new(
            ZebtuneConfig {
                step_budget: if self.config.budget == 0 { DEFAULT_BUDGET } else { self.config.budget },
                system_prompt: self.config.system_prompt.clone(),
                output_mode: if self.config.answer_only {
                    ZebtuneOutputMode::FinalOnly
                } else {
                    ZebtuneOutputMode::Full
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
            .verify
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
            return Err(PipelineError::new("FW_NODE_AI_TEXT_GENERATE_CALL", result.final_content));
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

        let mut text = json!({
            "value": result.final_content,
            "verified": result.verified,
        });
        if let Some(data) = data {
            text["data"] = data;
        }
        if !self.config.answer_only {
            text["tools_called"] = json!(tools_called);
            text["iterations"] = json!(result.metrics.llm_calls);
            text["budget_exhausted"] = json!(result.budget_exhausted);
            text["chain"] = json!(result.chain);
            text["tool_events"] = json!(result.tool_events);
            text["metrics"] = json!(result.metrics);
        }

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN.to_string()],
            payload: crate::pipeline::nodes::shared::util::with_answer(&input.payload, json!({ "text": text })),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn node(config: Config) -> Node {
        Node::new(config, None, None)
    }

    fn build(config: Value) -> Result<Node, PipelineError> {
        Node::build(&config, None, None)
    }

    #[test]
    fn the_prompt_is_explicit_and_never_read_off_the_payload() {
        let n = node(Config { prompt: Some("Summarise: hello".into()), ..Default::default() });
        assert_eq!(n.prompt().unwrap(), "Summarise: hello");
        let err = node(Config::default()).prompt().unwrap_err();
        assert_eq!(err.code, "FW_NODE_AI_TEXT_GENERATE_PROMPT");
        let err = node(Config { prompt: Some("   ".into()), ..Default::default() }).prompt().unwrap_err();
        assert_eq!(err.code, "FW_NODE_AI_TEXT_GENERATE_PROMPT");
    }

    #[test]
    fn a_malformed_schema_is_refused_not_ignored() {
        let n = node(Config { schema: Some("{not json".into()), ..Default::default() });
        let err = n.schema().unwrap_err();
        assert_eq!(err.code, "FW_NODE_AI_TEXT_GENERATE_SCHEMA");
        assert!(err.message.contains("single quotes"));
        let n = node(Config { schema: Some("   ".into()), ..Default::default() });
        assert!(n.schema().unwrap().is_none());
    }

    /// `--provider` is literal and closed; an `--option` neither provider's
    /// profile names is refused naming what it does take.
    #[test]
    fn the_profile_is_checked_when_the_node_is_built() {
        assert!(build(json!({ "provider": "openai", "credential_id": "c", "prompt": "hi" })).is_ok());
        for (config, says) in [
            (json!({ "prompt": "hi" }), "needs --provider: one of openai, openrouter"),
            (json!({ "provider": "anthropic", "prompt": "hi" }), "'anthropic' is not one of openai, openrouter"),
            (json!({ "provider": "{{ input.who }}", "prompt": "hi" }), "literal"),
            (json!({ "provider": "openai", "prompt": "hi", "option": { "temperature": "0.2" } }), "it takes no --option"),
        ] {
            let err = build(config).err().expect("refused");
            assert_eq!(err.code, "FW_NODE_AI_TEXT_GENERATE_CONFIG");
            assert!(err.message.contains(says), "{}", err.message);
        }
        let many: Vec<String> = (0..=MAX_TOOLS).map(|i| format!("t{i}")).collect();
        let err = build(json!({ "provider": "openai", "prompt": "hi", "tool": many })).err().expect("refused");
        assert!(err.message.contains("at most 32"), "{}", err.message);
    }

    #[test]
    fn no_credential_is_a_refusal_naming_the_providers_kind() {
        let n = node(Config { provider: "openrouter".into(), ..Default::default() });
        let err = match n.build_llm("o", "p") {
            Ok(_) => panic!("no credential must be refused"),
            Err(e) => e,
        };
        assert_eq!(err.code, "FW_NODE_AI_TEXT_GENERATE_CREDENTIAL");
        assert!(err.message.contains("of kind openrouter"), "{}", err.message);
    }

    #[test]
    fn no_tools_are_offered_unless_named() {
        let n = node(Config::default());
        assert!(n.build_tool_defs("o", "p").is_empty());
        let n = node(Config { tool: json!(["lookup-order"]), ..Default::default() });
        // No platform → no function pipelines to resolve against, and nothing else exists.
        assert!(n.build_tool_defs("o", "p").is_empty());
    }

    #[test]
    fn the_definition_documents_the_single_loop() {
        let def = definition();
        let flags: Vec<&str> = def.dsl_flags.iter().map(|f| f.flag.as_str()).collect();
        assert_eq!(
            flags,
            ["--provider", "--credential", "--prompt", "--system-prompt", "--tool", "--budget", "--schema", "--max-repairs", "--verify", "--model", "--answer-only", "--option"]
        );
        let tool = def.dsl_flags.iter().find(|f| f.flag == "--tool").unwrap();
        assert!(tool.description.contains("Nothing is offered unless named"));
        assert_eq!(
            crate::pipeline::nodes::provider_signature(&def, "openrouter").as_deref(),
            Some("ai.text.generate --provider openrouter --credential TEXT --prompt TEXT [--system-prompt TEXT] [--tool TEXT…] [--budget N] [--schema JSON] [--max-repairs N] [--verify TEXT] [--model TEXT] [--answer-only] → text")
        );
    }

    /// A local stand-in for the provider: one Chat Completions answer.
    async fn stub_provider() -> (String, tokio::task::JoinHandle<()>) {
        use axum::{Json, Router, routing::post};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
        let addr = listener.local_addr().expect("addr");
        let app = Router::new().route(
            "/chat/completions",
            post(|| async {
                Json(json!({
                    "choices": [{ "finish_reason": "stop", "message": { "role": "assistant", "content": "Opens Saturdays." } }],
                    "usage": { "prompt_tokens": 3, "completion_tokens": 2 }
                }))
            }),
        );
        let server = tokio::spawn(async move { axum::serve(listener, app).await.expect("serve") });
        (format!("http://{addr}"), server)
    }

    fn credential(platform: &crate::pipeline::nodes::shared::test_platform::TestPlatform, id: &str, kind: &str, base_url: &str) {
        platform
            .credentials
            .upsert_project_credential(
                "superadmin",
                "default",
                &crate::platform::model::UpsertProjectCredentialRequest {
                    credential_id: id.to_string(),
                    title: id.to_string(),
                    kind: kind.to_string(),
                    // A generated value, never a real key.
                    secret: json!({ "api_key": uuid::Uuid::new_v4().to_string(), "base_url": base_url }),
                    notes: String::new(),
                },
            )
            .expect("credential");
    }

    async fn run(platform: &crate::pipeline::nodes::shared::test_platform::TestPlatform, config: Value) -> Result<NodeExecutionOutput, PipelineError> {
        let node = Node::build(&config, Some(platform.credentials.clone()), None)?;
        node.execute_async(NodeExecutionInput {
            node_id: "a".to_string(),
            input_pin: INPUT_PIN.to_string(),
            payload: json!({ "kept": 1 }),
            metadata: json!({ "owner": "superadmin", "project": "default", "pipeline": "t", "request_id": "r" }),
            bus: None,
        })
        .await
    }

    /// The answer is `text`, the payload kept; `--answer-only` drops the step
    /// log, the tool events and the metrics; a credential of the other
    /// provider's kind is refused before any call.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_answer_is_text_and_the_payload_is_kept() {
        let (base_url, server) = stub_provider().await;
        let platform = crate::pipeline::nodes::shared::test_platform::test_platform();
        credential(&platform, "router", "openrouter", &base_url);
        credential(&platform, "direct", "openai", &base_url);

        let full = run(&platform, json!({ "provider": "openrouter", "credential_id": "router", "prompt": "When?" })).await.expect("answers");
        assert_eq!(full.payload["kept"], 1);
        assert_eq!(full.payload["text"]["value"], "Opens Saturdays.");
        assert_eq!(full.payload["text"]["verified"], true);
        assert_eq!(full.payload["text"]["iterations"], 1);
        assert!(full.payload["text"]["metrics"].is_object() && full.payload["text"]["chain"].is_array());
        assert!(full.payload.get("response").is_none(), "nothing outside the answer key");

        let short = run(&platform, json!({ "provider": "openrouter", "credential_id": "router", "prompt": "When?", "answer_only": true })).await.expect("answers");
        assert_eq!(short.payload["text"], json!({ "value": "Opens Saturdays.", "verified": true }));

        let err = run(&platform, json!({ "provider": "openrouter", "credential_id": "direct", "prompt": "When?" })).await.err().expect("refused");
        assert_eq!(err.code, "FW_NODE_AI_TEXT_GENERATE_CREDENTIAL");
        assert!(err.message.contains("is kind 'openai'; --provider openrouter takes a credential of kind openrouter"), "{}", err.message);
        server.abort();
    }
}
