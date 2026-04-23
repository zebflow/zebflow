//! n.file.pdf_convert — contract stub for project-scoped PDF breakdown.
//!
//! This file is intentionally **not wired into the built-in node registry yet**.
//! It exists only to freeze the node kind name and the expected input/output
//! contract before implementation details are chosen.
//!
//! # Goal
//!
//! Convert one project PDF file into page-level text artifacts under project
//! file storage.
//!
//! Version 1 is deliberately narrow:
//!
//! - page-split output
//! - text extraction only
//! - no page-image rendering yet
//! - no embedded-image extraction contract yet
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
//!   "access": "private"
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
//!     "page_count": 16,
//!     "pages": [
//!       {
//!         "page": 1,
//!         "text_path": "private/pdf/precise-paper/1/text.md",
//!         "page_meta_path": "private/pdf/precise-paper/1/page.json"
//!       },
//!       {
//!         "page": 2,
//!         "text_path": "private/pdf/precise-paper/2/text.md",
//!         "page_meta_path": "private/pdf/precise-paper/2/page.json"
//!       }
//!     ],
//!     "manifest_path": "private/pdf/precise-paper/manifest.json"
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
//!   manifest.json
//!   1/
//!     text.md
//!     page.json
//!   2/
//!     text.md
//!     page.json
//! ```
//!
//! This page-first layout is intentional even in text-only mode. It keeps the
//! node compatible with later expansion to page-image rendering, page-level
//! retries, and page-level ingestion flows.
//!
//! # Deliberately undecided
//!
//! This stub does **not** lock down the exact extraction library yet, but the
//! current v1 direction is a pure-Rust text path only.
//!
//! This stub does **not** commit to:
//!
//! - page-image rendering
//! - embedded-image extraction behavior
//! - richer JSON outputs beyond page and run manifests
//! - future output modes
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

/// Proposed static config contract for the future `n.file.pdf_convert` node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    /// Dot-path into payload containing the project-relative PDF path.
    pub source_key: String,
    /// Destination subdirectory under `files/{access}/`.
    pub output_dir: String,
    /// Access class: `public` or `private`.
    pub access: String,
}

/// One page-level artifact entry the node is expected to return.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageArtifact {
    pub page: usize,
    pub text_path: String,
    pub page_meta_path: String,
}

/// Top-level output payload contract for the future node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PdfConvertOutput {
    pub source_path: String,
    pub output_dir: String,
    pub manifest_path: String,
    pub page_count: usize,
    pub pages: Vec<PageArtifact>,
}
