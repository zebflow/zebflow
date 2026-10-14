//! MCP session management service.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    McpSession, McpSessionResponse, ProjectAccessSubject, ProjectCapability, ProjectPolicy,
    ProjectPolicyBinding, ProjectSubjectKind, now_ts, slug_segment,
};
use crate::platform::services::AuthorizationService;

fn mcp_url_for(base_url: &str, owner: &str, project: &str) -> String {
    format!(
        "{}/api/projects/{}/{}/mcp",
        base_url.trim_end_matches('/'),
        owner,
        project
    )
}

fn response_for(session: &McpSession, base_url: &str, min_created_at: i64) -> McpSessionResponse {
    McpSessionResponse {
        token: session.token.clone(),
        mcp_url: mcp_url_for(base_url, &session.owner, &session.project),
        capabilities: session
            .capabilities
            .iter()
            .map(|c| c.key().to_string())
            .collect(),
        enabled: session.enabled,
        created_at: session.created_at,
        auto_reset_seconds: session.auto_reset_seconds,
        rotation_epoch: min_created_at,
    }
}

/// In-memory MCP session store (tokens valid until revoked or server restart).
#[derive(Clone)]
pub struct McpSessionService {
    sessions: Arc<Mutex<HashMap<String, McpSession>>>,
    project_tokens: Arc<Mutex<HashMap<(String, String), String>>>,
    data: Arc<dyn DataAdapter>,
    authz: Arc<AuthorizationService>,
    min_created_at: i64,
}

impl McpSessionService {
    pub fn new(
        data: Arc<dyn DataAdapter>,
        authz: Arc<AuthorizationService>,
        min_created_at: i64,
    ) -> Self {
        let sessions_map = Arc::new(Mutex::new(HashMap::new()));
        let project_tokens_map = Arc::new(Mutex::new(HashMap::new()));

        // Reload persisted sessions on startup
        if let Ok(persisted) = data.list_all_mcp_sessions() {
            let now = now_ts() as u64;
            let mut sessions = sessions_map.lock().unwrap_or_else(|e| e.into_inner());
            let mut project_tokens = project_tokens_map.lock().unwrap_or_else(|e| e.into_inner());
            for session in persisted {
                if min_created_at > 0 && session.created_at < min_created_at {
                    let _ = data.delete_mcp_session(&session.token);
                    continue;
                }
                // Skip expired sessions
                if let Some(secs) = session.auto_reset_seconds {
                    let age = now.saturating_sub(session.created_at.max(0) as u64);
                    if age >= secs {
                        let _ = data.delete_mcp_session(&session.token);
                        continue;
                    }
                }
                let key = (session.owner.clone(), session.project.clone());
                project_tokens.insert(key, session.token.clone());
                sessions.insert(session.token.clone(), session);
            }
        }

        Self {
            sessions: sessions_map,
            project_tokens: project_tokens_map,
            data,
            authz,
            min_created_at,
        }
    }

    /// Validates that `granted_by` holds `McpSessionCreate`, holds every requested
    /// capability in `capabilities`, and satisfies any member `mcp_capability_ceiling`.
    pub fn validate_grant(
        &self,
        owner: &str,
        project: &str,
        granted_by: &str,
        capabilities: &[ProjectCapability],
    ) -> Result<(), PlatformError> {
        if capabilities.is_empty() {
            return Err(PlatformError::new(
                "INVALID_REQUEST",
                "At least one valid capability must be specified",
            ));
        }
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let granter = slug_segment(granted_by);
        if granter.is_empty() {
            return Err(PlatformError::new(
                "PLATFORM_AUTHZ_FORBIDDEN",
                "MCP session creator is required",
            ));
        }
        let subject = ProjectAccessSubject::user(&granter);
        let actor_caps = self
            .authz
            .resolve_project_capabilities(&subject, &owner, &project)?;
        if !actor_caps.contains(&ProjectCapability::McpSessionCreate) {
            return Err(PlatformError::new(
                "PLATFORM_AUTHZ_FORBIDDEN",
                format!(
                    "subject '{granter}' lacks capability '{}' for project '{owner}/{project}'",
                    ProjectCapability::McpSessionCreate.key()
                ),
            ));
        }
        for capability in capabilities {
            if !actor_caps.contains(capability) {
                return Err(PlatformError::new(
                    "PLATFORM_AUTHZ_FORBIDDEN",
                    format!(
                        "subject '{granter}' cannot grant capability '{}' to an MCP session because they do not hold it",
                        capability.key()
                    ),
                ));
            }
        }
        let is_superadmin = self
            .data
            .get_user_auth(&granter)?
            .is_some_and(|u| u.profile.role == "superadmin");
        if granter != owner
            && !is_superadmin
            && let Some(member) = self.data.get_project_member(&owner, &project, &granter)?
            && !member.mcp_capability_ceiling.is_empty()
        {
            for capability in capabilities {
                if !member.mcp_capability_ceiling.contains(capability) {
                    return Err(PlatformError::new(
                        "PLATFORM_AUTHZ_FORBIDDEN",
                        format!(
                            "capability '{}' exceeds member MCP capability ceiling",
                            capability.key()
                        ),
                    ));
                }
            }
        }
        Ok(())
    }

