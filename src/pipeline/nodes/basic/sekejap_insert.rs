//! `n.sekejap.insert` - structured high-throughput Sekejap insert node.
//!
//! See [`crate::pipeline::nodes`] (`src/pipeline/nodes/mod.rs`) for the
//! framework node contract and [`super`] (`src/pipeline/nodes/basic/mod.rs`)
//! for built-in node registration conventions.
//!
//! This node is the public DML insert path for Sekejap. It accepts structured
//! records and native edge payloads, avoids SQL string construction, and lets
//! the declared Sekejap schema route vector fields into optimized native vector
//! storage automatically.

use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::util::{metadata_scope, resolve_path};
use crate::pipeline::model::{DslFlag, DslFlagKind, LayoutItem, NodeFieldDef, NodeFieldType};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::sekejap::{
    self, StructuredInsertEdge, StructuredInsertRecord, StructuredWriteMode,
};

pub const NODE_KIND: &str = "n.sekejap.insert";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Sekejap Insert".to_string(),
        description: "Bulk insert records and native edges into the project's embedded Sekejap store through the schema-driven typed write path. Vector fields are optimized automatically.".to_string(),
        input_schema: json!({
            "type": "object",
            "description": "Payload containing records and optional native edges. Each record must include a key and fields object.",
            "properties": {
                "records": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["key", "fields"],
                        "properties": {
                            "key": { "type": ["string", "number"] },
                            "fields": { "type": "object" }
                        }
                    }
                },
                "edges": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "required": ["from", "type", "to"],
                        "properties": {
                            "from": { "type": "object" },
                            "type": { "type": "string" },
                            "to": { "type": "object" },
                            "fields": { "type": "object" },
                            "strength": { "type": "number" }
                        }
                    }
                }
            }
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "inserted_records": { "type": "integer" },
                "inserted_edges": { "type": "integer" },
                "optimized_fields": { "type": "array", "items": { "type": "string" } },
                "field_dimensions": { "type": "object" },
                "duration_ms": { "type": "integer" }
            }
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![
            DslFlag { flag: "--target".to_string(), config_key: "target".to_string(), description: "Sekejap collection/type for records.".to_string(), kind: DslFlagKind::Scalar, required: true },
            DslFlag { flag: "--records-path".to_string(), config_key: "records_path".to_string(), description: "Dot path to the input records array. Default: records.".to_string(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--edges-path".to_string(), config_key: "edges_path".to_string(), description: "Dot path to the optional native edges array. Default: edges.".to_string(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--key-path".to_string(), config_key: "key_path".to_string(), description: "Dot path inside each record for the record key. Default: key.".to_string(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--max-records".to_string(), config_key: "max_records".to_string(), description: "Maximum records accepted in one execution. Default: 1000.".to_string(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--max-edges".to_string(), config_key: "max_edges".to_string(), description: "Maximum edges accepted in one execution. Default: 1000.".to_string(), kind: DslFlagKind::Scalar, required: false },
        ],
        fields: vec![
            NodeFieldDef { name: "target".to_string(), label: "Target".to_string(), field_type: NodeFieldType::Text, help: Some("Sekejap collection/type for inserted records.".to_string()), ..Default::default() },
            NodeFieldDef { name: "records_path".to_string(), label: "Records Path".to_string(), field_type: NodeFieldType::Text, default_value: Some(json!("records")), help: Some("Dot path to records in the input payload.".to_string()), ..Default::default() },
            NodeFieldDef { name: "edges_path".to_string(), label: "Edges Path".to_string(), field_type: NodeFieldType::Text, default_value: Some(json!("edges")), help: Some("Dot path to optional native edges in the input payload.".to_string()), ..Default::default() },
            NodeFieldDef { name: "key_path".to_string(), label: "Key Path".to_string(), field_type: NodeFieldType::Text, default_value: Some(json!("key")), help: Some("Dot path inside each record for the record key.".to_string()), ..Default::default() },
            NodeFieldDef { name: "max_records".to_string(), label: "Max Records".to_string(), field_type: NodeFieldType::Number, default_value: Some(json!(1000)), help: Some("Maximum records accepted in one execution.".to_string()), ..Default::default() },
            NodeFieldDef { name: "max_edges".to_string(), label: "Max Edges".to_string(), field_type: NodeFieldType::Number, default_value: Some(json!(1000)), help: Some("Maximum edges accepted in one execution.".to_string()), ..Default::default() },
        ],
        layout: vec![
            LayoutItem::Field("target".to_string()),
            LayoutItem::Row { row: vec![LayoutItem::Field("records_path".to_string()), LayoutItem::Field("edges_path".to_string())] },
            LayoutItem::Row { row: vec![LayoutItem::Field("key_path".to_string()), LayoutItem::Field("max_records".to_string()), LayoutItem::Field("max_edges".to_string())] },
        ],
        ai_tool: crate::pipeline::model::NodeAiToolDefinition {
            registered: true,
            tool_name: "sekejap_insert".to_string(),
            tool_description: "Bulk insert records and native Sekejap edges without generating SQL strings. Args: target, records_path, edges_path, key_path, max_records, max_edges. Vector fields are optimized automatically.".to_string(),
            tool_input_schema: json!({
                "type": "object",
                "properties": {
                    "target": { "type": "string" },
                    "records_path": { "type": "string" },
                    "edges_path": { "type": "string" },
                    "key_path": { "type": "string" },
                    "max_records": { "type": "integer" },
                    "max_edges": { "type": "integer" }
                },
                "required": ["target"]
            }),
        },
        ..Default::default()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub target: String,
    #[serde(default = "default_records_path")]
    pub records_path: String,
    #[serde(default = "default_edges_path")]
    pub edges_path: String,
    #[serde(default = "default_key_path")]
    pub key_path: String,
    #[serde(default = "default_max_records")]
    pub max_records: usize,
    #[serde(default = "default_max_edges")]
    pub max_edges: usize,
}

fn default_records_path() -> String {
    "records".to_string()
}

fn default_edges_path() -> String {
    "edges".to_string()
}

fn default_key_path() -> String {
    "key".to_string()
}

fn default_max_records() -> usize {
    1000
}

fn default_max_edges() -> usize {
    1000
}

pub struct Node {
    config: Config,
    data_root: PathBuf,
}

impl Node {
    pub fn new(config: Config, data_root: PathBuf) -> Result<Self, PipelineError> {
        if config.target.trim().is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_SEKEJAP_INSERT_CONFIG",
                "target must not be empty",
            ));
        }
        if config.records_path.trim().is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_SEKEJAP_INSERT_CONFIG",
                "records_path must not be empty",
            ));
        }
        if config.edges_path.trim().is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_SEKEJAP_INSERT_CONFIG",
                "edges_path must not be empty",
            ));
        }
        if config.key_path.trim().is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_SEKEJAP_INSERT_CONFIG",
                "key_path must not be empty",
            ));
        }
        if config.max_records == 0 {
            return Err(PipelineError::new(
                "FW_NODE_SEKEJAP_INSERT_CONFIG",
                "max_records must be greater than zero",
            ));
        }
        if config.max_edges == 0 {
            return Err(PipelineError::new(
                "FW_NODE_SEKEJAP_INSERT_CONFIG",
                "max_edges must be greater than zero",
            ));
        }
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
        let records_array =
            optional_array_at_path(&input.payload, self.config.records_path.trim())?;
        let edges_array = optional_array_at_path(&input.payload, self.config.edges_path.trim())?;
        if records_array.len() > self.config.max_records {
            return Err(PipelineError::new(
                "FW_NODE_SEKEJAP_INSERT_LIMIT",
                format!(
                    "batch has {} records, max_records is {}",
                    records_array.len(),
                    self.config.max_records
                ),
            ));
        }
        if edges_array.len() > self.config.max_edges {
            return Err(PipelineError::new(
                "FW_NODE_SEKEJAP_INSERT_LIMIT",
                format!(
                    "batch has {} edges, max_edges is {}",
                    edges_array.len(),
                    self.config.max_edges
                ),
            ));
        }
        if records_array.is_empty() && edges_array.is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_SEKEJAP_INSERT_INPUT",
                "insert payload must include at least one record or edge",
            ));
        }

        let mut records = Vec::with_capacity(records_array.len());
        for (index, record) in records_array.iter().enumerate() {
            let key =
                scalar_key(resolve_path(record, self.config.key_path.trim())).ok_or_else(|| {
                    PipelineError::new(
                        "FW_NODE_SEKEJAP_INSERT_INPUT",
                        format!(
                            "record {index} missing scalar key at '{}'",
                            self.config.key_path
                        ),
                    )
                })?;
            let fields = record
                .get("fields")
                .and_then(Value::as_object)
                .cloned()
                .ok_or_else(|| {
                    PipelineError::new(
                        "FW_NODE_SEKEJAP_INSERT_INPUT",
                        format!("record {index} must include a fields object"),
                    )
                })?;
            records.push(StructuredInsertRecord { key, fields });
        }

        let mut edges = Vec::with_capacity(edges_array.len());
        for (index, edge) in edges_array.iter().enumerate() {
            edges.push(parse_edge(index, edge)?);
        }

        let data_root = self.data_root.clone();
        let owner = owner.to_string();
        let project = project.to_string();
        let target = self.config.target.clone();
        let record_count = records.len();
        let edge_count = edges.len();
        let result = tokio::task::spawn_blocking(move || {
            sekejap::bulk_insert(
                &data_root,
                &owner,
                &project,
                &target,
                records,
                edges,
                StructuredWriteMode::Insert,
            )
        })
        .await
        .map_err(|err| {
            PipelineError::new(
                "FW_NODE_SEKEJAP_INSERT_JOIN",
                format!("sekejap insert task failed: {err}"),
            )
        })?
        .map_err(|err| PipelineError::new("FW_NODE_SEKEJAP_INSERT", err.to_string()))?;

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: json!({
                "inserted_records": record_count,
                "inserted_edges": edge_count,
                "affected_rows": result.affected_rows,
                "optimized_fields": result.optimized_fields,
                "field_dimensions": result.field_dimensions,
                "duration_ms": result.duration_ms,
            }),
            trace: vec![
                format!("node_kind={NODE_KIND}"),
                format!("inserted_records={record_count}"),
                format!("inserted_edges={edge_count}"),
                format!("optimized_fields={}", result.optimized_fields.len()),
            ],
        })
    }
}

