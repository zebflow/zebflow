//! The door of a published route (`published-mcp.md`, Rules): the app's own
//! auth and nothing of Zebflow's.
//!
//! - Only the app's mechanisms are read: `X-API-Key` / `Authorization: ApiKey`
//!   for `api_key`, `Authorization: Bearer` for `jwt` and `oauth` (an access
//!   token this route issued, [`super::oauth`]). Cookies are never read
//!   — not the Studio's session, not a credential's `cookie_name` — so no
//!   platform account, Studio session, dev MCP token or member role is ever
//!   consulted. A dev MCP token presented as a bearer is just a JWT that does
//!   not verify.
//! - A missing, wrong or expired credential is one uniform 401; a valid one
//!   without the role is one uniform 403. Nothing says which. On an `oauth`
//!   route the 401 also carries `WWW-Authenticate: Bearer
//!   resource_metadata="…", scope="mcp"` (and `error="invalid_token"` when a
//!   token was sent), which is how an MCP client finds where to sign in.
//! - Every refusal is recorded on the run log and the error groups of the
//!   tool the request was for, with the reason and never the presented key.
//! - Refusals are counted per client address and route: [`MAX_FAILURES`] in
//!   [`WINDOW`] and the route answers 429 to that address until the window
//!   has passed.

use std::collections::HashMap;
use std::net::{IpAddr, SocketAddr};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::Json;
use axum::body::Bytes;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};

use super::PublishedTool;
use super::oauth::{AccessRefusal, Issuer, verify_access};
use crate::pipeline::nodes::basic::trigger::mcp_trigger;
use crate::platform::web::{AuthError, PlatformAppState, token_roles, verify_webhook_auth};

/// Refusals one client address may collect on one route within [`WINDOW`].
pub const MAX_FAILURES: u32 = 20;
/// How long refusals are counted, and how long a client that ran out waits.
pub const WINDOW: Duration = Duration::from_secs(60);
/// Entries kept before expired ones are swept.
const SWEEP_AT: usize = 10_000;

/// Refusals per `(route, client address)`, in memory: a restart forgets them,
/// which costs an attacker one restart's worth of guesses at most.
#[derive(Default)]
pub struct FailureLimiter {
    entries: Mutex<HashMap<String, (Instant, u32)>>,
}

impl FailureLimiter {
    /// How long `key` must still wait, when it has run out of refusals.
    pub(super) fn blocked(&self, key: &str) -> Option<Duration> {
        let entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        let (start, count) = entries.get(key)?;
        let elapsed = start.elapsed();
        (*count >= MAX_FAILURES && elapsed < WINDOW).then(|| WINDOW - elapsed)
    }

    pub(super) fn fail(&self, key: &str) {
        let mut entries = self.entries.lock().unwrap_or_else(|e| e.into_inner());
        if entries.len() >= SWEEP_AT {
            entries.retain(|_, (start, _)| start.elapsed() < WINDOW);
        }
        let entry = entries.entry(key.to_string()).or_insert((Instant::now(), 0));
        if entry.0.elapsed() >= WINDOW {
            *entry = (Instant::now(), 0);
        }
        entry.1 += 1;
    }
}

/// The limiter's key: one count per route of one project and client address.
pub(super) fn limit_key(owner: &str, project: &str, route: &str, client: &str) -> String {
    format!("{owner}/{project}{route}|{client}")
}

/// The client's address: the peer, or — when the peer is a proxy in front of
/// this office (this machine, or an address on a private or link-local
/// network: the ingress of a cluster) — the last `X-Forwarded-For` entry, the
/// one that proxy appended. Earlier entries are the client's own words and
/// never trusted; a public peer's header is never read at all.
pub(super) fn client_address(peer: Option<SocketAddr>, headers: &HeaderMap) -> String {
    let proxied = peer.is_none_or(|p| is_proxy_address(p.ip()));
    let forwarded = headers
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.rsplit(',').next())
        .and_then(|v| v.trim().parse::<IpAddr>().ok());
    match (proxied, forwarded, peer) {
        (true, Some(ip), _) => ip.to_string(),
        (_, _, Some(peer)) => peer.ip().to_string(),
        _ => "local".to_string(),
    }
}

