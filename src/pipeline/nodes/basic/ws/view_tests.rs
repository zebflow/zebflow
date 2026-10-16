//! Views (`docs/contracts/rooms.md` § Views): `ws.session.tag`,
//! `ws.state.* --visible-to` and `ws.message.send --recipient tag:`.

use std::sync::Arc;

use serde_json::{Value, json};

use super::{build, message_send, session_tag, state};
use crate::infra::transport::ws::{Attached, RoomDecl, SessionGuard, WsHub};
use crate::pipeline::PipelineError;
use crate::pipeline::nodes::{NodeExecutionInput, NodeExecutionOutput};

fn hub() -> Arc<WsHub> {
    let hub = Arc::new(WsHub::new());
    hub.declare("demo", "site-a", &[RoomDecl::new("table", "pipelines/table")]);
    hub
}

/// A socket seated at the table: its seat and its frames.
struct Sock {
    _guard: SessionGuard,
    attached: Attached,
}

fn sit(hub: &WsHub, id: &str) -> Sock {
    let (room, guard, _kicks) = hub.join("demo", "site-a", "table", id, None, None).expect("join");
    Sock { _guard: guard, attached: room.attach(id).expect("seated") }
}

/// Every frame queued for a socket, as sent.
fn raw(sock: &mut Sock) -> Vec<String> {
    std::iter::from_fn(|| sock.attached.frames.try_recv().ok()).map(|f| f.to_string()).collect()
}

/// A run started by `trigger.room` in the table, for `session`.
fn from(session: &str) -> NodeExecutionInput {
    let envelope = json!({ "room_id": "table", "session_id": session, "event": "sit", "payload": {} });
    // What trigger.room answers: its envelope under `room`.
    let answered = json!({ "room": envelope.clone() });
    NodeExecutionInput {
        node_id: "n1".to_string(),
        input_pin: "in".to_string(),
        payload: answered,
        metadata: json!({ "owner": "demo", "project": "site-a", "pipeline": "t", "request_id": "r", "trigger": envelope }),
        bus: None,
    }
}

async fn run(kind: &str, config: Value, hub: &Arc<WsHub>, session: &str) -> Result<NodeExecutionOutput, PipelineError> {
    let node: Box<dyn crate::pipeline::nodes::NodeHandler> = match kind {
        "ws.message.send" => Box::new(message_send::Node::new(serde_json::from_value(config).unwrap(), Some(hub.clone()), None)?),
        "ws.state.put" | "ws.state.update" | "ws.state.delete" => {
            Box::new(state::Node::new(state::Verb::of_kind(kind).unwrap(), serde_json::from_value(config).unwrap(), Some(hub.clone()))?)
        }
        _ => build(kind, &config, Some(hub.clone()))?.expect("a ws kind"),
    };
    node.execute_async(from(session)).await
}

fn code<T>(result: Result<T, PipelineError>) -> &'static str {
    match result {
        Err(err) => err.code,
        Ok(_) => panic!("accepted"),
    }
}

#[tokio::test]
async fn untag_applies_before_tag_so_a_cell_move_keeps_the_team() {
    let hub = hub();
    let _s = sit(&hub, "ws-1f0c");
    let set = run(session_tag::NODE_KIND, json!({ "tag": ["team:red", "cell:11:7"] }), &hub, "ws-1f0c").await.expect("tag");
    assert_eq!(set.payload["session"], json!({ "session_id": "ws-1f0c", "tags": ["cell:11:7", "team:red"] }), "the triggering session by default");
    let moved = run(
        session_tag::NODE_KIND,
        json!({ "session": "ws-1f0c", "untag": ["cell:*"], "tag": ["cell:12:7", "cell:12:8"] }),
        &hub,
        "someone-else",
    )
    .await
    .expect("move");
    assert_eq!(moved.payload["session"]["tags"], json!(["cell:12:7", "cell:12:8", "team:red"]));
    assert_eq!(moved.payload["room"]["session_id"], "someone-else", "the payload is kept");
    let readd = run(session_tag::NODE_KIND, json!({ "untag": "cell:12:7", "tag": "cell:12:7" }), &hub, "ws-1f0c").await.unwrap();
    assert_eq!(readd.payload["session"]["tags"], json!(["cell:12:7", "cell:12:8", "team:red"]), "removed, then added back");
    let all = run(session_tag::NODE_KIND, json!({ "untag": "*" }), &hub, "ws-1f0c").await.unwrap();
    assert_eq!(all.payload["session"]["tags"], json!([]), "* removes every tag");
}

