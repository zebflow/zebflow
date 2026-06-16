//! n.table.convert — convert table-like data between ZebFS objects and JSON payloads.

use std::collections::HashSet;
use std::io::Write;
use std::sync::Arc;

use async_trait::async_trait;
use datafusion::arrow::{
    array::{
        Array, ArrayRef, BooleanArray, BooleanBuilder, Float32Array, Float64Array, Float64Builder,
        Int8Array, Int16Array, Int32Array, Int64Array, Int64Builder, LargeStringArray, StringArray,
        StringBuilder, UInt8Array, UInt16Array, UInt32Array, UInt64Array,
    },
    datatypes::{DataType, Field, Schema},
    record_batch::RecordBatch,
    util::display::array_value_to_string,
};
use parquet::{
    arrow::{ArrowWriter, arrow_reader::ParquetRecordBatchReaderBuilder},
    basic::Compression,
    file::properties::WriterProperties,
};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use super::file_ref::file_ref_to_rel_path;
use super::util::{eval_deno_expr, metadata_scope};
use crate::language::LanguageEngine;
use crate::pipeline::{
    NodeDefinition, PipelineError,
    model::{DslFlag, DslFlagKind, LayoutItem, NodeFieldDef, NodeFieldType, SelectOptionDef},
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;
use crate::zebfs::{LocalZebFs, normalize_object_path};

pub const NODE_KIND: &str = "n.table.convert";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// ZebFS object path to read from.
    #[serde(default)]
    pub from_path: Option<String>,
    /// Deno expression that returns rows or a table-shaped object from upstream payload.
    #[serde(default)]
    pub from_expr: Option<String>,
    /// Source format: csv, json, ndjson, parquet. Inferred from path when omitted.
    #[serde(default)]
    pub from_format: Option<String>,
    /// ZebFS object path to write to.
    #[serde(default)]
    pub to_path: Option<String>,
    /// Target format: csv, json, ndjson, parquet. Inferred from path when omitted.
    #[serde(default)]
    pub to_format: Option<String>,
    /// Also emit rows as JSON for downstream nodes.
    #[serde(default)]
    pub to_json: bool,
    /// Number of rows to include under table.preview.
    #[serde(default)]
    pub preview: Option<usize>,
    /// Maximum number of rows to convert.
    #[serde(default)]
    pub limit: Option<usize>,
}

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "Table Convert".to_string(),
        description: "Convert table-shaped data between ZebFS objects and downstream JSON. \
            Supports CSV, JSON, NDJSON, and Parquet. Reads from `--from` ZebFS path or \
            `--from-expr` Deno expression such as `$input.rows`, writes to `--to`, and can emit \
            rows with `--to-json`."
            .to_string(),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        input_schema: json!({
            "type": "object",
            "description": "Any payload. Use --from-expr to select rows from upstream output, for example $input.rows."
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "table": {
                    "type": "object",
                    "properties": {
                        "from": { "type": "string" },
                        "to": { "type": ["string", "null"] },
                        "url": { "type": ["string", "null"] },
                        "from_format": { "type": "string" },
                        "to_format": { "type": ["string", "null"] },
                        "rows": { "type": "integer" },
                        "columns": { "type": "array", "items": { "type": "string" } },
                        "preview": { "type": "array" },
                        "data": {
                            "type": "array",
                            "description": "Rows are present only when --to-json is enabled."
                        }
                    }
                }
            }
        }),
        config_schema: Default::default(),
        dsl_flags: vec![
            DslFlag {
                flag: "--from".to_string(),
                config_key: "from_path".to_string(),
                description: "ZebFS object path to read from, for example uploads/data.csv."
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--from-expr".to_string(),
                config_key: "from_expr".to_string(),
                description:
                    "Deno expression returning rows from upstream payload, for example $input.rows."
                        .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--from-format".to_string(),
                config_key: "from_format".to_string(),
                description:
                    "Source format: csv, json, ndjson, or parquet. Required for string expression input."
                        .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--to".to_string(),
                config_key: "to_path".to_string(),
                description: "ZebFS object path to write to, for example datasets/data.ndjson."
                    .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--to-format".to_string(),
                config_key: "to_format".to_string(),
                description:
                    "Target format: csv, json, ndjson, or parquet. Inferred from --to when omitted."
                        .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--to-json".to_string(),
                config_key: "to_json".to_string(),
                description: "Emit converted rows as JSON for downstream nodes.".to_string(),
                kind: DslFlagKind::Bool,
                required: false,
            },
            DslFlag {
                flag: "--preview".to_string(),
                config_key: "preview".to_string(),
                description: "Number of rows to include under table.preview.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--limit".to_string(),
                config_key: "limit".to_string(),
                description: "Maximum number of rows to convert.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: vec![
            NodeFieldDef {
                name: "from_path".to_string(),
                label: "From path".to_string(),
                field_type: NodeFieldType::Text,
                placeholder: Some("uploads/data.csv".to_string()),
                help: Some(
                    "ZebFS object path. Leave empty when using From expression.".to_string(),
                ),
                span: Some("half".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "from_expr".to_string(),
                label: "From expression".to_string(),
                field_type: NodeFieldType::Text,
                placeholder: Some("$input.rows".to_string()),
                help: Some(
                    "Deno expression for upstream rows. Leave empty when using From path."
                        .to_string(),
                ),
                span: Some("half".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "from_format".to_string(),
                label: "From format".to_string(),
                field_type: NodeFieldType::Select,
                options: format_options(true),
                help: Some("Auto infers from path extension when possible.".to_string()),
                span: Some("half".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "to_path".to_string(),
                label: "To path".to_string(),
                field_type: NodeFieldType::Text,
                placeholder: Some("datasets/data.ndjson".to_string()),
                help: Some(
                    "ZebFS object path to write. Leave empty for JSON-only output.".to_string(),
                ),
                span: Some("half".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "to_format".to_string(),
                label: "To format".to_string(),
                field_type: NodeFieldType::Select,
                options: format_options(true),
                help: Some("Auto infers from target path extension when possible.".to_string()),
                span: Some("half".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "to_json".to_string(),
                label: "Emit JSON rows".to_string(),
                field_type: NodeFieldType::Checkbox,
                help: Some("Useful when the next node needs row data directly.".to_string()),
                span: Some("half".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "preview".to_string(),
                label: "Preview rows".to_string(),
                field_type: NodeFieldType::Number,
                placeholder: Some("20".to_string()),
                help: Some("Number of converted rows to include in the inline preview output.".to_string()),
                span: Some("half".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "limit".to_string(),
                label: "Limit rows".to_string(),
                field_type: NodeFieldType::Number,
                help: Some("Maximum number of source rows to convert. Leave empty to convert all rows.".to_string()),
                span: Some("half".to_string()),
                ..Default::default()
            },
        ],
        layout: vec![
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("from_path".to_string()),
                    LayoutItem::Field("from_expr".to_string()),
                ],
            },
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("from_format".to_string()),
                    LayoutItem::Field("to_path".to_string()),
                ],
            },
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("to_format".to_string()),
                    LayoutItem::Field("to_json".to_string()),
                ],
            },
            LayoutItem::Row {
                row: vec![
                    LayoutItem::Field("preview".to_string()),
                    LayoutItem::Field("limit".to_string()),
                ],
            },
        ],
        script_available: false,
        script_bridge: None,
        ai_tool: Default::default(),
        ..Default::default()
    }
}

