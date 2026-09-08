//! Per-office join token: shape, minting, hashing, and mutual proof.
//!
//! `offices.md` §8 requires a token that is "per-office, issued by the
//! controller, revocable for one office alone", carrying proof in both
//! directions. §8 also says the token "carries the controller's CA digest so
//! the office verifies the controller, and a secret so the controller verifies
//! the office" — two different materials for two different directions, which
//! is exactly what this module now carries.
//!
//! The shape is self-describing, the same principle the Credential contract
//! uses:
//!
//! ```text
//! zfjoin2:<office_id>:<controller_verify_key>:<secret>
//! ```
//!
//! - `zfjoin2` pins the format. §8 promises a later scheme "may change scheme
//!   while old tokens stay parseable rather than merely invalid", and this is
//!   that promise being kept: a `zfjoin1` value still parses far enough to be
//!   refused *by name*, with the re-mint named, rather than as gibberish.
//! - `office_id` names the holder, so the controller identifies who is
//!   presenting without searching every record by secret.
//! - `controller_verify_key` is the controller's Ed25519 public key, base64url.
//!   It is the office's whole basis for believing anything the controller
//!   says, and it is delivered here because a key delivered later would have to
//!   be trusted on first use.
//! - `secret` is 32 random bytes, hex encoded. The controller stores only
//!   `sha256(secret)`.
//!
//! ## The two directions are not symmetric, and must not be
//!
//! **Office proves itself to the controller.** The office presents the token;
//! the controller compares `sha256(secret)` against what it stored. Reading the
//! controller's database yields a digest, and a digest presented as a secret
//! hashes to `sha256(digest)` and fails. That direction was always asymmetric.
//!
//! **Controller proves itself to an office.** This one used to be an
//! `HMAC-SHA256` keyed by the very digest the controller stored, so the value
//! that *verified* a proof was the value that *produced* one. Anyone who read
//! `office_join_tokens`, a mint response, or a proxy log could forge a vouch
//! naming any identity at any office. Verifier equalled forger. It is now an
//! Ed25519 signature under a private key that never leaves the controller
//! ([`super::controller_key`]), and the office holds only the public half.
//!
//! Redemption stays self-authenticating — §3's "the controller's death costs
//! logins, never execution" and §6's rule about the unreachable party both
//! require it — because verifying a signature needs no network, exactly as
//! verifying an HMAC needed none.
//!
//! ## Every proof is bound to one office's *current* token
//!
//! One controller key signs for every office, so the message says which office
//! and which membership it is about. The binding value is the token
//! **fingerprint**, `sha256(sha256(secret))` — one hash past the digest the
//! controller stores, derivable by the controller from its record and by the
//! office from its secret, and useless as a key to anything.
//!
//! That keeps the property revocation leaned on before: rotating one office's
//! token changes its fingerprint, so every proof minted under the old one stops
//! verifying at that office, cryptographically and with no list to update. It
//! is also why one controller key does not flatten "revocable for one office
//! alone" into "revocable for all".
//!
//! Three scheme words separate the three uses, so a value captured in one
//! direction never replays in another:
//!
//! ```text
//! zfjoin2   the office proves itself to the controller (the token)
//! zfjoin2c  the controller proves itself on an internal call
//! zfjoin2v  the controller vouches for one identity at one office
//! ```

use rand::RngExt as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use super::controller_key::{ControllerSigningKey, verify as verify_signature};

/// Scheme word that opens every token this release mints or accepts.
pub const JOIN_TOKEN_SCHEME: &str = "zfjoin2";

/// The scheme this release replaced, kept so its refusal can name the fix.
///
/// `zfjoin1` keyed every controller-to-office proof by the digest the
/// controller stored, which made reading the controller's database enough to
/// mint a superadmin at any office. There is no migration: a `zfjoin1` token is
/// refused and re-minted.
pub const LEGACY_JOIN_TOKEN_SCHEME: &str = "zfjoin1";

/// Scheme word for the controller's half of an internal cluster call.
///
/// Deliberately distinct from [`JOIN_TOKEN_SCHEME`] so a value captured in one
/// direction cannot be replayed in the other.
pub const CONTROLLER_CALL_SCHEME: &str = "zfjoin2c";

/// Scheme word for a controller's vouch, `offices.md` §2's third verb.
///
/// Distinct from both [`JOIN_TOKEN_SCHEME`] and [`CONTROLLER_CALL_SCHEME`] for
/// the same reason those two are distinct from each other: three uses of one
/// key must not be interchangeable.
pub const VOUCH_SCHEME: &str = "zfjoin2v";

