//! A rendered page publishes the page, never the request: through a real
//! router, a page served on the project's own host with a proxy's address
//! chain, a host, a cookie and a bearer token carries none of them — not in
//! its hydration state, not as a `/wh/{owner}/{project}` route, not in an
//! `x-zebflow-project` header — while the server render still reads a header
//! it asks for. And the old unauthenticated-superadmin recipes, tried against
//! this code: a forged session, a user created or a password changed without
//! one, a forged controller call, an app's own token or an MCP token on the
//! platform API.

use std::fs;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use tower::ServiceExt;

use zebflow::platform::{PlatformConfig, build_router};

const OWNER: &str = "superadmin";
const PROJECT: &str = "default";
const HOST: &str = "default.superadmin.localhost";
const SIGNING_SECRET: &str = "page-privacy-test-signing-secret-0123456789";

struct TestDir(std::path::PathBuf);

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn temp_dir(name: &str) -> TestDir {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos();
    TestDir(std::env::temp_dir().join(format!("zebflow-platform-{name}-{now}")))
}

async fn app_at(root: &TestDir) -> axum::Router {
    let mut config = PlatformConfig::default();
    config.data_root = root.0.clone();
    config.default_password = "test-pass".to_string();
    build_router(config).await.expect("platform router")
}

async fn login(app: &axum::Router) -> String {
    let form = serde_urlencoded::to_string([("identifier", "superadmin"), ("password", "test-pass")]).expect("form");
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/login")
                .method("POST")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(form))
                .expect("request"),
        )
        .await
        .expect("login");
    assert_eq!(response.status(), StatusCode::SEE_OTHER);
    response.headers().get(header::SET_COOKIE).expect("cookie").to_str().expect("text").split(';').next().unwrap().to_string()
}

/// One request with these headers; status, headers and body as text.
async fn call(app: &axum::Router, method: &str, uri: &str, headers: &[(&str, &str)], body: Option<Value>) -> (StatusCode, axum::http::HeaderMap, String) {
    let mut request = Request::builder().uri(uri).method(method);
    for (name, value) in headers {
        request = request.header(*name, *value);
    }
    let body = match body {
        Some(json) => {
            request = request.header(header::CONTENT_TYPE, "application/json");
            Body::from(json.to_string())
        }
        None => Body::empty(),
    };
    let response = app.clone().oneshot(request.body(body).expect("request")).await.expect("response");
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("body");
    (status, headers, String::from_utf8_lossy(&bytes).to_string())
}

async fn studio(app: &axum::Router, cookie: &str, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
    let (status, _, text) = call(app, method, &format!("/api/projects/{OWNER}/{PROJECT}{path}"), &[("cookie", cookie)], Some(body)).await;
    (status, serde_json::from_str(&text).unwrap_or(Value::String(text)))
}

async fn write_file(app: &axum::Router, cookie: &str, path: &str, text: &str) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/projects/{OWNER}/{PROJECT}/repo/file?path={path}"))
                .method("PUT")
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "text/plain")
                .body(Body::from(text.to_string()))
                .expect("request"),
        )
        .await
        .expect("write");
    assert_eq!(response.status(), StatusCode::OK, "write {path}");
}

async fn publish(app: &axum::Router, cookie: &str, name: &str, body: &str) {
    for line in [format!("register pipelines/tests/{name} -- {body}"), format!("activate pipeline pipelines/tests/{name}.zf.json")] {
        let (_, answer) = studio(app, cookie, "POST", "/pipelines/dsl", json!({ "dsl": line })).await;
        assert_eq!(answer["ok"], json!(true), "{line}: {answer}");
    }
}

fn token(claims: Value) -> String {
    jsonwebtoken::encode(&jsonwebtoken::Header::default(), &claims, &jsonwebtoken::EncodingKey::from_secret(SIGNING_SECRET.as_bytes()))
        .expect("token")
}

/// The JSON inside `<script id="__rwe_payload">`.
fn hydration_state(html: &str) -> Value {
    let start = html.find("id=\"__rwe_payload\">").expect("a hydration payload") + "id=\"__rwe_payload\">".len();
    let end = start + html[start..].find("</script>").expect("its end");
    serde_json::from_str(&html[start..end]).expect("hydration JSON")
}

