//! Encryption of secrets Zebflow stores on behalf of a project.
//!
//! The Credential contract (`docs/contracts/kinds/credential/README.md`) states
//! what this has to be and why, and the two modules here split it the same way
//! `infra/cluster/security/` splits the controller's signing key: a primitive
//! that knows only bytes, and a resolver that knows the disk.
//!
//! | Module | Owns |
//! | --- | --- |
//! | [`envelope`] | the AEAD and the three self-describing formats — `zfk1`, `zfw1`, `zfc1` |
//! | [`keyring`] | the instance key file, the versioned data keys, and every refusal |
//!
//! What this protects is stated in the contract and worth repeating where the
//! code is: **a copied database** — a backup, a support bundle, a snapshot, a
//! misplaced volume. It does not protect against filesystem access, process
//! memory, or code running inside Zebflow, because the key must be readable by
//! the process that reads the data.

pub mod envelope;
pub mod keyring;

pub use envelope::{
    SECRET_TAG, SecretCryptoError, SecretKey, is_secret_envelope, parse_secret_envelope,
};
pub use keyring::{
    CREDENTIAL_KEY_REL, CREDENTIAL_KEY_VAR, CredentialKeyError, CredentialKeyring, KeySource,
    KeyringBootstrap, WrappedDataKey,
};
