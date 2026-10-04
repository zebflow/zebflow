//! `ai.*` — nodes that call a model: `ai.text.generate`, `ai.audio.generate`.
//! `ai.embedding.generate` is an official composite
//! (`src/pipeline/nodes/bundled/openai-embedding/`).
//!
//! Each is a swappable task (`node-conventions.md` §11): one kind, a literal
//! `--provider`, and a profile per provider in its definition.

use serde_json::Value;

use crate::pipeline::{NodeDefinition, PipelineError};

pub mod agent;
pub mod tts;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![agent::definition(), tts::definition()]
}

/// A config still holding `{{ }}` against its provider's profile, for the
/// kinds of this family; any other kind passes.
pub fn check_profile_of(kind: &str, config: &Value) -> Result<(), PipelineError> {
    match kind {
        agent::NODE_KIND => agent::check_config_profile(config),
        tts::NODE_KIND => tts::check_config_profile(config),
        _ => Ok(()),
    }
}
