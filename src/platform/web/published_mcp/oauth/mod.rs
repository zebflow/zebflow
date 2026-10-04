//! `--auth oauth` on a published route (`published-mcp.md` § `--auth
//! oauth`): the route is an OAuth 2.1 protected resource and its own
//! authorization server, built to MCP authorization 2026-07-28 (compatible
//! with 2025-11-25). People sign in through the **app's** login page; the
//! tokens are the app's JWTs, signed with the route's `--credential`. No
//! Zebflow account, session, cookie or role takes part anywhere.
//!
//! For a route whose connect URL is `R` (`https://library.example/_mcp/library`):
//!
//! | What | Where |
//! |---|---|
//! | protected resource metadata (RFC 9728) | `/.well-known/oauth-protected-resource` + path of `R` |
//! | authorization server metadata (RFC 8414) | `/.well-known/oauth-authorization-server` + path of `R` |
//! | authorize | `R/_oauth/authorize` |
//! | token | `R/_oauth/token` |
//! | client registration (RFC 7591) | `R/_oauth/register` |
//!
//! `resource` and `issuer` are both `R`, built from the arrival — a named
//! host is `https://{host}{mount}`, the dev host `http://{host}:{port}{mount}`,
//! the platform form `{ZEBFLOW_PLATFORM_BASE_URL}/mcp/{o}/{p}` (no OAuth when
//! that is unset) — never guessed from a forwarded header.

mod access;
mod authorize;
mod cimd;
mod metadata;
mod register;
mod token;

pub(super) use access::{verify_access, AccessRefusal};

use axum::Json;
use axum::body::Bytes;
use axum::http::{HeaderMap, HeaderValue, Method, StatusCode, Uri, header};
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};

use super::PlatformAppState;
use crate::platform::services::pipeline_runtime::McpTriggerSpec;
use crate::platform::web::ProjectHost;

/// What a request under a route asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::platform::web) enum Op {
    /// The MCP protocol itself.
    Mcp,
    Authorize,
    Token,
    Register,
    /// RFC 9728 metadata — from the well-known route, or a controller's hop.
    ProtectedResource,
    /// RFC 8414 metadata — likewise.
    AuthorizationServer,
}

impl Op {
    /// The word after `/_oauth/` a worker hop uses for this operation.
    pub(super) fn word(self) -> Option<&'static str> {
        Some(match self {
            Op::Mcp => return None,
            Op::Authorize => "authorize",
            Op::Token => "token",
            Op::Register => "register",
            Op::ProtectedResource => "protected-resource",
            Op::AuthorizationServer => "authorization-server",
        })
    }

    /// Whether only this office's controller may ask for it by path.
    pub(super) fn internal(self) -> bool {
        matches!(self, Op::ProtectedResource | Op::AuthorizationServer)
    }
}

/// `library/_oauth/token` → (`library`, `Token`); anything else is the MCP
/// protocol on the whole tail. A route never has a `_` segment (refused at
/// activation), so this never takes a route's own path.
pub(super) fn split_tail(tail: &str) -> (String, Op) {
    let trimmed = tail.trim_end_matches('/');
    let (route, word) = match trimmed.rsplit_once("_oauth/") {
        Some((route, word)) if route.is_empty() || route.ends_with('/') => (route.trim_end_matches('/'), word),
        _ => return (tail.to_string(), Op::Mcp),
    };
    let op = match word {
        "authorize" => Op::Authorize,
        "token" => Op::Token,
        "register" => Op::Register,
        "protected-resource" => Op::ProtectedResource,
        "authorization-server" => Op::AuthorizationServer,
        _ => return (tail.to_string(), Op::Mcp),
    };
    (route.to_string(), op)
}

/// `/.well-known/oauth-protected-resource/_mcp/library` →
/// (`oauth-protected-resource`, `/_mcp/library`).
pub fn well_known_split(path: &str) -> Option<(&'static str, &str)> {
    for document in ["oauth-protected-resource", "oauth-authorization-server"] {
        if let Some(inner) = path.strip_prefix("/.well-known/").and_then(|p| p.strip_prefix(document))
            && inner.starts_with('/')
            && inner.len() > 1
        {
            return Some((document, inner));
        }
    }
    None
}

/// The connect URL of a route and the addresses derived from it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::platform::web) struct Issuer {
    /// `scheme://host[:port]`.
    origin: String,
    /// The connect URL's path (`/_mcp/library`).
    path: String,
    /// Where the app's pages are, for `--login`: the origin on a project
    /// host, `{base}/wh/{o}/{p}` on the platform form.
    pages: String,
}

