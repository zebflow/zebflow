//! The project dashboard's system panel polls `/api/system/info`, which
//! answers a superadmin only. Through the real router: a superadmin's
//! dashboard carries the panel and its URL; a project member's carries
//! neither (so their browser never polls a 403), and the API refuses them.

use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
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

async fn get(app: &axum::Router, cookie: &str, uri: &str) -> (StatusCode, String) {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(uri).header(header::COOKIE, cookie).body(Body::empty()).expect("request"))
        .await
        .expect("response");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("body");
    (status, String::from_utf8_lossy(&bytes).to_string())
}

#[tokio::test]
async fn the_dashboard_shows_the_system_panel_to_a_superadmin_only() {
    let root = TestDir(std::env::temp_dir().join(format!(
        "zebflow-platform-system-panel-{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    )));
    let mut config = PlatformConfig::default();
    config.data_root = root.0.clone();
    config.default_password = uuid::Uuid::new_v4().to_string();
    let admin_pw = config.default_password.clone();
    let platform = Arc::new(PlatformService::from_config(config).expect("platform"));
    let app = zebflow::platform::web::router(platform.clone()).await;

    let member_pw = uuid::Uuid::new_v4().to_string();
    platform
        .users
        .create_user(&zebflow::platform::CreateUserRequest {
            owner: "developer-a".into(),
            password: member_pw.clone(),
            role: "member".into(),
            git_name: String::new(),
            git_email: String::new(),
        })
        .expect("create user");
    platform
        .project_members
        .upsert_member(
            OWNER,
            OWNER,
            PROJECT,
            &zebflow::platform::model::UpsertProjectMemberRequest {
                user_id: "developer-a".into(),
                role_preset: zebflow::platform::model::ProjectAccessRolePreset::Developer,
                custom_policy_ids: Vec::new(),
                mcp_capability_ceiling: Vec::new(),
            },
        )
        .expect("add member");

    let dashboard = format!("/projects/{OWNER}/{PROJECT}/dashboard");

    let admin = login_as(&app, OWNER, &admin_pw).await;
    let (status, body) = get(&app, &admin, &dashboard).await;
    assert_eq!(status, StatusCode::OK);
    assert!(!body.contains("RWE component error"), "{body}");
    assert!(body.contains("data-api-system-info="), "a superadmin sees the panel");
    assert!(body.contains("/api/system/info"), "a superadmin's panel polls the system API");
    let (status, _) = get(&app, &admin, "/api/system/info").await;
    assert_eq!(status, StatusCode::OK);

    let member = login_as(&app, "developer-a", &member_pw).await;
    let (status, body) = get(&app, &member, &dashboard).await;
    assert_eq!(status, StatusCode::OK, "a member still opens the dashboard");
    assert!(!body.contains("RWE component error"), "{body}");
    assert!(!body.contains("data-api-system-info="), "a member's dashboard has no system panel");
    assert!(!body.contains("/api/system/info"), "a member's browser is never handed the system API");
    assert!(body.contains("instance administrator only"), "a member is told why");
    let (status, _) = get(&app, &member, "/api/system/info").await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}
