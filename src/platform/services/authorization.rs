//! Project-level authorization service shared by REST, MCP, and assistant entrypoints.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    ProjectAccessSubject, ProjectCapability, ProjectPolicy, ProjectPolicyBinding, ProjectSubjectKind,
    now_ts, slug_segment,
};

/// Resolves project-scoped policies and capabilities.
pub struct AuthorizationService {
    data: Arc<dyn DataAdapter>,
}

impl AuthorizationService {
    /// Creates authorization service.
    pub fn new(data: Arc<dyn DataAdapter>) -> Self {
        Self { data }
    }

    /// Ensures the canonical managed project policies and owner binding exist.
    pub fn ensure_project_defaults(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<(), PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        if owner.is_empty() || project.is_empty() {
            return Err(PlatformError::new(
                "PLATFORM_AUTHZ_INVALID_SCOPE",
                "owner/project must not be empty",
            ));
        }
        if self.data.get_project(&owner, &project)?.is_none() {
            return Err(PlatformError::new(
                "PLATFORM_PROJECT_MISSING",
                format!("project '{owner}/{project}' not found"),
            ));
        }

        let now = now_ts();
        let existing = self
            .data
            .list_project_policies(&owner, &project)?
            .into_iter()
            .map(|policy| (policy.policy_id.clone(), policy))
            .collect::<BTreeMap<_, _>>();

        for managed in managed_project_policies(&owner, &project, now) {
            let created_at = existing
                .get(&managed.policy_id)
                .map(|row| row.created_at)
                .unwrap_or(now);
            let mut policy = managed;
            policy.created_at = created_at;
            self.data.put_project_policy(&policy)?;
        }

        let bindings = self.data.list_project_policy_bindings(&owner, &project)?;
        let owner_binding_exists = bindings.iter().any(|binding| {
            binding.subject_kind == ProjectSubjectKind::User
                && binding.subject_id == owner
                && binding.policy_id == "owner"
        });
        if !owner_binding_exists {
            self.data.put_project_policy_binding(&ProjectPolicyBinding {
                owner: owner.clone(),
                project: project.clone(),
                subject_kind: ProjectSubjectKind::User,
                subject_id: owner,
                policy_id: "owner".to_string(),
                created_at: now,
                updated_at: now,
            })?;
        }

        Ok(())
    }

    /// Resolves effective capabilities for one subject within one project.
    pub fn resolve_project_capabilities(
        &self,
        subject: &ProjectAccessSubject,
        owner: &str,
        project: &str,
    ) -> Result<BTreeSet<ProjectCapability>, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        self.ensure_project_defaults(&owner, &project)?;

        if subject.kind == ProjectSubjectKind::User
            && let Some(user) = self.data.get_user_auth(&subject.id)?
            && user.profile.role == "superadmin"
        {
            return Ok(all_project_capabilities().into_iter().collect());
        }

        let policies = self
            .data
            .list_project_policies(&owner, &project)?
            .into_iter()
            .map(|policy| (policy.policy_id.clone(), policy))
            .collect::<BTreeMap<_, _>>();
        let bindings = self.data.list_project_policy_bindings(&owner, &project)?;

        let mut out = BTreeSet::new();
        for binding in bindings {
            if binding.subject_kind != subject.kind || binding.subject_id != subject.id {
                continue;
            }
            if let Some(policy) = policies.get(&binding.policy_id) {
                out.extend(policy.capabilities.iter().copied());
            }
        }
        Ok(out)
    }

    /// Ensures one subject is allowed to perform one project-scoped capability.
    pub fn ensure_project_capability(
        &self,
        subject: &ProjectAccessSubject,
        owner: &str,
        project: &str,
        capability: ProjectCapability,
    ) -> Result<(), PlatformError> {
        let allowed = self.resolve_project_capabilities(subject, owner, project)?;
        if allowed.contains(&capability) {
            return Ok(());
        }
        Err(PlatformError::new(
            "PLATFORM_AUTHZ_FORBIDDEN",
            format!(
                "subject '{}' lacks capability '{}' for project '{}/{}'",
                subject.id,
                capability.key(),
                slug_segment(owner),
                slug_segment(project)
            ),
        ))
    }
}

fn managed_project_policies(owner: &str, project: &str, now: i64) -> Vec<ProjectPolicy> {
    vec![
        policy(owner, project, "owner", "Owner", all_project_capabilities(), now),
        policy(
            owner,
            project,
            "viewer",
            "Viewer",
            vec![
                ProjectCapability::ProjectRead,
                ProjectCapability::CredentialsRead,
                ProjectCapability::TemplatesRead,
                ProjectCapability::PipelinesRead,
                ProjectCapability::FilesRead,
                ProjectCapability::TablesRead,
                ProjectCapability::LibrariesRead,
                ProjectCapability::SettingsRead,
            ],
            now,
        ),
        policy(
            owner,
            project,
            "editor",
            "Editor",
            vec![
                ProjectCapability::ProjectRead,
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
                ProjectCapability::FilesRead,
                ProjectCapability::FilesWrite,
                ProjectCapability::TablesRead,
                ProjectCapability::LibrariesRead,
                ProjectCapability::SettingsRead,
            ],
            now,
        ),
        policy(
            owner,
            project,
            "maintainer",
            "Maintainer",
            vec![
                ProjectCapability::ProjectRead,
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
            now,
        ),
        policy(
            owner,
            project,
            "agent.templates",
            "Agent Templates",
            vec![
                ProjectCapability::ProjectRead,
                ProjectCapability::CredentialsRead,
                ProjectCapability::TemplatesRead,
                ProjectCapability::TemplatesWrite,
                ProjectCapability::TemplatesCreate,
                ProjectCapability::TemplatesDelete,
                ProjectCapability::TemplatesMove,
                ProjectCapability::TemplatesDiagnostics,
                ProjectCapability::FilesRead,
                ProjectCapability::PipelinesRead,
            ],
            now,
        ),
        policy(
            owner,
            project,
            "agent.project",
            "Agent Project",
            vec![
                ProjectCapability::ProjectRead,
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
                ProjectCapability::FilesRead,
                ProjectCapability::FilesWrite,
                ProjectCapability::TablesRead,
                ProjectCapability::LibrariesRead,
                ProjectCapability::SettingsRead,
            ],
            now,
        ),
    ]
}

fn policy(
    owner: &str,
    project: &str,
    policy_id: &str,
    title: &str,
    capabilities: Vec<ProjectCapability>,
    now: i64,
) -> ProjectPolicy {
    ProjectPolicy {
        owner: owner.to_string(),
        project: project.to_string(),
        policy_id: policy_id.to_string(),
        title: title.to_string(),
        capabilities,
        managed: true,
        created_at: now,
        updated_at: now,
    }
}

fn all_project_capabilities() -> Vec<ProjectCapability> {
    vec![
        ProjectCapability::ProjectRead,
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
    ]
}
