//! `fs.image.render` — an SVG, as a picture. No browser: resvg draws it.
//!
//! | Use | DSL |
//! |---|---|
//! | The SVG a model wrote, kept as a PNG | `\| ai.text.generate --provider openrouter --credential openrouter --schema '{"type":"object","required":["svg"],"properties":{"svg":{"type":"string"}}}' --prompt "…" \| fs.image.render --text "{{ input.text.data.svg }}" --folder posters --preview image` |
//! | A stored SVG at a size | `\| fs.image.render --from "{{ input.file }}" --width 512 --height 512 --fit contain --format webp --folder logos` |
//!
//! # The source
//!
//! Exactly one: `--from`, a stored `.svg` — a FileRef, an upload or a store
//! key — or `--text`, the SVG markup itself (`<svg …>…</svg>`). Up to 512 KB.
//!
//! # What the node adds to plain SVG
//!
//! - **Pictures from the project only.** An `<image href>` is a store path
//!   (`sandbox/posters/photos/venue.jpg`, or the same behind `zebfs://`) or
//!   a repository file under `static/` (`repo://static/brand/logo.svg`). A
//!   URL or a `data:` URI is refused, naming the href — fetch with
//!   `http.response.fetch --response-type bytes`, `fs.file.put` it, then name the path.
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
//! - **The layout report.** Inside `image` the answer carries `layout`: every
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
//! The payload plus `image`: a durable FileRef (`origin: fs.image.render`,
//! `trust: generated`) with `width`, `height` and `format` beside the eleven
//! contract fields, written under `--folder` (default `images/`) as
//! `--filename` or a UUID, with `image.layout`. `--format png|jpg|webp|pdf`
//! (default png; `--quality` is JPEG's). With `--delete-source` a stored source is removed once the
//! picture is written and `image.source_deleted` says so (`--text` has nothing
//! to delete and refuses it); the payload keeps every key.

use std::sync::Arc;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::pipeline::model::{
    DslFlag, DslFlagKind, LayoutItem, NodeCapability, NodeExample, NodeFailureSemantic, NodeFieldDef, NodeFieldType,
    SelectOptionDef,
};
use crate::pipeline::nodes::shared::limits::{choice, whole, within};
use crate::pipeline::nodes::shared::project_store::{OnConflict, on_conflict_flag, open_from, open_store, store_fields, store_flag, target_key};
use crate::pipeline::nodes::shared::util::{filename_stem, metadata_scope, with_answer};
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

pub const NODE_KIND: &str = "fs.image.render";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";
pub const ORIGIN: &str = "fs.image.render";
const DEFAULT_FOLDER: &str = "images";
const DEFAULT_QUALITY: u32 = 82;

/// A flag the author set wrong.
pub const CONFIG_CODE: &str = "FW_NODE_FS_IMAGE_RENDER_CONFIG";
/// No SVG, or one that does not parse, or a picture it may not load.
const SOURCE_CODE: &str = "FW_NODE_FS_IMAGE_RENDER_SOURCE";
/// A font the SVG names that the project does not have.
const FONT_CODE: &str = "FW_NODE_FS_IMAGE_RENDER_FONT";
/// resvg, the encoder or the store write.
const RASTER_CODE: &str = "FW_NODE_FS_IMAGE_RENDER_RASTER";

