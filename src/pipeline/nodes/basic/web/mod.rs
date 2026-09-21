//! `web.*` — answering and building the web: `web.response`,
//! `web.docs_generate`, `web.static_generate`, `web.static_site`.

use crate::pipeline::NodeDefinition;

pub mod docs_generate;
pub mod response;
pub mod static_generate;
pub mod static_site;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![
        response::definition(),
        docs_generate::definition(),
        static_generate::definition(),
    ]
}
