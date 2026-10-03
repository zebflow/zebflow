//! One test per behaviour of each `ws.*` action kind. Rooms live in a fresh
//! hub; a client socket is a channel standing in for the open connection.

use std::sync::Arc;

use serde_json::{Value, json};
use tokio::sync::mpsc;

use super::message_send::{self, Config as SendConfig};
use super::state::{self, Config as StateConfig, Verb};
use crate::infra::transport::ws::{RoomHandle, SessionGuard, WsHub};
use crate::infra::ws_client::WsClientManager;
use crate::pipeline::engines::BasicPipelineEngine;
use crate::pipeline::interface::PipelineEngine;
use crate::pipeline::model::PipelineContext;
use crate::pipeline::nodes::{NodeExecutionInput, NodeHandler};
use crate::pipeline::PipelineError;
use crate::platform::model::PlatformConfig;
use crate::platform::services::PlatformService;
use crate::platform::shell::parser::build_pipeline_graph;

fn input(payload: Value) -> NodeExecutionInput {
    NodeExecutionInput {
        node_id: "n1".to_string(),
        input_pin: "in".to_string(),
        payload,
        metadata: json!({ "owner": "demo", "project": "site-a", "pipeline": "t", "request_id": "r" }),
        bus: None,
    }
}

/// What `trigger.room` hands the next node.
fn from_room(room: &str, session: &str) -> Value {
    json!({ "room_id": room, "session_id": session, "event": "move", "payload": { "x": 1 }, "kept": 1 })
}

/// A hub with one socket joined to `room` of the test project.
fn hub_with(room: &str) -> (Arc<WsHub>, Arc<RoomHandle>, SessionGuard) {
    let hub = Arc::new(WsHub::new());
    let (handle, guard) = hub.join_room(&format!("demo/site-a/{room}"));
    (hub, handle, guard)
}

/// A client socket manager with one open socket, `file:node`, whose sends arrive on the receiver.
async fn manager_with(file: &str, node: &str) -> (Arc<WsClientManager>, mpsc::Receiver<String>, tempfile::TempDir) {
    let dir = tempfile::tempdir().expect("temp dir");
    let mut cfg = PlatformConfig::default();
    cfg.data_root = dir.path().to_path_buf();
    let platform = PlatformService::from_config(cfg).expect("platform");
    let manager = Arc::new(WsClientManager::new(
        platform.pipeline_runtime.clone(),
        Arc::new(BasicPipelineEngine::default()),
        platform.pipeline_hits.clone(),
        platform.data.clone(),
        platform.zebflow_cfg.clone(),
    ));
    let (tx, rx) = mpsc::channel(4);
    manager.attach_sender(&format!("demo/site-a/{file}:{node}"), tx).await;
    (manager, rx, dir)
}

fn send_node(config: Value, hub: Option<Arc<WsHub>>, manager: Option<Arc<WsClientManager>>) -> Result<message_send::Node, PipelineError> {
    message_send::Node::new(serde_json::from_value::<SendConfig>(config).expect("config"), hub, manager)
}

fn state_node(verb: Verb, config: Value, hub: &Arc<WsHub>) -> Result<state::Node, PipelineError> {
    state::Node::new(verb, serde_json::from_value::<StateConfig>(config).expect("config"), Some(hub.clone()))
}

fn code<T>(result: Result<T, PipelineError>) -> &'static str {
    match result {
        Err(err) => err.code,
        Ok(_) => panic!("accepted"),
    }
}

/// The room's state once the actor has applied the command: an immediate
/// change is broadcast after it is applied.
async fn state_after(handle: &RoomHandle, run: impl std::future::Future<Output = ()>) -> Value {
    let mut rx = handle.subscribe();
    run.await;
    let got = rx.recv().await.expect("state broadcast");
    let message: Value = serde_json::from_str(&got.text).expect("json");
    assert_eq!(message["type"], "state_patch");
    handle.get_state()
}

