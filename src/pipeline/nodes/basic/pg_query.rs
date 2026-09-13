/// Postgres query node using stored project credentials.
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use sqlx::{Column, Row, postgres::PgConnectOptions, postgres::PgRow};

use crate::pipeline::model::NodeCapability;
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::CredentialService;

use super::util::metadata_scope;
use crate::pipeline::model::{DslFlag, DslFlagKind, LayoutItem};

pub const NODE_KIND: &str = "n.pg.query";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

/// Unified node-definition metadata for `n.pg.query`.
pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Network, NodeCapability::Database, NodeCapability::Credential, NodeCapability::Process],
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
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![
            DslFlag { flag: "--credential".to_string(), config_key: "credential_id".to_string(), description: "Credential ID of the PostgreSQL connection to use (from credential_list, kind postgres).".to_string(), kind: DslFlagKind::Scalar, required: true },
            DslFlag { flag: "--query".to_string(), config_key: "query".to_string(), description: "SQL (alternative to the body `-- \"SELECT ...\"`). A literal or {{ expr }}.".to_string(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--params".to_string(), config_key: "params".to_string(), description: "Bind values for $1, $2, … — a literal or {{ expr }}. A whole {{ }} carries its typed value, so \"{{ [$trigger.params.id] }}\" is a real array; a single value is wrapped into one.".to_string(), kind: DslFlagKind::Scalar, required: false },
        ],
        fields: {
            use crate::pipeline::model::{NodeFieldDef, NodeFieldType, NodeFieldDataSource, SidebarSection, SidebarItem};
            vec![
                NodeFieldDef { name: "credential_id".to_string(), label: "Credential".to_string(), field_type: NodeFieldType::Select, data_source: Some(NodeFieldDataSource::CredentialsPostgres), help: Some("Loaded from project credentials filtered by kind=postgres.".to_string()), ..Default::default() },
                NodeFieldDef {
                    name: "query".to_string(),
                    label: "Query".to_string(),
                    field_type: NodeFieldType::CodeEditor,
                    language: Some("sql".to_string()),
                    span: Some("full".to_string()),
                    help: Some("SQL query. SELECT/WITH returns rows, others return affected_rows.".to_string()),
                    default_value: Some(serde_json::json!("SELECT 1\nAS ok;")),
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
            ]
        },
        layout: vec![
            LayoutItem::Field("credential_id".to_string()),
            LayoutItem::Field("query".to_string()),
        ],
        ai_tool: crate::pipeline::model::NodeAiToolDefinition {
            registered: true,
            tool_name: "database_query".to_string(),
            tool_description: "Execute a SQL query against the configured PostgreSQL database. Args: query (required).".to_string(),
            tool_input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "SQL query to execute" }
                },
                "required": ["query"]
            }),
        },
        ..Default::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// The credential slug — a literal or `{{ expr }}`, arriving final.
    pub credential_id: String,
    /// The SQL — a literal or `{{ expr }}`, arriving final.
    pub query: String,
    /// Bind values for `$1`, `$2`, … A whole `{{ }}` carries its typed value,
    /// so `--params "{{ [input.id, 10] }}"` is a real array. A single value is
    /// wrapped into a one-element list.
    #[serde(default)]
    pub params: Value,
}

pub struct Node {
    config: Config,
    credentials: Arc<CredentialService>,
}

impl Node {
    pub fn new(
        config: Config,
        credentials: Arc<CredentialService>,
    ) -> Result<Self, PipelineError> {
        // One flag each now, so "set one or the other, not both" has nothing
        // left to adjudicate. A `{{ }}` that resolves to empty is caught at
        // run time, where the resolved value is known.
        if config.credential_id.trim().is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_PG_CONFIG",
                "config.credential_id must not be empty",
            ));
        }
        if config.query.trim().is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_PG_CONFIG",
                "config.query must not be empty",
            ));
        }
        Ok(Self {
            config,
            credentials,
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
        // Both arrive final: `{{ }}` resolved engine-side before this node
        // ran (NodeIO §Value resolution), so a literal is a literal and an
        // expression already became its value.
        let credential_id = require_non_empty(&self.config.credential_id, "credential_id")?;
        let query = require_non_empty(&self.config.query, "query")?;
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

        // Bind values arrive final. A whole `{{ }}` carries its typed value,
        // so an array stays an array; anything else binds as one parameter.
        let param_values = match self.config.params.clone() {
            Value::Null => Vec::new(),
            Value::Array(items) => items,
            other => vec![other],
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

/// A required string field, after resolution.
fn require_non_empty(value: &str, field: &str) -> Result<String, PipelineError> {
    let out = value.trim();
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
    let port = u16::try_from(port)
        .map_err(|_| PipelineError::new("FW_NODE_PG_SECRET", "secret.port must be in 0..=65535"))?;
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
