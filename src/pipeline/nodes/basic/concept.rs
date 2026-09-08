//! `n.concept` — a step you have described but not built yet.
//!
//! It passes its input through untouched. That is the whole behaviour, and it
//! is the point: a pipeline can be wired end to end while some of its steps are
//! still prose. Run it and the payload reaches the far side unchanged, so the
//! shape of the flow can be tested before any of the hard parts exist.
//!
//! ```text
//! trigger.webhook | n.concept --text "look the researcher up in ORCID" | web.response
//! ```
//!
//! The editor renders `text` as markdown in the node's body instead of the
//! usual config summary, so the drawing reads as the description.
//!
//! **Why a node and not a note.** A note is drawn beside the graph and never
//! runs. This is inside the graph: it occupies a position in the flow, holds a
//! place between two real steps, and disappears from the drawing the day it is
//! replaced by the node that does the work. One is commentary; this is a
//! placeholder with pins.
//!
//! It declares no capabilities because it does nothing — which also makes it
//! safe to leave in a shipped pipeline: an unimplemented step cannot reach the
//! network, the filesystem, or a credential.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::pipeline::model::{DslFlag, DslFlagKind, LayoutItem, NodeFieldDef, NodeFieldType};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "n.concept";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Concept".to_string(),
        description: "A described but unbuilt step. Passes its input through unchanged so a \
                      pipeline can be wired and run before every step exists. The editor shows \
                      its text as markdown in the node body."
            .to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "description": "Whatever arrives — it leaves unchanged."
        }),
        output_schema: serde_json::json!({
            "type": "object",
            "description": "Exactly the input payload."
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "text": {
                    "type": "string",
                    "description": "What this step will do, in markdown. Rendered in the node body."
                }
            }
        }),
        dsl_flags: vec![DslFlag {
            flag: "--text".to_string(),
            config_key: "text".to_string(),
            description: "What this step will do, in markdown.".to_string(),
            kind: DslFlagKind::Scalar,
            required: false,
        }],
        fields: vec![NodeFieldDef {
            name: "text".to_string(),
            label: "Description".to_string(),
            field_type: NodeFieldType::Textarea,
            rows: Some(6),
            help: Some(
                "What this step will do once it is built. Markdown, shown in the node body."
                    .to_string(),
            ),
            ..Default::default()
        }],
        layout: vec![LayoutItem::Field("text".to_string())],
        ai_tool: Default::default(),
        ..Default::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// What this step will do, once someone builds it.
    #[serde(default)]
    pub text: String,
}

pub struct Node {
    config: Config,
}

impl Node {
    pub fn new(config: Config) -> Self {
        Self { config }
    }
}

#[async_trait]
impl NodeHandler for Node {
    fn kind(&self) -> &'static str {
        NODE_KIND
    }
    fn input_pins(&self) -> &'static [&'static str] {
        &[INPUT_PIN_IN]
    }
    fn output_pins(&self) -> &'static [&'static str] {
        &[OUTPUT_PIN_OUT]
    }

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        // The trace carries the description, so a run of a half-built pipeline
        // reads as a list of what is still prose rather than a silent gap.
        let note = self.config.text.trim();
        let trace = if note.is_empty() {
            format!("{NODE_KIND}: not built yet")
        } else {
            format!("{NODE_KIND}: not built yet — {note}")
        };
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: input.payload,
            trace: vec![trace],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn run(config: Config, payload: serde_json::Value) -> NodeExecutionOutput {
        let node = Node::new(config);
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
            .block_on(node.execute_async(NodeExecutionInput {
                node_id: "c".to_string(),
                input_pin: INPUT_PIN_IN.to_string(),
                payload,
                metadata: json!({}),
                bus: None,
            }))
            .expect("concept never fails")
    }

    /// The payload crosses untouched — that is what lets a drafted pipeline run
    /// end to end with holes in it.
    #[test]
    fn whatever_arrives_leaves_unchanged() {
        let payload = json!({ "researcher": { "name": "Sari" }, "n": 3 });
        let out = run(
            Config {
                text: "look them up in ORCID".to_string(),
            },
            payload.clone(),
        );
        assert_eq!(out.payload, payload);
        assert_eq!(out.output_pins, vec![OUTPUT_PIN_OUT.to_string()]);
    }

    /// A run says which steps are still prose, rather than passing in silence.
    #[test]
    fn the_trace_names_the_step_as_unbuilt() {
        let out = run(
            Config {
                text: "look them up in ORCID".to_string(),
            },
            json!({}),
        );
        assert!(out.trace[0].contains("not built yet"), "{:?}", out.trace);
        assert!(out.trace[0].contains("ORCID"), "{:?}", out.trace);

        let bare = run(Config::default(), json!({}));
        assert!(bare.trace[0].contains("not built yet"), "{:?}", bare.trace);
    }

    /// It touches nothing, so it is safe to leave in a pipeline that ships.
    #[test]
    fn a_concept_declares_no_capabilities() {
        assert!(definition().capabilities.is_empty());
    }
}
