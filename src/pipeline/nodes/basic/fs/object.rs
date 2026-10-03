//! Files and folders in a project store, as they are: `fs.file.get`,
//! `fs.file.head`, `fs.file.delete`, `fs.file.copy`, `fs.file.move`,
//! `fs.folder.list`, `fs.folder.create`. (`fs.file.put` is its own node,
//! `put.rs`.)
//!
//! Every subject is explicit (`node-conventions.md` §2): a file node reads the
//! file `--from` names — a FileRef or an upload, read from the store it names,
//! or a store key in the node's own `--store` — and `fs.folder.list` the
//! folder `--from` names. `fs.folder.create` makes `--folder`.
//!
//! Each answers one key, its noun, and keeps the rest of the payload:
//!
//! ```text
//! fs.file.get      → file:   { path, size, modified, kind, content_type, content, base64 }
//! fs.file.head     → file:   { path, size, modified, kind, content_type }
//! fs.file.delete   → file:   { ref, deleted: true }   (+ recursive: true for a folder)
//! fs.file.copy     → file:   the durable FileRef of the copy
//! fs.file.move     → file:   the durable FileRef at the new key
//! fs.folder.list   → folder: { path, items, count }
//! fs.folder.create → folder: { path, created }
//! ```

use std::sync::Arc;
use std::time::SystemTime;

use async_trait::async_trait;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::pipeline::model::NodeCapability;
use crate::pipeline::nodes::shared::limits::choice;
use crate::pipeline::nodes::shared::project_store::{
    NodeStore, OnConflict, on_conflict_flag, open_from, open_store, read_capped, store_fields, store_flag, target_key,
};
use crate::pipeline::nodes::shared::util::{metadata_scope, with_answer};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    model::{DslFlag, DslFlagKind, LayoutItem, NodeExample, NodeFieldDef, NodeFieldType, SelectOptionDef},
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;
use crate::zebfs::model::{ZebFsEntry, ZebFsEntryKind, ZebFsStat};

pub const LIST_NODE_KIND: &str = "fs.folder.list";
pub const HEAD_NODE_KIND: &str = "fs.file.head";
pub const GET_NODE_KIND: &str = "fs.file.get";
pub const DELETE_NODE_KIND: &str = "fs.file.delete";
pub const COPY_NODE_KIND: &str = "fs.file.copy";
pub const MOVE_NODE_KIND: &str = "fs.file.move";
pub const MKDIR_NODE_KIND: &str = "fs.folder.create";

const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

