//! `docs/contracts/published-mcp.md` and `addressing.md` §2: a project's dev
//! MCP and its published MCP servers share no tool, list, session or lookup.
//! Each `trigger.mcp --route` is its own server on the `mcp` surface, off by
//! default, behind the route's `--auth`; the dev MCP answers on the platform
//! address only. 0.11 seals the declaration and its checks; serving the
//! routes lands in 0.11.1. Also: what a served site and a file host never
//! answer.

use std::fs;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use tower::ServiceExt;

use zebflow::platform::{PlatformConfig, build_router};

const OWNER: &str = "superadmin";
const PROJECT: &str = "default";
const SHOP_KEY: &str = "test-shop-key";

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

async fn json_of(response: axum::response::Response) -> (StatusCode, Value) {
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("body");
    (status, serde_json::from_slice(&bytes).unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).to_string())))
}

async fn send(app: &axum::Router, cookie: &str, method: &str, uri: &str, body: Value) -> (StatusCode, Value) {
    let request = Request::builder()
        .uri(uri)
        .method(method)
        .header(header::COOKIE, cookie)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("request");
    json_of(app.clone().oneshot(request).await.expect("response")).await
}

async fn dsl(app: &axum::Router, cookie: &str, dsl: &str) -> Value {
    send(app, cookie, "POST", &format!("/api/projects/{OWNER}/{PROJECT}/pipelines/dsl"), json!({ "dsl": dsl })).await.1
}

/// Registers and activates one pipeline; panics with the answer if either is refused.
async fn publish(app: &axum::Router, cookie: &str, name: &str, body: &str) {
    for line in [format!("register pipelines/tests/{name} -- {body}"), format!("activate pipeline pipelines/tests/{name}.zf.json")] {
        let answer = dsl(app, cookie, &line).await;
        assert_eq!(answer["ok"], json!(true), "{line}: {answer}");
    }
}

