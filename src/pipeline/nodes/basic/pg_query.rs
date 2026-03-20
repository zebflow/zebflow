/// Postgres query node using stored project credentials.
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sqlx::{Column, Row, postgres::PgConnectOptions, postgres::PgRow};

use crate::pipeline::{
    PipelineError, NodeDefinition,
    nodes::{NodeHandler, NodeExecutionInput, NodeExecutionOutput},
};
use crate::language::LanguageEngine;
use crate::platform::services::CredentialService;

use crate::pipeline::model::LayoutItem;
use super::util::{eval_deno_expr, metadata_scope, resolve_array_values};

pub const NODE_KIND: &str = "n.pg.query";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

/// Unified node-definition metadata for `n.pg.query`.
pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Postgres Query".to_string(),
        description: "Execute SQL using project credential and return rows/affected count."
            .to_string(),
        input_schema: serde_json::json!({
            "type":"object",
            "description":"Input context for query/parameter bindings."
        }),
        output_schema: serde_json::json!({
            "oneOf":[
                {"type":"object","properties":{"rows":{"type":"array"}}},
                {"type":"object","properties":{"affected_rows":{"type":"integer"}}}
            ]
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: true,
        script_bridge: Some(crate::pipeline::NodeScriptBridge {
            name: "n.pg.query".to_string(),
            enabled: false,
        }),
        config_schema: Default::default(),
        dsl_flags: Default::default(),
        fields: {
            use crate::pipeline::model::{NodeFieldDef, NodeFieldType, NodeFieldDataSource, SidebarSection, SidebarItem};
            vec![
                NodeFieldDef { name: "title".to_string(), label: "Title".to_string(), field_type: NodeFieldType::Text, help: Some("Override display title for this node.".to_string()), ..Default::default() },
                NodeFieldDef { name: "credential_id".to_string(), label: "Credential".to_string(), field_type: NodeFieldType::Select, data_source: Some(NodeFieldDataSource::CredentialsPostgres), help: Some("Loaded from project credentials filtered by kind=postgres.".to_string()), ..Default::default() },
                NodeFieldDef {
                    name: "query".to_string(),
                    label: "Query".to_string(),
                    field_type: NodeFieldType::CodeEditor,
                    language: Some("sql".to_string()),
                    span: Some("full".to_string()),
                    help: Some("SQL query. SELECT/WITH returns rows, others return affected_rows.".to_string()),
                    default_value: Some(serde_json::json!("SELECT 1;")),
                    sidebar: vec![
                        SidebarSection {
                            title: "Input payload".to_string(),
                            items: vec![
                                SidebarItem { label: "input".to_string(), type_hint: Some("object".to_string()), description: Some("Upstream payload — bind params from this.".to_string()) },
                            ],
                        },
                        SidebarSection {
                            title: "Output".to_string(),
                            items: vec![
                                SidebarItem { label: "rows".to_string(), type_hint: Some("array".to_string()), description: Some("SELECT/WITH returns { rows: [...] }".to_string()) },
                                SidebarItem { label: "affected_rows".to_string(), type_hint: Some("integer".to_string()), description: Some("INSERT/UPDATE/DELETE returns { affected_rows: N }".to_string()) },
                            ],
                        },
                    ],
                    ..Default::default()
                },
                NodeFieldDef { name: "params_path".to_string(), label: "Params Path".to_string(), field_type: NodeFieldType::Text, help: Some("JSON pointer into upstream payload for $1/$2 bind params. e.g. /body".to_string()), ..Default::default() },
            ]
        },
        layout: vec![
            LayoutItem::Row { row: vec![LayoutItem::Field("title".to_string()), LayoutItem::Field("credential_id".to_string())] },
            LayoutItem::Field("query".to_string()),
            LayoutItem::Field("params_path".to_string()),
        ],
        ai_tool: Default::default(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub credential_id: String,
    pub query: String,
    #[serde(default)]
    pub params_path: Option<String>,
    #[serde(default)]
    pub credential_id_expr: Option<String>,
    #[serde(default)]
    pub query_expr: Option<String>,
    #[serde(default)]
    pub params_expr: Option<String>,
}

pub struct Node {
    config: Config,
    credentials: Arc<CredentialService>,
    language: Arc<dyn LanguageEngine>,
}

impl Node {
    pub fn new(
        config: Config,
        credentials: Arc<CredentialService>,
        language: Arc<dyn LanguageEngine>,
    ) -> Result<Self, PipelineError> {
        if config.credential_id.trim().is_empty()
            && config
                .credential_id_expr
                .as_deref()
                .map(str::trim)
                .unwrap_or_default()
                .is_empty()
        {
            return Err(PipelineError::new(
                "FW_NODE_PG_CONFIG",
                "config.credential_id must not be empty",
            ));
        }
        if config.query.trim().is_empty()
            && config
                .query_expr
                .as_deref()
                .map(str::trim)
                .unwrap_or_default()
                .is_empty()
        {
            return Err(PipelineError::new(
                "FW_NODE_PG_CONFIG",
                "config.query must not be empty",
            ));
        }
        Ok(Self {
            config,
            credentials,
            language,
        })
    }
}

#[async_trait]
impl NodeHandler for Node {
    fn kind(&self) -> &'static str {
        NODE_KIND
    }
    fn input_pins(&self) -> &'static [&'static str] {
        &[INPUT_PIN_IN]
    }
    fn output_pins(&self) -> &'static [&'static str] {
        &[OUTPUT_PIN_OUT]
    }

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        let (owner, project, _pipeline, _request_id) = metadata_scope(&input.metadata)?;
        let credential_id = resolve_string_binding(
            &self.language,
            &input.payload,
            &input.metadata,
            self.config.credential_id_expr.as_deref(),
            &self.config.credential_id,
            "credential_id",
        )?;
        let query = resolve_string_binding(
            &self.language,
            &input.payload,
            &input.metadata,
            self.config.query_expr.as_deref(),
            &self.config.query,
            "query",
        )?;
        let credential = self
            .credentials
            .get_project_credential(owner, project, &credential_id)
            .map_err(|err| PipelineError::new("FW_NODE_PG_CREDENTIAL", err.to_string()))?
            .ok_or_else(|| {
                PipelineError::new(
                    "FW_NODE_PG_CREDENTIAL_MISSING",
                    format!("credential '{}' not found", credential_id),
                )
            })?;
        if credential.kind != "postgres" {
            return Err(PipelineError::new(
                "FW_NODE_PG_CREDENTIAL_KIND",
                format!(
                    "credential '{}' is '{}' not 'postgres'",
                    credential.credential_id, credential.kind
                ),
            ));
        }
        let connect_options = build_postgres_connect_options(&credential.secret)?;

        // Guard: params_path uses dot notation (e.g. "body.id"), NOT JSON pointer ("/body/id").
        if let Some(path) = self.config.params_path.as_deref() {
            if path.starts_with('/') {
                return Err(PipelineError::new(
                    "PG_QUERY_PARAMS_PATH_SYNTAX",
                    format!(
                        "params_path uses dot notation, not JSON pointer. \
                         Use '{}' not '{}'",
                        &path[1..].replace('/', "."),
                        path
                    ),
                ));
            }
        }

        let param_values = if let Some(expr) = self.config.params_expr.as_deref() {
            let evaluated = eval_deno_expr(
                self.language.as_ref(),
                expr,
                &input.payload,
                &input.metadata,
            )?;
            match evaluated {
                Value::Array(items) => items,
                other => vec![other],
            }
        } else {
            resolve_array_values(&input.payload, self.config.params_path.as_deref())
        };

        let pool = sqlx::postgres::PgPoolOptions::new()
            .max_connections(1)
            .connect_with(connect_options)
            .await
            .map_err(|err| PipelineError::new("FW_NODE_PG_CONNECT", err.to_string()))?;

        let lower = query.trim_start().to_ascii_lowercase();
        let payload = if lower.starts_with("select") || lower.starts_with("with") {
            let mut sql_query = sqlx::query(&query);
            for param in &param_values {
                sql_query = bind_json_param(sql_query, param);
            }
            let rows = sql_query
                .fetch_all(&pool)
                .await
                .map_err(|err| PipelineError::new("FW_NODE_PG_QUERY", err.to_string()))?;
            let json_rows = rows
                .into_iter()
                .map(row_to_json)
                .collect::<Result<Vec<_>, _>>()?;
            json!({ "rows": json_rows })
        } else {
            let mut sql_query = sqlx::query(&query);
            for param in &param_values {
                sql_query = bind_json_param(sql_query, param);
            }
            let result = sql_query
                .execute(&pool)
                .await
                .map_err(|err| PipelineError::new("FW_NODE_PG_QUERY", err.to_string()))?;
            json!({ "affected_rows": result.rows_affected() })
        };

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload,
            trace: vec![format!("node_kind={NODE_KIND}")],
        })
    }
}

