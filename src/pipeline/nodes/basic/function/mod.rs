//! `function.*` — call a function pipeline: `function.result.call`.

use crate::pipeline::NodeDefinition;

pub mod call;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![call::definition()]
}
