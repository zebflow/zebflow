//! User management service.

use std::sync::Arc;

use argon2::{
    Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version,
    password_hash::{SaltString, rand_core::OsRng},
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    CreateUserRequest, PlatformUser, PlatformUserLocalAuth, StoredUser, UpdateUserSettingsRequest,
    now_ts, slug_segment,
};

/// User service backed by a swappable data adapter.
pub struct UserService {
    data: Arc<dyn DataAdapter>,
}

impl UserService {
    fn sha256_password(password: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        let digest = hasher.finalize();
        digest.iter().map(|b| format!("{b:02x}")).collect()
    }

    fn argon2id() -> Argon2<'static> {
        Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(65_536, 3, 4, None).expect("valid Argon2id params"),
        )
    }

    fn hash_password(password: &str) -> Result<String, PlatformError> {
        let salt = SaltString::generate(&mut OsRng);
        Self::argon2id()
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|err| PlatformError::new("PLATFORM_PASSWORD_HASH", err.to_string()))
    }

    fn verify_argon2_password(hash: &str, password: &str) -> bool {
        let Ok(parsed) = PasswordHash::new(hash) else {
            return false;
        };
        Self::argon2id()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok()
    }

    fn new_user_id() -> String {
        format!("usr_{}", Uuid::new_v4().simple())
    }

    /// Creates user service.
    pub fn new(data: Arc<dyn DataAdapter>) -> Self {
        Self { data }
    }

    /// Lists all users.
    pub fn list_users(&self) -> Result<Vec<PlatformUser>, PlatformError> {
        self.data.list_users()
    }

    /// Gets one user profile by owner id.
    pub fn get_user(&self, owner: &str) -> Result<Option<PlatformUser>, PlatformError> {
        Ok(self.data.get_user_auth(owner)?.map(|stored| stored.profile))
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
            user_id: existing
                .as_ref()
                .map(|u| u.profile.user_id.clone())
                .filter(|v| !v.is_empty())
                .unwrap_or_else(Self::new_user_id),
            owner: owner.clone(),
            role: req.role.trim().to_string(),
            git_name: req.git_name.trim().to_string(),
            git_email: req.git_email.trim().to_string(),
            created_at,
            updated_at: now,
        };
        let stored = StoredUser {
            profile: profile.clone(),
            auth: PlatformUserLocalAuth {
                user_id: profile.user_id.clone(),
                password_hash: Self::hash_password(&req.password)?,
                password_alg: "argon2id".to_string(),
                password_updated_at: now,
            },
        };
        self.data.put_user(&stored)?;
        Ok(profile)
    }

    /// Creates a new user and refuses to overwrite existing auth material.
    pub fn create_user(&self, req: &CreateUserRequest) -> Result<PlatformUser, PlatformError> {
        let owner = slug_segment(&req.owner);
        if owner.is_empty() {
            return Err(PlatformError::new(
                "PLATFORM_USER_INVALID",
                "owner must not be empty",
            ));
        }
        if self.data.get_user_auth(&owner)?.is_some() {
            return Err(PlatformError::new(
                "PLATFORM_USER_EXISTS",
                format!("user '{owner}' already exists"),
            ));
        }
        self.create_or_update_user(req)
    }

    /// Updates self-service user settings while preserving password and role.
    pub fn update_user_settings(
        &self,
        owner: &str,
        req: &UpdateUserSettingsRequest,
    ) -> Result<PlatformUser, PlatformError> {
        let owner = slug_segment(owner);
        if owner.is_empty() {
            return Err(PlatformError::new(
                "PLATFORM_USER_INVALID",
                "owner must not be empty",
            ));
        }
        let Some(existing) = self.data.get_user_auth(&owner)? else {
            return Err(PlatformError::new(
                "PLATFORM_USER_NOT_FOUND",
                format!("user '{owner}' not found"),
            ));
        };
        let profile = PlatformUser {
            user_id: existing.profile.user_id.clone(),
            owner: existing.profile.owner.clone(),
            role: existing.profile.role.clone(),
            git_name: req.git_name.trim().to_string(),
            git_email: req.git_email.trim().to_string(),
            created_at: existing.profile.created_at,
            updated_at: now_ts(),
        };
        self.data.put_user(&StoredUser {
            profile: profile.clone(),
            auth: existing.auth,
        })?;
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
        match found.auth.password_alg.as_str() {
            "argon2id" => Ok(Self::verify_argon2_password(
                &found.auth.password_hash,
                password,
            )),
            "sha256" | "" => {
                let ok = found.auth.password_hash == Self::sha256_password(password);
                if ok {
                    let mut upgraded = found.clone();
                    upgraded.auth.password_hash = Self::hash_password(password)?;
                    upgraded.auth.password_alg = "argon2id".to_string();
                    upgraded.auth.password_updated_at = now_ts();
                    let _ = self.data.put_user(&upgraded);
                }
                Ok(ok)
            }
            _ => Ok(false),
        }
    }
}
