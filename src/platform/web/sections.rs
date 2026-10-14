//! The addressing gate for a sectioned project (`docs/contracts/addressing.md`
//! §2c). Reached only for a host the index lists under a project with a root;
//! every other request keeps today's gate, untouched.
//!
//! One reading of the path ([`sections::read_path`]) decides everything here,
//! and the request is forwarded rebuilt from it ([`sections::serialize`]), so
//! the matcher behind decodes exactly what this gate read.

use std::sync::Arc;

use axum::http::{Method, StatusCode, Uri};
use axum::response::{IntoResponse, Response};

use super::{PlatformAppState, ProjectHost, VisitorPath, file_host, host_verify};
use crate::platform::services::addressing::Surface;
use crate::platform::services::addressing::sections::{self, HostTable, Routed, SectionTable};

/// How a request arrived on a sectioned project's host. The webhook ingress
/// reads it for the claim, the visitor form of the URLs it writes, and the
/// error pages' `original_path`.
#[derive(Clone)]
pub struct SectionArrival {
    pub table: Arc<SectionTable>,
    /// The host as the table knows it.
    pub host: String,
    /// A claimed path, or a path no route of the host matches: the ingress
    /// answers the project's 404 without matching.
    pub claimed: bool,
    /// The path the visitor asked for.
    pub visitor_path: String,
}

impl SectionArrival {
    /// The URL to emit for a written pages path on this arrival (§2c Visitor
    /// form), or `None`: nothing is emitted.
    pub fn pages_url(&self, written: &str) -> Option<String> {
        sections::pages_url(&self.table, &self.host, written)
    }
}

fn not_found(message: &'static str) -> Response {
    (StatusCode::NOT_FOUND, message).into_response()
}

/// Point the request at `target` (path and query), keeping everything else.
fn rewrite(request: &mut axum::extract::Request, target: &str) -> bool {
    let Ok(path_and_query) = target.parse::<axum::http::uri::PathAndQuery>() else {
        return false;
    };
    let mut parts = request.uri().clone().into_parts();
    parts.path_and_query = Some(path_and_query);
    match Uri::from_parts(parts) {
        Ok(uri) => {
            *request.uri_mut() = uri;
            true
        }
        Err(_) => false,
    }
}

fn with_query(path: String, request: &axum::extract::Request) -> String {
    match request.uri().query() {
        Some(query) => format!("{path}?{query}"),
        None => path,
    }
}

