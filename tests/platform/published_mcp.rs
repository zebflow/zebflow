//! `docs/contracts/published-mcp.md` and `addressing.md` §2: a project's dev
//! MCP and its published MCP servers share no tool, list, session or lookup.
//! Each `trigger.mcp --route` is its own server on the `mcp` surface, off by
//! default, behind the route's `--auth`; the dev MCP answers on the platform
//! address only. The door reads only the app's credentials — never a Studio
//! cookie, a platform login or a dev MCP token — answers one uniform 401,
//! 403 for a role, 429 after too many refusals, and records each refusal
//! without the key. Also: what a served site and a file host never answer.

use std::fs;

use axum::body::{Body, Bytes, to_bytes};
use axum::http::{HeaderMap, Request, StatusCode, header};
use serde_json::{Value, json};
use tower::ServiceExt;

use zebflow::platform::{PlatformConfig, build_router};

pub(super) const OWNER: &str = "superadmin";
pub(super) const PROJECT: &str = "default";
const SHOP_KEY: &str = "test-shop-key";
const SIGNING_SECRET: &str = "test-signing-secret-of-thirty-two-bytes";

pub(super) struct TestDir(pub(super) std::path::PathBuf);

impl Drop for TestDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub(super) fn temp_dir(name: &str) -> TestDir {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos();
    TestDir(std::env::temp_dir().join(format!("zebflow-platform-{name}-{now}")))
}

pub(super) async fn app_at(root: &TestDir) -> axum::Router {
    let mut config = PlatformConfig::default();
    config.data_root = root.0.clone();
    config.default_password = "test-pass".to_string();
    build_router(config).await.expect("platform router")
}

pub(super) async fn login(app: &axum::Router) -> String {
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

pub(super) async fn json_of(response: axum::response::Response) -> (StatusCode, Value) {
    let status = response.status();
    let bytes = to_bytes(response.into_body(), usize::MAX).await.expect("body");
    (status, serde_json::from_slice(&bytes).unwrap_or_else(|_| Value::String(String::from_utf8_lossy(&bytes).to_string())))
}

pub(super) async fn send(app: &axum::Router, cookie: &str, method: &str, uri: &str, body: Value) -> (StatusCode, Value) {
    let request = Request::builder()
        .uri(uri)
        .method(method)
        .header(header::COOKIE, cookie)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .expect("request");
    json_of(app.clone().oneshot(request).await.expect("response")).await
}

pub(super) async fn dsl(app: &axum::Router, cookie: &str, dsl: &str) -> Value {
    send(app, cookie, "POST", &format!("/api/projects/{OWNER}/{PROJECT}/pipelines/dsl"), json!({ "dsl": dsl })).await.1
}

/// Registers and activates one pipeline; panics with the answer if either is refused.
pub(super) async fn publish(app: &axum::Router, cookie: &str, name: &str, body: &str) {
    for line in [format!("register pipelines/tests/{name} -- {body}"), format!("activate pipeline pipelines/tests/{name}.zf.json")] {
        let answer = dsl(app, cookie, &line).await;
        assert_eq!(answer["ok"], json!(true), "{line}: {answer}");
    }
}

pub(super) async fn addressing(app: &axum::Router, cookie: &str, data: Value) {
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
    // The dev MCP may answer as an event stream; a published server answers JSON.
    let json_text = text.lines().find_map(|l| l.strip_prefix("data:")).map(str::trim).unwrap_or(text.trim());
    (status, serde_json::from_str(json_text).unwrap_or(Value::String(text.clone())))
}

/// The whole answer — status, headers and bytes — to compare refusals exactly.
async fn raw(app: &axum::Router, uri: &str, headers: &[(&str, &str)], message: Value) -> (StatusCode, HeaderMap, Bytes) {
    let mut builder = Request::builder()
        .uri(uri)
        .method("POST")
        .header(header::CONTENT_TYPE, "application/json")
        .header(header::ACCEPT, "application/json, text/event-stream");
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    let response = app.clone().oneshot(builder.body(Body::from(message.to_string())).expect("request")).await.expect("response");
    let (parts, body) = response.into_parts();
    (parts.status, parts.headers, to_bytes(body, usize::MAX).await.expect("body"))
}

pub(super) fn initialize() -> Value {
    json!({ "jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {
        "protocolVersion": "2025-06-18", "capabilities": {}, "clientInfo": { "name": "test-client", "version": "1" } } })
}

