//! User management service.

use std::sync::{Arc, Condvar, Mutex, PoisonError};

use argon2::{
    Algorithm, Argon2, Params, PasswordHash, PasswordHasher, PasswordVerifier, Version,
    password_hash::{SaltString, rand_core::OsRng},
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    CREDENTIAL_STATE_CHOSEN, CREDENTIAL_STATE_GENERATED, CreateUserRequest, PlatformUser,
    PlatformUserLocalAuth, StoredUser, UpdateUserSettingsRequest, now_ts, slug_segment,
};
use crate::platform::services::bootstrap::{is_insecure_password, random_password};

/// Argon2id as OWASP's password storage sheet sets it: 19 MiB, 2 passes, 1
/// lane. Every check costs that memory and anyone who can reach `/login`
/// chooses how many checks run, so this is what one check may cost on the
/// smallest device, not the most a large server could afford.
const ARGON2_MEMORY_KIB: u32 = 19_456;
const ARGON2_PASSES: u32 = 2;
const ARGON2_LANES: u32 = 1;

/// How many Argon2 computations run at once in the whole process. Password
/// memory is then at most this many times `ARGON2_MEMORY_KIB`, however many
/// attempts arrive; the rest wait their turn.
const HASHING_AT_ONCE: usize = 2;
static HASHING: HashingGate = HashingGate::new(HASHING_AT_ONCE);

/// A counting gate: `run` waits until fewer than `limit` callers are inside.
struct HashingGate {
    limit: usize,
    running: Mutex<usize>,
    freed: Condvar,
}

impl HashingGate {
    const fn new(limit: usize) -> Self {
        Self {
            limit,
            running: Mutex::new(0),
            freed: Condvar::new(),
        }
    }

    fn run<T>(&self, work: impl FnOnce() -> T) -> T {
        let mut running = self.running.lock().unwrap_or_else(PoisonError::into_inner);
        while *running >= self.limit {
            running = self.freed.wait(running).unwrap_or_else(PoisonError::into_inner);
        }
        *running += 1;
        drop(running);
        // Leaving is a drop, so a computation that panics still frees its turn.
        let _turn = HashingTurn(self);
        work()
    }
}

struct HashingTurn<'a>(&'a HashingGate);

impl Drop for HashingTurn<'_> {
    fn drop(&mut self) {
        *self.0.running.lock().unwrap_or_else(PoisonError::into_inner) -= 1;
        self.0.freed.notify_one();
    }
}

/// Failed sign-ins a name may have before each next attempt waits.
const FREE_FAILURES: u32 = 5;
/// The first wait after them, doubled with each further failure up to
/// `LONGEST_WAIT`: a guesser gets about one try a minute per name, and a
/// person whose name is being guessed waits at most that long.
const FIRST_WAIT: std::time::Duration = std::time::Duration::from_secs(1);
const LONGEST_WAIT: std::time::Duration = std::time::Duration::from_secs(60);
/// Names whose failures are remembered at once. Unknown names are counted
/// too — a wait only real names could earn would tell a guesser which
/// names exist — so the count is bounded rather than per account.
const FAILURES_KEPT: usize = 4096;

/// A name's failed sign-ins in a row, and when its next attempt may run.
struct Failures {
    count: u32,
    next_attempt: std::time::Instant,
}

/// A hash no password matches, checked for a name with no account so that
/// answering "no" costs the same as for a real one: a fast refusal would
/// tell a guesser which names exist.
static NO_ACCOUNT: std::sync::LazyLock<String> = std::sync::LazyLock::new(|| {
    UserService::hash_password(&random_password()).expect("hashing a random password")
});

