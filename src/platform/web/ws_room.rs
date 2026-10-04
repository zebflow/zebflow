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

/// Whether a `trigger.room` with `t_room` / `t_event` handles `event` in `room_id`.
/// Reserved `$` events reach only triggers that name them.
pub(super) fn trigger_matches(t_room: &str, t_event: &str, room_id: &str, event: &str) -> bool {
    let room_match = t_room.is_empty() || t_room == room_id;
    let event_match = t_event == event || (t_event.is_empty() && !event.starts_with('$'));
    room_match && event_match
}

/// Whether a connection may enter a room, from the auth its triggers require.
///
/// `requires_auth` holds one entry per `trigger.room` that can fire in the room.
/// A room with no triggers, or with any open trigger, admits everyone (anything
/// it shows can be acted on without signing in anyway); otherwise the connection
/// must pass at least one trigger's auth.
pub(super) fn admission(requires_auth: &[bool], passes_one: impl FnOnce() -> bool) -> bool {
    if requires_auth.is_empty() || requires_auth.iter().any(|required| !required) {
        return true;
    }
    passes_one()
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

    let (events_tx, events_rx) = tokio::sync::mpsc::unbounded_channel::<(String, Value)>();
    tokio::spawn(run_session_events(ctx, auth, events_rx));

    let mut reason = "close";
    if socket.send(Message::Text(joined.to_string().into())).await.is_err() {
        reason = "error";
    } else {
        let _ = events_tx.send((CONNECT_EVENT.to_string(), json!({})));
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
                        let _ = events_tx.send((event, payload));
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
    let _ = events_tx.send((DISCONNECT_EVENT.to_string(), json!({ "reason": reason })));
    drop(events_tx);
    hub.leave_room(&room_key, guard);
}

/// Decide admission for one connection, remembering the auth it passed.
fn admit(ctx: &SessionCtx, auth: &mut AuthCache) -> bool {
    let pipelines = ctx.state.platform.pipeline_runtime.list_project(&ctx.owner, &ctx.project);
    let specs: Vec<_> = pipelines
        .iter()
        .flat_map(|p| p.ws_triggers.iter())
        .filter(|t| t.room.is_empty() || t.room == ctx.room_id)
        .collect();
    let requires: Vec<bool> = specs.iter().map(|t| !is_open(&t.auth_type)).collect();
    // The room's own `--auth`, from the visitor's headers, on every office: a
    // connection a controller forwards carries the visitor's headers, and the
    // controller's call header proves only who forwarded it.
    admission(&requires, || {
        for spec in specs.iter().filter(|t| !is_open(&t.auth_type)) {
            if let Ok(claims) = verify_trigger_auth(ctx, spec) {
                auth.0.insert(AuthCache::key(spec), claims.unwrap_or(Value::Null));
                return true;
            }
        }
        false
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
    mut events: tokio::sync::mpsc::UnboundedReceiver<(String, Value)>,
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
    fn reserved_events_reach_only_triggers_that_name_them() {
        assert!(trigger_matches("", "$disconnect", "lobby", "$disconnect"));
        assert!(!trigger_matches("", "", "lobby", "$disconnect"), "a catch-all trigger must not see lifecycle events");
        assert!(trigger_matches("", "", "lobby", "move"));
        assert!(!trigger_matches("arena", "move", "lobby", "move"));
    }

    #[test]
    fn admission_follows_the_rooms_triggers() {
        assert!(admission(&[], || false), "no triggers: open");
        assert!(admission(&[true, false], || false), "any open trigger: open");
        assert!(!admission(&[true, true], || false), "all need auth and none passed");
        assert!(admission(&[true], || true), "passed one");
    }
}