fn optional_array_at_path(payload: &Value, path: &str) -> Result<Vec<Value>, PipelineError> {
    let Some(value) = resolve_path(payload, path) else {
        return Ok(Vec::new());
    };
    value.as_array().cloned().ok_or_else(|| {
        PipelineError::new(
            "FW_NODE_SEKEJAP_INSERT_INPUT",
            format!("{path} must resolve to an array"),
        )
    })
}

fn parse_edge(index: usize, edge: &Value) -> Result<StructuredInsertEdge, PipelineError> {
    let edge_type = edge
        .get("type")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            PipelineError::new(
                "FW_NODE_SEKEJAP_INSERT_INPUT",
                format!("edge {index} missing non-empty type"),
            )
        })?
        .to_string();

    let from = edge_endpoint(index, "from", edge.get("from"))?;
    let to = edge_endpoint(index, "to", edge.get("to"))?;
    let fields = edge
        .get("fields")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let strength = edge.get("strength").and_then(Value::as_f64).unwrap_or(1.0) as f32;

    Ok(StructuredInsertEdge {
        from_target: from.0,
        from_key: from.1,
        edge_type,
        to_target: to.0,
        to_key: to.1,
        fields,
        strength,
    })
}

fn edge_endpoint(
    index: usize,
    side: &str,
    value: Option<&Value>,
) -> Result<(String, String), PipelineError> {
    let value = value.and_then(Value::as_object).ok_or_else(|| {
        PipelineError::new(
            "FW_NODE_SEKEJAP_INSERT_INPUT",
            format!("edge {index} {side} must be an object"),
        )
    })?;
    let target = value
        .get("target")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            PipelineError::new(
                "FW_NODE_SEKEJAP_INSERT_INPUT",
                format!("edge {index} {side}.target must be a non-empty string"),
            )
        })?
        .to_string();
    let key = scalar_key(value.get("key")).ok_or_else(|| {
        PipelineError::new(
            "FW_NODE_SEKEJAP_INSERT_INPUT",
            format!("edge {index} {side}.key must be a string or number"),
        )
    })?;
    Ok((target, key))
}

