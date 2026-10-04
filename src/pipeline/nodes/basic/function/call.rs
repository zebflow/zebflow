//! `function.result.call` — invokes a function pipeline by slug.
//!
//! # Pipeline position
//!
//! Middleware node. Calls another active pipeline that starts with `trigger.function`.
//! On success routes through `out`. A failed call (the function missing, inactive
//! or failed) is this node failing, like any node (`node-conventions.md` §4): the
//! engine delivers it to a wired `:error`, otherwise the run fails here.
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
//! - **Output `out`:** the payload plus `result`, the function's result: its last node's
//!   answer (`PipelineOutput::function_result` — the rule composite nodes share)
//! - **Failure:** the function's own error (`FW_FUNCTION_NOT_FOUND`, or the code
//!   its failing node raised), through the engine's `:error` path

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
            answers under its trigger's key, `function`. `out` adds `result` — the function's result: what its last node answered (a \
            `javascript.script.run` ending the function answers `script`, so `result` is that value; a last `logic.*` node leaves the \
            function's final payload without `function`) — and keeps the caller's payload. A call to a function that is missing, inactive or failed \
            fails this node like any other: a wired `:error` receives the failure, otherwise the run fails here."
            .to_string(),
        input_pins: vec!["in".to_string()],
        output_pins: vec!["out".to_string()],
        output_schema: serde_json::json!({
            "description": "The payload plus `result`, what the called function's last node answered. A failed call fails the node.",
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
                .note("`result` is what `jobs/find-user`'s last node answered — here a `javascript.script.run` returning `{ user }`; the rest of the payload is kept."),
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
        &["out"]
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

        // A failed call is this node's failure, never a delivery of its own:
        // the engine routes it to a wired `:error` or fails the run (§4).
        // The failure is this node's: the function's own code and message,
        // attributed by the engine to this node rather than to a node of the
        // function's graph.
        let result = platform
            .execute_function_pipeline(&owner, &project, &slug, call_input)
            .await
            .map_err(|e| PipelineError::new(e.code, e.message))?;
        Ok(NodeExecutionOutput {
            output_pins: vec!["out".to_string()],
            payload: crate::pipeline::nodes::shared::util::with_answer(&input.payload, serde_json::json!({ "result": result })),
            trace: vec![format!("function.result.call: '{}' ok", slug)],
        })
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

    /// `result` is what the function's last node answered — the rule a
    /// composite node's answer shares — and the caller's payload is kept; a
    /// missing function is the node failing, never a delivery on a pin.
    #[tokio::test(flavor = "multi_thread")]
    async fn the_result_is_the_functions_last_answer_under_result() {
        use crate::pipeline::nodes::{NodeExecutionInput, NodeHandler};

        let platform = crate::pipeline::nodes::shared::test_platform::test_platform();
        let function = json!({
            "apiVersion": "zebflow.com/v1",
            "kind": "Pipeline",
            "metadata": { "name": "find-user" },
            "spec": {
                "id": "find-user",
                "nodes": [
                    { "id": "trigger", "kind": "trigger.function", "config": { "description": "Find one user by email." }, "input_pins": [], "output_pins": ["out"] },
                    { "id": "shape", "kind": "javascript.script.run",
                      "config": { "source": "return { user: { email: input.function.email } };" },
                      "input_pins": ["in"], "output_pins": ["out"] }
                ],
                "edges": [{ "from_node": "trigger", "from_pin": "out", "to_node": "shape", "to_pin": "in" }]
            }
        });
        let meta = platform
            .projects
            .upsert_pipeline_definition("superadmin", "default", "jobs/find-user.zf.json", "Find user", "", "trigger.function", &function.to_string())
            .expect("function saved");
        platform
            .projects
            .activate_pipeline_definition("superadmin", "default", &meta.file_rel_path)
            .expect("function activated");
        platform.pipeline_runtime.refresh_project("superadmin", "default").expect("runtime");

        let call = |function: &str| {
            let node = super::Node::new(
                serde_json::from_value(json!({ "function": function, "argument": { "email": "a@example.com" } })).unwrap(),
                Some((*platform).clone()),
            );
            async move {
                node.execute_async(NodeExecutionInput {
                    node_id: "n1".to_string(),
                    input_pin: "in".to_string(),
                    payload: json!({ "kept": 1 }),
                    metadata: json!({ "owner": "superadmin", "project": "default" }),
                    bus: None,
                })
                .await
            }
        };

        let out = call("find-user").await.expect("delivers");
        assert_eq!(out.output_pins, ["out"]);
        assert_eq!(
            out.payload,
            json!({ "kept": 1, "result": { "user": { "email": "a@example.com" } } }),
            "the last node's answer, not the function's whole payload"
        );

        let failed = call("no-such-function").await.err().expect("a missing function fails the node");
        assert_eq!(failed.code, "FW_FUNCTION_NOT_FOUND");
    }

    /// A failed call goes the way every node's failure goes (§4): a wired
    /// `:error` receives it from the engine, and with nothing wired the run
    /// fails at the call instead of carrying on.
    #[tokio::test(flavor = "multi_thread")]
    async fn a_failed_call_is_routed_by_the_engine_or_fails_the_run() {
        use std::sync::Arc;

        use crate::pipeline::engines::basic::BasicPipelineEngine;
        use crate::pipeline::{PipelineContext, PipelineEngine};

        let platform = crate::pipeline::nodes::shared::test_platform::test_platform();
        let run = |dsl: &'static str| {
            let platform: Arc<_> = (*platform).clone();
            async move {
                let graph = crate::platform::shell::parser::build_pipeline_graph("caller", dsl).expect("graph");
                let ctx = PipelineContext {
                    owner: "superadmin".into(),
                    project: "default".into(),
                    pipeline: "caller".into(),
                    request_id: "caller-run".into(),
                    route: String::new(),
                    input: json!({}),
                    trigger: None,
                    placeholder: None,
                };
                BasicPipelineEngine::default().with_platform(platform).execute_async(&graph, &ctx).await
            }
        };

        let routed = run("[a] trigger.manual\n\
                          [c] function.result.call --function no-such-function\n\
                          [h] javascript.script.run -- \"return { code: input.result.error.code, message: input.result.error.message, kept: input.manual };\"\n\
                          [after] javascript.script.run -- \"return 'carried on';\"\n\
                          [a] -> [c]\n[c] -> [after]\n[c]:error -> [h]\n")
        .await
        .expect("a wired :error handles it");
        assert_eq!(routed.value["script"]["code"], "FW_FUNCTION_NOT_FOUND", "{}", routed.value);
        assert!(routed.value["script"]["kept"].is_object(), "the caller's payload is kept beside `result`: {}", routed.value);
        assert!(routed.value["script"]["message"].as_str().unwrap_or_default().contains("no-such-function"), "{}", routed.value);
        let c = routed.node_trace.iter().find(|t| t.node_id == "c").expect("traced");
        assert_eq!(c.status, "error_routed");
        assert_eq!(routed.node_trace.iter().find(|t| t.node_id == "after").map(|t| t.status.as_str()), Some("skipped"));

        let failed = run("[a] trigger.manual\n\
                          [c] function.result.call --function no-such-function\n\
                          [after] javascript.script.run -- \"return 'carried on';\"\n\
                          [a] -> [c]\n[c] -> [after]\n")
        .await
        .expect_err("nothing wired: the run fails at the call");
        assert_eq!(failed.code, "FW_FUNCTION_NOT_FOUND");
        assert_eq!(failed.node_id.as_deref(), Some("c"), "attributed to the call, not to a node of the function");
    }
}
