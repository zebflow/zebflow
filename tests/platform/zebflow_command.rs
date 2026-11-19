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

/// The credential with `id` in the project's credential list, as Studio reads it.
async fn listed_credential(app: &axum::Router, cookie: &str, id: &str) -> Option<Value> {
    let (status, body) = call(app, Some(cookie), "GET", &format!("/api/projects/{OWNER}/{PROJECT}/credentials"), None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let body: Value = serde_json::from_str(&body).expect("json");
    let items = body.get("items").or_else(|| body.get("credentials")).and_then(Value::as_array).cloned().unwrap_or_default();
    items.into_iter().find(|item| item["credential_id"] == id)
}

/// `app-control.md` § Rules 5 to 8: a door sees what it was given and what it
/// made, creates only the kinds its fence names and only outside addresses,
/// changes and deletes only its own, and a made pipeline may use both.
#[tokio::test]
async fn a_door_manages_only_its_own_credentials() {
    let root = TestDir(std::env::temp_dir().join(format!(
        "zebflow-platform-zebflow-command-creds-{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    )));
    let mut config = PlatformConfig::default();
    config.data_root = root.0.clone();
    config.default_password = "test-pass".to_string();
    let platform = Arc::new(PlatformService::from_config(config).expect("platform"));
    let app = zebflow::platform::web::router(platform.clone()).await;
    let cookie = login(&app).await;
    let password = uuid::Uuid::new_v4().to_string();
    let pg = |host: &str| json!({ "host": host, "port": "5432", "database": "app", "user": "reader", "password": password, "sslmode": "require" });

    credential(
        &app,
        &cookie,
        "zf_lab",
        "zebflow_control",
        json!({
            "scope": "modules/",
            "routes": "/m/",
            "tools": ["credential_list", "credential_create", "credential_update", "credential_delete", "pipeline_register"],
            "delegate": ["pg_given"],
            "manage_kinds": ["postgres"],
            "manage_limit": 2,
            "record_as": "lab-app"
        }),
    )
    .await;
    credential(&app, &cookie, "pg_given", "postgres", pg("203.0.113.20")).await;
    credential(&app, &cookie, "main_pg", "postgres", pg("203.0.113.21")).await;
    let ops = PlatformOps::new(platform.clone(), OWNER, PROJECT);
    ops.pipeline_register(&door("zf_lab"), Some("pipelines/control.zf.json"), None, None, None, None).await;
    let activated = ops.pipeline_activate("pipelines/control.zf.json").await;
    assert!(!activated.text.starts_with("Error:"), "{}", activated.text);

    let create = |id: &str, kind: &str, secret: Value| json!({ "tool": "credential_create", "arguments": { "credential_id": id, "kind": kind, "secret": secret } });
    let (status, answer) = command(&app, create("ext_a", "postgres", pg("203.0.113.10"))).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert!(!answer.contains(&password), "a password is never answered: {answer}");

    for (body, word) in [
        (create("ext_local", "postgres", pg("localhost")), "not an outside address"),
        (create("ext_private", "postgres", pg("10.1.2.3")), "not an outside address"),
        (create("ext_my", "mysql", pg("203.0.113.11")), "outside what this door manages"),
        (create("ext_fence", "zebflow_control", json!({ "scope": "x/", "routes": "/x/", "tools": ["file_read"] })), "may never create"),
        (create("main_pg", "postgres", pg("203.0.113.12")), "already exists"),
        (json!({ "tool": "credential_update", "arguments": { "credential_id": "ext_a", "secret": { "host": "127.0.0.1" } } }), "not an outside address"),
        (json!({ "tool": "credential_update", "arguments": { "credential_id": "pg_given", "title": "mine now" } }), "delegated to this door"),
        (json!({ "tool": "credential_update", "arguments": { "credential_id": "main_pg", "title": "mine now" } }), "not managed by this door"),
        (json!({ "tool": "credential_delete", "arguments": { "credential_id": "main_pg" } }), "not managed by this door"),
    ] {
        let (status, answer) = command(&app, body.clone()).await;
        assert_eq!(status, StatusCode::FORBIDDEN, "{body}: {answer}");
        assert!(answer.contains(word), "{body} should say '{word}': {answer}");
    }

    // The limit is two: a second fits, a third does not.
    let (status, answer) = command(&app, create("ext_b", "postgres", pg("203.0.113.11"))).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let (status, answer) = command(&app, create("ext_c", "postgres", pg("203.0.113.13"))).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{answer}");
    assert!(answer.contains("its limit"), "{answer}");

    // The list: given and made, never the rest, never a password.
    let (status, answer) = command(&app, json!({ "tool": "credential_list" })).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let listed: Value = serde_json::from_str(&answer).expect("json");
    let rows = listed["results"][0]["answer"]["credentials"].as_array().expect("rows").clone();
    let ids: Vec<&str> = rows.iter().filter_map(|row| row["credential_id"].as_str()).collect();
    assert_eq!(ids, vec!["ext_a", "ext_b", "pg_given"], "{answer}");
    assert!(!answer.contains(&password) && !answer.contains("main_pg"), "{answer}");
    assert_eq!(rows[0]["fields"]["host"], "203.0.113.10");

    // Its own: changed, deleted.
    let (status, answer) = command(&app, json!({ "tool": "credential_update", "arguments": { "credential_id": "ext_a", "title": "Census", "secret": { "host": "203.0.113.30" } } })).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let (status, answer) = command(&app, json!({ "tool": "credential_delete", "arguments": { "credential_id": "ext_b" } })).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert!(listed_credential(&app, &cookie, "ext_b").await.is_none());
    assert!(listed_credential(&app, &cookie, "main_pg").await.is_some(), "the rest is untouched");

    // A made pipeline may use the made and the given, not the rest.
    let made = |cred: &str| json!({ "tool": "pipeline_register", "arguments": { "file_rel_path": format!("modules/q-{cred}.zf.json"),
        "body": format!("| trigger.webhook --route /m/{cred} --method GET | postgres.query.run --credential {cred} -- \"SELECT 1\" | web.response.send --body ok") } });
    for cred in ["ext_a", "pg_given"] {
        let (status, answer) = command(&app, made(cred)).await;
        assert_eq!(status, StatusCode::OK, "{cred}: {answer}");
    }
    let (status, answer) = command(&app, made("main_pg")).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{answer}");
    assert!(answer.contains("not delegated"), "{answer}");

    // Studio shows the mark, and a person editing it does not take it away.
    assert_eq!(listed_credential(&app, &cookie, "ext_a").await.expect("ext_a")["managed_by"], "zf_lab");
    credential(&app, &cookie, "ext_a", "postgres", pg("203.0.113.31")).await;
    assert_eq!(listed_credential(&app, &cookie, "ext_a").await.expect("ext_a")["managed_by"], "zf_lab");
    assert!(listed_credential(&app, &cookie, "pg_given").await.expect("pg_given").get("managed_by").is_none());

    // The change record names the door for its own, and the person for theirs.
    let (status, body) = call(&app, Some(&cookie), "GET", &format!("/api/projects/{OWNER}/{PROJECT}/changes?path=credential:ext_a"), None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let changes: Value = serde_json::from_str(&body).expect("json");
    let lines: Vec<(String, String)> = changes["changes"]
        .as_array()
        .expect("changes")
        .iter()
        .map(|line| (line["via"].as_str().unwrap_or_default().to_string(), line["action"].as_str().unwrap_or_default().to_string()))
        .collect();
    assert_eq!(
        lines,
        vec![
            ("studio".to_string(), "credential edit".to_string()),
            ("control · lab-app".to_string(), "credential edit".to_string()),
            ("control · lab-app".to_string(), "credential create".to_string()),
        ],
        "{body}"
    );
    assert!(!body.contains(&password));
}

/// The same door published as an MCP tool: an outside agent calls it, and the
/// fence holds exactly as it does behind a webhook.
#[tokio::test]
async fn a_door_can_be_a_published_mcp_tool() {
    use super::published_mcp::{addressing, call as mcp_call, rpc, text_of};
    let root = TestDir(std::env::temp_dir().join(format!(
        "zebflow-platform-zebflow-command-mcp-{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    )));
    let mut config = PlatformConfig::default();
    config.data_root = root.0.clone();
    config.default_password = "test-pass".to_string();
    let platform = Arc::new(PlatformService::from_config(config).expect("platform"));
    let app = zebflow::platform::web::router(platform.clone()).await;
    let cookie = login(&app).await;
    credential(&app, &cookie, "zf_lab", "zebflow_control", json!({ "scope": "modules/", "routes": "/m/", "tools": ["file_write"], "record_as": "lab-agent" })).await;
    addressing(&app, &cookie, json!({ "hosts": [], "routes": [], "disabled": ["files", "ms"] })).await;
    let door = "[t] trigger.mcp --route /lab --name zebflow_command --description \"Run commands in the lab.\" --parameter commands:array! \"The commands, each a tool and its arguments.\" --auth none\n\
                [c] zebflow.command.run --credential zf_lab --command \"{{ input.mcp.arguments.commands }}\" --as agent\n\
                [ok] web.response.send --body \"{{ input.command }}\"\n\
                [no] web.response.send --status 403 --body \"{{ input.command }}\"\n\
                [t] -> [c]\n[c] -> [ok]\n[c]:error -> [no]";
    let ops = PlatformOps::new(platform.clone(), OWNER, PROJECT);
    let registered = ops.pipeline_register(door, Some("pipelines/mcp-door.zf.json"), None, None, None, None).await;
    assert!(!registered.text.starts_with("Error:") && !registered.text.starts_with("Parse error:"), "{}", registered.text);
    let activated = ops.pipeline_activate("pipelines/mcp-door.zf.json").await;
    assert!(!activated.text.starts_with("Error:"), "{}", activated.text);

    let route = format!("/mcp/{OWNER}/{PROJECT}/lab");
    let write = |path: &str| json!({ "commands": [{ "tool": "file_write", "arguments": { "rel_path": path, "content": "hello" } }] });
    let (status, answer) = rpc(&app, &route, None, &[], mcp_call("zebflow_command", write("modules/note.md"))).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(answer["result"]["isError"], json!(false), "{answer}");
    assert!(root.0.join("users/superadmin/default/repo/modules/note.md").exists());
    let (_, answer) = rpc(&app, &route, None, &[], mcp_call("zebflow_command", write("pages/x.md"))).await;
    assert_eq!(answer["result"]["isError"], json!(true), "{answer}");
    assert!(text_of(&answer).contains("outside modules/"), "{answer}");
    assert!(!root.0.join("users/superadmin/default/repo/pages/x.md").exists());
}
