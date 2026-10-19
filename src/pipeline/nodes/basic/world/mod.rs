//! `world.*` — data for the `zeb/world` library, made on the server:
//! `world.roads.bake` (`roads/bake.rs`), a place's road pack.
//!
//! The family is a domain, not a vendor: a page draws a world from packs and
//! tiles, and a node here makes one of them (the pack's reader is
//! `bakedRoads({ url })`, `help("web/world")`).

use std::sync::Arc;

use serde_json::Value;

use crate::pipeline::nodes::NodeHandler;
use crate::pipeline::{NodeDefinition, PipelineError};
use crate::platform::services::PlatformService;

pub mod roads;

pub fn definitions() -> Vec<NodeDefinition> {
    vec![roads::bake::definition()]
}

/// The node for `kind`, built from its resolved config; `None` when `kind` is
/// not of this family. Every kind reads and writes the project's stores,
/// which an engine without the platform cannot offer.
pub fn build(
    kind: &str,
    config: &Value,
    platform: Option<Arc<PlatformService>>,
) -> Result<Option<Box<dyn NodeHandler>>, PipelineError> {
    let node: Box<dyn NodeHandler> = match kind {
        roads::bake::NODE_KIND => {
            let config = crate::pipeline::nodes::shared::util::typed_config(config)
                .map_err(|err| PipelineError::new(roads::bake::CONFIG_CODE, err.to_string()))?;
            let platform = platform.ok_or_else(|| {
                PipelineError::new(roads::bake::CODE, "platform service not available in this engine context")
            })?;
            Box::new(roads::bake::Node::new(config, platform)?)
        }
        _ => return Ok(None),
    };
    Ok(Some(node))
}
