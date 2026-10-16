//! Views through real sockets (`docs/contracts/rooms.md` § Views): a path
//! written `--visible-to` never reaches the raw frames of a socket without
//! the tag, sessions with the same tags receive the same bytes, and
//! `--recipient tag:` reaches only tagged sessions.

use futures::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::Message;

use super::published_mcp::{app_at, login, publish, temp_dir};
use super::ws_rooms::{Socket, connect, next_of, serve};

async fn say(socket: &mut Socket, event: &str, payload: Value) {
    socket.send(Message::Text(json!({ "event": event, "payload": payload }).to_string().into())).await.expect("send");
}

/// Every text frame the socket receives until `marker` appears in one, as sent.
async fn raw_until(socket: &mut Socket, marker: &str) -> Vec<String> {
    let mut frames = Vec::new();
    loop {
        let next = tokio::time::timeout(std::time::Duration::from_secs(10), socket.next()).await.expect("a frame within 10 s");
        if let Some(Ok(Message::Text(text))) = next {
            let text = text.to_string();
            let done = text.contains(marker);
            frames.push(text);
            if done {
                return frames;
            }
        }
    }
}

#[tokio::test]
async fn a_hidden_path_never_reaches_an_untagged_socket_and_one_view_is_one_set_of_bytes() {
    let root = temp_dir("ws-room-views");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    publish(&app, &cookie, "table-sit", r#"| trigger.room --room table --event sit | ws.session.tag --tag "seat:{{ input.room.payload.seat }}" | ws.message.send --event seated --recipient session --body "{{ input.session }}""#).await;
    publish(&app, &cookie, "table-deal", r#"| trigger.room --room table --event deal | ws.state.put --key "/hands/{{ input.room.payload.seat }}" --value "{{ input.room.payload.cards }}" --visible-to "seat:{{ input.room.payload.seat }}""#).await;
    publish(&app, &cookie, "table-turn", r#"| trigger.room --room table --event turn | ws.state.put --key /turn --value "{{ input.room.payload.n }}""#).await;
    publish(&app, &cookie, "table-whisper", r#"| trigger.room --room table --event whisper | ws.message.send --recipient "tag:seat:{{ input.room.payload.seat }}" --event whispered --body "{{ input.room.payload.text }}""#).await;
    let address = serve(app, &cookie).await;

    let mut sockets = Vec::new();
    for seat in [Some(1), Some(1), Some(2), None] {
        let mut socket = connect(address, "table", &[]).await;
        next_of(&mut socket, "joined").await;
        if let Some(seat) = seat {
            say(&mut socket, "sit", json!({ "seat": seat })).await;
            next_of(&mut socket, "event").await;
        }
        sockets.push(socket);
    }
    let [mut one, mut one_too, mut two, mut nobody]: [Socket; 4] = sockets.try_into().ok().expect("four");

    say(&mut nobody, "deal", json!({ "seat": 1, "cards": ["hidden-ace", "hidden-king"] })).await;
    say(&mut nobody, "turn", json!({ "n": 7 })).await;
    let (a, b) = (raw_until(&mut one, "\"turn\"").await, raw_until(&mut one_too, "\"turn\"").await);
    assert!(a.iter().any(|f| f.contains("hidden-ace")), "seat 1 sees its hand: {a:?}");
    assert_eq!(a, b, "two sessions with the same tags receive identical bytes");
    for (who, socket) in [("seat 2", &mut two), ("no seat", &mut nobody)] {
        let frames = raw_until(socket, "\"turn\"").await;
        assert!(frames.iter().all(|f| !f.contains("hidden")), "{who} never receives the hand's bytes: {frames:?}");
    }

    say(&mut nobody, "whisper", json!({ "seat": 2, "text": "psst" })).await;
    say(&mut nobody, "turn", json!({ "n": 8 })).await;
    assert!(raw_until(&mut two, "psst").await.iter().any(|f| f.contains("whispered")));
    for socket in [&mut one, &mut nobody] {
        let frames = raw_until(socket, "\"value\":8").await;
        assert!(frames.iter().all(|f| !f.contains("psst")), "a tag: recipient reaches only the tagged: {frames:?}");
    }
}
