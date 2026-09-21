//! `fs.svg.*` — content operations on SVG files: `fs.svg.convert`.
//!
//! The family rule: `fs.<verb>` works on store objects as bytes;
//! `fs.<format>.<verb>` understands the format (`fs.pdf.convert`,
//! `fs.image.thumbnail`, `fs.svg.convert`).

pub mod convert;
