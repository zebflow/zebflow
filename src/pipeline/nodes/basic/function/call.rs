//! `function.result.call` — invokes a function pipeline by slug.
//!
//! # Pipeline position
//!
//! Middleware node. Calls another active pipeline that starts with `trigger.function`.
//! On success routes through `out`; on failure (pipeline not found, execution error)
//! routes through `error`.
//!
//! # User-facing config
//! | Field | Type | Required | Description |
//! |---|---|---|---|
//! | `function` | string | yes | Slug of the function pipeline to call |
//! | `argument` | map or `{{ object }}` | no | What the function receives — repeated `key=value`, or one `{{ object }}`. Omitted: no arguments (`{}`) |
//!
//! # DSL
//! ```text
//! | function.result.call --function my-fn --argument "email={{ $trigger.body.email }}" --argument plan=pro
//! | function.result.call --function my-fn --argument "{{ { user_id: input.webhook.params.id } }}"
//! ```
//!
//! # Input/output
//! - **Input:** any payload; only `--argument` reaches the function
//! - **Output `out`:** the payload plus `result`, the function pipeline's last node payload
//! - **Output `error`:** the payload plus `result: { ok: false, error: { code, message } }`

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

pub const NODE_KIND: &str = "function.result.call";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// Slug of the function pipeline to call (matches `PipelineMeta.name`).
    pub function: Option<String>,
    /// What the called function receives: a map from repeated
    /// `--argument key=value` (each value a literal or `{{ expr }}`), or one
    /// `{{ object }}`, arriving final. Unset means no arguments.
    #[serde(default)]
    pub argument: serde_json::Value,
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
            `send-welcome` for `jobs/send-welcome`, not the path. What it receives is `--argument`: repeated `key=value` (`--argument \"email={{ $trigger.body.email }}\" --argument plan=pro`) \
            or one `{{ object }}` (`--argument \"{{ { email: $trigger.body.email } }}\"`); without it the function receives `{}` and \
            answers under its trigger's key, `function`. `out` adds `result` — the function's last node's payload — and keeps the caller's \
            payload. `error` adds `result: { ok: false, error: { code, message } }` when the function is missing, inactive or failed."
            .to_string(),
        input_pins: vec!["in".to_string()],
        output_pins: vec!["out".to_string(), "error".to_string()],
        output_schema: serde_json::json!({
            "description": "On `out`: the payload plus `result`, the called function's last node payload. On `error`: the payload plus `result: { ok: false, error: { code, message } }`.",
            "properties": { "result": {} }
        }),
        config_schema: serde_json::json!({
            "type": "object",
            "required": ["function"],
            "properties": {
                "function": {
                    "type": "string",
                    "description": "Slug of the function pipeline to call."
                },
                "argument": {
                    "description": "What the function receives: a map of key=value (each a literal or {{ expr }}), or one {{ object }}. Omitted: {}."
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
                value: "text".to_string(),
                ..Default::default()
            },
            DslFlag {
                flag: "--argument".to_string(),
                config_key: "argument".to_string(),
                description: "What the function receives, repeated key=value (each a literal or {{ expr }}), or one {{ object }}: \"email={{ $trigger.body.email }}\". Omitted: {}.".to_string(),
                kind: DslFlagKind::KeyValuePairs,
                required: false,
                value: "expression".to_string(),
                ..Default::default()
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
                name: "argument".to_string(),
                label: "Arguments".to_string(),
                field_type: NodeFieldType::KeyValuePairs,
                help: Some("What the function receives, one row per argument: a name and a literal or {{ expr }}. Empty: the function receives {}.".to_string()),
                ..Default::default()
            },
        ],
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Reuse a lookup function", r#"function.result.call --function find-user --argument "email={{ $trigger.body.email }}""#)
                .output(serde_json::json!({ "result": { "user": { "_key": "u_1", "email": "a@example.com" } } }))
                .note("`result` is whatever `jobs/find-user`'s last node produced; the rest of the payload is kept."),
        ],
        ..Default::default()
    }
}

/// What the called function receives: `--argument`, arriving final.
///
/// A map is the arguments themselves; one whole `{{ }}` arrives as the
/// object it evaluated to; a literal string that is JSON is parsed. Unset is
/// `{}` — a node reads the payload only through a flag (`node-conventions.md`
/// §3), so nothing of the caller's payload goes along by default.
fn call_arguments(argument: &serde_json::Value) -> serde_json::Value {
    match argument {
        serde_json::Value::Null => serde_json::json!({}),
        serde_json::Value::String(raw) if raw.trim().is_empty() => serde_json::json!({}),
        serde_json::Value::String(raw) => {
            serde_json::from_str(raw.trim()).unwrap_or_else(|_| argument.clone())
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
                    "FW_NODE_FUNCTION_RESULT_CALL_NO_PLATFORM",
                    "function.result.call: platform not injected into engine",
                ));
            }
        };

        let slug = match &self.config.function {
            Some(s) if !s.is_empty() => s.clone(),
            _ => {
                return Err(PipelineError::new("FW_NODE_FUNCTION_RESULT_CALL_CONFIG", "--function is required"));
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

        let call_input = call_arguments(&self.config.argument);

        match platform
            .execute_function_pipeline(&owner, &project, &slug, call_input)
            .await
        {
            Ok(result) => Ok(NodeExecutionOutput {
                output_pins: vec!["out".to_string()],
                payload: crate::pipeline::nodes::shared::util::with_answer(&input.payload, serde_json::json!({ "result": result })),
                trace: vec![format!("function.result.call: '{}' ok", slug)],
            }),
            Err(e) => Ok(NodeExecutionOutput {
                output_pins: vec!["error".to_string()],
                payload: crate::pipeline::nodes::shared::util::with_answer(
                    &input.payload,
                    serde_json::json!({ "result": { "ok": false, "error": { "code": e.code, "message": e.message } } }),
                ),
                trace: vec![format!(
                    "function.result.call: '{}' error: {} — {}",
                    slug, e.code, e.message
                )],
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::call_arguments;

    #[test]
    fn the_arguments_are_what_the_flag_says_and_nothing_else() {
        assert_eq!(call_arguments(&json!({ "email": "a@example.com", "plan": "pro" })), json!({ "email": "a@example.com", "plan": "pro" }));
        assert_eq!(call_arguments(&json!({ "user_id": 7 })), json!({ "user_id": 7 }), "a resolved {{ object }}");
        assert_eq!(call_arguments(&json!("{\"a\":1}")), json!({ "a": 1 }), "a JSON literal is parsed");
        assert_eq!(call_arguments(&json!(null)), json!({}), "unset: no arguments, not the payload");
        assert_eq!(call_arguments(&json!("  ")), json!({}));
    }
}
