//! Postgres query node using stored project credentials.

use std::sync::Arc;

use postgres::{Client, NoTls, types::{ToSql, Type}};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::framework::{
    FrameworkError,
    nodes::{FrameworkNode, NodeExecutionInput, NodeExecutionOutput},
};
use crate::platform::services::CredentialService;

use super::util::{metadata_scope, resolve_array_values};

pub const NODE_KIND: &str = "x.n.pg.query";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub credential_id: String,
    pub query: String,
    #[serde(default)]
    pub params_path: Option<String>,
}

pub struct Node {
    config: Config,
    credentials: Arc<CredentialService>,
}

impl Node {
    pub fn new(config: Config, credentials: Arc<CredentialService>) -> Result<Self, FrameworkError> {
        if config.credential_id.trim().is_empty() {
            return Err(FrameworkError::new(
                "FW_NODE_PG_CONFIG",
                "config.credential_id must not be empty",
            ));
        }
        if config.query.trim().is_empty() {
            return Err(FrameworkError::new(
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

impl FrameworkNode for Node {
    fn kind(&self) -> &'static str { NODE_KIND }
    fn input_pins(&self) -> &'static [&'static str] { &[INPUT_PIN_IN] }
    fn output_pins(&self) -> &'static [&'static str] { &[OUTPUT_PIN_OUT] }

    fn execute(&self, input: NodeExecutionInput) -> Result<NodeExecutionOutput, FrameworkError> {
        let (owner, project, _pipeline, _request_id) = metadata_scope(&input.metadata)?;
        let credential = self
            .credentials
            .get_project_credential(owner, project, &self.config.credential_id)
            .map_err(|err| FrameworkError::new("FW_NODE_PG_CREDENTIAL", err.to_string()))?
            .ok_or_else(|| {
                FrameworkError::new(
                    "FW_NODE_PG_CREDENTIAL_MISSING",
                    format!("credential '{}' not found", self.config.credential_id),
                )
            })?;
        if credential.kind != "postgres" {
            return Err(FrameworkError::new(
                "FW_NODE_PG_CREDENTIAL_KIND",
                format!(
                    "credential '{}' is '{}' not 'postgres'",
                    credential.credential_id, credential.kind
                ),
            ));
        }
        let connection_string = build_postgres_connection_string(&credential.secret)?;
        let mut client = Client::connect(&connection_string, NoTls)
            .map_err(|err| FrameworkError::new("FW_NODE_PG_CONNECT", err.to_string()))?;
        let param_values = resolve_array_values(&input.payload, self.config.params_path.as_deref());
        let param_boxes = build_postgres_params(param_values)?;
        let params: Vec<&(dyn ToSql + Sync)> = param_boxes
            .iter()
            .map(|value| &**value as &(dyn ToSql + Sync))
            .collect();

        let lower = self.config.query.trim_start().to_ascii_lowercase();
        let payload = if lower.starts_with("select") || lower.starts_with("with") {
            let rows = client
                .query(&self.config.query, &params)
                .map_err(|err| FrameworkError::new("FW_NODE_PG_QUERY", err.to_string()))?;
            let json_rows = rows.into_iter().map(row_to_json).collect::<Result<Vec<_>, _>>()?;
            json!({ "rows": json_rows })
        } else {
            let affected = client
                .execute(&self.config.query, &params)
                .map_err(|err| FrameworkError::new("FW_NODE_PG_QUERY", err.to_string()))?;
            json!({ "affected_rows": affected })
        };

        Ok(NodeExecutionOutput {
            output_pin: OUTPUT_PIN_OUT.to_string(),
            payload,
            trace: vec![format!("node_kind={NODE_KIND}")],
        })
    }
}

fn build_postgres_connection_string(secret: &Value) -> Result<String, FrameworkError> {
    let host = secret
        .get("host")
        .and_then(Value::as_str)
        .ok_or_else(|| FrameworkError::new("FW_NODE_PG_SECRET", "secret.host is required"))?;
    let port = secret.get("port").and_then(Value::as_u64).unwrap_or(5432);
    let database = secret
        .get("database")
        .and_then(Value::as_str)
        .ok_or_else(|| FrameworkError::new("FW_NODE_PG_SECRET", "secret.database is required"))?;
    let user = secret
        .get("user")
        .and_then(Value::as_str)
        .ok_or_else(|| FrameworkError::new("FW_NODE_PG_SECRET", "secret.user is required"))?;
    let password = secret
        .get("password")
        .and_then(Value::as_str)
        .ok_or_else(|| FrameworkError::new("FW_NODE_PG_SECRET", "secret.password is required"))?;
    Ok(format!(
        "host={host} port={port} dbname={database} user={user} password={password}"
    ))
}

fn build_postgres_params(
    values: Vec<Value>,
) -> Result<Vec<Box<dyn ToSql + Sync>>, FrameworkError> {
    let mut params: Vec<Box<dyn ToSql + Sync>> = Vec::new();
    for value in values {
        match value {
            Value::Null => params.push(Box::new(Option::<String>::None)),
            Value::Bool(v) => params.push(Box::new(v)),
            Value::Number(v) => {
                if let Some(i) = v.as_i64() {
                    params.push(Box::new(i));
                } else if let Some(u) = v.as_u64() {
                    let i = i64::try_from(u).map_err(|_| {
                        FrameworkError::new("FW_NODE_PG_PARAMS", "u64 parameter exceeds i64 range")
                    })?;
                    params.push(Box::new(i));
                } else if let Some(f) = v.as_f64() {
                    params.push(Box::new(f));
                }
            }
            Value::String(v) => params.push(Box::new(v)),
            other => {
                params.push(Box::new(other.to_string()));
            }
        }
    }
    Ok(params)
}

fn row_to_json(row: postgres::Row) -> Result<Value, FrameworkError> {
    let mut map = Map::new();
    for column in row.columns() {
        let name = column.name().to_string();
        let value = match *column.type_() {
            Type::BOOL => row.try_get::<_, Option<bool>>(name.as_str())
                .map(Value::from)
                .unwrap_or(Value::Null),
            Type::INT2 => row.try_get::<_, Option<i16>>(name.as_str())
                .map(Value::from)
                .unwrap_or(Value::Null),
            Type::INT4 => row.try_get::<_, Option<i32>>(name.as_str())
                .map(Value::from)
                .unwrap_or(Value::Null),
            Type::INT8 => row.try_get::<_, Option<i64>>(name.as_str())
                .map(Value::from)
                .unwrap_or(Value::Null),
            Type::FLOAT4 => row.try_get::<_, Option<f32>>(name.as_str())
                .map(|v| json!(v))
                .unwrap_or(Value::Null),
            Type::FLOAT8 => row.try_get::<_, Option<f64>>(name.as_str())
                .map(|v| json!(v))
                .unwrap_or(Value::Null),
            _ => row.try_get::<_, Option<String>>(name.as_str())
                .map(Value::from)
                .unwrap_or(Value::Null),
        };
        map.insert(name, value);
    }
    Ok(Value::Object(map))
}
