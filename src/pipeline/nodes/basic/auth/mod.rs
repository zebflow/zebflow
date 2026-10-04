//! `auth.*` — the app's tokens: `auth.token.create` answers `token: { access_token, … }`,
//! `auth.token.verify` answers `token: { valid, claims }`, `auth.oauth.approve` answers
//! `oauth: { approved, client, redirect_host }` and the redirect back to an MCP client.
//! [`sign`] reads a `jwt_signing_key` credential's keys for all of them.

use crate::pipeline::NodeDefinition;

pub mod oauth_approve;
pub mod sign;
pub mod token_create;
pub mod token_verify;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![token_create::definition(), token_verify::definition(), oauth_approve::definition()]
}
