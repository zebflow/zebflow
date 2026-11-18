//! MCP keys (`project-access` § MCP keys): any number per project, each acting
//! as one person, each stored as the SHA-256 of its secret.

use std::collections::{BTreeSet, HashMap};
use std::sync::{Arc, Mutex};

use sha2::{Digest, Sha256};

use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    McpKey, McpKeyCreateRequest, McpKeyCreated, McpKeyPerson, McpKeyView, ProjectAccessSubject,
    ProjectCapability, now_ts, slug_segment,
};
use crate::platform::services::AuthorizationService;

/// A secret's visible prefix, so a person or a secret scanner can tell what it is.
const SECRET_PREFIX: &str = "zfk_";
/// `last_used_at` is written at most this often.
const TOUCH_EVERY_SECS: i64 = 60;
/// The longest name a key keeps.
const NAME_MAX: usize = 60;
/// The longest a key may be made to last, when it is given an end at all.
const EXPIRES_MAX_DAYS: u32 = 3650;

fn mcp_url_for(base_url: &str, owner: &str, project: &str) -> String {
    format!(
        "{}/api/projects/{}/{}/mcp",
        base_url.trim_end_matches('/'),
        owner,
        project
    )
}

fn hash_secret(secret: &str) -> String {
    hex::encode(Sha256::digest(secret.as_bytes()))
}

fn random_hex(bytes: usize) -> String {
    hex::encode((0..bytes).map(|_| rand::random::<u8>()).collect::<Vec<u8>>())
}

fn forbidden(message: impl Into<String>) -> PlatformError {
    PlatformError::new("PLATFORM_AUTHZ_FORBIDDEN", message)
}

fn invalid(message: impl Into<String>) -> PlatformError {
    PlatformError::new("INVALID_REQUEST", message)
}

/// The keys, held in memory by secret hash and written through to the catalog.
#[derive(Clone)]
pub struct McpKeyService {
    keys: Arc<Mutex<HashMap<String, McpKey>>>,
    data: Arc<dyn DataAdapter>,
    authz: Arc<AuthorizationService>,
    min_created_at: i64,
}

impl McpKeyService {
    pub fn new(
        data: Arc<dyn DataAdapter>,
        authz: Arc<AuthorizationService>,
        min_created_at: i64,
    ) -> Self {
        let mut keys = HashMap::new();
        if let Ok(stored) = data.list_all_mcp_keys() {
            let now = now_ts();
            for key in stored {
                if Self::lapsed(&key, min_created_at, now) {
                    let _ = data.delete_mcp_key(&key.key_id);
                    continue;
                }
                keys.insert(key.token_hash.clone(), key);
            }
        }
        Self {
            keys: Arc::new(Mutex::new(keys)),
            data,
            authz,
            min_created_at,
        }
    }

    /// Made before the rotation epoch, or past its end.
    fn lapsed(key: &McpKey, min_created_at: i64, now: i64) -> bool {
        (min_created_at > 0 && key.created_at < min_created_at)
            || key.expires_at.is_some_and(|at| now >= at)
    }

    fn rights_of(
        &self,
        person: &str,
        owner: &str,
        project: &str,
    ) -> Result<BTreeSet<ProjectCapability>, PlatformError> {
        self.authz
            .resolve_project_capabilities(&ProjectAccessSubject::user(person), owner, project)
    }

    /// Whether `person` holds every one of `capabilities` on the project, within
    /// their MCP ceiling. A refusal names the first right missing.
    fn person_holds(
        &self,
        owner: &str,
        project: &str,
        person: &str,
        capabilities: &[ProjectCapability],
    ) -> Result<(), PlatformError> {
        let rights = self.rights_of(person, owner, project)?;
        if let Some(missing) = capabilities.iter().find(|c| !rights.contains(c)) {
            return Err(forbidden(format!(
                "'{person}' does not hold '{}' on {owner}/{project}",
                missing.key()
            )));
        }
        let is_superadmin = self
            .data
            .get_user_auth(person)?
            .is_some_and(|u| u.profile.role == "superadmin");
        if person != owner
            && !is_superadmin
            && let Some(member) = self.data.get_project_member(owner, project, person)?
            && !member.mcp_capability_ceiling.is_empty()
            && let Some(above) = capabilities
                .iter()
                .find(|c| !member.mcp_capability_ceiling.contains(c))
        {
            return Err(forbidden(format!(
                "'{}' is above the MCP ceiling of '{person}'",
                above.key()
            )));
        }
        Ok(())
    }

