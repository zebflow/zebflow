//! `zebflow.*` — Zebflow operating on itself, inside a fence:
//! `zebflow.command.run`.

use crate::pipeline::NodeDefinition;

pub mod command;
mod managed;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![command::definition()]
}
