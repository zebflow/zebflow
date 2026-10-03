//! `sqlite.*` — the project's built-in SQLite database: `sqlite.query.run`
//! (reads, and writes with `--write`).

use crate::pipeline::NodeDefinition;

pub mod query;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![query::definition()]
}
