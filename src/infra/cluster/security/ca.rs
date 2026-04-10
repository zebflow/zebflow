//! Cluster CA path model.

use std::path::PathBuf;

/// Filesystem locations for cluster CA assets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClusterCaPaths {
    /// Public cluster CA certificate.
    pub cert_path: PathBuf,
    /// Private cluster CA key.
    pub key_path: PathBuf,
}
