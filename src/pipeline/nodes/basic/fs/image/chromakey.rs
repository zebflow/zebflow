//! `n.fs.image.chromakey` — a green screen made transparent, so the picture
//! can sit on anything.
//!
//! | Use | DSL |
//! |---|---|
//! | A generated character, cut out for a poster | `\| fs.save --folder posters/photos \| fs.image.chromakey --folder posters/cutouts --preview image` |
//!
//! Plain pixel maths, no model: every pixel whose colour is within
//! `--tolerance` of `--color` (RGB distance, 0–441) becomes transparent, and
//! the next `--soften` units of distance ramp from transparent to opaque so
//! hair and edges keep a soft rim instead of a hard green fringe. Edge pixels
//! are despilled — the key colour's own channel is pulled down to the other
//! two — so a half-transparent rim is not tinted green. The subject itself
//! (beyond `tolerance + soften`) is untouched.
//!
//! The default key is broadcast chroma green, `#00b140`: what a studio
//! screen and an image model's "green screen background" both come out
//! as (Seedream answered 0,190,54). Pure `#00ff00` is 90 away from that
//! and keys nothing; measure a corner and pass `--color` when the screen
//! is another colour.
//!
//! Reads the source at `--source-key` (default `saved`, the FileRef `fs.save`
//! answers, or a store path string), decodes it with the same
//! decompression-bomb limits as `fs.image.thumbnail`, writes `--format
//! png|webp` (both keep alpha; default png) into `--folder` (default
//! `cutouts/`) as `--filename` or a UUID, and adds `image` — a durable FileRef
//! (`origin: fs.image.chromakey`, `trust: sanitized`) with `width`, `height`,
//! `format` — to the payload, keeping the rest. With `--delete-source` the
//! original is removed and its key dropped.
//!
//! Put it in a poster with `fs.svg.convert`: `<image href="<image.ref>"
//! …/>` composites the alpha over whatever is drawn under it.

use std::sync::Arc;

use async_trait::async_trait;
use image::{DynamicImage, RgbaImage};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use super::load_with_limits;
use crate::pipeline::model::{DslFlag, DslFlagKind, LayoutItem, NodeCapability, NodeExample, NodeFailureSemantic, NodeFieldDef, NodeFieldType, SelectOptionDef};
use crate::pipeline::nodes::shared::file_ref::{durable_file_ref, zebfs_rel_path_or_string};
use crate::pipeline::nodes::shared::util::{filename_stem, metadata_scope, resolve_path};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;

pub const NODE_KIND: &str = "n.fs.image.chromakey";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";
pub const ORIGIN: &str = "fs.image.chromakey";
const DEFAULT_COLOR: &str = "#00b140";
const DEFAULT_TOLERANCE: f32 = 60.0;
const DEFAULT_SOFTEN: f32 = 40.0;
const DEFAULT_FOLDER: &str = "cutouts";
const DEFAULT_SOURCE_KEY: &str = "saved";