/// Answer, or forward, one request on a host of a sectioned project.
pub(super) async fn gate(
    state: &PlatformAppState,
    mut request: axum::extract::Request,
    next: axum::middleware::Next,
    table: Arc<SectionTable>,
    host: String,
    raw_host: &str,
) -> Response {
    let Some(host_table) = table.hosts.get(&host).cloned() else {
        return not_found("not found");
    };
    let owner = table.owner.clone();
    let project = table.project.clone();
    let raw_path = request.uri().path().to_string();
    let Some(segments) = sections::read_path(&raw_path) else {
        return (StatusCode::BAD_REQUEST, "this path has no single reading").into_response();
    };
    let first = segments.first().map(String::as_str);

    // A named file host is the gateway's alone, as today (§2b).
    if host_table.file_host {
        if request.method() != Method::GET && request.method() != Method::HEAD {
            return StatusCode::METHOD_NOT_ALLOWED.into_response();
        }
        let rest = sections::serialize(&segments).unwrap_or_else(|| "/".to_string());
        return file_host::file_host_response(state, &owner, &project, &rest, request.headers()).await;
    }
    // A host a folder serves: the folder answers it, as today — unless it is
    // also a section host, which fails closed (§2c Saves add no lock).
    if host_table.serve_host {
        if host_table.section.is_some() {
            return not_found("not found");
        }
        let port = raw_host
            .rsplit_once(':')
            .map(|(_, port)| port)
            .filter(|port| port.bytes().all(|b| b.is_ascii_digit()));
        if let Some(response) =
            file_host::execute_site_response(state, &owner, &project, &host, port, &raw_path, request.headers())
        {
            return response;
        }
    }
    // Settings → Addressing → Verify: proves the host's section too.
    if first == Some("_verify") && segments.len() <= 2 {
        let probe = sections::serialize(&segments).unwrap_or_default();
        return host_verify::answer(&probe, &owner, &project, host_table.section.as_deref());
    }

    // A published MCP route's path-inserted OAuth documents.
    if first == Some(".well-known")
        && let Some(document) = segments.get(1).filter(|d| matches!(d.as_str(), "oauth-protected-resource" | "oauth-authorization-server"))
        && segments.len() > 2
    {
        let inner = &segments[2..];
        if let Routed::Surface { surface: Surface::Mcp, mount, segments: route, root } =
            sections::route(&table, &host_table, inner, |s| table.is_enabled(s))
        {
            let Some(route_path) = sections::serialize(&route) else {
                return not_found("not found");
            };
            let route_path = if route_path == "/" { String::new() } else { route_path };
            let target = with_query(format!("/.well-known/{document}/mcp/{owner}/{project}{route_path}"), &request);
            if !rewrite(&mut request, &target) {
                return not_found("not found");
            }
            request.extensions_mut().insert(ProjectHost {
                host: host.clone(),
                owner,
                project,
                dev_host: host_table.dev_host,
                mount: Some(mount),
                root,
            });
            return next.run(request).await;
        }
    }

    // The project's own platform forms.
    let own = segments.get(1).map(String::as_str) == Some(owner.as_str())
        && segments.get(2).map(String::as_str) == Some(project.as_str());
    match first {
        Some("assets") => return next.run(request).await,
        Some(form @ ("static" | "ws" | "ms")) if own => {
            let surface = match form {
                "static" => Surface::Static,
                "ws" => Surface::Ws,
                _ => Surface::Ms,
            };
            if !table.is_enabled(surface) {
                return not_found("this surface is switched off for the project (Settings → Addressing)");
            }
            let target = with_query(sections::serialize(&segments).unwrap_or_default(), &request);
            if !rewrite(&mut request, &target) {
                return not_found("not found");
            }
            request.extensions_mut().insert(ProjectHost {
                host: host.clone(),
                owner,
                project,
                dev_host: host_table.dev_host,
                mount: None,
                root: None,
            });
            return next.run(request).await;
        }
        Some(form @ ("wh" | "mcp")) if own => {
            // A section host passes neither; any other host passes its own,
            // as an arrival on that host: claimed paths stay the project's 404.
            if host_table.section.is_some() {
                return not_found("not found");
            }
            let surface = if form == "wh" { Surface::Pages } else { Surface::Mcp };
            if !table.is_enabled(surface) {
                return not_found("this surface is switched off for the project (Settings → Addressing)");
            }
            let tail = &segments[3..];
            let route_part: Vec<String> = tail.iter().take_while(|s| surface != Surface::Mcp || *s != "_oauth").cloned().collect();
            if table.claims(&route_part).is_some() {
                return forward_app(state, request, next, &table, &host, &host_table, Surface::Pages, String::new(), segments.clone(), None, true, &raw_path).await;
            }
            return forward_app(state, request, next, &table, &host, &host_table, surface, String::new(), tail.to_vec(), None, false, &raw_path).await;
        }
        Some("api") if own_api(&segments, &owner, &project) => {
            let dev_mcp = segments.get(4).map(String::as_str) == Some("mcp");
            if dev_mcp || !table.api_on_hosts {
                return not_found("not found");
            }
            return next.run(request).await;
        }
        _ => {}
    }

    match sections::route(&table, &host_table, &segments, |s| table.is_enabled(s)) {
        Routed::BadPath => (StatusCode::BAD_REQUEST, "this path has no single reading").into_response(),
        Routed::Disabled(surface) => (
            StatusCode::NOT_FOUND,
            format!("{} is switched off for this project (Settings → Addressing)", surface.title()),
        )
            .into_response(),
        // No route, or a claimed path: the project's own 404, as an unknown
        // path is — never a platform route.
        Routed::NotFound => {
            forward_app(state, request, next, &table, &host, &host_table, Surface::Pages, String::new(), segments, None, true, &raw_path).await
        }
        Routed::Surface { surface: Surface::Files, segments: rest, .. } => {
            if request.method() != Method::GET && request.method() != Method::HEAD {
                return StatusCode::METHOD_NOT_ALLOWED.into_response();
            }
            let rest = sections::serialize(&rest).unwrap_or_else(|| "/".to_string());
            file_host::file_host_response(state, &owner, &project, &rest, request.headers()).await
        }
        Routed::Surface { surface: surface @ (Surface::Pages | Surface::Mcp), mount, segments: app, root } => {
            forward_app(state, request, next, &table, &host, &host_table, surface, mount, app, root, false, &raw_path).await
        }
        Routed::Surface { surface, mount, segments: rest, .. } => {
            let Some(prefix) = surface.platform_prefix(&owner, &project) else {
                return not_found("not found");
            };
            let rest = sections::serialize(&rest).unwrap_or_else(|| "/".to_string());
            let target = with_query(format!("{prefix}{rest}"), &request);
            if !rewrite(&mut request, &target) {
                return not_found("not found");
            }
            request.extensions_mut().insert(VisitorPath(raw_path));
            request.extensions_mut().insert(ProjectHost {
                host: host.clone(),
                owner,
                project,
                dev_host: host_table.dev_host,
                mount: Some(mount),
                root: None,
            });
            next.run(request).await
        }
    }
}

