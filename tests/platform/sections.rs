//! Sections — one project, several sites (`docs/contracts/addressing.md` §2c),
//! through the real router: each host answers its own site, a section's
//! paths answer nowhere else by any spelling or form, the URLs the platform
//! writes are the visitor's, and a save refuses a route its section cannot
//! reach.

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use tower::ServiceExt;

use super::published_mcp::{OWNER, PROJECT, addressing, app_at, login, publish, send, temp_dir};

const SITE: &str = "example.org";

/// A signing key made per run, never a literal.
fn signing_key() -> String {
    format!("{}{}", uuid::Uuid::new_v4().simple(), uuid::Uuid::new_v4().simple())
}
const SECTION: &str = "a.example.net";
const SECTION_DEV: &str = "site-a.default.superadmin.localhost";

struct Answer {
    status: StatusCode,
    headers: axum::http::HeaderMap,
    body: String,
}

async fn on(app: &axum::Router, host: &str, uri: &str, headers: &[(&str, &str)]) -> Answer {
    let mut builder = Request::builder().uri(uri).header(header::HOST, host);
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    let response = app.clone().oneshot(builder.body(Body::empty()).expect("request")).await.expect("response");
    let status = response.status();
    let headers = response.headers().clone();
    let body = String::from_utf8_lossy(&to_bytes(response.into_body(), usize::MAX).await.expect("body")).to_string();
    Answer { status, headers, body }
}