/// `fs.file.get --encoding text` met bytes that are not UTF-8.
const GET_UTF8_CODE: &str = "FW_NODE_FS_FILE_GET_UTF8";
/// `fs.file.delete --from` names a folder that holds files, without `--recursive`.
const DELETE_FOLDER_CODE: &str = "FW_NODE_FS_FILE_DELETE_FOLDER";
const ENCODINGS: &[&str] = &["text", "base64"];
const ON_CONFLICT_WORDS: &[&str] = &["error", "skip", "overwrite"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operation {
    List,
    Head,
    Get,
    Delete,
    Copy,
    Move,
    Mkdir,
}

impl Operation {
    pub fn for_kind(kind: &str) -> Option<Self> {
        Some(match kind {
            LIST_NODE_KIND => Self::List,
            HEAD_NODE_KIND => Self::Head,
            GET_NODE_KIND => Self::Get,
            DELETE_NODE_KIND => Self::Delete,
            COPY_NODE_KIND => Self::Copy,
            MOVE_NODE_KIND => Self::Move,
            MKDIR_NODE_KIND => Self::Mkdir,
            _ => return None,
        })
    }

    fn kind(self) -> &'static str {
        match self {
            Self::List => LIST_NODE_KIND,
            Self::Head => HEAD_NODE_KIND,
            Self::Get => GET_NODE_KIND,
            Self::Delete => DELETE_NODE_KIND,
            Self::Copy => COPY_NODE_KIND,
            Self::Move => MOVE_NODE_KIND,
            Self::Mkdir => MKDIR_NODE_KIND,
        }
    }

    /// The store and the world failed.
    pub fn code(self) -> &'static str {
        match self {
            Self::List => "FW_NODE_FS_FOLDER_LIST",
            Self::Head => "FW_NODE_FS_FILE_HEAD",
            Self::Get => "FW_NODE_FS_FILE_GET",
            Self::Delete => "FW_NODE_FS_FILE_DELETE",
            Self::Copy => "FW_NODE_FS_FILE_COPY",
            Self::Move => "FW_NODE_FS_FILE_MOVE",
            Self::Mkdir => "FW_NODE_FS_FOLDER_CREATE",
        }
    }

    /// A flag the author set wrong.
    pub fn config_code(self) -> &'static str {
        match self {
            Self::List => "FW_NODE_FS_FOLDER_LIST_CONFIG",
            Self::Head => "FW_NODE_FS_FILE_HEAD_CONFIG",
            Self::Get => "FW_NODE_FS_FILE_GET_CONFIG",
            Self::Delete => "FW_NODE_FS_FILE_DELETE_CONFIG",
            Self::Copy => "FW_NODE_FS_FILE_COPY_CONFIG",
            Self::Move => "FW_NODE_FS_FILE_MOVE_CONFIG",
            Self::Mkdir => "FW_NODE_FS_FOLDER_CREATE_CONFIG",
        }
    }

    /// `--from` (or `fs.folder.create --folder`) is missing or names nothing usable.
    pub fn source_code(self) -> &'static str {
        match self {
            Self::List => "FW_NODE_FS_FOLDER_LIST_SOURCE",
            Self::Head => "FW_NODE_FS_FILE_HEAD_SOURCE",
            Self::Get => "FW_NODE_FS_FILE_GET_SOURCE",
            Self::Delete => "FW_NODE_FS_FILE_DELETE_SOURCE",
            Self::Copy => "FW_NODE_FS_FILE_COPY_SOURCE",
            Self::Move => "FW_NODE_FS_FILE_MOVE_SOURCE",
            Self::Mkdir => "FW_NODE_FS_FOLDER_CREATE_CONFIG",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// The subject: a FileRef, an upload or a store key (a folder key for `fs.folder.list`).
    #[serde(default)]
    pub from: Value,
    /// `fs.file.get`: `text` (default) or `base64`.
    #[serde(default)]
    pub encoding: String,
    /// `fs.folder.create`: the folder made. copy/move: the destination folder (default: the source's).
    #[serde(default)]
    pub folder: String,
    /// copy/move: the destination name (default: the source's).
    #[serde(default)]
    pub filename: Option<String>,
    /// copy/move: the exact destination key.
    #[serde(default)]
    pub path: Option<String>,
    /// The store a bare key is read from and a copy is written to; saved explicitly at registration.
    #[serde(default)]
    pub store: Option<String>,
    /// copy/move: `error` (default), `skip` or `overwrite`.
    #[serde(default)]
    pub on_conflict: Option<String>,
    /// `fs.file.delete`: a store key naming a folder removes everything under it.
    #[serde(default)]
    pub recursive: bool,
}

fn flag(name: &str, key: &str, description: &str, value: &str) -> DslFlag {
    DslFlag {
        flag: name.to_string(),
        config_key: key.to_string(),
        description: description.to_string(),
        kind: DslFlagKind::Scalar,
        required: false,
        value: value.to_string(),
        ..Default::default()
    }
}

fn field(name: &str, label: &str, help: &str) -> NodeFieldDef {
    NodeFieldDef {
        name: name.to_string(),
        label: label.to_string(),
        field_type: NodeFieldType::Text,
        help: Some(help.to_string()),
        ..Default::default()
    }
}

fn words(list: &[&str]) -> Vec<String> {
    list.iter().map(|word| word.to_string()).collect()
}

fn from_flag(description: &str) -> DslFlag {
    DslFlag { required: true, ..flag("--from", "from", description, "file") }
}

fn read_store_flag() -> DslFlag {
    DslFlag {
        description: "The store a bare key is read from: `local`, or the id of an `s3` credential; a FileRef is read from its own store. Saved explicitly when the pipeline is registered.".to_string(),
        value: "text".to_string(),
        ..store_flag()
    }
}

fn store_field() -> NodeFieldDef {
    store_fields(OnConflict::Error).into_iter().find(|field| field.name == "store").expect("the store field")
}

