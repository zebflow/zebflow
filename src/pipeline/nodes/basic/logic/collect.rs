//! `logic.collect` — explicit together-processing fan-in node.
//!
//! The engine buffers incoming payloads until every wired upstream has
//! delivered, then fires this node once with them as a list in DSL text
//! order. It answers one key, `collect: { items, count }`, on top of the
//! delivered payloads merged in that order (a key from a later one wins).

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::pipeline::nodes::shared::util::with_answer;
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const NODE_KIND: &str = "logic.collect";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";
/// The key `logic.collect` answers under.
pub const ANSWER_KEY: &str = "collect";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Collect".to_string(),
        description: "Fan-in. Waits until every node wired into it has delivered a payload, then fires once. Answers one key, \
             `collect: { items, count }` — `items` the delivered payloads in DSL text order — on top of those payloads merged \
             in the same order (a key from a later one wins). Each upstream's answer also stays at `$nodes.<id>.<key>`. \
             Graph mode only — it needs two or more incoming edges (`[b] -> [d]`, `[c] -> [d]`). It is not a join for \
             `logic.foreach` emissions — use `logic.reduce` for those."
            .to_string(),
        input_schema: serde_json::json!({ "type": "object" }),
        output_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "collect": {
                    "type": "object",
                    "properties": {
                        "items": { "type": "array" },
                        "count": { "type": "integer" }
                    }
                }
            }
        }),
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
                .output(serde_json::json!({
                    "response": { "status": 200, "body": { "b": 2 } },
                    "collect": {
                        "items": [
                            { "response": { "status": 200, "body": { "a": 1 } } },
                            { "response": { "status": 200, "body": { "b": 2 } } }
                        ],
                        "count": 2
                    }
                }))
                .note("After `[b] -> [d]` and `[c] -> [d]` with `b` declared first: `input.collect.items[0]` is b's payload; `$nodes.b.response` names it too."),
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

/// The delivered payloads merged in order (a later key wins), with
/// `collect: { items, count }` added. A payload that is not the engine's list
/// is one delivery.
pub fn answer(delivered: Value) -> Value {
    let items = match delivered {
        Value::Array(items) => items,
        one => vec![one],
    };
    let mut merged = Value::Object(serde_json::Map::new());
    for item in &items {
        merged = with_answer(&merged, item.clone());
    }
    let count = items.len();
    with_answer(&merged, json!({ ANSWER_KEY: { "items": items, "count": count } }))
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
            payload: answer(input.payload),
            trace: vec![format!("node_kind={NODE_KIND}")],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The answer is one key over the delivered payloads merged in order.
    #[test]
    fn collect_answers_items_and_count_over_the_merged_payloads() {
        let out = answer(json!([
            { "script": { "user": "u1" }, "shared": 1 },
            { "query": { "rows": [] }, "shared": 2 }
        ]));
        assert_eq!(out["collect"]["count"], 2);
        assert_eq!(out["collect"]["items"][0]["script"]["user"], "u1");
        assert_eq!(out["collect"]["items"][1]["query"]["rows"], json!([]));
        assert_eq!(out["script"]["user"], "u1");
        assert_eq!(out["shared"], 2, "a later delivery wins");
    }

    /// A single payload (one incoming edge) is one item.
    #[test]
    fn a_single_payload_is_one_item() {
        let out = answer(json!({ "a": 1 }));
        assert_eq!(out["collect"]["count"], 1);
        assert_eq!(out["a"], 1);
    }
}
