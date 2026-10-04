//! Built-in trigger nodes.
//!
//! Triggers are entry nodes: they have no input pins and they create the first
//! payload for a pipeline run. For the general node authoring contract, read
//! `src/pipeline/nodes/mod.rs`; for registration, read
//! `src/pipeline/nodes/basic/mod.rs`.
//!
//! Every trigger answers **one key, its source** (`node-conventions.md` §1,
//! §6), holding the envelope the ingress delivered — nothing else lands at the
//! payload root:
//!
//! | Kind | Answer |
//! |---|---|
//! | `trigger.webhook` | `webhook: { body, query, params, headers, files, method, path, auth }` |
//! | `trigger.manual` | `manual: <the caller's input>` (the Run form sends `{ body, files }`) |
//! | `trigger.schedule` | `schedule: { trigger, fired_at, node_id }` |
//! | `trigger.function` | `function: <the caller's arguments>` |
//! | `trigger.mcp` | `mcp: { route, tool_name, arguments }` |
//! | `trigger.error` | `error: { error_code, error_message, original_path, path, method, request_id, … }` |
//! | `trigger.room` | `room: { room_id, session_id, event, payload, auth? }` |
//! | `trigger.socket` | `socket: { trigger, url, node_id, message }` |
//! | `trigger.topic` | `topic: { topic, message, node_id }` |
//!
//! The same envelope is `$trigger` in every expression, for the whole run
//! (`metadata.trigger`; the engine falls back to the run's input when an
//! ingress sets no snapshot of its own).
//!
//! A webhook's uploads are FileRef metadata under `webhook.files.<field>`;
//! repeated field names, `field[]` and `field[0]` become arrays. Shared file
//! byte rules live in `src/pipeline/nodes/shared/file_ref.rs`.
//!
//! `trigger.room` lives with the WebSocket family (`basic/ws/trigger.rs`).

use serde_json::{Map, Value, json};

use crate::pipeline::nodes::shared::util::with_answer;
use crate::pipeline::NodeDefinition;

/// Every trigger's answer key (its source), in one place for the readers that
/// must find a trigger's answer in a payload — the render boundary that
/// filters claims, the Studio's Run form.
pub const SOURCE_KEYS: [&str; 9] =
    ["webhook", "manual", "schedule", "function", "mcp", "error", "room", "socket", "topic"];

/// A trigger's answer: the envelope the ingress delivered, nested under the
/// trigger's `source` — the only root key a trigger adds. The platform's
/// private `__zf_*` keys stay at the root, where the engine takes them off.
pub fn answer_under(source: &str, payload: Value) -> Value {
    let mut private = Map::new();
    let envelope = match payload {
        Value::Object(map) => {
            let mut envelope = Map::new();
            for (key, value) in map {
                if key.starts_with("__zf_") {
                    private.insert(key, value);
                } else {
                    envelope.insert(key, value);
                }
            }
            Value::Object(envelope)
        }
        Value::Null => json!({}),
        other => other,
    };
    let mut answer = Map::new();
    answer.insert(source.to_string(), envelope);
    with_answer(&Value::Object(private), Value::Object(answer))
}

/// The run's trigger envelope (`$trigger`), from a node's metadata.
pub fn envelope(metadata: &Value) -> &Value {
    metadata.get("trigger").unwrap_or(&Value::Null)
}

pub mod function;
pub mod kv_subscribe;
pub mod manual;
pub mod mcp_trigger;
pub mod schedule;
pub mod weberror;
pub mod webhook;
pub mod ws_client;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![
        function::definition(),
        kv_subscribe::definition(),
        webhook::definition(),
        schedule::definition(),
        manual::definition(),
        mcp_trigger::definition(),
        ws_client::definition(),
        weberror::definition(),
    ]
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::answer_under;

    #[test]
    fn the_envelope_goes_under_the_source_and_private_keys_stay_at_the_root() {
        let out = answer_under(
            "webhook",
            json!({ "body": { "a": 1 }, "query": {}, "__zf_private_trace_redact": ["x"] }),
        );
        assert_eq!(
            out,
            json!({ "webhook": { "body": { "a": 1 }, "query": {} }, "__zf_private_trace_redact": ["x"] })
        );
        assert_eq!(answer_under("manual", json!(null)), json!({ "manual": {} }));
    }
}

#[cfg(test)]
mod answer_tests {
    use serde_json::json;

    use crate::pipeline::nodes::{NodeExecutionInput, NodeHandler};

    /// Every trigger adds one key, its source, holding the envelope it was
    /// given — and nothing else lands at the root.
    #[tokio::test]
    async fn each_trigger_answers_under_its_source() {
        let envelope = json!({ "body": { "a": 1 }, "message": "m" });
        let nodes: Vec<(Box<dyn NodeHandler>, &str)> = vec![
            (Box::new(super::webhook::Node::new(Default::default())), "webhook"),
            (Box::new(super::manual::Node::new(Default::default())), "manual"),
            (Box::new(super::schedule::Node::new(Default::default())), "schedule"),
            (Box::new(super::function::Node::new(Default::default())), "function"),
            (Box::new(super::mcp_trigger::Node::new(Default::default())), "mcp"),
            (Box::new(super::weberror::Node::new(Default::default())), "error"),
            (Box::new(super::ws_client::Node::new(Default::default())), "socket"),
            (Box::new(super::kv_subscribe::Node::new(Default::default())), "topic"),
            (Box::new(crate::pipeline::nodes::basic::ws::trigger::Node::new(Default::default())), "room"),
        ];
        for (node, key) in nodes {
            let out = node
                .execute_async(NodeExecutionInput {
                    node_id: "t".into(),
                    input_pin: "in".into(),
                    payload: envelope.clone(),
                    metadata: json!({}),
                    bus: None,
                })
                .await
                .unwrap();
            assert_eq!(out.payload, json!({ key: envelope.clone() }), "{}", node.kind());
            assert_eq!(crate::pipeline::nodes::answer_key(node.kind()).as_deref(), Some(key));
            assert!(super::SOURCE_KEYS.contains(&key));
        }
    }
}
