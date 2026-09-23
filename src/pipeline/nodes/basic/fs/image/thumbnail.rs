//! n.fs.image.thumbnail — resize + compress an uploaded image into a small thumbnail.
//!
//! Reads `input.saved` by default (the FileRef `fs.save` answers, or a store path string), or a custom `--source-key` dot-path into
//! the payload. The source may be a string path or FileRef. Produces a resized,
//! re-encoded file and injects `thumbnail` FileRef metadata into the payload.
//!
//! Fit modes:
//!   cover   — scale to fill the target box, crop center (default)
//!   contain — scale to fit within the box, preserving aspect ratio
//!   fill    — stretch to exact dimensions, ignoring aspect ratio
//!
//! Formats: jpg (quality-controlled, default), png (lossless), webp (lossless)
//!
//! Decompression-bomb protection: the family's `load_with_limits` (16 000 px a
//! side, 128 MB decoded).

use std::io::Cursor;
use std::sync::Arc;

use async_trait::async_trait;
use image::{DynamicImage, GenericImageView, ImageFormat, imageops::FilterType};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use super::load_with_limits;
use crate::pipeline::nodes::shared::file_ref::{BACKEND_ZEBFS, FILE_REF_TYPE, LIFECYCLE_DURABLE, zebfs_rel_path_or_string};
use crate::pipeline::nodes::shared::util::{filename_stem, metadata_scope, resolve_path};
use crate::pipeline::model::NodeCapability;
use crate::pipeline::{
    NodeDefinition, PipelineError,
    model::{DslFlag, DslFlagKind, LayoutItem, NodeFieldDef, NodeFieldType, SelectOptionDef},
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;

pub const NODE_KIND: &str = "n.fs.image.thumbnail";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

fn default_width() -> u32 {
    256
}
fn default_height() -> u32 {
    256
}
fn default_fit() -> String {
    "cover".to_string()
}
fn default_format() -> String {
    "jpg".to_string()
}
fn default_quality() -> u8 {
    82
}
fn default_folder() -> String {
    "thumbnails".to_string()
}
fn default_source_key() -> String {
    "saved".to_string()
}
fn default_delete_source() -> bool {
    false
}

// ── Config ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Target width in pixels (default: 256).
    #[serde(default = "default_width")]
    pub width: u32,

    /// Target height in pixels (default: 256).
    #[serde(default = "default_height")]
    pub height: u32,

    /// Resize strategy: "cover" | "contain" | "fill" (default: "cover").
    #[serde(default = "default_fit")]
    pub fit: String,

    /// Output format: "jpg" | "png" | "webp" (default: "jpg").
    #[serde(default = "default_format")]
    pub format: String,

    /// JPEG quality 1–100 (default: 82). Ignored for PNG and WebP (lossless).
    #[serde(default = "default_quality")]
    pub quality: u8,

    /// Destination ZebFS object folder (default: "thumbnails").
    #[serde(default = "default_folder")]
    pub folder: String,

    /// Dot-path into the payload for the source file path
    /// (default: "saved" — the FileRef n.fs.save answers).
    #[serde(default = "default_source_key")]
    pub source_key: String,

    /// Delete the source file after the thumbnail is successfully written (default: false).
    /// Useful to remove the raw original once the safe re-encoded thumbnail exists.
    /// The delete is best-effort and non-fatal — pipeline continues even if it fails.
    #[serde(default = "default_delete_source")]
    pub delete_source: bool,

    /// Optional custom filename (without extension). If set, used instead of UUID.
    /// Useful for deterministic thumbnail paths (e.g. user avatars).
    #[serde(default)]
    pub filename: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            width: default_width(),
            height: default_height(),
            fit: default_fit(),
            format: default_format(),
            quality: default_quality(),
            folder: default_folder(),
            source_key: default_source_key(),
            delete_source: default_delete_source(),
            filename: None,
        }
    }
}

