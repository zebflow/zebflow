//! `n.fs.svg.convert` — an SVG, as a picture. No browser: resvg draws it.
//!
//! | Use | DSL |
//! |---|---|
//! | The SVG a model wrote, kept as a PNG | `\| ai.agent --credential openrouter --schema '{"type":"object","required":["svg"],"properties":{"svg":{"type":"string"}}}' --prompt "…" \| fs.svg.convert --source-key data.svg --folder posters --preview image` |
//! | A stored SVG at a size | `\| fs.svg.convert --source-key saved --width 512 --height 512 --fit contain --format webp --folder public/logos` |
//!
//! # The source
//!
//! `--source-key` (default `svg`) is a dot-path into the payload. The value
//! is SVG text (`<svg …>…</svg>`), a store path string (`brand/logo.svg`) or
//! a FileRef of a stored `.svg`. Up to 512 KB.
//!
//! # What the node adds to plain SVG
//!
//! - **Pictures from the project only.** An `<image href>` is a store path
//!   (`sandbox/posters/photos/venue.jpg`, or the same behind `zebfs://`) or
//!   a repository file under `static/` (`repo://static/brand/logo.svg`). A
//!   URL or a `data:` URI is refused, naming the href — fetch with
//!   `http.request --response-type bytes`, `fs.save` it, then name the path.
//!   An `.svg` picture draws with the same fonts and may load no pictures of
//!   its own. Caps: 10 MB a file, 40 MP a raster, 512 KB an SVG.
//! - **Fonts the project has.** `font-family` resolves against the bundled
//!   Inter (400/500/600/800) and every `.ttf`/`.otf` under the repository's
//!   `static/fonts/`, by family name. A stack takes the first known name;
//!   generic keywords alone (`sans-serif`) mean Inter; a family nobody has
//!   is refused with the list. Nothing falls back silently.
//! - **Text that wraps.** `<text inline-size="900">` (SVG 2) breaks its
//!   content into lines measured by the shaper itself: one `<tspan>` per
//!   line at the element's `x`, `line-height` (a multiplier or `px`, default
//!   1.2) apart. `text-anchor` still applies per line. resvg has no
//!   `inline-size` of its own. See [`text`].
//! - **Text that shrinks.** `data-fit="shrink"` beside `inline-size` steps
//!   the font size down until the text fits in `data-max-lines` lines
//!   (default 1), never below `data-min-size` (default 8): a name on a
//!   certificate stays on its line. See [`text`].
//! - **A size.** With neither `--width` nor `--height` the canvas is the
//!   SVG's own `width`×`height`; one of them scales the other in proportion;
//!   both go through `--fit cover|contain|fill`, the same three words as
//!   `fs.image.thumbnail`. See [`raster`].
//! - **PDF.** `--format pdf` writes one page the SVG's size in points through
//!   svg2pdf: vector shapes stay vector and text stays text with the font
//!   subset embedded, so it can be selected and searched. Size and fit flags
//!   do not apply and are refused with pdf.
//! - **The layout report.** Beside `image` the answer carries `layout`: every
//!   text and picture with its box in canvas pixels, the pairs that overlap
//!   with the overlap area, the boxes that leave the canvas, and `ok`. A
//!   script turns it into a verdict for `logic.retry`, so an agent can fix
//!   its composition without anyone looking at pixels. See [`layout`].
//! - **Effects.** Shadows, blur, glow, colour grading are SVG filters
//!   (`<filter>` with `feDropShadow`, `feGaussianBlur`, `feColorMatrix`,
//!   `feTurbulence` for grain); resvg renders them, the PDF keeps them. No
//!   node needed.
//!
//! # The answer
//!
//! The payload plus `image`: a durable FileRef (`origin: fs.svg.convert`,
//! `trust: generated`) with `width`, `height` and `format` beside the eleven
//! contract fields, written under `--folder` (default `images/`) as
//! `--filename` or a UUID, plus `layout`. `--format png|jpg|webp|pdf`
//! (default png; `--quality` is JPEG's). With `--delete-source` a stored source is removed and the
//! source key dropped from the payload.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::pipeline::model::{
    DslFlag, DslFlagKind, LayoutItem, NodeCapability, NodeExample, NodeFailureSemantic, NodeFieldDef, NodeFieldType,
    SelectOptionDef,
};
use crate::pipeline::nodes::shared::file_ref::{durable_file_ref, zebfs_rel_path_or_string};
use crate::pipeline::nodes::shared::util::{filename_stem, metadata_scope, resolve_path};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;

