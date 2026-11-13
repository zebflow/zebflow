//! Webhook signatures through a real router: an `hmac` credential describes
//! the whole check, a route names it with `--auth hmac`, and a delivery is
//! admitted only when its signature matches. The old provider shape is
//! refused when saved, a refusal reaches the pipeline's error list as a code,
//! and the Studio's test panel checks a pasted delivery against the saved
//! credential. No secret or signature appears in any answer.

use std::fs;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use hmac::{Hmac, Mac};
use serde_json::{Value, json};
use tower::ServiceExt;

use zebflow::platform::{PlatformConfig, build_router};

const OWNER: &str = "superadmin";
const PROJECT: &str = "default";
const SECRET: &str = "vendor-signing-secret-1";

struct TestDir(std::path::PathBuf);

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn temp_dir(name: &str) -> TestDir {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos();
    TestDir(std::env::temp_dir().join(format!("zebflow-platform-{name}-{now}")))
}

async fn app_at(root: &TestDir) -> axum::Router {
    let mut config = PlatformConfig::default();
    config.data_root = root.0.clone();
    config.default_password = "test-pass".to_string();
    build_router(config).await.expect("platform router")
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

async fn studio(app: &axum::Router, cookie: &str, method: &str, path: &str, body: Option<Value>) -> (StatusCode, String) {
    let mut request = Request::builder().uri(format!("/api/projects/{OWNER}/{PROJECT}{path}")).method(method).header(header::COOKIE, cookie);
    let body = match body {
        Some(json) => {
            request = request.header(header::CONTENT_TYPE, "application/json");
            Body::from(json.to_string())
        }
        None => Body::empty(),
    };
    let response = app.clone().oneshot(request.body(body).expect("request")).await.expect("response");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("body");
    (status, String::from_utf8_lossy(&bytes).to_string())
}

async fn publish(app: &axum::Router, cookie: &str, name: &str, body: &str) {
    for line in [format!("register pipelines/tests/{name} -- {body}"), format!("activate pipeline pipelines/tests/{name}.zf.json")] {
        let (_, answer) = studio(app, cookie, "POST", "/pipelines/dsl", Some(json!({ "dsl": line }))).await;
        assert!(answer.contains("\"ok\":true"), "{line}: {answer}");
    }
}

/// A delivery from the vendor: no Studio cookie, only its own headers.
async fn deliver(app: &axum::Router, headers: &[(&str, String)], body: &str) -> (StatusCode, String) {
    let mut request = Request::builder().uri(format!("/wh/{OWNER}/{PROJECT}/hooks/vendor")).method("POST").header(header::CONTENT_TYPE, "application/json");
    for (name, value) in headers {
        request = request.header(*name, value);
    }
    let response = app.clone().oneshot(request.body(Body::from(body.to_string())).expect("request")).await.expect("response");
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("body");
    (status, String::from_utf8_lossy(&bytes).to_string())
}

fn signature(timestamp: &str, body: &str) -> String {
    let mut mac = <Hmac<sha2::Sha256> as hmac::digest::KeyInit>::new_from_slice(SECRET.as_bytes()).unwrap();
    mac.update(format!("{timestamp}.{body}").as_bytes());
    format!("v1={}", hex::encode(mac.finalize().into_bytes()))
}

fn now() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs() as i64
}

