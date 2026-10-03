//! `input.audio` — one audio FileRef at `$trigger.files.<name>` — `input.file` with `--accept` preset to `audio`.
//!
//! One member of the `input.*` family; the checks, the flags and the handler
//! live in the parent module. This file is the kind's name and its
//! definition, so the registry has one `definition()` per kind.

use crate::pipeline::NodeDefinition;

pub const NODE_KIND: &str = "input.audio";

pub fn definition() -> NodeDefinition {
    super::definition_for(super::InputKind::Audio)
}
