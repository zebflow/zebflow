use std::fs;
use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use base64::Engine as _;
use serde_json::{Value, json};
use tower::ServiceExt;

use zebflow::infra::cluster::config::ClusterRole;
use zebflow::infra::cluster::security::{JoinToken, registration_proof};
use zebflow::platform::model::{
    CollectionAttribute, CreateHubTokenRequest, CreateSimpleTableRequest, TemplateSaveRequest,
    ZebflowJsonDistributionHub,
};
use zebflow::platform::sekejap;
use zebflow::platform::{
    CreateProjectRequest, CreateUserRequest, PlatformConfig, PlatformService, ProjectAccessSubject,
    ProjectCapability, build_router,
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

async fn response_text(response: axum::response::Response) -> String {
    let body = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("response body bytes");
    String::from_utf8(body.to_vec()).expect("utf8 response body")
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
        .expect("set-cookie")
        .to_str()
        .expect("cookie str")
        .to_string()
}

#[test]
fn first_bootstrap_generates_password_and_restart_preserves_it() {
    let data_root = temp_test_dir("generated-superadmin-password");
    let mut config = PlatformConfig::default();
    config.data_root = data_root.clone();

    let platform = PlatformService::from_config(config).expect("first platform bootstrap");
    let password_path = data_root.join(".bootstrap/superadmin-password");
    let generated = fs::read_to_string(&password_path)
        .expect("generated superadmin password")
        .trim()
        .to_string();
    assert_eq!(generated.len(), 48);
    assert!(
        platform
            .auth
            .login("superadmin", &generated)
            .expect("authenticate generated password")
            .is_some()
    );
    drop(platform);

    let mut restarted_config = PlatformConfig::default();
    restarted_config.data_root = data_root.clone();
    restarted_config.default_password = "must-not-replace-existing-password".to_string();
    let restarted =
        PlatformService::from_config(restarted_config).expect("restarted platform bootstrap");
    assert!(
        restarted
            .auth
            .login("superadmin", &generated)
            .expect("authenticate preserved password")
            .is_some()
    );
    assert!(
        !restarted
            .auth
            .login("superadmin", "must-not-replace-existing-password")
            .expect("reject replacement password")
            .is_some()
    );
    drop(restarted);

    let _ = fs::remove_dir_all(data_root);
}

#[tokio::test]
async fn office_refuses_to_start_without_cluster_configuration_and_starts_with_it() {
    let unconfigured_root = temp_test_dir("office-without-cluster-config");
    let mut unconfigured = PlatformConfig::default();
    unconfigured.data_root = unconfigured_root.clone();
    unconfigured.cluster.role = ClusterRole::Worker;
    // The binary always fills advertise_url from its own listen address, so the
    // realistic deployment failure is the two variables only an operator supplies.
    unconfigured.cluster.advertise_url = Some("http://office-a:10610".to_string());

    let refused = build_router(unconfigured)
        .await
        .err()
        .expect("office without a controller URL must refuse to start");
    assert_eq!(refused.code, "CLUSTER_CONFIG_INCOMPLETE");
    assert!(
        refused.message.contains("ZEBFLOW_CLUSTER_MASTER_URL")
            && refused.message.contains("ZEBFLOW_CLUSTER_JOIN_TOKEN"),
        "refusal must name every missing variable at once: {}",
        refused.message
    );
    assert!(
        !unconfigured_root.exists(),
        "a refused office must not create its data root"
    );

    let configured_root = temp_test_dir("office-with-cluster-config");
    let mut configured = PlatformConfig::default();
    configured.data_root = configured_root.clone();
    configured.cluster.role = ClusterRole::Worker;
    configured.cluster.advertise_url = Some("http://office-a:10610".to_string());
    // Unreachable on purpose: registration retries in the background and must not
    // stop the office from serving.
    configured.cluster.master_url = Some("http://127.0.0.1:1".to_string());
    // A real minted token, because `offices.md` §8's token is per-office and
    // self-describing; the shared environment secret this replaced would now
    // refuse to parse.
    configured.cluster.join_token = Some(JoinToken::mint("office-a").render());

    let app = build_router(configured)
        .await
        .expect("office with complete cluster configuration starts");
    let health = app
        .oneshot(
            Request::builder()
                .uri("/health")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("health response");
    assert_eq!(health.status(), StatusCode::OK);
    assert_eq!(response_json(health).await["status"], json!("ok"));

    // `offices.md` §4 keeps institutions on both sides of a join: "every office
    // seeds its own blessed shelf and data root, joined or not; identical bytes
    // for the same release". A standalone instance of the same binary is the
    // yardstick — the office must hold exactly the same coordinates.
    let standalone_root = temp_test_dir("standalone-beside-an-office");
    let mut standalone = PlatformConfig::default();
    standalone.data_root = standalone_root.clone();
    let _standalone = build_router(standalone)
        .await
        .expect("standalone instance starts");

    let office_shelf = blessed_shelf_coordinates(&configured_root);
    assert_eq!(
        office_shelf,
        blessed_shelf_coordinates(&standalone_root),
        "a joined office's blessed shelf must match a standalone instance's"
    );
    assert!(
        !office_shelf.is_empty(),
        "the release seeds a non-empty blessed shelf"
    );

    // §6 re-enables an account that is "disabled, not merely unknown", and §7
    // makes a detached office "a complete instance the moment it leaves".
    // Neither is possible on an office that never created a local account, so
    // the first-boot defaults run for an office exactly as they do standalone.
    assert!(
        configured_root
            .join("users")
            .join(PlatformConfig::default().default_owner)
            .join(PlatformConfig::default().default_project)
            .is_dir(),
        "an office creates its own default owner and project"
    );

    let _ = fs::remove_dir_all(configured_root);
    let _ = fs::remove_dir_all(standalone_root);
}

/// `offices.md` §8: the token is per-office, issued by the controller, and
/// revocable for one office alone, with proof in both directions.
///
/// The property the whole mechanism exists for is the revocation one — one
/// office stops, every other office keeps working — so it is proven explicitly
/// rather than inferred from a refusal count.
#[tokio::test]
async fn a_join_token_is_per_office_revocable_and_proves_both_directions() {
    let controller_root = temp_test_dir("controller-join-tokens");
    let mut controller = PlatformConfig::default();
    controller.data_root = controller_root.clone();
    controller.cluster.role = ClusterRole::Master;
    controller.default_password = "test-pass".to_string();
    // A controller holds no token of its own; it mints one per office.
    let app = build_router(controller).await.expect("controller starts");
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    // --- minting records the holder before it records the secret -----------
    let minted = mint_join_token(&app, &cookie, "office-a", false).await;
    assert_eq!(minted.0, StatusCode::CREATED);
    let token_a = minted.1["token"].as_str().expect("token").to_string();
    assert!(token_a.starts_with("zfjoin1:office-a:"), "{token_a}");
    assert_eq!(minted.1["office"]["office_id"], json!("office-a"));
    assert_eq!(minted.1["office"]["status"], json!("planned"));
    assert_eq!(minted.1["record"]["status"], json!("active"));
    let digest_a = minted.1["record"]["secret_digest"]
        .as_str()
        .expect("digest")
        .to_string();
    assert!(
        !token_a.contains(&digest_a),
        "the stored record must be the digest, not the token"
    );

    // The listing never carries a secret.
    let listed = response_json(
        app.clone()
            .oneshot(
                Request::builder()
                    .uri("/api/cluster/join-tokens")
                    .header(header::COOKIE, &cookie)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("list response"),
    )
    .await;
    let listed = serde_json::to_string(&listed["tokens"]).expect("tokens json");
    assert!(listed.contains(&digest_a));
    assert!(
        !listed.contains(token_a.rsplit(':').next().expect("secret")),
        "a listing must never carry the secret"
    );

    // Minting again for the same office refuses rather than silently rotating.
    let again = mint_join_token(&app, &cookie, "office-a", false).await;
    assert_eq!(again.0, StatusCode::CONFLICT);
    assert_eq!(again.1["error"]["code"], json!("CLUSTER_JOIN_TOKEN_EXISTS"));

    let token_b = mint_join_token(&app, &cookie, "office-b", false).await.1["token"]
        .as_str()
        .expect("token")
        .to_string();

    // --- an office with its own token registers and heartbeats -------------
    let nonce = "0123456789abcdef";
    let registered = register_office(&app, &token_a, "office-a", nonce).await;
    assert_eq!(registered.0, StatusCode::OK);
    assert_eq!(registered.1["ok"], json!(true));
    let proof = registered.1["proof"].as_str().expect("proof").to_string();

    // The office half of the mutual proof: the office derives the same digest
    // from the secret it holds, and an impostor that does not hold it cannot
    // produce this value.
    let identity_a = JoinToken::parse(&token_a).expect("parse");
    assert_eq!(
        proof,
        registration_proof(&identity_a.secret_digest(), "office-a", nonce)
    );
    assert_ne!(
        proof,
        registration_proof(
            &JoinToken::mint("office-a").secret_digest(),
            "office-a",
            nonce
        ),
        "a controller that does not hold the secret cannot answer"
    );

    assert_eq!(
        heartbeat_office(&app, &token_a, "office-a").await.0,
        StatusCode::OK
    );
    // A valid token that has not registered yet passes the door and is turned
    // away by the registry, not by the token check.
    let unregistered = heartbeat_office(&app, &token_b, "office-b").await;
    assert_ne!(unregistered.0, StatusCode::OK);
    assert_eq!(
        unregistered.1["error"]["code"],
        json!("CLUSTER_WORKER_UNKNOWN")
    );
    assert_eq!(
        register_office(&app, &token_b, "office-b", nonce).await.0,
        StatusCode::OK
    );
    assert_eq!(
        heartbeat_office(&app, &token_b, "office-b").await.0,
        StatusCode::OK
    );

    // --- office A's token presented by office B is refused -----------------
    let crossed = register_office(&app, &token_a, "office-b", nonce).await;
    assert_eq!(crossed.0, StatusCode::FORBIDDEN);
    assert_eq!(
        crossed.1["error"]["code"],
        json!("CLUSTER_JOIN_TOKEN_OFFICE_MISMATCH")
    );

    // --- a wrong or malformed token is refused, actionably -----------------
    let legacy = register_office(&app, "join-token", "office-a", nonce).await;
    assert_eq!(legacy.0, StatusCode::UNAUTHORIZED);
    assert_eq!(
        legacy.1["error"]["code"],
        json!("CLUSTER_JOIN_TOKEN_MALFORMED")
    );
    let message = legacy.1["error"]["message"].as_str().expect("message");
    assert!(
        message.contains("Mint one on the controller") && message.contains("join-tokens"),
        "refusal must name the fix: {message}"
    );

    let forged = JoinToken::mint("office-a").render();
    let forged = register_office(&app, &forged, "office-a", nonce).await;
    assert_eq!(forged.0, StatusCode::UNAUTHORIZED);
    assert_eq!(
        forged.1["error"]["code"],
        json!("CLUSTER_JOIN_TOKEN_INVALID")
    );

    let unknown = JoinToken::mint("office-z").render();
    let unknown = register_office(&app, &unknown, "office-z", nonce).await;
    assert_eq!(unknown.0, StatusCode::UNAUTHORIZED);
    assert_eq!(
        unknown.1["error"]["code"],
        json!("CLUSTER_JOIN_TOKEN_UNKNOWN")
    );

    // --- revoking one office stops that office and no other ----------------
    let revoked = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/cluster/join-tokens/office-a/revoke")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("revoke response");
    assert_eq!(revoked.status(), StatusCode::OK);
    assert_eq!(
        response_json(revoked).await["record"]["status"],
        json!("revoked")
    );

    let stopped = heartbeat_office(&app, &token_a, "office-a").await;
    assert_eq!(stopped.0, StatusCode::UNAUTHORIZED);
    assert_eq!(
        stopped.1["error"]["code"],
        json!("CLUSTER_JOIN_TOKEN_REVOKED")
    );
    // An already-registered office must not re-register its way back in.
    let rejoin = register_office(&app, &token_a, "office-a", nonce).await;
    assert_eq!(rejoin.0, StatusCode::UNAUTHORIZED);
    assert_eq!(
        rejoin.1["error"]["code"],
        json!("CLUSTER_JOIN_TOKEN_REVOKED")
    );

    // This is the whole point of the step: office B is untouched.
    assert_eq!(
        heartbeat_office(&app, &token_b, "office-b").await.0,
        StatusCode::OK,
        "revoking one office must not disturb another"
    );

    // Rotation re-issues for the revoked office alone.
    let rotated = mint_join_token(&app, &cookie, "office-a", true).await;
    assert_eq!(rotated.0, StatusCode::CREATED);
    let rotated_token = rotated.1["token"].as_str().expect("token").to_string();
    assert_ne!(rotated_token, token_a);
    assert_eq!(
        register_office(&app, &rotated_token, "office-a", nonce)
            .await
            .0,
        StatusCode::OK
    );
    assert_eq!(
        register_office(&app, &token_a, "office-a", nonce).await.0,
        StatusCode::UNAUTHORIZED,
        "the rotated-away token must not work"
    );

    let _ = fs::remove_dir_all(controller_root);
}

async fn mint_join_token(
    app: &axum::Router,
    cookie: &str,
    office_id: &str,
    rotate: bool,
) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/cluster/join-tokens")
                .method("POST")
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "office_id": office_id, "rotate": rotate }).to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("mint response");
    let status = response.status();
    (status, response_json(response).await)
}