    /// Make a key. `maker` needs `mcp.session.create`, `members.write` to make
    /// one for someone else, and every right it lists; the person must hold
    /// them too. The secret in the answer is never available again.
    pub fn make(
        &self,
        owner: &str,
        project: &str,
        maker: &str,
        req: &McpKeyCreateRequest,
        base_url: &str,
    ) -> Result<McpKeyCreated, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let maker = slug_segment(maker);
        if maker.is_empty() {
            return Err(forbidden("a key needs a maker"));
        }
        let person = req
            .person
            .as_deref()
            .map(slug_segment)
            .filter(|p| !p.is_empty())
            .unwrap_or_else(|| maker.clone());

        let mut capabilities = Vec::new();
        for raw in &req.capabilities {
            let capability = ProjectCapability::from_key(raw.trim())
                .ok_or_else(|| invalid(format!("unknown capability '{raw}'")))?;
            if !capabilities.contains(&capability) {
                capabilities.push(capability);
            }
        }
        if capabilities.is_empty() {
            return Err(invalid("a key needs at least one capability"));
        }
        let name = req.name.trim();
        let name = if name.is_empty() {
            "key".to_string()
        } else {
            name.chars().take(NAME_MAX).collect()
        };
        let expires_at = match req.expires_in_days {
            None => None,
            Some(days) if (1..=EXPIRES_MAX_DAYS).contains(&days) => {
                Some(now_ts() + i64::from(days) * 86_400)
            }
            Some(_) => {
                return Err(invalid(format!(
                    "expires_in_days must be 1 to {EXPIRES_MAX_DAYS}"
                )));
            }
        };

        let maker_rights = self.rights_of(&maker, &owner, &project)?;
        if !maker_rights.contains(&ProjectCapability::McpSessionCreate) {
            return Err(forbidden(format!(
                "'{maker}' may not make MCP keys on {owner}/{project}"
            )));
        }
        if person != maker && !maker_rights.contains(&ProjectCapability::MembersWrite) {
            return Err(forbidden(format!(
                "'{maker}' may not make a key for another person on {owner}/{project}"
            )));
        }
        if let Some(missing) = capabilities.iter().find(|c| !maker_rights.contains(c)) {
            return Err(forbidden(format!(
                "'{maker}' cannot give '{}', which they do not hold",
                missing.key()
            )));
        }
        self.person_holds(&owner, &project, &person, &capabilities)?;

