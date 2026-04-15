use std::fs;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use tower::ServiceExt;

use zebflow::platform::{
    CreateUserRequest, PlatformConfig, PlatformService, ProjectAccessSubject, ProjectCapability,
    build_router,
};

fn temp_test_dir(name: &str) -> std::path::PathBuf {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    std::env::temp_dir().join(format!("zebflow-platform-{name}-{now}"))
}

async fn response_json(response: axum::response::Response) -> Value {
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body bytes");
    serde_json::from_slice(&body).expect("json body")
}

fn multipart_body(field_name: &str, file_name: &str, bytes: &[u8]) -> (String, Vec<u8>) {
    let boundary = "zebflow-boundary";
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"{field_name}\"; filename=\"{file_name}\"\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(b"Content-Type: application/x-tar\r\n\r\n");
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{boundary}--\r\n").as_bytes());
    (boundary.to_string(), body)
}

#[tokio::test]
async fn platform_bootstrap_requires_explicit_default_password() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("missing-bootstrap-password");

    let err = build_router(config)
        .await
        .expect_err("bootstrap should fail without password");
    assert_eq!(err.code, "PLATFORM_BOOTSTRAP_PASSWORD_MISSING");
}

#[tokio::test]
async fn platform_bootstrap_and_login_flow_works() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("login-flow");
    config.default_password = "test-pass".to_string();
    let data_root = config.data_root.clone();

    let app = build_router(config).await.expect("platform router");

    let login = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/login")
                .method("POST")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from("identifier=superadmin&password=test-pass"))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(login.status(), axum::http::StatusCode::SEE_OTHER);

    let cookie = login
        .headers()
        .get(header::SET_COOKIE)
        .expect("set-cookie")
        .to_str()
        .expect("cookie str")
        .to_string();

    let home = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/home")
                .method("GET")
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(home.status(), axum::http::StatusCode::OK);
    let body = to_bytes(home.into_body(), usize::MAX)
        .await
        .expect("home body bytes");
    let html = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(html.contains("Projects for superadmin"));
    assert!(html.contains("default"));

    let project = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/projects/superadmin/default")
                .method("GET")
                .header(header::COOKIE, "zebflow_session=superadmin")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(project.status(), axum::http::StatusCode::OK);
    let body = to_bytes(project.into_body(), usize::MAX)
        .await
        .expect("project body bytes");
    let html = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(html.contains("Pipelines"));
    assert!(html.contains("Webhooks"));
    assert!(html.contains("Schedules"));
    assert!(html.contains("Functions"));
    assert!(html.contains("Templates"));
    assert!(html.contains("Build"));
    assert!(html.contains("Assets"));
    assert!(html.contains("Schema"));
    assert!(html.contains("Credentials"));
    assert!(html.contains("Tables"));
    assert!(html.contains("Files"));
    assert!(html.contains("Todo"));
    assert!(html.contains("Settings"));
    assert!(html.contains("Pipeline Registry"));
    assert!(html.contains("Path"));
    let project_root = data_root.join("users").join("superadmin").join("default");
    assert!(project_root.join("data").exists());
    assert!(project_root.join("data").join("sekejap").exists());
    assert!(
        project_root
            .join("data")
            .join("sqlite")
            .join("project.db")
            .exists()
    );
    assert!(project_root.join("files").exists());
    assert!(project_root.join("app").exists());
    assert!(project_root.join("app").join(".git").exists());
    assert!(project_root.join("app").join("pipelines").exists());

    let settings = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/projects/superadmin/default/settings")
                .method("GET")
                .header(header::COOKIE, "zebflow_session=superadmin")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(settings.status(), axum::http::StatusCode::OK);
    let body = to_bytes(settings.into_body(), usize::MAX)
        .await
        .expect("settings body bytes");
    let html = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(html.contains("Web Library Manager"));
    assert!(html.contains("Node Manager"));
}

