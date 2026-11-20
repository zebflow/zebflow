//! `linux.*` — a Linux machine, reached over SSH as one user
//! (`docs/contracts/linux-control.md`): `linux.command.run`.

use crate::pipeline::NodeDefinition;

pub mod command;
pub mod ssh;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![command::definition()]
}
