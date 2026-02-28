//! User management service.

use std::sync::Arc;

use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{CreateUserRequest, PlatformUser, StoredUser, now_ts, slug_segment};

/// User service backed by a swappable data adapter.
pub struct UserService {
    data: Arc<dyn DataAdapter>,
}

impl UserService {
    /// Creates user service.
    pub fn new(data: Arc<dyn DataAdapter>) -> Self {
        Self { data }
    }

    /// Lists all users.
    pub fn list_users(&self) -> Result<Vec<PlatformUser>, PlatformError> {
        self.data.list_users()
    }

    /// Creates or updates one user.
    pub fn create_or_update_user(
        &self,
        req: &CreateUserRequest,
    ) -> Result<PlatformUser, PlatformError> {
        let owner = slug_segment(&req.owner);
        if owner.is_empty() {
            return Err(PlatformError::new(
                "PLATFORM_USER_INVALID",
                "owner must not be empty",
            ));
        }
        if req.password.trim().is_empty() {
            return Err(PlatformError::new(
                "PLATFORM_USER_INVALID",
                "password must not be empty",
            ));
        }
        let now = now_ts();
        let existing = self.data.get_user_auth(&owner)?;
        let created_at = existing
            .as_ref()
            .map(|u| u.profile.created_at)
            .unwrap_or(now);
        let profile = PlatformUser {
            owner: owner.clone(),
            role: req.role.trim().to_string(),
            created_at,
            updated_at: now,
        };
        let stored = StoredUser {
            profile: profile.clone(),
            password: req.password.clone(),
        };
        self.data.put_user(&stored)?;
        Ok(profile)
    }

    /// Returns whether owner/password are valid.
    pub fn authenticate(&self, owner: &str, password: &str) -> Result<bool, PlatformError> {
        let owner = slug_segment(owner);
        if owner.is_empty() {
            return Ok(false);
        }
        let Some(found) = self.data.get_user_auth(&owner)? else {
            return Ok(false);
        };
        Ok(found.password == password)
    }
}