/// How long a minted vouch stays redeemable, in seconds.
///
/// A vouch is a hand-off, not a session: the controller mints one and the
/// operator's browser is redirected straight to the office that redeems it, so
/// the honest path costs well under a second. The number is therefore set by
/// what it must tolerate rather than by what it needs, and it tolerates exactly
/// one thing — two hosts disagreeing about the time. Two NTP-disciplined
/// machines are milliseconds apart; an undisciplined one drifts seconds to tens
/// of seconds. Two minutes absorbs that, and it is the *whole* allowance: no
/// separate skew-leniency window is added, because two numbers that both mean
/// "how wrong may the clock be" is how one of them ends up forgotten.
///
/// It is far tighter than the comparable credentials, deliberately. RFC 6749
/// §4.1.2 says an OAuth authorization code SHOULD live at most ten minutes and
/// SAML assertion windows default to about five, but both of those cover a
/// human typing at an identity provider. This covers a redirect. And unlike an
/// authorization code there is no back-channel exchange and no PKCE: the vouch
/// travels in a URL, where it can land in an access log or a `Referer` header.
/// The short life and the single-use rule together are what bound that; neither
/// would be enough alone.
pub const VOUCH_TTL_SECS: i64 = 120;

/// Random bytes behind one minted secret.
const SECRET_BYTES: usize = 32;

/// Random bytes behind one registration nonce.
const NONCE_BYTES: usize = 16;

/// A parsed or freshly minted join token.
///
/// The plaintext secret exists here and on the office that holds it. The
/// controller keeps [`JoinToken::secret_digest`] instead.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JoinToken {
    /// Office this token was issued to.
    pub office_id: String,
    /// The controller's Ed25519 verification key, base64url.
    ///
    /// The office's entire basis for believing a registration answer, an
    /// internal call, or a vouch. It verifies and forges nothing.
    pub controller_verify_key: String,
    /// High-entropy secret half.
    pub secret: String,
}

/// Why a presented token is not a token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JoinTokenError {
    /// The value has no `scheme:office:verify-key:secret` shape.
    Malformed,
    /// The value opens with a scheme this release does not mint.
    UnknownScheme {
        /// The scheme word actually found.
        found: String,
    },
    /// The office segment is empty.
    MissingOfficeId,
    /// The controller verification key segment is empty.
    MissingVerifyKey,
    /// The secret segment is empty.
    MissingSecret,
}

impl std::fmt::Display for JoinTokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let detail = match self {
            Self::Malformed => format!(
                "value is not shaped \
                 `{JOIN_TOKEN_SCHEME}:<office_id>:<controller_verify_key>:<secret>`"
            ),
            // The one scheme worth naming specifically. A `zfjoin1` token was
            // issued by a release whose controller-to-office proofs were
            // forgeable by anyone who read the controller's database, so it is
            // refused rather than upgraded in place: the office must be handed
            // a token that carries the controller's public key.
            Self::UnknownScheme { found } if found == LEGACY_JOIN_TOKEN_SCHEME => format!(
                "'{LEGACY_JOIN_TOKEN_SCHEME}' tokens are no longer accepted. They carried no \
                 controller verification key, so an office holding one could not tell a real \
                 controller from anything that had read the controller's records. Re-mint this \
                 office's token and delete the stored one"
            ),
            Self::UnknownScheme { found } => format!(
                "scheme '{found}' is not '{JOIN_TOKEN_SCHEME}'; this release mints \
                 `{JOIN_TOKEN_SCHEME}:<office_id>:<controller_verify_key>:<secret>` only"
            ),
            Self::MissingOfficeId => "the office id segment is empty".to_string(),
            Self::MissingVerifyKey => {
                "the controller verification key segment is empty".to_string()
            }
            Self::MissingSecret => "the secret segment is empty".to_string(),
        };
        // Every refusal names the one action that fixes it. There is no
        // migration from the old shared secret: pre-release, a value that is
        // not `zfjoin2:` shaped was never issued to anybody still running.
        write!(
            f,
            "invalid cluster join token: {detail}. Mint one on the controller \
             (POST /api/platform/cluster/join-tokens with an office id) and give it to this office as \
             ZEBFLOW_CLUSTER_JOIN_TOKEN"
        )
    }
}

impl std::error::Error for JoinTokenError {}

impl JoinToken {
    /// Mint a fresh token for `office_id` under one controller's key.
    pub fn mint(office_id: impl Into<String>, controller_verify_key: impl Into<String>) -> Self {
        let mut bytes = [0u8; SECRET_BYTES];
        rand::rng().fill(&mut bytes);
        Self {
            office_id: office_id.into(),
            controller_verify_key: controller_verify_key.into(),
            secret: hex::encode(bytes),
        }
    }