fn resolve_string_binding(
    language: &Arc<dyn LanguageEngine>,
    input: &Value,
    metadata: &Value,
    expr: Option<&str>,
    fallback: &str,
    field: &str,
) -> Result<String, PipelineError> {
    if let Some(expr) = expr {
        let value = eval_deno_expr(language.as_ref(), expr, input, metadata)?;
        return value.as_str().map(ToString::to_string).ok_or_else(|| {
            PipelineError::new(
                "FW_NODE_PG_BINDING",
                format!("binding expression for '{field}' must return string"),
            )
        });
    }
    let out = fallback.trim();
    if out.is_empty() {
        return Err(PipelineError::new(
            "FW_NODE_PG_BINDING",
            format!("resolved '{field}' must not be empty"),
        ));
    }
    Ok(out.to_string())
}

fn build_postgres_connect_options(secret: &Value) -> Result<PgConnectOptions, PipelineError> {
    let host = secret
        .get("host")
        .and_then(Value::as_str)
        .ok_or_else(|| PipelineError::new("FW_NODE_PG_SECRET", "secret.host is required"))?;
    let port = secret
        .get("port")
        .and_then(|value| {
            value.as_u64().or_else(|| {
                value
                    .as_str()
                    .and_then(|raw| raw.trim().parse::<u64>().ok())
            })
        })
        .unwrap_or(5432);
    let port = u16::try_from(port).map_err(|_| {
        PipelineError::new("FW_NODE_PG_SECRET", "secret.port must be in 0..=65535")
    })?;
    let database = secret
        .get("database")
        .and_then(Value::as_str)
        .ok_or_else(|| PipelineError::new("FW_NODE_PG_SECRET", "secret.database is required"))?;
    let user = secret
        .get("user")
        .and_then(Value::as_str)
        .ok_or_else(|| PipelineError::new("FW_NODE_PG_SECRET", "secret.user is required"))?;
    let password = secret
        .get("password")
        .and_then(Value::as_str)
        .ok_or_else(|| PipelineError::new("FW_NODE_PG_SECRET", "secret.password is required"))?;
    Ok(PgConnectOptions::new()
        .host(host)
        .port(port)
        .database(database)
        .username(user)
        .password(password))
}

