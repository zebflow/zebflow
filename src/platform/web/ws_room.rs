//! WebSocket rooms: one socket's life in a room.
//!
//! URL: `GET /ws/{owner}/{project}/rooms/{room_id}`
//!
//! 1. **Admission** — before anything of the room is sent, the connection must be
//!    allowed by the room's `trigger.room` pipelines (see [`admission`]).
//! 2. **Join** — through [`crate::infra::transport::ws::WsHub::join_room`]; the
//!    session receives `joined` with the current state.
//! 3. **Events** — inbound `{event, payload}` messages run the matching pipelines
//!    **one at a time, in arrival order**, so a `leave` can never be overtaken by
//!    the `join` before it.
//! 4. **Lifecycle** — the server raises the reserved events `$connect` after
//!    `joined` and `$disconnect` (payload `{ reason }`) when the socket ends, on the
//!    same ordered queue. Clients cannot send `$`-prefixed events, and a trigger
//!    with an empty `--event` does not receive them: only `--event $connect` /
//!    `--event $disconnect` do.
//! 5. **Delivery** — each broadcast carries its target and is forwarded only to
//!    the sessions it is for; a session that fell behind receives `resync`.
//! 6. **Leave** — through [`crate::infra::transport::ws::WsHub::leave_room`],
//!    which disposes the room with its last session.

use std::collections::HashMap;

use super::*;

/// Raised after `joined`.
pub(super) const CONNECT_EVENT: &str = "$connect";
/// Raised when the socket ends; payload `{ reason: "close" | "error" }`.
pub(super) const DISCONNECT_EVENT: &str = "$disconnect";

/// Close code sent when a connection is not admitted to a room.
const CLOSE_UNAUTHORIZED: u16 = 4401;

/// Whether a `trigger.room --room` value (exact name, `prefix*` glob, or
/// `prefix:param` pattern) matches `room_id`. An empty `t_room` never matches:
/// a room exists only when declared by `trigger.room --room <name-or-pattern>`.
pub(super) fn room_matches(t_room: &str, room_id: &str) -> bool {
    let t_room = t_room.trim();
    let room_id = room_id.trim();
    if t_room.is_empty() || room_id.is_empty() {
        return false;
    }
    if t_room == room_id || t_room == "*" {
        return true;
    }
    if let Some((prefix, suffix)) = t_room.split_once('*')
        && !suffix.contains('*')
    {
        return room_id.len() > prefix.len() + suffix.len()
            && room_id.starts_with(prefix)
            && room_id.ends_with(suffix);
    }
    if let Some((prefix, param)) = t_room.split_once(':')
        && !param.is_empty()
        && param.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
    {
        return room_id
            .strip_prefix(prefix)
            .is_some_and(|rest| !rest.is_empty() && !rest.contains('/'));
    }
    false
}

/// Whether a `trigger.room` with `t_room` / `t_event` handles `event` in `room_id`.
/// Reserved `$` events reach only triggers that name them.
pub(super) fn trigger_matches(t_room: &str, t_event: &str, room_id: &str, event: &str) -> bool {
    let room_match = room_matches(t_room, room_id);
    let event_match = t_event == event || (t_event.is_empty() && !event.starts_with('$'));
    room_match && event_match
}

/// Whether a connection may enter a room, from the auth its triggers require.
///
/// `requires_auth` holds one entry per `trigger.room` that matches the room.
/// A room with no matching `trigger.room` does not exist and is refused; if
/// every trigger on the room is open (`--auth none`), everyone is admitted; if
/// any trigger on the room requires auth, every required guard on the room must
/// pass (an unguarded event trigger in the same room never bypasses room auth).
pub(super) fn admission(requires_auth: &[bool], passes_all: impl FnOnce() -> bool) -> bool {
    if requires_auth.is_empty() {
        return false;
    }
    if requires_auth.iter().all(|required| !required) {
        return true;
    }
    passes_all()
}

fn is_open(auth_type: &str) -> bool {
    auth_type.is_empty() || auth_type == "none"
}

/// Claims this connection already proved, per auth rule. `$disconnect` falls back
/// to them: a token that expired while the socket was open must not leave a ghost.
#[derive(Default)]
struct AuthCache(HashMap<String, Value>);

impl AuthCache {
    fn key(spec: &crate::platform::services::pipeline_runtime::WsTriggerSpec) -> String {
        format!("{}|{}|{}", spec.auth_type, spec.auth_credential, spec.auth_required_role.join(","))
    }
}

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

    // Random, not a clock reading: two upgrades in the same nanosecond shared a slot.
    let session_id = format!("ws-{}", uuid::Uuid::new_v4().simple());
    ws.on_upgrade(move |socket| {
        handle_ws_room(SessionCtx { owner, project, room_id, session_id, headers, state }, socket)
    })
    .into_response()
}

