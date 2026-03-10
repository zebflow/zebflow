//! Built-in framework node set.

use crate::pipeline::NodeDefinition;

pub mod http_request;
pub mod logic;
pub mod pg_query;
pub mod script;
pub mod sjtable_query;
pub mod trigger;
mod util;
pub mod web_render;
pub mod zebtune;

/// Returns built-in node definitions sorted by kind.
pub fn builtin_node_definitions() -> Vec<NodeDefinition> {
    let mut items = vec![
        trigger::webhook::definition(),
        trigger::schedule::definition(),
        trigger::manual::definition(),
        script::definition(),
        http_request::definition(),
        sjtable_query::definition(),
        pg_query::definition(),
        web_render::definition(),
        zebtune::definition(),
        logic::if_::definition(),
        logic::switch::definition(),
        logic::branch::definition(),
        logic::merge::definition(),
    ];
    items.sort_by(|a, b| a.kind.cmp(&b.kind));
    items
}
