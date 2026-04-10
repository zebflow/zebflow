//! Issued node-certificate metadata.

use std::path::PathBuf;

/// Node certificate paths issued by the cluster trust root.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IssuedNodeCertificate {
    /// Stable node identifier.
    pub node_id: String,
    /// Node certificate path.
    pub cert_path: PathBuf,
    /// Node private key path.
    pub key_path: PathBuf,
}
