//! Project credential management service.

use std::collections::HashSet;
use std::sync::Arc;

use serde_json::Value;

use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    ProjectCredential, ProjectCredentialListItem, SecureRequestVariableDefinition,
    UpsertProjectCredentialRequest, now_ts, slug_segment,
};

/// Project-scoped credentials stored in the metadata catalog.
pub struct CredentialService {
    data: Arc<dyn DataAdapter>,
    http_client: reqwest::Client,
    /// Prevents concurrent token refresh for the same credential.
    refreshing: tokio::sync::Mutex<HashSet<String>>,
}

impl CredentialService {
    /// Creates the credential service.
    pub fn new(data: Arc<dyn DataAdapter>, http_client: reqwest::Client) -> Self {
        Self {
            data,
            http_client,
            refreshing: tokio::sync::Mutex::new(HashSet::new()),
        }
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
                let oauth2_status = if credential.kind == "oauth2" {
                    derive_oauth2_status(&credential.secret)
                } else {
                    String::new()
                };
                ProjectCredentialListItem {
                    credential_id: credential.credential_id,
                    title: credential.title,
                    kind: credential.kind,
                    has_secret: !credential.secret.is_null(),
                    notes: credential.notes,
                    auth_roles,
                    secure_request_vars,
                    oauth2_status,
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

    // ── OAuth2 token management ─────────────────────────────────────────────

    /// Exchanges an OAuth2 authorization code for tokens and stores them in the credential.
    pub async fn exchange_oauth2_code(
        &self,
        owner: &str,
        project: &str,
        credential_id: &str,
        code: &str,
        redirect_uri: &str,
    ) -> Result<(), PlatformError> {
        let credential = self
            .get_project_credential(owner, project, credential_id)?
            .ok_or_else(|| {
                PlatformError::new("PLATFORM_CREDENTIAL_MISSING", "credential not found")
            })?;
        if credential.kind != "oauth2" {
            return Err(PlatformError::new(
                "PLATFORM_CREDENTIAL_INVALID",
                "credential is not oauth2",
            ));
        }
        let secret = &credential.secret;
        let client_id = secret_str(secret, "client_id")?;
        let client_secret = secret_str(secret, "client_secret")?;
        let token_url = secret_str(secret, "token_url")?;

        let resp = self
            .http_client
            .post(&token_url)
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", code),
                ("redirect_uri", redirect_uri),
                ("client_id", &client_id),
                ("client_secret", &client_secret),
            ])
            .send()
            .await
            .map_err(|e| {
                PlatformError::new(
                    "PLATFORM_OAUTH2_EXCHANGE",
                    format!("token request failed: {e}"),
                )
            })?;

        let status = resp.status();
        let body: Value = resp.json().await.map_err(|e| {
            PlatformError::new(
                "PLATFORM_OAUTH2_EXCHANGE",
                format!("failed to parse token response: {e}"),
            )
        })?;

        if !status.is_success() {
            let err_desc = body
                .get("error_description")
                .or_else(|| body.get("error"))
                .and_then(Value::as_str)
                .unwrap_or("unknown error");
            return Err(PlatformError::new(
                "PLATFORM_OAUTH2_EXCHANGE",
                format!("token exchange failed ({status}): {err_desc}"),
            ));
        }

        let access_token = body
            .get("access_token")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let refresh_token = body
            .get("refresh_token")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let expires_in = body
            .get("expires_in")
            .and_then(Value::as_i64)
            .unwrap_or(3600);
        let token_type = body
            .get("token_type")
            .and_then(Value::as_str)
            .unwrap_or("Bearer");

        let mut updated_secret = credential.secret.clone();
        if let Value::Object(map) = &mut updated_secret {
            map.insert(
                "access_token".to_string(),
                Value::String(access_token.to_string()),
            );
            if !refresh_token.is_empty() {
                map.insert(
                    "refresh_token".to_string(),
                    Value::String(refresh_token.to_string()),
                );
            }
            map.insert(
                "expires_at".to_string(),
                Value::Number((now_ts() + expires_in - 60).into()),
            );
            map.insert(
                "token_type".to_string(),
                Value::String(token_type.to_string()),
            );
        }

        let updated = ProjectCredential {
            secret: updated_secret,
            updated_at: now_ts(),
            ..credential
        };
        self.data.put_project_credential(&updated)?;
        Ok(())
    }

