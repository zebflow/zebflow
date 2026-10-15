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
use crate::platform::services::AuthorizationService;
use crate::platform::services::access::roles::managed_role_policies;

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
    pub fn list_members(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<ProjectMember>, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        self.authz.ensure_project_defaults(&owner, &project)?;
        let project_row = self
            .data
            .get_project(&owner, &project)?
            .ok_or_else(|| PlatformError::new("PLATFORM_PROJECT_MISSING", "project not found"))?;
        let mut items = self.data.list_project_members(&owner, &project)?;
        if !items.iter().any(|item| item.user_id == owner) {
            items.insert(
                0,
                ProjectMember {
                    project_id: project_row.project_id.clone(),
                    owner: owner.clone(),
                    project: project.clone(),
                    user_id: owner.clone(),
                    member_user_id: project_row.owner_user_id.clone(),
                    role_preset: ProjectAccessRolePreset::Owner,
                    custom_policy_ids: Vec::new(),
                    mcp_capability_ceiling: Vec::new(),
                    created_by: owner.clone(),
                    created_by_user_id: project_row.owner_user_id.clone(),
                    created_at: 0,
                    updated_at: 0,
                },
            );
        }
        items.sort_by(|a, b| a.user_id.cmp(&b.user_id));
        Ok(items)
    }

    /// Create or update one project member and synchronize their policy bindings.
    /// The role one person currently holds, for callers outside this service.
    pub fn role_of_public(
        &self,
        owner: &str,
        project: &str,
        user_id: &str,
    ) -> Result<ProjectAccessRolePreset, PlatformError> {
        self.role_of(&slug_segment(owner), &slug_segment(project), &slug_segment(user_id))
    }

    /// The role one person currently holds in one project.
    ///
    /// The namespace owner is an Owner whether or not a member row says so —
    /// the project is theirs, and `ensure_project_defaults` binds them without
    /// creating a membership record.
    fn role_of(
        &self,
        owner: &str,
        project: &str,
        user_id: &str,
    ) -> Result<ProjectAccessRolePreset, PlatformError> {
        if user_id == owner {
            return Ok(ProjectAccessRolePreset::Owner);
        }
        Ok(self
            .data
            .get_project_member(owner, project, user_id)?
            .map(|member| member.role_preset)
            .unwrap_or(ProjectAccessRolePreset::Guest))
    }

    /// Verify that `actor_user` holds at least the requested `role_preset`,
    /// every policy in `custom_policy_ids`, and every capability in
    /// `mcp_capability_ceiling`. Shared by `upsert_member` and `create_invite`.
    pub fn validate_grant(
        &self,
        actor_user: &str,
        owner: &str,
        project: &str,
        role_preset: ProjectAccessRolePreset,
        custom_policy_ids: &[String],
        mcp_capability_ceiling: &[crate::platform::model::ProjectCapability],
    ) -> Result<ProjectAccessRolePreset, PlatformError> {
        let actor_user = slug_segment(actor_user);
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        self.authz.ensure_project_defaults(&owner, &project)?;
        ensure_managed_role_policies(&self.data, &owner, &project)?;

        let actor_role = self.role_of(&owner, &project, &actor_user)?;
        if !crate::platform::services::access::roles::can_grant(actor_role, role_preset) {
            return Err(PlatformError::new(
                "PLATFORM_MEMBER_ROLE_ABOVE_ACTOR",
                format!(
                    "a {} cannot grant the {} role",
                    actor_role.title(),
                    role_preset.title()
                ),
            ));
        }

        let actor_caps = self.authz.resolve_project_capabilities(
            &crate::platform::model::ProjectAccessSubject::user(&actor_user),
            &owner,
            &project,
        )?;
        let policies = self.data.list_project_policies(&owner, &project)?;
        for raw_id in custom_policy_ids {
            let policy_id = raw_id.trim();
            if policy_id.is_empty() {
                continue;
            }
            if let Some(preset) = crate::platform::services::access::roles::ROLE_LADDER
                .iter()
                .copied()
                .find(|r| r.policy_id() == policy_id)
                && (!crate::platform::services::access::roles::can_grant(actor_role, preset)
                    || !crate::platform::services::access::roles::can_grant(role_preset, preset))
            {
                return Err(PlatformError::new(
                    "PLATFORM_MEMBER_ROLE_ABOVE_ACTOR",
                    format!(
                        "a {} cannot grant policy '{}' on a {} membership",
                        actor_role.title(),
                        policy_id,
                        role_preset.title()
                    ),
                ));
            }
            let Some(policy) = policies.iter().find(|p| p.policy_id == policy_id) else {
                return Err(PlatformError::new(
                    "PLATFORM_MEMBER_POLICY_NOT_FOUND",
                    format!("policy '{policy_id}' not found"),
                ));
            };
            if policy.capabilities.iter().any(|cap| !actor_caps.contains(cap)) {
                return Err(PlatformError::new(
                    "PLATFORM_MEMBER_ROLE_ABOVE_ACTOR",
                    format!(
                        "a {} cannot grant policy '{}' above their own capabilities",
                        actor_role.title(),
                        policy_id
                    ),
                ));
            }
        }
        for cap in mcp_capability_ceiling {
            if !actor_caps.contains(cap) {
                return Err(PlatformError::new(
                    "PLATFORM_MEMBER_ROLE_ABOVE_ACTOR",
                    format!(
                        "a {} cannot grant MCP capability '{}' above their own capabilities",
                        actor_role.title(),
                        cap.key()
                    ),
                ));
            }
        }
        Ok(actor_role)
    }

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
        let project_row = self
            .data
            .get_project(&owner, &project)?
            .ok_or_else(|| PlatformError::new("PLATFORM_PROJECT_MISSING", "project not found"))?;
        let actor_user_row = self
            .data
            .get_user_auth(&actor_user)?
            .ok_or_else(|| PlatformError::new("PLATFORM_USER_NOT_FOUND", "actor user not found"))?;

        if user_id.is_empty() {
            return Err(PlatformError::new(
                "PLATFORM_MEMBER_INVALID",
                "user_id must not be empty",
            ));
        }
        let member_user_row = self
            .data
            .get_user_auth(&user_id)?
            .ok_or_else(|| PlatformError::new("PLATFORM_MEMBER_USER_MISSING", "user not found"))?;

        if user_id == owner && req.role_preset != ProjectAccessRolePreset::Owner {
            return Err(PlatformError::new(
                "PLATFORM_MEMBER_OWNER_ROLE_INVALID",
                "project owner must keep the owner role preset",
            ));
        }

        // Nobody hands out more than they hold. Without this, anyone who can
        // edit members can promote themselves: `MembersWrite` belongs to
        // Maintainer, and a Maintainer could mint an Owner and then be made one.
        let actor_role = self.validate_grant(
            &actor_user,
            &owner,
            &project,
            req.role_preset,
            &req.custom_policy_ids,
            &req.mcp_capability_ceiling,
        )?;
        let existing_role = self.role_of(&owner, &project, &user_id)?;
        if !crate::platform::services::access::roles::can_grant(actor_role, existing_role) {
            return Err(PlatformError::new(
                "PLATFORM_MEMBER_ROLE_ABOVE_ACTOR",
                format!(
                    "a {} cannot modify a {}",
                    actor_role.title(),
                    existing_role.title()
                ),
            ));
        }

        // An empty ceiling means "unrestricted", so a member who can edit
        // members (their own row included) could otherwise lift the ceiling
        // the owner set. Below the Owner rung, a ceiling only narrows.
        let existing_row = self.data.get_project_member(&owner, &project, &user_id)?;
        if actor_role != ProjectAccessRolePreset::Owner
            && let Some(existing) = &existing_row
            && !existing.mcp_capability_ceiling.is_empty()
            && (req.mcp_capability_ceiling.is_empty()
                || req
                    .mcp_capability_ceiling
                    .iter()
                    .any(|cap| !existing.mcp_capability_ceiling.contains(cap)))
        {
            return Err(PlatformError::new(
                "PLATFORM_MEMBER_ROLE_ABOVE_ACTOR",
                format!(
                    "a {} cannot widen the MCP capability ceiling of '{}'; only an owner can",
                    actor_role.title(),
                    user_id
                ),
            ));
        }

        let now = now_ts();
        let created_at = existing_row.map(|row| row.created_at).unwrap_or(now);
        let member = ProjectMember {
            project_id: project_row.project_id.clone(),
            owner: owner.clone(),
            project: project.clone(),
            user_id: user_id.clone(),
            member_user_id: member_user_row.profile.user_id,
            role_preset: req.role_preset,
            custom_policy_ids: req.custom_policy_ids.clone(),
            mcp_capability_ceiling: req.mcp_capability_ceiling.clone(),
            created_by: actor_user,
            created_by_user_id: actor_user_row.profile.user_id,
            created_at,
            updated_at: now,
        };
        self.data.put_project_member(&member)?;

        if user_id != owner {
            self.data
                .delete_project_policy_binding(&owner, &project, &user_id)?;
            for policy_id in member_policy_ids(&member) {
                self.data
                    .put_project_policy_binding(&ProjectPolicyBinding {
                        project_id: project_row.project_id.clone(),
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
        self.data
            .delete_project_member(&owner, &project, &user_id)?;
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
    // `managed_role_policies` builds rows with an empty `project_id` — it takes
    // an owner and a slug and has no way to know the id. Writing them through
    // unchanged put an empty string into a column with a foreign key onto
    // `projects(project_id)`, so every call failed with "FOREIGN KEY constraint
    // failed" and no indication of which key. It never surfaced because nothing
    // could reach `upsert_member`: the members routes did not exist.
    let project_row = data.get_project(owner, project)?.ok_or_else(|| {
        PlatformError::new(
            "PLATFORM_PROJECT_MISSING",
            format!("project '{owner}/{project}' not found"),
        )
    })?;
    let existing = data.list_project_policies(owner, project)?;
    let now = now_ts();
    for mut policy in managed_role_policies(owner, project, now) {
        policy.project_id = project_row.project_id.clone();
        policy.created_at = existing
            .iter()
            .find(|row| row.policy_id == policy.policy_id)
            .map(|row| row.created_at)
            .unwrap_or(now);
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
            project_id: "prj_1".to_string(),
            owner: "superadmin".to_string(),
            project: "default".to_string(),
            user_id: "alice".to_string(),
            member_user_id: "usr_2".to_string(),
            role_preset: ProjectAccessRolePreset::Developer,
            custom_policy_ids: vec![
                "developer".to_string(),
                "custom.debug".to_string(),
                "custom.debug".to_string(),
            ],
            mcp_capability_ceiling: Vec::new(),
            created_by: "superadmin".to_string(),
            created_by_user_id: "usr_1".to_string(),
            created_at: 1,
            updated_at: 1,
        });
        assert_eq!(
            ids,
            vec!["custom.debug".to_string(), "developer".to_string()]
        );
    }

    #[test]
    fn a_maintainer_cannot_escalate_via_custom_policy_ids_or_self_edit() {
        let dir = tempfile::tempdir().expect("tempdir");
        let platform = crate::platform::services::PlatformService::from_config(
            crate::platform::model::PlatformConfig {
                data_root: dir.path().to_path_buf(),
                ..crate::platform::model::PlatformConfig::default()
            },
        )
        .expect("platform");
        let pw = uuid::Uuid::new_v4().to_string();
        platform
            .users
            .create_user(&crate::platform::model::CreateUserRequest {
                owner: "maint".to_string(),
                password: pw.clone(),
                role: "member".to_string(),
                git_name: String::new(),
                git_email: String::new(),
            })
            .expect("create maint");
        platform
            .users
            .create_user(&crate::platform::model::CreateUserRequest {
                owner: "dev".to_string(),
                password: pw,
                role: "member".to_string(),
                git_name: String::new(),
                git_email: String::new(),
            })
            .expect("create dev");

        // Owner makes `maint` a Maintainer on `superadmin/default`.
        platform
            .project_members
            .upsert_member(
                "superadmin",
                "superadmin",
                "default",
                &UpsertProjectMemberRequest {
                    user_id: "maint".to_string(),
                    role_preset: ProjectAccessRolePreset::Maintainer,
                    custom_policy_ids: Vec::new(),
                    mcp_capability_ceiling: Vec::new(),
                },
            )
            .expect("owner grants maintainer");

        // Maintainer tries to attach the `owner` policy via `custom_policy_ids` while keeping role_preset = Maintainer.
        let err = platform
            .project_members
            .upsert_member(
                "maint",
                "superadmin",
                "default",
                &UpsertProjectMemberRequest {
                    user_id: "dev".to_string(),
                    role_preset: ProjectAccessRolePreset::Maintainer,
                    custom_policy_ids: vec!["owner".to_string()],
                    mcp_capability_ceiling: Vec::new(),
                },
            )
            .expect_err("maintainer must not bind owner policy via custom_policy_ids");
        assert_eq!(err.code, "PLATFORM_MEMBER_ROLE_ABOVE_ACTOR");

        // Normal Maintainer grant of Developer with no escalation succeeds.
        let member = platform
            .project_members
            .upsert_member(
                "maint",
                "superadmin",
                "default",
                &UpsertProjectMemberRequest {
                    user_id: "dev".to_string(),
                    role_preset: ProjectAccessRolePreset::Developer,
                    custom_policy_ids: Vec::new(),
                    mcp_capability_ceiling: Vec::new(),
                },
            )
            .expect("maintainer grants developer");
        assert_eq!(member.role_preset, ProjectAccessRolePreset::Developer);

        // The owner caps `maint`'s MCP sessions; `maint` then edits their own row.
        let maint_row = |ceiling: Vec<crate::platform::model::ProjectCapability>| UpsertProjectMemberRequest {
            user_id: "maint".to_string(),
            role_preset: ProjectAccessRolePreset::Maintainer,
            custom_policy_ids: Vec::new(),
            mcp_capability_ceiling: ceiling,
        };
        use crate::platform::model::ProjectCapability as C;
        platform
            .project_members
            .upsert_member("superadmin", "superadmin", "default", &maint_row(vec![C::ProjectRead, C::PipelinesRead]))
            .expect("owner sets a ceiling");
        for (lifted, why) in [
            (Vec::new(), "an empty ceiling is unrestricted"),
            (vec![C::ProjectRead, C::PipelinesRead, C::PipelinesWrite], "a capability outside the ceiling"),
        ] {
            let err = platform
                .project_members
                .upsert_member("maint", "superadmin", "default", &maint_row(lifted))
                .expect_err(why);
            assert_eq!(err.code, "PLATFORM_MEMBER_ROLE_ABOVE_ACTOR", "{why}: {}", err.message);
        }
        let own = platform
            .data
            .get_project_member("superadmin", "default", "maint")
            .unwrap()
            .expect("maint row");
        assert_eq!(own.mcp_capability_ceiling, vec![C::ProjectRead, C::PipelinesRead], "the refused self-edits changed nothing");
        // Narrowing their own ceiling is fine; the owner may lift it.
        platform
            .project_members
            .upsert_member("maint", "superadmin", "default", &maint_row(vec![C::PipelinesRead]))
            .expect("a maintainer narrows their own ceiling");
        platform
            .project_members
            .upsert_member("superadmin", "superadmin", "default", &maint_row(Vec::new()))
            .expect("the owner lifts the ceiling");
    }
}
