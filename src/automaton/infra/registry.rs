//! Registry for automaton engine implementations.

use std::collections::HashMap;
use std::sync::Arc;

use super::interface::AutomatonEngine;

/// In-memory registry mapping automaton engine id to implementation.
#[derive(Clone, Default)]
pub struct AutomatonEngineRegistry {
    engines: HashMap<String, Arc<dyn AutomatonEngine>>,
}

impl AutomatonEngineRegistry {
    /// Creates an empty automaton registry.
    pub fn new() -> Self {
        Self {
            engines: HashMap::new(),
        }
    }

    /// Registers/overwrites an automaton engine by id.
    pub fn register(&mut self, engine: Arc<dyn AutomatonEngine>) {
        self.engines.insert(engine.id().to_string(), engine);
    }

    /// Retrieves an automaton engine by id.
    pub fn get(&self, id: &str) -> Option<Arc<dyn AutomatonEngine>> {
        self.engines.get(id).map(Arc::clone)
    }

    /// Returns sorted engine ids for diagnostics/UI display.
    pub fn ids(&self) -> Vec<String> {
        let mut ids: Vec<String> = self.engines.keys().cloned().collect();
        ids.sort();
        ids
    }
}
