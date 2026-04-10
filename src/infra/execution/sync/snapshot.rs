//! Runtime snapshot model.
//!
//! A runtime snapshot describes environment-owned project state that is not guaranteed to exist in
//! Git, such as local SQLite contents or runtime-managed files.

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Current runtime snapshot schema version.
pub const PROJECT_RUNTIME_SNAPSHOT_SCHEMA_VERSION: u32 = 1;

/// Named runtime snapshot scopes.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeSnapshotScope {
    LocalDb,
    RuntimeDir,
    FilesPublic,
    FilesPrivate,
}

/// Snapshot part materialization format.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeSnapshotFormat {
    SqliteFile,
    TarArchive,
    DirectoryManifest,
}

/// One captured runtime snapshot artifact.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct RuntimeSnapshotPart {
    /// Logical snapshot scope.
    pub scope: RuntimeSnapshotScope,
    /// Storage format used for the part.
    pub format: RuntimeSnapshotFormat,
    /// Relative path or object key for the artifact.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub path: String,
    /// Optional SHA-256 checksum of the part.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sha256: Option<String>,
    /// Optional size hint in bytes.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
}

impl Default for RuntimeSnapshotScope {
    fn default() -> Self {
        Self::LocalDb
    }
}

impl Default for RuntimeSnapshotFormat {
    fn default() -> Self {
        Self::DirectoryManifest
    }
}

/// Versioned runtime snapshot manifest.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ProjectRuntimeSnapshot {
    /// Snapshot schema version.
    #[serde(default = "default_project_runtime_snapshot_schema_version")]
    pub schema_version: u32,
    /// Opaque snapshot identifier.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub snapshot_id: String,
    /// Owner identifier.
    pub owner: String,
    /// Project identifier.
    pub project: String,
    /// Source revision or commit-ish when known.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub revision: String,
    /// Unix timestamp when the snapshot was captured.
    #[serde(default)]
    pub captured_at: i64,
    /// Captured snapshot parts.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parts: Vec<RuntimeSnapshotPart>,
    /// Freeform future-safe metadata.
    #[serde(default, skip_serializing_if = "Value::is_null")]
    pub metadata: Value,
}

const fn default_project_runtime_snapshot_schema_version() -> u32 {
    PROJECT_RUNTIME_SNAPSHOT_SCHEMA_VERSION
}

impl Default for ProjectRuntimeSnapshot {
    fn default() -> Self {
        Self {
            schema_version: PROJECT_RUNTIME_SNAPSHOT_SCHEMA_VERSION,
            snapshot_id: String::new(),
            owner: String::new(),
            project: String::new(),
            revision: String::new(),
            captured_at: 0,
            parts: Vec::new(),
            metadata: Value::Object(Map::new()),
        }
    }
}
