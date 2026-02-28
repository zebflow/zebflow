//! Project-level runtime data adapter interfaces.
//!
//! This layer is intentionally separate from platform metadata storage.
//! Platform metadata lives in one global catalog DB, while each project gets
//! its own runtime data stores (for nodes such as SimpleTable).

use std::path::Path;
use std::sync::Arc;

use sekejap::SekejapDB;

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

/// Project SeKejap runtime DB engine.
#[derive(Default)]
pub struct ProjectSekejapEngine;

impl ProjectDataEngine for ProjectSekejapEngine {
    fn id(&self) -> &'static str {
        "project_data.sekejap"
    }

    fn initialize(&self, layout: &ProjectFileLayout) -> Result<(), PlatformError> {
        std::fs::create_dir_all(&layout.data_sekejap_dir)?;
        let _db = SekejapDB::new(&layout.data_sekejap_dir, 500_000)
            .map_err(|e| PlatformError::new("PROJECT_DATA_SEKEJAP_INIT", e.to_string()))?;
        Ok(())
    }
}

/// Project SQLite runtime DB engine.
#[derive(Default)]
pub struct ProjectSqliteEngine;

impl ProjectDataEngine for ProjectSqliteEngine {
    fn id(&self) -> &'static str {
        "project_data.sqlite"
    }

    fn initialize(&self, layout: &ProjectFileLayout) -> Result<(), PlatformError> {
        let Some(parent) = layout.data_sqlite_file.parent() else {
            return Err(PlatformError::new(
                "PROJECT_DATA_SQLITE_INIT",
                "invalid sqlite file path",
            ));
        };
        std::fs::create_dir_all(parent)?;
        if !layout.data_sqlite_file.exists() {
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .truncate(false)
                .write(true)
                .open(&layout.data_sqlite_file)?;
        }
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

/// Default factory enabling local project SeKejap + SQLite stores.
pub struct DefaultProjectDataFactory {
    engines: Vec<Arc<dyn ProjectDataEngine>>,
}

impl Default for DefaultProjectDataFactory {
    fn default() -> Self {
        Self {
            engines: vec![
                Arc::new(ProjectSekejapEngine),
                Arc::new(ProjectSqliteEngine),
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
