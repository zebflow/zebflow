use std::collections::BTreeMap;
use std::sync::Arc;

use super::driver::DbDriver;
use super::drivers::{PostgresqlDbDriver, SjtableDbDriver};

/// Registry of DB runtime drivers keyed by normalized database kind.
#[derive(Clone, Default)]
pub struct DbDriverRegistry {
    drivers: BTreeMap<String, Arc<dyn DbDriver>>,
}

impl DbDriverRegistry {
    /// Builds the default registry used by platform runtime.
    pub fn with_defaults() -> Self {
        let mut out = Self::default();
        out.register(Arc::new(SjtableDbDriver::default()));
        out.register(Arc::new(PostgresqlDbDriver::default()));
        out
    }

    /// Registers or replaces one driver.
    pub fn register(&mut self, driver: Arc<dyn DbDriver>) {
        self.drivers.insert(driver.kind().to_string(), driver);
    }

    /// Resolves one driver by database kind.
    pub fn get(&self, database_kind: &str) -> Option<Arc<dyn DbDriver>> {
        self.drivers.get(database_kind).cloned()
    }
}
