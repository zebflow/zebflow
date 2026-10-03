//! n.fs.save — validate and promote one uploaded/intermediate file to Zebflow FS.
//!
//! For general node authoring rules, read `src/pipeline/nodes/mod.rs`; for
//! FileRef IR and backend/lifecycle rules, read
//! `src/pipeline/nodes/shared/file_ref.rs`.
//!
//! Input: `input.files.{field}` as set by `trigger.webhook` multipart parsing,
//! where `{field}` is a dot-path so array uploads can be addressed as `photos.0`.
//! Output: `saved`, a **durable FileRef** and nothing else — the eleven
//! contract fields (`__zf_type: file_ref`, `backend`, `ref`, `filename`,
//! `mime`, `kind`, `size`, `sha256`, `lifecycle: durable`, `origin: fs.save`,
//! `trust` carried from the source). The store path is `saved.ref`; a URL is
//! not a node's business (the Studio reads objects through `files/object`; a
//! folder is exposed only by the owner, in Studio → Files).
//!
//! Files are stored as durable ZebFS object paths, usually `uploads/{uuid}.{ext}`.
//!
//! Content validation uses magic-byte inspection (via the `infer` crate) in addition to
//! the browser-reported MIME type. Both must agree, and both must match the allowed-types list.

use std::sync::Arc;

use async_trait::async_trait;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::pipeline::nodes::shared::project_store::{OnConflict, on_conflict_flag, open_store, store_fields, store_flag, target_key};
use crate::pipeline::nodes::shared::file_ref::{is_file_ref, read_file_ref_bytes};
use crate::pipeline::nodes::shared::project_store::open_source;
use crate::pipeline::nodes::shared::util::{metadata_scope, resolve_path};
use crate::pipeline::model::NodeCapability;
use crate::pipeline::{
    NodeDefinition, PipelineError,
    model::{DslFlag, DslFlagKind, LayoutItem, NodeFieldDef, NodeFieldType, SelectOptionDef},
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;

pub const NODE_KIND: &str = "n.fs.save";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

// ── Allowed file-type categories ─────────────────────────────────────────────

/// Top-level category of allowed file types. Each maps to a set of accepted MIME types
/// validated by both the browser-reported Content-Type AND actual magic bytes.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AllowedKind {
    Images,
    Pdf,
    Csv,
    Json,
    Glb,
    Audio,
    Video,
    Archive,
}

impl AllowedKind {
    /// Returns the MIME types accepted by this category.
    fn mime_types(&self) -> &'static [&'static str] {
        match self {
            AllowedKind::Images => &[
                "image/jpeg",
                "image/png",
                "image/webp",
                "image/gif",
                "image/bmp",
                "image/tiff",
                "image/svg+xml",
                "image/avif",
                "image/heic",
                "image/heif",
            ],
            AllowedKind::Pdf => &["application/pdf"],
            AllowedKind::Csv => &["text/csv", "text/plain"],
            AllowedKind::Json => &["application/json", "text/json"],
            AllowedKind::Glb => &["model/gltf-binary"],
            AllowedKind::Audio => &[
                "audio/mpeg",
                "audio/wav",
                "audio/x-wav",
                "audio/ogg",
                "audio/mp4",
                "audio/m4a",
            ],
            AllowedKind::Video => &["video/mp4", "video/webm", "video/ogg", "video/x-m4v"],
            AllowedKind::Archive => &[
                "application/zip",
                "application/gzip",
                "application/x-tar",
                "application/x-gzip",
                "application/x-bzip2",
                "application/x-7z-compressed",
                "application/x-rar-compressed",
                "application/vnd.android.package-archive",
                "application/java-archive",
                "application/vnd.apple.installer+xml",
            ],
        }
    }

    fn label(&self) -> &'static str {
        match self {
            AllowedKind::Images => "Images",
            AllowedKind::Pdf => "PDF",
            AllowedKind::Csv => "CSV",
            AllowedKind::Json => "JSON",
            AllowedKind::Glb => "3D Models (GLB)",
            AllowedKind::Audio => "Audio",
            AllowedKind::Video => "Video",
            AllowedKind::Archive => "Archives (ZIP/APK/JAR/TAR)",
        }
    }
}

