//! Zebflow platform module.
//!
//! Responsibility:
//!
//! - compose swappable adapters (data + file)
//! - provide service layer for auth/user/project
//! - expose Axum router for platform flow:
//!   `login -> home(project list) -> project page`

pub mod adapters;
pub mod db;
pub mod error;
pub mod mcp;
pub mod model;
pub mod operations;
pub mod services;
pub mod web;

use std::sync::Arc;

use axum::Router;

pub use error::PlatformError;
pub use model::{
    AuthSession, CreateProjectRequest, CreateUserRequest, DataAdapterKind, ExecutePipelineRequest,
    FileAdapterKind, LoginRequest, PipelineExecuteTrigger, PipelineLocateRequest, PipelineMeta,
    PipelineRegistryListing, PlatformConfig, PlatformProject, PlatformUser, ProjectAccessSubject,
    ProjectCapability, ProjectFileLayout, UpsertPipelineDefinitionRequest,
};
pub use services::{
    AssistantConfigService, AuthService, AuthorizationService, PlatformService, ProjectService,
    UserService,
};

/// Builds platform router + service graph from config.
pub fn build_router(config: PlatformConfig) -> Result<Router, PlatformError> {
    let platform = Arc::new(PlatformService::from_config(config)?);
    Ok(web::router(platform))
}