#[test]
fn the_family_is_four_kinds_with_their_signatures() {
    let signatures: Vec<String> = super::definitions()
        .iter()
        .filter(|def| !def.kind.starts_with("trigger."))
        .map(crate::pipeline::nodes::node_signature)
        .collect();
    assert_eq!(
        signatures,
        [
            "ws.message.send [--room TEXT] [--connection TEXT] --body JSON [--event TEXT] [--recipient all|session|others] → message",
            "ws.state.put --key TEXT --value JSON [--room TEXT] [--batch] → state",
            "ws.state.update --key TEXT --value JSON [--room TEXT] [--batch] → state",
            "ws.state.delete --key TEXT [--room TEXT] [--batch] → state",
        ]
    );
}

// ── ws.message.send ─────────────────────────────────────────────────────────

#[tokio::test]
async fn a_room_send_reaches_the_rooms_sockets_and_answers_message() {
    let (hub, handle, _guard) = hub_with("lobby");
    let mut rx = handle.subscribe();
    let node = send_node(json!({ "body": { "text": "hi" }, "event": "chat" }), Some(hub), None).unwrap();
    let out = node.execute_async(input(from_room("lobby", "s1"))).await.unwrap();

    assert_eq!(out.payload["message"], json!({ "sent": true, "room": "lobby", "event": "chat" }));
    assert_eq!(out.payload["kept"], 1, "the rest of the payload is kept");
    assert_eq!(out.payload["room_id"], "lobby");
    let got = rx.recv().await.expect("event");
    assert!(got.is_for("s1") && got.is_for("s2"), "all is the default recipient");
    let wire: Value = serde_json::from_str(&got.text).unwrap();
    assert_eq!(wire, json!({ "type": "event", "event": "chat", "payload": { "text": "hi" } }));
}

#[tokio::test]
async fn a_named_room_needs_no_trigger_and_the_event_defaults() {
    let (hub, handle, _guard) = hub_with("dashboard");
    let mut rx = handle.subscribe();
    let node = send_node(json!({ "room": "dashboard", "body": 3 }), Some(hub), None).unwrap();
    let out = node.execute_async(input(json!({}))).await.unwrap();
    assert_eq!(out.payload["message"], json!({ "sent": true, "room": "dashboard", "event": "event" }));
    let wire: Value = serde_json::from_str(&rx.recv().await.unwrap().text).unwrap();
    assert_eq!(wire["payload"], 3);
}

#[tokio::test]
async fn a_room_nobody_joined_answers_not_sent() {
    let hub = Arc::new(WsHub::new());
    let node = send_node(json!({ "room": "empty", "body": "x" }), Some(hub), None).unwrap();
    let out = node.execute_async(input(json!({}))).await.unwrap();
    assert_eq!(out.payload["message"]["sent"], false);
}

#[tokio::test]
async fn a_session_or_others_send_targets_the_triggering_socket() {
    let (hub, handle, _guard) = hub_with("lobby");
    let mut rx = handle.subscribe();
    for (recipient, to_s1, to_s2) in [("session", true, false), ("others", false, true)] {
        let node = send_node(json!({ "body": "x", "recipient": recipient }), Some(hub.clone()), None).unwrap();
        node.execute_async(input(from_room("lobby", "s1"))).await.unwrap();
        let got = rx.recv().await.unwrap();
        assert_eq!((got.is_for("s1"), got.is_for("s2")), (to_s1, to_s2), "{recipient}");
    }
    // Without a triggering session there is nobody to single out.
    let node = send_node(json!({ "room": "lobby", "body": "x", "recipient": "session" }), Some(hub), None).unwrap();
    assert_eq!(code(node.execute_async(input(json!({}))).await), message_send::SESSION_CODE);
}

