//! `fs.image.thumbnail` — a small, re-encoded copy of an image.
//!
//! `--from` names the image — a FileRef, an upload, or a store key. It is
//! decoded with the family's decompression-bomb limits (`load_with_limits`:
//! 16 000 px a side, 128 MB decoded), resized and re-encoded, so the
//! thumbnail is `sanitized` whatever the source was.
//!
//! Fit modes:
//!   cover   — scale to fill the target box, crop center (default)
//!   contain — scale to fit within the box, preserving aspect ratio
//!   fill    — stretch to exact dimensions, ignoring aspect ratio
//!
//! Formats: jpg (quality-controlled, default), png (lossless), webp (lossless)
//!
//! The answer is one key, `image`: the durable FileRef of the thumbnail
//! (`origin: fs.image.thumbnail`, `trust: sanitized`) with `width`,
//! `height`, `format` and `source_deleted`; the rest of the payload stays.

use std::io::Cursor;
use std::sync::Arc;

use async_trait::async_trait;
use image::{DynamicImage, GenericImageView, ImageFormat, imageops::FilterType};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use super::load_with_limits;
use crate::pipeline::model::NodeCapability;
use crate::pipeline::nodes::shared::limits::{self, choice, whole, within};
use crate::pipeline::nodes::shared::project_store::{OnConflict, on_conflict_flag, open_from, open_store, store_fields, store_flag, target_key};
use crate::pipeline::nodes::shared::util::{filename_stem, metadata_scope, with_answer};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    model::{DslFlag, DslFlagKind, LayoutItem, NodeExample, NodeFieldDef, NodeFieldType, SelectOptionDef},
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;

pub const NODE_KIND: &str = "fs.image.thumbnail";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

/// The encoder and the store: the world's side.
pub const CODE: &str = "FW_NODE_FS_IMAGE_THUMBNAIL";
/// A flag the author set wrong.
pub const CONFIG_CODE: &str = "FW_NODE_FS_IMAGE_THUMBNAIL_CONFIG";
/// `--from` is missing, not there, or not an image the decoder takes.
const SOURCE_CODE: &str = "FW_NODE_FS_IMAGE_THUMBNAIL_SOURCE";

const FITS: &[&str] = &["cover", "contain", "fill"];
const FORMATS: &[&str] = &["jpg", "png", "webp"];
const DEFAULT_SIDE: u32 = 256;
const DEFAULT_QUALITY: u32 = 82;
const DEFAULT_FOLDER: &str = "thumbnails";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// The image: a FileRef, an upload or a store key.
    #[serde(default)]
    pub from: Value,
    /// Target width in pixels (default 256).
    #[serde(default)]
    pub width: Value,
    /// Target height in pixels (default 256).
    #[serde(default)]
    pub height: Value,
    /// `cover` (default), `contain` or `fill`.
    #[serde(default)]
    pub fit: String,
    /// `jpg` (default), `png` or `webp`.
    #[serde(default)]
    pub format: String,
    /// JPEG quality 1–100 (default 82).
    #[serde(default)]
    pub quality: Value,
    /// Delete the source once the thumbnail is written.
    #[serde(default)]
    pub delete_source: bool,
    /// The store to write to; saved explicitly at registration.
    #[serde(default)]
    pub store: Option<String>,
    /// Destination folder (default `thumbnails`).
    #[serde(default)]
    pub folder: String,
    /// Name without extension (default: a generated name).
    #[serde(default)]
    pub filename: Option<String>,
    /// Exact store key; overrides `folder` and `filename`.
    #[serde(default)]
    pub path: Option<String>,
    /// `error` (default), `skip` or `overwrite`.
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

fn field(name: &str, label: &str, help: &str) -> NodeFieldDef {
    NodeFieldDef { name: name.to_string(), label: label.to_string(), field_type: NodeFieldType::Text, help: Some(help.to_string()), ..Default::default() }
}

fn words(list: &[&str]) -> Vec<String> {
    list.iter().map(|word| word.to_string()).collect()
}

