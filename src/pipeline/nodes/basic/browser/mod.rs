//! `browser.*` — a headless browser: `browser.run`.

use crate::pipeline::NodeDefinition;

pub mod run;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![run::definition()]
}
