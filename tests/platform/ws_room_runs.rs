//! How a room runs its pipelines (`docs/contracts/rooms.md` § Events and
//! ticks, § The fast lane): one thing at a time per room, and the pipelines
//! resolved again when the active set changes.

use futures::SinkExt;
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::Message;

use super::published_mcp::{app_at, dsl, login, publish, temp_dir};
use super::ws_rooms::{Socket, connect, next_of, serve};

async fn say(socket: &mut Socket, event: &str, payload: Value) {
    socket.send(Message::Text(json!({ "event": event, "payload": payload }).to_string().into())).await.expect("send");
}

/// The next `event` frame's name.
async fn next_event(socket: &mut Socket) -> String {
    next_of(socket, "event").await["event"].as_str().unwrap_or_default().to_string()
}

/// Send ten bids as `who` and answer every list the room sent back.
async fn bid_ten(mut socket: Socket, who: &str) -> Vec<Value> {
    for i in 0..10 {
        say(&mut socket, "bid", json!({ "n": format!("{who}{i}") })).await;
    }
    let mut lists = Vec::new();
    for _ in 0..10 {
        lists.push(next_of(&mut socket, "event").await["payload"].clone());
    }
    lists
}

/// Two sessions bid at once into one room. Each bid reads the list, waits,
/// and writes it back: run concurrently, one bid would overwrite the other.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_sessions_read_modify_write_never_interleave() {
    let root = temp_dir("ws-room-order");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    publish(
        &app,
        &cookie,
        "auction-bid",
        r#"| trigger.room --room auction --event bid | kv.entry.get --key auction:bids | javascript.script.run -- "const list = (input.entry && input.entry.value) || []; const until = Date.now() + 15; while (Date.now() < until) {} return list.concat([input.room.payload.n]);" | kv.entry.put --key auction:bids --value "{{ input.script }}" | ws.message.send --event bid.done --recipient session --body "{{ input.script }}""#,
    )
    .await;
    let address = serve(app, &cookie).await;
    let mut a = connect(address, "auction", &[]).await;
    let mut b = connect(address, "auction", &[]).await;
    next_of(&mut a, "joined").await;
    next_of(&mut b, "joined").await;

    let (from_a, from_b) = tokio::join!(bid_ten(a, "a"), bid_ten(b, "b"));
    let longest = from_a.iter().chain(&from_b).max_by_key(|list| list.as_array().map_or(0, Vec::len)).expect("lists");
    let bids: Vec<&str> = longest.as_array().expect("a list").iter().filter_map(Value::as_str).collect();
    assert_eq!(bids.len(), 20, "no bid lost to another session's write: {bids:?}");
    for who in ["a", "b"] {
        let own: Vec<&str> = bids.iter().copied().filter(|n| n.starts_with(who)).collect();
        let sent: Vec<String> = (0..10).map(|i| format!("{who}{i}")).collect();
        assert_eq!(own, sent, "each session's bids keep their order");
    }
}

#[tokio::test]
async fn activation_and_deactivation_change_what_an_open_room_runs() {
    let root = temp_dir("ws-room-resolve");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    publish(&app, &cookie, "ping-a", r#"| trigger.room --room lobby --event ping | ws.message.send --event a --recipient session --body 1"#).await;
    let address = serve(app.clone(), &cookie).await;
    let mut socket = connect(address, "lobby", &[]).await;
    next_of(&mut socket, "joined").await;
    say(&mut socket, "ping", json!({})).await;
    assert_eq!(next_event(&mut socket).await, "a");

    publish(&app, &cookie, "ping-b", r#"| trigger.room --room lobby --event ping | ws.message.send --event b --recipient session --body 1"#).await;
    say(&mut socket, "ping", json!({})).await;
    assert_eq!([next_event(&mut socket).await, next_event(&mut socket).await], ["a", "b"], "an activation reaches a room already running");

    let answer = dsl(&app, &cookie, "deactivate pipeline pipelines/tests/ping-a.zf.json").await;
    assert_eq!(answer["ok"], json!(true), "{answer}");
    say(&mut socket, "ping", json!({})).await;
    say(&mut socket, "ping", json!({})).await;
    assert_eq!([next_event(&mut socket).await, next_event(&mut socket).await], ["b", "b"], "a deactivated pipeline runs no more");
}
