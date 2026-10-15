//! The Studio's OAuth2 credential callback: whatever it answers — the
//! provider's denial, a missing or forged state, a finished exchange — it
//! clears the single-use nonce cookie, so an abandoned attempt leaves nothing
//! in the browser for a later callback to ride on.

use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use tower::ServiceExt;

use zebflow::platform::{PlatformConfig, build_router};

struct TestDir(std::path::PathBuf);

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[tokio::test]
async fn every_callback_answer_clears_the_nonce_cookie() {
    let root = TestDir(std::env::temp_dir().join(format!(
        "zebflow-platform-studio-oauth-{}",
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
    )));
    let mut config = PlatformConfig::default();
    config.data_root = root.0.clone();
    config.default_password = uuid::Uuid::new_v4().to_string();
    let app = build_router(config).await.expect("platform router");
    let nonce = uuid::Uuid::new_v4().simple().to_string();
    let cookie = format!("zebflow_oauth_nonce={nonce}");

    for (uri, lands) in [
        ("/oauth/callback?error=access_denied", "/home?oauth=denied"),
        ("/oauth/callback?code=c1", "/home?oauth=error"),
        ("/oauth/callback?state=forged.state&code=c1", "/home?oauth=error"),
    ] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(uri).header(header::COOKIE, &cookie).body(Body::empty()).expect("request"))
            .await
            .expect("response");
        assert_eq!(response.status(), StatusCode::SEE_OTHER, "{uri}");
        assert_eq!(response.headers().get(header::LOCATION).and_then(|v| v.to_str().ok()), Some(lands), "{uri}");
        let cleared = response
            .headers()
            .get_all(header::SET_COOKIE)
            .iter()
            .filter_map(|v| v.to_str().ok())
            .any(|v| v.starts_with("zebflow_oauth_nonce=;") && v.contains("Max-Age=0") && v.contains("Path=/oauth/callback"));
        assert!(cleared, "{uri} must clear the nonce cookie");
    }
}
