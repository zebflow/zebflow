//! The 0.11 migration as part of an upgrade (`services/migration/startup.rs`):
//! a server started over a project still holding 0.10 pipelines migrates it
//! when the plan is ready and holds nothing to review, and otherwise leaves
//! it alone, logs it and shows "needs migration" on the home and the
//! project's dashboard. A second start does nothing.

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use tower::ServiceExt;

use zebflow::platform::services::migration::Notice;
use zebflow::platform::{PipelineMeta, PlatformConfig, PlatformService};

const OWNER: &str = "superadmin";
const PROJECT: &str = "default";

struct TestDir(std::path::PathBuf);

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn temp_dir(name: &str) -> TestDir {
    TestDir(std::env::temp_dir().join(format!(
        "zebflow-platform-migration-start-{name}-{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    )))
}

fn platform_at(root: &TestDir) -> Arc<PlatformService> {
    let mut config = PlatformConfig::default();
    config.data_root = root.0.clone();
    config.default_password = "test-pass".to_string();
    Arc::new(PlatformService::from_config(config).expect("platform"))
}

/// A 0.10 pipeline document: `(id, kind, config)` nodes in a chain.
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

/// A 0.10 pipeline as an upgraded office holds it: the file, and its row
/// (active when `active_hash` is set; a draft when it differs from `hash`).
fn seed(platform: &PlatformService, file_rel_path: &str, doc: &Value, active_hash: Option<&str>) {
    platform
        .projects
        .write_repo_file(OWNER, PROJECT, file_rel_path, &serde_json::to_string_pretty(doc).unwrap())
        .expect("write 0.10 pipeline");
    platform
        .data
        .put_pipeline_meta(&PipelineMeta {
            owner: OWNER.into(),
            project: PROJECT.into(),
            name: file_rel_path.trim_end_matches(".zf.json").rsplit('/').next().unwrap().into(),
            title: file_rel_path.into(),
            virtual_path: "/".into(),
            file_rel_path: file_rel_path.into(),
            description: String::new(),
            trigger_kind: "webhook".into(),
            hash: "seeded".into(),
            active_hash: active_hash.map(str::to_string),
            activated_at: active_hash.map(|_| 1),
            created_at: 1,
            updated_at: 1,
        })
        .expect("seed row");
}

fn read(platform: &PlatformService, path: &str) -> Option<String> {
    platform.projects.read_repo_file_text(OWNER, PROJECT, path).ok()
}

fn active(platform: &PlatformService, file_rel_path: &str) -> bool {
    platform
        .projects
        .get_pipeline_meta_by_file_id(OWNER, PROJECT, file_rel_path)
        .unwrap()
        .is_some_and(|m| m.active_hash.is_some())
}

async fn login(app: &axum::Router) -> String {
    let form = serde_urlencoded::to_string([("identifier", OWNER), ("password", "test-pass")]).expect("form");
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
    response.headers().get(header::SET_COOKIE).expect("cookie").to_str().unwrap().split(';').next().unwrap().to_string()
}

async fn get(app: &axum::Router, cookie: &str, uri: &str) -> (StatusCode, String) {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(uri).header(header::COOKIE, cookie).body(Body::empty()).unwrap())
        .await
        .expect("response");
    let status = response.status();
    let body = to_bytes(response.into_body(), usize::MAX).await.expect("body");
    (status, String::from_utf8_lossy(&body).to_string())
}

/// The home and the project's dashboard both say what is left, and link to
/// the plan.
async fn assert_notice_visible(app: &axum::Router, items: usize) {
    let cookie = login(app).await;
    let words = format!("needs migration — {items} {} to review", if items == 1 { "item" } else { "items" });
    let href = format!("/api/projects/{OWNER}/{PROJECT}/migration/0.11/plan?format=markdown");
    for page in ["/home".to_string(), format!("/projects/{OWNER}/{PROJECT}/dashboard")] {
        let (status, html) = get(app, &cookie, &page).await;
        assert_eq!(status, StatusCode::OK, "{page}");
        assert!(!html.contains("RWE component error"), "{page}: {html}");
        assert!(html.contains(&words), "{page} does not say `{words}`");
        assert!(html.contains(&href), "{page} does not link the plan");
    }
}

fn answered_webhook() -> Value {
    chain_doc("ping", &[
        ("hook", "n.trigger.webhook", json!({ "path": "/api/ping", "method": "GET" })),
        ("done", "n.web.response", json!({ "body": "{{ { pong: true } }}" })),
    ])
}

/// Ready and nothing to review: migrated at start, each pipeline active
/// exactly as its original was, the route answering; a second start
/// changes nothing.
#[tokio::test]
async fn a_ready_plan_with_nothing_to_review_is_applied_at_start_and_once() {
    let root = temp_dir("apply");
    let platform = platform_at(&root);
    seed(&platform, "api/ping.zf.json", &answered_webhook(), Some("seeded"));
    seed(
        &platform,
        "api/off.zf.json",
        &chain_doc("off", &[
            ("hook", "n.trigger.webhook", json!({ "path": "/api/off", "method": "GET" })),
            ("done", "n.web.response", json!({ "body": "{{ { off: true } }}" })),
        ]),
        None,
    );

    let app = zebflow::platform::web::router(platform.clone()).await;

    for path in ["api/ping.zf.json", "api/off.zf.json"] {
        let live = read(&platform, path).unwrap();
        assert!(!live.contains("\"n."), "{path} still holds a 0.10 kind: {live}");
        assert!(read(&platform, &format!("archive/0.10/{path}")).is_some(), "{path} was not archived");
    }
    assert!(active(&platform, "api/ping.zf.json"), "an active original is active again");
    assert!(!active(&platform, "api/off.zf.json"), "an inactive original stays inactive");
    let record = read(&platform, "archive/0.10/MIGRATION.md").expect("the record");
    assert!(record.contains("api/ping.zf.json") && record.contains("activated"), "{record}");
    assert_eq!(platform.migration_notices.get(OWNER, PROJECT), None);

    let cookie = login(&app).await;
    let (status, body) = get(&app, &cookie, &format!("/wh/{OWNER}/{PROJECT}/api/ping")).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(serde_json::from_str::<Value>(&body).unwrap(), json!({ "pong": true }));
    let (_, home) = get(&app, &cookie, "/home").await;
    assert!(!home.contains("needs migration"), "a migrated project shows no notice");

    // A second start: nothing to do, nothing written.
    let files = ["api/ping.zf.json", "api/off.zf.json", "archive/0.10/MIGRATION.md", "archive/0.10/migration.json"];
    let before: Vec<_> = files.iter().map(|p| read(&platform, p)).collect();
    drop(app);
    let platform = platform_at(&root);
    let app = zebflow::platform::web::router(platform.clone()).await;
    let after: Vec<_> = files.iter().map(|p| read(&platform, p)).collect();
    assert_eq!(after, before, "a second start changed files");
    assert!(active(&platform, "api/ping.zf.json"));
    assert!(!active(&platform, "api/off.zf.json"));
    assert_eq!(platform.migration_notices.get(OWNER, PROJECT), None);
    let (status, _) = get(&app, &cookie, &format!("/wh/{OWNER}/{PROJECT}/api/ping")).await;
    assert_eq!(status, StatusCode::OK);
}

/// A webhook that answered its payload in 0.10 is a review item: the plan
/// is not applied at start, the project is logged and noticed, and the
/// review says why in plain words. A second start leaves it the same.
#[tokio::test]
async fn a_plan_with_a_review_item_is_not_applied_and_is_named() {
    let root = temp_dir("review");
    let platform = platform_at(&root);
    seed(&platform, "api/ping.zf.json", &answered_webhook(), Some("seeded"));
    let dump = chain_doc("dump", &[
        ("hook", "n.trigger.webhook", json!({ "path": "/api/dump", "method": "GET" })),
        ("pong", "n.script", json!({ "source": "return { ok: true };" })),
    ]);
    seed(&platform, "api/dump.zf.json", &dump, Some("seeded"));
    let original = read(&platform, "api/dump.zf.json").unwrap();

    let app = zebflow::platform::web::router(platform.clone()).await;

    assert_eq!(read(&platform, "api/dump.zf.json").unwrap(), original, "nothing is rewritten");
    assert!(read(&platform, "archive/0.10/api/dump.zf.json").is_none());
    assert!(read(&platform, "archive/0.10/MIGRATION.md").is_none());
    let notice: Notice = platform.migration_notices.get(OWNER, PROJECT).expect("a notice");
    assert_eq!(notice.items, 1, "{notice:?}");
    assert!(notice.line.contains("superadmin/default"), "{}", notice.line);
    assert!(notice.line.contains("2 pipeline(s)") && notice.line.contains("1 review item(s)"), "{}", notice.line);
    assert!(notice.line.contains("migration/0.11/plan"), "{}", notice.line);
    assert_notice_visible(&app, 1).await;

    let cookie = login(&app).await;
    let (_, report) = get(&app, &cookie, &notice.plan_href).await;
    assert!(
        report.contains("this webhook answered its payload in 0.10; in 0.11 it answers 204 until it ends with web.response.send"),
        "{report}"
    );

    // A second start: still named, still untouched.
    drop(app);
    let platform = platform_at(&root);
    let app = zebflow::platform::web::router(platform.clone()).await;
    assert_eq!(read(&platform, "api/dump.zf.json").unwrap(), original);
    assert_eq!(platform.migration_notices.get(OWNER, PROJECT), Some(notice));
    assert_notice_visible(&app, 1).await;
}

/// A plan that is not ready (here a 0.10 pipeline whose working tree was
/// never activated) is not applied: logged with what blocks it, noticed.
/// The owner applying it by hand clears the notice.
#[tokio::test]
async fn a_plan_that_is_not_ready_is_not_applied_and_is_named() {
    let root = temp_dir("blocked");
    let platform = platform_at(&root);
    seed(&platform, "api/ping.zf.json", &answered_webhook(), Some("older"));
    let original = read(&platform, "api/ping.zf.json").unwrap();

    let app = zebflow::platform::web::router(platform.clone()).await;

    assert_eq!(read(&platform, "api/ping.zf.json").unwrap(), original, "nothing is rewritten");
    let notice = platform.migration_notices.get(OWNER, PROJECT).expect("a notice");
    assert_eq!(notice.items, 1, "{notice:?}");
    assert!(notice.line.contains("NOT applied") && notice.line.contains("1 unresolved item(s)"), "{}", notice.line);
    assert_notice_visible(&app, 1).await;

    // Resolved by hand (the draft activated as it is) and applied through
    // the owner's route: the notice goes.
    let mut meta = platform.projects.get_pipeline_meta_by_file_id(OWNER, PROJECT, "api/ping.zf.json").unwrap().unwrap();
    meta.active_hash = Some(meta.hash.clone());
    platform.data.put_pipeline_meta(&meta).unwrap();
    let service = zebflow::platform::services::migration::MigrationService::new(platform.clone());
    let plan = service.plan_project(OWNER, PROJECT).unwrap();
    assert!(plan.ready, "{}", plan.report);
    let report = service.apply_plan(OWNER, PROJECT, &plan.fingerprint).await.unwrap();
    assert!(report.ok, "{report:?}");
    assert_eq!(platform.migration_notices.get(OWNER, PROJECT), None);
    let cookie = login(&app).await;
    let (_, home) = get(&app, &cookie, "/home").await;
    assert!(!home.contains("needs migration"));
}