// ── Definition ─────────────────────────────────────────────────────────────

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Filesystem],
        title: "Image Thumbnail".to_string(),
        description:
            "Make a small image from a stored one. Reads the source at `--source-key` (default `saved` — the FileRef `fs.save` answers, or a store path string — i.e. right after \
             `fs.save`), resizes to `--width`×`--height` with `--fit cover|contain|fill`, writes `--format jpg|png|webp` into `--folder` \
             (default `thumbnails/`; use `public/…` for a page to show it anonymously). Adds `thumbnail` (a FileRef: `ref`, `filename`, \
             `width`, `height`, `format`, `size`) to the payload and keeps the rest; with `--delete-source` the original is removed \
             and its key (`saved`) dropped. Store `thumbnail.ref` in the row."
            .to_string(),
        input_schema: json!({
            "type": "object",
            "description": "Payload must contain the source — a FileRef or a store path string — at the dot-key specified by source_key (default: `saved`)."
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "thumbnail": {
                    "type": "object",
                    "description": "A FileRef (kinds/file-ref/README.md) plus the image dimensions. The object path is `ref`; there is no `path` or `url` field.",
                    "properties": {
                        "ref":    { "type": "string" },
                        "width":  { "type": "integer" },
                        "height": { "type": "integer" },
                        "format": { "type": "string" },
                        "size":   { "type": "integer" }
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
                flag: "--width".to_string(),
                config_key: "width".to_string(),
                description: "Target width in pixels (default: 256)".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--height".to_string(),
                config_key: "height".to_string(),
                description: "Target height in pixels (default: 256)".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--fit".to_string(),
                config_key: "fit".to_string(),
                description: "cover | contain | fill (default: cover)".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--format".to_string(),
                config_key: "format".to_string(),
                description: "jpg | png | webp (default: jpg)".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--quality".to_string(),
                config_key: "quality".to_string(),
                description: "JPEG quality 1–100 (default: 82)".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--folder".to_string(),
                config_key: "folder".to_string(),
                description: "Destination ZebFS object folder (default: thumbnails)".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--source-key".to_string(),
                config_key: "source_key".to_string(),
                description: "Dot-path to the source in the payload: a FileRef or a store path string (default: `saved`)".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--delete-source".to_string(),
                config_key: "delete_source".to_string(),
                description: "Delete the source file after successful thumbnail write (default: false)".to_string(),
                kind: DslFlagKind::Bool,
                required: false,
            },
            DslFlag {
                flag: "--filename".to_string(),
                config_key: "filename".to_string(),
                description: "Custom filename without extension (default: random UUID). Overwrites if same name exists.".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: vec![
            NodeFieldDef {
                name: "width".to_string(),
                label: "Width (px)".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Target thumbnail width in pixels (default: 256)".to_string()),
                default_value: Some(json!("256")),
                ..Default::default()
            },
            NodeFieldDef {
                name: "height".to_string(),
                label: "Height (px)".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Target thumbnail height in pixels (default: 256)".to_string()),
                default_value: Some(json!("256")),
                ..Default::default()
            },
            NodeFieldDef {
                name: "fit".to_string(),
                label: "Fit".to_string(),
                field_type: NodeFieldType::Select,
                help: Some("cover = fill box and crop center. contain = fit within box. fill = stretch to exact size.".to_string()),
                default_value: Some(json!("cover")),
                options: vec![
                    SelectOptionDef { value: "cover".to_string(),   label: "Cover (fill + crop center)".to_string() },
                    SelectOptionDef { value: "contain".to_string(), label: "Contain (fit within)".to_string() },
                    SelectOptionDef { value: "fill".to_string(),    label: "Fill (stretch exact)".to_string() },
                ],
                ..Default::default()
            },
            NodeFieldDef {
                name: "format".to_string(),
                label: "Output format".to_string(),
                field_type: NodeFieldType::Select,
                help: Some("jpg = quality-controlled lossy (best for photos). png = lossless. webp = lossless WebP.".to_string()),
                default_value: Some(json!("jpg")),
                options: vec![
                    SelectOptionDef { value: "jpg".to_string(),  label: "JPEG (quality-controlled)".to_string() },
                    SelectOptionDef { value: "png".to_string(),  label: "PNG (lossless)".to_string() },
                    SelectOptionDef { value: "webp".to_string(), label: "WebP (lossless)".to_string() },
                ],
                ..Default::default()
            },
            NodeFieldDef {
                name: "quality".to_string(),
                label: "JPEG quality".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("1–100, default 82. Only applies to JPEG output.".to_string()),
                default_value: Some(json!("82")),
                ..Default::default()
            },
            NodeFieldDef {
                name: "folder".to_string(),
                label: "Folder".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Destination ZebFS object folder (default: thumbnails)".to_string()),
                default_value: Some(json!("thumbnails")),
                ..Default::default()
            },
            NodeFieldDef {
                name: "source_key".to_string(),
                label: "Source path key".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Dot-path into the payload for the source: a FileRef or a store path string. Default: saved (what n.fs.save answers).".to_string()),
                default_value: Some(json!("saved")),
                ..Default::default()
            },
            NodeFieldDef {
                name: "delete_source".to_string(),
                label: "Delete source file".to_string(),
                field_type: NodeFieldType::Checkbox,
                help: Some("Delete the original source file after the thumbnail is successfully written. Useful to discard the raw upload once a safe re-encoded thumbnail exists.".to_string()),
                default_value: Some(json!(false)),
                ..Default::default()
            },
            NodeFieldDef {
                name: "filename".to_string(),
                label: "Filename".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Custom filename without extension (default: random UUID). Thumbnail with same name will be overwritten.".to_string()),
                default_value: None,
                ..Default::default()
            },
        ],
        layout: vec![
            LayoutItem::Field("width".to_string()),
            LayoutItem::Field("height".to_string()),
            LayoutItem::Field("fit".to_string()),
            LayoutItem::Field("format".to_string()),
            LayoutItem::Field("quality".to_string()),
            LayoutItem::Field("folder".to_string()),
            LayoutItem::Field("source_key".to_string()),
            LayoutItem::Field("delete_source".to_string()),
            LayoutItem::Field("filename".to_string()),
        ],
        examples: vec![
            crate::pipeline::model::NodeExample::dsl("Avatar after an upload", "fs.image.thumbnail --width 320 --height 320 --fit cover --format webp --folder public/thumbs")
                .input(serde_json::json!({ "body": { "caption": "Sunset" }, "saved": { "__zf_type": "file_ref", "backend": "zebfs", "ref": "uploads/3f9c….jpg", "filename": "3f9c….jpg", "mime": "image/jpeg", "kind": "image", "size": 182331, "sha256": "sha256:…", "lifecycle": "durable", "origin": "fs.save", "trust": "untrusted" } }))
                .output(serde_json::json!({ "body": { "caption": "Sunset" }, "saved": { "__zf_type": "file_ref", "backend": "zebfs", "ref": "uploads/3f9c….jpg", "filename": "3f9c….jpg", "mime": "image/jpeg", "kind": "image", "size": 182331, "sha256": "sha256:…", "lifecycle": "durable", "origin": "fs.save", "trust": "untrusted" }, "thumbnail": { "__zf_type": "file_ref", "backend": "zebfs", "ref": "public/thumbs/9a1d….webp", "filename": "9a1d….webp", "mime": "image/webp", "width": 320, "height": 320, "format": "webp", "size": 8120 } })),
        ],
        ..Default::default()
    }
}

// ── Node ───────────────────────────────────────────────────────────────────

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

        // ── Resolve source path from payload ──────────────────────────────
        let source_key = if self.config.source_key.trim().is_empty() {
            "saved"
        } else {
            self.config.source_key.trim()
        };

        let source_value = resolve_path(&input.payload, source_key).ok_or_else(|| {
            PipelineError::new(
                "IMG_THUMBNAIL",
                format!(
                    "source path not found at payload key '{source_key}' — chain after n.fs.save or set --source-key"
                ),
            )
        })?;
        let rel_path = zebfs_rel_path_or_string(source_value)?.ok_or_else(|| {
            PipelineError::new(
                "IMG_THUMBNAIL",
                format!("payload key '{source_key}' must be a FileRef or string path"),
            )
        })?;

        // ── Resolve absolute path ─────────────────────────────────────────
        let layout = self
            .platform
            .file
            .ensure_project_layout(owner, project)
            .map_err(|e| PipelineError::new("IMG_THUMBNAIL", e.to_string()))?;

        let abs_path = layout.local_files_dir()?.join(&rel_path);
        if !abs_path.exists() {
            return Err(PipelineError::new(
                "IMG_THUMBNAIL",
                format!("source file not found: {rel_path}"),
            ));
        }

        // ── Reject unsupported formats before loading ─────────────────────
        let ext = abs_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();
        if matches!(ext.as_str(), "svg" | "heic" | "heif") {
            return Err(PipelineError::new(
                "IMG_THUMBNAIL",
                format!(
                    "unsupported format for thumbnailing: .{ext} — convert to jpg/png/webp first"
                ),
            ));
        }

        // ── Load with decompression-bomb limits ───────────────────────────
        let raw_bytes = std::fs::read(&abs_path)
            .map_err(|e| PipelineError::new("IMG_THUMBNAIL", format!("read error: {e}")))?;

        let img = load_with_limits(&raw_bytes)?;

        // ── Resize ────────────────────────────────────────────────────────
        let target_w = self.config.width.max(1);
        let target_h = self.config.height.max(1);
        let resized = match self.config.fit.trim() {
            "contain" => resize_contain(&img, target_w, target_h),
            "fill" => resize_fill(&img, target_w, target_h),
            _ => resize_cover(&img, target_w, target_h), // default: cover
        };

        let (actual_w, actual_h) = resized.dimensions();

        // ── Encode ────────────────────────────────────────────────────────
        let quality = self.config.quality.clamp(1, 100);
        let (encoded, ext_out, format_label) =
            encode_image(&resized, &self.config.format, quality)?;

        // ── Write to disk ─────────────────────────────────────────────────
        let folder = sanitize_folder(if self.config.folder.trim().is_empty() {
            "thumbnails"
        } else {
            self.config.folder.trim()
        });

        let storage_name = {
            let custom = self
                .config
                .filename
                .as_deref()
                .map(filename_stem)
                .filter(|s| !s.is_empty());
            match custom {
                Some(name) => format!("{name}.{ext_out}"),
                None => format!("{}.{ext_out}", Uuid::new_v4()),
            }
        };
        let thumb_rel = format!("{folder}/{storage_name}");
        let abs_dest = layout.local_files_dir()?.join(&thumb_rel);

        if let Some(parent) = abs_dest.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| PipelineError::new("IMG_THUMBNAIL", format!("mkdir: {e}")))?;
        }

        // Atomic write: temp → rename
        let tmp_dest = abs_dest.with_extension(format!("{ext_out}.tmp"));
        std::fs::write(&tmp_dest, &encoded)
            .map_err(|e| PipelineError::new("IMG_THUMBNAIL", format!("write: {e}")))?;
        std::fs::rename(&tmp_dest, &abs_dest)
            .map_err(|e| PipelineError::new("IMG_THUMBNAIL", format!("rename: {e}")))?;

        // ── Delete source file if requested ───────────────────────────────
        // Best-effort: non-fatal. Thumbnail is already written successfully.
        if self.config.delete_source {
            if let Err(e) = std::fs::remove_file(&abs_path) {
                eprintln!("[IMG_THUMBNAIL] delete-source failed for {rel_path}: {e}");
            }
        }

        let thumb_size = encoded.len();
        let thumb_mime = match format_label {
            "png" => "image/png",
            "webp" => "image/webp",
            _ => "image/jpeg",
        };
        let thumbnail = json!({
            "__zf_type": FILE_REF_TYPE,
            "backend": BACKEND_ZEBFS,
            "ref": thumb_rel,
            "filename": storage_name,
            "mime": thumb_mime,
            "kind": "image",
            "size": thumb_size,
            "sha256": format!("sha256:{:x}", Sha256::digest(&encoded)),
            "lifecycle": LIFECYCLE_DURABLE,
            "origin": "fs.image.thumbnail",
            "trust": "sanitized",
            "width": actual_w,
            "height": actual_h,
            "format": format_label,
        });

        // Merge, do not replace: the form fields and `saved` are still needed by
        // the INSERT that follows. With `--delete-source` the source's top-level
        // key is dropped, because its path no longer points at anything.
        let mut out = match &input.payload {
            Value::Object(map) => map.clone(),
            _ => serde_json::Map::new(),
        };
        if self.config.delete_source {
            if let Some(top) = source_key.split('.').next() {
                out.remove(top);
            }
        }
        out.insert("thumbnail".to_string(), thumbnail);
        let out_payload = Value::Object(out);

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: out_payload,
            trace: vec![format!(
                "node_kind={NODE_KIND} src={rel_path} out={thumb_rel} {actual_w}x{actual_h} {format_label} {thumb_size}B"
            )],
        })
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────

