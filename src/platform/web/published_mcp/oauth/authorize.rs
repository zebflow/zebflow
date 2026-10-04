//! `GET {R}/_oauth/authorize` — the browser's first stop. The client and its
//! redirect URI are checked before anything is redirected anywhere (a bad
//! one is a plain 400 page: never an open redirect); every later refusal
//! goes back to the client with `error`, `state` and `iss` (RFC 9207).
//! Success stores a ten-minute ticket and sends the person to the app's
//! `--login` page with `?oauth=<ticket>&client=<name>&redirect_host=<host>`
//! — the facts the page must show and `auth.oauth.approve` re-checks. No
//! cookie is read or set.

use std::collections::HashMap;

use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};

use super::metadata::SCOPES;
use super::{Ctx, cimd};
use crate::platform::services::published_oauth::{Ticket, now_unix, rules};

/// The page a person sees when the request cannot be sent back to its client.
fn page(message: &str) -> Response {
    let mut response = (StatusCode::BAD_REQUEST, format!("This sign-in request cannot continue: {message}\n")).into_response();
    response.headers_mut().insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}

fn see_other(location: &str) -> Response {
    let mut response = StatusCode::SEE_OTHER.into_response();
    if let Ok(value) = HeaderValue::from_str(location) {
        response.headers_mut().insert(header::LOCATION, value);
    }
    let headers = response.headers_mut();
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    headers.insert(header::REFERRER_POLICY, HeaderValue::from_static("no-referrer"));
    response
}

/// The query, each parameter once: a repeated one is refused (RFC 6749 §3.1).
fn parameters(query: &str) -> Result<HashMap<String, String>, String> {
    let mut out = HashMap::new();
    for (key, value) in serde_urlencoded::from_str::<Vec<(String, String)>>(query).map_err(|e| e.to_string())? {
        if out.insert(key.clone(), value).is_some() {
            return Err(format!("parameter '{key}' is given twice"));
        }
    }
    Ok(out)
}

/// The client's name and redirect URIs: a registration of this route, or a
/// metadata document.
async fn client_of(ctx: &Ctx<'_>, client_id: &str) -> Result<(String, Vec<String>), String> {
    let store = &ctx.state.platform.published_oauth;
    if cimd::is_document_id(client_id) {
        let client = cimd::resolve(store, client_id).await?;
        return Ok((client.client_name, client.redirect_uris));
    }
    match store.client(ctx.owner, ctx.project, ctx.route, client_id) {
        Ok(Some(client)) => Ok((client.client_name, client.redirect_uris)),
        Ok(None) => Err("this client is not registered with this server".to_string()),
        Err(err) => Err(format!("the registration could not be read: {}", err.0)),
    }
}

pub(super) async fn authorize(ctx: &Ctx<'_>, query: &str) -> Response {
    let params = match parameters(query) {
        Ok(params) => params,
        Err(message) => return page(&message),
    };
    let get = |key: &str| params.get(key).map(String::as_str).filter(|v| !v.is_empty());
    let Some(client_id) = get("client_id") else {
        return page("client_id is missing");
    };
    let (client_name, registered) = match client_of(ctx, client_id).await {
        Ok(found) => found,
        Err(message) => return page(&message),
    };
    let redirect_uri = match get("redirect_uri") {
        Some(uri) if registered.iter().any(|r| rules::redirect_matches(r, uri)) => uri.to_string(),
        Some(_) => return page("redirect_uri is not one this client registered"),
        None if registered.len() == 1 => registered[0].clone(),
        None => return page("redirect_uri is missing"),
    };

    // From here the client is known and its redirect checked: refusals go back to it.
    let issuer = ctx.issuer.resource();
    let state = get("state");
    let back = |error: &str, description: &str| {
        let mut pairs = vec![("error", error), ("error_description", description)];
        if let Some(state) = state {
            pairs.push(("state", state));
        }
        pairs.push(("iss", issuer.as_str()));
        see_other(&rules::with_query(&redirect_uri, &pairs))
    };
    if get("response_type") != Some("code") {
        return back("unsupported_response_type", "response_type must be code");
    }
    let challenge = match (get("code_challenge"), get("code_challenge_method")) {
        (Some(challenge), Some("S256")) if rules::valid_challenge(challenge) => challenge.to_string(),
        _ => return back("invalid_request", "PKCE is required: code_challenge with code_challenge_method S256"),
    };
    // RFC 8707: a token for this route only. Absent means this route.
    if let Some(resource) = get("resource")
        && resource.trim_end_matches('/') != issuer
    {
        return back("invalid_target", "resource is not this MCP server");
    }
    let requested: Vec<&str> = get("scope").unwrap_or("mcp").split_whitespace().collect();
    if requested.iter().any(|s| !SCOPES.contains(s)) {
        return back("invalid_scope", "scope may be mcp and offline_access");
    }
    let ticket = Ticket {
        route: ctx.route.to_string(),
        client_id: client_id.to_string(),
        client_name: client_name.clone(),
        redirect_host: rules::redirect_host(&redirect_uri),
        redirect_uri: redirect_uri.clone(),
        challenge,
        resource: issuer.clone(),
        issuer: issuer.clone(),
        scope: "mcp".to_string(),
        state: state.map(str::to_string),
        credential: ctx.spec.auth_credential.clone(),
        created_at: now_unix(),
    };
    match ctx.state.platform.published_oauth.open_ticket(ctx.owner, ctx.project, &ticket) {
        Ok(Some(secret)) => see_other(&rules::with_query(
            &ctx.issuer.login_url(&ctx.spec.login),
            &[("oauth", &secret), ("client", &client_name), ("redirect_host", &ticket.redirect_host)],
        )),
        Ok(None) => back("temporarily_unavailable", "too many sign-ins on this server just now; try again in a minute"),
        Err(err) => {
            eprintln!("warning: oauth ticket not stored for {}/{}: {}", ctx.owner, ctx.project, err.0);
            back("server_error", "the sign-in could not be stored; try again")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parameters;

    #[test]
    fn a_parameter_given_twice_is_refused() {
        assert_eq!(parameters("a=1&b=2").unwrap().len(), 2);
        assert!(parameters("client_id=a&client_id=b").is_err());
    }
}
