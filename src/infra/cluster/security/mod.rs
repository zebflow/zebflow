//! Cluster security model.
//!
//! The target order of operations is:
//!
//! 1. one-time join token bootstrap
//! 2. cluster CA trust establishment
//! 3. node certificate issue/rotation
//! 4. mTLS-secured control transport

pub mod ca;
pub mod cert;
pub mod join_token;

pub use ca::ClusterCaPaths;
pub use cert::IssuedNodeCertificate;
pub use join_token::JoinToken;