fn kind_accepts_mime(kinds: &[AllowedKind], mime: &str) -> bool {
    let target = normalized_mime(mime);
    kinds.iter().any(|kind| {
        kind.mime_types()
            .iter()
            .any(|candidate| normalized_mime(candidate) == target)
    })
}

fn normalized_mime(mime: &str) -> String {
    let raw = mime.split(';').next().unwrap_or("").trim().to_lowercase();
    match raw.as_str() {
        "image/jpg" => "image/jpeg".to_string(),
        "audio/x-wav" => "audio/wav".to_string(),
        "audio/m4a" => "audio/mp4".to_string(),
        _ => raw,
    }
}

fn detect_binary_mime(bytes: &[u8]) -> Option<&'static str> {
    if is_glb(bytes) {
        return Some("model/gltf-binary");
    }

    if is_ogg_theora(bytes) {
        return Some("video/ogg");
    }

    infer::get(bytes).map(|kind| kind.mime_type())
}

fn is_glb(bytes: &[u8]) -> bool {
    if bytes.len() < 12 || &bytes[0..4] != b"glTF" {
        return false;
    }

    let version = u32::from_le_bytes([bytes[4], bytes[5], bytes[6], bytes[7]]);
    let declared_len = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    (version == 1 || version == 2) && declared_len >= 12 && declared_len <= bytes.len()
}

fn is_ogg_theora(bytes: &[u8]) -> bool {
    if bytes.len() < 35 || &bytes[0..4] != b"OggS" {
        return false;
    }

    let page_segments = bytes[26] as usize;
    let payload_start = 27 + page_segments;
    bytes.get(payload_start..payload_start + 7) == Some(&b"\x80theora"[..])
}

fn browser_mime_matches_detected(browser_mime: &str, detected_mime: &str) -> bool {
    let browser = normalized_mime(browser_mime);
    let detected = normalized_mime(detected_mime);
    // Accept generic/unknown MIME types — the actual content bytes are the authority.
    // "binary/octet-stream" is non-standard but widely used (e.g. AWS S3).
    if browser.is_empty()
        || browser == "application/octet-stream"
        || browser == "binary/octet-stream"
        || browser == detected
    {
        return true;
    }
    // ZIP-based formats: APK, JAR, DOCX, etc. have ZIP magic bytes so infer
    // reports application/zip, but the browser sends a more specific MIME.
    if detected == "application/zip" {
        let zip_based = [
            "application/vnd.android.package-archive",
            "application/java-archive",
        ];
        return zip_based.contains(&browser.as_str());
    }
    false
}

fn default_allowed_kinds() -> Vec<AllowedKind> {
    vec![AllowedKind::Images]
}

fn default_source_key() -> String {
    "files.file".to_string()
}

fn default_max_size_mb() -> f64 {
    10.0
}

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Dot-path to the file in the payload (default `files.file`, the upload
    /// field `file`): an upload, a FileRef, or a byte envelope.
    #[serde(default = "default_source_key")]
    pub source_key: String,

    /// Exact ZebFS object path. If empty, `folder` + generated filename is used.
    #[serde(default)]
    pub path: Option<String>,

    /// Subdirectory under the project ZebFS namespace (default: "uploads").
    #[serde(default)]
    pub folder: String,

    /// Allowed file-type categories. Default: [Images].
    #[serde(default = "default_allowed_kinds")]
    pub allowed_kinds: Vec<AllowedKind>,

    /// Maximum file size in MB (default: 10).
    #[serde(default = "default_max_size_mb")]
    pub max_size_mb: f64,

    /// Optional custom filename (without extension). If set, used instead of UUID.
    /// Useful for deterministic file paths (e.g. profile avatars).
    #[serde(default)]
    pub filename: Option<String>,

    /// The store to write to (`local` or an s3 credential id); saved
    /// explicitly when the pipeline is registered.
    #[serde(default)]
    pub store: Option<String>,

    /// `overwrite`, `skip` or `error` (default: error).
    #[serde(default)]
    pub on_conflict: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            source_key: default_source_key(),
            path: None,
            folder: String::new(),
            allowed_kinds: default_allowed_kinds(),
            max_size_mb: default_max_size_mb(),
            filename: None,
            store: None,
            on_conflict: None,
        }
    }
}