/// User service backed by a swappable data adapter.
pub struct UserService {
    data: Arc<dyn DataAdapter>,
    failures: Mutex<std::collections::HashMap<String, Failures>>,
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
            Params::new(ARGON2_MEMORY_KIB, ARGON2_PASSES, ARGON2_LANES, None)
                .expect("valid Argon2id params"),
        )
    }

    fn hash_password(password: &str) -> Result<String, PlatformError> {
        let salt = SaltString::generate(&mut OsRng);
        HASHING
            .run(|| {
                Self::argon2id()
                    .hash_password(password.as_bytes(), &salt)
                    .map(|hash| hash.to_string())
            })
            .map_err(|err| PlatformError::new("PLATFORM_PASSWORD_HASH", err.to_string()))
    }

    /// Verifies with the settings the hash records, not the current ones, so a
    /// hash made before the settings changed still verifies.
    fn verify_argon2_password(hash: &str, password: &str) -> bool {
        let Ok(parsed) = PasswordHash::new(hash) else {
            return false;
        };
        HASHING.run(|| {
            Self::argon2id()
                .verify_password(password.as_bytes(), &parsed)
                .is_ok()
        })
    }

    /// Whether a stored hash was made the way `hash_password` makes one now.
    fn hashed_as_now(hash: &str) -> bool {
        let Ok(parsed) = PasswordHash::new(hash) else {
            return false;
        };
        let Ok(params) = Params::try_from(&parsed) else {
            return false;
        };
        parsed.algorithm == Algorithm::Argon2id.ident()
            && parsed.version == Some(Version::V0x13.into())
            && (params.m_cost(), params.t_cost(), params.p_cost())
                == (ARGON2_MEMORY_KIB, ARGON2_PASSES, ARGON2_LANES)
    }

    fn new_user_id() -> String {
        format!("usr_{}", Uuid::new_v4().simple())
    }

    /// Creates user service.
    pub fn new(data: Arc<dyn DataAdapter>) -> Self {
        Self { data, failures: Mutex::new(std::collections::HashMap::new()) }
    }

    /// Refuses an attempt for `owner` while its failures still make it wait.
    fn wait_for_turn(&self, owner: &str) -> Result<(), PlatformError> {
        let failures = self.failures.lock().unwrap_or_else(PoisonError::into_inner);
        let Some(failed) = failures.get(owner) else { return Ok(()) };
        let now = std::time::Instant::now();
        if failed.next_attempt <= now {
            return Ok(());
        }
        let wait = failed.next_attempt.duration_since(now).as_secs().max(1);
        Err(PlatformError::new(
            "PLATFORM_LOGIN_THROTTLED",
            format!("too many failed sign-ins for this name; try again in {wait}s"),
        ))
    }

    /// Remembers how an attempt for `owner` went: a success forgets its
    /// failures, a failure past the free ones pushes the next attempt back.
    fn note_attempt(&self, owner: &str, ok: bool) {
        let mut failures = self.failures.lock().unwrap_or_else(PoisonError::into_inner);
        if ok {
            failures.remove(owner);
            return;
        }
        let now = std::time::Instant::now();
        if failures.len() >= FAILURES_KEPT && !failures.contains_key(owner) {
            failures.retain(|_, failed| failed.next_attempt > now);
            if failures.len() >= FAILURES_KEPT
                && let Some(soonest) = failures.iter().min_by_key(|(_, failed)| failed.next_attempt).map(|(name, _)| name.clone())
            {
                failures.remove(&soonest);
            }
        }
        let failed = failures.entry(owner.to_string()).or_insert(Failures { count: 0, next_attempt: now });
        failed.count += 1;
        if failed.count > FREE_FAILURES {
            let doublings = (failed.count - FREE_FAILURES - 1).min(16);
            failed.next_attempt = now + FIRST_WAIT.saturating_mul(1 << doublings).min(LONGEST_WAIT);
        }
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
                // A password that arrives through a create/update request was
                // named by whoever made the request. First-boot bootstrap
                // downgrades this to `generated` right after creating the
                // account, via `mark_credential_generated`.
                credential_state: CREDENTIAL_STATE_CHOSEN.to_string(),
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
    ///
    /// A name with failed attempts behind it waits its turn
    /// (`PLATFORM_LOGIN_THROTTLED`), and a name with no account costs one
    /// check like any other, so neither the answer nor its time says which
    /// names exist.
    pub fn authenticate(&self, owner: &str, password: &str) -> Result<bool, PlatformError> {
        let owner = slug_segment(owner);
        if owner.is_empty() {
            return Ok(false);
        }
        self.wait_for_turn(&owner)?;
        let ok = self.check_password(&owner, password)?;
        self.note_attempt(&owner, ok);
        Ok(ok)
    }

    fn check_password(&self, owner: &str, password: &str) -> Result<bool, PlatformError> {
        let Some(found) = self.data.get_user_auth(owner)? else {
            let _ = Self::verify_argon2_password(&NO_ACCOUNT, password);
            return Ok(false);
        };
        match found.auth.password_alg.as_str() {
            "argon2id" => {
                let ok = Self::verify_argon2_password(&found.auth.password_hash, password);
                if ok && !Self::hashed_as_now(&found.auth.password_hash) {
                    // The password is unchanged; only how it is stored moves
                    // to the current settings. A failed re-hash keeps the old
                    // one, which still verifies, so the login stands.
                    if let Ok(hash) = Self::hash_password(password) {
                        let mut rehashed = found.clone();
                        rehashed.auth.password_hash = hash;
                        let _ = self.data.put_user(&rehashed);
                    }
                }
                Ok(ok)
            }
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

    /// Marks the account's current password as platform-generated.
    ///
    /// First-boot bootstrap calls this after creating the account with a
    /// random password, so browser logins are forced to choose one.
    pub fn mark_credential_generated(&self, owner: &str) -> Result<(), PlatformError> {
        let mut stored = self.require_stored(owner)?;
        stored.auth.credential_state = CREDENTIAL_STATE_GENERATED.to_string();
        self.data.put_user(&stored)
    }

    /// Whether browser logins for this account must change the password first.
    pub fn must_change_password(&self, owner: &str) -> Result<bool, PlatformError> {
        let owner = slug_segment(owner);
        Ok(self
            .data
            .get_user_auth(&owner)?
            .map(|stored| stored.auth.credential_state == CREDENTIAL_STATE_GENERATED)
            .unwrap_or(false))
    }

    /// Replaces the account's password after re-verifying the current one.
    ///
    /// The new password is hashed with the same scheme `authenticate` verifies
    /// (argon2id), refused when it is on the insecure-value list the boot
    /// guard refuses, and the credential is marked operator-chosen.
    pub fn change_password(
        &self,
        owner: &str,
        current_password: &str,
        new_password: &str,
    ) -> Result<(), PlatformError> {
        if new_password.trim().is_empty() {
            return Err(PlatformError::new(
                "PLATFORM_PASSWORD_INVALID",
                "new password must not be empty",
            ));
        }
        if is_insecure_password(new_password) {
            return Err(PlatformError::new(
                "PLATFORM_PASSWORD_INSECURE",
                "refusing a well-known insecure password; choose a different one",
            ));
        }
        if !self.authenticate(owner, current_password)? {
            return Err(PlatformError::new(
                "PLATFORM_PASSWORD_INCORRECT",
                "current password is incorrect",
            ));
        }
        let mut stored = self.require_stored(owner)?;
        stored.auth.password_hash = Self::hash_password(new_password)?;
        stored.auth.password_alg = "argon2id".to_string();
        stored.auth.password_updated_at = now_ts();
        stored.auth.credential_state = CREDENTIAL_STATE_CHOSEN.to_string();
        self.data.put_user(&stored)
    }

    /// Offline recovery: replaces the password with a fresh random one.
    ///
    /// Returns the cleartext exactly once, for the caller to print. The
    /// credential is marked `generated`, so the next browser login is forced
    /// through the change-password screen again.
    pub fn reset_password_generated(&self, owner: &str) -> Result<String, PlatformError> {
        let mut stored = self.require_stored(owner)?;
        let password = random_password();
        stored.auth.password_hash = Self::hash_password(&password)?;
        stored.auth.password_alg = "argon2id".to_string();
        stored.auth.password_updated_at = now_ts();
        stored.auth.credential_state = CREDENTIAL_STATE_GENERATED.to_string();
        self.data.put_user(&stored)?;
        Ok(password)
    }

    fn require_stored(&self, owner: &str) -> Result<StoredUser, PlatformError> {
        let owner = slug_segment(owner);
        self.data.get_user_auth(&owner)?.ok_or_else(|| {
            PlatformError::new(
                "PLATFORM_USER_NOT_FOUND",
                format!("user '{owner}' not found"),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::adapters::data::build_data_adapter;
    use crate::platform::model::DataAdapterKind;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::time::{Duration, Instant};

    const PASSWORD: &str = "<password>";

    /// A hash as the earlier settings made it: 64 MiB, 3 passes, 4 lanes.
    fn earlier_hash(password: &str) -> String {
        let salt = SaltString::generate(&mut OsRng);
        Argon2::new(
            Algorithm::Argon2id,
            Version::V0x13,
            Params::new(65_536, 3, 4, None).expect("params"),
        )
        .hash_password(password.as_bytes(), &salt)
        .expect("hash")
        .to_string()
    }

    fn users() -> (tempfile::TempDir, UserService) {
        let dir = tempfile::tempdir().expect("tempdir");
        let data = build_data_adapter(DataAdapterKind::Sqlite, dir.path()).expect("data");
        (dir, UserService::new(data))
    }

    #[test]
    fn failed_sign_ins_past_the_free_ones_make_the_next_wait_for_any_name() {
        let (_dir, users) = users();
        for name in ["demo", "no-such-account"] {
            for _ in 0..=FREE_FAILURES {
                assert!(!users.authenticate(name, "<wrong>").expect("refused, not throttled"));
            }
            let throttled = users.authenticate(name, "<wrong>").expect_err("throttled");
            assert_eq!(throttled.code, "PLATFORM_LOGIN_THROTTLED", "{name}");
        }
        // A success forgets them.
        users.note_attempt("demo", true);
        assert!(users.wait_for_turn("demo").is_ok());
    }

    #[test]
    fn the_failures_remembered_stay_bounded() {
        let (_dir, users) = users();
        for n in 0..FAILURES_KEPT + 50 {
            for _ in 0..=FREE_FAILURES {
                users.note_attempt(&format!("name-{n}"), false);
            }
        }
        assert!(users.failures.lock().expect("lock").len() <= FAILURES_KEPT);
    }

    #[test]
    fn a_hash_made_with_the_earlier_settings_still_verifies() {
        let earlier = earlier_hash(PASSWORD);
        assert!(!UserService::hashed_as_now(&earlier));
        assert!(UserService::verify_argon2_password(&earlier, PASSWORD));
        assert!(!UserService::verify_argon2_password(&earlier, "<other>"));
        assert!(UserService::hashed_as_now(
            &UserService::hash_password(PASSWORD).expect("hash")
        ));
    }

    #[test]
    fn a_login_moves_an_earlier_hash_to_the_current_settings() {
        let (_dir, users) = users();
        users
            .create_user(&CreateUserRequest {
                owner: "demo".to_string(),
                password: PASSWORD.to_string(),
                role: "member".to_string(),
                git_name: String::new(),
                git_email: String::new(),
            })
            .expect("create");
        let mut stored = users.data.get_user_auth("demo").expect("read").expect("user");
        stored.auth.password_hash = earlier_hash(PASSWORD);
        users.data.put_user(&stored).expect("write");

        assert!(!users.authenticate("demo", "<other>").expect("auth"));
        let unchanged = users.data.get_user_auth("demo").expect("read").expect("user");
        assert_eq!(unchanged.auth.password_hash, stored.auth.password_hash);

        assert!(users.authenticate("demo", PASSWORD).expect("auth"));
        let moved = users.data.get_user_auth("demo").expect("read").expect("user");
        assert!(UserService::hashed_as_now(&moved.auth.password_hash));
        assert!(users.authenticate("demo", PASSWORD).expect("auth"));
    }

    #[test]
    fn the_gate_lets_no_more_than_its_limit_in_at_once() {
        let gate = Arc::new(HashingGate::new(HASHING_AT_ONCE));
        let inside = Arc::new(AtomicUsize::new(0));
        let most = Arc::new(AtomicUsize::new(0));
        let open = Arc::new(AtomicBool::new(false));
        let callers: Vec<_> = (0..HASHING_AT_ONCE + 3)
            .map(|_| {
                let (gate, inside, most, open) =
                    (gate.clone(), inside.clone(), most.clone(), open.clone());
                std::thread::spawn(move || {
                    gate.run(|| {
                        most.fetch_max(inside.fetch_add(1, Ordering::SeqCst) + 1, Ordering::SeqCst);
                        while !open.load(Ordering::SeqCst) {
                            std::thread::sleep(Duration::from_millis(1));
                        }
                        inside.fetch_sub(1, Ordering::SeqCst);
                    })
                })
            })
            .collect();

        let until = Instant::now() + Duration::from_secs(10);
        while inside.load(Ordering::SeqCst) < HASHING_AT_ONCE && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(1));
        }
        // The rest are waiting, not inside: give them time to get in wrongly.
        std::thread::sleep(Duration::from_millis(100));
        assert_eq!(inside.load(Ordering::SeqCst), HASHING_AT_ONCE);

        open.store(true, Ordering::SeqCst);
        for caller in callers {
            caller.join().expect("caller");
        }
        assert_eq!(most.load(Ordering::SeqCst), HASHING_AT_ONCE);
        assert_eq!(*gate.running.lock().expect("running"), 0);
    }
}