const FITS: &[&str] = &["cover", "contain", "fill"];
const FORMATS: &[&str] = &["png", "jpg", "webp", "pdf"];

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// A stored `.svg`: a FileRef, an upload or a store key.
    #[serde(default)]
    pub from: Value,
    /// The SVG markup itself.
    #[serde(default)]
    pub text: Value,
    /// Canvas width in pixels; absent = the SVG's own (or follows `height`).
    #[serde(default)]
    pub width: Value,
    /// Canvas height in pixels; absent = the SVG's own (or follows `width`).
    #[serde(default)]
    pub height: Value,
    /// cover | contain | fill (default cover) — only with both sides given.
    #[serde(default)]
    pub fit: String,
    /// png | jpg | webp | pdf (default png).
    #[serde(default)]
    pub format: String,
    /// JPEG quality 1–100 (default 82).
    #[serde(default)]
    pub quality: Value,
    /// Remove the `--from` file after the picture is written.
    #[serde(default)]
    pub delete_source: bool,
    /// The store to write to; saved explicitly at registration.
    #[serde(default)]
    pub store: Option<String>,
    /// Destination store folder (default `images`).
    #[serde(default)]
    pub folder: Option<String>,
    /// Filename without extension (default: a generated name).
    #[serde(default)]
    pub filename: Option<String>,
    /// Exact store key; overrides `folder` and `filename`.
    #[serde(default)]
    pub path: Option<String>,
    /// `overwrite`, `skip` or `error` (default: error).
    #[serde(default)]
    pub on_conflict: Option<String>,
}

fn flag(name: &str, key: &str, description: &str, value: &str) -> DslFlag {
    DslFlag { flag: name.into(), config_key: key.into(), description: description.into(), kind: DslFlagKind::Scalar, required: false, value: value.into(), ..Default::default() }
}

