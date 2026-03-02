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
        Self {
            sessions: Arc::new(Mutex::new(HashMap::new())),
            project_tokens: Arc::new(Mutex::new(HashMap::new())),
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
        };

        let mut sessions = self.sessions.lock().unwrap();
        let mut project_tokens = self.project_tokens.lock().unwrap();

        let key = (owner.to_string(), project.to_string());
        if let Some(old_token) = project_tokens.get(&key) {
            sessions.remove(old_token);
            let _ = self.revoke_policy_binding(owner, project, old_token);
        }

        sessions.insert(token.clone(), session.clone());
        project_tokens.insert(key, token.clone());

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
        let mut sessions = self.sessions.lock().unwrap();
        if let Some(session) = sessions.remove(token) {
            let mut project_tokens = self.project_tokens.lock().unwrap();
            let key = (session.owner.clone(), session.project.clone());
            project_tokens.remove(&key);
            drop(sessions);
            drop(project_tokens);
            let _ = self.revoke_policy_binding(&session.owner, &session.project, token);
        }
        Ok(())
    }

    /// Revoke session for a specific project (if any).
    pub fn revoke_for_project(&self, owner: &str, project: &str) -> Result<(), PlatformError> {
        let key = (owner.to_string(), project.to_string());
        let mut project_tokens = self.project_tokens.lock().unwrap();
        if let Some(token) = project_tokens.remove(&key) {
            let mut sessions = self.sessions.lock().unwrap();
            sessions.remove(&token);
            drop(sessions);
            drop(project_tokens);
            let _ = self.revoke_policy_binding(owner, project, &token);
        }
        Ok(())
    }

    /// Look up a session by token.
    pub fn lookup(&self, token: &str) -> Option<McpSession> {
        let sessions = self.sessions.lock().unwrap();
        sessions.get(token).cloned()
    }

    /// Get current session for a project (if any).
    pub fn get_for_project(&self, owner: &str, project: &str) -> Option<McpSession> {
        let key = (owner.to_string(), project.to_string());
        let project_tokens = self.project_tokens.lock().unwrap();
        let token = project_tokens.get(&key)?;
        let sessions = self.sessions.lock().unwrap();
        sessions.get(token).cloned()
    }

    fn generate_token() -> String {
        let bytes: Vec<u8> = (0..32).map(|_| rand::random::<u8>()).collect();
        hex::encode(bytes)
    }
}