    /// Parse a presented token.
    ///
    /// `splitn(4, ':')` on purpose: the secret is hex today, but a later
    /// scheme may carry a value containing separators, and a parser that
    /// refuses those would make `zfjoin3` a breaking change for `zfjoin2`
    /// readers. The secret is last for the same reason — the tail segment is
    /// the one that can absorb anything.
    pub fn parse(raw: &str) -> Result<Self, JoinTokenError> {
        let raw = raw.trim();
        let mut parts = raw.splitn(4, ':');
        let scheme = parts.next().unwrap_or_default();
        let (Some(office_id), Some(verify_key), Some(secret)) =
            (parts.next(), parts.next(), parts.next())
        else {
            // A legacy token has three segments, so it lands here rather than
            // in the scheme branch. Named anyway, because "malformed" would
            // send an operator looking for a typo that is not there.
            if scheme == LEGACY_JOIN_TOKEN_SCHEME {
                return Err(JoinTokenError::UnknownScheme {
                    found: scheme.to_string(),
                });
            }
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
        if verify_key.trim().is_empty() {
            return Err(JoinTokenError::MissingVerifyKey);
        }
        if secret.trim().is_empty() {
            return Err(JoinTokenError::MissingSecret);
        }
        Ok(Self {
            office_id: office_id.trim().to_string(),
            controller_verify_key: verify_key.trim().to_string(),
            secret: secret.trim().to_string(),
        })
    }

    /// Render the token as the operator copies it.
    pub fn render(&self) -> String {
        format!(
            "{JOIN_TOKEN_SCHEME}:{}:{}:{}",
            self.office_id, self.controller_verify_key, self.secret
        )
    }

    /// The value the controller stores instead of the secret.
    pub fn secret_digest(&self) -> String {
        secret_digest(&self.secret)
    }

    /// The value every controller proof to this office is bound to.
    pub fn fingerprint(&self) -> String {
        token_fingerprint(&self.secret_digest())
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

/// `sha256(secret_digest)` — which membership a proof is about.
///
/// One hash past the digest the controller stores, so it names a token without
/// carrying anything that could be used to authenticate as one. Both sides
/// derive it: the controller from its record, the office from its secret.
pub fn token_fingerprint(secret_digest: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(secret_digest.as_bytes());
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

fn registration_message(office_id: &str, fingerprint: &str, nonce: &str) -> String {
    format!("{JOIN_TOKEN_SCHEME}/registration:{office_id}:{fingerprint}:{nonce}")
}

fn controller_call_message(office_id: &str, fingerprint: &str) -> String {
    format!("{JOIN_TOKEN_SCHEME}/controller-call:{office_id}:{fingerprint}")
}

/// The controller's proof that it is this office's controller.
///
/// Signed, not keyed by anything the controller stores about the office: a
/// reader of `office_join_tokens` learns the digest and can still not answer a
/// nonce, which is the point of the whole scheme.
pub fn registration_proof(
    key: &ControllerSigningKey,
    office_id: &str,
    fingerprint: &str,
    nonce: &str,
) -> String {
    key.sign(&registration_message(office_id, fingerprint, nonce))
}

/// Whether `proof` answers this office's nonce under its controller's key.
pub fn verify_registration_proof(
    controller_verify_key: &str,
    office_id: &str,
    fingerprint: &str,
    nonce: &str,
    proof: &str,
) -> bool {
    if nonce.trim().is_empty() || proof.trim().is_empty() {
        return false;
    }
    verify_signature(
        controller_verify_key,
        &registration_message(office_id, fingerprint, nonce),
        proof,
    )
}

/// The header value a controller presents when it calls one of its offices.
///
/// It is deterministic, so a captured header replays until that office's token
/// is rotated — recorded as an open item rather than solved here — but it is
/// scoped to one office and to this direction, so it can never be replayed
/// *back* at the controller as a registration, and it stops working the moment
/// that one office's token is rotated.
pub fn controller_call_header(
    key: &ControllerSigningKey,
    office_id: &str,
    fingerprint: &str,
) -> String {
    let proof = key.sign(&controller_call_message(office_id, fingerprint));
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

/// Whether a presented header is really this office's controller calling.
pub fn verify_controller_call_header(
    controller_verify_key: &str,
    office_id: &str,
    fingerprint: &str,
    raw: &str,
) -> bool {
    let Some((named, proof)) = parse_controller_call_header(raw) else {
        return false;
    };
    if named != office_id {
        return false;
    }
    verify_signature(
        controller_verify_key,
        &controller_call_message(office_id, fingerprint),
        &proof,
    )
}

/// A controller's vouch for one identity at one office.
///
/// `offices.md` §2 gives the controller three verbs, and this is the third:
/// "one identity the offices accept for administration". §4 makes it the
/// office's normal door. The office must be able to open that door with nothing
/// but what it already holds, because §3 records that "the controller's death
/// costs logins, never execution" and §6 exists precisely so the credential
/// needed to repair a relationship is never held by the unreachable party. So a
/// vouch is self-authenticating: everything the office checks, it checks with
/// its own stored verification key, and it calls nobody.
///
/// ```text
/// zfjoin2v:<office_id>:<identity>:<expires_at>:<nonce>:<proof>
/// ```
///
/// `proof` is an Ed25519 signature by the controller over every other field
/// *and* over the office's current token fingerprint, so all four are
/// authenticated: an office id that can be edited would let one office's vouch
/// open another, and an expiry that can be edited would not be an expiry. The
/// fingerprint is not transmitted because both sides already derive it; an
/// office running a rotated token computes a different one and the signature
/// fails, which is how rotation kills vouching without a revocation list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfficeVouch {
    /// Office this vouch may be redeemed at, and no other.
    pub office_id: String,
    /// Identity the controller vouches for.
    pub identity: String,
    /// Unix timestamp seconds after which the office must refuse it.
    pub expires_at: i64,
    /// Random value that makes this vouch one vouch, so it can be spent once.
    pub nonce: String,
    /// The controller's signature over the other four fields and the fingerprint.
    pub proof: String,
}

/// Why a presented vouch is refused.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OfficeVouchError {
    /// The value has no `scheme:office:identity:expiry:nonce:proof` shape.
    Malformed,
    /// The value opens with a scheme this release does not mint.
    UnknownScheme {
        /// The scheme word actually found.
        found: String,
    },
    /// A segment that must carry a value is empty.
    MissingField {
        /// Which segment.
        field: &'static str,
    },
    /// The expiry segment is not a Unix timestamp.
    MalformedExpiry,
    /// The vouch names an office other than the one it was presented to.
    OfficeMismatch {
        /// Office reading the vouch.
        expected: String,
        /// Office the vouch names.
        found: String,
    },
    /// The proof does not verify under this office's controller key.
    ProofInvalid,
    /// The vouch is past its expiry.
    Expired {
        /// The expiry it carried.
        expires_at: i64,
        /// The office's clock when it read it.
        now: i64,
    },
}

impl std::fmt::Display for OfficeVouchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed => write!(
                f,
                "value is not shaped \
                 `{VOUCH_SCHEME}:<office_id>:<identity>:<expires_at>:<nonce>:<proof>`. \
                 Ask the controller for a fresh one"
            ),
            Self::UnknownScheme { found } => write!(
                f,
                "scheme '{found}' is not '{VOUCH_SCHEME}'; this release accepts \
                 `{VOUCH_SCHEME}:` vouches only"
            ),
            Self::MissingField { field } => {
                write!(f, "the {field} segment of the vouch is empty")
            }
            Self::MalformedExpiry => write!(
                f,
                "the expiry segment of the vouch is not a Unix timestamp in seconds"
            ),
            Self::OfficeMismatch { expected, found } => write!(
                f,
                "this vouch was minted for office '{found}' and this office is '{expected}'. \
                 A vouch opens exactly one office. Ask the controller to mint one for \
                 '{expected}'"
            ),
            Self::ProofInvalid => write!(
                f,
                "the vouch does not prove it came from this office's controller. \
                 If this office's join token was rotated, every vouch minted under the old \
                 one is now unusable and the controller must mint a fresh vouch"
            ),
            Self::Expired { expires_at, now } => write!(
                f,
                "this vouch expired at {expires_at} and it is now {now}. A vouch lives \
                 {VOUCH_TTL_SECS}s because it is a hand-off, not a session. Ask the \
                 controller for a fresh one"
            ),
        }
    }
}

