//! *Hide from home* (`project.md` § Home listing), through a real router: a
//! hidden project leaves the home list and waits under *Hidden projects*,
//! stays reachable by its address and in the project listing API, and comes
//! back when shown again. Hiding is not a commit, and only someone who may
//! change the project's settings may do it.

use std::fs;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use tower::ServiceExt;

use zebflow::platform::{PlatformConfig, build_router};

const OWNER: &str = "superadmin";

struct TestDir(std::path::PathBuf);

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn temp_dir(name: &str) -> TestDir {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    TestDir(std::env::temp_dir().join(format!("zebflow-platform-{name}-{now}")))
}

async fn login(app: &axum::Router) -> String {
    let form = serde_urlencoded::to_string([("identifier", OWNER), ("password", "test-pass")])
        .expect("form");
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
    response
        .headers()
        .get(header::SET_COOKIE)
        .expect("cookie")
        .to_str()
        .expect("text")
        .split(';')
        .next()
        .unwrap()
        .to_string()
}

async fn call(
    app: &axum::Router,
    method: &str,
    uri: &str,
    cookie: Option<&str>,
    body: Option<Value>,
) -> (StatusCode, String) {
    let mut request = Request::builder().uri(uri).method(method);
    if let Some(cookie) = cookie {
        request = request.header(header::COOKIE, cookie);
    }
    let body = match body {
        Some(json) => {
            request = request.header(header::CONTENT_TYPE, "application/json");
            Body::from(json.to_string())
        }
        None => Body::empty(),
    };
    let response = app
        .clone()
        .oneshot(request.body(body).expect("request"))
        .await
        .expect("response");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("body");
    (status, String::from_utf8_lossy(&bytes).to_string())
}

/// The project's Edit link as the home list renders it. The page's
/// hydration state names every path too, so the rendered anchor is the
/// thing that says whether the project is *listed*.
fn listed(home: &str, project: &str) -> bool {
    home.contains(&format!("href=\"/projects/{OWNER}/{project}\""))
}

async fn home(app: &axum::Router, cookie: &str) -> String {
    let (status, html) = call(app, "GET", "/home", Some(cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!html.contains("RWE component error"), "home rendered with a component error");
    html
}

#[tokio::test]
async fn a_hidden_project_leaves_the_home_list_and_comes_back_when_shown() {
    let root = temp_dir("home-hidden");
    let mut config = PlatformConfig::default();
    config.data_root = root.0.clone();
    config.default_password = "test-pass".to_string();
    let app = build_router(config).await.expect("platform router");
    let cookie = login(&app).await;

    let form = serde_urlencoded::to_string([("project", "catalog"), ("title", "Catalog")])
        .expect("form");
    let created = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/home/projects/create")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(form))
                .expect("request"),
        )
        .await
        .expect("create");
    assert_eq!(created.status(), StatusCode::SEE_OTHER);

    // A standalone instance shows its own starter project.
    let page = home(&app, &cookie).await;
    assert!(listed(&page, "default") && listed(&page, "catalog"));
    assert!(!page.contains("data-hidden-projects"), "nothing hidden, no hidden section");

    let api = format!("/api/projects/{OWNER}/default/settings/home");
    let (status, body) = call(&app, "GET", &api, Some(&cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(serde_json::from_str::<Value>(&body).unwrap()["data"]["hidden"], json!(false));

    // Hidden: no commit message asked for, because nothing is committed.
    let (status, body) =
        call(&app, "PUT", &api, Some(&cookie), Some(json!({"data": {"hidden": true}}))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(serde_json::from_str::<Value>(&body).unwrap()["data"]["hidden"], json!(true));

    let page = home(&app, &cookie).await;
    assert!(!listed(&page, "default"), "a hidden project is still on the home list");
    assert!(listed(&page, "catalog"), "hiding one project hid another");
    assert!(page.contains("data-hidden-projects"), "the hidden section is missing");
    assert!(page.contains("Hidden projects"), "the hidden section is unlabelled");

    // Reachable by address and through the project listing.
    let (status, studio) = call(&app, "GET", &format!("/projects/{OWNER}/default"), Some(&cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!studio.contains("RWE component error"));
    let (_, projects) = call(&app, "GET", &format!("/api/users/{OWNER}/projects"), Some(&cookie), None).await;
    assert!(projects.contains("\"default\""), "the listing API dropped a hidden project");

    // The settings page carries the toggle, ticked.
    let (status, settings) =
        call(&app, "GET", &format!("/projects/{OWNER}/default/settings"), Some(&cookie), None).await;
    assert_eq!(status, StatusCode::OK);
    assert!(settings.contains("data-home-visibility") && settings.contains("Hide from home"));
    assert!(!settings.contains("RWE component error"));

    // Shown again.
    let (status, _) =
        call(&app, "PUT", &api, Some(&cookie), Some(json!({"data": {"hidden": false}}))).await;
    assert_eq!(status, StatusCode::OK);
    let page = home(&app, &cookie).await;
    assert!(listed(&page, "default"), "showing a project did not restore it");
    assert!(!page.contains("data-hidden-projects"));

    // Refusals: no session, an unknown field, a project that does not exist.
    let (status, _) = call(&app, "PUT", &api, None, Some(json!({"data": {"hidden": true}}))).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    let (status, _) =
        call(&app, "PUT", &api, Some(&cookie), Some(json!({"data": {"hidden": true, "x": 1}}))).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, _) = call(
        &app,
        "PUT",
        &format!("/api/projects/{OWNER}/nope/settings/home"),
        Some(&cookie),
        Some(json!({"data": {"hidden": true}})),
    )
    .await;
    assert!(status.is_client_error(), "hiding a missing project answered {status}");
}
