//! `sqlite.*` — SQLite files in the project: `sqlite.query`, `sqlite.mutate`.

use crate::pipeline::NodeDefinition;

pub mod mutate;
pub mod query;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![mutate::definition(), query::definition()]
}
