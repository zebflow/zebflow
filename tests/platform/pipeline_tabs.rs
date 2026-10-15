//! The pipelines area's tabs, rendered through the real router. A page whose
//! component threw still answers 200, so each tab is read for the error
//! marker as well as its status.

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use tower::ServiceExt;

use zebflow::platform::{PlatformConfig, PlatformService};

const OWNER: &str = "superadmin";
const PROJECT: &str = "default";

struct TestDir(std::path::PathBuf);

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

async fn login_as(app: &axum::Router, identifier: &str, password: &str) -> String {
    let form = serde_urlencoded::to_string([("identifier", identifier), ("password", password)]).expect("form");
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

async fn get(app: &axum::Router, cookie: &str, uri: &str) -> (StatusCode, String) {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(uri).header(header::COOKIE, cookie).body(Body::empty()).expect("request"))
        .await
        .expect("response");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("body");
    (status, String::from_utf8_lossy(&bytes).to_string())
}

async fn studio() -> (TestDir, axum::Router, String) {
    let root = TestDir(std::env::temp_dir().join(format!(
        "zebflow-platform-pipeline-tabs-{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    )));
    let mut config = PlatformConfig::default();
    config.data_root = root.0.clone();
    config.default_password = uuid::Uuid::new_v4().to_string();
    let password = config.default_password.clone();
    let platform = Arc::new(PlatformService::from_config(config).expect("platform"));
    let app = zebflow::platform::web::router(platform).await;
    let cookie = login_as(&app, OWNER, &password).await;
    (root, app, cookie)
}

/// Every tab renders, and every tab, the editor's included, carries the same
/// tab bar.
#[tokio::test]
async fn every_pipelines_tab_renders_with_the_shared_tab_bar() {
    let (_root, app, cookie) = studio().await;
    let base = format!("/projects/{OWNER}/{PROJECT}/pipelines");
    for tab in ["webhooks", "schedules", "manual", "functions", "rooms", "registry?path=/"] {
        let (status, body) = get(&app, &cookie, &format!("{base}/{tab}")).await;
        assert_eq!(status, StatusCode::OK, "{tab}");
        assert!(!body.contains("RWE component error"), "{tab}: {body}");
        for link in ["webhooks", "schedules", "manual", "functions", "rooms"] {
            assert!(body.contains(&format!("href=\"{base}/{link}\"")), "{tab} has no {link} tab");
        }
    }
    for tab in ["schedules", "manual", "functions"] {
        let (_, body) = get(&app, &cookie, &format!("{base}/{tab}")).await;
        assert!(body.contains("Create one in the Registry tab."), "{tab} lists no pipelines panel");
    }
    let (status, _) = get(&app, &cookie, &format!("{base}/no-such-tab")).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// The rooms tab lists live rooms from the rooms API, not pipeline files:
/// the page is handed that API and draws no pipeline list.
#[tokio::test]
async fn the_rooms_tab_reads_live_rooms_from_the_rooms_api() {
    let (_root, app, cookie) = studio().await;
    let (status, body) = get(&app, &cookie, &format!("/projects/{OWNER}/{PROJECT}/pipelines/rooms")).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!body.contains("RWE component error"), "{body}");
    assert!(body.contains("data-rooms-panel=\"true\""), "the rooms panel is drawn");
    assert!(
        body.contains(&format!("data-rooms-api=\"/api/projects/{OWNER}/{PROJECT}/ws/rooms\"")),
        "the panel reads the project's rooms API"
    );
    assert!(!body.contains("Create one in the Registry tab."), "rooms are not a pipeline list");

    let (_, schedules) = get(&app, &cookie, &format!("/projects/{OWNER}/{PROJECT}/pipelines/schedules")).await;
    assert!(!schedules.contains("data-rooms-panel"), "only the rooms tab draws the rooms panel");
}
