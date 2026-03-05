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

    /// Resolves engine by id with fallback order:
    ///
    /// 1. requested id
    /// 2. `rwe`
    /// 3. first registered engine
    pub fn resolve_or_default(&self, id: Option<&str>) -> Option<Arc<dyn ReactiveWebEngine>> {
        if let Some(id) = id
            && let Some(engine) = self.get(id)
        {
            return Some(engine);
        }
        if let Some(engine) = self.get("rwe") {
            return Some(engine);
        }
        self.engines.values().next().map(Arc::clone)
    }
}
