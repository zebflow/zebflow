//! `web.*` — answering and building the web: `web.response.send` answers the
//! request, `web.site.generate` writes a static site (`site/`) on the shared
//! site machinery (`static_site.rs`).

use crate::pipeline::NodeDefinition;

pub mod response;
pub mod site;
pub mod static_site;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![response::definition(), site::definition()]
}
