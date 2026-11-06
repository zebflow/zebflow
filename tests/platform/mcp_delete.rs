//! An agent removes what it made over MCP alone: `pipeline_delete` switches a
//! live pipeline off before its file goes, and `file_delete` removes one file
//! or an empty folder, never a folder that still holds something.

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
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

async fn get_status(app: &axum::Router, uri: &str) -> StatusCode {
    let request = Request::builder().uri(uri).method("GET").body(Body::empty()).expect("request");
    app.clone().oneshot(request).await.expect("response").status()
}

async fn mcp_token(app: &axum::Router, cookie: &str, capabilities: &[&str]) -> String {
    let request = Request::builder()
        .uri(format!("/api/projects/{OWNER}/{PROJECT}/mcp/session"))
        .method("POST")
        .header(header::COOKIE, cookie)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(json!({ "capabilities": capabilities }).to_string()))
        .expect("request");
    let response = app.clone().oneshot(request).await.expect("session");
    let status = response.status();
    let body: Value = serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.expect("body")).expect("json");
    assert_eq!(status, StatusCode::OK, "{body}");
    body["session"]["token"].as_str().expect("token").to_string()
}

async fn mcp_call(app: &axum::Router, token: &str, tool: &str, arguments: Value) -> Result<String, String> {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/projects/{OWNER}/{PROJECT}/mcp"))
                .method("POST")
                .header(header::AUTHORIZATION, format!("Bearer {token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::ACCEPT, "application/json, text/event-stream")
                .body(Body::from(
                    json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": tool, "arguments": arguments } })
                        .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("mcp response");
    let text = String::from_utf8_lossy(&to_bytes(response.into_body(), usize::MAX).await.expect("body")).to_string();
    let json_text = text.lines().find_map(|l| l.strip_prefix("data:")).map(str::trim).unwrap_or(text.trim());
    let message: Value = serde_json::from_str(json_text).unwrap_or_else(|_| panic!("mcp answer: {text}"));
    if let Some(error) = message.get("error") {
        return Err(error["message"].as_str().unwrap_or_default().to_string());
    }
    if message["result"]["isError"] == true {
        return Err(message["result"]["content"][0]["text"].as_str().unwrap_or_default().to_string());
    }
    Ok(message["result"]["content"][0]["text"].as_str().unwrap_or_default().to_string())
}

#[tokio::test]
async fn an_agent_deletes_its_own_pipeline_and_files_over_mcp() {
    let root = TestDir(std::env::temp_dir().join(format!(
        "zebflow-platform-mcp-delete-{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    )));
    let mut config = PlatformConfig::default();
    config.data_root = root.0.clone();
    config.default_password = "test-pass".to_string();
    let platform = Arc::new(PlatformService::from_config(config).expect("platform"));
    let app = zebflow::platform::web::router(platform.clone()).await;
    let cookie = login(&app).await;
    let token = mcp_token(
        &app,
        &cookie,
        &["pipelines.read", "pipelines.write", "pipelines.delete", "templates.read", "templates.write", "templates.delete"],
    )
    .await;

    // A live pipeline: deleting it stops the route before the file goes.
    let path = "pipelines/sandbox/preview.zf.json";
    mcp_call(
        &app,
        &token,
        "pipeline_register",
        json!({ "file_rel_path": path, "body": "| trigger.webhook --route /sandbox/preview --method GET | web.response.send --body \"{{ { ok: true } }}\"" }),
    )
    .await
    .expect("register");
    mcp_call(&app, &token, "pipeline_activate", json!({ "file_rel_path": path })).await.expect("activate");
    let route = format!("/wh/{OWNER}/{PROJECT}/sandbox/preview");
    assert_eq!(get_status(&app, &route).await, StatusCode::OK, "live before the delete");

    let deleted: Value = serde_json::from_str(&mcp_call(&app, &token, "pipeline_delete", json!({ "file_rel_path": path })).await.expect("delete"))
        .expect("answer json");
    assert_eq!(deleted["ok"], true);
    assert_eq!(deleted["was_active"], true);
    assert_eq!(get_status(&app, &route).await, StatusCode::NOT_FOUND, "the route stops with the pipeline");
    assert!(mcp_call(&app, &token, "pipeline_get", json!({ "file_rel_path": path })).await.is_err(), "the source is gone");
    assert!(
        mcp_call(&app, &token, "pipeline_delete", json!({ "file_rel_path": path })).await.is_err(),
        "a second delete names the missing pipeline"
    );

    // Files: one at a time; a folder only once it is empty.
    mcp_call(&app, &token, "file_write", json!({ "rel_path": "sandbox/preview.tsx", "content": "export default function P() { return <p>x</p>; }" }))
        .await
        .expect("write");
    let refused = mcp_call(&app, &token, "file_delete", json!({ "rel_path": "sandbox" })).await.expect_err("a folder that holds a file");
    assert!(refused.contains("still holds files"), "{refused}");
    let refused = mcp_call(&app, &token, "file_delete", json!({ "rel_path": "pipelines/x.zf.json" })).await.expect_err("a pipeline file");
    assert!(refused.contains("pipeline_delete"), "{refused}");
    assert!(mcp_call(&app, &token, "file_delete", json!({ "rel_path": "zebflow.yaml" })).await.is_err(), "the platform's own file stays");

    let gone: Value = serde_json::from_str(&mcp_call(&app, &token, "file_delete", json!({ "rel_path": "sandbox/preview.tsx" })).await.expect("delete file"))
        .expect("answer json");
    assert_eq!(gone["ok"], true);
    assert!(mcp_call(&app, &token, "file_read", json!({ "rel_path": "sandbox/preview.tsx" })).await.is_err(), "the file is gone");
    mcp_call(&app, &token, "file_delete", json!({ "rel_path": "sandbox" })).await.expect("the empty folder");

    // A session without the delete capabilities is refused, naming what it lacks.
    let reader = mcp_token(&app, &cookie, &["pipelines.read", "templates.read", "templates.write"]).await;
    mcp_call(&app, &reader, "file_write", json!({ "rel_path": "notes.md", "content": "x" })).await.expect("write");
    let refused = mcp_call(&app, &reader, "file_delete", json!({ "rel_path": "notes.md" })).await.expect_err("no templates.delete");
    assert!(refused.contains("templates.delete"), "{refused}");
}
