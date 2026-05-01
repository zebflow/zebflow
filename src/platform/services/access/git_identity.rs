//! User-bound git identity resolution.
//!
//! Zebflow historically stored git author identity in project settings only.
//! That is workable for a single owner, but it is the wrong long-term model for
//! multi-user collaboration. This service establishes the baseline precedence
//! order for `0.2.0`:
//!
//! 1. current user profile
//! 2. generated fallback identity
//!
//! Project config no longer owns author identity. Commit attribution belongs to
//! the acting platform user.

use std::sync::Arc;

use crate::platform::model::{GitIdentitySource, ResolvedGitIdentity, slug_segment};
use crate::platform::services::UserService;

/// Resolve git identity for the current actor and project.
pub struct GitIdentityService {
    users: Arc<UserService>,
}

impl GitIdentityService {
    /// Create git identity service.
    pub fn new(users: Arc<UserService>) -> Self {
        Self { users }
    }

    /// Resolve the author identity for one concrete action.
    pub fn resolve_for_actor(
        &self,
        actor_user: Option<&str>,
        project_slug: &str,
    ) -> ResolvedGitIdentity {
        let actor = actor_user.map(slug_segment).unwrap_or_default();
        let user_profile = if actor.is_empty() {
            None
        } else {
            self.users.get_user(&actor).ok().flatten()
        };

        let (name, email, source) = if let Some(user) = user_profile {
            let name = pick_name(&user.git_name, project_slug);
            let email = pick_email(&user.git_email, project_slug);
            let source = if !user.git_name.trim().is_empty() || !user.git_email.trim().is_empty() {
                GitIdentitySource::UserProfile
            } else {
                GitIdentitySource::Fallback
            };
            (name, email, source)
        } else {
            (
                fallback_name(project_slug),
                fallback_email(project_slug),
                GitIdentitySource::Fallback,
            )
        };

        ResolvedGitIdentity {
            name,
            email,
            source,
        }
    }
}

fn pick_name(user_name: &str, project_slug: &str) -> String {
    if !user_name.trim().is_empty() {
        user_name.trim().to_string()
    } else {
        fallback_name(project_slug)
    }
}

fn pick_email(user_email: &str, project_slug: &str) -> String {
    if !user_email.trim().is_empty() {
        user_email.trim().to_string()
    } else {
        fallback_email(project_slug)
    }
}

fn fallback_name(project_slug: &str) -> String {
    slug_segment(project_slug)
}

fn fallback_email(project_slug: &str) -> String {
    format!("{}@zebflow.local", slug_segment(project_slug))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    use crate::platform::adapters::data::build_data_adapter;
    use crate::platform::model::{CreateUserRequest, DataAdapterKind};
    use crate::platform::services::UserService;

    fn temp_test_dir(label: &str) -> std::path::PathBuf {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("zebflow-{label}-{ts}"));
        let _ = std::fs::create_dir_all(&path);
        path
    }

    #[test]
    fn falls_back_to_generated_identity_without_user_profile() {
        let temp_dir = temp_test_dir("git-identity-project");
        let data = build_data_adapter(DataAdapterKind::Sqlite, &temp_dir).unwrap();
        let users = Arc::new(UserService::new(data));
        let svc = GitIdentityService::new(users);

        let fallback = svc.resolve_for_actor(None, "demo");
        assert_eq!(fallback.name, "demo");
        assert_eq!(fallback.email, "demo@zebflow.local");
        assert_eq!(fallback.source, GitIdentitySource::Fallback);
    }

    #[test]
    fn user_profile_overrides_project_defaults() {
        let temp_dir = temp_test_dir("git-identity-user");
        let data = build_data_adapter(DataAdapterKind::Sqlite, &temp_dir).unwrap();
        let users = Arc::new(UserService::new(data));
        let _ = users.create_or_update_user(&CreateUserRequest {
            owner: "alice".to_string(),
            password: "secret".to_string(),
            role: "member".to_string(),
            git_name: "Alice Smith".to_string(),
            git_email: "alice@example.com".to_string(),
        });
        let svc = GitIdentityService::new(users);

        let resolved = svc.resolve_for_actor(Some("alice"), "default");
        assert_eq!(resolved.name, "Alice Smith");
        assert_eq!(resolved.email, "alice@example.com");
        assert_eq!(resolved.source, GitIdentitySource::UserProfile);
    }
}
