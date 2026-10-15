//! `fs.pdf.convert` — one PDF broken into page-level files.
//!
//! The Zebflow wrapper around the standalone `pdfwrangler` library. `--from`
//! names the PDF — a FileRef, an upload or a store key; pdfium speaks paths,
//! so it is pulled into a scratch folder, exported there, and the export is
//! written under `--folder` with the tree writer's destination set.
//! `--include text|image|raster` (repeat; default all three) picks what each
//! page yields: its text as `text.md`, its embedded images, a PNG of the page
//! at `--dpi`. Every page also gets `page.json`, and the folder a `manifest.json`.
//!
//! The answer is one key, `pdf: { folder, items, count, page_count,
//! manifest_path, pages }`; the rest of the payload stays.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use pdfwrangler::{ExportOptions, export_document};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::pipeline::model::NodeCapability;
use crate::pipeline::nodes::shared::limits::{choice, within};
use crate::pipeline::nodes::shared::project_store::{OnConflict, on_conflict_flag, open_from, open_store, store_fields, store_flag, target_key};
use crate::pipeline::nodes::shared::store_scratch::StoreScratch;
use crate::pipeline::nodes::shared::util::{metadata_scope, with_answer};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    model::{DslFlag, DslFlagKind, LayoutItem, NodeExample, NodeFieldDef, NodeFieldType, SelectOptionDef},
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;

pub const NODE_KIND: &str = "fs.pdf.convert";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

/// pdfium, the scratch folder and the store: the world's side.
pub const CODE: &str = "FW_NODE_FS_PDF_CONVERT";
/// A flag the author set wrong.
pub const CONFIG_CODE: &str = "FW_NODE_FS_PDF_CONVERT_CONFIG";
/// `--from` is missing, not there, or not a PDF.
const SOURCE_CODE: &str = "FW_NODE_FS_PDF_CONVERT_SOURCE";

