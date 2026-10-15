//! One test per behaviour of the room-management kinds: open, ticket,
//! close, list and session close, each through the family's builder.

use std::sync::Arc;
use std::time::Duration;

use serde_json::{Value, json};

use super::{ROOM_NOT_OPEN_CODE, build, room_close, room_list, room_open, session_close, ticket_create};
use crate::infra::transport::ws::{KICK_CLOSED, KICK_ROOM_CLOSED, RoomSettings, WsHub};
use crate::pipeline::PipelineError;
use crate::pipeline::nodes::{NodeExecutionInput, NodeExecutionOutput};

fn hub() -> Arc<WsHub> {
    let hub = Arc::new(WsHub::new());
    hub.declare(
        "demo",
        "site-a",
        &[("lobby".to_string(), "pipelines/lobby".to_string()), ("match-:id".to_string(), "pipelines/match".to_string())],
    );
    hub
}

fn input(payload: Value) -> NodeExecutionInput {
    NodeExecutionInput {
        node_id: "n1".to_string(),
        input_pin: "in".to_string(),
        payload,
        metadata: json!({ "owner": "demo", "project": "site-a", "pipeline": "pipelines/matches/open", "request_id": "r" }),
        bus: None,
    }
}

async fn run(kind: &str, config: Value, hub: &Arc<WsHub>) -> Result<NodeExecutionOutput, PipelineError> {
    let node = build(kind, &config, Some(hub.clone()))?.expect("a room-management kind");
    node.execute_async(input(json!({ "kept": 1 }))).await
}

fn code<T>(result: Result<T, PipelineError>) -> &'static str {
    match result {
        Err(err) => err.code,
        Ok(_) => panic!("accepted"),
    }
}

#[tokio::test]
async fn open_answers_the_room_as_listed_and_keeps_the_payload() {
    let hub = hub();
    let out = run(
        room_open::NODE_KIND,
        json!({ "room": "match-1", "private": true, "max_sessions": "4", "meta": { "mode": "ranked" }, "idle": "30s", "hold": "30s", "max_state": "64KB" }),
        &hub,
    )
    .await
    .expect("open");
    assert_eq!(out.payload["kept"], 1);
    let room = &out.payload["room"];
    assert_eq!((room["room"].as_str(), room["kind"].as_str(), room["private"].as_bool()), (Some("match-1"), Some("opened"), Some(true)));
    assert_eq!(room["max_sessions"], 4);
    assert_eq!(room["meta"], json!({ "mode": "ranked" }));
    assert_eq!(room["opened_by"], "pipelines/matches/open");
    let settings = hub.room("demo", "site-a", "match-1").unwrap().settings().clone();
    assert_eq!((settings.idle, settings.hold, settings.max_state), (Duration::from_secs(30), Some(Duration::from_secs(30)), 64_000));
}

#[tokio::test]
async fn open_has_defaults_and_refuses_what_the_contract_refuses() {
    let hub = hub();
    run(room_open::NODE_KIND, json!({ "room": "match-1" }), &hub).await.expect("defaults");
    let settings = hub.room("demo", "site-a", "match-1").unwrap().settings().clone();
    assert_eq!((settings.private, settings.max_sessions, settings.max_state), (false, Some(1000), 1_000_000));
    assert_eq!(settings.idle, Duration::from_secs(600));
    assert_eq!(code(run(room_open::NODE_KIND, json!({ "room": "match-1" }), &hub).await), room_open::TAKEN_CODE);
    assert_eq!(code(run(room_open::NODE_KIND, json!({ "room": "zone-1" }), &hub).await), room_open::PATTERN_CODE);
    assert_eq!(code(run(room_open::NODE_KIND, json!({ "room": "lobby" }), &hub).await), room_open::FIXED_CODE);
    for bad in [
        json!({ "room": "match-*" }),
        json!({ "room": "" }),
        json!({ "room": "match-2", "meta": [1] }),
        json!({ "room": "match-2", "meta": { "blob": "x".repeat(5000) } }),
        json!({ "room": "match-2", "max_sessions": 0 }),
        json!({ "room": "match-2", "max_sessions": 10_001 }),
        json!({ "room": "match-2", "idle": "500ms" }),
        json!({ "room": "match-2", "max_state": "17MB" }),
        json!({ "room": "match-2", "hold": "2h" }),
    ] {
        assert_eq!(code(build(room_open::NODE_KIND, &bad, Some(hub.clone()))), room_open::CONFIG_CODE, "{bad}");
    }
    assert_eq!(code(build(room_open::NODE_KIND, &json!({ "room": "match-2" }), None)), room_open::UNAVAILABLE_CODE);
}

