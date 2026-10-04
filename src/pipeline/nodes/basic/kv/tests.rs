//! One test per behaviour of the `kv.*` kinds, on an in-memory bus.

use std::sync::Arc;

use serde_json::{Value, json};

use super::{KINDS, build, definitions};
use crate::infra::io::state::{DynStateBus, MemStateBus};
use crate::pipeline::nodes::{NodeExecutionInput, NodeExecutionOutput};

fn bus() -> DynStateBus {
    Arc::new(MemStateBus::new())
}

fn input(payload: Value) -> NodeExecutionInput {
    NodeExecutionInput {
        node_id: "n1".to_string(),
        input_pin: "in".to_string(),
        payload,
        metadata: json!({ "owner": "superadmin", "project": "default", "pipeline": "t", "request_id": "r" }),
        bus: None,
    }
}

/// Build `kind` on `bus` and run it once on `{ "kept": 1 }`.
async fn run(kind: &str, config: Value, bus: &DynStateBus) -> NodeExecutionOutput {
    let node = build(kind, &config, Some(bus.clone())).expect("builds").expect("a kv kind");
    node.execute_async(input(json!({ "kept": 1 }))).await.expect("runs")
}

fn refusal(kind: &str, config: Value) -> &'static str {
    match build(kind, &config, Some(bus())) {
        Err(err) => err.code,
        Ok(_) => panic!("{kind} accepted {config}"),
    }
}

#[tokio::test]
async fn put_then_get_answers_entry_and_keeps_the_payload() {
    let bus = bus();
    let put = run("kv.entry.put", json!({ "key": "k", "value": { "a": 1 }, "ttl": "10m" }), &bus).await;
    assert_eq!(put.payload, json!({ "kept": 1, "entry": { "key": "k", "ttl": "600s" } }));

    let got = run("kv.entry.get", json!({ "key": "k" }), &bus).await;
    assert_eq!(got.payload, json!({ "kept": 1, "entry": { "key": "k", "value": { "a": 1 }, "found": true } }));
}

#[tokio::test]
async fn a_missing_key_answers_found_false_with_the_default() {
    let got = run("kv.entry.get", json!({ "key": "nothing", "default": { "theme": "light" } }), &bus()).await;
    assert_eq!(got.payload["entry"], json!({ "key": "nothing", "value": { "theme": "light" }, "found": false }));
    let bare = run("kv.entry.get", json!({ "key": "nothing" }), &bus()).await;
    assert_eq!(bare.payload["entry"]["value"], Value::Null);
}

#[tokio::test]
async fn head_delete_increment_and_expire_answer_entry() {
    let bus = bus();
    run("kv.entry.put", json!({ "key": "k", "value": "v" }), &bus).await;
    let head = run("kv.entry.head", json!({ "key": "k" }), &bus).await;
    assert_eq!(head.payload, json!({ "kept": 1, "entry": { "key": "k", "exists": true } }));

    let expire = run("kv.entry.expire", json!({ "key": "k", "ttl": "30m" }), &bus).await;
    assert_eq!(expire.payload["entry"], json!({ "key": "k", "ttl": "1800s", "updated": true }));
    let persist = run("kv.entry.expire", json!({ "key": "k" }), &bus).await;
    assert_eq!(persist.payload["entry"], json!({ "key": "k", "updated": true }), "no --ttl removes the expiry");

    let deleted = run("kv.entry.delete", json!({ "key": "k" }), &bus).await;
    assert_eq!(deleted.payload, json!({ "kept": 1, "entry": { "key": "k", "deleted": true } }));
    let again = run("kv.entry.delete", json!({ "key": "k" }), &bus).await;
    assert_eq!(again.payload["entry"]["deleted"], json!(false));

    run("kv.entry.increment", json!({ "key": "c" }), &bus).await;
    let counted = run("kv.entry.increment", json!({ "key": "c", "amount": "5" }), &bus).await;
    assert_eq!(counted.payload, json!({ "kept": 1, "entry": { "key": "c", "value": 6 } }));
}

