//! WebSocket rooms: one socket's life in a room (`docs/contracts/rooms.md`).
//!
//! URL: `GET /ws/{owner}/{project}/rooms/{room}[?ticket=…]`
//!
//! 1. **Admission** — once, before anything of the room is sent
//!    ([`admission`]): `4404` for a room that is not open, `4401` for a
//!    refused join rule or ticket, `4429` for a full room.
//! 2. **Join** — through [`crate::infra::transport::ws::WsHub::join`]; the
//!    session receives `joined` with the current state.
//! 3. **Events** — inbound `{event, payload}` messages run the matching
//!    pipelines **one at a time, in arrival order** ([`events`]).
//! 4. **Lifecycle** — the server raises `$connect` after `joined` and
//!    `$disconnect { reason }` when the socket ends, on the same ordered
//!    queue. Clients cannot send `$`-prefixed events, and a trigger with an
//!    empty `--event` does not receive them.
//! 5. **Delivery** — each broadcast carries its target and is forwarded only
//!    to the sessions it is for; a session that fell behind receives `resync`.
//! 6. **Closing** — the server closes a socket with `4410` (`ws.room.close`),
//!    `4409` (`ws.session.close`), `4408` (the login's `exp` passed) or
//!    `1008` (flooding); the `$disconnect` reason says which.

mod admission;
mod events;

use super::*;
use crate::infra::transport::ws::{Counter, Kick};
use events::{enqueue_event, run_session_events, session_event_channel};

/// Raised after `joined`.
pub(super) const CONNECT_EVENT: &str = "$connect";
/// Raised when the socket ends; payload `{ reason }`.
pub(super) const DISCONNECT_EVENT: &str = "$disconnect";

/// Close code and reason for a login whose `exp` passed.
const KICK_EXPIRED: Kick = Kick { code: 4408, reason: "expired" };
/// Close code for a client that broke the room's rules (RFC 6455 §7.4.1).
const KICK_FLOOD: Kick = Kick { code: 1008, reason: "flood" };

/// What one session's event worker needs.
struct SessionCtx {
    owner: String,
    project: String,
    room_id: String,
    session_id: String,
    headers: HeaderMap,
    state: PlatformAppState,
}

pub(super) async fn ws_room_handler(
    ws: WebSocketUpgrade,
    Path((owner, project, room_id)): Path<(String, String, String)>,
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    match remote_project_worker_id(&state, &owner, &project) {
        Ok(Some(worker_id)) => {
            let worker = match state.platform.cluster_registry.get_worker(&worker_id) {
                Ok(Some(worker)) => worker,
                Ok(None) => {
                    return (
                        StatusCode::BAD_GATEWAY,
                        Json(json!({"ok": false, "error": "office not registered"})),
                    )
                        .into_response();
                }
                Err(err) => return internal_error(err),
            };
            let worker_url = worker_websocket_url(&worker.base_url, &uri);
            let proxy_headers = forwarded_websocket_headers(&headers);
            return ws
                .on_upgrade(move |socket| proxy_websocket_to_worker(socket, worker_url, proxy_headers))
                .into_response();
        }
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }

    let ticket = uri
        .query()
        .and_then(|query| serde_urlencoded::from_str::<Vec<(String, String)>>(query).ok())
        .and_then(|pairs| pairs.into_iter().find(|(k, _)| k == "ticket").map(|(_, v)| v));
    let owner = crate::platform::model::slug_segment(&owner);
    let project = crate::platform::model::slug_segment(&project);
    // Random, not a clock reading: two upgrades in the same nanosecond shared a slot.
    let session_id = format!("ws-{}", uuid::Uuid::new_v4().simple());
    ws.on_upgrade(move |socket| {
        handle_ws_room(SessionCtx { owner, project, room_id, session_id, headers, state }, ticket, socket)
    })
    .into_response()
}

async fn close_with(socket: &mut WebSocket, code: u16, reason: &str) {
    let frame = axum::extract::ws::CloseFrame { code, reason: reason.to_string().into() };
    let _ = socket.send(Message::Close(Some(frame))).await;
}

/// Seconds from now until `exp`, never negative.
fn until(exp: i64) -> std::time::Duration {
    let left = exp - chrono::Utc::now().timestamp();
    std::time::Duration::from_secs(left.max(0) as u64)
}

