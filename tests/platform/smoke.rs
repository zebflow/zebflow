use axum::body::{Body, to_bytes};
use axum::http::{Request, header};
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

#[test]
fn platform_bootstrap_requires_explicit_default_password() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("missing-bootstrap-password");

    let err = build_router(config).expect_err("bootstrap should fail without password");
    assert_eq!(err.code, "PLATFORM_BOOTSTRAP_PASSWORD_MISSING");
}

#[tokio::test]
async fn platform_bootstrap_and_login_flow_works() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("login-flow");
    config.default_password = "test-pass".to_string();
    let data_root = config.data_root.clone();

    let app = build_router(config).expect("platform router");

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
async fn platform_sidebar_active_classes_have_tailwind_utilities_on_section_pages() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("sidebar-tailwind");
    config.default_password = "test-pass".to_string();

    let app = build_router(config).expect("platform router");

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

    let app = build_router(config).expect("platform router");

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

    let app = build_router(config).expect("platform router");

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
        response.headers().get(header::CONTENT_TYPE).and_then(|v| v.to_str().ok()),
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

    let app = build_router(config).expect("platform router");

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
    assert!(policies.iter().any(|policy| policy.policy_id == "agent.templates"));

    let bindings = platform
        .data
        .list_project_policy_bindings("superadmin", "default")
        .expect("project policy bindings");
    assert!(bindings.iter().any(|binding| {
        binding.subject_id == "superadmin" && binding.policy_id == "owner"
    }));

    platform
        .users
        .create_or_update_user(&CreateUserRequest {
            owner: "alice".to_string(),
            password: "alice-pass".to_string(),
            role: "member".to_string(),
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

    let app = build_router(config).expect("platform router");

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

    let app = build_router(config).expect("platform router");

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