#[tokio::test]
async fn a_ticket_names_its_room_and_expiry_and_admits_once() {
    let hub = hub();
    assert_eq!(code(run(ticket_create::NODE_KIND, json!({ "room": "match-1", "user": "u-17" }), &hub).await), ROOM_NOT_OPEN_CODE);
    hub.open("demo", "site-a", "match-1", RoomSettings { max_sessions: Some(1), ..RoomSettings::fixed("t") }).unwrap();
    let out = run(ticket_create::NODE_KIND, json!({ "room": "match-1", "user": "u-17", "ttl": "2m" }), &hub).await.expect("ticket");
    let ticket = &out.payload["ticket"];
    assert_eq!(ticket["room"], "match-1");
    let expires = chrono::DateTime::parse_from_rfc3339(ticket["expires_at"].as_str().unwrap()).expect("rfc3339");
    let left = expires.timestamp() - chrono::Utc::now().timestamp();
    assert!((100..=121).contains(&left), "about two minutes: {left}");
    let pass = hub.read_ticket("demo", "site-a", "match-1", ticket["ticket"].as_str().unwrap()).expect("verifies");
    assert_eq!(pass.user, "u-17");
    let _seat = hub.join("demo", "site-a", "match-1", "s1", Some(pass.user.clone()), Some(&pass)).unwrap();
    assert_eq!(code(run(ticket_create::NODE_KIND, json!({ "room": "match-1", "user": "u-18" }), &hub).await), ticket_create::FULL_CODE);
    for (bad, want) in [
        (json!({ "room": "match-1", "user": "" }), ticket_create::EMPTY_CODE),
        (json!({ "room": "", "user": "u-1" }), ticket_create::EMPTY_CODE),
        (json!({ "room": "match-*", "user": "u-1" }), ticket_create::CONFIG_CODE),
        (json!({ "room": "match-1", "user": "u-1", "ttl": "11m" }), ticket_create::CONFIG_CODE),
    ] {
        assert_eq!(code(build(ticket_create::NODE_KIND, &bad, Some(hub.clone()))), want, "{bad}");
    }
}

#[tokio::test]
async fn close_kicks_every_session_and_answers_what_it_closed() {
    let hub = hub();
    for room in ["match-1", "match-2"] {
        hub.open("demo", "site-a", room, RoomSettings::fixed("t")).unwrap();
    }
    let (_r, _g, mut kicked) = hub.join("demo", "site-a", "match-1", "s1", None, None).unwrap();
    assert_eq!(code(run(room_close::NODE_KIND, json!({ "room": "match-9" }), &hub).await), ROOM_NOT_OPEN_CODE);
    let out = run(room_close::NODE_KIND, json!({ "room": "match-*" }), &hub).await.expect("close");
    assert_eq!(out.payload["room"], json!({ "room": "match-*", "closed": ["match-1", "match-2"], "sessions": 1 }));
    assert_eq!(kicked.try_recv().unwrap(), KICK_ROOM_CLOSED);
    assert!(hub.room("demo", "site-a", "match-1").is_none());
    assert_eq!(code(run(room_close::NODE_KIND, json!({}), &hub).await), room_close::ROOM_CODE, "no room and no trigger");
}

#[tokio::test]
async fn list_answers_the_contracts_json_under_room() {
    let hub = hub();
    hub.open("demo", "site-a", "match-1", RoomSettings::fixed("t")).unwrap();
    let (_r, _g, _k) = hub.join("demo", "site-a", "match-1", "s1", Some("u-17".into()), None).unwrap();
    let out = run(room_list::NODE_KIND, json!({}), &hub).await.expect("list");
    assert_eq!(out.payload["room"]["total"], json!({ "rooms": 2, "sessions": 1 }), "lobby and match-1");
    assert_eq!(out.payload["kept"], 1);
    let out = run(room_list::NODE_KIND, json!({ "room": "match-*", "members": true }), &hub).await.expect("list");
    assert_eq!(out.payload["room"]["rooms"][0]["members"], json!(["u-17"]));
    assert_eq!(out.payload["room"], hub.list("demo", "site-a", "match-*", true).unwrap(), "the same JSON the API answers");
    assert_eq!(code(build(room_list::NODE_KIND, &json!({ "room": "a b" }), Some(hub.clone()))), room_list::CONFIG_CODE);
}

#[tokio::test]
async fn session_close_takes_one_session_or_one_user() {
    let hub = hub();
    let (_r1, _g1, mut a) = hub.join("demo", "site-a", "lobby", "s1", Some("u-17".into()), None).unwrap();
    let (_r2, _g2, mut b) = hub.join("demo", "site-a", "lobby", "s2", Some("u-18".into()), None).unwrap();
    let out = run(session_close::NODE_KIND, json!({ "room": "lobby", "user": "u-17" }), &hub).await.expect("close");
    assert_eq!(out.payload["session"], json!({ "room": "lobby", "closed": 1 }));
    assert_eq!((a.try_recv().ok(), b.try_recv().ok()), (Some(KICK_CLOSED), None));
    run(session_close::NODE_KIND, json!({ "room": "lobby", "session": "s2" }), &hub).await.expect("close");
    assert_eq!(b.try_recv().ok(), Some(KICK_CLOSED));
    for bad in [json!({ "room": "lobby" }), json!({ "room": "lobby", "session": "s1", "user": "u-17" })] {
        assert_eq!(code(build(session_close::NODE_KIND, &bad, Some(hub.clone()))), session_close::CONFIG_CODE, "{bad}");
    }
    assert_eq!(code(run(session_close::NODE_KIND, json!({ "room": "match-3", "user": "u-1" }), &hub).await), ROOM_NOT_OPEN_CODE);
}
