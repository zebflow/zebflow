//! `fs.file.put` — write one file into a project store.
//!
//! One source, exactly: `--from` (a file the run holds — an upload, a FileRef
//! from any store, a byte envelope, or a store key), `--text` (content as
//! given, or decoded with `--encoding base64`) or `--value` (any JSON value,
//! written as JSON). A file given with `--from` is checked before a byte is
//! stored: its type is read from its content (`put/sniff.rs`), the claimed
//! type must agree, `--accept` must allow it, and its stored extension
//! follows that type. Every source is held to `--max-size`.
//!
//! The answer is one key, `file`: the durable FileRef of what was written
//! (`origin: fs.file.put`); the rest of the payload stays, so the form fields
//! beside an upload reach the next node. The store path is `file.ref`; a URL
//! is not a node's business.

pub mod sniff;

use std::sync::Arc;

use async_trait::async_trait;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::pipeline::model::NodeCapability;
use crate::pipeline::nodes::shared::file_ref::{is_file_ref, mime_for_filename, read_file_ref_bytes};
use crate::pipeline::nodes::shared::limits::choice;
use crate::pipeline::nodes::shared::project_store::{
    MAX_NODE_OBJECT_BYTES, OnConflict, on_conflict_flag, open_source, open_store, store_fields, store_flag, target_key,
};
use crate::pipeline::nodes::shared::units;
use crate::pipeline::nodes::shared::util::{filename_stem, metadata_scope, with_answer};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    model::{DslFlag, DslFlagKind, LayoutItem, NodeExample, NodeFieldDef, NodeFieldType, SelectOptionDef},
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;

use sniff::{ACCEPT_CODE, ACCEPT_WORDS};

pub const NODE_KIND: &str = "fs.file.put";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

pub const CODE: &str = "FW_NODE_FS_FILE_PUT";
pub const CONFIG_CODE: &str = "FW_NODE_FS_FILE_PUT_CONFIG";
const SOURCE_CODE: &str = "FW_NODE_FS_FILE_PUT_SOURCE";
const SIZE_CODE: &str = "FW_NODE_FS_FILE_PUT_SIZE";
const ENCODING_CODE: &str = "FW_NODE_FS_FILE_PUT_ENCODING";
const EXTENSION_CODE: &str = "FW_NODE_FS_FILE_PUT_EXTENSION";

const DEFAULT_FOLDER: &str = "uploads";
const DEFAULT_MAX_SIZE: &str = "10MB";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// A file the run holds: an upload, a FileRef, a byte envelope, or a store key.
    #[serde(default)]
    pub from: Value,
    /// Content written as given (or decoded, with `encoding: base64`).
    #[serde(default)]
    pub text: Value,
    /// `text` (default) or `base64`: how `text` is read.
    #[serde(default)]
    pub encoding: String,
    /// Any JSON value, written as JSON; a string is written as its text.
    #[serde(default)]
    pub value: Value,
    /// The file kinds `from` may be (default `image`).
    #[serde(default)]
    pub accept: Vec<String>,
    /// The largest file written, with its unit (default `10MB`).
    #[serde(default)]
    pub max_size: String,
    #[serde(default)]
    pub store: Option<String>,
    #[serde(default)]
    pub folder: String,
    #[serde(default)]
    pub filename: Option<String>,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub on_conflict: Option<String>,
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

fn field(name: &str, label: &str, field_type: NodeFieldType, help: &str) -> NodeFieldDef {
    NodeFieldDef {
        name: name.to_string(),
        label: label.to_string(),
        field_type,
        help: Some(help.to_string()),
        ..Default::default()
    }
}

fn words(list: &[&str]) -> Vec<String> {
    list.iter().map(|word| word.to_string()).collect()
}