mod error;
mod fonts;
pub mod layout;
mod limits;
mod raster;
#[cfg(test)]
mod security;
mod sources;
mod text;
#[cfg(test)]
mod tests;

pub use error::{ConvertError, ConvertErrorKind};
pub use fonts::FontSet;
pub use raster::{Fit, OutputFormat, Rendered, Target, render};
pub use layout::report as layout_report;
pub use limits::{MAX_BLUR_DEVIATION, MAX_DEPTH, MAX_FILTER_PRIMITIVES, MAX_FILTER_REGION_PERCENT, MAX_TURBULENCE_OCTAVES, check_depth, check_filters};
pub use sources::{MAX_SVG_BYTES, MemoryStore, Resolver, Source, SourceStore, looks_like_svg, strip_safe_doctype};
#[cfg(test)]
pub(crate) use tests::test_support;

pub const NODE_KIND: &str = "n.fs.svg.convert";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";
pub const ORIGIN: &str = "fs.svg.convert";
const DEFAULT_FOLDER: &str = "images";
const DEFAULT_SOURCE_KEY: &str = "svg";
const DEFAULT_QUALITY: u8 = 82;

/// A number, or the number as text — the dialog sends text, the DSL sends
/// what was typed; empty is absent.
fn number_or_text<'de, D: Deserializer<'de>>(d: D) -> Result<Option<u32>, D::Error> {
    let v = Option::<Value>::deserialize(d)?;
    Ok(match v {
        None | Some(Value::Null) => None,
        Some(Value::Number(n)) => n.as_u64().map(|n| n as u32),
        Some(Value::String(s)) if s.trim().is_empty() => None,
        Some(Value::String(s)) => s.trim().parse::<u32>().ok().or(Some(u32::MAX)),
        Some(_) => Some(u32::MAX),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// Canvas width in pixels; absent = the SVG's own (or follows `height`).
    #[serde(default, deserialize_with = "number_or_text")]
    pub width: Option<u32>,
    /// Canvas height in pixels; absent = the SVG's own (or follows `width`).
    #[serde(default, deserialize_with = "number_or_text")]
    pub height: Option<u32>,
    /// cover | contain | fill (default cover) — only with both sides given.
    #[serde(default)]
    pub fit: Option<String>,
    /// png | jpg | webp (default png).
    #[serde(default)]
    pub format: Option<String>,
    /// JPEG quality 1–100 (default 82).
    #[serde(default, deserialize_with = "number_or_text")]
    pub quality: Option<u32>,
    /// Destination store folder (default `images`).
    #[serde(default)]
    pub folder: Option<String>,
    /// Dot-path to the source in the payload (default `svg`).
    #[serde(default)]
    pub source_key: Option<String>,
    /// Remove a stored source after the picture is written.
    #[serde(default)]
    pub delete_source: bool,
    /// Filename without extension (default: a UUID). Overwrites a same-named file.
    #[serde(default)]
    pub filename: Option<String>,
}

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Filesystem],
        title: "SVG Convert".to_string(),
        description: "Draw an SVG as a picture, no browser. Reads the SVG at `--source-key` (default `svg`): SVG text, a store path, or a FileRef of a stored .svg. \
            Pictures inside it come from the project only — `<image href>` is a store path or `repo://static/…`, never a URL or a data URI. Text is shaped with the bundled Inter \
            or a family the project has under `static/fonts/` (named by family; an unknown one is refused with the list); `<text inline-size=\"900\">` wraps its lines and \
            `data-fit=\"shrink\"` beside it shrinks the size until the text fits `data-max-lines` (default 1). Effects are SVG filters. \
            With no `--width`/`--height` the canvas is the SVG's own size; one side scales the other; both use `--fit cover|contain|fill`. Writes `--format png|jpg|webp|pdf` \
            (default png, `--quality` for jpg; pdf keeps text as text and takes no size flags) into `--folder` (default `images/`) and adds `image` — a durable FileRef with \
            `width`, `height`, `format` — plus `layout` (every text and picture box, the overlaps, what leaves the canvas, `ok`) to the payload; `--delete-source` removes a stored source. \
            Look at it with `--preview image`."
            .to_string(),
        input_schema: json!({
            "type": "object",
            "description": "The SVG at the dot-key `source_key` (default `svg`): SVG text, a store path string, or a FileRef of a stored .svg."
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "image": {
                    "type": "object",
                    "description": "A durable FileRef (kinds/file-ref/README.md) of the picture, `origin` fs.svg.convert, plus `width`, `height`, `format`. The store path is `ref`.",
                    "properties": {
                        "ref":    { "type": "string" },
                        "width":  { "type": "integer" },
                        "height": { "type": "integer" },
                        "format": { "type": "string" },
                        "size":   { "type": "integer" }
                    }
                },
                "layout": {
                    "type": "object",
                    "description": "The layout report: canvas {w,h}; texts[] and images[] with label and box {x,y,w,h} in canvas pixels (texts also lines); overlaps[] {a,b,area,w,h}; outside[] {label,by}; ok when both are empty.",
                    "required": ["canvas", "texts", "images", "overlaps", "outside", "ok"]
                }
            },
            "required": ["image", "layout"]
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        config_schema: json!({
            "type": "object",
            "properties": {
                "width":         { "type": "integer", "description": "Canvas width; absent = the SVG's own." },
                "height":        { "type": "integer", "description": "Canvas height; absent = the SVG's own." },
                "fit":           { "type": "string", "enum": ["cover", "contain", "fill"] },
                "format":        { "type": "string", "enum": ["png", "jpg", "webp", "pdf"] },
                "quality":       { "type": "integer", "description": "JPEG quality 1–100 (default 82)." },
                "folder":        { "type": "string", "description": "Destination store folder (default images)." },
                "source_key":    { "type": "string", "description": "Dot-path to the SVG in the payload (default svg)." },
                "delete_source": { "type": "boolean" },
                "filename":      { "type": "string", "description": "Filename without extension (default a UUID)." }
            }
        }),
        dsl_flags: vec![
            DslFlag { flag: "--width".into(), config_key: "width".into(), description: "Canvas width in pixels (default: the SVG's own; alone, height follows in proportion)".into(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--height".into(), config_key: "height".into(), description: "Canvas height in pixels (default: the SVG's own; alone, width follows in proportion)".into(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--fit".into(), config_key: "fit".into(), description: "cover | contain | fill (default: cover) — how the SVG reaches --width × --height".into(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--format".into(), config_key: "format".into(), description: "png | jpg | webp | pdf (default: png; pdf keeps text selectable and takes no --width/--height/--fit)".into(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--quality".into(), config_key: "quality".into(), description: "JPEG quality 1–100 (default: 82)".into(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--folder".into(), config_key: "folder".into(), description: "Destination store folder (default: images)".into(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--source-key".into(), config_key: "source_key".into(), description: "Dot-path to the SVG in the payload: SVG text, a store path, or a FileRef (default: `svg`)".into(), kind: DslFlagKind::Scalar, required: false },
            DslFlag { flag: "--delete-source".into(), config_key: "delete_source".into(), description: "Remove a stored source after the picture is written (default: false)".into(), kind: DslFlagKind::Bool, required: false },
            DslFlag { flag: "--filename".into(), config_key: "filename".into(), description: "Filename without extension (default: random UUID). Overwrites if same name exists.".into(), kind: DslFlagKind::Scalar, required: false },
        ],
        fields: vec![
            NodeFieldDef { name: "width".into(), label: "Width (px)".into(), field_type: NodeFieldType::Text, placeholder: Some("the SVG's own".into()), help: Some("Canvas width. Empty = the SVG's own width; alone, the height follows in proportion.".into()), ..Default::default() },
            NodeFieldDef { name: "height".into(), label: "Height (px)".into(), field_type: NodeFieldType::Text, placeholder: Some("the SVG's own".into()), help: Some("Canvas height. Empty = the SVG's own height; alone, the width follows in proportion.".into()), ..Default::default() },
            NodeFieldDef { name: "fit".into(), label: "Fit".into(), field_type: NodeFieldType::Select, default_value: Some(json!("cover")), help: Some("With both sides given: cover = fill the box and crop the middle; contain = fit inside it; fill = stretch.".into()), options: vec![
                SelectOptionDef { value: "cover".into(), label: "Cover (fill + crop center)".into() },
                SelectOptionDef { value: "contain".into(), label: "Contain (fit within)".into() },
                SelectOptionDef { value: "fill".into(), label: "Fill (stretch exact)".into() },
            ], ..Default::default() },
            NodeFieldDef { name: "format".into(), label: "Output format".into(), field_type: NodeFieldType::Select, default_value: Some(json!("png")), help: Some("png (default, keeps transparency), jpg (quality-controlled), webp (lossless), pdf (one page, text stays text; no size or fit).".into()), options: vec![
                SelectOptionDef { value: "png".into(), label: "PNG (lossless)".into() },
                SelectOptionDef { value: "jpg".into(), label: "JPEG (quality-controlled)".into() },
                SelectOptionDef { value: "webp".into(), label: "WebP (lossless)".into() },
                SelectOptionDef { value: "pdf".into(), label: "PDF (vector, selectable text)".into() },
            ], ..Default::default() },
            NodeFieldDef { name: "quality".into(), label: "JPEG quality".into(), field_type: NodeFieldType::Text, default_value: Some(json!("82")), help: Some("1–100, default 82. Only applies to JPEG output.".into()), ..Default::default() },
            NodeFieldDef { name: "folder".into(), label: "Folder".into(), field_type: NodeFieldType::Text, default_value: Some(json!(DEFAULT_FOLDER)), help: Some("Destination store folder (default: images). Under public/ for a page to show it anonymously.".into()), ..Default::default() },
            NodeFieldDef { name: "source_key".into(), label: "Source key".into(), field_type: NodeFieldType::Text, default_value: Some(json!(DEFAULT_SOURCE_KEY)), help: Some("Dot-path into the payload: SVG text, a store path string, or a FileRef of a stored .svg. Default: svg.".into()), ..Default::default() },
            NodeFieldDef { name: "delete_source".into(), label: "Delete source file".into(), field_type: NodeFieldType::Checkbox, default_value: Some(json!(false)), help: Some("Remove a stored source after the picture is written; the source key is dropped from the payload.".into()), ..Default::default() },
            NodeFieldDef { name: "filename".into(), label: "Filename".into(), field_type: NodeFieldType::Text, help: Some("Without extension (default: random UUID). A same-named file is overwritten.".into()), ..Default::default() },
        ],
        layout: vec![
            LayoutItem::Field("source_key".into()),
            LayoutItem::Field("width".into()),
            LayoutItem::Field("height".into()),
            LayoutItem::Field("fit".into()),
            LayoutItem::Field("format".into()),
            LayoutItem::Field("quality".into()),
            LayoutItem::Field("folder".into()),
            LayoutItem::Field("filename".into()),
            LayoutItem::Field("delete_source".into()),
        ],
        failure_semantics: vec![
            NodeFailureSemantic { code: "FW_NODE_FS_SVG_CONVERT_CONFIG".into(), description: "A --fit that is not cover, contain or fill; a --format that is not png, jpg, webp or pdf; a --width, --height or --quality that is not a whole number in range; --width, --height or --fit given with --format pdf.".into(), ..Default::default() },
            NodeFailureSemantic { code: "FS_SVG_CONVERT_SOURCE".into(), description: "Nothing at --source-key, or it is not SVG; the SVG does not parse or has no size; an <image href> that is a URL, a data URI or a path outside the project, or a picture over its cap. The message names the href.".into(), ..Default::default() },
            NodeFailureSemantic { code: "FS_SVG_CONVERT_FONT".into(), description: "A font-family the SVG names is neither the bundled Inter nor a face the project has under static/fonts/; the message lists what exists.".into(), ..Default::default() },
            NodeFailureSemantic { code: "FS_SVG_CONVERT_RASTER".into(), description: "resvg, the encoder or the store write failed.".into(), retryable: true, ..Default::default() },
        ],
        examples: vec![
            NodeExample::dsl("A poster the model wrote, kept as a PNG", "fs.svg.convert --source-key data.svg --folder sandbox/posters/out --preview image")
                .input(json!({ "data": { "svg": "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1080\" height=\"1350\"><rect width=\"1080\" height=\"1350\" fill=\"#012169\"/><text x=\"540\" y=\"640\" font-family=\"Inter\" font-weight=\"800\" font-size=\"120\" fill=\"#fff\" text-anchor=\"middle\" inline-size=\"918\">RESEARCH SHOWCASE NIGHT</text></svg>" } }))
                .output(json!({ "data": { "svg": "<svg …>" }, "image": { "__zf_type": "file_ref", "backend": "zebfs", "ref": "sandbox/posters/out/9f2c….png", "filename": "9f2c….png", "mime": "image/png", "kind": "image", "size": 412300, "sha256": "sha256:…", "lifecycle": "durable", "origin": "fs.svg.convert", "trust": "generated", "width": 1080, "height": 1350, "format": "png" } }))
                .note("`ai.agent --schema '{\"type\":\"object\",\"required\":[\"svg\"],\"properties\":{\"svg\":{\"type\":\"string\"}}}'` before it answers the SVG as `data.svg`. `inline-size` wraps the headline; pictures are `<image href=\"sandbox/posters/photos/venue.jpg\">`."),
            NodeExample::dsl("A certificate as a PDF, the name shrunk to its line", "fs.svg.convert --source-key svg --format pdf --folder certificates --filename cert-2026-0412")
                .input(json!({ "svg": "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1123\" height=\"794\">…<text x=\"561\" y=\"420\" font-family=\"Inter\" font-size=\"64\" text-anchor=\"middle\" inline-size=\"900\" data-fit=\"shrink\" data-min-size=\"28\">Alexandra Josephine Montgomery Whitfield</text>…</svg>" }))
                .note("A `fs.get` of the template .svg and a `script` that fills the placeholders come before it. The name is real text in the PDF; a long one shrinks instead of wrapping. `--width/--height/--fit` are refused with pdf."),
            NodeExample::dsl("A stored SVG at a size", "fs.svg.convert --source-key saved --width 512 --height 512 --fit contain --format webp --folder public/logos")
                .input(json!({ "saved": { "__zf_type": "file_ref", "backend": "zebfs", "ref": "uploads/logo.svg", "filename": "logo.svg", "mime": "image/svg+xml", "kind": "image", "size": 2210, "sha256": "sha256:…", "lifecycle": "durable", "origin": "fs.save", "trust": "untrusted" } }))
                .note("After `fs.save --allowed-kinds images`. `contain` keeps the proportions, so a wide logo answers 512×n."),
        ],
        ..Default::default()
    }
}

/// The whole conversion over text, without the store: what the node does
/// once it holds the source.
pub fn convert(
    svg: &str,
    fonts: &FontSet,
    resolver: &Resolver,
    target: &Target,
    format: OutputFormat,
    quality: u8,
) -> Result<Rendered, ConvertError> {
    if svg.len() > MAX_SVG_BYTES {
        return Err(ConvertError::source(format!("the svg is {} bytes; the limit is {MAX_SVG_BYTES}", svg.len())));
    }
    if !looks_like_svg(svg.as_bytes()) {
        return Err(ConvertError::source("the source is not an SVG: it does not start with <svg or <?xml"));
    }
    // Depth first, off the text: every parser and walker below recurses,
    // roxmltree's own included, so a post-parse check would never run.
    limits::check_depth(svg)?;
    let svg = sources::strip_safe_doctype(svg)?;
    let doc = roxmltree::Document::parse(&svg).map_err(|e| ConvertError::source(format!("svg does not parse: {e}")))?;
    // Filters are the one input whose cost the canvas caps do not bound.
    let declared = declared_size(&doc);
    limits::check_filters(&doc, declared)?;
    for href in sources::image_hrefs(&doc) {
        if let Some(source) = Source::parse_href(&href)? {
            resolver.prefetch(&source)?;
        }
    }
    let prepared = text::prepare(&doc, fonts)?;
    render(&prepared, fonts, resolver, target, format, quality)
}

/// The page size the root element declares, for reading a filter region
/// given in absolute lengths. Zero when it says nothing, which only makes
/// such a region unmeasurable and therefore unrefused.
fn declared_size(doc: &roxmltree::Document<'_>) -> (f32, f32) {
    let root = doc.root_element();
    let side = |name: &str| {
        root.attribute(name)
            .and_then(|v| v.trim().trim_end_matches("px").parse::<f32>().ok())
            .unwrap_or(0.0)
    };
    let (w, h) = (side("width"), side("height"));
    if w > 0.0 && h > 0.0 {
        return (w, h);
    }
    // No width/height: fall back to the viewBox's own extent.
    if let Some(vb) = root.attribute("viewBox") {
        let n: Vec<f32> = vb
            .split(|c: char| c == ',' || c.is_whitespace())
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse::<f32>().ok())
            .collect();
        if n.len() == 4 {
            return (n[2], n[3]);
        }
    }
    (w, h)
}

pub struct Node {
    config: Config,
    platform: Arc<PlatformService>,
    target: Target,
    format: OutputFormat,
    quality: u8,
}

fn config_error(message: impl Into<String>) -> PipelineError {
    PipelineError::new("FW_NODE_FS_SVG_CONVERT_CONFIG", message)
}

impl Node {
    pub fn new(config: Config, platform: Arc<PlatformService>) -> Result<Self, PipelineError> {
        let side = |name: &str, v: Option<u32>| match v {
            Some(0) | Some(u32::MAX) => Err(config_error(format!("--{name} must be a whole number of pixels, 1 or more"))),
            other => Ok(other),
        };
        let width = side("width", config.width)?;
        let height = side("height", config.height)?;
        let fit = match config.fit.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            None => Fit::Cover,
            Some(raw) => Fit::parse(raw).ok_or_else(|| config_error(format!("--fit '{raw}' is not cover, contain or fill")))?,
        };
        let format = match config.format.as_deref().map(str::trim).filter(|s| !s.is_empty()) {
            None => OutputFormat::Png,
            Some(raw) => OutputFormat::parse(raw).ok_or_else(|| config_error(format!("--format '{raw}' is not png, jpg or webp")))?,
        };
        let quality = match config.quality {
            None => DEFAULT_QUALITY,
            Some(q @ 1..=100) => q as u8,
            Some(_) => return Err(config_error("--quality must be a whole number from 1 to 100")),
        };
        if format == OutputFormat::Pdf && (width.is_some() || height.is_some() || config.fit.as_deref().is_some_and(|f| !f.trim().is_empty())) {
            return Err(config_error("--format pdf is the SVG's own size: it takes no --width, --height or --fit"));
        }
        Ok(Self { config, platform, target: Target { width, height, fit }, format, quality })
    }

    fn source_key(&self) -> &str {
        self.config.source_key.as_deref().map(str::trim).filter(|s| !s.is_empty()).unwrap_or(DEFAULT_SOURCE_KEY)
    }

    fn folder(&self) -> &str {
        self.config.folder.as_deref().map(str::trim).filter(|s| !s.is_empty()).unwrap_or(DEFAULT_FOLDER)
    }
}

/// `root/rel`, or nothing when `rel` climbs out.
fn contained(root: &Path, rel: &str) -> Option<PathBuf> {
    let rel = rel.replace('\\', "/");
    if rel.starts_with('/') || rel.split('/').any(|seg| seg == ".." || seg.is_empty()) {
        return None;
    }
    Some(root.join(rel))
}

/// The project's bytes for the resolver: repo `static/` and the store.
struct ProjectStore {
    layout: crate::platform::model::ProjectFileLayout,
}

impl SourceStore for ProjectStore {
    fn read(&self, source: &Source) -> Result<Vec<u8>, ConvertError> {
        match source {
            Source::Repo(path) => {
                let abs = contained(&self.layout.repo_source_dir(), path)
                    .ok_or_else(|| ConvertError::source(format!("repo file {path} must stay inside the project")))?;
                std::fs::read(&abs).map_err(|e| ConvertError::source(format!("repo file {path}: {e}")))
            }
            Source::Store(path) => self
                .layout
                .open_files()
                .get(path)
                .map(|o| o.bytes)
                .map_err(|e| ConvertError::source(format!("store object {path}: {}", e.message))),
        }
    }
}

fn convert_error(e: ConvertError) -> PipelineError {
    let code = match e.kind {
        ConvertErrorKind::Source => "FS_SVG_CONVERT_SOURCE",
        ConvertErrorKind::Font => "FS_SVG_CONVERT_FONT",
        ConvertErrorKind::Raster => "FS_SVG_CONVERT_RASTER",
    };
    PipelineError::new(code, e.message)
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
            .map_err(|e| PipelineError::new("FS_SVG_CONVERT_RASTER", e.to_string()))?;
        let zebfs = layout.open_files();

        // 1. The source: SVG text, or a stored .svg by path or FileRef.
        let key = self.source_key();
        let value = resolve_path(&input.payload, key).ok_or_else(|| {
            PipelineError::new("FS_SVG_CONVERT_SOURCE", format!("nothing at payload key '{key}' — set --source-key to where the SVG is"))
        })?;
        let (svg, stored_source) = match value.as_str().filter(|s| looks_like_svg(s.as_bytes())) {
            Some(text) => (text.to_string(), None),
            None => {
                let rel = zebfs_rel_path_or_string(value)?.ok_or_else(|| {
                    PipelineError::new(
                        "FS_SVG_CONVERT_SOURCE",
                        format!("payload key '{key}' must be SVG text, a store path string, or a FileRef of a stored .svg"),
                    )
                })?;
                let rel = crate::zebfs::normalize_object_path(rel.trim_start_matches('/'))
                    .map_err(|e| PipelineError::new("FS_SVG_CONVERT_SOURCE", e.to_string()))?;
                let object = zebfs
                    .get(&rel)
                    .map_err(|e| PipelineError::new("FS_SVG_CONVERT_SOURCE", format!("store object {rel}: {}", e.message)))?;
                if object.bytes.len() > MAX_SVG_BYTES {
                    return Err(PipelineError::new(
                        "FS_SVG_CONVERT_SOURCE",
                        format!("store object {rel} is {} bytes; the limit is {MAX_SVG_BYTES}", object.bytes.len()),
                    ));
                }
                let text = String::from_utf8(object.bytes)
                    .map_err(|_| PipelineError::new("FS_SVG_CONVERT_SOURCE", format!("store object {rel} is not UTF-8 text")))?;
                if !looks_like_svg(text.as_bytes()) {
                    return Err(PipelineError::new("FS_SVG_CONVERT_SOURCE", format!("store object {rel} is not an SVG")));
                }
                (text, Some(rel))
            }
        };

        // 2. Fonts, pictures, wrapping, pixels.
        let fonts = FontSet::for_project(&self.platform.fonts, owner, project).map_err(convert_error)?;
        let resolver = Resolver::new(Arc::new(ProjectStore { layout: layout.clone() }));
        let rendered = convert(&svg, &fonts, &resolver, &self.target, self.format, self.quality).map_err(convert_error)?;

        // 3. Into the store, as a durable FileRef.
        let stem = self.config.filename.as_deref().map(filename_stem).filter(|s| !s.is_empty()).unwrap_or_else(|| Uuid::new_v4().to_string());
        let filename = format!("{stem}.{}", self.format.extension());
        let rel = crate::zebfs::normalize_object_path(&format!("{}/{filename}", self.folder().trim_matches('/')))
            .map_err(|e| config_error(format!("--folder: {e}")))?;
        zebfs
            .put(&rel, &rendered.bytes)
            .map_err(|e| PipelineError::new("FS_SVG_CONVERT_RASTER", format!("store write {rel}: {}", e.message)))?;
        let mut image = durable_file_ref(layout.file_backend(), &rel, &filename, self.format.mime(), &rendered.bytes, ORIGIN, "generated");
        if let Some(obj) = image.as_object_mut() {
            obj.insert("width".into(), json!(rendered.width));
            obj.insert("height".into(), json!(rendered.height));
            obj.insert("format".into(), json!(self.format.extension()));
        }

        // 4. Merge, do not replace; a deleted source loses its key.
        let mut out = match &input.payload {
            Value::Object(map) => map.clone(),
            _ => serde_json::Map::new(),
        };
        if self.config.delete_source {
            if let Some(src) = &stored_source {
                if let Err(e) = std::fs::remove_file(layout.local_files_dir()?.join(src)) {
                    eprintln!("[{NODE_KIND}] delete-source failed for {src}: {e}");
                }
            }
            if let Some(top) = key.split('.').next() {
                out.remove(top);
            }
        }
        out.insert("image".to_string(), image);
        out.insert("layout".to_string(), rendered.layout);

        let total_ms = started.elapsed().as_millis() as u64;
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: Value::Object(out),
            trace: vec![
                format!("node={} node_kind={NODE_KIND}", input.node_id),
                format!(
                    "source={} out={rel} {}x{} {} bytes={} parse_ms={} raster_ms={} encode_ms={} total_ms={total_ms}",
                    stored_source.as_deref().unwrap_or("text"),
                    rendered.width,
                    rendered.height,
                    self.format.extension(),
                    rendered.bytes.len(),
                    rendered.timing.parse_ms,
                    rendered.timing.raster_ms,
                    rendered.timing.encode_ms
                ),
            ],
        })
    }
}