async fn register_office(
    app: &axum::Router,
    token: &str,
    node_id: &str,
    nonce: &str,
) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/internal/cluster/workers/register")
                .method("POST")
                .header("x-zebflow-cluster-token", token)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "node_id": node_id,
                        "label": node_id,
                        "base_url": format!("http://{node_id}:10610"),
                        "nonce": nonce,
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("register response");
    let status = response.status();
    (status, response_json(response).await)
}

async fn heartbeat_office(app: &axum::Router, token: &str, node_id: &str) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/internal/cluster/workers/heartbeat")
                .method("POST")
                .header("x-zebflow-cluster-token", token)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({ "node_id": node_id, "status": "online" }).to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("heartbeat response");
    let status = response.status();
    (status, response_json(response).await)
}

/// Every `{package}@{version}` coordinate on one instance's blessed shelf.
fn blessed_shelf_coordinates(data_root: &Path) -> Vec<String> {
    let packages_dir = data_root
        .join("services")
        .join("hub-local")
        .join("packages");
    let mut coordinates = Vec::new();
    let Ok(packages) = fs::read_dir(&packages_dir) else {
        return coordinates;
    };
    for package in packages.flatten() {
        let package_id = package.file_name().to_string_lossy().to_string();
        let Ok(versions) = fs::read_dir(package.path().join("versions")) else {
            continue;
        };
        for version in versions.flatten() {
            coordinates.push(format!(
                "{package_id}@{}",
                version.file_name().to_string_lossy()
            ));
        }
    }
    coordinates.sort();
    coordinates
}

