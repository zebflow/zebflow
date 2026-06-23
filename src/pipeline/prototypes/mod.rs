//! Pipeline execution prototypes.
//!
//! These modules are intentionally not part of the production runtime dispatch.
//! They are small, measurable execution-model experiments used to validate
//! foundational changes before moving them into [`crate::pipeline::engines`].
//!
//! Start with [`visual`] for the "Visual Rust" prototype: node-based execution
//! where payloads move like Rust values, large data uses typed payloads or
//! handles, and trace records summaries rather than cloning full node input and
//! output JSON.

pub mod visual;
