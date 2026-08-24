//! The client's transport to an instance, running or not.
//!
//! `interface.md` §6 settles it: **HTTP when a server owns the state, direct
//! when none does.** An install touches an in-memory node registry inside a
//! live process, so a CLI that wrote to the data directory behind a running
//! server would leave it believing something false about its own nodes. That
//! argument only holds while a server is running. With none, there is nothing
//! to keep coherent, and starting a background daemon so that the client has
//! something to talk to is a surprise nobody asked for.
//!
//! So this type carries two transports behind one surface. [`Transport::Http`]
//! reaches a listening instance over the same routes the web UI uses.
//! [`Transport::Local`] calls the very same router in this process, with no
//! socket bound and nothing left listening when the command exits — which is
//! also why it is not a loopback server: an ephemeral port would put `/login`
//! in front of every other process on the machine for the length of an install.
//!
//! The Group 3 maintenance commands still go straight at the data directory,
//! because they exist to repair a server that will not start.

use std::io;

use axum::Router;
use axum::body::Body;
use axum::http::header::{CONTENT_TYPE, COOKIE, SET_COOKIE};
use axum::http::{Method, Request};
use reqwest::header::{COOKIE as REQWEST_COOKIE, SET_COOKIE as REQWEST_SET_COOKIE};
use serde_json::Value;
use tower::ServiceExt as _;

/// Cookie the platform issues a browser and accepts from any client.
const SESSION_COOKIE_NAME: &str = "zebflow_session";

/// Response bodies larger than this are refused rather than buffered. The
/// review report is the largest thing the CLI reads and is far below it.
const MAX_LOCAL_BODY_BYTES: usize = 64 * 1024 * 1024;

/// How a request reaches the platform router.
enum Transport {
    /// A server owns the data root; speak to it over the network.
    Http(reqwest::Client),
    /// Nothing owns the data root, so this process opened it and holds the
    /// router itself. Requests are calls, not packets.
    Local(Box<Router>),
}

/// An instance and the credential to speak to it with.
pub struct Instance {
    base: String,
    token: String,
    transport: Transport,
}

impl Instance {
    pub fn new(base: &str, token: &str) -> Result<Self, io::Error> {
        Ok(Self {
            base: base.trim_end_matches('/').to_string(),
            token: token.to_string(),
            transport: Transport::Http(build_http_client()?),
        })
    }

    /// An instance this process is holding open itself.
    ///
    /// `base` is the URL a server here *would* listen on, so printed links and
    /// the "now serve it" hint name the address the project will answer at.
    pub fn local(base: &str, token: &str, router: Router) -> Self {
        Self {
            base: base.trim_end_matches('/').to_string(),
            token: token.to_string(),
            transport: Transport::Local(Box::new(router)),
        }
    }

    /// True when no server owns this data root and the command is doing the
    /// work in-process.
    pub fn is_local(&self) -> bool {
        matches!(self.transport, Transport::Local(_))
    }

