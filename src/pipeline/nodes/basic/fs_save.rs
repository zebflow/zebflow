//! n.fs.save — validate and promote one uploaded/intermediate file to Zebflow FS.
//!
//! For general node authoring rules, read `src/pipeline/nodes/mod.rs`; for
//! FileRef IR and backend/lifecycle rules, read
//! `src/pipeline/nodes/basic/file_ref.rs`.
//!
//! Input: `input.files.{field}` as set by `trigger.webhook` multipart parsing,
//! where `{field}` is a dot-path so array uploads can be addressed as `photos.0`.
//! Output: `{ saved: { path, url, original_name, content_type, size } }`
//!
//! Files are stored as durable ZebFS object paths, usually `uploads/{uuid}.{ext}`.
//!
//! Content validation uses magic-byte inspection (via the `infer` crate) in addition to
//! the browser-reported MIME type. Both must agree, and both must match the allowed-types list.

use std::sync::Arc;

use async_trait::async_trait;
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;

use super::file_ref::{is_file_ref, read_file_ref_bytes};
use super::util::{metadata_scope, resolve_path};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    model::{DslFlag, DslFlagKind, LayoutItem, NodeFieldDef, NodeFieldType, SelectOptionDef},
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;
use crate::zebfs::LocalZebFs;

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

fn default_field() -> String {
    "file".to_string()
}

fn default_max_size_mb() -> f64 {
    10.0
}

// ── Config ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Multipart field name (default: "file").
    #[serde(default = "default_field")]
    pub field: String,

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
}

impl Default for Config {
    fn default() -> Self {
        Self {
            field: default_field(),
            path: None,
            folder: String::new(),
            allowed_kinds: default_allowed_kinds(),
            max_size_mb: default_max_size_mb(),
            filename: None,
        }
    }
}

