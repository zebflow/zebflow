//! `sekejap.record.create` — records (and graph edges) written into a
//! Sekejap table in one commit, from structured values rather than SQL text.
//!
//! `--record` and `--edge` take the values themselves: one `{{ expr }}` that
//! is a list, or repeated for single items; no payload key is read. Each
//! record is `{ <key>: …, fields: { … } }`, its key found at `--key`
//! (default `key`). `--max-items` (default 1000) is one ceiling on records
//! and edges together. The declared schema routes vector fields into the
//! native vector storage. The answer is one key, `record`:
//! `{ created, edges, table }`.

use std::path::PathBuf;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::pipeline::model::{DslFlag, DslFlagKind, LayoutItem, NodeCapability, NodeExample, NodeFieldDef, NodeFieldType};
use crate::pipeline::nodes::shared::util::{metadata_scope, resolve_path, with_answer};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::sekejap::{
    self, StructuredInsertEdge, StructuredInsertRecord, StructuredWriteMode,
};

pub const NODE_KIND: &str = "sekejap.record.create";
pub const INPUT_PIN_IN: &str = "in";
pub const OUTPUT_PIN_OUT: &str = "out";

/// The store refused the batch, or failed.
pub const CODE: &str = "FW_NODE_SEKEJAP_RECORD_CREATE";
/// A flag the author set wrong.
pub const CONFIG_CODE: &str = "FW_NODE_SEKEJAP_RECORD_CREATE_CONFIG";
/// A record or edge that is not the shape this node writes.
const INPUT_CODE: &str = "FW_NODE_SEKEJAP_RECORD_CREATE_INPUT";
/// More records and edges than `--max-items`.
const MAX_ITEMS_CODE: &str = "FW_NODE_SEKEJAP_RECORD_CREATE_MAX_ITEMS";

const DEFAULT_KEY: &str = "key";
const DEFAULT_MAX_ITEMS: u64 = 1_000;
/// The highest `--max-items` one run may set.
const MAX_ITEMS_CEILING: u64 = 10_000;

fn flag(name: &str, key: &str, description: &str, kind: DslFlagKind, value: &str) -> DslFlag {
    DslFlag {
        flag: name.to_string(),
        config_key: key.to_string(),
        description: description.to_string(),
        kind,
        value: value.to_string(),
        ..Default::default()
    }
}

