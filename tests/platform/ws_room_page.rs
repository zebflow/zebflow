//! A page learns where its rooms live from the server that answered it: the
//! room base `zeb/rooms` reads rides on the page-data element, worked out for
//! the address the page was asked for on — the default mount or the host's
//! own `ws` route on a project host, the platform form on the platform form —
//! and absent while the project's `ws` surface is off or the host has no way
//! to it (`docs/contracts/addressing.md` §2).

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::json;
use tower::ServiceExt;

use super::published_mcp::{OWNER, PROJECT, addressing, app_at, login, publish, temp_dir};

const DEV_HOST: &str = "default.superadmin.localhost";

async fn page(app: &axum::Router, uri: &str, host: Option<&str>) -> String {
    let mut request = Request::builder().uri(uri).method("GET");
    if let Some(host) = host {
        request = request.header(header::HOST, host);
    }
    let response = app.clone().oneshot(request.body(Body::empty()).expect("request")).await.expect("response");
    assert_eq!(response.status(), StatusCode::OK, "{uri} on {host:?}");
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("body");
    let html = String::from_utf8_lossy(&bytes).to_string();
    assert!(!html.contains("RWE component error"), "{html}");
    assert!(html.contains("id=\"__rwe_payload\">"), "the page-data element is still found the usual way: {html}");
    html
}

/// The room base on the page-data element, if any.
fn room_base(html: &str) -> Option<String> {
    let start = html.find("data-rooms=\"")? + "data-rooms=\"".len();
    Some(html[start..start + html[start..].find('"')?].to_string())
}

#[tokio::test]
async fn a_page_names_the_room_base_of_the_address_it_was_served_on() {
    let root = temp_dir("ws-room-page");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    let request = Request::builder()
        .uri(format!("/api/projects/{OWNER}/{PROJECT}/repo/file?path=pages/arena.tsx"))
        .method("PUT")
        .header(header::COOKIE, &cookie)
        .header(header::CONTENT_TYPE, "text/plain")
        .body(Body::from("export default function Arena() { return <main>arena</main>; }"))
        .expect("request");
    assert_eq!(app.clone().oneshot(request).await.expect("write").status(), StatusCode::OK);
    publish(&app, &cookie, "arena", "| trigger.webhook --route /arena --method GET | web.response.send --template pages/arena.tsx").await;

    // `ws` starts off: no page has rooms.
    assert_eq!(room_base(&page(&app, "/arena", Some(DEV_HOST)).await), None);
    assert_eq!(room_base(&page(&app, &format!("/wh/{OWNER}/{PROJECT}/arena"), None).await), None);

    addressing(
        &app,
        &cookie,
        json!({
            "hosts": ["play.example.com", "www.example.com"],
            "routes": [
                { "host": "play.example.com", "path": "/", "surface": "pages" },
                { "host": "play.example.com", "path": "/live/", "surface": "ws" },
                { "host": "www.example.com", "path": "/", "surface": "pages" }
            ],
            "disabled": ["files", "ms", "mcp"]
        }),
    )
    .await;
    assert_eq!(room_base(&page(&app, "/arena", Some(DEV_HOST)).await).as_deref(), Some("/_ws/rooms/"));
    assert_eq!(
        room_base(&page(&app, "/arena", Some("play.example.com")).await).as_deref(),
        Some("/live/rooms/"),
        "the operator route that mounts ws on that host"
    );
    assert_eq!(
        room_base(&page(&app, &format!("/wh/{OWNER}/{PROJECT}/arena"), None).await).as_deref(),
        Some("/ws/superadmin/default/rooms/")
    );
    let elsewhere = page(&app, "/arena", Some("www.example.com")).await;
    assert_eq!(room_base(&elsewhere), None, "a host whose routes leave ws out has no rooms");
    assert!(!elsewhere.contains("/ws/superadmin/default"), "a project host never names the owner and project");
}
