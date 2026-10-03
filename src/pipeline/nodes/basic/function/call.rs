//! `n.function.call` — invokes a function pipeline by slug.
//!
//! # Pipeline position
//!
//! Middleware node. Calls another active pipeline that starts with `n.trigger.function`.
//! On success routes through `out`; on failure (pipeline not found, execution error)
//! routes through `error`.
//!
//! # User-facing config
//! | Field | Type | Required | Description |
//! |---|---|---|---|
//! | `function` | string | yes | Slug of the function pipeline to call |
//! | `input` | any | no | What the function receives — a literal (JSON is parsed) or `{{ expr }}`. Omit to pass the whole payload |
//!
//! # DSL
//! ```text
//! | function.call --function my-fn --input-value "{{ input.body }}"
//! | function.call --function my-fn --input '{"user_id": "abc"}'
//! ```
//!
//! # Input/output
//! - **Input:** any payload (or sub-section via `input_path`, or static `input`)
//! - **Output `out`:** the function pipeline's last node output value
//! - **Output `error`:** `{ "error": "..." }` on failure

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::pipeline::model::NodeCapability;
use crate::pipeline::model::{
    DslFlag, DslFlagKind, NodeFieldDataSource, NodeFieldDef, NodeFieldType,
};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;

pub const NODE_KIND: &str = "n.function.call";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// Slug of the function pipeline to call (matches `PipelineMeta.name`).
    pub function: Option<String>,
    /// What the called function receives — a literal or `{{ expr }}`,
    /// arriving final. A literal that is JSON is parsed; null (unset) means
    /// the whole payload.
    #[serde(default)]
    pub input: serde_json::Value,
}

pub struct Node {
    config: Config,
    platform: Option<Arc<PlatformService>>,
}

impl Node {
    pub fn new(config: Config, platform: Option<Arc<PlatformService>>) -> Self {
        Self { config, platform }
    }
}

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Process],
        title: "Call Function".to_string(),
        description: "Calls another active pipeline that starts with `trigger.function`. `--function` is that pipeline's slug — the file stem, \
            `send-welcome` for `jobs/send-welcome`, not the path. What it receives: `--input` (a literal — JSON is parsed — or `{{ expr }}`, e.g. \
            `\"{{ { email: input.body.email } }}\"`), else the whole current payload. `out` adds `result` — the function's last node's payload — \
            and keeps the caller's payload. `error` adds `error: { code, message }` when the function is missing, inactive or failed."
            .to_string(),
        input_pins: vec!["in".to_string()],
        output_pins: vec!["out".to_string(), "error".to_string()],
        output_schema: serde_json::json!({
            "description": "On `out`: the payload plus `result`, the called function's last node payload. On `error`: the payload plus `error: { code, message }`.",
            "properties": { "result": {}, "error": { "type": "object" } }
        }),
        config_schema: serde_json::json!({
            "type": "object",
            "required": ["function"],
            "properties": {
                "function": {
                    "type": "string",
                    "description": "Slug of the function pipeline to call."
                },
                "input": {
                    "description": "What the function receives: a literal (JSON is parsed) or {{ expr }}. Omit to pass the whole payload."
                }
            }
        }),
        dsl_flags: vec![
            DslFlag {
                flag: "--function".to_string(),
                config_key: "function".to_string(),
                description: "Slug of the function pipeline to call.".to_string(),
                kind: DslFlagKind::Scalar,
                required: true,
            },
            DslFlag {
                flag: "--input".to_string(),
                config_key: "input".to_string(),
                description: "What the function receives — a literal (JSON is parsed) or {{ expr }}, e.g. \"{{ { email: input.body.email } }}\". Omit to pass the whole payload.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: vec![
            NodeFieldDef {
                name: "function".to_string(),
                label: "Function".to_string(),
                field_type: NodeFieldType::Datalist,
                data_source: Some(NodeFieldDataSource::FunctionPipelines),
                placeholder: Some("select or type a function slug".to_string()),
                help: Some("Slug of the function pipeline to invoke.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "input".to_string(),
                label: "Input".to_string(),
                field_type: NodeFieldType::Text,
                placeholder: Some("{{ { email: input.body.email } }}  (leave empty for the full payload)".to_string()),
                help: Some("What the function receives: a literal (JSON is parsed) or {{ expr }}. Leave empty to pass the full payload.".to_string()),
                ..Default::default()
            },
        ],
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Reuse a lookup function", r#"function.call --function find-user --input "{{ { email: input.body.email } }}""#)
                .output(serde_json::json!({ "user": { "_key": "u_1", "email": "a@x.io" } }))
                .note("Whatever `jobs/find-user`'s last node produced."),
        ],
        ..Default::default()
    }
}

/// What the called function receives.
///
/// `input` arrives final — a whole `{{ }}` carries its typed value (NodeIO
/// §Value resolution), and a literal string that is JSON is parsed. Null
/// means "not set", so the whole payload goes.
///
/// The pointer this replaces had a trap worth naming: a path that matched
/// nothing fell back to the *entire payload*, so a typo silently handed the
/// function everything instead of the one field it asked for. An expression
/// that matches nothing is null, and null is visible.
fn extract_payload_input(
    input: &serde_json::Value,
    payload: serde_json::Value,
) -> serde_json::Value {
    match input {
        serde_json::Value::Null => payload,
        serde_json::Value::String(raw) if raw.trim().is_empty() => payload,
        serde_json::Value::String(raw) => {
            serde_json::from_str(raw.trim()).unwrap_or_else(|_| input.clone())
        }
        other => other.clone(),
    }
}

#[async_trait]
impl NodeHandler for Node {
    fn kind(&self) -> &'static str {
        NODE_KIND
    }

    fn input_pins(&self) -> &'static [&'static str] {
        &["in"]
    }

    fn output_pins(&self) -> &'static [&'static str] {
        &["out", "error"]
    }

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        let platform = match &self.platform {
            Some(p) => p.clone(),
            None => {
                return Err(PipelineError::new(
                    "FW_NODE_FUNCTION_CALL_NO_PLATFORM",
                    "function.call: platform not injected into engine",
                ));
            }
        };

        let slug = match &self.config.function {
            Some(s) if !s.is_empty() => s.clone(),
            _ => {
                return Err(PipelineError::new("FW_NODE_FUNCTION_CALL_CONFIG", "--function is required"));
            }
        };

        let owner = input
            .metadata
            .get("owner")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let project = input
            .metadata
            .get("project")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        let call_input = extract_payload_input(&self.config.input, input.payload.clone());

        match platform
            .execute_function_pipeline(&owner, &project, &slug, call_input)
            .await
        {
            Ok(result) => Ok(NodeExecutionOutput {
                output_pins: vec!["out".to_string()],
                payload: crate::pipeline::nodes::shared::util::with_answer(&input.payload, serde_json::json!({ "result": result })),
                trace: vec![format!("function.call: '{}' ok", slug)],
            }),
            Err(e) => Ok(NodeExecutionOutput {
                output_pins: vec!["error".to_string()],
                payload: crate::pipeline::nodes::shared::util::with_answer(
                    &input.payload,
                    serde_json::json!({ "error": { "code": e.code, "message": e.message } }),
                ),
                trace: vec![format!(
                    "function.call: '{}' error: {} — {}",
                    slug, e.code, e.message
                )],
            }),
        }
    }
}