        let secret = format!("{SECRET_PREFIX}{}", random_hex(32));
        let key = McpKey {
            key_id: format!("k_{}", random_hex(6)),
            owner,
            project,
            person,
            name,
            capabilities,
            token_hash: hash_secret(&secret),
            made_by: maker,
            created_at: now_ts(),
            expires_at,
            last_used_at: None,
        };
        self.data.put_mcp_key(&key)?;
        let created = McpKeyCreated {
            key: Self::view(&key),
            secret,
            mcp_url: mcp_url_for(base_url, &key.owner, &key.project),
        };
        self.keys
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .insert(key.token_hash.clone(), key);
        Ok(created)
    }

    /// A key as listed: everything but its hash.
    pub fn view(key: &McpKey) -> McpKeyView {
        McpKeyView {
            key_id: key.key_id.clone(),
            name: key.name.clone(),
            person: key.person.clone(),
            capabilities: key.capabilities.iter().map(|c| c.key().to_string()).collect(),
            made_by: key.made_by.clone(),
            created_at: key.created_at,
            expires_at: key.expires_at,
            last_used_at: key.last_used_at,
        }
    }

    /// Every key of a project, oldest first.
    pub fn list(&self, owner: &str, project: &str) -> Vec<McpKeyView> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let mut keys: Vec<McpKey> = self
            .keys
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .filter(|k| k.owner == owner && k.project == project)
            .cloned()
            .collect();
        keys.sort_by(|a, b| a.created_at.cmp(&b.created_at).then_with(|| a.key_id.cmp(&b.key_id)));
        keys.iter().map(Self::view).collect()
    }

    /// Who a key on this project can act as, with what each holds: the
    /// caller first, then the owner, then every member. Someone holding
    /// nothing is left out.
    pub fn people(
        &self,
        owner: &str,
        project: &str,
        caller: &str,
    ) -> Result<Vec<McpKeyPerson>, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let caller = slug_segment(caller);
        let members = self.data.list_project_members(&owner, &project)?;
        let mut names = vec![caller, owner.clone()];
        names.extend(members.iter().map(|m| slug_segment(&m.user_id)));
        let mut out: Vec<McpKeyPerson> = Vec::new();
        for name in names {
            if name.is_empty() || out.iter().any(|p| p.person == name) {
                continue;
            }
            let rights = self.rights_of(&name, &owner, &project)?;
            if rights.is_empty() {
                continue;
            }
            let role = if name == owner {
                "owner".to_string()
            } else if let Some(member) = members.iter().find(|m| slug_segment(&m.user_id) == name) {
                member.role_preset.key().to_string()
            } else {
                "superadmin".to_string()
            };
            out.push(McpKeyPerson {
                person: name,
                role,
                rights: rights.iter().map(|c| c.key().to_string()).collect(),
            });
        }
        Ok(out)
    }

    /// Revoke one key: `actor` holds `mcp.session.revoke`, or is its person.
    pub fn revoke(
        &self,
        owner: &str,
        project: &str,
        key_id: &str,
        actor: &str,
    ) -> Result<(), PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let actor = slug_segment(actor);
        let key = self
            .keys
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .find(|k| k.owner == owner && k.project == project && k.key_id == key_id)
            .cloned()
            .ok_or_else(|| PlatformError::new("MCP_KEY_NOT_FOUND", "no such MCP key in this project"))?;
        if key.person != actor
            && !self
                .rights_of(&actor, &owner, &project)?
                .contains(&ProjectCapability::McpSessionRevoke)
        {
            return Err(forbidden(format!(
                "'{actor}' may not revoke a key of '{}'",
                key.person
            )));
        }
        self.drop_key(&key);
        Ok(())
    }

    fn drop_key(&self, key: &McpKey) {
        self.keys
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&key.token_hash);
        let _ = self.data.delete_mcp_key(&key.key_id);
    }

    fn drop_where(&self, gone: impl Fn(&McpKey) -> bool) {
        let doomed: Vec<McpKey> = self
            .keys
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .values()
            .filter(|k| gone(k))
            .cloned()
            .collect();
        for key in doomed {
            self.drop_key(&key);
        }
    }

    /// Take a project's keys (deleted, renamed, transferred), or only one
    /// person's keys in it (removed from it).
    pub fn forget(&self, owner: &str, project: &str, person: Option<&str>) {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let person = person.map(slug_segment);
        self.drop_where(|k| {
            k.owner == owner && k.project == project && person.as_ref().is_none_or(|p| &k.person == p)
        });
    }

    /// Take every key that acts as `person` or sits in their namespace
    /// (the account deleted or renamed).
    pub fn forget_person(&self, person: &str) {
        let person = slug_segment(person);
        self.drop_where(|k| k.person == person || k.owner == person);
    }

    /// The key a secret opens, while it may: not lapsed, and its person still
    /// holds every right it carries. A key whose person lost one is revoked
    /// here; a store that cannot answer denies without revoking.
    pub fn lookup(&self, secret: &str) -> Option<McpKey> {
        let secret = secret.trim();
        if !secret.starts_with(SECRET_PREFIX) {
            return None;
        }
        let key = self
            .keys
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get(&hash_secret(secret))
            .cloned()?;
        let now = now_ts();
        if Self::lapsed(&key, self.min_created_at, now) {
            self.drop_key(&key);
            return None;
        }
        match self.person_holds(&key.owner, &key.project, &key.person, &key.capabilities) {
            Ok(()) => Some(self.touch(key, now)),
            Err(err) if err.code == "PLATFORM_AUTHZ_FORBIDDEN" => {
                self.drop_key(&key);
                None
            }
            Err(_) => None,
        }
    }

    fn touch(&self, mut key: McpKey, now: i64) -> McpKey {
        if key.last_used_at.is_some_and(|at| now - at < TOUCH_EVERY_SECS) {
            return key;
        }
        key.last_used_at = Some(now);
        if let Some(held) = self
            .keys
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .get_mut(&key.token_hash)
        {
            held.last_used_at = Some(now);
        }
        let _ = self.data.put_mcp_key(&key);
        key
    }

    /// Global rotation cutoff: keys made before it no longer open anything.
    pub fn min_created_at(&self) -> i64 {
        self.min_created_at
    }
}

#[cfg(test)]
mod tests {
    use crate::platform::model::{
        CreateUserRequest, McpKeyCreateRequest, PlatformConfig, ProjectAccessRolePreset,
        ProjectCapability as C, UpsertProjectMemberRequest,
    };
    use crate::platform::services::PlatformService;

