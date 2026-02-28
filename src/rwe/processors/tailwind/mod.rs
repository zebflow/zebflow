//! Tailwind-like style processing for Zebflow RWE.
//!
//! Responsibility:
//!
//! - scan static `class="..."` tokens from rendered HTML
//! - compile supported utility tokens into CSS rules
//! - inject generated CSS into `<style data-rwe-tw>...</style>`
//!
//! Notes:
//!
//! - this module is the local Zebflow tailwind-like compiler lineage
//! - it is intentionally "Tailwind-like", not full Tailwind parity
//! - unsupported tokens are ignored safely (no panic, no hard failure)

mod compiler;
mod variants;

pub use compiler::{process_tailwind, rebuild_tailwind};
pub use variants::{TwVariantManifest, collect_tw_variants, dynamic_runtime_css_for_patterns};
