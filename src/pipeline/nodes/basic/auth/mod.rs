//! `auth.*` — signed tokens: `auth.token_create`, `auth.token_verify`.

use crate::pipeline::NodeDefinition;

pub mod token_create;
pub mod token_verify;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![token_create::definition(), token_verify::definition()]
}