async fn addressing(app: &axum::Router, cookie: &str, data: Value) {
    let (status, body) = send(app, cookie, "PUT", &format!("/api/projects/{OWNER}/{PROJECT}/settings/addressing"), json!({ "data": data })).await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

async fn api_key_credential(app: &axum::Router, cookie: &str) {
    let (status, body) = send(
        app,
        cookie,
        "POST",
        &format!("/api/projects/{OWNER}/{PROJECT}/credentials"),
        json!({ "credential_id": "shop-key", "title": "Shop agents", "kind": "api_key", "notes": "", "secret": { "key": SHOP_KEY } }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
}

/// One JSON-RPC message to an MCP endpoint, as a streamable-HTTP client sends it.
async fn rpc(app: &axum::Router, uri: &str, host: Option<&str>, headers: &[(&str, &str)], message: Value) -> (StatusCode, Value) {
    let mut builder = Request::builder()
        .uri(uri)
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ACCEPT, "application/json, text/event-stream");
    if let Some(host) = host {
        builder = builder.header(header::HOST, host);
    }
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    let response = app.clone().oneshot(builder.body(Body::from(message.to_string())).expect("request")).await.expect("response");
    let status = response.status();
    let text = String::from_utf8_lossy(&to_bytes(response.into_body(), usize::MAX).await.expect("body")).to_string();
    // The dev MCP may answer as an event stream.
    let json_text = text.lines().find_map(|l| l.strip_prefix("data:")).map(str::trim).unwrap_or(text.trim());
    (status, serde_json::from_str(json_text).unwrap_or(Value::String(text.clone())))
}

fn list() -> Value {
    json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" })
}

fn call(name: &str, arguments: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": { "name": name, "arguments": arguments } })
}

fn tool_names(answer: &Value) -> Vec<String> {
    answer["result"]["tools"].as_array().map(|t| t.iter().filter_map(|t| t["name"].as_str().map(str::to_string)).collect()).unwrap_or_default()
}

/// The fixture: a key, the `mcp` surface on, and two routes in one project —
/// `/shop` (api_key, two tools) and `/library` (open, one tool).
async fn two_servers(app: &axum::Router, cookie: &str) {
    api_key_credential(app, cookie).await;
    addressing(app, cookie, json!({ "hosts": [], "routes": [], "disabled": ["files", "ms"] })).await;
    publish(app, cookie, "shop-search", r#"| trigger.mcp --route /shop --name search --description "Find products by words." --parameter q:string! "The words." --auth api_key --credential shop-key | web.response.send --body "{{ { found: input.mcp.arguments.q, route: $trigger.route } }}""#).await;
    publish(app, cookie, "shop-stock", r#"| trigger.mcp --route /shop --name stock --description "Stock for one SKU." --parameter sku:string! --auth api_key --credential shop-key | web.response.send --status 404 --body "{{ { error: 'no such sku' } }}""#).await;
    publish(app, cookie, "library-lookup", r#"| trigger.mcp --route /library --name lookup --description "Look a title up." --auth none | javascript.script.run -- "return { title: 'Atlas of ' + input.mcp.arguments.topic }""#).await;
}

/// The dev MCP never lists, looks up or calls a published tool — before
/// 0.11 it merged every active `trigger.mcp` into its own list.
#[tokio::test]
async fn the_dev_mcp_never_lists_or_calls_a_published_tool() {
    let root = temp_dir("published-mcp-dev-mcp");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    two_servers(&app, &cookie).await;

    let (status, session) = send(&app, &cookie, "POST", &format!("/api/projects/{OWNER}/{PROJECT}/mcp/session"), json!({ "capabilities": ["pipelines.read", "pipelines.write"] })).await;
    assert_eq!(status, StatusCode::OK, "{session}");
    let bearer = format!("Bearer {}", session["session"]["token"].as_str().expect("token"));
    let dev = format!("/api/projects/{OWNER}/{PROJECT}/mcp");
    let (status, listed) = rpc(&app, &dev, None, &[("authorization", &bearer)], list()).await;
    assert_eq!(status, StatusCode::OK, "{listed}");
    let names = tool_names(&listed);
    assert!(names.contains(&"pipeline_register".to_string()), "the dev tools are there: {listed}");
    for published in ["search", "stock", "lookup"] {
        assert!(!names.contains(&published.to_string()), "the dev MCP lists {published}");
        let (_, called) = rpc(&app, &dev, None, &[("authorization", &bearer)], call(published, json!({ "q": "mug", "sku": "x", "topic": "maps" }))).await;
        assert!(called["error"].is_object(), "the dev MCP called {published}: {called}");
        assert!(called.to_string().contains("Unknown tool"), "{called}");
    }
}

/// The `mcp` surface carries published servers only — never under `/wh/…`,
/// never the dev MCP. 0.11 seals the declaration; serving lands in 0.11.1, so
/// the surface answers 404 on both forms even switched on. The dev MCP
/// answers on the platform address only, whatever `api_on_hosts` says.
#[tokio::test]
async fn the_mcp_surface_is_never_the_dev_mcp_and_serves_nothing_yet() {
    let root = temp_dir("published-mcp-surface");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    two_servers(&app, &cookie).await;
    let dev_host = "default.superadmin.localhost";

    for (host, uri) in [
        (Some(dev_host), "/_mcp/library".to_string()),
        (None, format!("/mcp/{OWNER}/{PROJECT}/library")),
        (None, format!("/wh/{OWNER}/{PROJECT}/library")),
        (Some(dev_host), "/library".to_string()),
        (Some(dev_host), "/_mcp".to_string()),
    ] {
        let (status, answer) = rpc(&app, &uri, host, &[], list()).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{host:?}{uri}: {answer}");
        assert!(tool_names(&answer).is_empty(), "{uri} listed tools: {answer}");
    }

    let (status, session) = send(&app, &cookie, "POST", &format!("/api/projects/{OWNER}/{PROJECT}/mcp/session"), json!({ "capabilities": ["pipelines.read"] })).await;
    assert_eq!(status, StatusCode::OK, "{session}");
    let bearer = format!("Bearer {}", session["session"]["token"].as_str().expect("token"));
    let dev = format!("/api/projects/{OWNER}/{PROJECT}/mcp");
    let (status, _) = rpc(&app, &dev, None, &[("authorization", &bearer)], list()).await;
    assert_eq!(status, StatusCode::OK, "the platform address serves the dev MCP");
    for api_on_hosts in [false, true] {
        addressing(&app, &cookie, json!({ "hosts": [], "routes": [], "disabled": ["files", "ms"], "api_on_hosts": api_on_hosts })).await;
        for uri in [dev.clone(), "/_mcp".to_string(), "/_mcp/".to_string()] {
            let (status, answer) = rpc(&app, &uri, Some(dev_host), &[("authorization", &bearer)], list()).await;
            assert_eq!(status, StatusCode::NOT_FOUND, "api_on_hosts {api_on_hosts} {uri}: {answer}");
            assert!(tool_names(&answer).is_empty());
        }
    }
    // The rest of the API still follows `api_on_hosts`.
    let response = app
        .clone()
        .oneshot(Request::builder().uri(format!("/api/projects/{OWNER}/{PROJECT}/pipelines")).header(header::HOST, dev_host).header(header::COOKIE, &cookie).body(Body::empty()).expect("request"))
        .await
        .expect("api");
    assert_eq!(response.status(), StatusCode::OK, "api_on_hosts on serves the rest of the API");
}

#[tokio::test]
async fn activation_refuses_a_second_tool_name_and_mixed_auth_on_one_route() {
    let root = temp_dir("published-mcp-activation");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    api_key_credential(&app, &cookie).await;
    publish(&app, &cookie, "first", r#"| trigger.mcp --route /shop --name search --auth api_key --credential shop-key | web.response.send --body ok"#).await;

    for (name, body, words) in [
        ("same-name", r#"| trigger.mcp --route /shop --name search --auth api_key --credential shop-key | web.response.send --body ok"#, "already publishes a tool named 'search'"),
        ("open-on-shop", r#"| trigger.mcp --route /shop --name browse --auth none | web.response.send --body ok"#, "one server with one guard"),
    ] {
        let registered = dsl(&app, &cookie, &format!("register pipelines/tests/{name} -- {body}")).await;
        assert_eq!(registered["ok"], json!(true), "{registered}");
        let refused = dsl(&app, &cookie, &format!("activate pipeline pipelines/tests/{name}.zf.json")).await;
        let text = refused.to_string();
        assert_ne!(refused["ok"], json!(true), "{name} must be refused: {refused}");
        assert!(text.contains(words), "{name}: {refused}");
        assert!(text.contains("pipelines/tests/first.zf.json") && text.contains(&format!("pipelines/tests/{name}.zf.json")), "names both pipelines: {refused}");
    }
    // The same name on another route is another server's tool.
    publish(&app, &cookie, "other-route", r#"| trigger.mcp --route /library --name search --auth none | web.response.send --body ok"#).await;
    // A missing --auth is refused when saved.
    let refused = dsl(&app, &cookie, r#"register pipelines/tests/no-auth -- | trigger.mcp --route /shop --name quiet | web.response.send --body ok"#).await;
    assert_ne!(refused["ok"], json!(true), "{refused}");
    assert!(refused.to_string().contains("--auth"), "{refused}");
}

/// What a served site and a file host never answer: a path with a segment
/// starting with `.` (the generator's manifest names template paths), except
/// `/.well-known/…`.
#[tokio::test]
async fn a_dot_path_is_never_served_except_well_known() {
    let root = temp_dir("dot-paths");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    let files = root.0.join("users").join(OWNER).join(PROJECT).join("files").join("site");
    fs::create_dir_all(files.join(".well-known")).expect("dirs");
    fs::write(files.join("index.html"), "<h1>home</h1>").expect("index");
    fs::write(files.join(".zebflow-static-site.json"), "{\"pages\":[]}").expect("manifest");
    fs::write(files.join(".well-known").join("security.txt"), "Contact: mailto:security@example.com").expect("well-known");
    let (status, body) = send(
        &app,
        &cookie,
        "PUT",
        &format!("/api/projects/{OWNER}/{PROJECT}/files/access"),
        json!({ "path": "site", "access": "public_execute", "scope": "prefix", "serve": ["http://default.superadmin.localhost"] }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    let get = |host: &'static str, uri: &'static str| {
        let app = app.clone();
        async move {
            app.oneshot(Request::builder().uri(uri).header(header::HOST, host).body(Body::empty()).expect("request"))
                .await
                .expect("response")
                .status()
        }
    };
    let site = "default.superadmin.localhost";
    assert_eq!(get(site, "/").await, StatusCode::OK);
    assert_eq!(get(site, "/.zebflow-static-site.json").await, StatusCode::NOT_FOUND);
    assert_eq!(get(site, "/%2Ezebflow-static-site.json").await, StatusCode::NOT_FOUND);
    assert_eq!(get(site, "/.well-known/security.txt").await, StatusCode::OK);
    let file_host = "default.superadmin.fs.localhost";
    assert_eq!(get(file_host, "/site/index.html").await, StatusCode::OK);
    assert_eq!(get(file_host, "/site/.zebflow-static-site.json").await, StatusCode::NOT_FOUND);
}
