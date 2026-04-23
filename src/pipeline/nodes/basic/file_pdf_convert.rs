//! n.file.pdf_convert — contract stub for project-scoped PDF breakdown.
//!
//! This file is intentionally **not wired into the built-in node registry yet**.
//! It exists only to freeze the node kind name and the expected input/output
//! contract before implementation details are chosen.
//!
//! # Goal
//!
//! Convert one project PDF file into page-level artifacts under project file
//! storage. The node is intended to support text extraction, page-image
//! generation, and later richer outputs such as JSON manifests.
//!
//! # Proposed node kind
//!
//! `n.file.pdf_convert`
//!
//! # Source resolution
//!
//! Follows the same pattern as `n.img.thumbnail`:
//!
//! - resolve the source PDF path from the input payload using `source_key`
//! - default `source_key = "saved.path"`
//! - the resolved value is a path relative to the project's `files_dir`
//!
//! This makes the node chain naturally after `n.file.save`.
//!
//! # Initial config contract
//!
//! ```json
//! {
//!   "source_key": "saved.path",
//!   "output_dir": "pdf/precise-paper",
//!   "access": "private",
//!   "mode": "text_and_images",
//!   "dpi": 180
//! }
//! ```
//!
//! ## Config field meaning
//!
//! - `source_key`
//!   - dot-path into input payload where the source PDF path is stored
//!   - default: `saved.path`
//! - `output_dir`
//!   - destination subdirectory under `files/{access}/`
//! - `access`
//!   - `public` or `private`
//! - `mode`
//!   - `text_only`
//!   - `images_only`
//!   - `text_and_images`
//! - `dpi`
//!   - target render density for page-image outputs
//!   - ignored when image generation is disabled
//!
//! # Input payload contract
//!
//! The node expects to find a project-relative PDF path at the payload key
//! specified by `source_key`.
//!
//! Minimal example:
//!
//! ```json
//! {
//!   "saved": {
//!     "path": "private/uploads/paper.pdf"
//!   }
//! }
//! ```
//!
//! # Output payload contract
//!
//! The node should emit one structured summary object describing generated
//! artifacts. Downstream ingestion/indexing nodes should consume this payload
//! rather than re-scanning the filesystem.
//!
//! Example:
//!
//! ```json
//! {
//!   "pdf_convert": {
//!     "source_path": "private/uploads/paper.pdf",
//!     "output_dir": "private/pdf/precise-paper",
//!     "mode": "text_and_images",
//!     "page_count": 16,
//!     "pages": [
//!       {
//!         "page": 1,
//!         "text_path": "private/pdf/precise-paper/1/text.md",
//!         "image_path": "private/pdf/precise-paper/1/page.png"
//!       },
//!       {
//!         "page": 2,
//!         "text_path": "private/pdf/precise-paper/2/text.md",
//!         "image_path": "private/pdf/precise-paper/2/page.png"
//!       }
//!     ]
//!   }
//! }
//! ```
//!
//! # Filesystem shape
//!
//! Proposed output layout:
//!
//! ```text
//! files/private/pdf/precise-paper/
//!   1/
//!     text.md
//!     page.png
//!     page.json
//!   2/
//!     text.md
//!     page.png
//!     page.json
//! ```
//!
//! Depending on mode:
//!
//! - `text_only` emits `text.md` and `page.json`
//! - `images_only` emits `page.png` and `page.json`
//! - `text_and_images` emits both
//!
//! # Deliberately undecided
//!
//! This stub does **not** choose the implementation technology yet.
//! In particular, it does not commit to:
//!
//! - text extraction library
//! - page-image renderer
//! - embedded-image extraction behavior
//! - JSON manifest richness
//! - whether the node will later support more output modes
//!
//! # Next implementation step
//!
//! Once implementation begins, this module should grow into the standard ZebFlow
//! node shape:
//!
//! - `definition() -> NodeDefinition`
//! - `Config`
//! - `Node`
//! - `impl NodeHandler`
//! - registration in `basic/mod.rs`
//! - dispatch in `BasicPipelineEngine::build_node()`

/// Stable proposed node kind for project-scoped PDF conversion.
pub const NODE_KIND: &str = "n.file.pdf_convert";

/// Proposed output mode for the future `n.file.pdf_convert` node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PdfConvertMode {
    TextOnly,
    ImagesOnly,
    TextAndImages,
}

/// Proposed static config contract for the future `n.file.pdf_convert` node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Dot-path into payload containing the project-relative PDF path.
    pub source_key: String,
    /// Destination subdirectory under `files/{access}/`.
    pub output_dir: String,
    /// Access class: `public` or `private`.
    pub access: String,
    /// Which artifact families to emit.
    pub mode: PdfConvertMode,
    /// Target DPI for page-image rendering.
    pub dpi: u32,
}

/// One page-level artifact entry the node is expected to return.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageArtifact {
    pub page: usize,
    pub text_path: Option<String>,
    pub image_path: Option<String>,
}

/// Top-level output payload contract for the future node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfConvertOutput {
    pub source_path: String,
    pub output_dir: String,
    pub mode: PdfConvertMode,
    pub page_count: usize,
    pub pages: Vec<PageArtifact>,
}