#[tokio::test]
async fn project_docs_support_nested_folder_create_move_and_registry_render() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("docs-nested-registry");
    config.default_password = "test-pass".to_string();

    let app = build_router(config).await.expect("platform router");
    let cookie = "zebflow_session=superadmin";

    let create_folder = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/docs/folder")
                .method("POST")
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({ "path": "guides/archive" })).expect("folder body"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(create_folder.status(), StatusCode::OK);

    let create_doc = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/docs/file?path=guides/archive/intro.md")
                .method("PUT")
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "text/plain")
                .body(Body::from("# Intro"))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(create_doc.status(), StatusCode::OK);

    let move_doc = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/docs/move")
                .method("POST")
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    serde_json::to_vec(&json!({
                        "from_path": "guides/archive/intro.md",
                        "to_parent_path": "guides",
                    }))
                    .expect("move body"),
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(move_doc.status(), StatusCode::OK);
    let moved = response_json(move_doc).await;
    assert_eq!(moved["path"], "guides/intro.md");

    let docs = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/docs")
                .method("GET")
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(docs.status(), StatusCode::OK);
    let docs_json = response_json(docs).await;
    let items = docs_json["items"].as_array().expect("doc items");
    assert!(items.iter().any(|item| item["path"] == "guides"));
    assert!(items.iter().any(|item| item["path"] == "guides/archive"));
    assert!(items.iter().any(|item| item["path"] == "guides/intro.md"));

    let registry = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/projects/superadmin/default/pipelines/registry?path=/docs/guides")
                .method("GET")
                .header(header::COOKIE, cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(registry.status(), StatusCode::OK);
    let body = to_bytes(registry.into_body(), usize::MAX)
        .await
        .expect("registry body bytes");
    let html = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(html.contains("intro.md"));
    assert!(html.contains("/docs/guides"));
}

#[tokio::test]
async fn platform_sidebar_active_classes_have_tailwind_utilities_on_section_pages() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("sidebar-tailwind");
    config.default_password = "test-pass".to_string();

    let app = build_router(config).await.expect("platform router");

    let studio = app
        .oneshot(
            Request::builder()
                .uri("/projects/superadmin/default/build/templates")
                .method("GET")
                .header(header::COOKIE, "zebflow_session=superadmin")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(studio.status(), axum::http::StatusCode::OK);
    let body = to_bytes(studio.into_body(), usize::MAX)
        .await
        .expect("studio body bytes");
    let html = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(html.contains("<style data-rwe-tw>"));
    assert!(html.contains("Zebflow Assistant"));
    // These classes come from dynamic nav class payloads (input.nav.classes.*).
    assert!(html.contains(".py-2{"));
    assert!(html.contains(".rounded-md{border-radius:0.375rem;}"));
}

#[tokio::test]
async fn platform_templates_workspace_renders_seeded_tree_and_editor_bootstrap() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("templates-workspace");
    config.default_password = "test-pass".to_string();

    let app = build_router(config).await.expect("platform router");

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/projects/superadmin/default/build/templates")
                .method("GET")
                .header(header::COOKIE, "zebflow_session=superadmin")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("templates body bytes");
    let html = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(html.contains("template-workspace"));
    assert!(html.contains("data-template-workspace"));
    assert!(html.contains("pages/home.tsx"));
    assert!(html.contains("styles/main.css"));
    assert!(html.contains("Search"));
    assert!(html.contains("Git"));
    assert!(html.contains("/assets/platform/template-editor.mjs"));
    assert!(html.contains("data-template-sonner"));
    assert!(html.contains("data-template-api-diagnostics"));
    assert!(html.contains("zeb/codemirror@0.1"));
}

#[tokio::test]
async fn platform_serves_local_codemirror_library_asset() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("templates-library-asset");
    config.default_password = "test-pass".to_string();

    let app = build_router(config).await.expect("platform router");

    let response = app
        .oneshot(
            Request::builder()
                .uri("/assets/libraries/zeb/codemirror/0.1/runtime/codemirror.bundle.mjs")
                .method("GET")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    assert_eq!(
        response
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("text/javascript; charset=utf-8")
    );
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("library asset body");
    let js = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(js.contains("EditorView"));
    assert!(js.contains("basicSetup"));
}

