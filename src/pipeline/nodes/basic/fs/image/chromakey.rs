//! `fs.image.chromakey` — a green screen made transparent, so the picture
//! can sit on anything.
//!
//! | Use | DSL |
//! |---|---|
//! | A generated character, cut out for a poster | `\| fs.file.put --from "{{ input.response.body }}" --folder posters/photos \| fs.image.chromakey --from "{{ input.file }}" --folder posters/cutouts --preview image` |
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
//! Reads the image `--from` names — a FileRef, an upload or a store key —
//! decodes it with the same decompression-bomb limits as
//! `fs.image.thumbnail`, writes `--format png|webp` (both keep alpha; default
//! png) under `--folder` (default `cutouts/`) as `--filename` or a generated
//! name, and adds `image` — a durable FileRef (`origin: fs.image.chromakey`,
//! `trust: sanitized`) with `width`, `height`, `format` and `source_deleted`
//! — keeping the rest of the payload. With `--delete-source` the original is
//! removed once the cutout is written.
//!
//! Put it in a poster with `fs.image.render`: `<image href="<image.ref>"
//! …/>` composites the alpha over whatever is drawn under it.

use std::sync::Arc;

use async_trait::async_trait;
use image::{DynamicImage, RgbaImage};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use super::load_with_limits;
use crate::pipeline::model::{DslFlag, DslFlagKind, LayoutItem, NodeCapability, NodeExample, NodeFailureSemantic, NodeFieldDef, NodeFieldType, SelectOptionDef};
use crate::pipeline::nodes::shared::limits::choice;
use crate::pipeline::nodes::shared::project_store::{OnConflict, on_conflict_flag, open_from, open_store, store_fields, store_flag, target_key};
use crate::pipeline::nodes::shared::util::{filename_stem, metadata_scope, with_answer};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;

pub const NODE_KIND: &str = "fs.image.chromakey";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";
pub const ORIGIN: &str = "fs.image.chromakey";
/// A flag the author set wrong.
pub const CONFIG_CODE: &str = "FW_NODE_FS_IMAGE_CHROMAKEY_CONFIG";
/// `--from` is missing, not there, or not an image the decoder takes.
const SOURCE_CODE: &str = "FW_NODE_FS_IMAGE_CHROMAKEY_SOURCE";
/// The encoder or the store write.
const RASTER_CODE: &str = "FW_NODE_FS_IMAGE_CHROMAKEY_RASTER";
const FORMATS: &[&str] = &["png", "webp"];
const DEFAULT_COLOR: &str = "#00b140";
const DEFAULT_TOLERANCE: f32 = 60.0;
const DEFAULT_SOFTEN: f32 = 40.0;
const DEFAULT_FOLDER: &str = "cutouts";

fn default_color() -> String {
    DEFAULT_COLOR.to_string()
}
fn default_tolerance() -> Value {
    json!(DEFAULT_TOLERANCE)
}
fn default_soften() -> Value {
    json!(DEFAULT_SOFTEN)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// The image: a FileRef, an upload or a store key.
    #[serde(default)]
    pub from: Value,
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
    #[serde(default)]
    pub format: String,
    /// Destination store folder (default `cutouts`).
    #[serde(default)]
    pub folder: String,
    /// Remove the source after the cutout is written.
    #[serde(default)]
    pub delete_source: bool,
    /// Filename without extension (default: a UUID).
    #[serde(default)]
    pub filename: Option<String>,
    /// Exact store key; overrides `folder` and `filename`.
    #[serde(default)]
    pub path: Option<String>,
    /// The store to write to; saved explicitly at registration.
    #[serde(default)]
    pub store: Option<String>,
    /// `overwrite`, `skip` or `error` (default: error).
    #[serde(default)]
    pub on_conflict: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            from: Value::Null,
            color: default_color(),
            tolerance: default_tolerance(),
            soften: default_soften(),
            format: String::new(),
            folder: String::new(),
            delete_source: false,
            filename: None,
            path: None,
            store: None,
            on_conflict: None,
        }
    }
}

fn flag(name: &str, key: &str, description: &str, value: &str) -> DslFlag {
    DslFlag { flag: name.into(), config_key: key.into(), description: description.into(), kind: DslFlagKind::Scalar, required: false, value: value.into(), ..Default::default() }
}

fn words(list: &[&str]) -> Vec<String> {
    list.iter().map(|word| word.to_string()).collect()
}