pub fn definition() -> NodeDefinition {
    let file_ref_schema = json!({
        "type": "object",
        "description": "The durable FileRef of the written file.",
        "required": ["__zf_type", "backend", "store", "ref", "filename", "mime", "kind", "size", "sha256", "lifecycle", "origin", "trust"],
        "properties": {
            "__zf_type": { "type": "string", "const": "file_ref" },
            "backend":   { "type": "string" },
            "store":     { "type": "string", "description": "The store it was written to" },
            "ref":       { "type": "string", "description": "The store key, e.g. `uploads/3f9c….jpg`" },
            "filename":  { "type": "string" },
            "mime":      { "type": "string" },
            "kind":      { "type": "string", "description": "image, pdf, csv, json, audio, video, archive, … (the FileRef vocabulary)" },
            "size":      { "type": "integer" },
            "sha256":    { "type": "string" },
            "lifecycle": { "type": "string", "const": "durable" },
            "origin":    { "type": "string", "const": NODE_KIND },
            "trust":     { "type": "string", "description": "--from carries the source's word (an upload stays untrusted); --text and --value are generated" }
        }
    });
    let upload = json!({ "__zf_type": "file_ref", "backend": "zebfs", "store": "local", "ref": "tmp/runs/r1/files/IMG_1.jpg", "filename": "IMG_1.jpg", "mime": "image/jpeg", "kind": "image", "size": 182331, "sha256": "sha256:…", "lifecycle": "temporary", "origin": "webhook", "trust": "untrusted" });
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Filesystem],
        title: "File Put".to_string(),
        description: "Write one file into a project store. Exactly one source: `--from` a file the run holds — an upload \
            (`{{ $trigger.files.photo }}`), a FileRef from any store, a byte envelope, or a store key — checked by its content \
            (the claimed type must agree, `--accept` must allow it, default `image`, and the stored extension follows the detected type); \
            `--text` written as given (`--encoding base64` decodes it to bytes first); or `--value`, any JSON value written as JSON \
            (a string is written as its text). Every source is refused above `--max-size` (default 10MB). Writes under `--folder` \
            (default `uploads`) as a generated name, as `--filename`, or at an exact `--path`; a named file that exists is an error \
            unless `--on-conflict` says otherwise. Adds `file` — the durable FileRef (`ref`, `store`, `mime`, `kind`, `size`, `sha256`, \
            `origin: fs.file.put`, `trust`) — and keeps the rest of the payload, so `input.webhook.body.caption` from the same form is still there. \
            Store `file.ref` in a row, never a URL."
            .to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: json!({ "type": "object", "properties": { "file": file_ref_schema } }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: Default::default(),
        dsl_flags: vec![
            flag("--from", "from", "The file to write: an upload, a FileRef, a byte envelope or a store key, usually `{{ $trigger.files.<field> }}`.", "file"),
            flag("--text", "text", "Content written as given.", "text"),
            DslFlag {
                choices: words(&["text", "base64"]),
                ..flag("--encoding", "encoding", "How --text is read: text (default) or base64, decoded to bytes first.", "")
            },
            flag("--value", "value", "Any JSON value, written as JSON; a string is written as its text.", "json"),
            DslFlag {
                kind: DslFlagKind::RepeatedList,
                choices: words(ACCEPT_WORDS),
                max_repeat: Some(ACCEPT_WORDS.len() as u32),
                ..flag("--accept", "accept", "A file kind --from may be, by content (repeat; default: image).", "")
            },
            flag("--max-size", "max_size", "The largest file written, with its unit (default: 10MB).", "size"),
            DslFlag { value: "text".to_string(), ..store_flag() },
            flag("--folder", "folder", "Destination folder (default: uploads).", "text"),
            flag("--filename", "filename", "Destination name (default: a generated name); for --from, a missing extension is the detected one.", "text"),
            flag("--path", "path", "Exact destination key; overrides --folder and --filename.", "text"),
            DslFlag { choices: words(&["error", "skip", "overwrite"]), ..on_conflict_flag(OnConflict::Error) },
        ],
        fields: vec![
            field("from", "From", NodeFieldType::Text, "The file to write, e.g. {{ $trigger.files.photo }}. Set one of From, Text or Value."),
            NodeFieldDef { rows: Some(6), ..field("text", "Text", NodeFieldType::Textarea, "Content written as given.") },
            NodeFieldDef {
                default_value: Some(json!("text")),
                options: ["text", "base64"].iter().map(|v| SelectOptionDef { value: v.to_string(), label: v.to_string() }).collect(),
                ..field("encoding", "Encoding", NodeFieldType::Select, "How Text is read: as text, or base64 decoded to bytes.")
            },
            field("value", "Value", NodeFieldType::Text, "Any JSON value, e.g. {{ input.report }}; written as JSON."),
            NodeFieldDef {
                options: ACCEPT_WORDS.iter().map(|v| SelectOptionDef { value: v.to_string(), label: v.to_string() }).collect(),
                ..field("accept", "Accept", NodeFieldType::MultiCheckbox, "The kinds From may be, read from its content. None ticked: image.")
            },
            NodeFieldDef { default_value: Some(json!(DEFAULT_MAX_SIZE)), ..field("max_size", "Max size", NodeFieldType::Text, "The largest file written, e.g. 10MB or 512KiB.") },
            field("folder", "Folder", NodeFieldType::Text, "Destination folder (default: uploads)."),
            field("filename", "Filename", NodeFieldType::Text, "Destination name (default: a generated name)."),
            field("path", "Path", NodeFieldType::Text, "Exact destination key; overrides folder and filename."),
        ]
        .into_iter()
        .chain(store_fields(OnConflict::Error))
        .collect(),
        layout: ["from", "text", "encoding", "value", "accept", "max_size", "folder", "filename", "path", "store", "on_conflict"]
            .iter()
            .map(|name| LayoutItem::Field(name.to_string()))
            .collect(),
        examples: vec![
            NodeExample::dsl("Keep an uploaded photo", "fs.file.put --from \"{{ $trigger.files.photo }}\" --folder uploads --accept image --max-size 10MB")
                .input(json!({ "body": { "caption": "Sunset" }, "files": { "photo": upload } }))
                .output(json!({ "body": { "caption": "Sunset" }, "files": { "photo": upload }, "file": { "__zf_type": "file_ref", "backend": "zebfs", "store": "local", "ref": "uploads/3f9c….jpg", "filename": "3f9c….jpg", "mime": "image/jpeg", "kind": "image", "size": 182331, "sha256": "sha256:…", "lifecycle": "durable", "origin": NODE_KIND, "trust": "untrusted" } }))
                .note("Then `sekejap.query.run --write --param \"1={{ $trigger.body.caption }}\" --param \"2={{ input.file.ref }}\" -- \"INSERT INTO photos (caption, path) VALUES ($1, $2)\"`. `file` is a FileRef, so `fs.image.thumbnail --source-key file` and a `--preview image` take it as it is."),
            NodeExample::dsl("Write a report", "fs.file.put --value \"{{ input.report }}\" --folder exports --filename report.json")
                .input(json!({ "report": { "total": 3 } }))
                .output(json!({ "report": { "total": 3 }, "file": { "__zf_type": "file_ref", "backend": "zebfs", "store": "local", "ref": "exports/report.json", "filename": "report.json", "mime": "application/json", "kind": "json", "size": 11, "sha256": "sha256:…", "lifecycle": "durable", "origin": NODE_KIND, "trust": "generated" } })),
            NodeExample::dsl("Write text", "fs.file.put --text \"{{ input.csv }}\" --folder exports --filename totals.csv --on-conflict overwrite"),
        ],
        ..Default::default()
    }
}

