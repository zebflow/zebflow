//! `http.*` — outbound HTTP: `http.request`.

use crate::pipeline::NodeDefinition;

pub mod request;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![request::definition()]
}
