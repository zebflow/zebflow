use std::collections::BTreeSet;
use std::time::Instant;

use async_trait::async_trait;
use serde_json::{Value, json};

use crate::platform::db::driver::{DbDriver, DbDriverContext};
use crate::platform::error::PlatformError;
use crate::platform::model::{
    DbObjectNode, DbQueryColumn, DescribeProjectDbConnectionRequest,
    ProjectDbConnectionDescribeResult, ProjectDbConnectionQueryResult,
    QueryProjectDbConnectionRequest, SimpleTableQueryRequest, slug_segment,
};

#[derive(Default)]
pub struct SjtableDbDriver;

#[async_trait]
impl DbDriver for SjtableDbDriver {
    fn kind(&self) -> &'static str {
        "sjtable"
    }

    async fn describe(
        &self,
        ctx: &DbDriverContext,
        req: &DescribeProjectDbConnectionRequest,
    ) -> Result<ProjectDbConnectionDescribeResult, PlatformError> {
        let scope = normalize_scope(req.scope.as_deref());
        let mut tables = ctx.simple_tables.list_tables(&ctx.owner, &ctx.project)?;
        tables.sort_by(|a, b| a.table.cmp(&b.table));

        let table_nodes = tables
            .iter()
            .map(|table| DbObjectNode {
                kind: "table".to_string(),
                name: table.table.clone(),
                schema: Some("default".to_string()),
                children: Vec::new(),
                meta: json!({
                    "row_count": table.row_count,
                    "collection": table.collection,
                    "hash_indexed_fields": table.hash_indexed_fields,
                    "range_indexed_fields": table.range_indexed_fields,
                }),
            })
            .collect::<Vec<_>>();

        let nodes = match scope.as_str() {
            "tables" => table_nodes,
            "schemas" => vec![DbObjectNode {
                kind: "schema".to_string(),
                name: "default".to_string(),
                schema: None,
                children: Vec::new(),
                meta: json!({"table_count": table_nodes.len()}),
            }],
            "functions" => Vec::new(),
            _ => vec![DbObjectNode {
                kind: "schema".to_string(),
                name: "default".to_string(),
                schema: None,
                children: table_nodes,
                meta: json!({"table_count": tables.len()}),
            }],
        };

        Ok(ProjectDbConnectionDescribeResult {
            connection_id: ctx.connection.connection_id.clone(),
            connection_slug: ctx.connection.connection_slug.clone(),
            database_kind: ctx.connection.database_kind.clone(),
            scope,
            nodes,
        })
    }

    async fn query(
        &self,
        ctx: &DbDriverContext,
        req: &QueryProjectDbConnectionRequest,
    ) -> Result<ProjectDbConnectionQueryResult, PlatformError> {
        let started = Instant::now();
        let sql = req.sql.trim();

        if looks_like_json_query(sql) {
            let payload: Value = serde_json::from_str(sql).map_err(|err| {
                PlatformError::new("PLATFORM_DB_QUERY_INVALID", format!("invalid JSON query: {err}"))
            })?;
            let payload = normalize_native_query_payload(payload)?;
            let rows = ctx
                .simple_tables
                .query_native_rows(&ctx.owner, &ctx.project, &payload)?;
            return Ok(rows_to_result(ctx, rows, started.elapsed().as_millis() as u64));
        }

        let table = resolve_target_table(req);
        if table.is_empty() {
            return Err(PlatformError::new(
                "PLATFORM_DB_QUERY_INVALID",
                "sjtable query requires request.table, SQL SELECT ... FROM <table>, or a JSON query payload",
            ));
        }

        let limit = req.limit.unwrap_or(120).clamp(1, 500);
        let result = ctx.simple_tables.query_rows(
            &ctx.owner,
            &ctx.project,
            &SimpleTableQueryRequest {
                table,
                where_field: None,
                where_value: None,
                limit,
            },
        )?;

        let mut columns = BTreeSet::<String>::new();
        for row in &result.rows {
            if let Some(obj) = row.as_object() {
                for key in obj.keys() {
                    columns.insert(key.to_string());
                }
            }
        }
        let ordered_columns = columns.into_iter().collect::<Vec<_>>();
        let rows = result
            .rows
            .iter()
            .map(|row| {
                ordered_columns
                    .iter()
                    .map(|column| row.get(column).cloned().unwrap_or(Value::Null))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        Ok(ProjectDbConnectionQueryResult {
            connection_id: ctx.connection.connection_id.clone(),
            connection_slug: ctx.connection.connection_slug.clone(),
            database_kind: ctx.connection.database_kind.clone(),
            columns: ordered_columns
                .iter()
                .map(|name| DbQueryColumn {
                    name: name.clone(),
                    data_type: None,
                })
                .collect(),
            row_count: rows.len(),
            rows,
            truncated: false,
            affected_rows: None,
            duration_ms: started.elapsed().as_millis() as u64,
        })
    }
}

fn normalize_scope(raw: Option<&str>) -> String {
    let normalized = raw
        .map(slug_segment)
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "tree".to_string());
    match normalized.as_str() {
        "tree" | "schemas" | "tables" | "functions" => normalized,
        _ => "tree".to_string(),
    }
}

