//! `sekejap.*` — the Sekejap database: `sekejap.record.create`, `sekejap.query.run`.

use crate::pipeline::NodeDefinition;

pub mod query;
pub mod record;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![record::definition(), query::definition()]
}