fn words(list: &[&str]) -> Vec<String> {
    list.iter().map(|word| word.to_string()).collect()
}

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Filesystem],
        title: "Image Render".to_string(),
        description: "Draw an SVG as a picture, no browser. The SVG is exactly one of `--from` — a stored .svg: a FileRef, an upload or a store key — or `--text`, the markup itself. \
            Pictures inside it come from the project only — `<image href>` is a store path or `repo://static/…`, never a URL or a data URI. Text is shaped with the bundled Inter \
            or a family the project has under `static/fonts/` (named by family; an unknown one is refused with the list); `<text inline-size=\"900\">` wraps its lines and \
            `data-fit=\"shrink\"` beside it shrinks the size until the text fits `data-max-lines` (default 1). Effects are SVG filters. \
            With no `--width`/`--height` the canvas is the SVG's own size; one side scales the other; both use `--fit cover|contain|fill`. Writes `--format png|jpg|webp|pdf` \
            (default png, `--quality` for jpg; pdf keeps text as text and takes no size flags) under `--folder` (default `images/`) and adds `image` — a durable FileRef with \
            `width`, `height`, `format`, `source_deleted` and `layout` (every text and picture box, the overlaps, what leaves the canvas, `ok`) — keeping the rest of the payload; \
            `--delete-source` removes the `--from` file once the picture is written. Look at it with `--preview image`."
            .to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "image": {
                    "type": "object",
                    "description": "A durable FileRef (kinds/file-ref/README.md) of the picture, `origin` fs.image.render, plus `width`, `height`, `format`, `layout` and `source_deleted`. The store path is `ref`.",
                    "properties": {
                        "ref":    { "type": "string" },
                        "width":  { "type": "integer" },
                        "height": { "type": "integer" },
                        "format": { "type": "string" },
                        "size":   { "type": "integer" },
                        "source_deleted": { "type": "boolean" },
                        "layout": {
                            "type": "object",
                            "description": "The layout report: canvas {w,h}; texts[] and images[] with label and box {x,y,w,h} in canvas pixels (texts also lines); overlaps[] {a,b,area,w,h}; outside[] {label,by}; ok when both are empty.",
                            "required": ["canvas", "texts", "images", "overlaps", "outside", "ok"]
                        }
                    }
                }
            },
            "required": ["image"]
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        config_schema: Default::default(),
        dsl_flags: vec![
            flag("--from", "from", "A stored .svg: a FileRef, an upload or a store key. Set --from or --text.", "file"),
            flag("--text", "text", "The SVG markup itself, usually {{ input.… }}. Set --from or --text.", "text"),
            flag("--width", "width", "Canvas width in pixels (default: the SVG's own; alone, height follows in proportion).", "number"),
            flag("--height", "height", "Canvas height in pixels (default: the SVG's own; alone, width follows in proportion).", "number"),
            DslFlag { choices: words(FITS), ..flag("--fit", "fit", "cover (default), contain or fill — how the SVG reaches --width × --height.", "") },
            DslFlag { choices: words(FORMATS), ..flag("--format", "format", "png (default), jpg, webp or pdf (text stays selectable; no --width/--height/--fit).", "") },
            flag("--quality", "quality", "JPEG quality 1–100 (default: 82).", "number"),
            DslFlag { kind: DslFlagKind::Bool, ..flag("--delete-source", "delete_source", "Delete the --from file once the picture is written.", "") },
            DslFlag { value: "text".into(), ..store_flag() },
            flag("--folder", "folder", "Destination folder (default: images).", "text"),
            flag("--filename", "filename", "Destination name without extension (default: a generated name).", "text"),
            flag("--path", "path", "Exact destination key; overrides --folder and --filename.", "text"),
            DslFlag { choices: words(&["error", "skip", "overwrite"]), ..on_conflict_flag(OnConflict::Error) },
        ],
        fields: vec![
            NodeFieldDef { name: "from".into(), label: "From".into(), field_type: NodeFieldType::Text, help: Some("A stored .svg, e.g. {{ input.file }}. Set From or Text.".into()), ..Default::default() },
            NodeFieldDef { name: "text".into(), label: "Text".into(), field_type: NodeFieldType::Textarea, rows: Some(6), help: Some("The SVG markup, e.g. {{ input.text.data.svg }}. Set From or Text.".into()), ..Default::default() },
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
            NodeFieldDef { name: "delete_source".into(), label: "Delete source file".into(), field_type: NodeFieldType::Checkbox, default_value: Some(json!(false)), help: Some("Remove the From file once the picture is written.".into()), ..Default::default() },
            NodeFieldDef { name: "folder".into(), label: "Folder".into(), field_type: NodeFieldType::Text, default_value: Some(json!(DEFAULT_FOLDER)), help: Some("Destination folder (default: images). Private until the owner exposes it in Studio → Files.".into()), ..Default::default() },
            NodeFieldDef { name: "filename".into(), label: "Filename".into(), field_type: NodeFieldType::Text, help: Some("Without extension (default: a generated name).".into()), ..Default::default() },
            NodeFieldDef { name: "path".into(), label: "Path".into(), field_type: NodeFieldType::Text, help: Some("Exact destination key; overrides folder and filename.".into()), ..Default::default() },
        ].into_iter().chain(store_fields(OnConflict::Error)).collect(),
        layout: ["from", "text", "width", "height", "fit", "format", "quality", "delete_source", "folder", "filename", "path", "store", "on_conflict"]
            .iter()
            .map(|name| LayoutItem::Field(name.to_string()))
            .collect(),
        failure_semantics: vec![
            NodeFailureSemantic { code: CONFIG_CODE.into(), description: "A --fit that is not cover, contain or fill; a --format that is not png, jpg, webp or pdf; a --width, --height or --quality that is not a whole number in range; --width, --height or --fit given with --format pdf; --delete-source with --text.".into(), ..Default::default() },
            NodeFailureSemantic { code: SOURCE_CODE.into(), description: "Neither or both of --from and --text, or what they hold is not SVG; the SVG does not parse or has no size; an <image href> that is a URL, a data URI or a path outside the project, or a picture over its cap. The message names the href.".into(), ..Default::default() },
            NodeFailureSemantic { code: FONT_CODE.into(), description: "A font-family the SVG names is neither the bundled Inter nor a face the project has under static/fonts/; the message lists what exists.".into(), ..Default::default() },
            NodeFailureSemantic { code: RASTER_CODE.into(), description: "resvg, the encoder or the store write failed.".into(), retryable: true, ..Default::default() },
        ],
        examples: vec![
            NodeExample::dsl("A poster the model wrote, kept as a PNG", "fs.image.render --text \"{{ input.text.data.svg }}\" --folder sandbox/posters/out --preview image")
                .input(json!({ "text": { "data": { "svg": "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1080\" height=\"1350\"><rect width=\"1080\" height=\"1350\" fill=\"#012169\"/><text x=\"540\" y=\"640\" font-family=\"Inter\" font-weight=\"800\" font-size=\"120\" fill=\"#fff\" text-anchor=\"middle\" inline-size=\"918\">RESEARCH SHOWCASE NIGHT</text></svg>" } } }))
                .output(json!({ "text": { "data": { "svg": "<svg …>" } }, "image": { "__zf_type": "file_ref", "backend": "zebfs", "store": "local", "ref": "sandbox/posters/out/9f2c….png", "filename": "9f2c….png", "mime": "image/png", "kind": "image", "size": 412300, "sha256": "sha256:…", "lifecycle": "durable", "origin": "fs.image.render", "trust": "generated", "width": 1080, "height": 1350, "format": "png", "source_deleted": false } }))
                .note("`ai.text.generate --schema '{\"type\":\"object\",\"required\":[\"svg\"],\"properties\":{\"svg\":{\"type\":\"string\"}}}'` before it answers the SVG as `text.data.svg`. `inline-size` wraps the headline; pictures are `<image href=\"sandbox/posters/photos/venue.jpg\">`."),
            NodeExample::dsl("A certificate as a PDF, the name shrunk to its line", "fs.image.render --text \"{{ input.svg }}\" --format pdf --folder certificates --filename cert-2026-0412")
                .input(json!({ "svg": "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"1123\" height=\"794\">…<text x=\"561\" y=\"420\" font-family=\"Inter\" font-size=\"64\" text-anchor=\"middle\" inline-size=\"900\" data-fit=\"shrink\" data-min-size=\"28\">Alexandra Josephine Montgomery Whitfield</text>…</svg>" }))
                .note("A `fs.file.get` of the template .svg and a `script` that fills the placeholders come before it. The name is real text in the PDF; a long one shrinks instead of wrapping. `--width/--height/--fit` are refused with pdf."),
            NodeExample::dsl("A stored SVG at a size", "fs.image.render --from \"{{ input.file }}\" --width 512 --height 512 --fit contain --format webp --folder logos")
                .input(json!({ "file": { "__zf_type": "file_ref", "backend": "zebfs", "store": "local", "ref": "uploads/logo.svg", "filename": "logo.svg", "mime": "image/svg+xml", "kind": "image", "size": 2210, "sha256": "sha256:…", "lifecycle": "durable", "origin": "fs.file.put", "trust": "untrusted" } }))
                .note("After `fs.file.put --from \"{{ input.webhook.files.logo }}\" --accept image`. `contain` keeps the proportions, so a wide logo answers 512×n."),
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
    PipelineError::new(CONFIG_CODE, message)
}