/// Which flag the bytes come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Source {
    From,
    Text,
    Value,
}

impl Source {
    fn flag(self) -> &'static str {
        match self {
            Self::From => "--from",
            Self::Text => "--text",
            Self::Value => "--value",
        }
    }
}

/// The bytes to write and what is known about them before a name is chosen.
struct Content {
    bytes: Vec<u8>,
    /// The detected type (`--from`) or the type the source implies.
    mime: String,
    /// The extension a generated name takes.
    extension: String,
    /// `true` when the mime came from the content and the name must agree.
    detected: bool,
    trust: String,
}

pub struct Node {
    config: Config,
    platform: Arc<PlatformService>,
}

impl Node {
    pub fn new(config: Config, platform: Arc<PlatformService>) -> Result<Self, PipelineError> {
        Ok(Self { config, platform })
    }

    /// Exactly one of `--from`, `--text`, `--value`. A null or an empty
    /// string is no value, as an unset field in the editor sends it.
    fn source(&self) -> Result<Source, PipelineError> {
        let given = |value: &Value| !(value.is_null() || value.as_str().is_some_and(str::is_empty));
        let set: Vec<Source> = [(Source::From, &self.config.from), (Source::Text, &self.config.text), (Source::Value, &self.config.value)]
            .into_iter()
            .filter(|(_, value)| given(value))
            .map(|(source, _)| source)
            .collect();
        match set.as_slice() {
            [one] => Ok(*one),
            [] => Err(PipelineError::new(SOURCE_CODE, "set one source: --from, --text or --value")),
            many => Err(PipelineError::new(
                SOURCE_CODE,
                format!("set one source, not {} — --from, --text or --value", many.iter().map(|s| s.flag()).collect::<Vec<_>>().join(" and ")),
            )),
        }
    }

    fn max_size(&self) -> Result<u64, PipelineError> {
        let raw = if self.config.max_size.trim().is_empty() { DEFAULT_MAX_SIZE } else { self.config.max_size.trim() };
        let max = units::size(raw, "--max-size", SIZE_CODE)?;
        if max == 0 || max > MAX_NODE_OBJECT_BYTES {
            return Err(PipelineError::new(
                SIZE_CODE,
                format!("--max-size {raw} must be above 0B and at most {MAX_NODE_OBJECT_BYTES}B"),
            ));
        }
        Ok(max)
    }

