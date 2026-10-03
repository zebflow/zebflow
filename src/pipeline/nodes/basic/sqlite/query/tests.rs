use serde_json::{Value, json};

use super::*;

async fn exec(root: &Path, config: Value, payload: Value) -> Result<Value, PipelineError> {
    let config: Config = serde_json::from_value(config).expect("config");
    let node = Node::new(config, root.to_path_buf())?;
    let out = node
        .execute_async(NodeExecutionInput {
            node_id: "n0".to_string(),
            input_pin: "in".to_string(),
            payload,
            metadata: json!({ "owner": "demo", "project": "demo", "pipeline": "test", "request_id": "r1" }),
            bus: None,
        })
        .await?;
    Ok(out.payload)
}

async fn with_table() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("tempdir");
    exec(tmp.path(), json!({ "query": "CREATE TABLE users (id INTEGER PRIMARY KEY, email TEXT UNIQUE NOT NULL, age INTEGER)", "write": true }), json!({}))
        .await
        .expect("create table");
    tmp
}

#[test]
fn the_signature_is_the_shared_query_shape() {
    assert_eq!(
        crate::pipeline::nodes::node_signature(&definition()),
        "sqlite.query.run [--query TEXT] [--param KEY=EXPR…] [--write] [--limit N] → query"
    );
}

/// Without `--write` the connection is read-only, so SQLite refuses the
/// write itself — also one whose first word is not INSERT.
#[tokio::test]
async fn a_write_without_write_is_refused_by_the_engine() {
    let tmp = with_table().await;
    for sql in [
        "INSERT INTO users (email) VALUES ('a@example.com')",
        "WITH n AS (SELECT 'b@example.com' AS e) INSERT INTO users (email) SELECT e FROM n",
        "/* note */ DELETE FROM users",
        "CREATE TABLE other (id INTEGER)",
    ] {
        let err = exec(tmp.path(), json!({ "query": sql }), json!({})).await.unwrap_err();
        assert_eq!(err.code, WRITE_CODE, "{sql}: {}", err.message);
        assert!(err.message.contains("add --write"), "{}", err.message);
    }
    let count = exec(tmp.path(), json!({ "query": "SELECT count(*) AS n FROM users" }), json!({})).await.unwrap();
    assert_eq!(count["query"]["rows"][0]["n"], 0, "nothing was written");
}

/// No statement may attach another file, with or without `--write`: an
/// attached database would be any file the process can open.
#[tokio::test]
async fn attach_is_refused_even_with_write() {
    let tmp = with_table().await;
    let other = tmp.path().join("other.db");
    for write in [false, true] {
        let sql = format!("ATTACH DATABASE '{}' AS other", other.display());
        let err = exec(tmp.path(), json!({ "query": sql, "write": write }), json!({})).await.unwrap_err();
        assert!(err.message.contains("too many attached databases"), "write={write}: {}", err.message);
    }
    assert!(!other.exists(), "nothing was created outside the project database");
}

/// The merged write: `--write` runs the INSERT, answers the rows RETURNING
/// gives back and how many rows changed.
#[tokio::test]
async fn write_inserts_and_answers_rows_affected() {
    let tmp = with_table().await;
    let out = exec(
        tmp.path(),
        json!({ "query": "INSERT INTO users (email, age) VALUES (?1, ?2) RETURNING id", "param": { "1": "a@example.com", "2": 41 }, "write": true }),
        json!({}),
    )
    .await
    .unwrap();
    assert_eq!(out["query"]["rows"], json!([{ "id": 1 }]));
    assert_eq!(out["query"]["rows_affected"], 1);
    let plain = exec(tmp.path(), json!({ "query": "UPDATE users SET age = age + 1", "write": true }), json!({})).await.unwrap();
    assert_eq!(plain["query"], json!({ "rows": [], "columns": [], "row_count": 0, "truncated": false, "rows_affected": 1 }));
}

/// A whole `{{ }}` arrives as its JSON type and binds as that type; a
/// literal arrives as text.
#[tokio::test]
async fn a_param_binds_with_its_json_type() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let out = exec(
        tmp.path(),
        json!({ "query": "SELECT typeof(?1) AS a, ?1 + 1 AS next, typeof(?2) AS b, typeof(?3) AS c", "param": { "1": 7, "2": "7", "3": null } }),
        json!({}),
    )
    .await
    .unwrap();
    assert_eq!(out["query"]["rows"][0], json!({ "a": "integer", "next": 8, "b": "text", "c": "null" }));
    let named = exec(
        tmp.path(),
        json!({ "query": "SELECT :who AS who, @n * 2 AS twice", "param": { "who": "Ana", "n": 21 } }),
        json!({}),
    )
    .await
    .unwrap();
    assert_eq!(named["query"]["rows"][0], json!({ "who": "Ana", "twice": 42 }));
}

#[tokio::test]
async fn mixed_gapped_or_miscounted_params_are_refused() {
    let tmp = tempfile::tempdir().expect("tempdir");
    for (sql, param) in [
        ("SELECT ?1, ?3", json!({ "1": 1, "3": 3 })),
        ("SELECT ?1, :name", json!({ "1": 1, "name": "x" })),
        ("SELECT ?1, ?2", json!({ "1": 1 })),
        ("SELECT :name", json!({ "other": 1 })),
        ("SELECT ?1", json!([1])),
    ] {
        let err = exec(tmp.path(), json!({ "query": sql, "param": param }), json!({})).await.unwrap_err();
        assert_eq!(err.code, PARAM_CODE, "{sql}: {}", err.message);
    }
}

/// One key, `query`, beside the payload it was given; nothing at the root.
#[tokio::test]
async fn the_answer_is_one_query_key_and_the_payload_stays() {
    let tmp = with_table().await;
    exec(tmp.path(), json!({ "query": "INSERT INTO users (email) VALUES ('a@example.com'), ('b@example.com'), ('c@example.com')", "write": true }), json!({}))
        .await
        .unwrap();
    let input = json!({ "body": { "page": 1 }, "rows": "kept" });
    let out = exec(tmp.path(), json!({ "query": "SELECT email FROM users ORDER BY email", "limit": 2 }), input).await.unwrap();
    let mut keys: Vec<&str> = out.as_object().unwrap().keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(keys, vec!["body", "query", "rows"]);
    assert_eq!(out["rows"], "kept");
    assert_eq!(
        out["query"],
        json!({ "rows": [{ "email": "a@example.com" }, { "email": "b@example.com" }], "columns": ["email"], "row_count": 2, "truncated": true })
    );
    let err = exec(tmp.path(), json!({ "query": "SELECT 1", "limit": 5001 }), json!({})).await.unwrap_err();
    assert_eq!(err.code, LIMIT_CODE);
}
