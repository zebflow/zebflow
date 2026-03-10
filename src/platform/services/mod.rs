//! Platform service layer (auth, user, project, platform bootstrap).

pub mod assistant_config;
pub mod assistant_tools;
pub mod auth;
pub mod authorization;
pub mod credential;
pub mod db_connection;
pub mod db_runtime;
pub mod mcp_session;
pub mod pipeline_hits;
pub mod pipeline_runtime;
pub mod platform;
pub mod project;
pub mod project_config;
pub mod simple_table;
pub mod user;

pub use assistant_config::AssistantConfigService;
pub use assistant_tools::AssistantPlatformTools;
pub use auth::AuthService;
pub use authorization::AuthorizationService;
pub use credential::CredentialService;
pub use db_connection::DbConnectionService;
pub use db_runtime::DbRuntimeService;
pub use mcp_session::McpSessionService;
pub use pipeline_hits::PipelineHitsService;
pub use pipeline_runtime::PipelineRuntimeService;
pub use platform::PlatformService;
pub use project::ProjectService;
pub use project_config::ZebflowJsonService;
pub use simple_table::SimpleTableService;
pub use user::UserService;