    /// Refreshes an OAuth2 access token using the stored refresh_token.
    /// Returns the new access_token.
    pub async fn refresh_oauth2_token(
        &self,
        owner: &str,
        project: &str,
        credential_id: &str,
    ) -> Result<String, PlatformError> {
        let key = format!("{owner}/{project}/{credential_id}");

        // Dedup: if already refreshing, wait then re-read
        {
            let mut lock = self.refreshing.lock().await;
            if lock.contains(&key) {
                drop(lock);
                // Another task is refreshing — wait briefly, then read updated credential
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                let cred = self
                    .get_project_credential(owner, project, credential_id)?
                    .ok_or_else(|| {
                        PlatformError::new("PLATFORM_CREDENTIAL_MISSING", "credential not found")
                    })?;
                return Ok(cred
                    .secret
                    .get("access_token")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string());
            }
            lock.insert(key.clone());
        }

        let result = self
            .do_refresh_oauth2_token(owner, project, credential_id)
            .await;

        // Always remove from refreshing set
        {
            let mut lock = self.refreshing.lock().await;
            lock.remove(&key);
        }

        result
    }

    async fn do_refresh_oauth2_token(
        &self,
        owner: &str,
        project: &str,
        credential_id: &str,
    ) -> Result<String, PlatformError> {
        let credential = self
            .get_project_credential(owner, project, credential_id)?
            .ok_or_else(|| {
                PlatformError::new("PLATFORM_CREDENTIAL_MISSING", "credential not found")
            })?;
        let secret = &credential.secret;
        let client_id = secret_str(secret, "client_id")?;
        let client_secret = secret_str(secret, "client_secret")?;
        let token_url = secret_str(secret, "token_url")?;
        let refresh_token = secret_str(secret, "refresh_token")?;

        if refresh_token.is_empty() {
            return Err(PlatformError::new(
                "PLATFORM_OAUTH2_REFRESH",
                "no refresh_token stored — credential needs re-authorization",
            ));
        }

        let resp = self
            .http_client
            .post(&token_url)
            .form(&[
                ("grant_type", "refresh_token"),
                ("refresh_token", &refresh_token),
                ("client_id", &client_id),
                ("client_secret", &client_secret),
            ])
            .send()
            .await
            .map_err(|e| {
                PlatformError::new(
                    "PLATFORM_OAUTH2_REFRESH",
                    format!("refresh request failed: {e}"),
                )
            })?;

        let status = resp.status();
        let body: Value = resp.json().await.map_err(|e| {
            PlatformError::new(
                "PLATFORM_OAUTH2_REFRESH",
                format!("failed to parse refresh response: {e}"),
            )
        })?;

        if !status.is_success() {
            let err_desc = body
                .get("error_description")
                .or_else(|| body.get("error"))
                .and_then(Value::as_str)
                .unwrap_or("unknown error");
            return Err(PlatformError::new(
                "PLATFORM_OAUTH2_REFRESH",
                format!("token refresh failed ({status}): {err_desc}"),
            ));
        }

        let new_access_token = body
            .get("access_token")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let new_refresh_token = body
            .get("refresh_token")
            .and_then(Value::as_str)
            .map(ToString::to_string);
        let expires_in = body
            .get("expires_in")
            .and_then(Value::as_i64)
            .unwrap_or(3600);

        let mut updated_secret = credential.secret.clone();
        if let Value::Object(map) = &mut updated_secret {
            map.insert(
                "access_token".to_string(),
                Value::String(new_access_token.clone()),
            );
            if let Some(rt) = new_refresh_token {
                if !rt.is_empty() {
                    map.insert("refresh_token".to_string(), Value::String(rt));
                }
            }
            map.insert(
                "expires_at".to_string(),
                Value::Number((now_ts() + expires_in - 60).into()),
            );
        }

        let updated = ProjectCredential {
            secret: updated_secret,
            updated_at: now_ts(),
            ..credential
        };
        self.data.put_project_credential(&updated)?;
        Ok(new_access_token)
    }

    /// Returns a valid OAuth2 access token, refreshing first if expired.
    pub async fn get_valid_oauth2_token(
        &self,
        owner: &str,
        project: &str,
        credential_id: &str,
    ) -> Result<String, PlatformError> {
        let credential = self
            .get_project_credential(owner, project, credential_id)?
            .ok_or_else(|| {
                PlatformError::new("PLATFORM_CREDENTIAL_MISSING", "credential not found")
            })?;
        let secret = &credential.secret;
        let expires_at = secret
            .get("expires_at")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let now = now_ts();

        if expires_at > now + 60 {
            // Token still valid (with 60s buffer)
            Ok(secret
                .get("access_token")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string())
        } else {
            self.refresh_oauth2_token(owner, project, credential_id)
                .await
        }
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

/// Helper to extract a string field from a secret JSON object.
fn secret_str(secret: &Value, key: &str) -> Result<String, PlatformError> {
    Ok(secret
        .get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string())
}

fn derive_oauth2_status(secret: &Value) -> String {
    let has_refresh = secret
        .get("refresh_token")
        .and_then(Value::as_str)
        .map(|s| !s.is_empty())
        .unwrap_or(false);
    if !has_refresh {
        return "not_configured".to_string();
    }
    let expires_at = secret
        .get("expires_at")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let now = now_ts();
    if expires_at > 0 && expires_at < now {
        "expired".to_string()
    } else {
        "authorized".to_string()
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
