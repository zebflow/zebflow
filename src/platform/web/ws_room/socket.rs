//! One socket on its seat: admission, `joined`, then frames out and events
//! in until it ends (`docs/contracts/rooms.md` § Clients and upgrades).
//!
//! Frames come from the session's own queue
//! ([`crate::infra::transport::ws::delivery`]); a session that fell behind is
//! woken, drops what it had queued and is sent `resync`, its whole view.

use std::sync::Arc;

use super::events::RoomCtx;
use super::*;
use crate::infra::transport::ws::{Attached, Counter, RoomHandle, Seat};

pub(super) async fn close_with(socket: &mut WebSocket, code: u16, reason: &str) {
    let frame = axum::extract::ws::CloseFrame { code, reason: reason.to_string().into() };
    send_within(socket, Message::Close(Some(frame))).await;
}

/// Sends one message, or answers false when the client has not taken it
/// within SEND_LIMIT. A client that stopped reading fills its TCP buffer, and
/// a send left to wait would stop every timer of the loop with it.
async fn send_within(socket: &mut WebSocket, message: Message) -> bool {
    matches!(tokio::time::timeout(SEND_LIMIT, socket.send(message)).await, Ok(Ok(())))
}

/// Seconds from now until `exp`, never negative.
fn until(exp: i64) -> std::time::Duration {
    let left = exp - chrono::Utc::now().timestamp();
    std::time::Duration::from_secs(left.max(0) as u64)
}

/// What the socket came with: a ticket (or nothing) to join, or a resume
/// token to take back a held seat.
pub(super) enum Pass {
    Ticket(Option<String>),
    Resume(String),
}

/// A seat a socket now holds.
struct Holding {
    room: Arc<RoomHandle>,
    seat: Seat,
    user: Option<String>,
    expires_at: Option<i64>,
    /// Raised after `joined`: `$connect`, or `$back` on a resumed seat.
    first: &'static str,
}

pub(super) async fn handle_ws_room(mut ctx: SessionCtx, pass: Pass, mut socket: WebSocket) {
    let holding = match pass {
        Pass::Ticket(ticket) => join(&ctx, ticket.as_deref()),
        Pass::Resume(token) => resume(&mut ctx, &token).await,
    };
    let Holding { room, mut seat, user, expires_at, first } = match holding {
        Ok(holding) => holding,
        Err((code, reason)) => return close_with(&mut socket, code, reason).await,
    };
    let Some(attached) = room.attach(&ctx.session_id) else { return };
    let hub = ctx.state.platform.ws_hub.clone();
    let hold = room.settings().hold;
    let token = hold.map(|_| hub.resume_token(&ctx.owner, &ctx.project, &room, &ctx.session_id));
    let resume = token.as_ref().map(|(token, _)| token.as_str());
    let reason = pump(&ctx, &room, attached, &mut seat, expires_at, &mut socket, first, resume).await;
    if let (Some(hold), Some((_, nonce)), "dropped") = (hold, token, reason) {
        return hold::keep(&ctx, &room, seat, nonce, user, hold);
    }
    // Queued behind every event this socket sent, and never refused.
    seat.queue.raise(DISCONNECT_EVENT, json!({ "reason": reason }));
}

/// Take a new seat: admission, the join, and the room loop if none runs.
fn join(ctx: &SessionCtx, ticket: Option<&str>) -> Result<Holding, admission::Refusal> {
    let admitted = admission::admit(ctx, ticket)?;
    let hub = &ctx.state.platform.ws_hub;
    let (room, guard, kicks) = hub
        .join(&ctx.owner, &ctx.project, &ctx.room_id, &ctx.session_id, admitted.user.clone(), admitted.pass.as_ref())
        .map_err(|refusal| admission::join_refusal(&refusal))?;
    let (queue, idle) = room.session_queue(&ctx.session_id, admitted.auth);
    if let Some(taken) = idle {
        let room_ctx = RoomCtx { owner: ctx.owner.clone(), project: ctx.project.clone(), room_id: ctx.room_id.clone(), state: ctx.state.clone() };
        lane().spawn(super::room_loop::run_room(room_ctx, room.clone(), taken));
    }
    let seat = Seat { guard, queue, kicks };
    Ok(Holding { room, seat, user: admitted.user, expires_at: admitted.expires_at, first: CONNECT_EVENT })
}

/// Take back a held seat with its resume token; the socket becomes that
/// session.
async fn resume(ctx: &mut SessionCtx, token: &str) -> Result<Holding, admission::Refusal> {
    let resumed = admission::admit_resume(ctx, token)?;
    let room = ctx.state.platform.ws_hub.room(&ctx.owner, &ctx.project, &ctx.room_id).ok_or(admission::CLOSE_NOT_OPEN)?;
    let (reply, seat) = tokio::sync::oneshot::channel();
    resumed.resume.reclaim.send(reply).map_err(|_| admission::CLOSE_UNAUTHORIZED)?;
    // The hold ended between the token's check and here: nothing to take.
    let seat = seat.await.map_err(|_| admission::CLOSE_UNAUTHORIZED)?;
    ctx.session_id = resumed.resume.session_id;
    Ok(Holding { room, seat, user: resumed.resume.user, expires_at: resumed.expires_at, first: BACK_EVENT })
}