pub(super) fn list() -> Value {
    json!({ "jsonrpc": "2.0", "id": 2, "method": "tools/list" })
}

pub(super) fn call(name: &str, arguments: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": { "name": name, "arguments": arguments } })
}

pub(super) fn tool_names(answer: &Value) -> Vec<String> {
    answer["result"]["tools"].as_array().map(|t| t.iter().filter_map(|t| t["name"].as_str().map(str::to_string)).collect()).unwrap_or_default()
}

pub(super) fn text_of(answer: &Value) -> String {
    answer["result"]["content"][0]["text"].as_str().unwrap_or_default().to_string()
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

/// A third server, `/staff`, behind `--auth jwt --role staff`. The signing
/// credential names an app cookie, which a published route still never reads.
async fn staff_server(app: &axum::Router, cookie: &str) {
    let (status, body) = send(
        app,
        cookie,
        "POST",
        &format!("/api/projects/{OWNER}/{PROJECT}/credentials"),
        json!({ "credential_id": "staff-signing", "title": "Staff tokens", "kind": "jwt_signing_key", "notes": "",
            "secret": { "algorithm": "HS256", "secret": SIGNING_SECRET, "cookie_name": "site_member" } }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    publish(app, cookie, "staff-report", r#"| trigger.mcp --route /staff --name report --description "Today's report." --auth jwt --credential staff-signing --role staff | web.response.send --body "{{ { report: 'quiet day' } }}""#).await;
}

/// A token the app would sign (`auth.token.create`), with these roles, expiring in `ttl` seconds.
fn app_token(roles: &[&str], ttl: i64, secret: &str) -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
    let claims = json!({ "sub": "agent-1", "roles": roles, "iat": now, "exp": now + ttl });
    jsonwebtoken::encode(&jsonwebtoken::Header::default(), &claims, &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes())).expect("token")
}

/// The dev MCP token of a fresh session, as a bearer header.
async fn dev_bearer(app: &axum::Router, cookie: &str) -> String {
    let (status, session) = send(app, cookie, "POST", &format!("/api/projects/{OWNER}/{PROJECT}/mcp/session"), json!({ "capabilities": ["pipelines.read", "pipelines.write"] })).await;
    assert_eq!(status, StatusCode::OK, "{session}");
    format!("Bearer {}", session["session"]["token"].as_str().expect("token"))
}

/// Two routes are two servers: each lists and calls only its own tools; `none`
/// admits everyone; a call answers the response body, a 4xx is a tool error.
#[tokio::test]
async fn two_routes_are_two_servers() {
    let root = temp_dir("published-mcp-two-routes");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    two_servers(&app, &cookie).await;
    let shop = format!("/mcp/{OWNER}/{PROJECT}/shop");
    let library = format!("/mcp/{OWNER}/{PROJECT}/library");
    let key = [("x-api-key", SHOP_KEY)];

    let (status, init) = rpc(&app, &shop, None, &key, initialize()).await;
    assert_eq!(status, StatusCode::OK, "{init}");
    assert!(init["result"]["capabilities"]["tools"].is_object(), "{init}");
    assert!(init["result"]["capabilities"]["prompts"].is_null() && init["result"]["capabilities"]["resources"].is_null(), "tools only: {init}");

    let (_, listed) = rpc(&app, &shop, None, &key, list()).await;
    assert_eq!(tool_names(&listed), vec!["search", "stock"], "{listed}");
    assert_eq!(listed["result"]["tools"][0]["inputSchema"]["required"], json!(["q"]));
    let (status, listed) = rpc(&app, &library, None, &[], list()).await;
    assert_eq!(status, StatusCode::OK, "`--auth none` admits everyone");
    assert_eq!(tool_names(&listed), vec!["lookup"], "{listed}");
    // Nothing of the project rides along: no prompts, no resources.
    for method in ["prompts/list", "resources/list"] {
        let (_, answer) = rpc(&app, &library, None, &[], json!({ "jsonrpc": "2.0", "id": 9, "method": method })).await;
        let empty = answer["result"].as_object().map(|r| r.values().all(|v| v.as_array().is_some_and(|a| a.is_empty()) || !v.is_array())).unwrap_or(true);
        assert!(empty, "{method}: {answer}");
    }

    let (_, called) = rpc(&app, &shop, None, &key, call("search", json!({ "q": "mug" }))).await;
    assert_eq!(called["result"]["isError"], json!(false), "{called}");
    assert_eq!(serde_json::from_str::<Value>(&text_of(&called)).expect("json"), json!({ "found": "mug", "route": "/shop" }));
    // `web.response.send --status 404` is a tool error with that body.
    let (_, called) = rpc(&app, &shop, None, &key, call("stock", json!({ "sku": "MUG-01" }))).await;
    assert_eq!(called["result"]["isError"], json!(true), "{called}");
    assert_eq!(serde_json::from_str::<Value>(&text_of(&called)).expect("json"), json!({ "error": "no such sku" }));
    // A missing argument is refused before the run.
    let (_, called) = rpc(&app, &shop, None, &key, call("search", json!({}))).await;
    assert_eq!(called["result"]["isError"], json!(true), "{called}");
    assert!(text_of(&called).contains("missing_required_arg"), "{called}");
    // Without a response node, an empty result: the run's value (the
    // arguments, what the script made) is never sent on its own.
    let (_, called) = rpc(&app, &library, None, &[], call("lookup", json!({ "topic": "maps" }))).await;
    assert_eq!(called["result"], json!({ "content": [], "isError": false }), "{called}");
    assert!(!called.to_string().contains("Atlas of maps"), "{called}");

    // Routes share nothing: one server's tool is unknown on the other.
    let (_, called) = rpc(&app, &library, None, &[], call("search", json!({ "q": "mug" }))).await;
    assert!(called["error"].is_object(), "{called}");
    // A route nothing publishes is no server.
    let (status, _) = rpc(&app, &format!("/mcp/{OWNER}/{PROJECT}/nothing-here"), None, &[], list()).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
}

/// `api_key` and `jwt` at the door: missing, wrong and expired are one 401 —
/// the same status, headers and bytes — the right one works, a wrong role is 403.
#[tokio::test]
async fn the_door_answers_one_uniform_401_and_a_403_for_a_role() {
    let root = temp_dir("published-mcp-door");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    two_servers(&app, &cookie).await;
    staff_server(&app, &cookie).await;
    let shop = format!("/mcp/{OWNER}/{PROJECT}/shop");
    let staff = format!("/mcp/{OWNER}/{PROJECT}/staff");

    let missing = raw(&app, &shop, &[], list()).await;
    assert_eq!(missing.0, StatusCode::UNAUTHORIZED);
    for wrong in [&[("x-api-key", "wrong-key")][..], &[("authorization", "ApiKey wrong-key")][..], &[("x-api-key", "")][..]] {
        assert_eq!(raw(&app, &shop, wrong, list()).await, missing, "{wrong:?} answers exactly as a missing key");
    }
    let api_key_header = format!("ApiKey {SHOP_KEY}");
    for right in [&[("x-api-key", SHOP_KEY)][..], &[("authorization", api_key_header.as_str())][..]] {
        let (status, listed) = rpc(&app, &shop, None, right, list()).await;
        assert_eq!((status, tool_names(&listed)), (StatusCode::OK, vec!["search".to_string(), "stock".to_string()]), "{right:?}");
    }

    let staff_missing = raw(&app, &staff, &[], list()).await;
    assert_eq!(staff_missing.0, StatusCode::UNAUTHORIZED);
    assert_eq!((&staff_missing.1, &staff_missing.2), (&missing.1, &missing.2), "one 401 on every route");
    let expired = format!("Bearer {}", app_token(&["staff"], -3600, SIGNING_SECRET));
    let forged = format!("Bearer {}", app_token(&["staff"], 3600, "another-secret-of-thirty-two-bytes!!"));
    for wrong in [expired.as_str(), forged.as_str(), "Bearer not-a-token"] {
        assert_eq!(raw(&app, &staff, &[("authorization", wrong)], list()).await, staff_missing, "{wrong}");
    }
    let good = format!("Bearer {}", app_token(&["staff"], 3600, SIGNING_SECRET));
    let (status, called) = rpc(&app, &staff, None, &[("authorization", &good)], call("report", json!({}))).await;
    assert_eq!(status, StatusCode::OK, "{called}");
    assert_eq!(serde_json::from_str::<Value>(&text_of(&called)).expect("json"), json!({ "report": "quiet day" }));
    let guest = format!("Bearer {}", app_token(&["guest"], 3600, SIGNING_SECRET));
    let (status, refused) = rpc(&app, &staff, None, &[("authorization", &guest)], list()).await;
    assert_eq!(status, StatusCode::FORBIDDEN, "{refused}");
    assert_eq!(refused["error"]["code"], json!("forbidden"));
    // A valid app token in the app's own cookie is still a cookie: never read.
    let in_cookie = format!("site_member={}", app_token(&["staff"], 3600, SIGNING_SECRET));
    assert_eq!(raw(&app, &staff, &[("cookie", &in_cookie)], list()).await, staff_missing);
}

/// Zebflow's own auth is never a key to a published route: a Studio session
/// cookie, a platform login and a dev MCP token are each refused 401.
#[tokio::test]
async fn zebflows_own_auth_never_opens_a_published_route() {
    let root = temp_dir("published-mcp-not-zebflow-auth");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    two_servers(&app, &cookie).await;
    staff_server(&app, &cookie).await;
    let bearer = dev_bearer(&app, &cookie).await;
    let dev_token = bearer.trim_start_matches("Bearer ").to_string();
    use base64::Engine as _;
    let basic = format!("Basic {}", base64::engine::general_purpose::STANDARD.encode("superadmin:test-pass"));
    let shop = format!("/mcp/{OWNER}/{PROJECT}/shop");
    let staff = format!("/mcp/{OWNER}/{PROJECT}/staff");
    let refusal = raw(&app, &shop, &[], list()).await;

    for route in [&shop, &staff] {
        for (what, headers) in [
            ("a Studio session cookie", vec![("cookie", cookie.as_str())]),
            ("a platform login", vec![("authorization", basic.as_str())]),
            ("a dev MCP token as a bearer", vec![("authorization", bearer.as_str())]),
            ("a dev MCP token as a key", vec![("x-api-key", dev_token.as_str())]),
            ("a dev MCP session header", vec![("x-zebflow-mcp-session", dev_token.as_str())]),
            ("all of them at once", vec![("cookie", cookie.as_str()), ("authorization", bearer.as_str())]),
        ] {
            let answer = raw(&app, route, &headers, call("search", json!({ "q": "mug" }))).await;
            assert_eq!(answer, refusal, "{what} on {route}");
        }
    }
    // The same cookie still opens the Studio: it was refused as a key, not broken.
    let (status, _) = send(&app, &cookie, "GET", &format!("/api/projects/{OWNER}/{PROJECT}/pipelines"), Value::Null).await;
    assert_eq!(status, StatusCode::OK);
}

/// Twenty refusals a minute per client address and route; the twenty-first
/// attempt is 429, even with the right key. Another address, or another
/// route, counts on its own. Each refusal is recorded without the key.
#[tokio::test]
async fn refusals_are_limited_per_client_and_recorded_without_the_key() {
    let root = temp_dir("published-mcp-limit");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    two_servers(&app, &cookie).await;
    staff_server(&app, &cookie).await;
    let shop = format!("/mcp/{OWNER}/{PROJECT}/shop");
    let staff = format!("/mcp/{OWNER}/{PROJECT}/staff");
    let guessed = "guessed-key-must-not-be-recorded";
    let from = |address: &'static str| ("x-forwarded-for", address);

    for attempt in 1..=20 {
        let (status, _) = rpc(&app, &shop, None, &[from("203.0.113.50"), ("x-api-key", guessed)], call("search", json!({ "q": "x" }))).await;
        assert_eq!(status, StatusCode::UNAUTHORIZED, "attempt {attempt}");
    }
    let (status, headers, body) = raw(&app, &shop, &[from("203.0.113.50"), ("x-api-key", SHOP_KEY)], list()).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS, "{}", String::from_utf8_lossy(&body));
    let wait: u64 = headers.get(header::RETRY_AFTER).and_then(|v| v.to_str().ok()).and_then(|v| v.parse().ok()).expect("retry-after");
    assert!((1..=60).contains(&wait), "{wait}");
    let (status, _) = rpc(&app, &shop, None, &[from("203.0.113.51"), ("x-api-key", SHOP_KEY)], list()).await;
    assert_eq!(status, StatusCode::OK, "another address is another count");
    let good = format!("Bearer {}", app_token(&["staff"], 3600, SIGNING_SECRET));
    let (status, _) = rpc(&app, &staff, None, &[from("203.0.113.50"), ("authorization", &good)], list()).await;
    assert_eq!(status, StatusCode::OK, "another route is another count");
    // An open route never counts anything.
    let (status, _) = rpc(&app, &format!("/mcp/{OWNER}/{PROJECT}/library"), None, &[from("203.0.113.50")], list()).await;
    assert_eq!(status, StatusCode::OK);

    let (status, log) = send(&app, &cookie, "GET", &format!("/api/projects/{OWNER}/{PROJECT}/pipelines/invocations?pipeline=pipelines/tests/shop-search.zf.json"), Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{log}");
    let (status, groups) = send(&app, &cookie, "GET", &format!("/api/projects/{OWNER}/{PROJECT}/pipelines/errors?file_rel_path=pipelines/tests/shop-search.zf.json"), Value::Null).await;
    assert_eq!(status, StatusCode::OK, "{groups}");
    let code = zebflow::pipeline::nodes::basic::trigger::mcp_trigger::AUTH_CODE;
    for (what, record) in [("the run log", log.to_string()), ("the error groups", groups.to_string())] {
        assert!(record.contains("refused at the door of MCP route /shop"), "{what}: {record}");
        assert!(record.contains(code), "{what} names the code: {record}");
        assert!(!record.contains(guessed), "{what} holds the presented key: {record}");
        assert!(!record.contains(SHOP_KEY), "{what} holds the stored key");
    }
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

/// The `mcp` surface: `/_mcp/ROUTE` on a project host, `/mcp/{o}/{p}/ROUTE`
/// on the platform, both off until switched on, never under `/wh/…`, never
/// the dev MCP. The dev MCP answers on the platform address only, whatever
/// `api_on_hosts` says.
#[tokio::test]
async fn a_published_server_answers_only_on_the_mcp_surface_and_only_when_it_is_on() {
    let root = temp_dir("published-mcp-surface");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    two_servers(&app, &cookie).await;
    let dev_host = "default.superadmin.localhost";
    let platform_form = format!("/mcp/{OWNER}/{PROJECT}/library");

    let (status, listed) = rpc(&app, "/_mcp/library", Some(dev_host), &[], list()).await;
    assert_eq!((status, tool_names(&listed)), (StatusCode::OK, vec!["lookup".to_string()]), "{listed}");
    let (status, listed) = rpc(&app, &platform_form, None, &[], list()).await;
    assert_eq!((status, tool_names(&listed)), (StatusCode::OK, vec!["lookup".to_string()]), "{listed}");
    // Never a webhook path, on either form; never the surface's bare root.
    for (host, uri) in [
        (None, format!("/wh/{OWNER}/{PROJECT}/library")),
        (Some(dev_host), "/library".to_string()),
        (Some(dev_host), "/_mcp".to_string()),
        (Some(dev_host), "/_mcp/nothing-here".to_string()),
    ] {
        let (status, answer) = rpc(&app, &uri, host, &[], list()).await;
        assert_eq!(status, StatusCode::NOT_FOUND, "{host:?}{uri}: {answer}");
        assert!(tool_names(&answer).is_empty(), "{uri} listed tools: {answer}");
    }

    // Switched off (the default): 404 on both forms.
    addressing(&app, &cookie, json!({ "hosts": [], "routes": [], "disabled": ["files", "ms", "mcp"] })).await;
    let (status, answer) = rpc(&app, "/_mcp/library", Some(dev_host), &[], list()).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "off on the project host: {answer}");
    let (status, answer) = rpc(&app, &platform_form, None, &[], list()).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "off on the platform form too: {answer}");
    assert!(tool_names(&answer).is_empty());
    let (status, _) = rpc(&app, &format!("/mcp/{OWNER}/{PROJECT}/shop"), None, &[("x-api-key", SHOP_KEY)], list()).await;
    assert_eq!(status, StatusCode::NOT_FOUND, "a right key does not switch it on");

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

/// An MCP client's round trip over a real socket, served as the binary
/// serves (with the peer address): initialize, the `initialized`
/// notification, tools/list and tools/call, on two routes of one project.
#[tokio::test]
async fn an_mcp_client_round_trip_over_http_on_two_routes() {
    let root = temp_dir("published-mcp-round-trip");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    two_servers(&app, &cookie).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let address = listener.local_addr().expect("address");
    let server = tokio::spawn(async move {
        axum::serve(listener, app.into_make_service_with_connect_info::<std::net::SocketAddr>()).await
    });
    let client = reqwest::Client::new();

    for (route, headers, tools, tool, arguments, expect) in [
        ("shop", vec![("x-api-key", SHOP_KEY)], vec!["search", "stock"], "search", json!({ "q": "lamp" }), json!({ "found": "lamp", "route": "/shop" })),
        // `lookup` ends without `web.response.send`: an empty result.
        ("library", vec![], vec!["lookup"], "lookup", json!({ "topic": "rivers" }), Value::Null),
    ] {
        let url = format!("http://{address}/mcp/{OWNER}/{PROJECT}/{route}");
        let post = |message: Value, version: Option<String>| {
            let mut request = client.post(&url).header("content-type", "application/json").header("accept", "application/json, text/event-stream");
            for (name, value) in &headers {
                request = request.header(*name, *value);
            }
            if let Some(version) = version {
                request = request.header("mcp-protocol-version", version);
            }
            request.body(message.to_string()).send()
        };
        let init: Value = post(initialize(), None).await.expect("initialize").json().await.expect("json");
        let version = init["result"]["protocolVersion"].as_str().expect("negotiated version").to_string();
        let notified = post(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }), Some(version.clone())).await.expect("notify");
        assert_eq!(notified.status().as_u16(), 202, "a notification is accepted");
        let listed: Value = post(list(), Some(version.clone())).await.expect("list").json().await.expect("json");
        assert_eq!(tool_names(&listed), tools, "{listed}");
        let called: Value = post(call(tool, arguments), Some(version)).await.expect("call").json().await.expect("json");
        if expect.is_null() {
            assert_eq!(called["result"], json!({ "content": [], "isError": false }), "{called}");
            continue;
        }
        let result: Value = serde_json::from_str(&text_of(&called)).expect("result json");
        assert_eq!(result, expect, "{called}");
    }
    // Over the socket, a refusal is the same 401 whatever was presented.
    let shop = format!("http://{address}/mcp/{OWNER}/{PROJECT}/shop");
    let mut seen = Vec::new();
    for key in [None, Some("wrong-key")] {
        let mut request = client.post(&shop).header("content-type", "application/json").header("accept", "application/json, text/event-stream");
        if let Some(key) = key {
            request = request.header("x-api-key", key);
        }
        let response = request.body(list().to_string()).send().await.expect("refused");
        let status = response.status().as_u16();
        let mut headers: Vec<(String, String)> = response
            .headers()
            .iter()
            .filter(|(name, _)| name.as_str() != "date")
            .map(|(name, value)| (name.to_string(), value.to_str().unwrap_or_default().to_string()))
            .collect();
        headers.sort();
        seen.push((status, headers, response.bytes().await.expect("body")));
    }
    assert_eq!(seen[0].0, 401);
    assert_eq!(seen[0], seen[1]);
    server.abort();
}