// ── Definition ────────────────────────────────────────────────────────────────

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Filesystem],
        title: "FS Save".to_string(),
        description: "Keep a file that came through the run. Reads the file at `--source-key` (default `files.file`: the upload field `file` set by \
            `trigger.webhook`; `files.photo` for a field named photo, `response.body` after `http.response.fetch --response-type bytes`, `image` after `fs.image.render`) — \
            an upload, a FileRef or a byte envelope, and nothing else is looked at — checks the kind by MIME and magic bytes (`--allowed-kinds images|documents|…`) and the \
            size (`--max-size` MB), then writes it under `--folder` (default `uploads/`; private until the owner exposes the folder in Studio → Files) or at \
            an exact `--path`. Adds `saved` — a durable FileRef and nothing else (`ref`, `filename`, `mime`, `kind`, `size`, `sha256`, \
            `lifecycle: durable`, `origin: fs.save`, `trust`) — to the \
            payload and keeps the rest, so `input.body.title` from the same form is still there for the INSERT. Store `saved.ref` \
            (the store path) in the row, never a URL."
            .to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "description": "Must contain input.files.{field} as set by trigger.webhook multipart."
        }),
        output_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "saved": {
                    "type": "object",
                    "description": "A durable FileRef for the stored object: the eleven contract fields and nothing else.",
                    "additionalProperties": false,
                    "required": ["__zf_type", "backend", "ref", "filename", "mime", "kind", "size", "sha256", "lifecycle", "origin", "trust"],
                    "properties": {
                        "__zf_type":     { "type": "string", "const": "file_ref" },
                        "backend":       { "type": "string", "description": "The project's native store, `zebfs`" },
                        "ref":           { "type": "string", "description": "The stored object path, e.g. `uploads/3f9c….jpg`" },
                        "filename":      { "type": "string", "description": "The stored name, e.g. `3f9c….jpg`" },
                        "mime":          { "type": "string" },
                        "kind":          { "type": "string", "description": "image, pdf, csv, json, audio, video, archive, … (the FileRef vocabulary)" },
                        "size":          { "type": "integer" },
                        "sha256":        { "type": "string", "description": "`sha256:` + 64 lowercase hex digits of the stored bytes" },
                        "lifecycle":     { "type": "string", "const": "durable" },
                        "origin":        { "type": "string", "const": "fs.save" },
                        "trust":         { "type": "string", "description": "Carried from the source file: untrusted for an upload, user for an operator's file" }
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
            DslFlag {
                flag: "--source-key".to_string(),
                config_key: "source_key".to_string(),
                description: "Dot-path to the file in the payload (default: files.file — the upload field `file`; files.photo, response.body, image)".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
                ..Default::default()
            },
            DslFlag {
                flag: "--path".to_string(),
                config_key: "path".to_string(),
                description:
                    "Exact ZebFS object path. If omitted, folder + generated filename is used."
                        .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
                ..Default::default()
            },
            DslFlag {
                flag: "--folder".to_string(),
                config_key: "folder".to_string(),
                description: "ZebFS object folder (default: \"uploads\")".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
                ..Default::default()
            },
            DslFlag {
                flag: "--allowed-kinds".to_string(),
                config_key: "allowed_kinds".to_string(),
                description:
                    "Comma-separated allowed categories: images,pdf,csv,json,glb,audio,video (default: images)"
                        .to_string(),
                kind: DslFlagKind::CommaSeparatedList,
                required: false,
                ..Default::default()
            },
            DslFlag {
                flag: "--max-size".to_string(),
                config_key: "max_size_mb".to_string(),
                description: "Maximum file size in MB (default: 10)".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
                ..Default::default()
            },
            DslFlag {
                flag: "--filename".to_string(),
                config_key: "filename".to_string(),
                description:
                    "Custom filename without extension (default: random UUID)."
                        .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
                ..Default::default()
            },
            store_flag(),
            on_conflict_flag(OnConflict::Error),
        ],
        fields: vec![
            NodeFieldDef {
                name: "source_key".to_string(),
                label: "Source key".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Dot-path to the file in the payload (default: files.file — the upload field `file`)".to_string()),
                default_value: Some(json!("files.file")),
                ..Default::default()
            },
            NodeFieldDef {
                name: "path".to_string(),
                label: "Object path".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Exact ZebFS object path. Leave empty to use folder + generated filename.".to_string()),
                default_value: None,
                ..Default::default()
            },
            NodeFieldDef {
                name: "folder".to_string(),
                label: "Folder".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("ZebFS object folder used when Object path is empty.".to_string()),
                default_value: Some(json!("uploads")),
                ..Default::default()
            },
            NodeFieldDef {
                name: "allowed_kinds".to_string(),
                label: "Allowed file types".to_string(),
                field_type: NodeFieldType::MultiCheckbox,
                help: Some("Magic-byte validated. Images = jpg/png/webp/gif/etc.".to_string()),
                default_value: Some(json!(["images"])),
                options: vec![
                    SelectOptionDef {
                        value: "images".to_string(),
                        label: "Images (jpg, png, webp, gif, …)".to_string(),
                    },
                    SelectOptionDef {
                        value: "pdf".to_string(),
                        label: "PDF".to_string(),
                    },
                    SelectOptionDef {
                        value: "csv".to_string(),
                        label: "CSV".to_string(),
                    },
                    SelectOptionDef {
                        value: "json".to_string(),
                        label: "JSON".to_string(),
                    },
                    SelectOptionDef {
                        value: "glb".to_string(),
                        label: "3D Models (GLB)".to_string(),
                    },
                    SelectOptionDef {
                        value: "audio".to_string(),
                        label: "Audio".to_string(),
                    },
                    SelectOptionDef {
                        value: "video".to_string(),
                        label: "Video".to_string(),
                    },
                ],
                ..Default::default()
            },
            NodeFieldDef {
                name: "max_size_mb".to_string(),
                label: "Max size (MB)".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Maximum file size in megabytes (default: 10)".to_string()),
                default_value: Some(json!("10")),
                ..Default::default()
            },
            NodeFieldDef {
                name: "filename".to_string(),
                label: "Filename".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Custom filename without extension (default: random UUID).".to_string()),
                default_value: None,
                ..Default::default()
            },
        ].into_iter().chain(store_fields(OnConflict::Error)).collect(),
        layout: vec![
            LayoutItem::Field("source_key".to_string()),
            LayoutItem::Field("path".to_string()),
            LayoutItem::Field("folder".to_string()),
            LayoutItem::Field("allowed_kinds".to_string()),
            LayoutItem::Field("max_size_mb".to_string()),
            LayoutItem::Field("filename".to_string()),
            LayoutItem::Field("store".to_string()),
            LayoutItem::Field("on_conflict".to_string()),
        ],
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Photo with a caption", "fs.save --source-key files.photo --folder uploads --allowed-kinds images --max-size 10")
                .input(serde_json::json!({ "body": { "caption": "Sunset" }, "files": { "photo": { "__zf_type": "file_ref", "filename": "IMG_1.jpg", "mime": "image/jpeg", "size": 182331 } } }))
                .output(serde_json::json!({ "body": { "caption": "Sunset" }, "files": { "photo": { "__zf_type": "file_ref", "filename": "IMG_1.jpg", "mime": "image/jpeg", "size": 182331 } }, "saved": { "__zf_type": "file_ref", "backend": "zebfs", "ref": "uploads/3f9c….jpg", "filename": "3f9c….jpg", "mime": "image/jpeg", "kind": "image", "size": 182331, "sha256": "sha256:…", "lifecycle": "durable", "origin": "fs.save", "trust": "untrusted" } }))
                .note("Then `sekejap.query.run --read-only false --params \"{{ [input.body.caption, input.saved.ref] }}\" -- \"INSERT INTO photos (caption, path) VALUES ($1, $2)\"`. `saved` is a FileRef, so `fs.image.thumbnail`, `fs.file.copy --from \"{{ input.saved }}\"` and a `--preview image` all take it as it is."),
        ],
        ..Default::default()
    }
}

