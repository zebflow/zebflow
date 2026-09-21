//! Manual trigger node.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::pipeline::model::{LayoutItem, NodeFieldDef, NodeFieldType};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "n.trigger.manual";
pub const OUTPUT_PIN_OUT: &str = "out";

/// Unified node-definition metadata for `n.trigger.manual`.
pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Manual Trigger".to_string(),
        description: "Starts the pipeline when someone runs it by hand — the Studio's Run button, `POST /pipelines/execute` with \
            `trigger: \"manual\"`, or the console's `execute pipeline`. The payload is whatever `input` the caller sent, unchanged \
            (`{}` when nothing was sent). Use it for one-off jobs and admin actions; a pipeline another pipeline should call is \
            `trigger.function`, and one that runs on its own is `trigger.schedule`."
            .to_string(),
        input_schema: serde_json::json!({
            "type":"object",
            "description":"Manual execution payload."
        }),
        output_schema: serde_json::json!({
            "type":"object",
            "description":"Unmodified manual payload for downstream nodes."
        }),
        input_pins: vec![],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: Default::default(),
        fields: vec![NodeFieldDef {
            name: "__manual_note".to_string(),
            label: "Manual Trigger".to_string(),
            field_type: NodeFieldType::Text,
            readonly: true,
            default_value: Some(serde_json::json!(
                "Runs only when pipeline execute trigger=manual."
            )),
            help: Some(
                "Read-only note: this trigger fires only from explicit manual pipeline execution."
                    .to_string(),
            ),
            ..Default::default()
        }],
        layout: vec![LayoutItem::Field("__manual_note".to_string())],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("A job run from the Studio", "trigger.manual")
                .input(serde_json::json!({ "dry_run": true }))
                .output(serde_json::json!({ "dry_run": true }))
                .note("The next node reads `input.dry_run`; with no input the payload is `{}`."),
        ],
        ..Default::default()
    }
}

/// The `$trigger` snapshot of a manual run.
///
/// `kinds/node-io`: the trigger's context is the initial payload and "the
/// originals are reachable forever via `$trigger`". A manual run has no
/// route, so `params`, `query` and `auth` are empty — but its envelope,
/// `body` and `files`, is what an `input.*` node declared and what a script
/// reads back after a node has replaced the payload.
pub fn trigger_snapshot(input: &serde_json::Value) -> serde_json::Value {
    serde_json::json!({
        "body": input.get("body").cloned().unwrap_or(serde_json::Value::Null),
        "files": input.get("files").cloned().unwrap_or_else(|| serde_json::json!({})),
        "params": {},
        "query": {},
        "auth": serde_json::Value::Null,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {}

pub struct Node;

impl Node {
    pub fn new(_config: Config) -> Self {
        Self
    }
}

#[async_trait]
impl NodeHandler for Node {
    fn kind(&self) -> &'static str {
        NODE_KIND
    }
    fn input_pins(&self) -> &'static [&'static str] {
        &[]
    }
    fn output_pins(&self) -> &'static [&'static str] {
        &[OUTPUT_PIN_OUT]
    }

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: input.payload,
            trace: vec![format!("node_kind={NODE_KIND}")],
        })
    }
}
