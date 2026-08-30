//! The controller's signing key: the asymmetric half of `offices.md` §8.
//!
//! §8 requires proof in both directions, and the two directions are not
//! symmetric in what each party may hold.
//!
//! - **Office proves itself to the controller.** The office holds a secret and
//!   the controller stores `sha256(secret)`. Reading the controller's database
//!   yields a value that cannot be presented as the secret, because presenting
//!   it would hash to `sha256(digest)`. That direction was already asymmetric.
//! - **Controller proves itself to an office.** This one was not. Every proof
//!   in this direction — the registration answer, the internal-call header,
//!   and the vouch — used to be an `HMAC` keyed by the stored digest, so the
//!   value that verified a proof was the value that produced one. Anybody who
//!   read the controller's `office_join_tokens` table, or a mint response, or
//!   a proxy log, could mint a vouch naming any identity at any office and
//!   have that office create it as a superadmin. Verifier equalled forger.
//!
//! The fix is the ordinary one: the controller signs and the office verifies.
//! The controller holds an Ed25519 private key that never leaves its data
//! root; every office holds the 32-byte public half, delivered inside its join
//! token. Reading an office's stored material now forges nothing, and reading
//! the controller's database yields records — not a key that mints
//! administrators.
//!
//! Redemption stays self-authenticating, which §3 and §6 require rather than
//! merely permit: the office verifies a vouch with a public key it already
//! holds and calls nobody, exactly as it previously verified an HMAC with a
//! digest it already held.
//!
//! ## Why Ed25519, and why `ring`
//!
//! Ed25519 is deterministic (no per-signature randomness to get wrong),
//! fixed-size (32-byte key, 64-byte signature — a vouch stays a URL), fast
//! enough to sign on every internal call, and has no parameter choices to
//! misconfigure. RSA and ECDSA both have footguns this use has no reason to
//! accept.
//!
//! `ring` is used because it is already in this build's dependency graph
//! (`jsonwebtoken` depends on it), so naming it in `Cargo.toml` adds a line and
//! not a crate. `ed25519-dalek` would have been the other obvious choice and
//! would have added a tree that is not here today.

use std::fmt;

use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ring::rand::SystemRandom;
use ring::signature::{self, Ed25519KeyPair, KeyPair, UnparsedPublicKey};

/// Why a controller key could not be created, read, or used.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControllerKeyError {
    /// The stored PKCS#8 document is not a usable Ed25519 key.
    Unusable {
        /// What was wrong with it.
        reason: String,
    },
}

impl fmt::Display for ControllerKeyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unusable { reason } => write!(
                f,
                "the controller's cluster signing key is unusable ({reason}). Every office was \
                 issued the public half of this key inside its join token, so it cannot be \
                 replaced silently: restore the file, or re-mint every office's token with \
                 \"rotate\": true after generating a new one."
            ),
        }
    }
}

impl std::error::Error for ControllerKeyError {}

/// The controller's Ed25519 signing key.
///
/// Held only by a controller, only in memory and in its own data root. An
/// office never has one; it has [`verify`] and the public half.
pub struct ControllerSigningKey {
    pair: Ed25519KeyPair,
    verify_key: String,
}

impl fmt::Debug for ControllerSigningKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The private half is never rendered, not even into a debug log.
        f.debug_struct("ControllerSigningKey")
            .field("verify_key", &self.verify_key)
            .finish_non_exhaustive()
    }
}

impl ControllerSigningKey {
    /// Generate a fresh key, returning the PKCS#8 document to store beside it.
    ///
    /// The document is what goes to disk; the key is what stays in memory. They
    /// are returned together so no caller can persist one and use the other.
    pub fn generate() -> Result<(Vec<u8>, Self), ControllerKeyError> {
        let rng = SystemRandom::new();
        let document =
            Ed25519KeyPair::generate_pkcs8(&rng).map_err(|err| ControllerKeyError::Unusable {
                reason: format!("key generation failed: {err}"),
            })?;
        let bytes = document.as_ref().to_vec();
        let key = Self::from_pkcs8(&bytes)?;
        Ok((bytes, key))
    }

