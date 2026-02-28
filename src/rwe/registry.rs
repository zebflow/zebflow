//! Registry for reactive web engine implementations.

use std::collections::HashMap;
use std::sync::Arc;

use super::interface::ReactiveWebEngine;

/// In-memory registry mapping RWE engine id to implementation.
#[derive(Clone, Default)]
pub struct ReactiveWebEngineRegistry {
    engines: HashMap<String, Arc<dyn ReactiveWebEngine>>,
}

impl ReactiveWebEngineRegistry {
    /// Creates an empty RWE registry.
    pub fn new() -> Self {
        Self {
            engines: HashMap::new(),
        }
    }

    /// Registers/overwrites an RWE engine by id.
    pub fn register(&mut self, engine: Arc<dyn ReactiveWebEngine>) {
        self.engines.insert(engine.id().to_string(), engine);
    }

    /// Retrieves an RWE engine by id.
    pub fn get(&self, id: &str) -> Option<Arc<dyn ReactiveWebEngine>> {
        self.engines.get(id).map(Arc::clone)
    }

    /// Returns sorted engine ids for diagnostics/UI display.
    pub fn ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.engines.keys().cloned().collect();
        ids.sort();
        ids
    }
}
