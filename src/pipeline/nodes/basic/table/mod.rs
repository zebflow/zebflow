//! `table.*` — tabular files (CSV, Parquet, …): `table.convert`, `table.query`.

use crate::pipeline::NodeDefinition;

pub mod convert;
pub mod query;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![convert::definition(), query::definition()]
}