// ── Definition ────────────────────────────────────────────────────────────────

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "FS Save".to_string(),
        description: "Save an uploaded file from a multipart webhook payload to Zebflow FS \
            storage. Reads `input.files.{field}` (set by trigger.webhook), validates MIME type \
            AND magic bytes (content inspection), checks size, then writes to an object path such as \
            `uploads/{uuid}.{ext}`. \
            Output: `{ saved: { path, url, original_name, content_type, size } }`."
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
                    "properties": {
                "path":          { "type": "string", "description": "ZebFS object path" },
                "url":           { "type": "string", "description": "ZebFS object URL" },
                "original_name": { "type": "string" },
                "content_type":  { "type": "string" },
                "size":          { "type": "integer" }
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
                flag: "--field".to_string(),
                config_key: "field".to_string(),
                description: "Multipart field name (default: \"file\")".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--path".to_string(),
                config_key: "path".to_string(),
                description:
                    "Exact ZebFS object path. If omitted, folder + generated filename is used."
                        .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--folder".to_string(),
                config_key: "folder".to_string(),
                description: "ZebFS object folder (default: \"uploads\")".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--allowed-kinds".to_string(),
                config_key: "allowed_kinds".to_string(),
                description:
                    "Comma-separated allowed categories: images,pdf,csv,json,glb,audio,video (default: images)"
                        .to_string(),
                kind: DslFlagKind::CommaSeparatedList,
                required: false,
            },
            DslFlag {
                flag: "--max-size".to_string(),
                config_key: "max_size_mb".to_string(),
                description: "Maximum file size in MB (default: 10)".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--filename".to_string(),
                config_key: "filename".to_string(),
                description:
                    "Custom filename without extension (default: random UUID). Overwrites if same name exists."
                        .to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: vec![
            NodeFieldDef {
                name: "field".to_string(),
                label: "Field name".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Multipart field name from the upload form (default: \"file\")".to_string()),
                default_value: Some(json!("file")),
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
                help: Some("Custom filename without extension (default: random UUID). File with same name will be overwritten.".to_string()),
                default_value: None,
                ..Default::default()
            },
        ],
        layout: vec![
            LayoutItem::Field("field".to_string()),
            LayoutItem::Field("path".to_string()),
            LayoutItem::Field("folder".to_string()),
            LayoutItem::Field("allowed_kinds".to_string()),
            LayoutItem::Field("max_size_mb".to_string()),
            LayoutItem::Field("filename".to_string()),
        ],
        ai_tool: crate::pipeline::model::NodeAiToolDefinition {
            registered: false,
            tool_name: String::new(),
            tool_description: String::new(),
            tool_input_schema: json!({}),
        },
        ..Default::default()
    }
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
        let field = if self.config.field.trim().is_empty() {
            "file"
        } else {
            self.config.field.trim()
        };

        // Primary source: input.files.{field} (multipart webhook convention).
        // `field` is resolved as a dot-path so array uploads can use e.g. `photos.0`.
        let file_from_webhook = input
            .payload
            .get("files")
            .and_then(|files| resolve_path(files, field));

        // Fallback: response.body from n.http.request --response-type bytes (FileRef now,
        // __zf_bytes in older payloads).
        let file_obj = match file_from_webhook {
            Some(obj) => obj,
            None => {
                let zf_body = input
                    .payload
                    .get("response")
                    .and_then(|r| r.get("body"))
                    .filter(|b| is_file_ref(b) || b.get("__zf_bytes").is_some());
                zf_body.ok_or_else(|| {
                    PipelineError::new(
                        "FW_NODE_FILE_SAVE",
                        format!(
                            "input.files.{field} not found and no FileRef/__zf_bytes in response.body — \
                            is this triggered by a multipart webhook or http.request with --response-type bytes?"
                        ),
                    )
                })?
            }
        };

        // Determine if this is a FileRef, __zf_bytes object, or legacy webhook file object.
        let is_file_ref = is_file_ref(file_obj);
        let is_zf_bytes = file_obj.get("__zf_bytes").is_some();

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
                    PipelineError::new("FW_NODE_FILE_SAVE", "__zf_bytes field is not a string")
                })?;
            bytes = base64::engine::general_purpose::STANDARD
                .decode(data_b64)
                .map_err(|err| {
                    PipelineError::new("FW_NODE_FILE_SAVE", format!("base64 decode error: {err}"))
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
                    PipelineError::new("FW_NODE_FILE_SAVE", "input.files.{field}.data is missing")
                })?;
            bytes = base64::engine::general_purpose::STANDARD
                .decode(data_b64)
                .map_err(|err| {
                    PipelineError::new("FW_NODE_FILE_SAVE", format!("base64 decode error: {err}"))
                })?;
        }

        // ── Validate size (pre-decode, from reported size) ────────────────────
        let max_bytes = (self.config.max_size_mb * 1024.0 * 1024.0) as usize;
        if size > max_bytes {
            return Err(PipelineError::new(
                "FW_NODE_FILE_SAVE",
                format!(
                    "file size {} bytes exceeds limit of {} MB",
                    size, self.config.max_size_mb
                ),
            ));
        }

        // Re-check against actual decoded size
        if bytes.len() > max_bytes {
            return Err(PipelineError::new(
                "FW_NODE_FILE_SAVE",
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
                "FW_NODE_FILE_SAVE",
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
                        "FW_NODE_FILE_SAVE",
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
                "FW_NODE_FILE_SAVE",
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
                    "FW_NODE_FILE_SAVE",
                    format!(
                        "MIME mismatch: browser declared '{browser_mime}' but file content is '{inferred}'. \
                        Possible spoofing attempt rejected."
                    ),
                ));
            }
        }

        // ── Determine ZebFS object path ───────────────────────────────────────
        let folder = sanitize_dest_path(if self.config.folder.trim().is_empty() {
            "uploads"
        } else {
            self.config.folder.trim()
        });

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
            .map(sanitize_dest_path);
        let rel_path = match configured_path.filter(|value| !value.is_empty()) {
            Some(path) if path.ends_with('/') => format!("{path}{storage_name}"),
            Some(path) => path,
            None => format!("{folder}/{storage_name}"),
        };

        // ── Write to disk ──────────────────────────────────────────────────────
        let layout = self
            .platform
            .file
            .ensure_project_layout(owner, project)
            .map_err(|err| PipelineError::new("FW_NODE_FILE_SAVE", err.to_string()))?;

        let zebfs = LocalZebFs::new(layout.files_dir);
        zebfs
            .put(&rel_path, &bytes)
            .map_err(|err| PipelineError::new("FW_NODE_FILE_SAVE", err.to_string()))?;

        let url = format!("/fs/{owner}/{project}/{rel_path}");

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: json!({
                "saved": {
                    "path": rel_path,
                    "url": url,
                    "original_name": original_name,
                    "content_type": effective_mime,
                    "size": bytes.len(),
                }
            }),
            trace: vec![format!(
                "node_kind={NODE_KIND} field={field} path={rel_path}"
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

/// Sanitize a dest path: strip leading/trailing slashes, reject `..` and `.` components.
fn sanitize_dest_path(dest: &str) -> String {
    dest.split('/')
        .filter(|segment| !segment.is_empty() && *segment != "." && *segment != "..")
        .collect::<Vec<_>>()
        .join("/")
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

    #[test]
    fn safe_extension_knows_new_formats() {
        assert_eq!(safe_extension("upload", "model/gltf-binary"), "glb");
        assert_eq!(safe_extension("upload", "audio/x-wav"), "wav");
        assert_eq!(safe_extension("upload", "video/webm"), "webm");
        assert_eq!(safe_extension("upload", "audio/m4a"), "m4a");
    }
}
