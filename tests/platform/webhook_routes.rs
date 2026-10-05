//! Webhook routes through a real router: a `GET` route answers `HEAD` with
//! the same headers and no body; a quoted DSL value keeps an escaped quote
//! through register, the stored pipeline JSON and the DSL written back from
//! it; and a `trigger.webhook` without `--method` is a GET route both when it
//! is saved and when it is called.

use std::fs;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use tower::ServiceExt;

use zebflow::platform::{PlatformConfig, build_router};

const OWNER: &str = "superadmin";
const PROJECT: &str = "default";

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

async fn studio(app: &axum::Router, cookie: &str, method: &str, path: &str, body: Option<Value>) -> (StatusCode, Value) {
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
    (status, serde_json::from_slice(&bytes).unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).to_string())))
}

async fn dsl(app: &axum::Router, cookie: &str, line: &str) -> Value {
    studio(app, cookie, "POST", "/pipelines/dsl", Some(json!({ "dsl": line }))).await.1
}

async fn publish(app: &axum::Router, cookie: &str, name: &str, body: &str) {
    for line in [format!("register pipelines/tests/{name} -- {body}"), format!("activate pipeline pipelines/tests/{name}.zf.json")] {
        let answer = dsl(app, cookie, &line).await;
        assert_eq!(answer["ok"], json!(true), "{line}: {answer}");
    }
}

/// A visitor's call: status, headers, body.
async fn visit(app: &axum::Router, method: &str, path: &str) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let response = app
        .clone()
        .oneshot(Request::builder().uri(format!("/wh/{OWNER}/{PROJECT}{path}")).method(method).body(Body::empty()).expect("request"))
        .await
        .expect("response");
    let status = response.status();
    let headers = response.headers().clone();
    (status, headers, to_bytes(response.into_body(), usize::MAX).await.expect("body").to_vec())
}

/// The stored pipeline JSON, through the read API.
async fn stored_graph(app: &axum::Router, cookie: &str, name: &str) -> Value {
    let (status, body) = studio(app, cookie, "GET", &format!("/pipelines/by-id?id=pipelines/tests/{name}.zf.json"), None).await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let source = body["pipeline"]["source"].as_str().or_else(|| body["source"].as_str()).unwrap_or_default().to_string();
    serde_json::from_str(&source).unwrap_or(body)
}

fn node_of<'a>(graph: &'a Value, kind: &str) -> &'a Value {
    let nodes = graph["spec"]["nodes"].as_array().or_else(|| graph["nodes"].as_array()).expect("nodes");
    nodes.iter().find(|n| n["kind"] == kind).unwrap_or_else(|| panic!("no {kind} in {graph}"))
}

#[tokio::test]
async fn a_get_route_answers_head_with_its_headers_and_no_body() {
    let root = temp_dir("webhook-head");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    publish(&app, &cookie, "hello", r#"| trigger.webhook --route /hello --method GET | web.response.send --header "X-Greeting=hi" --body "{{ { hello: 'world' } }}""#).await;
    publish(&app, &cookie, "posted", r#"| trigger.webhook --route /posted --method POST | web.response.send --body ok"#).await;

    let (status, get_headers, body) = visit(&app, "GET", "/hello").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(serde_json::from_slice::<Value>(&body).expect("json"), json!({ "hello": "world" }));
    let (status, head_headers, head_body) = visit(&app, "HEAD", "/hello").await;
    assert_eq!(status, StatusCode::OK, "a GET route answers HEAD");
    assert!(head_body.is_empty(), "no body");
    for name in ["content-type", "x-greeting"] {
        assert_eq!(head_headers.get(name), get_headers.get(name), "{name}");
    }
    assert_eq!(head_headers.get("content-length").and_then(|v| v.to_str().ok()), Some(body.len().to_string().as_str()));
    assert!(head_headers.get("x-request-id").is_some());
    // HEAD is GET's; a POST route does not answer it.
    assert_eq!(visit(&app, "HEAD", "/posted").await.0, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn an_escaped_quote_survives_register_the_json_and_the_dsl_written_back() {
    let root = temp_dir("webhook-escaped-quote");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    let wanted = r#"attachment; filename="a b.pdf""#;
    publish(
        &app,
        &cookie,
        "download",
        r#"| trigger.webhook --route /download --method GET | web.response.send --header "Content-Disposition=attachment; filename=\"a b.pdf\"" --body ok"#,
    )
    .await;
    let graph = stored_graph(&app, &cookie, "download").await;
    assert_eq!(node_of(&graph, "web.response.send")["config"]["headers"]["Content-Disposition"], json!(wanted), "{graph}");
    let (status, headers, _) = visit(&app, "GET", "/download").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers.get("content-disposition").and_then(|v| v.to_str().ok()), Some(wanted));

    // The DSL the platform writes back from that JSON registers the same pipeline.
    let described = dsl(&app, &cookie, "describe pipeline pipelines/tests/download.zf.json").await;
    let body: Vec<String> = described["lines"]
        .as_array()
        .expect("lines")
        .iter()
        .filter_map(|line| line["text"].as_str())
        .map(str::trim)
        .filter(|text| text.starts_with("| "))
        .map(str::to_string)
        .collect();
    assert_eq!(body.len(), 2, "{described}");
    let copy = body.join(" ").replace("/download", "/download-copy");
    assert!(copy.contains(r#"filename=\"a b.pdf\""#), "written back escaped: {copy}");
    publish(&app, &cookie, "download-copy", &copy).await;
    let copied = stored_graph(&app, &cookie, "download-copy").await;
    assert_eq!(node_of(&copied, "web.response.send")["config"]["headers"], node_of(&graph, "web.response.send")["config"]["headers"]);
    let (_, headers, _) = visit(&app, "GET", "/download-copy").await;
    assert_eq!(headers.get("content-disposition").and_then(|v| v.to_str().ok()), Some(wanted));
}

#[tokio::test]
async fn a_webhook_without_a_method_is_get_when_saved_and_when_called() {
    let root = temp_dir("webhook-default-method");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    // Saved and activated: the save check accepts it as a GET route.
    publish(&app, &cookie, "plain", r#"| trigger.webhook --route /plain | web.response.send --body ok"#).await;
    let (status, _, body) = visit(&app, "GET", "/plain").await;
    assert_eq!((status, body.as_slice()), (StatusCode::OK, b"ok".as_slice()), "the runtime serves it as GET");
    assert_eq!(visit(&app, "POST", "/plain").await.0, StatusCode::NOT_FOUND, "and only as GET");
    // The Studio's run of the webhook trigger picks the same route without being told the method.
    let (status, run) = studio(
        &app,
        &cookie,
        "POST",
        "/pipelines/execute",
        Some(json!({ "file_rel_path": "pipelines/tests/plain.zf.json", "trigger": "webhook", "webhook_path": "/plain" })),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{run}");
}
