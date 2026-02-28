//! Sekejap-backed data adapter used by default in Zebflow platform.

use std::path::Path;
use std::sync::Arc;

use sekejap::SekejapDB;
use serde_json::{Value, json};

use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    PipelineMeta, PlatformProject, PlatformUser, ProjectCredential, ProjectPolicy,
    ProjectPolicyBinding, StoredUser, normalize_virtual_path, slug_segment,
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

    fn pipeline_slug(owner: &str, project: &str, virtual_path: &str, name: &str) -> String {
        let vp = normalize_virtual_path(virtual_path)
            .trim_start_matches('/')
            .replace('/', "__");
        format!(
            "pipeline_meta/{}/{}/{}/{}",
            slug_segment(owner),
            slug_segment(project),
            slug_segment(&vp),
            slug_segment(name)
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
        let v: Value = serde_json::from_str(&raw)?;
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
        let v: Value = serde_json::from_str(&raw)?;
        Ok(Some(PlatformProject {
            owner: Self::pick_non_empty(v.get("owner").and_then(Value::as_str), owner),
            project: Self::pick_non_empty(v.get("project").and_then(Value::as_str), project),
            title: v
                .get("title")
                .and_then(Value::as_str)
                .unwrap_or(project)
                .to_string(),
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
            "title": project.title,
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
                    project: project.clone(),
                    title: v
                        .get("title")
                        .and_then(Value::as_str)
                        .unwrap_or(&project)
                        .to_string(),
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

    fn put_project_credential(
        &self,
        credential: &ProjectCredential,
    ) -> Result<(), PlatformError> {
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

    fn put_pipeline_meta(&self, meta: &PipelineMeta) -> Result<(), PlatformError> {
        let data = json!({
            "_id": Self::pipeline_slug(&meta.owner, &meta.project, &meta.virtual_path, &meta.name),
            "_collection": "pipeline_meta",
            "owner": meta.owner,
            "project": meta.project,
            "name": meta.name,
            "title": meta.title,
            "virtual_path": normalize_virtual_path(&meta.virtual_path),
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
        let mut out = rows
            .into_iter()
            .filter_map(|v| {
                let name = v.get("name").and_then(Value::as_str)?.trim().to_string();
                if name.is_empty() {
                    return None;
                }
                let fallback_title = name.clone();
                Some(PipelineMeta {
                    owner: Self::pick_non_empty(v.get("owner").and_then(Value::as_str), owner),
                    project: Self::pick_non_empty(
                        v.get("project").and_then(Value::as_str),
                        project,
                    ),
                    name: name.clone(),
                    title: v
                        .get("title")
                        .and_then(Value::as_str)
                        .unwrap_or(&fallback_title)
                        .to_string(),
                    virtual_path: normalize_virtual_path(
                        v.get("virtual_path").and_then(Value::as_str).unwrap_or("/"),
                    ),
                    file_rel_path: v
                        .get("file_rel_path")
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
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
                })
            })
            .collect::<Vec<_>>();
        out.sort_by(|a, b| {
            a.virtual_path
                .cmp(&b.virtual_path)
                .then(a.name.cmp(&b.name))
        });
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
}
