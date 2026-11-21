//! `trigger.repo`: a pipeline runs when files under a repository path change —
//! once per burst of saves, never for a neighbouring folder, and not at all
//! once it is switched off.

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use tower::ServiceExt;

use super::published_mcp::{OWNER, PROJECT, app_at, dsl, login, publish, send, temp_dir};

const PIPELINE: &str = "pipelines/tests/docs-rebuild.zf.json";

async fn put_file(app: &axum::Router, cookie: &str, path: &str, content: &str) {
    let request = Request::builder()
        .uri(format!("/api/projects/{OWNER}/{PROJECT}/repo/file?path={path}"))
        .method("PUT")
        .header(header::COOKIE, cookie)
        .header(header::CONTENT_TYPE, "text/plain")
        .body(Body::from(content.to_string()))
        .expect("request");
    let status = app.clone().oneshot(request).await.expect("response").status();
    assert!(status.is_success(), "writing {path}: {status}");
}

async fn runs(app: &axum::Router, cookie: &str) -> Vec<Value> {
    let (status, body) = send(app, cookie, "GET", &format!("/api/projects/{OWNER}/{PROJECT}/pipelines/invocations?pipeline={PIPELINE}"), Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["entries"].as_array().cloned().unwrap_or_default()
}

/// Waits until there are `n` runs, then a little longer to be sure there are
/// no more.
async fn settle_at(app: &axum::Router, cookie: &str, n: usize) -> Vec<Value> {
    for _ in 0..80 {
        if runs(app, cookie).await.len() >= n {
            break;
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }
    tokio::time::sleep(std::time::Duration::from_millis(2_600)).await;
    runs(app, cookie).await
}

#[tokio::test]
async fn a_burst_of_saves_under_the_path_is_one_run() {
    let root = temp_dir("repo-trigger");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    publish(
        &app,
        &cookie,
        "docs-rebuild",
        "| trigger.repo --path docs | javascript.script.run -- \"return { changed: input.repo.changes.map((c) => c.path) }\"",
    )
    .await;

    for page in ["docs/a.md", "docs/b.md", "docs/a.md"] {
        put_file(&app, &cookie, page, &format!("# {page} {}", uuid::Uuid::new_v4())).await;
    }
    let after_burst = settle_at(&app, &cookie, 1).await;
    assert_eq!(after_burst.len(), 1, "three saves are one run: {after_burst:?}");
    let run = after_burst[0].to_string();
    assert!(run.contains("trigger.repo"), "{run}");
    assert!(run.contains("\"ok\""), "the run succeeded: {run}");

    put_file(&app, &cookie, "docs-old/a.md", "# old").await;
    put_file(&app, &cookie, "pages/notes.md", "# elsewhere").await;
    assert_eq!(settle_at(&app, &cookie, 2).await.len(), 1, "a neighbouring folder is not the path");

    let off = dsl(&app, &cookie, &format!("deactivate pipeline {PIPELINE}")).await;
    assert_eq!(off["ok"], json!(true), "{off}");
    put_file(&app, &cookie, "docs/c.md", "# after").await;
    assert_eq!(settle_at(&app, &cookie, 2).await.len(), 1, "a pipeline switched off does not run");
}
