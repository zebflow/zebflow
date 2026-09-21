//! `geo.*` — geospatial files: `geo.convert`, `geo.inspect`.

use crate::pipeline::NodeDefinition;

pub mod convert;
pub mod inspect;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![convert::definition(), inspect::definition()]
}