#[tokio::test]
async fn platform_template_api_supports_create_save_move_delete_and_git_status() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("template-api");
    config.default_password = "test-pass".to_string();

    let app = build_router(config).await.expect("platform router");

    let create = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/templates/create")
                .method("POST")
                .header(header::COOKIE, "zebflow_session=superadmin")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"kind":"component","name":"editor-panel","parent_rel_path":"components"}"#,
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(create.status(), axum::http::StatusCode::OK);
    let body = to_bytes(create.into_body(), usize::MAX)
        .await
        .expect("create body");
    let json = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(json.contains("components/editor-panel.tsx"));

    let save = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/templates/file")
                .method("PUT")
                .header(header::COOKIE, "zebflow_session=superadmin")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"rel_path":"components/editor-panel.tsx","content":"export default function EditorPanel(props) {\n  return <div>Editor</div>;\n}\n"}"#,
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(save.status(), axum::http::StatusCode::OK);

    let git_status = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/templates/git-status")
                .method("GET")
                .header(header::COOKIE, "zebflow_session=superadmin")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(git_status.status(), axum::http::StatusCode::OK);
    let body = to_bytes(git_status.into_body(), usize::MAX)
        .await
        .expect("git body");
    let json = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(json.contains("components/editor-panel.tsx"));
    assert!(json.contains("??"));

    let moved = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/templates/move")
                .method("POST")
                .header(header::COOKIE, "zebflow_session=superadmin")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"from_rel_path":"components/editor-panel.tsx","to_parent_rel_path":"pages"}"#,
                ))
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(moved.status(), axum::http::StatusCode::OK);
    let body = to_bytes(moved.into_body(), usize::MAX)
        .await
        .expect("move body");
    let json = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(json.contains("pages/editor-panel.tsx"));

    let delete = app
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/templates/file?path=pages/editor-panel.tsx")
                .method("DELETE")
                .header(header::COOKIE, "zebflow_session=superadmin")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(delete.status(), axum::http::StatusCode::NO_CONTENT);
}

#[tokio::test]
async fn project_transfer_export_import_roundtrip_restores_repo_and_files() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("transfer-roundtrip");
    config.default_password = "test-pass".to_string();
    let data_root = config.data_root.clone();

    let app = build_router(config).await.expect("platform router");

    let project_root = data_root.join("users").join("superadmin").join("default");
    fs::create_dir_all(project_root.join("files").join("public")).expect("public dir");
    fs::write(
        project_root.join("files").join("public").join("hello.txt"),
        "hello static export\n",
    )
    .expect("seed public file");

    let export_bundle = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/transfer/export/bundle")
                .method("POST")
                .header(header::COOKIE, "zebflow_session=superadmin")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("bundle export response");
    assert_eq!(export_bundle.status(), StatusCode::OK);
    let export_bundle = response_json(export_bundle).await;
    let bundle_op = export_bundle["operation"]["operation_id"]
        .as_str()
        .expect("bundle operation id");

    let export_files = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/transfer/export/files")
                .method("POST")
                .header(header::COOKIE, "zebflow_session=superadmin")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("files export response");
    assert_eq!(export_files.status(), StatusCode::OK);
    let export_files = response_json(export_files).await;
    let files_op = export_files["operation"]["operation_id"]
        .as_str()
        .expect("files operation id");

    let bundle_archive = data_root
        .join("platform")
        .join("project-operations")
        .join(bundle_op)
        .join("project.bundle.tar");
    let files_archive = data_root
        .join("platform")
        .join("project-operations")
        .join(files_op)
        .join("project.files.tar");
    assert!(bundle_archive.exists());
    assert!(files_archive.exists());

    let zebflow_json = project_root.join("repo").join("zebflow.json");
    let mutated = fs::read_to_string(&zebflow_json)
        .expect("zebflow.json")
        .replace("\"Default\"", "\"Mutated Before Import\"");
    fs::write(&zebflow_json, mutated).expect("mutate zebflow.json");
    fs::write(
        project_root.join("files").join("public").join("hello.txt"),
        "mutated file before import\n",
    )
    .expect("mutate hello.txt");

    let bundle_bytes = fs::read(&bundle_archive).expect("bundle archive bytes");
    let (boundary, body) = multipart_body("archive", "project.bundle.tar", &bundle_bytes);
    let import_bundle = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/transfer/import/bundle")
                .method("POST")
                .header(header::COOKIE, "zebflow_session=superadmin")
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .expect("request"),
        )
        .await
        .expect("bundle import response");
    assert_eq!(import_bundle.status(), StatusCode::OK);

    let files_bytes = fs::read(&files_archive).expect("files archive bytes");
    let (boundary, body) = multipart_body("archive", "project.files.tar", &files_bytes);
    let import_files = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/transfer/import/files")
                .method("POST")
                .header(header::COOKIE, "zebflow_session=superadmin")
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .expect("request"),
        )
        .await
        .expect("files import response");
    assert_eq!(import_files.status(), StatusCode::OK);

    let restored = fs::read_to_string(&zebflow_json).expect("restored zebflow.json");
    assert!(restored.contains("\"title\": \"Default\""));
    assert_eq!(
        fs::read_to_string(project_root.join("files").join("public").join("hello.txt"))
            .expect("restored hello.txt"),
        "hello static export\n"
    );

    let operations = app
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/transfer/operations")
                .method("GET")
                .header(header::COOKIE, "zebflow_session=superadmin")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("operations response");
    assert_eq!(operations.status(), StatusCode::OK);
    let operations = response_json(operations).await;
    let items = operations["items"].as_array().expect("operation items");
    assert!(items.iter().any(|item| item["kind"] == "export_bundle"));
    assert!(items.iter().any(|item| item["kind"] == "export_files"));
    assert!(items.iter().any(|item| item["kind"] == "import_bundle"));
    assert!(items.iter().any(|item| item["kind"] == "import_files"));
}