#[tokio::test]
async fn a_connection_send_goes_through_the_same_kind() {
    let (manager, mut rx, _dir) = manager_with("pipelines/feed.zf.json", "n0").await;
    // A bare node id finds the one open socket of that node.
    let node = send_node(json!({ "connection": "n0", "body": { "op": "subscribe" } }), None, Some(manager.clone())).unwrap();
    let out = node.execute_async(input(json!({ "kept": 1 }))).await.unwrap();
    assert_eq!(out.payload["message"], json!({ "sent": true, "connection": "n0" }));
    assert_eq!(out.payload["kept"], 1);
    assert_eq!(rx.recv().await.unwrap(), r#"{"op":"subscribe"}"#, "a non-text body goes as JSON text");

    // The qualified form names the pipeline; a text body goes as is.
    let node = send_node(json!({ "connection": "pipelines/feed.zf.json:n0", "body": "ping" }), None, Some(manager.clone())).unwrap();
    node.execute_async(input(json!({}))).await.unwrap();
    assert_eq!(rx.recv().await.unwrap(), "ping");

    // A socket that is not open fails, naming the connection.
    let node = send_node(json!({ "connection": "n9", "body": "ping" }), None, Some(manager)).unwrap();
    assert_eq!(code(node.execute_async(input(json!({}))).await), message_send::CONNECTION_CODE);
}

#[tokio::test]
async fn both_targets_or_neither_are_refused() {
    let hub = Arc::new(WsHub::new());
    let (manager, _rx, _dir) = manager_with("pipelines/feed.zf.json", "n0").await;
    let both = send_node(json!({ "room": "lobby", "connection": "n0", "body": "x" }), Some(hub.clone()), Some(manager.clone()));
    assert_eq!(code(both), message_send::CONFIG_CODE);

    // No --room, no --connection and no trigger.room upstream: no target.
    let neither = send_node(json!({ "body": "x" }), Some(hub), None).unwrap();
    assert_eq!(code(neither.execute_async(input(json!({ "kept": 1 }))).await), message_send::ROOM_CODE);
}

#[test]
fn room_only_flags_a_missing_body_and_unknown_words_are_refused() {
    let hub = Arc::new(WsHub::new());
    let refused = |config: Value| code(send_node(config, Some(hub.clone()), None));
    // `--event` and `--recipient` choose within a room; the manager is not
    // even asked for when the flags already disagree.
    assert_eq!(code(send_node(json!({ "connection": "n0", "body": "x", "event": "e" }), None, None)), message_send::CONFIG_CODE);
    assert_eq!(code(send_node(json!({ "connection": "n0", "body": "x", "recipient": "all" }), None, None)), message_send::CONFIG_CODE);
    assert_eq!(refused(json!({ "room": "lobby", "body": "x", "recipient": "everyone" })), message_send::CONFIG_CODE);
    assert_eq!(refused(json!({ "room": "lobby" })), message_send::EMPTY_CODE, "every source is explicit");
    assert_eq!(refused(json!({ "room": "lobby", "body": "" })), message_send::EMPTY_CODE);
    assert_eq!(code(send_node(json!({ "room": "lobby", "body": "x" }), None, None)), message_send::UNAVAILABLE_CODE);
}

// ── ws.state.* ──────────────────────────────────────────────────────────────

#[tokio::test]
async fn put_update_and_delete_change_the_rooms_state() {
    let (hub, handle, _guard) = hub_with("lobby");
    let run = |verb: Verb, config: Value| {
        let node = state_node(verb, config, &hub).unwrap();
        async move {
            let out = node.execute_async(input(from_room("lobby", "s1"))).await.unwrap();
            assert_eq!(out.payload["kept"], 1, "the rest of the payload is kept");
            assert_eq!(out.payload["room_id"], "lobby");
        }
    };

    let after = state_after(&handle, run(Verb::Put, json!({ "key": "/players/s1", "value": { "x": 0, "y": 0 } }))).await;
    assert_eq!(after, json!({ "players": { "s1": { "x": 0, "y": 0 } } }));

    let after = state_after(&handle, run(Verb::Update, json!({ "key": "/players/s1", "value": { "x": 5 } }))).await;
    assert_eq!(after, json!({ "players": { "s1": { "x": 5, "y": 0 } } }), "update merges");

    let after = state_after(&handle, run(Verb::Put, json!({ "key": "/players/s1", "value": { "x": 9 } }))).await;
    assert_eq!(after, json!({ "players": { "s1": { "x": 9 } } }), "put replaces");

    let after = state_after(&handle, run(Verb::Delete, json!({ "key": "/players/s1" }))).await;
    assert_eq!(after, json!({ "players": {} }));
}

#[tokio::test]
async fn each_verb_answers_state_with_key_and_room() {
    let (hub, _handle, _guard) = hub_with("arena");
    for (verb, config) in [
        (Verb::Put, json!({ "key": "/score", "value": 1, "room": "arena" })),
        (Verb::Update, json!({ "key": "/world", "value": { "rain": true }, "room": "arena", "batch": true })),
        (Verb::Delete, json!({ "key": "/score", "room": "arena" })),
    ] {
        let key = config["key"].clone();
        let out = state_node(verb, config, &hub).unwrap().execute_async(input(json!({ "kept": 1 }))).await.unwrap();
        assert_eq!(out.payload, json!({ "kept": 1, "state": { "key": key, "room": "arena" } }), "{verb:?}");
    }
}

#[tokio::test]
async fn a_key_part_from_the_payload_is_written_with_braces_and_resolved_by_the_engine() {
    let (hub, handle, _guard) = hub_with("lobby");
    let graph = build_pipeline_graph(
        "ws-state",
        r#"| trigger.room | ws.state.update --key "/players/{{ input.session_id }}" --value "{{ input.payload }}""#,
    )
    .expect("graph");
    let ctx = PipelineContext {
        owner: "demo".into(),
        project: "site-a".into(),
        pipeline: "ws-state".into(),
        request_id: "r".into(),
        route: String::new(),
        input: from_room("lobby", "s1"),
        trigger: None,
        placeholder: None,
    };
    let engine = BasicPipelineEngine::default().with_ws_hub(hub);
    let mut rx = handle.subscribe();
    let out = engine.execute_async(&graph, &ctx).await.expect("run");
    assert_eq!(out.value["state"], json!({ "key": "/players/s1", "room": "lobby" }));
    rx.recv().await.expect("state broadcast");
    assert_eq!(handle.get_state(), json!({ "players": { "s1": { "x": 1 } } }));

    // A part that resolves empty leaves an empty segment, which is refused
    // rather than written into (or deleted from) the whole map.
    let ctx = PipelineContext { input: json!({ "room_id": "lobby", "payload": { "x": 1 } }), ..ctx };
    let err = engine.execute_async(&graph, &ctx).await.expect_err("an empty segment");
    assert_eq!(err.code, Verb::Update.codes().key);
}

#[test]
fn a_key_that_is_not_a_full_pointer_is_refused() {
    let hub = Arc::new(WsHub::new());
    for key in ["", "players", "/players/", "/players//x", "/", "/players/{session_id}"] {
        let refused = state_node(Verb::Delete, json!({ "key": key }), &hub);
        assert_eq!(code(refused), Verb::Delete.codes().key, "{key:?}");
    }
}

#[tokio::test]
async fn values_rooms_and_engines_are_checked() {
    let hub = Arc::new(WsHub::new());
    assert_eq!(code(state_node(Verb::Put, json!({ "key": "/a" }), &hub)), Verb::Put.codes().empty, "no whole-payload default");
    assert_eq!(code(state_node(Verb::Update, json!({ "key": "/a" }), &hub)), Verb::Update.codes().empty);
    assert_eq!(code(state_node(Verb::Update, json!({ "key": "/a", "value": 3 }), &hub)), state::UPDATE_VALUE_CODE);
    let no_hub = state::Node::new(Verb::Put, serde_json::from_value(json!({ "key": "/a", "value": 1 })).unwrap(), None);
    assert_eq!(code(no_hub), Verb::Put.codes().unavailable);

    let node = state_node(Verb::Delete, json!({ "key": "/a" }), &hub).unwrap();
    assert_eq!(code(node.execute_async(input(json!({}))).await), Verb::Delete.codes().room);
}
