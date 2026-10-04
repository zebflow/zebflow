//! Published MCP servers — the `mcp` surface (`docs/contracts/published-mcp.md`,
//! `docs/contracts/addressing.md` §2).
//!
//! Every active `trigger.mcp` with the same `--route` in one project is one
//! tool of one MCP server, answering at `/_mcp/ROUTE` on the project's hosts
//! (the addressing gate rewrites that) and `/mcp/{owner}/{project}/ROUTE` on
//! the platform. A request reaches its tools in this order:
//!
//! 1. the `mcp` switch: off by default, and off means 404 on both forms;
//! 2. the client's refusal count on this route: out of refusals is 429;
//! 3. a project placed on a worker office is answered there, whole;
//! 4. the route's tools, read from the active runtime of this one project —
//!    no tools on the route, no server (404);
//! 5. the route's `--auth`, checked at the door ([`guard`]) with the app's
//!    own credentials before anything runs — on an `oauth` route, an access
//!    token its own sign-in ([`oauth`]: `/_oauth/…` under the route and the
//!    path-inserted well-known documents) issued;
//! 6. rmcp's streamable-HTTP server, stateless, built for this request around
//!    exactly those tools: `initialize`, `tools/list`, `tools/call`.
//!
//! Nothing here touches the project's dev MCP (`crate::platform::mcp`), and
//! nothing there reads `trigger.mcp`: the two share no tool, list, session or
//! lookup.

mod guard;
mod oauth;
mod run;
mod server;