/// Resize to fill target box exactly, crop center to target size (aspect-preserving).
fn resize_cover(img: &DynamicImage, target_w: u32, target_h: u32) -> DynamicImage {
    let (src_w, src_h) = img.dimensions();
    let scale_w = target_w as f64 / src_w as f64;
    let scale_h = target_h as f64 / src_h as f64;
    let scale = scale_w.max(scale_h);
    let new_w = ((src_w as f64 * scale).ceil() as u32).max(target_w);
    let new_h = ((src_h as f64 * scale).ceil() as u32).max(target_h);
    let resized = img.resize(new_w, new_h, FilterType::Lanczos3);
    let x = (new_w.saturating_sub(target_w)) / 2;
    let y = (new_h.saturating_sub(target_h)) / 2;
    resized.crop_imm(x, y, target_w, target_h)
}

/// Resize to fit within target box, preserving aspect ratio (may be smaller than target).
fn resize_contain(img: &DynamicImage, target_w: u32, target_h: u32) -> DynamicImage {
    img.resize(target_w, target_h, FilterType::Lanczos3)
}

/// Stretch to exact target dimensions, ignoring aspect ratio.
fn resize_fill(img: &DynamicImage, target_w: u32, target_h: u32) -> DynamicImage {
    img.resize_exact(target_w, target_h, FilterType::Lanczos3)
}