fn field(name: &str, label: &str, field_type: NodeFieldType, help: &str) -> NodeFieldDef {
    NodeFieldDef { name: name.to_string(), label: label.to_string(), field_type, help: Some(help.to_string()), ..Default::default() }
}

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Database],
        title: "Sekejap Records".to_string(),
        description: "Write records (and optional graph edges) into a Sekejap table, as one commit. `--record` takes the records \
            themselves — one `{{ expr }}` that is a list (`--record \"{{ input.items }}\"`), or repeated for single items — each \
            `{ key, fields: { … } }`, its key at `--key` (default `key`). They are written typed against the table's declared columns \
            into `--table` — the table must already exist (`CREATE TABLE`), a `VECTOR(n)` field must have exactly n numbers. `--edge` \
            takes edges the same way, each `{ from: { target, key }, type, to: { target, key }, fields? }`; both endpoints must exist, \
            in this batch or before it, and an edge whose `type` is an edge table's label is written into that table. `--max-items` \
            (default 1000, at most 10000) is the ceiling on records and edges together. Adds `record: { created, edges, table }` and \
            keeps the rest of the payload. For one row from a form use `sekejap.query.run --write … INSERT`; this node is for imports \
            and seeds."
            .to_string(),
        input_schema: json!({ "type": "object", "description": "Records and edges reach the node only through --record and --edge." }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "record": {
                    "type": "object",
                    "properties": {
                        "created": { "type": "integer", "description": "Records written" },
                        "edges": { "type": "integer", "description": "Edges written" },
                        "table": { "type": "string" }
                    }
                }
            }
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![
            DslFlag { required: true, ..flag("--table", "table", "The Sekejap table the records go into.", DslFlagKind::Scalar, "text") },
            flag("--record", "record", "Records to write, each { key, fields }: one {{ expr }} that is a list, or repeated for single records.", DslFlagKind::RepeatedList, "json"),
            flag("--edge", "edge", "Edges to write, each { from: { target, key }, type, to: { target, key }, fields? }: one {{ expr }} list, or repeated.", DslFlagKind::RepeatedList, "json"),
            flag("--key", "key", "The field inside each record that holds its key, a dot path (default: key).", DslFlagKind::Scalar, "text"),
            flag("--max-items", "max_items", "Most records and edges together in one run (default: 1000, at most 10000).", DslFlagKind::Scalar, "number"),
        ],
        fields: vec![
            field("table", "Table", NodeFieldType::Text, "The Sekejap table the records go into."),
            field("record", "Records", NodeFieldType::Text, "{{ expr }} that is a list of { key, fields }, e.g. {{ input.items }}."),
            field("edge", "Edges", NodeFieldType::Text, "Optional {{ expr }} list of { from, type, to, fields? }."),
            NodeFieldDef { default_value: Some(json!(DEFAULT_KEY)), ..field("key", "Key", NodeFieldType::Text, "The field inside each record that holds its key.") },
            NodeFieldDef { default_value: Some(json!(DEFAULT_MAX_ITEMS)), ..field("max_items", "Max items", NodeFieldType::Number, "Most records and edges together in one run.") },
        ],
        layout: vec![
            LayoutItem::Field("table".to_string()),
            LayoutItem::Row { row: vec![LayoutItem::Field("record".to_string()), LayoutItem::Field("edge".to_string())] },
            LayoutItem::Row { row: vec![LayoutItem::Field("key".to_string()), LayoutItem::Field("max_items".to_string())] },
        ],
        ai_tool: crate::pipeline::model::NodeAiToolDefinition {
            registered: true,
            tool_name: "sekejap_insert".to_string(),
            tool_description: "Write records and native Sekejap edges into a table without generating SQL strings. Args: table, record (a list of { key, fields }), edge (a list), key, max_items. Vector fields are optimized automatically.".to_string(),
            tool_input_schema: json!({
                "type": "object",
                "properties": {
                    "table": { "type": "string" },
                    "record": { "type": "array" },
                    "edge": { "type": "array" },
                    "key": { "type": "string" },
                    "max_items": { "type": "integer" }
                },
                "required": ["table"]
            }),
        },
        examples: vec![
            NodeExample::dsl("Seed from a prepared list", r#"sekejap.record.create --table products --record "{{ input.items }}""#)
                .input(json!({ "items": [{ "key": "sku-1", "fields": { "name": "Mug", "price": 12 } }] }))
                .output(json!({ "items": [{ "key": "sku-1", "fields": { "name": "Mug", "price": 12 } }], "record": { "created": 1, "edges": 0, "table": "products" } })),
        ],
        ..Default::default()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub table: String,
    /// The records: a list, or a list of lists (one `{{ }}` each).
    #[serde(default)]
    pub record: Value,
    /// The edges, the same way.
    #[serde(default)]
    pub edge: Value,
    /// Dot path inside each record to its key.
    #[serde(default)]
    pub key: String,
    /// Records and edges together, at most.
    #[serde(default)]
    pub max_items: Value,
}

pub struct Node {
    config: Config,
    data_root: PathBuf,
}

impl Node {
    pub fn new(config: Config, data_root: PathBuf) -> Result<Self, PipelineError> {
        if config.table.trim().is_empty() {
            return Err(PipelineError::new(CONFIG_CODE, "--table must not be empty"));
        }
        Ok(Self { config, data_root })
    }

    fn max_items(&self) -> Result<usize, PipelineError> {
        let n = match &self.config.max_items {
            Value::Null => Some(DEFAULT_MAX_ITEMS),
            Value::String(text) if text.trim().is_empty() => Some(DEFAULT_MAX_ITEMS),
            Value::Number(n) => n.as_u64(),
            Value::String(text) => text.trim().parse().ok(),
            _ => None,
        };
        match n {
            Some(n) if (1..=MAX_ITEMS_CEILING).contains(&n) => Ok(n as usize),
            _ => Err(PipelineError::new(
                CONFIG_CODE,
                format!("--max-items {} must be a whole number from 1 to {MAX_ITEMS_CEILING}", self.config.max_items),
            )),
        }
    }
}

