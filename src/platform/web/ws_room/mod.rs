//! WebSocket rooms: one socket's life in a room (`docs/contracts/rooms.md`).
//!
//! URL: `GET /ws/{owner}/{project}/rooms/{room}[?ticket=…|?resume=…]`, naming the wire
//! protocol `zebflow.rooms.v1` in `Sec-WebSocket-Protocol`.
//!
//! 0. **Protocol** — a request naming no protocol this server speaks is
//!    refused before the upgrade: `426 Upgrade Required`, the supported
//!    protocols in `Sec-WebSocket-Protocol` and in the JSON body.
//! 1. **Admission** — once, before anything of the room is sent
//!    ([`admission`]): `4404` for a room that is not open, `4401` for a
//!    refused join rule or ticket, `4429` for a full room.
//! 2. **Join** — through [`crate::infra::transport::ws::WsHub::join`]; the
//!    session receives `joined` with its whole view of the state.
//! 3. **Events** — inbound `{event, payload}` messages go on the room's one
//!    queue; the room loop ([`room_loop`]) runs the matching pipelines
//!    ([`events`]) **one at a time, in arrival order across every session**.
//!    A session with 256 events waiting is closed with `1008`. In a tick
//!    room client events wait for the next tick ([`tick`]).
//! 4. **Lifecycle** — the server raises `$connect` after `joined` and
//!    `$disconnect { reason }` when the socket ends, on the same queue.
//!    Clients cannot send `$`-prefixed events, and a trigger with an empty
//!    `--event` does not receive them.
//! 5. **Delivery** — the room queues each session only the frames meant for
//!    it: `patch` (what changed since its last frame), `event`; a session
//!    that fell behind receives `resync` ([`socket`]).
//! 6. **The fast lane** — the socket's loop, the room loop and every run a
//!    room starts are spawned on the rooms' own runtime
//!    ([`crate::infra::transport::ws::lane`]), never on the threads serving
//!    HTTP. Room runs write no run record; a failure is recorded at most
//!    once a minute per pipeline ([`events`]).
//! 7. **Closing** — the server closes a socket with `4410` (`ws.room.close`),
//!    `4409` (`ws.session.close`), `4408` (the login's `exp` passed) or
//!    `1008` (flooding); the `$disconnect` reason says which.
//! 8. **Holding** — in a room opened with `--hold`, a socket that drops
//!    keeps its seat ([`hold`]): `$away`, then `$back` when a socket resumes
//!    it with the token `joined` gave, or `$disconnect { reason: "timeout" }`.

mod admission;
mod events;
mod hold;
mod room_loop;
mod socket;
mod tick;

use super::*;
use crate::infra::transport::ws::{Kick, PROTOCOL, lane};

/// Raised after `joined`.
pub(super) const CONNECT_EVENT: &str = "$connect";
/// Raised when the socket ends; payload `{ reason }`.
pub(super) const DISCONNECT_EVENT: &str = "$disconnect";
/// Raised when a held room's socket drops and its seat is kept.
pub(super) const AWAY_EVENT: &str = "$away";
/// Raised when a socket resumes a held seat, after its `joined`.
pub(super) const BACK_EVENT: &str = "$back";

/// Close code and reason for a login whose `exp` passed.
const KICK_EXPIRED: Kick = Kick { code: 4408, reason: "expired" };
/// Close code for a client that broke the room's rules (RFC 6455 §7.4.1).
const KICK_FLOOD: Kick = Kick { code: 1008, reason: "flood" };

/// What one socket needs.
struct SessionCtx {
    owner: String,
    project: String,
    room_id: String,
    session_id: String,
    headers: HeaderMap,
    state: PlatformAppState,
}

/// Whether the request names [`PROTOCOL`] among its `Sec-WebSocket-Protocol`s.
fn names_protocol(headers: &HeaderMap) -> bool {
    headers
        .get_all(axum::http::header::SEC_WEBSOCKET_PROTOCOL)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .flat_map(|value| value.split(','))
        .any(|offered| offered.trim() == PROTOCOL)
}

/// A request naming no protocol this server speaks, refused before the
/// upgrade as RFC 6455 §4.4 refuses a version: `426` with the supported list.
fn protocol_refused() -> Response {
    let mut response = (
        StatusCode::UPGRADE_REQUIRED,
        Json(json!({
            "ok": false,
            "error": format!("a room socket names its wire protocol in Sec-WebSocket-Protocol: {PROTOCOL}"),
            "supported": [PROTOCOL],
        })),
    )
        .into_response();
    response.headers_mut().insert(axum::http::header::SEC_WEBSOCKET_PROTOCOL, HeaderValue::from_static(PROTOCOL));
    response
}

pub(super) async fn ws_room_handler(
    ws: WebSocketUpgrade,
    Path((owner, project, room_id)): Path<(String, String, String)>,
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    if !names_protocol(&headers) {
        return protocol_refused();
    }
    let ws = ws.protocols([PROTOCOL]);
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
            let mut proxy_headers = forwarded_websocket_headers(&headers);
            proxy_headers.push(("sec-websocket-protocol".to_string(), PROTOCOL.to_string()));
            return ws
                .on_upgrade(move |socket| proxy_websocket_to_worker(socket, worker_url, proxy_headers))
                .into_response();
        }
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }

    let pairs = uri.query().and_then(|query| serde_urlencoded::from_str::<Vec<(String, String)>>(query).ok()).unwrap_or_default();
    let param = |name: &str| pairs.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone()).filter(|v| !v.is_empty());
    let pass = match param("resume") {
        Some(token) => socket::Pass::Resume(token),
        None => socket::Pass::Ticket(param("ticket")),
    };
    let owner = crate::platform::model::slug_segment(&owner);
    let project = crate::platform::model::slug_segment(&project);
    // Random, not a clock reading: two upgrades in the same nanosecond shared a slot.
    let session_id = format!("ws-{}", uuid::Uuid::new_v4().simple());
    ws.read_buffer_size(SOCKET_BUFFER).write_buffer_size(SOCKET_BUFFER).on_upgrade(move |socket| async move {
        // The socket lives on the fast lane from here on.
        lane().spawn(socket::handle_ws_room(SessionCtx { owner, project, room_id, session_id, headers, state }, pass, socket));
    })
    .into_response()
}

/// Each room socket's read and write buffer. axum's are 128 KiB each, a
/// quarter of a megabyte a socket before it carries anything, so a 1 GB
/// device could hold only a few thousand. A room message is small; a larger
/// one is read and written in more pieces, never refused for it (the message
/// limit is its own setting).
const SOCKET_BUFFER: usize = 8 * 1024;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_names_the_protocol_among_any_it_offers() {
        let offered = |values: &[&str]| {
            let mut headers = HeaderMap::new();
            for value in values {
                headers.append(axum::http::header::SEC_WEBSOCKET_PROTOCOL, HeaderValue::from_str(value).unwrap());
            }
            names_protocol(&headers)
        };
        assert!(offered(&["zebflow.rooms.v1"]));
        assert!(offered(&["chat, zebflow.rooms.v1"]), "a browser joins the list with commas");
        assert!(offered(&["chat", "zebflow.rooms.v1"]));
        assert!(!offered(&[]));
        assert!(!offered(&["zebflow.rooms.v0, chat"]));
        assert!(!offered(&["zebflow.rooms.v1x"]));
    }
}