#[tokio::test]
async fn publish_answers_message_and_reaches_a_listener() {
    let bus = bus();
    let mut sub = bus.subscribe("superadmin", "default", "order.placed").expect("subscribe");
    let out = run("kv.message.publish", json!({ "topic": "order.placed", "body": { "id": 7 } }), &bus).await;
    assert_eq!(out.payload, json!({ "kept": 1, "message": { "topic": "order.placed", "delivered": 1 } }));
    assert_eq!(sub.recv().await.expect("a message"), json!({ "id": 7 }));
}

#[test]
fn empty_values_and_bad_lifetimes_are_refused() {
    assert_eq!(refusal("kv.entry.get", json!({ "key": " " })), "FW_NODE_KV_ENTRY_GET_KEY");
    assert_eq!(refusal("kv.entry.put", json!({ "key": "k" })), "FW_NODE_KV_ENTRY_PUT_VALUE");
    assert_eq!(refusal("kv.entry.put", json!({ "key": "k", "value": "" })), "FW_NODE_KV_ENTRY_PUT_VALUE");
    for ttl in [json!("600"), json!(600), json!("0s"), json!("500ms"), json!("soon")] {
        assert_eq!(refusal("kv.entry.put", json!({ "key": "k", "value": 1, "ttl": ttl })), "FW_NODE_KV_ENTRY_PUT_CONFIG", "{ttl}");
    }
    assert_eq!(refusal("kv.entry.expire", json!({ "key": "k", "ttl": "0" })), "FW_NODE_KV_ENTRY_EXPIRE_CONFIG");
    assert_eq!(refusal("kv.entry.increment", json!({ "key": "k", "amount": "1.5" })), "FW_NODE_KV_ENTRY_INCREMENT_AMOUNT");
    assert_eq!(refusal("kv.message.publish", json!({ "body": 1 })), "FW_NODE_KV_MESSAGE_PUBLISH_TOPIC");
    assert_eq!(refusal("kv.message.publish", json!({ "topic": "t" })), "FW_NODE_KV_MESSAGE_PUBLISH_BODY");
}

/// `zf.` is the platform's own: a published route's sign-ins live there.
#[test]
fn the_platform_prefix_is_refused_to_every_kv_node() {
    assert_eq!(refusal("kv.entry.put", json!({ "key": "zf.oauth/x", "value": 1 })), "FW_NODE_KV_ENTRY_PUT_KEY");
    assert_eq!(refusal("kv.entry.get", json!({ "key": "zf.oauth/ticket/x" })), "FW_NODE_KV_ENTRY_GET_KEY");
    assert_eq!(refusal("kv.entry.delete", json!({ "key": "zf.x" })), "FW_NODE_KV_ENTRY_DELETE_KEY");
    assert_eq!(refusal("kv.entry.increment", json!({ "key": "zf.x" })), "FW_NODE_KV_ENTRY_INCREMENT_KEY");
}

#[test]
fn retired_flags_are_refused_by_the_dsl() {
    use crate::platform::shell::parser::build_pipeline_graph;
    for retired in [
        "kv.entry.get --key k --out-key x",
        "kv.entry.head --key k --out-key cached",
        "kv.message.publish --channel t --payload x",
    ] {
        let dsl = format!("| trigger.manual | {retired}");
        assert!(build_pipeline_graph("kv-retired", &dsl).is_err(), "{retired} must be refused");
    }
    assert!(build_pipeline_graph("kv-ok", "| trigger.manual | kv.message.publish --topic t --body x").is_ok());
}

#[test]
fn an_engine_without_a_bus_says_so() {
    let err = build("kv.entry.get", &json!({ "key": "k" }), None).err().expect("refused");
    assert_eq!(err.code, "FW_NODE_KV_ENTRY_GET_UNAVAILABLE");
}

#[test]
fn the_flags_are_the_0_11_words() {
    let flags = |kind: &str| -> Vec<String> {
        definitions().into_iter().find(|d| d.kind == kind).expect(kind).dsl_flags.into_iter().map(|f| f.flag).collect()
    };
    assert_eq!(flags("kv.entry.get"), ["--key", "--default", "--durable"]);
    assert_eq!(flags("kv.entry.put"), ["--key", "--value", "--ttl", "--durable"]);
    assert_eq!(flags("kv.entry.increment"), ["--key", "--amount", "--durable"]);
    assert_eq!(flags("kv.message.publish"), ["--topic", "--body"]);
    assert_eq!(KINDS.len(), definitions().len());
}
