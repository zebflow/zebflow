//! Script execution node backed by the language engine.

use serde::{Deserialize, Serialize};
use crate::framework::{FrameworkError, nodes::{FrameworkNode, NodeExecutionInput, NodeExecutionOutput}};
use crate::language::{
    COMPILE_TARGET_BACKEND, CompileOptions, CompiledProgram, ExecutionContext, LanguageEngine,
    ModuleSource, SourceKind,
};

pub const NODE_KIND: &str = "x.n.script";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub language: String,
    #[serde(default)]
    pub source: String,
}

pub struct Node {
    node_id: String,
    compiled: CompiledProgram,
    language: std::sync::Arc<dyn LanguageEngine>,
}

impl Node {
    pub fn new(
        node_id: &str,
        config: Config,
        language: std::sync::Arc<dyn LanguageEngine>,
    ) -> Result<Self, FrameworkError> {
        if config.source.trim().is_empty() {
            return Err(FrameworkError::new(
                "FW_NODE_SCRIPT_CONFIG",
                format!("node '{}' requires config.source", node_id),
            ));
        }
        let module = ModuleSource {
            id: format!("framework:{node_id}"),
            source_path: None,
            kind: SourceKind::Tsx,
            code: config.source,
        };
        let ir = language.parse(&module).map_err(|err| {
            FrameworkError::new("FW_NODE_SCRIPT_PARSE", format!("node '{}': {}", node_id, err))
        })?;
        let compiled = language
            .compile(
                &ir,
                &CompileOptions {
                    target: COMPILE_TARGET_BACKEND.to_string(),
                    optimize_level: 1,
                    emit_trace_hints: true,
                },
            )
            .map_err(|err| {
                FrameworkError::new("FW_NODE_SCRIPT_COMPILE", format!("node '{}': {}", node_id, err))
            })?;
        Ok(Self {
            node_id: node_id.to_string(),
            compiled,
            language,
        })
    }
}

impl FrameworkNode for Node {
    fn kind(&self) -> &'static str { NODE_KIND }
    fn input_pins(&self) -> &'static [&'static str] { &[INPUT_PIN_IN] }
    fn output_pins(&self) -> &'static [&'static str] { &[OUTPUT_PIN_OUT] }

    fn execute(&self, input: NodeExecutionInput) -> Result<NodeExecutionOutput, FrameworkError> {
        if input.input_pin != INPUT_PIN_IN {
            return Err(FrameworkError::new(
                "FW_NODE_SCRIPT_INPUT_PIN",
                format!("unsupported input pin '{}'", input.input_pin),
            ));
        }

        let ctx = ExecutionContext {
            project: input
                .metadata
                .get("project")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
            pipeline: input
                .metadata
                .get("pipeline")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
            request_id: input
                .metadata
                .get("request_id")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default()
                .to_string(),
            metadata: input.metadata.clone(),
        };

        let out = self
            .language
            .run(&self.compiled, input.payload, &ctx)
            .map_err(|err| {
                FrameworkError::new("FW_NODE_SCRIPT_RUN", format!("node '{}': {}", self.node_id, err))
            })?;

        let mut trace = vec![format!("node_kind={NODE_KIND}")];
        trace.extend(out.trace);

        Ok(NodeExecutionOutput {
            output_pin: OUTPUT_PIN_OUT.to_string(),
            payload: out.value,
            trace,
        })
    }
}
