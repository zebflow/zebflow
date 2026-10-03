//! `sekejap.*` — the Sekejap database: `sekejap.record.create`, `sekejap.query.run`.

use crate::pipeline::NodeDefinition;

pub mod insert;
pub mod query;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![insert::definition(), query::definition()]
}
