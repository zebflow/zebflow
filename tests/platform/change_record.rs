//! `project.md` § Change record: every change in a project's repository names
//! who made it and how — a signed-in person in Studio, an agent through one of
//! a person's MCP keys — and a write that changes nothing is not a change.

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

async fn send(app: &axum::Router, cookie: &str, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, Value) {
    let mut builder = Request::builder().uri(uri).method(method).header(header::COOKIE, cookie);
    let body = match body {
        Some(json) => {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
            Body::from(json.to_string())
        }
        None => Body::empty(),
    };
    let response = app.clone().oneshot(builder.body(body).expect("request")).await.expect("response");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("body");
    (status, serde_json::from_slice(&bytes).unwrap_or(Value::Null))
}

/// The repository write route takes the file's own bytes as the body.
async fn put_file(app: &axum::Router, cookie: &str, uri: &str, content: &str) -> StatusCode {
    let request = Request::builder()
        .uri(uri)
        .method("PUT")
        .header(header::COOKIE, cookie)
        .header(header::CONTENT_TYPE, "text/plain")
        .body(Body::from(content.to_string()))
        .expect("request");
    app.clone().oneshot(request).await.expect("response").status()
}

async fn login(app: &axum::Router, identifier: &str, password: &str) -> String {
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

async fn mcp_call(app: &axum::Router, secret: &str, tool: &str, arguments: Value) -> Result<String, String> {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/api/projects/{OWNER}/{PROJECT}/mcp"))
                .method("POST")
                .header(header::AUTHORIZATION, format!("Bearer {secret}"))
                .header(header::CONTENT_TYPE, "application/json")
                .header(header::ACCEPT, "application/json, text/event-stream")
                .body(Body::from(
                    json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": { "name": tool, "arguments": arguments } }).to_string(),
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

/// The record for `path`, newest first, as `(person, via, action, path)`.
async fn record(app: &axum::Router, cookie: &str, path: &str) -> Vec<(String, String, String, String)> {
    let (status, body) = send(app, cookie, "GET", &format!("/api/projects/{OWNER}/{PROJECT}/changes?path={path}"), None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["changes"]
        .as_array()
        .expect("changes")
        .iter()
        .map(|l| {
            let s = |k: &str| l[k].as_str().unwrap_or_default().to_string();
            (s("person"), s("via"), s("action"), s("path"))
        })
        .collect()
}

#[tokio::test]
async fn every_change_names_who_made_it_and_how() {
    let root = TestDir(std::env::temp_dir().join(format!(
        "zebflow-platform-change-record-{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    )));
    let mut config = PlatformConfig::default();
    config.data_root = root.0.clone();
    config.default_password = "test-pass".to_string();
    let platform = Arc::new(PlatformService::from_config(config).expect("platform"));
    let app = zebflow::platform::web::router(platform.clone()).await;
    let owner = login(&app, "superadmin", "test-pass").await;

    // A developer, and a key that acts as them.
    let dev_password = uuid::Uuid::new_v4().to_string();
    platform
        .users
        .create_user(&zebflow::platform::CreateUserRequest {
            owner: "dev-one".into(),
            password: dev_password.clone(),
            role: "member".into(),
            git_name: String::new(),
            git_email: String::new(),
        })
        .expect("user");
    platform
        .project_members
        .upsert_member(
            OWNER,
            OWNER,
            PROJECT,
            &zebflow::platform::model::UpsertProjectMemberRequest {
                user_id: "dev-one".into(),
                role_preset: zebflow::platform::model::ProjectAccessRolePreset::Developer,
                custom_policy_ids: Vec::new(),
                mcp_capability_ceiling: Vec::new(),
            },
        )
        .expect("member");
    let (status, made) = send(
        &app,
        &owner,
        "POST",
        &format!("/api/projects/{OWNER}/{PROJECT}/mcp/keys"),
        Some(json!({ "name": "laptop", "person": "dev-one", "capabilities": ["project.read", "templates.read", "templates.write", "templates.create", "templates.delete"] })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{made}");
    let key = made["created"]["secret"].as_str().expect("secret").to_string();

    // Studio: the owner writes a file, saves it unchanged, then changes it.
    let file = format!("/api/projects/{OWNER}/{PROJECT}/repo/file?path=pages/notes.tsx");
    for content in ["export default function N() { return <p>a</p>; }", "export default function N() { return <p>a</p>; }", "export default function N() { return <p>b</p>; }"] {
        assert_eq!(put_file(&app, &owner, &file, content).await, StatusCode::OK);
    }

    // MCP: the developer's agent edits the same file and writes another.
    mcp_call(&app, &key, "file_edit", json!({ "rel_path": "pages/notes.tsx", "old_string": "<p>b</p>", "new_string": "<p>c</p>" }))
        .await
        .expect("edit over mcp");
    mcp_call(&app, &key, "file_write", json!({ "rel_path": "pages/more.tsx", "content": "export default function M() { return <p>m</p>; }" }))
        .await
        .expect("write over mcp");

    // Studio again: the owner moves one and deletes it.
    let (status, body) = send(&app, &owner, "POST", &format!("/api/projects/{OWNER}/{PROJECT}/repo/move"), Some(json!({ "from_path": "pages/more.tsx", "to_path": "pages/moved.tsx" }))).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = send(&app, &owner, "DELETE", &format!("/api/projects/{OWNER}/{PROJECT}/repo/file?path=pages/moved.tsx"), None).await;
    assert_eq!(status, StatusCode::OK, "{body}");

    let mcp = "mcp · laptop".to_string();
    let line = |person: &str, via: &str, action: &str, path: &str| (person.to_string(), via.to_string(), action.to_string(), path.to_string());
    assert_eq!(
        record(&app, &owner, "pages").await,
        vec![
            line("superadmin", "studio", "delete", "pages/moved.tsx"),
            line("superadmin", "studio", "move", "pages/more.tsx"),
            line("dev-one", &mcp, "create", "pages/more.tsx"),
            line("dev-one", &mcp, "edit", "pages/notes.tsx"),
            line("superadmin", "studio", "edit", "pages/notes.tsx"),
            line("superadmin", "studio", "create", "pages/notes.tsx"),
        ],
        "newest first; the unchanged save is not there"
    );

    // Anyone who may read the project reads the record, and so does an agent.
    let dev = login(&app, "dev-one", &dev_password).await;
    assert_eq!(record(&app, &dev, "pages/notes.tsx").await.len(), 3);
    let listed: Value = serde_json::from_str(&mcp_call(&app, &key, "change_list", json!({ "path": "pages/more.tsx" })).await.expect("change_list"))
        .expect("json");
    assert_eq!(listed.as_array().map(Vec::len), Some(2), "{listed}");
    assert_eq!(listed[0]["to_path"], "pages/moved.tsx", "{listed}");
}