/// Multipart body with text fields plus one file field, for platform import.
fn multipart_import_body(owner: &str, project: &str, archive: &[u8]) -> (String, Vec<u8>) {
    let boundary = format!(
        "zebflow-boundary-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let mut body = Vec::new();
    for (name, value) in [("owner", owner), ("project", project)] {
        body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
        body.extend_from_slice(
            format!("Content-Disposition: form-data; name=\"{name}\"\r\n\r\n{value}\r\n")
                .as_bytes(),
        );
    }
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(
        b"Content-Disposition: form-data; name=\"archive\"; filename=\"project.full.tar\"\r\n",
    );
    body.extend_from_slice(b"Content-Type: application/x-tar\r\n\r\n");
    body.extend_from_slice(archive);
    body.extend_from_slice(format!("\r\n--{}--\r\n", boundary).as_bytes());
    (boundary, body)
}

fn multipart_body(field_name: &str, file_name: &str, bytes: &[u8]) -> (String, Vec<u8>) {
    multipart_body_with_type(field_name, file_name, "application/x-tar", bytes)
}

fn multipart_body_with_type(
    field_name: &str,
    file_name: &str,
    content_type: &str,
    bytes: &[u8],
) -> (String, Vec<u8>) {
    let boundary = format!(
        "zebflow-boundary-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    let mut body = Vec::new();
    body.extend_from_slice(format!("--{}\r\n", boundary).as_bytes());
    body.extend_from_slice(
        format!(
            "Content-Disposition: form-data; name=\"{field_name}\"; filename=\"{file_name}\"\r\n"
        )
        .as_bytes(),
    );
    body.extend_from_slice(format!("Content-Type: {content_type}\r\n\r\n").as_bytes());
    body.extend_from_slice(bytes);
    body.extend_from_slice(format!("\r\n--{}--\r\n", boundary).as_bytes());
    (boundary, body)
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
                .header(header::COOKIE, &cookie)
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
    assert!(html.contains("Projects for"));
    assert!(html.contains("superadmin"));
    assert!(html.contains("default"));

    let project = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/projects/superadmin/default")
                .method("GET")
                .header(header::COOKIE, cookie.clone())
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
    assert!(html.contains("Dashboard"));
    assert!(html.contains("Pipelines"));
    assert!(html.contains("Credentials"));
    assert!(html.contains("Files"));
    assert!(html.contains("Settings"));
    // `data/` splits into the four tiers `project-directory.md` §3 draws:
    // store (durable), cache (disposable, rebuilds from repo/), recovery and
    // logs (disposable, bounded). A fresh project gets all four up front.
    let project_root = data_root.join("users").join("superadmin").join("default");
    assert!(project_root.join("data").exists());
    assert!(project_root.join("data").join("store").exists());
    assert!(project_root.join("data").join("cache").exists());
    assert!(
        project_root
            .join("data")
            .join("cache")
            .join("pipelines")
            .exists()
    );
    assert!(
        project_root
            .join("data")
            .join("cache")
            .join("agent_docs")
            .exists()
    );
    assert!(project_root.join("data").join("recovery").exists());
    assert!(project_root.join("data").join("logs").exists());
    assert!(project_root.join("files").exists());
    // files/public and files/private are not scaffolded: ZebFS has no physical
    // public/private split, only ZebFsAclManifest per-path visibility
    // (project-directory.md section 4).
    assert!(project_root.join("repo").exists());
    assert!(project_root.join("repo").join(".git").exists());
    assert!(project_root.join("repo").join("pipelines").exists());

    let settings = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/projects/superadmin/default/settings")
                .method("GET")
                .header(header::COOKIE, cookie)
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
    assert!(html.contains("Settings"));
    assert!(html.contains("Project Export"));
    assert!(html.contains("Portability"));
}

#[tokio::test]
async fn public_hub_requires_service_and_hides_project_internals() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("public-hub-boundary");
    config.default_password = "test-pass".to_string();
    let data_root = config.data_root.clone();
    let platform = Arc::new(PlatformService::from_config(config).expect("platform service"));
    let app = zebflow::platform::web::router(platform.clone()).await;

    let disabled = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/hub/remote/assets")
                .method("GET")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("disabled response");
    assert_eq!(disabled.status(), StatusCode::NOT_FOUND);

    platform
        .hub
        .ensure_default_service_instance("standalone", "http://127.0.0.1/api", true)
        .expect("enable hub service");
    platform
        .hub
        .set_authority_enabled("superadmin", "default", true)
        .expect("authority");
    platform
        .hub
        .upsert_publisher(
            "superadmin",
            "default",
            "calc-studio",
            "Calc Studio",
            "https://publishers.example/calc-studio",
            "private@example.com",
            "",
            "",
            "",
            true,
            true,
            true,
            true,
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
            "default",
            "pipelines/internal-calc.zf.json",
            "Internal Calculator",
            "Internal project path should not leak",
            "manual",
            r#"{
  "apiVersion":"zebflow.com/v1",
  "kind":"Pipeline",
  "metadata":{"name":"internal-calc"},
  "spec":{
  "id":"internal-calc",
  "entry_nodes":[],
  "nodes":[],
  "edges":[]}
}"#,
        )
        .expect("pipeline");
    let files_dir = data_root
        .join("users")
        .join("superadmin")
        .join("default")
        .join("files")
        .join("images");
    std::fs::create_dir_all(&files_dir).expect("image dir");
    let mut cover_png = Vec::new();
    image::DynamicImage::ImageRgba8(image::ImageBuffer::from_fn(32, 18, |_x, _y| {
        image::Rgba([32, 96, 160, 255])
    }))
    .write_to(&mut Cursor::new(&mut cover_png), image::ImageFormat::Png)
    .expect("png");
    std::fs::write(files_dir.join("cover.png"), cover_png).expect("write cover");
    platform
        .hub
        .publish_asset(
            "superadmin",
            "default",
            "superadmin",
            "calc-studio",
            "",
            "",
            "",
            "superadmin",
            "default",
            "pipeline_with_dependencies",
            "pipelines/internal-calc.zf.json",
            "calc-tools",
            "1.0.0",
            "Calculator Tools",
            "Reusable calculator pipeline.",
            "",
            "public",
            Default::default(),
            vec!["math".to_string()],
        )
        .expect("publish");

    platform
        .zebflow_cfg
        .set_hub_distribution(
            "superadmin",
            "default",
            ZebflowJsonDistributionHub {
                producer_enabled: true,
                ..Default::default()
            },
        )
        .expect("enable producer mode");
    let (_token, publisher_token) = platform
        .hub
        .create_token(
            "superadmin",
            "default",
            &CreateHubTokenRequest {
                publisher_id: "calc-studio".to_string(),
                title: "Publish token".to_string(),
                scopes: vec!["hub:publish".to_string()],
                expires_at: None,
            },
        )
        .expect("publisher token");
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;
    let no_token_publish = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/hub/remote/assets/publish")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "source_type": "pipeline_with_dependencies",
                        "source_ref": "pipelines/internal-calc.zf.json",
                        "package_id": "calc-tools-no-token",
                        "version": "1.0.0",
                        "publisher_id": "calc-studio",
                        "title": "No Token",
                        "visibility": "public"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("no token publish response");
    assert_eq!(no_token_publish.status(), StatusCode::UNAUTHORIZED);
    let token_publish_review = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/hub/assets/publish-review")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "source_type": "pipeline_with_dependencies",
                        "source_ref": "pipelines/internal-calc.zf.json",
                        "package_id": "calc-tools-token",
                        "version": "1.0.0",
                        "publisher_token": publisher_token,
                        "title": "Token Publish",
                        "image_file_path": "images/cover.png",
                        "visibility": "public"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("token publish review response");
    let token_publish_review_status = token_publish_review.status();
    let token_publish_review_body = response_text(token_publish_review).await;
    assert_eq!(
        token_publish_review_status,
        StatusCode::OK,
        "{token_publish_review_body}"
    );
    let token_publish_review: Value =
        serde_json::from_str(&token_publish_review_body).expect("review json");
    assert_eq!(
        token_publish_review["review"]["package_id"],
        json!("calc-studio.calc-tools-token")
    );
    assert_eq!(
        token_publish_review["review"]["media"][0]["name"],
        json!("cover.webp")
    );
    assert_eq!(
        token_publish_review["review"]["media"][0]["content_type"],
        json!("image/webp")
    );
    let token_publish = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/hub/remote/assets/publish")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "source_type": "pipeline_with_dependencies",
                        "source_ref": "pipelines/internal-calc.zf.json",
                        "package_id": "calc-tools-token",
                        "version": "1.0.0",
                        "publisher_id": "spoofed-publisher",
                        "publisher_token": publisher_token,
                        "title": "Token Publish",
                        "image_file_path": "images/cover.png",
                        "visibility": "public"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("token publish response");
    assert_eq!(token_publish.status(), StatusCode::OK);
    let token_publish = response_json(token_publish).await;
    assert_eq!(
        token_publish["package"]["publisher_id"],
        json!("calc-studio")
    );
    assert_eq!(
        token_publish["package"]["image_url"],
        json!("/api/hub/remote/assets/calc-studio.calc-tools-token/media/cover.webp")
    );
    // Presentation is the mutable half: a typo is corrected by writing the
    // package row, and the cover it does not mention stays where it was.
    let presentation_patch = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/hub/assets/calc-tools-token/presentation")
                .method("PATCH")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "publisher_token": publisher_token,
                        "summary": "Calculators, batteries included.",
                        "description_md": "## Calc Tools\nA long description."
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("presentation patch response");
    assert_eq!(presentation_patch.status(), StatusCode::OK);
    let presentation_patch = response_json(presentation_patch).await;
    assert_eq!(
        presentation_patch["presentation"]["summary"],
        json!("Calculators, batteries included.")
    );
    assert_eq!(
        presentation_patch["presentation"]["image_url"],
        json!("/api/hub/remote/assets/calc-studio.calc-tools-token/media/cover.webp")
    );
    // A scope the system cannot use is refused where it is given, rather than
    // stored as nothing and failing later at a different endpoint.
    let bad_scope_token = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/hub/tokens")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "publisher_id": "calc-studio",
                        "title": "Bad scope token",
                        "scopes": ["read", "publish"],
                        "expires_at": null
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("bad scope token response");
    assert_eq!(bad_scope_token.status(), StatusCode::BAD_REQUEST);
    let bad_scope_token = response_json(bad_scope_token).await;
    assert_eq!(
        bad_scope_token["error"]["code"],
        json!("HUB_TOKEN_SCOPE_INVALID")
    );
    let public_remote_publish = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/hub/remote/assets")
                .method("POST")
                .header(header::AUTHORIZATION, format!("Bearer {publisher_token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "package_id": "calc-tools-remote",
                        "version": "1.0.0",
                        "title": "Remote Calculator Tools",
                        "description": "Published through the platform Hub API.",
                        // Presentation rides beside the release, never inside it.
                        "summary": "Quick calculator pack.",
                        "description_md": "## Remote Calculator Tools\nReusable calculator workflow.",
                        "media": [{
                            "name": "cover.png",
                            "role": "cover",
                            "content_type": "image/png",
                            "size_bytes": 3,
                            "sha256": "b29814cf5792e684cd75d6a7fce7a67a11887e312f87ca2ac2496d81f365ff72",
                            "encoding": "base64",
                            "content": "aW1n"
                        }],
                        "gallery": {
                            "cover": {
                                "kind": "image",
                                "media_name": "cover.png",
                                "alt": "Calculator cover"
                            },
                            "items": [
                                {
                                    "kind": "image",
                                    "media_name": "cover.png",
                                    "alt": "Calculator screenshot"
                                },
                                {
                                    "kind": "youtube",
                                    "url": "https://www.youtube.com/watch?v=dQw4w9WgXcQ",
                                    "title": "Demo video"
                                }
                            ]
                        },
                        "visibility": "public",
                        "tags": ["math", "remote"],
                        "source_owner": "external",
                        "source_project": "remote",
                        "source_kind": "pipeline",
                        "source_ref": "pipelines/remote-calc.zf.json",
                        "artifact": {
                            "apiVersion": "zebflow.com/v1",
                            "kind": "HubPackage",
                            "metadata": {
                                "name": "calc-studio.calc-tools-remote",
                                "version": "1.0.0"
                            },
                            "spec": {
                                "asset_kind": "pipeline_bundle",
                                "title": "Remote Calculator Tools",
                                "description": "Published through the platform Hub API.",
                                "files": [{
                                    "rel_path": "pipelines/remote-calc.zf.json",
                                    "kind": "pipeline",
                                    "size_bytes": 2,
                                    "reason": "primary",
                                    "encoding": "text",
                                    "content": "{}"
                                }]
                            }
                        }
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("public remote publish response");
    assert_eq!(public_remote_publish.status(), StatusCode::OK);
    let public_remote_publish = response_json(public_remote_publish).await;
    assert_eq!(
        public_remote_publish["package"]["package_id"],
        json!("calc-studio.calc-tools-remote")
    );
    assert_eq!(
        public_remote_publish["package"]["publisher_id"],
        json!("calc-studio")
    );
    assert_eq!(
        public_remote_publish["package"]["image_url"],
        json!("/api/hub/remote/assets/calc-studio.calc-tools-remote/media/cover.png")
    );
    let invalid_kind_publish = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/hub/remote/assets")
                .method("POST")
                .header(header::AUTHORIZATION, format!("Bearer {publisher_token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "package_id": "bad-kind-pack",
                        "version": "1.0.0",
                        "title": "Bad Kind",
                        "visibility": "public",
                        "source_owner": "external",
                        "source_project": "remote",
                        "source_kind": "pipeline",
                        "source_ref": "pipelines/bad.zf.json",
                        "artifact": {
                            "apiVersion": "zebflow.com/v1",
                            "kind": "HubPackage",
                            "metadata": {
                                "name": "calc-studio.bad-kind-pack",
                                "version": "1.0.0"
                            },
                            "spec": {
                            "asset_kind": "made_up_kind",
                            "title": "Bad Kind",
                            "description": "Invalid hub asset kind.",
                            "files": [{
                                "rel_path": "pipelines/bad.zf.json",
                                "kind": "pipeline",
                                "size_bytes": 2,
                                "reason": "primary",
                                "encoding": "text",
                                "content": "{}"
                            }]
                            }
                        }
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("invalid kind response");
    assert_eq!(invalid_kind_publish.status(), StatusCode::BAD_REQUEST);
    let invalid_kind_publish = response_json(invalid_kind_publish).await;
    assert_eq!(
        invalid_kind_publish["error"]["code"],
        json!("HUB_ASSET_KIND_INVALID")
    );
    let invalid_gallery_publish = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/hub/remote/assets")
                .method("POST")
                .header(header::AUTHORIZATION, format!("Bearer {publisher_token}"))
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "package_id": "bad-gallery-pack",
                        "version": "1.0.0",
                        "title": "Bad Gallery",
                        "visibility": "public",
                        "gallery": {
                            "items": [{
                                "kind": "youtube",
                                "url": "https://example.com/video",
                                "title": "Not YouTube"
                            }]
                        },
                        "source_owner": "external",
                        "source_project": "remote",
                        "source_kind": "pipeline",
                        "source_ref": "pipelines/bad-gallery.zf.json",
                        "artifact": {
                            "apiVersion": "zebflow.com/v1",
                            "kind": "HubPackage",
                            "metadata": {
                                "name": "calc-studio.bad-gallery-pack",
                                "version": "1.0.0"
                            },
                            "spec": {
                            "asset_kind": "pipeline_bundle",
                            "title": "Bad Gallery",
                            "description": "Invalid gallery.",
                            "files": [{
                                "rel_path": "pipelines/bad-gallery.zf.json",
                                "kind": "pipeline",
                                "size_bytes": 2,
                                "reason": "primary",
                                "encoding": "text",
                                "content": "{}"
                            }]
                            }
                        }
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("invalid gallery response");
    assert_eq!(invalid_gallery_publish.status(), StatusCode::BAD_REQUEST);
    let invalid_gallery_publish = response_json(invalid_gallery_publish).await;
    assert_eq!(
        invalid_gallery_publish["error"]["code"],
        json!("HUB_GALLERY_INVALID")
    );

    let listed = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/hub/remote/assets")
                .method("GET")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("list response");
    assert_eq!(listed.status(), StatusCode::OK);
    let list_body = response_text(listed).await;
    assert!(list_body.contains("calc-studio.calc-tools"));
    assert!(list_body.contains("Quick calculator pack."));
    assert!(list_body.contains("youtube"));
    assert!(
        list_body.contains("/api/hub/remote/assets/calc-studio.calc-tools-token/media/cover.webp")
    );
    assert!(
        list_body.contains("/api/hub/remote/assets/calc-studio.calc-tools-remote/media/cover.png")
    );
    assert!(!list_body.contains("publisher_owner"));
    assert!(!list_body.contains("authority_owner"));
    assert!(!list_body.contains("source_owner"));
    assert!(!list_body.contains("superadmin"));
    assert!(!list_body.contains("private@example.com"));

    let detail = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/hub/remote/assets/calc-studio.calc-tools/1.0.0")
                .method("GET")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("detail response");
    assert_eq!(detail.status(), StatusCode::OK);
    let detail_body = response_text(detail).await;
    assert!(detail_body.contains("calc-studio.calc-tools"));
    assert!(!detail_body.contains("publisher_owner"));
    assert!(!detail_body.contains("authority_owner"));
    assert!(!detail_body.contains("source_owner"));
    assert!(!detail_body.contains("source_project"));
    assert!(!detail_body.contains("source_ref"));
    assert!(!detail_body.contains("\"files\""));
    assert!(!detail_body.contains("superadmin"));
    assert!(!detail_body.contains("private@example.com"));
    assert!(!detail_body.contains("pipelines/internal-calc.zf.json"));

    let remote_detail = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/hub/remote/assets/calc-studio.calc-tools-remote/1.0.0")
                .method("GET")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("remote detail response");
    assert_eq!(remote_detail.status(), StatusCode::OK);
    let remote_detail = response_json(remote_detail).await;
    assert_eq!(
        remote_detail["artifact"]["apiVersion"],
        json!("zebflow.com/v1")
    );
    assert_eq!(remote_detail["artifact"]["kind"], json!("HubPackage"));
    // The release carries only what installing it needs. Everything a human
    // reads while choosing comes from the mutable row beside it.
    for field in [
        "description_md",
        "summary",
        "image_url",
        "gallery",
        "media",
        "publisher_id",
        "publisher_display_name",
        "publisher_url",
        "publisher_email",
        "source_type",
        "source_owner",
        "source_project",
        "source_ref",
    ] {
        assert!(
            remote_detail["artifact"]["spec"].get(field).is_none(),
            "spec.{field} must not be carried by the release"
        );
    }
    assert_eq!(
        remote_detail["presentation"]["description_md"],
        json!("## Remote Calculator Tools\nReusable calculator workflow.")
    );
    assert_eq!(
        remote_detail["presentation"]["summary"],
        json!("Quick calculator pack.")
    );
    assert_eq!(
        remote_detail["presentation"]["gallery"]["items"][1]["kind"],
        json!("youtube")
    );
    assert_eq!(
        remote_detail["presentation"]["media"][0]["name"],
        json!("cover.png")
    );
    // A cover is served from the artifact store, so its bytes never appear in
    // any document a client parses to find files.
    assert!(
        remote_detail["presentation"]["media"][0]
            .get("content")
            .is_none()
    );
    assert!(
        remote_detail["presentation"]["media"][0]
            .get("artifact_sha256")
            .is_none()
    );

    let media = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/hub/remote/assets/calc-studio.calc-tools-token/media/cover.webp")
                .method("GET")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("media response");
    assert_eq!(media.status(), StatusCode::OK);
    assert_eq!(
        media
            .headers()
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok()),
        Some("image/webp")
    );

    let artifact = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/hub/remote/assets/calc-studio.calc-tools/1.0.0/artifact")
                .method("GET")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("artifact response");
    assert_eq!(artifact.status(), StatusCode::OK);
    let artifact_body = response_text(artifact).await;
    assert!(artifact_body.contains("\"artifact_sha256\""));
    assert!(artifact_body.contains("\"artifact_size_bytes\""));
    assert!(artifact_body.contains("\"files\""));
    assert!(artifact_body.contains("pipelines/internal-calc.zf.json"));
    // Token publishes land in the Public Hub store; the blessed shelf keeps
    // its own catalog beside it and is never written by a publish.
    assert!(
        data_root
            .join("services")
            .join("hub-public")
            .join("hub.db")
            .is_file()
    );
    assert!(
        data_root
            .join("services")
            .join("hub-local")
            .join("hub.db")
            .is_file()
    );
    assert!(
        data_root
            .join("services")
            .join("hub-public")
            .join("packages")
            .join("calc-studio.calc-tools")
            .join("versions")
            .join("1.0.0")
            .join("artifact.json")
            .is_file()
    );
    assert!(
        !data_root
            .join("services")
            .join("hub-local")
            .join("packages")
            .join("calc-studio.calc-tools")
            .exists(),
        "a publisher token cannot land a release in the blessed shelf"
    );
    let delete_remote = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/hub/remote/assets/calc-studio.calc-tools-remote")
                .method("DELETE")
                .header(header::AUTHORIZATION, format!("Bearer {publisher_token}"))
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("delete response");
    assert_eq!(delete_remote.status(), StatusCode::OK);
    let delete_remote = response_json(delete_remote).await;
    assert_eq!(delete_remote["retracted_versions"], json!(1));
    // DELETE retracts: the coordinate stays listed and marked, so a project
    // that pinned it reads what happened instead of finding nothing there.
    let listed_after_delete = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/hub/remote/assets")
                .method("GET")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("list after retract response");
    assert_eq!(listed_after_delete.status(), StatusCode::OK);
    let listed_after_delete = response_json(listed_after_delete).await;
    let retracted_item = listed_after_delete["items"]
        .as_array()
        .expect("items")
        .iter()
        .find(|item| item["package_id"] == json!("calc-studio.calc-tools-remote"))
        .expect("a retracted package stays listed");
    assert!(retracted_item["retracted"]["retracted_at"].is_i64());
    assert_eq!(retracted_item["latest_version"], json!(""));
    assert!(
        !data_root
            .join("services")
            .join("hub-public")
            .join("packages")
            .join("calc-studio.calc-tools-remote")
            .join("versions")
            .join("1.0.0")
            .join("artifact.json")
            .exists()
    );
    assert!(
        !data_root
            .join("platform")
            .join("hub")
            .join("assets")
            .exists()
    );
}

#[tokio::test]
async fn hub_scoped_tokens_split_prosumer_and_consumer_projects() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("hub-prosumer-consumer");
    config.default_password = "test-pass".to_string();
    let platform = Arc::new(PlatformService::from_config(config).expect("platform service"));
    let app = zebflow::platform::web::router(platform.clone()).await;
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    platform
        .projects
        .create_or_update_project(
            "superadmin",
            &CreateProjectRequest {
                project: "prosumer-app".to_string(),
                title: Some("Prosumer App".to_string()),
                local_branch: None,
                runtime: Default::default(),
            },
        )
        .expect("prosumer project");
    platform
        .projects
        .create_or_update_project(
            "superadmin",
            &CreateProjectRequest {
                project: "consumer-app".to_string(),
                title: Some("Consumer App".to_string()),
                local_branch: None,
                runtime: Default::default(),
            },
        )
        .expect("consumer project");
    platform
        .hub
        .ensure_default_service_instance("standalone", "http://127.0.0.1/api", true)
        .expect("enable hub service");
    platform
        .hub
        .set_authority_enabled("superadmin", "prosumer-app", true)
        .expect("authority");
    platform
        .hub
        .upsert_publisher(
            "superadmin",
            "prosumer-app",
            "zebflow-labs",
            "Zebflow Labs",
            "https://publishers.example/zebflow-labs",
            "labs@example.com",
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
            "prosumer-app",
            "pipelines/prosumer-demo.zf.json",
            "Prosumer Demo",
            "Published by a scoped publisher token",
            "manual",
            r#"{
  "apiVersion":"zebflow.com/v1",
  "kind":"Pipeline",
  "metadata":{"name":"prosumer-demo"},
  "spec":{
  "id":"prosumer-demo",
  "entry_nodes":[],
  "nodes":[],
  "edges":[]}
}"#,
        )
        .expect("prosumer pipeline");
    let (_prosumer_row, prosumer_token) = platform
        .hub
        .create_token(
            "superadmin",
            "prosumer-app",
            &CreateHubTokenRequest {
                publisher_id: "zebflow-labs".to_string(),
                title: "Prosumer publish".to_string(),
                scopes: vec!["hub:read".to_string(), "hub:publish".to_string()],
                expires_at: None,
            },
        )
        .expect("prosumer token");
    let (_consumer_row, consumer_token) = platform
        .hub
        .create_token(
            "superadmin",
            "consumer-app",
            &CreateHubTokenRequest {
                publisher_id: "zebflow-labs".to_string(),
                title: "Consumer read".to_string(),
                scopes: vec!["hub:read".to_string()],
                expires_at: None,
            },
        )
        .expect("consumer token");

    let prosumer_publish = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/prosumer-app/hub/remote/assets/publish")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "source_type": "pipeline_with_dependencies",
                        "source_ref": "pipelines/prosumer-demo.zf.json",
                        "package_id": "prosumer-demo-pack",
                        "version": "1.0.0",
                        "publisher_token": prosumer_token,
                        "title": "Prosumer Demo Pack",
                        "visibility": "public"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("prosumer publish response");
    assert_eq!(prosumer_publish.status(), StatusCode::OK);
    let prosumer_publish = response_json(prosumer_publish).await;
    assert_eq!(
        prosumer_publish["package"]["publisher_id"],
        json!("zebflow-labs")
    );

    let consumer_publish = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/consumer-app/hub/remote/assets/publish")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "source_type": "pipeline_with_dependencies",
                        "source_ref": "pipelines/prosumer-demo.zf.json",
                        "package_id": "consumer-should-not-publish",
                        "version": "1.0.0",
                        "publisher_token": consumer_token,
                        "title": "Consumer Cannot Publish",
                        "visibility": "public"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("consumer publish response");
    assert_eq!(consumer_publish.status(), StatusCode::UNAUTHORIZED);

    let consumer_install = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(
                    "/api/projects/superadmin/consumer-app/hub/assets/zebflow-labs.prosumer-demo-pack/1.0.0/add",
                )
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"target_folder": "/hub/zebflow-labs.prosumer-demo-pack"}).to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("consumer install response");
    assert_eq!(consumer_install.status(), StatusCode::OK);
    let consumer_install = response_json(consumer_install).await;
    assert_eq!(consumer_install["ok"], json!(true));

    let installed = platform
        .projects
        .read_pipeline_source(
            "superadmin",
            "consumer-app",
            "pipelines/hub/zebflow-labs.prosumer-demo-pack/prosumer-demo.zf.json",
        )
        .expect("installed pipeline");
    assert!(installed.contains("prosumer-demo"));
}

