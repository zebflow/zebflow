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
        description: "Break the PDF `--from` names — a FileRef, an upload or a store key — into page-level files under `--folder` (default `pdf/<file stem>`). \
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
            Ok(stat) if stat.kind == crate::zebfs::ZebFsEntryKind::Object => {}
            _ => return Err(PipelineError::new(SOURCE_CODE, format!("--from: no PDF at '{rel_path}'"))),
        }
        let scratch = StoreScratch::new(CODE)?;
        let abs_path = scratch.pull(&source_store.fs, &rel_path)?;
        validate_pdf_magic(&abs_path)?;

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

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::json;

    use super::{CONFIG_CODE, Config, Node, SOURCE_CODE, dpi, include, prefixed_output_path, resolve_output_dir};
    use crate::pipeline::nodes::{NodeExecutionInput, NodeHandler};
    use crate::platform::model::PlatformConfig;
    use crate::platform::services::PlatformService;

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

    fn minimal_pdf_bytes(text: &str) -> Vec<u8> {
        fn push_obj(buf: &mut Vec<u8>, offsets: &mut Vec<usize>, id: usize, body: &str) {
            offsets.push(buf.len());
            buf.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
        }

        let content = format!("BT /F1 24 Tf 72 72 Td ({}) Tj ET", escape_pdf_text(text));
        let mut pdf = Vec::new();
        pdf.extend_from_slice(b"%PDF-1.4\n");
        let mut offsets = vec![0usize];
        push_obj(
            &mut pdf,
            &mut offsets,
            1,
            "<< /Type /Catalog /Pages 2 0 R >>",
        );
        push_obj(
            &mut pdf,
            &mut offsets,
            2,
            "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        );
        push_obj(
            &mut pdf,
            &mut offsets,
            3,
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 300 144] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> >> >>",
        );
        push_obj(
            &mut pdf,
            &mut offsets,
            4,
            &format!(
                "<< /Length {} >>\nstream\n{}\nendstream",
                content.len(),
                content
            ),
        );
        push_obj(
            &mut pdf,
            &mut offsets,
            5,
            "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>",
        );
        let xref_offset = pdf.len();
        pdf.extend_from_slice(format!("xref\n0 {}\n", offsets.len()).as_bytes());
        pdf.extend_from_slice(b"0000000000 65535 f \n");
        for offset in offsets.iter().skip(1) {
            pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
        }
        pdf.extend_from_slice(
            format!(
                "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
                offsets.len(),
                xref_offset
            )
            .as_bytes(),
        );
        pdf
    }

    fn escape_pdf_text(text: &str) -> String {
        text.replace('\\', "\\\\")
            .replace('(', "\\(")
            .replace(')', "\\)")
    }
}