/// A definition with the shape every node here shares; `answer` is the one
/// key it adds.
fn object_definition(
    kind: &str,
    title: &str,
    description: &str,
    answer: (&str, Value),
    dsl_flags: Vec<DslFlag>,
    fields: Vec<NodeFieldDef>,
    examples: Vec<NodeExample>,
) -> NodeDefinition {
    let layout = fields.iter().map(|field| LayoutItem::Field(field.name.clone())).collect();
    NodeDefinition {
        kind: kind.to_string(),
        capabilities: vec![NodeCapability::Filesystem],
        title: title.to_string(),
        description: description.to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: json!({ "type": "object", "properties": { answer.0: answer.1 } }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags,
        fields,
        layout,
        examples,
        ..Default::default()
    }
}

fn stat_schema(description: &str) -> Value {
    json!({
        "type": "object",
        "description": description,
        "properties": {
            "path": { "type": "string" },
            "size": { "type": "integer" },
            "modified": { "type": ["string", "null"] },
            "kind": { "type": "string", "description": "object or prefix" },
            "content_type": { "type": "string" }
        }
    })
}

fn file_ref_schema(origin: &str) -> Value {
    json!({
        "type": "object",
        "description": "The durable FileRef of the stored file; the store path is `ref`.",
        "properties": {
            "ref": { "type": "string" },
            "store": { "type": "string" },
            "origin": { "type": "string", "const": origin }
        }
    })
}

fn stored(r: &str, mime: &str, kind: &str, size: u64, origin: &str) -> Value {
    json!({ "__zf_type": "file_ref", "backend": "zebfs", "store": "local", "ref": r, "filename": r.rsplit('/').next().unwrap_or(r), "mime": mime, "kind": kind, "size": size, "sha256": "sha256:…", "lifecycle": "durable", "origin": origin, "trust": "generated" })
}

pub fn list_definition() -> NodeDefinition {
    object_definition(
        LIST_NODE_KIND,
        "Folder List",
        "List the immediate children of the folder `--from` names in a project store (`/` is the store's root). One level; it does not recurse. \
         Adds `folder: { path, items, count }` — each item `{ name, path, size, modified, kind, content_type }`, `kind` being `object` or `prefix` — \
         and keeps the rest of the payload, so the next node reads `input.folder.items`.",
        ("folder", json!({
            "type": "object",
            "properties": { "path": { "type": "string" }, "items": { "type": "array" }, "count": { "type": "integer" } }
        })),
        vec![
            DslFlag { required: true, ..flag("--from", "from", "The folder to list, a store key; `/` is the store's root.", "text") },
            read_store_flag(),
        ],
        vec![field("from", "Folder", "The folder to list, e.g. uploads; / is the store's root."), store_field()],
        vec![NodeExample::dsl("List a folder", "fs.folder.list --from uploads").input(json!({ "keep": true })).output(json!({
            "keep": true,
            "folder": { "path": "uploads", "count": 1, "items": [{ "name": "a1.jpg", "path": "uploads/a1.jpg", "size": 1723, "modified": "2026-09-13T04:00:00Z", "kind": "object", "content_type": "image/jpeg" }] }
        }))],
    )
}

pub fn head_definition() -> NodeDefinition {
    object_definition(
        HEAD_NODE_KIND,
        "File Head",
        "Read what is known about the file `--from` names — a FileRef, an upload or a store key — without reading its bytes. \
         Adds `file: { path, size, modified, kind, content_type }` and keeps the rest of the payload. A missing file fails the node; it does not answer null.",
        ("file", stat_schema("The file's stat.")),
        vec![from_flag("The file: a FileRef, an upload or a store key."), read_store_flag()],
        vec![field("from", "From", "The file, e.g. {{ input.file }} or uploads/a1.jpg."), store_field()],
        vec![NodeExample::dsl("Stat a stored file", "fs.file.head --from uploads/a1.jpg").output(json!({
            "file": { "path": "uploads/a1.jpg", "size": 1723, "modified": "2026-09-13T04:00:00Z", "kind": "object", "content_type": "image/jpeg" }
        }))],
    )
}

pub fn get_definition() -> NodeDefinition {
    let mut schema = stat_schema("The file's stat and its content: UTF-8 text in `content`, or bytes in `base64`.");
    schema["properties"]["content"] = json!({ "type": ["string", "null"] });
    schema["properties"]["base64"] = json!({ "type": ["string", "null"] });
    object_definition(
        GET_NODE_KIND,
        "File Get",
        "Read the content of the file `--from` names — a FileRef, an upload or a store key. `--encoding text` (default) puts UTF-8 in `file.content`; \
         `--encoding base64` puts the bytes in `file.base64`. Adds `file: { path, size, modified, kind, content_type, content, base64 }` and keeps the rest of the payload. \
         A binary file read as text fails with FW_NODE_FS_FILE_GET_UTF8; use base64.",
        ("file", schema),
        vec![
            from_flag("The file: a FileRef, an upload or a store key."),
            DslFlag { choices: words(ENCODINGS), ..flag("--encoding", "encoding", "text (default) or base64.", "") },
            read_store_flag(),
        ],
        vec![
            field("from", "From", "The file, e.g. {{ input.file }} or docs/notes.md."),
            NodeFieldDef {
                field_type: NodeFieldType::Select,
                default_value: Some(json!("text")),
                options: ENCODINGS.iter().map(|v| SelectOptionDef { value: v.to_string(), label: v.to_string() }).collect(),
                ..field("encoding", "Encoding", "UTF-8 text, or base64 for bytes.")
            },
            store_field(),
        ],
        vec![NodeExample::dsl("Read a text file", "fs.file.get --from docs/notes.md").output(json!({
            "file": { "path": "docs/notes.md", "size": 42, "modified": "2026-09-13T04:00:00Z", "kind": "object", "content_type": "text/plain; charset=utf-8", "content": "# Notes\n…", "base64": null }
        }))],
    )
}

pub fn delete_definition() -> NodeDefinition {
    object_definition(
        DELETE_NODE_KIND,
        "File Delete",
        "Delete the file `--from` names — a FileRef from the store it names, or a store key — and forget its exposure rule, so a later file at that key starts private. \
         A store key naming a folder that holds files is refused (FW_NODE_FS_FILE_DELETE_FOLDER) unless `--recursive` is set, which removes the folder and everything \
         under it — no confirmation, no trash. A FileRef is always one file. A file that is already gone succeeds. \
         Adds `file: { ref, deleted: true }` (`recursive: true` beside them when a folder went) and keeps the rest of the payload.",
        ("file", json!({ "type": "object", "properties": {
            "ref": { "type": "string" },
            "deleted": { "type": "boolean", "const": true },
            "recursive": { "type": "boolean", "description": "Present, true, when --recursive removed a folder" }
        } })),
        vec![
            from_flag("The file to delete: a FileRef or a store key."),
            DslFlag { kind: DslFlagKind::Bool, ..flag("--recursive", "recursive", "Let a store key naming a folder remove the folder and everything under it.", "") },
            read_store_flag(),
        ],
        vec![
            field("from", "From", "The file to delete, e.g. {{ input.file }}."),
            NodeFieldDef {
                field_type: NodeFieldType::Checkbox,
                default_value: Some(json!(false)),
                ..field("recursive", "Recursive", "Allow a folder: remove it and everything under it.")
            },
            store_field(),
        ],
        vec![NodeExample::dsl("Delete an upload once it is processed", "fs.file.delete --from \"{{ input.file }}\"")
            .input(json!({ "file": stored("uploads/a1.jpg", "image/jpeg", "image", 1723, "fs.file.put") }))
            .output(json!({ "file": { "ref": "uploads/a1.jpg", "deleted": true } }))],
    )
}

fn copy_like_definition(kind: &str, title: &str, description: &str, example: NodeExample) -> NodeDefinition {
    object_definition(
        kind,
        title,
        description,
        ("file", file_ref_schema(kind)),
        vec![
            from_flag("The file: a FileRef, an upload or a store key."),
            DslFlag {
                description: "The store to write to: `local`, or the id of an `s3` credential; a bare --from key is read from it too. Saved explicitly when the pipeline is registered.".to_string(),
                ..read_store_flag()
            },
            flag("--folder", "folder", "Destination folder (default: the source's folder).", "text"),
            flag("--filename", "filename", "Destination name (default: the source's name).", "text"),
            flag("--path", "path", "Exact destination key; overrides --folder and --filename.", "text"),
            DslFlag { choices: words(ON_CONFLICT_WORDS), ..on_conflict_flag(OnConflict::Error) },
        ],
        [
            field("from", "From", "The file, e.g. {{ input.file }} or uploads/a1.jpg."),
            store_field(),
            field("folder", "Folder", "Destination folder (default: the source's folder)."),
            field("filename", "Filename", "Destination name (default: the source's name)."),
            field("path", "Path", "Exact destination key; overrides folder and filename."),
        ]
        .into_iter()
        .chain(store_fields(OnConflict::Error).into_iter().filter(|field| field.name == "on_conflict"))
        .collect(),
        vec![example],
    )
}

pub fn copy_definition() -> NodeDefinition {
    copy_like_definition(
        COPY_NODE_KIND,
        "File Copy",
        "Copy the file `--from` names — a FileRef from any store, an upload, or a store key — to `--folder` (keeps the name), `--filename`, or an exact `--path`, \
         in the node's `--store`. An existing destination is an error unless `--on-conflict` says otherwise. Adds `file`, the durable FileRef of the copy \
         (`origin: fs.file.copy`), and keeps the rest of the payload. A copy is private like any file until the owner exposes its folder in Studio → Files.",
        NodeExample::dsl("Keep a copy of an upload", "fs.file.copy --from \"{{ input.file }}\" --folder images")
            .input(json!({ "file": stored("uploads/a1.jpg", "image/jpeg", "image", 1723, "fs.file.put") }))
            .output(json!({ "file": stored("images/a1.jpg", "image/jpeg", "image", 1723, COPY_NODE_KIND) })),
    )
}

pub fn move_definition() -> NodeDefinition {
    copy_like_definition(
        MOVE_NODE_KIND,
        "File Move",
        "Move the file `--from` names: copy it to the destination (`--folder`, `--filename` or `--path`, in the node's `--store`), then delete the source and \
         forget its exposure rule. Adds `file`, the durable FileRef at the new key (`origin: fs.file.move`), and keeps the rest of the payload. \
         Any FileRef still pointing at the source is now dangling — update the row that stored it.",
        NodeExample::dsl("Archive a processed file", "fs.file.move --from \"{{ input.file }}\" --folder archive/inbox")
            .input(json!({ "file": stored("inbox/a1.csv", "text/csv", "csv", 9021, "fs.file.put") }))
            .output(json!({ "file": stored("archive/inbox/a1.csv", "text/csv", "csv", 9021, MOVE_NODE_KIND) })),
    )
}

pub fn mkdir_definition() -> NodeDefinition {
    object_definition(
        MKDIR_NODE_KIND,
        "Folder Create",
        "Create `--folder` in a project store. Rarely needed: every file writer creates the folders it writes into; use this only for a folder that \
         must exist before anything is in it. A folder that already exists is left as it is. Adds `folder: { path, created }` — `created` is false when \
         it was already there — and keeps the rest of the payload.",
        ("folder", json!({ "type": "object", "properties": { "path": { "type": "string" }, "created": { "type": "boolean" } } })),
        vec![
            DslFlag { required: true, ..flag("--folder", "folder", "The folder to create, a store key.", "text") },
            DslFlag {
                description: "The store to create it in: `local`, or the id of an `s3` credential. Saved explicitly when the pipeline is registered.".to_string(),
                ..read_store_flag()
            },
        ],
        vec![field("folder", "Folder", "The folder to create, e.g. uploads/2026."), store_field()],
        vec![NodeExample::dsl("Create an upload folder", "fs.folder.create --folder uploads/2026")
            .output(json!({ "folder": { "path": "uploads/2026", "created": true } }))],
    )
}

pub struct Node {
    config: Config,
    platform: Arc<PlatformService>,
    operation: Operation,
}

impl Node {
    pub fn new(config: Config, platform: Arc<PlatformService>, operation: Operation) -> Result<Self, PipelineError> {
        Ok(Self { config, platform, operation })
    }

    /// The file `--from` names, opened in its store.
    fn source(&self, owner: &str, project: &str) -> Result<(NodeStore, String), PipelineError> {
        open_from(&self.platform, owner, project, &self.config.from, self.config.store.as_deref(), "--from", self.operation.source_code())
    }

    fn list(&self, owner: &str, project: &str) -> Result<Value, PipelineError> {
        let op = self.operation;
        let folder = match &self.config.from {
            Value::String(raw) if raw.trim() == "/" => String::new(),
            Value::String(raw) if !raw.trim().is_empty() => crate::zebfs::normalize_object_path(raw)
                .map_err(|err| PipelineError::new(op.source_code(), format!("--from '{raw}': {}", err.message)))?,
            Value::Null | Value::String(_) => {
                return Err(PipelineError::new(op.source_code(), "--from is required: the folder to list; `/` is the store's root"));
            }
            other => return Err(PipelineError::new(op.source_code(), format!("--from is a folder key, not {other}"))),
        };
        let store = open_store(&self.platform, owner, project, self.config.store.as_deref())?;
        let entries = store.fs.list(&folder).map_err(|err| PipelineError::new(op.code(), format!("'{folder}': {err}")))?;
        let items: Vec<Value> = entries.iter().map(entry_json).collect();
        Ok(json!({ "folder": { "path": folder, "count": items.len(), "items": items } }))
    }

    fn head(&self, owner: &str, project: &str) -> Result<Value, PipelineError> {
        let (store, key) = self.source(owner, project)?;
        let stat = store.fs.head(&key).map_err(|err| PipelineError::new(self.operation.code(), format!("'{key}': {err}")))?;
        Ok(json!({ "file": stat_json(&stat) }))
    }

    fn get(&self, owner: &str, project: &str) -> Result<Value, PipelineError> {
        let op = self.operation;
        let encoding = choice(&self.config.encoding, ENCODINGS, "text", "--encoding", op.config_code())?;
        let (store, key) = self.source(owner, project)?;
        let stat = store.fs.head(&key).map_err(|err| PipelineError::new(op.code(), format!("'{key}': {err}")))?;
        // Capped like every node read: the bytes go into the payload.
        let bytes = read_capped(&store.fs, &key, op.code())?;
        let mut file = stat_json(&stat);
        if encoding == "base64" {
            file["content"] = Value::Null;
            file["base64"] = Value::String(base64::engine::general_purpose::STANDARD.encode(&bytes));
        } else {
            let text = String::from_utf8(bytes)
                .map_err(|err| PipelineError::new(GET_UTF8_CODE, format!("'{key}' is not UTF-8; use --encoding base64: {err}")))?;
            file["content"] = Value::String(text);
            file["base64"] = Value::Null;
        }
        Ok(json!({ "file": file }))
    }

    fn delete(&self, owner: &str, project: &str) -> Result<Value, PipelineError> {
        let code = self.operation.code();
        let (store, key) = self.source(owner, project)?;
        let is_folder = match store.fs.head(&key) {
            Ok(stat) => stat.kind == ZebFsEntryKind::Prefix,
            // Already gone: deleting it again succeeds.
            Err(err) if err.code == "ZEBFS_NOT_FOUND" => false,
            Err(err) => return Err(PipelineError::new(code, format!("'{key}': {err}"))),
        };
        let holds_files = is_folder && !store.fs.list(&key).map_err(|err| PipelineError::new(code, format!("'{key}': {err}")))?.is_empty();
        if holds_files && crate::pipeline::nodes::shared::file_ref::is_file_ref(&self.config.from) {
            return Err(PipelineError::new(DELETE_FOLDER_CODE, format!("--from is a FileRef, which names one file, but '{key}' is a folder")));
        }
        if holds_files && !self.config.recursive {
            return Err(PipelineError::new(
                DELETE_FOLDER_CODE,
                format!("'{key}' is a folder that holds files; set --recursive to remove it and everything under it"),
            ));
        }
        store.delete_named(&self.platform, owner, project, &key, code)?;
        if holds_files {
            return Ok(json!({ "file": { "ref": key, "deleted": true, "recursive": true } }));
        }
        Ok(json!({ "file": { "ref": key, "deleted": true } }))
    }

    fn copy_or_move(&self, owner: &str, project: &str) -> Result<Value, PipelineError> {
        let op = self.operation;
        let code = op.code();
        let on_conflict = OnConflict::parse(self.config.on_conflict.as_deref(), OnConflict::Error, op.config_code())?;
        let (source, from) = self.source(owner, project)?;
        let store = open_store(&self.platform, owner, project, self.config.store.as_deref())?;
        let (source_folder, source_name) = match from.rsplit_once('/') {
            Some((folder, name)) => (folder.to_string(), name.to_string()),
            None => (String::new(), from.clone()),
        };
        let folder = if self.config.folder.trim().is_empty() { source_folder.as_str() } else { self.config.folder.trim() };
        let filename = self.config.filename.as_deref().map(str::trim).filter(|name| !name.is_empty()).unwrap_or(&source_name);
        let to = target_key(self.config.path.as_deref(), folder, filename, op.config_code())?;
        let same_store = source.id == store.id;
        if same_store && to == from {
            return Err(PipelineError::new(op.config_code(), "the destination is the source; set --folder, --filename or --path"));
        }
        if !on_conflict.allows(&store.fs, &to, code)? {
            // Skipped: the answer is the file already there, as it is.
            return Ok(json!({ "file": store.stored_ref(&to, op.kind(), "generated", code)? }));
        }
        let stat = if same_store {
            store.fs.copy(&from, &to).map_err(|err| PipelineError::new(code, err.to_string()))?
        } else {
            // Between stores the bytes stream through a scratch file.
            let scratch = crate::pipeline::nodes::shared::store_scratch::StoreScratch::new(code)?;
            let local = scratch.local("object");
            source
                .fs
                .get_to_file(&from, &local)
                .map_err(|err| PipelineError::new(code, format!("read '{from}' from '{}': {err}", source.id)))?;
            store.fs.put_from_file(&to, &local).map_err(|err| PipelineError::new(code, err.to_string()))?
        };
        if op == Operation::Move {
            source.delete_named(&self.platform, owner, project, &from, code)?;
        }
        // A copied file answers its FileRef; a copied folder stays a plain stat.
        let file = if matches!(stat.kind, ZebFsEntryKind::Object) {
            store.stored_ref(&stat.path, op.kind(), "generated", code)?
        } else {
            stat_json(&stat)
        };
        Ok(json!({ "file": file }))
    }

    fn mkdir(&self, owner: &str, project: &str) -> Result<Value, PipelineError> {
        let op = self.operation;
        if self.config.folder.trim().is_empty() {
            return Err(PipelineError::new(op.config_code(), "--folder is required: the folder to create"));
        }
        let folder = target_key(Some(&self.config.folder), "", "", op.config_code())?;
        let store = open_store(&self.platform, owner, project, self.config.store.as_deref())?;
        let created = match store.fs.head(&folder) {
            Ok(stat) if stat.kind == ZebFsEntryKind::Object => {
                return Err(PipelineError::new(op.code(), format!("'{folder}' is a file, not a folder")));
            }
            Ok(_) => false,
            Err(err) if err.code == "ZEBFS_NOT_FOUND" => true,
            Err(err) => return Err(PipelineError::new(op.code(), format!("'{folder}': {err}"))),
        };
        if created {
            store.fs.create_prefix(&folder).map_err(|err| PipelineError::new(op.code(), format!("'{folder}': {err}")))?;
        }
        Ok(json!({ "folder": { "path": folder, "created": created } }))
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

    async fn execute_async(&self, input: NodeExecutionInput) -> Result<NodeExecutionOutput, PipelineError> {
        let (owner, project, ..) = metadata_scope(&input.metadata)?;
        let answer = match self.operation {
            Operation::List => self.list(owner, project)?,
            Operation::Head => self.head(owner, project)?,
            Operation::Get => self.get(owner, project)?,
            Operation::Delete => self.delete(owner, project)?,
            Operation::Copy | Operation::Move => self.copy_or_move(owner, project)?,
            Operation::Mkdir => self.mkdir(owner, project)?,
        };
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: with_answer(&input.payload, answer),
            trace: vec![format!("node_kind={}", self.operation.kind())],
        })
    }
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
mod tests;
