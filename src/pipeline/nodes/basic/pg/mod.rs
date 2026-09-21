//! `pg.*` — Postgres: `pg.query`.

use crate::pipeline::NodeDefinition;

pub mod query;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![query::definition()]
}
