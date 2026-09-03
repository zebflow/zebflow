//! Regression cover for the shared path-containment mistake (S1-S3).
//!
//! Each of these reached the filesystem through an *authorised* request: the
//! caller had the capability the route asks for, on the project the route
//! names, and then handed a path in the request body that pointed somewhere
//! else entirely. What was checked was `joined.starts_with(root)`, which
//! compares path components without resolving them — so `root/../../elsewhere`
//! "starts with" `root`.

use std::fs;
use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use tower::ServiceExt;

use zebflow::platform::model::CreateProjectRequest;
use zebflow::platform::{PlatformConfig, PlatformService};

fn temp_test_dir(name: &str) -> std::path::PathBuf {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!("zebflow-containment-{name}-{now}"))
}

async fn login_cookie(app: axum::Router, identifier: &str, password: &str) -> String {
    let login = app
        .oneshot(
            Request::builder()
                .uri("/login")
                .method("POST")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from(format!(
                    "identifier={identifier}&password={password}"
                )))
                .expect("request"),
        )
        .await
        .expect("login response");
    assert_eq!(login.status(), StatusCode::SEE_OTHER);
    login
        .headers()
        .get(header::SET_COOKIE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .expect("session cookie")
        .to_string()
}

async fn post_json(
    app: &axum::Router,
    cookie: &str,
    uri: &str,
    body: Value,
) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(uri)
                .method("POST")
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body.to_string()))
                .expect("request"),
        )
        .await
        .expect("response");
    let status = response.status();
    let text = to_bytes(response.into_body(), usize::MAX)
        .await
        .map(|bytes| String::from_utf8_lossy(&bytes).to_string())
        .unwrap_or_default();
    let value = serde_json::from_str::<Value>(&text).unwrap_or(Value::String(text));
    (status, value)
}

fn pipeline_source(id: &str, path: &str) -> String {
    json!({
        "apiVersion": "zebflow.com/v1",
        "kind": "Pipeline",
        "metadata": { "name": id },
        "spec": {
            "id": id,
            "entry_nodes": ["wh"],
            "nodes": [{
                "id": "wh",
                "kind": "n.trigger.webhook",
                "input_pins": [],
                "output_pins": ["out"],
                "config": { "path": path, "method": "POST" }
            }],
            "edges": []
        }
    })
    .to_string()
}

