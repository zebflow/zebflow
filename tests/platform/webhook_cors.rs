//! Webhook CORS through the real router (`addressing.md` §2a `origins`): a
//! browser page on one of the project's own hosts, or on an origin the project
//! lists, may call its webhooks with the visitor's sign-in; the platform
//! answers the preflight before any pipeline runs and puts the allow headers
//! on every answer, its own refusals included. A page anywhere else is
//! refused, as is a plain-http page on a project host when the request came
//! over https; the platform address never answers CORS, and a pipeline that
//! sets its own `Access-Control-Allow-Origin` is never given a second one.

use axum::body::{Body, to_bytes};
use axum::http::{HeaderMap, Request, StatusCode, header};
use serde_json::json;
use tower::ServiceExt;

use super::published_mcp::{OWNER, PROJECT, addressing, app_at, login, publish, send, temp_dir};

const SITE: &str = "site.example";
const GAME: &str = "https://play.site.example";
const PARTNER: &str = "https://partner.example";

async fn call(app: &axum::Router, method: &str, host: &str, uri: &str, headers: &[(&str, &str)]) -> (StatusCode, HeaderMap, String) {
    let mut builder = Request::builder().method(method).uri(uri).header(header::HOST, host);
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    let response = app.clone().oneshot(builder.body(Body::empty()).expect("request")).await.expect("response");
    let status = response.status();
    let headers = response.headers().clone();
    let body = String::from_utf8_lossy(&to_bytes(response.into_body(), usize::MAX).await.expect("body")).to_string();
    (status, headers, body)
}

fn allow_origin(headers: &HeaderMap) -> Vec<String> {
    headers.get_all(header::ACCESS_CONTROL_ALLOW_ORIGIN).iter().filter_map(|v| v.to_str().ok().map(str::to_string)).collect()
}

#[tokio::test]
async fn pages_on_the_projects_hosts_and_listed_sites_may_call_its_webhooks() {
    let root = temp_dir("webhook-cors");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    let key = format!("k-{}", uuid::Uuid::new_v4().simple());
    let (status, body) = send(
        &app,
        &cookie,
        "POST",
        &format!("/api/projects/{OWNER}/{PROJECT}/credentials"),
        json!({ "credential_id": "game-key", "title": "Game", "kind": "api_key", "notes": "", "secret": { "key": key } }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    publish(&app, &cookie, "ping", r#"| trigger.webhook --route /api/ping --method POST | web.response.send --body pong"#).await;
    publish(&app, &cookie, "own", r#"| trigger.webhook --route /api/own --method GET | web.response.send --header "Access-Control-Allow-Origin=https://play.site.example" --body own"#).await;
    publish(&app, &cookie, "private", r#"| trigger.webhook --route /api/private --method POST --auth api_key --credential game-key | web.response.send --body secret"#).await;
    addressing(&app, &cookie, json!({ "hosts": [SITE, "play.site.example"], "routes": [], "disabled": ["files", "ms"], "origins": [PARTNER] })).await;

    // The preflight, answered by the platform: the path's methods, the asked headers, sign-in allowed.
    let preflight = [("origin", GAME), ("access-control-request-method", "POST"), ("access-control-request-headers", "content-type")];
    let (status, headers, _) = call(&app, "OPTIONS", SITE, "/api/ping", &preflight).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(allow_origin(&headers), vec![GAME.to_string()]);
    assert_eq!(headers.get(header::ACCESS_CONTROL_ALLOW_CREDENTIALS).and_then(|v| v.to_str().ok()), Some("true"));
    assert!(headers.get(header::ACCESS_CONTROL_ALLOW_METHODS).and_then(|v| v.to_str().ok()).is_some_and(|m| m.contains("POST")));
    assert_eq!(headers.get(header::ACCESS_CONTROL_ALLOW_HEADERS).and_then(|v| v.to_str().ok()), Some("content-type"));
    // A listed site too; an unknown one is refused; a path with no webhook is a 404.
    let partner = [("origin", PARTNER), ("access-control-request-method", "POST")];
    assert_eq!(call(&app, "OPTIONS", SITE, "/api/ping", &partner).await.0, StatusCode::NO_CONTENT);
    let stranger = [("origin", "https://other.example"), ("access-control-request-method", "POST")];
    assert_eq!(call(&app, "OPTIONS", SITE, "/api/ping", &stranger).await.0, StatusCode::FORBIDDEN);
    assert_eq!(call(&app, "OPTIONS", SITE, "/api/nothing", &preflight).await.0, StatusCode::NOT_FOUND);

    // The answer itself carries the allow headers for an allowed page only.
    let (status, headers, body) = call(&app, "POST", SITE, "/api/ping", &[("origin", GAME)]).await;
    assert_eq!((status, body.as_str()), (StatusCode::OK, "pong"));
    assert_eq!(allow_origin(&headers), vec![GAME.to_string()]);
    assert!(headers.get_all(header::VARY).iter().any(|v| v.to_str().is_ok_and(|v| v.contains("Origin"))));
    let (_, headers, _) = call(&app, "POST", SITE, "/api/ping", &[("origin", "https://other.example")]).await;
    assert!(allow_origin(&headers).is_empty(), "no allow headers for an unknown site");
    let tls = ("x-forwarded-proto", "https");
    let (_, headers, _) = call(&app, "POST", SITE, "/api/ping", &[("origin", "https://site.example"), tls]).await;
    assert!(allow_origin(&headers).is_empty(), "the page's own host needs no CORS");
    // A plain-http page on a project host, calling over https, is not the
    // project's page: anyone on the network path can write one.
    let forged = [("origin", "http://play.site.example"), ("access-control-request-method", "POST"), tls];
    assert_eq!(call(&app, "OPTIONS", SITE, "/api/ping", &forged).await.0, StatusCode::FORBIDDEN);
    let (_, headers, _) = call(&app, "POST", SITE, "/api/ping", &[("origin", "http://play.site.example"), tls]).await;
    assert!(allow_origin(&headers).is_empty(), "no allow headers for a plain-http page");

    // A refusal the platform makes before the pipeline runs carries them too.
    let (status, headers, _) = call(&app, "POST", SITE, "/api/private", &[("origin", GAME)]).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(allow_origin(&headers), vec![GAME.to_string()], "a page can tell not signed in from a network failure");

    // A pipeline's own header is never doubled.
    let (_, headers, _) = call(&app, "GET", SITE, "/api/own", &[("origin", GAME)]).await;
    assert_eq!(allow_origin(&headers), vec![GAME.to_string()], "exactly one Access-Control-Allow-Origin");

    // The platform address answers no CORS.
    let (status, headers, _) = call(&app, "OPTIONS", "localhost", &format!("/wh/{OWNER}/{PROJECT}/api/ping"), &preflight).await;
    assert_ne!(status, StatusCode::NO_CONTENT);
    assert!(allow_origin(&headers).is_empty());
}
