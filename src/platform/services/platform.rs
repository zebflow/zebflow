//! Platform composition root for adapters + services.

use std::sync::Arc;

use crate::platform::adapters::data::{DataAdapter, build_data_adapter};
use crate::platform::adapters::file::{FileAdapter, build_file_adapter};
use crate::platform::adapters::project_data::{ProjectDataFactory, build_project_data_factory};
use crate::platform::error::PlatformError;
use crate::platform::model::{CreateProjectRequest, CreateUserRequest, PlatformConfig};
use crate::platform::services::{AuthService, ProjectService, UserService};

/// Main platform service graph, created once per process.
#[derive(Clone)]
pub struct PlatformService {
    /// Effective config.
    pub config: PlatformConfig,
    /// Metadata backend.
    pub data: Arc<dyn DataAdapter>,
    /// File/project backend.
    pub file: Arc<dyn FileAdapter>,
    /// Project runtime data factory (sekejap/sqlite/...).
    pub project_data: Arc<dyn ProjectDataFactory>,
    /// User domain service.
    pub users: Arc<UserService>,
    /// Auth domain service.
    pub auth: Arc<AuthService>,
    /// Project domain service.
    pub projects: Arc<ProjectService>,
}

impl PlatformService {
    /// Builds platform from config and runs bootstrap initialization.
    pub fn from_config(config: PlatformConfig) -> Result<Self, PlatformError> {
        std::fs::create_dir_all(&config.data_root)?;
        let data = build_data_adapter(config.data_adapter, &config.data_root)?;
        let file = build_file_adapter(config.file_adapter, config.data_root.clone());
        let project_data = build_project_data_factory(&config.data_root);
        file.initialize()?;

        let users = Arc::new(UserService::new(data.clone()));
        let projects = Arc::new(ProjectService::new(
            data.clone(),
            file.clone(),
            project_data.clone(),
        ));
        let auth = Arc::new(AuthService::new(users.clone()));

        let svc = Self {
            config,
            data,
            file,
            project_data,
            users,
            auth,
            projects,
        };
        svc.bootstrap_defaults()?;
        Ok(svc)
    }

    /// Creates default superadmin + default project if missing.
    pub fn bootstrap_defaults(&self) -> Result<(), PlatformError> {
        self.users.create_or_update_user(&CreateUserRequest {
            owner: self.config.default_owner.clone(),
            password: self.config.default_password.clone(),
            role: "superadmin".to_string(),
        })?;

        self.projects.create_or_update_project(
            &self.config.default_owner,
            &CreateProjectRequest {
                project: self.config.default_project.clone(),
                title: Some("Default".to_string()),
            },
        )?;
        Ok(())
    }
}
