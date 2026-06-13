//! S3-like object operations for Zebflow FS.
//!
//! For general node authoring rules, read `src/pipeline/nodes/mod.rs`; for
//! FileRef IR and backend/lifecycle rules, read
//! `src/pipeline/nodes/basic/file_ref.rs`.
//!
//! `fs.put --from-key` accepts text-like JSON, legacy byte envelopes, and FileRef
//! metadata. FileRef values are read through the shared helper instead of treating
//! `ref` as a local filesystem path.

use std::sync::Arc;
use std::time::SystemTime;

use async_trait::async_trait;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use super::util::{metadata_scope, resolve_path};
use crate::pipeline::nodes::basic::file_ref::{is_file_ref, read_file_ref_bytes};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    model::{DslFlag, DslFlagKind, LayoutItem, NodeFieldDef, NodeFieldType, SelectOptionDef},
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;
use crate::zebfs::{
    LocalZebFs,
    model::{ZebFsEntry, ZebFsEntryKind, ZebFsStat},
};

pub const LIST_NODE_KIND: &str = "n.fs.list";
pub const HEAD_NODE_KIND: &str = "n.fs.head";
pub const GET_NODE_KIND: &str = "n.fs.get";
pub const PUT_NODE_KIND: &str = "n.fs.put";
pub const DELETE_NODE_KIND: &str = "n.fs.delete";
pub const COPY_NODE_KIND: &str = "n.fs.copy";
pub const MOVE_NODE_KIND: &str = "n.fs.move";
pub const MKDIR_NODE_KIND: &str = "n.fs.mkdir";

const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

#[derive(Debug, Clone, Copy)]
pub enum Operation {
    List,
    Head,
    Get,
    Put,
    Delete,
    Copy,
    Move,
    Mkdir,
}