fn format_options(include_auto: bool) -> Vec<SelectOptionDef> {
    let mut out = Vec::new();
    if include_auto {
        out.push(SelectOptionDef {
            value: String::new(),
            label: "Auto".to_string(),
        });
    }
    out.extend([
        SelectOptionDef {
            value: "csv".to_string(),
            label: "CSV".to_string(),
        },
        SelectOptionDef {
            value: "json".to_string(),
            label: "JSON".to_string(),
        },
        SelectOptionDef {
            value: "ndjson".to_string(),
            label: "NDJSON".to_string(),
        },
        SelectOptionDef {
            value: "parquet".to_string(),
            label: "Parquet".to_string(),
        },
    ]);
    out
}

pub struct Node {
    config: Config,
    platform: Arc<PlatformService>,
    language: Arc<dyn LanguageEngine>,
}

impl Node {
    pub fn new(
        config: Config,
        platform: Arc<PlatformService>,
        language: Arc<dyn LanguageEngine>,
    ) -> Result<Self, PipelineError> {
        Ok(Self {
            config,
            platform,
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
        let (owner, project, ..) = metadata_scope(&input.metadata)?;
        let layout = self
            .platform
            .file
            .ensure_project_layout(owner, project)
            .map_err(|err| PipelineError::new("FW_NODE_TABLE_CONVERT", err.to_string()))?;
        let zebfs = LocalZebFs::new(layout.files_dir);

        let source = self.read_source(&input, &zebfs)?;
        let from_format = source.format;
        let mut rows = rows_from_source(source.value, from_format)?;
        if let Some(limit) = self.config.limit {
            rows.truncate(limit);
        }

        let columns = collect_columns(&rows);
        let mut to_path = None;
        let mut url = None;
        let mut to_format_value = None;

        if let Some(path) = non_empty(self.config.to_path.as_deref()) {
            let rel_path = normalize_object_path(path)
                .map_err(|err| PipelineError::new("FW_NODE_TABLE_CONVERT", err.to_string()))?;
            let to_format =
                normalize_format(self.config.to_format.as_deref(), Some(&rel_path), "target")?;
            let bytes = encode_rows(&rows, &columns, to_format)?;
            zebfs
                .put(&rel_path, &bytes)
                .map_err(|err| PipelineError::new("FW_NODE_TABLE_CONVERT", err.to_string()))?;
            url = Some(format!("/fs/{owner}/{project}/{rel_path}"));
            to_path = Some(rel_path);
            to_format_value = Some(to_format.as_str().to_string());
        }

        if to_path.is_none() && !self.config.to_json {
            return Err(PipelineError::new(
                "FW_NODE_TABLE_CONVERT",
                "set --to to write a ZebFS object or --to-json to emit rows downstream",
            ));
        }

        let preview_len = self.config.preview.unwrap_or(0).min(rows.len());
        let mut table = Map::new();
        table.insert("from".to_string(), Value::String(source.label));
        table.insert("to".to_string(), option_string(to_path));
        table.insert("url".to_string(), option_string(url));
        table.insert(
            "from_format".to_string(),
            Value::String(from_format.as_str().to_string()),
        );
        table.insert("to_format".to_string(), option_string(to_format_value));
        table.insert("rows".to_string(), json!(rows.len()));
        table.insert("columns".to_string(), json!(columns));
        table.insert(
            "preview".to_string(),
            Value::Array(rows.iter().take(preview_len).cloned().collect()),
        );
        if self.config.to_json {
            table.insert("data".to_string(), Value::Array(rows.clone()));
        }

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: json!({ "table": table }),
            trace: vec![format!(
                "node_kind={NODE_KIND} from_format={} rows={}",
                from_format.as_str(),
                rows.len()
            )],
        })
    }
}

