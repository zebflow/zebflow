//! Built-in framework node set.

use crate::pipeline::NodeDefinition;

pub mod agent;
pub mod auth_token_create;
pub mod browser_run;
pub mod crypto;
pub mod file_save;
pub mod function_call;
pub mod img_thumbnail;
pub mod http_request;
pub mod logic;
pub mod pg_query;
pub mod script;
pub mod sqlite_mutate;
pub mod sqlite_query;
pub mod trigger;
mod util;
pub mod web_response;
pub mod ws_emit;
pub mod ws_sync_state;
pub mod ws_trigger;

/// Returns built-in node definitions sorted by kind.
pub fn builtin_node_definitions() -> Vec<NodeDefinition> {
    let mut items = vec![
        agent::definition(),
        auth_token_create::definition(),
        browser_run::definition(),
        file_save::definition(),
        function_call::definition(),
        img_thumbnail::definition(),
        trigger::function::definition(),
        trigger::webhook::definition(),
        trigger::schedule::definition(),
        trigger::manual::definition(),
        ws_trigger::definition(),
        script::definition(),
        http_request::definition(),
        sqlite_mutate::definition(),
        sqlite_query::definition(),
        pg_query::definition(),
        web_response::definition(),
        ws_sync_state::definition(),
        ws_emit::definition(),
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
