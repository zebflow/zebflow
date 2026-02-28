//! Platform service layer (auth, user, project, platform bootstrap).

pub mod authorization;
pub mod auth;
pub mod platform;
pub mod project;
pub mod user;

pub use authorization::AuthorizationService;
pub use auth::AuthService;
pub use platform::PlatformService;
pub use project::ProjectService;
pub use user::UserService;
