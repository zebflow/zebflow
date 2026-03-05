//! MCP session management service.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    McpSession, McpSessionResponse, ProjectCapability, ProjectPolicy, ProjectPolicyBinding,
    ProjectSubjectKind, now_ts,
};

/// In-memory MCP session store (tokens valid until revoked or server restart).
#[derive(Clone)]
pub struct McpSessionService {
    sessions: Arc<Mutex<HashMap<String, McpSession>>>,
    project_tokens: Arc<Mutex<HashMap<(String, String), String>>>,
    data: Arc<dyn DataAdapter>,
}

impl McpSessionService {
    pub fn new(data: Arc<dyn DataAdapter>) -> Self {
        let sessions_map = Arc::new(Mutex::new(HashMap::new()));
        let project_tokens_map = Arc::new(Mutex::new(HashMap::new()));

        // Reload persisted sessions on startup
        if let Ok(persisted) = data.list_all_mcp_sessions() {
            let now = now_ts() as u64;
            let mut sessions = sessions_map.lock().unwrap_or_else(|e| e.into_inner());
            let mut project_tokens =
                project_tokens_map.lock().unwrap_or_else(|e| e.into_inner());
            for session in persisted {
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
        }
    }

    /// Create a new MCP session for a project; revokes any existing session for that project.
    pub fn create(
        &self,
        owner: &str,
        project: &str,
        capabilities: Vec<ProjectCapability>,
        base_url: &str,
        auto_reset_seconds: Option<u64>,
    ) -> Result<McpSessionResponse, PlatformError> {
        let token = Self::generate_token();
        let mcp_url = format!(
            "{}/api/projects/{}/{}/mcp",
            base_url.trim_end_matches('/'),
            owner,
            project
        );

        let session = McpSession {
            owner: owner.to_string(),
            project: project.to_string(),
            capabilities: capabilities.clone(),
            token: token.clone(),
            created_at: now_ts(),
            auto_reset_seconds,
        };

        let mut sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        let mut project_tokens = self.project_tokens.lock().unwrap_or_else(|e| e.into_inner());

        let key = (owner.to_string(), project.to_string());
        if let Some(old_token) = project_tokens.get(&key).cloned() {
            sessions.remove(&old_token);
            let _ = self.revoke_policy_binding(owner, project, &old_token);
            let _ = self.data.delete_mcp_session(&old_token);
        }

        sessions.insert(token.clone(), session.clone());
        project_tokens.insert(key, token.clone());

        // Persist the session
        let _ = self.data.put_mcp_session(&session);

        self.create_session_policy_and_binding(owner, project, &token, &capabilities)?;

        Ok(McpSessionResponse {
            token,
            mcp_url,
            capabilities: capabilities.iter().map(|c| c.key().to_string()).collect(),
        })
    }

    fn create_session_policy_and_binding(
        &self,
        owner: &str,
        project: &str,
        token: &str,
        capabilities: &[ProjectCapability],
    ) -> Result<(), PlatformError> {
        let now = now_ts();
        let policy_id = format!("mcp.session.{}", &token[..8]);

        let policy = ProjectPolicy {
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
            let mut project_tokens = self.project_tokens.lock().unwrap_or_else(|e| e.into_inner());
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
        let mut project_tokens = self.project_tokens.lock().unwrap_or_else(|e| e.into_inner());
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

    /// Look up a session by token. Returns None if expired (and revokes it).
    pub fn lookup(&self, token: &str) -> Option<McpSession> {
        let session = {
            let sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
            sessions.get(token).cloned()?
        };
        if let Some(secs) = session.auto_reset_seconds {
            let now = now_ts() as u64;
            let age = now.saturating_sub(session.created_at.max(0) as u64);
            if age >= secs {
                let _ = self.revoke_by_token(token);
                return None;
            }
        }
        Some(session)
    }

    /// Get current session for a project (if any).
    pub fn get_for_project(&self, owner: &str, project: &str) -> Option<McpSession> {
        let key = (owner.to_string(), project.to_string());
        let project_tokens = self.project_tokens.lock().unwrap_or_else(|e| e.into_inner());
        let token = project_tokens.get(&key)?;
        let sessions = self.sessions.lock().unwrap_or_else(|e| e.into_inner());
        sessions.get(token).cloned()
    }

    fn generate_token() -> String {
        let bytes: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();
        hex::encode(bytes)
    }
}
