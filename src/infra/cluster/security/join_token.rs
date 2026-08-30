//! Per-office join token: shape, minting, hashing, and mutual proof.
//!
//! `offices.md` §8 requires a token that is "per-office, issued by the
//! controller, revocable for one office alone", carrying proof in both
//! directions. A shared environment secret is explicitly not a token there,
//! because possession would be membership with no way to revoke one office.
//!
//! The shape is self-describing, the same principle the Credential contract
//! uses:
//!
//! ```text
//! zfjoin1:<office_id>:<secret>
//! ```
//!
//! - `zfjoin1` pins the format, so a later `zfjoin2` may change scheme while
//!   old tokens stay parseable rather than merely invalid.
//! - `office_id` names the holder, so the controller identifies who is
//!   presenting without searching every record by secret.
//! - `secret` is 32 random bytes, hex encoded.
//!
//! The controller stores only `sha256(secret)`. That digest is also the key of
//! the mutual proof: the office derives it from the secret it holds, the
//! controller reads it from its record, and nobody who lacks one or the other
//! can produce an HMAC under it. That is how the office verifies the
//! controller without a certificate authority, and it is why the plaintext
//! secret never has to be kept anywhere on the controller.

use hmac::{Hmac, Mac};
use rand::RngExt as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

/// Scheme word that opens every token this release mints or accepts.
pub const JOIN_TOKEN_SCHEME: &str = "zfjoin1";

/// Scheme word for the controller's half of an internal cluster call.
///
/// Deliberately distinct from [`JOIN_TOKEN_SCHEME`] so a value captured in one
/// direction cannot be replayed in the other.
pub const CONTROLLER_CALL_SCHEME: &str = "zfjoin1c";

/// Random bytes behind one minted secret.
const SECRET_BYTES: usize = 32;

/// Random bytes behind one registration nonce.
const NONCE_BYTES: usize = 16;

type HmacSha256 = Hmac<Sha256>;

/// A parsed or freshly minted join token.
///
/// The plaintext secret exists here and on the office that holds it. The
/// controller keeps [`JoinToken::secret_digest`] instead.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JoinToken {
    /// Office this token was issued to.
    pub office_id: String,
    /// High-entropy secret half.
    pub secret: String,
}

/// Why a presented token is not a token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JoinTokenError {
    /// The value has no `scheme:office:secret` shape.
    Malformed,
    /// The value opens with a scheme this release does not mint.
    UnknownScheme {
        /// The scheme word actually found.
        found: String,
    },
    /// The office segment is empty.
    MissingOfficeId,
    /// The secret segment is empty.
    MissingSecret,
}

impl std::fmt::Display for JoinTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let detail = match self {
            Self::Malformed => {
                format!("value is not shaped `{JOIN_TOKEN_SCHEME}:<office_id>:<secret>`")
            }
            Self::UnknownScheme { found } => format!(
                "scheme '{found}' is not '{JOIN_TOKEN_SCHEME}'; this release mints \
                 `{JOIN_TOKEN_SCHEME}:<office_id>:<secret>` only"
            ),
            Self::MissingOfficeId => "the office id segment is empty".to_string(),
            Self::MissingSecret => "the secret segment is empty".to_string(),
        };
        // Every refusal names the one action that fixes it. There is no
        // migration from the old shared secret: pre-release, a value that is
        // not `zfjoin1:` shaped was never issued to anybody.
        write!(
            f,
            "invalid cluster join token: {detail}. Mint one on the controller \
             (POST /api/cluster/join-tokens with an office id) and give it to this office as \
             ZEBFLOW_CLUSTER_JOIN_TOKEN"
        )
    }
}

impl std::error::Error for JoinTokenError {}

impl JoinToken {
    /// Mint a fresh token for `office_id`.
    pub fn mint(office_id: impl Into<String>) -> Self {
        let mut bytes = [0u8; SECRET_BYTES];
        rand::rng().fill(&mut bytes);
        Self {
            office_id: office_id.into(),
            secret: hex::encode(bytes),
        }
    }

