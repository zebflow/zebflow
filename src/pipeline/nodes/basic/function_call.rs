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
//! | `input_path` | string | no | JSON Pointer into payload to use as function input |
//!
//! # DSL
//! ```text
//! | function.call --function my-fn --input-path /body
//! ```
//!
//! # Input/output
//! - **Input:** any payload (or sub-section via `input_path`)
//! - **Output `out`:** the function pipeline's last node output value
//! - **Output `error`:** `{ "error": "..." }` on failure

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::pipeline::model::{DslFlag, DslFlagKind, NodeFieldDataSource, NodeFieldDef, NodeFieldType};
use crate::platform::services::PlatformService;

pub const NODE_KIND: &str = "n.function.call";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// Slug of the function pipeline to call (matches `PipelineMeta.name`).
    pub function: Option<String>,
    /// JSON Pointer into the flowing payload to use as function input.
    /// Empty string = use entire payload.
    #[serde(default)]
    pub input_path: String,
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
        title: "Call Function".to_string(),
        description: "Calls a function pipeline by slug and injects its output into the flow. \
            Routes to 'out' on success, 'error' on failure or when the function is not found."
            .to_string(),
        input_pins: vec!["in".to_string()],
        output_pins: vec!["out".to_string(), "error".to_string()],
        config_schema: serde_json::json!({
            "type": "object",
            "required": ["function"],
            "properties": {
                "function": {
                    "type": "string",
                    "description": "Slug of the function pipeline to call."
                },
                "input_path": {
                    "type": "string",
                    "description": "JSON Pointer into the flowing payload to use as function input (e.g. /body). Leave empty to pass the full payload."
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
                flag: "--input-path".to_string(),
                config_key: "input_path".to_string(),
                description: "JSON Pointer into the flowing payload to extract as function input."
                    .to_string(),
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
                name: "input_path".to_string(),
                label: "Input Path".to_string(),
                field_type: NodeFieldType::Text,
                placeholder: Some("/user (leave empty for full payload)".to_string()),
                help: Some(
                    "JSON Pointer into the flowing payload to extract as function input."
                        .to_string(),
                ),
                ..Default::default()
            },
        ],
        ..Default::default()
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
                ))
            }
        };

        let slug = match &self.config.function {
            Some(s) if !s.is_empty() => s.clone(),
            _ => {
                return Ok(NodeExecutionOutput {
                    output_pins: vec!["error".to_string()],
                    payload: serde_json::json!({"error": "no function slug configured"}),
                    trace: vec!["function.call: no slug configured".to_string()],
                })
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

        // Extract sub-input from payload using input_path JSON Pointer.
        let call_input = if self.config.input_path.is_empty() {
            input.payload.clone()
        } else {
            input
                .payload
                .pointer(&self.config.input_path)
                .cloned()
                .unwrap_or_else(|| input.payload.clone())
        };

        match platform
            .execute_function_pipeline(&owner, &project, &slug, call_input)
            .await
        {
            Ok(result) => Ok(NodeExecutionOutput {
                output_pins: vec!["out".to_string()],
                payload: result,
                trace: vec![format!("function.call: '{}' ok", slug)],
            }),
            Err(e) => Ok(NodeExecutionOutput {
                output_pins: vec!["error".to_string()],
                payload: serde_json::json!({"error": format!("{}: {}", e.code, e.message)}),
                trace: vec![format!("function.call: '{}' error: {} — {}", slug, e.code, e.message)],
            }),
        }
    }
}