#[tokio::test]
async fn project_bundle_installs_spatial_blog_with_sekejap_schema_across_two_instances() {
    unsafe {
        std::env::set_var("ZEBFLOW_HUB_ALLOW_LOCALHOST_REMOTE", "1");
    }

    let mut publisher_config = PlatformConfig::default();
    publisher_config.data_root = temp_test_dir("hub-project-bundle-publisher");
    publisher_config.default_password = "test-pass".to_string();
    let publisher = Arc::new(PlatformService::from_config(publisher_config).expect("publisher"));
    let publisher_app = zebflow::platform::web::router(publisher.clone()).await;
    let publisher_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("publisher listener");
    let publisher_addr = publisher_listener.local_addr().expect("publisher addr");
    let publisher_server = tokio::spawn(async move {
        axum::serve(publisher_listener, publisher_app)
            .await
            .expect("publisher server");
    });

    publisher
        .projects
        .create_or_update_project(
            "superadmin",
            &CreateProjectRequest {
                project: "spatial-blogging-source".to_string(),
                title: Some("Spatial Blogging".to_string()),
                local_branch: None,
                runtime: Default::default(),
            },
        )
        .expect("source project");
    sekejap::create_table(
        &publisher.config.data_root,
        "superadmin",
        "spatial-blogging-source",
        &CreateSimpleTableRequest {
            table: "posts".to_string(),
            title: Some("Spatial Posts".to_string()),
            attributes: vec![
                CollectionAttribute {
                    name: "title".to_string(),
                    kind: "string".to_string(),
                    index_types: vec!["hash".to_string(), "fulltext".to_string()],
                    default_value: None,
                },
                CollectionAttribute {
                    name: "body".to_string(),
                    kind: "text".to_string(),
                    index_types: vec!["fulltext".to_string()],
                    default_value: None,
                },
                CollectionAttribute {
                    name: "geometry".to_string(),
                    kind: "geo".to_string(),
                    index_types: vec!["spatial".to_string()],
                    default_value: None,
                },
                CollectionAttribute {
                    name: "embedding".to_string(),
                    kind: "vector".to_string(),
                    index_types: vec!["vector".to_string()],
                    default_value: None,
                },
            ],
            hash_indexed_fields: Vec::new(),
            range_indexed_fields: Vec::new(),
        },
    )
    .expect("source sekejap schema");
    let source_layout = publisher
        .projects
        .project_layout("superadmin", "spatial-blogging-source")
        .expect("source layout");
    let seed_path = source_layout
        .repo_dir
        .join("initial-data")
        .join("sekejap")
        .join("posts.sql");
    std::fs::create_dir_all(seed_path.parent().unwrap()).expect("seed dir");
    std::fs::write(
        &seed_path,
        "INSERT INTO posts (_key, title, body) VALUES ('hello-melbourne', 'Hello Melbourne', 'Seeded spatial starter post');\n",
    )
    .expect("seed file");
    publisher
        .projects
        .write_template_file(
            "superadmin",
            "spatial-blogging-source",
            &TemplateSaveRequest {
                rel_path: "pages/spatial-blog.tsx".to_string(),
                content: r#"
export default function SpatialBlogPage({ input }) {
  const count = Array.isArray(input?.rows) ? input.rows.length : 0;
  return (
    <main className="min-h-screen bg-slate-950 px-8 py-10 text-white">
      <section className="mx-auto max-w-4xl">
        <p className="text-sm uppercase tracking-wide text-emerald-300">Spatial blogging</p>
        <h1 className="mt-3 text-4xl font-semibold">Spatial Blogging Starter</h1>
        <p className="mt-4 text-slate-300">A project bundle with a live Sekejap schema for text, geometry, and embedding fields.</p>
        <div className="mt-8 grid gap-3 sm:grid-cols-3">
          <div className="border border-emerald-500/40 bg-emerald-500/10 p-4">
            <p className="text-xs text-emerald-200">Rows</p>
            <p className="mt-2 text-2xl font-semibold">{count}</p>
          </div>
          <div className="border border-sky-500/40 bg-sky-500/10 p-4">Geo field ready</div>
          <div className="border border-fuchsia-500/40 bg-fuchsia-500/10 p-4">Vector field ready</div>
        </div>
      </section>
    </main>
  );
}
"#
                .trim()
                .to_string(),
            },
        )
        .expect("template");
    publisher
        .projects
        .upsert_pipeline_definition(
            "superadmin",
            "spatial-blogging-source",
            "pipelines/pages/spatial-blog.zf.json",
            "Spatial Blog",
            "Render spatial blog posts from Sekejap.",
            "webhook",
            r#"{
  "apiVersion":"zebflow.com/v1",
  "kind":"Pipeline",
  "metadata":{"name":"spatial-blog"},
  "spec":{
  "id":"spatial-blog",
  "entry_nodes":["trigger"],
  "nodes":[
    {"id":"trigger","kind":"n.trigger.webhook","input_pins":[],"output_pins":["out"],"config":{"path":"/blog","method":"GET"}},
    {"id":"query","kind":"n.sekejap.query","input_pins":["in"],"output_pins":["out"],"config":{"query":"SELECT * FROM posts LIMIT 20","limit":20,"read_only":true}},
    {"id":"response","kind":"n.web.response","input_pins":["in"],"output_pins":["out"],"config":{"template":"pages/spatial-blog.tsx"}}
  ],
  "edges":[
    {"from_node":"trigger","from_pin":"out","to_node":"query","to_pin":"in"},
    {"from_node":"query","from_pin":"out","to_node":"response","to_pin":"in"}
  ]}
}"#,
        )
        .expect("pipeline");
    publisher
        .projects
        .activate_pipeline_definition(
            "superadmin",
            "spatial-blogging-source",
            "pipelines/pages/spatial-blog.zf.json",
        )
        .expect("activate source pipeline");
    publisher
        .hub
        .ensure_default_service_instance(
            "standalone",
            &format!("http://{publisher_addr}/api"),
            true,
        )
        .expect("enable publisher hub");
    publisher
        .hub
        .upsert_publisher(
            "superadmin",
            "spatial-blogging-source",
            "zebflow-labs",
            "Zebflow Labs",
            "https://publishers.example/zebflow-labs",
            "labs@example.com",
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
        .expect("publisher identity");
    publisher
        .hub
        .publish_asset(
            "superadmin",
            "spatial-blogging-source",
            "superadmin",
            "zebflow-labs",
            "",
            "",
            "",
            "superadmin",
            "spatial-blogging-source",
            "project_files",
            ".",
            "spatial-blogging",
            "1.0.0",
            "Spatial Blogging Starter",
            "Cloneable project bundle with Sekejap spatial and vector schema.",
            "",
            "public",
            zebflow::platform::services::hub::HubProjectBundlePublishOptions {
                include_sekejap_schema: true,
                include_sqlite_schema: false,
                include_libraries: Vec::new(),
                include_initial_data: true,
                initial_data_paths: vec!["initial-data/sekejap/posts.sql".to_string()],
            },
            vec!["spatial".to_string(), "blog".to_string()],
        )
        .expect("publish project bundle");

    let mut consumer_config = PlatformConfig::default();
    consumer_config.data_root = temp_test_dir("hub-project-bundle-consumer");
    consumer_config.default_password = "test-pass".to_string();
    let consumer = Arc::new(PlatformService::from_config(consumer_config).expect("consumer"));
    let consumer_app = zebflow::platform::web::router(consumer.clone()).await;
    let cookie = login_cookie(consumer_app.clone(), "superadmin", "test-pass").await;
    consumer
        .hub
        .upsert_platform_repository(
            "superadmin",
            "local-publisher",
            "Local Publisher",
            &format!("http://{publisher_addr}/api"),
            "",
            "",
            "",
            "api",
            None,
            "public",
            true,
        )
        .expect("consumer hub source");

    // The review comes first and must describe this install without performing
    // any part of it.
    let review = consumer_app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/platform/hub/install/review")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "repository_id": "local-publisher",
                        "package_id": "zebflow-labs.spatial-blogging",
                        "version": "1.0.0"
                    })
                    .to_string(),
                ))
                .expect("review request"),
        )
        .await
        .expect("review response");
    let review_status = review.status();
    let review = response_json(review).await;
    assert_eq!(review_status, StatusCode::OK, "{review}");
    let review = review["review"].clone();
    println!(
        "install review: {}",
        serde_json::to_string_pretty(&review).expect("review json")
    );
    assert_eq!(review["installable"], json!(true), "{review}");
    assert!(
        !review["database_initialization"]
            .as_array()
            .expect("database_initialization array")
            .is_empty(),
        "the review says what the seed SQL does before it runs"
    );
    let reviewed_project = review["project"]
        .as_str()
        .expect("reviewed project slug")
        .to_string();
    assert!(
        consumer
            .projects
            .get_project("superadmin", &reviewed_project)
            .expect("project lookup")
            .is_none(),
        "the review created no project"
    );
    assert!(
        !consumer
            .config
            .data_root
            .join(format!("users/superadmin/{reviewed_project}"))
            .exists(),
        "the review wrote no files and no store"
    );

    // The account-scoped route is the same review under a different auth rule,
    // and a review can be repeated because it changes nothing.
    let account_review = consumer_app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/users/superadmin/hub/install/review")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "repository_id": "local-publisher",
                        "package_id": "zebflow-labs.spatial-blogging",
                        "version": "1.0.0"
                    })
                    .to_string(),
                ))
                .expect("account review request"),
        )
        .await
        .expect("account review response");
    assert_eq!(account_review.status(), StatusCode::OK);
    assert_eq!(response_json(account_review).await["review"], review);

    let install = consumer_app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/platform/hub/install")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "repository_id": "local-publisher",
                        "package_id": "zebflow-labs.spatial-blogging",
                        "version": "1.0.0"
                    })
                    .to_string(),
                ))
                .expect("install request"),
        )
        .await
        .expect("install response");
    let install_status = install.status();
    let install = response_json(install).await;
    assert_eq!(install_status, StatusCode::OK, "{install}");
    let installed_project = install["project"]["project"]
        .as_str()
        .expect("installed project slug")
        .to_string();

    // What the review promised is what the install reports having done.
    assert_eq!(installed_project, reviewed_project);
    for field in [
        "files_written",
        "skipped_files",
        "pipelines_registered",
        "pipelines_activated",
        "pipelines_not_activated",
        "unexecuted_initial_data",
        "database_initialization",
        "schema_executed",
    ] {
        assert_eq!(
            install["install"][field], review[field],
            "the install's {field} is what the review showed"
        );
    }

    let cloned_tables =
        sekejap::list_tables(&consumer.config.data_root, "superadmin", &installed_project)
            .expect("cloned tables");
    assert_eq!(cloned_tables.len(), 1);
    assert_eq!(cloned_tables[0].table, "posts");
    assert!(
        cloned_tables[0]
            .spatial_fields
            .contains(&"geometry".to_string())
    );
    assert!(
        cloned_tables[0]
            .vector_fields
            .contains(&"embedding".to_string())
    );
    let seeded = sekejap::execute_sql(
        &consumer.config.data_root,
        "superadmin",
        &installed_project,
        "SELECT * FROM posts",
        &[],
        20,
        true,
    )
    .expect("seeded rows");
    assert_eq!(seeded.rows.len(), 1);

    let rendered = consumer_app
        .clone()
        .oneshot(
            Request::builder()
                .uri(format!("/wh/superadmin/{installed_project}/blog"))
                .method("GET")
                .body(Body::empty())
                .expect("render request"),
        )
        .await
        .expect("render response");
    assert_eq!(rendered.status(), StatusCode::OK);
    let html = response_text(rendered).await;
    assert!(html.contains("Spatial Blogging Starter"));
    assert!(html.contains("Vector field ready"));

    publisher_server.abort();
}

