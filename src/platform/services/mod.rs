//! Platform service layer (auth, user, project, platform bootstrap).

pub mod authorization;
pub mod auth;
pub mod credential;
pub mod pipeline_runtime;
pub mod platform;
pub mod project;
pub mod simple_table;
pub mod user;

pub use authorization::AuthorizationService;
pub use auth::AuthService;
pub use credential::CredentialService;
pub use pipeline_runtime::PipelineRuntimeService;
pub use platform::PlatformService;
pub use project::ProjectService;
pub use simple_table::SimpleTableService;
pub use user::UserService;