/// What a page yields, in the order the help lists them.
const INCLUDE_WORDS: &[&str] = &["text", "image", "raster"];
const DEFAULT_DPI: f32 = 144.0;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Config {
    /// The PDF: a FileRef, an upload or a store key.
    #[serde(default)]
    pub from: Value,
    /// What each page yields: `text`, `image`, `raster` (default: all three).
    #[serde(default)]
    pub include: Vec<String>,
    /// Raster resolution, 36–600 (default 144).
    #[serde(default)]
    pub dpi: Value,
    /// The store to write to; saved explicitly at registration.
    #[serde(default)]
    pub store: Option<String>,
    /// The folder the export goes into (default: `pdf/<source-file-stem>`).
    #[serde(default)]
    pub folder: String,
    /// `error` (default, when the folder is not empty), `skip` or `overwrite`.
    #[serde(default)]
    pub on_conflict: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageArtifact {
    pub page: u32,
    pub text_path: Option<String>,
    pub page_meta_path: String,
    pub page_raster_path: Option<String>,
    pub embedded_images: Vec<String>,
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
    let upload = json!({ "__zf_type": "file_ref", "backend": "zebfs", "store": "local", "ref": "uploads/brief.pdf", "filename": "brief.pdf", "mime": "application/pdf", "kind": "pdf", "size": 182331, "sha256": "sha256:…", "lifecycle": "durable", "origin": "fs.file.put", "trust": "untrusted" });
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        capabilities: vec![NodeCapability::Filesystem],
        title: "PDF Convert".to_string(),
        description: "Breaks a PDF into page-level files. The PDF is the one `--from` names — a FileRef, an upload or a store key — into page-level files under `--folder` (default `pdf/<file stem>`). \
            `--include text|image|raster` (repeat; default all three) picks what each page yields: `text.md`, its embedded images, a PNG at `--dpi` (default 144). \
            Every page also gets `page.json` and the folder a `manifest.json`. A folder that is not empty is an error unless `--on-conflict` says otherwise. \
            Adds `pdf: { folder, items, count, page_count, manifest_path, pages }` — `items` a durable FileRef per file written — and keeps the rest of the payload."
            .to_string(),
        input_schema: json!({ "type": "object" }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "pdf": {
                    "type": "object",
                    "properties": {
                        "folder": { "type": "string" },
                        "items": { "type": "array", "description": "A durable FileRef per file written" },
                        "count": { "type": "integer" },
                        "page_count": { "type": "integer" },
                        "manifest_path": { "type": "string" },
                        "pages": { "type": "array", "description": "Per page: page, text_path, page_meta_path, page_raster_path, embedded_images" }
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
            DslFlag { required: true, ..flag("--from", "from", "The PDF: a FileRef, an upload or a store key.", "file:pdf") },
            DslFlag {
                kind: DslFlagKind::RepeatedList,
                choices: words(INCLUDE_WORDS),
                max_repeat: Some(INCLUDE_WORDS.len() as u32),
                ..flag("--include", "include", "What each page yields (repeat; default: text, image and raster).", "")
            },
            flag("--dpi", "dpi", "Page raster resolution, 36–600 (default: 144).", "number"),
            DslFlag { value: "text".to_string(), ..store_flag() },
            flag("--folder", "folder", "The folder the export goes into (default: pdf/<file stem>).", "text"),
            DslFlag { choices: words(&["error", "skip", "overwrite"]), ..on_conflict_flag(OnConflict::Error) },
        ],
        fields: vec![
            field("from", "From", "The PDF, e.g. {{ input.file }}."),
            NodeFieldDef {
                field_type: NodeFieldType::MultiCheckbox,
                options: INCLUDE_WORDS.iter().map(|v| SelectOptionDef { value: v.to_string(), label: v.to_string() }).collect(),
                ..field("include", "Include", "What each page yields. None ticked: all three.")
            },
            NodeFieldDef { default_value: Some(json!("144")), ..field("dpi", "Raster DPI", "Page PNG resolution, 36–600 (default 144).") },
            field("folder", "Folder", "The folder the export goes into (default: pdf/<file stem>)."),
        ]
        .into_iter()
        .chain(store_fields(OnConflict::Error))
        .collect(),
        layout: ["from", "include", "dpi", "folder", "store", "on_conflict"].iter().map(|name| LayoutItem::Field(name.to_string())).collect(),
        examples: vec![
            NodeExample::dsl("Text and page images from an uploaded PDF", "fs.pdf.convert --from \"{{ input.file }}\" --include text --include raster --folder pdf/brief --dpi 110")
                .input(json!({ "file": upload.clone() }))
                .output(json!({ "file": upload, "pdf": { "folder": "pdf/brief", "count": 9, "page_count": 4, "manifest_path": "pdf/brief/manifest.json", "items": ["…a FileRef per file"], "pages": ["…"] } }))
                .note("Each page's text is `pdf/brief/<page>/text.md`; read one back with `fs.file.get --from pdf/brief/1/text.md`."),
        ],
        ..Default::default()
    }
}

/// `--dpi` as a number or the number as text; unset is the default.
fn dpi(value: &Value) -> Result<f32, PipelineError> {
    let bad = || PipelineError::new(CONFIG_CODE, format!("--dpi '{value}' is not a number"));
    let dpi = match value {
        Value::Null => DEFAULT_DPI,
        Value::String(s) if s.trim().is_empty() => DEFAULT_DPI,
        Value::String(s) => s.trim().parse::<f32>().map_err(|_| bad())?,
        Value::Number(n) => n.as_f64().ok_or_else(bad)? as f32,
        _ => return Err(bad()),
    };
    within(dpi, 36.0, 600.0, "--dpi", CONFIG_CODE)
}

/// `--include`, read: (text, image, raster). None given is all three.
fn include(words: &[String]) -> Result<(bool, bool, bool), PipelineError> {
    let mut chosen = Vec::new();
    for word in words.iter().filter(|word| !word.trim().is_empty()) {
        chosen.push(choice(word, INCLUDE_WORDS, "", "--include", CONFIG_CODE)?);
    }
    if chosen.is_empty() {
        return Ok((true, true, true));
    }
    Ok((chosen.contains(&"text"), chosen.contains(&"image"), chosen.contains(&"raster")))
}

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
        let (emit_fulltext, emit_page_images, emit_page_raster) = include(&self.config.include)?;
        let dpi = dpi(&self.config.dpi)?;
        let on_conflict = OnConflict::parse(self.config.on_conflict.as_deref(), OnConflict::Error, CONFIG_CODE)?;
        let (source_store, rel_path) =
            open_from(&self.platform, owner, project, &self.config.from, self.config.store.as_deref(), "--from", SOURCE_CODE)?;

        // pdfium speaks paths and the project's files live in their stores:
        // the PDF is pulled into a scratch folder, exported there, and the
        // export is put into this node's store.
        match source_store.fs.head(&rel_path) {
            Ok(stat) if stat.kind == crate::zebfs::ZebFsEntryKind::Object => {
                if stat.size > MAX_PDF_INPUT_BYTES {
                    return Err(PipelineError::new(
                        SOURCE_CODE,
                        format!(
                            "--from: PDF '{rel_path}' is {} bytes, exceeding limit of {MAX_PDF_INPUT_BYTES} bytes",
                            stat.size
                        ),
                    ));
                }
            }
            _ => return Err(PipelineError::new(SOURCE_CODE, format!("--from: no PDF at '{rel_path}'"))),
        }
        let scratch = StoreScratch::new(CODE)?;
        let abs_path = scratch.pull(&source_store.fs, &rel_path)?;
        validate_pdf_magic(&abs_path)?;
        let limits_path = abs_path.clone();
        tokio::task::spawn_blocking(move || validate_pdf_limits(&limits_path, dpi, emit_page_raster))
            .await
            .map_err(|err| PipelineError::new(CODE, format!("pdf inspect task failed: {err}")))??;

        let output_rel_dir = target_key(Some(&resolve_output_dir(&self.config.folder, &rel_path)), "", "", CONFIG_CODE)?;
        let store = open_store(&self.platform, owner, project, self.config.store.as_deref())?;
        if !on_conflict.allows_tree(&store.fs, &output_rel_dir, CODE)? {
            // Skipped: nothing was written.
            return Ok(NodeExecutionOutput {
                output_pins: vec![OUTPUT_PIN_OUT.to_string()],
                payload: with_answer(&input.payload, json!({ "pdf": { "folder": output_rel_dir, "items": [], "count": 0 } })),
                trace: vec![format!("node_kind={NODE_KIND} src={rel_path} out={output_rel_dir} skipped=true")],
            });
        }
        let output_root = scratch.path().join(".zf-out");
        let output_root_local = output_root.clone();
        let options = ExportOptions { emit_fulltext, emit_page_images, emit_page_raster, dpi };
        let source_path = abs_path.clone();
        let manifest = tokio::task::spawn_blocking(move || export_document(&source_path, &output_root, options))
            .await
            .map_err(|err| PipelineError::new(CODE, format!("pdf export task failed: {err}")))?
            .map_err(|err| PipelineError::new(CODE, err.to_string()))?;
        // What came out of the PDF was never re-encoded by this node.
        let items = scratch.push_tree_refs(&store, &output_root_local, &output_rel_dir, NODE_KIND, "untrusted")?;

        let pages = manifest
            .pages
            .into_iter()
            .map(|page| PageArtifact {
                page: page.page,
                text_path: page.text_path.map(|path| prefixed_output_path(&output_rel_dir, &path)),
                page_meta_path: prefixed_output_path(&output_rel_dir, &format!("{}/page.json", page.page)),
                page_raster_path: page.page_raster_path.map(|path| prefixed_output_path(&output_rel_dir, &path)),
                embedded_images: page.embedded_images.into_iter().map(|path| prefixed_output_path(&output_rel_dir, &path)).collect(),
            })
            .collect::<Vec<_>>();
        let count = items.len();
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: with_answer(
                &input.payload,
                json!({ "pdf": {
                    "folder": output_rel_dir,
                    "items": items,
                    "count": count,
                    "page_count": manifest.page_count,
                    "manifest_path": format!("{output_rel_dir}/manifest.json"),
                    "pages": pages,
                } }),
            ),
            trace: vec![format!("node_kind={NODE_KIND} src={rel_path} out={output_rel_dir} pages={}", manifest.page_count)],
        })
    }
}

