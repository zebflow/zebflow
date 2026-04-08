//! Project credential management service.

use std::sync::Arc;

use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    ProjectCredential, ProjectCredentialListItem, SecureRequestVariableDefinition,
    UpsertProjectCredentialRequest, now_ts, slug_segment,
};
use serde_json::Value;

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
            .map(|credential| {
                let auth_roles = credential
                    .secret
                    .get("auth_roles")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(ToString::to_string))
                            .collect()
                    })
                    .unwrap_or_default();
                let secure_request_vars = if credential.kind == "secure_request" {
                    extract_secure_request_vars(&credential.secret)
                } else {
                    Vec::new()
                };
                ProjectCredentialListItem {
                    credential_id: credential.credential_id,
                    title: credential.title,
                    kind: credential.kind,
                    has_secret: !credential.secret.is_null(),
                    notes: credential.notes,
                    auth_roles,
                    secure_request_vars,
                    created_at: credential.created_at,
                    updated_at: credential.updated_at,
                }
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

fn extract_secure_request_vars(secret: &Value) -> Vec<SecureRequestVariableDefinition> {
    let Some(items) = secret.get("variables").and_then(Value::as_array) else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(|item| {
            let name = item
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_string();
            if name.is_empty() {
                return None;
            }
            Some(SecureRequestVariableDefinition {
                name: name.clone(),
                label: item
                    .get("label")
                    .and_then(Value::as_str)
                    .unwrap_or(&name)
                    .trim()
                    .to_string(),
                value_type: item
                    .get("value_type")
                    .or_else(|| item.get("type"))
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .trim()
                    .to_string(),
                required: item
                    .get("required")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
                default_expr: item
                    .get("default_expr")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .trim()
                    .to_string(),
                description: item
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .trim()
                    .to_string(),
            })
        })
        .collect()
}
