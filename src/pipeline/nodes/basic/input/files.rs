//! `n.input.files` — an array of FileRefs at `files.<name>`; `--accept` and `--max` count.
//!
//! One member of the `input.*` family; the checks, the flags and the handler
//! live in the parent module. This file is the kind's name and its
//! definition, so the registry has one `definition()` per kind.

use crate::pipeline::NodeDefinition;

pub const NODE_KIND: &str = "n.input.files";

pub fn definition() -> NodeDefinition {
    super::definition_for(super::InputKind::Files)
}