#[cfg(test)]
mod node_tests {
    use super::*;

    fn test_platform() -> Arc<PlatformService> {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut config = crate::platform::model::PlatformConfig::default();
        config.data_root = tmp.keep().join("platform");
        config.default_password = "secret".to_string();
        Arc::new(PlatformService::from_config(config).expect("platform"))
    }

    #[test]
    fn the_config_is_checked_and_numbers_arrive_as_numbers_or_text() {
        let platform = test_platform();
        let bad = |config: Value| Node::new(serde_json::from_value(config).unwrap(), platform.clone()).map(|_| ()).unwrap_err();
        assert!(bad(json!({ "fit": "stretch" })).message.contains("stretch"));
        assert!(bad(json!({ "format": "gif" })).message.contains("gif"));
        assert!(bad(json!({ "format": "pdf", "width": 500 })).message.contains("pdf"));
        assert!(bad(json!({ "width": "wide" })).message.contains("--width"));
        assert!(bad(json!({ "height": 0 })).message.contains("--height"));
        assert!(bad(json!({ "quality": 101 })).message.contains("--quality"));
        assert_eq!(bad(json!({ "quality": "0" })).code, "FW_NODE_FS_SVG_CONVERT_CONFIG");
        let ok = Node::new(serde_json::from_value(json!({ "width": "512", "height": 512, "fit": "Contain", "format": "JPG", "quality": "90", "folder": "", "source_key": " " })).unwrap(), platform.clone()).unwrap();
        assert_eq!((ok.target.width, ok.target.height, ok.target.fit), (Some(512), Some(512), Fit::Contain));
        assert_eq!((ok.format, ok.quality), (OutputFormat::Jpg, 90));
        assert_eq!((ok.folder(), ok.source_key()), (DEFAULT_FOLDER, DEFAULT_SOURCE_KEY));
        let plain = Node::new(Config::default(), platform).unwrap();
        assert_eq!((plain.target.width, plain.target.height, plain.format, plain.quality), (None, None, OutputFormat::Png, DEFAULT_QUALITY));
    }

    #[test]
    fn the_definition_carries_the_thumbnail_flag_set_and_the_four_failures() {
        let def = definition();
        let flags: Vec<&str> = def.dsl_flags.iter().map(|f| f.flag.as_str()).collect();
        assert_eq!(
            flags,
            vec!["--width", "--height", "--fit", "--format", "--quality", "--folder", "--source-key", "--delete-source", "--filename"]
        );
        let codes: Vec<&str> = def.failure_semantics.iter().map(|f| f.code.as_str()).collect();
        assert_eq!(codes, vec!["FW_NODE_FS_SVG_CONVERT_CONFIG", "FS_SVG_CONVERT_SOURCE", "FS_SVG_CONVERT_FONT", "FS_SVG_CONVERT_RASTER"]);
        assert!(contained(Path::new("/root"), "../x.svg").is_none());
        assert_eq!(contained(Path::new("/root"), "static/x.svg"), Some(PathBuf::from("/root/static/x.svg")));
    }
}
