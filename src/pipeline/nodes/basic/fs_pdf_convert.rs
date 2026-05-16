//! n.fs.pdf.convert — break a project PDF into page-level artifacts.
//!
//! This node is the Zebflow wrapper around the standalone `pdfwrangler` library.
//! It resolves a project-scoped source PDF path from the payload, writes exported
//! page assets under Zebflow FS, and emits a structured summary
//! for downstream indexing flows.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use pdfwrangler::{ExportOptions, export_document};
use serde::{Deserialize, Serialize};
use serde_json::json;

use super::util::{metadata_scope, resolve_path};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    model::{DslFlag, DslFlagKind, LayoutItem, NodeFieldDef, NodeFieldType},
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};
use crate::platform::services::PlatformService;

pub const NODE_KIND: &str = "n.fs.pdf.convert";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

fn default_source_key() -> String {
    "saved.path".to_string()
}

fn default_emit_fulltext() -> bool {
    true
}

fn default_emit_page_images() -> bool {
    true
}

fn default_emit_page_raster() -> bool {
    true
}

fn default_dpi() -> f32 {
    144.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Dot-path in the payload containing the project-relative PDF path.
    #[serde(default = "default_source_key")]
    pub source_key: String,
    /// Destination ZebFS object directory.
    #[serde(default)]
    pub output_dir: String,
    /// Export text markdown per page.
    #[serde(default = "default_emit_fulltext")]
    pub emit_fulltext: bool,
    /// Export embedded images per page.
    #[serde(default = "default_emit_page_images")]
    pub emit_page_images: bool,
    /// Export a raster PNG per page.
    #[serde(default = "default_emit_page_raster")]
    pub emit_page_raster: bool,
    /// Raster DPI when page PNG export is enabled.
    #[serde(default = "default_dpi")]
    pub dpi: f32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            source_key: default_source_key(),
            output_dir: String::new(),
            emit_fulltext: default_emit_fulltext(),
            emit_page_images: default_emit_page_images(),
            emit_page_raster: default_emit_page_raster(),
            dpi: default_dpi(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PageArtifact {
    pub page: u32,
    pub text_path: Option<String>,
    pub page_meta_path: String,
    pub page_raster_path: Option<String>,
    pub embedded_images: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfConvertOutput {
    pub source_path: String,
    pub output_dir: String,
    pub manifest_path: String,
    pub page_count: usize,
    pub options: PdfConvertOptions,
    pub pages: Vec<PageArtifact>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PdfConvertOptions {
    pub emit_fulltext: bool,
    pub emit_page_images: bool,
    pub emit_page_raster: bool,
    pub dpi: f32,
}

pub fn definition() -> NodeDefinition {
    NodeDefinition {
        kind: NODE_KIND.to_string(),
        title: "PDF Convert".to_string(),
        description: "Convert one project PDF file into page-level artifacts under Zebflow FS. \
            Reads the source PDF path from `input.saved.path` by default and writes \
            `text.md`, `page.json`, optional page rasters, and optional extracted embedded \
            images. Output payload includes a page manifest for downstream indexing."
            .to_string(),
        input_schema: json!({
            "type": "object",
            "description": "Payload must contain a project-relative PDF path at the configured source_key (default: saved.path)."
        }),
        output_schema: json!({
            "type": "object",
            "properties": {
                "pdf_convert": {
                    "type": "object",
                    "properties": {
                        "source_path": { "type": "string" },
                        "output_dir": { "type": "string" },
                        "manifest_path": { "type": "string" },
                        "page_count": { "type": "integer" }
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
                description: "Dot-path to the source PDF path in payload (default: saved.path)".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--output-dir".to_string(),
                config_key: "output_dir".to_string(),
                description: "Destination ZebFS object directory. Defaults to pdf/<source-file-stem>".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
            DslFlag {
                flag: "--emit-fulltext".to_string(),
                config_key: "emit_fulltext".to_string(),
                description: "Write text.md per page (default: true)".to_string(),
                kind: DslFlagKind::Bool,
                required: false,
            },
            DslFlag {
                flag: "--emit-page-images".to_string(),
                config_key: "emit_page_images".to_string(),
                description: "Extract embedded images per page (default: true)".to_string(),
                kind: DslFlagKind::Bool,
                required: false,
            },
            DslFlag {
                flag: "--emit-page-raster".to_string(),
                config_key: "emit_page_raster".to_string(),
                description: "Render PNG raster per page (default: true)".to_string(),
                kind: DslFlagKind::Bool,
                required: false,
            },
            DslFlag {
                flag: "--dpi".to_string(),
                config_key: "dpi".to_string(),
                description: "Raster DPI when page PNG export is enabled (default: 144)".to_string(),
                kind: DslFlagKind::Scalar,
                required: false,
            },
        ],
        fields: vec![
            NodeFieldDef {
                name: "source_key".to_string(),
                label: "Source Key".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Dot-path to the project-relative PDF path in the input payload.".to_string()),
                default_value: Some(json!("saved.path")),
                ..Default::default()
            },
            NodeFieldDef {
                name: "output_dir".to_string(),
                label: "Output Dir".to_string(),
                field_type: NodeFieldType::Text,
                help: Some("Destination ZebFS object directory. Leave blank to use pdf/<source-file-stem>.".to_string()),
                ..Default::default()
            },
            NodeFieldDef {
                name: "emit_fulltext".to_string(),
                label: "Emit Text".to_string(),
                field_type: NodeFieldType::Checkbox,
                default_value: Some(json!(true)),
                ..Default::default()
            },
            NodeFieldDef {
                name: "emit_page_images".to_string(),
                label: "Extract Embedded Images".to_string(),
                field_type: NodeFieldType::Checkbox,
                default_value: Some(json!(true)),
                ..Default::default()
            },
            NodeFieldDef {
                name: "emit_page_raster".to_string(),
                label: "Render Page PNG".to_string(),
                field_type: NodeFieldType::Checkbox,
                default_value: Some(json!(true)),
                ..Default::default()
            },
            NodeFieldDef {
                name: "dpi".to_string(),
                label: "Raster DPI".to_string(),
                field_type: NodeFieldType::Text,
                default_value: Some(json!("144")),
                ..Default::default()
            },
        ],
        layout: vec![LayoutItem::Col {
            col: vec![
                LayoutItem::Field("source_key".to_string()),
                LayoutItem::Field("output_dir".to_string()),
                LayoutItem::Field("emit_fulltext".to_string()),
                LayoutItem::Field("emit_page_images".to_string()),
                LayoutItem::Field("emit_page_raster".to_string()),
                LayoutItem::Field("dpi".to_string()),
            ],
        }],
        ai_tool: Default::default(),
    }
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

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        let (owner, project, ..) = metadata_scope(&input.metadata)?;

        let source_key = if self.config.source_key.trim().is_empty() {
            "saved.path"
        } else {
            self.config.source_key.trim()
        };

        let rel_path = resolve_path(&input.payload, source_key)
            .and_then(|value| value.as_str())
            .ok_or_else(|| {
                PipelineError::new(
                    "FW_NODE_PDF_CONVERT",
                    format!(
                        "source PDF path not found at payload key '{source_key}' — chain after n.fs.save or set --source-key"
                    ),
                )
            })?;

        let rel_path = sanitize_rel_path(rel_path);
        if rel_path.is_empty() {
            return Err(PipelineError::new(
                "FW_NODE_PDF_CONVERT",
                "resolved source path is empty after sanitization",
            ));
        }

        let layout = self
            .platform
            .file
            .ensure_project_layout(owner, project)
            .map_err(|err| PipelineError::new("FW_NODE_PDF_CONVERT", err.to_string()))?;

        let abs_path = layout.files_dir.join(&rel_path);
        if !abs_path.is_file() {
            return Err(PipelineError::new(
                "FW_NODE_PDF_CONVERT",
                format!("source PDF not found: {rel_path}"),
            ));
        }

        validate_pdf_magic(&abs_path)?;

        let output_rel_dir = resolve_output_dir(&self.config.output_dir, &rel_path);
        let output_root = layout.files_dir.join(&output_rel_dir);

        let options = ExportOptions {
            emit_fulltext: self.config.emit_fulltext,
            emit_page_images: self.config.emit_page_images,
            emit_page_raster: self.config.emit_page_raster,
            dpi: self.config.dpi,
        };

        let source_path = abs_path.clone();
        let manifest = tokio::task::spawn_blocking(move || {
            export_document(&source_path, &output_root, options)
        })
        .await
        .map_err(|err| {
            PipelineError::new(
                "FW_NODE_PDF_CONVERT",
                format!("pdf export task failed: {err}"),
            )
        })?
        .map_err(|err| PipelineError::new("FW_NODE_PDF_CONVERT", err.to_string()))?;

        let pages = manifest
            .pages
            .into_iter()
            .map(|page| PageArtifact {
                page: page.page,
                text_path: page
                    .text_path
                    .map(|path| prefixed_output_path(&output_rel_dir, &path)),
                page_meta_path: prefixed_output_path(
                    &output_rel_dir,
                    &format!("{}/page.json", page.page),
                ),
                page_raster_path: page
                    .page_raster_path
                    .map(|path| prefixed_output_path(&output_rel_dir, &path)),
                embedded_images: page
                    .embedded_images
                    .into_iter()
                    .map(|path| prefixed_output_path(&output_rel_dir, &path))
                    .collect(),
            })
            .collect::<Vec<_>>();

        let output = PdfConvertOutput {
            source_path: rel_path.clone(),
            output_dir: output_rel_dir.clone(),
            manifest_path: format!("{output_rel_dir}/manifest.json"),
            page_count: manifest.page_count,
            options: PdfConvertOptions {
                emit_fulltext: self.config.emit_fulltext,
                emit_page_images: self.config.emit_page_images,
                emit_page_raster: self.config.emit_page_raster,
                dpi: self.config.dpi,
            },
            pages,
        };

        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload: json!({ "pdf_convert": output }),
            trace: vec![format!(
                "node_kind={NODE_KIND} src={} out={} pages={}",
                rel_path, output_rel_dir, manifest.page_count
            )],
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
    let configured = sanitize_rel_path(configured);
    if !configured.is_empty() {
        return configured;
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
    let bytes = std::fs::read(path).map_err(|err| {
        PipelineError::new("FW_NODE_PDF_CONVERT", format!("read source PDF: {err}"))
    })?;
    if !bytes.starts_with(b"%PDF-") {
        return Err(PipelineError::new(
            "FW_NODE_PDF_CONVERT",
            format!("source file is not a PDF: {}", path.display()),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use serde_json::json;

    use super::{Config, Node, prefixed_output_path, resolve_output_dir, sanitize_rel_path};
    use crate::pipeline::nodes::{NodeExecutionInput, NodeHandler};
    use crate::platform::model::PlatformConfig;
    use crate::platform::services::PlatformService;

    #[test]
    fn sanitizes_relative_paths() {
        assert_eq!(
            sanitize_rel_path("../uploads/./paper.pdf"),
            "uploads/paper.pdf"
        );
    }

    #[test]
    fn derives_default_output_dir_from_source_stem() {
        assert_eq!(
            resolve_output_dir("", "uploads/My Paper.pdf"),
            "pdf/my-paper"
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
        config.default_password = "secret".to_string();
        let platform = Arc::new(PlatformService::from_config(config).expect("platform"));

        let layout = platform
            .file
            .ensure_project_layout("superadmin", "default")
            .expect("layout");
        let source_rel = "uploads/test.pdf";
        let source_abs = layout.files_dir.join(source_rel);
        std::fs::create_dir_all(source_abs.parent().expect("parent")).expect("create parent");
        std::fs::write(&source_abs, minimal_pdf_bytes("Hello PDF")).expect("write pdf");

        let node = Node::new(
            Config {
                emit_fulltext: true,
                emit_page_images: false,
                emit_page_raster: true,
                ..Config::default()
            },
            platform,
        )
        .expect("node");

        let output = node
            .execute_async(NodeExecutionInput {
                node_id: "n-test".to_string(),
                input_pin: "in".to_string(),
                payload: json!({
                    "saved": {
                        "path": source_rel
                    }
                }),
                metadata: json!({
                    "owner": "superadmin",
                    "project": "default",
                    "pipeline": "test",
                    "request_id": "req-1"
                }),
                step_tx: None,
            })
            .await
            .expect("pdf convert output");

        let pdf = output
            .payload
            .get("pdf_convert")
            .cloned()
            .expect("pdf_convert payload");
        let manifest_rel = pdf
            .get("manifest_path")
            .and_then(|value| value.as_str())
            .expect("manifest path");
        let output_dir = pdf
            .get("output_dir")
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
