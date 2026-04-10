//! Project membership service.
//!
//! Membership is the product-level sharing primitive for `0.2.0`.
//! This service persists explicit member rows, then projects them into the
//! lower-level authorization model by keeping the user's policy bindings in sync
//! with their selected role preset and any extra project policies.

use std::collections::BTreeSet;
use std::sync::Arc;

use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    ProjectAccessRolePreset, ProjectMember, ProjectPolicyBinding, ProjectSubjectKind,
    UpsertProjectMemberRequest, now_ts, slug_segment,
};
use crate::platform::services::access::roles::managed_role_policies;
use crate::platform::services::AuthorizationService;

/// Product-facing project membership service.
pub struct ProjectMembershipService {
    data: Arc<dyn DataAdapter>,
    authz: Arc<AuthorizationService>,
}

impl ProjectMembershipService {
    /// Create membership service on top of the shared metadata adapter and
    /// authorization service.
    pub fn new(data: Arc<dyn DataAdapter>, authz: Arc<AuthorizationService>) -> Self {
        Self { data, authz }
    }

    /// List explicit project members, plus a synthesized owner row if needed.
    pub fn list_members(&self, owner: &str, project: &str) -> Result<Vec<ProjectMember>, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        self.authz.ensure_project_defaults(&owner, &project)?;
        let mut items = self.data.list_project_members(&owner, &project)?;
        if !items.iter().any(|item| item.user_id == owner) {
            items.insert(
                0,
                ProjectMember {
                    owner: owner.clone(),
                    project: project.clone(),
                    user_id: owner.clone(),
                    role_preset: ProjectAccessRolePreset::Owner,
                    custom_policy_ids: Vec::new(),
                    mcp_capability_ceiling: Vec::new(),
                    created_by: owner.clone(),
                    created_at: 0,
                    updated_at: 0,
                },
            );
        }
        items.sort_by(|a, b| a.user_id.cmp(&b.user_id));
        Ok(items)
    }

    /// Create or update one project member and synchronize their policy bindings.
    pub fn upsert_member(
        &self,
        actor_user: &str,
        owner: &str,
        project: &str,
        req: &UpsertProjectMemberRequest,
    ) -> Result<ProjectMember, PlatformError> {
        let actor_user = slug_segment(actor_user);
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let user_id = slug_segment(&req.user_id);

        if user_id.is_empty() {
            return Err(PlatformError::new(
                "PLATFORM_MEMBER_INVALID",
                "user_id must not be empty",
            ));
        }
        if self.data.get_user_auth(&user_id)?.is_none() {
            return Err(PlatformError::new(
                "PLATFORM_MEMBER_USER_MISSING",
                format!("user '{user_id}' not found"),
            ));
        }

        self.authz.ensure_project_defaults(&owner, &project)?;
        ensure_managed_role_policies(&self.data, &owner, &project)?;

        if user_id == owner && req.role_preset != ProjectAccessRolePreset::Owner {
            return Err(PlatformError::new(
                "PLATFORM_MEMBER_OWNER_ROLE_INVALID",
                "project owner must keep the owner role preset",
            ));
        }

        let now = now_ts();
        let created_at = self
            .data
            .get_project_member(&owner, &project, &user_id)?
            .map(|row| row.created_at)
            .unwrap_or(now);
        let member = ProjectMember {
            owner: owner.clone(),
            project: project.clone(),
            user_id: user_id.clone(),
            role_preset: req.role_preset,
            custom_policy_ids: req.custom_policy_ids.clone(),
            mcp_capability_ceiling: req.mcp_capability_ceiling.clone(),
            created_by: actor_user,
            created_at,
            updated_at: now,
        };
        self.data.put_project_member(&member)?;

        if user_id != owner {
            self.data
                .delete_project_policy_binding(&owner, &project, &user_id)?;
            for policy_id in member_policy_ids(&member) {
                self.data.put_project_policy_binding(&ProjectPolicyBinding {
                    owner: owner.clone(),
                    project: project.clone(),
                    subject_kind: ProjectSubjectKind::User,
                    subject_id: user_id.clone(),
                    policy_id,
                    created_at: now,
                    updated_at: now,
                })?;
            }
        }

        Ok(member)
    }

    /// Remove one explicit project member and clear their user bindings.
    pub fn remove_member(
        &self,
        owner: &str,
        project: &str,
        user_id: &str,
    ) -> Result<(), PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let user_id = slug_segment(user_id);
        if user_id == owner {
            return Err(PlatformError::new(
                "PLATFORM_MEMBER_OWNER_REMOVE_FORBIDDEN",
                "project owner cannot be removed from membership",
            ));
        }
        self.data.delete_project_member(&owner, &project, &user_id)?;
        self.data
            .delete_project_policy_binding(&owner, &project, &user_id)?;
        Ok(())
    }
}

fn ensure_managed_role_policies(
    data: &Arc<dyn DataAdapter>,
    owner: &str,
    project: &str,
) -> Result<(), PlatformError> {
    let now = now_ts();
    for policy in managed_role_policies(owner, project, now) {
        let created_at = data
            .list_project_policies(owner, project)?
            .into_iter()
            .find(|row| row.policy_id == policy.policy_id)
            .map(|row| row.created_at)
            .unwrap_or(now);
        let mut policy = policy;
        policy.created_at = created_at;
        data.put_project_policy(&policy)?;
    }
    Ok(())
}

fn member_policy_ids(member: &ProjectMember) -> Vec<String> {
    let mut ids = BTreeSet::new();
    ids.insert(member.role_preset.policy_id().to_string());
    ids.extend(
        member
            .custom_policy_ids
            .iter()
            .map(|item| item.trim())
            .filter(|item| !item.is_empty())
            .map(ToString::to_string),
    );
    ids.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn member_policy_ids_include_role_and_custom_bindings_once() {
        let ids = member_policy_ids(&ProjectMember {
            owner: "superadmin".to_string(),
            project: "default".to_string(),
            user_id: "alice".to_string(),
            role_preset: ProjectAccessRolePreset::Developer,
            custom_policy_ids: vec![
                "developer".to_string(),
                "custom.debug".to_string(),
                "custom.debug".to_string(),
            ],
            mcp_capability_ceiling: Vec::new(),
            created_by: "superadmin".to_string(),
            created_at: 1,
            updated_at: 1,
        });
        assert_eq!(ids, vec!["custom.debug".to_string(), "developer".to_string()]);
    }
}
