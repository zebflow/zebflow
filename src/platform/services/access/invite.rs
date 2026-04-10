//! Project invite service.
//!
//! Invites are kept separate from active membership so the product can support
//! clone-first collaboration and later approval/acceptance flows without
//! mutating active authorization state too early.

use std::sync::Arc;

use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    CreateProjectInviteRequest, ProjectInvite, ProjectInviteStatus, now_ts, slug_segment,
};

/// Product-facing invite service.
pub struct ProjectInviteService {
    data: Arc<dyn DataAdapter>,
}

impl ProjectInviteService {
    /// Create invite service backed by the platform metadata adapter.
    pub fn new(data: Arc<dyn DataAdapter>) -> Self {
        Self { data }
    }

    /// List stored invites for one project.
    pub fn list_invites(&self, owner: &str, project: &str) -> Result<Vec<ProjectInvite>, PlatformError> {
        self.data
            .list_project_invites(&slug_segment(owner), &slug_segment(project))
    }

    /// Create or replace one pending invite.
    pub fn create_invite(
        &self,
        actor_user: &str,
        owner: &str,
        project: &str,
        req: &CreateProjectInviteRequest,
    ) -> Result<ProjectInvite, PlatformError> {
        let actor_user = slug_segment(actor_user);
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let target_user = slug_segment(&req.target_user);

        if target_user.is_empty() {
            return Err(PlatformError::new(
                "PLATFORM_INVITE_INVALID",
                "target_user must not be empty",
            ));
        }
        if self.data.get_project(&owner, &project)?.is_none() {
            return Err(PlatformError::new(
                "PLATFORM_PROJECT_MISSING",
                format!("project '{owner}/{project}' not found"),
            ));
        }
        if self.data.get_user_auth(&target_user)?.is_none() {
            return Err(PlatformError::new(
                "PLATFORM_INVITE_USER_MISSING",
                format!("user '{target_user}' not found"),
            ));
        }

        let now = now_ts();
        let invite_id = format!(
            "invite-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis()
        );
        let invite = ProjectInvite {
            owner,
            project,
            invite_id,
            target_user,
            role_preset: req.role_preset,
            custom_policy_ids: req.custom_policy_ids.clone(),
            mcp_capability_ceiling: req.mcp_capability_ceiling.clone(),
            note: req.note.trim().to_string(),
            invited_by: actor_user,
            status: ProjectInviteStatus::Pending,
            expires_at: req.expires_at,
            created_at: now,
            updated_at: now,
        };
        self.data.put_project_invite(&invite)?;
        Ok(invite)
    }

    /// Mark one invite as revoked.
    pub fn revoke_invite(
        &self,
        owner: &str,
        project: &str,
        invite_id: &str,
    ) -> Result<(), PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let invite_id = invite_id.trim();
        let Some(mut invite) = self.data.get_project_invite(&owner, &project, invite_id)? else {
            return Err(PlatformError::new(
                "PLATFORM_INVITE_NOT_FOUND",
                format!("invite '{invite_id}' not found"),
            ));
        };
        invite.status = ProjectInviteStatus::Revoked;
        invite.updated_at = now_ts();
        self.data.put_project_invite(&invite)?;
        Ok(())
    }
}
