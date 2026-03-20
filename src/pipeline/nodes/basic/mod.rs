//! Built-in framework node set.

use crate::pipeline::NodeDefinition;

pub mod auth_token_create;
pub mod crypto;
pub mod http_request;
pub mod logic;
pub mod pg_query;
pub mod script;
pub mod sjtable_mutate;
pub mod sjtable_query;
pub mod trigger;
mod util;
pub mod web_render;
pub mod ws_emit;
pub mod ws_sync_state;
pub mod ws_trigger;
pub mod zebtune;

/// Returns built-in node definitions sorted by kind.
pub fn builtin_node_definitions() -> Vec<NodeDefinition> {
    let mut items = vec![
        auth_token_create::definition(),
        trigger::webhook::definition(),
        trigger::schedule::definition(),
        trigger::manual::definition(),
        ws_trigger::definition(),
        script::definition(),
        http_request::definition(),
        sjtable_mutate::definition(),
        sjtable_query::definition(),
        pg_query::definition(),
        web_render::definition(),
        ws_sync_state::definition(),
        ws_emit::definition(),
        zebtune::definition(),
        logic::if_::definition(),
        logic::switch::definition(),
        logic::branch::definition(),
        logic::merge::definition(),
        crypto::definition(),
        trigger::weberror::definition(),
    ];
    items.sort_by(|a, b| a.kind.cmp(&b.kind));
    items
}