#[tokio::test]
async fn a_rendered_page_never_publishes_the_request() {
    let root = temp_dir("page-privacy");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    let (status, body) = studio(
        &app,
        &cookie,
        "POST",
        "/credentials",
        json!({ "credential_id": "site-key", "title": "Site tokens", "kind": "jwt_signing_key", "notes": "",
            "secret": { "algorithm": "HS256", "secret": SIGNING_SECRET } }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    // The server render may read a header when it asks for one by name.
    write_file(
        &app,
        &cookie,
        "pages/probe.tsx",
        r#"export default function Probe(input) {
  return <main><h1>Probe</h1><p id="agent">{input.headers?.["user-agent"] ?? "none"}</p><p id="route">{input.route}</p></main>;
}"#,
    )
    .await;
    publish(&app, &cookie, "probe", "| trigger.webhook --route /probe --method GET --auth jwt --credential site-key --auth-optional | web.response.send --template pages/probe.tsx").await;

    let bearer = token(json!({ "sub": "member-a", "email": "member-a@example.com", "_zf_public": ["sub"], "exp": 4_102_444_800i64 }));
    let authorization = format!("Bearer {bearer}");
    let (status, headers, html) = call(
        &app,
        "GET",
        "/probe?tab=one",
        &[
            ("host", HOST),
            ("x-forwarded-for", "203.0.113.7, 10.1.2.3"),
            ("x-real-ip", "10.1.2.3"),
            ("x-forwarded-host", "site-a.example"),
            ("cookie", "zebflow_session=probe-cookie-value; theme=cv-123"),
            ("authorization", authorization.as_str()),
            ("user-agent", "probe-agent/1.0"),
        ],
        None,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{html}");
    assert!(!html.contains("RWE component error"), "{html}");
    assert!(html.contains("<p id=\"agent\">probe-agent/1.0</p>"), "the server render still reads a header it asks for: {html}");
    assert!(html.contains("<p id=\"route\">/probe</p>"), "the route is the visitor's path: {html}");

    let state = hydration_state(&html);
    let published = state.to_string();
    for leaked in ["10.1.2.3", "203.0.113.7", "site-a.example", HOST, "probe-cookie-value", "cv-123", bearer.as_str(), "member-a@example.com", "probe-agent/1.0", "/wh/superadmin/default"] {
        assert!(!published.contains(leaked), "the hydration state publishes {leaked}: {published}");
    }
    assert!(state.get("headers").is_none(), "{published}");
    assert!(state["webhook"].get("headers").is_none() && state["webhook"].get("auth").is_none(), "{published}");
    assert_eq!(state["auth"], json!({ "sub": "member-a" }), "only the claims the token marks public");
    assert_eq!(state["route"], "/probe");
    assert_eq!(state["webhook"]["query"]["tab"], "one", "the page's own data stays");

    for leaked in ["10.1.2.3", "203.0.113.7", "probe-cookie-value", "cv-123", bearer.as_str(), "member-a@example.com", "/wh/superadmin/default"] {
        assert!(!html.contains(leaked), "the page publishes {leaked}");
    }
    assert!(headers.get("x-zebflow-project").is_none(), "no response header names the owner and project");
    for (name, value) in headers.iter() {
        let value = value.to_str().unwrap_or_default();
        assert!(!value.contains("superadmin/default") && !value.contains("10.1.2.3"), "{name}: {value}");
    }

    // The platform form is the address that visitor used, so it may show it;
    // it still never publishes the request.
    let (status, _, html) = call(&app, "GET", "/wh/superadmin/default/probe", &[("x-real-ip", "10.1.2.3")], None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!hydration_state(&html).to_string().contains("10.1.2.3"));
}

/// The recipes for an unauthenticated superadmin takeover, against this
/// code: none of them reaches the roster, a password, or the platform API.
#[tokio::test]
async fn nobody_becomes_superadmin_without_signing_in_as_one() {
    let root = temp_dir("takeover");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    // Invented for this run only.
    let attempt = uuid::Uuid::new_v4().simple().to_string();
    let new_user = json!({ "owner": "intruder", "password": attempt, "role": "superadmin" });

    // A session cookie is an opaque token, never the owner's name.
    for forged in ["zebflow_session=superadmin", "zebflow_session=superadmin:superadmin", "zebflow_session=", "zebflow_session=null"] {
        let (status, _, _) = call(&app, "GET", "/api/platform/users", &[("cookie", forged)], None).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{forged}");
        let (status, _, _) = call(&app, "POST", "/api/platform/users", &[("cookie", forged)], Some(new_user.clone())).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{forged}");
        let (status, _, _) = call(&app, "POST", "/api/profile/password", &[("cookie", forged)], Some(json!({ "current_password": "", "new_password": attempt }))).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "{forged}");
    }
    // No credentials at all.
    let (status, _, _) = call(&app, "POST", "/api/platform/users", &[], Some(new_user.clone())).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    // A forged controller call reaches no internal route.
    let (status, _, _) = call(&app, "POST", "/api/internal/runtime/webhook/superadmin/default/x", &[("x-zebflow-cluster-token", "forged")], None).await;
    assert!(status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN, "{status}");
    // An app's own token, claiming any role, is not a platform session.
    let app_token = token(json!({ "sub": "superadmin", "roles": ["superadmin"], "role": "superadmin", "exp": 4_102_444_800i64 }));
    let (status, _, _) = call(&app, "POST", "/api/platform/users", &[("authorization", &format!("Bearer {app_token}"))], Some(new_user.clone())).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    // A project's MCP token builds that project; it never manages the instance.
    let (status, session) = studio(&app, &cookie, "POST", "/mcp/session", json!({ "capabilities": ["pipelines.read", "pipelines.write"] })).await;
    assert_eq!(status, StatusCode::OK, "{session}");
    let mcp = format!("Bearer {}", session["session"]["token"].as_str().expect("token"));
    let (status, _, _) = call(&app, "POST", "/api/platform/users", &[("authorization", &mcp)], Some(new_user.clone())).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    // A wrong password is not a session.
    let form = serde_urlencoded::to_string([("identifier", "superadmin"), ("password", "")]).expect("form");
    let (status, headers, _) = call(&app, "POST", "/login", &[("content-type", "application/x-www-form-urlencoded")], None).await;
    assert!(headers.get(header::SET_COOKIE).is_none() || status != StatusCode::SEE_OTHER, "an empty login is no session");
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/login")
                .method("POST")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(form))
                .expect("request"),
        )
        .await
        .expect("login");
    let set = response.headers().get(header::SET_COOKIE).and_then(|v| v.to_str().ok()).unwrap_or_default().to_string();
    assert!(!set.starts_with("zebflow_session=") || set.starts_with("zebflow_session=;"), "a wrong password opens no session: {set}");

    // And nothing got in: the roster is the one superadmin.
    let (status, _, roster) = call(&app, "GET", "/api/platform/users", &[("cookie", &cookie)], None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!roster.contains("intruder"), "{roster}");
}
