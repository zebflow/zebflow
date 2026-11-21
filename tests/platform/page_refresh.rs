//! A compiled page follows the files it is built from, whoever changes them:
//! a writer that does not refresh pages itself (Zebflow control, a git sync, an
//! import) still makes the next request serve the new code.

use axum::body::{Body, to_bytes};
use axum::http::{Request, header};
use tower::ServiceExt;

use super::published_mcp::{OWNER, PROJECT, login, publish, temp_dir};

async fn get_text(app: &axum::Router, uri: &str) -> String {
    let response = app.clone().oneshot(Request::builder().uri(uri).body(Body::empty()).expect("request")).await.expect("response");
    String::from_utf8_lossy(&to_bytes(response.into_body(), usize::MAX).await.expect("body")).to_string()
}

async fn put_file(app: &axum::Router, cookie: &str, path: &str, content: &str) {
    let request = Request::builder()
        .uri(format!("/api/projects/{OWNER}/{PROJECT}/repo/file?path={path}"))
        .method("PUT")
        .header(header::COOKIE, cookie)
        .header(header::CONTENT_TYPE, "text/plain")
        .body(Body::from(content.to_string()))
        .expect("request");
    assert!(app.clone().oneshot(request).await.expect("response").status().is_success(), "writing {path}");
}

#[tokio::test]
async fn a_page_serves_a_component_changed_behind_its_back() {
    let root = temp_dir("page-refresh");
    let mut config = zebflow::platform::PlatformConfig::default();
    config.data_root = root.0.clone();
    config.default_password = "test-pass".to_string();
    let platform = std::sync::Arc::new(zebflow::platform::PlatformService::from_config(config).expect("platform"));
    let app = zebflow::platform::web::router(platform.clone()).await;
    let cookie = login(&app).await;

    let badge = |word: &str| format!("export default function Badge() {{\n  return <span>badge-{word}</span>;\n}}\n");
    put_file(&app, &cookie, "components/badge.tsx", &badge("one")).await;
    put_file(&app, &cookie, "pages/refresh.tsx", "import Badge from \"@/components/badge\";\nexport default function Page() {\n  return <main><Badge /></main>;\n}\n").await;
    publish(&app, &cookie, "refresh-page", "| trigger.webhook --route /refresh-page --method GET | web.response.send --template pages/refresh.tsx").await;
    let page = format!("/wh/{OWNER}/{PROJECT}/refresh-page");
    assert!(get_text(&app, &page).await.contains("badge-one"), "first render");

    // Written below the writers that refresh pages: the change record hears it.
    platform.projects.write_repo_file(OWNER, PROJECT, "components/badge.tsx", &badge("two")).expect("write");
    let mut body = String::new();
    for _ in 0..50 {
        body = get_text(&app, &page).await;
        if body.contains("badge-two") {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    assert!(body.contains("badge-two"), "the page serves the changed component: {}", &body[..body.len().min(400)]);
}
