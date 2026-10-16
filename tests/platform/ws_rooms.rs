//! WebSocket rooms through a real server (`docs/contracts/rooms.md`): which
//! rooms exist, who may join, one join rule per room, and how the server
//! closes a socket.

use futures::{SinkExt, StreamExt};
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

use super::published_mcp::{OWNER, PROJECT, addressing, app_at, call, dsl, login, publish, rpc, send, temp_dir, text_of};

pub(super) type Socket = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;

/// Switch the `ws` surface on (it is off by default) and serve `app` on a
/// free port, as the binary does; the address.
pub(super) async fn serve(app: axum::Router, cookie: &str) -> std::net::SocketAddr {
    addressing(&app, cookie, json!({ "hosts": [], "routes": [], "disabled": ["files", "ms"] })).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let address = listener.local_addr().expect("address");
    tokio::spawn(async move {
        axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>()).await
    });
    address
}

/// The wire protocol every room socket names.
pub(super) const PROTOCOL: &str = "zebflow.rooms.v1";

/// Connect to a room with these headers, naming [`PROTOCOL`];
/// `path_and_query` follows `/rooms/`.
pub(super) async fn connect(address: std::net::SocketAddr, path_and_query: &str, headers: &[(&str, &str)]) -> Socket {
    let mut with_protocol = vec![("sec-websocket-protocol", PROTOCOL)];
    with_protocol.extend_from_slice(headers);
    connect_raw(address, path_and_query, &with_protocol).await.expect("upgrade")
}

/// Connect with exactly these headers: the socket, or the refused upgrade.
pub(super) async fn connect_raw(
    address: std::net::SocketAddr,
    path_and_query: &str,
    headers: &[(&str, &str)],
) -> Result<Socket, tokio_tungstenite::tungstenite::Error> {
    let url = format!("ws://{address}/ws/{OWNER}/{PROJECT}/rooms/{path_and_query}");
    let mut request = url.into_client_request().expect("request");
    for (name, value) in headers {
        request.headers_mut().insert(
            tokio_tungstenite::tungstenite::http::HeaderName::from_bytes(name.as_bytes()).unwrap(),
            value.parse().unwrap(),
        );
    }
    tokio_tungstenite::connect_async(request).await.map(|(socket, _)| socket)
}

