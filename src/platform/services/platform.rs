//! Platform composition root for adapters + services.

use std::sync::Arc;

use crate::platform::adapters::data::{DataAdapter, build_data_adapter};
use crate::platform::adapters::file::{FileAdapter, build_file_adapter};
use crate::platform::adapters::project_data::{ProjectDataFactory, build_project_data_factory};
use crate::platform::error::PlatformError;
use crate::platform::model::{CreateProjectRequest, CreateUserRequest, PlatformConfig};
use crate::platform::services::{
    AssistantConfigService, AuthService, AuthorizationService, CredentialService,
    DbConnectionService, DbRuntimeService, McpSessionService, PipelineHitsService,
    PipelineRuntimeService, ProjectService, SimpleTableService, UserService, ZebflowJsonService,
};

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
    /// Project-level authorization service shared by REST/MCP/assistant.
    pub authz: Arc<AuthorizationService>,
    /// Project credential management service.
    pub credentials: Arc<CredentialService>,
    /// Project assistant config service.
    pub assistant_configs: Arc<AssistantConfigService>,
    /// Layer 2 project config (zebflow.json) service.
    pub zebflow_cfg: Arc<ZebflowJsonService>,
    /// Project DB connection management service.
    pub db_connections: Arc<DbConnectionService>,
    /// Project DB runtime service (kind-dispatched describe/query).
    pub db_runtime: Arc<DbRuntimeService>,
    /// Project domain service.
    pub projects: Arc<ProjectService>,
    /// Active production pipeline registry compiled from activated snapshots.
    pub pipeline_runtime: Arc<PipelineRuntimeService>,
    /// Lightweight execution hit/error counters per pipeline.
    pub pipeline_hits: Arc<PipelineHitsService>,
    /// Project Simple Table management service.
    pub simple_tables: Arc<SimpleTableService>,
    /// MCP session management (in-memory tokens for project-scoped remote control).
    pub mcp_sessions: Arc<McpSessionService>,
}

impl PlatformService {
    /// Builds platform from config and runs bootstrap initialization.
    pub fn from_config(config: PlatformConfig) -> Result<Self, PlatformError> {
        std::fs::create_dir_all(&config.data_root)?;
        let data = build_data_adapter(config.data_adapter, &config.data_root)?;
        let file = build_file_adapter(config.file_adapter, config.data_root.clone());
        let project_data = build_project_data_factory(&config.data_root);
        file.initialize()?;

        let zebflow_cfg = Arc::new(ZebflowJsonService::new(config.data_root.join("users")));
        let users = Arc::new(UserService::new(data.clone()));
        let projects = Arc::new(ProjectService::new(
            data.clone(),
            file.clone(),
            project_data.clone(),
            zebflow_cfg.clone(),
        ));
        let auth = Arc::new(AuthService::new(users.clone()));
        let authz = Arc::new(AuthorizationService::new(data.clone()));
        let credentials = Arc::new(CredentialService::new(data.clone()));
        let assistant_configs = Arc::new(AssistantConfigService::new(data.clone(), zebflow_cfg.clone()));
        let db_connections = Arc::new(DbConnectionService::new(data.clone()));
        let simple_tables = Arc::new(SimpleTableService::new(file.clone(), project_data.clone()));
        let db_runtime = Arc::new(DbRuntimeService::new(
            db_connections.clone(),
            credentials.clone(),
            simple_tables.clone(),
        ));
        let pipeline_runtime = Arc::new(PipelineRuntimeService::new(projects.clone()));
        let pipeline_hits = Arc::new(PipelineHitsService::new(10));
        let mcp_sessions = Arc::new(McpSessionService::new(data.clone()));

        let svc = Self {
            config,
            data,
            file,
            project_data,
            users,
            auth,
            authz,
            credentials,
            assistant_configs,
            zebflow_cfg,
            db_connections,
            db_runtime,
            projects,
            pipeline_runtime,
            pipeline_hits,
            simple_tables,
            mcp_sessions,
        };
        svc.bootstrap_defaults()?;
        let _ = svc
            .pipeline_runtime
            .refresh_project(&svc.config.default_owner, &svc.config.default_project);
        Ok(svc)
    }

    /// Creates default superadmin + default project if missing.
    pub fn bootstrap_defaults(&self) -> Result<(), PlatformError> {
        if self.config.default_password.trim().is_empty() {
            return Err(PlatformError::new(
                "PLATFORM_BOOTSTRAP_PASSWORD_MISSING",
                "default superadmin password is missing; set ZEBFLOW_PLATFORM_DEFAULT_PASSWORD or provide PlatformConfig.default_password",
            ));
        }

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