#[tokio::test]
async fn hub_add_reviews_risks_and_respects_target_folders() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("hub-add-review");
    config.default_password = "test-pass".to_string();
    let platform = Arc::new(PlatformService::from_config(config).expect("platform service"));
    let app = zebflow::platform::web::router(platform.clone()).await;
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    platform
        .projects
        .create_or_update_project(
            "superadmin",
            &CreateProjectRequest {
                project: "producer-review".to_string(),
                title: Some("Producer Review".to_string()),
                local_branch: None,
                runtime: Default::default(),
            },
        )
        .expect("producer project");
    platform
        .projects
        .create_or_update_project(
            "superadmin",
            &CreateProjectRequest {
                project: "consumer-review".to_string(),
                title: Some("Consumer Review".to_string()),
                local_branch: None,
                runtime: Default::default(),
            },
        )
        .expect("consumer project");
    platform
        .hub
        .ensure_default_service_instance("standalone", "http://127.0.0.1/api", true)
        .expect("enable hub service");
    platform
        .hub
        .set_authority_enabled("superadmin", "producer-review", true)
        .expect("authority");
    platform
        .hub
        .upsert_publisher(
            "superadmin",
            "producer-review",
            "review-lab",
            "Review Lab",
            "https://publishers.example/review-lab",
            "review@example.com",
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
            "producer-review",
            "pipelines/safety-demo.zf.json",
            "Safety Demo",
            "Pipeline with reviewable effects",
            "webhook",
            r#"{
  "apiVersion":"zebflow.com/v1",
  "kind":"Pipeline",
  "metadata":{"name":"safety-demo"},
  "spec":{
  "id":"safety-demo",
  "entry_nodes":["wh"],
  "nodes":[
    {"id":"wh","kind":"n.trigger.webhook","config":{"path":"/unsafe-public-hook"}},
    {"id":"http","kind":"n.http.request","config":{"url":"https://api.example.com/v1/items","credential":"secure-egress"}},
    {"id":"pg","kind":"n.pg.query","config":{"credential":"pg-main"}},
    {"id":"fs","kind":"n.fs.put","config":{"path":"exports/out.json"}}
  ],
  "edges":[]}
}"#,
        )
        .expect("producer pipeline");
    platform
        .hub
        .publish_asset(
            "superadmin",
            "producer-review",
            "superadmin",
            "review-lab",
            "",
            "",
            "",
            "superadmin",
            "producer-review",
            "pipeline_with_dependencies",
            "pipelines/safety-demo.zf.json",
            "safety-demo",
            "1.0.0",
            "Safety Demo",
            "Reviewable risky pipeline.",
            "",
            "public",
            Default::default(),
            vec!["review".to_string()],
        )
        .expect("publish");

    let review = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(
                    "/api/projects/superadmin/consumer-review/hub/assets/review-lab.safety-demo/1.0.0/review",
                )
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"target_folder": "/safety-demo"}).to_string()))
                .expect("request"),
        )
        .await
        .expect("review response");
    assert_eq!(review.status(), StatusCode::OK);
    let review = response_json(review).await;
    assert_eq!(review["review"]["risk_level"], json!("high"));
    assert_eq!(
        review["review"]["files_added"][0],
        json!("pipelines/safety-demo/safety-demo.zf.json")
    );
    assert!(
        review["review"]["nodes_used"]
            .as_array()
            .expect("nodes")
            .contains(&json!("n.http.request"))
    );
    assert!(
        review["review"]["credentials_required"]
            .as_array()
            .expect("credentials")
            .contains(&json!("secure-egress"))
    );
    assert!(
        review["review"]["external_urls"]
            .as_array()
            .expect("urls")
            .contains(&json!("https://api.example.com/v1/items"))
    );
    assert!(
        review["review"]["public_endpoints"]
            .as_array()
            .expect("endpoints")
            .contains(&json!("/unsafe-public-hook"))
    );
    assert!(
        review["review"]["database_effects"]
            .as_array()
            .expect("database effects")
            .contains(&json!("n.pg.query"))
    );
    assert!(
        review["review"]["filesystem_effects"]
            .as_array()
            .expect("filesystem effects")
            .contains(&json!("n.fs.put"))
    );

    let nested_review = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(
                    "/api/projects/superadmin/consumer-review/hub/assets/review-lab.safety-demo/1.0.0/review",
                )
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({"target_folder": "/functions/blogging/safety-demo"}).to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("nested review response");
    assert_eq!(nested_review.status(), StatusCode::OK);
    let nested_review = response_json(nested_review).await;
    assert_eq!(
        nested_review["review"]["files_added"][0],
        json!("pipelines/functions/blogging/safety-demo/safety-demo.zf.json")
    );

    let add = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(
                    "/api/projects/superadmin/consumer-review/hub/assets/review-lab.safety-demo/1.0.0/add",
                )
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({"target_folder": "/safety-demo"}).to_string()))
                .expect("request"),
        )
        .await
        .expect("add response");
    assert_eq!(add.status(), StatusCode::OK);
    let added = platform
        .projects
        .read_pipeline_source(
            "superadmin",
            "consumer-review",
            "safety-demo/safety-demo.zf.json",
        )
        .expect("added pipeline");
    assert!(added.contains("unsafe-public-hook"));

    // A target folder used to be pulled into the source root only when the
    // caller typed a leading slash, so `billing` installed outside it, was
    // reviewed as risk-free with every finding list empty, and registered
    // nothing. All three spellings now review and register the same pipeline.
    // One webhook path may only be claimed once, so each spelling is installed
    // on its own and removed again.
    let mut previous_install = Some("safety-demo/safety-demo.zf.json".to_string());
    for (target_folder, expected_root) in [
        ("", "pipelines/hub/review-lab.safety-demo"),
        ("billing", "pipelines/billing"),
        ("/billing", "pipelines/billing"),
    ] {
        if let Some(previous) = previous_install.take() {
            platform
                .projects
                .delete_pipeline("superadmin", "consumer-review", &previous)
                .expect("remove previous install");
        }
        let review = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(
                        "/api/projects/superadmin/consumer-review/hub/assets/review-lab.safety-demo/1.0.0/review",
                    )
                    .method("POST")
                    .header(header::COOKIE, &cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({ "target_folder": target_folder }).to_string(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("target folder review response");
        assert_eq!(review.status(), StatusCode::OK);
        let review = response_json(review).await;
        assert_eq!(
            review["review"]["install_root"],
            json!(expected_root),
            "install root for target_folder {target_folder:?}"
        );
        assert_eq!(
            review["review"]["risk_level"],
            json!("high"),
            "risk level for target_folder {target_folder:?}"
        );
        assert!(
            review["review"]["public_endpoints"]
                .as_array()
                .expect("endpoints")
                .contains(&json!("/unsafe-public-hook")),
            "public endpoints for target_folder {target_folder:?}"
        );

        let add = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(
                        "/api/projects/superadmin/consumer-review/hub/assets/review-lab.safety-demo/1.0.0/add",
                    )
                    .method("POST")
                    .header(header::COOKIE, &cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        json!({ "target_folder": target_folder }).to_string(),
                    ))
                    .expect("request"),
            )
            .await
            .expect("target folder add response");
        assert_eq!(add.status(), StatusCode::OK);
        let add = response_json(add).await;
        let registered = add["result"]["pipelines_registered"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        assert_eq!(
            registered,
            review["review"]["pipelines_registered"]
                .as_array()
                .cloned()
                .unwrap_or_default(),
            "review and install disagree for target_folder {target_folder:?}"
        );
        assert_eq!(
            registered.len(),
            1,
            "nothing registered for target_folder {target_folder:?}: {add}"
        );
        let identity = registered[0].as_str().expect("identity");
        assert!(
            platform
                .projects
                .get_pipeline_meta_by_file_id("superadmin", "consumer-review", identity)
                .expect("meta lookup")
                .is_some(),
            "no catalog row for target_folder {target_folder:?}"
        );
        previous_install = Some(identity.to_string());
    }
}

