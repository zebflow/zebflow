//! Concrete RWE engine implementations.

mod rwe;

use std::sync::Arc;

use super::interface::ReactiveWebEngine;

pub use rwe::RweReactiveWebEngine;

/// Instantiates one RWE engine implementation by id.
pub fn instantiate_engine_by_id(id: &str) -> Option<Arc<dyn ReactiveWebEngine>> {
    match id {
        "rwe" => Some(Arc::new(RweReactiveWebEngine::default())),
        _ => None,
    }
}

/// Resolves one RWE engine id with default.
pub fn resolve_engine_or_default(id: Option<&str>) -> Arc<dyn ReactiveWebEngine> {
    if let Some(id) = id
        && let Some(engine) = instantiate_engine_by_id(id)
    {
        return engine;
    }
    Arc::new(RweReactiveWebEngine::default())
}