/// S2 + S3. Two projects, one owner, one session. Every pipeline door takes
/// `file_rel_path` from the request body while authorising against the project
/// in the route, so a body full of `..` used the attacker's capability on the
/// victim's disk. Revert `normalize_pipeline_file_rel_path` and the file
/// appears under `victim/repo/src/`; revert the lock handler and it reads and
/// rewrites the victim's pipeline in place.
#[tokio::test]
async fn a_pipeline_body_cannot_reach_across_projects_through_any_door() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("pipeline-cross-project");
    config.default_password = "test-pass".to_string();
    let platform = Arc::new(PlatformService::from_config(config).expect("platform service"));
    let app = zebflow::platform::web::router(platform.clone()).await;
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    for project in ["attacker", "victim"] {
        platform
            .projects
            .create_or_update_project(
                "superadmin",
                &CreateProjectRequest {
                    project: project.to_string(),
                    title: Some(project.to_string()),
                    local_branch: None,
                    runtime: Default::default(),
                },
            )
            .expect("project");
    }
    let attacker = platform
        .projects
        .project_layout("superadmin", "attacker")
        .expect("attacker layout");
    let victim = platform
        .projects
        .project_layout("superadmin", "victim")
        .expect("victim layout");

    // The victim owns a real, unlocked pipeline.
    let (status, _) = post_json(
        &app,
        &cookie,
        "/api/projects/superadmin/victim/pipelines/definition",
        json!({
            "file_rel_path": "api/payouts",
            "title": "Payouts",
            "description": "",
            "trigger_kind": "webhook",
            "source": pipeline_source("payouts", "/payouts"),
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let victim_file = victim.repo_source_dir().join("api/payouts.zf.json");
    assert!(victim_file.is_file());
    let victim_before = fs::read_to_string(&victim_file).expect("victim pipeline");

    // Counted so it genuinely lands on the victim's file: the pipeline doors
    // join onto `<data_root>/users/superadmin/attacker/repo/<source>`, and
    // `<data_root>/users/superadmin` is the source's own depth plus two —
    // out of `repo/` and out of the project. The source root is the repository
    // itself unless a project declares otherwise, so the depth is counted
    // rather than assumed.
    let source_rel = attacker
        .repo_source_dir()
        .strip_prefix(&attacker.repo_dir)
        .expect("source root under repo")
        .to_string_lossy()
        .to_string();
    let ups = "../".repeat(source_rel.split('/').filter(|s| !s.is_empty()).count() + 2);
    let source_segment = if source_rel.is_empty() {
        String::new()
    } else {
        format!("{source_rel}/")
    };
    let hostile = format!("{ups}victim/repo/{source_segment}api/payouts.zf.json");
    let hostile = hostile.as_str();
    assert_eq!(
        attacker.repo_source_dir().join(hostile).canonicalize().ok(),
        victim_file.canonicalize().ok(),
        "the fixture must actually aim at the victim's pipeline"
    );

    // Door 1: write. It is contained, not refused -- the caller wrote into
    // their own project at a mangled name, which is their business.
    let (status, body) = post_json(
        &app,
        &cookie,
        "/api/projects/superadmin/attacker/pipelines/definition",
        json!({
            "file_rel_path": hostile,
            "title": "Owned",
            "description": "",
            "trigger_kind": "webhook",
            "source": pipeline_source("owned", "/owned"),
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(
        fs::read_to_string(&victim_file).expect("victim pipeline"),
        victim_before,
        "the victim's pipeline was overwritten from another project"
    );
    let identity = body["meta"]["file_rel_path"]
        .as_str()
        .expect("identity")
        .to_string();
    assert!(
        !identity.contains(".."),
        "the stored identity kept traversal: {identity}"
    );
    assert!(
        attacker.repo_source_dir().join(&identity).is_file(),
        "the contained write did not land in the caller's own project"
    );

    // Door 2: the lock toggle, which had no path guard at all -- it read any
    // file decoding as a Pipeline envelope and wrote the file back.
    let (status, body) = post_json(
        &app,
        &cookie,
        "/api/projects/superadmin/attacker/pipelines/lock-toggle",
        json!({ "file_rel_path": hostile, "locked": true }),
    )
    .await;
    assert!(
        status == StatusCode::NOT_FOUND || status == StatusCode::OK,
        "unexpected lock-toggle status {status}: {body}"
    );
    assert_eq!(
        fs::read_to_string(&victim_file).expect("victim pipeline"),
        victim_before,
        "the lock toggle rewrote a pipeline in another project"
    );

    // Door 3: read. It must not hand back the victim's source.
    let (_, body) = post_json(
        &app,
        &cookie,
        "/api/projects/superadmin/attacker/pipelines/by-id",
        json!({ "file_rel_path": hostile }),
    )
    .await;
    assert!(
        !body.to_string().contains("/payouts"),
        "the victim's pipeline source was read across projects: {body}"
    );

    // Door 4: delete.
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/attacker/pipelines/definition")
                .method("DELETE")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "file_rel_path": hostile }).to_string()))
                .expect("request"),
        )
        .await
        .expect("delete response");
    let _ = response.status();
    assert!(
        victim_file.is_file(),
        "the victim's pipeline was deleted from another project"
    );
}

