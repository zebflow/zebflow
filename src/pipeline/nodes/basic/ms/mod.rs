//! `ms.*` — the project MapServer: `ms.layer.publish`, `ms.layer.unpublish`, `ms.layer.get`,
//! `ms.layer.list`. One file, four operations on the same layer registry.

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
