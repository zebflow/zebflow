use axum::body::{Body, to_bytes};
use axum::http::Request;
use tower::ServiceExt;

#[tokio::test]
async fn axum_demo_renders_list_hydration_route() {
    let app = zebflow::rwe::axum_demo::build_demo_router().expect("build demo router");

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/list-hydration")
                .method("GET")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(resp.status(), axum::http::StatusCode::OK);
    let body = to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let html = String::from_utf8(body.to_vec()).expect("utf8 body");
    assert!(html.contains("Keyed List + Hydration Islands"));
    assert!(html.contains("Alpha (#101)"));
    assert!(html.contains("data-rwe-for-template=\"1\""));
}

#[tokio::test]
async fn axum_demo_renders_recycling_route() {
    let app = zebflow::rwe::axum_demo::build_demo_router().expect("build demo router");

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/recycling")
                .method("GET")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(resp.status(), axum::http::StatusCode::OK);
    let body = to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let html = String::from_utf8(body.to_vec()).expect("utf8 body");
    assert!(html.contains("Recycle Better, Restore Nature Faster"));
    assert!(html.contains("Sort at the source"));
    assert!(html.contains("data-rwe-runtime"));
}

#[tokio::test]
async fn axum_demo_renders_showcase_route() {
    let app = zebflow::rwe::axum_demo::build_demo_router().expect("build demo router");

    let resp = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/showcase")
                .method("GET")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(resp.status(), axum::http::StatusCode::OK);
    let body = to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let html = String::from_utf8(body.to_vec()).expect("utf8 body");
    assert!(html.contains("Deploy Once,"));
    assert!(html.contains("ui_component.tsx"));
    assert!(html.contains("data-rwe-runtime"));
    assert!(html.contains("https://unpkg.com/lucide@0.469.0/dist/umd/lucide.min.js"));
    assert!(html.contains("data-lucide=\"shield\""));
}

#[tokio::test]
async fn axum_demo_renders_state_sharing_with_query_seed() {
    let app = zebflow::rwe::axum_demo::build_demo_router().expect("build demo router");

    let resp = app
        .oneshot(
            Request::builder()
                .uri("/state-sharing?seed=33")
                .method("GET")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(resp.status(), axum::http::StatusCode::OK);
    let body = to_bytes(resp.into_body(), usize::MAX)
        .await
        .expect("body bytes");
    let html = String::from_utf8(body.to_vec()).expect("utf8 body");
    assert!(html.contains("SSR seed value: 33"));
    assert!(html.contains("A Root Component"));
    assert!(html.contains("F reads shared value"));
}
