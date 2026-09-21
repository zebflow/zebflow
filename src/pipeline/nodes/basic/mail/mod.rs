//! `mail.*` — outbound mail: `mail.send`.

use crate::pipeline::NodeDefinition;

pub mod send;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![send::definition()]
}
