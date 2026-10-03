//! The server tests read a connection URL from `ZEBFLOW_TEST_PG_URL` (a
//! throwaway database the test may create and drop tables in) and skip when
//! it is unset.

use std::str::FromStr;

use serde_json::json;

use super::*;

fn server() -> Option<PgConnectOptions> {
    let Ok(url) = std::env::var("ZEBFLOW_TEST_PG_URL") else {
        eprintln!("ZEBFLOW_TEST_PG_URL is not set; skipping");
        return None;
    };
    Some(PgConnectOptions::from_str(&url).expect("a postgres URL"))
}

#[test]
fn the_signature_is_the_shared_query_shape() {
    assert_eq!(
        crate::pipeline::nodes::node_signature(&definition()),
        "postgres.query.run --credential TEXT [--query TEXT] [--param KEY=EXPR…] [--write] [--limit N] → query"
    );
}

#[test]
fn named_and_gapped_params_are_refused() {
    let named = query::params(&json!({ "email": "a@example.com" }), false, PARAM_CODE).unwrap_err();
    assert_eq!(named.code, PARAM_CODE);
    let gap = query::params(&json!({ "1": 1, "3": 3 }), false, PARAM_CODE).unwrap_err();
    assert_eq!(gap.code, PARAM_CODE);
}

/// Without `--write` the transaction is READ ONLY and Postgres refuses the
/// write itself (25006); with it the write commits and is counted. A number
/// binds as a number, a literal as text.
#[tokio::test]
async fn a_write_needs_write_and_params_keep_their_type() {
    let Some(options) = server() else { return };
    let table = format!("zf_query_run_{}", uuid::Uuid::new_v4().simple());
    run(options.clone(), &format!("CREATE TABLE {table} (id INT PRIMARY KEY, email TEXT)"), &[], true, 10)
        .await
        .expect("create");
    let insert = format!("INSERT INTO {table} (id, email) VALUES ($1, $2)");
    let disguised = format!("WITH n AS (SELECT 9 AS id) INSERT INTO {table} (id) SELECT id FROM n");
    for (sql, params) in [(insert.as_str(), vec![json!(7), json!("a@example.com")]), (disguised.as_str(), vec![])] {
        let err = run(options.clone(), sql, &params, false, 10).await.unwrap_err();
        assert_eq!(err.code, WRITE_CODE, "{sql}: {}", err.message);
        assert!(err.message.contains("add --write"), "{}", err.message);
    }
    let written = run(options.clone(), &format!("{insert} RETURNING id"), &[json!(7), json!("a@example.com")], true, 10)
        .await
        .expect("insert");
    assert_eq!(written["query"]["rows"], json!([{ "id": 7 }]));
    assert_eq!(written["query"]["rows_affected"], 1);

    let read = run(options.clone(), &format!("SELECT email, pg_typeof($1::int)::text AS t FROM {table} WHERE id = $1"), &[json!(7)], false, 10)
        .await
        .expect("read");
    assert_eq!(read["query"], json!({ "rows": [{ "email": "a@example.com", "t": "integer" }], "columns": ["email", "t"], "row_count": 1, "truncated": false }));
    let typed = run(options.clone(), "SELECT pg_typeof($1)::text AS a, pg_typeof($2)::text AS b", &[json!(7), json!("7")], false, 10)
        .await
        .expect("typed");
    assert_eq!(typed["query"]["rows"][0], json!({ "a": "bigint", "b": "text" }));

    let many = run(options.clone(), "SELECT generate_series(1, 5) AS n", &[], false, 2).await.expect("limit");
    assert_eq!((many["query"]["row_count"].as_u64(), many["query"]["truncated"].as_bool()), (Some(2), Some(true)));

    run(options, &format!("DROP TABLE {table}"), &[], true, 10).await.expect("drop");
}
