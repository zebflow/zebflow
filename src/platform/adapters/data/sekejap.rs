//! Sekejap-backed data adapter used by default in Zebflow platform.

use std::path::Path;
use std::sync::Arc;

use sekejap::SekejapDB;
use serde_json::{Value, json};

use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    McpSession, PipelineInvocationEntry, PipelineMeta, PlatformProject, PlatformUser,
    ProjectCredential, ProjectDbConnection, ProjectPolicy, ProjectPolicyBinding, StoredUser,
    normalize_virtual_path, slug_segment,
};

const QUERY_LIMIT: usize = 10_000;

/// Data adapter using SekejapDB node collection storage.
pub struct SekejapDataAdapter {
    db: Arc<SekejapDB>,
}

impl SekejapDataAdapter {
    /// Opens/creates a Sekejap database under `{data_root}/platform/catalog`.
    pub fn new(data_root: &Path) -> Result<Self, PlatformError> {
        let main = data_root.join("platform").join("catalog");
        std::fs::create_dir_all(&main)?;
        let db = SekejapDB::new(&main, 2_000_000)
            .map_err(|e| PlatformError::new("PLATFORM_SEKEJAP_OPEN", e.to_string()))?;
        Ok(Self { db: Arc::new(db) })
    }

    fn user_slug(owner: &str) -> String {
        format!("user/{}", slug_segment(owner))
    }

    fn project_slug(owner: &str, project: &str) -> String {
        format!("project/{}/{}", slug_segment(owner), slug_segment(project))
    }

    fn project_credential_slug(owner: &str, project: &str, credential_id: &str) -> String {
        format!(
            "project_credential/{}/{}/{}",
            slug_segment(owner),
            slug_segment(project),
            slug_segment(credential_id)
        )
    }

    fn project_db_connection_slug(owner: &str, project: &str, connection_slug: &str) -> String {
        format!(
            "project_db_connection/{}/{}/{}",
            slug_segment(owner),
            slug_segment(project),
            slug_segment(connection_slug)
        )
    }

    fn pipeline_slug(owner: &str, project: &str, file_rel_path: &str) -> String {
        // Normalize file_rel_path into a flat slug key:
        // "pipelines/api/foo.zf.json" → "pipelines-api-foo-zf-json"
        let key = file_rel_path
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("-");
        format!(
            "pipeline_meta/{}/{}/{}",
            slug_segment(owner),
            slug_segment(project),
            key
        )
    }

    fn project_policy_slug(owner: &str, project: &str, policy_id: &str) -> String {
        format!(
            "project_policy/{}/{}/{}",
            slug_segment(owner),
            slug_segment(project),
            slug_segment(policy_id)
        )
    }

    fn project_policy_binding_slug(
        owner: &str,
        project: &str,
        subject_kind: &str,
        subject_id: &str,
        policy_id: &str,
    ) -> String {
        format!(
            "project_policy_binding/{}/{}/{}/{}/{}",
            slug_segment(owner),
            slug_segment(project),
            slug_segment(subject_kind),
            slug_segment(subject_id),
            slug_segment(policy_id)
        )
    }

    fn pick_non_empty(value: Option<&str>, fallback: &str) -> String {
        let v = value.unwrap_or(fallback).trim();
        if v.is_empty() {
            fallback.to_string()
        } else {
            v.to_string()
        }
    }

    fn query_payloads(&self, pipeline: Vec<Value>) -> Result<Vec<Value>, PlatformError> {
        let q = json!({ "pipeline": pipeline }).to_string();
        let out = self
            .db
            .query(&q)
            .map_err(|e| PlatformError::new("PLATFORM_SEKEJAP_QUERY", e.to_string()))?;
        let mut rows = Vec::new();
        for hit in out.data {
            if let Some(payload) = hit.payload
                && let Ok(v) = serde_json::from_str::<Value>(&payload)
            {
                rows.push(v);
            }
        }
        Ok(rows)
    }