/// The next JSON frame, or the close code when the server closes instead.
pub(super) async fn next(socket: &mut Socket) -> Result<Value, u16> {
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

/// The next frame of `type`, skipping others. An `event` is also the one
/// event a tick room's `patch` carries.
pub(super) async fn next_of(socket: &mut Socket, kind: &str) -> Value {
    loop {
        let frame = next(socket).await.unwrap_or_else(|code| panic!("closed {code} while waiting for {kind}"));
        if frame["type"] == kind {
            return frame;
        }
        if kind == "event" && frame["type"] == "patch" && frame["events"].as_array().is_some_and(|e| !e.is_empty()) {
            let events = frame["events"].as_array().expect("events");
            assert_eq!(events.len(), 1, "one event per tick here: {frame}");
            return json!({ "type": "event", "event": events[0]["event"], "payload": events[0]["payload"] });
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
async fn a_socket_naming_no_protocol_or_another_is_refused_before_the_upgrade() {
    let root = temp_dir("ws-rooms-protocol");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    publish(&app, &cookie, "lobby-chat", r#"| trigger.room --room lobby --event chat | ws.message.send --body 1"#).await;
    let address = serve(app, &cookie).await;
    for headers in [vec![], vec![("sec-websocket-protocol", "zebflow.rooms.v0")], vec![("sec-websocket-protocol", "chat, graphql-ws")]] {
        match connect_raw(address, "lobby", &headers).await {
            Err(tokio_tungstenite::tungstenite::Error::Http(response)) => {
                assert_eq!(response.status().as_u16(), 426, "{headers:?}");
                assert_eq!(response.headers()["sec-websocket-protocol"], PROTOCOL, "the supported list: {headers:?}");
                let body: Value = serde_json::from_slice(response.body().as_deref().unwrap_or_default()).expect("json body");
                assert_eq!(body["supported"], json!([PROTOCOL]));
            }
            other => panic!("{headers:?} upgraded: {:?}", other.map(|_| ())),
        }
    }
    let mut socket = connect(address, "lobby", &[]).await;
    assert_eq!(next(&mut socket).await.expect("joined")["type"], "joined");
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

#[tokio::test]
async fn a_change_sends_each_session_one_small_patch_not_the_state() {
    let root = temp_dir("ws-rooms-patch");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    publish(&app, &cookie, "lobby-set", r#"| trigger.room --room lobby --event set | ws.state.put --key "/players/{{ input.room.session_id }}" --value "{{ input.room.payload }}""#).await;
    let address = serve(app, &cookie).await;
    let mut a = connect(address, "lobby", &[]).await;
    let a_id = next_of(&mut a, "joined").await["session_id"].as_str().expect("id").to_string();
    a.send(Message::Text(json!({ "event": "set", "payload": { "x": 1, "y": 1 } }).to_string().into())).await.unwrap();
    next_of(&mut a, "patch").await;
    let mut b = connect(address, "lobby", &[]).await;
    let joined = next_of(&mut b, "joined").await;
    let b_id = joined["session_id"].as_str().expect("id").to_string();
    assert_eq!(joined["state"], json!({ "players": { a_id.clone(): { "x": 1, "y": 1 } } }), "joined carries the whole state");

    b.send(Message::Text(json!({ "event": "set", "payload": { "x": 9, "y": 9 } }).to_string().into())).await.unwrap();
    next_of(&mut b, "patch").await; // its own change
    let patch = next_of(&mut a, "patch").await;
    assert_eq!(patch, json!({ "type": "patch", "ops": [ { "op": "set", "path": ["players", b_id], "value": { "x": 9, "y": 9 } } ] }), "only b's entry, not a's");
    a.send(Message::Text(json!({ "event": "set", "payload": { "x": 2, "y": 1 } }).to_string().into())).await.unwrap();
    assert_eq!(next_of(&mut b, "patch").await["ops"], json!([ { "op": "set", "path": ["players", a_id, "x"], "value": 2 } ]), "one changed leaf, one set");
}

/// A webhook of the test project, called with a JSON body; its answer.
pub(super) async fn hook(app: &axum::Router, route: &str, body: Value) -> Value {
    use tower::ServiceExt;
    let request = axum::http::Request::builder()
        .uri(format!("/wh/{OWNER}/{PROJECT}{route}"))
        .method("POST")
        .header("content-type", "application/json")
        .body(axum::body::Body::from(body.to_string()))
        .expect("request");
    let response = app.clone().oneshot(request).await.expect("response");
    let bytes = axum::body::to_bytes(response.into_body(), usize::MAX).await.expect("body");
    serde_json::from_slice(&bytes).unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).to_string()))
}

async fn ticket(app: &axum::Router, room: &str, user: &str) -> String {
    let answer = hook(app, "/ticket", json!({ "room": room, "user": user })).await;
    answer["ticket"].as_str().unwrap_or_else(|| panic!("a ticket: {answer}")).to_string()
}

#[tokio::test]
async fn an_opened_private_room_admits_ticket_holders_up_to_its_cap_and_closes() {
    let root = temp_dir("ws-rooms-opened");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    publish(&app, &cookie, "match-chat", r#"| trigger.room --room "match-:id" --event chat | ws.message.send --event said --body "{{ input.room.auth }}""#).await;
    publish(&app, &cookie, "match-left", r#"| trigger.room --room "match-:id" --event $disconnect | ws.message.send --event gone --body "{{ input.room.payload }}""#).await;
    publish(&app, &cookie, "open", r#"| trigger.webhook --route /open --method POST | ws.room.open --room "match-{{ input.webhook.body.id }}" --private --max-sessions 2 | web.response.send --body "{{ input.room }}""#).await;
    publish(&app, &cookie, "ticket", r#"| trigger.webhook --route /ticket --method POST --errors show | ws.ticket.create --room "{{ input.webhook.body.room }}" --user "{{ input.webhook.body.user }}" | web.response.send --body "{{ input.ticket }}""#).await;
    publish(&app, &cookie, "kick", r#"| trigger.webhook --route /kick --method POST | ws.session.close --room "{{ input.webhook.body.room }}" --user "{{ input.webhook.body.user }}" | web.response.send --body "{{ input.session }}""#).await;
    let address = serve(app.clone(), &cookie).await;

    let opened = hook(&app, "/open", json!({ "id": "1" })).await;
    assert_eq!((opened["room"].as_str(), opened["private"].as_bool(), opened["max_sessions"].as_u64()), (Some("match-1"), Some(true), Some(2)), "{opened}");
    assert_eq!(refused(address, "match-1", &[]).await, 4401, "a private room admits ticket holders only");

    let first = ticket(&app, "match-1", "u-17").await;
    let mut a = connect(address, &format!("match-1?ticket={first}"), &[]).await;
    next_of(&mut a, "joined").await;
    a.send(Message::Text(json!({ "event": "chat", "payload": {} }).to_string().into())).await.unwrap();
    assert_eq!(next_of(&mut a, "event").await["payload"], json!({ "sub": "u-17" }), "under --auth none the ticket's user is the identity");
    assert_eq!(refused(address, &format!("match-1?ticket={first}"), &[]).await, 4401, "a ticket is used once");

    let second = ticket(&app, "match-1", "u-18").await;
    let third = ticket(&app, "match-1", "u-19").await;
    let mut b = connect(address, &format!("match-1?ticket={second}"), &[]).await;
    next_of(&mut b, "joined").await;
    assert_eq!(refused(address, &format!("match-1?ticket={third}"), &[]).await, 4429, "past --max-sessions");
    let full = hook(&app, "/ticket", json!({ "room": "match-1", "user": "u-19" })).await;
    assert!(full.to_string().contains("FW_NODE_WS_TICKET_CREATE_FULL"), "{full}");

    let kicked = hook(&app, "/kick", json!({ "room": "match-1", "user": "u-18" })).await;
    assert_eq!(kicked, json!({ "room": "match-1", "closed": 1 }));
    assert_eq!(next(&mut b).await.expect_err("closed"), 4409);
    let gone = next_of(&mut a, "event").await;
    assert_eq!((gone["event"].as_str(), gone["payload"]["reason"].as_str()), (Some("gone"), Some("closed")));

    // The Studio API and the project MCP see and close the same rooms.
    let (status, list) = send(&app, &cookie, "GET", &format!("/api/projects/{OWNER}/{PROJECT}/ws/rooms?room=match-*&members=1"), Value::Null).await;
    assert_eq!(status.as_u16(), 200, "{list}");
    assert_eq!(list["total"], json!({ "rooms": 1, "sessions": 1 }));
    let room = &list["rooms"][0];
    assert_eq!(room["members"], json!(["u-17"]));
    assert!(room["counters"]["messages_in"].as_u64() >= Some(1) && room["counters"]["pipeline_runs"].as_u64() >= Some(1), "{room}");
    let (_, session) = send(&app, &cookie, "POST", &format!("/api/projects/{OWNER}/{PROJECT}/mcp/session"), json!({ "capabilities": ["pipelines.read", "pipelines.execute"] })).await;
    let bearer = format!("Bearer {}", session["session"]["token"].as_str().expect("token"));
    let mcp = format!("/api/projects/{OWNER}/{PROJECT}/mcp");
    let (_, listed) = rpc(&app, &mcp, None, &[("authorization", &bearer)], call("ws_rooms_list", json!({ "room": "match-*" }))).await;
    let listed: Value = serde_json::from_str(&text_of(&listed)).unwrap_or_else(|_| panic!("{listed}"));
    assert_eq!(listed["total"], list["total"]);
    assert!(listed["rooms"][0].get("members").is_none(), "no identities without members");

    let (status, closed) = send(&app, &cookie, "POST", &format!("/api/projects/{OWNER}/{PROJECT}/ws/rooms/match-1/close"), Value::Null).await;
    assert_eq!(status.as_u16(), 200, "{closed}");
    assert_eq!(closed, json!({ "room": "match-1", "closed": ["match-1"], "sessions": 1 }));
    assert_eq!(next(&mut a).await.expect_err("closed"), 4410);
    let (status, _) = send(&app, &cookie, "POST", &format!("/api/projects/{OWNER}/{PROJECT}/ws/rooms/match-1/close"), Value::Null).await;
    assert_eq!(status.as_u16(), 404, "a room that is not open");
    let (_, again) = rpc(&app, &mcp, None, &[("authorization", &bearer)], call("ws_room_close", json!({ "room": "match-1" }))).await;
    assert!(again.to_string().contains("not open"), "{again}");
    assert_eq!(refused(address, "match-1", &[]).await, 4404, "closed: not open any more");
}

#[tokio::test]
async fn a_login_is_checked_once_and_closes_the_session_when_it_expires() {
    let root = temp_dir("ws-rooms-expiry");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    let secret = format!("{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple());
    credential(&app, &cookie, json!({ "credential_id": "members", "title": "Members", "kind": "jwt_signing_key", "notes": "", "secret": { "algorithm": "HS256", "secret": secret } })).await;
    publish(&app, &cookie, "members-chat", r#"| trigger.room --room members --event chat --auth jwt --credential members | ws.message.send --event said --body "{{ input.room.auth.sub }}""#).await;
    let address = serve(app, &cookie).await;
    let now = chrono::Utc::now().timestamp();
    let token = |exp: i64| {
        let claims = json!({ "sub": "u-17", "iat": now, "exp": exp });
        jsonwebtoken::encode(&jsonwebtoken::Header::default(), &claims, &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes())).expect("token")
    };

    assert_eq!(refused(address, "members", &[]).await, 4401);
    let bearer = format!("Bearer {}", token(now + 3));
    let mut socket = connect(address, "members", &[("authorization", &bearer)]).await;
    next_of(&mut socket, "joined").await;
    socket.send(Message::Text(json!({ "event": "chat", "payload": {} }).to_string().into())).await.unwrap();
    assert_eq!(next_of(&mut socket, "event").await["payload"], "u-17", "the claims proved at connect ride on the event");
    assert_eq!(next(&mut socket).await.expect_err("the login expired"), 4408);
}