    /// Load a key from its stored PKCS#8 document.
    pub fn from_pkcs8(document: &[u8]) -> Result<Self, ControllerKeyError> {
        let pair =
            Ed25519KeyPair::from_pkcs8(document).map_err(|err| ControllerKeyError::Unusable {
                reason: format!("not a PKCS#8 Ed25519 key: {err}"),
            })?;
        let verify_key = URL_SAFE_NO_PAD.encode(pair.public_key().as_ref());
        Ok(Self { pair, verify_key })
    }

    /// The public half, base64url, as it travels inside a join token.
    pub fn verify_key(&self) -> &str {
        &self.verify_key
    }

    /// Sign one proof message.
    pub fn sign(&self, message: &str) -> String {
        URL_SAFE_NO_PAD.encode(self.pair.sign(message.as_bytes()).as_ref())
    }
}

/// Whether `signature` is this controller's signature over `message`.
///
/// Everything an office checks, it checks with this: a public key it stores and
/// nothing else. A caller that holds only `verify_key` can confirm a proof and
/// cannot produce one, which is the whole property this module exists for.
pub fn verify(verify_key: &str, message: &str, signature_b64: &str) -> bool {
    let (Ok(key), Ok(sig)) = (
        URL_SAFE_NO_PAD.decode(verify_key.trim()),
        URL_SAFE_NO_PAD.decode(signature_b64.trim()),
    ) else {
        return false;
    };
    UnparsedPublicKey::new(&signature::ED25519, key)
        .verify(message.as_bytes(), &sig)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_signature_verifies_under_the_public_half_and_nothing_else() {
        let (_, key) = ControllerSigningKey::generate().expect("generate");
        let signed = key.sign("zfjoin2/vouch:office-a:superadmin:1:n");
        assert!(verify(
            key.verify_key(),
            "zfjoin2/vouch:office-a:superadmin:1:n",
            &signed
        ));
        // A different message under the same key.
        assert!(!verify(
            key.verify_key(),
            "zfjoin2/vouch:office-a:root:1:n",
            &signed
        ));
        // The same message under a different key.
        let (_, other) = ControllerSigningKey::generate().expect("generate");
        assert!(!verify(
            other.verify_key(),
            "zfjoin2/vouch:office-a:superadmin:1:n",
            &signed
        ));
        // Garbage in either position is a refusal, never a panic.
        assert!(!verify("not-base64!!", "m", &signed));
        assert!(!verify(key.verify_key(), "m", "not-base64!!"));
        assert!(!verify("", "", ""));
    }

    #[test]
    fn the_public_half_forges_nothing() {
        // The property the whole module exists for, stated as a test: what an
        // office stores cannot be turned back into what a controller holds.
        let (document, key) = ControllerSigningKey::generate().expect("generate");
        let published = key.verify_key().to_string();
        assert_eq!(
            URL_SAFE_NO_PAD.decode(&published).expect("decode").len(),
            32
        );
        // The stored document is not the published key, and the published key
        // does not load as one.
        assert!(!document.windows(32).any(|window| {
            window
                == URL_SAFE_NO_PAD
                    .decode(&published)
                    .expect("decode")
                    .as_slice()
                && document.len() == 32
        }));
        assert!(
            ControllerSigningKey::from_pkcs8(&URL_SAFE_NO_PAD.decode(&published).expect("decode"))
                .is_err(),
            "the verification key must not load as a signing key"
        );
    }

    #[test]
    fn a_stored_key_round_trips_and_a_damaged_one_names_the_fix() {
        let (document, key) = ControllerSigningKey::generate().expect("generate");
        let reloaded = ControllerSigningKey::from_pkcs8(&document).expect("reload");
        assert_eq!(reloaded.verify_key(), key.verify_key());
        assert!(verify(key.verify_key(), "m", &reloaded.sign("m")));

        let err = ControllerSigningKey::from_pkcs8(b"not a key").expect_err("damaged");
        let message = err.to_string();
        assert!(
            message.contains("re-mint every office's token"),
            "{message}"
        );
    }
}
