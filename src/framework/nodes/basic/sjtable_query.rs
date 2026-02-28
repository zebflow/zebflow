//! Simple Table query/upsert node.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::framework::{
    FrameworkError,
    nodes::{FrameworkNode, NodeExecutionInput, NodeExecutionOutput},
};
use crate::platform::model::{SimpleTableQueryRequest, UpsertSimpleTableRowRequest};
use crate::platform::services::SimpleTableService;

use super::util::{metadata_scope, resolve_path_cloned};

pub const NODE_KIND: &str = "x.n.sjtable.query";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum Operation {
    #[default]
    Query,
    Upsert,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub table: String,
    #[serde(default)]
    pub operation: Operation,
    #[serde(default)]
    pub where_field: Option<String>,
    #[serde(default)]
    pub where_value_path: Option<String>,
    #[serde(default)]
    pub row_id_path: Option<String>,
    #[serde(default)]
    pub data_path: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

pub struct Node {
    config: Config,
    simple_tables: Arc<SimpleTableService>,
}

impl Node {
    pub fn new(config: Config, simple_tables: Arc<SimpleTableService>) -> Result<Self, FrameworkError> {
        if config.table.trim().is_empty() {
            return Err(FrameworkError::new(
                "FW_NODE_SJTABLE_CONFIG",
                "config.table must not be empty",
            ));
        }
        Ok(Self { config, simple_tables })
    }
}

impl FrameworkNode for Node {
    fn kind(&self) -> &'static str { NODE_KIND }
    fn input_pins(&self) -> &'static [&'static str] { &[INPUT_PIN_IN] }
    fn output_pins(&self) -> &'static [&'static str] { &[OUTPUT_PIN_OUT] }

    fn execute(&self, input: NodeExecutionInput) -> Result<NodeExecutionOutput, FrameworkError> {
        let (owner, project, _pipeline, _request_id) = metadata_scope(&input.metadata)?;
        let payload = match self.config.operation {
            Operation::Query => {
                let result = self
                    .simple_tables
                    .query_rows(
                        owner,
                        project,
                        &SimpleTableQueryRequest {
                            table: self.config.table.clone(),
                            where_field: self.config.where_field.clone(),
                            where_value: resolve_path_cloned(
                                &input.payload,
                                self.config.where_value_path.as_deref(),
                            ),
                            limit: self.config.limit.unwrap_or(100),
                        },
                    )
                    .map_err(|err| FrameworkError::new("FW_NODE_SJTABLE_QUERY", err.to_string()))?;
                json!({
                    "table": result.table,
                    "rows": result.rows,
                })
            }
            Operation::Upsert => {
                let row_id = resolve_path_cloned(&input.payload, self.config.row_id_path.as_deref())
                    .and_then(|v| v.as_str().map(ToString::to_string))
                    .ok_or_else(|| {
                        FrameworkError::new(
                            "FW_NODE_SJTABLE_UPSERT",
                            "row_id_path must resolve to a string",
                        )
                    })?;
                let data = resolve_path_cloned(&input.payload, self.config.data_path.as_deref())
                    .unwrap_or_else(|| input.payload.clone());
                let row = self
                    .simple_tables
                    .upsert_row(
                        owner,
                        project,
                        &UpsertSimpleTableRowRequest {
                            table: self.config.table.clone(),
                            row_id,
                            data,
                        },
                    )
                    .map_err(|err| FrameworkError::new("FW_NODE_SJTABLE_UPSERT", err.to_string()))?;
                json!({ "row": row })
            }
        };

        Ok(NodeExecutionOutput {
            output_pin: OUTPUT_PIN_OUT.to_string(),
            payload,
            trace: vec![format!("node_kind={NODE_KIND}")],
        })
    }
}
