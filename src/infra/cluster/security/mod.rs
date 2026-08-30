//! Cluster security model.
//!
//! The target order of operations is:
//!
//! 1. per-office join token bootstrap (`join_token`, live)
//! 2. cluster CA trust establishment
//! 3. node certificate issue/rotation
//! 4. mTLS-secured control transport

pub mod ca;
pub mod cert;
pub mod controller_key;
pub mod join_token;

pub use ca::ClusterCaPaths;
pub use cert::IssuedNodeCertificate;
pub use controller_key::{ControllerKeyError, ControllerSigningKey};
pub use join_token::{
    CONTROLLER_CALL_SCHEME, JOIN_TOKEN_SCHEME, JoinToken, JoinTokenError, LEGACY_JOIN_TOKEN_SCHEME,
    OfficeVouch, OfficeVouchError, VOUCH_SCHEME, VOUCH_TTL_SECS, controller_call_header,
    digests_match, parse_controller_call_header, registration_nonce, registration_proof,
    secret_digest, token_fingerprint, verify_controller_call_header, verify_registration_proof,
    vouch_nonce,
};