#[tokio::test]
async fn project_transfer_failed_import_is_recorded() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("transfer-failure");
    config.default_password = "test-pass".to_string();
    let data_root = config.data_root.clone();

    let app = build_router(config).await.expect("platform router");

    let export_bundle = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/transfer/export/bundle")
                .method("POST")
                .header(header::COOKIE, "zebflow_session=superadmin")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("bundle export response");
    assert_eq!(export_bundle.status(), StatusCode::OK);
    let export_bundle = response_json(export_bundle).await;
    let bundle_op = export_bundle["operation"]["operation_id"]
        .as_str()
        .expect("bundle operation id");
    let bundle_archive = data_root
        .join("platform")
        .join("project-operations")
        .join(bundle_op)
        .join("project.bundle.tar");

    let create_project = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/users/superadmin/projects")
                .method("POST")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "project": "other-project",
                        "title": "Other Project",
                        "runtime": {}
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("create project response");
    assert_eq!(create_project.status(), StatusCode::OK);

    let bundle_bytes = fs::read(&bundle_archive).expect("bundle archive bytes");
    let (boundary, body) = multipart_body("archive", "project.bundle.tar", &bundle_bytes);
    let import_bundle = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/other-project/transfer/import/bundle")
                .method("POST")
                .header(header::COOKIE, "zebflow_session=superadmin")
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .expect("request"),
        )
        .await
        .expect("import response");
    assert_eq!(import_bundle.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let operations = app
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/other-project/transfer/operations")
                .method("GET")
                .header(header::COOKIE, "zebflow_session=superadmin")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("operations response");
    assert_eq!(operations.status(), StatusCode::OK);
    let operations = response_json(operations).await;
    let failed = operations["items"]
        .as_array()
        .expect("operation items")
        .iter()
        .find(|item| item["kind"] == "import_bundle")
        .expect("failed import record");
    assert_eq!(failed["status"], "failed");
    assert_eq!(failed["current_step"], "import failed");
    assert!(
        failed["error_message"]
            .as_str()
            .expect("error message")
            .contains("archive belongs to superadmin/default")
    );
}

