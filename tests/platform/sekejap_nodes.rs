//! `sekejap.record.create` and `sekejap.query.run` through real pipelines on a
//! real platform router — not the node structs alone. A sekejap connection is
//! created through the connections API, its tables through that connection's
//! query API, and everything else through registered, activated pipelines:
//! records in, rows out (filters, `--param`, an empty result), the one-key
//! answers of `node-conventions.md` §6 (`record`, `query`), a failure routed
//! to `:error` with the family's code, and a webhook route answering the rows
//! through `web.response.send`.
//!
//! Every sekejap connection of a project reaches the same project store
//! (`db/drivers/sekejap` resolves the store from owner and project, never from
//! the connection), so the nodes — which take no connection — and the
//! connection's query API must see the same rows. One test proves that.

use std::fs;

use axum::body::{Body, to_bytes};
use axum::http::{Request, StatusCode, header};
use serde_json::{Value, json};
use tower::ServiceExt;

use zebflow::pipeline::nodes::basic::sekejap::{query, record};
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

fn api(path: &str) -> String {
    format!("/api/projects/{OWNER}/{PROJECT}{path}")
}

async fn dsl(app: &axum::Router, cookie: &str, line: &str) -> Value {
    send(app, cookie, "POST", &api("/pipelines/dsl"), json!({ "dsl": line })).await.1
}

/// Registers `pipelines/tests/{name}` from a pipe-mode or graph-mode body and
/// activates it; panics with the answer if either is refused.
async fn publish(app: &axum::Router, cookie: &str, name: &str, body: &str) {
    // Graph mode starts with `[id]` on its own line; pipe mode follows `--`.
    let register = if body.trim_start().starts_with('[') {
        format!("register pipelines/tests/{name}\n{}", body.trim())
    } else {
        format!("register pipelines/tests/{name} -- {body}")
    };
    for line in [register, format!("activate pipeline pipelines/tests/{name}.zf.json")] {
        let answer = dsl(app, cookie, &line).await;
        assert_eq!(answer["ok"], json!(true), "{line}: {answer}");
    }
}

/// Runs a `trigger.manual` pipeline once with `input` (read as `input.manual.*`).
async fn execute(app: &axum::Router, cookie: &str, name: &str, input: Value) -> (StatusCode, Value) {
    send(
        app,
        cookie,
        "POST",
        &api("/pipelines/execute"),
        json!({ "file_rel_path": format!("pipelines/tests/{name}.zf.json"), "trigger": "manual", "input": input }),
    )
    .await
}

