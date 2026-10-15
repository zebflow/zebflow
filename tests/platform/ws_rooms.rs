//! WebSocket rooms through a real server (`docs/contracts/rooms.md`): which
//! rooms exist, who may join, one join rule per room, and how the server
//! closes a socket.

use futures::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

use super::published_mcp::{OWNER, PROJECT, addressing, app_at, dsl, login, publish, send, temp_dir};

type Socket = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// Switch the `ws` surface on (it is off by default) and serve `app` on a
/// free port, as the binary does; the address.
async fn serve(app: axum::Router, cookie: &str) -> std::net::SocketAddr {
    addressing(&app, cookie, json!({ "hosts": [], "routes": [], "disabled": ["files", "ms", "mcp"] })).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let address = listener.local_addr().expect("address");
    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>()).await
    });
    address
}

/// Connect to a room with these headers; `path_and_query` follows `/rooms/`.
async fn connect(address: std::net::SocketAddr, path_and_query: &str, headers: &[(&str, &str)]) -> Socket {
    let url = format!("ws://{address}/ws/{OWNER}/{PROJECT}/rooms/{path_and_query}");
    let mut request = url.into_client_request().expect("request");
    for (name, value) in headers {
        request.headers_mut().insert(
            tokio_tungstenite::tungstenite::http::HeaderName::from_bytes(name.as_bytes()).unwrap(),
            value.parse().unwrap(),
        );
    }
    tokio_tungstenite::connect_async(request).await.expect("upgrade").0
}

/// The next JSON frame, or the close code when the server closes instead.
async fn next(socket: &mut Socket) -> Result<Value, u16> {
    let wait = std::time::Duration::from_secs(10);
    loop {
        match tokio::time::timeout(wait, socket.next()).await.expect("a frame within 10 s") {
            Some(Ok(Message::Text(text))) => return Ok(serde_json::from_str(text.as_str()).expect("json")),
            Some(Ok(Message::Close(frame))) => return Err(frame.map(|f| u16::from(f.code)).unwrap_or(1005)),
            Some(Ok(_)) => continue,
            Some(Err(_)) | None => return Err(1006),
        }
    }
}

/// The next frame of `type`, skipping others.
async fn next_of(socket: &mut Socket, kind: &str) -> Value {
    loop {
        let frame = next(socket).await.unwrap_or_else(|code| panic!("closed {code} while waiting for {kind}"));
        if frame["type"] == kind {
            return frame;
        }
    }
}

/// A connection the server closes before anything of the room is sent.
async fn refused(address: std::net::SocketAddr, path: &str, headers: &[(&str, &str)]) -> u16 {
    let mut socket = connect(address, path, headers).await;
    next(&mut socket).await.expect_err("refused before joined")
}

async fn credential(app: &axum::Router, cookie: &str, body: Value) {
    let (status, answer) = send(app, cookie, "POST", &format!("/api/projects/{OWNER}/{PROJECT}/credentials"), body).await;
    assert_eq!(status.as_u16(), 200, "{answer}");
}

#[tokio::test]
async fn activation_refuses_two_join_rules_for_one_room() {
    let root = temp_dir("ws-rooms-rules");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    let key = uuid::Uuid::new_v4().simple().to_string();
    credential(&app, &cookie, json!({ "credential_id": "room-key", "title": "Rooms", "kind": "api_key", "notes": "", "secret": { "key": key } })).await;
    publish(&app, &cookie, "open-lobbies", r#"| trigger.room --room "lob*" --event chat | ws.message.send --body "{{ input.room.payload }}""#).await;

    // `lobby` is a room `lob*` matches: two rules for one room.
    let registered = dsl(&app, &cookie, r#"register pipelines/tests/guarded-lobby -- | trigger.room --room lobby --auth api_key --credential room-key | ws.message.send --body 1"#).await;
    assert_eq!(registered["ok"], json!(true), "{registered}");
    let refused = dsl(&app, &cookie, "activate pipeline pipelines/tests/guarded-lobby.zf.json").await;
    assert_ne!(refused["ok"], json!(true), "{refused}");
    let text = refused.to_string();
    assert!(text.contains("one join rule"), "{refused}");
    assert!(text.contains("pipelines/tests/open-lobbies.zf.json") && text.contains("pipelines/tests/guarded-lobby.zf.json"), "names both: {refused}");

    // Overlapping patterns count too; a room no other trigger can match is free.
    let registered = dsl(&app, &cookie, r#"register pipelines/tests/guarded-l -- | trigger.room --room "l:id" --auth api_key --credential room-key | ws.message.send --body 1"#).await;
    assert_eq!(registered["ok"], json!(true), "{registered}");
    assert_ne!(dsl(&app, &cookie, "activate pipeline pipelines/tests/guarded-l.zf.json").await["ok"], json!(true));
    publish(&app, &cookie, "guarded-vault", r#"| trigger.room --room vault --auth api_key --credential room-key | ws.message.send --body 1"#).await;
    publish(&app, &cookie, "same-rule-lobby", r#"| trigger.room --room lobby --event $connect | ws.message.send --body 1"#).await;
}

#[tokio::test]
async fn a_socket_joins_only_an_open_room_its_rule_admits() {
    let root = temp_dir("ws-rooms-join");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    let key = uuid::Uuid::new_v4().simple().to_string();
    credential(&app, &cookie, json!({ "credential_id": "room-key", "title": "Rooms", "kind": "api_key", "notes": "", "secret": { "key": key } })).await;
    publish(&app, &cookie, "lobby-chat", r#"| trigger.room --room lobby --event chat | ws.message.send --event said --body "{{ { text: input.room.payload.text, auth: input.room.auth } }}""#).await;
    publish(&app, &cookie, "lobby-left", r#"| trigger.room --room lobby --event $disconnect | ws.message.send --event gone --body "{{ input.room.payload }}""#).await;
    publish(&app, &cookie, "matches", r#"| trigger.room --room "match-:id" --event move | ws.message.send --body 1"#).await;
    publish(&app, &cookie, "vault", r#"| trigger.room --room vault --auth api_key --credential room-key --event chat | ws.message.send --body 1"#).await;
    let address = serve(app, &cookie).await;

    assert_eq!(refused(address, "nowhere", &[]).await, 4404, "no trigger declares it");
    assert_eq!(refused(address, "match-1", &[]).await, 4404, "a pattern nobody opened");
    assert_eq!(refused(address, "vault", &[]).await, 4401, "the join rule refuses");
    assert_eq!(refused(address, "vault", &[("x-api-key", "wrong")]).await, 4401);
    let mut vault = connect(address, "vault", &[("x-api-key", &key)]).await;
    assert_eq!(next(&mut vault).await.expect("joined")["type"], "joined");

    let mut a = connect(address, "lobby", &[]).await;
    let mut b = connect(address, "lobby", &[]).await;
    assert_eq!(next_of(&mut a, "joined").await["room"], "lobby");
    next_of(&mut b, "joined").await;
    a.send(Message::Text(json!({ "event": "chat", "payload": { "text": "hi" } }).to_string().into())).await.unwrap();
    let said = next_of(&mut b, "event").await;
    assert_eq!(said["payload"], json!({ "text": "hi", "auth": null }), "auth rides on every event, null when nothing was proved");
    a.close(None).await.unwrap();
    let gone = next_of(&mut b, "event").await;
    assert_eq!(gone["event"], "gone");
    assert_eq!(gone["payload"], json!({ "reason": "left" }));
}
