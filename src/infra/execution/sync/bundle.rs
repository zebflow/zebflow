//! Project runtime bundle model.
//!
//! A bundle is the portable unit used to move a project between runtimes. It is intentionally
//! smaller than “copy the whole platform database” and richer than “just clone the git repo”.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::infra::execution::placement::ProjectRuntimeProfile;

/// Stable project identity carried by a portable bundle.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectBundleIdentity {
    /// Owner identifier.
    pub owner: String,
    /// Project identifier.
    pub project: String,
    /// Human-readable title.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub title: String,
    /// Source revision label or commit-ish used when the bundle was prepared.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub revision: String,
}

/// Repo-owned activation/bootstrap intent.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectBootstrapPlan {
    /// Pipeline glob patterns that should auto-activate after clone/import.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub activate: Vec<String>,
}

/// Bundle content categories included in a transfer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectBundleContents {
    /// Include the git-tracked repo working tree.
    #[serde(default = "default_true")]
    pub include_repo: bool,
    /// Include public project files.
    #[serde(default)]
    pub include_files_public: bool,
    /// Include private project files.
    #[serde(default)]
    pub include_files_private: bool,
    /// Include an attached runtime snapshot reference.
    #[serde(default)]
    pub include_runtime_snapshot: bool,
}

impl Default for ProjectBundleContents {
    fn default() -> Self {
        Self {
            include_repo: true,
            include_files_public: false,
            include_files_private: false,
            include_runtime_snapshot: false,
        }
    }
}

const fn default_true() -> bool {
    true
}

/// One file carried inside a portable runtime bundle.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectBundleFile {
    /// Relative path inside the project repo or files area.
    pub path: String,
    /// File contents encoded as base64.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub data_base64: String,
}

/// Supported secret-binding requirement kinds.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SecretBindingKind {
    Credential,
    DbConnection,
    Environment,
}

/// One secret or environment binding required by a portable project bundle.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SecretBindingRequirement {
    /// Stable logical binding id inside the project bundle.
    pub binding_id: String,
    /// Binding category.
    pub kind: SecretBindingKind,
    /// Kind hint such as `postgres`, `openai`, or `jwt_signing_key`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub kind_hint: String,
    /// Whether the destination must provide this binding before cutover.
    #[serde(default)]
    pub required: bool,
    /// Human-readable purpose shown in migration UI.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub description: String,
}

impl Default for SecretBindingKind {
    fn default() -> Self {
        Self::Credential
    }
}

/// Safe manifest of secret bindings required by a project bundle.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SecretBindingsManifest {
    /// Required bindings, without secret values.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub bindings: Vec<SecretBindingRequirement>,
}

impl Default for SecretBindingsManifest {
    fn default() -> Self {
        Self {
            bindings: Vec::new(),
        }
    }
}

/// Portable description of repo-owned project runtime state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ProjectRuntimeBundle {
    /// Opaque bundle identifier.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub bundle_id: String,
    /// Stable project identity.
    #[serde(default)]
    pub identity: ProjectBundleIdentity,
    /// Unix timestamp when the bundle was prepared.
    #[serde(default)]
    pub created_at: i64,
    /// Portable runtime profile.
    #[serde(default)]
    pub runtime_profile: ProjectRuntimeProfile,
    /// Repo-owned activation/bootstrap intent.
    #[serde(default)]
    pub bootstrap: ProjectBootstrapPlan,
    /// What this bundle actually contains.
    #[serde(default)]
    pub contents: ProjectBundleContents,
    /// Repo files included in the bundle when `contents.include_repo == true`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub repo_files: Vec<ProjectBundleFile>,
    /// Secret bindings that must be satisfied on the destination.
    #[serde(default)]
    pub secret_bindings: SecretBindingsManifest,
    /// Freeform non-secret metadata for future evolution.
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub metadata: Value,
}

impl Default for ProjectRuntimeBundle {
    fn default() -> Self {
        Self {
            bundle_id: String::new(),
            identity: ProjectBundleIdentity::default(),
            created_at: 0,
            runtime_profile: ProjectRuntimeProfile::default(),
            bootstrap: ProjectBootstrapPlan::default(),
            contents: ProjectBundleContents::default(),
            repo_files: Vec::new(),
            secret_bindings: SecretBindingsManifest::default(),
            metadata: Value::Object(Map::new()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ProjectBundleContents, ProjectRuntimeBundle, SecretBindingsManifest};

    #[test]
    fn defaults_are_portable() {
        let bundle = ProjectRuntimeBundle::default();
        assert!(bundle.contents.include_repo);
        assert!(!bundle.contents.include_runtime_snapshot);

        let manifest = SecretBindingsManifest::default();
        assert!(manifest.bindings.is_empty());
    }

    #[test]
    fn repo_only_bundle_is_the_default_contents_shape() {
        let contents = ProjectBundleContents::default();
        assert!(contents.include_repo);
        assert!(!contents.include_files_public);
        assert!(!contents.include_files_private);
        assert!(!contents.include_runtime_snapshot);
    }
}
