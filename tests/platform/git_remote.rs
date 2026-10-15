//! The project's git remote, through the routes the Studio calls:
//! `PUT /git/remote` (connect, disconnect) and `POST /git/commit` with `push`.

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use tower::ServiceExt;

use zebflow::platform::{PlatformConfig, build_router};

const BASE: &str = "/api/projects/superadmin/default";

async fn login_cookie(app: axum::Router) -> String {
    let login = app
        .oneshot(
            Request::builder()
                .uri("/login")
                .method("POST")
                .header(header::CONTENT_TYPE, "application/x-www-form-urlencoded")
                .body(Body::from("identifier=superadmin&password=test-pass"))
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

async fn call(
    app: &axum::Router,
    cookie: &str,
    method: &str,
    uri: &str,
    content_type: &str,
    body: String,
) -> (StatusCode, Value) {
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .uri(uri)
                .method(method)
                .header(header::COOKIE, cookie)
                .header(header::CONTENT_TYPE, content_type)
                .body(Body::from(body))
                .expect("request"),
        )
        .await
        .expect("response");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("body");
    let text = String::from_utf8_lossy(&bytes).to_string();
    (status, serde_json::from_str(&text).unwrap_or(Value::String(text)))
}

async fn json_call(app: &axum::Router, cookie: &str, method: &str, path: &str, body: Value) -> (StatusCode, Value) {
    call(app, cookie, method, &format!("{BASE}{path}"), "application/json", body.to_string()).await
}

/// A platform on a fresh data root, signed in, plus a bare repository outside
/// that root to push to. The `TempDir`s live as long as the test holds them.
async fn platform_with_bare_remote() -> (tempfile::TempDir, tempfile::TempDir, axum::Router, String, String) {
    let data = tempfile::tempdir().expect("data root");
    let remotes = tempfile::tempdir().expect("remote root");
    let mut config = PlatformConfig::default();
    config.data_root = data.path().join("platform");
    config.default_password = "test-pass".to_string();
    let app = build_router(config).await.expect("platform router");
    let cookie = login_cookie(app.clone()).await;
    // The commit carries the signed-in user's identity, not a fallback address.
    let (status, body) = call(
        &app,
        &cookie,
        "PUT",
        "/api/profile",
        "application/json",
        json!({"git_name": "Studio", "git_email": "studio@example.test"}).to_string(),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let bare = remotes.path().join("remote.git");
    let init = std::process::Command::new("git")
        .args(["init", "-q", "--bare", "--initial-branch=main"])
        .arg(&bare)
        .output()
        .expect("git runs");
    assert!(init.status.success(), "{}", String::from_utf8_lossy(&init.stderr));
    let remote_url = format!("file://{}", bare.display());
    (data, remotes, app, cookie, remote_url)
}

async fn saved_remote(app: &axum::Router, cookie: &str) -> Value {
    let (status, body) = json_call(app, cookie, "GET", "/git/remote", Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    body
}

/// Disconnecting sends an empty URL, which means "no remote" — not an invalid
/// one. A commit-and-push that names only a branch keeps the saved remote and
/// pushes to it.
#[tokio::test]
async fn disconnecting_a_remote_clears_it_and_a_push_naming_only_a_branch_keeps_it() {
    let (_data, _remotes, app, cookie, remote_url) = platform_with_bare_remote().await;

    let (status, body) =
        json_call(&app, &cookie, "PUT", "/git/remote", json!({"credential_id": "", "repo_url": remote_url, "branch": "main"})).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(saved_remote(&app, &cookie).await["repo_url"], json!(remote_url));

    let (status, body) = call(&app, &cookie, "PUT", &format!("{BASE}/repo/file?path=docs/notes.md"), "text/plain", "one\n".into()).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = json_call(
        &app,
        &cookie,
        "POST",
        "/git/commit",
        json!({"files": ["docs/notes.md"], "message": "first", "push": true, "branch": "main"}),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "a push naming only a branch keeps the saved remote: {body}");
    assert_eq!(saved_remote(&app, &cookie).await["repo_url"], json!(remote_url));

    // The Studio's disconnect, exactly as the panel sends it.
    let (status, body) =
        json_call(&app, &cookie, "PUT", "/git/remote", json!({"credential_id": "", "repo_url": "", "branch": "main"})).await;
    assert_eq!(status, StatusCode::OK, "disconnect: {body}");
    let saved = saved_remote(&app, &cookie).await;
    assert_eq!(saved["repo_url"], json!(""), "{saved}");
    assert_eq!(saved["credential_id"], json!(""), "{saved}");

    // A URL that is sent is still checked.
    let (status, _) =
        json_call(&app, &cookie, "PUT", "/git/remote", json!({"credential_id": "", "repo_url": "/srv/bare-path", "branch": "main"})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
}

/// A token typed into the remote URL is refused at the route, before it can
/// reach zebflow.yaml (committed and pushed) or .git/config. Credentials
/// belong in a git credential.
#[tokio::test]
async fn a_remote_url_carrying_a_token_is_refused_and_never_stored() {
    let (data, _remotes, app, cookie, remote_url) = platform_with_bare_remote().await;
    let token = uuid::Uuid::new_v4().simple().to_string();
    let with_token = format!("https://ci-bot:{token}@203.0.113.10/org/site-a.git");

    let (status, body) =
        json_call(&app, &cookie, "PUT", "/git/remote", json!({"credential_id": "", "repo_url": with_token, "branch": "main"})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(!body.to_string().contains(&token), "{body}");

    let (status, body) = call(&app, &cookie, "PUT", &format!("{BASE}/repo/file?path=docs/notes.md"), "text/plain", "one\n".into()).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let (status, body) = json_call(
        &app,
        &cookie,
        "POST",
        "/git/commit",
        json!({"files": ["docs/notes.md"], "message": "first", "push": true, "repo_url": with_token, "branch": "main"}),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{body}");
    assert!(!body.to_string().contains(&token), "{body}");

    let repo = data.path().join("platform/users/superadmin/default/repo");
    for file in ["zebflow.yaml", ".git/config"] {
        let text = std::fs::read_to_string(repo.join(file)).unwrap_or_default();
        assert!(!text.contains(&token), "{file} holds the token");
    }
    assert_eq!(saved_remote(&app, &cookie).await["repo_url"], json!(""));

    // The same remote without the token is accepted.
    let (status, body) =
        json_call(&app, &cookie, "PUT", "/git/remote", json!({"credential_id": "", "repo_url": remote_url, "branch": "main"})).await;
    assert_eq!(status, StatusCode::OK, "{body}");
}
