//! `mapserver.*` — the project MapServer: `mapserver.layer.publish`, `mapserver.layer.unpublish`, `mapserver.layer.get`,
//! `mapserver.layer.list`. One file, four operations on the same layer registry.

use crate::pipeline::NodeDefinition;

pub mod crud;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![
        crud::publish_definition(),
        crud::unpublish_definition(),
        crud::get_definition(),
        crud::list_definition(),
    ]
}