impl std::error::Error for OfficeVouchError {}

impl OfficeVouch {
    /// Mint a vouch under the controller's key, bound to one office's token.
    ///
    /// `expires_at` is supplied rather than derived, because this layer holds
    /// no clock: a pure function is testable at any instant, and the caller
    /// already knows what "now" means for the process it runs in.
    pub fn mint(
        key: &ControllerSigningKey,
        office_id: &str,
        identity: &str,
        expires_at: i64,
        nonce: &str,
        fingerprint: &str,
    ) -> Self {
        let proof = key.sign(&vouch_message(
            office_id,
            identity,
            expires_at,
            nonce,
            fingerprint,
        ));
        Self {
            office_id: office_id.to_string(),
            identity: identity.to_string(),
            expires_at,
            nonce: nonce.to_string(),
            proof,
        }
    }

    /// Parse a presented vouch without verifying it.
    ///
    /// The proof lands in the final segment, so `splitn` and not `split`: a
    /// later scheme may carry a value containing separators, and the same
    /// forward-compatibility argument [`JoinToken::parse`] records applies
    /// here unchanged.
    pub fn parse(raw: &str) -> Result<Self, OfficeVouchError> {
        let raw = raw.trim();
        let mut parts = raw.splitn(6, ':');
        let scheme = parts.next().unwrap_or_default();
        let (Some(office_id), Some(identity), Some(expires_at), Some(nonce), Some(proof)) = (
            parts.next(),
            parts.next(),
            parts.next(),
            parts.next(),
            parts.next(),
        ) else {
            return Err(OfficeVouchError::Malformed);
        };
        if scheme != VOUCH_SCHEME {
            return Err(OfficeVouchError::UnknownScheme {
                found: scheme.to_string(),
            });
        }
        for (field, value) in [
            ("office id", office_id),
            ("identity", identity),
            ("nonce", nonce),
            ("proof", proof),
        ] {
            if value.trim().is_empty() {
                return Err(OfficeVouchError::MissingField { field });
            }
        }
        let expires_at = expires_at
            .trim()
            .parse::<i64>()
            .map_err(|_| OfficeVouchError::MalformedExpiry)?;
        Ok(Self {
            office_id: office_id.trim().to_string(),
            identity: identity.trim().to_string(),
            expires_at,
            nonce: nonce.trim().to_string(),
            proof: proof.trim().to_string(),
        })
    }