#[tokio::test]
async fn a_vendor_delivery_is_admitted_only_with_a_matching_signature() {
    let root = temp_dir("webhook-signatures");
    let app = app_at(&root).await;
    let cookie = login(&app).await;

    // The shape before 0.11.28 is refused when saved, by name.
    let (status, answer) = studio(
        &app,
        &cookie,
        "POST",
        "/credentials",
        Some(json!({ "credential_id": "old-hooks", "title": "Old", "kind": "hmac", "secret": { "provider": "github", "secret": "x" } })),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{answer}");
    assert!(answer.contains("before 0.11.28"), "{answer}");

    let settings = json!({
        "secrets": { "current": SECRET },
        "signed": "{timestamp}.{body}",
        "signature": { "from": "header:X-Vendor-Signature", "prefix": "v1=" },
        "timestamp": { "from": "header:X-Vendor-Timestamp", "tolerance": 300 }
    });
    let (status, answer) = studio(
        &app,
        &cookie,
        "POST",
        "/credentials",
        Some(json!({ "credential_id": "vendor-hooks", "title": "Vendor", "kind": "hmac", "secret": settings })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let (_, read) = studio(&app, &cookie, "GET", "/credentials/vendor-hooks", None).await;
    assert!(!read.contains(SECRET), "the secret is masked: {read}");
    assert!(read.contains("X-Vendor-Signature") && read.contains("{timestamp}.{body}"), "settings are shown: {read}");

    publish(
        &app,
        &cookie,
        "vendor",
        r#"| trigger.webhook --route /hooks/vendor --method POST --auth hmac --credential vendor-hooks | web.response.send --body accepted"#,
    )
    .await;

    let body = r#"{"event":"login","user":"u_1"}"#;
    let ts = now().to_string();
    let good = signature(&ts, body);
    let (status, answer) = deliver(&app, &[("X-Vendor-Timestamp", ts.clone()), ("X-Vendor-Signature", good.clone())], body).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert_eq!(answer, "accepted");

    // Saved again exactly as the form reads it, masks and all, the secret is
    // kept: the mask is never stored as the secret.
    let masked: Value = serde_json::from_str(&read).expect("json");
    let (status, answer) = studio(
        &app,
        &cookie,
        "PUT",
        "/credentials/vendor-hooks",
        Some(json!({ "credential_id": "vendor-hooks", "title": "Vendor", "kind": "hmac", "secret": masked["credential"]["secret"] })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    let (status, answer) = deliver(&app, &[("X-Vendor-Timestamp", ts.clone()), ("X-Vendor-Signature", good.clone())], body).await;
    assert_eq!((status, answer.as_str()), (StatusCode::OK, "accepted"), "the secret survived a masked save");

    // A changed body, an old timestamp, a missing prefix: each refused with its code.
    let (status, answer) = deliver(&app, &[("X-Vendor-Timestamp", ts.clone()), ("X-Vendor-Signature", good.clone())], r#"{"event":"login","user":"u_2"}"#).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(answer.contains("HMAC_SIGNATURE_MISMATCH"), "{answer}");
    let old = (now() - 3600).to_string();
    let (status, answer) = deliver(&app, &[("X-Vendor-Timestamp", old.clone()), ("X-Vendor-Signature", signature(&old, body))], body).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(answer.contains("HMAC_TIMESTAMP_STALE"), "{answer}");
    let (status, answer) = deliver(&app, &[("X-Vendor-Timestamp", ts.clone()), ("X-Vendor-Signature", good.replacen("v1=", "", 1))], body).await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert!(answer.contains("HMAC_PREFIX_MISSING"), "{answer}");
    assert!(!answer.contains(SECRET) && !answer.contains(&good[3..]), "a refusal carries no secret or signature: {answer}");

    // The refusals show on the pipeline as codes, never as values.
    let (_, hits) = studio(&app, &cookie, "GET", "/pipelines/hits", None).await;
    assert!(hits.contains("HMAC_TIMESTAMP_STALE") && hits.contains("webhook.hmac"), "{hits}");
    assert!(!hits.contains(SECRET) && !hits.contains(&good[3..]), "{hits}");

    // The Studio's test panel: a pasted delivery against the saved check.
    let pasted = format!("POST /hooks/vendor HTTP/1.1\nX-Vendor-Timestamp: {old}\nX-Vendor-Signature: {}\n", signature(&old, body));
    let (status, answer) = studio(&app, &cookie, "POST", "/credentials/vendor-hooks/hmac/check", Some(json!({ "headers": pasted, "body": body }))).await;
    assert_eq!(status, StatusCode::OK, "{answer}");
    assert!(answer.contains("\"passed\":true"), "the time window is ignored for a pasted delivery: {answer}");
    let (_, answer) = studio(
        &app,
        &cookie,
        "POST",
        "/credentials/vendor-hooks/hmac/check",
        Some(json!({ "headers": pasted, "body": body, "ignore_time": false })),
    )
    .await;
    assert!(answer.contains("HMAC_TIMESTAMP_STALE"), "{answer}");

    // The presets are listed for the form, without secrets.
    let (status, presets) = studio(&app, &cookie, "GET", "/credential-types/hmac/presets", None).await;
    assert_eq!(status, StatusCode::OK);
    let presets: Value = serde_json::from_str(&presets).expect("json");
    let ids: Vec<&str> = presets["presets"].as_array().expect("presets").iter().filter_map(|p| p["id"].as_str()).collect();
    for id in ["github", "meta", "stripe", "slack", "standard-webhooks"] {
        assert!(ids.contains(&id), "{id} in {ids:?}");
    }
}