/// Where the SVG comes from: `--from` or `--text`, exactly one.
enum SvgSource<'a> {
    From(&'a Value),
    Text(&'a Value),
}

impl Node {
    pub fn new(config: Config, platform: Arc<PlatformService>) -> Result<Self, PipelineError> {
        let side = |name: &str, v: &Value| match whole(v, name, CONFIG_CODE)? {
            Some(0) => Err(config_error(format!("{name} must be a whole number of pixels, 1 or more"))),
            other => Ok(other),
        };
        let width = side("--width", &config.width)?;
        let height = side("--height", &config.height)?;
        let fit = Fit::parse(choice(&config.fit, FITS, "cover", "--fit", CONFIG_CODE)?).expect("a listed fit");
        let format = OutputFormat::parse(choice(&config.format, FORMATS, "png", "--format", CONFIG_CODE)?).expect("a listed format");
        let quality = within(whole(&config.quality, "--quality", CONFIG_CODE)?.unwrap_or(DEFAULT_QUALITY), 1, 100, "--quality", CONFIG_CODE)? as u8;
        if format == OutputFormat::Pdf && (width.is_some() || height.is_some() || !config.fit.trim().is_empty()) {
            return Err(config_error("--format pdf is the SVG's own size: it takes no --width, --height or --fit"));
        }
        OnConflict::parse(config.on_conflict.as_deref(), OnConflict::Error, CONFIG_CODE)?;
        Ok(Self { config, platform, target: Target { width, height, fit }, format, quality })
    }

    /// Exactly one of `--from` and `--text`; null or an empty string is unset.
    fn svg_source(&self) -> Result<SvgSource<'_>, PipelineError> {
        let given = |value: &Value| !(value.is_null() || value.as_str().is_some_and(|s| s.trim().is_empty()));
        match (given(&self.config.from), given(&self.config.text)) {
            (true, false) => Ok(SvgSource::From(&self.config.from)),
            (false, true) if self.config.delete_source => Err(config_error("--delete-source removes a --from file; --text has nothing to delete")),
            (false, true) => Ok(SvgSource::Text(&self.config.text)),
            (false, false) => Err(PipelineError::new(SOURCE_CODE, "set one source: --from (a stored .svg) or --text (the markup)")),
            (true, true) => Err(PipelineError::new(SOURCE_CODE, "set one source, not --from and --text")),
        }
    }

    fn folder(&self) -> &str {
        self.config.folder.as_deref().map(str::trim).filter(|s| !s.is_empty()).unwrap_or(DEFAULT_FOLDER)
    }
}