#[test]
fn platform_project_authorization_is_policy_based_and_shared() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("project-authz");
    config.default_password = "test-pass".to_string();

    let platform = PlatformService::from_config(config).expect("platform service");

    let owner_subject = ProjectAccessSubject::user("superadmin");
    let owner_caps = platform
        .authz
        .resolve_project_capabilities(&owner_subject, "superadmin", "default")
        .expect("owner capabilities");
    assert!(owner_caps.contains(&ProjectCapability::TemplatesWrite));
    assert!(owner_caps.contains(&ProjectCapability::SettingsWrite));
    assert!(owner_caps.contains(&ProjectCapability::McpSessionCreate));

    let policies = platform
        .data
        .list_project_policies("superadmin", "default")
        .expect("project policies");
    assert!(policies.iter().any(|policy| policy.policy_id == "owner"));
    assert!(policies.iter().any(|policy| policy.policy_id == "viewer"));
    assert!(
        policies
            .iter()
            .any(|policy| policy.policy_id == "agent.templates")
    );

    let bindings = platform
        .data
        .list_project_policy_bindings("superadmin", "default")
        .expect("project policy bindings");
    assert!(
        bindings
            .iter()
            .any(|binding| { binding.subject_id == "superadmin" && binding.policy_id == "owner" })
    );

    platform
        .users
        .create_or_update_user(&CreateUserRequest {
            owner: "alice".to_string(),
            password: "alice-pass".to_string(),
            role: "member".to_string(),
            git_name: String::new(),
            git_email: String::new(),
        })
        .expect("create alice");
    let alice_subject = ProjectAccessSubject::user("alice");
    let alice_caps = platform
        .authz
        .resolve_project_capabilities(&alice_subject, "superadmin", "default")
        .expect("alice capabilities");
    assert!(alice_caps.is_empty());

    let err = platform
        .authz
        .ensure_project_capability(
            &alice_subject,
            "superadmin",
            "default",
            ProjectCapability::TemplatesRead,
        )
        .expect_err("alice must be denied");
    assert_eq!(err.code, "PLATFORM_AUTHZ_FORBIDDEN");
}

#[tokio::test]
async fn platform_template_diagnostics_reports_compile_errors() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("template-diagnostics");
    config.default_password = "test-pass".to_string();

    let app = build_router(config).await.expect("platform router");

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/templates/diagnostics")
                .method("POST")
                .header(header::COOKIE, "zebflow_session=superadmin")
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"rel_path":"pages/home.tsx","content":"export default function Page(input) { return (<Page><main><div></main></Page>); }"}"#,
                ))
                .expect("request"),
        )
        .await
        .expect("response");

    assert_eq!(response.status(), axum::http::StatusCode::OK);
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("diagnostics body");
    let json = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(json.contains("\"ok\":false"));
    assert!(json.contains("\"severity\":\"error\""));
}

#[tokio::test]
async fn platform_registry_is_hierarchical_from_virtual_path() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("registry-tree");
    config.default_password = "test-pass".to_string();

    let app = build_router(config).await.expect("platform router");

    let root_registry = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/projects/superadmin/default/pipelines/registry?path=/")
                .method("GET")
                .header(header::COOKIE, "zebflow_session=superadmin")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(root_registry.status(), axum::http::StatusCode::OK);
    let body = to_bytes(root_registry.into_body(), usize::MAX)
        .await
        .expect("root registry body");
    let html = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(html.contains("Path"));
    assert!(html.contains("contents"));

    let blog_registry = app
        .oneshot(
            Request::builder()
                .uri("/projects/superadmin/default/pipelines/registry?path=/contents/blog")
                .method("GET")
                .header(header::COOKIE, "zebflow_session=superadmin")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(blog_registry.status(), axum::http::StatusCode::OK);
    let body = to_bytes(blog_registry.into_body(), usize::MAX)
        .await
        .expect("blog registry body");
    let html = String::from_utf8(body.to_vec()).expect("utf8");
    assert!(html.contains("List Posts"));
    assert!(html.contains("Get Post"));
}
