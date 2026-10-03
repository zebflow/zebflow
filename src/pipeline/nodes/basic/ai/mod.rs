//! `ai.*` — nodes that call a model: `ai.text.generate`, `ai.audio.generate`.

use crate::pipeline::NodeDefinition;

pub mod agent;
pub mod tts;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![agent::definition(), tts::definition()]
}
