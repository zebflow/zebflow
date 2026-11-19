//! `zebflow.command.run` behind a webhook (`kinds/credential` § zebflow_control):
//! a downstream app sends commands, the node carries them out inside the fence
//! its credential draws, and refuses everything past it.

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use tower::ServiceExt;

use zebflow::platform::services::ops::PlatformOps;
use zebflow::platform::{PlatformConfig, PlatformService};

const OWNER: &str = "superadmin";
const PROJECT: &str = "default";

struct TestDir(std::path::PathBuf);

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

async fn call(app: &axum::Router, cookie: Option<&str>, method: &str, uri: &str, body: Option<Value>) -> (StatusCode, String) {
    let mut builder = Request::builder().uri(uri).method(method);
    if let Some(cookie) = cookie {
        builder = builder.header(header::COOKIE, cookie);
    }
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
    (status, String::from_utf8_lossy(&bytes).to_string())
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

async fn credential(app: &axum::Router, cookie: &str, id: &str, kind: &str, secret: Value) {
    let (status, body) = call(
        app,
        Some(cookie),
        "POST",
        &format!("/api/projects/{OWNER}/{PROJECT}/credentials"),
        Some(json!({ "credential_id": id, "title": id, "kind": kind, "secret": secret, "notes": "" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

/// The app's control door: a webhook handing its body to the node, answering
/// a refusal with 403 and the reason, as a real door does.
fn door(fence: &str) -> String {
    format!(
        "[t] trigger.webhook --route /zf/control --method POST\n\
         [c] zebflow.command.run --credential {fence} --command \"{{{{ input.webhook.body }}}}\" --as tester\n\
         [ok] web.response.send --body \"{{{{ input.command }}}}\"\n\
         [no] web.response.send --status 403 --body \"{{{{ input.command }}}}\"\n\
         [t] -> [c]\n[c] -> [ok]\n[c]:error -> [no]"
    )
}

/// One command through the app's control webhook.
async fn command(app: &axum::Router, body: Value) -> (StatusCode, String) {
    call(app, None, "POST", &format!("/wh/{OWNER}/{PROJECT}/zf/control"), Some(body)).await
}

#[tokio::test]
async fn a_control_webhook_works_inside_its_fence_and_nowhere_else() {
    let root = TestDir(std::env::temp_dir().join(format!(
        "zebflow-platform-zebflow-command-{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    )));
    let mut config = PlatformConfig::default();
    config.data_root = root.0.clone();
    config.default_password = "test-pass".to_string();
    let platform = Arc::new(PlatformService::from_config(config).expect("platform"));
    let app = zebflow::platform::web::router(platform.clone()).await;
    let cookie = login(&app).await;

    // The fence, and two database credentials: one delegated, one not.
    credential(
        &app,
        &cookie,
        "zf_modules",
        "zebflow_control",
        json!({
            "scope": "modules/",
            "routes": "/m/",
            "tools": ["file_write", "file_read", "file_delete", "pipeline_register", "pipeline_activate"],
            "delegate": ["pg_modules"],
            "record_as": "site-a-app"
        }),
    )
    .await;
    let pg = json!({ "host": "db.example.com", "port": "5432", "database": "app", "user": "u", "password": uuid::Uuid::new_v4().to_string(), "sslmode": "disable" });
    credential(&app, &cookie, "pg_modules", "postgres", pg.clone()).await;
    credential(&app, &cookie, "main_pg", "postgres", pg).await;

    // The app's control door: a webhook handing its body to the node.
    let ops = PlatformOps::new(platform.clone(), OWNER, PROJECT);
    let door = door("zf_modules");
    let registered = ops.pipeline_register(&door, Some("pipelines/control.zf.json"), None, None, None, None).await;
    assert!(!registered.text.starts_with("Error:"), "{}", registered.text);
    let activated = ops.pipeline_activate("pipelines/control.zf.json").await;
    assert!(!activated.text.starts_with("Error:"), "{}", activated.text);

    // Inside the fence: a file, then a pipeline that answers under /m/.
    let (status, body) = command(&app, json!({ "tool": "file_write", "arguments": { "rel_path": "modules/rain/notes.md", "content": "# Rain" } })).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert!(root.0.join("users/superadmin/default/repo/modules/rain/notes.md").is_file());

    let made = "| trigger.webhook --route /m/rain/latest --method GET | web.response.send --body \"{{ { rain_mm: 2.5 } }}\"";
    let (status, body) = command(
        &app,
        json!([
            { "tool": "pipeline_register", "arguments": { "file_rel_path": "modules/rain/api/latest.zf.json", "body": made } },
            { "tool": "pipeline_activate", "arguments": { "file_rel_path": "modules/rain/api/latest.zf.json" } }
        ]),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let answer: Value = serde_json::from_str(&body).expect("json");
    assert_eq!(answer["ok"], true, "{answer}");
    assert_eq!(answer["results"].as_array().map(Vec::len), Some(2), "{answer}");
    let (status, body) = call(&app, None, "GET", &format!("/wh/{OWNER}/{PROJECT}/m/rain/latest"), None).await;
    assert_eq!(status, StatusCode::OK, "the made pipeline answers: {body}");
    assert!(body.contains("2.5"), "{body}");

    // Past the fence: each is refused, and nothing is written.
    let refusals = [
        (json!({ "tool": "file_write", "arguments": { "rel_path": "pages/home.tsx", "content": "x" } }), "outside modules/"),
        (json!({ "tool": "file_write", "arguments": { "rel_path": "modules/../pages/home.tsx", "content": "x" } }), "outside modules/"),
        (json!({ "tool": "file_write", "arguments": { "rel_path": "modules-old/x.md", "content": "x" } }), "outside modules/"),
        (json!({ "tool": "pipeline_delete", "arguments": { "file_rel_path": "modules/rain/api/latest.zf.json" } }), "not opened"),
        (json!({ "tool": "pipeline_register", "arguments": { "file_rel_path": "pipelines/admin.zf.json", "body": made } }), "outside modules/"),
        (json!({ "tool": "pipeline_register", "arguments": { "file_rel_path": "modules/rain/api/admin.zf.json",
            "body": "| trigger.webhook --route /admin/x | web.response.send --body ok" } }), "outside /m/"),
        (json!({ "tool": "pipeline_register", "arguments": { "file_rel_path": "modules/rain/api/db.zf.json",
            "body": "| trigger.webhook --route /m/rain/db | postgres.query.run --credential main_pg -- \"SELECT 1\"" } }), "not delegated"),
        (json!({ "tool": "pipeline_register", "arguments": { "file_rel_path": "modules/rain/api/kv.zf.json",
            "body": "| trigger.webhook --route /m/rain/kv | kv.entry.get --key login" } }), "denies kv.*"),
        // The review's findings, each as an attack through the door.
        (json!({ "tool": "file_write", "arguments": { "rel_path": "modules/rain/api/latest.zf.json", "content": "{}" } }), "pipeline file"),
        (json!({ "tool": "pipeline_register", "arguments": { "file_rel_path": "modules/../pages/pwn.zf.json", "body": made } }), "not a plain path"),
        (json!({ "tool": "file_delete", "arguments": { "rel_path": "modules/" } }), "own folder"),
        (json!({ "tool": "pipeline_register", "arguments": { "file_rel_path": "modules/rain/api/enc.zf.json",
            "body": "| trigger.webhook --route /m/%2e%2e/admin | web.response.send --body x" } }), "outside /m/"),
        (json!({ "tool": "pipeline_register", "arguments": { "file_rel_path": "modules/rain/api/dyn.zf.json",
            "body": "| trigger.webhook --route /m/rain/dyn | postgres.query.run --credential \"{{ input.webhook.body.c }}\" -- \"SELECT 1\"" } }), "expression"),
        (json!({ "tool": "pipeline_register", "arguments": { "file_rel_path": "modules/rain/api/again.zf.json",
            "body": "| trigger.webhook --route /m/rain/again | zebflow.command.run --credential zf_modules --command x" } }), "may never use zebflow"),
    ];
    for (body, why) in refusals {
        let (status, answer) = command(&app, body.clone()).await;
        assert_ne!(status, StatusCode::OK, "{body} was not refused: {answer}");
        assert!(answer.contains(why), "{body}: expected '{why}' in {answer}");
    }
    assert!(!root.0.join("users/superadmin/default/repo/pages/home.tsx").exists());
    assert!(!root.0.join("users/superadmin/default/repo/modules-old").exists());
    assert!(ops.pipeline_get("modules/rain/api/kv.zf.json", None).text.starts_with("Error:")
        || !root.0.join("users/superadmin/default/repo/modules/rain/api/kv.zf.json").exists(), "a refused pipeline is not saved");

    // A link inside the folder that leads out of it is not followed.
    #[cfg(unix)]
    {
        let repo = root.0.join("users/superadmin/default/repo");
        std::fs::create_dir_all(repo.join("pages")).expect("pages");
        std::os::unix::fs::symlink(repo.join("pages"), repo.join("modules/rain/out")).expect("link");
        let (status, answer) = command(&app, json!({ "tool": "file_write", "arguments": { "rel_path": "modules/rain/out/pwn.tsx", "content": "x" } })).await;
        assert_ne!(status, StatusCode::OK, "{answer}");
        assert!(answer.contains("through a link"), "{answer}");
        assert!(!repo.join("pages/pwn.tsx").exists());
        std::fs::remove_file(repo.join("modules/rain/out")).expect("unlink");
    }

    // A valid pipeline file planted straight into the folder, using a store
    // the fence forbids, is caught when it is switched on.
    let graph = zebflow::platform::shell::parser::build_pipeline_graph(
        "smuggled",
        "| trigger.webhook --route /m/rain/smuggled | kv.entry.get --key login | web.response.send --body x",
    )
    .expect("graph");
    let bytes = zebflow::contracts::kinds::encode_pipeline_graph(graph).expect("encode");
    platform
        .projects
        .write_repo_bytes(OWNER, PROJECT, "modules/rain/api/smuggled.zf.json", &bytes)
        .expect("plant");
    let (status, answer) = command(&app, json!({ "tool": "pipeline_activate", "arguments": { "file_rel_path": "modules/rain/api/smuggled.zf.json" } })).await;
    assert_ne!(status, StatusCode::OK, "a planted pipeline is not switched on unchecked: {answer}");
    assert!(answer.contains("denies kv.*"), "{answer}");

    // The change record names the fence and the person.
    let (status, body) = call(&app, Some(&cookie), "GET", &format!("/api/projects/{OWNER}/{PROJECT}/changes?path=modules/rain"), None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let record: Value = serde_json::from_str(&body).expect("json");
    let lines = record["changes"].as_array().expect("changes");
    assert!(!lines.is_empty(), "{record}");
    // The planted file above was written by the test itself, not through the
    // door, and is recorded as the system's.
    let through_door: Vec<&Value> = lines.iter().filter(|l| !l["path"].as_str().unwrap_or_default().ends_with("smuggled.zf.json")).collect();
    assert_eq!(through_door.len(), 2, "the note and the made pipeline: {record}");
    assert!(
        through_door.iter().all(|l| l["via"] == "control · site-a-app" && l["person"] == "tester"),
        "every change through the door names the fence and the person: {record}"
    );
}

#[tokio::test]
async fn a_credential_that_is_not_a_fence_opens_nothing() {
    let root = TestDir(std::env::temp_dir().join(format!(
        "zebflow-platform-zebflow-command-kind-{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    )));
    let mut config = PlatformConfig::default();
    config.data_root = root.0.clone();
    config.default_password = "test-pass".to_string();
    let platform = Arc::new(PlatformService::from_config(config).expect("platform"));
    let app = zebflow::platform::web::router(platform.clone()).await;
    let cookie = login(&app).await;
    credential(&app, &cookie, "pg_modules", "postgres", json!({ "host": "db.example.com", "password": uuid::Uuid::new_v4().to_string() })).await;
    let ops = PlatformOps::new(platform.clone(), OWNER, PROJECT);
    let door = door("pg_modules");
    ops.pipeline_register(&door, Some("pipelines/control.zf.json"), None, None, None, None).await;
    ops.pipeline_activate("pipelines/control.zf.json").await;
    let (status, answer) = command(&app, json!({ "tool": "file_write", "arguments": { "rel_path": "modules/a.md", "content": "x" } })).await;
    assert_ne!(status, StatusCode::OK, "{answer}");
    assert!(answer.contains("not zebflow_control"), "{answer}");
    assert!(!root.0.join("users/superadmin/default/repo/modules/a.md").exists());
}
