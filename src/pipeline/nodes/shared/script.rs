//! The script nodes' engine, shared by the families whose language runs in
//! the Deno sandbox: `js.script.run` (JavaScript) and `ts.script.run`
//! (TypeScript). Each family folder holds only its [`Language`]; the
//! definition, the build and the run are here, once.
//!
//! The source is the body after `--`; it compiles when the node is built. The
//! answer is one key, `script: <what the code returned>`, the rest of the
//! payload kept. A `__signal` or `__zf_files` key in a returned object is
//! lifted out to the top of the payload, where the engine reads and strips it.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::language::{
    COMPILE_TARGET_BACKEND, CompileOptions, CompiledProgram, ExecutionContext, LanguageEngine, ModuleSource, SourceKind,
};
use crate::pipeline::model::{LayoutItem, NodeCapability, NodeExample, NodeFieldDef, NodeFieldType, SidebarItem, SidebarSection};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";
/// The key every script kind answers under.
pub const ANSWER_KEY: &str = "script";

/// One language the sandbox runs: its kind, the words its definition uses,
/// and its family's error codes.
#[derive(Debug)]
pub struct Language {
    /// `js.script.run`, `ts.script.run`.
    pub kind: &'static str,
    /// The canvas title.
    pub title: &'static str,
    /// The language's name in prose: JavaScript, TypeScript.
    pub name: &'static str,
    /// The editor's syntax mode.
    pub editor: &'static str,
    /// The code ran and failed: `FW_NODE_<FAMILY>_SCRIPT_RUN`.
    pub run_code: &'static str,
    /// No source, or a config that does not parse.
    pub config_code: &'static str,
    /// The source does not parse.
    pub parse_code: &'static str,
    /// The sandbox refuses the source (a policy violation).
    pub rejected_code: &'static str,
    /// The compiler itself failed.
    pub compile_code: &'static str,
    /// A pin other than `in`.
    pub input_pin_code: &'static str,
}