struct SourceData {
    label: String,
    value: SourceValue,
    format: TableFormat,
}

enum SourceValue {
    Bytes(Vec<u8>),
    Json(Value),
}

impl Node {
    fn read_source(
        &self,
        input: &NodeExecutionInput,
        zebfs: &LocalZebFs,
    ) -> Result<SourceData, PipelineError> {
        let from_path = non_empty(self.config.from_path.as_deref());
        let from_expr = non_empty(self.config.from_expr.as_deref());
        match (from_path, from_expr) {
            (Some(_), Some(_)) => Err(PipelineError::new(
                "FW_NODE_TABLE_CONVERT",
                "use either --from or --from-expr, not both",
            )),
            (Some(path), None) => {
                let rel_path = normalize_object_path(path)
                    .map_err(|err| PipelineError::new("FW_NODE_TABLE_CONVERT", err.to_string()))?;
                let format = normalize_format(
                    self.config.from_format.as_deref(),
                    Some(&rel_path),
                    "source",
                )?;
                let object = zebfs
                    .get(&rel_path)
                    .map_err(|err| PipelineError::new("FW_NODE_TABLE_CONVERT", err.to_string()))?;
                Ok(SourceData {
                    label: rel_path,
                    value: SourceValue::Bytes(object.bytes),
                    format,
                })
            }
            (None, Some(expr)) => {
                let value = eval_deno_expr(
                    self.language.as_ref(),
                    expr,
                    &input.payload,
                    &input.metadata,
                )?;
                if let Some(path) = file_ref_to_rel_path(&value) {
                    let rel_path = normalize_object_path(&path).map_err(|err| {
                        PipelineError::new("FW_NODE_TABLE_CONVERT", err.to_string())
                    })?;
                    let format = normalize_format(
                        self.config.from_format.as_deref(),
                        Some(&rel_path),
                        "source",
                    )?;
                    let object = zebfs.get(&rel_path).map_err(|err| {
                        PipelineError::new("FW_NODE_TABLE_CONVERT", err.to_string())
                    })?;
                    return Ok(SourceData {
                        label: rel_path,
                        value: SourceValue::Bytes(object.bytes),
                        format,
                    });
                }
                let format = normalize_format(self.config.from_format.as_deref(), None, "source")
                    .or_else(|_| infer_json_source_format(&value))?;
                Ok(SourceData {
                    label: "expr".to_string(),
                    value: SourceValue::Json(value),
                    format,
                })
            }
            (None, None) => Err(PipelineError::new(
                "FW_NODE_TABLE_CONVERT",
                "set --from for a ZebFS object or --from-expr for upstream rows",
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TableFormat {
    Csv,
    Json,
    Ndjson,
    Parquet,
}

impl TableFormat {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Csv => "csv",
            Self::Json => "json",
            Self::Ndjson => "ndjson",
            Self::Parquet => "parquet",
        }
    }
}

fn normalize_format(
    explicit: Option<&str>,
    path: Option<&str>,
    role: &str,
) -> Result<TableFormat, PipelineError> {
    if let Some(value) = non_empty(explicit) {
        return parse_format(value);
    }
    if let Some(path) = path {
        let ext = std::path::Path::new(path)
            .extension()
            .and_then(|value| value.to_str())
            .unwrap_or_default();
        if !ext.is_empty() {
            return parse_format(ext);
        }
    }
    Err(PipelineError::new(
        "FW_NODE_TABLE_CONVERT",
        format!("{role} format is not known; set the matching format flag"),
    ))
}

fn infer_json_source_format(value: &Value) -> Result<TableFormat, PipelineError> {
    match value {
        Value::Array(_) | Value::Object(_) => Ok(TableFormat::Json),
        Value::String(_) => Err(PipelineError::new(
            "FW_NODE_TABLE_CONVERT",
            "string expression input needs --from-format csv, json, or ndjson",
        )),
        _ => Ok(TableFormat::Json),
    }
}

pub(crate) fn parse_format(value: &str) -> Result<TableFormat, PipelineError> {
    match value.trim().to_ascii_lowercase().as_str() {
        "csv" => Ok(TableFormat::Csv),
        "json" => Ok(TableFormat::Json),
        "ndjson" | "jsonl" => Ok(TableFormat::Ndjson),
        "parquet" => Ok(TableFormat::Parquet),
        other => Err(PipelineError::new(
            "FW_NODE_TABLE_CONVERT",
            format!("unsupported table format '{other}'"),
        )),
    }
}

fn rows_from_source(source: SourceValue, format: TableFormat) -> Result<Vec<Value>, PipelineError> {
    match (source, format) {
        (SourceValue::Json(value), TableFormat::Json) => rows_from_json_value(value),
        (SourceValue::Json(Value::String(text)), TableFormat::Csv) => rows_from_csv_text(&text),
        (SourceValue::Json(Value::String(text)), TableFormat::Ndjson) => {
            rows_from_ndjson_text(&text)
        }
        (SourceValue::Json(value), TableFormat::Csv) => rows_from_json_value(value),
        (SourceValue::Json(value), TableFormat::Ndjson) => rows_from_json_value(value),
        (SourceValue::Bytes(bytes), TableFormat::Csv) => {
            let text = String::from_utf8(bytes).map_err(|err| {
                PipelineError::new("FW_NODE_TABLE_CONVERT", format!("CSV is not UTF-8: {err}"))
            })?;
            rows_from_csv_text(&text)
        }
        (SourceValue::Bytes(bytes), TableFormat::Json) => {
            let value: Value = serde_json::from_slice(&bytes).map_err(|err| {
                PipelineError::new("FW_NODE_TABLE_CONVERT", format!("JSON parse error: {err}"))
            })?;
            rows_from_json_value(value)
        }
        (SourceValue::Bytes(bytes), TableFormat::Ndjson) => {
            let text = String::from_utf8(bytes).map_err(|err| {
                PipelineError::new(
                    "FW_NODE_TABLE_CONVERT",
                    format!("NDJSON is not UTF-8: {err}"),
                )
            })?;
            rows_from_ndjson_text(&text)
        }
        (SourceValue::Bytes(bytes), TableFormat::Parquet) => rows_from_parquet_bytes(bytes),
        (SourceValue::Json(value), TableFormat::Parquet) => rows_from_json_value(value),
    }
}

fn rows_from_json_value(value: Value) -> Result<Vec<Value>, PipelineError> {
    match value {
        Value::Array(items) => Ok(items.into_iter().map(row_object).collect()),
        Value::Object(mut map) => {
            if let Some(Value::Array(rows)) = map.remove("rows") {
                return Ok(rows.into_iter().map(row_object).collect());
            }
            Ok(vec![Value::Object(map)])
        }
        Value::Null => Ok(Vec::new()),
        other => Ok(vec![json!({ "value": other })]),
    }
}

fn row_object(value: Value) -> Value {
    match value {
        Value::Object(_) => value,
        other => json!({ "value": other }),
    }
}

fn rows_from_csv_text(text: &str) -> Result<Vec<Value>, PipelineError> {
    let records = parse_csv_records(text)?;
    if records.is_empty() {
        return Ok(Vec::new());
    }
    let headers = records[0]
        .iter()
        .enumerate()
        .map(|(index, header)| {
            let trimmed = header.trim();
            if trimmed.is_empty() {
                format!("column_{}", index + 1)
            } else {
                trimmed.to_string()
            }
        })
        .collect::<Vec<_>>();
    let mut rows = Vec::new();
    for record in records.into_iter().skip(1) {
        if record.iter().all(|cell| cell.is_empty()) {
            continue;
        }
        let mut map = Map::new();
        for (index, header) in headers.iter().enumerate() {
            map.insert(
                header.clone(),
                Value::String(record.get(index).cloned().unwrap_or_default()),
            );
        }
        rows.push(Value::Object(map));
    }
    Ok(rows)
}

fn rows_from_ndjson_text(text: &str) -> Result<Vec<Value>, PipelineError> {
    let mut rows = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let value: Value = serde_json::from_str(line).map_err(|err| {
            PipelineError::new(
                "FW_NODE_TABLE_CONVERT",
                format!("NDJSON parse error on line {}: {err}", index + 1),
            )
        })?;
        rows.push(row_object(value));
    }
    Ok(rows)
}

pub(crate) fn encode_rows(
    rows: &[Value],
    columns: &[String],
    format: TableFormat,
) -> Result<Vec<u8>, PipelineError> {
    match format {
        TableFormat::Csv => Ok(encode_csv(rows, columns).into_bytes()),
        TableFormat::Json => serde_json::to_vec_pretty(rows)
            .map_err(|err| PipelineError::new("FW_NODE_TABLE_CONVERT", err.to_string())),
        TableFormat::Ndjson => {
            let mut out = String::new();
            for row in rows {
                out.push_str(
                    &serde_json::to_string(row).map_err(|err| {
                        PipelineError::new("FW_NODE_TABLE_CONVERT", err.to_string())
                    })?,
                );
                out.push('\n');
            }
            Ok(out.into_bytes())
        }
        TableFormat::Parquet => encode_parquet(rows, columns),
    }
}

fn rows_from_parquet_bytes(bytes: Vec<u8>) -> Result<Vec<Value>, PipelineError> {
    let mut temp = tempfile::NamedTempFile::new()
        .map_err(|err| PipelineError::new("FW_NODE_TABLE_CONVERT", err.to_string()))?;
    temp.write_all(&bytes)
        .map_err(|err| PipelineError::new("FW_NODE_TABLE_CONVERT", err.to_string()))?;
    let file = temp
        .reopen()
        .map_err(|err| PipelineError::new("FW_NODE_TABLE_CONVERT", err.to_string()))?;
    let reader = ParquetRecordBatchReaderBuilder::try_new(file)
        .map_err(|err| PipelineError::new("FW_NODE_TABLE_CONVERT", err.to_string()))?
        .build()
        .map_err(|err| PipelineError::new("FW_NODE_TABLE_CONVERT", err.to_string()))?;

    let mut rows = Vec::new();
    for batch in reader {
        let batch =
            batch.map_err(|err| PipelineError::new("FW_NODE_TABLE_CONVERT", err.to_string()))?;
        rows.extend(record_batch_to_rows(&batch)?);
    }
    Ok(rows)
}

fn encode_parquet(rows: &[Value], columns: &[String]) -> Result<Vec<u8>, PipelineError> {
    if columns.is_empty() {
        return Err(PipelineError::new(
            "FW_NODE_TABLE_CONVERT",
            "parquet output needs at least one column",
        ));
    }
    let batch = rows_to_record_batch(rows, columns)?;
    let props = WriterProperties::builder()
        .set_compression(Compression::SNAPPY)
        .build();
    let mut buffer = Vec::new();
    {
        let mut writer = ArrowWriter::try_new(&mut buffer, batch.schema(), Some(props))
            .map_err(|err| PipelineError::new("FW_NODE_TABLE_CONVERT", err.to_string()))?;
        writer
            .write(&batch)
            .map_err(|err| PipelineError::new("FW_NODE_TABLE_CONVERT", err.to_string()))?;
        writer
            .close()
            .map_err(|err| PipelineError::new("FW_NODE_TABLE_CONVERT", err.to_string()))?;
    }
    Ok(buffer)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ColumnKind {
    Boolean,
    Int64,
    Float64,
    Utf8,
}

fn rows_to_record_batch(rows: &[Value], columns: &[String]) -> Result<RecordBatch, PipelineError> {
    let kinds = columns
        .iter()
        .map(|column| infer_column_kind(rows, column))
        .collect::<Vec<_>>();
    let fields = columns
        .iter()
        .zip(kinds.iter())
        .map(|(column, kind)| Field::new(column, column_data_type(*kind), true))
        .collect::<Vec<_>>();
    let schema = Arc::new(Schema::new(fields));
    let arrays = columns
        .iter()
        .zip(kinds.iter())
        .map(|(column, kind)| build_arrow_array(rows, column, *kind))
        .collect::<Result<Vec<_>, _>>()?;
    RecordBatch::try_new(schema, arrays)
        .map_err(|err| PipelineError::new("FW_NODE_TABLE_CONVERT", err.to_string()))
}

fn column_data_type(kind: ColumnKind) -> DataType {
    match kind {
        ColumnKind::Boolean => DataType::Boolean,
        ColumnKind::Int64 => DataType::Int64,
        ColumnKind::Float64 => DataType::Float64,
        ColumnKind::Utf8 => DataType::Utf8,
    }
}

fn infer_column_kind(rows: &[Value], column: &str) -> ColumnKind {
    let mut kind: Option<ColumnKind> = None;
    for row in rows {
        let value = row
            .as_object()
            .and_then(|object| object.get(column))
            .unwrap_or(&Value::Null);
        let Some(next) = value_column_kind(value) else {
            continue;
        };
        kind = Some(match (kind, next) {
            (None, next) => next,
            (Some(ColumnKind::Int64), ColumnKind::Float64)
            | (Some(ColumnKind::Float64), ColumnKind::Int64)
            | (Some(ColumnKind::Float64), ColumnKind::Float64) => ColumnKind::Float64,
            (Some(existing), next) if existing == next => existing,
            _ => ColumnKind::Utf8,
        });
        if kind == Some(ColumnKind::Utf8) {
            break;
        }
    }
    kind.unwrap_or(ColumnKind::Utf8)
}

fn value_column_kind(value: &Value) -> Option<ColumnKind> {
    match value {
        Value::Null => None,
        Value::Bool(_) => Some(ColumnKind::Boolean),
        Value::Number(number) => {
            if number.as_i64().is_some() {
                Some(ColumnKind::Int64)
            } else if let Some(unsigned) = number.as_u64() {
                if i64::try_from(unsigned).is_ok() {
                    Some(ColumnKind::Int64)
                } else {
                    Some(ColumnKind::Float64)
                }
            } else {
                Some(ColumnKind::Float64)
            }
        }
        Value::String(_) | Value::Array(_) | Value::Object(_) => Some(ColumnKind::Utf8),
    }
}

fn build_arrow_array(
    rows: &[Value],
    column: &str,
    kind: ColumnKind,
) -> Result<ArrayRef, PipelineError> {
    match kind {
        ColumnKind::Boolean => {
            let mut builder = BooleanBuilder::with_capacity(rows.len());
            for row in rows {
                match row.as_object().and_then(|object| object.get(column)) {
                    Some(Value::Bool(value)) => builder.append_value(*value),
                    _ => builder.append_null(),
                }
            }
            Ok(Arc::new(builder.finish()) as ArrayRef)
        }
        ColumnKind::Int64 => {
            let mut builder = Int64Builder::with_capacity(rows.len());
            for row in rows {
                match row.as_object().and_then(|object| object.get(column)) {
                    Some(Value::Number(number)) => {
                        if let Some(value) = number.as_i64() {
                            builder.append_value(value);
                        } else if let Some(value) =
                            number.as_u64().and_then(|value| i64::try_from(value).ok())
                        {
                            builder.append_value(value);
                        } else {
                            builder.append_null();
                        }
                    }
                    _ => builder.append_null(),
                }
            }
            Ok(Arc::new(builder.finish()) as ArrayRef)
        }
        ColumnKind::Float64 => {
            let mut builder = Float64Builder::with_capacity(rows.len());
            for row in rows {
                match row.as_object().and_then(|object| object.get(column)) {
                    Some(Value::Number(number)) => {
                        if let Some(value) = number.as_f64() {
                            builder.append_value(value);
                        } else {
                            builder.append_null();
                        }
                    }
                    _ => builder.append_null(),
                }
            }
            Ok(Arc::new(builder.finish()) as ArrayRef)
        }
        ColumnKind::Utf8 => {
            let mut builder = StringBuilder::with_capacity(rows.len(), rows.len() * 16);
            for row in rows {
                match row.as_object().and_then(|object| object.get(column)) {
                    Some(Value::Null) | None => builder.append_null(),
                    Some(value) => builder.append_value(&csv_cell_value(value)),
                }
            }
            Ok(Arc::new(builder.finish()) as ArrayRef)
        }
    }
}

pub(crate) fn record_batch_to_rows(batch: &RecordBatch) -> Result<Vec<Value>, PipelineError> {
    let schema = batch.schema();
    let mut rows = Vec::with_capacity(batch.num_rows());
    for row_index in 0..batch.num_rows() {
        let mut row = Map::new();
        for (column_index, field) in schema.fields().iter().enumerate() {
            let array = batch.column(column_index);
            row.insert(
                field.name().clone(),
                arrow_value_to_json(array.as_ref(), row_index)?,
            );
        }
        rows.push(Value::Object(row));
    }
    Ok(rows)
}

fn arrow_value_to_json(array: &dyn Array, row: usize) -> Result<Value, PipelineError> {
    if array.is_null(row) {
        return Ok(Value::Null);
    }
    match array.data_type() {
        DataType::Boolean => Ok(json!(
            array
                .as_any()
                .downcast_ref::<BooleanArray>()
                .expect("boolean array")
                .value(row)
        )),
        DataType::Int8 => Ok(json!(
            array
                .as_any()
                .downcast_ref::<Int8Array>()
                .expect("int8 array")
                .value(row)
        )),
        DataType::Int16 => Ok(json!(
            array
                .as_any()
                .downcast_ref::<Int16Array>()
                .expect("int16 array")
                .value(row)
        )),
        DataType::Int32 => Ok(json!(
            array
                .as_any()
                .downcast_ref::<Int32Array>()
                .expect("int32 array")
                .value(row)
        )),
        DataType::Int64 => Ok(json!(
            array
                .as_any()
                .downcast_ref::<Int64Array>()
                .expect("int64 array")
                .value(row)
        )),
        DataType::UInt8 => Ok(json!(
            array
                .as_any()
                .downcast_ref::<UInt8Array>()
                .expect("uint8 array")
                .value(row)
        )),
        DataType::UInt16 => Ok(json!(
            array
                .as_any()
                .downcast_ref::<UInt16Array>()
                .expect("uint16 array")
                .value(row)
        )),
        DataType::UInt32 => Ok(json!(
            array
                .as_any()
                .downcast_ref::<UInt32Array>()
                .expect("uint32 array")
                .value(row)
        )),
        DataType::UInt64 => Ok(json!(
            array
                .as_any()
                .downcast_ref::<UInt64Array>()
                .expect("uint64 array")
                .value(row)
        )),
        DataType::Float32 => {
            let value = array
                .as_any()
                .downcast_ref::<Float32Array>()
                .expect("float32 array")
                .value(row);
            Ok(serde_json::Number::from_f64(value as f64)
                .map(Value::Number)
                .unwrap_or(Value::Null))
        }
        DataType::Float64 => {
            let value = array
                .as_any()
                .downcast_ref::<Float64Array>()
                .expect("float64 array")
                .value(row);
            Ok(serde_json::Number::from_f64(value)
                .map(Value::Number)
                .unwrap_or(Value::Null))
        }
        DataType::Utf8 => Ok(Value::String(
            array
                .as_any()
                .downcast_ref::<StringArray>()
                .expect("utf8 array")
                .value(row)
                .to_string(),
        )),
        DataType::LargeUtf8 => Ok(Value::String(
            array
                .as_any()
                .downcast_ref::<LargeStringArray>()
                .expect("large utf8 array")
                .value(row)
                .to_string(),
        )),
        _ => array_value_to_string(array, row)
            .map(Value::String)
            .map_err(|err| PipelineError::new("FW_NODE_TABLE_CONVERT", err.to_string())),
    }
}

pub(crate) fn collect_columns(rows: &[Value]) -> Vec<String> {
    let mut seen = HashSet::<String>::new();
    let mut columns = Vec::new();
    for row in rows {
        if let Value::Object(map) = row {
            for key in map.keys() {
                if seen.insert(key.clone()) {
                    columns.push(key.clone());
                }
            }
        }
    }
    columns
}

fn encode_csv(rows: &[Value], columns: &[String]) -> String {
    let mut out = String::new();
    write_csv_record(&mut out, columns.iter().map(String::as_str));
    for row in rows {
        let object = row.as_object();
        let cells = columns
            .iter()
            .map(|column| {
                object
                    .and_then(|map| map.get(column))
                    .map(csv_cell_value)
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>();
        write_csv_record(&mut out, cells.iter().map(String::as_str));
    }
    out
}

fn write_csv_record<'a, I>(out: &mut String, cells: I)
where
    I: IntoIterator<Item = &'a str>,
{
    let mut first = true;
    for cell in cells {
        if !first {
            out.push(',');
        }
        first = false;
        out.push_str(&escape_csv_cell(cell));
    }
    out.push('\n');
}

fn csv_cell_value(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Null => String::new(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Array(_) | Value::Object(_) => serde_json::to_string(value).unwrap_or_default(),
    }
}

fn escape_csv_cell(cell: &str) -> String {
    if cell.contains(',') || cell.contains('"') || cell.contains('\n') || cell.contains('\r') {
        format!("\"{}\"", cell.replace('"', "\"\""))
    } else {
        cell.to_string()
    }
}

fn parse_csv_records(text: &str) -> Result<Vec<Vec<String>>, PipelineError> {
    let mut records = Vec::<Vec<String>>::new();
    let mut record = Vec::<String>::new();
    let mut cell = String::new();
    let mut chars = text.chars().peekable();
    let mut in_quotes = false;
    let mut saw_any = false;

    while let Some(char) = chars.next() {
        saw_any = true;
        match char {
            '"' if in_quotes && chars.peek() == Some(&'"') => {
                cell.push('"');
                chars.next();
            }
            '"' => in_quotes = !in_quotes,
            ',' if !in_quotes => {
                record.push(std::mem::take(&mut cell));
            }
            '\n' if !in_quotes => {
                record.push(std::mem::take(&mut cell));
                records.push(std::mem::take(&mut record));
            }
            '\r' if !in_quotes => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                record.push(std::mem::take(&mut cell));
                records.push(std::mem::take(&mut record));
            }
            other => cell.push(other),
        }
    }

    if in_quotes {
        return Err(PipelineError::new(
            "FW_NODE_TABLE_CONVERT",
            "CSV has an unterminated quoted cell",
        ));
    }

    if saw_any && (!cell.is_empty() || !record.is_empty()) {
        record.push(cell);
        records.push(record);
    }

    Ok(records)
}

fn non_empty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn option_string(value: Option<String>) -> Value {
    value.map(Value::String).unwrap_or(Value::Null)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_quoted_csv_rows() {
        let rows = rows_from_csv_text("name,note\nAda,\"hello, world\"\nBob,\"a \"\"quote\"\"\"\n")
            .expect("csv rows");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0]["name"], "Ada");
        assert_eq!(rows[0]["note"], "hello, world");
        assert_eq!(rows[1]["note"], "a \"quote\"");
    }

    #[test]
    fn object_with_rows_is_table_source() {
        let rows = rows_from_json_value(json!({
            "rows": [
                { "id": 1, "title": "A" },
                { "id": 2, "title": "B" }
            ],
            "ignored": true
        }))
        .expect("json rows");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[1]["title"], "B");
    }

    #[test]
    fn encodes_csv_with_escaped_cells() {
        let rows = vec![json!({ "name": "Ada", "note": "hello, world", "score": 42 })];
        let csv = encode_csv(
            &rows,
            &["name".to_string(), "note".to_string(), "score".to_string()],
        );
        assert_eq!(csv, "name,note,score\nAda,\"hello, world\",42\n");
    }

    #[test]
    fn parquet_roundtrip_preserves_basic_types() {
        let rows = vec![
            json!({ "id": 1, "title": "A", "score": 1.5, "active": true }),
            json!({ "id": 2, "title": "B", "score": 2.0, "active": false }),
        ];
        let columns = collect_columns(&rows);
        let parquet = encode_parquet(&rows, &columns).expect("parquet encode");
        let decoded = rows_from_parquet_bytes(parquet).expect("parquet decode");
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0]["id"], 1);
        assert_eq!(decoded[0]["title"], "A");
        assert_eq!(decoded[0]["score"], 1.5);
        assert_eq!(decoded[1]["active"], false);
    }
}