    /// Parse a presented token.
    ///
    /// `splitn(3, ':')` on purpose: the secret is hex today, but a later
    /// scheme may carry a value containing separators, and a parser that
    /// refuses those would make `zfjoin2` a breaking change for `zfjoin1`
    /// readers.
    pub fn parse(raw: &str) -> Result<Self, JoinTokenError> {
        let raw = raw.trim();
        let mut parts = raw.splitn(3, ':');
        let scheme = parts.next().unwrap_or_default();
        let (Some(office_id), Some(secret)) = (parts.next(), parts.next()) else {
            return Err(JoinTokenError::Malformed);
        };
        if scheme != JOIN_TOKEN_SCHEME {
            return Err(JoinTokenError::UnknownScheme {
                found: scheme.to_string(),
            });
        }
        if office_id.trim().is_empty() {
            return Err(JoinTokenError::MissingOfficeId);
        }
        if secret.trim().is_empty() {
            return Err(JoinTokenError::MissingSecret);
        }
        Ok(Self {
            office_id: office_id.trim().to_string(),
            secret: secret.trim().to_string(),
        })
    }

    /// Render the token as the operator copies it.
    pub fn render(&self) -> String {
        format!("{JOIN_TOKEN_SCHEME}:{}:{}", self.office_id, self.secret)
    }

    /// The value the controller stores instead of the secret.
    pub fn secret_digest(&self) -> String {
        secret_digest(&self.secret)
    }
}

/// Hash one secret for storage.
///
/// SHA-256 and not a password KDF. The secret is 256 bits of machine-generated
/// randomness, so there is no dictionary to slow an attacker down through, and
/// the digest is recomputed on every heartbeat from every office — an
/// intentionally slow hash here would be a self-inflicted denial of service on
/// the control plane. Argon2 stays where it belongs, on human passwords.
pub fn secret_digest(secret: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret.as_bytes());
    hex::encode(hasher.finalize())
}

/// Compare two secrets or digests without leaking where they first differ.
pub fn digests_match(left: &str, right: &str) -> bool {
    left.as_bytes().ct_eq(right.as_bytes()).into()
}

/// A fresh nonce an office sends with its registration.
pub fn registration_nonce() -> String {
    let mut bytes = [0u8; NONCE_BYTES];
    rand::rng().fill(&mut bytes);
    hex::encode(bytes)
}

/// The controller's proof that it holds this office's secret.
///
/// Keyed by the digest rather than the secret, because the controller stores
/// only the digest and must still be able to answer. Deriving the digest
/// requires the secret, so an impostor that never held the token cannot
/// produce this value.
pub fn registration_proof(secret_digest: &str, office_id: &str, nonce: &str) -> String {
    proof_under(
        secret_digest,
        &format!("{JOIN_TOKEN_SCHEME}/registration:{office_id}:{nonce}"),
    )
}

/// The header value a controller presents when it calls one of its offices.
///
/// The office holds the secret, derives the digest, and recomputes this. It is
/// a bearer value of the same power as the digest — it is deterministic, so a
/// captured header replays until the token is rotated — but it is scoped to
/// one office and to this direction, so it can never be replayed *back* at the
/// controller as a registration.
pub fn controller_call_header(secret_digest: &str, office_id: &str) -> String {
    let proof = proof_under(
        secret_digest,
        &format!("{JOIN_TOKEN_SCHEME}/controller-call:{office_id}"),
    );
    format!("{CONTROLLER_CALL_SCHEME}:{office_id}:{proof}")
}

/// Parse a controller-call header into its office and proof halves.
pub fn parse_controller_call_header(raw: &str) -> Option<(String, String)> {
    let raw = raw.trim();
    let mut parts = raw.splitn(3, ':');
    let scheme = parts.next()?;
    let office_id = parts.next()?;
    let proof = parts.next()?;
    if scheme != CONTROLLER_CALL_SCHEME || office_id.is_empty() || proof.is_empty() {
        return None;
    }
    Some((office_id.to_string(), proof.to_string()))
}

