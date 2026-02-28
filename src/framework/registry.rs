//! Registry for framework engine implementations.

use std::collections::HashMap;
use std::sync::Arc;

use super::interface::FrameworkEngine;

/// In-memory registry mapping framework engine id to implementation.
#[derive(Clone, Default)]
pub struct FrameworkEngineRegistry {
    engines: HashMap<String, Arc<dyn FrameworkEngine>>,
}

impl FrameworkEngineRegistry {
    /// Creates an empty framework registry.
    pub fn new() -> Self {
        Self {
            engines: HashMap::new(),
        }
    }

    /// Registers/overwrites a framework engine by its [`FrameworkEngine::id`].
    pub fn register(&mut self, engine: Arc<dyn FrameworkEngine>) {
        self.engines.insert(engine.id().to_string(), engine);
    }

    /// Retrieves a framework engine by id.
    pub fn get(&self, id: &str) -> Option<Arc<dyn FrameworkEngine>> {
        self.engines.get(id).map(Arc::clone)
    }

    /// Returns sorted engine ids for diagnostics/UI display.
    pub fn ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.engines.keys().cloned().collect();
        ids.sort();
        ids
    }
}