    /// Render the vouch as it travels.
    pub fn render(&self) -> String {
        format!(
            "{VOUCH_SCHEME}:{}:{}:{}:{}:{}",
            self.office_id, self.identity, self.expires_at, self.nonce, self.proof
        )
    }

    /// Check a parsed vouch against this office's controller key, its own
    /// token fingerprint, and its own clock.
    ///
    /// The order is deliberate. The office check comes first because it is the
    /// one refusal with an action attached — the operator is at the wrong
    /// office and needs to be told which. The proof comes next, before the
    /// expiry, for the reason
    /// [`ClusterJoinTokenService::verify_office_token`] gives about revocation:
    /// a caller who cannot produce a proof must not learn from the refusal
    /// whether it also held a live vouch. Only something that already
    /// authenticated is told its vouch is stale.
    ///
    /// The office check is redundant against a forger and is kept anyway: the
    /// office id is inside the signed message, so a vouch for another office
    /// fails the proof as well.
    ///
    /// [`ClusterJoinTokenService::verify_office_token`]: crate::platform::services::cluster::ClusterJoinTokenService::verify_office_token
    pub fn verify(
        &self,
        controller_verify_key: &str,
        office_id: &str,
        fingerprint: &str,
        now: i64,
    ) -> Result<(), OfficeVouchError> {
        if self.office_id != office_id {
            return Err(OfficeVouchError::OfficeMismatch {
                expected: office_id.to_string(),
                found: self.office_id.clone(),
            });
        }
        if !verify_signature(
            controller_verify_key,
            &vouch_message(
                &self.office_id,
                &self.identity,
                self.expires_at,
                &self.nonce,
                fingerprint,
            ),
            &self.proof,
        ) {
            return Err(OfficeVouchError::ProofInvalid);
        }
        if self.expires_at <= now {
            return Err(OfficeVouchError::Expired {
                expires_at: self.expires_at,
                now,
            });
        }
        Ok(())
    }
}

/// A fresh nonce for one vouch. Reusing [`registration_nonce`]'s entropy.
pub fn vouch_nonce() -> String {
    registration_nonce()
}