/// Encode `img` to the requested format bytes. Returns (bytes, extension, label).
fn encode_image(
    img: &DynamicImage,
    format: &str,
    quality: u8,
) -> Result<(Vec<u8>, &'static str, &'static str), PipelineError> {
    let mut buf = Vec::new();
    match format.trim().to_lowercase().as_str() {
        "png" => {
            img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Png)
                .map_err(|e| PipelineError::new("IMG_THUMBNAIL", format!("PNG encode: {e}")))?;
            Ok((buf, "png", "png"))
        }
        "webp" => {
            img.write_to(&mut Cursor::new(&mut buf), ImageFormat::WebP)
                .map_err(|e| PipelineError::new("IMG_THUMBNAIL", format!("WebP encode: {e}")))?;
            Ok((buf, "webp", "webp"))
        }
        _ => {
            // Default: JPEG with quality control
            use image::codecs::jpeg::JpegEncoder;
            let mut cursor = Cursor::new(&mut buf);
            JpegEncoder::new_with_quality(&mut cursor, quality)
                .encode_image(img)
                .map_err(|e| PipelineError::new("IMG_THUMBNAIL", format!("JPEG encode: {e}")))?;
            Ok((buf, "jpg", "jpg"))
        }
    }
}

/// Strip path traversal components from folder config.
fn sanitize_folder(folder: &str) -> String {
    folder
        .split('/')
        .filter(|seg| !seg.is_empty() && *seg != "." && *seg != "..")
        .collect::<Vec<_>>()
        .join("/")
}
