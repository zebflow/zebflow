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
//!
//! Three scheme words share that key, one per direction of use, so a value
//! captured in one direction never replays in another:
//!
//! ```text
//! zfjoin1   the office proves itself to the controller (the token)
//! zfjoin1c  the controller proves itself on an internal call
//! zfjoin1v  the controller vouches for one identity at one office
//! ```

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

/// Scheme word for a controller's vouch, `offices.md` §2's third verb.
///
/// Distinct from both [`JOIN_TOKEN_SCHEME`] and [`CONTROLLER_CALL_SCHEME`] for
/// the same reason those two are distinct from each other: three uses of one
/// key must not be interchangeable.
pub const VOUCH_SCHEME: &str = "zfjoin1v";

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

/// A controller's vouch for one identity at one office.
///
/// `offices.md` §2 gives the controller three verbs, and this is the third:
/// "one identity the offices accept for administration". §4 makes it the
/// office's normal door. The office must be able to open that door with nothing
/// but what it already holds, because §3 records that "the controller's death
/// costs logins, never execution" and §6 exists precisely so the credential
/// needed to repair a relationship is never held by the unreachable party. So a
/// vouch is self-authenticating: everything the office checks, it checks with
/// its own stored secret, and it calls nobody.
///
/// ```text
/// zfjoin1v:<office_id>:<identity>:<expires_at>:<nonce>:<proof>
/// ```
///
/// `proof` is `HMAC-SHA256` under the office's secret digest over every other
/// field, so all four are authenticated: an office id that can be edited would
/// let one office's vouch open another, and an expiry that can be edited would
/// not be an expiry.
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
    /// `HMAC-SHA256` over the other four fields, keyed by the secret digest.
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
    /// The proof does not verify under this office's own secret.
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
    /// Mint a vouch under one office's stored secret digest.
    ///
    /// `expires_at` is supplied rather than derived, because this layer holds
    /// no clock: a pure function is testable at any instant, and the caller
    /// already knows what "now" means for the process it runs in.
    pub fn mint(
        secret_digest: &str,
        office_id: &str,
        identity: &str,
        expires_at: i64,
        nonce: &str,
    ) -> Self {
        let proof = proof_under(
            secret_digest,
            &vouch_message(office_id, identity, expires_at, nonce),
        );
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

    /// Check a parsed vouch against this office's own secret digest and clock.
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
    /// office id is inside the signed message and the key is that office's own
    /// digest, so a vouch for another office fails the proof as well.
    pub fn verify(
        &self,
        secret_digest: &str,
        office_id: &str,
        now: i64,
    ) -> Result<(), OfficeVouchError> {
        if self.office_id != office_id {
            return Err(OfficeVouchError::OfficeMismatch {
                expected: office_id.to_string(),
                found: self.office_id.clone(),
            });
        }
        let expected = proof_under(
            secret_digest,
            &vouch_message(
                &self.office_id,
                &self.identity,
                self.expires_at,
                &self.nonce,
            ),
        );
        if !digests_match(&expected, &self.proof) {
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

fn vouch_message(office_id: &str, identity: &str, expires_at: i64, nonce: &str) -> String {
    format!("{JOIN_TOKEN_SCHEME}/vouch:{office_id}:{identity}:{expires_at}:{nonce}")
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
    fn a_vouch_round_trips_and_verifies_only_under_the_offices_own_secret() {
        let token = JoinToken::mint("office-a");
        let now = 1_700_000_000;
        let vouch = OfficeVouch::mint(
            &token.secret_digest(),
            "office-a",
            "joseph",
            now + VOUCH_TTL_SECS,
            &vouch_nonce(),
        );
        let rendered = vouch.render();
        assert!(
            rendered.starts_with("zfjoin1v:office-a:joseph:"),
            "{rendered}"
        );
        assert_eq!(OfficeVouch::parse(&rendered).expect("parse"), vouch);
        assert!(
            vouch
                .verify(&token.secret_digest(), "office-a", now)
                .is_ok()
        );

        // A host that never held this office's secret cannot mint one it will
        // accept. This is the same property revocation leans on: rotate the
        // token and every vouch minted under the old digest stops verifying.
        let rotated = JoinToken::mint("office-a");
        assert_eq!(
            vouch
                .verify(&rotated.secret_digest(), "office-a", now)
                .expect_err("a stranger's key must not open this office"),
            OfficeVouchError::ProofInvalid
        );
    }

    #[test]
    fn a_vouch_for_one_office_is_refused_by_another() {
        let token_a = JoinToken::mint("office-a");
        let token_b = JoinToken::mint("office-b");
        let now = 1_700_000_000;
        let for_a = OfficeVouch::mint(
            &token_a.secret_digest(),
            "office-a",
            "joseph",
            now + VOUCH_TTL_SECS,
            "n1",
        );

        let err = for_a
            .verify(&token_b.secret_digest(), "office-b", now)
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
        // office id is inside the signed message and the key is per-office.
        let relabelled = OfficeVouch {
            office_id: "office-b".to_string(),
            ..for_a
        };
        assert_eq!(
            relabelled
                .verify(&token_b.secret_digest(), "office-b", now)
                .expect_err("a relabelled vouch is a forgery"),
            OfficeVouchError::ProofInvalid
        );
    }

    #[test]
    fn every_field_of_a_vouch_is_authenticated() {
        let token = JoinToken::mint("office-a");
        let digest = token.secret_digest();
        let now = 1_700_000_000;
        let honest = OfficeVouch::mint(&digest, "office-a", "joseph", now + 120, "n1");

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
                proof: "0".repeat(honest.proof.len()),
                ..honest.clone()
            },
        ] {
            assert_eq!(
                tampered
                    .verify(&digest, "office-a", now)
                    .expect_err("a tampered vouch must not verify"),
                OfficeVouchError::ProofInvalid
            );
        }
    }

    #[test]
    fn an_expired_vouch_is_refused_and_the_proof_is_checked_first() {
        let token = JoinToken::mint("office-a");
        let digest = token.secret_digest();
        let now = 1_700_000_000;
        let expires_at = now + VOUCH_TTL_SECS;
        let vouch = OfficeVouch::mint(&digest, "office-a", "joseph", expires_at, "n1");

        assert!(vouch.verify(&digest, "office-a", expires_at - 1).is_ok());
        // The boundary is closed: a vouch is dead at its own expiry second.
        assert_eq!(
            vouch
                .verify(&digest, "office-a", expires_at)
                .expect_err("expired"),
            OfficeVouchError::Expired {
                expires_at,
                now: expires_at
            }
        );

        // A caller that cannot produce a proof learns nothing about whether
        // the vouch was also stale.
        let forged = OfficeVouch::mint(
            &JoinToken::mint("office-a").secret_digest(),
            "office-a",
            "joseph",
            now - 10_000,
            "n1",
        );
        assert_eq!(
            forged
                .verify(&digest, "office-a", now)
                .expect_err("forged and expired"),
            OfficeVouchError::ProofInvalid,
            "authenticity is decided before freshness"
        );
    }

    #[test]
    fn a_vouch_is_not_a_token_and_a_token_is_not_a_vouch() {
        let token = JoinToken::mint("office-a");
        let vouch =
            OfficeVouch::mint(&token.secret_digest(), "office-a", "joseph", 1, "n1").render();
        // Three scheme words, three parsers, no overlap: nothing captured in
        // one direction replays in another.
        assert!(JoinToken::parse(&vouch).is_err());
        assert!(parse_controller_call_header(&vouch).is_none());
        assert!(OfficeVouch::parse(&token.render()).is_err());
        assert!(
            OfficeVouch::parse(&controller_call_header(&token.secret_digest(), "office-a"))
                .is_err()
        );
    }

    #[test]
    fn a_malformed_vouch_is_refused_by_name() {
        assert_eq!(
            OfficeVouch::parse("zfjoin1v:office-a:joseph").expect_err("short"),
            OfficeVouchError::Malformed
        );
        assert_eq!(
            OfficeVouch::parse("zfjoin2v:office-a:joseph:1:n:p").expect_err("scheme"),
            OfficeVouchError::UnknownScheme {
                found: "zfjoin2v".to_string()
            }
        );
        assert_eq!(
            OfficeVouch::parse("zfjoin1v:office-a::1:n:p").expect_err("identity"),
            OfficeVouchError::MissingField { field: "identity" }
        );
        assert_eq!(
            OfficeVouch::parse("zfjoin1v:office-a:joseph:soon:n:p").expect_err("expiry"),
            OfficeVouchError::MalformedExpiry
        );
    }

    #[test]
    fn nonces_do_not_repeat() {
        assert_ne!(registration_nonce(), registration_nonce());
    }
}