    /// Execute a raw SekejapQL pipeline JSON string and return payload values.
    pub fn raw_query(&self, pipeline_json: &str) -> Result<Vec<Value>, PlatformError> {
        let out = self
            .db
            .query(pipeline_json)
            .map_err(|e| PlatformError::new("PLATFORM_SEKEJAP_QUERY", e.to_string()))?;
        let mut rows = Vec::new();
        for hit in out.data {
            if let Some(payload) = hit.payload
                && let Ok(v) = serde_json::from_str::<Value>(&payload)
            {
                rows.push(v);
            }
        }
        Ok(rows)
    }

    /// Get a raw node by slug, returns the JSON string payload if present.
    pub fn get_node_raw(&self, slug: &str) -> Option<String> {
        self.db.nodes().get(slug)
    }

    /// Delete a node by slug. Returns true if the node existed.
    pub fn delete_node_raw(&self, slug: &str) -> Result<bool, PlatformError> {
        match self.db.nodes().remove(slug) {
            Ok(()) => Ok(true),
            Err(e) => {
                let msg = e.to_string();
                if msg.contains("not found") || msg.contains("NotFound") {
                    Ok(false)
                } else {
                    Err(PlatformError::new("PLATFORM_SEKEJAP_MUTATE", msg))
                }
            }
        }
    }

    /// List known collection names with approximate row counts.
    /// Uses the fixed set of collections known to this adapter.
    pub fn list_collections(&self) -> Vec<(String, usize)> {
        let known = [
            "user",
            "project",
            "project_credential",
            "project_db_connection",
            "project_assistant_config",
            "pipeline_meta",
            "project_policy",
            "project_policy_binding",
            "mcp_session",
        ];
        known
            .iter()
            .map(|name| {
                let count = self
                    .query_payloads(vec![
                        json!({"op": "collection", "name": name}),
                        json!({"op": "take", "n": 1_000_000}),
                    ])
                    .map(|rows| rows.len())
                    .unwrap_or(0);
                (name.to_string(), count)
            })
            .collect()
    }
}