impl Operation {
    fn kind(self) -> &'static str {
        match self {
            Self::List => LIST_NODE_KIND,
            Self::Head => HEAD_NODE_KIND,
            Self::Get => GET_NODE_KIND,
            Self::Put => PUT_NODE_KIND,
            Self::Delete => DELETE_NODE_KIND,
            Self::Copy => COPY_NODE_KIND,
            Self::Move => MOVE_NODE_KIND,
            Self::Mkdir => MKDIR_NODE_KIND,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::List => "list",
            Self::Head => "head",
            Self::Get => "get",
            Self::Put => "put",
            Self::Delete => "delete",
            Self::Copy => "copy",
            Self::Move => "move",
            Self::Mkdir => "mkdir",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub prefix: String,
    #[serde(default)]
    pub from: String,
    #[serde(default)]
    pub to: String,
    #[serde(default)]
    pub from_key: String,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub base64: Option<String>,
    #[serde(default)]
    pub encoding: String,
}

pub fn list_definition() -> NodeDefinition {
    object_definition(
        LIST_NODE_KIND,
        "FS List",
        "List immediate children under a Zebflow FS prefix. Output: `{ fs: { operation, path, count, entries } }`.",
        vec![
            scalar_flag(
                "--path",
                "path",
                "Prefix to list. Empty lists the project root.",
            ),
            scalar_flag("--prefix", "prefix", "Alias for --path."),
        ],
        vec![
            text_field(
                "path",
                "Path",
                "Prefix to list. Empty lists the project root.",
            ),
            text_field(
                "prefix",
                "Prefix",
                "Alias for Path; useful for S3-style wording.",
            ),
        ],
        vec![
            LayoutItem::Field("path".to_string()),
            LayoutItem::Field("prefix".to_string()),
        ],
    )
}

pub fn head_definition() -> NodeDefinition {
    object_definition(
        HEAD_NODE_KIND,
        "FS Head",
        "Read metadata for one Zebflow FS object or prefix without reading content. Output: `{ fs: { object } }`.",
        vec![scalar_flag("--path", "path", "Object or prefix path.")],
        vec![text_field("path", "Path", "Object or prefix path.")],
        vec![LayoutItem::Field("path".to_string())],
    )
}

pub fn get_definition() -> NodeDefinition {
    let mut def = object_definition(
        GET_NODE_KIND,
        "FS Get",
        "Read a Zebflow FS object. Default output is UTF-8 text; use `--encoding base64` for binary objects.",
        vec![
            scalar_flag("--path", "path", "Object path."),
            scalar_flag("--encoding", "encoding", "text or base64. Default: text."),
        ],
        vec![
            text_field("path", "Path", "Object path."),
            NodeFieldDef {
                name: "encoding".to_string(),
                label: "Encoding".to_string(),
                field_type: NodeFieldType::Select,
                default_value: Some(json!("text")),
                options: vec![
                    SelectOptionDef {
                        value: "text".to_string(),
                        label: "Text".to_string(),
                    },
                    SelectOptionDef {
                        value: "base64".to_string(),
                        label: "Base64".to_string(),
                    },
                ],
                ..Default::default()
            },
        ],
        vec![
            LayoutItem::Field("path".to_string()),
            LayoutItem::Field("encoding".to_string()),
        ],
    );
    def.output_schema = json!({
        "type": "object",
        "properties": {
            "fs": {
                "type": "object",
                "properties": {
                    "object": {
                        "type": "object",
                        "properties": {
                            "content": { "type": ["string", "null"] },
                            "base64": { "type": ["string", "null"] }
                        }
                    }
                }
            }
        }
    });
    def
}

pub fn put_definition() -> NodeDefinition {
    object_definition(
        PUT_NODE_KIND,
        "FS Put",
        "Write one Zebflow FS object from literal text, base64, FileRef, or a payload dot-path. Output: `{ fs: { object } }`.",
        vec![
            scalar_flag("--path", "path", "Destination object path."),
            scalar_flag(
                "--from-key",
                "from_key",
                "Dot-path in payload to write. FileRef values are read as file bytes.",
            ),
            scalar_flag("--text", "text", "Literal UTF-8 content."),
            scalar_flag("--base64", "base64", "Base64 encoded content."),
        ],
        vec![
            text_field("path", "Path", "Destination object path."),
            text_field(
                "from_key",
                "From Key",
                "Dot-path in payload to write. FileRef values are read as file bytes.",
            ),
            NodeFieldDef {
                name: "text".to_string(),
                label: "Text".to_string(),
                field_type: NodeFieldType::Textarea,
                rows: Some(6),
                help: Some(
                    "Literal UTF-8 content. Ignored when From Key or Base64 is set.".to_string(),
                ),
                ..Default::default()
            },
            NodeFieldDef {
                name: "base64".to_string(),
                label: "Base64".to_string(),
                field_type: NodeFieldType::Textarea,
                rows: Some(4),
                help: Some("Base64 encoded content. Ignored when From Key is set.".to_string()),
                ..Default::default()
            },
        ],
        vec![
            LayoutItem::Field("path".to_string()),
            LayoutItem::Field("from_key".to_string()),
            LayoutItem::Field("text".to_string()),
            LayoutItem::Field("base64".to_string()),
        ],
    )
}

pub fn delete_definition() -> NodeDefinition {
    object_definition(
        DELETE_NODE_KIND,
        "FS Delete",
        "Delete one Zebflow FS object or prefix tree. Delete is idempotent when the path is absent.",
        vec![scalar_flag(
            "--path",
            "path",
            "Object or prefix path to delete.",
        )],
        vec![text_field(
            "path",
            "Path",
            "Object or prefix path to delete.",
        )],
        vec![LayoutItem::Field("path".to_string())],
    )
}

pub fn copy_definition() -> NodeDefinition {
    copy_like_definition(COPY_NODE_KIND, "FS Copy", "Copy one Zebflow FS object.")
}

pub fn move_definition() -> NodeDefinition {
    copy_like_definition(
        MOVE_NODE_KIND,
        "FS Move",
        "Move one Zebflow FS object by copying it then deleting the source.",
    )
}

pub fn mkdir_definition() -> NodeDefinition {
    object_definition(
        MKDIR_NODE_KIND,
        "FS Mkdir",
        "Create a Zebflow FS prefix directory. Output: `{ fs: { object } }`.",
        vec![scalar_flag("--path", "path", "Prefix path to create.")],
        vec![text_field("path", "Path", "Prefix path to create.")],
        vec![LayoutItem::Field("path".to_string())],
    )
}

fn copy_like_definition(kind: &str, title: &str, description: &str) -> NodeDefinition {
    object_definition(
        kind,
        title,
        description,
        vec![
            scalar_flag("--from", "from", "Source object path."),
            scalar_flag("--to", "to", "Destination object path."),
        ],
        vec![
            text_field("from", "From", "Source object path."),
            text_field("to", "To", "Destination object path."),
        ],
        vec![
            LayoutItem::Field("from".to_string()),
            LayoutItem::Field("to".to_string()),
        ],
    )
}

fn object_definition(
    kind: &str,
    title: &str,
    description: &str,
    dsl_flags: Vec<DslFlag>,
    fields: Vec<NodeFieldDef>,
    layout: Vec<LayoutItem>,
) -> NodeDefinition {
    NodeDefinition {
        kind: kind.to_string(),
        title: title.to_string(),
        description: description.to_string(),
        input_schema: json!({"type": "object"}),
        output_schema: json!({
            "type": "object",
            "properties": {
                "fs": {
                    "type": "object",
                    "properties": {
                        "operation": { "type": "string" },
                        "path": { "type": "string" },
                        "object": { "type": "object" },
                        "entries": { "type": "array" }
                    }
                }
            }
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags,
        fields,
        layout,
        ai_tool: Default::default(),
        ..Default::default()
    }
}

fn scalar_flag(flag: &str, config_key: &str, description: &str) -> DslFlag {
    DslFlag {
        flag: flag.to_string(),
        config_key: config_key.to_string(),
        description: description.to_string(),
        kind: DslFlagKind::Scalar,
        required: false,
    }
}

fn text_field(name: &str, label: &str, help: &str) -> NodeFieldDef {
    NodeFieldDef {
        name: name.to_string(),
        label: label.to_string(),
        field_type: NodeFieldType::Text,
        help: Some(help.to_string()),
        ..Default::default()
    }
}

pub struct Node {
    config: Config,
    platform: Arc<PlatformService>,
    operation: Operation,
}

impl Node {
    pub fn new(
        config: Config,
        platform: Arc<PlatformService>,
        operation: Operation,
    ) -> Result<Self, PipelineError> {
        Ok(Self {
            config,
            platform,
            operation,
        })
    }
}

#[async_trait]
impl NodeHandler for Node {
    fn kind(&self) -> &'static str {
        self.operation.kind()
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
            .map_err(|err| PipelineError::new("FW_NODE_FS_OBJECT", err.to_string()))?;
        let zebfs = LocalZebFs::new(layout.files_dir);
        let op = self.operation;
        let payload = match op {
            Operation::List => {
                let path =
                    first_non_empty([self.config.path.as_str(), self.config.prefix.as_str()])
                        .unwrap_or("");
                let entries = zebfs
                    .list(path)
                    .map_err(|err| PipelineError::new("FW_NODE_FS_LIST", err.to_string()))?;
                json!({
                    "fs": {
                        "operation": op.label(),
                        "path": normalize_output_path(path),
                        "count": entries.len(),
                        "entries": entries
                            .iter()
                            .map(|entry| entry_json(owner, project, entry))
                            .collect::<Vec<_>>()
                    }
                })
            }
            Operation::Head => {
                let path = required(&self.config.path, "--path", "FW_NODE_FS_HEAD")?;
                let stat = zebfs
                    .head(path)
                    .map_err(|err| PipelineError::new("FW_NODE_FS_HEAD", err.to_string()))?;
                json!({
                    "fs": {
                        "operation": op.label(),
                        "path": stat.path,
                        "object": stat_json(owner, project, &stat)
                    }
                })
            }
            Operation::Get => {
                let path = required(&self.config.path, "--path", "FW_NODE_FS_GET")?;
                let object = zebfs
                    .get(path)
                    .map_err(|err| PipelineError::new("FW_NODE_FS_GET", err.to_string()))?;
                let mut object_out = stat_json(owner, project, &object.stat);
                let encoding = self.config.encoding.trim().to_ascii_lowercase();
                if encoding == "base64" {
                    object_out["base64"] = Value::String(
                        base64::engine::general_purpose::STANDARD.encode(&object.bytes),
                    );
                    object_out["content"] = Value::Null;
                } else {
                    object_out["content"] =
                        Value::String(String::from_utf8(object.bytes).map_err(|err| {
                            PipelineError::new(
                                "FW_NODE_FS_GET_UTF8",
                                format!("object is not UTF-8; use --encoding base64: {err}"),
                            )
                        })?);
                    object_out["base64"] = Value::Null;
                }
                json!({
                    "fs": {
                        "operation": op.label(),
                        "path": object.path,
                        "object": object_out
                    }
                })
            }
            Operation::Put => {
                let path = required(&self.config.path, "--path", "FW_NODE_FS_PUT")?;
                let bytes = self.resolve_put_bytes(owner, project, &input.payload)?;
                let stat = zebfs
                    .put(path, &bytes)
                    .map_err(|err| PipelineError::new("FW_NODE_FS_PUT", err.to_string()))?;
                json!({
                    "fs": {
                        "operation": op.label(),
                        "path": stat.path,
                        "object": stat_json(owner, project, &stat)
                    }
                })
            }
            Operation::Delete => {
                let path = required(&self.config.path, "--path", "FW_NODE_FS_DELETE")?;
                zebfs
                    .delete(path)
                    .map_err(|err| PipelineError::new("FW_NODE_FS_DELETE", err.to_string()))?;
                json!({
                    "fs": {
                        "operation": op.label(),
                        "path": normalize_output_path(path),
                        "deleted": true
                    }
                })
            }
            Operation::Copy | Operation::Move => {
                let from = required(&self.config.from, "--from", "FW_NODE_FS_COPY")?;
                let to = required(&self.config.to, "--to", "FW_NODE_FS_COPY")?;
                let stat = zebfs
                    .copy(from, to)
                    .map_err(|err| PipelineError::new("FW_NODE_FS_COPY", err.to_string()))?;
                if matches!(op, Operation::Move) {
                    zebfs
                        .delete(from)
                        .map_err(|err| PipelineError::new("FW_NODE_FS_MOVE", err.to_string()))?;
                }
                json!({
                    "fs": {
                        "operation": op.label(),
                        "path": stat.path,
                        "source_path": normalize_output_path(from),
                        "object": stat_json(owner, project, &stat)
                    }
                })
            }
            Operation::Mkdir => {
                let path = required(&self.config.path, "--path", "FW_NODE_FS_MKDIR")?;
                let stat = zebfs
                    .create_prefix(path)
                    .map_err(|err| PipelineError::new("FW_NODE_FS_MKDIR", err.to_string()))?;
                json!({
                    "fs": {
                        "operation": op.label(),
                        "path": stat.path,
                        "object": stat_json(owner, project, &stat)
                    }
                })
            }
        };

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload,
            trace: vec![format!("node_kind={} operation={}", op.kind(), op.label())],
        })
    }
}

