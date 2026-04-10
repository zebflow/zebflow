//! Platform service layer.
//!
//! This module contains product-facing orchestration services: auth, project lifecycle,
//! studio-facing helpers, and the future cluster control-plane services that sit on top of
//! `infra/`.
//!
//! Boundary rule:
//!
//! - `platform/services/*` owns business orchestration and product policy
//! - `infra/*` owns reusable transport, storage, state, execution, and cluster mechanics
//!
//! That split lets the product add master/worker UX, placement, and bootstrap flows without
//! burying transport/security logic inside the web router or individual handlers.

pub mod assistant_config;
pub mod assistant_tools;
pub mod access;
pub mod auth;
pub mod authorization;
pub mod cluster;
pub mod credential;
pub mod db_connection;
pub mod db_runtime;
pub mod library;
pub mod mcp_session;
pub mod ops;
pub mod pipeline_hits;
pub mod pipeline_runtime;
pub mod platform;
pub mod project;
pub mod project_config;
pub mod tsx_outline;
pub mod user;
pub mod zeb_lock;

pub use assistant_config::AssistantConfigService;
pub use assistant_tools::AssistantPlatformTools;
pub use access::{GitIdentityService, ProjectInviteService, ProjectMembershipService};
pub use auth::AuthService;
pub use authorization::AuthorizationService;
pub use cluster::{
    ClusterBootstrapService, ClusterPlacementService, ClusterRegistryService,
    ClusterRuntimeSyncService,
};
pub use credential::CredentialService;
pub use db_connection::DbConnectionService;
pub use db_runtime::DbRuntimeService;
pub use library::LibraryService;
pub use mcp_session::McpSessionService;
pub use ops::PlatformOps;
pub use pipeline_hits::PipelineHitsService;
pub use pipeline_runtime::{PipelineRuntimeService, WsTriggerSpec};
pub use platform::PlatformService;
pub use project::ProjectService;
pub use project_config::ZebflowJsonService;
pub use zeb_lock::ZebLockService;
pub use user::UserService;
