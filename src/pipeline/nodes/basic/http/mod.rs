//! `http.*` — outbound HTTP: `http.response.fetch`.

use crate::pipeline::NodeDefinition;

pub mod request;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![request::definition()]
}