fn bind_json_param<'q>(
    query: sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments>,
    value: &Value,
) -> sqlx::query::Query<'q, sqlx::Postgres, sqlx::postgres::PgArguments> {
    // Bind owned values so query lifetime never depends on input payload references.
    match value {
        Value::Null => query.bind(Option::<String>::None),
        Value::Bool(v) => query.bind(*v),
        Value::Number(n) => query.bind(n.to_string()),
        Value::String(s) => query.bind(s.clone()),
        other => query.bind(other.to_string()),
    }
}

fn row_to_json(row: PgRow) -> Result<Value, PipelineError> {
    let mut map = Map::new();
    let columns = row.columns();

    for (idx, column) in columns.iter().enumerate() {
        let name = column.name().to_string();
        let value = row_cell_to_json(&row, idx);
        map.insert(name, value);
    }
    Ok(Value::Object(map))
}

fn row_cell_to_json(row: &PgRow, idx: usize) -> Value {
    if let Ok(v) = row.try_get::<Option<serde_json::Value>, _>(idx) {
        return v.unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<String>, _>(idx) {
        return v.map(Value::String).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<bool>, _>(idx) {
        return v.map(Value::Bool).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<i64>, _>(idx) {
        return v.map(|x| json!(x)).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<i32>, _>(idx) {
        return v.map(|x| json!(x)).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<i16>, _>(idx) {
        return v.map(|x| json!(x)).unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<f64>, _>(idx) {
        return v
            .and_then(serde_json::Number::from_f64)
            .map(Value::Number)
            .unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<f32>, _>(idx) {
        return v
            .and_then(|x| serde_json::Number::from_f64(x as f64))
            .map(Value::Number)
            .unwrap_or(Value::Null);
    }
    if let Ok(v) = row.try_get::<Option<Vec<u8>>, _>(idx) {
        return v
            .map(|bytes| Value::String(hex::encode(bytes)))
            .unwrap_or(Value::Null);
    }
    Value::Null
}
