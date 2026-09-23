//! Project credential management service.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use serde_json::Value;

use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    CredentialFieldDef, CredentialFieldOption, CredentialTypeDef, ProjectCredential,
    ProjectCredentialListItem, SecureRequestVariableDefinition, UpsertProjectCredentialRequest,
    now_ts, slug_segment,
};

/// Project-scoped credentials stored in the metadata catalog.
// ---------------------------------------------------------------------------
// Confidential value registry — `kinds/invocation-record` rule 3
// ---------------------------------------------------------------------------

/// Values that have come out of the credential store, per project.
///
/// A node that reads a secret cannot be relied on to declare it: seven nodes
/// take a credential and one ever marked its secrets, which is why an OAuth
/// `code` reached a trace in full. Registering here — at the one place every
/// credential is resolved — means no node has to remember, and the 34 callers
/// of `get_project_credential` are covered without any of them changing.
///
/// Scoped by owner and project, and only ever read back for that same pair, so
/// this is the same isolation boundary the credential store itself has. It is
/// never a server-wide bag of every tenant's secrets.
static CONFIDENTIAL_VALUES: std::sync::LazyLock<
    std::sync::Mutex<
        std::collections::HashMap<(String, String), std::collections::HashSet<String>>,
    >,
> = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

/// Shortest value worth registering.
///
/// A secret payload holds more than secrets — a host name, a port, a boolean,
/// a username. Masking every short string would black out ordinary diagnostics
/// and teach people to turn masking off. Real keys, tokens and passwords are
/// comfortably longer than this.
const MIN_CONFIDENTIAL_LEN: usize = 8;

/// Ceiling per project, so a project that rotates constantly cannot grow this
/// without bound.
const MAX_CONFIDENTIAL_PER_PROJECT: usize = 512;

/// Record every string inside a resolved secret as confidential.
pub(crate) fn register_confidential(owner: &str, project: &str, secret: &serde_json::Value) {
    let mut found: Vec<String> = Vec::new();
    collect_strings(secret, &mut found, 0);
    if found.is_empty() {
        return;
    }
    let Ok(mut map) = CONFIDENTIAL_VALUES.lock() else {
        return;
    };
    let bucket = map
        .entry((owner.to_string(), project.to_string()))
        .or_default();
    for value in found {
        if bucket.len() >= MAX_CONFIDENTIAL_PER_PROJECT {
            break;
        }
        bucket.insert(value);
    }
}

/// Register confidential values directly. Test-only: production registers at
/// the two places a credential is resolved, never by hand.
#[doc(hidden)]
pub fn register_confidential_for_test(owner: &str, project: &str, secret: &serde_json::Value) {
    register_confidential(owner, project, secret);
}

/// Every confidential value known for this project.
pub fn confidential_values(owner: &str, project: &str) -> Vec<String> {
    CONFIDENTIAL_VALUES
        .lock()
        .ok()
        .and_then(|map| {
            map.get(&(owner.to_string(), project.to_string()))
                .map(|set| set.iter().cloned().collect())
        })
        .unwrap_or_default()
}

fn collect_strings(value: &serde_json::Value, out: &mut Vec<String>, depth: usize) {
    if depth > 8 {
        return;
    }
    match value {
        serde_json::Value::String(text) => {
            if text.len() >= MIN_CONFIDENTIAL_LEN {
                out.push(text.clone());
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                collect_strings(item, out, depth + 1);
            }
        }
        serde_json::Value::Object(map) => {
            for item in map.values() {
                collect_strings(item, out, depth + 1);
            }
        }
        _ => {}
    }
}

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
        let resolved = self
            .data
            .get_project_credential(&owner, &project, &credential_id)?;
        // Every value that leaves the store is confidential from here on, in
        // every diagnostic of every node it reaches. Registered here rather
        // than by each caller, because the caller is exactly who forgets.
        if let Some(credential) = &resolved {
            register_confidential(&owner, &project, &credential.secret);
        }
        Ok(resolved)
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

        // A refreshed token never passed through `get_project_credential`, so
        // it would not be registered by the hook there. It is exactly as
        // confidential as the one it replaces.
        register_confidential(&credential.owner, &credential.project, &updated_secret);

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

// ---------------------------------------------------------------------------
// Built-in credential type definitions
// ---------------------------------------------------------------------------

