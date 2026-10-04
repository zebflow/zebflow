//! `POST {R}/_oauth/token` (form-encoded) — a code for tokens, or a refresh
//! token for new ones.
//!
//! - `authorization_code`: the code is spent first (a second presentation
//!   revokes what the first was exchanged for), then it must have been
//!   issued to this `client_id`, for this exact `redirect_uri` and this
//!   route, and `BASE64URL(SHA256(code_verifier))` must be its challenge.
//! - `refresh_token`: spent and replaced (rotation); a spent one presented
//!   again revokes its family. The family ends 30 days after sign-in.
//!
//! Public clients only: there is no client secret, so a request that
//! authenticates the client any other way is `invalid_client`. Every
//! refusal counts on the route's refusal limit.

use std::collections::HashMap;

use axum::body::Bytes;
use axum::http::{HeaderMap, StatusCode, header};
use axum::response::Response;
use serde_json::{Value, json};

use super::{Ctx, access, json_answer, oauth_error};
use crate::platform::services::published_oauth::{ACCESS_TTL, REFRESH_FAMILY_TTL, Refresh, Spent, now_unix, rules};

fn form(body: &Bytes) -> Result<HashMap<String, String>, String> {
    let mut out = HashMap::new();
    for (key, value) in serde_urlencoded::from_bytes::<Vec<(String, String)>>(body).map_err(|e| e.to_string())? {
        if out.insert(key.clone(), value).is_some() {
            return Err(format!("parameter '{key}' is given twice"));
        }
    }
    Ok(out)
}

pub(super) fn token(ctx: &Ctx<'_>, headers: &HeaderMap, body: &Bytes) -> Response {
    let refuse = |status: StatusCode, error: &str, description: &str| {
        ctx.refused();
        oauth_error(status, error, description)
    };
    if headers.contains_key(header::AUTHORIZATION) {
        return refuse(StatusCode::UNAUTHORIZED, "invalid_client", "clients of this server are public: no client authentication, PKCE instead");
    }
    let params = match form(body) {
        Ok(params) => params,
        Err(message) => return refuse(StatusCode::BAD_REQUEST, "invalid_request", &message),
    };
    let get = |key: &str| params.get(key).map(String::as_str).filter(|v| !v.is_empty());
    if params.contains_key("client_secret") {
        return refuse(StatusCode::UNAUTHORIZED, "invalid_client", "clients of this server are public: no client secret");
    }
    let Some(client_id) = get("client_id") else {
        return refuse(StatusCode::BAD_REQUEST, "invalid_request", "client_id is required");
    };
    let resource = ctx.issuer.resource();
    if let Some(asked) = get("resource")
        && asked.trim_end_matches('/') != resource
    {
        return refuse(StatusCode::BAD_REQUEST, "invalid_target", "resource is not this MCP server");
    }
    let store = &ctx.state.platform.published_oauth;
    let (owner, project, route) = (ctx.owner, ctx.project, ctx.route);
    let unavailable = |err: crate::platform::services::published_oauth::StoreError| {
        eprintln!("warning: oauth state not reachable for {owner}/{project}: {}", err.0);
        oauth_error(StatusCode::SERVICE_UNAVAILABLE, "temporarily_unavailable", "try again")
    };
    let refresh = match get("grant_type") {
        Some("authorization_code") => {
            let (Some(code), Some(redirect_uri), Some(verifier)) = (get("code"), get("redirect_uri"), get("code_verifier")) else {
                return refuse(StatusCode::BAD_REQUEST, "invalid_request", "code, redirect_uri and code_verifier are required");
            };
            let code = match store.take_code(owner, project, route, code) {
                Ok(Spent::Fresh(code)) => code,
                Ok(Spent::Refused) => return refuse(StatusCode::BAD_REQUEST, "invalid_grant", "the code is unknown, expired or already used"),
                Err(err) => return unavailable(err),
            };
            if code.client_id != client_id || code.redirect_uri != redirect_uri || code.resource != resource || code.issuer != resource {
                return refuse(StatusCode::BAD_REQUEST, "invalid_grant", "the code was not issued for this client, redirect_uri or server");
            }
            if !rules::pkce_matches(verifier, &code.challenge) {
                return refuse(StatusCode::BAD_REQUEST, "invalid_grant", "code_verifier does not match the code_challenge");
            }
            Refresh {
                family: code.family,
                client_id: code.client_id,
                resource: code.resource,
                issuer: code.issuer,
                scope: code.scope,
                claims: code.claims,
                family_expires_at: now_unix() + REFRESH_FAMILY_TTL as i64,
            }
        }
        Some("refresh_token") => {
            let Some(presented) = get("refresh_token") else {
                return refuse(StatusCode::BAD_REQUEST, "invalid_request", "refresh_token is required");
            };
            let refresh = match store.rotate_refresh(owner, project, route, presented) {
                Ok(Spent::Fresh(refresh)) => refresh,
                Ok(Spent::Refused) => return refuse(StatusCode::BAD_REQUEST, "invalid_grant", "the refresh token is unknown, expired, revoked or already used"),
                Err(err) => return unavailable(err),
            };
            if refresh.client_id != client_id || refresh.resource != resource || refresh.issuer != resource {
                return refuse(StatusCode::BAD_REQUEST, "invalid_grant", "the refresh token was not issued for this client or server");
            }
            refresh
        }
        Some(_) => return refuse(StatusCode::BAD_REQUEST, "unsupported_grant_type", "grant_type may be authorization_code or refresh_token"),
        None => return refuse(StatusCode::BAD_REQUEST, "invalid_request", "grant_type is required"),
    };
    issue(ctx, &refresh)
}

/// The access token and the family's next refresh token.
fn issue(ctx: &Ctx<'_>, refresh: &Refresh) -> Response {
    let credential = ctx
        .state
        .platform
        .credentials
        .get_project_credential(ctx.owner, ctx.project, &ctx.spec.auth_credential)
        .ok()
        .flatten()
        .filter(|c| c.kind == "jwt_signing_key");
    let Some(credential) = credential else {
        eprintln!("warning: MCP route {} of {}/{}: --credential '{}' is not a jwt_signing_key", ctx.route, ctx.owner, ctx.project, ctx.spec.auth_credential);
        return oauth_error(StatusCode::INTERNAL_SERVER_ERROR, "server_error", "this server cannot sign tokens; its owner can see why");
    };
    let access_token = match access::mint(&credential.secret, &refresh.claims, &refresh.issuer, &refresh.client_id, &refresh.scope) {
        Ok(token) => token,
        Err(reason) => {
            eprintln!("warning: MCP route {} of {}/{}: access token not signed: {reason}", ctx.route, ctx.owner, ctx.project);
            return oauth_error(StatusCode::INTERNAL_SERVER_ERROR, "server_error", "this server cannot sign tokens; its owner can see why");
        }
    };
    let store = &ctx.state.platform.published_oauth;
    let refresh_token = match store.issue_refresh(ctx.owner, ctx.project, ctx.route, refresh) {
        Ok(token) => token,
        Err(err) => {
            eprintln!("warning: oauth refresh token not stored for {}/{}: {}", ctx.owner, ctx.project, err.0);
            return oauth_error(StatusCode::SERVICE_UNAVAILABLE, "temporarily_unavailable", "try again");
        }
    };
    store.renew_client(ctx.owner, ctx.project, ctx.route, &refresh.client_id);
    let answer: Value = json!({
        "access_token": access_token,
        "token_type": "Bearer",
        "expires_in": ACCESS_TTL,
        "refresh_token": refresh_token,
        "scope": refresh.scope,
    });
    json_answer(StatusCode::OK, answer)
}
