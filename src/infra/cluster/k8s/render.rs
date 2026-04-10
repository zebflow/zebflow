//! Rendered Kubernetes manifest model.

/// One rendered Kubernetes manifest.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedK8sManifest {
    /// Manifest label or filename hint.
    pub name: String,
    /// YAML body.
    pub yaml: String,
}
