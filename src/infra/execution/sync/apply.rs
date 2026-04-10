//! Bundle application result model.

use serde::{Deserialize, Serialize};

/// Result of applying a runtime bundle on a worker/runtime target.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct ProjectBundleApplyReport {
    /// Bundle schema version that was applied.
    #[serde(default)]
    pub bundle_schema_version: u32,
    /// Bundle identifier that was applied.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub bundle_id: String,
    /// Number of files or bundle items applied.
    pub applied_items: u32,
    /// Pipeline glob patterns activated as part of bootstrap.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub activated_globs: Vec<String>,
    /// Non-fatal warnings raised during apply.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    /// Human-readable status message.
    #[serde(default)]
    pub message: String,
}