impl Node {
    fn resolve_put_bytes(
        &self,
        owner: &str,
        project: &str,
        payload: &Value,
    ) -> Result<Vec<u8>, PipelineError> {
        let from_key = self.config.from_key.trim();
        if !from_key.is_empty() {
            let value = resolve_path(payload, from_key).ok_or_else(|| {
                PipelineError::new(
                    "FW_NODE_FS_PUT_SOURCE",
                    format!("payload key '{from_key}' was not found"),
                )
            })?;
            if is_file_ref(value) {
                return read_file_ref_bytes(&self.platform, owner, project, value);
            }
            return value_to_bytes(value);
        }
        if let Some(encoded) = self
            .config
            .base64
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .map_err(|err| PipelineError::new("FW_NODE_FS_PUT_BASE64", err.to_string()));
        }
        if let Some(text) = &self.config.text {
            return Ok(text.as_bytes().to_vec());
        }
        Err(PipelineError::new(
            "FW_NODE_FS_PUT_SOURCE",
            "set one of --from-key, --base64, or --text",
        ))
    }
}

fn value_to_bytes(value: &Value) -> Result<Vec<u8>, PipelineError> {
    if let Some(text) = value.as_str() {
        return Ok(text.as_bytes().to_vec());
    }
    if let Some(encoded) = value.get("__zf_bytes").and_then(Value::as_str) {
        return base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|err| PipelineError::new("FW_NODE_FS_PUT_BASE64", err.to_string()));
    }
    if value.get("filename").is_some()
        && value.get("content_type").is_some()
        && value.get("size").is_some()
        && let Some(encoded) = value.get("data").and_then(Value::as_str)
    {
        return base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|err| PipelineError::new("FW_NODE_FS_PUT_BASE64", err.to_string()));
    }
    serde_json::to_vec(value)
        .map_err(|err| PipelineError::new("FW_NODE_FS_PUT_JSON", err.to_string()))
}

