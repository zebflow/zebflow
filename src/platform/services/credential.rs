//! Project credential management service.

use std::sync::Arc;

use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    ProjectCredential, ProjectCredentialListItem, UpsertProjectCredentialRequest, now_ts,
    slug_segment,
};

/// Project-scoped credentials stored in the metadata catalog.
pub struct CredentialService {
    data: Arc<dyn DataAdapter>,
}

impl CredentialService {
    /// Creates the credential service.
    pub fn new(data: Arc<dyn DataAdapter>) -> Self {
        Self { data }
    }

    /// Lists safe project credential summaries.
    pub fn list_project_credentials(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<ProjectCredentialListItem>, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        self.ensure_project_exists(&owner, &project)?;
        let mut items = self
            .data
            .list_project_credentials(&owner, &project)?
            .into_iter()
            .map(|credential| ProjectCredentialListItem {
                credential_id: credential.credential_id,
                title: credential.title,
                kind: credential.kind,
                has_secret: !credential.secret.is_null(),
                notes: credential.notes,
                created_at: credential.created_at,
                updated_at: credential.updated_at,
            })
            .collect::<Vec<_>>();
        items.sort_by(|a, b| a.credential_id.cmp(&b.credential_id));
        Ok(items)
    }

    /// Resolves one full credential payload for runtime use.
    pub fn get_project_credential(
        &self,
        owner: &str,
        project: &str,
        credential_id: &str,
    ) -> Result<Option<ProjectCredential>, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let credential_id = slug_segment(credential_id);
        self.ensure_project_exists(&owner, &project)?;
        if credential_id.is_empty() {
            return Err(PlatformError::new(
                "PLATFORM_CREDENTIAL_INVALID",
                "credential id must not be empty",
            ));
        }
        self.data
            .get_project_credential(&owner, &project, &credential_id)
    }

    /// Creates or updates one credential.
    pub fn upsert_project_credential(
        &self,
        owner: &str,
        project: &str,
        req: &UpsertProjectCredentialRequest,
    ) -> Result<ProjectCredential, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        self.ensure_project_exists(&owner, &project)?;

        let credential_id = slug_segment(&req.credential_id);
        if credential_id.is_empty() {
            return Err(PlatformError::new(
                "PLATFORM_CREDENTIAL_INVALID",
                "credential id must not be empty",
            ));
        }
        let kind = slug_segment(&req.kind);
        if kind.is_empty() {
            return Err(PlatformError::new(
                "PLATFORM_CREDENTIAL_INVALID",
                "credential kind must not be empty",
            ));
        }

        let now = now_ts();
        let existing = self
            .data
            .get_project_credential(&owner, &project, &credential_id)?;
        let created_at = existing.as_ref().map(|row| row.created_at).unwrap_or(now);
        let credential = ProjectCredential {
            owner: owner.clone(),
            project: project.clone(),
            credential_id: credential_id.clone(),
            title: if req.title.trim().is_empty() {
                credential_id.replace('-', " ")
            } else {
                req.title.trim().to_string()
            },
            kind,
            secret: req.secret.clone(),
            notes: req.notes.trim().to_string(),
            created_at,
            updated_at: now,
        };
        self.data.put_project_credential(&credential)?;
        Ok(credential)
    }

    /// Deletes one credential.
    pub fn delete_project_credential(
        &self,
        owner: &str,
        project: &str,
        credential_id: &str,
    ) -> Result<(), PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let credential_id = slug_segment(credential_id);
        self.ensure_project_exists(&owner, &project)?;
        if credential_id.is_empty() {
            return Err(PlatformError::new(
                "PLATFORM_CREDENTIAL_INVALID",
                "credential id must not be empty",
            ));
        }
        self.data
            .delete_project_credential(&owner, &project, &credential_id)
    }

    fn ensure_project_exists(&self, owner: &str, project: &str) -> Result<(), PlatformError> {
        if self.data.get_project(owner, project)?.is_some() {
            return Ok(());
        }
        Err(PlatformError::new(
            "PLATFORM_PROJECT_MISSING",
            format!("project '{owner}/{project}' not found"),
        ))
    }
}