/// Returns all official credential type definitions shipped with Zebflow.
/// Same format as custom types from composite/WASM packages.
pub fn builtin_credential_types() -> Vec<CredentialTypeDef> {
    use CredentialFieldDef as F;
    use CredentialFieldOption as O;

    let f = |key: &str, label: &str| -> F {
        F {
            key: key.into(),
            label: label.into(),
            ..Default::default()
        }
    };
    let fp = |key: &str, label: &str| -> F {
        F {
            key: key.into(),
            label: label.into(),
            field_type: "password".into(),
            ..Default::default()
        }
    };

    vec![
        CredentialTypeDef {
            kind: "postgres".into(),
            title: "PostgreSQL".into(),
            description: "PostgreSQL database connection.".into(),
            fields: vec![
                F { help: Some("Hostname or IP of PostgreSQL server.".into()), ..f("host", "Host") },
                F { placeholder: Some("5432".into()), help: Some("TCP port for PostgreSQL.".into()), ..f("port", "Port") },
                F { help: Some("Database name.".into()), ..f("database", "Database") },
                F { help: Some("Login username.".into()), ..f("user", "User") },
                F { full_width: true, help: Some("Login password.".into()), ..fp("password", "Password") },
                F { placeholder: Some("prefer".into()), help: Some("disable, prefer, require, verify-ca, verify-full.".into()), ..f("sslmode", "SSL Mode") },
            ],
            ..Default::default()
        },
        CredentialTypeDef {
            kind: "mysql".into(),
            title: "MySQL".into(),
            description: "MySQL database connection.".into(),
            fields: vec![
                F { help: Some("Hostname or IP of MySQL server.".into()), ..f("host", "Host") },
                F { placeholder: Some("3306".into()), help: Some("TCP port for MySQL.".into()), ..f("port", "Port") },
                F { help: Some("Database name.".into()), ..f("database", "Database") },
                F { help: Some("Login username.".into()), ..f("user", "User") },
                F { help: Some("Login password.".into()), full_width: true, ..fp("password", "Password") },
            ],
            ..Default::default()
        },
        CredentialTypeDef {
            kind: "openai".into(),
            title: "OpenAI".into(),
            description: "OpenAI API credential. Defaults to the Responses API.".into(),
            fields: vec![
                F { full_width: true, help: Some("Provider API token.".into()), ..fp("api_key", "API Key") },
                F { full_width: true, placeholder: Some("https://api.openai.com/v1".into()), default: Some("https://api.openai.com/v1".into()), help: Some("Provider API root. Paths below are joined to this URL.".into()), ..f("base_url", "Base URL") },
                F { full_width: true, placeholder: Some("gpt-5.5".into()), help: Some("Fallback model id for requests.".into()), ..f("model", "Default Model") },
                F {
                    field_type: "select".into(),
                    default: Some("responses".into()),
                    options: vec![
                        O { value: "responses".into(), label: "Responses API".into() },
                        O { value: "chat_completions".into(), label: "Chat Completions API".into() },
                    ],
                    help: Some("Protocol shape used for request payloads, tool calls, and response parsing.".into()),
                    ..f("response_surface", "Response Surface")
                },
                F { placeholder: Some("/responses".into()), default: Some("/responses".into()), help: Some("Path appended to Base URL for text/tool responses.".into()), ..f("response_path", "Response Path") },
                F { placeholder: Some("/embeddings".into()), default: Some("/embeddings".into()), help: Some("Path appended to Base URL for embeddings.".into()), ..f("embedding_path", "Embedding Path") },
                F {
                    field_type: "select".into(),
                    default: Some("false".into()),
                    options: vec![
                        O { value: "false".into(), label: "Do not store".into() },
                        O { value: "true".into(), label: "Allow provider storage".into() },
                    ],
                    help: Some("Responses API store flag. Zebflow defaults to no provider-side response storage.".into()),
                    ..f("store", "Provider Storage")
                },
            ],
            ..Default::default()
        },
        CredentialTypeDef {
            kind: "openrouter".into(),
            title: "OpenRouter".into(),
            description: "OpenRouter credential. Defaults to Chat Completions compatibility.".into(),
            fields: vec![
                F { full_width: true, help: Some("OpenRouter API token.".into()), ..fp("api_key", "API Key") },
                F { full_width: true, placeholder: Some("https://openrouter.ai/api/v1".into()), default: Some("https://openrouter.ai/api/v1".into()), help: Some("Provider API root. Paths below are joined to this URL.".into()), ..f("base_url", "Base URL") },
                F { full_width: true, placeholder: Some("openai/gpt-4o-mini".into()), help: Some("Fallback model id for requests.".into()), ..f("model", "Default Model") },
                F {
                    field_type: "select".into(),
                    default: Some("chat_completions".into()),
                    options: vec![
                        O { value: "chat_completions".into(), label: "Chat Completions API".into() },
                        O { value: "responses".into(), label: "Responses API".into() },
                    ],
                    help: Some("Protocol shape used for request payloads, tool calls, and response parsing.".into()),
                    ..f("response_surface", "Response Surface")
                },
                F { placeholder: Some("/chat/completions".into()), default: Some("/chat/completions".into()), help: Some("Path appended to Base URL for text/tool responses.".into()), ..f("response_path", "Response Path") },
                F { placeholder: Some("/embeddings".into()), default: Some("/embeddings".into()), help: Some("Path appended to Base URL for embeddings if the provider/model supports them.".into()), ..f("embedding_path", "Embedding Path") },
            ],
            ..Default::default()
        },
        CredentialTypeDef {
            kind: "http".into(),
            title: "HTTP".into(),
            description: "HTTP Bearer token or API key.".into(),
            fields: vec![
                F { help: Some("Service root URL.".into()), ..f("base_url", "Base URL") },
                F { help: Some("Bearer token or API key.".into()), ..fp("token", "Token") },
            ],
            ..Default::default()
        },
        CredentialTypeDef {
            kind: "github".into(),
            title: "GitHub".into(),
            description: "GitHub API and git operations.".into(),
            fields: vec![
                F { help: Some("Your GitHub username for API auth and git push.".into()), ..f("username", "GitHub Username") },
                F { help: Some("Full name for git commits (git config user.name).".into()), ..f("git_name", "Git Name") },
                F { help: Some("Email for git commits (git config user.email). Must match GitHub account.".into()), ..f("git_email", "Git Email") },
                F { full_width: true, help: Some("PAT with repo scope. Starts with ghp_ or github_pat_.".into()), ..fp("token", "Personal Access Token") },
            ],
            ..Default::default()
        },
        CredentialTypeDef {
            kind: "gitlab".into(),
            title: "GitLab".into(),
            description: "GitLab API and git operations.".into(),
            fields: vec![
                F { full_width: true, placeholder: Some("https://gitlab.com".into()), help: Some("GitLab instance URL. Use https://gitlab.com for SaaS.".into()), ..f("url", "Instance URL") },
                F { help: Some("Your GitLab username for API auth and git push.".into()), ..f("username", "GitLab Username") },
                F { full_width: true, help: Some("PAT with read_repository and write_repository scope.".into()), ..fp("token", "Personal Access Token") },
                F { help: Some("Full name for git commits (git config user.name).".into()), ..f("git_name", "Git Name") },
                F { help: Some("Email for git commits (git config user.email).".into()), ..f("git_email", "Git Email") },
            ],
            ..Default::default()
        },
        CredentialTypeDef {
            kind: "jwt_signing_key".into(),
            title: "JWT Signing Key".into(),
            description: "JWT signing for auth tokens and webhook verification.".into(),
            fields: vec![
                F {
                    field_type: "select".into(),
                    default: Some("HS256".into()),
                    options: vec![
                        O { value: "HS256".into(), label: "HS256 — HMAC-SHA256 (symmetric)".into() },
                        O { value: "HS384".into(), label: "HS384 — HMAC-SHA384 (symmetric)".into() },
                        O { value: "HS512".into(), label: "HS512 — HMAC-SHA512 (symmetric)".into() },
                        O { value: "RS256".into(), label: "RS256 — RSA-PKCS1v15-SHA256 (asymmetric)".into() },
                        O { value: "RS384".into(), label: "RS384 — RSA-PKCS1v15-SHA384 (asymmetric)".into() },
                        O { value: "RS512".into(), label: "RS512 — RSA-PKCS1v15-SHA512 (asymmetric)".into() },
                        O { value: "ES256".into(), label: "ES256 — ECDSA P-256 (asymmetric)".into() },
                        O { value: "ES384".into(), label: "ES384 — ECDSA P-384 (asymmetric)".into() },
                    ],
                    help: Some("JWT signing algorithm. HS* uses a shared secret; RS*/ES* use a private key.".into()),
                    ..f("algorithm", "Algorithm")
                },
                F { full_width: true, generate: Some("random_hex_32".into()), help: Some("Secret for HS* algorithms. Click Generate for a secure 256-bit random value.".into()), ..fp("secret", "HMAC Secret") },
                F { field_type: "textarea".into(), rows: Some(6), full_width: true, help: Some("PEM private key for RS*/ES* algorithms. Leave blank for HS*.".into()), ..f("private_key", "Private Key (PEM)") },
                F { placeholder: Some("/login".into()), help: Some("Where to redirect when the token is missing or invalid. Leave blank to return 401 JSON.".into()), ..f("auth_redirect", "Unauthenticated Redirect") },
                F { placeholder: Some("/403".into()), help: Some("Where to redirect when the token is valid but the role is insufficient. Leave blank to return 403 JSON.".into()), ..f("auth_forbidden_redirect", "Forbidden Redirect") },
                F { field_type: "tags".into(), full_width: true, placeholder: Some("e.g. admin".into()), help: Some("Roles available for this credential. Used by webhook nodes to populate the Required Role checkboxes.".into()), ..f("auth_roles", "Allowed Roles") },
            ],
            ..Default::default()
        },
        CredentialTypeDef {
            kind: "browser_browserless".into(),
            title: "Browserless".into(),
            description: "Browserless browser automation service.".into(),
            fields: vec![
                F { full_width: true, placeholder: Some("http://localhost:3000".into()), help: Some("Browserless instance root URL. Self-hosted or cloud endpoint.".into()), ..f("url", "URL") },
                F { full_width: true, help: Some("Optional API token. Leave blank for unauthenticated self-hosted instances.".into()), ..fp("token", "Token") },
            ],
            ..Default::default()
        },
        CredentialTypeDef {
            kind: "oauth2".into(),
            title: "OAuth2".into(),
            description: "OAuth2 authorization code flow with automatic token refresh.".into(),
            fields: vec![
                F { help: Some("OAuth2 client identifier.".into()), ..f("client_id", "Client ID") },
                F { help: Some("OAuth2 client secret.".into()), ..fp("client_secret", "Client Secret") },
                F { full_width: true, placeholder: Some("https://provider.com/oauth/token".into()), help: Some("Token endpoint URL.".into()), ..f("token_url", "Token URL") },
            ],
            ..Default::default()
        },
        CredentialTypeDef {
            kind: "hmac".into(),
            title: "HMAC".into(),
            description: "HMAC signing secret for webhook verification.".into(),
            fields: vec![
                F { full_width: true, generate: Some("random_hex_32".into()), help: Some("HMAC shared secret.".into()), ..fp("secret", "Secret") },
            ],
            ..Default::default()
        },
        CredentialTypeDef {
            kind: "api_key".into(),
            title: "API Key".into(),
            description: "Static API key for X-API-Key or Authorization header.".into(),
            fields: vec![
                F { full_width: true, generate: Some("random_hex_32".into()), help: Some("API key value.".into()), ..fp("key", "Key") },
            ],
            ..Default::default()
        },
        CredentialTypeDef {
            kind: "tts".into(),
            title: "TTS".into(),
            description: "Local text-to-speech runtime binding. Paths are Zebflow FS object paths. For Piper, point to the ONNX model, its JSON config, and the espeak-ng-data directory.".into(),
            fields: vec![
                F {
                    field_type: "select".into(),
                    default: Some("piper".into()),
                    options: vec![O { value: "piper".into(), label: "Piper".into() }],
                    help: Some("TTS engine provider.".into()),
                    ..f("provider", "Provider")
                },
                F { placeholder: Some("narrator".into()), help: Some("Optional human label for this voice preset.".into()), ..f("voice", "Voice Label") },
                F { full_width: true, placeholder: Some("voices/narrator/narrator.onnx".into()), help: Some("Private-relative ONNX model path.".into()), ..f("model_file", "Model File") },
                F { full_width: true, placeholder: Some("voices/narrator/narrator.onnx.json".into()), help: Some("Private-relative Piper JSON config path.".into()), ..f("config_file", "Config File") },
                F { full_width: true, placeholder: Some("runtime/espeak-ng-data".into()), help: Some("Private-relative directory path to espeak-ng-data.".into()), ..f("espeak_data_dir", "Espeak Data Dir") },
            ],
            ..Default::default()
        },
        CredentialTypeDef {
            kind: "secure_request".into(),
            title: "Secure Request".into(),
            description: "HTTP request template with secret placeholders and runtime variable bindings.".into(),
            fields: vec![], // Complex UI rendered client-side
            ..Default::default()
        },
        CredentialTypeDef {
            kind: "smtp".into(),
            title: "SMTP".into(),
            description: "Outgoing mail submission — a relay account, not a mail server."
                .into(),
            fields: vec![
                F { help: Some("SMTP relay hostname, e.g. smtp.fastmail.com.".into()), ..f("host", "Host") },
                F { placeholder: Some("587".into()), help: Some("587 = STARTTLS submission (default), 465 = implicit TLS.".into()), ..f("port", "Port") },
                F { help: Some("Relay login username.".into()), ..f("user", "User") },
                F { full_width: true, help: Some("Relay login password or app password.".into()), ..fp("password", "Password") },
                F { placeholder: Some("Zebflow <no-reply@example.com>".into()), full_width: true, help: Some("Default From address; a send may override it.".into()), ..f("from", "From") },
                F { placeholder: Some("starttls".into()), help: Some("starttls (default), tls, starttls-insecure (encrypted, certificate unchecked — a self-hosted server without a signed certificate yet, on a private network only), or none (a localhost test sink).".into()), ..f("tls", "TLS Mode") },
            ],
            ..Default::default()
        },
        CredentialTypeDef {
            kind: "s3".into(),
            title: "S3 object store".into(),
            description: "An S3-compatible bucket — AWS S3, Cloudflare R2, MinIO, SeaweedFS, Garage, Backblaze B2, Tigris. Select it on the Files page to make it the store this project keeps its files in.".into(),
            fields: vec![
                F { placeholder: Some("https://s3.amazonaws.com".into()), full_width: true, help: Some("Scheme and host, no path: https://s3.amazonaws.com, https://<account>.r2.cloudflarestorage.com, or http://127.0.0.1:8333 for a local SeaweedFS.".into()), ..f("endpoint", "Endpoint") },
                F { help: Some("Bucket name. The bucket must already exist.".into()), ..f("bucket", "Bucket") },
                F { placeholder: Some("us-east-1".into()), help: Some("Signing region: us-east-1 for most self-hosted stores (default), auto for R2.".into()), ..f("region", "Region") },
                F { placeholder: Some("projects/demo".into()), help: Some("Key prefix every object of this project lives under; empty means the bucket root. Lets several projects share one bucket.".into()), ..f("prefix", "Prefix") },
                F { ..f("access_key_id", "Access key ID") },
                F { full_width: true, ..fp("secret_access_key", "Secret access key") },
                F { placeholder: Some("path".into()), help: Some("path (default): endpoint/bucket/key — MinIO, SeaweedFS, Garage, R2. virtual: bucket.endpoint/key — AWS's default.".into()), ..f("addressing", "Addressing") },
            ],
            ..Default::default()
        },
        CredentialTypeDef {
            kind: "custom".into(),
            title: "Custom".into(),
            description: "Freeform JSON secret for custom integrations.".into(),
            fields: vec![
                F { field_type: "textarea".into(), rows: Some(10), full_width: true, placeholder: Some("{\n  \"key\": \"value\"\n}".into()), help: Some("Stored as raw JSON object for custom nodes.".into()), ..f("json", "Secret JSON") },
            ],
            ..Default::default()
        },
    ]
}

impl Default for CredentialTypeDef {
    fn default() -> Self {
        Self {
            kind: String::new(),
            title: String::new(),
            description: String::new(),
            fields: Vec::new(),
            placeholders: HashMap::new(),
            config_key: String::new(),
        }
    }
}

impl Default for CredentialFieldDef {
    fn default() -> Self {
        Self {
            key: String::new(),
            label: String::new(),
            field_type: "text".into(),
            required: false,
            placeholder: None,
            help: None,
            default: None,
            generate: None,
            options: Vec::new(),
            full_width: false,
            rows: None,
        }
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
