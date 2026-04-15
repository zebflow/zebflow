//! Built-in framework node set.

use crate::pipeline::NodeDefinition;

pub mod agent;
pub mod auth_token_create;
pub mod browser_run;
pub mod crypto;
pub mod file_save;
pub mod function_call;
pub mod http_request;
pub mod img_thumbnail;
pub mod logic;
pub mod mem_del;
pub mod mem_exists;
pub mod mem_expire;
pub mod mem_get;
pub mod mem_incr;
pub mod mem_publish;
pub mod mem_set;
pub mod pg_query;
pub mod script;
pub mod sekejap_query;
pub mod sqlite_mutate;
pub mod sqlite_query;
pub mod trigger;
mod util;
pub mod web_response;
pub mod web_static_generate;
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
        mem_del::definition(),
        mem_exists::definition(),
        mem_expire::definition(),
        mem_get::definition(),
        mem_incr::definition(),
        mem_publish::definition(),
        mem_set::definition(),
        trigger::function::definition(),
        trigger::memsubscribe::definition(),
        trigger::webhook::definition(),
        trigger::schedule::definition(),
        trigger::manual::definition(),
        ws_trigger::definition(),
        script::definition(),
        http_request::definition(),
        sekejap_query::definition(),
        sqlite_mutate::definition(),
        sqlite_query::definition(),
        pg_query::definition(),
        web_response::definition(),
        web_static_generate::definition(),
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
