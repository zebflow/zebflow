//! `browser.*` — a headless browser: `browser.page.run`.

use crate::pipeline::NodeDefinition;

pub mod run;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![run::definition()]
}