fn resolve_target_table(req: &QueryProjectDbConnectionRequest) -> String {
    if let Some(table) = req.table.as_deref() {
        let normalized = slug_segment(table);
        if !normalized.is_empty() {
            return normalized;
        }
    }
    parse_table_from_sql(&req.sql)
}

fn parse_table_from_sql(sql: &str) -> String {
    let tokens = sql
        .split_whitespace()
        .map(|token| token.trim().trim_matches(|ch: char| ch == ';' || ch == ','))
        .collect::<Vec<_>>();
    for (idx, token) in tokens.iter().enumerate() {
        if token.eq_ignore_ascii_case("from") {
            if let Some(next) = tokens.get(idx + 1) {
                let raw = next
                    .split('.')
                    .next_back()
                    .unwrap_or(next)
                    .trim_matches('"');
                let out = slug_segment(raw);
                if !out.is_empty() {
                    return out;
                }
            }
        }
    }
    String::new()
}

fn looks_like_json_query(raw: &str) -> bool {
    let trimmed = raw.trim_start();
    trimmed.starts_with('{') || trimmed.starts_with('[')
}

fn normalize_native_query_payload(payload: Value) -> Result<Value, PlatformError> {
    match payload {
        Value::Object(map) => Ok(Value::Object(map)),
        Value::Array(items) => Ok(json!({ "pipeline": items })),
        _ => Err(PlatformError::new(
            "PLATFORM_DB_QUERY_INVALID",
            "sjtable JSON query must be object or array",
        )),
    }
}

fn rows_to_result(
    ctx: &DbDriverContext,
    raw_rows: Vec<Value>,
    duration_ms: u64,
) -> ProjectDbConnectionQueryResult {
    let mut columns = BTreeSet::<String>::new();
    for row in &raw_rows {
        if let Some(obj) = row.as_object() {
            for key in obj.keys() {
                columns.insert(key.to_string());
            }
        } else {
            columns.insert("value".to_string());
        }
    }
    let ordered_columns = columns.into_iter().collect::<Vec<_>>();
    let rows = raw_rows
        .iter()
        .map(|row| {
            if let Some(obj) = row.as_object() {
                ordered_columns
                    .iter()
                    .map(|column| obj.get(column).cloned().unwrap_or(Value::Null))
                    .collect::<Vec<_>>()
            } else {
                ordered_columns
                    .iter()
                    .map(|column| {
                        if column == "value" {
                            row.clone()
                        } else {
                            Value::Null
                        }
                    })
                    .collect::<Vec<_>>()
            }
        })
        .collect::<Vec<_>>();

    ProjectDbConnectionQueryResult {
        connection_id: ctx.connection.connection_id.clone(),
        connection_slug: ctx.connection.connection_slug.clone(),
        database_kind: ctx.connection.database_kind.clone(),
        columns: ordered_columns
            .iter()
            .map(|name| DbQueryColumn {
                name: name.clone(),
                data_type: None,
            })
            .collect(),
        row_count: rows.len(),
        rows,
        truncated: false,
        affected_rows: None,
        duration_ms,
    }
}