/// `--record` / `--edge`: a repeated flag arrives as a list whose items are
/// single objects or, from one `{{ }}`, lists of them; both flatten one level.
fn items(value: &Value, flag: &str) -> Result<Vec<Value>, PipelineError> {
    let given = match value {
        Value::Null => return Ok(Vec::new()),
        Value::Array(items) => items.clone(),
        other => vec![other.clone()],
    };
    let mut out = Vec::new();
    for item in given {
        match item {
            Value::Array(inner) => {
                for one in inner {
                    if !one.is_object() {
                        return Err(PipelineError::new(INPUT_CODE, format!("{flag}: every item is an object, got {one}")));
                    }
                    out.push(one);
                }
            }
            Value::Object(_) => out.push(item),
            other => return Err(PipelineError::new(INPUT_CODE, format!("{flag}: an item is an object or a list of them, got {other}"))),
        }
    }
    Ok(out)
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
        let table = self.config.table.trim().to_string();
        if table.is_empty() {
            return Err(PipelineError::new(CONFIG_CODE, "--table resolved empty"));
        }
        let key_path = match self.config.key.trim() {
            "" => DEFAULT_KEY,
            given => given,
        };
        let max = self.max_items()?;
        let records_given = items(&self.config.record, "--record")?;
        let edges_given = items(&self.config.edge, "--edge")?;
        let total = records_given.len() + edges_given.len();
        if total > max {
            return Err(PipelineError::new(
                MAX_ITEMS_CODE,
                format!("{} records and {} edges are {total} items; --max-items is {max}", records_given.len(), edges_given.len()),
            ));
        }
        if total == 0 {
            return Err(PipelineError::new(INPUT_CODE, "nothing to write: give --record or --edge"));
        }

        let mut records = Vec::with_capacity(records_given.len());
        for (index, record) in records_given.iter().enumerate() {
            let key = scalar_key(resolve_path(record, key_path)).ok_or_else(|| {
                PipelineError::new(INPUT_CODE, format!("record {index} has no string or number key at '{key_path}'"))
            })?;
            let fields = record
                .get("fields")
                .and_then(Value::as_object)
                .cloned()
                .ok_or_else(|| PipelineError::new(INPUT_CODE, format!("record {index} must include a fields object")))?;
            records.push(StructuredInsertRecord { key, fields });
        }
        let mut edges = Vec::with_capacity(edges_given.len());
        for (index, edge) in edges_given.iter().enumerate() {
            edges.push(parse_edge(index, edge)?);
        }

        let data_root = self.data_root.clone();
        let owner = owner.to_string();
        let project = project.to_string();
        let target = table.clone();
        let created = records.len();
        let edge_count = edges.len();
        let result = tokio::task::spawn_blocking(move || {
            sekejap::bulk_insert(&data_root, &owner, &project, &target, records, edges, StructuredWriteMode::Insert)
        })
        .await
        .map_err(|err| PipelineError::new(CODE, format!("sekejap insert task failed: {err}")))?
        .map_err(|err| PipelineError::new(CODE, err.to_string()))?;

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: with_answer(&input.payload, json!({ "record": { "created": created, "edges": edge_count, "table": table } })),
            trace: vec![
                format!("node_kind={NODE_KIND}"),
                format!("created={created}"),
                format!("edges={edge_count}"),
                format!("optimized_fields={}", result.optimized_fields.len()),
            ],
        })
    }
}

