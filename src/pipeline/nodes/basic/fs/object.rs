//! S3-like object operations for Zebflow FS.
//!
//! For general node authoring rules, read `src/pipeline/nodes/mod.rs`; for
//! FileRef IR and backend/lifecycle rules, read
//! `src/pipeline/nodes/shared/file_ref.rs`.
//!
//! `fs.put --source-key` accepts text-like JSON, legacy byte envelopes, and FileRef
//! metadata. FileRef values are read through the shared helper instead of treating
//! `ref` as a local filesystem path.
//!
//! `fs.put`, `fs.copy` and `fs.move` answer the stored file at `fs.object` as a
//! **bare durable FileRef** — the eleven contract fields and nothing else. The
//! store path is `fs.object.ref` (and the envelope's `fs.path`); a URL is not a
//! node's business. `fs.list` entries and `fs.head` describe objects the node
//! did not write, so they stay plain stats.

use std::sync::Arc;
use std::time::SystemTime;

use async_trait::async_trait;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::pipeline::nodes::shared::project_store::{OnConflict, on_conflict_flag, open_store, store_fields, store_flag, target_key};
use crate::pipeline::nodes::shared::util::{metadata_scope, resolve_path};
use crate::pipeline::model::NodeCapability;
use crate::pipeline::nodes::shared::file_ref::{
    is_file_ref, read_file_ref_bytes,
};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    model::{DslFlag, DslFlagKind, LayoutItem, NodeExample, NodeFieldDef, NodeFieldType, SelectOptionDef},
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;
use crate::zebfs::model::{ZebFsEntry, ZebFsEntryKind, ZebFsStat};

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
    pub from: String,
    #[serde(default)]
    pub source_key: String,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub base64: Option<String>,
    #[serde(default)]
    pub encoding: String,
    /// Destination folder for put/copy/move (put default: `files`; copy and
    /// move default: the source's folder).
    #[serde(default)]
    pub folder: String,
    /// Destination name for put/copy/move (copy and move default: the source's name).
    #[serde(default)]
    pub filename: Option<String>,
    /// The store this operation works in; writers save it explicitly at registration.
    #[serde(default)]
    pub store: Option<String>,
    /// `overwrite`, `skip` or `error` for put/copy/move (default: error).
    #[serde(default)]
    pub on_conflict: Option<String>,
}

pub fn list_definition() -> NodeDefinition {
    let mut def = object_definition(
        LIST_NODE_KIND,
        "FS List",
        "List the immediate children of a folder in the project's file store (the same store `fs.save` writes to). \
         Needs `--path` (empty = project root). Adds `fs: { operation, path, count, entries: [{ path, kind, size, modified }] }` to the payload — \
         the next node reads `input.fs.entries`, not `input.entries`. One level only; it does not recurse.",
        vec![
            scalar_flag(
                "--path",
                "path",
                "Prefix to list. Empty lists the project root.",
            ),
        ],
        vec![
            text_field(
                "path",
                "Path",
                "Prefix to list. Empty lists the project root.",
            ),
        ],
        vec![
            LayoutItem::Field("path".to_string()),
        ],
    );
    def.examples = vec![example(
        "List a folder",
        "fs.list --path uploads",
        json!({ "fs": { "operation": "list", "path": "uploads", "count": 1, "entries": [{ "path": "uploads/a1.jpg", "kind": "object", "size": 1723, "modified": "2026-09-13T04:00:00Z" }] } }),
    )];
    def
}

pub fn head_definition() -> NodeDefinition {
    let mut def = object_definition(
        HEAD_NODE_KIND,
        "FS Head",
        "Read one file's metadata (size, kind, modified, url, content_type) without reading its bytes. Needs `--path`. \
         Adds `fs: { operation, path, object }` to the payload. Use it to check that a file exists before `fs.get`; \
         a missing path fails the node, it does not return null.",
        vec![scalar_flag("--path", "path", "Object or prefix path.")],
        vec![text_field("path", "Path", "Object or prefix path.")],
        vec![LayoutItem::Field("path".to_string())],
    );
    def.examples = vec![example(
        "Stat a file",
        "fs.head --path uploads/a1.jpg",
        json!({ "fs": { "operation": "head", "path": "uploads/a1.jpg", "object": { "path": "uploads/a1.jpg", "size": 1723, "modified": "2026-09-13T04:00:00Z", "kind": "object", "content_type": "image/jpeg" } } }),
    )];
    def
}