pub fn definition() -> NodeDefinition {
    let upload = json!({ "__zf_type": "file_ref", "backend": "zebfs", "store": "local", "ref": "uploads/3f9c….jpg", "filename": "3f9c….jpg", "mime": "image/jpeg", "kind": "image", "size": 182331, "sha256": "sha256:…", "lifecycle": "durable", "origin": "fs.file.put", "trust": "untrusted" });
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Filesystem],
        title: "Image Thumbnail".to_string(),
        description: "Make a small image from the one `--from` names — a FileRef, an upload or a store key. Resizes to `--width`×`--height` (default 256) \
            with `--fit cover|contain|fill` and re-encodes as `--format jpg|png|webp` (`--quality` for jpg), so the result is sanitized whatever came in. \
            Writes under `--folder` (default `thumbnails`) as a generated name, as `--filename`, or at an exact `--path`; a named file that exists is an \
            error unless `--on-conflict` says otherwise. `--delete-source` removes the original once the thumbnail is written. Adds `image` — the durable \
            FileRef (`ref`, `store`, …, `origin: fs.image.thumbnail`, `trust: sanitized`) with `width`, `height`, `format` and `source_deleted` — and keeps \
            the rest of the payload. Store `image.ref` in the row."
            .to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "image": {
                    "type": "object",
                    "description": "The durable FileRef of the thumbnail plus its dimensions. The store path is `ref`.",
                    "properties": {
                        "ref":    { "type": "string" },
                        "store":  { "type": "string" },
                        "origin": { "type": "string", "const": NODE_KIND },
                        "width":  { "type": "integer" },
                        "height": { "type": "integer" },
                        "format": { "type": "string", "enum": FORMATS },
                        "size":   { "type": "integer" },
                        "source_deleted": { "type": "boolean" }
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
            DslFlag { required: true, ..flag("--from", "from", "The image: a FileRef, an upload or a store key.", "file:image") },
            flag("--width", "width", "Target width in pixels (default: 256).", "number"),
            flag("--height", "height", "Target height in pixels (default: 256).", "number"),
            DslFlag { choices: words(FITS), ..flag("--fit", "fit", "cover (default: fill the box, crop the middle), contain or fill.", "") },
            DslFlag { choices: words(FORMATS), ..flag("--format", "format", "jpg (default), png or webp.", "") },
            flag("--quality", "quality", "JPEG quality 1–100 (default: 82).", "number"),
            DslFlag { kind: DslFlagKind::Bool, ..flag("--delete-source", "delete_source", "Delete the source once the thumbnail is written.", "") },
            DslFlag { value: "text".to_string(), ..store_flag() },
            flag("--folder", "folder", "Destination folder (default: thumbnails).", "text"),
            flag("--filename", "filename", "Destination name without extension (default: a generated name).", "text"),
            flag("--path", "path", "Exact destination key; overrides --folder and --filename.", "text"),
            DslFlag { choices: words(&["error", "skip", "overwrite"]), ..on_conflict_flag(OnConflict::Error) },
        ],
        fields: vec![
            field("from", "From", "The image, e.g. {{ input.file }}."),
            NodeFieldDef { default_value: Some(json!("256")), ..field("width", "Width (px)", "Target width (default: 256).") },
            NodeFieldDef { default_value: Some(json!("256")), ..field("height", "Height (px)", "Target height (default: 256).") },
            NodeFieldDef {
                field_type: NodeFieldType::Select,
                default_value: Some(json!("cover")),
                options: vec![
                    SelectOptionDef { value: "cover".to_string(), label: "Cover (fill + crop center)".to_string() },
                    SelectOptionDef { value: "contain".to_string(), label: "Contain (fit within)".to_string() },
                    SelectOptionDef { value: "fill".to_string(), label: "Fill (stretch exact)".to_string() },
                ],
                ..field("fit", "Fit", "cover = fill the box and crop the middle; contain = fit inside it; fill = stretch.")
            },
            NodeFieldDef {
                field_type: NodeFieldType::Select,
                default_value: Some(json!("jpg")),
                options: vec![
                    SelectOptionDef { value: "jpg".to_string(), label: "JPEG (quality-controlled)".to_string() },
                    SelectOptionDef { value: "png".to_string(), label: "PNG (lossless)".to_string() },
                    SelectOptionDef { value: "webp".to_string(), label: "WebP (lossless)".to_string() },
                ],
                ..field("format", "Output format", "jpg for photos, png or webp lossless.")
            },
            NodeFieldDef { default_value: Some(json!("82")), ..field("quality", "JPEG quality", "1–100, default 82. Only applies to JPEG output.") },
            NodeFieldDef {
                field_type: NodeFieldType::Checkbox,
                default_value: Some(json!(false)),
                ..field("delete_source", "Delete source file", "Delete the original once the thumbnail is written.")
            },
            field("folder", "Folder", "Destination folder (default: thumbnails)."),
            field("filename", "Filename", "Without extension (default: a generated name)."),
            field("path", "Path", "Exact destination key; overrides folder and filename."),
        ]
        .into_iter()
        .chain(store_fields(OnConflict::Error))
        .collect(),
        layout: ["from", "width", "height", "fit", "format", "quality", "delete_source", "folder", "filename", "path", "store", "on_conflict"]
            .iter()
            .map(|name| LayoutItem::Field(name.to_string()))
            .collect(),
        examples: vec![
            NodeExample::dsl("Avatar after an upload", "fs.image.thumbnail --from \"{{ input.file }}\" --width 320 --height 320 --fit cover --format webp --folder thumbs")
                .input(json!({ "webhook": { "body": { "caption": "Sunset" } }, "file": upload.clone() }))
                .output(json!({ "webhook": { "body": { "caption": "Sunset" } }, "file": upload, "image": { "__zf_type": "file_ref", "backend": "zebfs", "store": "local", "ref": "thumbs/9a1d….webp", "filename": "9a1d….webp", "mime": "image/webp", "kind": "image", "size": 8120, "sha256": "sha256:…", "lifecycle": "durable", "origin": NODE_KIND, "trust": "sanitized", "width": 320, "height": 320, "format": "webp", "source_deleted": false } }))
                .note("After `fs.file.put --from \"{{ input.webhook.files.photo }}\"`; store `input.image.ref` in the row."),
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

    async fn execute_async(&self, input: NodeExecutionInput) -> Result<NodeExecutionOutput, PipelineError> {
        let (owner, project, ..) = metadata_scope(&input.metadata)?;

        // Every flag is read before a byte is.
        let fit = choice(&self.config.fit, FITS, "cover", "--fit", CONFIG_CODE)?;
        let format = choice(&self.config.format, FORMATS, "jpg", "--format", CONFIG_CODE)?;
        let width = whole(&self.config.width, "--width", CONFIG_CODE)?.unwrap_or(DEFAULT_SIDE);
        let height = whole(&self.config.height, "--height", CONFIG_CODE)?.unwrap_or(DEFAULT_SIDE);
        let (target_w, target_h) = limits::raster(width, height, CONFIG_CODE)?;
        let quality = within(whole(&self.config.quality, "--quality", CONFIG_CODE)?.unwrap_or(DEFAULT_QUALITY), 1, 100, "--quality", CONFIG_CODE)? as u8;
        let on_conflict = OnConflict::parse(self.config.on_conflict.as_deref(), OnConflict::Error, CONFIG_CODE)?;

        // The source is read from the store that holds it (a FileRef's own
        // `store`, or this node's store for a bare key); the thumbnail is
        // written to this node's store.
        let (source_store, rel_path) =
            open_from(&self.platform, owner, project, &self.config.from, self.config.store.as_deref(), "--from", SOURCE_CODE)?;
        let ext = std::path::Path::new(&rel_path).extension().and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
        if matches!(ext.as_str(), "svg" | "heic" | "heif") {
            return Err(PipelineError::new(
                SOURCE_CODE,
                format!("--from is a .{ext}, which this node cannot read; draw an svg with fs.image.render, convert heic to jpg/png/webp first"),
            ));
        }
        let raw_bytes = source_store.read_capped(&rel_path, SOURCE_CODE)?;
        let img = load_with_limits(&raw_bytes, SOURCE_CODE)?;

        let resized = match fit {
            "contain" => resize_contain(&img, target_w, target_h),
            "fill" => resize_fill(&img, target_w, target_h),
            _ => resize_cover(&img, target_w, target_h),
        };
        let (actual_w, actual_h) = resized.dimensions();
        let (encoded, ext_out, mime) = encode_image(&resized, format, quality)?;

        let folder = if self.config.folder.trim().is_empty() { DEFAULT_FOLDER } else { self.config.folder.trim() };
        let stem = self.config.filename.as_deref().map(filename_stem).filter(|s| !s.is_empty()).unwrap_or_else(|| Uuid::new_v4().to_string());
        let filename = format!("{stem}.{ext_out}");
        let thumb_rel = target_key(self.config.path.as_deref(), folder, &filename, CONFIG_CODE)?;
        let store = open_store(&self.platform, owner, project, self.config.store.as_deref())?;
        // A skipped write answers the file already there, as it is.
        if !on_conflict.allows(&store.fs, &thumb_rel, CODE)? {
            let existing = store.stored_ref(&thumb_rel, NODE_KIND, "sanitized", CODE)?;
            return Ok(NodeExecutionOutput {
                output_pins: vec![OUTPUT_PIN_OUT.to_string()],
                payload: with_answer(&input.payload, json!({ "image": existing })),
                trace: vec![format!("node_kind={NODE_KIND} src={rel_path} out={thumb_rel} skipped")],
            });
        }
        store.fs.put(&thumb_rel, &encoded).map_err(|e| PipelineError::new(CODE, format!("write {thumb_rel}: {e}")))?;

        // The source goes only once the thumbnail is written, never when the
        // thumbnail replaced it, and a failed delete fails the node.
        let source_deleted = self.config.delete_source && !(source_store.id == store.id && rel_path == thumb_rel);
        if source_deleted {
            source_store.delete_named(&self.platform, owner, project, &rel_path, CODE)?;
        }

        let mut image = store.file_ref(&thumb_rel, &filename, mime, &encoded, NODE_KIND, "sanitized");
        if let Some(obj) = image.as_object_mut() {
            obj.insert("width".into(), json!(actual_w));
            obj.insert("height".into(), json!(actual_h));
            obj.insert("format".into(), json!(format));
            obj.insert("source_deleted".into(), json!(source_deleted));
        }
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: with_answer(&input.payload, json!({ "image": image })),
            trace: vec![format!("node_kind={NODE_KIND} src={rel_path} out={thumb_rel} {actual_w}x{actual_h} {format} {}B", encoded.len())],
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

/// Encode `img` as `format`. Answers (bytes, extension, mime).
fn encode_image(img: &DynamicImage, format: &str, quality: u8) -> Result<(Vec<u8>, &'static str, &'static str), PipelineError> {
    let mut buf = Vec::new();
    match format {
        "png" => {
            img.write_to(&mut Cursor::new(&mut buf), ImageFormat::Png).map_err(|e| PipelineError::new(CODE, format!("PNG encode: {e}")))?;
            Ok((buf, "png", "image/png"))
        }
        "webp" => {
            img.write_to(&mut Cursor::new(&mut buf), ImageFormat::WebP).map_err(|e| PipelineError::new(CODE, format!("WebP encode: {e}")))?;
            Ok((buf, "webp", "image/webp"))
        }
        _ => {
            // `jpg`: --format is a closed choice, parsed before this.
            use image::codecs::jpeg::JpegEncoder;
            let mut cursor = Cursor::new(&mut buf);
            JpegEncoder::new_with_quality(&mut cursor, quality).encode_image(img).map_err(|e| PipelineError::new(CODE, format!("JPEG encode: {e}")))?;
            Ok((buf, "jpg", "image/jpeg"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pipeline::nodes::shared::project_store::NodeStore;

    fn platform() -> Arc<PlatformService> {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut config = crate::platform::model::PlatformConfig::default();
        config.data_root = tmp.keep().join("platform");
        Arc::new(PlatformService::from_config(config).expect("platform"))
    }

    fn store(platform: &Arc<PlatformService>) -> NodeStore {
        open_store(platform, "demo", "demo", None).expect("store")
    }

    fn png(w: u32, h: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        DynamicImage::ImageRgb8(image::RgbImage::from_pixel(w, h, image::Rgb([200, 30, 30])))
            .write_to(&mut Cursor::new(&mut bytes), ImageFormat::Png)
            .unwrap();
        bytes
    }

    async fn run(platform: &Arc<PlatformService>, config: Value, payload: Value) -> Result<Value, PipelineError> {
        let node = Node::new(serde_json::from_value(config).expect("config"), platform.clone())?;
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

    #[tokio::test]
    async fn from_is_required_and_never_read_from_the_payload() {
        let p = platform();
        let file = store(&p).file_ref("uploads/a.png", "a.png", "image/png", b"", "fs.file.put", "untrusted");
        let err = run(&p, json!({}), json!({ "saved": "uploads/a.png", "file": file })).await.unwrap_err();
        assert_eq!(err.code, SOURCE_CODE);
        assert!(err.message.contains("--from is required"), "{}", err.message);
        assert_eq!(run(&p, json!({ "from": "uploads/none.png" }), json!({})).await.unwrap_err().code, SOURCE_CODE);
        store(&p).fs.put("logo.svg", b"<svg/>").unwrap();
        assert_eq!(run(&p, json!({ "from": "logo.svg" }), json!({})).await.unwrap_err().code, SOURCE_CODE);
    }

    #[tokio::test]
    async fn the_thumbnail_is_image_and_the_payload_stays() {
        let p = platform();
        store(&p).fs.put("uploads/a.png", &png(64, 32)).unwrap();
        let file = store(&p).stored_ref("uploads/a.png", "fs.file.put", "untrusted", "T").unwrap();
        let input = json!({ "webhook": { "body": { "caption": "Sunset" } }, "file": file });
        let out = run(&p, json!({ "from": file, "width": "16", "height": 16, "format": "webp", "folder": "thumbs", "delete_source": true }), input.clone()).await.unwrap();
        let mut keys: Vec<&str> = out.as_object().unwrap().keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, vec!["file", "image", "webhook"]);
        assert_eq!(out["webhook"], input["webhook"]);
        let image = &out["image"];
        crate::pipeline::nodes::shared::file_ref::validate_file_ref(image).expect("a contract FileRef");
        assert!(image["ref"].as_str().unwrap().starts_with("thumbs/") && image["ref"].as_str().unwrap().ends_with(".webp"));
        assert_eq!((image["width"].as_u64(), image["height"].as_u64(), image["format"].as_str()), (Some(16), Some(16), Some("webp")));
        assert_eq!((image["origin"].as_str(), image["trust"].as_str(), image["mime"].as_str()), (Some(NODE_KIND), Some("sanitized"), Some("image/webp")));
        assert_eq!(image["source_deleted"], true);
        assert!(store(&p).fs.head("uploads/a.png").is_err());
    }

    #[tokio::test]
    async fn every_flag_is_checked() {
        let p = platform();
        store(&p).fs.put("a.png", &png(8, 8)).unwrap();
        for bad in [json!({ "fit": "stretch" }), json!({ "format": "gif" }), json!({ "width": "wide" }), json!({ "quality": 0 }), json!({ "width": 9000 }), json!({ "on_conflict": "replace" })] {
            let mut config = bad.clone();
            config["from"] = json!("a.png");
            assert_eq!(run(&p, config, json!({})).await.unwrap_err().code, CONFIG_CODE, "{bad}");
        }
        let out = run(&p, json!({ "from": "a.png", "filename": "t", "fit": "contain" }), json!({})).await.unwrap();
        assert_eq!(out["image"]["ref"], "thumbnails/t.jpg");
        assert!(run(&p, json!({ "from": "a.png", "filename": "t" }), json!({})).await.unwrap_err().message.contains("already exists"));
    }

    #[test]
    fn the_signature_takes_from() {
        assert_eq!(
            crate::pipeline::nodes::node_signature(&definition()),
            "fs.image.thumbnail --from IMAGE [--width N] [--height N] [--fit cover|contain|fill] [--format jpg|png|webp] [--quality N] \
             [--delete-source] [--store TEXT] [--folder TEXT] [--filename TEXT] [--path TEXT] [--on-conflict error|skip|overwrite] → image"
        );
    }
}
