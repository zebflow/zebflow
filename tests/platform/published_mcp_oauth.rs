//! `published-mcp.md` § `--auth oauth`: a published route signs people in
//! through the app's own login page and hands the MCP client the app's
//! tokens. An invented app — a reading list — publishes `/library` with
//! `--auth oauth --login /auth/login --role reader`; its login pipeline
//! signs the app's token and ends in `auth.oauth.approve`.
//!
//! Proven here, over HTTP on the project's dev host: the discovery
//! documents and the 401 challenge, registration, authorize → login →
//! approve → code → token (PKCE) → tools/list and tools/call; every refusal
//! (verifier, reused code, redirect, ticket, doctored consent, foreign
//! token, role); refresh rotation; and that no Zebflow cookie is set, read
//! or accepted anywhere in the flow.

use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, Request, StatusCode, header};
use serde_json::{Value, json};
use tower::ServiceExt;

use super::published_mcp::{OWNER, PROJECT, addressing, app_at, call, initialize, list, login, publish, send, temp_dir, text_of, tool_names};

const HOST: &str = "default.superadmin.localhost";
const R: &str = "http://default.superadmin.localhost/_mcp/library";
const REDIRECT: &str = "https://client.example/oauth/callback";
// RFC 7636 Appendix B.
const VERIFIER: &str = "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk";
const CHALLENGE: &str = "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM";

struct Answer {
    status: StatusCode,
    headers: HeaderMap,
    body: String,
}

impl Answer {
    fn json(&self) -> Value {
        serde_json::from_str(&self.body).unwrap_or(Value::Null)
    }
    fn location(&self) -> String {
        self.headers.get(header::LOCATION).and_then(|v| v.to_str().ok()).unwrap_or_default().to_string()
    }
    fn header(&self, name: header::HeaderName) -> String {
        self.headers.get(name).and_then(|v| v.to_str().ok()).unwrap_or_default().to_string()
    }
}

/// One request on the project's dev host.
async fn http(app: &axum::Router, method: &str, uri: &str, headers: &[(&str, &str)], body: Body) -> Answer {
    let mut builder = Request::builder().uri(uri).method(method).header(header::HOST, HOST);
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    let response = app.clone().oneshot(builder.body(body).expect("request")).await.expect("response");
    let (parts, body) = response.into_parts();
    let body = String::from_utf8_lossy(&to_bytes(body, usize::MAX).await.expect("body")).to_string();
    // No step of the flow ever sets a cookie — the platform's or any other.
    assert!(parts.headers.get(header::SET_COOKIE).is_none(), "{method} {uri} set a cookie");
    Answer { status: parts.status, headers: parts.headers, body }
}

fn query_of(url: &str) -> std::collections::HashMap<String, String> {
    let query = url.split_once('?').map(|(_, q)| q).unwrap_or_default();
    serde_urlencoded::from_str(query).expect("query")
}