fn scalar_key(value: Option<&Value>) -> Option<String> {
    match value? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn executes_records_and_native_edges_from_insert_contract() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let node = Node::new(
            Config {
                target: "documents".to_string(),
                records_path: "records".to_string(),
                edges_path: "edges".to_string(),
                key_path: "key".to_string(),
                max_records: 10,
                max_edges: 10,
            },
            tmp.path().to_path_buf(),
        )
        .expect("node");

        let output = node
            .execute_async(NodeExecutionInput {
                node_id: "insert".to_string(),
                input_pin: "in".to_string(),
                payload: json!({
                    "records": [
                        {
                            "key": "doc:1",
                            "fields": {
                                "title": "One"
                            }
                        }
                    ],
                    "edges": [
                        {
                            "from": { "target": "authors", "key": "author:1" },
                            "type": "wrote",
                            "to": { "target": "documents", "key": "doc:1" },
                            "fields": { "year": 2026 },
                            "strength": 1.0
                        }
                    ]
                }),
                metadata: json!({
                    "owner": "alice",
                    "project": "demo",
                    "pipeline": "insert-test",
                    "request_id": "req-1"
                }),
                bus: None,
            })
            .await
            .expect("execute");

        assert_eq!(output.payload["inserted_records"], json!(1));
        assert_eq!(output.payload["inserted_edges"], json!(1));
        assert_eq!(output.payload["affected_rows"], json!(2));
    }
}
