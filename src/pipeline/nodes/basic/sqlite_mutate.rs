//! SQLite mutate node — INSERT / UPDATE / DELETE against the project's embedded SQLite database.

use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::pipeline::model::{DslFlag, DslFlagKind, NodeFieldDef, NodeFieldType};
use super::util::metadata_scope;

pub const NODE_KIND: &str = "n.sqlite.mutate";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

/// Unified node-definition metadata for `n.sqlite.mutate`.
pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "SQLite Mutate".to_string(),
        description: "Run a SQL mutation (INSERT INTO, UPDATE, DELETE FROM, CREATE TABLE) \
            against the project's embedded SQLite database. Write the mutation in the body \
            using `-- \"INSERT INTO ...\"`. \
            Use `{{ expr }}` placeholders anywhere in the SQL — they are resolved before the node \
            runs. Output: `{ ok: true, affected_rows: N }`."
            .to_string(),
        input_schema: json!({
            "type": "object",
            "description": "Input context — values accessible via {{ $input.* }} in the SQL."
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "ok": { "type": "boolean" },
                "affected_rows": { "type": "integer" }
            }
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![DslFlag {
            flag: "--sql".to_string(),
            config_key: "sql".to_string(),
            description: "SQL mutation (alternative to body `-- \"INSERT INTO ...\"`)".to_string(),
            kind: DslFlagKind::Scalar,
            required: false,
        }],
        fields: vec![NodeFieldDef {
            name: "sql".to_string(),
            label: "SQL".to_string(),
            field_type: NodeFieldType::Textarea,
            rows: Some(6),
            help: Some(
                "INSERT INTO items (id, title) VALUES ('{{ $input.id }}', '{{ $input.title }}')\n\
                 Supports: INSERT INTO, UPDATE, DELETE FROM, CREATE TABLE, DROP TABLE."
                    .to_string(),
            ),
            ..Default::default()
        }],
        layout: vec![crate::pipeline::model::LayoutItem::Field("sql".to_string())],
        ai_tool: crate::pipeline::model::NodeAiToolDefinition {
            registered: true,
            tool_name: "sqlite_mutate".to_string(),
            tool_description: "Run a SQL mutation against the project's embedded SQLite database. \
                Arg: sql (required) — INSERT / UPDATE / DELETE / CREATE TABLE."
                .to_string(),
            tool_input_schema: json!({
                "type": "object",
                "properties": {
                    "sql": { "type": "string", "description": "SQL mutation string" }
                },
                "required": ["sql"]
            }),
        },
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// SQL mutation — populated from DSL body `-- "INSERT INTO ..."`.
    /// `{{ expr }}` placeholders are resolved before the node runs.
    #[serde(default)]
    pub sql: String,
}

pub struct Node {
    config: Config,
    data_root: PathBuf,
}

impl Node {
    pub fn new(config: Config, data_root: PathBuf) -> Result<Self, PipelineError> {
        Ok(Self { config, data_root })
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
        let sql = self.config.sql.trim().to_string();
        if sql.is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_SQLITE_MUTATE",
                "sql must not be empty — use: -- \"INSERT INTO ...\"",
            ));
        }
        let db_path = self.data_root
            .join("users")
            .join(owner)
            .join(project)
            .join("data")
            .join("local.db");
        let affected_rows = tokio::task::spawn_blocking(move || -> Result<usize, String> {
            let conn = rusqlite::Connection::open(&db_path)
                .map_err(|e| format!("open db: {e}"))?;
            conn.execute_batch("PRAGMA journal_mode=WAL;")
                .map_err(|e| format!("pragma: {e}"))?;
            let n = conn.execute(&sql, []).map_err(|e| format!("execute: {e}"))?;
            Ok(n)
        })
        .await
        .map_err(|e| PipelineError::new("FW_NODE_SQLITE_MUTATE", format!("task: {e}")))?
        .map_err(|e| PipelineError::new("FW_NODE_SQLITE_MUTATE", e))?;

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: json!({ "ok": true, "affected_rows": affected_rows }),
            trace: vec![format!("node_kind={NODE_KIND}")],
        })
    }
}