/// The file `fs.save` keeps: exactly what `--source-key` points at — an
/// upload, a FileRef or a byte envelope. Nothing else in the payload is
/// looked at (`node-conventions.md` §2).
pub fn locate_source<'a>(payload: &'a Value, key: &str) -> Result<&'a Value, PipelineError> {
    let key = if key.trim().is_empty() { "files.file" } else { key.trim() };
    resolve_path(payload, key)
        .filter(|value| value.is_object())
        .ok_or_else(|| {
            PipelineError::new(
                "FW_NODE_FS_SAVE",
                format!("no file at payload key '{key}' — an upload field is files.<name>; set --source-key"),
            )
        })
}

// ── Node ──────────────────────────────────────────────────────────────────────

pub struct Node {
    config: Config,
    platform: Arc<PlatformService>,
}

impl Node {
    pub fn new(config: Config, platform: Arc<PlatformService>) -> Result<Self, PipelineError> {
        Ok(Self { config, platform })
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

        // ── Locate the file in the payload ────────────────────────────────────
        let field = self.config.source_key.trim();
        let file_obj = locate_source(&input.payload, field)?;
        let max_bytes = (self.config.max_size_mb * 1024.0 * 1024.0) as usize;

        // Determine if this is a FileRef, __zf_bytes object, or legacy webhook file object.
        let is_file_ref = is_file_ref(file_obj);
        let is_zf_bytes = file_obj.get("__zf_bytes").is_some();

        // The bytes are not re-encoded here, only checked, so the stored
        // object is exactly as trusted as what arrived: an upload stays
        // `untrusted`, an operator's file stays `user`. The legacy shapes
        // predate the word and arrived from outside.
        let trust = file_obj
            .get("trust")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("untrusted")
            .to_string();
        let (original_name, browser_mime, size, bytes);
        if is_file_ref {
            original_name = file_obj
                .get("filename")
                .or_else(|| file_obj.get("name"))
                .and_then(|v| v.as_str())
                .unwrap_or("upload");
            browser_mime = file_obj
                .get("mime")
                .or_else(|| file_obj.get("content_type"))
                .and_then(|v| v.as_str())
                .unwrap_or("application/octet-stream");
            size = file_obj.get("size").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            // The limit is checked against the stored object before a byte is read.
            if let Some((source_store, key)) = open_source(&self.platform, owner, project, file_obj, None)?
                && let Ok(stat) = source_store.fs.head(&key)
                && stat.size as usize > max_bytes
            {
                return Err(PipelineError::new(
                    "FW_NODE_FS_SAVE",
                    format!("file size {} bytes exceeds limit of {} MB", stat.size, self.config.max_size_mb),
                ));
            }
            bytes = read_file_ref_bytes(&self.platform, owner, project, file_obj)?;
        } else if is_zf_bytes {
            original_name = "download";
            browser_mime = file_obj
                .get("__zf_mime")
                .and_then(|v| v.as_str())
                .unwrap_or("application/octet-stream");
            size = file_obj
                .get("__zf_size")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;
            let data_b64 = file_obj
                .get("__zf_bytes")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    PipelineError::new("FW_NODE_FS_SAVE", "__zf_bytes field is not a string")
                })?;
            // Decoded size is three quarters of the text: refuse before decoding.
            if data_b64.len() / 4 * 3 > max_bytes + 3 {
                return Err(PipelineError::new(
                    "FW_NODE_FS_SAVE",
                    format!("file exceeds limit of {} MB", self.config.max_size_mb),
                ));
            }
            bytes = base64::engine::general_purpose::STANDARD
                .decode(data_b64)
                .map_err(|err| {
                    PipelineError::new("FW_NODE_FS_SAVE", format!("base64 decode error: {err}"))
                })?;
        } else {
            original_name = file_obj
                .get("filename")
                .and_then(|v| v.as_str())
                .unwrap_or("upload");
            browser_mime = file_obj
                .get("content_type")
                .and_then(|v| v.as_str())
                .unwrap_or("application/octet-stream");
            size = file_obj.get("size").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
            let data_b64 = file_obj
                .get("data")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    PipelineError::new("FW_NODE_FS_SAVE", "input.files.{field}.data is missing")
                })?;
            // Decoded size is three quarters of the text: refuse before decoding.
            if data_b64.len() / 4 * 3 > max_bytes + 3 {
                return Err(PipelineError::new(
                    "FW_NODE_FS_SAVE",
                    format!("file exceeds limit of {} MB", self.config.max_size_mb),
                ));
            }
            bytes = base64::engine::general_purpose::STANDARD
                .decode(data_b64)
                .map_err(|err| {
                    PipelineError::new("FW_NODE_FS_SAVE", format!("base64 decode error: {err}"))
                })?;
        }

        // ── Validate size (pre-decode, from reported size) ────────────────────
        if size > max_bytes {
            return Err(PipelineError::new(
                "FW_NODE_FS_SAVE",
                format!(
                    "file size {} bytes exceeds limit of {} MB",
                    size, self.config.max_size_mb
                ),
            ));
        }

        // Re-check against actual decoded size
        if bytes.len() > max_bytes {
            return Err(PipelineError::new(
                "FW_NODE_FS_SAVE",
                format!(
                    "decoded file size {} bytes exceeds limit of {} MB",
                    bytes.len(),
                    self.config.max_size_mb
                ),
            ));
        }

        // ── Magic-byte content inspection ─────────────────────────────────────
        // We use the `infer` crate to determine the real file type from the first bytes.
        // This is the only reliable check — the browser-reported MIME cannot be trusted.
        let allowed = &self.config.allowed_kinds;
        if allowed.is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_FS_SAVE",
                "no file types are allowed — enable at least one in allowed_kinds",
            ));
        }

        let inferred_mime = detect_binary_mime(&bytes).map(normalized_mime);

        // Special case: SVG and JSON/CSV are text — infer won't detect them from bytes.
        // Fall back to browser MIME for those, but still check against the allowed list.
        let effective_mime = match inferred_mime.as_deref() {
            Some(mime) => mime.to_string(),
            None => {
                let fallback_mime = normalized_mime(browser_mime);
                let text_allowed = [
                    "image/svg+xml",
                    "application/json",
                    "text/json",
                    "text/csv",
                    "text/plain",
                ];
                if text_allowed.contains(&fallback_mime.as_str()) {
                    fallback_mime
                } else {
                    return Err(PipelineError::new(
                        "FW_NODE_FS_SAVE",
                        format!(
                            "file type could not be determined from content (browser reported: '{browser_mime}'). \
                            Upload a supported file type."
                        ),
                    ));
                }
            }
        };

        if !kind_accepts_mime(allowed, &effective_mime) {
            let allowed_labels: Vec<&str> = allowed.iter().map(|kind| kind.label()).collect();
            return Err(PipelineError::new(
                "FW_NODE_FS_SAVE",
                format!(
                    "file content is '{effective_mime}', which is not in allowed types: {}",
                    allowed_labels.join(", ")
                ),
            ));
        }

        // Detect MIME mismatch (potential spoofing: browser says image/jpeg, content is application/pdf)
        if let Some(inferred) = inferred_mime.as_deref() {
            if !browser_mime_matches_detected(browser_mime, inferred) {
                return Err(PipelineError::new(
                    "FW_NODE_FS_SAVE",
                    format!(
                        "MIME mismatch: browser declared '{browser_mime}' but file content is '{inferred}'. \
                        Possible spoofing attempt rejected."
                    ),
                ));
            }
        }

        // ── Determine ZebFS object path ───────────────────────────────────────
        // Normalised (and `..` refused) by `target_key`, never quietly cleaned.
        let folder = if self.config.folder.trim().is_empty() { "uploads" } else { self.config.folder.trim() };

        let ext = safe_extension(original_name, &effective_mime);
        let storage_name = {
            let custom = self
                .config
                .filename
                .as_deref()
                .map(|filename| sanitize_filename(filename.trim()))
                .filter(|name| !name.is_empty());

            match (custom, ext.is_empty()) {
                (Some(name), true) => name,
                (Some(name), false) => format!("{name}.{ext}"),
                (None, true) => Uuid::new_v4().to_string(),
                (None, false) => format!("{}.{}", Uuid::new_v4(), ext),
            }
        };

        let configured_path = self
            .config
            .path
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        if configured_path.as_deref().is_some_and(|path| path.ends_with('/')) {
            return Err(PipelineError::new(
                "FW_NODE_FS_SAVE",
                "--path is the exact key of the file; a folder is --folder",
            ));
        }
        let rel_path = target_key(configured_path.as_deref(), folder, &storage_name, "FW_NODE_FS_SAVE")?;

        // ── Write to the node's store ─────────────────────────────────────────
        let store = open_store(&self.platform, owner, project, self.config.store.as_deref())?;
        let on_conflict = OnConflict::parse(self.config.on_conflict.as_deref(), OnConflict::Error, "FW_NODE_FS_SAVE")?;
        let written = on_conflict.allows(&store.fs, &rel_path, "FW_NODE_FS_SAVE")?;
        // The form's other fields ride along: an upload form has a title and a
        // caption beside the file, and the INSERT after this node needs them.
        let saved = if written {
            store
                .fs
                .put(&rel_path, &bytes)
                .map_err(|err| PipelineError::new("FW_NODE_FS_SAVE", err.to_string()))?;
            store.file_ref(&rel_path, &storage_name, &effective_mime, &bytes, "fs.save", &trust)
        } else {
            // Skipped: the answer is the object already there, as it is.
            store.stored_ref(&rel_path, "fs.save", "untrusted", "FW_NODE_FS_SAVE")?
        };
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: crate::pipeline::nodes::shared::util::with_answer(&input.payload, serde_json::json!({ "saved": saved })),
            trace: vec![format!(
                "node_kind={NODE_KIND} field={field} path={rel_path} store={} written={written}",
                store.id
            )],
        })
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Extract a safe, lowercase file extension from the original filename or MIME type.
/// Only alphanumeric chars, max 10 chars.
fn safe_extension(original_name: &str, mime: &str) -> String {
    if let Some(ext) = std::path::Path::new(original_name).extension() {
        if let Some(value) = ext.to_str() {
            let ext: String = value
                .chars()
                .filter(|char| char.is_alphanumeric())
                .take(10)
                .collect::<String>()
                .to_lowercase();
            if !ext.is_empty() {
                return ext;
            }
        }
    }

    let mime = normalized_mime(mime);
    match mime.as_str() {
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        "image/bmp" => "bmp",
        "image/tiff" => "tif",
        "image/svg+xml" => "svg",
        "image/avif" => "avif",
        "image/heic" => "heic",
        "image/heif" => "heif",
        "application/pdf" => "pdf",
        "text/plain" => "txt",
        "text/csv" => "csv",
        "application/json" => "json",
        "text/json" => "json",
        "application/zip" => "zip",
        "video/mp4" => "mp4",
        "video/webm" => "webm",
        "video/ogg" => "ogv",
        "video/x-m4v" => "m4v",
        "audio/mpeg" => "mp3",
        "audio/wav" => "wav",
        "audio/ogg" => "ogg",
        "audio/mp4" => "m4a",
        "model/gltf-binary" => "glb",
        "application/gzip" | "application/x-gzip" => "gz",
        "application/x-tar" => "tar",
        "application/x-bzip2" => "bz2",
        "application/x-7z-compressed" => "7z",
        "application/x-rar-compressed" => "rar",
        "application/vnd.android.package-archive" => "apk",
        "application/java-archive" => "jar",
        _ => "",
    }
    .to_string()
}

