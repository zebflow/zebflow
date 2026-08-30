//! Auth service facade.
//!
//! One door, and one gate in front of it. `offices.md` §4 disables local
//! accounts for login while an office is joined, so the check belongs here
//! rather than in a route: every caller that turns an identifier and a password
//! into a session comes through [`AuthService::login`], and a gate on the route
//! would be a gate one new route could forget.

use std::sync::Arc;

use crate::platform::error::PlatformError;
use crate::platform::model::AuthSession;
use crate::platform::services::cluster::local_authority::OfficeLocalAuthorityService;
use crate::platform::services::user::UserService;

/// Auth service delegates credential checks to user service.
pub struct AuthService {
    users: Arc<UserService>,
    local_authority: Arc<OfficeLocalAuthorityService>,
}

impl AuthService {
    /// Creates auth service.
    pub fn new(users: Arc<UserService>, local_authority: Arc<OfficeLocalAuthorityService>) -> Self {
        Self {
            users,
            local_authority,
        }
    }

    /// Validates credentials and returns session on success.
    ///
    /// Three outcomes, not two. `Ok(Some)` is a session, `Ok(None)` is a
    /// credential that did not check out, and `Err` with
    /// [`crate::platform::services::cluster::local_authority::LOCAL_LOGIN_DISABLED_CODE`]
    /// is a joined office refusing the whole mechanism (`offices.md` §4).
    ///
    /// The gate runs **before** the credential is looked at, and that ordering
    /// is the privacy property rather than an optimisation: an office that
    /// checked the password first would answer differently for a real account
    /// than for a name it has never held, and a caller who cannot log in at all
    /// would still learn which local accounts exist and which passwords are
    /// good — an oracle handed to exactly the caller the term exists to turn
    /// away.
    pub fn login(&self, owner: &str, password: &str) -> Result<Option<AuthSession>, PlatformError> {
        if !self.local_authority.local_login_allowed()? {
            return Err(self.local_authority.refusal());
        }
        if self.users.authenticate(owner, password)? {
            Ok(Some(AuthSession {
                owner: owner.to_ascii_lowercase(),
            }))
        } else {
            Ok(None)
        }
    }
}
