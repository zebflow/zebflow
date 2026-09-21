//! `sekejap.*` — the Sekejap database: `sekejap.insert`, `sekejap.query`.

use crate::pipeline::NodeDefinition;

pub mod insert;
pub mod query;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![insert::definition(), query::definition()]
}