/// The project's bytes for the resolver: repo `static/` and the node's store.
struct ProjectStore {
    layout: crate::platform::model::ProjectFileLayout,
    store: crate::zebfs::ZebFs,
}

impl SourceStore for ProjectStore {
    fn read(&self, source: &Source) -> Result<Vec<u8>, ConvertError> {
        match source {
            Source::Repo(path) => crate::pipeline::nodes::shared::project_store::read_repo_file(
                &self.layout.repo_source_dir(),
                path,
                SOURCE_CODE,
            )
            .map_err(|e| ConvertError::source(format!("repo file {path}: {}", e.message))),
            Source::Store(path) => {
                crate::pipeline::nodes::shared::project_store::read_capped(&self.store, path, SOURCE_CODE)
                    .map_err(|e| ConvertError::source(format!("store object {path}: {}", e.message)))
            }
        }
    }
}

fn convert_error(e: ConvertError) -> PipelineError {
    let code = match e.kind {
        ConvertErrorKind::Source => SOURCE_CODE,
        ConvertErrorKind::Font => FONT_CODE,
        ConvertErrorKind::Raster => RASTER_CODE,
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
            .map_err(|e| PipelineError::new(RASTER_CODE, e.to_string()))?;
        let store = open_store(&self.platform, owner, project, self.config.store.as_deref())?;

        // 1. The source: the markup, or a stored .svg by FileRef or key.
        let (svg, stored_source) = match self.svg_source()? {
            SvgSource::Text(value) => {
                let Some(text) = value.as_str().filter(|text| looks_like_svg(text.as_bytes())) else {
                    return Err(PipelineError::new(SOURCE_CODE, "--text is not SVG markup: it does not start with <svg or <?xml"));
                };
                (text.to_string(), None)
            }
            SvgSource::From(value) => {
                let (source_store, rel) =
                    open_from(&self.platform, owner, project, value, self.config.store.as_deref(), "--from", SOURCE_CODE)?;
                let stat = source_store.fs.head(&rel).map_err(|e| PipelineError::new(SOURCE_CODE, format!("--from '{rel}': {}", e.message)))?;
                if stat.size > MAX_SVG_BYTES as u64 {
                    return Err(PipelineError::new(SOURCE_CODE, format!("--from '{rel}' is {} bytes; the limit is {MAX_SVG_BYTES}", stat.size)));
                }
                let bytes = source_store.read_capped(&rel, SOURCE_CODE)?;
                let text = String::from_utf8(bytes).map_err(|_| PipelineError::new(SOURCE_CODE, format!("--from '{rel}' is not UTF-8 text")))?;
                if !looks_like_svg(text.as_bytes()) {
                    return Err(PipelineError::new(SOURCE_CODE, format!("--from '{rel}' is not an SVG")));
                }
                (text, Some((source_store, rel)))
            }
        };

        // 2. Fonts, pictures, wrapping, pixels.
        let fonts = FontSet::for_project(&self.platform.fonts, owner, project).map_err(convert_error)?;
        let resolver = Resolver::new(Arc::new(ProjectStore { layout: layout.clone(), store: store.fs.clone() }));
        let rendered = convert(&svg, &fonts, &resolver, &self.target, self.format, self.quality).map_err(convert_error)?;

        // 3. Into the store, as a durable FileRef.
        let stem = self.config.filename.as_deref().map(filename_stem).filter(|s| !s.is_empty()).unwrap_or_else(|| Uuid::new_v4().to_string());
        let filename = format!("{stem}.{}", self.format.extension());
        let rel = target_key(self.config.path.as_deref(), self.folder(), &filename, CONFIG_CODE)?;
        let on_conflict = OnConflict::parse(self.config.on_conflict.as_deref(), OnConflict::Error, CONFIG_CODE)?;
        // A skipped write answers the file already there, as it is.
        if !on_conflict.allows(&store.fs, &rel, RASTER_CODE)? {
            let existing = store.stored_ref(&rel, ORIGIN, "generated", RASTER_CODE)?;
            return Ok(NodeExecutionOutput {
                output_pins: vec![OUTPUT_PIN_OUT.to_string()],
                payload: with_answer(&input.payload, json!({ "image": existing })),
                trace: vec![format!("node={} node_kind={NODE_KIND} out={rel} skipped", input.node_id)],
            });
        }
        store
            .fs
            .put(&rel, &rendered.bytes)
            .map_err(|e| PipelineError::new(RASTER_CODE, format!("store write {rel}: {}", e.message)))?;
        // The source goes only once the output is written and is not it.
        let mut source_deleted = false;
        if self.config.delete_source
            && let Some((source_store, src)) = &stored_source
            && !(source_store.id == store.id && *src == rel)
        {
            source_store.delete_named(&self.platform, owner, project, src, RASTER_CODE)?;
            source_deleted = true;
        }
        let mut image = store.file_ref(&rel, &filename, self.format.mime(), &rendered.bytes, ORIGIN, "generated");
        if let Some(obj) = image.as_object_mut() {
            obj.insert("width".into(), json!(rendered.width));
            obj.insert("height".into(), json!(rendered.height));
            obj.insert("format".into(), json!(self.format.extension()));
            obj.insert("layout".into(), rendered.layout.clone());
            obj.insert("source_deleted".into(), json!(source_deleted));
        }
        let out = with_answer(&input.payload, json!({ "image": image }));

        let total_ms = started.elapsed().as_millis() as u64;
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: out,
            trace: vec![
                format!("node={} node_kind={NODE_KIND}", input.node_id),
                format!(
                    "source={} out={rel} {}x{} {} bytes={} parse_ms={} raster_ms={} encode_ms={} total_ms={total_ms}",
                    stored_source.as_ref().map(|(_, rel)| rel.as_str()).unwrap_or("text"),
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

    use crate::pipeline::nodes::shared::test_platform::test_platform;

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
        assert_eq!(bad(json!({ "quality": "0" })).code, CONFIG_CODE);
        assert_eq!(bad(json!({ "format": "jpeg" })).code, CONFIG_CODE, "a choice has one word per meaning");
        let ok = Node::new(serde_json::from_value(json!({ "width": "512", "height": 512, "fit": "Contain", "format": "JPG", "quality": "90", "folder": "" })).unwrap(), platform.clone()).unwrap();
        assert_eq!((ok.target.width, ok.target.height, ok.target.fit), (Some(512), Some(512), Fit::Contain));
        assert_eq!((ok.format, ok.quality), (OutputFormat::Jpg, 90));
        assert_eq!(ok.folder(), DEFAULT_FOLDER);
        let plain = Node::new(Config::default(), platform.clone()).unwrap();
        assert_eq!((plain.target.width, plain.target.height, plain.format, plain.quality), (None, None, OutputFormat::Png, DEFAULT_QUALITY as u8));
    }

    async fn run(platform: &Arc<PlatformService>, config: Value, payload: Value) -> Result<Value, PipelineError> {
        let node = Node::new(serde_json::from_value(config).unwrap(), platform.clone())?;
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

    const SVG: &str = "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"20\" height=\"10\"><rect width=\"20\" height=\"10\" fill=\"#012169\"/></svg>";

    #[tokio::test]
    async fn the_svg_is_from_or_text_exactly_and_the_answer_is_image() {
        let platform = test_platform();
        let none = run(&platform, json!({}), json!({ "svg": SVG })).await.unwrap_err();
        assert_eq!(none.code, SOURCE_CODE, "the payload is never read without a flag");
        let both = run(&platform, json!({ "from": "a.svg", "text": SVG }), json!({})).await.unwrap_err();
        assert!(both.message.contains("--from and --text"), "{}", both.message);
        assert_eq!(run(&platform, json!({ "text": "hello" }), json!({})).await.unwrap_err().code, SOURCE_CODE);
        assert_eq!(run(&platform, json!({ "text": SVG, "delete_source": true }), json!({})).await.unwrap_err().code, CONFIG_CODE);

        let out = run(&platform, json!({ "text": SVG, "filename": "inline" }), json!({ "data": { "svg": SVG } })).await.unwrap();
        let mut keys: Vec<&str> = out.as_object().unwrap().keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(keys, vec!["data", "image"]);
        assert_eq!((out["image"]["ref"].as_str(), out["image"]["origin"].as_str()), (Some("images/inline.png"), Some(ORIGIN)));
        assert_eq!((out["image"]["width"].as_u64(), out["image"]["height"].as_u64()), (Some(20), Some(10)));

        let store = open_store(&platform, "demo", "demo", None).unwrap();
        store.fs.put("brand/logo.svg", SVG.as_bytes()).unwrap();
        let file = store.stored_ref("brand/logo.svg", "fs.file.put", "untrusted", "T").unwrap();
        let out = run(&platform, json!({ "from": file, "width": 40, "delete_source": true }), json!({})).await.unwrap();
        assert_eq!((out["image"]["width"].as_u64(), out["image"]["source_deleted"].as_bool()), (Some(40), Some(true)));
        assert!(store.fs.head("brand/logo.svg").is_err());
    }

    #[test]
    fn the_definition_carries_the_thumbnail_flag_set_and_the_four_failures() {
        let def = definition();
        let flags: Vec<&str> = def.dsl_flags.iter().map(|f| f.flag.as_str()).collect();
        assert_eq!(
            crate::pipeline::nodes::node_signature(&def),
            "fs.image.render [--from FILE] [--text TEXT] [--width N] [--height N] [--fit cover|contain|fill] [--format png|jpg|webp|pdf] \
             [--quality N] [--delete-source] [--store TEXT] [--folder TEXT] [--filename TEXT] [--path TEXT] [--on-conflict error|skip|overwrite] → image"
        );
        assert_eq!(flags.len(), 13);
        let codes: Vec<&str> = def.failure_semantics.iter().map(|f| f.code.as_str()).collect();
        assert_eq!(codes, vec![CONFIG_CODE, SOURCE_CODE, FONT_CODE, RASTER_CODE]);
    }
}
