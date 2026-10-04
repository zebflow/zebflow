//! RFC 7591 dynamic client registration — deprecated by the MCP spec in
//! favour of client ID metadata documents, still what claude.ai, the
//! TypeScript SDK and most CLIs do. Public clients only (`none`), PKCE
//! always; a route takes [`MAX_REGISTRATIONS_PER_DAY`] a day.

use axum::body::Bytes;
use axum::http::StatusCode;
use axum::response::Response;
use serde::Deserialize;
use serde_json::json;

use super::{Ctx, json_answer, oauth_error};
use crate::platform::services::published_oauth::{Client, MAX_REGISTRATIONS_PER_DAY, now_unix, rules};

const GRANTS: [&str; 2] = ["authorization_code", "refresh_token"];

#[derive(Debug, Deserialize)]
struct Request {
    #[serde(default)]
    redirect_uris: Vec<String>,
    #[serde(default)]
    client_name: Option<String>,
    #[serde(default)]
    grant_types: Option<Vec<String>>,
    #[serde(default)]
    response_types: Option<Vec<String>>,
    #[serde(default)]
    token_endpoint_auth_method: Option<String>,
}

/// The registration a request asks for, or the RFC 7591 §3.2.2 refusal.
fn read(body: &Bytes) -> Result<Client, (&'static str, String)> {
    let request: Request = serde_json::from_slice(body).map_err(|e| ("invalid_client_metadata", format!("the body is not client metadata JSON: {e}")))?;
    if request.redirect_uris.is_empty() || request.redirect_uris.len() > rules::MAX_REDIRECT_URIS {
        return Err(("invalid_redirect_uri", format!("redirect_uris needs 1 to {} URIs", rules::MAX_REDIRECT_URIS)));
    }
    for uri in &request.redirect_uris {
        rules::acceptable_redirect(uri).map_err(|m| ("invalid_redirect_uri", m))?;
    }
    if let Some(method) = request.token_endpoint_auth_method.as_deref()
        && method != "none"
    {
        return Err(("invalid_client_metadata", format!("token_endpoint_auth_method '{method}' is not offered; public clients use none, with PKCE")));
    }
    let grant_types = request.grant_types.unwrap_or_else(|| vec!["authorization_code".into(), "refresh_token".into()]);
    if grant_types.is_empty() || grant_types.iter().any(|g| !GRANTS.contains(&g.as_str())) {
        return Err(("invalid_client_metadata", "grant_types may be authorization_code and refresh_token".to_string()));
    }
    if request.response_types.is_some_and(|types| types.iter().any(|t| t != "code")) {
        return Err(("invalid_client_metadata", "response_types may only be code".to_string()));
    }
    let name = request.client_name.unwrap_or_default();
    let name: String = name.trim().chars().filter(|c| !c.is_control()).take(rules::MAX_CLIENT_NAME).collect();
    Ok(Client {
        client_id: rules::new_client_id(),
        client_name: if name.is_empty() { "MCP client".to_string() } else { name },
        redirect_uris: request.redirect_uris,
        grant_types,
        created_at: now_unix(),
    })
}

pub(super) fn register(ctx: &Ctx<'_>, body: &Bytes) -> Response {
    let client = match read(body) {
        Ok(client) => client,
        Err((error, description)) => {
            ctx.refused();
            return oauth_error(StatusCode::BAD_REQUEST, error, &description);
        }
    };
    match ctx.state.platform.published_oauth.register_client(ctx.owner, ctx.project, ctx.route, &client) {
        Ok(true) => json_answer(
            StatusCode::CREATED,
            json!({
                "client_id": client.client_id,
                "client_id_issued_at": client.created_at,
                "client_name": client.client_name,
                "redirect_uris": client.redirect_uris,
                "grant_types": client.grant_types,
                "response_types": ["code"],
                "token_endpoint_auth_method": "none",
            }),
        ),
        Ok(false) => oauth_error(
            StatusCode::TOO_MANY_REQUESTS,
            "temporarily_unavailable",
            &format!("this server registers {MAX_REGISTRATIONS_PER_DAY} clients a day; try again tomorrow"),
        ),
        Err(err) => {
            eprintln!("warning: oauth registration not stored for {}/{}: {}", ctx.owner, ctx.project, err.0);
            oauth_error(StatusCode::SERVICE_UNAVAILABLE, "temporarily_unavailable", "the registration could not be stored; try again")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body(value: serde_json::Value) -> Bytes {
        Bytes::from(value.to_string())
    }

    #[test]
    fn a_public_client_registers_and_anything_else_is_named() {
        let client = read(&body(json!({ "redirect_uris": ["https://client.example/cb"], "client_name": "Reader\napp" }))).unwrap();
        assert!(rules::is_issued_client_id(&client.client_id));
        assert_eq!(client.client_name, "Readerapp");
        assert_eq!(client.grant_types, vec!["authorization_code", "refresh_token"]);
        for (value, error) in [
            (json!({ "redirect_uris": [] }), "invalid_redirect_uri"),
            (json!({ "redirect_uris": ["http://client.example/cb"] }), "invalid_redirect_uri"),
            (json!({ "redirect_uris": ["https://client.example/cb#x"] }), "invalid_redirect_uri"),
            (json!({ "redirect_uris": ["https://client.example/cb"], "token_endpoint_auth_method": "client_secret_basic" }), "invalid_client_metadata"),
            (json!({ "redirect_uris": ["https://client.example/cb"], "grant_types": ["implicit"] }), "invalid_client_metadata"),
            (json!({ "redirect_uris": ["https://client.example/cb"], "response_types": ["token"] }), "invalid_client_metadata"),
        ] {
            assert_eq!(read(&body(value.clone())).unwrap_err().0, error, "{value}");
        }
        let many: Vec<String> = (0..11).map(|i| format!("https://client.example/{i}")).collect();
        assert_eq!(read(&body(json!({ "redirect_uris": many }))).unwrap_err().0, "invalid_redirect_uri");
    }
}