    /// Create or re-enable an MCP session for a project.
    ///
    /// If a session already exists for this project and was created by the same user,
    /// the token is preserved and capabilities are updated. Otherwise, a new token is generated.
    pub fn create(
        &self,
        owner: &str,
        project: &str,
        capabilities: Vec<ProjectCapability>,
        base_url: &str,
        auto_reset_seconds: Option<u64>,
        granted_by: &str,
    ) -> Result<McpSessionResponse, PlatformError> {
        self.validate_grant(owner, project, granted_by, &capabilities)?;
        let key = (owner.to_string(), project.to_string());

        let mut sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        let mut project_tokens = self
            .project_tokens
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        // If a session already exists for the same granter, keep the token; if a different
        // user is replacing the session, rotate the token so a prior user cannot inherit
        // elevated capabilities on a token they previously read.
        if let Some(existing_token) = project_tokens.get(&key).cloned() {
            let same_granter = sessions
                .get(&existing_token)
                .is_some_and(|existing| existing.granted_by == granted_by);
            if same_granter {
                if let Some(existing) = sessions.get_mut(&existing_token) {
                    existing.capabilities = capabilities.clone();
                    existing.enabled = true;
                    existing.granted_by = granted_by.to_string();
                    if auto_reset_seconds.is_some() {
                        existing.auto_reset_seconds = auto_reset_seconds;
                    }
                    let session_clone = existing.clone();
                    drop(sessions);
                    drop(project_tokens);
                    let _ = self.data.put_mcp_session(&session_clone);
                    // Update policy binding with new capabilities
                    let _ = self.revoke_policy_binding(owner, project, &existing_token);
                    self.create_session_policy_and_binding(
                        owner,
                        project,
                        &existing_token,
                        &capabilities,
                    )?;
                    return Ok(response_for(&session_clone, base_url, self.min_created_at));
                }
            } else {
                sessions.remove(&existing_token);
                project_tokens.remove(&key);
                drop(sessions);
                drop(project_tokens);
                let _ = self.revoke_policy_binding(owner, project, &existing_token);
                let _ = self.data.delete_mcp_session(&existing_token);
                sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
                project_tokens = self
                    .project_tokens
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
            }
        }

        // No existing session — generate a new token.
        let token = Self::generate_token();
        let session = McpSession {
            owner: owner.to_string(),
            project: project.to_string(),
            capabilities: capabilities.clone(),
            token: token.clone(),
            created_at: now_ts(),
            auto_reset_seconds,
            enabled: true,
            granted_by: granted_by.to_string(),
        };

        sessions.insert(token.clone(), session.clone());
        project_tokens.insert(key, token.clone());
        drop(sessions);
        drop(project_tokens);

        let _ = self.data.put_mcp_session(&session);
        self.create_session_policy_and_binding(owner, project, &token, &capabilities)?;

        Ok(response_for(&session, base_url, self.min_created_at))
    }

