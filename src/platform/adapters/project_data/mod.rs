//! Project-level runtime data adapter interfaces.
//!
//! This layer is intentionally separate from platform metadata storage.
//! Platform metadata lives in one global catalog DB, while each project gets
//! its own runtime data stores (for nodes such as SQLite and Sekejap).

use std::path::Path;
use std::sync::Arc;

use crate::platform::error::PlatformError;
use crate::platform::model::ProjectFileLayout;

/// One project runtime data engine implementation.
pub trait ProjectDataEngine: Send + Sync {
    /// Stable engine id.
    fn id(&self) -> &'static str;
    /// Ensure engine storage exists for a project layout.
    fn initialize(&self, layout: &ProjectFileLayout) -> Result<(), PlatformError>;
}

/// Factory used by project service to initialize all configured project DB engines.
pub trait ProjectDataFactory: Send + Sync {
    /// Stable factory id.
    fn id(&self) -> &'static str;
    /// Ensure all configured engines are initialized for one project.
    fn initialize_project(&self, layout: &ProjectFileLayout) -> Result<(), PlatformError>;
    /// Engine ids currently enabled by this factory.
    fn enabled_engines(&self) -> Vec<&'static str>;
}

/// Project SQLite runtime DB engine — creates a WAL-mode `local.db` in the project data dir.
#[derive(Default)]
pub struct ProjectSqliteEngine;

impl ProjectDataEngine for ProjectSqliteEngine {
    fn id(&self) -> &'static str {
        "project_data.sqlite"
    }

    fn initialize(&self, layout: &ProjectFileLayout) -> Result<(), PlatformError> {
        std::fs::create_dir_all(&layout.data_dir)?;
        let db_path = layout.data_dir.join("local.db");
        let conn = rusqlite::Connection::open(&db_path)
            .map_err(|e| PlatformError::new("PROJECT_DATA_SQLITE_INIT", e.to_string()))?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .map_err(|e| PlatformError::new("PROJECT_DATA_SQLITE_PRAGMA", e.to_string()))?;
        Ok(())
    }
}

/// Optional future Postgres runtime configuration placeholder.
#[derive(Default)]
pub struct ProjectPostgresEngine;

impl ProjectDataEngine for ProjectPostgresEngine {
    fn id(&self) -> &'static str {
        "project_data.postgres"
    }

    fn initialize(&self, _layout: &ProjectFileLayout) -> Result<(), PlatformError> {
        Ok(())
    }
}

/// Project Sekejap runtime engine — creates the persistent `data/sekejap` directory.
#[derive(Default)]
pub struct ProjectSekejapEngine;

impl ProjectDataEngine for ProjectSekejapEngine {
    fn id(&self) -> &'static str {
        "project_data.sekejap"
    }

    fn initialize(&self, layout: &ProjectFileLayout) -> Result<(), PlatformError> {
        std::fs::create_dir_all(layout.data_dir.join("sekejap"))?;
        Ok(())
    }
}

/// Default factory enabling local project SQLite store.
pub struct DefaultProjectDataFactory {
    engines: Vec<Arc<dyn ProjectDataEngine>>,
}

impl Default for DefaultProjectDataFactory {
    fn default() -> Self {
        Self {
            engines: vec![
                Arc::new(ProjectSqliteEngine),
                Arc::new(ProjectSekejapEngine),
            ],
        }
    }
}

impl ProjectDataFactory for DefaultProjectDataFactory {
    fn id(&self) -> &'static str {
        "project_data.default"
    }

    fn initialize_project(&self, layout: &ProjectFileLayout) -> Result<(), PlatformError> {
        for engine in &self.engines {
            engine.initialize(layout)?;
        }
        Ok(())
    }

    fn enabled_engines(&self) -> Vec<&'static str> {
        self.engines.iter().map(|e| e.id()).collect()
    }
}

/// Build default runtime project data factory.
pub fn build_project_data_factory(_data_root: &Path) -> Arc<dyn ProjectDataFactory> {
    Arc::new(DefaultProjectDataFactory::default())
}