fn required<'a>(value: &'a str, flag: &str, code: &'static str) -> Result<&'a str, PipelineError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(PipelineError::new(code, format!("{flag} is required")));
    }
    Ok(trimmed)
}

fn first_non_empty<'a>(items: impl IntoIterator<Item = &'a str>) -> Option<&'a str> {
    items
        .into_iter()
        .map(str::trim)
        .find(|value| !value.is_empty())
}

fn normalize_output_path(path: &str) -> String {
    path.trim().trim_start_matches('/').replace('\\', "/")
}

fn entry_json(owner: &str, project: &str, entry: &ZebFsEntry) -> Value {
    json!({
        "name": entry.name,
        "path": entry.path,
        "url": format!("/fs/{owner}/{project}/{}", entry.path),
        "size": entry.size,
        "modified": system_time_to_rfc3339(entry.modified),
        "kind": entry_kind(entry.kind),
        "content_type": content_type_for_path(&entry.path),
    })
}

fn stat_json(owner: &str, project: &str, stat: &ZebFsStat) -> Value {
    json!({
        "path": stat.path,
        "url": format!("/fs/{owner}/{project}/{}", stat.path),
        "size": stat.size,
        "modified": system_time_to_rfc3339(stat.modified),
        "kind": entry_kind(stat.kind),
        "content_type": content_type_for_path(&stat.path),
    })
}

