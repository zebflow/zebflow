//! Keeping a dropped session's seat (`docs/contracts/rooms.md` § Leaving,
//! dropping and closing): `$away`, the seat still counting, a resume token
//! that gives the same session back once and only to its identity, and
//! `$disconnect { reason: "timeout" }` when nobody comes back.

use futures::SinkExt;
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::Message;

use super::published_mcp::{app_at, login, publish, temp_dir};
use super::ws_rooms::{Socket, connect, credential, hook, next_of, refused, serve};

/// The next event frame: its name and payload.
async fn event(socket: &mut Socket) -> (String, Value) {
    let frame = next_of(socket, "event").await;
    (frame["event"].as_str().unwrap_or_default().to_string(), frame["payload"].clone())
}

#[tokio::test]
async fn a_dropped_session_keeps_its_seat_and_resumes_once_as_itself_until_the_hold_ends() {
    let root = temp_dir("ws-room-hold");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    let secret = format!("{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple());
    credential(&app, &cookie, json!({ "credential_id": "members", "title": "Members", "kind": "jwt_signing_key", "notes": "", "secret": { "algorithm": "HS256", "secret": secret } })).await;
    let rule = r#"--room "duel-:id" --auth jwt --credential members"#;
    for (event, said, body) in [
        ("$away", "away", r#""{{ input.room.session_id }}""#),
        ("$back", "back", r#""{{ input.room.session_id }}""#),
        ("$disconnect", "gone", r#""{{ { session: input.room.session_id, reason: input.room.payload.reason } }}""#),
    ] {
        publish(&app, &cookie, &format!("duel-{said}"), &format!("| trigger.room {rule} --event {event} | ws.message.send --event {said} --body {body}")).await;
    }
    publish(&app, &cookie, "duel-set", &format!(r#"| trigger.room {rule} --event set | ws.state.put --key /n --value "{{{{ input.room.payload.n }}}}""#)).await;
    publish(&app, &cookie, "open", r#"| trigger.webhook --route /open --method POST | ws.room.open --room "duel-{{ input.webhook.body.id }}" --max-sessions 2 --hold 3s | web.response.send --body "{{ input.room }}""#).await;
    let address = serve(app.clone(), &cookie).await;
    hook(&app, "/open", json!({ "id": "1" })).await;
    let now = chrono::Utc::now().timestamp();
    let bearer = |sub: &str| {
        let claims = json!({ "sub": sub, "iat": now, "exp": now + 600 });
        let token = jsonwebtoken::encode(&jsonwebtoken::Header::default(), &claims, &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes())).expect("token");
        format!("Bearer {token}")
    };
    let as_17 = bearer("u-17");
    let as_18 = bearer("u-18");

    let mut a = connect(address, "duel-1", &[("authorization", &as_17)]).await;
    let joined = next_of(&mut a, "joined").await;
    let a_id = joined["session_id"].as_str().expect("id").to_string();
    let first = joined["resume"].as_str().expect("a held room's joined carries a resume token").to_string();
    let mut b = connect(address, "duel-1", &[("authorization", &as_18)]).await;
    next_of(&mut b, "joined").await;

    // A drop, not a close: the seat is kept.
    drop(a);
    assert_eq!(event(&mut b).await, ("away".to_string(), json!(a_id)));
    assert_eq!(refused(address, "duel-1", &[("authorization", &bearer("u-19"))]).await, 4429, "the held seat still counts");
    b.send(Message::Text(json!({ "event": "set", "payload": { "n": 5 } }).to_string().into())).await.unwrap();
    next_of(&mut b, "patch").await;

    let with = |token: &str| format!("duel-1?resume={token}");
    assert_eq!(refused(address, &with(&first), &[("authorization", &as_18)]).await, 4401, "another identity");
    let mut a = connect(address, &with(&first), &[("authorization", &as_17)]).await;
    let back = next_of(&mut a, "joined").await;
    assert_eq!(back["session_id"], json!(a_id), "the same session");
    assert_eq!(back["state"], json!({ "n": 5 }), "its whole view, with what changed while it was away");
    let second = back["resume"].as_str().expect("a new token").to_string();
    assert_ne!(second, first);
    assert_eq!(event(&mut b).await, ("back".to_string(), json!(a_id)));

    drop(a);
    assert_eq!(event(&mut b).await.0, "away");
    assert_eq!(refused(address, &with(&first), &[("authorization", &as_17)]).await, 4401, "a token works once");
    let mut a = connect(address, &with(&second), &[("authorization", &as_17)]).await;
    let third = next_of(&mut a, "joined").await["resume"].as_str().expect("token").to_string();
    assert_eq!(event(&mut b).await, ("back".to_string(), json!(a_id)));

    drop(a);
    assert_eq!(event(&mut b).await.0, "away");
    let (name, gone) = event(&mut b).await;
    assert_eq!((name.as_str(), gone), ("gone", json!({ "session": a_id, "reason": "timeout" })), "the hold ended unused");
    assert_eq!(refused(address, &with(&third), &[("authorization", &as_17)]).await, 4401, "only during its hold");
    let mut c = connect(address, "duel-1", &[("authorization", &bearer("u-19"))]).await;
    next_of(&mut c, "joined").await;
}

#[tokio::test]
async fn a_client_close_gives_the_seat_up_and_a_held_session_can_be_closed() {
    let root = temp_dir("ws-room-hold-close");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    publish(&app, &cookie, "duel-gone", r#"| trigger.room --room "duel-:id" --event $disconnect | ws.message.send --event gone --body "{{ input.room.payload.reason }}""#).await;
    publish(&app, &cookie, "duel-away", r#"| trigger.room --room "duel-:id" --event $away | ws.message.send --event away --body 1"#).await;
    publish(&app, &cookie, "open", r#"| trigger.webhook --route /open --method POST | ws.room.open --room "duel-{{ input.webhook.body.id }}" --hold 30s | web.response.send --body "{{ input.room }}""#).await;
    publish(&app, &cookie, "kick", r#"| trigger.webhook --route /kick --method POST | ws.session.close --room duel-1 --session "{{ input.webhook.body.session }}" | web.response.send --body "{{ input.session }}""#).await;
    let address = serve(app.clone(), &cookie).await;
    hook(&app, "/open", json!({ "id": "1" })).await;
    let mut watcher = connect(address, "duel-1", &[]).await;
    next_of(&mut watcher, "joined").await;

    let mut leaver = connect(address, "duel-1", &[]).await;
    let resume = next_of(&mut leaver, "joined").await["resume"].as_str().expect("token").to_string();
    leaver.close(None).await.expect("close");
    assert_eq!(event(&mut watcher).await, ("gone".to_string(), json!("left")), "a close frame is leaving, not a drop");
    assert_eq!(refused(address, &format!("duel-1?resume={resume}"), &[]).await, 4401, "nothing is held");

    let mut dropper = connect(address, "duel-1", &[]).await;
    let session = next_of(&mut dropper, "joined").await["session_id"].clone();
    drop(dropper);
    assert_eq!(event(&mut watcher).await.0, "away");
    assert_eq!(hook(&app, "/kick", json!({ "session": session })).await["closed"], 1, "a held session can be closed");
    assert_eq!(event(&mut watcher).await, ("gone".to_string(), json!("closed")));
}