    fn platform() -> (tempfile::TempDir, PlatformService) {
        let dir = tempfile::tempdir().expect("tempdir");
        let platform = PlatformService::from_config(PlatformConfig {
            data_root: dir.path().to_path_buf(),
            ..PlatformConfig::default()
        })
        .expect("platform");
        (dir, platform)
    }

    fn person(platform: &PlatformService, name: &str) {
        platform
            .users
            .create_user(&CreateUserRequest {
                owner: name.to_string(),
                password: uuid::Uuid::new_v4().to_string(),
                role: "member".to_string(),
                git_name: String::new(),
                git_email: String::new(),
            })
            .expect("create person");
    }

    fn set_role(platform: &PlatformService, name: &str, role: ProjectAccessRolePreset, ceiling: Vec<C>) {
        platform
            .project_members
            .upsert_member(
                "superadmin",
                "superadmin",
                "default",
                &UpsertProjectMemberRequest {
                    user_id: name.to_string(),
                    role_preset: role,
                    custom_policy_ids: Vec::new(),
                    mcp_capability_ceiling: ceiling,
                },
            )
            .expect("owner edits member");
    }

    fn ask(person: Option<&str>, caps: &[&str]) -> McpKeyCreateRequest {
        McpKeyCreateRequest {
            name: "laptop".to_string(),
            person: person.map(str::to_string),
            capabilities: caps.iter().map(|c| c.to_string()).collect(),
            expires_in_days: None,
        }
    }

    #[test]
    fn every_capability_key_names_its_capability() {
        // A key's capabilities arrive as keys; one that does not parse back
        // would be refused as unknown.
        for capability in C::all() {
            assert_eq!(C::from_key(capability.key()), Some(capability), "{}", capability.key());
        }
    }

    #[test]
    fn a_key_acts_as_its_person_and_never_above_them() {
        let (_dir, platform) = platform();
        person(&platform, "dev");
        set_role(&platform, "dev", ProjectAccessRolePreset::Developer, Vec::new());
        let keys = &platform.mcp_keys;

        // The owner makes one for a developer; it carries the developer as its person.
        let made = keys
            .make("superadmin", "default", "superadmin", &ask(Some("dev"), &["pipelines.write", "files.write"]), "http://example.com")
            .expect("key for dev");
        assert!(made.secret.starts_with("zfk_") && made.secret.len() == 4 + 64);
        assert_eq!(made.mcp_url, "http://example.com/api/projects/superadmin/default/mcp");
        let key = keys.lookup(&made.secret).expect("the secret opens it");
        assert_eq!(key.person, "dev");
        assert_eq!(key.made_by, "superadmin");
        assert_ne!(key.token_hash, made.secret, "only the hash is kept");

        // Not above the person: a developer holds no credentials.
        let err = keys
            .make("superadmin", "default", "superadmin", &ask(Some("dev"), &["credentials.read"]), "http://example.com")
            .expect_err("above the developer");
        assert_eq!(err.code, "PLATFORM_AUTHZ_FORBIDDEN");

        // A developer cannot make keys at all, for themselves or anyone.
        assert!(keys.make("superadmin", "default", "dev", &ask(None, &["pipelines.read"]), "http://example.com").is_err());

        // Unknown or empty rights are refused, not dropped.
        assert_eq!(keys.make("superadmin", "default", "superadmin", &ask(None, &["pipelines.fly"]), "x").expect_err("unknown").code, "INVALID_REQUEST");
        assert_eq!(keys.make("superadmin", "default", "superadmin", &ask(None, &[]), "x").expect_err("empty").code, "INVALID_REQUEST");

        // A secret without the prefix, or a wrong one, opens nothing.
        assert!(keys.lookup(made.secret.trim_start_matches("zfk_")).is_none());
        assert!(keys.lookup("zfk_0000").is_none());
    }

    #[test]
    fn a_maintainer_needs_members_write_and_every_right_to_make_one_for_someone_else() {
        let (_dir, platform) = platform();
        person(&platform, "maint");
        person(&platform, "dev");
        set_role(&platform, "maint", ProjectAccessRolePreset::Maintainer, Vec::new());
        set_role(&platform, "dev", ProjectAccessRolePreset::Developer, Vec::new());
        let keys = &platform.mcp_keys;

        assert!(keys.make("superadmin", "default", "maint", &ask(Some("dev"), &["pipelines.write"]), "x").is_ok());
        // `project.delete` is the owner's; a maintainer cannot hand it out.
        assert!(keys.make("superadmin", "default", "maint", &ask(None, &["project.delete"]), "x").is_err());
        // Someone with no rights on the project gets no key.
        person(&platform, "stranger");
        assert!(keys.make("superadmin", "default", "superadmin", &ask(Some("stranger"), &["pipelines.read"]), "x").is_err());
    }

