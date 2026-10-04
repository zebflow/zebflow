//! The 0.10 → 0.11 migration end to end, against a project holding 0.10
//! pipelines the way an office upgraded to 0.11 holds them.
//!
//! The fixtures under `tests/fixtures/migration/0.10/` were made with the
//! 0.10 release itself (`v0.10.12`): `pipelines.json` is what its DSL
//! parser built from the bodies those pipelines were registered with, and
//! `answers.json` is what its webhook answered each request in
//! `requests.json`. After the plan is applied, 0.11 must answer every
//! request the same.

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use tower::ServiceExt;

use zebflow::platform::{PipelineMeta, PlatformConfig, PlatformService};

const FIXTURES: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/migration/0.10");
const OWNER: &str = "superadmin";
const PROJECT: &str = "default";

struct TestDir(std::path::PathBuf);

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn fixture(name: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(format!("{FIXTURES}/{name}")).expect("fixture")).expect("json")
}

async fn login(app: &axum::Router) -> String {
    login_as(app, "superadmin", "test-pass").await
}

/// Signs in through the real form and answers the session cookie.
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

async fn send(app: &axum::Router, cookie: &str, method: &str, uri: &str, body: Option<Value>) -> axum::response::Response {
    let mut builder = Request::builder().uri(uri).method(method).header(header::COOKIE, cookie);
    let body = match body {
        Some(json) => {
            builder = builder.header(header::CONTENT_TYPE, "application/json");
            Body::from(serde_json::to_vec(&json).expect("json"))
        }
        None => Body::empty(),
    };
    app.clone().oneshot(builder.body(body).expect("request")).await.expect("response")
}

async fn json_of(response: axum::response::Response) -> (StatusCode, Value) {
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("body");
    (status, serde_json::from_slice(&bytes).unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).to_string())))
}

/// What a webhook answered, in the shape the 0.10 recording kept.
async fn answer(app: &axum::Router, cookie: &str, request: &Value) -> Value {
    let uri = format!("/wh/{OWNER}/{PROJECT}{}", request["path"].as_str().unwrap());
    let response = send(app, cookie, request["method"].as_str().unwrap(), &uri, request.get("json").cloned()).await;
    let status = response.status().as_u16();
    let headers = response.headers().clone();
    let text = String::from_utf8_lossy(&to_bytes(response.into_body(), usize::MAX).await.expect("body")).to_string();
    let get = |name: &str| headers.get(name).and_then(|v| v.to_str().ok()).map(str::to_string);
    let body = match request.get("html").and_then(Value::as_array) {
        Some(ids) => {
            let mut found = serde_json::Map::new();
            for id in ids {
                let id = id.as_str().unwrap();
                let marker = format!("id=\"{id}\"");
                let value = text.find(&marker).and_then(|at| {
                    let rest = &text[at..];
                    let start = rest.find('>')? + 1;
                    let end = rest[start..].find('<')?;
                    Some(rest[start..start + end].to_string())
                });
                found.insert(id.to_string(), json!(value));
            }
            Value::Object(found)
        }
        None => serde_json::from_str(&text).unwrap_or(Value::String(text.clone())),
    };
    json!({
        "name": request["name"],
        "status": status,
        "content_type": get("content-type").map(|c| c.split(';').next().unwrap().to_string()),
        "set_cookie": get("set-cookie"),
        "location": get("location"),
        "body": body,
    })
}

/// The project as an office upgraded to 0.11 holds it: the 0.10 files in
/// the repository and their rows, active.
fn seed(platform: &PlatformService) {
    let pipelines = fixture("pipelines.json");
    for item in pipelines.as_array().unwrap() {
        let file_rel_path = item["file_rel_path"].as_str().unwrap();
        let source = serde_json::to_string_pretty(&item["source"]).unwrap();
        platform.projects.write_repo_file(OWNER, PROJECT, file_rel_path, &source).expect("write 0.10 pipeline");
        let name = file_rel_path.trim_end_matches(".zf.json").rsplit('/').next().unwrap().to_string();
        let virtual_path = format!("/{}", file_rel_path.rsplit_once('/').map(|(d, _)| d).unwrap_or(""));
        platform
            .data
            .put_pipeline_meta(&PipelineMeta {
                owner: OWNER.into(),
                project: PROJECT.into(),
                name,
                title: file_rel_path.into(),
                virtual_path,
                file_rel_path: file_rel_path.into(),
                description: String::new(),
                trigger_kind: if file_rel_path.starts_with("fn/") { "function".into() } else { "webhook".into() },
                hash: "seeded".into(),
                active_hash: Some("seeded".into()),
                activated_at: Some(1),
                created_at: 1,
                updated_at: 1,
            })
            .expect("seed row");
    }
    let page = std::fs::read_to_string(format!("{FIXTURES}/pages/items.tsx")).unwrap();
    platform.projects.write_repo_file(OWNER, PROJECT, "pages/items.tsx", &page).expect("write page");
}

