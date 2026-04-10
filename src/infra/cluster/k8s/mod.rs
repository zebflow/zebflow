//! Kubernetes bootstrap and manifest rendering helpers.
//!
//! The first goal is YAML generation, not a full operator or controller.
//!
//! This module should stay focused on generating predictable manifests from Zebflow's own runtime
//! model. Higher-level deployment UX can live elsewhere; this is the place for reusable manifest
//! fragments and render helpers.

pub mod render;
pub mod templates;

pub use render::RenderedK8sManifest;
