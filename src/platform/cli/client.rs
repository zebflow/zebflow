//! HTTP client for the running instance.
//!
//! `interface.md` §6 settles the transport: everything a server owns the state
//! for is reached over the same routes the web UI uses. An install touches an
//! in-memory node registry inside the live process, so a CLI that wrote to the
//! data directory behind it would leave the server believing something false
//! about its own nodes. Only the Group 3 maintenance commands, which exist to
//! repair a server that will not start, go direct.

use std::io;

use reqwest::header::{COOKIE, SET_COOKIE};
use serde_json::Value;

/// Cookie the platform issues a browser and accepts from any client.
const SESSION_COOKIE_NAME: &str = "zebflow_session";

/// An instance and the credential to speak to it with.
pub struct Instance {
    base: String,
    token: String,
    http: reqwest::Client,
}

impl Instance {
    pub fn new(base: &str, token: &str) -> Result<Self, io::Error> {
        Ok(Self {
            base: base.trim_end_matches('/').to_string(),
            token: token.to_string(),
            http: build_http_client()?,
        })
    }

    pub fn base_url(&self) -> &str {
        &self.base
    }

    /// Whether the instance answers its liveness route at all, which is the
    /// only part of `zeb status` that does not need a credential.
    pub async fn reachable(&self) -> bool {
        self.http
            .get(format!("{}/health", self.base))
            .send()
            .await
            .map(|response| response.status().is_success())
            .unwrap_or(false)
    }

    pub async fn get_json(&self, path: &str) -> Result<Value, io::Error> {
        let response = self
            .http
            .get(format!("{}{path}", self.base))
            .header(COOKIE, self.cookie())
            .send()
            .await
            .map_err(transport_error)?;
        read_json(response).await
    }

    pub async fn post_json(&self, path: &str, body: &Value) -> Result<Value, io::Error> {
        let response = self
            .http
            .post(format!("{}{path}", self.base))
            .header(COOKIE, self.cookie())
            .json(body)
            .send()
            .await
            .map_err(transport_error)?;
        read_json(response).await
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
        self.http
            .post(format!("{}/logout", self.base))
            .header(COOKIE, self.cookie())
            .send()
            .await
            .map(|response| response.status().is_success() || response.status().is_redirection())
            .unwrap_or(false)
    }

    /// `DELETE` with a body, which is how the platform takes the confirmation
    /// an irreversible delete requires.
    pub async fn delete_json(&self, path: &str, body: &Value) -> Result<Value, io::Error> {
        let response = self
            .http
            .delete(format!("{}{path}", self.base))
            .header(COOKIE, self.cookie())
            .json(body)
            .send()
            .await
            .map_err(transport_error)?;
        read_json(response).await
    }

    fn cookie(&self) -> String {
        format!("{SESSION_COOKIE_NAME}={}", self.token)
    }
}

/// Exchanges a password for a session token.
///
/// The token is what gets stored; the password is never written down. `POST
/// /login` answers 303 and sets the cookie on the redirect, so redirects are
/// not followed — following one would discard the header this call exists for.
pub async fn login(base: &str, identifier: &str, password: &str) -> Result<String, io::Error> {
    let http = build_http_client()?;
    let response = http
        .post(format!("{}/login", base.trim_end_matches('/')))
        .form(&[("identifier", identifier), ("password", password)])
        .send()
        .await
        .map_err(transport_error)?;

    let status = response.status();
    let token = response
        .headers()
        .get_all(SET_COOKIE)
        .iter()
        .filter_map(|value| value.to_str().ok())
        .find_map(session_token_from_set_cookie);

    match token {
        Some(token) if !token.is_empty() => Ok(token),
        // A wrong password re-renders the login page with 401 and sets no
        // cookie, so an absent cookie on a non-redirect status is a rejected
        // credential rather than a protocol surprise.
        _ if status.as_u16() == 401 => Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            "invalid credentials",
        )),
        _ => Err(io::Error::other(format!(
            "login did not return a session cookie (HTTP {status})"
        ))),
    }
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
async fn read_json(response: reqwest::Response) -> Result<Value, io::Error> {
    let status = response.status();
    let body = response.text().await.map_err(transport_error)?;
    let parsed = serde_json::from_str::<Value>(&body).ok();

    if status.is_success()
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

    let kind = match status.as_u16() {
        401 => io::ErrorKind::PermissionDenied,
        403 => io::ErrorKind::PermissionDenied,
        404 => io::ErrorKind::NotFound,
        _ => io::ErrorKind::Other,
    };
    Err(io::Error::new(
        kind,
        format!("HTTP {}: {message}", status.as_u16()),
    ))
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