    /// The same instance with a different credential, which is what a login
    /// performed mid-command produces.
    pub fn with_token(self, token: &str) -> Self {
        Self {
            token: token.to_string(),
            ..self
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base
    }

    /// Whether the instance answers its liveness route at all, which is the
    /// only part of `zeb status` that does not need a credential.
    pub async fn reachable(&self) -> bool {
        match &self.transport {
            // In-process: the router is right here, so there is nothing to ask.
            Transport::Local(_) => true,
            Transport::Http(http) => http
                .get(format!("{}/health", self.base))
                .send()
                .await
                .map(|response| response.status().is_success())
                .unwrap_or(false),
        }
    }

    pub async fn get_json(&self, path: &str) -> Result<Value, io::Error> {
        self.send(Method::GET, path, None).await
    }

    pub async fn post_json(&self, path: &str, body: &Value) -> Result<Value, io::Error> {
        self.send(Method::POST, path, Some(body)).await
    }

    /// Exchanges a password for a session token on this instance.
    ///
    /// One implementation for both transports, so an in-process install
    /// authenticates by exactly the route a person's `zeb login` uses rather
    /// than by a second mechanism that trusts something else.
    pub async fn authenticate(
        &self,
        identifier: &str,
        password: &str,
    ) -> Result<String, io::Error> {
        let form =
            serde_urlencoded::to_string([("identifier", identifier), ("password", password)])
                .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err.to_string()))?;
        let (status, cookies, _) = match &self.transport {
            Transport::Http(http) => {
                let response = http
                    .post(format!("{}/login", self.base))
                    .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(form)
                    .send()
                    .await
                    .map_err(transport_error)?;
                let status = response.status().as_u16();
                let cookies = response
                    .headers()
                    .get_all(REQWEST_SET_COOKIE)
                    .iter()
                    .filter_map(|value| value.to_str().ok().map(str::to_string))
                    .collect::<Vec<_>>();
                (status, cookies, String::new())
            }
            Transport::Local(router) => {
                let request = Request::builder()
                    .method(Method::POST)
                    .uri("/login")
                    .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
                    .body(Body::from(form))
                    .map_err(|err| io::Error::other(err.to_string()))?;
                let response = router
                    .as_ref()
                    .clone()
                    .oneshot(request)
                    .await
                    .map_err(|err| io::Error::other(err.to_string()))?;
                let status = response.status().as_u16();
                let cookies = response
                    .headers()
                    .get_all(SET_COOKIE)
                    .iter()
                    .filter_map(|value| value.to_str().ok().map(str::to_string))
                    .collect::<Vec<_>>();
                (status, cookies, String::new())
            }
        };

        match cookies
            .iter()
            .find_map(|raw| session_token_from_set_cookie(raw))
        {
            Some(token) if !token.is_empty() => Ok(token),
            // A wrong password re-renders the login page with 401 and sets no
            // cookie, so an absent cookie on a non-redirect status is a rejected
            // credential rather than a protocol surprise.
            _ if status == 401 => Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "invalid credentials",
            )),
            _ => Err(io::Error::other(format!(
                "login did not return a session cookie (HTTP {status})"
            ))),
        }
    }

    async fn send(
        &self,
        method: Method,
        path: &str,
        body: Option<&Value>,
    ) -> Result<Value, io::Error> {
        let (status, text) = match &self.transport {
            Transport::Http(http) => {
                let mut request = http
                    .request(method, format!("{}{path}", self.base))
                    .header(REQWEST_COOKIE, self.cookie());
                if let Some(body) = body {
                    request = request.json(body);
                }
                let response = request.send().await.map_err(transport_error)?;
                let status = response.status().as_u16();
                (status, response.text().await.map_err(transport_error)?)
            }
            Transport::Local(router) => {
                let mut builder = Request::builder()
                    .method(method)
                    .uri(path)
                    .header(COOKIE, self.cookie());
                if body.is_some() {
                    builder = builder.header(CONTENT_TYPE, "application/json");
                }
                let payload = match body {
                    Some(value) => Body::from(serde_json::to_vec(value).map_err(|err| {
                        io::Error::new(io::ErrorKind::InvalidData, err.to_string())
                    })?),
                    None => Body::empty(),
                };
                let request = builder
                    .body(payload)
                    .map_err(|err| io::Error::other(err.to_string()))?;
                let response = router
                    .as_ref()
                    .clone()
                    .oneshot(request)
                    .await
                    .map_err(|err| io::Error::other(err.to_string()))?;
                let status = response.status().as_u16();
                let bytes = axum::body::to_bytes(response.into_body(), MAX_LOCAL_BODY_BYTES)
                    .await
                    .map_err(|err| io::Error::other(err.to_string()))?;
                (status, String::from_utf8_lossy(&bytes).into_owned())
            }
        };
        read_json(status, &text)
    }

    /// Ends this token's session on the server.
    ///
    /// The route answers with a redirect rather than JSON, so the body is not
    /// read: the only question is whether the server accepted it. Returns
    /// `false` when the instance could not be reached, so the caller can say
    /// that the local context was forgotten while the session was not ended --
    /// a distinction that matters, because a token nobody revoked stays usable
    /// by anyone who already copied it.
    pub async fn end_session(&self) -> bool {
        match &self.transport {
            Transport::Http(http) => http
                .post(format!("{}/logout", self.base))
                .header(REQWEST_COOKIE, self.cookie())
                .send()
                .await
                .map(|response| {
                    response.status().is_success() || response.status().is_redirection()
                })
                .unwrap_or(false),
            // Nothing to revoke: an in-process instance dies with the command.
            Transport::Local(_) => true,
        }
    }

    /// `DELETE` with a body, which is how the platform takes the confirmation
    /// an irreversible delete requires.
    pub async fn delete_json(&self, path: &str, body: &Value) -> Result<Value, io::Error> {
        self.send(Method::DELETE, path, Some(body)).await
    }

    fn cookie(&self) -> String {
        format!("{SESSION_COOKIE_NAME}={}", self.token)
    }
}