fn default_color() -> String {
    DEFAULT_COLOR.to_string()
}
fn default_tolerance() -> Value {
    json!(DEFAULT_TOLERANCE)
}
fn default_soften() -> Value {
    json!(DEFAULT_SOFTEN)
}
fn default_format() -> String {
    "png".to_string()
}
fn default_folder() -> String {
    DEFAULT_FOLDER.to_string()
}
fn default_source_key() -> String {
    DEFAULT_SOURCE_KEY.to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// The key colour, `#rrggbb` (default `#00b140`, broadcast green).
    #[serde(default = "default_color")]
    pub color: String,
    /// RGB distance from the key that is fully transparent (0–441, default 60).
    #[serde(default = "default_tolerance")]
    pub tolerance: Value,
    /// Distance beyond `tolerance` over which alpha ramps to opaque (default 40).
    #[serde(default = "default_soften")]
    pub soften: Value,
    /// png | webp (default png) — both keep alpha.
    #[serde(default = "default_format")]
    pub format: String,
    /// Destination store folder (default `cutouts`).
    #[serde(default = "default_folder")]
    pub folder: String,
    /// Dot-path to the source in the payload (default `saved`).
    #[serde(default = "default_source_key")]
    pub source_key: String,
    /// Remove the source after the cutout is written.
    #[serde(default)]
    pub delete_source: bool,
    /// Filename without extension (default: a UUID).
    #[serde(default)]
    pub filename: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            color: default_color(),
            tolerance: default_tolerance(),
            soften: default_soften(),
            format: default_format(),
            folder: default_folder(),
            source_key: default_source_key(),
            delete_source: false,
            filename: None,
        }
    }
}

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Filesystem],
        title: "Chroma Key".to_string(),
        description: "Make a green screen transparent. Reads the source at `--source-key` (default `saved`, right after `fs.save`), turns every pixel within \
            `--tolerance` of `--color` (default #00b140, broadcast chroma green — what image models produce for \"green screen\") transparent with a `--soften` ramp at the edge and despill, writes `--format png|webp` (both keep alpha) into \
            `--folder` (default `cutouts/`) and adds `image` — a durable FileRef with `width`, `height`, `format` — to the payload. No model: plain pixel maths. \
            Generate the picture on a flat green background, then place the cutout in a poster with `fs.svg.convert` as `<image href=\"<image.ref>\">`."
            .to_string(),
        input_schema: json!({
            "type": "object",
            "description": "Payload must contain the source — a FileRef or a store path string — at the dot-key `source_key` (default `saved`)."
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "image": {
                    "type": "object",
                    "description": "A durable FileRef (kinds/file-ref/README.md) of the cutout with alpha, `origin` fs.image.chromakey, plus `width`, `height`, `format`. The store path is `ref`.",
                    "properties": {
                        "ref":    { "type": "string" },
                        "width":  { "type": "integer" },
                        "height": { "type": "integer" },
                        "format": { "type": "string" },
                        "size":   { "type": "integer" }
                    }
                }
            },
            "required": ["image"]
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        config_schema: json!({
            "type": "object",
            "properties": {
                "color":         { "type": "string", "description": "#rrggbb key colour (default #00b140)." },
                "tolerance":     { "type": "number", "description": "RGB distance that is fully transparent (default 60)." },
                "soften":        { "type": "number", "description": "Ramp beyond tolerance to opaque (default 40)." },
                "format":        { "type": "string", "enum": ["png", "webp"] },
                "folder":        { "type": "string" },
                "source_key":    { "type": "string" },
                "delete_source": { "type": "boolean" },
                "filename":      { "type": "string" }
            }
        }),
        dsl_flags: vec![
            DslFlag { flag: "--color".into(), config_key: "color".into(), description: "Key colour as #rrggbb (default: #00b140, broadcast chroma green)".into(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--tolerance".into(), config_key: "tolerance".into(), description: "RGB distance from the key that is fully transparent, 0–441 (default: 60)".into(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--soften".into(), config_key: "soften".into(), description: "Distance beyond --tolerance over which the edge ramps to opaque (default: 40)".into(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--format".into(), config_key: "format".into(), description: "png | webp (default: png) — both keep alpha".into(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--folder".into(), config_key: "folder".into(), description: "Destination store folder (default: cutouts)".into(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--source-key".into(), config_key: "source_key".into(), description: "Dot-path to the source in the payload: a FileRef or a store path string (default: `saved`)".into(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--delete-source".into(), config_key: "delete_source".into(), description: "Delete the source file after the cutout is written (default: false)".into(), kind: DslFlagKind::Bool, required: false },
            DslFlag { flag: "--filename".into(), config_key: "filename".into(), description: "Custom filename without extension (default: random UUID). Overwrites if same name exists.".into(), kind: DslFlagKind::Scalar, required: false },
        ],
        fields: vec![
            NodeFieldDef { name: "color".into(), label: "Key colour".into(), field_type: NodeFieldType::Text, default_value: Some(json!(DEFAULT_COLOR)), help: Some("#rrggbb of the screen to remove. Default #00b140, the broadcast chroma green image models produce; measure a corner pixel for anything else.".into()), ..Default::default() },
            NodeFieldDef { name: "tolerance".into(), label: "Tolerance".into(), field_type: NodeFieldType::Text, default_value: Some(json!("60")), help: Some("RGB distance from the key colour that is fully transparent (0–441). Raise it if green remains; lower it if the subject loses colour.".into()), ..Default::default() },
            NodeFieldDef { name: "soften".into(), label: "Soften".into(), field_type: NodeFieldType::Text, default_value: Some(json!("40")), help: Some("Distance beyond the tolerance over which the edge ramps from transparent to opaque.".into()), ..Default::default() },
            NodeFieldDef { name: "format".into(), label: "Output format".into(), field_type: NodeFieldType::Select, default_value: Some(json!("png")), help: Some("png (default) or lossless webp; both keep the alpha channel.".into()), options: vec![
                SelectOptionDef { value: "png".into(), label: "PNG".into() },
                SelectOptionDef { value: "webp".into(), label: "WebP (lossless)".into() },
            ], ..Default::default() },
            NodeFieldDef { name: "folder".into(), label: "Folder".into(), field_type: NodeFieldType::Text, default_value: Some(json!(DEFAULT_FOLDER)), help: Some("Destination store folder (default: cutouts). Under public/ for a page to show it anonymously.".into()), ..Default::default() },
            NodeFieldDef { name: "source_key".into(), label: "Source key".into(), field_type: NodeFieldType::Text, default_value: Some(json!(DEFAULT_SOURCE_KEY)), help: Some("Dot-path into the payload: a FileRef or a store path string. Default: saved (what fs.save answers).".into()), ..Default::default() },
            NodeFieldDef { name: "delete_source".into(), label: "Delete source file".into(), field_type: NodeFieldType::Checkbox, default_value: Some(json!(false)), help: Some("Remove the green-screen original after the cutout is written; its key is dropped from the payload.".into()), ..Default::default() },
            NodeFieldDef { name: "filename".into(), label: "Filename".into(), field_type: NodeFieldType::Text, help: Some("Without extension (default: random UUID).".into()), ..Default::default() },
        ],
        layout: vec![
            LayoutItem::Field("source_key".into()),
            LayoutItem::Field("color".into()),
            LayoutItem::Field("tolerance".into()),
            LayoutItem::Field("soften".into()),
            LayoutItem::Field("format".into()),
            LayoutItem::Field("folder".into()),
            LayoutItem::Field("filename".into()),
            LayoutItem::Field("delete_source".into()),
        ],
        failure_semantics: vec![
            NodeFailureSemantic { code: "FW_NODE_FS_IMAGE_CHROMAKEY_CONFIG".into(), description: "A --color that is not #rrggbb; a --tolerance or --soften that is not a number in range; a --format that is not png or webp.".into(), ..Default::default() },
            NodeFailureSemantic { code: "FS_IMAGE_CHROMAKEY_SOURCE".into(), description: "Nothing at --source-key, the object is missing, or it is not a PNG/JPEG/WebP/GIF the decoder accepts within its limits.".into(), ..Default::default() },
            NodeFailureSemantic { code: "FS_IMAGE_CHROMAKEY_RASTER".into(), description: "The encoder or the store write failed.".into(), retryable: true, ..Default::default() },
        ],
        examples: vec![
            NodeExample::dsl("A generated character, cut out for a poster", "fs.image.chromakey --folder sandbox/posters/cutouts --preview image")
                .input(json!({ "saved": { "__zf_type": "file_ref", "backend": "zebfs", "ref": "sandbox/posters/photos/3f9c….png", "filename": "3f9c….png", "mime": "image/png", "kind": "image", "size": 1822310, "sha256": "sha256:…", "lifecycle": "durable", "origin": "fs.save", "trust": "untrusted" } }))
                .output(json!({ "saved": { "__zf_type": "file_ref", "backend": "zebfs", "ref": "sandbox/posters/photos/3f9c….png", "filename": "3f9c….png", "mime": "image/png", "kind": "image", "size": 1822310, "sha256": "sha256:…", "lifecycle": "durable", "origin": "fs.save", "trust": "untrusted" },
                    "image": { "__zf_type": "file_ref", "backend": "zebfs", "ref": "sandbox/posters/cutouts/9a1d….png", "filename": "9a1d….png", "mime": "image/png", "kind": "image", "size": 912400, "sha256": "sha256:…", "lifecycle": "durable", "origin": "fs.image.chromakey", "trust": "sanitized", "width": 1024, "height": 1536, "format": "png" } }))
                .note("The picture was generated \"standing on a solid flat bright green chroma key background\"; the default key #00b140 is what the model paints. Then `fs.svg.convert` draws it with `<image href=\"sandbox/posters/cutouts/9a1d….png\" x=… y=… width=… height=…/>`."),
        ],
        ..Default::default()
    }
}

/// `#rrggbb` → (r, g, b).
pub fn parse_color(raw: &str) -> Option<[u8; 3]> {
    let hex = raw.trim().strip_prefix('#')?;
    if hex.len() != 6 || !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let v = u32::from_str_radix(hex, 16).ok()?;
    Some([(v >> 16) as u8, (v >> 8) as u8, v as u8])
}

fn number(v: &Value, name: &str) -> Result<f32, PipelineError> {
    let n = match v {
        Value::Number(n) => n.as_f64(),
        Value::String(s) if s.trim().is_empty() => None,
        Value::String(s) => s.trim().parse::<f64>().ok(),
        Value::Null => None,
        _ => None,
    };
    n.map(|n| n as f32)
        .filter(|n| (0.0..=441.0).contains(n))
        .ok_or_else(|| PipelineError::new("FW_NODE_FS_IMAGE_CHROMAKEY_CONFIG", format!("--{name} must be a number from 0 to 441")))
}

/// The keyed image: alpha by RGB distance from `key`, a ramp of `soften`
/// beyond `tolerance`, edge pixels despilled.
pub fn key_out(img: &DynamicImage, key: [u8; 3], tolerance: f32, soften: f32) -> RgbaImage {
    let mut out = img.to_rgba8();
    let soften = soften.max(0.001);
    let key_channel = (0..3).max_by(|&a, &b| key[a].cmp(&key[b])).unwrap_or(1);
    for px in out.pixels_mut() {
        let [r, g, b, a] = px.0;
        let d = (((r as f32 - key[0] as f32).powi(2) + (g as f32 - key[1] as f32).powi(2) + (b as f32 - key[2] as f32).powi(2)) as f32).sqrt();
        let k = ((d - tolerance) / soften).clamp(0.0, 1.0);
        if k < 1.0 {
            // Despill the rim: the key's own channel may not exceed the other two.
            let mut c = [r, g, b];
            let others = (0..3).filter(|&i| i != key_channel).map(|i| c[i]).max().unwrap_or(0);
            c[key_channel] = c[key_channel].min(others);
            px.0 = [c[0], c[1], c[2], (a as f32 * k).round() as u8];
        }
    }
    out
}

pub struct Node {
    config: Config,
    platform: Arc<PlatformService>,
    key: [u8; 3],
    tolerance: f32,
    soften: f32,
    webp: bool,
}

impl Node {
    pub fn new(config: Config, platform: Arc<PlatformService>) -> Result<Self, PipelineError> {
        let key = parse_color(&config.color)
            .ok_or_else(|| PipelineError::new("FW_NODE_FS_IMAGE_CHROMAKEY_CONFIG", format!("--color '{}' is not #rrggbb", config.color)))?;
        let tolerance = number(&config.tolerance, "tolerance")?;
        let soften = number(&config.soften, "soften")?;
        let webp = match config.format.trim().to_ascii_lowercase().as_str() {
            "" | "png" => false,
            "webp" => true,
            other => return Err(PipelineError::new("FW_NODE_FS_IMAGE_CHROMAKEY_CONFIG", format!("--format '{other}' is not png or webp"))),
        };
        Ok(Self { config, platform, key, tolerance, soften, webp })
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
        let started = std::time::Instant::now();
        let (owner, project, ..) = metadata_scope(&input.metadata)?;
        let layout = self
            .platform
            .file
            .ensure_project_layout(owner, project)
            .map_err(|e| PipelineError::new("FS_IMAGE_CHROMAKEY_RASTER", e.to_string()))?;
        let zebfs = layout.open_files();

        let key = self.config.source_key.trim();
        let key = if key.is_empty() { DEFAULT_SOURCE_KEY } else { key };
        let value = resolve_path(&input.payload, key).ok_or_else(|| {
            PipelineError::new("FS_IMAGE_CHROMAKEY_SOURCE", format!("nothing at payload key '{key}' — chain after fs.save or set --source-key"))
        })?;
        let rel = zebfs_rel_path_or_string(value)?
            .ok_or_else(|| PipelineError::new("FS_IMAGE_CHROMAKEY_SOURCE", format!("payload key '{key}' must be a FileRef or a store path string")))?;
        let rel = crate::zebfs::normalize_object_path(rel.trim_start_matches('/'))
            .map_err(|e| PipelineError::new("FS_IMAGE_CHROMAKEY_SOURCE", e.to_string()))?;
        let object = zebfs
            .get(&rel)
            .map_err(|e| PipelineError::new("FS_IMAGE_CHROMAKEY_SOURCE", format!("store object {rel}: {}", e.message)))?;
        let img = load_with_limits(&object.bytes).map_err(|e| PipelineError::new("FS_IMAGE_CHROMAKEY_SOURCE", e.message))?;

        let keyed = key_out(&img, self.key, self.tolerance, self.soften);
        let (width, height) = keyed.dimensions();
        let mut bytes = std::io::Cursor::new(Vec::new());
        let (ext, mime) = if self.webp { ("webp", "image/webp") } else { ("png", "image/png") };
        DynamicImage::ImageRgba8(keyed)
            .write_to(&mut bytes, if self.webp { image::ImageFormat::WebP } else { image::ImageFormat::Png })
            .map_err(|e| PipelineError::new("FS_IMAGE_CHROMAKEY_RASTER", format!("{ext} encode: {e}")))?;
        let bytes = bytes.into_inner();

        let stem = self.config.filename.as_deref().map(filename_stem).filter(|s| !s.is_empty()).unwrap_or_else(|| Uuid::new_v4().to_string());
        let filename = format!("{stem}.{ext}");
        let folder = self.config.folder.trim().trim_matches('/');
        let folder = if folder.is_empty() { DEFAULT_FOLDER } else { folder };
        let out_rel = crate::zebfs::normalize_object_path(&format!("{folder}/{filename}"))
            .map_err(|e| PipelineError::new("FW_NODE_FS_IMAGE_CHROMAKEY_CONFIG", format!("--folder: {e}")))?;
        zebfs
            .put(&out_rel, &bytes)
            .map_err(|e| PipelineError::new("FS_IMAGE_CHROMAKEY_RASTER", format!("store write {out_rel}: {}", e.message)))?;
        let mut image = durable_file_ref(layout.file_backend(), &out_rel, &filename, mime, &bytes, ORIGIN, "sanitized");
        if let Some(obj) = image.as_object_mut() {
            obj.insert("width".into(), json!(width));
            obj.insert("height".into(), json!(height));
            obj.insert("format".into(), json!(ext));
        }

        let mut out = match &input.payload {
            Value::Object(map) => map.clone(),
            _ => serde_json::Map::new(),
        };
        if self.config.delete_source {
            if let Err(e) = std::fs::remove_file(layout.local_files_dir()?.join(&rel)) {
                eprintln!("[{NODE_KIND}] delete-source failed for {rel}: {e}");
            }
            if let Some(top) = key.split('.').next() {
                out.remove(top);
            }
        }
        out.insert("image".to_string(), image);
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: Value::Object(out),
            trace: vec![format!(
                "node_kind={NODE_KIND} src={rel} out={out_rel} {width}x{height} {ext} key=#{:02x}{:02x}{:02x} tolerance={} soften={} bytes={} total_ms={}",
                self.key[0], self.key[1], self.key[2], self.tolerance, self.soften, bytes.len(), started.elapsed().as_millis()
            )],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn platform() -> Arc<PlatformService> {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut config = crate::platform::model::PlatformConfig::default();
        config.data_root = tmp.keep().join("platform");
        config.default_password = "secret".to_string();
        Arc::new(PlatformService::from_config(config).expect("platform"))
    }

    #[test]
    fn the_key_colour_and_numbers_are_checked() {
        assert_eq!(parse_color("#00FF00"), Some([0, 255, 0]));
        assert_eq!(parse_color("00ff00"), None);
        assert_eq!(parse_color("#0f0"), None);
        let p = platform();
        let bad = |c: Value| Node::new(serde_json::from_value(c).unwrap(), p.clone()).map(|_| ()).unwrap_err();
        assert!(bad(json!({ "color": "green" })).message.contains("#rrggbb"));
        assert!(bad(json!({ "tolerance": "lots" })).message.contains("--tolerance"));
        assert!(bad(json!({ "soften": 900 })).message.contains("--soften"));
        assert!(bad(json!({ "format": "jpg" })).message.contains("jpg"));
        let ok = Node::new(serde_json::from_value(json!({ "tolerance": "80", "format": "WEBP" })).unwrap(), p).unwrap();
        assert_eq!((ok.key, ok.tolerance, ok.soften, ok.webp), ([0, 177, 64], 80.0, DEFAULT_SOFTEN, true));
        let def = definition();
        let flags: Vec<&str> = def.dsl_flags.iter().map(|f| f.flag.as_str()).collect();
        assert_eq!(flags, vec!["--color", "--tolerance", "--soften", "--format", "--folder", "--source-key", "--delete-source", "--filename"]);
    }

    #[test]
    fn the_screen_goes_transparent_the_subject_stays_and_the_rim_is_despilled() {
        // A green field with a red square, and a greenish-red rim pixel.
        let mut img = RgbaImage::from_pixel(64, 64, image::Rgba([0, 255, 0, 255]));
        for x in 16..48 {
            for y in 16..48 {
                img.put_pixel(x, y, image::Rgba([220, 30, 30, 255]));
            }
        }
        // Distance 80 from the key: inside the 60..100 ramp, so half transparent.
        img.put_pixel(15, 32, image::Rgba([60, 220, 40, 255]));
        let out = key_out(&DynamicImage::ImageRgba8(img), [0, 255, 0], 60.0, 40.0);
        assert_eq!(out.get_pixel(2, 2).0[3], 0, "screen is transparent");
        assert_eq!(out.get_pixel(32, 32).0, [220, 30, 30, 255], "subject untouched");
        let rim = out.get_pixel(15, 32).0;
        assert!(rim[3] > 0 && rim[3] < 255, "rim is partly transparent: {rim:?}");
        assert!(rim[1] <= rim[0].max(rim[2]), "rim is despilled: {rim:?}");
    }
}
