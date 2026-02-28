//! Auth service facade.

use std::sync::Arc;

use crate::platform::error::PlatformError;
use crate::platform::model::AuthSession;
use crate::platform::services::user::UserService;

/// Auth service delegates credential checks to user service.
pub struct AuthService {
    users: Arc<UserService>,
}

impl AuthService {
    /// Creates auth service.
    pub fn new(users: Arc<UserService>) -> Self {
        Self { users }
    }

    /// Validates credentials and returns session on success.
    pub fn login(&self, owner: &str, password: &str) -> Result<Option<AuthSession>, PlatformError> {
        if self.users.authenticate(owner, password)? {
            Ok(Some(AuthSession {
                owner: owner.to_ascii_lowercase(),
            }))
        } else {
            Ok(None)
        }
    }
}
