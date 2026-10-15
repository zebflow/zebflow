//! The project's rooms over the Studio API (`docs/contracts/rooms.md`
//! § Listing): the same operations as `ws.room.list` and `ws.room.close`,
//! answering the same JSON.
//!
//! | Route | Guard | Answers |
//! |---|---|---|
//! | `GET /api/projects/{o}/{p}/ws/rooms[?room=<pattern>][&members=1]` | `pipelines.read` | `{ rooms, total }` |
//! | `POST /api/projects/{o}/{p}/ws/rooms/{room}/close` | `pipelines.execute` | `{ room, closed, sessions }` |
//!
//! Rooms live in the office that serves the project, so a controller
//! forwards both to it.

use super::*;
use crate::infra::transport::ws::HubRefusal;

/// A hub refusal as an HTTP answer: a room not open is 404, anything else 400.
fn refusal(refusal: HubRefusal) -> Response {
    let status = match refusal {
        HubRefusal::NotOpen(_) => StatusCode::NOT_FOUND,
        _ => StatusCode::BAD_REQUEST,
    };
    (status, Json(json!({ "ok": false, "error": refusal.to_string() }))).into_response()
}

async fn forwarded(state: &PlatformAppState, owner: &str, project: &str, uri: &Uri, method: Method, headers: &HeaderMap) -> Option<Response> {
    let Ok(Some(worker_id)) = remote_project_worker_id(state, owner, project) else {
        return None;
    };
    Some(match forward_project_api_request_to_worker(state, uri, &method, headers, Bytes::new(), &worker_id).await {
        Ok(response) => response,
        Err(err) => internal_error(err),
    })
}

/// `GET …/ws/rooms`: this project's open rooms, as `ws.room.list` answers them.
pub(super) async fn api_ws_rooms_list(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Query(params): Query<HashMap<String, String>>,
) -> Response {
    if let Err(response) = require_project_api_capability(&state, &headers, &owner, &project, ProjectCapability::PipelinesRead) {
        return response;
    }
    if let Some(response) = forwarded(&state, &owner, &project, &uri, Method::GET, &headers).await {
        return response;
    }
    let (owner, project) = (slug_segment(&owner), slug_segment(&project));
    let selection = params.get("room").map(String::as_str).unwrap_or("*");
    let members = params.get("members").is_some_and(|v| matches!(v.as_str(), "1" | "true"));
    match state.platform.ws_hub.list(&owner, &project, selection, members) {
        Ok(list) => Json(list).into_response(),
        Err(err) => refusal(err),
    }
}

/// `POST …/ws/rooms/{room}/close`: close an exact room or every room a
/// pattern matches, as `ws.room.close` does.
pub(super) async fn api_ws_room_close(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, room)): Path<(String, String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(&state, &headers, &owner, &project, ProjectCapability::PipelinesExecute) {
        return response;
    }
    if let Some(response) = forwarded(&state, &owner, &project, &uri, Method::POST, &headers).await {
        return response;
    }
    let (owner, project) = (slug_segment(&owner), slug_segment(&project));
    match state.platform.ws_hub.close_report(&owner, &project, &room) {
        Ok(report) => Json(report).into_response(),
        Err(err) => refusal(err),
    }
}