    fn content(&self, owner: &str, project: &str, source: Source, max: u64) -> Result<Content, PipelineError> {
        let encoding = choice(&self.config.encoding, &["text", "base64"], "text", "--encoding", ENCODING_CODE)?;
        if encoding == "base64" && source != Source::Text {
            return Err(PipelineError::new(ENCODING_CODE, "--encoding base64 reads --text only"));
        }
        let accept_given = self.config.accept.iter().any(|word| !word.trim().is_empty());
        if accept_given && source != Source::From {
            return Err(PipelineError::new(ACCEPT_CODE, format!("--accept checks a --from file; {} is written as given", source.flag())));
        }
        match source {
            Source::From => self.from_content(owner, project, max),
            Source::Text => {
                let text = match &self.config.text {
                    Value::String(text) => text.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => return Err(PipelineError::new(SOURCE_CODE, "--text takes text; a JSON value is --value")),
                };
                if encoding == "base64" {
                    let encoded = text.trim();
                    check_size(encoded.len() as u64 / 4 * 3, max + 3)?;
                    let bytes = base64::engine::general_purpose::STANDARD
                        .decode(encoded)
                        .map_err(|err| PipelineError::new(ENCODING_CODE, format!("--text is not base64: {err}")))?;
                    check_size(bytes.len() as u64, max)?;
                    let mime = sniff::detect_binary_mime(&bytes).map(sniff::normalized_mime).unwrap_or_else(|| "application/octet-stream".to_string());
                    let extension = sniff::stored_extension("", &mime).to_string();
                    return Ok(Content { bytes, mime, extension, detected: false, trust: "generated".to_string() });
                }
                check_size(text.len() as u64, max)?;
                Ok(Content { bytes: text.into_bytes(), mime: "text/plain".to_string(), extension: "txt".to_string(), detected: false, trust: "generated".to_string() })
            }
            Source::Value => {
                let (bytes, mime, extension) = match &self.config.value {
                    Value::String(text) => (text.clone().into_bytes(), "text/plain", "txt"),
                    value => (
                        serde_json::to_vec(value).map_err(|err| PipelineError::new(SOURCE_CODE, format!("--value: {err}")))?,
                        "application/json",
                        "json",
                    ),
                };
                check_size(bytes.len() as u64, max)?;
                Ok(Content { bytes, mime: mime.to_string(), extension: extension.to_string(), detected: false, trust: "generated".to_string() })
            }
        }
    }

    /// The `--from` file, read and checked: an upload or FileRef through the
    /// store it names, a byte envelope decoded, or a key in this node's store.
    fn from_content(&self, owner: &str, project: &str, max: u64) -> Result<Content, PipelineError> {
        let from = &self.config.from;
        let accept = sniff::parse_accept(&self.config.accept)?;
        // The bytes are only checked, never re-encoded, and a FileRef's own
        // `trust` word is whatever the payload says — a request body can claim
        // `sanitized`. A copy is therefore always `untrusted`.
        let trust = "untrusted".to_string();
        let text_of = |key: &str| from.get(key).and_then(Value::as_str);
        let (name, claimed, bytes) = if is_file_ref(from) || from.is_string() {
            let Some((store, key)) = open_source(&self.platform, owner, project, from, self.config.store.as_deref())? else {
                return Err(PipelineError::new(SOURCE_CODE, "--from is empty"));
            };
            // The ceiling is checked against the stored object before a byte is read.
            let stat = store.fs.head(&key).map_err(|err| PipelineError::new(SOURCE_CODE, format!("--from '{key}': {err}")))?;
            check_size(stat.size, max)?;
            let leaf = key.rsplit('/').next().unwrap_or(&key).to_string();
            if is_file_ref(from) {
                let name = text_of("filename").unwrap_or(&leaf).to_string();
                let claimed = text_of("mime").unwrap_or("application/octet-stream").to_string();
                (name, claimed, read_file_ref_bytes(&self.platform, owner, project, from)?)
            } else {
                let claimed = mime_for_filename(&leaf).to_string();
                (leaf, claimed, store.read_capped(&key, SOURCE_CODE)?)
            }
        } else if let Some(encoded) = text_of("__zf_bytes") {
            let claimed = text_of("__zf_mime").unwrap_or("application/octet-stream").to_string();
            ("download".to_string(), claimed, decode_capped(encoded, max)?)
        } else if let Some(encoded) = text_of("data") {
            let name = text_of("filename").unwrap_or("upload").to_string();
            let claimed = text_of("content_type").unwrap_or("application/octet-stream").to_string();
            (name, claimed, decode_capped(encoded, max)?)
        } else if from.is_array() {
            return Err(PipelineError::new(SOURCE_CODE, "--from is a list; name one file, e.g. {{ $trigger.files.photos[0] }}"));
        } else {
            return Err(PipelineError::new(SOURCE_CODE, "--from is not a file: give an upload, a FileRef, a byte envelope or a store key"));
        };
        check_size(bytes.len() as u64, max)?;
        let mime = sniff::effective_mime(&bytes, &claimed, &accept)?;
        let extension = sniff::stored_extension(&name, &mime).to_string();
        Ok(Content { bytes, mime, extension, detected: true, trust })
    }