async fn sectioned_app(app: &axum::Router, cookie: &str) {
    publish(app, cookie, "root-map", r#"| trigger.webhook --route /map --method GET | web.response.send --body "root-map""#).await;
    publish(app, cookie, "a-home", r#"| trigger.webhook --route /site-a --method GET | web.response.send --body "a-home""#).await;
    publish(app, cookie, "a-map", r#"| trigger.webhook --route /site-a/map --method GET | web.response.send --body "a-map""#).await;
    publish(app, cookie, "shared", r#"| trigger.webhook --route /api/data --method GET | web.response.send --body "shared""#).await;
    publish(
        app,
        cookie,
        "missing",
        r#"| trigger.error --status 404 | web.response.send --status 404 --body "{{ { missing: input.error.path } }}""#,
    )
    .await;
    addressing(
        app,
        cookie,
        json!({
            "hosts": [SITE, SECTION],
            "routes": [
                { "host": SECTION, "path": "/", "surface": "pages", "root": "/site-a" },
                { "host": SECTION, "path": "/api/", "surface": "pages" }
            ],
            "disabled": ["files", "ms"]
        }),
    )
    .await;
}

#[tokio::test]
async fn each_host_answers_its_own_site_and_no_other() {
    let root = temp_dir("sections-routing");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    sectioned_app(&app, &cookie).await;

    for (host, uri, body) in [
        (SECTION, "/", "a-home"),
        (SECTION, "/map", "a-map"),
        (SECTION, "/api/data", "shared"),
        (SITE, "/map", "root-map"),
        (SECTION_DEV, "/map", "a-map"),
        (SECTION_DEV, "/api/data", "shared"),
        ("default.superadmin.localhost", "/map", "root-map"),
        // The platform address answers every path, as today.
        ("localhost:10610", "/wh/superadmin/default/site-a/map", "a-map"),
        ("127.0.0.1", "/wh/superadmin/default/map", "root-map"),
    ] {
        let answer = on(&app, host, uri, &[]).await;
        assert_eq!((answer.status, answer.body.as_str()), (StatusCode::OK, body), "{host}{uri}");
    }

    // A section's paths answer nowhere else, by any spelling or form: the
    // project's own 404, naming the path the visitor asked for.
    for (host, uri) in [
        (SITE, "/site-a/map"),
        (SITE, "//site-a/map"),
        (SITE, "/%73ite-a/map"),
        (SITE, "/site-a"),
        (SITE, "/wh/superadmin/default/site-a/map"),
        ("default.superadmin.localhost", "/site-a/map"),
        ("stray.example", "/wh/superadmin/default/site-a/map"),
        (SECTION, "/wh/superadmin/default/map"),
        (SECTION, "/%77h/superadmin/default/map"),
        (SECTION, "/_nothing"),
    ] {
        let answer = on(&app, host, uri, &[("accept", "application/json")]).await;
        assert_eq!(answer.status, StatusCode::NOT_FOUND, "{host}{uri}: {}", answer.body);
        assert!(!answer.body.contains("a-map") && !answer.body.contains("root-map"), "{host}{uri}: {}", answer.body);
    }
    let claimed = on(&app, SITE, "/site-a/map", &[]).await;
    assert_eq!(serde_json::from_str::<Value>(&claimed.body).expect("json")["missing"], json!("/site-a/map"), "the project's 404 page, as for an unknown path");
    let unknown = on(&app, SITE, "/nowhere", &[]).await;
    assert_eq!(serde_json::from_str::<Value>(&unknown.body).expect("json")["missing"], json!("/nowhere"));
    let inside = on(&app, SECTION, "/nowhere", &[]).await;
    assert_eq!(serde_json::from_str::<Value>(&inside.body).expect("json")["missing"], json!("/nowhere"), "the visitor's path, never the root's");

    // No single reading: 400.
    for uri in ["/%2Fsite-a/map", "/x/%2e%2e/map", "/a%00"] {
        assert_eq!(on(&app, SECTION, uri, &[]).await.status, StatusCode::BAD_REQUEST, "{uri}");
    }
    // The stray host keeps today's rootless reach.
    let stray = on(&app, "stray.example", "/wh/superadmin/default/map", &[]).await;
    assert_eq!((stray.status, stray.body.as_str()), (StatusCode::OK, "root-map"));

    // Verify proves the section: the bare probe answers on a section host.
    assert_eq!(on(&app, SECTION, "/_verify", &[]).await.status, StatusCode::NO_CONTENT);

    // Settings lists the section with its dev host; the stored file holds
    // the root only where it was set.
    let (status, body) = send(&app, &cookie, "GET", &format!("/api/projects/{OWNER}/{PROJECT}/settings/addressing"), json!(null)).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let data = if body["data"].is_null() { &body["section"]["data"] } else { &body["data"] };
    let sections = data["sections"].as_array().cloned().unwrap_or_default();
    assert_eq!(sections.len(), 1, "{data}");
    assert_eq!(sections[0]["dev_host"], json!(SECTION_DEV));
    let stored = std::fs::read_to_string(root.0.join("users/superadmin/default/data/store/addressing.json")).expect("stored");
    assert_eq!(stored.matches("\"root\"").count(), 1, "{stored}");
}

#[tokio::test]
async fn the_urls_the_platform_writes_are_the_visitors() {
    let root = temp_dir("sections-visitor-form");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    let (status, body) = send(
        &app,
        &cookie,
        "POST",
        &format!("/api/projects/{OWNER}/{PROJECT}/credentials"),
        json!({
            "credential_id": "members",
            "title": "Members",
            "kind": "jwt_signing_key",
            "notes": "",
            "secret": { "algorithm": "HS256", "secret": signing_key(), "cookie_name": "member", "auth_redirect": "/site-a/login" }
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    publish(&app, &cookie, "mine", r#"| trigger.webhook --route /site-a/mine --method GET --auth jwt --credential members | web.response.send --body ok"#).await;
    publish(&app, &cookie, "a-login", r#"| trigger.webhook --route /site-a/login --method GET | web.response.send --body login"#).await;
    sectioned_app(&app, &cookie).await;

    let nav = [("accept", "text/html"), ("sec-fetch-mode", "navigate"), ("sec-fetch-dest", "document")];
    let section = on(&app, SECTION, "/mine?tab=2", &nav).await;
    assert_eq!(section.status, StatusCode::SEE_OTHER);
    assert_eq!(section.headers[header::LOCATION], "/login?next=%2Fmine%3Ftab%3D2", "both the target and next as the visitor reaches them");
    let platform = on(&app, "localhost", "/wh/superadmin/default/site-a/mine", &nav).await;
    assert_eq!(
        platform.headers[header::LOCATION],
        "/wh/superadmin/default/site-a/login?next=%2Fwh%2Fsuperadmin%2Fdefault%2Fsite-a%2Fmine",
        "on the platform address, the platform form"
    );
}

#[tokio::test]
async fn a_section_publishes_its_own_mcp_servers_at_its_own_address() {
    let root = temp_dir("sections-mcp");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    let (status, body) = send(
        &app,
        &cookie,
        "POST",
        &format!("/api/projects/{OWNER}/{PROJECT}/credentials"),
        json!({ "credential_id": "tools-jwt", "title": "Tools", "kind": "jwt_signing_key", "notes": "", "secret": { "algorithm": "HS256", "secret": signing_key() } }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    publish(&app, &cookie, "a-tools", r#"| trigger.mcp --route /site-a/tools --name ping --auth oauth --credential tools-jwt --login /site-a/login | web.response.send --body pong"#).await;
    sectioned_app(&app, &cookie).await;
    addressing(
        &app,
        &cookie,
        json!({
            "hosts": [SITE, SECTION],
            "routes": [
                { "host": SECTION, "path": "/", "surface": "pages", "root": "/site-a" },
                { "host": SECTION, "path": "/api/", "surface": "pages" }
            ],
            "disabled": ["files", "ms"]
        }),
    )
    .await;

    let prm = on(&app, SECTION, "/.well-known/oauth-protected-resource/_mcp/tools", &[]).await;
    assert_eq!(prm.status, StatusCode::OK, "{}", prm.body);
    let prm: Value = serde_json::from_str(&prm.body).expect("json");
    assert_eq!(prm["resource"], json!("https://a.example.net/_mcp/tools"), "the connect URL is the visitor's");
    // The section's server answers nowhere else.
    for (host, uri) in [(SITE, "/_mcp/site-a/tools"), (SITE, "/mcp/superadmin/default/site-a/tools"), (SITE, "/.well-known/oauth-protected-resource/_mcp/site-a/tools")] {
        assert_eq!(on(&app, host, uri, &[("accept", "application/json")]).await.status, StatusCode::NOT_FOUND, "{host}{uri}");
    }
}

#[tokio::test]
async fn a_save_refuses_a_route_its_section_cannot_reach() {
    let root = temp_dir("sections-refusals");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    publish(&app, &cookie, "hidden", r#"| trigger.webhook --route /site-a/api/x --method GET | web.response.send --body x"#).await;
    let save = |routes: Value| {
        let app = app.clone();
        let cookie = cookie.clone();
        async move {
            send(&app, &cookie, "PUT", &format!("/api/projects/{OWNER}/{PROJECT}/settings/addressing"), json!({ "data": { "hosts": [SITE, SECTION], "routes": routes } })).await
        }
    };
    let (status, body) = save(json!([
        { "host": SECTION, "path": "/", "surface": "pages", "root": "/site-a" },
        { "host": SECTION, "path": "/api/", "surface": "pages" }
    ]))
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert_eq!(body["error"]["code"], json!("PIPELINE_ROUTE_SHADOWED"));
    // Without the shared route it saves, and the route answers.
    let (status, body) = save(json!([{ "host": SECTION, "path": "/", "surface": "pages", "root": "/site-a" }])).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(on(&app, SECTION, "/api/x", &[]).await.body, "x");
    // A pages route off `/` stands only beside a root.
    let (status, _) = save(json!([{ "host": SITE, "path": "/api/", "surface": "pages" }])).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    // Taking the root away leaves a rootless project answering as before.
    let (status, body) = save(json!([])).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(on(&app, SITE, "/site-a/api/x", &[]).await.body, "x");
    assert_eq!(on(&app, "site-a.default.superadmin.localhost", "/api/x", &[]).await.status == StatusCode::OK, false, "the section dev host is gone");
}