#[tokio::test]
async fn session_tag_refuses_what_it_cannot_do() {
    let hub = hub();
    let _s = sit(&hub, "s1");
    for bad in [json!({}), json!({ "tag": "" }), json!({ "tag": "team red" }), json!({ "tag": "cell:*" }), json!({ "untag": "ce*ll" }), json!({ "tag": "x".repeat(129) })] {
        assert_eq!(code(build(session_tag::NODE_KIND, &bad, Some(hub.clone()))), session_tag::CONFIG_CODE, "{bad}");
    }
    assert_eq!(code(run(session_tag::NODE_KIND, json!({ "session": "s9", "tag": "a" }), &hub, "s1").await), session_tag::SESSION_CODE, "not in the room");
    assert_eq!(code(run(session_tag::NODE_KIND, json!({ "room": "t*", "tag": "a" }), &hub, "s1").await), session_tag::ROOM_CODE);
    assert_eq!(code(run(session_tag::NODE_KIND, json!({ "room": "hall", "tag": "a" }), &hub, "s1").await), super::ROOM_NOT_OPEN_CODE);
}

#[tokio::test]
async fn a_visible_to_path_never_reaches_an_untagged_socket_and_tagged_ones_share_its_bytes() {
    let hub = hub();
    let (mut seat2, mut seat2_again, mut watcher) = (sit(&hub, "s2"), sit(&hub, "s2b"), sit(&hub, "w"));
    for id in ["s2", "s2b"] {
        run(session_tag::NODE_KIND, json!({ "tag": "seat:2" }), &hub, id).await.unwrap();
    }
    let put = json!({ "key": "/hands/s2", "value": ["queen-of-hidden"], "visible_to": ["seat:2"] });
    run("ws.state.put", put, &hub, "s2").await.expect("put");
    run("ws.state.put", json!({ "key": "/turn", "value": 2 }), &hub, "s2").await.expect("put");
    let (mine, mine_again, theirs) = (raw(&mut seat2), raw(&mut seat2_again), raw(&mut watcher));
    assert!(theirs.iter().all(|f| !f.contains("queen-of-hidden")), "never in the untagged socket's frames: {theirs:?}");
    assert!(theirs.iter().any(|f| f.contains("\"turn\"")), "the open part still reaches it");
    assert!(mine.iter().any(|f| f.contains("queen-of-hidden")));
    assert_eq!(mine, mine_again, "the same tags, the same bytes");
    let delete = serde_json::from_value(json!({ "key": "/a", "visible_to": "x" })).unwrap();
    assert_eq!(code(state::Node::new(state::Verb::Delete, delete, Some(hub.clone()))), "FW_NODE_WS_STATE_DELETE_CONFIG", "a delete writes nothing to limit");
}

#[tokio::test]
async fn a_tag_recipient_reaches_only_the_sessions_holding_it() {
    let hub = hub();
    let (mut red, mut blue) = (sit(&hub, "red"), sit(&hub, "blue"));
    run(session_tag::NODE_KIND, json!({ "tag": "team:red" }), &hub, "red").await.unwrap();
    let out = run("ws.message.send", json!({ "body": "attack at dawn", "event": "team.chat", "recipient": "tag:team:red" }), &hub, "blue")
        .await
        .expect("send");
    assert_eq!(out.payload["message"]["sent"], true);
    assert!(raw(&mut red).iter().any(|f| f.contains("attack at dawn")));
    assert!(raw(&mut blue).is_empty(), "blue holds no team:red");
}

#[tokio::test]
async fn the_member_list_shows_each_session_s_tags() {
    let hub = hub();
    let _s = sit(&hub, "s1");
    run(session_tag::NODE_KIND, json!({ "tag": ["seat:1", "team:red"] }), &hub, "s1").await.unwrap();
    let listed = run("ws.room.list", json!({ "room": "table", "members": true }), &hub, "s1").await.unwrap();
    assert_eq!(listed.payload["room"]["rooms"][0]["members"], json!([ { "session": "s1", "sub": null, "tags": ["seat:1", "team:red"], "away": false } ]));
}