#[tokio::test]
async fn project_hub_ui_is_project_surface_with_explicit_grant() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("project-hub-surface");
    config.default_password = "test-pass".to_string();
    let app = build_router(config).await.expect("platform router");
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;
    let create_project = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/users/superadmin/projects")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "project": "market-test",
                        "title": "Market Test"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("create project response");
    assert_eq!(create_project.status(), StatusCode::OK);

    let page = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/projects/superadmin/market-test/hub")
                .method("GET")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("hub page response");
    let page_status = page.status();
    let page_body = response_text(page).await;
    assert_eq!(page_status, StatusCode::OK, "hub page: {page_body}");
    assert!(page_body.contains("Browse"));
    assert!(page_body.contains("Published"));
    assert!(page_body.contains("Publish"));
    assert!(!page_body.contains("Create Hub Token"));
    assert!(!page_body.contains("Publisher Registry"));
    assert!(!page_body.contains("Pack Repositories"));
    assert!(!page_body.contains("Project Publishing Access"));

    let repos = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/market-test/hub/repositories")
                .method("GET")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("repository response");
    assert_eq!(repos.status(), StatusCode::OK);
    let repos = response_json(repos).await;
    let items = repos["items"].as_array().expect("repository items");
    assert!(
        items.is_empty(),
        "project should not inherit Hub access without an explicit grant"
    );

    let source = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/platform/hub/repositories")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "repository_id": "zebflow-com",
                        "title": "Zebflow Hub",
                        "base_url": "https://93.184.216.34/api",
                        "remote_owner": "",
                        "remote_project": "",
                        "visibility": "public",
                        "enabled": true
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("platform source response");
    let source_status = source.status();
    let source_body = response_text(source).await;
    assert_eq!(
        source_status,
        StatusCode::OK,
        "platform source: {source_body}"
    );

    let grant = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/platform/hub/grants")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "repository_id": "zebflow-com",
                        "grant_scope": "all_projects",
                        "can_read": true,
                        "can_publish": false,
                        "can_manage": false,
                        "enabled": true
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("grant response");
    let grant_status = grant.status();
    let grant_body = response_text(grant).await;
    assert_eq!(grant_status, StatusCode::OK, "grant: {grant_body}");
    let grant: Value = serde_json::from_str(&grant_body).expect("grant json");
    assert_eq!(grant["grant"]["grant_scope"], json!("all_projects"));

    let repos = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/market-test/hub/access")
                .method("GET")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("effective access response");
    assert_eq!(repos.status(), StatusCode::OK);
    let repos = response_json(repos).await;
    let items = repos["repositories"].as_array().expect("repository items");
    assert!(items.iter().any(|item| {
        item["repository_id"] == json!("zebflow-com")
            && item["base_url"] == json!("https://93.184.216.34/api")
            && item["enabled"] == json!(true)
    }));
}

#[tokio::test]
async fn private_project_files_require_project_capability() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("private-file-authz");
    config.default_password = "test-pass".to_string();
    let data_root = config.data_root.clone();

    let app = build_router(config).await.expect("platform router");
    let project_files = data_root
        .join("users")
        .join("superadmin")
        .join("default")
        .join("files");
    fs::create_dir_all(project_files.join("public")).expect("public dir");
    fs::create_dir_all(project_files.join("private")).expect("private dir");
    fs::write(project_files.join("public").join("hello.txt"), "hello").expect("public file");
    fs::write(project_files.join("private").join("secret.txt"), "secret").expect("private file");

    let public = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/files/superadmin/default/public/hello.txt")
                .method("GET")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("public response");
    assert_eq!(public.status(), StatusCode::OK);
    assert_eq!(response_text(public).await, "hello");

    let anonymous_private = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/files/superadmin/default/private/secret.txt")
                .method("GET")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("anonymous private response");
    assert_eq!(anonymous_private.status(), StatusCode::UNAUTHORIZED);

    let forged_private = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/files/superadmin/default/private/secret.txt")
                .method("GET")
                .header(header::COOKIE, "zebflow_session=superadmin")
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("forged private response");
    assert_eq!(forged_private.status(), StatusCode::UNAUTHORIZED);

    let superadmin_cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;
    let create_user = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/users")
                .method("POST")
                .header(header::COOKIE, &superadmin_cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "owner": "bob",
                        "password": "bob-pass",
                        "role": "member"
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("create user response");
    assert_eq!(create_user.status(), StatusCode::OK);

    let bob_cookie = login_cookie(app.clone(), "bob", "bob-pass").await;
    let other_user_private = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/files/superadmin/default/private/secret.txt")
                .method("GET")
                .header(header::COOKIE, bob_cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("other user private response");
    assert_eq!(other_user_private.status(), StatusCode::FORBIDDEN);

    let owner_private = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/files/superadmin/default/private/secret.txt")
                .method("GET")
                .header(header::COOKIE, &superadmin_cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("owner private response");
    assert_eq!(owner_private.status(), StatusCode::OK);
    assert_eq!(response_text(owner_private).await, "secret");

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("/etc/passwd", project_files.join("private").join("escape"))
            .expect("symlink");
        let symlink_escape = app
            .oneshot(
                Request::builder()
                    .uri("/files/superadmin/default/private/escape")
                    .method("GET")
                    .header(header::COOKIE, superadmin_cookie)
                    .body(Body::empty())
                    .expect("request"),
            )
            .await
            .expect("symlink response");
        assert_eq!(symlink_escape.status(), StatusCode::BAD_REQUEST);
    }
}

#[tokio::test]
#[ignore = "manual local TTS smoke using Narrator Piper assets"]
async fn platform_tts_upload_credential_and_execute_smoke() {
    let model_src = Path::new("/path/to/piper/narrator.onnx");
    let config_src = Path::new("/path/to/piper/narrator.onnx.json");
    if !(model_src.is_file() && config_src.is_file()) {
        eprintln!("local TTS smoke assets are not present; skipping");
        return;
    }

    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("tts-api-smoke");
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
    assert_eq!(login.status(), StatusCode::SEE_OTHER);
    let cookie = login
        .headers()
        .get(header::SET_COOKIE)
        .expect("set-cookie")
        .to_str()
        .expect("cookie str")
        .to_string();

    let upload_specs = vec![
        (
            "private/voices/narrator".to_string(),
            "narrator.onnx".to_string(),
            fs::read(model_src).expect("read model"),
            "application/octet-stream".to_string(),
        ),
        (
            "private/voices/narrator".to_string(),
            "narrator.onnx.json".to_string(),
            fs::read(config_src).expect("read config"),
            "application/json".to_string(),
        ),
    ];
    for (path, file_name, bytes, content_type) in upload_specs {
        let (boundary, body) = multipart_body_with_type("file", &file_name, &content_type, &bytes);
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/projects/superadmin/default/files/upload?path={path}"
                    ))
                    .method("POST")
                    .header(header::COOKIE, &cookie)
                    .header(
                        header::CONTENT_TYPE,
                        format!("multipart/form-data; boundary={boundary}"),
                    )
                    .body(Body::from(body))
                    .expect("request"),
            )
            .await
            .expect("upload response");
        let status = response.status();
        let body_text = response_text(response).await;
        assert_eq!(
            status,
            StatusCode::OK,
            "upload failed for {path}/{file_name}: {body_text}"
        );
    }

    let cred_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/credentials")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "credential_id": "narrator-tts",
                        "title": "Narrator TTS",
                        "kind": "tts",
                        "notes": "",
                        "secret": {
                            "provider": "piper",
                            "model_file": "voices/narrator/narrator.onnx",
                            "config_file": "voices/narrator/narrator.onnx.json"
                        }
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("credential response");
    assert_eq!(cred_response.status(), StatusCode::OK);

    let dsl = r#"register pipelines/tests/tts-api
[a] trigger.manual
[b] ai.tts --provider piper --credential narrator-tts --text-expr "$input.text" --output-path-expr "'audio/' + $input.slug + '.wav'" --return both
[a] -> [b]
&& activate pipelines/tests/tts-api.zf.json"#;
    let dsl_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/pipelines/dsl")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(json!({ "dsl": dsl }).to_string()))
                .expect("request"),
        )
        .await
        .expect("dsl response");
    assert_eq!(dsl_response.status(), StatusCode::OK);
    let dsl_json = response_json(dsl_response).await;
    assert_eq!(dsl_json["ok"], json!(true), "dsl output: {dsl_json}");

    let exec_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/pipelines/execute")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "file_rel_path": "pipelines/tests/tts-api.zf.json",
                        "trigger": "manual",
                        "input": {
                            "text": "Halo, ini Narrator dari API smoke n.ai.tts.",
                            "slug": "narrator-api-smoke"
                        }
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("execute response");
    let exec_status = exec_response.status();
    let exec_body_text = response_text(exec_response).await;
    assert_eq!(
        exec_status,
        StatusCode::OK,
        "execute failed: {exec_body_text}"
    );
    let exec_json: Value = serde_json::from_str(&exec_body_text).expect("execute json");
    assert_eq!(exec_json["ok"], json!(true), "execute output: {exec_json}");
    let output = &exec_json["output"];
    let file_path = output["audio"]["path"]
        .as_str()
        .expect("audio.path should be present");
    let file_url = output["audio"]["url"]
        .as_str()
        .expect("audio.url should be present");
    let blob = output["audio_blob_base64"]
        .as_str()
        .expect("audio_blob_base64 should be present");
    let blob_bytes = base64::engine::general_purpose::STANDARD
        .decode(blob)
        .expect("decode blob");
    assert!(
        blob_bytes.starts_with(b"RIFF"),
        "expected WAV blob to start with RIFF"
    );
    assert_eq!(file_path, "private/audio/narrator-api-smoke.wav");
    assert!(file_url.ends_with("/private/audio/narrator-api-smoke.wav"));

    let file_response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(file_url)
                .method("GET")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("file response");
    assert_eq!(file_response.status(), StatusCode::OK);
    let file_bytes = to_bytes(file_response.into_body(), usize::MAX)
        .await
        .expect("file bytes");
    assert!(file_bytes.starts_with(b"RIFF"));

    let disk_path = data_root
        .join("users")
        .join("superadmin")
        .join("default")
        .join("files")
        .join(file_path);
    assert!(
        disk_path.is_file(),
        "expected file on disk: {}",
        disk_path.display()
    );
}

#[tokio::test]
async fn project_docs_support_nested_folder_create_move_and_registry_render() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("docs-nested-registry");
    config.default_password = "test-pass".to_string();

    let app = build_router(config).await.expect("platform router");
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    let create_folder = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/docs/folder")
                .method("POST")
                .header(header::COOKIE, &cookie)
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
                .header(header::COOKIE, &cookie)
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
                .header(header::COOKIE, &cookie)
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
                .header(header::COOKIE, &cookie)
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
                .header(header::COOKIE, &cookie)
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
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    let studio = app
        .oneshot(
            Request::builder()
                .uri("/projects/superadmin/default/pipelines/registry?path=/")
                .method("GET")
                .header(header::COOKIE, &cookie)
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
    assert!(html.contains("Editor"));
    assert!(html.contains("Root"));
    assert!(html.contains("Registry"));
}

#[tokio::test]
async fn platform_templates_workspace_renders_seeded_tree_and_editor_bootstrap() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("templates-workspace");
    config.default_password = "test-pass".to_string();

    let app = build_router(config).await.expect("platform router");
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/projects/superadmin/default/pipelines/registry?path=/")
                .method("GET")
                .header(header::COOKIE, &cookie)
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
    assert!(html.contains("Root"));
    assert!(html.contains("Templates"));
    assert!(html.contains("Editor"));
    assert!(html.contains("Registry"));
    assert!(html.contains("Add+"));
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
async fn a_declared_source_root_moves_templates_pipelines_and_assets() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("declared-source-root");
    config.default_password = "test-pass".to_string();
    let platform = Arc::new(PlatformService::from_config(config).expect("platform service"));
    let app = zebflow::platform::web::router(platform.clone()).await;
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    platform
        .projects
        .create_or_update_project(
            "superadmin",
            &CreateProjectRequest {
                project: "declared".to_string(),
                title: Some("Declared".to_string()),
                local_branch: None,
                runtime: Default::default(),
            },
        )
        .expect("project");
    platform
        .zebflow_cfg
        .update("superadmin", "declared", |cfg| {
            cfg.configs.layout.source = Some("src".to_string());
        })
        .expect("declare layout");

    let layout = platform
        .projects
        .project_layout("superadmin", "declared")
        .expect("layout");
    assert_eq!(layout.repo_source_dir(), layout.repo_dir.join("src"));
    assert_eq!(layout.repo_assets_dir(), layout.repo_dir.join("src/assets"));

    let create = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/declared/templates/create")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    r#"{"kind":"component","name":"panel","parent_rel_path":"components"}"#,
                ))
                .expect("request"),
        )
        .await
        .expect("create response");
    assert_eq!(create.status(), StatusCode::OK);
    assert!(layout.repo_dir.join("src/components/panel.tsx").is_file());
    assert!(
        !layout
            .repo_dir
            .join("pipelines/components/panel.tsx")
            .exists()
    );

    let register = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/declared/pipelines/definition")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(
                    json!({
                        "file_rel_path": "blog/feed",
                        "title": "Feed",
                        "description": "",
                        "trigger_kind": "webhook",
                        "source": r#"{"apiVersion":"zebflow.com/v1","kind":"Pipeline","metadata":{"name":"feed"},"spec":{"id":"feed","entry_nodes":["wh"],"nodes":[{"id":"wh","kind":"n.trigger.webhook","input_pins":[],"output_pins":["out"],"config":{"path":"/feed","method":"GET"}}],"edges":[]}}"#
                    })
                    .to_string(),
                ))
                .expect("request"),
        )
        .await
        .expect("register response");
    assert_eq!(register.status(), StatusCode::OK);
    let register = response_json(register).await;
    // Identity has no root segment; the root lives in zebflow.yaml.
    assert_eq!(
        register["meta"]["file_rel_path"],
        json!("blog/feed.zf.json")
    );
    assert!(layout.repo_dir.join("src/blog/feed.zf.json").is_file());

    let registry = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/declared/pipelines/registry?scope=project")
                .method("GET")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("registry response");
    assert_eq!(registry.status(), StatusCode::OK);
    let registry = response_json(registry).await;
    let listed = registry["items"].as_array().cloned().unwrap_or_default();
    assert_eq!(
        listed.len(),
        1,
        "registry did not discover the pipeline: {registry}"
    );
    assert_eq!(listed[0]["file_rel_path"], json!("blog/feed.zf.json"));

    std::fs::create_dir_all(layout.repo_assets_dir()).expect("assets dir");
    std::fs::write(layout.repo_assets_dir().join("logo.txt"), b"declared-asset")
        .expect("asset file");
    let asset = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/assets/superadmin/declared/logo.txt")
                .method("GET")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("asset response");
    assert_eq!(asset.status(), StatusCode::OK);
    assert_eq!(response_text(asset).await, "declared-asset");
}