pub fn definition() -> NodeDefinition {
    let photo = json!({ "__zf_type": "file_ref", "backend": "zebfs", "store": "local", "ref": "sandbox/posters/photos/3f9c….png", "filename": "3f9c….png", "mime": "image/png", "kind": "image", "size": 1822310, "sha256": "sha256:…", "lifecycle": "durable", "origin": "fs.file.put", "trust": "untrusted" });
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Filesystem],
        title: "Chroma Key".to_string(),
        description: "Make a green screen transparent. Reads the image `--from` names — a FileRef, an upload or a store key — turns every pixel within \
            `--tolerance` of `--color` (default #00b140, broadcast chroma green — what image models produce for \"green screen\") transparent with a `--soften` ramp at the edge and despill, \
            writes `--format png|webp` (both keep alpha) under `--folder` (default `cutouts/`) and adds `image` — a durable FileRef with `width`, `height`, `format`, `source_deleted` — \
            keeping the rest of the payload. No model: plain pixel maths. Generate the picture on a flat green background, then place the cutout in a poster with \
            `fs.image.render` as `<image href=\"<image.ref>\">`."
            .to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "image": {
                    "type": "object",
                    "description": "A durable FileRef (kinds/file-ref/README.md) of the cutout with alpha, `origin` fs.image.chromakey, plus `width`, `height`, `format`, `source_deleted`. The store path is `ref`.",
                    "properties": {
                        "ref":    { "type": "string" },
                        "width":  { "type": "integer" },
                        "height": { "type": "integer" },
                        "format": { "type": "string", "enum": FORMATS },
                        "size":   { "type": "integer" },
                        "source_deleted": { "type": "boolean" }
                    }
                }
            },
            "required": ["image"]
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        config_schema: Default::default(),
        dsl_flags: vec![
            DslFlag { required: true, ..flag("--from", "from", "The image: a FileRef, an upload or a store key.", "file:image") },
            flag("--color", "color", "Key colour as #rrggbb (default: #00b140, broadcast chroma green).", "text"),
            flag("--tolerance", "tolerance", "RGB distance from the key that is fully transparent, 0–441 (default: 60).", "number"),
            flag("--soften", "soften", "Distance beyond --tolerance over which the edge ramps to opaque (default: 40).", "number"),
            DslFlag { choices: words(FORMATS), ..flag("--format", "format", "png (default) or webp; both keep alpha.", "") },
            DslFlag { kind: DslFlagKind::Bool, ..flag("--delete-source", "delete_source", "Delete the source once the cutout is written.", "") },
            DslFlag { value: "text".into(), ..store_flag() },
            flag("--folder", "folder", "Destination folder (default: cutouts).", "text"),
            flag("--filename", "filename", "Destination name without extension (default: a generated name).", "text"),
            flag("--path", "path", "Exact destination key; overrides --folder and --filename.", "text"),
            DslFlag { choices: words(&["error", "skip", "overwrite"]), ..on_conflict_flag(OnConflict::Error) },
        ],
        fields: vec![
            NodeFieldDef { name: "from".into(), label: "From".into(), field_type: NodeFieldType::Text, help: Some("The image, e.g. {{ input.file }}.".into()), ..Default::default() },
            NodeFieldDef { name: "color".into(), label: "Key colour".into(), field_type: NodeFieldType::Text, default_value: Some(json!(DEFAULT_COLOR)), help: Some("#rrggbb of the screen to remove. Default #00b140, the broadcast chroma green image models produce; measure a corner pixel for anything else.".into()), ..Default::default() },
            NodeFieldDef { name: "tolerance".into(), label: "Tolerance".into(), field_type: NodeFieldType::Text, default_value: Some(json!("60")), help: Some("RGB distance from the key colour that is fully transparent (0–441). Raise it if green remains; lower it if the subject loses colour.".into()), ..Default::default() },
            NodeFieldDef { name: "soften".into(), label: "Soften".into(), field_type: NodeFieldType::Text, default_value: Some(json!("40")), help: Some("Distance beyond the tolerance over which the edge ramps from transparent to opaque.".into()), ..Default::default() },
            NodeFieldDef { name: "format".into(), label: "Output format".into(), field_type: NodeFieldType::Select, default_value: Some(json!("png")), help: Some("png (default) or lossless webp; both keep the alpha channel.".into()), options: vec![
                SelectOptionDef { value: "png".into(), label: "PNG".into() },
                SelectOptionDef { value: "webp".into(), label: "WebP (lossless)".into() },
            ], ..Default::default() },
            NodeFieldDef { name: "delete_source".into(), label: "Delete source file".into(), field_type: NodeFieldType::Checkbox, default_value: Some(json!(false)), help: Some("Remove the green-screen original once the cutout is written.".into()), ..Default::default() },
            NodeFieldDef { name: "folder".into(), label: "Folder".into(), field_type: NodeFieldType::Text, default_value: Some(json!(DEFAULT_FOLDER)), help: Some("Destination folder (default: cutouts). Private until the owner exposes it in Studio → Files.".into()), ..Default::default() },
            NodeFieldDef { name: "filename".into(), label: "Filename".into(), field_type: NodeFieldType::Text, help: Some("Without extension (default: a generated name).".into()), ..Default::default() },
            NodeFieldDef { name: "path".into(), label: "Path".into(), field_type: NodeFieldType::Text, help: Some("Exact destination key; overrides folder and filename.".into()), ..Default::default() },
        ].into_iter().chain(store_fields(OnConflict::Error)).collect(),
        layout: ["from", "color", "tolerance", "soften", "format", "delete_source", "folder", "filename", "path", "store", "on_conflict"]
            .iter()
            .map(|name| LayoutItem::Field(name.to_string()))
            .collect(),
        failure_semantics: vec![
            NodeFailureSemantic { code: CONFIG_CODE.into(), description: "A --color that is not #rrggbb; a --tolerance or --soften that is not a number in range; a --format that is not png or webp; a destination that leaves the store.".into(), ..Default::default() },
            NodeFailureSemantic { code: SOURCE_CODE.into(), description: "No --from, the file is missing, or it is not a PNG/JPEG/WebP/GIF the decoder accepts within its limits.".into(), ..Default::default() },
            NodeFailureSemantic { code: RASTER_CODE.into(), description: "The encoder or the store write failed.".into(), retryable: true, ..Default::default() },
        ],
        examples: vec![
            NodeExample::dsl("A generated character, cut out for a poster", "fs.image.chromakey --from \"{{ input.file }}\" --folder sandbox/posters/cutouts --preview image")
                .input(json!({ "file": photo.clone() }))
                .output(json!({ "file": photo,
                    "image": { "__zf_type": "file_ref", "backend": "zebfs", "store": "local", "ref": "sandbox/posters/cutouts/9a1d….png", "filename": "9a1d….png", "mime": "image/png", "kind": "image", "size": 912400, "sha256": "sha256:…", "lifecycle": "durable", "origin": "fs.image.chromakey", "trust": "sanitized", "width": 1024, "height": 1536, "format": "png", "source_deleted": false } }))
                .note("The picture was generated \"standing on a solid flat bright green chroma key background\"; the default key #00b140 is what the model paints. Then `fs.image.render` draws it with `<image href=\"sandbox/posters/cutouts/9a1d….png\" x=… y=… width=… height=…/>`."),
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
        .ok_or_else(|| PipelineError::new(CONFIG_CODE, format!("--{name} must be a number from 0 to 441")))
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
            .ok_or_else(|| PipelineError::new(CONFIG_CODE, format!("--color '{}' is not #rrggbb", config.color)))?;
        let tolerance = number(&config.tolerance, "tolerance")?;
        let soften = number(&config.soften, "soften")?;
        let webp = choice(&config.format, FORMATS, "png", "--format", CONFIG_CODE)? == "webp";
        OnConflict::parse(config.on_conflict.as_deref(), OnConflict::Error, CONFIG_CODE)?;
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
        let (source_store, rel) =
            open_from(&self.platform, owner, project, &self.config.from, self.config.store.as_deref(), "--from", SOURCE_CODE)?;
        let source_bytes = source_store.read_capped(&rel, SOURCE_CODE)?;
        let img = load_with_limits(&source_bytes, SOURCE_CODE)?;

        let keyed = key_out(&img, self.key, self.tolerance, self.soften);
        let (width, height) = keyed.dimensions();
        let mut bytes = std::io::Cursor::new(Vec::new());
        let (ext, mime) = if self.webp { ("webp", "image/webp") } else { ("png", "image/png") };
        DynamicImage::ImageRgba8(keyed)
            .write_to(&mut bytes, if self.webp { image::ImageFormat::WebP } else { image::ImageFormat::Png })
            .map_err(|e| PipelineError::new(RASTER_CODE, format!("{ext} encode: {e}")))?;
        let bytes = bytes.into_inner();

        let stem = self.config.filename.as_deref().map(filename_stem).filter(|s| !s.is_empty()).unwrap_or_else(|| Uuid::new_v4().to_string());
        let filename = format!("{stem}.{ext}");
        let folder = self.config.folder.trim().trim_matches('/');
        let folder = if folder.is_empty() { DEFAULT_FOLDER } else { folder };
        let out_rel = target_key(self.config.path.as_deref(), folder, &filename, CONFIG_CODE)?;
        let store = open_store(&self.platform, owner, project, self.config.store.as_deref())?;
        let on_conflict = OnConflict::parse(self.config.on_conflict.as_deref(), OnConflict::Error, CONFIG_CODE)?;
        // A skipped write answers the file already there, as it is.
        if !on_conflict.allows(&store.fs, &out_rel, RASTER_CODE)? {
            let existing = store.stored_ref(&out_rel, ORIGIN, "sanitized", RASTER_CODE)?;
            return Ok(NodeExecutionOutput {
                output_pins: vec![OUTPUT_PIN_OUT.to_string()],
                payload: with_answer(&input.payload, json!({ "image": existing })),
                trace: vec![format!("node_kind={NODE_KIND} src={rel} out={out_rel} skipped")],
            });
        }
        store
            .fs
            .put(&out_rel, &bytes)
            .map_err(|e| PipelineError::new(RASTER_CODE, format!("store write {out_rel}: {}", e.message)))?;
        // The source goes only once the output is written and is not it.
        let source_deleted = self.config.delete_source && !(source_store.id == store.id && rel == out_rel);
        if source_deleted {
            source_store.delete_named(&self.platform, owner, project, &rel, RASTER_CODE)?;
        }
        let mut image = store.file_ref(&out_rel, &filename, mime, &bytes, ORIGIN, "sanitized");
        if let Some(obj) = image.as_object_mut() {
            obj.insert("width".into(), json!(width));
            obj.insert("height".into(), json!(height));
            obj.insert("format".into(), json!(ext));
            obj.insert("source_deleted".into(), json!(source_deleted));
        }
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: with_answer(&input.payload, json!({ "image": image })),
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

    fn platform() -> crate::pipeline::nodes::shared::test_platform::TestPlatform {
        crate::pipeline::nodes::shared::test_platform::test_platform()
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
        assert_eq!(bad(json!({ "on_conflict": "replace" })).code, CONFIG_CODE);
        let ok = Node::new(serde_json::from_value(json!({ "tolerance": "80", "format": "WEBP" })).unwrap(), p.clone()).unwrap();
        assert_eq!((ok.key, ok.tolerance, ok.soften, ok.webp), ([0, 177, 64], 80.0, DEFAULT_SOFTEN, true));
        let def = definition();
        let flags: Vec<&str> = def.dsl_flags.iter().map(|f| f.flag.as_str()).collect();
        assert_eq!(flags, vec!["--from", "--color", "--tolerance", "--soften", "--format", "--delete-source", "--store", "--folder", "--filename", "--path", "--on-conflict"]);
        assert_eq!(
            crate::pipeline::nodes::node_signature(&def),
            "fs.image.chromakey --from IMAGE [--color TEXT] [--tolerance N] [--soften N] [--format png|webp] [--delete-source] \
             [--store TEXT] [--folder TEXT] [--filename TEXT] [--path TEXT] [--on-conflict error|skip|overwrite] → image"
        );
    }

    async fn run(p: &Arc<PlatformService>, config: Value, payload: Value) -> Result<Value, PipelineError> {
        let node = Node::new(serde_json::from_value(config).unwrap(), p.clone())?;
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
    async fn from_is_required_and_the_cutout_is_image_beside_the_payload() {
        let p = platform();
        let err = run(&p, json!({}), json!({ "saved": "uploads/green.png" })).await.unwrap_err();
        assert_eq!(err.code, SOURCE_CODE);
        assert!(err.message.contains("--from is required"), "{}", err.message);

        let mut png = Vec::new();
        DynamicImage::ImageRgba8(RgbaImage::from_pixel(8, 4, image::Rgba([0, 177, 64, 255])))
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let store = open_store(&p, "demo", "demo", None).unwrap();
        store.fs.put("uploads/green.png", &png).unwrap();
        let file = store.stored_ref("uploads/green.png", "fs.file.put", "untrusted", "T").unwrap();
        let out = run(&p, json!({ "from": file, "filename": "cut" }), json!({ "file": file, "keep": 1 })).await.unwrap();
        let mut keys: Vec<&str> = out.as_object().unwrap().keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, vec!["file", "image", "keep"]);
        assert_eq!((out["image"]["ref"].as_str(), out["image"]["origin"].as_str()), (Some("cutouts/cut.png"), Some(ORIGIN)));
        assert_eq!((out["image"]["width"].as_u64(), out["image"]["source_deleted"].as_bool()), (Some(8), Some(false)));
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
