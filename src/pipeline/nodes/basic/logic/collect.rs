//! `n.logic.collect` — explicit together-processing fan-in node.
//!
//! The node itself is a passthrough. The engine buffers incoming payloads by input pin until all
//! declared inputs have arrived, then fires this node once with a grouped payload object.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "n.logic.collect";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Collect".to_string(),
        description: "Fan-in. Waits until every node wired into it has delivered a payload, then fires once with all of them keyed by \
             the upstream node id: `{ b: <b's payload>, c: <c's payload> }`. Graph mode only — it needs two or more incoming edges \
             (`[b] -> [d]`, `[c] -> [d]`). It does not merge the payloads; a `script` after it composes what the next node needs. \
             It is not a join for `logic.foreach` emissions — use `logic.reduce` for those."
            .to_string(),
        input_schema: serde_json::json!({ "type": "object" }),
        output_schema: serde_json::json!({ "type": "object" }),
        // Declared because edges arrive here: this node exists to gather
        // several upstreams before continuing. The definition said `[]` while
        // the DSL declared `in` on every instance, and the engine validated
        // edges against the DSL's copy — so the disagreement stayed invisible
        // until the parser started reading definitions.
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![],
        fields: vec![],
        layout: vec![],
        ai_tool: Default::default(),
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Wait for two fetches", "logic.collect")
                .output(serde_json::json!({ "b": { "response": { "status": 200, "body": { "a": 1 } } }, "c": { "response": { "status": 200, "body": { "b": 2 } } } }))
                .note("After `[b] -> [d]` and `[c] -> [d]`, node `d` sees both under the ids `b` and `c`."),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {}

pub struct Node;

impl Node {
    pub fn new(_: Config) -> Self {
        Self
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
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: input.payload,
            trace: vec![format!("node_kind={NODE_KIND}")],
        })
    }
}