#[tokio::test]
async fn platform_template_api_supports_create_save_move_delete_and_git_status() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("template-api");
    config.default_password = "test-pass".to_string();

    let app = build_router(config).await.expect("platform router");
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    let create = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/templates/create")
                .method("POST")
                .header(header::COOKIE, &cookie)
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
                .header(header::COOKIE, &cookie)
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
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("response");
    assert_eq!(git_status.status(), axum::http::StatusCode::OK);
    let json = response_json(git_status).await;
    assert!(json.is_array());

    let moved = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/templates/move")
                .method("POST")
                .header(header::COOKIE, &cookie)
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
                .header(header::COOKIE, &cookie)
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
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

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
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("bundle export response");
    if export_bundle.status() != StatusCode::OK {
        let status = export_bundle.status();
        let body = response_text(export_bundle).await;
        panic!("bundle export failed with {status}: {body}");
    }
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
                .header(header::COOKIE, &cookie)
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

    let zebflow_yaml = project_root.join("repo").join("zebflow.yaml");
    let mutated = fs::read_to_string(&zebflow_yaml)
        .expect("zebflow.yaml")
        .replace("title: Default", "title: Mutated Before Import");
    fs::write(&zebflow_yaml, mutated).expect("mutate zebflow.yaml");
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
                .header(header::COOKIE, &cookie)
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
                .header(header::COOKIE, &cookie)
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

    let restored = fs::read_to_string(&zebflow_yaml).expect("restored zebflow.yaml");
    assert!(restored.contains("title: Default"));
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
                .header(header::COOKIE, &cookie)
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
async fn platform_import_creates_project_and_auto_initiates_repo_only_store() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("platform-import");
    config.default_password = "test-pass".to_string();
    let data_root = config.data_root.clone();

    let app = build_router(config).await.expect("platform router");
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    // Fixture state on the default project: declared initial data in repo/
    // (the layout's `initial-data/sekejap` prefix) plus one live-only store
    // row the initial data does not know about — the discriminator between
    // "steps replayed" and "snapshot restored".
    let seed_dir = data_root.join("users/superadmin/default/repo/initial-data/sekejap");
    fs::create_dir_all(&seed_dir).expect("seed dir");
    fs::write(
        seed_dir.join("seed.sql"),
        "CREATE TABLE seeded (id TEXT, title TEXT);\nINSERT INTO seeded (id, title) VALUES ('s1', 'from-initial-data');\n",
    )
    .expect("seed file");
    zebflow::platform::sekejap::execute_sql(
        &data_root,
        "superadmin",
        "default",
        "CREATE TABLE seeded (id TEXT, title TEXT)",
        &[],
        0,
        false,
    )
    .expect("create live table");
    zebflow::platform::sekejap::execute_sql(
        &data_root,
        "superadmin",
        "default",
        "INSERT INTO seeded (id, title) VALUES ('live1', 'live-only-row')",
        &[],
        0,
        false,
    )
    .expect("insert live row");

    let export_full = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/transfer/export/full")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("full export response");
    assert_eq!(export_full.status(), StatusCode::OK);
    let export_full = response_json(export_full).await;
    let full_archive = data_root.join("platform").join("project-operations").join(
        export_full["operation"]["artifact_rel_path"]
            .as_str()
            .expect("artifact path"),
    );
    let full_bytes = fs::read(&full_archive).expect("full archive bytes");

    // A repo + store import restores the snapshot and replays nothing.
    let (boundary, body) = multipart_import_body("superadmin", "full-clone", &full_bytes);
    let full_import = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/platform/transfer/import")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .expect("request"),
        )
        .await
        .expect("full import response");
    assert_eq!(full_import.status(), StatusCode::OK);
    let full_import = response_json(full_import).await;
    assert_eq!(full_import["store_auto_initiated"], json!(false));
    assert_eq!(full_import["initial_data_replayed"], json!([]));
    let cloned = zebflow::platform::sekejap::execute_sql(
        &data_root,
        "superadmin",
        "full-clone",
        "SELECT id FROM seeded",
        &[],
        0,
        true,
    )
    .expect("query full clone");
    let mut cloned_ids = cloned
        .rows
        .iter()
        .map(|row| row[0].as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();
    cloned_ids.sort();
    // The source never replayed its declared seed, so the snapshot holds only
    // the live row — and the clone restores exactly that, replaying nothing.
    assert_eq!(cloned_ids, vec!["live1"], "snapshot wins whole");

    // A repo-only import auto-initiates: declared steps replay into a fresh
    // store, so the seeded row exists and the live-only row does not.
    let unpack = data_root.join("tmp-repo-only");
    fs::create_dir_all(&unpack).expect("unpack dir");
    let extract = std::process::Command::new("tar")
        .arg("-xf")
        .arg(&full_archive)
        .arg("-C")
        .arg(&unpack)
        .status()
        .expect("tar extract");
    assert!(extract.success());
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(unpack.join("manifest.json")).expect("manifest"))
            .expect("manifest json");
    manifest["spec"]["classes"] = json!(["repo"]);
    let repo_digest = manifest["spec"]["class_digests"]["repo"].clone();
    manifest["spec"]["class_digests"] = json!({ "repo": repo_digest });
    let repo_count = manifest["spec"]["counts"]["repo"].clone();
    let total = manifest["spec"]["counts"]["total_bytes"].clone();
    manifest["spec"]["counts"] = json!({ "repo": repo_count, "total_bytes": total });
    fs::write(
        unpack.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest).expect("manifest bytes"),
    )
    .expect("manifest write");
    fs::remove_dir_all(unpack.join("store")).expect("drop store class");
    fs::remove_dir_all(unpack.join("files")).expect("drop files class");
    let repo_only_archive = data_root.join("repo-only.tar");
    let pack = std::process::Command::new("tar")
        .arg("-cf")
        .arg(&repo_only_archive)
        .arg("-C")
        .arg(&unpack)
        .arg(".")
        .status()
        .expect("tar create");
    assert!(pack.success());

    let repo_only_bytes = fs::read(&repo_only_archive).expect("repo-only bytes");
    let (boundary, body) = multipart_import_body("superadmin", "init-clone", &repo_only_bytes);
    let repo_only_import = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/platform/transfer/import")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .expect("request"),
        )
        .await
        .expect("repo-only import response");
    assert_eq!(repo_only_import.status(), StatusCode::OK);
    let repo_only_import = response_json(repo_only_import).await;
    assert_eq!(repo_only_import["store_auto_initiated"], json!(true));
    assert_eq!(
        repo_only_import["initial_data_replayed"],
        json!(["initial-data/sekejap/seed.sql"])
    );
    let fresh = zebflow::platform::sekejap::execute_sql(
        &data_root,
        "superadmin",
        "init-clone",
        "SELECT id FROM seeded",
        &[],
        0,
        true,
    )
    .expect("query fresh clone");
    let fresh_ids = fresh
        .rows
        .iter()
        .map(|row| row[0].as_str().unwrap_or_default().to_string())
        .collect::<Vec<_>>();
    assert_eq!(fresh_ids, vec!["s1"], "fresh store has only declared data");

    // A second platform import at the same name refuses: it creates, never
    // replaces — replacing is the project-scope import's job.
    let (boundary, body) = multipart_import_body("superadmin", "init-clone", &repo_only_bytes);
    let duplicate = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/platform/transfer/import")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .expect("request"),
        )
        .await
        .expect("duplicate import response");
    assert_eq!(duplicate.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn project_transfer_import_records_provenance_and_failures() {
    let mut config = PlatformConfig::default();
    config.data_root = temp_test_dir("transfer-failure");
    config.default_password = "test-pass".to_string();
    let data_root = config.data_root.clone();

    let app = build_router(config).await.expect("platform router");
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    let export_bundle = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/transfer/export/bundle")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("bundle export response");
    if export_bundle.status() != StatusCode::OK {
        let status = export_bundle.status();
        let body = response_text(export_bundle).await;
        panic!("bundle export failed with {status}: {body}");
    }
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
                .header(header::COOKIE, &cookie)
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

    // Identity is provenance, never a gate (`kinds/project-bundle/README.md`):
    // importing superadmin/default's archive into other-project succeeds, the
    // provenance is recorded, and the displaced classes have recovery copies.
    let bundle_bytes = fs::read(&bundle_archive).expect("bundle archive bytes");
    let (boundary, body) = multipart_body("archive", "project.bundle.tar", &bundle_bytes);
    let import_bundle = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/other-project/transfer/import/bundle")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .expect("request"),
        )
        .await
        .expect("import response");
    if import_bundle.status() != StatusCode::OK {
        let status = import_bundle.status();
        let body = response_text(import_bundle).await;
        panic!("foreign-provenance import failed with {status}: {body}");
    }
    let import_bundle = response_json(import_bundle).await;
    assert_eq!(import_bundle["import"]["provenance"], "superadmin/default");
    let recovery = import_bundle["import"]["recovery"]
        .as_array()
        .expect("recovery swaps");
    assert_eq!(recovery.len(), 2);
    for swap in recovery {
        let recovery_dir = data_root
            .join("users/superadmin/other-project/data/recovery")
            .join(swap["recovery_dir"].as_str().expect("recovery dir"));
        assert!(recovery_dir.is_dir(), "missing {}", recovery_dir.display());
    }

    // A genuinely broken archive fails, and the operation record says so.
    // (Operation ids carry second resolution; step past the completed one.)
    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    let (boundary, body) = multipart_body("archive", "project.bundle.tar", b"not a tar archive");
    let import_garbage = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/other-project/transfer/import/bundle")
                .method("POST")
                .header(header::COOKIE, &cookie)
                .header(
                    header::CONTENT_TYPE,
                    format!("multipart/form-data; boundary={boundary}"),
                )
                .body(Body::from(body))
                .expect("request"),
        )
        .await
        .expect("import response");
    assert_eq!(import_garbage.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let operations = app
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/other-project/transfer/operations")
                .method("GET")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("request"),
        )
        .await
        .expect("operations response");
    assert_eq!(operations.status(), StatusCode::OK);
    let operations = response_json(operations).await;
    let items = operations["items"].as_array().expect("operation items");
    let failed = items
        .iter()
        .find(|item| item["kind"] == "import_bundle" && item["status"] == "failed")
        .expect("failed import record");
    assert_eq!(failed["current_step"], "import failed");
    let completed = items
        .iter()
        .find(|item| item["kind"] == "import_bundle" && item["status"] == "completed")
        .expect("completed import record");
    assert!(
        completed["current_step"]
            .as_str()
            .expect("completed step")
            .contains("import completed from 'superadmin/default'")
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
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    let response = app
        .oneshot(
            Request::builder()
                .uri("/api/projects/superadmin/default/templates/diagnostics")
                .method("POST")
                .header(header::COOKIE, &cookie)
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
    let cookie = login_cookie(app.clone(), "superadmin", "test-pass").await;

    let root_registry = app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/projects/superadmin/default/pipelines/registry?path=/")
                .method("GET")
                .header(header::COOKIE, &cookie)
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
    assert!(html.contains("Editor"));
    assert!(html.contains("Registry"));

    let blog_registry = app
        .oneshot(
            Request::builder()
                .uri("/projects/superadmin/default/pipelines/registry?path=/contents/blog")
                .method("GET")
                .header(header::COOKIE, &cookie)
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
    assert!(html.contains("Editor"));
    assert!(html.contains("Registry"));
}

/// A static repository is a directory over HTTP, and it reaches the same gates.
///
/// `distribution.md` §3 says every channel runs one review. This proves it for
/// the channel that has no server behind it: the package is served as two plain
/// files, and the install refuses it for exactly the reasons a hub-served one
/// would be refused, through the same `ProjectBundleInstallPlan`.
///
/// It also proves the answer to the mutability question the same section asks.
/// The index pins the release document's digest, so replacing that document
/// under its path fails closed rather than installing whatever is there now.
#[tokio::test]
async fn a_static_repository_installs_through_the_same_review_and_pins_its_release() {
    unsafe {
        std::env::set_var("ZEBFLOW_HUB_ALLOW_LOCALHOST_REMOTE", "1");
        // The two official sources are seeded into every instance and this test
        // must not reach the internet to find that out. Pointed at the discard
        // port they refuse instantly, which is also the state this test wants
        // them in: an unreachable source is reported, not dropped.
        std::env::set_var("ZEBFLOW_HUB_DEFAULT_BASE_URL", "http://127.0.0.1:9/api");
        std::env::set_var("ZEBFLOW_HUB_DEFAULT_STATIC_BASE_URL", "http://127.0.0.1:9");
    }

    // --- a publisher, only so the fixture is a real project bundle ---------
    let mut publisher_config = PlatformConfig::default();
    publisher_config.data_root = temp_test_dir("static-repo-publisher");
    publisher_config.default_password = "test-pass".to_string();
    let publisher = Arc::new(PlatformService::from_config(publisher_config).expect("publisher"));
    publisher
        .projects
        .create_or_update_project(
            "superadmin",
            &CreateProjectRequest {
                project: "static-source".to_string(),
                title: Some("Static Source".to_string()),
                local_branch: None,
                runtime: Default::default(),
            },
        )
        .expect("source project");
    publisher
        .projects
        .write_template_file(
            "superadmin",
            "static-source",
            &TemplateSaveRequest {
                rel_path: "pages/home.tsx".to_string(),
                content: "export default function Home() { return <main>Static</main>; }\n"
                    .to_string(),
            },
        )
        .expect("template");
    publisher
        .projects
        .upsert_pipeline_definition(
            "superadmin",
            "static-source",
            "pipelines/pages/home.zf.json",
            "Home",
            "Serve the home page.",
            "webhook",
            r#"{
  "apiVersion":"zebflow.com/v1",
  "kind":"Pipeline",
  "metadata":{"name":"home"},
  "spec":{
  "id":"home",
  "entry_nodes":["trigger"],
  "nodes":[
    {"id":"trigger","kind":"n.trigger.webhook","input_pins":[],"output_pins":["out"],"config":{"path":"/","method":"GET"}},
    {"id":"response","kind":"n.web.response","input_pins":["in"],"output_pins":["out"],"config":{"template":"pages/home.tsx"}}
  ],
  "edges":[
    {"from_node":"trigger","from_pin":"out","to_node":"response","to_pin":"in"}
  ]}
}"#,
        )
        .expect("pipeline");
    publisher
        .hub
        .ensure_default_service_instance("standalone", "http://127.0.0.1:1/api", true)
        .expect("enable publisher hub");
    publisher
        .hub
        .upsert_publisher(
            "superadmin",
            "static-source",
            "zebflow-labs",
            "Zebflow Labs",
            "https://publishers.example/zebflow-labs",
            "labs@example.com",
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
        .expect("publisher identity");
    publisher
        .hub
        .publish_asset(
            "superadmin",
            "static-source",
            "superadmin",
            "zebflow-labs",
            "",
            "",
            "",
            "superadmin",
            "static-source",
            "project_files",
            ".",
            "static-tool",
            "1.0.0",
            "Static Tool",
            "A project bundle served by a plain directory.",
            "",
            "public",
            zebflow::platform::services::hub::HubProjectBundlePublishOptions::default(),
            vec!["demo".to_string()],
        )
        .expect("publish project bundle");
    let (_, document) = publisher
        .hub
        .get_asset_version_artifact("zebflow-labs.static-tool", "1.0.0")
        .expect("artifact document");

    // --- the repository: an index and one document, and nothing else -------
    let static_root = temp_test_dir("static-repo-files");
    let release_rel = "packages/zebflow-labs.static-tool/1.0.0/package.json";
    let release_path = static_root.join(release_rel);
    fs::create_dir_all(release_path.parent().expect("release dir")).expect("release dir");
    let release_bytes = serde_json::to_vec_pretty(&document).expect("release bytes");
    fs::write(&release_path, &release_bytes).expect("release file");
    let write_index = |sha256: &str, size_bytes: usize| {
        fs::write(
            static_root.join("zebflow-repository.json"),
            serde_json::to_vec_pretty(&json!({
                "apiVersion": "zebflow.com/v1",
                "kind": "HubRepositoryIndex",
                "metadata": {"name": "test-repository"},
                "spec": {"packages": [{
                    "package_id": "zebflow-labs.static-tool",
                    "asset_kind": "project_bundle",
                    "title": "Static Tool",
                    "description": "A project bundle served by a plain directory.",
                    "latest_version": "1.0.0",
                    "releases": [{
                        "version": "1.0.0",
                        "path": release_rel,
                        "sha256": sha256,
                        "size_bytes": size_bytes,
                    }],
                }]},
            }))
            .expect("index bytes"),
        )
        .expect("index file");
    };
    write_index(&sha256_hex(&release_bytes), release_bytes.len());

    // A plain file server. No Zebflow, no API, no logic: the whole point of the
    // channel is that this is all a repository has to be.
    let served_root = static_root.clone();
    let files = axum::Router::new().route(
        "/{*path}",
        axum::routing::get(
            move |axum::extract::Path(path): axum::extract::Path<String>| {
                let root = served_root.clone();
                async move {
                    use axum::response::IntoResponse as _;
                    match fs::read(root.join(path)) {
                        Ok(bytes) => (StatusCode::OK, bytes).into_response(),
                        Err(_) => StatusCode::NOT_FOUND.into_response(),
                    }
                }
            },
        ),
    );
    let files_listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("file listener");
    let files_addr = files_listener.local_addr().expect("file addr");
    let files_server = tokio::spawn(async move {
        axum::serve(files_listener, files)
            .await
            .expect("file server");
    });

    // --- a consumer that has that directory as its second source -----------
    let mut consumer_config = PlatformConfig::default();
    consumer_config.data_root = temp_test_dir("static-repo-consumer");
    consumer_config.default_password = "test-pass".to_string();
    let consumer = Arc::new(PlatformService::from_config(consumer_config).expect("consumer"));
    let consumer_app = zebflow::platform::web::router(consumer.clone()).await;
    let cookie = login_cookie(consumer_app.clone(), "superadmin", "test-pass").await;
    let repository = consumer
        .hub
        .upsert_platform_repository(
            "superadmin",
            "test-static",
            "Test Static Repository",
            &format!("http://{files_addr}"),
            "",
            "",
            "",
            "static",
            Some(20),
            "public",
            true,
        )
        .expect("static source");
    assert_eq!(repository.kind, "static");
    assert_eq!(repository.priority, 20);

    let body = json!({
        "repository_id": "test-static",
        "package_id": "zebflow-labs.static-tool",
        "version": "1.0.0",
    })
    .to_string();
    let post = |uri: &'static str, body: String, cookie: String, app: axum::Router| async move {
        app.oneshot(
            Request::builder()
                .uri(uri)
                .method("POST")
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(body))
                .expect("request"),
        )
        .await
        .expect("response")
    };

    // The listing walks the configured sources in order and names each one.
    let listing = consumer_app
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/platform/hub/assets")
                .header(header::COOKIE, &cookie)
                .body(Body::empty())
                .expect("listing request"),
        )
        .await
        .expect("listing response");
    let listing = response_json(listing).await;
    let sources = listing["sources"].as_array().expect("sources array");
    // Priority ascending, then repository id: the official static source at
    // 10 first, then this test's static source and the official API hub at
    // 20, id-ordered (`distribution.md` §2, decided 2026-08-27).
    assert_eq!(
        sources
            .iter()
            .map(|item| item["repository_id"].as_str().unwrap_or_default())
            .collect::<Vec<_>>(),
        vec!["zebflow-hub", "test-static", "zebflow-com"],
        "sources are returned in resolution order: {listing}"
    );
    assert_eq!(sources[1]["kind"], json!("static"), "{listing}");
    assert_eq!(sources[1]["ok"], json!(true), "{listing}");
    // A source that did not answer is named as such rather than dropped, which
    // is what lets one "not found" error tell the two cases apart.
    assert_eq!(sources[2]["ok"], json!(false), "{listing}");
    assert!(
        !sources[2]["error"].as_str().unwrap_or_default().is_empty(),
        "an unreachable source says why: {listing}"
    );
    assert_eq!(
        listing["items"]
            .as_array()
            .expect("items")
            .iter()
            .filter(|item| item["repository_id"] == json!("test-static"))
            .count(),
        1,
        "the static repository's package is listed: {listing}"
    );

    let review = post(
        "/api/platform/hub/install/review",
        body.clone(),
        cookie.clone(),
        consumer_app.clone(),
    )
    .await;
    let review_status = review.status();
    let review = response_json(review).await;
    assert_eq!(review_status, StatusCode::OK, "{review}");
    assert_eq!(review["review"]["installable"], json!(true), "{review}");

    let install = post(
        "/api/platform/hub/install",
        body.clone(),
        cookie.clone(),
        consumer_app.clone(),
    )
    .await;
    let install_status = install.status();
    let install = response_json(install).await;
    assert_eq!(install_status, StatusCode::OK, "{install}");
    let installed = install["install"]["project"]
        .as_str()
        .expect("installed project")
        .to_string();
    assert!(
        consumer
            .projects
            .get_project("superadmin", &installed)
            .expect("project lookup")
            .is_some(),
        "a static repository materialises a project like any other channel"
    );

    // --- the release is pinned: replacing it under its path fails closed ---
    let mut tampered: Value = serde_json::from_slice(&release_bytes).expect("document json");
    tampered["spec"]["description"] = json!("Replaced after the index was published.");
    fs::write(
        &release_path,
        serde_json::to_vec_pretty(&tampered).expect("tampered bytes"),
    )
    .expect("tampered file");

    let refused = post(
        "/api/platform/hub/install",
        body.clone(),
        cookie.clone(),
        consumer_app.clone(),
    )
    .await;
    let refused = response_json(refused).await;
    assert_eq!(
        refused["error"]["code"],
        json!("HUB_REMOTE_HASH_MISMATCH"),
        "a document swapped under its path is refused: {refused}"
    );

    // --- and the review's refusals are the install's refusals --------------
    // A pipeline whose bytes the review cannot read is a violation, and it is a
    // violation on this channel too, reached through the same plan.
    let mut hostile: Value = serde_json::from_slice(&release_bytes).expect("document json");
    for file in hostile["spec"]["files"]
        .as_array_mut()
        .expect("files array")
        .iter_mut()
    {
        if file["rel_path"]
            .as_str()
            .is_some_and(|path| path.ends_with(".zf.json"))
        {
            let raw = vec![0x00_u8, 0x01, 0x02, 0xff, 0xfe].repeat(20);
            file["encoding"] = json!("base64");
            file["content"] = json!(base64::engine::general_purpose::STANDARD.encode(&raw));
            file["size_bytes"] = json!(raw.len());
        }
    }
    let hostile_bytes = serde_json::to_vec_pretty(&hostile).expect("hostile bytes");
    fs::write(&release_path, &hostile_bytes).expect("hostile file");
    write_index(&sha256_hex(&hostile_bytes), hostile_bytes.len());

    let refused = post(
        "/api/platform/hub/install",
        body,
        cookie,
        consumer_app.clone(),
    )
    .await;
    let refused = response_json(refused).await;
    assert_eq!(
        refused["error"]["code"],
        json!("HUB_REMOTE_INSTALL_REFUSED"),
        "the one review refuses a static repository's package too: {refused}"
    );

    files_server.abort();
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::Digest as _;
    let mut hasher = sha2::Sha256::new();
    hasher.update(bytes);
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