/// The app: its signing key, the surface on, `/library` and `/other` behind
/// oauth, a `jwt` webhook on the same key, and the login pipeline — whose
/// `:error` shows "this link has expired" with the failure's code.
async fn library_app(app: &axum::Router, cookie: &str) -> String {
    let secret = uuid::Uuid::new_v4().simple().to_string();
    let (status, body) = send(
        app,
        cookie,
        "POST",
        &format!("/api/projects/{OWNER}/{PROJECT}/credentials"),
        json!({ "credential_id": "library-jwt", "title": "Library tokens", "kind": "jwt_signing_key", "notes": "", "secret": { "algorithm": "HS256", "secret": secret } }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    addressing(app, cookie, json!({ "hosts": [], "routes": [], "disabled": ["files", "ms"] })).await;
    publish(app, cookie, "library-search", r#"| trigger.mcp --route /library --name search --description "Find a book by words." --parameter q:string! "The words." --auth oauth --credential library-jwt --login /auth/login --role reader | web.response.send --body "{{ { found: input.mcp.arguments.q } }}""#).await;
    publish(app, cookie, "other-search", r#"| trigger.mcp --route /other --name search --auth oauth --credential library-jwt --login /auth/login | web.response.send --body "{{ { other: true } }}""#).await;
    publish(app, cookie, "library-me", r#"| trigger.webhook --route /account/me --method GET --auth jwt --credential library-jwt | web.response.send --body "{{ { me: true } }}""#).await;
    let login = [
        "[t] trigger.webhook --route /auth/login --method POST",
        "[sign] auth.token.create --credential library-jwt --claim \"sub={{ input.webhook.body.user }}\" --claim \"roles={{ input.webhook.body.roles }}\"",
        "[ok] auth.oauth.approve --ticket \"{{ input.webhook.body.oauth }}\" --token \"{{ input.token.access_token }}\" --client \"{{ input.webhook.body.client }}\" --redirect-host \"{{ input.webhook.body.redirect_host }}\"",
        "[expired] web.response.send --status 400 --body \"{{ { expired: true, code: input.oauth.error.code } }}\"",
        "[t] -> [sign]",
        "[sign] -> [ok]",
        "[ok]:error -> [expired]",
    ]
    .join("\n");
    publish(app, cookie, "library-login", &format!("\n{login}")).await;
    secret
}

async fn register(app: &axum::Router, redirect: &str) -> String {
    let answer = http(
        app,
        "POST",
        "/_mcp/library/_oauth/register",
        &[("content-type", "application/json")],
        Body::from(json!({ "client_name": "Reader agent", "redirect_uris": [redirect] }).to_string()),
    )
    .await;
    assert_eq!(answer.status, StatusCode::CREATED, "{}", answer.body);
    answer.json()["client_id"].as_str().expect("client_id").to_string()
}

fn authorize_uri(client_id: &str, redirect: &str, extra: &[(&str, &str)]) -> String {
    let mut pairs = vec![
        ("response_type", "code"),
        ("client_id", client_id),
        ("redirect_uri", redirect),
        ("code_challenge", CHALLENGE),
        ("code_challenge_method", "S256"),
        ("state", "st-1"),
        ("resource", R),
        ("scope", "mcp"),
    ];
    for (key, value) in extra {
        pairs.retain(|(k, _)| k != key);
        if !value.is_empty() {
            pairs.push((key, value));
        }
    }
    format!("/_mcp/library/_oauth/authorize?{}", serde_urlencoded::to_string(&pairs).expect("query"))
}

/// The login page's post: the person, and the facts the page showed.
async fn approve(app: &axum::Router, login_location: &str, roles: Value, doctor: Option<(&str, &str)>, cookie: Option<&str>) -> Answer {
    let shown = query_of(login_location);
    let mut form = json!({ "user": "reader-1", "roles": roles, "oauth": shown["oauth"], "client": shown["client"], "redirect_host": shown["redirect_host"] });
    if let Some((key, value)) = doctor {
        form[key] = json!(value);
    }
    let mut headers = vec![("content-type", "application/json")];
    if let Some(cookie) = cookie {
        headers.push(("cookie", cookie));
    }
    http(app, "POST", "/auth/login", &headers, Body::from(form.to_string())).await
}

/// authorize → login page → approve: the code the client receives.
async fn sign_in(app: &axum::Router, client_id: &str, redirect: &str, roles: Value) -> String {
    let started = http(app, "GET", &authorize_uri(client_id, redirect, &[]), &[], Body::empty()).await;
    assert_eq!(started.status, StatusCode::SEE_OTHER, "{}", started.body);
    let back = approve(app, &started.location(), roles, None, None).await;
    assert_eq!(back.status, StatusCode::SEE_OTHER, "{}", back.body);
    let location = back.location();
    assert!(location.starts_with(redirect), "{location}");
    let params = query_of(&location);
    assert_eq!(params["state"], "st-1");
    assert_eq!(params["iss"], R, "RFC 9207: the issuer rides with the code");
    params["code"].clone()
}

async fn token(app: &axum::Router, form: &[(&str, &str)]) -> Answer {
    http(
        app,
        "POST",
        "/_mcp/library/_oauth/token",
        &[("content-type", "application/x-www-form-urlencoded")],
        Body::from(serde_urlencoded::to_string(form).expect("form")),
    )
    .await
}

async fn exchange(app: &axum::Router, client_id: &str, redirect: &str, code: &str, verifier: &str) -> Answer {
    token(app, &[("grant_type", "authorization_code"), ("code", code), ("redirect_uri", redirect), ("client_id", client_id), ("code_verifier", verifier), ("resource", R)]).await
}

async fn mcp(app: &axum::Router, route: &str, bearer: Option<&str>, message: Value) -> Answer {
    let auth = bearer.map(|t| format!("Bearer {t}"));
    let mut headers = vec![("content-type", "application/json"), ("accept", "application/json, text/event-stream")];
    if let Some(auth) = &auth {
        headers.push(("authorization", auth));
    }
    http(app, "POST", &format!("/_mcp/{route}"), &headers, Body::from(message.to_string())).await
}

#[tokio::test]
async fn a_client_discovers_where_to_sign_in() {
    let root = temp_dir("oauth-discovery");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    library_app(&app, &cookie).await;

    let refused = mcp(&app, "library", None, list()).await;
    assert_eq!(refused.status, StatusCode::UNAUTHORIZED);
    let challenge = refused.header(header::WWW_AUTHENTICATE);
    assert_eq!(challenge, format!("Bearer resource_metadata=\"http://{HOST}/.well-known/oauth-protected-resource/_mcp/library\", scope=\"mcp\""));
    let wrong = mcp(&app, "library", Some("not-a-token"), list()).await;
    assert!(wrong.header(header::WWW_AUTHENTICATE).ends_with(", error=\"invalid_token\""), "a sent token that fails says so");

    let prm = http(&app, "GET", "/.well-known/oauth-protected-resource/_mcp/library", &[], Body::empty()).await;
    assert_eq!(prm.status, StatusCode::OK, "{}", prm.body);
    assert_eq!(prm.json()["resource"], R);
    assert_eq!(prm.json()["authorization_servers"], json!([R]));
    assert_eq!(prm.header(header::CACHE_CONTROL), "no-store");
    let meta = http(&app, "GET", "/.well-known/oauth-authorization-server/_mcp/library", &[], Body::empty()).await;
    let meta = meta.json();
    assert_eq!(meta["issuer"], R, "the issuer is the URL the document was fetched for");
    assert_eq!(meta["token_endpoint"], format!("{R}/_oauth/token"));
    assert_eq!(meta["code_challenge_methods_supported"], json!(["S256"]));
    assert_eq!(meta["authorization_response_iss_parameter_supported"], json!(true));
    assert_eq!(meta["client_id_metadata_document_supported"], json!(true));

    // The old root documents are gone; a route that is not oauth has none;
    // the platform form names no address without a configured base (D4).
    assert_eq!(http(&app, "GET", "/.well-known/oauth-protected-resource", &[], Body::empty()).await.status, StatusCode::NOT_FOUND);
    let platform = app
        .clone()
        .oneshot(Request::builder().uri("/.well-known/oauth-authorization-server").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(platform.status(), StatusCode::NOT_FOUND);
    publish(&app, &cookie, "open-tool", r#"| trigger.mcp --route /open --name ping --auth none | web.response.send --body "pong""#).await;
    assert_eq!(http(&app, "GET", "/.well-known/oauth-protected-resource/_mcp/open", &[], Body::empty()).await.status, StatusCode::NOT_FOUND);
    let platform_form = app
        .clone()
        .oneshot(Request::builder().uri(format!("/.well-known/oauth-protected-resource/mcp/{OWNER}/{PROJECT}/library")).body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(platform_form.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn registration_and_authorize_refuse_before_redirecting_anywhere() {
    let root = temp_dir("oauth-authorize");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    library_app(&app, &cookie).await;
    let bad = http(&app, "POST", "/_mcp/library/_oauth/register", &[("content-type", "application/json")], Body::from(json!({ "redirect_uris": ["http://client.example/cb"] }).to_string())).await;
    assert_eq!((bad.status, bad.json()["error"].clone()), (StatusCode::BAD_REQUEST, json!("invalid_redirect_uri")));
    let client = register(&app, REDIRECT).await;

    // An unknown client or an unregistered redirect: a page, never a redirect.
    for uri in [authorize_uri("zfc_aaaaaaaaaaaaaaaaaaaaaaaaaa", REDIRECT, &[]), authorize_uri(&client, "https://evil.example/cb", &[])] {
        let page = http(&app, "GET", &uri, &[], Body::empty()).await;
        assert_eq!(page.status, StatusCode::BAD_REQUEST, "{uri}");
        assert!(page.headers.get(header::LOCATION).is_none());
    }
    // Later refusals go back to the client with state and iss.
    for (extra, error) in [
        (vec![("code_challenge", "")], "invalid_request"),
        (vec![("code_challenge_method", "plain")], "invalid_request"),
        (vec![("response_type", "token")], "unsupported_response_type"),
        (vec![("resource", "http://default.superadmin.localhost/_mcp/other")], "invalid_target"),
        (vec![("scope", "admin")], "invalid_scope"),
    ] {
        let back = http(&app, "GET", &authorize_uri(&client, REDIRECT, &extra), &[], Body::empty()).await;
        assert_eq!(back.status, StatusCode::SEE_OTHER);
        let params = query_of(&back.location());
        assert_eq!((params["error"].as_str(), params["state"].as_str(), params["iss"].as_str()), (error, "st-1", R), "{extra:?}");
    }
    // A loopback redirect matches whatever port the client listens on.
    let native = register(&app, "http://127.0.0.1/callback").await;
    let started = http(&app, "GET", &authorize_uri(&native, "http://127.0.0.1:53121/callback", &[]), &[], Body::empty()).await;
    assert_eq!(started.status, StatusCode::SEE_OTHER);
    let shown = query_of(&started.location());
    assert!(started.location().starts_with(&format!("http://{HOST}/auth/login?")), "{}", started.location());
    assert_eq!((shown["client"].as_str(), shown["redirect_host"].as_str()), ("Reader agent", "127.0.0.1"));
}

#[tokio::test]
async fn the_whole_flow_signs_in_through_the_apps_login_and_calls_a_tool() {
    let root = temp_dir("oauth-flow");
    let app = app_at(&root).await;
    let studio = login(&app).await;
    let app_secret = library_app(&app, &studio).await;
    let client = register(&app, REDIRECT).await;

    // The Studio's own cookie rides along on every step and changes nothing.
    let started = http(&app, "GET", &authorize_uri(&client, REDIRECT, &[]), &[("cookie", &studio)], Body::empty()).await;
    let back = approve(&app, &started.location(), json!(["reader"]), None, Some(&studio)).await;
    let code = query_of(&back.location())["code"].clone();
    let tokens = exchange(&app, &client, REDIRECT, &code, VERIFIER).await;
    assert_eq!(tokens.status, StatusCode::OK, "{}", tokens.body);
    let tokens = tokens.json();
    assert_eq!((tokens["token_type"].as_str(), tokens["expires_in"].as_i64(), tokens["scope"].as_str()), (Some("Bearer"), Some(3600), Some("mcp")));
    let access = tokens["access_token"].as_str().expect("access").to_string();

    assert_eq!(mcp(&app, "library", Some(&access), initialize()).await.status, StatusCode::OK);
    let listed = mcp(&app, "library", Some(&access), list()).await;
    assert_eq!(tool_names(&listed.json()), vec!["search"]);
    let called = mcp(&app, "library", Some(&access), call("search", json!({ "q": "rivers" }))).await;
    assert_eq!(serde_json::from_str::<Value>(&text_of(&called.json())).expect("json"), json!({ "found": "rivers" }));

    // A Zebflow session opens nothing, with or without a token beside it.
    let studio_only = http(&app, "POST", "/_mcp/library", &[("content-type", "application/json"), ("accept", "application/json, text/event-stream"), ("cookie", &studio)], Body::from(list().to_string())).await;
    assert_eq!(studio_only.status, StatusCode::UNAUTHORIZED);

    // The token opens this route only: not /other, not the app's jwt webhook.
    assert_eq!(mcp(&app, "other", Some(&access), list()).await.status, StatusCode::UNAUTHORIZED);
    let bearer = format!("Bearer {access}");
    let webhook = http(&app, "GET", "/account/me", &[("authorization", &bearer)], Body::empty()).await;
    assert_eq!(webhook.status, StatusCode::UNAUTHORIZED, "{}", webhook.body);
    // And the app's own session token never opens the route.
    let now = chrono::Utc::now().timestamp();
    let session = jsonwebtoken::encode(&jsonwebtoken::Header::default(), &json!({ "sub": "reader-1", "roles": ["reader"], "exp": now + 600 }), &jsonwebtoken::EncodingKey::from_secret(app_secret.as_bytes())).unwrap();
    assert_eq!(mcp(&app, "library", Some(&session), list()).await.status, StatusCode::UNAUTHORIZED);
    let session_bearer = format!("Bearer {session}");
    assert_eq!(http(&app, "GET", "/account/me", &[("authorization", &session_bearer)], Body::empty()).await.status, StatusCode::OK, "the app's token still opens its own webhook");

    // Refresh rotation: R1 → R2; R1 again is refused and takes R2 with it.
    let r1 = tokens["refresh_token"].as_str().expect("refresh").to_string();
    let rotated = token(&app, &[("grant_type", "refresh_token"), ("refresh_token", &r1), ("client_id", &client), ("resource", R)]).await;
    assert_eq!(rotated.status, StatusCode::OK, "{}", rotated.body);
    let r2 = rotated.json()["refresh_token"].as_str().expect("r2").to_string();
    assert_ne!(r1, r2);
    let new_access = rotated.json()["access_token"].as_str().expect("access").to_string();
    assert_eq!(mcp(&app, "library", Some(&new_access), list()).await.status, StatusCode::OK);
    let replay = token(&app, &[("grant_type", "refresh_token"), ("refresh_token", &r1), ("client_id", &client)]).await;
    assert_eq!((replay.status, replay.json()["error"].clone()), (StatusCode::BAD_REQUEST, json!("invalid_grant")));
    let dead = token(&app, &[("grant_type", "refresh_token"), ("refresh_token", &r2), ("client_id", &client)]).await;
    assert_eq!(dead.json()["error"], "invalid_grant", "a replay revokes the family");
}

#[tokio::test]
async fn a_code_is_bound_single_use_and_proven_with_pkce() {
    let root = temp_dir("oauth-code");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    library_app(&app, &cookie).await;
    let client = register(&app, REDIRECT).await;
    let other_redirect = "https://client.example/other-callback";

    let wrong_verifier = sign_in(&app, &client, REDIRECT, json!(["reader"])).await;
    let refused = exchange(&app, &client, REDIRECT, &wrong_verifier, "x".repeat(43).as_str()).await;
    assert_eq!((refused.status, refused.json()["error"].clone()), (StatusCode::BAD_REQUEST, json!("invalid_grant")));
    let spent = exchange(&app, &client, REDIRECT, &wrong_verifier, VERIFIER).await;
    assert_eq!(spent.json()["error"], "invalid_grant", "a code tried once is spent");

    let wrong_redirect = sign_in(&app, &client, REDIRECT, json!(["reader"])).await;
    assert_eq!(exchange(&app, &client, other_redirect, &wrong_redirect, VERIFIER).await.json()["error"], "invalid_grant");

    let good = sign_in(&app, &client, REDIRECT, json!(["reader"])).await;
    let tokens = exchange(&app, &client, REDIRECT, &good, VERIFIER).await;
    assert_eq!(tokens.status, StatusCode::OK);
    let refresh = tokens.json()["refresh_token"].as_str().expect("refresh").to_string();
    let reused = exchange(&app, &client, REDIRECT, &good, VERIFIER).await;
    assert_eq!(reused.json()["error"], "invalid_grant");
    let revoked = token(&app, &[("grant_type", "refresh_token"), ("refresh_token", &refresh), ("client_id", &client)]).await;
    assert_eq!(revoked.json()["error"], "invalid_grant", "reusing a code revokes what it was exchanged for");

    let foreign = sign_in(&app, &client, REDIRECT, json!(["reader"])).await;
    let mut form = vec![("grant_type", "authorization_code"), ("code", foreign.as_str()), ("redirect_uri", REDIRECT), ("client_id", client.as_str()), ("code_verifier", VERIFIER)];
    form.push(("resource", "http://default.superadmin.localhost/_mcp/other"));
    assert_eq!(token(&app, &form).await.json()["error"], "invalid_target");
    let unsupported = token(&app, &[("grant_type", "password"), ("client_id", &client)]).await;
    assert_eq!(unsupported.json()["error"], "unsupported_grant_type");
}

#[tokio::test]
async fn approve_refuses_a_spent_ticket_doctored_consent_and_a_role_short_token() {
    let root = temp_dir("oauth-approve");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    library_app(&app, &cookie).await;
    let client = register(&app, REDIRECT).await;

    // The page changed what the person saw: the app's :error renders it.
    for (key, value) in [("client", "Trusted Bank"), ("redirect_host", "evil.example")] {
        let started = http(&app, "GET", &authorize_uri(&client, REDIRECT, &[]), &[], Body::empty()).await;
        let doctored = approve(&app, &started.location(), json!(["reader"]), Some((key, value)), None).await;
        assert_eq!(doctored.status, StatusCode::BAD_REQUEST, "{}", doctored.body);
        assert_eq!(doctored.json(), json!({ "expired": true, "code": "FW_NODE_AUTH_OAUTH_APPROVE_MISMATCH" }));
        // The ticket is spent either way.
        let again = approve(&app, &started.location(), json!(["reader"]), None, None).await;
        assert_eq!(again.json()["code"], "FW_NODE_AUTH_OAUTH_APPROVE_TICKET");
    }

    // A person without the route's role signs in, and is refused at the door.
    let code = sign_in(&app, &client, REDIRECT, json!(["guest"])).await;
    let access = exchange(&app, &client, REDIRECT, &code, VERIFIER).await.json()["access_token"].as_str().expect("access").to_string();
    let forbidden = mcp(&app, "library", Some(&access), list()).await;
    assert_eq!(forbidden.status, StatusCode::FORBIDDEN);
    assert!(forbidden.headers.get(header::WWW_AUTHENTICATE).is_none(), "a role is not a sign-in problem");
}

#[tokio::test]
async fn registration_stops_at_the_daily_cap() {
    let root = temp_dir("oauth-cap");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    library_app(&app, &cookie).await;
    let body = json!({ "redirect_uris": [REDIRECT] }).to_string();
    let mut last = StatusCode::OK;
    for _ in 0..=zebflow::platform::services::published_oauth::MAX_REGISTRATIONS_PER_DAY {
        last = http(&app, "POST", "/_mcp/library/_oauth/register", &[("content-type", "application/json")], Body::from(body.clone())).await.status;
    }
    assert_eq!(last, StatusCode::TOO_MANY_REQUESTS);
}