fn entry_kind(kind: ZebFsEntryKind) -> &'static str {
    match kind {
        ZebFsEntryKind::Object => "object",
        ZebFsEntryKind::Prefix => "prefix",
    }
}

fn system_time_to_rfc3339(time: Option<SystemTime>) -> Option<String> {
    time.map(|value| chrono::DateTime::<chrono::Utc>::from(value).to_rfc3339())
}

fn content_type_for_path(path: &str) -> &'static str {
    let lower = path.rsplit('/').next().unwrap_or(path).to_ascii_lowercase();
    match lower.rsplit('.').next().unwrap_or("") {
        "txt" | "md" | "log" => "text/plain; charset=utf-8",
        "html" | "htm" => "text/html; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "csv" => "text/csv; charset=utf-8",
        "json" => "application/json",
        "ndjson" => "application/x-ndjson",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "pdf" => "application/pdf",
        "parquet" => "application/vnd.apache.parquet",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use base64::Engine as _;
    use serde_json::json;

    use super::value_to_bytes;

    #[test]
    fn put_decodes_legacy_webhook_file_object() {
        let original = b"{\"type\":\"FeatureCollection\",\"features\":[]}";
        let value = json!({
            "filename": "data.geojson",
            "content_type": "application/geo+json",
            "size": original.len(),
            "data": base64::engine::general_purpose::STANDARD.encode(original),
        });

        let bytes = value_to_bytes(&value).expect("decode webhook file object");
        assert_eq!(bytes, original);
    }
}