/// An address only a proxy of this office's own network can have: loopback,
/// RFC 1918 (`10/8`, `172.16/12`, `192.168/16`), link-local (`169.254/16`,
/// `fe80::/10`) and RFC 4193 (`fc00::/7`). A constant rule: no setting widens it.
fn is_proxy_address(ip: IpAddr) -> bool {
    match ip.to_canonical() {
        IpAddr::V4(v4) => v4.is_loopback() || v4.is_private() || v4.is_link_local(),
        IpAddr::V6(v6) => {
            let first = v6.segments()[0];
            v6.is_loopback() || (first & 0xfe00) == 0xfc00 || (first & 0xffc0) == 0xfe80
        }
    }
}

/// The only headers the app's auth reads; everything else — cookies first —
/// never reaches the verifier.
fn app_credentials(headers: &HeaderMap) -> HeaderMap {
    let mut kept = HeaderMap::new();
    for name in [header::AUTHORIZATION, header::HeaderName::from_static("x-api-key")] {
        if let Some(value) = headers.get(&name) {
            kept.insert(name, value.clone());
        }
    }
    kept
}

fn uniform(status: StatusCode, code: &str, message: &str) -> Response {
    (status, Json(json!({ "ok": false, "error": { "code": code, "message": message } }))).into_response()
}

/// Missing, wrong or expired: one answer, so the answer says nothing.
fn unauthenticated() -> Response {
    uniform(StatusCode::UNAUTHORIZED, "unauthenticated", "a valid credential for this MCP server is required")
}

fn forbidden() -> Response {
    uniform(StatusCode::FORBIDDEN, "forbidden", "this credential may not use this MCP server")
}

pub(super) fn too_many(wait: Duration) -> Response {
    let mut response = uniform(StatusCode::TOO_MANY_REQUESTS, "too_many_failures", "too many refused attempts; wait before trying again");
    if let Ok(value) = HeaderValue::from_str(&wait.as_secs().max(1).to_string()) {
        response.headers_mut().insert(header::RETRY_AFTER, value);
    }
    response
}

/// Everything about one request at the door.
pub(super) struct Door<'a> {
    pub state: &'a PlatformAppState,
    pub owner: &'a str,
    pub project: &'a str,
    pub route: &'a str,
    pub tools: &'a [PublishedTool],
    pub headers: &'a HeaderMap,
    pub body: &'a Bytes,
    /// The route's address, on an `oauth` route.
    pub issuer: Option<&'a Issuer>,
    /// The limiter key this server counts refusals under; `None` on a worker
    /// answering its controller, which counts them where the client called.
    pub limit: Option<&'a str>,
}