    /// Soft-enable or soft-disable a session without changing its token.
    pub fn set_enabled(
        &self,
        owner: &str,
        project: &str,
        enabled: bool,
        actor: &str,
    ) -> Result<(), PlatformError> {
        let key = (owner.to_string(), project.to_string());
        if enabled {
            let existing_caps = {
                let sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
                let project_tokens = self
                    .project_tokens
                    .lock()
                    .unwrap_or_else(|e| e.into_inner());
                project_tokens
                    .get(&key)
                    .and_then(|token| sessions.get(token))
                    .map(|session| session.capabilities.clone())
            };
            let Some(caps) = existing_caps else {
                return Err(PlatformError::new(
                    "MCP_SESSION_NOT_FOUND",
                    "No MCP session found for this project",
                ));
            };
            self.validate_grant(owner, project, actor, &caps)?;
        }
        let mut sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        let project_tokens = self
            .project_tokens
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(token) = project_tokens.get(&key) {
            if let Some(session) = sessions.get_mut(token) {
                session.enabled = enabled;
                if enabled {
                    session.granted_by = actor.to_string();
                }
                let session_clone = session.clone();
                drop(sessions);
                drop(project_tokens);
                let _ = self.data.put_mcp_session(&session_clone);
                return Ok(());
            }
        }
        Err(PlatformError::new(
            "MCP_SESSION_NOT_FOUND",
            "No MCP session found for this project",
        ))
    }

    /// Generate a fresh token for the existing session, revoking the old one.
    pub fn reset_token(
        &self,
        owner: &str,
        project: &str,
        base_url: &str,
        granted_by: &str,
    ) -> Result<McpSessionResponse, PlatformError> {
        let key = (owner.to_string(), project.to_string());
        let existing_caps = {
            let sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
            let project_tokens = self
                .project_tokens
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let token = project_tokens.get(&key).ok_or_else(|| {
                PlatformError::new(
                    "MCP_SESSION_NOT_FOUND",
                    "No MCP session found for this project",
                )
            })?;
            sessions
                .get(token)
                .map(|s| s.capabilities.clone())
                .ok_or_else(|| {
                    PlatformError::new("MCP_SESSION_NOT_FOUND", "Session data not found")
                })?
        };
        self.validate_grant(owner, project, granted_by, &existing_caps)?;

        let new_token = Self::generate_token();

        let mut sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        let mut project_tokens = self
            .project_tokens
            .lock()
            .unwrap_or_else(|e| e.into_inner());

        let old_token = project_tokens.get(&key).cloned().ok_or_else(|| {
            PlatformError::new(
                "MCP_SESSION_NOT_FOUND",
                "No MCP session found for this project",
            )
        })?;

        let old_session = sessions
            .remove(&old_token)
            .ok_or_else(|| PlatformError::new("MCP_SESSION_NOT_FOUND", "Session data not found"))?;

        let capabilities = old_session.capabilities.clone();
        let new_session = McpSession {
            token: new_token.clone(),
            created_at: now_ts(),
            enabled: true,
            granted_by: granted_by.to_string(),
            ..old_session
        };

        sessions.insert(new_token.clone(), new_session.clone());
        project_tokens.insert(key, new_token.clone());
        drop(sessions);
        drop(project_tokens);

        // Revoke old policy binding and delete old persisted session
        let _ = self.revoke_policy_binding(owner, project, &old_token);
        let _ = self.data.delete_mcp_session(&old_token);

        // Persist new session and create new policy binding
        let _ = self.data.put_mcp_session(&new_session);
        self.create_session_policy_and_binding(owner, project, &new_token, &capabilities)?;

        Ok(response_for(&new_session, base_url, self.min_created_at))
    }