async fn handle_ws_room(ctx: SessionCtx, ticket: Option<String>, mut socket: WebSocket) {
    let admitted = match admission::admit(&ctx, ticket.as_deref()) {
        Ok(admitted) => admitted,
        Err((code, reason)) => return close_with(&mut socket, code, reason).await,
    };
    let hub = ctx.state.platform.ws_hub.clone();
    let joined = hub.join(&ctx.owner, &ctx.project, &ctx.room_id, &ctx.session_id, admitted.user.clone(), admitted.pass.as_ref());
    let (room, guard, mut kicks) = match joined {
        Ok(joined) => joined,
        Err(refusal) => {
            let (code, reason) = admission::join_refusal(&refusal);
            return close_with(&mut socket, code, reason).await;
        }
    };
    let session_id = ctx.session_id.clone();
    let user = admitted.user.clone();
    // Subscribe before reading state, so no patch falls between the two.
    let mut broadcast_rx = room.subscribe();
    let hello = json!({ "type": "joined", "session_id": session_id, "room": ctx.room_id, "state": room.get_state() });

    let (events_tx, events_rx) = session_event_channel();
    tokio::spawn(run_session_events(ctx, room.clone(), admitted.auth, events_rx));

    // A login that never expires waits on a timer that never fires.
    let expiry = tokio::time::sleep(admitted.expires_at.map(until).unwrap_or(std::time::Duration::from_secs(u32::MAX as u64)));
    tokio::pin!(expiry);
    let mut reason = "left";
    if socket.send(Message::Text(hello.to_string().into())).await.is_err() {
        reason = "dropped";
    } else {
        room.count(Counter::MessagesOut);
        let _ = enqueue_event(&events_tx, CONNECT_EVENT.to_string(), json!({}));
        loop {
            tokio::select! {
                broadcast = broadcast_rx.recv() => match broadcast {
                    Ok(msg) => {
                        if !msg.is_for(&session_id, user.as_deref()) {
                            continue;
                        }
                        if socket.send(Message::Text(msg.text.to_string().into())).await.is_err() {
                            reason = "dropped";
                            break;
                        }
                        room.count(Counter::MessagesOut);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        // Fell behind: messages were dropped, so send the whole state again.
                        room.count(Counter::Resyncs);
                        let resync = json!({ "type": "resync", "state": room.get_state() });
                        if socket.send(Message::Text(resync.to_string().into())).await.is_err() {
                            reason = "dropped";
                            break;
                        }
                        room.count(Counter::MessagesOut);
                    }
                },
                Some(kick) = kicks.recv() => {
                    close_with(&mut socket, kick.code, kick.reason).await;
                    reason = kick.reason;
                    break;
                }
                _ = &mut expiry => {
                    close_with(&mut socket, KICK_EXPIRED.code, KICK_EXPIRED.reason).await;
                    reason = KICK_EXPIRED.reason;
                    break;
                }
                incoming = socket.recv() => match incoming {
                    Some(Ok(Message::Text(text))) => {
                        let Ok(val) = serde_json::from_str::<Value>(text.as_str()) else { continue };
                        let event = val.get("event").and_then(Value::as_str).unwrap_or("message").to_string();
                        if event.starts_with('$') {
                            continue; // reserved for the server
                        }
                        room.count(Counter::MessagesIn);
                        room.touch();
                        let payload = val.get("payload").cloned().unwrap_or_else(|| json!({}));
                        if !enqueue_event(&events_tx, event, payload) {
                            // A client sending faster than its pipelines run is
                            // closed, not given an unbounded queue of runs.
                            room.count(Counter::FloodCloses);
                            close_with(&mut socket, KICK_FLOOD.code, "too many pending events").await;
                            reason = KICK_FLOOD.reason;
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(_)) => {
                        reason = "dropped";
                        break;
                    }
                    _ => {}
                },
            }
        }
    }

    // Queued behind every event this socket sent; the worker drains, then ends.
    // Awaited, so a full queue delays `$disconnect` rather than losing it.
    let _ = events_tx.send((DISCONNECT_EVENT.to_string(), json!({ "reason": reason }))).await;
    drop(events_tx);
    drop(guard);
}