fn vouch_message(
    office_id: &str,
    identity: &str,
    expires_at: i64,
    nonce: &str,
    fingerprint: &str,
) -> String {
    format!("{JOIN_TOKEN_SCHEME}/vouch:{office_id}:{identity}:{expires_at}:{nonce}:{fingerprint}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn controller() -> ControllerSigningKey {
        ControllerSigningKey::generate().expect("generate").1
    }

    fn token_for(office_id: &str, key: &ControllerSigningKey) -> JoinToken {
        JoinToken::mint(office_id, key.verify_key())
    }

    #[test]
    fn a_minted_token_round_trips_through_its_rendered_form() {
        let key = controller();
        let token = token_for("office-a", &key);
        let rendered = token.render();
        assert!(rendered.starts_with("zfjoin2:office-a:"));
        assert!(rendered.contains(key.verify_key()));
        assert_eq!(JoinToken::parse(&rendered).expect("parse"), token);
        // 32 random bytes, hex encoded.
        assert_eq!(token.secret.len(), SECRET_BYTES * 2);
    }

    #[test]
    fn two_mints_never_share_a_secret() {
        let key = controller();
        let first = token_for("office-a", &key);
        let second = token_for("office-a", &key);
        assert_ne!(first.secret, second.secret);
        assert_ne!(first.secret_digest(), second.secret_digest());
        // Same controller, so the same verification key. Different membership,
        // so a different fingerprint — which is what binds a proof to one of
        // them and not the other.
        assert_eq!(first.controller_verify_key, second.controller_verify_key);
        assert_ne!(first.fingerprint(), second.fingerprint());
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
        assert!(message.contains("/api/platform/cluster/join-tokens"), "{message}");
    }

    #[test]
    fn a_zfjoin1_token_is_refused_by_name_because_it_carries_no_controller_key() {
        // §8 promises a scheme change leaves old tokens parseable rather than
        // merely invalid. This is that: three segments, so it does not even
        // have the shape, and it is still named.
        let err = JoinToken::parse("zfjoin1:office-a:deadbeef").expect_err("legacy scheme");
        assert_eq!(
            err,
            JoinTokenError::UnknownScheme {
                found: "zfjoin1".to_string()
            }
        );
        let message = err.to_string();
        assert!(
            message.contains("no controller verification key"),
            "{message}"
        );
        assert!(message.contains("Re-mint this office's token"), "{message}");
    }

    #[test]
    fn a_future_scheme_is_refused_by_name_rather_than_as_gibberish() {
        let err = JoinToken::parse("zfjoin3:office-a:key:abc").expect_err("unknown scheme");
        assert_eq!(
            err,
            JoinTokenError::UnknownScheme {
                found: "zfjoin3".to_string()
            }
        );
        assert!(err.to_string().contains("zfjoin3"));
    }

    #[test]
    fn empty_segments_are_refused() {
        assert_eq!(
            JoinToken::parse("zfjoin2::key:secret").expect_err("no office"),
            JoinTokenError::MissingOfficeId
        );
        assert_eq!(
            JoinToken::parse("zfjoin2:office-a::secret").expect_err("no verify key"),
            JoinTokenError::MissingVerifyKey
        );
        assert_eq!(
            JoinToken::parse("zfjoin2:office-a:key:").expect_err("no secret"),
            JoinTokenError::MissingSecret
        );
        assert_eq!(
            JoinToken::parse("zfjoin2:office-a:key").expect_err("three segments"),
            JoinTokenError::Malformed
        );
    }

    #[test]
    fn the_digest_is_stable_and_the_comparison_is_by_value() {
        let key = controller();
        let token = token_for("office-a", &key);
        assert_eq!(token.secret_digest(), secret_digest(&token.secret));
        assert_eq!(
            token.fingerprint(),
            token_fingerprint(&token.secret_digest())
        );
        assert_ne!(
            token.fingerprint(),
            token.secret_digest(),
            "the fingerprint must be one hash past the stored digest"
        );
        assert!(digests_match(
            &token.secret_digest(),
            &token.secret_digest()
        ));
        assert!(!digests_match(&token.secret_digest(), "0"));
        assert!(!digests_match(
            &token.secret_digest(),
            &token_for("office-a", &key).secret_digest()
        ));
    }

    #[test]
    fn what_the_controller_stores_no_longer_forges_a_proof() {
        // The whole point of the release. A reader of `office_join_tokens`
        // holds the digest and therefore the fingerprint; neither produces a
        // registration answer, a call header, or a vouch.
        let key = controller();
        let token = token_for("office-a", &key);
        let stolen_digest = token.secret_digest();
        let stolen_fingerprint = token_fingerprint(&stolen_digest);
        let nonce = registration_nonce();

        let honest = registration_proof(&key, "office-a", &stolen_fingerprint, &nonce);
        assert!(verify_registration_proof(
            key.verify_key(),
            "office-a",
            &stolen_fingerprint,
            &nonce,
            &honest
        ));
        // Everything a database reader could try, in place of the key.
        for forged in [
            stolen_digest.as_str(),
            stolen_fingerprint.as_str(),
            token.secret.as_str(),
            "",
        ] {
            assert!(
                !verify_registration_proof(
                    key.verify_key(),
                    "office-a",
                    &stolen_fingerprint,
                    &nonce,
                    forged
                ),
                "stored material must not answer a nonce: {forged}"
            );
        }
        // And a whole second controller does not answer for this one.
        let impostor = controller();
        assert!(!verify_registration_proof(
            key.verify_key(),
            "office-a",
            &stolen_fingerprint,
            &nonce,
            &registration_proof(&impostor, "office-a", &stolen_fingerprint, &nonce)
        ));
    }

    #[test]
    fn a_registration_proof_is_bound_to_its_nonce_office_and_membership() {
        let key = controller();
        let token = token_for("office-a", &key);
        let fingerprint = token.fingerprint();
        let nonce = registration_nonce();
        let honest = registration_proof(&key, "office-a", &fingerprint, &nonce);

        assert!(!verify_registration_proof(
            key.verify_key(),
            "office-a",
            &fingerprint,
            &registration_nonce(),
            &honest
        ));
        assert!(!verify_registration_proof(
            key.verify_key(),
            "office-b",
            &fingerprint,
            &nonce,
            &honest
        ));
        // A rotated token is a different membership under the same controller.
        assert!(!verify_registration_proof(
            key.verify_key(),
            "office-a",
            &token_for("office-a", &key).fingerprint(),
            &nonce,
            &honest
        ));
    }

    #[test]
    fn a_controller_call_header_cannot_be_replayed_as_a_registration() {
        let key = controller();
        let token = token_for("office-a", &key);
        let header = controller_call_header(&key, "office-a", &token.fingerprint());
        let (office_id, proof) = parse_controller_call_header(&header).expect("parse");
        assert_eq!(office_id, "office-a");
        assert!(!proof.is_empty());
        assert!(verify_controller_call_header(
            key.verify_key(),
            "office-a",
            &token.fingerprint(),
            &header
        ));
        // Another office's header, another controller's header, and a rotated
        // membership all fail.
        assert!(!verify_controller_call_header(
            key.verify_key(),
            "office-b",
            &token.fingerprint(),
            &header
        ));
        assert!(!verify_controller_call_header(
            controller().verify_key(),
            "office-a",
            &token.fingerprint(),
            &header
        ));
        assert!(!verify_controller_call_header(
            key.verify_key(),
            "office-a",
            &token_for("office-a", &key).fingerprint(),
            &header
        ));
        // The two directions use different scheme words, so neither parser
        // accepts the other's value.
        assert!(JoinToken::parse(&header).is_err());
        assert!(parse_controller_call_header(&token.render()).is_none());
    }

    #[test]
    fn a_vouch_round_trips_and_verifies_only_under_the_controllers_key() {
        let key = controller();
        let token = token_for("office-a", &key);
        let now = 1_700_000_000;
        let vouch = OfficeVouch::mint(
            &key,
            "office-a",
            "joseph",
            now + VOUCH_TTL_SECS,
            &vouch_nonce(),
            &token.fingerprint(),
        );
        let rendered = vouch.render();
        assert!(
            rendered.starts_with("zfjoin2v:office-a:joseph:"),
            "{rendered}"
        );
        assert_eq!(OfficeVouch::parse(&rendered).expect("parse"), vouch);
        assert!(
            vouch
                .verify(key.verify_key(), "office-a", &token.fingerprint(), now)
                .is_ok()
        );

        // A host that never held the controller's private key cannot mint one
        // this office accepts — including a host that read every byte the
        // controller stores about this office.
        assert_eq!(
            vouch
                .verify(
                    controller().verify_key(),
                    "office-a",
                    &token.fingerprint(),
                    now
                )
                .expect_err("a stranger's key must not open this office"),
            OfficeVouchError::ProofInvalid
        );
        // And rotation kills it, because the fingerprint moves.
        assert_eq!(
            vouch
                .verify(
                    key.verify_key(),
                    "office-a",
                    &token_for("office-a", &key).fingerprint(),
                    now
                )
                .expect_err("a rotated token invalidates every vouch under the old one"),
            OfficeVouchError::ProofInvalid
        );
    }

    #[test]
    fn a_vouch_for_one_office_is_refused_by_another() {
        let key = controller();
        let token_a = token_for("office-a", &key);
        let token_b = token_for("office-b", &key);
        let now = 1_700_000_000;
        let for_a = OfficeVouch::mint(
            &key,
            "office-a",
            "joseph",
            now + VOUCH_TTL_SECS,
            "n1",
            &token_a.fingerprint(),
        );

        let err = for_a
            .verify(key.verify_key(), "office-b", &token_b.fingerprint(), now)
            .expect_err("office B must refuse office A's vouch");
        assert_eq!(
            err,
            OfficeVouchError::OfficeMismatch {
                expected: "office-b".to_string(),
                found: "office-a".to_string(),
            }
        );
        assert!(err.to_string().contains("opens exactly one office"));

        // Even with the name check removed the proof would fail, because the
        // office id and the office's own token fingerprint are both inside the
        // signed message. One controller key does not flatten §8's "revocable
        // for one office alone".
        let relabelled = OfficeVouch {
            office_id: "office-b".to_string(),
            ..for_a
        };
        assert_eq!(
            relabelled
                .verify(key.verify_key(), "office-b", &token_b.fingerprint(), now)
                .expect_err("a relabelled vouch is a forgery"),
            OfficeVouchError::ProofInvalid
        );
    }

    #[test]
    fn every_field_of_a_vouch_is_authenticated() {
        let key = controller();
        let token = token_for("office-a", &key);
        let fingerprint = token.fingerprint();
        let now = 1_700_000_000;
        let honest = OfficeVouch::mint(&key, "office-a", "joseph", now + 120, "n1", &fingerprint);

        // Each edit is a different vouch, so each must fail the proof. An
        // editable expiry would not be an expiry, and an editable identity
        // would let anyone who saw one vouch mint every other.
        for tampered in [
            OfficeVouch {
                identity: "root".to_string(),
                ..honest.clone()
            },
            OfficeVouch {
                expires_at: now + 86_400,
                ..honest.clone()
            },
            OfficeVouch {
                nonce: "n2".to_string(),
                ..honest.clone()
            },
            OfficeVouch {
                proof: "A".repeat(honest.proof.len()),
                ..honest.clone()
            },
        ] {
            assert_eq!(
                tampered
                    .verify(key.verify_key(), "office-a", &fingerprint, now)
                    .expect_err("a tampered vouch must not verify"),
                OfficeVouchError::ProofInvalid
            );
        }
    }

    #[test]
    fn an_expired_vouch_is_refused_and_the_proof_is_checked_first() {
        let key = controller();
        let token = token_for("office-a", &key);
        let fingerprint = token.fingerprint();
        let now = 1_700_000_000;
        let expires_at = now + VOUCH_TTL_SECS;
        let vouch = OfficeVouch::mint(&key, "office-a", "joseph", expires_at, "n1", &fingerprint);

        assert!(
            vouch
                .verify(key.verify_key(), "office-a", &fingerprint, expires_at - 1)
                .is_ok()
        );
        // The boundary is closed: a vouch is dead at its own expiry second.
        assert_eq!(
            vouch
                .verify(key.verify_key(), "office-a", &fingerprint, expires_at)
                .expect_err("expired"),
            OfficeVouchError::Expired {
                expires_at,
                now: expires_at
            }
        );

        // A caller that cannot produce a proof learns nothing about whether
        // the vouch was also stale.
        let forged = OfficeVouch::mint(
            &controller(),
            "office-a",
            "joseph",
            now - 10_000,
            "n1",
            &fingerprint,
        );
        assert_eq!(
            forged
                .verify(key.verify_key(), "office-a", &fingerprint, now)
                .expect_err("forged and expired"),
            OfficeVouchError::ProofInvalid,
            "authenticity is decided before freshness"
        );
    }

    #[test]
    fn a_vouch_is_not_a_token_and_a_token_is_not_a_vouch() {
        let key = controller();
        let token = token_for("office-a", &key);
        let vouch =
            OfficeVouch::mint(&key, "office-a", "joseph", 1, "n1", &token.fingerprint()).render();
        // Three scheme words, three parsers, no overlap: nothing captured in
        // one direction replays in another.
        assert!(JoinToken::parse(&vouch).is_err());
        assert!(parse_controller_call_header(&vouch).is_none());
        assert!(OfficeVouch::parse(&token.render()).is_err());
        assert!(
            OfficeVouch::parse(&controller_call_header(
                &key,
                "office-a",
                &token.fingerprint()
            ))
            .is_err()
        );
    }

    #[test]
    fn a_malformed_vouch_is_refused_by_name() {
        assert_eq!(
            OfficeVouch::parse("zfjoin2v:office-a:joseph").expect_err("short"),
            OfficeVouchError::Malformed
        );
        assert_eq!(
            OfficeVouch::parse("zfjoin1v:office-a:joseph:1:n:p").expect_err("scheme"),
            OfficeVouchError::UnknownScheme {
                found: "zfjoin1v".to_string()
            }
        );
        assert_eq!(
            OfficeVouch::parse("zfjoin2v:office-a::1:n:p").expect_err("identity"),
            OfficeVouchError::MissingField { field: "identity" }
        );
        assert_eq!(
            OfficeVouch::parse("zfjoin2v:office-a:joseph:soon:n:p").expect_err("expiry"),
            OfficeVouchError::MalformedExpiry
        );
    }

    #[test]
    fn nonces_do_not_repeat() {
        assert_ne!(registration_nonce(), registration_nonce());
    }
}
