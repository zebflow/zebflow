//! Registry for language engine implementations.

use std::collections::HashMap;
use std::sync::Arc;

use super::interface::LanguageEngine;

/// In-memory registry mapping language engine id to implementation.
#[derive(Clone, Default)]
pub struct LanguageEngineRegistry {
    engines: HashMap<String, Arc<dyn LanguageEngine>>,
}

impl LanguageEngineRegistry {
    /// Creates an empty language registry.
    pub fn new() -> Self {
        Self {
            engines: HashMap::new(),
        }
    }

    /// Registers/overwrites a language engine by id.
    pub fn register(&mut self, engine: Arc<dyn LanguageEngine>) {
        self.engines.insert(engine.id().to_string(), engine);
    }

    /// Retrieves a language engine by id.
    pub fn get(&self, id: &str) -> Option<Arc<dyn LanguageEngine>> {
        self.engines.get(id).map(Arc::clone)
    }

    /// Returns sorted engine ids for diagnostics/UI display.
    pub fn ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.engines.keys().cloned().collect();
        ids.sort();
        ids
    }
}