fn proof_under(key: &str, message: &str) -> String {
    let mut mac =
        HmacSha256::new_from_slice(key.as_bytes()).expect("hmac accepts a key of any length");
    mac.update(message.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_minted_token_round_trips_through_its_rendered_form() {
        let token = JoinToken::mint("office-a");
        let rendered = token.render();
        assert!(rendered.starts_with("zfjoin1:office-a:"));
        assert_eq!(JoinToken::parse(&rendered).expect("parse"), token);
        // 32 random bytes, hex encoded.
        assert_eq!(token.secret.len(), SECRET_BYTES * 2);
    }

    #[test]
    fn two_mints_never_share_a_secret() {
        let first = JoinToken::mint("office-a");
        let second = JoinToken::mint("office-a");
        assert_ne!(first.secret, second.secret);
        assert_ne!(first.secret_digest(), second.secret_digest());
    }

    #[test]
    fn the_old_shared_secret_shape_is_refused_with_an_actionable_message() {
        // Pre-release: there is no migration path. A bare environment secret
        // was the defect, so it must not parse.
        let err = JoinToken::parse("join-token").expect_err("bare secret is not a token");
        assert_eq!(err, JoinTokenError::Malformed);
        let message = err.to_string();
        assert!(
            message.contains("Mint one on the controller"),
            "refusal must name the action: {message}"
        );
        assert!(message.contains("/api/cluster/join-tokens"), "{message}");
    }

    #[test]
    fn a_future_scheme_is_refused_by_name_rather_than_as_gibberish() {
        let err = JoinToken::parse("zfjoin2:office-a:abc").expect_err("unknown scheme");
        assert_eq!(
            err,
            JoinTokenError::UnknownScheme {
                found: "zfjoin2".to_string()
            }
        );
        assert!(err.to_string().contains("zfjoin2"));
    }

    #[test]
    fn empty_segments_are_refused() {
        assert_eq!(
            JoinToken::parse("zfjoin1::secret").expect_err("no office"),
            JoinTokenError::MissingOfficeId
        );
        assert_eq!(
            JoinToken::parse("zfjoin1:office-a:").expect_err("no secret"),
            JoinTokenError::MissingSecret
        );
        assert_eq!(
            JoinToken::parse("zfjoin1:office-a").expect_err("two segments"),
            JoinTokenError::Malformed
        );
    }

    #[test]
    fn the_digest_is_stable_and_the_comparison_is_by_value() {
        let token = JoinToken::mint("office-a");
        assert_eq!(token.secret_digest(), secret_digest(&token.secret));
        assert!(digests_match(
            &token.secret_digest(),
            &token.secret_digest()
        ));
        assert!(!digests_match(&token.secret_digest(), "0"));
        assert!(!digests_match(
            &token.secret_digest(),
            &JoinToken::mint("office-a").secret_digest()
        ));
    }

    #[test]
    fn the_registration_proof_needs_the_offices_own_secret() {
        let token = JoinToken::mint("office-a");
        let impostor = JoinToken::mint("office-a");
        let nonce = registration_nonce();

        let honest = registration_proof(&token.secret_digest(), "office-a", &nonce);
        assert_eq!(
            honest,
            registration_proof(&token.secret_digest(), "office-a", &nonce),
            "the same inputs must produce the same proof"
        );
        assert_ne!(
            honest,
            registration_proof(&impostor.secret_digest(), "office-a", &nonce),
            "a host that does not hold the secret cannot answer"
        );
        assert_ne!(
            honest,
            registration_proof(&token.secret_digest(), "office-a", &registration_nonce()),
            "a proof for one nonce must not answer another"
        );
        assert_ne!(
            honest,
            registration_proof(&token.secret_digest(), "office-b", &nonce),
            "a proof is bound to the office it names"
        );
    }

    #[test]
    fn a_controller_call_header_cannot_be_replayed_as_a_registration() {
        let token = JoinToken::mint("office-a");
        let header = controller_call_header(&token.secret_digest(), "office-a");
        let (office_id, proof) = parse_controller_call_header(&header).expect("parse");
        assert_eq!(office_id, "office-a");
        assert!(!proof.is_empty());
        // The two directions use different scheme words, so neither parser
        // accepts the other's value.
        assert!(JoinToken::parse(&header).is_err());
        assert!(parse_controller_call_header(&token.render()).is_none());
    }

    #[test]
    fn nonces_do_not_repeat() {
        assert_ne!(registration_nonce(), registration_nonce());
    }
}