pub use guard::FailureLimiter;
pub use oauth::well_known_split;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::{ConnectInfo, Path, State};
use axum::http::{Extensions, HeaderMap, Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;

use super::{PlatformAppState, ProjectHost};
use oauth::{Issuer, Op};
use server::{PublishedServer, serve};
use crate::infra::execution::placement::ProjectRuntimePlacementTarget;
use crate::pipeline::nodes::basic::trigger::mcp_trigger;
use crate::platform::services::addressing::Surface;
use crate::platform::services::pipeline_runtime::{CompiledPipeline, McpTriggerSpec};

/// Where a forwarded request reaches a worker office.
const WORKER_PREFIX: &str = "/api/internal/runtime/mcp/";
/// On a worker hop: the surface's address and the app's pages, as the
/// controller read them off the client's arrival (`{base} {pages}`). Read
/// only from the controller; a client's copy is dropped.
const BASE_HEADER: &str = "x-zebflow-mcp-base";

/// One published tool: the active pipeline and the trigger that publishes it.
#[derive(Clone)]
struct PublishedTool {
    compiled: CompiledPipeline,
    spec: McpTriggerSpec,
}

/// How a request arrived at a route.
enum Arrival {
    /// From a client, over the platform form or a project host.
    Client { peer: Option<SocketAddr> },
    /// From this office's controller, which checked the switch and counts
    /// the client's refusals itself.
    Controller,
}

/// The tools one route publishes in one project, in a stable order.
fn tools_on_route(state: &PlatformAppState, owner: &str, project: &str, route: &str) -> Vec<PublishedTool> {
    let mut tools: Vec<PublishedTool> = state
        .platform
        .pipeline_runtime
        .list_project(owner, project)
        .into_iter()
        .flat_map(|compiled| {
            compiled
                .mcp_triggers
                .iter()
                .filter(|spec| spec.route == route)
                .map(|spec| PublishedTool { compiled: compiled.clone(), spec: spec.clone() })
                .collect::<Vec<_>>()
        })
        .collect();
    tools.sort_by(|a, b| {
        a.spec.tool_name.cmp(&b.spec.tool_name).then(a.compiled.file_rel_path.cmp(&b.compiled.file_rel_path))
    });
    tools
}

fn refusal(status: StatusCode, code: &str, message: &str) -> Response {
    (status, Json(json!({ "ok": false, "error": { "code": code, "message": message } }))).into_response()
}

fn peer_of(extensions: &Extensions) -> Option<SocketAddr> {
    extensions.get::<ConnectInfo<SocketAddr>>().map(|ConnectInfo(addr)| *addr)
}

/// One request at a route: who sent it, and what it asks.
struct Request {
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
    host: Option<ProjectHost>,
    arrival: Arrival,
}

fn from_client(method: Method, uri: Uri, headers: HeaderMap, extensions: &Extensions, body: Bytes) -> Request {
    let host = extensions.get::<ProjectHost>().cloned();
    Request { method, uri, headers, body, host, arrival: Arrival::Client { peer: peer_of(extensions) } }
}

/// `/mcp/{owner}/{project}` — a route of `/`.
pub(super) async fn ingress_root(
    State(state): State<PlatformAppState>,
    Path((owner, project)): Path<(String, String)>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    extensions: Extensions,
    body: Bytes,
) -> Response {
    let request = from_client(method, uri, headers, &extensions, body);
    answer(state, owner, project, String::new(), None, request).await
}

/// `/mcp/{owner}/{project}/{route}` — one published server, or its
/// `/_oauth/…` sign-in.
pub(super) async fn ingress(
    State(state): State<PlatformAppState>,
    Path((owner, project, tail)): Path<(String, String, String)>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    extensions: Extensions,
    body: Bytes,
) -> Response {
    let request = from_client(method, uri, headers, &extensions, body);
    answer(state, owner, project, tail, None, request).await
}

macro_rules! well_known {
    ($root:ident, $named:ident, $op:expr, $what:literal) => {
        #[doc = concat!("`/.well-known/", $what, "/mcp/{owner}/{project}` — a route of `/`.")]
        pub(super) async fn $root(
            State(state): State<PlatformAppState>,
            Path((owner, project)): Path<(String, String)>,
            method: Method,
            uri: Uri,
            headers: HeaderMap,
            extensions: Extensions,
            body: Bytes,
        ) -> Response {
            let request = from_client(method, uri, headers, &extensions, body);
            answer(state, owner, project, String::new(), Some($op), request).await
        }

        #[doc = concat!("`/.well-known/", $what, "/mcp/{owner}/{project}/{route}` — the platform form; a project host's path-inserted document is rewritten here.")]
        pub(super) async fn $named(
            State(state): State<PlatformAppState>,
            Path((owner, project, route)): Path<(String, String, String)>,
            method: Method,
            uri: Uri,
            headers: HeaderMap,
            extensions: Extensions,
            body: Bytes,
        ) -> Response {
            let request = from_client(method, uri, headers, &extensions, body);
            answer(state, owner, project, route, Some($op), request).await
        }
    };
}

well_known!(protected_resource_root, protected_resource, Op::ProtectedResource, "oauth-protected-resource");
well_known!(authorization_server_root, authorization_server, Op::AuthorizationServer, "oauth-authorization-server");

/// `/api/internal/runtime/mcp/{owner}/{project}` — a route of `/`, from the controller.
pub(super) async fn internal_ingress_root(
    State(state): State<PlatformAppState>,
    Path((owner, project)): Path<(String, String)>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Err(response) = super::require_controller_call(&state, &headers) {
        return response;
    }
    let request = Request { method, uri, headers, body, host: None, arrival: Arrival::Controller };
    answer(state, owner, project, String::new(), None, request).await
}

/// `/api/internal/runtime/mcp/{owner}/{project}/{route}` — from the controller.
pub(super) async fn internal_ingress(
    State(state): State<PlatformAppState>,
    Path((owner, project, tail)): Path<(String, String, String)>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if let Err(response) = super::require_controller_call(&state, &headers) {
        return response;
    }
    let request = Request { method, uri, headers, body, host: None, arrival: Arrival::Controller };
    answer(state, owner, project, tail, None, request).await
}

/// The surface's address and the app's pages for this request, if it can
/// name them: from the arrival, or — on a worker hop — from the controller.
fn base_of(owner: &str, project: &str, request: &Request) -> Option<(String, String)> {
    match request.arrival {
        Arrival::Client { .. } => {
            oauth::arrival_base(owner, project, request.host.as_ref(), &request.headers, oauth::platform_base().as_deref())
        }
        Arrival::Controller => {
            let value = request.headers.get(BASE_HEADER)?.to_str().ok()?;
            let (base, pages) = value.split_once(' ')?;
            Some((base.to_string(), pages.to_string()))
        }
    }
}

async fn answer(state: PlatformAppState, owner: String, project: String, tail: String, op_hint: Option<Op>, request: Request) -> Response {
    let owner = crate::platform::model::slug_segment(&owner);
    let project = crate::platform::model::slug_segment(&project);
    let (route_tail, op) = match op_hint {
        Some(op) => (tail, op),
        None => oauth::split_tail(&tail),
    };
    // The metadata documents answer at their well-known addresses; by path
    // under the route only on a controller's hop.
    if op_hint.is_none() && op.internal() && !matches!(request.arrival, Arrival::Controller) {
        return refusal(StatusCode::NOT_FOUND, "not_found", "no MCP server is published on this route");
    }
    let route = mcp_trigger::normalize_route(&route_tail);
    let base = base_of(&owner, &project, &request);

    let limit = match request.arrival {
        Arrival::Controller => None,
        Arrival::Client { peer } => {
            // The gate refuses a disabled surface before this; asked again
            // here because this is the door, whatever routed the request.
            let enabled = state
                .platform
                .addressing
                .read(&owner, &project)
                .map(|a| a.is_enabled(Surface::Mcp))
                .unwrap_or(false);
            if !enabled {
                return refusal(StatusCode::NOT_FOUND, "not_found", "the mcp surface is switched off for this project (Settings → Addressing)");
            }
            let key = guard::limit_key(&owner, &project, &route, &guard::client_address(peer, &request.headers));
            if let Some(wait) = state.mcp_failures.blocked(&key) {
                return guard::too_many(wait);
            }
            if let Some(worker_id) = worker_for(&state, &owner, &project) {
                let path = match op.word().filter(|_| op_hint.is_some()) {
                    // A well-known document: the worker reads it by path.
                    Some(word) => Some(format!("{WORKER_PREFIX}{owner}/{project}{}/_oauth/{word}", if route == "/" { "" } else { route.as_str() })),
                    None => worker_path_and_query(&request.uri),
                };
                let response = forward(&state, &request, path, base.as_ref(), &worker_id).await;
                if matches!(response.status(), StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN) {
                    state.mcp_failures.fail(&key);
                }
                return response;
            }
            Some(key)
        }
    };

    let tools = tools_on_route(&state, &owner, &project, &route);
    let Some(first) = tools.first() else {
        return refusal(StatusCode::NOT_FOUND, "not_found", "no MCP server is published on this route");
    };
    // Activation refuses a route whose tools disagree; a route that still
    // does (a snapshot older than the rule) is refused whole, never guessed.
    let guard_of_route = first.spec.guard();
    if tools.iter().any(|t| t.spec.guard() != guard_of_route) {
        return refusal(StatusCode::INTERNAL_SERVER_ERROR, "mcp_route_auth_mixed", "the tools on this route declare different --auth; activate them again with one");
    }
    let oauth_route = first.spec.auth_type == "oauth";
    let issuer = base.as_ref().filter(|_| oauth_route).and_then(|(base, pages)| Issuer::new(base, &route, pages));
    if op != Op::Mcp && !oauth_route {
        return refusal(StatusCode::NOT_FOUND, "not_found", "this MCP server does not sign people in (--auth is not oauth)");
    }
    if oauth_route && issuer.is_none() {
        // D4: the platform form names its address only from the configured base.
        return refusal(StatusCode::NOT_FOUND, "not_found", "this MCP server signs people in on the project's host; connect there");
    }
    if op != Op::Mcp {
        let spec = first.spec.clone();
        let issuer = issuer.expect("checked above");
        let ctx = oauth::Ctx { state: &state, owner: &owner, project: &project, route: &route, spec: &spec, issuer: &issuer, limit: limit.as_deref() };
        return oauth::dispatch(ctx, op, &request.method, &request.uri, &request.headers, &request.body).await;
    }
    let door = guard::Door {
        state: &state,
        owner: &owner,
        project: &project,
        route: &route,
        tools: &tools,
        headers: &request.headers,
        body: &request.body,
        issuer: issuer.as_ref(),
        limit: limit.as_deref(),
    };
    if let Err(response) = door.check() {
        return response;
    }

    let server = PublishedServer { state: state.clone(), owner, project, route, tools: Arc::new(tools) };
    serve(server, request.method, request.uri, request.headers, request.body).await
}

/// The worker office this project runs on, when it is not this one.
fn worker_for(state: &PlatformAppState, owner: &str, project: &str) -> Option<String> {
    let placement = state.platform.cluster_placement.get(owner, project).ok()??;
    if placement.target != ProjectRuntimePlacementTarget::Worker {
        return None;
    }
    placement.worker_id.filter(|id| *id != state.platform.cluster_bootstrap.node_id())
}

/// The worker's path for a published route: `/mcp/{o}/{p}/…` →
/// `/api/internal/runtime/mcp/{o}/{p}/…`, raw, query kept.
fn worker_path_and_query(uri: &Uri) -> Option<String> {
    let suffix = uri.path().strip_prefix("/mcp/")?;
    let query = uri.query().map(|q| format!("?{q}")).unwrap_or_default();
    Some(format!("{WORKER_PREFIX}{suffix}{query}"))
}

async fn forward(state: &PlatformAppState, request: &Request, path: Option<String>, base: Option<&(String, String)>, worker_id: &str) -> Response {
    let Some(path_and_query) = path else {
        return refusal(StatusCode::NOT_FOUND, "not_found", "not a published MCP route");
    };
    let mut headers = request.headers.clone();
    headers.remove(BASE_HEADER);
    if let Some((base, pages)) = base
        && let Ok(value) = axum::http::HeaderValue::from_str(&format!("{base} {pages}"))
    {
        headers.insert(BASE_HEADER, value);
    }
    match super::forward_to_worker(state, &request.method, &path_and_query, &headers, &request.body, worker_id).await {
        Ok(response) => response,
        Err(err) => super::internal_error(err),
    }
}

#[cfg(test)]
mod tests {
    use super::worker_path_and_query;

    #[test]
    fn a_worker_hop_changes_only_the_prefix() {
        let uri = "/mcp/site-a/demo/shop/?x=1".parse().unwrap();
        assert_eq!(worker_path_and_query(&uri).as_deref(), Some("/api/internal/runtime/mcp/site-a/demo/shop/?x=1"));
        let root = "/mcp/site-a/demo".parse().unwrap();
        assert_eq!(worker_path_and_query(&root).as_deref(), Some("/api/internal/runtime/mcp/site-a/demo"));
        let other = "/wh/site-a/demo/shop".parse().unwrap();
        assert_eq!(worker_path_and_query(&other), None);
    }
}