fn sanitize_rel_path(path: &str) -> String {
    path.split('/')
        .filter(|segment| !segment.is_empty() && *segment != "." && *segment != "..")
        .collect::<Vec<_>>()
        .join("/")
}

fn sanitize_filename_stem(name: &str) -> String {
    let stem = Path::new(name)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or(name);
    let sanitized: String = stem
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() || ch == '-' || ch == '_' {
                ch.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect();
    sanitized
        .split('-')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

fn resolve_output_dir(configured: &str, source_rel_path: &str) -> String {
    // Normalised (and `..` refused) by `target_key`, never quietly cleaned.
    let configured = configured.trim().trim_matches('/');
    if !configured.is_empty() {
        return configured.to_string();
    }
    let source_leaf = Path::new(source_rel_path)
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or("document");
    let stem = sanitize_filename_stem(source_leaf);
    if stem.is_empty() {
        "pdf/document".to_string()
    } else {
        format!("pdf/{stem}")
    }
}

fn prefixed_output_path(output_rel_dir: &str, leaf: &str) -> String {
    let leaf = sanitize_rel_path(leaf);
    if leaf.is_empty() {
        output_rel_dir.to_string()
    } else {
        format!("{output_rel_dir}/{leaf}")
    }
}

const MAX_PDF_INPUT_BYTES: u64 = 64 * 1024 * 1024;
const MAX_PDF_PAGES: u64 = 500;
const MAX_PDF_RASTER_DIM: f32 = 8_192.0;
const MAX_PDF_RASTER_PIXELS: f32 = 33_554_432.0;

fn validate_pdf_magic(path: &PathBuf) -> Result<(), PipelineError> {
    use std::io::Read;
    // Five bytes say whether it is a PDF; the file is not read whole for that.
    let mut head = [0u8; 5];
    std::fs::File::open(path)
        .and_then(|mut file| file.read_exact(&mut head))
        .map_err(|err| PipelineError::new(SOURCE_CODE, format!("--from: {err}")))?;
    if &head != b"%PDF-" {
        return Err(PipelineError::new(SOURCE_CODE, "--from is not a PDF"));
    }
    Ok(())
}

/// pdfwrangler draws a page that names no MediaBox on a 1000 x 1000 pt canvas.
const RENDERER_DEFAULT_PAGE_PT: f32 = 1_000.0;

/// The page count and every page's raster size, read by the same PDF library
/// that will render it: the page tree is walked after object streams are
/// decompressed and inherited MediaBoxes resolved, so a compressed page tree
/// cannot hide a page, and an outline's `/Count` is not mistaken for one.
fn validate_pdf_limits(
    path: &Path,
    dpi: f32,
    emit_page_raster: bool,
) -> Result<(), PipelineError> {
    let meta = std::fs::metadata(path)
        .map_err(|err| PipelineError::new(SOURCE_CODE, format!("--from: {err}")))?;
    if meta.len() > MAX_PDF_INPUT_BYTES {
        return Err(PipelineError::new(
            SOURCE_CODE,
            format!(
                "--from: PDF is {} bytes, exceeding limit of {MAX_PDF_INPUT_BYTES} bytes",
                meta.len()
            ),
        ));
    }
    let report = pdfwrangler::inspect_pdf(path)
        .map_err(|err| PipelineError::new(SOURCE_CODE, format!("--from: not a readable PDF: {err:#}")))?;
    if report.page_count as u64 > MAX_PDF_PAGES {
        return Err(PipelineError::new(
            SOURCE_CODE,
            format!(
                "--from: PDF has {} pages, exceeding limit of {MAX_PDF_PAGES}",
                report.page_count
            ),
        ));
    }
    if !emit_page_raster {
        return Ok(());
    }
    for page in &report.pages {
        let (width_pt, height_pt) = match page.media_box {
            Some(b) => ((b.x1 - b.x0).abs(), (b.y1 - b.y0).abs()),
            None => (RENDERER_DEFAULT_PAGE_PT, RENDERER_DEFAULT_PAGE_PT),
        };
        let width_px = (width_pt / 72.0) * dpi;
        let height_px = (height_pt / 72.0) * dpi;
        let pixels = width_px * height_px;
        if !width_px.is_finite()
            || !height_px.is_finite()
            || !pixels.is_finite()
            || width_px > MAX_PDF_RASTER_DIM
            || height_px > MAX_PDF_RASTER_DIM
            || pixels > MAX_PDF_RASTER_PIXELS
        {
            return Err(PipelineError::new(
                SOURCE_CODE,
                format!(
                    "--from: PDF page {} raster size ({:.0}x{:.0} px at {dpi} DPI) exceeds maximum allowed dimensions",
                    page.page_number, width_px, height_px
                ),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::json;

    use super::{CONFIG_CODE, Config, Node, SOURCE_CODE, dpi, include, prefixed_output_path, resolve_output_dir, validate_pdf_limits};
    use crate::pipeline::nodes::{NodeExecutionInput, NodeHandler};
    use crate::platform::model::PlatformConfig;
    use crate::platform::services::PlatformService;

    #[test]
    fn rejects_oversized_pdf_page_count_and_raster_dimensions() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let write = |name: &str, bytes: Vec<u8>| {
            let path = tmp.path().join(name);
            std::fs::write(&path, bytes).expect("write pdf");
            path
        };

        let huge_box_pdf = write(
            "huge-box.pdf",
            classic_pdf(&[
                "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 14400 14400] >>".to_string(),
            ]),
        );
        let err = validate_pdf_limits(&huge_box_pdf, 600.0, true).expect_err("huge mediabox at 600 dpi must fail");
        assert_eq!(err.code, SOURCE_CODE);
        assert!(err.message.contains("exceeds maximum allowed dimensions"), "{err:?}");
        validate_pdf_limits(&huge_box_pdf, 600.0, false).expect("no raster, no raster limit");

        // 501 real pages in the page tree.
        let pages = 501;
        let mut objects = vec![
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            format!(
                "<< /Type /Pages /Kids [{}] /Count {pages} >>",
                (0..pages).map(|i| format!("{} 0 R", i + 3)).collect::<Vec<_>>().join(" ")
            ),
        ];
        objects.extend((0..pages).map(|_| "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>".to_string()));
        let many_pages_pdf = write("many-pages.pdf", classic_pdf(&objects));
        let err = validate_pdf_limits(&many_pages_pdf, 144.0, false).expect_err("excessive page count must fail");
        assert_eq!(err.code, SOURCE_CODE);
        assert!(err.message.contains("501 pages, exceeding limit"), "{err:?}");

        let garbage = write("garbage.pdf", b"%PDF-1.4\nnot a pdf at all\n".to_vec());
        let err = validate_pdf_limits(&garbage, 144.0, true).expect_err("unreadable PDF must fail");
        assert_eq!(err.code, SOURCE_CODE);

        validate_pdf_limits(&write("ok.pdf", minimal_pdf_bytes("ok")), 144.0, true).expect("an ordinary page passes");
    }

    /// An outline's `/Count` is the number of open bookmarks, not pages: a
    /// one-page PDF with a large outline is not refused.
    #[test]
    fn an_outline_count_is_not_a_page_count() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("outline.pdf");
        std::fs::write(
            &path,
            classic_pdf(&[
                "<< /Type /Catalog /Pages 2 0 R /Outlines 4 0 R >>".to_string(),
                "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>".to_string(),
                "<< /Type /Outlines /Count 5000 >>".to_string(),
            ]),
        )
        .expect("write pdf");
        validate_pdf_limits(&path, 144.0, true).expect("one page, whatever the outline says");
    }

    /// A page tree inside a Flate-compressed object stream, reached through a
    /// cross-reference stream: no `/MediaBox` appears in the file's bytes,
    /// yet the page is still measured.
    #[test]
    fn a_page_box_inside_a_compressed_object_stream_is_measured() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("objstm.pdf");
        let bytes = object_stream_pdf("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 14400 14400] >>");
        assert!(!bytes.windows(9).any(|w| w == b"/MediaBox"), "the box is compressed");
        std::fs::write(&path, &bytes).expect("write pdf");
        let err = validate_pdf_limits(&path, 600.0, true).expect_err("hidden huge mediabox must fail");
        assert!(err.message.contains("exceeds maximum allowed dimensions"), "{err:?}");

        std::fs::write(&path, object_stream_pdf("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] >>"))
            .expect("write pdf");
        validate_pdf_limits(&path, 144.0, true).expect("an ordinary compressed page passes");
    }

    #[test]
    fn derives_default_output_dir_from_source_stem() {
        assert_eq!(
            resolve_output_dir("", "uploads/My Paper.pdf"),
            "pdf/my-paper"
        );
    }

    #[test]
    fn include_and_dpi_are_closed() {
        assert_eq!(include(&[]).unwrap(), (true, true, true));
        assert_eq!(include(&["raster".to_string()]).unwrap(), (false, false, true));
        assert_eq!(include(&["pages".to_string()]).unwrap_err().code, CONFIG_CODE);
        assert_eq!(dpi(&json!(null)).unwrap(), 144.0);
        assert_eq!(dpi(&json!("110")).unwrap(), 110.0);
        assert_eq!(dpi(&json!(1200)).unwrap_err().code, CONFIG_CODE);
        assert_eq!(dpi(&json!("high")).unwrap_err().code, CONFIG_CODE);
        assert_eq!(
            crate::pipeline::nodes::node_signature(&super::definition()),
            "fs.pdf.convert --from PDF [--include text|image|raster…] [--dpi N] [--store TEXT] [--folder TEXT] \
             [--on-conflict error|skip|overwrite] → pdf"
        );
    }

    #[test]
    fn prefixes_manifest_children_under_output_dir() {
        assert_eq!(
            prefixed_output_path("pdf/my-paper", "1/text.md"),
            "pdf/my-paper/1/text.md"
        );
    }

    #[tokio::test]
    async fn executes_pdf_convert_and_writes_manifest_text_and_raster() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let mut config = PlatformConfig::default();
        config.data_root = tmp.path().join("platform");
        let platform = Arc::new(PlatformService::from_config(config).expect("platform"));

        let layout = platform
            .file
            .ensure_project_layout("superadmin", "default")
            .expect("layout");
        let source_rel = "uploads/test.pdf";
        let source_abs = layout.files_dir.join(source_rel);
        std::fs::create_dir_all(source_abs.parent().expect("parent")).expect("create parent");
        std::fs::write(&source_abs, minimal_pdf_bytes("Hello PDF")).expect("write pdf");

        let missing = Node::new(Config::default(), platform.clone())
            .expect("node")
            .execute_async(NodeExecutionInput {
                node_id: "n-test".to_string(),
                input_pin: "in".to_string(),
                payload: json!({ "saved": source_rel }),
                metadata: json!({ "owner": "superadmin", "project": "default", "pipeline": "test", "request_id": "req-0" }),
                bus: None,
            })
            .await
            .expect_err("no --from");
        assert_eq!(missing.code, SOURCE_CODE, "the payload is never read without a flag");

        let config = json!({ "from": source_rel, "include": ["text", "raster"] });
        let node = Node::new(serde_json::from_value(config).expect("config"), platform).expect("node");

        let output = node
            .execute_async(NodeExecutionInput {
                node_id: "n-test".to_string(),
                input_pin: "in".to_string(),
                payload: json!({ "keep": true }),
                metadata: json!({
                    "owner": "superadmin",
                    "project": "default",
                    "pipeline": "test",
                    "request_id": "req-1"
                }),
                bus: None,
            })
            .await
            .expect("pdf convert output");

        let pdf = output
            .payload
            .get("pdf")
            .cloned()
            .expect("pdf payload");
        assert_eq!(output.payload["keep"], true, "the payload stays");
        assert_eq!(pdf["count"].as_u64(), pdf["items"].as_array().map(|items| items.len() as u64));
        let manifest_rel = pdf
            .get("manifest_path")
            .and_then(|value| value.as_str())
            .expect("manifest path");
        let output_dir = pdf
            .get("folder")
            .and_then(|value| value.as_str())
            .expect("output dir");
        let page_count = pdf
            .get("page_count")
            .and_then(|value| value.as_u64())
            .expect("page count");

        assert_eq!(page_count, 1);

        let manifest_abs = layout.files_dir.join(manifest_rel);
        let text_abs = layout.files_dir.join(format!("{output_dir}/1/text.md"));
        let meta_abs = layout.files_dir.join(format!("{output_dir}/1/page.json"));
        let raster_abs = layout.files_dir.join(format!("{output_dir}/1/page-1.png"));

        assert!(
            manifest_abs.is_file(),
            "manifest missing: {}",
            manifest_abs.display()
        );
        assert!(text_abs.is_file(), "text missing: {}", text_abs.display());
        assert!(
            meta_abs.is_file(),
            "page json missing: {}",
            meta_abs.display()
        );
        assert!(
            raster_abs.is_file(),
            "raster missing: {}",
            raster_abs.display()
        );

        let text = std::fs::read_to_string(&text_abs).expect("read text");
        assert!(
            text.to_ascii_lowercase().contains("hello"),
            "expected extracted text to contain hello, got: {text}"
        );
    }

    /// A PDF 1.4 file with a classic cross-reference table; `objects[i]` is
    /// object `i + 1`.
    fn classic_pdf(objects: &[String]) -> Vec<u8> {
        let mut pdf = b"%PDF-1.4\n".to_vec();
        let mut offsets = Vec::with_capacity(objects.len());
        for (index, body) in objects.iter().enumerate() {
            offsets.push(pdf.len());
            pdf.extend_from_slice(format!("{} 0 obj\n{body}\nendobj\n", index + 1).as_bytes());
        }
        let xref_offset = pdf.len();
        pdf.extend_from_slice(format!("xref\n0 {}\n", offsets.len() + 1).as_bytes());
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        for offset in &offsets {
            pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        pdf.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
                offsets.len() + 1
            )
            .as_bytes(),
        );
        pdf
    }

    /// A PDF 1.5 file whose page tree (objects 2 and 3) sits in a
    /// Flate-compressed object stream (object 4), indexed by a
    /// cross-reference stream (object 5).
    fn object_stream_pdf(page: &str) -> Vec<u8> {
        use std::io::Write;
        let pages = "<< /Type /Pages /Kids [3 0 R] /Count 1 >>";
        let header = format!("2 0 3 {} ", pages.len() + 1);
        let body = format!("{header}{pages}\n{page}");
        let mut encoder = flate2::write::ZlibEncoder::new(Vec::new(), flate2::Compression::default());
        encoder.write_all(body.as_bytes()).expect("deflate");
        let packed = encoder.finish().expect("deflate");

        let mut pdf = b"%PDF-1.5\n".to_vec();
        let catalog_at = pdf.len();
        pdf.extend_from_slice(b"1 0 obj\n<< /Type /Catalog /Pages 2 0 R >>\nendobj\n");
        let objstm_at = pdf.len();
        pdf.extend_from_slice(
            format!(
                "4 0 obj\n<< /Type /ObjStm /N 2 /First {} /Filter /FlateDecode /Length {} >>\nstream\n",
                header.len(),
                packed.len()
            )
            .as_bytes(),
        );
        pdf.extend_from_slice(&packed);
        pdf.extend_from_slice(b"\nendstream\nendobj\n");
        let xref_at = pdf.len();
        let entry = |kind: u8, field: u32, index: u16| {
            let mut row = vec![kind];
            row.extend_from_slice(&field.to_be_bytes());
            row.extend_from_slice(&index.to_be_bytes());
            row
        };
        let rows = [
            entry(0, 0, 0xffff),
            entry(1, catalog_at as u32, 0),
            entry(2, 4, 0),
            entry(2, 4, 1),
            entry(1, objstm_at as u32, 0),
            entry(1, xref_at as u32, 0),
        ]
        .concat();
        pdf.extend_from_slice(
            format!(
                "5 0 obj\n<< /Type /XRef /Size 6 /W [1 4 2] /Root 1 0 R /Length {} >>\nstream\n",
                rows.len()
            )
            .as_bytes(),
        );
        pdf.extend_from_slice(&rows);
        pdf.extend_from_slice(format!("\nendstream\nendobj\nstartxref\n{xref_at}\n%%EOF\n").as_bytes());
        pdf
    }

    fn minimal_pdf_bytes(text: &str) -> Vec<u8> {
        let content = format!("BT /F1 24 Tf 72 72 Td ({}) Tj ET", escape_pdf_text(text));
        classic_pdf(&[
            "<< /Type /Catalog /Pages 2 0 R >>".to_string(),
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_string(),
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 144] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>".to_string(),
            format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()),
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
        ])
    }

    fn escape_pdf_text(text: &str) -> String {
        text.replace('\\', "\\\\")
            .replace('(', "\\(")
            .replace(')', "\\)")
    }
}