impl Issuer {
    /// `base` is the surface's address (`https://library.example/_mcp`).
    pub(super) fn new(base: &str, route: &str, pages: &str) -> Option<Issuer> {
        let url = reqwest::Url::parse(base).ok()?;
        let host = url.host_str()?.to_ascii_lowercase();
        let origin = match url.port() {
            Some(port) => format!("{}://{host}:{port}", url.scheme()),
            None => format!("{}://{host}", url.scheme()),
        };
        let mount = url.path().trim_end_matches('/');
        let path = if route == "/" { mount.to_string() } else { format!("{mount}{route}") };
        Some(Issuer { origin, path, pages: pages.trim_end_matches('/').to_string() })
    }

    /// `resource` = `issuer` = the connect URL.
    pub(super) fn resource(&self) -> String {
        format!("{}{}", self.origin, self.path)
    }

    pub(super) fn endpoint(&self, name: &str) -> String {
        format!("{}/_oauth/{name}", self.resource())
    }

    pub(super) fn protected_resource_url(&self) -> String {
        format!("{}/.well-known/oauth-protected-resource{}", self.origin, self.path)
    }

    #[cfg(test)]
    pub(super) fn authorization_server_url(&self) -> String {
        format!("{}/.well-known/oauth-authorization-server{}", self.origin, self.path)
    }

    pub(super) fn login_url(&self, login: &str) -> String {
        format!("{}{login}", self.pages)
    }
}

/// The surface's address and where the app's pages are, for a request from
/// a client: built from the host it arrived on, or the configured platform
/// base for the platform form. `None`: this arrival cannot name its own
/// address, so it has no OAuth.
pub(super) fn arrival_base(
    owner: &str,
    project: &str,
    host: Option<&ProjectHost>,
    headers: &HeaderMap,
    platform_base: Option<&str>,
) -> Option<(String, String)> {
    if let Some(project_host) = host
        && let Some(mount) = &project_host.mount
    {
        let origin = if project_host.dev_host {
            // The dev host answers on this office's own port.
            let port = headers
                .get(header::HOST)
                .and_then(|v| v.to_str().ok())
                .and_then(|h| h.rsplit_once(':'))
                .map(|(_, port)| port)
                .filter(|port| !port.is_empty() && port.bytes().all(|b| b.is_ascii_digit()));
            match port {
                Some(port) => format!("http://{}:{port}", project_host.host),
                None => format!("http://{}", project_host.host),
            }
        } else {
            format!("https://{}", project_host.host)
        };
        return Some((format!("{origin}{mount}"), origin));
    }
    let base = platform_base?.trim().trim_end_matches('/');
    if base.is_empty() {
        return None;
    }
    Some((format!("{base}/mcp/{owner}/{project}"), format!("{base}/wh/{owner}/{project}")))
}

/// The platform's configured public address (`ZEBFLOW_PLATFORM_BASE_URL`).
pub(super) fn platform_base() -> Option<String> {
    std::env::var("ZEBFLOW_PLATFORM_BASE_URL").ok()
}

/// Everything one OAuth request of one route needs.
pub(super) struct Ctx<'a> {
    pub state: &'a PlatformAppState,
    pub owner: &'a str,
    pub project: &'a str,
    pub route: &'a str,
    pub spec: &'a McpTriggerSpec,
    pub issuer: &'a Issuer,
    /// The client's refusal count on this route, when this office counts it.
    pub limit: Option<&'a str>,
}

impl Ctx<'_> {
    /// A refusal the client caused, counted against it.
    pub(super) fn refused(&self) {
        if let Some(key) = self.limit {
            self.state.mcp_failures.fail(key);
        }
    }
}

/// Answers one OAuth operation of an `oauth` route.
pub(super) async fn dispatch(ctx: Ctx<'_>, op: Op, method: &Method, uri: &Uri, headers: &HeaderMap, body: &Bytes) -> Response {
    if method == Method::OPTIONS && op != Op::Authorize {
        return preflight();
    }
    match (op, method.as_str()) {
        (Op::ProtectedResource, "GET") => metadata::protected_resource(&ctx),
        (Op::AuthorizationServer, "GET") => metadata::authorization_server(&ctx),
        (Op::Register, "POST") => register::register(&ctx, body),
        (Op::Authorize, "GET") => authorize::authorize(&ctx, uri.query().unwrap_or_default()).await,
        (Op::Token, "POST") => token::token(&ctx, headers, body),
        _ => oauth_error(StatusCode::METHOD_NOT_ALLOWED, "invalid_request", "this endpoint does not answer that method"),
    }
}

fn with_common_headers(mut response: Response) -> Response {
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*"));
    response
}

/// A JSON answer: `no-store`, readable from any origin.
pub(super) fn json_answer(status: StatusCode, value: Value) -> Response {
    with_common_headers((status, Json(value)).into_response())
}