fn own_api(segments: &[String], owner: &str, project: &str) -> bool {
    segments.get(1).map(String::as_str) == Some("projects")
        && segments.get(2).map(String::as_str) == Some(owner)
        && segments.get(3).map(String::as_str) == Some(project)
}

/// Forward a pages or mcp request to its platform form with the arrival.
#[allow(clippy::too_many_arguments)]
async fn forward_app(
    _state: &PlatformAppState,
    mut request: axum::extract::Request,
    next: axum::middleware::Next,
    table: &Arc<SectionTable>,
    host: &str,
    host_table: &HostTable,
    surface: Surface,
    mount: String,
    app: Vec<String>,
    root: Option<String>,
    claimed: bool,
    raw_path: &str,
) -> Response {
    let prefix = if surface == Surface::Mcp { "mcp" } else { "wh" };
    let Some(path) = sections::serialize(&app) else {
        return not_found("not found");
    };
    let path = if path == "/" && surface == Surface::Mcp { String::new() } else { path };
    let target = with_query(format!("/{prefix}/{}/{}{path}", table.owner, table.project), &request);
    if !rewrite(&mut request, &target) {
        return not_found("not found");
    }
    request.extensions_mut().insert(VisitorPath(raw_path.to_string()));
    request.extensions_mut().insert(ProjectHost {
        host: host.to_string(),
        owner: table.owner.clone(),
        project: table.project.clone(),
        dev_host: host_table.dev_host,
        mount: Some(mount),
        root,
    });
    request.extensions_mut().insert(SectionArrival {
        table: table.clone(),
        host: host.to_string(),
        claimed,
        visitor_path: raw_path.to_string(),
    });
    next.run(request).await
}

/// A `/wh/` or `/mcp/` platform form (or its OAuth documents) of a sectioned
/// project on a host that is neither a project host nor the platform address:
/// a rootless arrival, so a claimed path answers 404 (§2c Claim).
pub(super) fn stray_claimed(state: &PlatformAppState, host: Option<&str>, path: &str) -> bool {
    let inner = path
        .strip_prefix("/.well-known/oauth-protected-resource")
        .or_else(|| path.strip_prefix("/.well-known/oauth-authorization-server"))
        .unwrap_or(path);
    let mut parts = inner.trim_start_matches('/').splitn(4, '/');
    let (Some(form), Some(owner), Some(project)) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    if form != "wh" && form != "mcp" {
        return false;
    }
    let Some(table) = state.platform.addressing.sections(owner, project) else {
        return false;
    };
    if host.is_some_and(sections::is_platform_host) {
        return false;
    }
    let Some(tail) = sections::read_path(parts.next().unwrap_or_default()) else {
        return true;
    };
    let route_part: Vec<String> = tail.iter().take_while(|s| form != "mcp" || *s != "_oauth").cloned().collect();
    table.claims(&route_part).is_some()
}