/// The definition of `language`'s script kind.
pub fn definition(language: &Language) -> NodeDefinition {
    let kind = language.kind;
    NodeDefinition {
        kind: kind.to_string(),
        capabilities: vec![NodeCapability::Filesystem, NodeCapability::Process],
        title: language.title.to_string(),
        description: format!(
            "Runs {name} in a sandbox and adds what it returns as `script`. The body after `--` is the function body of `async function(input, n, ctx)`, and whatever it \
             `return`s is added to the payload as `script` — the rest is kept, so the next node reads `input.script.x`. `input` is the \
             current payload (right after a webhook, `input.webhook.body.x`; the request anywhere, `$trigger.body.x`); \
             `ctx.trigger.params/query/auth` is the request and `ctx.nodes.<id>` an earlier node's output. It cannot set a status or header \
             (`web.response.send` does), `return null` does not stop the run (`logic.if` does), and `fetch`, `setTimeout`, `require` and \
             `import` are blocked (`http.response.fetch` calls out). Keep it to shaping data: compose, rename, compute — a run has 1 s and \
             its return 256 KB (`DenoSandboxError: timeout exceeded` means do less here or split the work). A `__signal` key in \
             the returned object is stripped and shown as live progress in the Studio; `__zf_files` stores files for the run.",
            name = language.name
        ),
        input_schema: json!({ "type": "object", "description": "Upstream payload available as `input`; it is kept and `script` is added." }),
        output_schema: json!({ "type": "object", "properties": { "script": { "description": "What the code returned." } } }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![],
        fields: vec![NodeFieldDef {
            name: "source".to_string(),
            label: "Source".to_string(),
            field_type: NodeFieldType::CodeEditor,
            language: Some(language.editor.to_string()),
            span: Some("full".to_string()),
            help: Some("The function body. What it returns is added as `script`.".to_string()),
            default_value: Some(json!("return input;")),
            sidebar: sidebar(),
            ..Default::default()
        }],
        layout: vec![LayoutItem::Field("source".to_string())],
        ai_tool: crate::pipeline::model::NodeAiToolDefinition {
            registered: true,
            tool_name: format!("run_{}", kind.split('.').next().unwrap_or("javascript")),
            tool_description: format!("Execute a {} snippet in the Deno sandbox. Args: code (required).", language.name),
            tool_input_schema: json!({
                "type": "object",
                "properties": { "code": { "type": "string", "description": format!("{} code to execute", language.name) } },
                "required": ["code"]
            }),
        },
        examples: vec![
            NodeExample::dsl("Shape rows for a page", &format!(r#"{kind} -- "return {{ posts: input.query.rows.map(r => ({{ ...r, when: r.created_at.slice(0, 10) }})), total: input.query.row_count }}""#))
                .input(json!({ "query": { "rows": [{ "title": "Hi", "created_at": "2026-09-13T04:00:00Z" }], "row_count": 1 } }))
                .output(json!({ "query": { "rows": [{ "title": "Hi", "created_at": "2026-09-13T04:00:00Z" }], "row_count": 1 }, "script": { "posts": [{ "title": "Hi", "created_at": "2026-09-13T04:00:00Z", "when": "2026-09-13" }], "total": 1 } }))
                .note("The next node reads `input.script.posts`."),
            NodeExample::dsl("Combine two earlier nodes", &format!(r#"{kind} -- "return {{ user: $nodes.b.query.rows[0], orders: $nodes.c.query.rows }}""#))
                .note("Anywhere in graph mode downstream of `b` and `c`: wired from both, it runs once, when both have answered (a `$nodes` reference must point upstream)."),
        ],
        ..Default::default()
    }
}

/// The editor's sidebar: what a body can reach.
fn sidebar() -> Vec<SidebarSection> {
    vec![
        SidebarSection {
            title: "Input".to_string(),
            items: vec![
                SidebarItem { label: "input".to_string(), type_hint: Some("any".to_string()), description: Some("Upstream payload from the previous node.".to_string()) },
            ],
        },
        SidebarSection {
            title: "Context".to_string(),
            items: vec![
                SidebarItem { label: "ctx.pipeline".to_string(), type_hint: Some("string".to_string()), description: Some("Current pipeline id.".to_string()) },
                SidebarItem { label: "ctx.request_id".to_string(), type_hint: Some("string".to_string()), description: Some("Unique execution request id.".to_string()) },
                SidebarItem { label: "ctx.nodes".to_string(), type_hint: Some("object".to_string()), description: Some("Map of all previous node outputs. Access by node id: ctx.nodes['a'].".to_string()) },
                SidebarItem { label: "ctx.placeholder".to_string(), type_hint: Some("object".to_string()), description: Some("Resolved placeholder values (credentials, config).".to_string()) },
            ],
        },
        SidebarSection {
            title: "Trigger".to_string(),
            items: vec![
                SidebarItem { label: "ctx.trigger".to_string(), type_hint: Some("object|null".to_string()), description: Some("Full trigger event snapshot from the entry node.".to_string()) },
                SidebarItem { label: "ctx.trigger.auth".to_string(), type_hint: Some("object|null".to_string()), description: Some("Verified JWT claims — immutable across the pipeline.".to_string()) },
                SidebarItem { label: "ctx.trigger.params".to_string(), type_hint: Some("object".to_string()), description: Some("URL path params (:id etc) from the request.".to_string()) },
                SidebarItem { label: "ctx.trigger.query".to_string(), type_hint: Some("object".to_string()), description: Some("Query string params from the request.".to_string()) },
                SidebarItem { label: "ctx.trigger.headers".to_string(), type_hint: Some("object".to_string()), description: Some("Safe subset of request headers.".to_string()) },
            ],
        },
        SidebarSection {
            title: "Return".to_string(),
            items: vec![
                SidebarItem { label: "return value".to_string(), type_hint: Some("any".to_string()), description: Some("Added to the payload as `script`; downstream reads input.script.".to_string()) },
                SidebarItem { label: "__signal".to_string(), type_hint: Some("string|object|array".to_string()), description: Some("Optional key in return value for real-time signals. Stripped before downstream.".to_string()) },
            ],
        },
        SidebarSection {
            title: "Built-ins".to_string(),
            items: vec![
                SidebarItem { label: "console.log(...)".to_string(), type_hint: Some("void".to_string()), description: Some("Log to pipeline trace output.".to_string()) },
                SidebarItem { label: "n.time.now()".to_string(), type_hint: Some("number".to_string()), description: Some("Current Unix timestamp in milliseconds.".to_string()) },
                SidebarItem { label: "n.math.imul(a, b)".to_string(), type_hint: Some("number".to_string()), description: Some("32-bit integer multiply.".to_string()) },
                SidebarItem { label: "n.math.u32(v)".to_string(), type_hint: Some("number".to_string()), description: Some("Coerce to an unsigned 32-bit integer.".to_string()) },
            ],
        },
    ]
}

/// The script kinds' config: the body.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// The function body.
    #[serde(default)]
    pub source: String,
}

pub struct Node {
    node_id: String,
    compiled: CompiledProgram,
    language: &'static Language,
    engine: std::sync::Arc<dyn LanguageEngine>,
}

impl Node {
    /// Parses `config` and compiles its source, refusing an empty or invalid
    /// one under `language`'s codes.
    pub fn build(
        node_id: &str,
        config: &Value,
        language: &'static Language,
        engine: std::sync::Arc<dyn LanguageEngine>,
    ) -> Result<Self, PipelineError> {
        let config: Config =
            serde_json::from_value(config.clone()).map_err(|err| PipelineError::new(language.config_code, err.to_string()))?;
        if config.source.trim().is_empty() {
            return Err(PipelineError::new(language.config_code, format!("node '{node_id}' has no source; write it after --")));
        }
        let module = ModuleSource {
            id: format!("pipeline:{node_id}"),
            source_path: None,
            kind: SourceKind::Tsx,
            code: config.source,
        };
        let ir = engine
            .parse(&module)
            .map_err(|err| PipelineError::new(language.parse_code, format!("node '{node_id}': {err}")))?;
        let compiled = engine
            .compile(
                &ir,
                &CompileOptions { target: COMPILE_TARGET_BACKEND.to_string(), optimize_level: 1, emit_trace_hints: true },
            )
            .map_err(|err| {
                // A script the author wrote wrongly stays refused; only a real
                // compiler fault is a failure worth retrying.
                PipelineError::new(
                    crate::pipeline::error_class::wrapper_for(&err.code, language.rejected_code, language.compile_code),
                    format!("node '{node_id}': {err}"),
                )
            })?;
        Ok(Self { node_id: node_id.to_string(), compiled, language, engine })
    }
}

/// The keys a returned object hands to the engine rather than to the next
/// node: `__signal` (live progress) and `__zf_files` (files to store, whose
/// `{{file:name}}` placeholders become FileRefs).
const ENGINE_KEYS: &[&str] = &["__signal", "__zf_files"];

/// What the code returned, as the answer: `script` added to the payload the
/// node was given. The engine's own keys are lifted out of the answer to the
/// top of the payload, where the engine reads and strips them.
pub fn answer(payload: &Value, mut returned: Value) -> Value {
    let lifted: Vec<(&str, Value)> = ENGINE_KEYS
        .iter()
        .filter_map(|key| returned.as_object_mut().and_then(|map| map.remove(*key)).map(|value| (*key, value)))
        .collect();
    let mut answer = json!({ ANSWER_KEY: returned });
    for (key, value) in lifted {
        answer[key] = value;
    }
    crate::pipeline::nodes::shared::util::with_answer(payload, answer)
}

#[async_trait]
impl NodeHandler for Node {
    fn kind(&self) -> &'static str {
        self.language.kind
    }
    fn input_pins(&self) -> &'static [&'static str] {
        &[INPUT_PIN_IN]
    }
    fn output_pins(&self) -> &'static [&'static str] {
        &[OUTPUT_PIN_OUT]
    }

    async fn execute_async(&self, input: NodeExecutionInput) -> Result<NodeExecutionOutput, PipelineError> {
        if input.input_pin != INPUT_PIN_IN {
            return Err(PipelineError::new(
                self.language.input_pin_code,
                format!("unsupported input pin '{}'", input.input_pin),
            ));
        }
        let read = |key: &str| input.metadata.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
        let ctx = ExecutionContext {
            project: read("project"),
            pipeline: read("pipeline"),
            request_id: read("request_id"),
            trigger: input.metadata.get("trigger").cloned().unwrap_or(Value::Null),
            metadata: input.metadata.clone(),
        };
        let out = self
            .engine
            .run(&self.compiled, input.payload.clone(), &ctx)
            .map_err(|err| PipelineError::new(self.language.run_code, format!("node '{}': {}", self.node_id, err)))?;
        let mut trace = vec![format!("node_kind={}", self.language.kind)];
        trace.extend(out.trace);
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: answer(&input.payload, out.value),
            trace,
        })
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    /// The return is `script`; the payload is kept; the engine's keys rise
    /// to where the engine reads them.
    #[test]
    fn the_return_is_script_and_the_payload_is_kept() {
        assert_eq!(super::answer(&json!({ "kept": 1 }), json!({ "a": 2 })), json!({ "kept": 1, "script": { "a": 2 } }));
        assert_eq!(super::answer(&json!({ "kept": 1 }), json!([1, 2])), json!({ "kept": 1, "script": [1, 2] }));
        assert_eq!(super::answer(&json!("bare"), json!(3)), json!({ "script": 3 }));
        assert_eq!(
            super::answer(&json!({}), json!({ "a": 1, "__signal": "half", "__zf_files": [] })),
            json!({ "script": { "a": 1 }, "__signal": "half", "__zf_files": [] })
        );
    }
}