/// RFC 6749 §5.2: `{ error, error_description }`.
pub(super) fn oauth_error(status: StatusCode, error: &str, description: &str) -> Response {
    json_answer(status, json!({ "error": error, "error_description": description }))
}

fn preflight() -> Response {
    let mut response = with_common_headers(StatusCode::NO_CONTENT.into_response());
    let headers = response.headers_mut();
    headers.insert(header::ACCESS_CONTROL_ALLOW_METHODS, HeaderValue::from_static("GET, POST, OPTIONS"));
    headers.insert(
        header::ACCESS_CONTROL_ALLOW_HEADERS,
        HeaderValue::from_static("authorization, content-type, mcp-protocol-version"),
    );
    headers.insert(header::ACCESS_CONTROL_MAX_AGE, HeaderValue::from_static("600"));
    response
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tail_names_its_operation_and_a_route_keeps_its_path() {
        assert_eq!(split_tail("library"), ("library".to_string(), Op::Mcp));
        assert_eq!(split_tail("library/_oauth/token"), ("library".to_string(), Op::Token));
        assert_eq!(split_tail("shelf/library/_oauth/authorize"), ("shelf/library".to_string(), Op::Authorize));
        assert_eq!(split_tail("_oauth/register"), (String::new(), Op::Register));
        assert_eq!(split_tail("library/_oauth/other"), ("library/_oauth/other".to_string(), Op::Mcp));
        assert_eq!(split_tail("library_oauth/token"), ("library_oauth/token".to_string(), Op::Mcp));
    }

    #[test]
    fn the_documents_sit_at_the_path_inserted_well_known_uris() {
        let issuer = Issuer::new("http://Library.Site-A.localhost:10610/_mcp", "/library", "http://library.site-a.localhost:10610").unwrap();
        assert_eq!(issuer.resource(), "http://library.site-a.localhost:10610/_mcp/library");
        assert_eq!(issuer.protected_resource_url(), "http://library.site-a.localhost:10610/.well-known/oauth-protected-resource/_mcp/library");
        assert_eq!(issuer.authorization_server_url(), "http://library.site-a.localhost:10610/.well-known/oauth-authorization-server/_mcp/library");
        assert_eq!(issuer.endpoint("token"), "http://library.site-a.localhost:10610/_mcp/library/_oauth/token");
        assert_eq!(issuer.login_url("/auth/login"), "http://library.site-a.localhost:10610/auth/login");
        let root = Issuer::new("https://library.example/_mcp", "/", "https://library.example").unwrap();
        assert_eq!(root.resource(), "https://library.example/_mcp");
        assert_eq!(well_known_split("/.well-known/oauth-protected-resource/_mcp/library"), Some(("oauth-protected-resource", "/_mcp/library")));
        assert_eq!(well_known_split("/.well-known/oauth-protected-resource"), None);
        assert_eq!(well_known_split("/.well-known/oauth-protected-resourcex/a"), None);
    }

    #[test]
    fn a_named_host_is_https_and_the_dev_host_keeps_its_port() {
        let named = ProjectHost { host: "library.example".into(), owner: "site-a".into(), project: "demo".into(), dev_host: false, mount: Some("/_mcp".into()) };
        let mut headers = HeaderMap::new();
        headers.insert(header::HOST, HeaderValue::from_static("library.example:8080"));
        headers.insert("x-forwarded-proto", HeaderValue::from_static("http"));
        assert_eq!(arrival_base("site-a", "demo", Some(&named), &headers, None).unwrap().0, "https://library.example/_mcp", "a forwarded header changes nothing");
        let dev = ProjectHost { host: "demo.site-a.localhost".into(), dev_host: true, ..named };
        headers.insert(header::HOST, HeaderValue::from_static("demo.site-a.localhost:10610"));
        assert_eq!(arrival_base("site-a", "demo", Some(&dev), &headers, None).unwrap(), ("http://demo.site-a.localhost:10610/_mcp".into(), "http://demo.site-a.localhost:10610".into()));
        // The platform form: only from the configured base (D4).
        assert_eq!(arrival_base("site-a", "demo", None, &headers, None), None);
        let platform = arrival_base("site-a", "demo", None, &headers, Some("https://zebflow.example/")).unwrap();
        assert_eq!(platform, ("https://zebflow.example/mcp/site-a/demo".into(), "https://zebflow.example/wh/site-a/demo".into()));
        let issuer = Issuer::new(&platform.0, "/library", &platform.1).unwrap();
        assert_eq!(issuer.resource(), "https://zebflow.example/mcp/site-a/demo/library");
        assert_eq!(issuer.protected_resource_url(), "https://zebflow.example/.well-known/oauth-protected-resource/mcp/site-a/demo/library");
        assert_eq!(issuer.login_url("/auth/login"), "https://zebflow.example/wh/site-a/demo/auth/login");
    }
}
