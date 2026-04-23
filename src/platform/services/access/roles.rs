//! Managed role presets for project membership.
//!
//! Zebflow keeps capability enforcement separate from the product-facing sharing
//! vocabulary. This module defines that vocabulary and the managed policy bundles
//! backing it.
//!
//! The baseline intentionally follows familiar GitLab-style names:
//!
//! - `guest`
//! - `reporter`
//! - `developer`
//! - `maintainer`
//! - `owner`
//!
//! Legacy aliases such as `viewer` and `editor` can still exist as managed
//! policies so older policy ids do not immediately become invalid during the
//! `0.2.0` baseline transition.

use crate::platform::model::{ProjectAccessRolePreset, ProjectCapability, ProjectPolicy};

/// Return the managed capability bundle for one user-facing role preset.
pub fn role_capabilities(role: ProjectAccessRolePreset) -> Vec<ProjectCapability> {
    match role {
        ProjectAccessRolePreset::Guest => vec![
            ProjectCapability::ProjectRead,
            ProjectCapability::MembersRead,
            ProjectCapability::TemplatesRead,
            ProjectCapability::PipelinesRead,
            ProjectCapability::FilesRead,
            ProjectCapability::LibrariesRead,
        ],
        ProjectAccessRolePreset::Reporter => vec![
            ProjectCapability::ProjectRead,
            ProjectCapability::MembersRead,
            ProjectCapability::TemplatesRead,
            ProjectCapability::PipelinesRead,
            ProjectCapability::FilesRead,
            ProjectCapability::TablesRead,
            ProjectCapability::LibrariesRead,
            ProjectCapability::SettingsRead,
        ],
        ProjectAccessRolePreset::Developer => vec![
            ProjectCapability::ProjectRead,
            ProjectCapability::MembersRead,
            ProjectCapability::TemplatesRead,
            ProjectCapability::TemplatesWrite,
            ProjectCapability::TemplatesCreate,
            ProjectCapability::TemplatesDelete,
            ProjectCapability::TemplatesMove,
            ProjectCapability::TemplatesDiagnostics,
            ProjectCapability::PipelinesRead,
            ProjectCapability::PipelinesWrite,
            ProjectCapability::PipelinesCreate,
            ProjectCapability::PipelinesDelete,
            ProjectCapability::PipelinesMove,
            ProjectCapability::PipelinesExecute,
            ProjectCapability::FilesRead,
            ProjectCapability::FilesWrite,
            ProjectCapability::FilesDelete,
            ProjectCapability::TablesRead,
            ProjectCapability::LibrariesRead,
            ProjectCapability::SettingsRead,
        ],
        ProjectAccessRolePreset::Maintainer => vec![
            ProjectCapability::ProjectRead,
            ProjectCapability::MembersRead,
            ProjectCapability::MembersWrite,
            ProjectCapability::CredentialsRead,
            ProjectCapability::CredentialsWrite,
            ProjectCapability::TemplatesRead,
            ProjectCapability::TemplatesWrite,
            ProjectCapability::TemplatesCreate,
            ProjectCapability::TemplatesDelete,
            ProjectCapability::TemplatesMove,
            ProjectCapability::TemplatesDiagnostics,
            ProjectCapability::PipelinesRead,
            ProjectCapability::PipelinesWrite,
            ProjectCapability::PipelinesCreate,
            ProjectCapability::PipelinesDelete,
            ProjectCapability::PipelinesMove,
            ProjectCapability::PipelinesExecute,
            ProjectCapability::FilesRead,
            ProjectCapability::FilesWrite,
            ProjectCapability::FilesDelete,
            ProjectCapability::TablesRead,
            ProjectCapability::TablesWrite,
            ProjectCapability::LibrariesRead,
            ProjectCapability::LibrariesInstall,
            ProjectCapability::LibrariesRemove,
            ProjectCapability::SettingsRead,
            ProjectCapability::SettingsWrite,
            ProjectCapability::McpSessionCreate,
            ProjectCapability::McpSessionRevoke,
        ],
        ProjectAccessRolePreset::Owner => ProjectCapability::all(),
    }
}

/// Build the managed policy rows for the core access presets.
pub fn managed_role_policies(owner: &str, project: &str, now: i64) -> Vec<ProjectPolicy> {
    [
        ProjectAccessRolePreset::Owner,
        ProjectAccessRolePreset::Guest,
        ProjectAccessRolePreset::Reporter,
        ProjectAccessRolePreset::Developer,
        ProjectAccessRolePreset::Maintainer,
    ]
    .into_iter()
    .map(|role| ProjectPolicy {
        project_id: String::new(),
        owner: owner.to_string(),
        project: project.to_string(),
        policy_id: role.policy_id().to_string(),
        title: role.title().to_string(),
        capabilities: role_capabilities(role),
        managed: true,
        created_at: now,
        updated_at: now,
    })
    .collect()
}

/// Transitional aliases that keep older policy ids readable while the product
/// moves toward GitLab-style role language.
pub fn managed_role_alias_policies(owner: &str, project: &str, now: i64) -> Vec<ProjectPolicy> {
    vec![
        ProjectPolicy {
            project_id: String::new(),
            owner: owner.to_string(),
            project: project.to_string(),
            policy_id: "viewer".to_string(),
            title: "Viewer".to_string(),
            capabilities: role_capabilities(ProjectAccessRolePreset::Reporter),
            managed: true,
            created_at: now,
            updated_at: now,
        },
        ProjectPolicy {
            project_id: String::new(),
            owner: owner.to_string(),
            project: project.to_string(),
            policy_id: "editor".to_string(),
            title: "Editor".to_string(),
            capabilities: role_capabilities(ProjectAccessRolePreset::Developer),
            managed: true,
            created_at: now,
            updated_at: now,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn owner_role_covers_all_capabilities() {
        assert_eq!(
            role_capabilities(ProjectAccessRolePreset::Owner),
            ProjectCapability::all()
        );
    }

    #[test]
    fn role_aliases_point_at_stable_capability_bundles() {
        let aliases = managed_role_alias_policies("superadmin", "default", 1);
        assert_eq!(aliases.len(), 2);
        assert_eq!(aliases[0].policy_id, "viewer");
        assert_eq!(
            aliases[0].capabilities,
            role_capabilities(ProjectAccessRolePreset::Reporter)
        );
        assert_eq!(aliases[1].policy_id, "editor");
        assert_eq!(
            aliases[1].capabilities,
            role_capabilities(ProjectAccessRolePreset::Developer)
        );
    }
}
