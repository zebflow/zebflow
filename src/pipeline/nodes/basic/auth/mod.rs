//! `auth.*` — signed tokens: `auth.token.create` answers `token: { access_token, … }`, `auth.token.verify` answers `token: { valid, claims }`.

use crate::pipeline::NodeDefinition;

pub mod token_create;
pub mod token_verify;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![token_create::definition(), token_verify::definition()]
}
