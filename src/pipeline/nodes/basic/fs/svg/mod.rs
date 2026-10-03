//! `fs.svg.*` — content operations on SVG files: `fs.image.render`.
//!
//! The family rule: `fs.<verb>` works on store objects as bytes;
//! `fs.<format>.<verb>` understands the format (`fs.pdf.convert`,
//! `fs.image.thumbnail`, `fs.image.render`).

pub mod convert;