/// `joined { session_id, room, state, resume? }`.
fn joined_frame(ctx: &SessionCtx, view: &Value, resume: Option<&str>) -> String {
    let mut frame = json!({ "type": "joined", "session_id": ctx.session_id, "room": ctx.room_id, "state": view });
    if let Some(token) = resume {
        frame["resume"] = json!(token);
    }
    frame.to_string()
}

/// Send `joined`, raise `first` (`$connect` or `$back`), then carry frames
/// out and events in until the socket ends; the `$disconnect` reason.
#[allow(clippy::too_many_arguments)]
async fn pump(
    ctx: &SessionCtx,
    room: &Arc<RoomHandle>,
    attached: Attached,
    seat: &mut Seat,
    expires_at: Option<i64>,
    socket: &mut WebSocket,
    first: &'static str,
    resume: Option<&str>,
) -> &'static str {
    let Attached { mut frames, wake, view } = attached;
    let session_id = ctx.session_id.as_str();
    let Seat { queue, kicks, .. } = seat;
    if !send_within(socket, Message::Text(joined_frame(ctx, &view, resume).into())).await {
        return "dropped";
    }
    room.count(Counter::MessagesOut);
    queue.raise(first, json!({}));
    // A login that never expires waits on a timer that never fires.
    let expiry = tokio::time::sleep(expires_at.map(until).unwrap_or(std::time::Duration::from_secs(u32::MAX as u64)));
    tokio::pin!(expiry);
    // A connection that died without a close (a phone off the network, a
    // half-open TCP) would keep its seat: the server pings, and a socket that
    // stays silent past SILENT_LIMIT counts as dropped, as a lost connection
    // does (`$away` under `--hold`, else `$disconnect { reason: "dropped" }`).
    let mut heartbeat = tokio::time::interval_at(tokio::time::Instant::now() + PING_EVERY, PING_EVERY);
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);
    let mut heard = tokio::time::Instant::now();
    // `--session-idle`: a session that sends no event for that long is closed.
    let session_idle = room.session_idle();
    let idle = tokio::time::sleep(session_idle.unwrap_or(std::time::Duration::from_secs(u32::MAX as u64)));
    tokio::pin!(idle);
    let mut seated = true;
    loop {
        tokio::select! {
            _ = heartbeat.tick() => {
                if heard.elapsed() >= SILENT_LIMIT {
                    return "dropped";
                }
                if !send_within(socket, Message::Ping(Vec::new().into())).await {
                    return "dropped";
                }
            }
            _ = &mut idle, if session_idle.is_some() => {
                close_with(socket, KICK_IDLE.code, KICK_IDLE.reason).await;
                return KICK_IDLE.reason;
            }
            frame = frames.recv(), if seated => {
                // The queue ends only when the seat is gone, and a kick says why.
                let Some(frame) = frame else {
                    seated = false;
                    continue;
                };
                if !send_within(socket, Message::Text(frame.to_string().into())).await {
                    return "dropped";
                }
                room.count(Counter::MessagesOut);
            }
            _ = wake.notified() => {
                if !room.is_behind(session_id) {
                    continue;
                }
                // Fell behind: what is queued is stale, and the view replaces it.
                while frames.try_recv().is_ok() {}
                let Some(state) = room.resync(session_id) else { continue };
                let resync = json!({ "type": "resync", "state": state });
                if !send_within(socket, Message::Text(resync.to_string().into())).await {
                    return "dropped";
                }
                room.count(Counter::MessagesOut);
            }
            Some(kick) = kicks.recv() => {
                close_with(socket, kick.code, kick.reason).await;
                return kick.reason;
            }
            _ = &mut expiry => {
                close_with(socket, KICK_EXPIRED.code, KICK_EXPIRED.reason).await;
                return KICK_EXPIRED.reason;
            }
            incoming = socket.recv() => {
                // Anything the client sends, a pong included, says it is there.
                if matches!(incoming, Some(Ok(_))) {
                    heard = tokio::time::Instant::now();
                }
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        let Ok(val) = serde_json::from_str::<Value>(text.as_str()) else { continue };
                        let event = val.get("event").and_then(Value::as_str).unwrap_or("message").to_string();
                        if event.starts_with('$') {
                            continue; // reserved for the server
                        }
                        room.count(Counter::MessagesIn);
                        room.touch();
                        if let Some(limit) = session_idle {
                            idle.as_mut().reset(tokio::time::Instant::now() + limit);
                        }
                        let payload = val.get("payload").cloned().unwrap_or_else(|| json!({}));
                        if !queue.push(event, payload) {
                            // A client sending faster than the room runs is
                            // closed, not given an unbounded queue of runs.
                            room.count(Counter::FloodCloses);
                            close_with(socket, KICK_FLOOD.code, "too many pending events").await;
                            return KICK_FLOOD.reason;
                        }
                    }
                    Some(Ok(Message::Close(_))) => {
                        // The socket library queues the answering close frame
                        // and sends it on the next write, flush or read. Returning
                        // without one dropped it, so the client never got a close
                        // frame back (RFC 6455 §5.5.1) and a browser reported
                        // 1006 instead of the code it sent.
                        let _ = tokio::time::timeout(SEND_LIMIT, futures::SinkExt::flush(socket)).await;
                        return "left";
                    }
                    // The socket ended without a close frame: a drop.
                    None | Some(Err(_)) => return "dropped",
                    _ => {}
                }
            }
        }
    }
}