    fn create_session_policy_and_binding(
        &self,
        owner: &str,
        project: &str,
        token: &str,
        capabilities: &[ProjectCapability],
    ) -> Result<(), PlatformError> {
        let project_row = self
            .data
            .get_project(owner, project)?
            .ok_or_else(|| PlatformError::new("PLATFORM_PROJECT_MISSING", "project not found"))?;
        let now = now_ts();
        let policy_id = format!("mcp.session.{}", &token[..8]);

        let policy = ProjectPolicy {
            project_id: project_row.project_id.clone(),
            owner: owner.to_string(),
            project: project.to_string(),
            policy_id: policy_id.clone(),
            title: format!("MCP Session {}", &token[..8]),
            capabilities: capabilities.to_vec(),
            managed: false,
            created_at: now,
            updated_at: now,
        };
        self.data.put_project_policy(&policy)?;

        let binding = ProjectPolicyBinding {
            project_id: project_row.project_id,
            owner: owner.to_string(),
            project: project.to_string(),
            subject_kind: ProjectSubjectKind::McpSession,
            subject_id: token.to_string(),
            policy_id,
            created_at: now,
            updated_at: now,
        };
        self.data.put_project_policy_binding(&binding)?;

        Ok(())
    }

    fn revoke_policy_binding(
        &self,
        owner: &str,
        project: &str,
        token: &str,
    ) -> Result<(), PlatformError> {
        let bindings = self.data.list_project_policy_bindings(owner, project)?;
        for binding in bindings {
            if binding.subject_kind == ProjectSubjectKind::McpSession && binding.subject_id == token
            {
                let policy_id = binding.policy_id.clone();
                self.data
                    .delete_project_policy_binding(owner, project, token)?;
                let _ = self.data.delete_project_policy(owner, project, &policy_id);
            }
        }
        Ok(())
    }

    /// Revoke session by token.
    pub fn revoke_by_token(&self, token: &str) -> Result<(), PlatformError> {
        let mut sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(session) = sessions.remove(token) {
            let mut project_tokens = self
                .project_tokens
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            let key = (session.owner.clone(), session.project.clone());
            project_tokens.remove(&key);
            drop(sessions);
            drop(project_tokens);
            let _ = self.revoke_policy_binding(&session.owner, &session.project, token);
            let _ = self.data.delete_mcp_session(token);
        }
        Ok(())
    }

    /// Revoke session for a specific project (if any).
    pub fn revoke_for_project(&self, owner: &str, project: &str) -> Result<(), PlatformError> {
        let key = (owner.to_string(), project.to_string());
        let mut project_tokens = self
            .project_tokens
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if let Some(token) = project_tokens.remove(&key) {
            let mut sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
            sessions.remove(&token);
            drop(sessions);
            drop(project_tokens);
            let _ = self.revoke_policy_binding(owner, project, &token);
            let _ = self.data.delete_mcp_session(&token);
        }
        Ok(())
    }

    /// Look up a session by token. Returns None if disabled or expired.
    pub fn lookup(&self, token: &str) -> Option<McpSession> {
        let session = {
            let sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
            sessions.get(token).cloned()?
        };
        // Disabled sessions are invisible to the MCP handler
        if !session.enabled {
            return None;
        }
        if self.is_rotated_or_expired(&session) {
            let _ = self.revoke_by_token(token);
            return None;
        }
        Some(session)
    }

    /// Get current session for a project (if any).
    pub fn get_for_project(&self, owner: &str, project: &str) -> Option<McpSession> {
        let key = (owner.to_string(), project.to_string());
        let token = {
            let project_tokens = self
                .project_tokens
                .lock()
                .unwrap_or_else(|e| e.into_inner());
            project_tokens.get(&key).cloned()?
        };
        let session = {
            let sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
            sessions.get(&token).cloned()?
        };
        if self.is_rotated_or_expired(&session) {
            let _ = self.revoke_by_token(&token);
            return None;
        }
        Some(session)
    }

    /// Global rotation cutoff used to invalidate older persisted sessions.
    pub fn min_created_at(&self) -> i64 {
        self.min_created_at
    }

    fn is_rotated_or_expired(&self, session: &McpSession) -> bool {
        if self.min_created_at > 0 && session.created_at < self.min_created_at {
            return true;
        }
        if let Some(secs) = session.auto_reset_seconds {
            let now = now_ts() as u64;
            let age = now.saturating_sub(session.created_at.max(0) as u64);
            if age >= secs {
                return true;
            }
        }
        false
    }

    fn generate_token() -> String {
        let bytes: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();
        hex::encode(bytes)
    }
}