async fn handle_ws_room(ctx: SessionCtx, mut socket: WebSocket) {
    let mut auth = AuthCache::default();
    if !admit(&ctx, &mut auth) {
        let _ = socket
            .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                code: CLOSE_UNAUTHORIZED,
                reason: "unauthorized".into(),
            })))
            .await;
        return;
    }

    let hub = ctx.state.platform.ws_hub.clone();
    let room_key = format!("{}/{}/{}", ctx.owner, ctx.project, ctx.room_id);
    let session_id = ctx.session_id.clone();
    let room_id = ctx.room_id.clone();
    let (room, guard) = hub.join_room(&room_key);
    // Subscribe before reading state, so no patch falls between the two.
    let mut broadcast_rx = room.subscribe();
    let joined = json!({ "type": "joined", "session_id": session_id, "room": room_id, "state": room.get_state() });

    let (events_tx, events_rx) = session_event_channel();
    tokio::spawn(run_session_events(ctx, auth, events_rx));

    let mut reason = "close";
    if socket.send(Message::Text(joined.to_string().into())).await.is_err() {
        reason = "error";
    } else {
        let _ = enqueue_event(&events_tx, CONNECT_EVENT.to_string(), json!({}));
        loop {
            tokio::select! {
                broadcast = broadcast_rx.recv() => match broadcast {
                    Ok(msg) => {
                        if !msg.is_for(&session_id) {
                            continue;
                        }
                        if socket.send(Message::Text(msg.text.to_string().into())).await.is_err() {
                            reason = "error";
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        // Fell behind: messages were dropped, so send the whole state again.
                        let resync = json!({ "type": "resync", "state": room.get_state() });
                        if socket.send(Message::Text(resync.to_string().into())).await.is_err() {
                            reason = "error";
                            break;
                        }
                    }
                },
                incoming = socket.recv() => match incoming {
                    Some(Ok(Message::Text(text))) => {
                        let Ok(val) = serde_json::from_str::<Value>(text.as_str()) else { continue };
                        let event = val.get("event").and_then(Value::as_str).unwrap_or("message").to_string();
                        if event.starts_with('$') {
                            continue; // reserved for the server
                        }
                        let payload = val.get("payload").cloned().unwrap_or_else(|| json!({}));
                        if !enqueue_event(&events_tx, event, payload) {
                            // A client sending faster than its pipelines run is
                            // closed, not given an unbounded queue of runs.
                            let _ = socket
                                .send(Message::Close(Some(axum::extract::ws::CloseFrame {
                                    code: CLOSE_POLICY_VIOLATION,
                                    reason: "too many pending events".into(),
                                })))
                                .await;
                            reason = "error";
                            break;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(_)) => {
                        reason = "error";
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
    hub.leave_room(&room_key, guard);
}

/// Most events one session may have waiting for its pipelines.
pub(super) const MAX_PENDING_SESSION_EVENTS: usize = 256;

/// Close code for a client that broke the room's rules (RFC 6455 §7.4.1).
const CLOSE_POLICY_VIOLATION: u16 = 1008;

type SessionEvent = (String, Value);

fn session_event_channel() -> (
    tokio::sync::mpsc::Sender<SessionEvent>,
    tokio::sync::mpsc::Receiver<SessionEvent>,
) {
    tokio::sync::mpsc::channel(MAX_PENDING_SESSION_EVENTS)
}

/// Queues one event without waiting; `false` when the queue is full or closed.
fn enqueue_event(tx: &tokio::sync::mpsc::Sender<SessionEvent>, event: String, payload: Value) -> bool {
    tx.try_send((event, payload)).is_ok()
}

/// Decide admission for one connection, remembering the auth it passed.
fn admit(ctx: &SessionCtx, auth: &mut AuthCache) -> bool {
    let pipelines = ctx.state.platform.pipeline_runtime.list_project(&ctx.owner, &ctx.project);
    let specs: Vec<_> = pipelines
        .iter()
        .flat_map(|p| p.ws_triggers.iter())
        .filter(|t| room_matches(&t.room, &ctx.room_id))
        .collect();
    let requires: Vec<bool> = specs.iter().map(|t| !is_open(&t.auth_type)).collect();
    // The room's own `--auth`, from the visitor's headers, on every office: a
    // connection a controller forwards carries the visitor's headers, and the
    // controller's call header proves only who forwarded it.
    admission(&requires, || {
        for spec in specs.iter().filter(|t| !is_open(&t.auth_type)) {
            match verify_trigger_auth(ctx, spec) {
                Ok(claims) => {
                    auth.0.insert(AuthCache::key(spec), claims.unwrap_or(Value::Null));
                }
                Err(()) => return false,
            }
        }
        true
    })
}

fn verify_trigger_auth(
    ctx: &SessionCtx,
    spec: &crate::platform::services::pipeline_runtime::WsTriggerSpec,
) -> Result<Option<Value>, ()> {
    if is_open(&spec.auth_type) {
        return Ok(None);
    }
    verify_webhook_auth(
        &ctx.headers,
        &axum::body::Bytes::new(), // WS messages have no body to sign
        &spec.auth_type,
        &spec.auth_credential,
        &spec.auth_required_role,
        &ctx.state.platform.credentials,
        &ctx.owner,
        &ctx.project,
    )
    .map_err(|_| ())
}

/// Run one session's events strictly in order.
async fn run_session_events(
    ctx: SessionCtx,
    mut auth: AuthCache,
    mut events: tokio::sync::mpsc::Receiver<SessionEvent>,
) {
    while let Some((event, payload)) = events.recv().await {
        run_event(&ctx, &mut auth, &event, payload).await;
    }
}

/// Run every pipeline whose `trigger.room` matches `event`, one after another.
async fn run_event(ctx: &SessionCtx, auth: &mut AuthCache, event: &str, payload: Value) {
    let pipelines = ctx.state.platform.pipeline_runtime.list_project(&ctx.owner, &ctx.project);
    for compiled in pipelines {
        let Some(spec) = compiled
            .ws_triggers
            .iter()
            .find(|t| trigger_matches(&t.room, &t.event, &ctx.room_id, event))
            .cloned()
        else {
            continue;
        };
        let claims = match verify_trigger_auth(ctx, &spec) {
            Ok(claims) => {
                if let Some(c) = &claims {
                    auth.0.insert(AuthCache::key(&spec), c.clone());
                }
                claims
            }
            // The socket is gone; use what this connection proved while it was open.
            Err(()) if event == DISCONNECT_EVENT => match auth.0.get(&AuthCache::key(&spec)) {
                Some(c) => Some(c.clone()),
                None => continue,
            },
            Err(()) => continue, // same as a 401 on a webhook
        };

        let mut input = json!({
            "room_id": ctx.room_id,
            "session_id": ctx.session_id,
            "event": event,
            "payload": payload,
        });
        if let (Some(c), Value::Object(map)) = (&claims, &mut input) {
            map.insert("auth".to_string(), c.clone());
        }
        // `$trigger` is the envelope `trigger.room` answers under `room`, with
        // `auth` always present (null for an open trigger).
        let mut snapshot = input.clone();
        if let Value::Object(map) = &mut snapshot {
            map.entry("auth".to_string()).or_insert(Value::Null);
        }
        let pctx = PipelineContext {
            owner: ctx.owner.clone(),
            project: ctx.project.clone(),
            pipeline: compiled.graph.id.clone(),
            request_id: format!("{}-{}", ctx.session_id, uuid::Uuid::new_v4().simple()),
            route: Default::default(),
            input,
            trigger: Some(snapshot),
            placeholder: None,
        };
        let state = &ctx.state;
        let engine = crate::pipeline::BasicPipelineEngine::new(
            std::sync::Arc::new(state.platform.project_sandbox(&ctx.owner, &ctx.project)),
            state.frontend.rwe.clone(),
            Some(state.platform.credentials.clone()),
        )
        .with_platform(state.platform.clone())
        .with_ws_hub(state.platform.ws_hub.clone())
        .with_ws_client_manager(state.ws_client_manager.clone())
        .with_state_bus(state.platform.state_bus.clone())
        .with_data_root(state.platform.config.data_root.clone());
        if let Err(err) = engine.execute_async(&compiled.graph, &pctx).await {
            eprintln!(
                "warning: ws pipeline {} ({}) failed on {}: {} {}",
                compiled.graph.id, ctx.room_id, event, err.code, err.message
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reserved_events_reach_only_triggers_that_name_them_and_require_room() {
        assert!(!trigger_matches("", "$disconnect", "lobby", "$disconnect"), "empty --room never matches");
        assert!(trigger_matches("lobby", "$disconnect", "lobby", "$disconnect"));
        assert!(!trigger_matches("lobby", "", "lobby", "$disconnect"), "a catch-all event trigger must not see lifecycle events");
        assert!(trigger_matches("lobby", "", "lobby", "move"));
        assert!(!trigger_matches("arena", "move", "lobby", "move"));
        assert!(trigger_matches("match-*", "move", "match-42", "move"));
        assert!(!trigger_matches("match-*", "move", "match-", "move"));
        assert!(trigger_matches("thread-:id", "chat", "thread-7", "chat"));
        assert!(!trigger_matches("thread-:id", "chat", "thread-", "chat"));
    }

    #[test]
    fn admission_follows_the_rooms_triggers() {
        assert!(!admission(&[], || true), "undeclared room (no matching trigger.room) is refused");
        assert!(admission(&[false], || false), "declared room with only open triggers admits");
        assert!(!admission(&[true, false], || false), "an open trigger in the same room does not bypass a guarded trigger");
        assert!(admission(&[true, false], || true), "guarded room admits when required auth passes");
        assert!(!admission(&[true, true], || false), "all need auth and failed");
        assert!(admission(&[true], || true), "passed required auth");
    }

    #[tokio::test]
    async fn a_session_event_queue_is_bounded_and_refuses_past_its_capacity() {
        let (tx, _rx) = session_event_channel();
        for i in 0..MAX_PENDING_SESSION_EVENTS {
            assert!(enqueue_event(&tx, "move".into(), json!({ "i": i })), "event {i} fits");
        }
        assert!(
            !enqueue_event(&tx, "move".into(), json!({})),
            "one past the cap is refused, not queued without limit"
        );
    }
}