pub fn get_definition() -> NodeDefinition {
    let mut def = object_definition(
        GET_NODE_KIND,
        "FS Get",
        "Read one file's content from the project's file store. Needs `--path`; `--encoding text` (default) puts UTF-8 in `object.content`, \
         `--encoding base64` puts bytes in `object.base64`. Adds `fs: { operation, path, object: { content | base64, size, … } }` to the payload — \
         the text is `input.fs.object.content`. A binary file read as text fails with FW_NODE_FS_GET_UTF8; use base64.",
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
                help: Some(
                    "Decode object bytes as UTF-8 text or return base64 for binary-safe payloads."
                        .to_string(),
                ),
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
    def.examples = vec![example(
        "Read a text file",
        "fs.get --path docs/notes.md",
        json!({ "fs": { "operation": "get", "path": "docs/notes.md", "object": { "path": "docs/notes.md", "content": "# Notes\n…", "base64": null, "size": 42, "kind": "object" } } }),
    )];
    def
}

pub fn put_definition() -> NodeDefinition {
    let mut def = object_definition(
        PUT_NODE_KIND,
        "FS Put",
        "Write one file into the project's file store. Needs `--filename` (in `--folder`, default `files`) or an exact `--path`, and one source: \
         `--text` (literal or {{ expr }}), `--base64`, or `--source-key <dot.path>` (a string, a FileRef, or JSON in the payload). \
         Adds `fs: { operation, path, object }` to the payload, \
         where `object` is a bare durable FileRef (`ref`, `filename`, `mime`, `kind`, `size`, `sha256`, `origin: fs.put`, …) and nothing else. \
         For a browser upload use `fs.save`, not this; `fs.put` is for content the pipeline already has.",
        destination_flags("Destination folder (default: files).", "Destination name.")
            .into_iter()
            .chain([
            scalar_flag(
                "--source-key",
                "source_key",
                "Dot-path in payload to write. FileRef values are read as file bytes.",
            ),
            scalar_flag("--text", "text", "Literal UTF-8 content."),
            scalar_flag("--base64", "base64", "Base64 encoded content."),
        ])
            .collect(),
        destination_fields("Destination folder (default: files).", "Destination name.")
            .into_iter()
            .chain([
            text_field(
                "source_key",
                "Source Key",
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
        ])
            .collect(),
        vec![
            LayoutItem::Field("folder".to_string()),
            LayoutItem::Field("filename".to_string()),
            LayoutItem::Field("path".to_string()),
            LayoutItem::Field("on_conflict".to_string()),
            LayoutItem::Field("source_key".to_string()),
            LayoutItem::Field("text".to_string()),
            LayoutItem::Field("base64".to_string()),
        ],
    );
    def.examples = vec![example(
        "Write a generated file",
        "fs.put --folder exports --filename report.json --source-key report",
        json!({ "report": { "total": 3 }, "fs": { "operation": "put", "path": "exports/report.json", "object": { "__zf_type": "file_ref", "backend": "zebfs", "store": "local", "ref": "exports/report.json", "filename": "report.json", "mime": "application/json", "kind": "json", "size": 812, "sha256": "sha256:…", "lifecycle": "durable", "origin": "fs.put", "trust": "generated" } } }),
    )];
    def
}

pub fn delete_definition() -> NodeDefinition {
    let mut def = object_definition(
        DELETE_NODE_KIND,
        "FS Delete",
        "Delete one file, or a whole folder tree, from the project's file store. Needs `--path`. Adds \
         `fs: { operation: \"delete\", path, deleted: true }` to the payload; deleting a path that is already gone succeeds. \
         There is no confirmation and no trash — a folder path removes everything under it.",
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
    );
    def.examples = vec![example(
        "Delete an upload",
        "fs.delete --path \"{{ input.body.path }}\"",
        json!({ "fs": { "operation": "delete", "path": "uploads/a1.jpg", "deleted": true } }),
    )];
    def
}

pub fn copy_definition() -> NodeDefinition {
    let mut def = copy_like_definition(
        COPY_NODE_KIND,
        "FS Copy",
        "Copy one file inside the project's file store. Needs `--from` and a destination: `--folder` (keeps the name), \
         `--filename`, or an exact `--path`. Adds `fs: { operation: \"copy\", path, object }` to the payload, where `object` is a durable FileRef for the copy \
         (`origin: fs.copy`). An existing destination is an error unless `--on-conflict` says otherwise. \
         A copy is private like any object until the owner exposes its folder in Studio → Files.",
    );
    def.examples = vec![example(
        "Keep a copy of an upload",
        "fs.copy --from \"{{ input.saved.ref }}\" --folder images",
        json!({ "fs": { "operation": "copy", "path": "images/a1.jpg", "object": { "__zf_type": "file_ref", "backend": "zebfs", "store": "local", "ref": "images/a1.jpg", "filename": "a1.jpg", "mime": "image/jpeg", "kind": "image", "size": 1723, "sha256": "sha256:…", "lifecycle": "durable", "origin": "fs.copy", "trust": "generated" } } }),
    )];
    def
}

pub fn move_definition() -> NodeDefinition {
    let mut def = copy_like_definition(
        MOVE_NODE_KIND,
        "FS Move",
        "Move one file inside the project's file store: copy to the destination (`--folder`, `--filename` or `--path`), then delete `--from`. \
         Adds `fs: { operation: \"move\", path, object }` to the payload, where `object` is a durable FileRef at the \
         new path (`origin: fs.move`). Any FileRef still pointing at `--from` \
         is now dangling — update the row that stored it.",
    );
    def.examples = vec![example(
        "Archive a processed file",
        "fs.move --from \"{{ input.fs.path }}\" --folder archive/inbox",
        json!({ "fs": { "operation": "move", "path": "archive/inbox/a1.csv", "object": { "__zf_type": "file_ref", "backend": "zebfs", "store": "local", "ref": "archive/inbox/a1.csv", "filename": "a1.csv", "mime": "text/csv", "kind": "csv", "size": 9021, "sha256": "sha256:…", "lifecycle": "durable", "origin": "fs.move", "trust": "generated" } } }),
    )];
    def
}

pub fn mkdir_definition() -> NodeDefinition {
    let mut def = object_definition(
        MKDIR_NODE_KIND,
        "FS Mkdir",
        "Create a folder in the project's file store. Needs `--path`. Adds `fs: { operation: \"mkdir\", path, object }` to the payload. \
         Rarely needed: `fs.put`, `fs.save` and `fs.copy` create the folders they write into; use this only for a folder that \
         must exist before anything is in it (a listing page, an upload target).",
        vec![scalar_flag("--path", "path", "Prefix path to create.")],
        vec![text_field("path", "Path", "Prefix path to create.")],
        vec![LayoutItem::Field("path".to_string())],
    );
    def.examples = vec![example(
        "Create an upload folder",
        "fs.mkdir --path uploads/2026",
        json!({ "fs": { "operation": "mkdir", "path": "uploads/2026", "object": { "path": "uploads/2026", "kind": "prefix" } } }),
    )];
    def
}

fn example(title: &str, dsl: &str, output: Value) -> NodeExample {
    NodeExample::dsl(title, dsl).output(output)
}

fn copy_like_definition(kind: &str, title: &str, description: &str) -> NodeDefinition {
    object_definition(
        kind,
        title,
        description,
        [scalar_flag("--from", "from", "Source object path.")]
            .into_iter()
            .chain(destination_flags("Destination folder (default: the source's folder).", "Destination name (default: the source's name)."))
            .collect(),
        [text_field("from", "From", "Source object path.")]
            .into_iter()
            .chain(destination_fields("Destination folder (default: the source's folder).", "Destination name (default: the source's name)."))
            .collect(),
        vec![
            LayoutItem::Field("from".to_string()),
            LayoutItem::Field("folder".to_string()),
            LayoutItem::Field("filename".to_string()),
            LayoutItem::Field("path".to_string()),
            LayoutItem::Field("on_conflict".to_string()),
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
        capabilities: vec![NodeCapability::Filesystem],
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
                        "path": { "type": "string", "description": "The object path the operation answered for" },
                        "object": { "type": "object", "description": "put, copy, move: a bare durable FileRef (the eleven contract fields). head, get, mkdir: a plain stat." },
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
        dsl_flags: dsl_flags.into_iter().chain([store_flag()]).collect(),
        fields: fields
            .into_iter()
            .chain(store_fields(OnConflict::Error).into_iter().filter(|field| field.name == "store"))
            .collect(),
        layout: layout.into_iter().chain([LayoutItem::Field("store".to_string())]).collect(),
        ai_tool: Default::default(),
        ..Default::default()
    }
}

/// `--folder`, `--filename`, `--path` and `--on-conflict` for the writers.
fn destination_flags(folder_help: &str, filename_help: &str) -> Vec<DslFlag> {
    vec![
        scalar_flag("--folder", "folder", folder_help),
        scalar_flag("--filename", "filename", filename_help),
        scalar_flag("--path", "path", "Exact destination key; overrides --folder and --filename."),
        on_conflict_flag(OnConflict::Error),
    ]
}

fn destination_fields(folder_help: &str, filename_help: &str) -> Vec<NodeFieldDef> {
    vec![
        text_field("folder", "Folder", folder_help),
        text_field("filename", "Filename", filename_help),
        text_field("path", "Path", "Exact destination key; overrides folder and filename."),
    ]
    .into_iter()
    .chain(store_fields(OnConflict::Error).into_iter().filter(|field| field.name == "on_conflict"))
    .collect()
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
        let store = open_store(&self.platform, owner, project, self.config.store.as_deref())?;
        let zebfs = &store.fs;
        let op = self.operation;
        let payload = match op {
            Operation::List => {
                let path = self.config.path.trim();
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
                            .map(entry_json)
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
                        "object": stat_json(&stat)
                    }
                })
            }
            Operation::Get => {
                let path = required(&self.config.path, "--path", "FW_NODE_FS_GET")?;
                let object = zebfs
                    .get(path)
                    .map_err(|err| PipelineError::new("FW_NODE_FS_GET", err.to_string()))?;
                let mut object_out = stat_json(&object.stat);
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
                let filename = self.config.filename.as_deref().map(str::trim).unwrap_or("");
                if self.config.path.trim().is_empty() && filename.is_empty() {
                    return Err(PipelineError::new(
                        "FW_NODE_FS_PUT",
                        "fs.put needs --filename (in --folder, default files) or an exact --path",
                    ));
                }
                let folder = if self.config.folder.trim().is_empty() { "files" } else { self.config.folder.trim() };
                let path = target_key(Some(self.config.path.as_str()), folder, filename, "FW_NODE_FS_PUT")?;
                let bytes = self.resolve_put_bytes(owner, project, &input.payload)?;
                let bytes = if self.on_conflict("FW_NODE_FS_PUT")?.allows(zebfs, &path, "FW_NODE_FS_PUT")? {
                    zebfs
                        .put(&path, &bytes)
                        .map_err(|err| PipelineError::new("FW_NODE_FS_PUT", err.to_string()))?;
                    bytes
                } else {
                    zebfs.get(&path).map_err(|err| PipelineError::new("FW_NODE_FS_PUT", err.to_string()))?.bytes
                };
                let leaf = path.rsplit('/').next().unwrap_or(&path).to_string();
                let object = store.file_ref(&path, &leaf, content_type_for_path(&path), &bytes, "fs.put", "generated");
                let stat = zebfs.head(&path).map_err(|err| PipelineError::new("FW_NODE_FS_PUT", err.to_string()))?;
                json!({
                    "fs": {
                        "operation": op.label(),
                        "path": stat.path,
                        "object": object
                    }
                })
            }
            Operation::Delete => {
                let path = required(&self.config.path, "--path", "FW_NODE_FS_DELETE")?;
                zebfs
                    .delete(path)
                    .map_err(|err| PipelineError::new("FW_NODE_FS_DELETE", err.to_string()))?;
                // A deleted path takes its exposure rules with it, so a new
                // object of the same name starts private.
                crate::platform::services::zebfs_acl::forget(&layout.data_store_dir(), path)
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
                let from_norm = normalize_output_path(from);
                let (source_folder, source_name) = match from_norm.rsplit_once('/') {
                    Some((folder, name)) => (folder.to_string(), name.to_string()),
                    None => (String::new(), from_norm.clone()),
                };
                let folder = if self.config.folder.trim().is_empty() { source_folder.as_str() } else { self.config.folder.trim() };
                let filename = self
                    .config
                    .filename
                    .as_deref()
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .unwrap_or(&source_name);
                let to = target_key(Some(self.config.path.as_str()), folder, filename, "FW_NODE_FS_COPY")?;
                if to == from_norm {
                    return Err(PipelineError::new("FW_NODE_FS_COPY", "the destination is the source; set --folder, --filename or --path"));
                }
                if !self.on_conflict("FW_NODE_FS_COPY")?.allows(zebfs, &to, "FW_NODE_FS_COPY")? {
                    let bytes = zebfs.get(&to).map_err(|err| PipelineError::new("FW_NODE_FS_COPY", err.to_string()))?.bytes;
                    let leaf = to.rsplit('/').next().unwrap_or(&to).to_string();
                    let object = store.file_ref(&to, &leaf, content_type_for_path(&to), &bytes, "fs.copy", "generated");
                    return Ok(merged_output(op, &input.payload, json!({
                        "operation": op.label(), "path": to, "source_path": from_norm, "object": object, "skipped": true
                    })));
                }
                let stat = zebfs
                    .copy(from, &to)
                    .map_err(|err| PipelineError::new("FW_NODE_FS_COPY", err.to_string()))?;
                if matches!(op, Operation::Move) {
                    zebfs
                        .delete(from)
                        .map_err(|err| PipelineError::new("FW_NODE_FS_MOVE", err.to_string()))?;
                    crate::platform::services::zebfs_acl::forget(&layout.data_store_dir(), from)
                        .map_err(|err| PipelineError::new("FW_NODE_FS_MOVE", err.to_string()))?;
                }
                // A copied object is a stored file this node answers for, so
                // it answers a bare FileRef (the bytes are read once for the
                // digest); a copied prefix stays the plain stat it was.
                let object = if matches!(stat.kind, ZebFsEntryKind::Object) {
                    let origin = if matches!(op, Operation::Move) { "fs.move" } else { "fs.copy" };
                    let bytes = zebfs
                        .get(&stat.path)
                        .map_err(|err| PipelineError::new("FW_NODE_FS_COPY", err.to_string()))?
                        .bytes;
                    let leaf = stat.path.rsplit('/').next().unwrap_or(&stat.path).to_string();
                    store.file_ref(&stat.path, &leaf, content_type_for_path(&stat.path), &bytes, origin, "generated")
                } else {
                    stat_json(&stat)
                };
                json!({
                    "fs": {
                        "operation": op.label(),
                        "path": stat.path,
                        "source_path": normalize_output_path(from),
                        "object": object
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
                        "object": stat_json(&stat)
                    }
                })
            }
        };

        let fs = payload.get("fs").cloned().unwrap_or(Value::Null);
        Ok(merged_output(op, &input.payload, fs))
    }
}