fn read(platform: &PlatformService, path: &str) -> Option<String> {
    platform.projects.read_repo_file_text(OWNER, PROJECT, path).ok()
}

#[tokio::test]
async fn a_0_10_project_is_planned_applied_and_answers_as_it_did() {
    let root = TestDir(std::env::temp_dir().join(format!(
        "zebflow-platform-migration-{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    )));
    let mut config = PlatformConfig::default();
    config.data_root = root.0.clone();
    config.default_password = "test-pass".to_string();
    let platform = Arc::new(PlatformService::from_config(config).expect("platform"));
    let app = zebflow::platform::web::router(platform.clone()).await;
    let cookie = login(&app).await;
    seed(&platform);

    // Owner only: no session, no plan.
    let anonymous = send(&app, "", "GET", &format!("/api/projects/{OWNER}/{PROJECT}/migration/0.11/plan"), None).await;
    assert_eq!(anonymous.status(), StatusCode::UNAUTHORIZED);

    // The plan: every 0.10 pipeline and the page rewritten, nothing left
    // unresolved, every rewrite passing the save-time check without a warning.
    let (status, body) = json_of(send(&app, &cookie, "GET", &format!("/api/projects/{OWNER}/{PROJECT}/migration/0.11/plan"), None).await).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let plan = &body["plan"];
    assert_eq!(plan["ready"], true, "{}", plan["report"].as_str().unwrap_or_default());
    assert_eq!(plan["counts"]["pipelines_to_rewrite"], 11, "{}", plan["report"].as_str().unwrap_or_default());
    assert_eq!(plan["counts"]["pages_to_rewrite"], 1);
    assert_eq!(plan["counts"]["unresolved"], 0);
    assert_eq!(plan["counts"]["refusals"], 0);
    assert_eq!(plan["counts"]["warnings"], 0, "{}", plan["report"].as_str().unwrap_or_default());
    let report = send(&app, &cookie, "GET", &format!("/api/projects/{OWNER}/{PROJECT}/migration/0.11/plan?format=markdown"), None).await;
    let report = String::from_utf8_lossy(&to_bytes(report.into_body(), usize::MAX).await.unwrap()).to_string();
    assert!(report.contains("**ready to apply**") && report.contains("```diff"), "{report}");
    let fingerprint = plan["fingerprint"].as_str().unwrap().to_string();

    // A plan the project moved on from is refused.
    let (status, _) = json_of(
        send(&app, &cookie, "POST", &format!("/api/projects/{OWNER}/{PROJECT}/migration/0.11/apply"), Some(json!({ "fingerprint": "stale" }))).await,
    )
    .await;
    assert_eq!(status, StatusCode::CONFLICT);

    // Apply.
    let (status, body) = json_of(
        send(&app, &cookie, "POST", &format!("/api/projects/{OWNER}/{PROJECT}/migration/0.11/apply"), Some(json!({ "fingerprint": fingerprint }))).await,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["ok"], true, "{body}");

    // The originals are archived at their paths; the rewrites took their place.
    for item in fixture("pipelines.json").as_array().unwrap() {
        let path = item["file_rel_path"].as_str().unwrap();
        let archived = read(&platform, &format!("archive/0.10/{path}")).expect("archived");
        assert_eq!(serde_json::from_str::<Value>(&archived).unwrap(), item["source"], "{path}");
        let live = read(&platform, path).expect("rewritten");
        assert!(!live.contains("\"n."), "{path} still holds a 0.10 kind");
        let meta = platform.projects.get_pipeline_meta_by_file_id(OWNER, PROJECT, path).unwrap().expect("row");
        assert!(meta.active_hash.is_some(), "{path} is active again");
    }
    let page = read(&platform, "pages/items.tsx").unwrap();
    assert!(page.contains("input.script.title") && page.contains("input.params.who"), "{page}");
    assert!(read(&platform, "archive/0.10/pages/items.tsx").unwrap().contains("{input.title}"));
    let record = read(&platform, "archive/0.10/MIGRATION.md").expect("record");
    assert!(record.contains("api/hello.zf.json") && record.contains("activated"), "{record}");

    // Nothing under archive/ is a pipeline: not listed, not registrable.
    let (_, listed) = json_of(send(&app, &cookie, "GET", &format!("/api/projects/{OWNER}/{PROJECT}/pipelines"), None).await).await;
    assert!(!listed.to_string().contains("archive/"), "{listed}");
    let refused = platform.projects.upsert_pipeline_definition(
        OWNER,
        PROJECT,
        "archive/0.10/api/hello.zf.json",
        "",
        "",
        "webhook",
        &read(&platform, "api/hello.zf.json").unwrap(),
    );
    assert_eq!(refused.unwrap_err().code, "PLATFORM_PIPELINE_ARCHIVED");

    // Every request is answered as 0.10 answered it.
    let expected = fixture("answers.json");
    for (request, want) in fixture("requests.json").as_array().unwrap().iter().zip(expected.as_array().unwrap()) {
        let got = answer(&app, &cookie, request).await;
        assert_eq!(&got, want, "{}", request["name"]);
    }

    // Idempotent: a second plan finds everything migrated, and applying it
    // changes nothing.
    let snapshot = |platform: &PlatformService| -> Vec<Option<String>> {
        let mut paths: Vec<String> = fixture("pipelines.json")
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|i| {
                let p = i["file_rel_path"].as_str().unwrap().to_string();
                [p.clone(), format!("archive/0.10/{p}")]
            })
            .collect();
        paths.extend(["pages/items.tsx", "archive/0.10/pages/items.tsx", "archive/0.10/MIGRATION.md", "archive/0.10/migration.json"].map(String::from));
        paths.iter().map(|p| read(platform, p)).collect()
    };
    let before = snapshot(&platform);
    let (_, body) = json_of(send(&app, &cookie, "GET", &format!("/api/projects/{OWNER}/{PROJECT}/migration/0.11/plan"), None).await).await;
    let again = &body["plan"];
    assert_eq!(again["counts"]["pipelines_to_rewrite"], 0, "{again}");
    assert_eq!(again["counts"]["pipelines_done"], 11);
    assert_eq!(again["counts"]["pages_to_rewrite"], 0);
    let (status, body) = json_of(
        send(
            &app,
            &cookie,
            "POST",
            &format!("/api/projects/{OWNER}/{PROJECT}/migration/0.11/apply"),
            Some(json!({ "fingerprint": again["fingerprint"] })),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["report"]["done"], json!([]), "{body}");
    assert_eq!(snapshot(&platform), before, "a second apply changed files");
}

/// A 0.10 pipeline document written the way the 0.10 parser stored one:
/// `(id, kind, config)` nodes in a chain.
fn chain_doc(name: &str, nodes: &[(&str, &str, Value)]) -> Value {
    let edges: Vec<Value> = nodes
        .windows(2)
        .map(|w| json!({ "from_node": w[0].0, "from_pin": "out", "to_node": w[1].0, "to_pin": "in" }))
        .collect();
    json!({
        "apiVersion": "zebflow.com/v1",
        "kind": "Pipeline",
        "metadata": { "name": name },
        "spec": {
            "id": name,
            "nodes": nodes.iter().enumerate().map(|(i, (id, kind, config))| json!({
                "id": id, "kind": kind, "config": config,
                "input_pins": if i == 0 { json!([]) } else { json!(["in"]) },
                "output_pins": ["out"],
            })).collect::<Vec<_>>(),
            "edges": edges,
        }
    })
}

fn seed_one(platform: &PlatformService, file_rel_path: &str, doc: &Value, trigger_kind: &str) {
    platform
        .projects
        .write_repo_file(OWNER, PROJECT, file_rel_path, &serde_json::to_string_pretty(doc).unwrap())
        .expect("write 0.10 pipeline");
    platform
        .data
        .put_pipeline_meta(&PipelineMeta {
            owner: OWNER.into(),
            project: PROJECT.into(),
            name: file_rel_path.trim_end_matches(".zf.json").into(),
            title: file_rel_path.into(),
            virtual_path: "/".into(),
            file_rel_path: file_rel_path.into(),
            description: String::new(),
            trigger_kind: trigger_kind.into(),
            hash: "seeded".into(),
            active_hash: Some("seeded".into()),
            activated_at: Some(1),
            created_at: 1,
            updated_at: 1,
        })
        .expect("seed row");
}

/// One MCP tool call; the result's text.
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

async fn mcp_token(app: &axum::Router, cookie: &str) -> String {
    let (status, body) = json_of(
        send(
            app,
            cookie,
            "POST",
            &format!("/api/projects/{OWNER}/{PROJECT}/mcp/session"),
            Some(json!({ "capabilities": ["pipelines.read", "pipelines.write"] })),
        )
        .await,
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body["session"]["token"].as_str().expect("token").to_string()
}

/// MCP apply switches pipelines exactly as the HTTP route does: a
/// topic-triggered pipeline is subscribed again, so a message published
/// after the apply runs it. And only the project's owner may apply: a
/// session a maintainer created is refused, whatever capabilities it lists.
#[tokio::test]
async fn mcp_apply_reactivates_a_topic_pipeline_with_its_subscriber_and_is_the_owners() {
    let root = TestDir(std::env::temp_dir().join(format!(
        "zebflow-platform-migration-mcp-{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    )));
    let mut config = PlatformConfig::default();
    config.data_root = root.0.clone();
    config.default_password = "test-pass".to_string();
    let platform = Arc::new(PlatformService::from_config(config).expect("platform"));
    let app = zebflow::platform::web::router(platform.clone()).await;
    let cookie = login(&app).await;

    // Seeded after the router started: nothing subscribed to the topic yet.
    seed_one(
        &platform,
        "news/listen.zf.json",
        &chain_doc("listen", &[
            ("sub", "n.trigger.kv.subscribe", json!({ "channel": "news" })),
            ("keep", "n.kv.set", json!({ "key": "last-news", "value": "{{ input.message }}" })),
        ]),
        "kv.subscribe",
    );
    seed_one(
        &platform,
        "news/publish.zf.json",
        &chain_doc("publish", &[
            ("hook", "n.trigger.webhook", json!({ "path": "/news/publish", "method": "POST" })),
            ("send", "n.kv.publish", json!({ "channel": "news", "payload": "{{ input.body.text }}" })),
            ("done", "n.web.response", json!({ "body": "{{ { ok: true } }}" })),
        ]),
        "webhook",
    );
    seed_one(
        &platform,
        "news/read.zf.json",
        &chain_doc("read", &[
            ("hook", "n.trigger.webhook", json!({ "path": "/news/last", "method": "GET" })),
            ("get", "n.kv.get", json!({ "key": "last-news", "out_key": "value" })),
            ("done", "n.web.response", json!({ "body": "{{ { last: input.value } }}" })),
        ]),
        "webhook",
    );

    let token = mcp_token(&app, &cookie).await;
    let plan: Value = serde_json::from_str(&mcp_call(&app, &token, "migration_plan", json!({})).await.expect("plan")).expect("plan json");
    assert_eq!(plan["ready"], true, "{}", plan["report"].as_str().unwrap_or_default());
    let applied = mcp_call(&app, &token, "migration_apply", json!({ "fingerprint": plan["fingerprint"] })).await.expect("apply");
    let applied: Value = serde_json::from_str(&applied).expect("report json");
    assert_eq!(applied["ok"], true, "{applied}");

    // A message published now reaches the re-activated subscriber.
    let mut last = Value::Null;
    for _ in 0..50 {
        let (status, body) = json_of(
            send(&app, &cookie, "POST", &format!("/wh/{OWNER}/{PROJECT}/news/publish"), Some(json!({ "text": "hello" }))).await,
        )
        .await;
        assert_eq!(status, StatusCode::OK, "{body}");
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let (_, body) = json_of(send(&app, &cookie, "GET", &format!("/wh/{OWNER}/{PROJECT}/news/last"), None).await).await;
        last = body["last"].clone();
        if last == "hello" {
            break;
        }
    }
    assert_eq!(last, "hello", "the topic pipeline was not subscribed after the MCP apply");

    // A maintainer's session holds the pipeline capabilities, but the
    // maintainer is not the owner: the owner-only tools refuse it.
    let maintainer_password = uuid::Uuid::new_v4().to_string();
    platform
        .users
        .create_user(&zebflow::platform::CreateUserRequest {
            owner: "maintainer-one".into(),
            password: maintainer_password.clone(),
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
                user_id: "maintainer-one".into(),
                role_preset: zebflow::platform::model::ProjectAccessRolePreset::Maintainer,
                custom_policy_ids: Vec::new(),
                mcp_capability_ceiling: Vec::new(),
            },
        )
        .expect("member");
    let maintainer = login_as(&app, "maintainer-one", &maintainer_password).await;
    let token = mcp_token(&app, &maintainer).await;
    let refused = mcp_call(&app, &token, "migration_plan", json!({})).await.expect_err("a maintainer's session is refused");
    assert!(refused.contains("owner"), "{refused}");
    let refused = mcp_call(&app, &token, "migration_apply", json!({ "fingerprint": "x" })).await.expect_err("refused");
    assert!(refused.contains("owner"), "{refused}");
    // The HTTP route refuses the maintainer the same way.
    let http = send(&app, &maintainer, "GET", &format!("/api/projects/{OWNER}/{PROJECT}/migration/0.11/plan"), None).await;
    assert_eq!(http.status(), StatusCode::FORBIDDEN);
}