impl DataAdapter for SekejapDataAdapter {
    fn id(&self) -> &'static str {
        "data.sekejap"
    }

    fn get_user_auth(&self, owner: &str) -> Result<Option<StoredUser>, PlatformError> {
        let slug = Self::user_slug(owner);
        let Some(raw) = self.db.nodes().get(&slug) else {
            return Ok(None);
        };
        let v: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_err) => {
                // Corrupted legacy value should not hard-fail auth paths; treat as missing so bootstrap/upsert can heal it.
                return Ok(None);
            }
        };
        let profile = PlatformUser {
            owner: Self::pick_non_empty(v.get("owner").and_then(Value::as_str), owner),
            role: v
                .get("role")
                .and_then(Value::as_str)
                .unwrap_or("member")
                .to_string(),
            created_at: v.get("created_at").and_then(Value::as_i64).unwrap_or(0),
            updated_at: v.get("updated_at").and_then(Value::as_i64).unwrap_or(0),
        };
        let password = v
            .get("password")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        Ok(Some(StoredUser { profile, password }))
    }

    fn put_user(&self, user: &StoredUser) -> Result<(), PlatformError> {
        let data = json!({
            "_id": Self::user_slug(&user.profile.owner),
            "_collection": "user",
            "owner": user.profile.owner,
            "role": user.profile.role,
            "password": user.password,
            "created_at": user.profile.created_at,
            "updated_at": user.profile.updated_at,
        });
        let op = json!({"mutation":"put_json", "data": data}).to_string();
        self.db
            .mutate(&op)
            .map_err(|e| PlatformError::new("PLATFORM_SEKEJAP_MUTATE", e.to_string()))?;
        Ok(())
    }

    fn list_users(&self) -> Result<Vec<PlatformUser>, PlatformError> {
        let rows = self.query_payloads(vec![
            json!({"op":"collection","name":"user"}),
            json!({"op":"take","n":QUERY_LIMIT}),
        ])?;
        let mut users = rows
            .into_iter()
            .filter_map(|v| {
                let owner = v.get("owner").and_then(Value::as_str)?.to_string();
                Some(PlatformUser {
                    owner,
                    role: v
                        .get("role")
                        .and_then(Value::as_str)
                        .unwrap_or("member")
                        .to_string(),
                    created_at: v.get("created_at").and_then(Value::as_i64).unwrap_or(0),
                    updated_at: v.get("updated_at").and_then(Value::as_i64).unwrap_or(0),
                })
            })
            .collect::<Vec<_>>();
        users.sort_by(|a, b| a.owner.cmp(&b.owner));
        Ok(users)
    }

    fn get_project(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Option<PlatformProject>, PlatformError> {
        let slug = Self::project_slug(owner, project);
        let Some(raw) = self.db.nodes().get(&slug) else {
            return Ok(None);
        };
        let v: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_err) => {
                // Corrupted legacy value should not block bootstrap/upsert; treat as missing so writer can heal it.
                return Ok(None);
            }
        };
        Ok(Some(PlatformProject {
            owner: Self::pick_non_empty(v.get("owner").and_then(Value::as_str), owner),
            project: Self::pick_non_empty(v.get("project").and_then(Value::as_str), project),
            title: String::new(), // populated from zebflow.json by ProjectService
            created_at: v.get("created_at").and_then(Value::as_i64).unwrap_or(0),
            updated_at: v.get("updated_at").and_then(Value::as_i64).unwrap_or(0),
        }))
    }

    fn put_project(&self, project: &PlatformProject) -> Result<(), PlatformError> {
        let data = json!({
            "_id": Self::project_slug(&project.owner, &project.project),
            "_collection": "project",
            "owner": project.owner,
            "project": project.project,
            "created_at": project.created_at,
            "updated_at": project.updated_at,
        });
        let op = json!({"mutation":"put_json", "data": data}).to_string();
        self.db
            .mutate(&op)
            .map_err(|e| PlatformError::new("PLATFORM_SEKEJAP_MUTATE", e.to_string()))?;
        Ok(())
    }

    fn list_projects(&self, owner: &str) -> Result<Vec<PlatformProject>, PlatformError> {
        let rows = self.query_payloads(vec![
            json!({"op":"collection","name":"project"}),
            json!({"op":"where_eq","field":"owner","value":owner}),
            json!({"op":"take","n":QUERY_LIMIT}),
        ])?;
        let mut projects = rows
            .into_iter()
            .filter_map(|v| {
                let project = v.get("project").and_then(Value::as_str)?.trim().to_string();
                if project.is_empty() {
                    return None;
                }
                Some(PlatformProject {
                    owner: Self::pick_non_empty(v.get("owner").and_then(Value::as_str), owner),
                    project,
                    title: String::new(), // populated from zebflow.json by ProjectService
                    created_at: v.get("created_at").and_then(Value::as_i64).unwrap_or(0),
                    updated_at: v.get("updated_at").and_then(Value::as_i64).unwrap_or(0),
                })
            })
            .collect::<Vec<_>>();
        projects.sort_by(|a, b| a.project.cmp(&b.project));
        Ok(projects)
    }

    fn get_project_credential(
        &self,
        owner: &str,
        project: &str,
        credential_id: &str,
    ) -> Result<Option<ProjectCredential>, PlatformError> {
        let slug = Self::project_credential_slug(owner, project, credential_id);
        let Some(raw) = self.db.nodes().get(&slug) else {
            return Ok(None);
        };
        let v: Value = serde_json::from_str(&raw)?;
        Ok(Some(ProjectCredential {
            owner: Self::pick_non_empty(v.get("owner").and_then(Value::as_str), owner),
            project: Self::pick_non_empty(v.get("project").and_then(Value::as_str), project),
            credential_id: Self::pick_non_empty(
                v.get("credential_id").and_then(Value::as_str),
                credential_id,
            ),
            title: v
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or(credential_id)
                .to_string(),
            kind: v
                .get("kind")
                .and_then(Value::as_str)
                .unwrap_or("generic")
                .to_string(),
            secret: v.get("secret").cloned().unwrap_or(Value::Null),
            notes: v
                .get("notes")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            created_at: v.get("created_at").and_then(Value::as_i64).unwrap_or(0),
            updated_at: v.get("updated_at").and_then(Value::as_i64).unwrap_or(0),
        }))
    }

    fn put_project_credential(&self, credential: &ProjectCredential) -> Result<(), PlatformError> {
        let data = json!({
            "_id": Self::project_credential_slug(
                &credential.owner,
                &credential.project,
                &credential.credential_id,
            ),
            "_collection": "project_credential",
            "owner": credential.owner,
            "project": credential.project,
            "credential_id": credential.credential_id,
            "title": credential.title,
            "kind": credential.kind,
            "secret": credential.secret,
            "notes": credential.notes,
            "created_at": credential.created_at,
            "updated_at": credential.updated_at,
        });
        let op = json!({"mutation":"put_json", "data": data}).to_string();
        self.db
            .mutate(&op)
            .map_err(|e| PlatformError::new("PLATFORM_SEKEJAP_MUTATE", e.to_string()))?;
        Ok(())
    }

    fn list_project_credentials(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<ProjectCredential>, PlatformError> {
        let rows = self.query_payloads(vec![
            json!({"op":"collection","name":"project_credential"}),
            json!({"op":"where_eq","field":"owner","value":owner}),
            json!({"op":"where_eq","field":"project","value":project}),
            json!({"op":"take","n":QUERY_LIMIT}),
        ])?;
        let mut credentials = rows
            .into_iter()
            .filter_map(|v| {
                let credential_id = v
                    .get("credential_id")
                    .and_then(Value::as_str)?
                    .trim()
                    .to_string();
                if credential_id.is_empty() {
                    return None;
                }
                Some(ProjectCredential {
                    owner: Self::pick_non_empty(v.get("owner").and_then(Value::as_str), owner),
                    project: Self::pick_non_empty(
                        v.get("project").and_then(Value::as_str),
                        project,
                    ),
                    credential_id: credential_id.clone(),
                    title: v
                        .get("title")
                        .and_then(Value::as_str)
                        .unwrap_or(&credential_id)
                        .to_string(),
                    kind: v
                        .get("kind")
                        .and_then(Value::as_str)
                        .unwrap_or("generic")
                        .to_string(),
                    secret: v.get("secret").cloned().unwrap_or(Value::Null),
                    notes: v
                        .get("notes")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    created_at: v.get("created_at").and_then(Value::as_i64).unwrap_or(0),
                    updated_at: v.get("updated_at").and_then(Value::as_i64).unwrap_or(0),
                })
            })
            .collect::<Vec<_>>();
        credentials.sort_by(|a, b| a.credential_id.cmp(&b.credential_id));
        Ok(credentials)
    }

    fn delete_project_credential(
        &self,
        owner: &str,
        project: &str,
        credential_id: &str,
    ) -> Result<(), PlatformError> {
        let slug = Self::project_credential_slug(owner, project, credential_id);
        self.db
            .nodes()
            .remove(&slug)
            .map_err(|e| PlatformError::new("PLATFORM_SEKEJAP_MUTATE", e.to_string()))?;
        Ok(())
    }

    fn get_project_db_connection(
        &self,
        owner: &str,
        project: &str,
        connection_slug: &str,
    ) -> Result<Option<ProjectDbConnection>, PlatformError> {
        let slug = Self::project_db_connection_slug(owner, project, connection_slug);
        let Some(raw) = self.db.nodes().get(&slug) else {
            return Ok(None);
        };
        let v: Value = serde_json::from_str(&raw)?;
        Ok(Some(ProjectDbConnection {
            owner: Self::pick_non_empty(v.get("owner").and_then(Value::as_str), owner),
            project: Self::pick_non_empty(v.get("project").and_then(Value::as_str), project),
            connection_id: Self::pick_non_empty(
                v.get("connection_id").and_then(Value::as_str),
                connection_slug,
            ),
            connection_slug: Self::pick_non_empty(
                v.get("connection_slug").and_then(Value::as_str),
                connection_slug,
            ),
            connection_label: v
                .get("connection_label")
                .and_then(Value::as_str)
                .unwrap_or(connection_slug)
                .to_string(),
            database_kind: v
                .get("database_kind")
                .and_then(Value::as_str)
                .unwrap_or("sekejap")
                .to_string(),
            credential_id: v
                .get("credential_id")
                .and_then(Value::as_str)
                .map(ToString::to_string),
            config: v.get("config").cloned().unwrap_or(Value::Null),
            created_at: v.get("created_at").and_then(Value::as_i64).unwrap_or(0),
            updated_at: v.get("updated_at").and_then(Value::as_i64).unwrap_or(0),
        }))
    }

    fn put_project_db_connection(
        &self,
        connection: &ProjectDbConnection,
    ) -> Result<(), PlatformError> {
        let data = json!({
            "_id": Self::project_db_connection_slug(
                &connection.owner,
                &connection.project,
                &connection.connection_slug,
            ),
            "_collection": "project_db_connection",
            "owner": connection.owner,
            "project": connection.project,
            "connection_id": connection.connection_id,
            "connection_slug": connection.connection_slug,
            "connection_label": connection.connection_label,
            "database_kind": connection.database_kind,
            "credential_id": connection.credential_id,
            "config": connection.config,
            "created_at": connection.created_at,
            "updated_at": connection.updated_at,
        });
        let op = json!({"mutation":"put_json", "data": data}).to_string();
        self.db
            .mutate(&op)
            .map_err(|e| PlatformError::new("PLATFORM_SEKEJAP_MUTATE", e.to_string()))?;
        Ok(())
    }

    fn list_project_db_connections(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<ProjectDbConnection>, PlatformError> {
        let rows = self.query_payloads(vec![
            json!({"op":"collection","name":"project_db_connection"}),
            json!({"op":"where_eq","field":"owner","value":owner}),
            json!({"op":"where_eq","field":"project","value":project}),
            json!({"op":"take","n":QUERY_LIMIT}),
        ])?;
        let mut items = rows
            .into_iter()
            .filter_map(|v| {
                let connection_slug = v
                    .get("connection_slug")
                    .and_then(Value::as_str)?
                    .trim()
                    .to_string();
                if connection_slug.is_empty() {
                    return None;
                }
                Some(ProjectDbConnection {
                    owner: Self::pick_non_empty(v.get("owner").and_then(Value::as_str), owner),
                    project: Self::pick_non_empty(
                        v.get("project").and_then(Value::as_str),
                        project,
                    ),
                    connection_id: Self::pick_non_empty(
                        v.get("connection_id").and_then(Value::as_str),
                        &connection_slug,
                    ),
                    connection_slug: connection_slug.clone(),
                    connection_label: v
                        .get("connection_label")
                        .and_then(Value::as_str)
                        .unwrap_or(&connection_slug)
                        .to_string(),
                    database_kind: v
                        .get("database_kind")
                        .and_then(Value::as_str)
                        .unwrap_or("sekejap")
                        .to_string(),
                    credential_id: v
                        .get("credential_id")
                        .and_then(Value::as_str)
                        .map(ToString::to_string),
                    config: v.get("config").cloned().unwrap_or(Value::Null),
                    created_at: v.get("created_at").and_then(Value::as_i64).unwrap_or(0),
                    updated_at: v.get("updated_at").and_then(Value::as_i64).unwrap_or(0),
                })
            })
            .collect::<Vec<_>>();
        items.sort_by(|a, b| a.connection_slug.cmp(&b.connection_slug));
        Ok(items)
    }

    fn delete_project_db_connection(
        &self,
        owner: &str,
        project: &str,
        connection_slug: &str,
    ) -> Result<(), PlatformError> {
        let slug = Self::project_db_connection_slug(owner, project, connection_slug);
        self.db
            .nodes()
            .remove(&slug)
            .map_err(|e| PlatformError::new("PLATFORM_SEKEJAP_MUTATE", e.to_string()))?;
        Ok(())
    }

    fn delete_pipeline_meta(
        &self,
        owner: &str,
        project: &str,
        file_rel_path: &str,
    ) -> Result<(), PlatformError> {
        let slug = Self::pipeline_slug(owner, project, file_rel_path);
        self.db
            .nodes()
            .remove(&slug)
            .map_err(|e| PlatformError::new("PLATFORM_SEKEJAP_MUTATE", e.to_string()))?;
        Ok(())
    }

    fn put_pipeline_meta(&self, meta: &PipelineMeta) -> Result<(), PlatformError> {
        let data = json!({
            "_id": Self::pipeline_slug(&meta.owner, &meta.project, &meta.file_rel_path),
            "_collection": "pipeline_meta",
            "owner": meta.owner,
            "project": meta.project,
            "name": meta.name,
            "title": meta.title,
            "file_rel_path": meta.file_rel_path,
            "description": meta.description,
            "trigger_kind": meta.trigger_kind,
            "hash": meta.hash,
            "active_hash": meta.active_hash,
            "activated_at": meta.activated_at,
            "created_at": meta.created_at,
            "updated_at": meta.updated_at,
        });
        let op = json!({"mutation":"put_json", "data": data}).to_string();
        self.db
            .mutate(&op)
            .map_err(|e| PlatformError::new("PLATFORM_SEKEJAP_MUTATE", e.to_string()))?;
        Ok(())
    }

    fn list_pipeline_meta(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<PipelineMeta>, PlatformError> {
        let rows = self.query_payloads(vec![
            json!({"op":"collection","name":"pipeline_meta"}),
            json!({"op":"where_eq","field":"owner","value":owner}),
            json!({"op":"where_eq","field":"project","value":project}),
            json!({"op":"take","n":QUERY_LIMIT}),
        ])?;
        let mut out = Vec::new();
        for v in rows {
            let name = match v.get("name").and_then(Value::as_str) {
                Some(n) if !n.trim().is_empty() => n.trim().to_string(),
                _ => continue,
            };

            let file_rel_path = v
                .get("file_rel_path")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();

            // Legacy entries with no file_rel_path are invalid — remove and skip.
            if file_rel_path.is_empty() {
                let stored_id = v.get("_id").and_then(Value::as_str).unwrap_or("");
                if !stored_id.is_empty() {
                    let _ = self.db.nodes().remove(stored_id);
                }
                continue;
            }

            let fallback_title = name.clone();
            let meta = PipelineMeta {
                owner: Self::pick_non_empty(v.get("owner").and_then(Value::as_str), owner),
                project: Self::pick_non_empty(v.get("project").and_then(Value::as_str), project),
                name: name.clone(),
                title: v
                    .get("title")
                    .and_then(Value::as_str)
                    .unwrap_or(&fallback_title)
                    .to_string(),
                // virtual_path is derived on read in the service layer; use stored value as
                // temporary fallback (will be overwritten by project.rs before use).
                virtual_path: normalize_virtual_path(
                    v.get("virtual_path").and_then(Value::as_str).unwrap_or("/"),
                ),
                file_rel_path: file_rel_path.clone(),
                description: v
                    .get("description")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                trigger_kind: v
                    .get("trigger_kind")
                    .and_then(Value::as_str)
                    .unwrap_or("webhook")
                    .to_string(),
                hash: v
                    .get("hash")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                active_hash: v
                    .get("active_hash")
                    .and_then(Value::as_str)
                    .map(ToString::to_string),
                activated_at: v.get("activated_at").and_then(Value::as_i64),
                created_at: v.get("created_at").and_then(Value::as_i64).unwrap_or(0),
                updated_at: v.get("updated_at").and_then(Value::as_i64).unwrap_or(0),
            };

            // Auto-migrate: if stored _id doesn't match the new file_rel_path key scheme,
            // re-save with the correct key and remove the stale old key.
            let expected_id = Self::pipeline_slug(owner, project, &file_rel_path);
            let stored_id = v.get("_id").and_then(Value::as_str).unwrap_or("");
            if !stored_id.is_empty() && stored_id != expected_id {
                let old = stored_id.to_string();
                let _ = self.put_pipeline_meta(&meta);
                let _ = self.db.nodes().remove(&old);
            }

            out.push(meta);
        }
        out.sort_by(|a, b| a.file_rel_path.cmp(&b.file_rel_path));
        Ok(out)
    }

    fn put_project_policy(&self, policy: &ProjectPolicy) -> Result<(), PlatformError> {
        let data = json!({
            "_id": Self::project_policy_slug(&policy.owner, &policy.project, &policy.policy_id),
            "_collection": "project_policy",
            "owner": policy.owner,
            "project": policy.project,
            "policy_id": policy.policy_id,
            "title": policy.title,
            "capabilities": policy.capabilities,
            "managed": policy.managed,
            "created_at": policy.created_at,
            "updated_at": policy.updated_at,
        });
        let op = json!({"mutation":"put_json", "data": data}).to_string();
        self.db
            .mutate(&op)
            .map_err(|e| PlatformError::new("PLATFORM_SEKEJAP_MUTATE", e.to_string()))?;
        Ok(())
    }

    fn list_project_policies(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<ProjectPolicy>, PlatformError> {
        let rows = self.query_payloads(vec![
            json!({"op":"collection","name":"project_policy"}),
            json!({"op":"where_eq","field":"owner","value":owner}),
            json!({"op":"where_eq","field":"project","value":project}),
            json!({"op":"take","n":QUERY_LIMIT}),
        ])?;
        let mut out = rows
            .into_iter()
            .filter_map(|v| serde_json::from_value::<ProjectPolicy>(v).ok())
            .collect::<Vec<_>>();
        out.sort_by(|a, b| a.policy_id.cmp(&b.policy_id));
        Ok(out)
    }

    fn put_project_policy_binding(
        &self,
        binding: &ProjectPolicyBinding,
    ) -> Result<(), PlatformError> {
        let data = json!({
            "_id": Self::project_policy_binding_slug(
                &binding.owner,
                &binding.project,
                binding.subject_kind.key(),
                &binding.subject_id,
                &binding.policy_id,
            ),
            "_collection": "project_policy_binding",
            "owner": binding.owner,
            "project": binding.project,
            "subject_kind": binding.subject_kind,
            "subject_id": binding.subject_id,
            "policy_id": binding.policy_id,
            "created_at": binding.created_at,
            "updated_at": binding.updated_at,
        });
        let op = json!({"mutation":"put_json", "data": data}).to_string();
        self.db
            .mutate(&op)
            .map_err(|e| PlatformError::new("PLATFORM_SEKEJAP_MUTATE", e.to_string()))?;
        Ok(())
    }

    fn list_project_policy_bindings(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<ProjectPolicyBinding>, PlatformError> {
        let rows = self.query_payloads(vec![
            json!({"op":"collection","name":"project_policy_binding"}),
            json!({"op":"where_eq","field":"owner","value":owner}),
            json!({"op":"where_eq","field":"project","value":project}),
            json!({"op":"take","n":QUERY_LIMIT}),
        ])?;
        let mut out = rows
            .into_iter()
            .filter_map(|v| serde_json::from_value::<ProjectPolicyBinding>(v).ok())
            .collect::<Vec<_>>();
        out.sort_by(|a, b| {
            a.subject_kind
                .cmp(&b.subject_kind)
                .then(a.subject_id.cmp(&b.subject_id))
                .then(a.policy_id.cmp(&b.policy_id))
        });
        Ok(out)
    }

    fn delete_project_policy(
        &self,
        owner: &str,
        project: &str,
        policy_id: &str,
    ) -> Result<(), PlatformError> {
        let slug = Self::project_policy_slug(owner, project, policy_id);
        let op = json!({"mutation":"delete_node", "id": slug}).to_string();
        self.db
            .mutate(&op)
            .map_err(|e| PlatformError::new("PLATFORM_SEKEJAP_MUTATE", e.to_string()))?;
        Ok(())
    }

    fn delete_project_policy_binding(
        &self,
        owner: &str,
        project: &str,
        subject_id: &str,
    ) -> Result<(), PlatformError> {
        let bindings = self.list_project_policy_bindings(owner, project)?;
        for binding in bindings {
            if binding.subject_id == subject_id {
                let slug = Self::project_policy_binding_slug(
                    owner,
                    project,
                    &binding.subject_kind.key(),
                    subject_id,
                    &binding.policy_id,
                );
                let op = json!({"mutation":"delete_node", "id": slug}).to_string();
                self.db
                    .mutate(&op)
                    .map_err(|e| PlatformError::new("PLATFORM_SEKEJAP_MUTATE", e.to_string()))?;
            }
        }
        Ok(())
    }

    fn list_all_mcp_sessions(&self) -> Result<Vec<McpSession>, PlatformError> {
        let rows = self.query_payloads(vec![
            json!({"op": "collection", "name": "mcp_session"}),
            json!({"op": "take", "n": QUERY_LIMIT}),
        ])?;
        let sessions = rows
            .into_iter()
            .filter_map(|v| serde_json::from_value::<McpSession>(v).ok())
            .collect();
        Ok(sessions)
    }

    fn put_mcp_session(&self, session: &McpSession) -> Result<(), PlatformError> {
        let slug = format!(
            "mcp_session/{}/{}",
            slug_segment(&session.owner),
            slug_segment(&session.project)
        );
        let mut data = serde_json::to_value(session)
            .map_err(|e| PlatformError::new("PLATFORM_SEKEJAP_MUTATE", e.to_string()))?;
        data["_id"] = json!(slug);
        data["_collection"] = json!("mcp_session");
        let op = json!({"mutation": "put_json", "data": data}).to_string();
        self.db
            .mutate(&op)
            .map_err(|e| PlatformError::new("PLATFORM_SEKEJAP_MUTATE", e.to_string()))?;
        Ok(())
    }

    fn delete_mcp_session(&self, token: &str) -> Result<(), PlatformError> {
        // Find the session first to get owner/project for slug
        let sessions = self.list_all_mcp_sessions()?;
        if let Some(session) = sessions.iter().find(|s| s.token == token) {
            let slug = format!(
                "mcp_session/{}/{}",
                slug_segment(&session.owner),
                slug_segment(&session.project)
            );
            self.db
                .nodes()
                .remove(&slug)
                .map_err(|e| PlatformError::new("PLATFORM_SEKEJAP_MUTATE", e.to_string()))?;
        }
        Ok(())
    }

    fn admin_list_collections(&self) -> Result<Vec<(String, usize)>, PlatformError> {
        Ok(self.list_collections())
    }

    fn admin_raw_query(&self, pipeline_json: &str) -> Result<Vec<Value>, PlatformError> {
        self.raw_query(pipeline_json)
    }

    fn admin_get_node(&self, slug: &str) -> Result<Option<String>, PlatformError> {
        Ok(self.get_node_raw(slug))
    }

    fn admin_delete_node(&self, slug: &str) -> Result<bool, PlatformError> {
        self.delete_node_raw(slug)
    }

    fn log_pipeline_invocation(
        &self,
        owner: &str,
        project: &str,
        file_rel_path: &str,
        entry: &PipelineInvocationEntry,
        max_n: usize,
    ) -> Result<(), PlatformError> {
        let slug = format!(
            "invlog/{}/{}/{}",
            slug_segment(owner),
            slug_segment(project),
            slug_segment(file_rel_path),
        );
        // Read existing entries.
        let mut entries: Vec<Value> = if let Some(raw) = self.db.nodes().get(&slug) {
            serde_json::from_str::<Value>(&raw)
                .ok()
                .and_then(|v| v.get("entries").and_then(Value::as_array).cloned())
                .unwrap_or_default()
        } else {
            vec![]
        };
        // Prepend new entry, truncate to max_n.
        let new_entry = serde_json::to_value(entry)
            .map_err(|e| PlatformError::new("PIPELINE_LOG_SERIALIZE", e.to_string()))?;
        entries.insert(0, new_entry);
        entries.truncate(max_n);
        let data = json!({
            "_id": slug,
            "_collection": "invlog",
            "owner": owner,
            "project": project,
            "file_rel_path": file_rel_path,
            "entries": entries,
        });
        let op = json!({"mutation": "put_json", "data": data}).to_string();
        self.db
            .mutate(&op)
            .map_err(|e| PlatformError::new("PLATFORM_SEKEJAP_MUTATE", e.to_string()))?;
        Ok(())
    }

    fn get_pipeline_invocations(
        &self,
        owner: &str,
        project: &str,
        file_rel_path: &str,
    ) -> Result<Vec<PipelineInvocationEntry>, PlatformError> {
        let slug = format!(
            "invlog/{}/{}/{}",
            slug_segment(owner),
            slug_segment(project),
            slug_segment(file_rel_path),
        );
        let Some(raw) = self.db.nodes().get(&slug) else {
            return Ok(vec![]);
        };
        let entries: Vec<PipelineInvocationEntry> = serde_json::from_str::<Value>(&raw)
            .ok()
            .and_then(|v| v.get("entries").cloned())
            .and_then(|arr| serde_json::from_value(arr).ok())
            .unwrap_or_default();
        Ok(entries)
    }
}