fn parse_edge(index: usize, edge: &Value) -> Result<StructuredInsertEdge, PipelineError> {
    let edge_type = edge
        .get("type")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            PipelineError::new(
                INPUT_CODE,
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
            INPUT_CODE,
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
                INPUT_CODE,
                format!("edge {index} {side}.target must be a non-empty string"),
            )
        })?
        .to_string();
    let key = scalar_key(value.get("key")).ok_or_else(|| {
        PipelineError::new(
            INPUT_CODE,
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

    fn seed(root: &std::path::Path, statements: &[&str]) {
        for sql in statements {
            crate::platform::sekejap::execute_sql(root, "demo", "site-a", sql, &[], 10, false).expect(sql);
        }
    }

    async fn exec(root: &std::path::Path, config: Value, payload: Value) -> Result<Value, PipelineError> {
        let node = Node::new(serde_json::from_value(config).expect("config"), root.to_path_buf())?;
        let out = node
            .execute_async(NodeExecutionInput {
                node_id: "n0".to_string(),
                input_pin: "in".to_string(),
                payload,
                metadata: json!({ "owner": "demo", "project": "site-a", "pipeline": "t", "request_id": "r" }),
                bus: None,
            })
            .await?;
        Ok(out.payload)
    }

    #[test]
    fn the_signature_takes_the_values_themselves() {
        assert_eq!(
            crate::pipeline::nodes::node_signature(&definition()),
            "sekejap.record.create --table TEXT [--record JSON…] [--edge JSON…] [--key TEXT] [--max-items N] → record"
        );
    }

    /// sekejap 0.18: an edge whose type belongs to an edge table is written
    /// into that table, in the same commit as the records, and a graph walk
    /// reads its typed property back. `--record` given as one `{{ }}` list
    /// arrives as a list inside the repeated flag's list.
    #[tokio::test]
    async fn writes_edges_whose_type_is_an_edge_table_into_that_table() {
        let tmp = tempfile::tempdir().expect("tempdir");
        seed(tmp.path(), &[
            "CREATE TABLE people (_key TEXT PRIMARY KEY, name TEXT)",
            "CREATE TABLE knows (src TEXT REFERENCES people, dst TEXT REFERENCES people, since INT, PRIMARY KEY (src, dst))",
            "CREATE PROPERTY GRAPH social VERTEX TABLES (people) EDGE TABLES (knows SOURCE KEY (src) REFERENCES people (_key) DESTINATION KEY (dst) REFERENCES people (_key))",
        ]);
        let out = exec(
            tmp.path(),
            json!({
                "table": "people",
                "record": [[
                    { "key": "ann", "fields": { "name": "Ann" } },
                    { "key": "bo", "fields": { "name": "Bo" } }
                ]],
                "edge": [{
                    "from": { "target": "people", "key": "ann" },
                    "type": "knows",
                    "to": { "target": "people", "key": "bo" },
                    "fields": { "since": 2020 }
                }]
            }),
            json!({ "batch": "b1" }),
        )
        .await
        .expect("execute");
        assert_eq!(out, json!({ "batch": "b1", "record": { "created": 2, "edges": 1, "table": "people" } }));

        let walked = crate::platform::sekejap::execute_sql(
            tmp.path(),
            "demo",
            "site-a",
            "SELECT name, since FROM GRAPH_TABLE (social MATCH (a WHERE a._key = 'ann')-[e:knows]->(b) RETURN b.name AS name, e.since AS since)",
            &[],
            10,
            true,
        )
        .expect("walk");
        assert_eq!(walked.rows, vec![vec![json!("Bo"), json!(2020)]]);
    }

    /// `--key` names the field holding each record's key; a native edge
    /// between two tables is written with the records.
    #[tokio::test]
    async fn writes_records_keyed_by_key_and_native_edges() {
        let tmp = tempfile::tempdir().expect("tempdir");
        // The tables exist before the node writes into them, and so does the
        // author the edge starts from: sekejap validates both endpoints.
        seed(tmp.path(), &[
            "CREATE TABLE documents (_key TEXT PRIMARY KEY, title TEXT)",
            "CREATE TABLE authors (_key TEXT PRIMARY KEY, name TEXT)",
            "INSERT INTO authors (_key, name) VALUES ('author:1', 'Ann')",
        ]);
        let out = exec(
            tmp.path(),
            json!({
                "table": "documents",
                "key": "meta.id",
                "record": [{ "meta": { "id": "doc:1" }, "fields": { "title": "One" } }],
                "edge": [{
                    "from": { "target": "authors", "key": "author:1" },
                    "type": "wrote",
                    "to": { "target": "documents", "key": "doc:1" },
                    "fields": { "year": 2026 }
                }]
            }),
            json!({}),
        )
        .await
        .expect("execute");
        assert_eq!(out["record"], json!({ "created": 1, "edges": 1, "table": "documents" }));
        let read = crate::platform::sekejap::execute_sql(tmp.path(), "demo", "site-a", "SELECT title FROM documents WHERE _key = 'doc:1'", &[], 10, true)
            .expect("read");
        assert_eq!(read.rows, vec![vec![json!("One")]]);
    }

    /// One ceiling covers records and edges together; nothing is read from
    /// a payload key.
    #[tokio::test]
    async fn max_items_counts_records_and_edges_together() {
        let tmp = tempfile::tempdir().expect("tempdir");
        seed(tmp.path(), &["CREATE TABLE people (_key TEXT PRIMARY KEY, name TEXT)"]);
        let records = json!([{ "key": "a", "fields": {} }, { "key": "b", "fields": {} }]);
        let edge = json!({ "from": { "target": "people", "key": "a" }, "type": "knows", "to": { "target": "people", "key": "b" } });
        let err = exec(tmp.path(), json!({ "table": "people", "record": [records], "edge": [edge], "max_items": 2 }), json!({}))
            .await
            .unwrap_err();
        assert_eq!(err.code, MAX_ITEMS_CODE, "{}", err.message);
        let none = exec(tmp.path(), json!({ "table": "people" }), json!({ "records": [{ "key": "a", "fields": {} }] }))
            .await
            .unwrap_err();
        assert_eq!(none.code, INPUT_CODE, "a payload `records` key is not read: {}", none.message);
        let over = exec(tmp.path(), json!({ "table": "people", "record": [records], "max_items": 10_001 }), json!({}))
            .await
            .unwrap_err();
        assert_eq!(over.code, CONFIG_CODE);
    }
}