/// Exchanges a password for a session token on a listening instance.
///
/// The token is what gets stored; the password is never written down. `POST
/// /login` answers 303 and sets the cookie on the redirect, so redirects are
/// not followed — following one would discard the header this call exists for.
pub async fn login(base: &str, identifier: &str, password: &str) -> Result<String, io::Error> {
    Instance::new(base, "")?
        .authenticate(identifier, password)
        .await
}

fn build_http_client() -> Result<reqwest::Client, io::Error> {
    reqwest::Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|err| io::Error::other(err.to_string()))
}

fn session_token_from_set_cookie(raw: &str) -> Option<String> {
    let first = raw.split(';').next()?.trim();
    let (name, value) = first.split_once('=')?;
    (name.trim() == SESSION_COOKIE_NAME).then(|| value.trim().to_string())
}

/// Turns a response into either its JSON body or the error the API reported.
///
/// The platform answers failures as `{"ok": false, "error": ...}` where the
/// error is sometimes a string and sometimes a `{code, message}` pair, so both
/// are unwrapped to the message a person should read.
fn read_json(status: u16, body: &str) -> Result<Value, io::Error> {
    let parsed = serde_json::from_str::<Value>(body).ok();
    let success = (200..300).contains(&status);

    if success
        && let Some(value) = parsed.as_ref()
        && value.get("ok").and_then(Value::as_bool) != Some(false)
    {
        return Ok(parsed.expect("checked above"));
    }

    let message = parsed
        .as_ref()
        .and_then(|value| value.get("error"))
        .map(describe_api_error)
        .unwrap_or_else(|| body.trim().chars().take(300).collect());

    let kind = match status {
        401 => io::ErrorKind::PermissionDenied,
        403 => io::ErrorKind::PermissionDenied,
        404 => io::ErrorKind::NotFound,
        _ => io::ErrorKind::Other,
    };
    Err(io::Error::new(kind, format!("HTTP {status}: {message}")))
}

fn describe_api_error(error: &Value) -> String {
    if let Some(text) = error.as_str() {
        return text.to_string();
    }
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .unwrap_or("request failed");
    match error.get("code").and_then(Value::as_str) {
        Some(code) if !code.is_empty() => format!("{message} ({code})"),
        _ => message.to_string(),
    }
}

fn transport_error(err: reqwest::Error) -> io::Error {
    io::Error::new(io::ErrorKind::ConnectionRefused, err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_token_is_read_from_the_platform_cookie_only() {
        assert_eq!(
            session_token_from_set_cookie("zebflow_session=abc123; Path=/; HttpOnly"),
            Some("abc123".to_string())
        );
        assert_eq!(session_token_from_set_cookie("other=abc123; Path=/"), None);
    }

    #[test]
    fn api_errors_read_as_message_and_code() {
        assert_eq!(
            describe_api_error(&serde_json::json!("login required")),
            "login required"
        );
        assert_eq!(
            describe_api_error(
                &serde_json::json!({"code": "HUB_ASSET_MISSING", "message": "no such package"})
            ),
            "no such package (HUB_ASSET_MISSING)"
        );
    }
}