    #[test]
    fn a_key_ends_when_its_person_loses_a_right_it_carries() {
        let (_dir, platform) = platform();
        person(&platform, "dev");
        let keys = &platform.mcp_keys;
        let mint = || {
            keys.make("superadmin", "default", "superadmin", &ask(Some("dev"), &["pipelines.read", "pipelines.write"]), "x")
                .expect("key")
                .secret
        };

        // Removed from the project.
        set_role(&platform, "dev", ProjectAccessRolePreset::Developer, Vec::new());
        let secret = mint();
        assert!(keys.lookup(&secret).is_some(), "a developer's key works");
        platform
            .project_members
            .remove_member("superadmin", "superadmin", "default", "dev")
            .expect("remove dev");
        assert!(keys.lookup(&secret).is_none(), "removal ends the key");
        assert!(keys.list("superadmin", "default").is_empty(), "and it is gone");

        // Demoted below a right the key carries.
        set_role(&platform, "dev", ProjectAccessRolePreset::Developer, Vec::new());
        let secret = mint();
        set_role(&platform, "dev", ProjectAccessRolePreset::Guest, Vec::new());
        assert!(keys.lookup(&secret).is_none(), "demotion ends the key");

        // A narrower MCP ceiling.
        set_role(&platform, "dev", ProjectAccessRolePreset::Developer, Vec::new());
        let secret = mint();
        set_role(&platform, "dev", ProjectAccessRolePreset::Developer, vec![C::PipelinesRead]);
        assert!(keys.lookup(&secret).is_none(), "a narrowed ceiling ends the key");
    }

    #[test]
    fn revoking_one_key_leaves_the_others_and_only_the_right_people_may() {
        let (_dir, platform) = platform();
        person(&platform, "dev");
        person(&platform, "maint");
        set_role(&platform, "dev", ProjectAccessRolePreset::Developer, Vec::new());
        set_role(&platform, "maint", ProjectAccessRolePreset::Maintainer, Vec::new());
        let keys = &platform.mcp_keys;
        let for_dev = keys.make("superadmin", "default", "superadmin", &ask(Some("dev"), &["pipelines.read"]), "x").expect("dev key");
        let own = keys.make("superadmin", "default", "superadmin", &ask(None, &["pipelines.read"]), "x").expect("own key");
        assert_eq!(keys.list("superadmin", "default").len(), 2);

        // The developer may revoke their own key, not the owner's.
        assert!(keys.revoke("superadmin", "default", &own.key.key_id, "dev").is_err());
        keys.revoke("superadmin", "default", &for_dev.key.key_id, "dev").expect("own key");
        assert!(keys.lookup(&for_dev.secret).is_none());
        assert!(keys.lookup(&own.secret).is_some(), "the other key still works");

        // A maintainer may revoke anyone's.
        keys.revoke("superadmin", "default", &own.key.key_id, "maint").expect("maintainer revokes");
        assert!(keys.lookup(&own.secret).is_none());
        assert_eq!(keys.revoke("superadmin", "default", "k_missing", "superadmin").expect_err("missing").code, "MCP_KEY_NOT_FOUND");
    }

    #[test]
    fn a_key_survives_a_restart_and_lapses_at_its_end() {
        let dir = tempfile::tempdir().expect("tempdir");
        let config = || PlatformConfig {
            data_root: dir.path().to_path_buf(),
            ..PlatformConfig::default()
        };
        let platform = PlatformService::from_config(config()).expect("platform");
        let made = platform
            .mcp_keys
            .make("superadmin", "default", "superadmin", &ask(None, &["pipelines.read"]), "x")
            .expect("key");
        drop(platform);
        let platform = PlatformService::from_config(config()).expect("platform again");
        assert!(platform.mcp_keys.lookup(&made.secret).is_some(), "the stored hash still opens it");

        let mut short = ask(None, &["pipelines.read"]);
        short.expires_in_days = Some(0);
        assert_eq!(platform.mcp_keys.make("superadmin", "default", "superadmin", &short, "x").expect_err("zero days").code, "INVALID_REQUEST");
    }
}