    /// The store key: `--path` exactly, `--filename` in `--folder`, or a
    /// generated name. A name given for a detected file must agree with it.
    fn destination(&self, content: &Content) -> Result<(String, String), PipelineError> {
        let folder = if self.config.folder.trim().is_empty() { DEFAULT_FOLDER } else { self.config.folder.trim() };
        let path = self.config.path.as_deref().map(str::trim).filter(|p| !p.is_empty());
        let filename = self.config.filename.as_deref().map(str::trim).filter(|n| !n.is_empty());
        if path.is_some_and(|p| p.ends_with('/')) {
            return Err(PipelineError::new(CODE, "--path is the exact key of the file; a folder is --folder"));
        }
        let name = match (path, filename) {
            (Some(path), _) => {
                let leaf = path.rsplit('/').next().unwrap_or(path);
                self.check_extension(leaf, content)?;
                leaf.to_string()
            }
            (None, Some(given)) => {
                let stem = filename_stem(given);
                if stem.is_empty() {
                    return Err(PipelineError::new(CODE, format!("--filename '{given}' has no usable characters")));
                }
                self.check_extension(given, content)?;
                match sniff::extension_of(given).or_else(|| (content.detected && !content.extension.is_empty()).then(|| content.extension.clone())) {
                    Some(ext) => format!("{stem}.{ext}"),
                    None => stem,
                }
            }
            (None, None) if content.extension.is_empty() => Uuid::new_v4().to_string(),
            (None, None) => format!("{}.{}", Uuid::new_v4(), content.extension),
        };
        let key = target_key(path, folder, &name, CODE)?;
        Ok((key, name))
    }

    fn check_extension(&self, name: &str, content: &Content) -> Result<(), PipelineError> {
        if !content.detected {
            return Ok(());
        }
        match sniff::extension_of(name) {
            Some(ext) if !sniff::extension_agrees(&ext, &content.mime) => Err(PipelineError::new(
                EXTENSION_CODE,
                format!("'{name}' names a .{ext} file but the content is '{}' (.{})", content.mime, sniff::extensions(&content.mime).join(", .")),
            )),
            _ => Ok(()),
        }
    }

    /// The written file's type: what the content is (`--from`, `--value`
    /// JSON), else what its name says, else what the source implies.
    fn stored_mime(content: &Content, name: &str) -> String {
        if content.detected || content.mime == "application/json" {
            return content.mime.clone();
        }
        match mime_for_filename(name) {
            "application/octet-stream" => content.mime.clone(),
            by_name => by_name.to_string(),
        }
    }
}

fn check_size(size: u64, max: u64) -> Result<(), PipelineError> {
    if size > max {
        return Err(PipelineError::new(SIZE_CODE, format!("the file is {size} bytes, over --max-size ({max} bytes)")));
    }
    Ok(())
}

