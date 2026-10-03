//! `fs.barcode.render` — codes a scanner reads, written as a stored file.
//! `node.rs` is the node; each symbology's encoder is one folder beside it —
//! `qr/` (2D) and `code128/` (linear) — written here, against the standards,
//! with no barcode crate; `render.rs` draws any of them as SVG or PNG.

pub mod code128;
pub mod node;
pub mod qr;
pub mod render;

pub use node::{Config, NODE_KIND, Node, definition};