impl Door<'_> {
    /// Admits the request, or answers the refusal. The caller has checked
    /// that the route has tools and that they declare the same guard.
    pub(super) fn check(&self) -> Result<(), Response> {
        let (auth_type, credential, roles, _) = self.tools[0].spec.guard();
        if auth_type == "none" {
            return Ok(());
        }
        let verified = if auth_type == "oauth" {
            self.oauth(&credential, &roles)
        } else {
            verify_webhook_auth(
                &app_credentials(self.headers),
                self.body,
                &auth_type,
                &credential,
                &roles,
                &self.state.platform.credentials,
                self.owner,
                self.project,
            )
        };
        let (response, reason, counted) = match verified {
            Ok(_) => return Ok(()),
            Err(AuthError::Unauthenticated { message, .. }) => (self.challenge(unauthenticated()), message, true),
            Err(AuthError::Forbidden { message, .. }) => (forbidden(), message, true),
            // The operator's mistake (a credential gone or of the wrong
            // shape): the caller learns only that it failed, the record why.
            // Not the caller's refusal, so not counted against them.
            Err(AuthError::Internal(message)) => (
                uniform(StatusCode::INTERNAL_SERVER_ERROR, "internal", "this MCP server cannot check credentials; its owner can see why"),
                message,
                false,
            ),
        };
        if counted && let Some(key) = self.limit {
            self.state.mcp_failures.fail(key);
        }
        self.record(&reason);
        Err(response)
    }

    /// An `oauth` route's door: an access token this route issued, in the
    /// `Authorization` header only, then `--role` against its `roles`.
    fn oauth(&self, credential: &str, roles: &[String]) -> Result<Option<Value>, AuthError> {
        let Some(issuer) = self.issuer else {
            return Err(AuthError::Internal("this oauth route cannot name its own address on this arrival".to_string()));
        };
        let token = self
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "))
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .ok_or_else(|| AuthError::Unauthenticated { message: "missing Authorization: Bearer <access token>".to_string(), redirect_url: None })?;
        let stored = self
            .state
            .platform
            .credentials
            .get_project_credential(self.owner, self.project, credential)
            .map_err(|e| AuthError::Internal(e.message))?
            .filter(|c| c.kind == "jwt_signing_key")
            .ok_or_else(|| AuthError::Internal(format!("--credential '{credential}' is not a jwt_signing_key of this project")))?;
        let claims = verify_access(&stored.secret, token, &issuer.resource()).map_err(|refusal| match refusal {
            AccessRefusal::Token(message) => AuthError::Unauthenticated { message, redirect_url: None },
            AccessRefusal::Key(message) => AuthError::Internal(message),
        })?;
        if !roles.is_empty() {
            let held = token_roles(claims.get("roles"));
            if !roles.iter().any(|r| held.contains(r)) {
                return Err(AuthError::Forbidden { message: format!("roles {held:?} are not permitted for this route"), redirect_url: None });
            }
        }
        Ok(Some(claims))
    }

    /// The 401 of an `oauth` route says where to sign in (RFC 9728 §5.1).
    fn challenge(&self, mut response: Response) -> Response {
        let Some(issuer) = self.issuer.filter(|_| self.tools[0].spec.auth_type == "oauth") else {
            return response;
        };
        let presented = self.headers.get(header::AUTHORIZATION).is_some();
        let mut value = format!("Bearer resource_metadata=\"{}\", scope=\"mcp\"", issuer.protected_resource_url());
        if presented {
            value.push_str(", error=\"invalid_token\"");
        }
        if let Ok(value) = HeaderValue::from_str(&value) {
            response.headers_mut().insert(header::WWW_AUTHENTICATE, value);
        }
        response
    }

    /// The refusal on the run log and the error groups of the tool the
    /// request named (`tools/call`), else of the route's first tool. The
    /// reason is the verifier's sentence, which never quotes the key.
    fn record(&self, reason: &str) {
        let wanted = serde_json::from_slice::<Value>(self.body)
            .ok()
            .filter(|m| m["method"] == "tools/call")
            .and_then(|m| m["params"]["name"].as_str().map(str::to_string));
        let tool = wanted
            .and_then(|name| self.tools.iter().find(|t| t.spec.tool_name == name))
            .unwrap_or(&self.tools[0]);
        let platform = &self.state.platform;
        let file = &tool.compiled.file_rel_path;
        let message = format!("refused at the door of MCP route {}: {reason}", self.route);
        let entry = crate::platform::model::PipelineInvocationEntry {
            run_id: uuid::Uuid::new_v4().simple().to_string(),
            at: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64,
            duration_ms: 0,
            status: "error".to_string(),
            trigger: "mcp".to_string(),
            error: Some(message.clone()),
            trace: vec![crate::pipeline::model::NodeTraceEntry {
                node_id: tool.spec.node_id.clone(),
                node_kind: mcp_trigger::NODE_KIND.to_string(),
                config: None,
                duration_ms: 0,
                input: Value::Null,
                output: Value::Null,
                error: Some(message.clone()),
                status: "refused".to_string(),
                error_code: Some(mcp_trigger::AUTH_CODE.to_string()),
                preview_snapshot: None,
            }],
        };
        let cfg = platform.zebflow_cfg.read_or_default(self.owner, self.project).unwrap_or_default();
        let retention = crate::platform::model::resolve_invocation_retention(&cfg, Some(&tool.compiled.graph));
        platform.pipeline_hits.record_failure(self.owner, self.project, file, "mcp.door", mcp_trigger::AUTH_CODE, &message);
        let _ = platform.data.log_pipeline_invocation(self.owner, self.project, file, &entry, retention.max_invocations, retention.max_age_secs);
        let bounds = cfg.configs.pipelines.logging.error_group_bounds();
        if let Err(e) = platform.data.record_pipeline_error(self.owner, self.project, file, &entry, bounds) {
            eprintln!("warning: error group not recorded for {}/{}: {}", self.owner, self.project, e.message);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_client_runs_out_of_refusals_and_waits_out_the_window() {
        let limiter = FailureLimiter::default();
        for _ in 0..MAX_FAILURES {
            assert!(limiter.blocked("r|a").is_none());
            limiter.fail("r|a");
        }
        assert!(limiter.blocked("r|a").is_some(), "out of refusals");
        assert!(limiter.blocked("r|b").is_none(), "another address is another count");
        // A window that has passed starts again.
        limiter.entries.lock().unwrap().get_mut("r|a").unwrap().0 = Instant::now() - WINDOW;
        assert!(limiter.blocked("r|a").is_none());
        limiter.fail("r|a");
        assert_eq!(limiter.entries.lock().unwrap()["r|a"].1, 1);
    }

    #[test]
    fn only_a_proxy_on_this_offices_network_names_the_client() {
        let mut headers = HeaderMap::new();
        headers.insert("x-forwarded-for", HeaderValue::from_static("198.51.100.7, 203.0.113.9"));
        let remote: SocketAddr = "192.0.2.4:40000".parse().unwrap();
        // This machine, a cluster ingress on a private or link-local network:
        // the entry that proxy appended, so two clients behind it are two counts.
        for proxy in ["127.0.0.1:40000", "10.0.0.7:40000", "172.20.1.2:40000", "192.168.1.5:40000", "169.254.3.4:40000", "[fd00::5]:40000", "[fe80::1]:40000", "[::1]:40000", "[::ffff:10.0.0.9]:40000"] {
            let peer: SocketAddr = proxy.parse().unwrap();
            assert_eq!(client_address(Some(peer), &headers), "203.0.113.9", "behind {proxy}");
        }
        let mut other = HeaderMap::new();
        other.insert("x-forwarded-for", HeaderValue::from_static("203.0.113.10"));
        let ingress: SocketAddr = "10.0.0.7:40000".parse().unwrap();
        assert_ne!(client_address(Some(ingress), &headers), client_address(Some(ingress), &other), "one guesser does not lock out the next client");
        // A public peer cannot name another address, nor can a public address
        // that merely looks close to a private range.
        assert_eq!(client_address(Some(remote), &headers), "192.0.2.4", "a remote peer cannot name another address");
        for public in ["172.32.0.1:40000", "11.0.0.1:40000", "[2001:db8::1]:40000"] {
            let peer: SocketAddr = public.parse().unwrap();
            assert_eq!(client_address(Some(peer), &headers), peer.ip().to_string(), "{public} is not a proxy");
        }
        assert_eq!(client_address(Some(remote), &HeaderMap::new()), "192.0.2.4");
        assert_eq!(client_address(Some(ingress), &HeaderMap::new()), "10.0.0.7", "a proxy that adds no header is the client");
        assert_eq!(client_address(None, &HeaderMap::new()), "local");
    }

    #[test]
    fn the_verifier_never_sees_a_cookie() {
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, HeaderValue::from_static("zebflow_session=s"));
        headers.insert("x-api-key", HeaderValue::from_static("k"));
        headers.insert(header::AUTHORIZATION, HeaderValue::from_static("Bearer t"));
        headers.insert("x-zebflow-mcp-session", HeaderValue::from_static("d"));
        let kept = app_credentials(&headers);
        assert!(kept.get(header::COOKIE).is_none());
        assert_eq!(kept.len(), 2);
    }

    #[test]
    fn missing_wrong_and_expired_answer_alike() {
        let a = unauthenticated();
        let b = unauthenticated();
        assert_eq!(a.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(a.headers(), b.headers());
    }
}