/// Base64 decoded, refused before decoding when it would pass `max`.
fn decode_capped(encoded: &str, max: u64) -> Result<Vec<u8>, PipelineError> {
    check_size(encoded.len() as u64 / 4 * 3, max + 3)?;
    base64::engine::general_purpose::STANDARD
        .decode(encoded.trim())
        .map_err(|err| PipelineError::new(SOURCE_CODE, format!("--from holds bytes that are not base64: {err}")))
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

    async fn execute_async(&self, input: NodeExecutionInput) -> Result<NodeExecutionOutput, PipelineError> {
        let (owner, project, ..) = metadata_scope(&input.metadata)?;
        let source = self.source()?;
        let max = self.max_size()?;
        let on_conflict = OnConflict::parse(self.config.on_conflict.as_deref(), OnConflict::Error, CODE)?;
        let content = self.content(owner, project, source, max)?;
        let (key, name) = self.destination(&content)?;
        let mime = Self::stored_mime(&content, &name);
        let store = open_store(&self.platform, owner, project, self.config.store.as_deref())?;
        let written = on_conflict.allows(&store.fs, &key, CODE)?;
        let file = if written {
            store.fs.put(&key, &content.bytes).map_err(|err| PipelineError::new(CODE, err.to_string()))?;
            store.file_ref(&key, &name, &mime, &content.bytes, NODE_KIND, &content.trust)
        } else {
            // Skipped: the answer is the file already there, as it is.
            store.stored_ref(&key, NODE_KIND, &content.trust, CODE)?
        };
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: with_answer(&input.payload, json!({ "file": file })),
            trace: vec![format!("node_kind={NODE_KIND} source={} path={key} store={} written={written}", source.flag(), store.id)],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR\0\0\0\x01\0\0\0\x01\x08\x02\0\0\0";

    fn platform() -> Arc<PlatformService> {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut config = crate::platform::model::PlatformConfig::default();
        config.data_root = tmp.keep().join("platform");
        config.default_password = "secret".to_string();
        Arc::new(PlatformService::from_config(config).expect("platform"))
    }

    /// An upload as the webhook delivers it: a temporary FileRef in the store.
    fn upload(platform: &Arc<PlatformService>, name: &str, mime: &str, bytes: &[u8]) -> Value {
        let store = open_store(platform, "demo", "demo", None).expect("store");
        let key = format!("tmp/runs/r1/files/{name}");
        store.fs.put(&key, bytes).expect("put");
        let mut file = store.file_ref(&key, name, mime, bytes, "webhook", "untrusted");
        file["lifecycle"] = json!("temporary");
        file
    }

    async fn run(platform: &Arc<PlatformService>, config: Value, payload: Value) -> Result<Value, PipelineError> {
        let config: Config = serde_json::from_value(config).expect("config");
        let node = Node::new(config, platform.clone())?;
        let out = node
            .execute_async(NodeExecutionInput {
                node_id: "n0".to_string(),
                input_pin: "in".to_string(),
                payload,
                metadata: json!({ "owner": "demo", "project": "demo", "pipeline": "test", "request_id": "r1" }),
                bus: None,
            })
            .await?;
        Ok(out.payload)
    }

    fn stored(platform: &Arc<PlatformService>, key: &str) -> Vec<u8> {
        open_store(platform, "demo", "demo", None).unwrap().read_capped(key, "T").unwrap()
    }

    #[tokio::test]
    async fn exactly_one_source_is_written() {
        let p = platform();
        let none = run(&p, json!({ "text": "" }), json!({})).await.unwrap_err();
        assert_eq!(none.code, SOURCE_CODE);
        assert!(none.message.contains("--from, --text or --value"), "{}", none.message);
        let two = run(&p, json!({ "text": "a", "value": { "b": 1 } }), json!({})).await.unwrap_err();
        assert_eq!(two.code, SOURCE_CODE);
        assert!(two.message.contains("--text and --value"), "{}", two.message);
        let file = upload(&p, "a.png", "image/png", PNG);
        let three = run(&p, json!({ "from": file, "text": "a", "value": 1 }), json!({})).await.unwrap_err();
        assert!(three.message.contains("--from and --text and --value"), "{}", three.message);
        assert_eq!(run(&p, json!({ "text": { "a": 1 } }), json!({})).await.unwrap_err().code, SOURCE_CODE);
    }

    /// The answer is one key, `file`, beside the payload it was given — no
    /// `saved`, no `fs` envelope.
    #[tokio::test]
    async fn an_upload_is_kept_and_the_payload_stays() {
        let p = platform();
        let file = upload(&p, "IMG_1.jpeg", "image/png", PNG);
        let input = json!({ "body": { "caption": "Sunset" }, "files": { "photo": file } });
        let out = run(&p, json!({ "from": file }), input.clone()).await.unwrap();
        let mut keys: Vec<&str> = out.as_object().unwrap().keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, vec!["body", "file", "files"]);
        assert_eq!(out["body"], input["body"]);
        let saved = &out["file"];
        crate::pipeline::nodes::shared::file_ref::validate_file_ref(saved).expect("a contract FileRef");
        let r = saved["ref"].as_str().unwrap();
        assert!(r.starts_with("uploads/") && r.ends_with(".png"), "the extension follows the content, not the name: {r}");
        assert_eq!((saved["origin"].as_str(), saved["trust"].as_str(), saved["lifecycle"].as_str()), (Some(NODE_KIND), Some("untrusted"), Some("durable")));
        assert_eq!((saved["mime"].as_str(), saved["kind"].as_str(), saved["store"].as_str()), (Some("image/png"), Some("image"), Some("local")));
        assert_eq!(stored(&p, r), PNG);
    }

    #[tokio::test]
    async fn base64_text_is_decoded_and_text_is_written_as_given() {
        let p = platform();
        let out = run(&p, json!({ "text": "aGVsbG8=", "encoding": "base64", "folder": "notes", "filename": "hi.txt" }), json!({})).await.unwrap();
        assert_eq!(out["file"]["ref"], "notes/hi.txt");
        assert_eq!(stored(&p, "notes/hi.txt"), b"hello");
        let out = run(&p, json!({ "text": "a,b\n1,2\n", "folder": "exports", "filename": "t.csv" }), json!({})).await.unwrap();
        assert_eq!((out["file"]["mime"].as_str(), out["file"]["kind"].as_str(), out["file"]["trust"].as_str()), (Some("text/csv"), Some("csv"), Some("generated")));
        let out = run(&p, json!({ "text": "plain" }), json!({})).await.unwrap();
        assert!(out["file"]["ref"].as_str().unwrap().ends_with(".txt"));
        assert_eq!(run(&p, json!({ "text": "not base64!", "encoding": "base64" }), json!({})).await.unwrap_err().code, ENCODING_CODE);
        assert_eq!(run(&p, json!({ "value": 1, "encoding": "base64" }), json!({})).await.unwrap_err().code, ENCODING_CODE);
        assert_eq!(run(&p, json!({ "text": "x", "encoding": "hex" }), json!({})).await.unwrap_err().code, ENCODING_CODE);
    }

    #[tokio::test]
    async fn a_value_is_written_as_json() {
        let p = platform();
        let out = run(&p, json!({ "value": { "total": 3 }, "folder": "exports", "filename": "report.json" }), json!({ "keep": true })).await.unwrap();
        assert_eq!(out["keep"], true);
        assert_eq!((out["file"]["mime"].as_str(), out["file"]["kind"].as_str()), (Some("application/json"), Some("json")));
        assert_eq!(serde_json::from_slice::<Value>(&stored(&p, "exports/report.json")).unwrap(), json!({ "total": 3 }));
        let out = run(&p, json!({ "value": [1, 2] }), json!({})).await.unwrap();
        assert!(out["file"]["ref"].as_str().unwrap().ends_with(".json"));
        run(&p, json!({ "value": "just text", "path": "notes/a.md" }), json!({})).await.unwrap();
        assert_eq!(stored(&p, "notes/a.md"), b"just text");
    }

    #[tokio::test]
    async fn accept_is_closed_checks_the_content_and_reads_from_only() {
        let p = platform();
        let file = upload(&p, "a.png", "image/png", PNG);
        let wrong = run(&p, json!({ "from": file, "accept": ["pdf"] }), json!({})).await.unwrap_err();
        assert_eq!(wrong.code, ACCEPT_CODE);
        assert!(wrong.message.contains("--accept pdf"), "{}", wrong.message);
        assert_eq!(run(&p, json!({ "from": file, "accept": ["images"] }), json!({})).await.unwrap_err().code, ACCEPT_CODE);
        assert!(run(&p, json!({ "from": file, "accept": ["pdf", "image"] }), json!({})).await.is_ok());
        let spoof = upload(&p, "b.jpg", "image/jpeg", PNG);
        assert!(run(&p, json!({ "from": spoof }), json!({})).await.unwrap_err().message.contains("claims 'image/jpeg'"));
        let text = run(&p, json!({ "text": "x", "accept": ["image"] }), json!({})).await.unwrap_err();
        assert_eq!(text.code, ACCEPT_CODE);
    }

    #[tokio::test]
    async fn max_size_is_a_size_with_a_ceiling() {
        let p = platform();
        let over = run(&p, json!({ "text": "twenty bytes of text", "max_size": "10B" }), json!({})).await.unwrap_err();
        assert_eq!(over.code, SIZE_CODE);
        assert!(over.message.contains("over --max-size"), "{}", over.message);
        assert_eq!(run(&p, json!({ "text": "x", "max_size": "10" }), json!({})).await.unwrap_err().code, SIZE_CODE);
        assert_eq!(run(&p, json!({ "text": "x", "max_size": "1GB" }), json!({})).await.unwrap_err().code, SIZE_CODE);
        let file = upload(&p, "a.png", "image/png", PNG);
        assert_eq!(run(&p, json!({ "from": file, "max_size": "8B" }), json!({})).await.unwrap_err().code, SIZE_CODE);
        assert!(run(&p, json!({ "text": "x", "max_size": "1KiB" }), json!({})).await.is_ok());
    }

    #[tokio::test]
    async fn a_name_for_a_from_file_follows_its_content() {
        let p = platform();
        let file = upload(&p, "a.png", "image/png", PNG);
        let out = run(&p, json!({ "from": file, "folder": "avatars", "filename": "ana smith" }), json!({})).await.unwrap();
        assert_eq!(out["file"]["ref"], "avatars/ana_smith.png");
        let out = run(&p, json!({ "from": file, "folder": "avatars", "filename": "b.PNG" }), json!({})).await.unwrap();
        assert_eq!(out["file"]["ref"], "avatars/b.png");
        let named = run(&p, json!({ "from": file, "filename": "c.jpg" }), json!({})).await.unwrap_err();
        assert_eq!(named.code, EXTENSION_CODE);
        assert_eq!(run(&p, json!({ "from": file, "path": "x/y.gif" }), json!({})).await.unwrap_err().code, EXTENSION_CODE);
        assert_eq!(run(&p, json!({ "from": file, "path": "x/" }), json!({})).await.unwrap_err().code, CODE);
        assert_eq!(run(&p, json!({ "from": file, "filename": "../.." }), json!({})).await.unwrap_err().code, CODE);
    }

    #[tokio::test]
    async fn a_named_file_that_exists_is_an_error_unless_told() {
        let p = platform();
        let config = json!({ "text": "one", "path": "notes/n.txt" });
        run(&p, config.clone(), json!({})).await.unwrap();
        assert!(run(&p, config.clone(), json!({})).await.unwrap_err().message.contains("already exists"));
        let skip = run(&p, json!({ "text": "two", "path": "notes/n.txt", "on_conflict": "skip" }), json!({})).await.unwrap();
        assert_eq!((skip["file"]["size"].as_u64(), stored(&p, "notes/n.txt")), (Some(3), b"one".to_vec()));
        run(&p, json!({ "text": "two", "path": "notes/n.txt", "on_conflict": "overwrite" }), json!({})).await.unwrap();
        assert_eq!(stored(&p, "notes/n.txt"), b"two");
    }

    #[tokio::test]
    async fn byte_envelopes_and_store_keys_are_files_too() {
        let p = platform();
        let b64 = base64::engine::general_purpose::STANDARD.encode(PNG);
        let legacy = json!({ "filename": "x.png", "content_type": "image/png", "size": PNG.len(), "data": b64 });
        let out = run(&p, json!({ "from": legacy }), json!({})).await.unwrap();
        assert_eq!(out["file"]["trust"], "untrusted");
        let envelope = json!({ "__zf_bytes": b64, "__zf_mime": "image/png" });
        assert!(run(&p, json!({ "from": envelope }), json!({})).await.is_ok());
        open_store(&p, "demo", "demo", None).unwrap().fs.put("images/k.png", PNG).unwrap();
        let out = run(&p, json!({ "from": "images/k.png", "folder": "copies" }), json!({})).await.unwrap();
        assert!(out["file"]["ref"].as_str().unwrap().starts_with("copies/"));
        assert_eq!(run(&p, json!({ "from": [legacy] }), json!({})).await.unwrap_err().code, SOURCE_CODE);
        assert_eq!(run(&p, json!({ "from": { "name": "x" } }), json!({})).await.unwrap_err().code, SOURCE_CODE);
    }

    #[test]
    fn the_definition_declares_one_source_set_and_the_destination_set() {
        let def = definition();
        let flags: Vec<&str> = def.dsl_flags.iter().map(|f| f.flag.as_str()).collect();
        assert_eq!(
            flags,
            vec!["--from", "--text", "--encoding", "--value", "--accept", "--max-size", "--store", "--folder", "--filename", "--path", "--on-conflict"]
        );
        assert_eq!(
            crate::pipeline::nodes::node_signature(&def),
            "fs.file.put [--from FILE] [--text TEXT] [--encoding text|base64] [--value JSON] \
             [--accept image|pdf|csv|json|glb|audio|video|archive…] [--max-size SIZE] [--store TEXT] [--folder TEXT] \
             [--filename TEXT] [--path TEXT] [--on-conflict error|skip|overwrite] → file"
        );
    }
}
