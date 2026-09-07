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
    pub fn list_invites(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<ProjectInvite>, PlatformError> {
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

    /// Every pending invite addressed to one person, across all projects.
    ///
    /// This is what makes an invitation something a person answers rather than
    /// something done to them: they can see what they have been asked to join
    /// before they are in it.
    pub fn list_invites_for_user(
        &self,
        target_user: &str,
    ) -> Result<Vec<ProjectInvite>, PlatformError> {
        let target_user = slug_segment(target_user);
        let now = now_ts();
        Ok(self
            .data
            .list_project_invites_for_user(&target_user)?
            .into_iter()
            .filter(|invite| invite.status == ProjectInviteStatus::Pending)
            .filter(|invite| invite.expires_at.is_none_or(|at| at > now))
            .collect())
    }

    /// Accept one invite, becoming a member.
    ///
    /// Only the person the invite names may accept it, and only while it is
    /// pending and unexpired. Expiry is judged here rather than by a sweep, so
    /// a lapsed invite is refused even if nothing has swept yet.
    pub fn accept_invite(
        &self,
        actor_user: &str,
        owner: &str,
        project: &str,
        invite_id: &str,
    ) -> Result<ProjectInvite, PlatformError> {
        let actor_user = slug_segment(actor_user);
        let mut invite = self.answerable_invite(&actor_user, owner, project, invite_id)?;
        invite.status = ProjectInviteStatus::Accepted;
        invite.updated_at = now_ts();
        self.data.put_project_invite(&invite)?;
        Ok(invite)
    }

    /// Decline one invite. The record stays so the inviter can see the answer.
    pub fn decline_invite(
        &self,
        actor_user: &str,
        owner: &str,
        project: &str,
        invite_id: &str,
    ) -> Result<ProjectInvite, PlatformError> {
        let actor_user = slug_segment(actor_user);
        let mut invite = self.answerable_invite(&actor_user, owner, project, invite_id)?;
        invite.status = ProjectInviteStatus::Revoked;
        invite.updated_at = now_ts();
        self.data.put_project_invite(&invite)?;
        Ok(invite)
    }

    /// The invite this actor may answer, without answering it.
    ///
    /// Lets a caller do the work an acceptance implies before recording that it
    /// happened, so a failure halfway leaves the invite still answerable.
    pub fn peek_answerable(
        &self,
        actor_user: &str,
        owner: &str,
        project: &str,
        invite_id: &str,
    ) -> Result<ProjectInvite, PlatformError> {
        self.answerable_invite(&slug_segment(actor_user), owner, project, invite_id)
    }

    /// The invite this actor is allowed to answer right now, or why not.
    fn answerable_invite(
        &self,
        actor_user: &str,
        owner: &str,
        project: &str,
        invite_id: &str,
    ) -> Result<ProjectInvite, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let Some(invite) = self
            .data
            .get_project_invite(&owner, &project, invite_id.trim())?
        else {
            return Err(PlatformError::new(
                "PLATFORM_INVITE_NOT_FOUND",
                format!("invite '{}' not found", invite_id.trim()),
            ));
        };
        // Not "forbidden": an invite addressed to someone else is not this
        // person's to know about at all.
        if invite.target_user != actor_user {
            return Err(PlatformError::new(
                "PLATFORM_INVITE_NOT_FOUND",
                format!("invite '{}' not found", invite_id.trim()),
            ));
        }
        if invite.status != ProjectInviteStatus::Pending {
            return Err(PlatformError::new(
                "PLATFORM_INVITE_NOT_PENDING",
                format!(
                    "invite '{}' was already {}",
                    invite.invite_id,
                    invite.status.key()
                ),
            ));
        }
        if invite.expires_at.is_some_and(|at| at <= now_ts()) {
            return Err(PlatformError::new(
                "PLATFORM_INVITE_EXPIRED",
                format!("invite '{}' has expired", invite.invite_id),
            ));
        }
        Ok(invite)
    }
}