/// S1. `target_folder` arrives from the hub install request body and names
/// where a package's files land. Its only cleaning stripped a leading `./` and
/// `/`, and the destination guard was the same non-resolving `starts_with`, so
/// a hostile folder wrote the package anywhere the server process could reach.
/// Revert `normalize_repo_rel` and the install root escapes the repository.
#[tokio::test]
async fn a_hub_install_target_folder_cannot_write_outside_the_repository() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("hub-target-folder");
    config.default_password = "test-pass".to_string();
    let data_root = config.data_root.clone();
    let platform = Arc::new(PlatformService::from_config(config).expect("platform service"));
    let app = zebflow::platform::web::router(platform.clone()).await;
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    for project in ["producer", "consumer"] {
        platform
            .projects
            .create_or_update_project(
                "superadmin",
                &CreateProjectRequest {
                    project: project.to_string(),
                    title: Some(project.to_string()),
                    local_branch: None,
                    runtime: Default::default(),
                },
            )
            .expect("project");
    }
    platform
        .hub
        .ensure_default_service_instance("standalone", "http://127.0.0.1/api", true)
        .expect("enable hub service");
    platform
        .hub
        .set_authority_enabled("superadmin", "producer", true)
        .expect("authority");
    platform
        .hub
        .upsert_publisher(
            "superadmin",
            "producer",
            "escape-lab",
            "Escape Lab",
            "https://publishers.example/escape-lab",
            "escape@example.com",
            "",
            "",
            "",
            true,
            true,
            true,
            false,
            20,
            10 * 1024 * 1024,
            8,
            2 * 1024 * 1024,
        )
        .expect("publisher");
    platform
        .projects
        .upsert_pipeline_definition(
            "superadmin",
            "producer",
            "pipelines/escapee.zf.json",
            "Escapee",
            "",
            "webhook",
            &pipeline_source("escapee", "/escapee"),
        )
        .expect("producer pipeline");
    platform
        .hub
        .publish_asset(
            "superadmin",
            "producer",
            "superadmin",
            "escape-lab",
            "",
            "",
            "",
            "superadmin",
            "producer",
            "pipeline_with_dependencies",
            "pipelines/escapee.zf.json",
            "escapee",
            "1.0.0",
            "Escapee",
            "A package that asks to be installed outside the repository.",
            "",
            "public",
            Default::default(),
            vec!["escape".to_string()],
        )
        .expect("publish");

    let consumer = platform
        .projects
        .project_layout("superadmin", "consumer")
        .expect("consumer layout");
    // A canary outside the repository, at the depth the traversal aims for.
    let canary = data_root.join("users").join("canary.txt");
    fs::write(&canary, "untouched").expect("canary");

    for hostile in [
        "../../../../tmp/zebflow-hub-escape",
        "billing/../../../../..",
        "/../../../../users",
    ] {
        let (status, body) = post_json(
            &app,
            &cookie,
            "/api/projects/superadmin/consumer/hub/assets/escape-lab.escapee/1.0.0/review",
            json!({ "target_folder": hostile }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "review for {hostile:?}: {body}");
        let install_root = body["review"]["install_root"]
            .as_str()
            .expect("install root")
            .to_string();
        assert!(
            !install_root.contains(".."),
            "target_folder {hostile:?} produced install root {install_root:?}"
        );
        for added in body["review"]["files_added"]
            .as_array()
            .cloned()
            .unwrap_or_default()
        {
            let added = added.as_str().unwrap_or_default();
            assert!(
                !added.contains(".."),
                "target_folder {hostile:?} would write {added}"
            );
        }

        let (status, body) = post_json(
            &app,
            &cookie,
            "/api/projects/superadmin/consumer/hub/assets/escape-lab.escapee/1.0.0/add",
            json!({ "target_folder": hostile }),
        )
        .await;
        assert_eq!(status, StatusCode::OK, "add for {hostile:?}: {body}");
        for identity in body["result"]["pipelines_registered"]
            .as_array()
            .cloned()
            .unwrap_or_default()
        {
            let identity = identity.as_str().expect("identity");
            assert!(!identity.contains(".."), "registered identity {identity}");
            platform
                .projects
                .delete_pipeline("superadmin", "consumer", identity)
                .expect("remove install");
        }
        assert_eq!(
            fs::read_to_string(&canary).expect("canary"),
            "untouched",
            "target_folder {hostile:?} reached outside the project repository"
        );
        assert!(
            !std::path::Path::new("/tmp/zebflow-hub-escape").exists(),
            "target_folder {hostile:?} wrote into /tmp"
        );
        assert!(consumer.repo_dir.is_dir());
    }
}