/// Creates a sekejap connection through the connections API and answers its
/// `connection_id` (the query API is addressed by id, deletion by slug).
async fn sekejap_connection(app: &axum::Router, cookie: &str, slug: &str) -> String {
    let (status, body) = send(
        app,
        cookie,
        "POST",
        &api("/db/connections"),
        json!({ "connection_slug": slug, "connection_label": "Demo store", "database_kind": "sekejap", "config": {} }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{body}");
    assert_eq!(body["connection"]["database_kind"], json!("sekejap"), "{body}");
    body["connection"]["connection_id"].as_str().expect("connection_id").to_string()
}

/// One statement through the connection's query API.
async fn connection_sql(app: &axum::Router, cookie: &str, connection_id: &str, sql: &str, write: bool) -> Value {
    let (status, body) = send(
        app,
        cookie,
        "POST",
        &api(&format!("/db/connections/{connection_id}/query")),
        json!({ "sql": sql, "read_only": !write }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{sql}: {body}");
    body["result"].clone()
}

/// The fixture: a connection, an `albums` table, and the pipelines below.
async fn albums(app: &axum::Router, cookie: &str) -> String {
    let connection = sekejap_connection(app, cookie, "demo-store").await;
    connection_sql(
        app,
        cookie,
        &connection,
        "CREATE TABLE albums (_key TEXT PRIMARY KEY, title TEXT NOT NULL, genre TEXT, year INT)",
        true,
    )
    .await;
    publish(app, cookie, "albums-seed", r#"| trigger.manual | sekejap.record.create --table albums --record "{{ input.manual.items }}""#).await;
    publish(
        app,
        cookie,
        "albums-find",
        r#"| trigger.manual | sekejap.query.run --param "1={{ input.manual.genre }}" --param "2={{ input.manual.min_year }}" -- "SELECT _key, title, year FROM albums WHERE genre = $1 AND year >= $2 ORDER BY year""#,
    )
    .await;
    connection
}

fn seed_items() -> Value {
    json!([
        { "key": "a1", "fields": { "title": "First Light", "genre": "jazz", "year": 1999 } },
        { "key": "a2", "fields": { "title": "Second Wind", "genre": "jazz", "year": 2004 } },
        { "key": "a3", "fields": { "title": "Third Coast", "genre": "folk", "year": 2010 } }
    ])
}

fn keys_of(payload: &Value) -> Vec<String> {
    let mut keys: Vec<String> = payload.as_object().expect("payload object").keys().cloned().collect();
    keys.sort();
    keys
}

/// `record` and `query` are each one key beside the payload they received;
/// `--param` binds by position with its JSON type; a filter that matches
/// nothing answers an empty list, not a failure.
#[tokio::test]
async fn records_go_in_and_rows_come_out_under_one_key_each() {
    let root = temp_dir("sekejap-nodes-answers");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    albums(&app, &cookie).await;

    let (status, seeded) = execute(&app, &cookie, "albums-seed", json!({ "items": seed_items() })).await;
    assert_eq!(status, StatusCode::OK, "{seeded}");
    let output = &seeded["output"];
    assert_eq!(output["record"], json!({ "created": 3, "edges": 0, "table": "albums" }), "{output}");
    // The run's value keeps the manual trigger's envelope under `manual`.
    assert_eq!(keys_of(output), vec!["manual", "record"], "one key added, the rest kept: {output}");

    let (status, found) = execute(&app, &cookie, "albums-find", json!({ "genre": "jazz", "min_year": 2000 })).await;
    assert_eq!(status, StatusCode::OK, "{found}");
    let answer = &found["output"]["query"];
    assert_eq!(answer["rows"], json!([{ "_key": "a2", "title": "Second Wind", "year": 2004 }]), "{answer}");
    assert_eq!(answer["columns"], json!(["_key", "title", "year"]), "{answer}");
    assert_eq!(answer["row_count"], json!(1));
    assert_eq!(answer["truncated"], json!(false));
    assert!(answer.get("rows_affected").is_none(), "a read has no rows_affected: {answer}");
    assert_eq!(keys_of(&found["output"]), vec!["manual", "query"]);

    // A number bound as a number: every jazz album from year 0 on.
    let (_, all_jazz) = execute(&app, &cookie, "albums-find", json!({ "genre": "jazz", "min_year": 0 })).await;
    let titles: Vec<&str> = all_jazz["output"]["query"]["rows"].as_array().expect("rows").iter().filter_map(|r| r["title"].as_str()).collect();
    assert_eq!(titles, vec!["First Light", "Second Wind"], "{all_jazz}");

    // Nothing matches: an empty list, row_count 0, still under `query`.
    let (status, none) = execute(&app, &cookie, "albums-find", json!({ "genre": "polka", "min_year": 0 })).await;
    assert_eq!(status, StatusCode::OK, "{none}");
    assert_eq!(none["output"]["query"]["rows"], json!([]), "{none}");
    assert_eq!(none["output"]["query"]["row_count"], json!(0));
}

/// The nodes take no connection, and the Studio's DB pages read through a
/// connection: both must be the one project store.
#[tokio::test]
async fn rows_a_node_wrote_are_what_the_connection_reads() {
    let root = temp_dir("sekejap-nodes-connection");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    let connection = albums(&app, &cookie).await;
    let (status, seeded) = execute(&app, &cookie, "albums-seed", json!({ "items": seed_items() })).await;
    assert_eq!(status, StatusCode::OK, "{seeded}");

    let result = connection_sql(&app, &cookie, &connection, "SELECT count(*) AS n FROM albums", false).await;
    // The query API answers positionally: columns + value arrays.
    assert_eq!(result["rows"], json!([[3]]), "{result}");

    // The built-in connection sees the same rows.
    let builtin = zebflow::platform::sekejap::BUILTIN_CONNECTION_SLUG;
    let (status, listed) = send(&app, &cookie, "GET", &api("/db/connections"), json!(null)).await;
    assert_eq!(status, StatusCode::OK, "{listed}");
    let builtin_id = listed["items"]
        .as_array()
        .expect("connection list")
        .iter()
        .find(|item| item["connection_slug"] == json!(builtin))
        .and_then(|item| item["connection_id"].as_str())
        .expect("the built-in sekejap connection is listed")
        .to_string();
    let via_builtin = connection_sql(&app, &cookie, &builtin_id, "SELECT count(*) AS n FROM albums", false).await;
    assert_eq!(via_builtin["rows"], json!([[3]]), "{via_builtin}");

    // Removing the extra connection removes the address, never the rows.
    let (status, body) = send(&app, &cookie, "DELETE", &api("/db/connections/demo-store"), json!(null)).await;
    assert_eq!(status, StatusCode::NO_CONTENT, "{body}");
    let after = connection_sql(&app, &cookie, &builtin_id, "SELECT count(*) AS n FROM albums", false).await;
    assert_eq!(after["rows"], json!([[3]]), "{after}");
}

/// A malformed statement routed to `:error` arrives as the payload kept plus
/// `query: { ok: false, error: { code, message } }` with the family's code
/// (`node-conventions.md` §6); unrouted, the
/// run fails with that code. A write without `--write`, and a record whose
/// key is taken, fail with their own family codes.
#[tokio::test]
async fn failures_carry_the_family_code() {
    let root = temp_dir("sekejap-nodes-errors");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    albums(&app, &cookie).await;

    publish(
        &app,
        &cookie,
        "albums-broken",
        r#"
[a] trigger.manual
[q] sekejap.query.run -- "SELECT FROM WHERE albums"
[e] javascript.script.run -- "return { ok: input.query.ok, code: input.query.error.code, message: input.query.error.message, kept: Object.keys(input).filter(k => !k.startsWith('__')).sort() }"
[a] -> [q]
[q]:error -> [e]
"#,
    )
    .await;
    let (status, routed) = execute(&app, &cookie, "albums-broken", json!({})).await;
    assert_eq!(status, StatusCode::OK, "a routed failure is a finished run: {routed}");
    let script = &routed["output"]["script"];
    assert_eq!(script["code"], json!(query::CODE), "{routed}");
    assert_eq!(script["ok"], json!(false), "{routed}");
    assert!(!script["message"].as_str().unwrap_or_default().is_empty(), "{routed}");
    // The payload the failing node received is kept, plus its own key.
    assert_eq!(script["kept"], json!(["manual", "query"]), "{routed}");

    publish(&app, &cookie, "albums-broken-unrouted", r#"| trigger.manual | sekejap.query.run -- "SELECT FROM WHERE albums""#).await;
    let (status, failed) = execute(&app, &cookie, "albums-broken-unrouted", json!({})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{failed}");
    // The execute API answers the node's own code, not a wrapper.
    assert_eq!(failed["error"]["code"], json!(query::CODE), "{failed}");

    publish(&app, &cookie, "albums-write-unflagged", r#"| trigger.manual | sekejap.query.run -- "DELETE FROM albums""#).await;
    let (status, refused) = execute(&app, &cookie, "albums-write-unflagged", json!({})).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{refused}");
    assert_eq!(refused["error"]["code"], json!(query::WRITE_CODE), "{refused}");

    // A plain insert never overwrites: the second seed of the same keys fails.
    let (status, _) = execute(&app, &cookie, "albums-seed", json!({ "items": seed_items() })).await;
    assert_eq!(status, StatusCode::OK);
    let (status, again) = execute(&app, &cookie, "albums-seed", json!({ "items": seed_items() })).await;
    assert_eq!(status, StatusCode::BAD_REQUEST, "{again}");
    assert_eq!(again["error"]["code"], json!(record::CODE), "{again}");
}

/// What the next apps do most: a public route answering rows as JSON.
#[tokio::test]
async fn a_webhook_route_answers_the_rows() {
    let root = temp_dir("sekejap-nodes-webhook");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    albums(&app, &cookie).await;
    let (status, seeded) = execute(&app, &cookie, "albums-seed", json!({ "items": seed_items() })).await;
    assert_eq!(status, StatusCode::OK, "{seeded}");
    publish(
        &app,
        &cookie,
        "albums-route",
        r#"| trigger.webhook --route /albums --method GET | sekejap.query.run --param "1={{ input.webhook.query.genre }}" -- "SELECT _key, title FROM albums WHERE genre = $1 ORDER BY _key" | web.response.send --body "{{ { albums: input.query.rows, count: input.query.row_count } }}""#,
    )
    .await;

    let get = |uri: String| {
        let app = app.clone();
        async move { json_of(app.oneshot(Request::builder().uri(uri).body(Body::empty()).expect("request")).await.expect("response")).await }
    };
    let (status, jazz) = get(format!("/wh/{OWNER}/{PROJECT}/albums?genre=jazz")).await;
    assert_eq!(status, StatusCode::OK, "{jazz}");
    assert_eq!(
        jazz,
        json!({ "albums": [{ "_key": "a1", "title": "First Light" }, { "_key": "a2", "title": "Second Wind" }], "count": 2 })
    );
    let (status, none) = get(format!("/wh/{OWNER}/{PROJECT}/albums?genre=polka")).await;
    assert_eq!(status, StatusCode::OK, "{none}");
    assert_eq!(none, json!({ "albums": [], "count": 0 }));
}

/// `--edge` writes into an edge table in the same commit as the records, and
/// a graph walk through `sekejap.query.run` reads it back — the shape the
/// relations graph page draws.
#[tokio::test]
async fn edges_written_with_records_are_walked_by_a_query() {
    let root = temp_dir("sekejap-nodes-edges");
    let app = app_at(&root).await;
    let cookie = login(&app).await;
    let connection = sekejap_connection(&app, &cookie, "demo-graph").await;
    for sql in [
        "CREATE TABLE people (_key TEXT PRIMARY KEY, name TEXT)",
        "CREATE TABLE knows (src TEXT REFERENCES people, dst TEXT REFERENCES people, since INT, PRIMARY KEY (src, dst))",
        "CREATE PROPERTY GRAPH social VERTEX TABLES (people) EDGE TABLES (knows SOURCE KEY (src) REFERENCES people (_key) DESTINATION KEY (dst) REFERENCES people (_key))",
    ] {
        connection_sql(&app, &cookie, &connection, sql, true).await;
    }
    publish(&app, &cookie, "people-seed", r#"| trigger.manual | sekejap.record.create --table people --record "{{ input.manual.people }}" --edge "{{ input.manual.edges }}""#).await;
    publish(
        &app,
        &cookie,
        "people-walk",
        r#"| trigger.manual | sekejap.query.run --param "1={{ input.manual.from }}" -- "SELECT name, since FROM GRAPH_TABLE (social MATCH (a WHERE a._key = $1)-[e:knows]->(b) RETURN b.name AS name, e.since AS since)""#,
    )
    .await;

    let (status, seeded) = execute(
        &app,
        &cookie,
        "people-seed",
        json!({
            "people": [{ "key": "p1", "fields": { "name": "Ann" } }, { "key": "p2", "fields": { "name": "Bo" } }],
            "edges": [{ "from": { "target": "people", "key": "p1" }, "type": "knows", "to": { "target": "people", "key": "p2" }, "fields": { "since": 2020 } }]
        }),
    )
    .await;
    assert_eq!(status, StatusCode::OK, "{seeded}");
    assert_eq!(seeded["output"]["record"], json!({ "created": 2, "edges": 1, "table": "people" }), "{seeded}");

    let (status, walked) = execute(&app, &cookie, "people-walk", json!({ "from": "p1" })).await;
    assert_eq!(status, StatusCode::OK, "{walked}");
    assert_eq!(walked["output"]["query"]["rows"], json!([{ "name": "Bo", "since": 2020 }]), "{walked}");
}