/// Sanitize a user-provided filename: keep alphanumeric, dash, underscore only.
/// Strips any extension (the caller adds extension from content type).
/// Returns empty string if nothing remains (caller falls back to UUID).
fn sanitize_filename(name: &str) -> String {
    let stem = std::path::Path::new(name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(name);
    let sanitized: String = stem
        .chars()
        .map(|char| {
            if char.is_alphanumeric() || char == '-' || char == '_' {
                char
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = sanitized.trim_matches('_');
    trimmed.chars().take(200).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::nodes::shared::file_ref::durable_file_ref;

    fn file_ref(name: &str) -> Value {
        serde_json::json!({ "__zf_type": "file_ref", "backend": "zebfs", "store": "local", "ref": format!("tmp/runs/r/files/{name}"), "filename": name,
            "mime": "image/png", "kind": "image", "size": 1, "sha256": format!("sha256:{}", "0".repeat(64)),
            "lifecycle": "temporary", "origin": "fs.image.render", "trust": "generated" })
    }

    #[test]
    fn the_source_is_exactly_where_source_key_points() {
        let upload = serde_json::json!({ "files": { "file": file_ref("a.png"), "photo": file_ref("b.png") }, "image": file_ref("p.png") });
        assert_eq!(locate_source(&upload, "").unwrap()["filename"], "a.png");
        assert_eq!(locate_source(&upload, "files.photo").unwrap()["filename"], "b.png");
        assert_eq!(locate_source(&upload, "image").unwrap()["filename"], "p.png");
        // Nothing is guessed: no fallback to another key or a lone FileRef.
        let product = serde_json::json!({ "image": file_ref("p.png") });
        assert!(locate_source(&product, "").is_err());
        assert!(locate_source(&serde_json::json!({ "image": "not a file" }), "image").is_err());
    }

    #[test]
    fn allowed_kinds_flag_uses_comma_separated_list() {
        let flag = definition()
            .dsl_flags
            .into_iter()
            .find(|flag| flag.flag == "--allowed-kinds")
            .expect("missing --allowed-kinds");
        assert_eq!(flag.kind, DslFlagKind::CommaSeparatedList);
    }

    #[test]
    fn audio_aliases_are_accepted() {
        assert!(kind_accepts_mime(&[AllowedKind::Audio], "audio/x-wav"));
        assert!(kind_accepts_mime(&[AllowedKind::Audio], "audio/m4a"));
    }

    #[test]
    fn detects_glb_by_header() {
        let mut bytes = b"glTF".to_vec();
        bytes.extend_from_slice(&2u32.to_le_bytes());
        bytes.extend_from_slice(&(12u32).to_le_bytes());
        assert_eq!(detect_binary_mime(&bytes), Some("model/gltf-binary"));
    }

    #[test]
    fn detects_ogg_theora_as_video() {
        let mut bytes = vec![0; 35];
        bytes[0..4].copy_from_slice(b"OggS");
        bytes[26] = 1;
        bytes[27] = 7;
        bytes[28..35].copy_from_slice(b"\x80theora");
        assert_eq!(detect_binary_mime(&bytes), Some("video/ogg"));
    }

    #[test]
    fn generic_browser_mime_is_allowed_when_content_is_verified() {
        assert!(browser_mime_matches_detected(
            "application/octet-stream",
            "model/gltf-binary"
        ));
    }

    /// `saved` is a FileRef the contract accepts — the eleven fields, and
    /// nothing beside them: no `path`, `url`, `original_name`, `content_type`.
    #[test]
    fn saved_is_exactly_a_durable_file_ref() {
        let bytes = b"\x89PNG\r\n\x1a\nnot really a png";
        let saved = durable_file_ref(crate::zebfs::FileBackend::Zebfs, "local", "uploads/abc.png", "abc.png", "image/png", bytes, "fs.save", "untrusted");
        crate::pipeline::nodes::shared::file_ref::validate_file_ref(&saved).expect("a contract FileRef");
        let mut keys: Vec<&str> = saved.as_object().unwrap().keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            ["__zf_type", "backend", "filename", "kind", "lifecycle", "mime", "origin", "ref", "sha256", "size", "store", "trust"]
        );
        assert_eq!(saved["__zf_type"], "file_ref");
        assert_eq!(saved["ref"], "uploads/abc.png");
        assert_eq!(saved["filename"], "abc.png");
        assert_eq!(saved["mime"], "image/png");
        assert_eq!(saved["kind"], "image");
        assert_eq!(saved["lifecycle"], "durable");
        assert_eq!(saved["origin"], "fs.save");
        assert_eq!(saved["trust"], "untrusted");
        assert_eq!(saved["size"], bytes.len());
        assert!(saved["sha256"].as_str().unwrap().starts_with("sha256:"));
    }

    #[test]
    fn safe_extension_knows_new_formats() {
        assert_eq!(safe_extension("upload", "model/gltf-binary"), "glb");
        assert_eq!(safe_extension("upload", "audio/x-wav"), "wav");
        assert_eq!(safe_extension("upload", "video/webm"), "webm");
        assert_eq!(safe_extension("upload", "audio/m4a"), "m4a");
    }
}