/// The operation's answer added to the payload under `fs`, the rest kept
/// (`docs/contracts/node-conventions.md` §5).
fn merged_output(op: Operation, input: &Value, fs: Value) -> NodeExecutionOutput {
    let mut payload = match input {
        Value::Object(map) => map.clone(),
        _ => serde_json::Map::new(),
    };
    payload.insert("fs".to_string(), fs);
    NodeExecutionOutput {
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        payload: Value::Object(payload),
        trace: vec![format!("node_kind={} operation={}", op.kind(), op.label())],
    }
}

impl Node {
    fn on_conflict(&self, code: &'static str) -> Result<OnConflict, PipelineError> {
        OnConflict::parse(self.config.on_conflict.as_deref(), OnConflict::Error, code)
    }

    fn resolve_put_bytes(
        &self,
        owner: &str,
        project: &str,
        payload: &Value,
    ) -> Result<Vec<u8>, PipelineError> {
        let source_key = self.config.source_key.trim();
        if !source_key.is_empty() {
            let value = resolve_path(payload, source_key).ok_or_else(|| {
                PipelineError::new(
                    "FW_NODE_FS_PUT_SOURCE",
                    format!("payload key '{source_key}' was not found"),
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
            "set one of --source-key, --base64, or --text",
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

fn normalize_output_path(path: &str) -> String {
    path.trim().trim_start_matches('/').replace('\\', "/")
}

fn entry_json(entry: &ZebFsEntry) -> Value {
    json!({
        "name": entry.name,
        "path": entry.path,
        "size": entry.size,
        "modified": system_time_to_rfc3339(entry.modified),
        "kind": entry_kind(entry.kind),
        "content_type": content_type_for_path(&entry.path),
    })
}

fn stat_json(stat: &ZebFsStat) -> Value {
    json!({
        "path": stat.path,
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
