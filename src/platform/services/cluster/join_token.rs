//! Per-office join tokens: minting, verification, revocation, and the office's
//! own stored identity.
//!
//! `offices.md` §8 states the whole agreement: the token is "per-office, issued
//! by the controller, revocable for one office alone", and it carries proof in
//! both directions. Five things follow, and this module owns all five.
//!
//! 1. **Minting creates the office record first.** An operator mints for a
//!    *planned* office, so the controller knows who holds what before anything
//!    is presented. That is GitLab's modern runner flow, and it is what turns
//!    possession into a record rather than into membership.
//! 2. **The secret is stored hashed, and never leaves storage.** The controller
//!    keeps `sha256(secret)`, never the secret, and never serialises the digest
//!    into an API response either — see [`PlatformOfficeJoinToken`].
//! 3. **Verification is constant-time, per-office, and atomic.** Flipping one
//!    record's status locks out exactly one office, by construction, and the
//!    use is recorded by a conditional `UPDATE` rather than by a full-row
//!    rewrite, so a revoke landing mid-verification is never undone.
//! 4. **The office verifies the controller with a public key.** It sends a
//!    nonce and refuses a response whose Ed25519 signature does not verify
//!    under the verification key its token carried. What the office stores
//!    verifies and forges nothing; what the controller stores about the office
//!    forges nothing either.
//! 5. **The same key carries the vouch.** §2's third verb needs the office to
//!    accept an identity it may never have heard of, without the controller
//!    knowing any password there. The controller's signature already proves the
//!    controller to the office, so the vouch is one more signed message — no
//!    second credential and no key exchange. Every signed message names the
//!    office's current token fingerprint, so rotating one office's token stops
//!    every proof to that office and to no other.

use rand::RngExt as _;

use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::infra::cluster::config::ClusterRole;
use crate::infra::cluster::security::controller_key::ControllerSigningKey;
use crate::infra::cluster::security::join_token::{
    self, JoinToken, JoinTokenError, OfficeVouch, OfficeVouchError, VOUCH_TTL_SECS,
    controller_call_header, digests_match, registration_proof, secret_digest, token_fingerprint,
    verify_controller_call_header, verify_registration_proof, vouch_nonce,
};
use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    AcceptedOfficeVouch, ClusterJoinTokenMintRequest, ClusterMintedJoinToken, ClusterOfficeVouch,
    JOIN_TOKEN_STATUS_ACTIVE, JOIN_TOKEN_STATUS_REVOKED, PlatformOffice,
    PlatformOfficeIdentityWrite, PlatformOfficeJoinToken, PlatformOfficeVouchRedemption, now_ts,
    slug_segment,
};

/// Where an office keeps the token it was issued.
///
/// `platform/` and not `.bootstrap/`: `instance-directory.md` classes the
/// bootstrap password as a "bootstrap secret" that dies at first use, and this
/// is the opposite — STORE tier, instance-local, read on every start for the
/// life of the join. It is deliberately a separate file from the credential
/// encryption key (`stability-matrix.md` row 14e, unbuilt): that key protects
/// the catalog and must outlive any membership change, while this token is
/// rewritten whenever the office is re-issued one. One file, one lifecycle.
const OFFICE_TOKEN_REL: &str = "platform/office-join-token";

/// Path on an office that spends a vouch.
pub const OFFICE_VOUCH_REDEEM_PATH: &str = "/office/vouch";

/// Where a controller keeps the private key it signs with.
///
/// `platform/` for the same reason the office's token is there: STORE tier,
/// instance-local, read on every start for the life of the controller. It is
/// the one file on a controller whose loss is not recoverable by re-reading
/// anything else — every office was issued its public half inside a join token,
/// so replacing it means re-minting every office's token.
const CONTROLLER_KEY_REL: &str = "platform/cluster-signing-key";

/// What an office holds after it has been issued a token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfficeJoinIdentity {
    /// Office this token names.
    pub office_id: String,
    /// The token as presented to the controller.
    pub token: String,
    /// `sha256(secret)` — what the controller stores, derived here from the
    /// secret this office holds. Used only to derive [`Self::fingerprint`].
    pub secret_digest: String,
    /// The controller's Ed25519 verification key, as the token delivered it.
    ///
    /// Everything this office believes about its controller, it believes
    /// because of this value. It verifies and forges nothing.
    pub controller_verify_key: String,
    /// `sha256(secret_digest)` — which membership a controller proof is about.
    pub fingerprint: String,
}

impl OfficeJoinIdentity {
    fn from_token(token: JoinToken) -> Self {
        let secret_digest = token.secret_digest();
        Self {
            office_id: token.office_id.clone(),
            fingerprint: token_fingerprint(&secret_digest),
            secret_digest,
            controller_verify_key: token.controller_verify_key.clone(),
            token: token.render(),
        }
    }

    /// Whether `proof` proves the responder is this office's controller.
    pub fn verify_registration_proof(&self, nonce: &str, proof: &str) -> bool {
        verify_registration_proof(
            &self.controller_verify_key,
            &self.office_id,
            &self.fingerprint,
            nonce,
            proof,
        )
    }

    /// Whether a presented controller-call header is really from the controller.
    pub fn verify_controller_header(&self, raw: &str) -> bool {
        verify_controller_call_header(
            &self.controller_verify_key,
            &self.office_id,
            &self.fingerprint,
            raw,
        )
    }
}

/// One office that presented a valid token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VerifiedOffice {
    /// Office the token named.
    pub office_id: String,
    /// The stored record, with `last_used_at` already advanced.
    pub record: PlatformOfficeJoinToken,
}

/// Controller-side minting and verification, office-side identity.
#[derive(Clone)]
pub struct ClusterJoinTokenService {
    data: Arc<dyn DataAdapter>,
    role: ClusterRole,
    identity: Option<OfficeJoinIdentity>,
    /// The controller's private signing key. `None` on an office, always: an
    /// office that held one could sign for its own controller.
    signing: Option<Arc<ControllerSigningKey>>,
}

impl ClusterJoinTokenService {
    /// Build the service for one process.
    pub fn new(
        data: Arc<dyn DataAdapter>,
        role: ClusterRole,
        identity: Option<OfficeJoinIdentity>,
        signing: Option<Arc<ControllerSigningKey>>,
    ) -> Self {
        Self {
            data,
            role,
            identity,
            // An office never signs. Enforced here rather than trusted to
            // every caller, because the one thing that must never be true is
            // an office holding a key its own controller's offices verify.
            signing: if role == ClusterRole::Worker {
                None
            } else {
                signing
            },
        }
    }

    /// This office's own issued token, when it has one.
    pub fn office_identity(&self) -> Option<&OfficeJoinIdentity> {
        self.identity.as_ref()
    }

    /// The controller's public verification key, as offices are issued it.
    pub fn controller_verify_key(&self) -> Option<&str> {
        self.signing
            .as_deref()
            .map(ControllerSigningKey::verify_key)
    }

    fn signing_key(&self) -> Result<&ControllerSigningKey, PlatformError> {
        self.signing.as_deref().ok_or_else(|| {
            PlatformError::new(
                "CLUSTER_CONTROLLER_KEY_MISSING",
                format!(
                    "this instance holds no cluster signing key, so it cannot prove itself to \
                     any office. The key lives at '{CONTROLLER_KEY_REL}' in the data root and \
                     is created on the first start of a controller; an office never has one."
                ),
            )
        })
    }

    /// Mint a token for one office, creating the office record if needed.
    pub fn mint(
        &self,
        request: &ClusterJoinTokenMintRequest,
    ) -> Result<ClusterMintedJoinToken, PlatformError> {
        let office_id = normalize_office_id(&request.office_id)?;
        let now = now_ts();
        // "An office without a `base_url` — an office nothing can reach is not
        // an office" (`kinds/office-topology/README.md`, Rejections). Minting
        // is where the record is created, so it is where the refusal belongs:
        // a planned office with no address is a row nothing can ever place
        // work on, and no later call is obliged to supply one.
        let base_url = super::normalize_office_base_url(&request.base_url)?;

        if let Some(existing) = self.data.get_office_join_token(&office_id)?
            && !request.rotate
        {
            return Err(PlatformError::new(
                "CLUSTER_JOIN_TOKEN_EXISTS",
                format!(
                    "office '{office_id}' already holds a join token (status {}, minted at {}). \
                     Re-mint with \"rotate\": true to replace it, which invalidates the token \
                     that office is using now.",
                    existing.status, existing.created_at
                ),
            ));
        }

        // The office record comes first: a token with no office is a secret
        // nobody can attribute, which is the defect this replaces.
        let office = match self.data.get_platform_office(&office_id)? {
            Some(mut existing) => {
                if !request.label.trim().is_empty() {
                    existing.label = request.label.trim().to_string();
                }
                existing.base_url = base_url.clone();
                existing.updated_at = now;
                existing
            }
            None => PlatformOffice {
                office_id: office_id.clone(),
                office_slug: office_id.clone(),
                label: if request.label.trim().is_empty() {
                    office_id.clone()
                } else {
                    request.label.trim().to_string()
                },
                office_kind: "office".to_string(),
                base_url: base_url.clone(),
                // Planned, not online: nothing has registered yet, and saying
                // "online" here would put an unreachable office in the
                // directory as a healthy one.
                status: "planned".to_string(),
                created_at: now,
                updated_at: now,
            },
        };
        self.data.put_platform_office(&office)?;

        // The token carries the controller's public key, because an office
        // handed a key later would have to trust it on first use.
        let token = JoinToken::mint(office_id.clone(), self.signing_key()?.verify_key());
        let record = PlatformOfficeJoinToken {
            office_id: office_id.clone(),
            secret_digest: token.secret_digest(),
            status: JOIN_TOKEN_STATUS_ACTIVE.to_string(),
            note: request.note.trim().to_string(),
            created_at: now,
            last_used_at: 0,
        };
        self.data.put_office_join_token(&record)?;

        Ok(ClusterMintedJoinToken {
            record,
            office,
            token: token.render(),
        })
    }

    /// Revoke one office's token, locking out that office and no other.
    pub fn revoke(&self, office_id: &str) -> Result<PlatformOfficeJoinToken, PlatformError> {
        let office_id = normalize_office_id(office_id)?;
        let mut record = self
            .data
            .get_office_join_token(&office_id)?
            .ok_or_else(|| {
                PlatformError::new(
                    "CLUSTER_JOIN_TOKEN_UNKNOWN",
                    format!("no join token has been minted for office '{office_id}'"),
                )
            })?;
        // The row is kept rather than deleted: a revoked token is a record of
        // who held what, and deleting it would erase exactly the traceability
        // minting exists to create.
        record.status = JOIN_TOKEN_STATUS_REVOKED.to_string();
        self.data.put_office_join_token(&record)?;
        Ok(record)
    }

    /// Every issued token record. Digests, never secrets.
    pub fn list(&self) -> Result<Vec<PlatformOfficeJoinToken>, PlatformError> {
        self.data.list_office_join_tokens()
    }

    /// Verify a token an office presented, and record the use.
    ///
    /// **Direction.** Only a controller accepts this. An office holds no
    /// `office_join_tokens` rows, so the lookup would fail anyway, but the role
    /// is checked first and by name: the two halves of §8 are different
    /// credentials for different directions, and an instance that accepted the
    /// wrong half would be treating one direction's value as the other's.
    ///
    /// **Recording the use is a conditional `UPDATE`, never a row rewrite.**
    /// Offices heartbeat every ten seconds, so a read-modify-write of the whole
    /// row loses any revoke or rotate that commits in between — silently
    /// putting `status` back to `active`, or restoring the digest of a token
    /// that was just replaced. The write is therefore
    /// `SET last_used_at WHERE office_id AND status = 'active' AND
    /// secret_digest = ?`, and zero affected rows is a *verification failure*
    /// rather than a bookkeeping miss: the record the checks above ran against
    /// no longer exists.
    pub fn verify_office_token(&self, raw: &str) -> Result<VerifiedOffice, PlatformError> {
        if self.role != ClusterRole::Master {
            return Err(PlatformError::new(
                "CLUSTER_JOIN_TOKEN_NOT_A_CONTROLLER",
                "only a controller verifies an office's join token; this instance is not one. \
                 An office proves a peer with the controller's signature, never with a token \
                 it was issued (`offices.md` §8).",
            ));
        }
        let presented = JoinToken::parse(raw).map_err(join_token_error)?;
        let office_id = presented.office_id.clone();
        let mut record = self
            .data
            .get_office_join_token(&office_id)?
            .ok_or_else(|| {
                PlatformError::new(
                    "CLUSTER_JOIN_TOKEN_UNKNOWN",
                    format!(
                        "no join token has been minted for office '{office_id}'. \
                     Mint one on the controller (POST /api/cluster/join-tokens)."
                    ),
                )
            })?;
        let presented_digest = presented.secret_digest();
        if !digests_match(&record.secret_digest, &presented_digest) {
            return Err(PlatformError::new(
                "CLUSTER_JOIN_TOKEN_INVALID",
                format!(
                    "the join token presented for office '{office_id}' does not match the one \
                     issued to it. Mint a replacement on the controller \
                     (POST /api/cluster/join-tokens with \"rotate\": true)."
                ),
            ));
        }
        if !record.is_active() {
            // Checked after the secret, so a revoked office learns no more
            // from the refusal than a wrong one does.
            return Err(PlatformError::new(
                "CLUSTER_JOIN_TOKEN_REVOKED",
                format!(
                    "the join token for office '{office_id}' is revoked. \
                     Mint a replacement on the controller \
                     (POST /api/cluster/join-tokens with \"rotate\": true)."
                ),
            ));
        }
        let used_at = now_ts();
        if !self
            .data
            .touch_office_join_token(&office_id, &presented_digest, used_at)?
        {
            return Err(PlatformError::new(
                "CLUSTER_JOIN_TOKEN_STALE",
                format!(
                    "the join token for office '{office_id}' was revoked or rotated while this \
                     request was being verified, so it is not the token in force. Present the \
                     current token, or mint a replacement on the controller \
                     (POST /api/cluster/join-tokens with \"rotate\": true)."
                ),
            ));
        }
        record.last_used_at = used_at;
        Ok(VerifiedOffice { office_id, record })
    }

    /// The controller's proof that it is this office's controller.
    ///
    /// A signature, not a value derived from the record: a host that read every
    /// byte the controller stores about this office still cannot answer the
    /// office's nonce.
    pub fn registration_proof_for(&self, office_id: &str, nonce: &str) -> Option<String> {
        if nonce.trim().is_empty() {
            return None;
        }
        let record = self.data.get_office_join_token(office_id).ok().flatten()?;
        Some(registration_proof(
            self.signing.as_deref()?,
            office_id,
            &token_fingerprint(&record.secret_digest),
            nonce,
        ))
    }

    /// The header value this controller presents when calling one of its offices.
    pub fn controller_call_header_for(&self, office_id: &str) -> Result<String, PlatformError> {
        let office_id = normalize_office_id(office_id)?;
        let record = self.data.get_office_join_token(&office_id)?.ok_or_else(|| {
            PlatformError::new(
                "CLUSTER_JOIN_TOKEN_UNKNOWN",
                format!(
                    "no join token has been minted for office '{office_id}', so this controller \
                     cannot prove itself to it. Mint one (POST /api/cluster/join-tokens)."
                ),
            )
        })?;
        if !record.is_active() {
            return Err(PlatformError::new(
                "CLUSTER_JOIN_TOKEN_REVOKED",
                format!("the join token for office '{office_id}' is revoked"),
            ));
        }
        Ok(controller_call_header(
            self.signing_key()?,
            &office_id,
            &token_fingerprint(&record.secret_digest),
        ))
    }

    /// Mint a vouch for one identity at one office (`offices.md` §2, "vouch").
    ///
    /// Nothing is written. A vouch is one signature over four values plus the
    /// office's current token fingerprint, so minting is a read plus a sign —
    /// which is also why it survives a read-only controller and why re-minting
    /// after a lost redirect costs nothing.
    ///
    /// Revocation reaches this for free, and that is the design rather than a
    /// coincidence: the same record and the same `is_active` check that stop a
    /// heartbeat stop a mint. There is no second list to remember to update.
    /// Rotation goes further and kills vouching *cryptographically* — the
    /// office's token fingerprint is inside the signed message, so an office
    /// running a rotated token computes a different one and nothing minted
    /// under the old membership verifies there. Plain revocation is the
    /// controller declining to sign with a key it still has, so a vouch minted
    /// moments before a revoke stays redeemable at that office until it
    /// expires. That window is the TTL and no longer, and it is the reason the
    /// TTL is two minutes rather than an hour; an operator who needs an office
    /// shut out *now*, with no network to that office, rotates rather than
    /// revokes, and the office enforces that itself.
    pub fn mint_vouch(
        &self,
        office_id: &str,
        identity: &str,
        now: i64,
    ) -> Result<ClusterOfficeVouch, PlatformError> {
        if self.role != ClusterRole::Master {
            return Err(PlatformError::new(
                "CLUSTER_VOUCH_NOT_A_CONTROLLER",
                "only a controller vouches for an identity at an office                  (`offices.md` §2 gives the three verbs to the controller alone)",
            ));
        }
        let office_id = normalize_office_id(office_id)?;
        let identity = slug_segment(identity);
        if identity.is_empty() {
            return Err(PlatformError::new(
                "CLUSTER_VOUCH_IDENTITY_INVALID",
                "a vouch must name the identity it vouches for",
            ));
        }
        let record = self.data.get_office_join_token(&office_id)?.ok_or_else(|| {
            PlatformError::new(
                "CLUSTER_JOIN_TOKEN_UNKNOWN",
                format!(
                    "no join token has been minted for office '{office_id}', so this controller \
                     holds nothing that office would accept a vouch under. Mint one \
                     (POST /api/cluster/join-tokens)."
                ),
            )
        })?;
        if !record.is_active() {
            return Err(PlatformError::new(
                "CLUSTER_JOIN_TOKEN_REVOKED",
                format!(
                    "the join token for office '{office_id}' is revoked, so this controller no \
                     longer vouches for anybody there. Re-mint with \"rotate\": true to restore \
                     membership, which also invalidates every vouch issued under the old token."
                ),
            ));
        }
        let expires_at = now + VOUCH_TTL_SECS;
        let vouch = OfficeVouch::mint(
            self.signing_key()?,
            &office_id,
            &identity,
            expires_at,
            &vouch_nonce(),
            &token_fingerprint(&record.secret_digest),
        )
        .render();
        let base_url = self
            .data
            .get_platform_office(&office_id)?
            .map(|office| office.base_url.trim_end_matches('/').to_string())
            .unwrap_or_default();
        let redeem_url = if base_url.is_empty() {
            String::new()
        } else {
            format!(
                "{base_url}{OFFICE_VOUCH_REDEEM_PATH}?v={}",
                query_encode(&vouch)
            )
        };
        Ok(ClusterOfficeVouch {
            office_id,
            identity,
            expires_at,
            ttl_seconds: VOUCH_TTL_SECS,
            vouch,
            redeem_url,
        })
    }

    /// Verify and spend one vouch presented to this office.
    ///
    /// The controller is not contacted, and could not usefully be: everything
    /// checked here is checked with the secret this office already stores.
    /// That is required rather than convenient. §3 records that "the
    /// controller's death costs logins, never execution", and §6 exists so the
    /// credential that repairs a relationship is never held by the unreachable
    /// party — a redemption that phoned home would reintroduce exactly that
    /// dependency at exactly the moment it hurts.
    ///
    /// Spending is the last step and it is atomic, so two redemptions of one
    /// vouch arriving together produce one session and one refusal rather than
    /// two sessions.
    pub fn redeem_vouch(&self, raw: &str, now: i64) -> Result<AcceptedOfficeVouch, PlatformError> {
        let Some(identity) = self.identity.as_ref() else {
            return Err(PlatformError::new(
                "CLUSTER_VOUCH_NOT_AN_OFFICE",
                "this instance has joined no controller, so nobody vouches for anybody here. \
                 A vouch is redeemable only on an office that holds a join token \
                 (`offices.md` §4).",
            ));
        };
        let vouch = OfficeVouch::parse(raw).map_err(vouch_error)?;
        vouch
            .verify(
                &identity.controller_verify_key,
                &identity.office_id,
                &identity.fingerprint,
                now,
            )
            .map_err(vouch_error)?;

        let claimed = self
            .data
            .claim_office_vouch_nonce(&PlatformOfficeVouchRedemption {
                nonce: vouch.nonce.clone(),
                office_id: vouch.office_id.clone(),
                identity: vouch.identity.clone(),
                expires_at: vouch.expires_at,
                redeemed_at: now,
            })?;
        if !claimed {
            return Err(PlatformError::new(
                "CLUSTER_VOUCH_ALREADY_REDEEMED",
                "this vouch has already been spent. A vouch is a hand-off and opens one \
                 session; ask the controller for a fresh one.",
            ));
        }
        Ok(AcceptedOfficeVouch {
            office_id: vouch.office_id,
            identity: vouch.identity,
            expires_at: vouch.expires_at,
        })
    }

    /// Append one row to this office's identity-write log (`offices.md` §8).
    pub fn record_identity_write(
        &self,
        owner: &str,
        action: &str,
        role: &str,
        source: &str,
        detail: &str,
    ) -> Result<PlatformOfficeIdentityWrite, PlatformError> {
        let mut bytes = [0u8; 16];
        rand::rng().fill(&mut bytes);
        let entry = PlatformOfficeIdentityWrite {
            write_id: hex::encode(bytes),
            office_id: self
                .identity
                .as_ref()
                .map(|identity| identity.office_id.clone())
                .unwrap_or_default(),
            owner: owner.to_string(),
            action: action.to_string(),
            role: role.to_string(),
            source: source.to_string(),
            detail: detail.to_string(),
            written_at: now_ts(),
        };
        self.data.put_office_identity_write(&entry)?;
        Ok(entry)
    }

    /// Read this office's identity-write log, newest first.
    pub fn list_identity_writes(
        &self,
        limit: usize,
    ) -> Result<Vec<PlatformOfficeIdentityWrite>, PlatformError> {
        self.data.list_office_identity_writes(limit.clamp(1, 500))
    }

    /// Whether a presented header is **this office's controller** calling it.
    ///
    /// One direction and one only. It used to be a role-switching predicate
    /// that, on a controller, returned true for any office's join token — and
    /// every project-capability check short-circuited on it, so one office's
    /// token authorised acting as any owner on the controller. The two halves
    /// of §8 are different credentials for different directions and are no
    /// longer reachable through one name.
    ///
    /// A controller answers `false` here always: nothing calls *into* a
    /// controller with a controller-call header, and a controller that accepted
    /// one would be accepting a value it mints itself.
    pub fn controller_call_authenticates(&self, raw: &str) -> bool {
        if self.role != ClusterRole::Worker {
            return false;
        }
        self.identity
            .as_ref()
            .is_some_and(|identity| identity.verify_controller_header(raw))
    }
}

/// Map a refusal from the vouch scheme onto a platform error code.
///
/// One code per reason and never a merged one: an operator at the wrong office
/// and an operator holding a stale vouch need different next actions, and a
/// single `CLUSTER_VOUCH_INVALID` would hide which they are.
fn vouch_error(err: OfficeVouchError) -> PlatformError {
    let code = match err {
        OfficeVouchError::OfficeMismatch { .. } => "CLUSTER_VOUCH_OFFICE_MISMATCH",
        OfficeVouchError::ProofInvalid => "CLUSTER_VOUCH_INVALID",
        OfficeVouchError::Expired { .. } => "CLUSTER_VOUCH_EXPIRED",
        _ => "CLUSTER_VOUCH_MALFORMED",
    };
    PlatformError::new(code, err.to_string())
}

/// Percent-encode one query-string value.
///
/// A rendered vouch is slugs, digits, hex, and `:` separators today, so this is
/// mostly a no-op — which is the point of doing it anyway. A later scheme word
/// carrying anything else must not silently produce a URL that truncates at the
/// first reserved character.
fn query_encode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char);
            }
            byte => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn normalize_office_id(raw: &str) -> Result<String, PlatformError> {
    let value = slug_segment(raw);
    if value.is_empty() {
        return Err(PlatformError::new(
            "CLUSTER_JOIN_TOKEN_OFFICE_INVALID",
            format!("'{raw}' is not a usable office id"),
        ));
    }
    Ok(value)
}

fn join_token_error(err: JoinTokenError) -> PlatformError {
    PlatformError::new("CLUSTER_JOIN_TOKEN_MALFORMED", err.to_string())
}

/// Where this office keeps its issued token.
pub fn office_token_path(data_root: &Path) -> PathBuf {
    data_root.join(OFFICE_TOKEN_REL)
}

/// Reconcile the configured token with the one this office already stored.
///
/// The environment variable stays the way a token is supplied for a *first*
/// join; after that the office uses what it stored. Disagreement is a refusal,
/// never a silent overwrite in either direction — n8n's mismatch behaviour, and
/// the same rule the Credential contract states for encryption keys. Two
/// different tokens on one office means somebody is wrong about which office
/// this is, and guessing turns that into a mystery a week later.
pub fn resolve_office_identity(
    data_root: &Path,
    configured: Option<&str>,
) -> Result<Option<OfficeJoinIdentity>, PlatformError> {
    let path = office_token_path(data_root);
    let stored = read_stored_token(&path)?;
    let configured = configured.map(str::trim).filter(|value| !value.is_empty());

    match (stored, configured) {
        (None, None) => Ok(None),
        (Some(stored), None) => Ok(Some(OfficeJoinIdentity::from_token(
            JoinToken::parse(&stored).map_err(|err| stored_token_error(&path, err))?,
        ))),
        (None, Some(configured)) => {
            let token = JoinToken::parse(configured).map_err(join_token_error)?;
            write_stored_token(&path, &token.render())?;
            Ok(Some(OfficeJoinIdentity::from_token(token)))
        }
        (Some(stored), Some(configured)) => {
            let stored_token =
                JoinToken::parse(&stored).map_err(|err| stored_token_error(&path, err))?;
            let configured_token = JoinToken::parse(configured).map_err(join_token_error)?;
            if stored_token != configured_token {
                return Err(PlatformError::new(
                    "CLUSTER_JOIN_TOKEN_MISMATCH",
                    format!(
                        "ZEBFLOW_CLUSTER_JOIN_TOKEN names office '{}' but this data root already \
                         holds a token for office '{}' at '{}'. Refusing to start rather than \
                         overwrite either. Keep the stored token and unset the variable, or \
                         delete the file if this office is being re-issued.",
                        configured_token.office_id,
                        stored_token.office_id,
                        path.display()
                    ),
                ));
            }
            Ok(Some(OfficeJoinIdentity::from_token(stored_token)))
        }
    }
}

fn stored_token_error(path: &Path, err: JoinTokenError) -> PlatformError {
    PlatformError::new(
        "CLUSTER_JOIN_TOKEN_STORED_INVALID",
        format!(
            "the token stored at '{}' is unusable: {err}",
            path.display()
        ),
    )
}

fn read_stored_token(path: &Path) -> Result<Option<String>, PlatformError> {
    match fs::read_to_string(path) {
        Ok(value) => {
            let value = value.trim().to_string();
            if value.is_empty() {
                Ok(None)
            } else {
                Ok(Some(value))
            }
        }
        Err(err) if err.kind() == ErrorKind::NotFound => Ok(None),
        Err(err) => Err(PlatformError::new(
            "CLUSTER_JOIN_TOKEN_STORED_UNREADABLE",
            format!("failed reading '{}': {err}", path.display()),
        )),
    }
}

fn write_stored_token(path: &Path, value: &str) -> Result<(), PlatformError> {
    let mut bytes = value.as_bytes().to_vec();
    bytes.push(b'\n');
    write_private_file(path, &bytes, "CLUSTER_JOIN_TOKEN_STORE_WRITE")
}

/// Write one file only this instance's user may read.
///
/// Shared by the office's token and the controller's signing key because both
/// are STORE-tier private material in the same directory, and two copies of a
/// 0600 dance is how one of them ends up 0644.
fn write_private_file(path: &Path, bytes: &[u8], code: &'static str) -> Result<(), PlatformError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            PlatformError::new(
                code,
                format!("failed creating '{}': {err}", parent.display()),
            )
        })?;
    }
    let mut options = OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(path).map_err(|err| {
        PlatformError::new(code, format!("failed creating '{}': {err}", path.display()))
    })?;
    file.write_all(bytes)
        .and_then(|()| file.sync_all())
        .map_err(|err| {
            PlatformError::new(code, format!("failed writing '{}': {err}", path.display()))
        })?;
    // An existing file keeps its old mode through `create`, so tighten after
    // the fact as well as at open time.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|err| {
            PlatformError::new(code, format!("failed securing '{}': {err}", path.display()))
        })?;
    }
    Ok(())
}

/// Where this instance keeps the private key it signs office proofs with.
pub fn controller_signing_key_path(data_root: &Path) -> PathBuf {
    data_root.join(CONTROLLER_KEY_REL)
}

/// Load this controller's signing key, generating one on its first start.
///
/// Generated rather than configured, and never regenerated on a read failure: a
/// key that quietly replaced itself would invalidate every token this
/// controller ever issued, because the public half travelled inside each one.
/// A damaged file is a refusal that says exactly that.
///
/// An office calls this for nothing. Enforced by the caller — an office is not
/// built with one, and [`ClusterJoinTokenService::new`] drops one if handed it.
pub fn resolve_controller_signing_key(
    data_root: &Path,
) -> Result<ControllerSigningKey, PlatformError> {
    let path = controller_signing_key_path(data_root);
    match fs::read(&path) {
        Ok(document) => ControllerSigningKey::from_pkcs8(&document).map_err(|err| {
            PlatformError::new(
                "CLUSTER_CONTROLLER_KEY_UNUSABLE",
                format!("'{}': {err}", path.display()),
            )
        }),
        Err(err) if err.kind() == ErrorKind::NotFound => {
            let (document, key) = ControllerSigningKey::generate().map_err(|err| {
                PlatformError::new("CLUSTER_CONTROLLER_KEY_UNUSABLE", err.to_string())
            })?;
            write_private_file(&path, &document, "CLUSTER_CONTROLLER_KEY_WRITE")?;
            Ok(key)
        }
        Err(err) => Err(PlatformError::new(
            "CLUSTER_CONTROLLER_KEY_UNUSABLE",
            format!("failed reading '{}': {err}", path.display()),
        )),
    }
}

/// Digest of a secret, re-exported for callers that hold only the secret half.
pub fn digest_of(secret: &str) -> String {
    secret_digest(secret)
}

/// Fresh nonce for one registration attempt.
pub fn nonce() -> String {
    join_token::registration_nonce()
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::infra::cluster::registry::WorkerRegistryRecord;
    use crate::infra::execution::placement::ProjectRuntimePlacement;
    use crate::platform::model::{
        McpSession, PipelineMeta, PlatformProject, PlatformUser, ProjectCredential,
        ProjectDbConnection, ProjectInvite, ProjectMember, ProjectPolicy, ProjectPolicyBinding,
        StoredUser,
    };

    fn temp_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "zebflow-join-token-{name}-{}-{}",
            std::process::id(),
            now_ts()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).expect("temp root");
        root
    }

    fn controller() -> ControllerSigningKey {
        ControllerSigningKey::generate().expect("generate").1
    }

    fn mint_for(office_id: &str, key: &ControllerSigningKey) -> JoinToken {
        JoinToken::mint(office_id, key.verify_key())
    }

    #[test]
    fn a_first_join_stores_the_supplied_token_privately() {
        let root = temp_root("first-join");
        let key = controller();
        let minted = mint_for("office-a", &key);
        let identity = resolve_office_identity(&root, Some(&minted.render()))
            .expect("first join")
            .expect("identity");
        assert_eq!(identity.office_id, "office-a");
        assert_eq!(identity.secret_digest, minted.secret_digest());
        assert_eq!(identity.controller_verify_key, key.verify_key());
        assert_eq!(identity.fingerprint, minted.fingerprint());

        let path = office_token_path(&root);
        assert_eq!(
            fs::read_to_string(&path).expect("stored token").trim(),
            minted.render()
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).expect("meta").permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }

        // A later start with the variable gone reads what it stored.
        let reloaded = resolve_office_identity(&root, None)
            .expect("second start")
            .expect("identity");
        assert_eq!(reloaded, identity);
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_disagreeing_environment_variable_refuses_to_start() {
        let root = temp_root("mismatch");
        let key = controller();
        let stored = mint_for("office-a", &key);
        resolve_office_identity(&root, Some(&stored.render())).expect("first join");

        let other = mint_for("office-b", &key);
        let err = resolve_office_identity(&root, Some(&other.render()))
            .expect_err("mismatch must refuse");
        assert_eq!(err.code, "CLUSTER_JOIN_TOKEN_MISMATCH");
        assert!(err.message.contains("office-a") && err.message.contains("office-b"));

        // Neither side was overwritten.
        assert_eq!(
            fs::read_to_string(office_token_path(&root))
                .expect("stored")
                .trim(),
            stored.render()
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn the_same_token_supplied_again_is_not_a_mismatch() {
        let root = temp_root("same-token");
        let key = controller();
        let stored = mint_for("office-a", &key);
        resolve_office_identity(&root, Some(&stored.render())).expect("first join");
        let again = resolve_office_identity(&root, Some(&stored.render()))
            .expect("same token")
            .expect("identity");
        assert_eq!(again.office_id, "office-a");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_office_refuses_a_response_that_does_not_prove_the_controller() {
        let key = controller();
        let issued = mint_for("office-a", &key);
        let identity = OfficeJoinIdentity::from_token(issued.clone());
        let sent = nonce();

        let honest = registration_proof(&key, "office-a", &issued.fingerprint(), &sent);
        assert!(identity.verify_registration_proof(&sent, &honest));

        // Anything that is not this controller's key.
        let impostor = controller();
        assert!(!identity.verify_registration_proof(
            &sent,
            &registration_proof(&impostor, "office-a", &issued.fingerprint(), &sent)
        ));
        assert!(!identity.verify_registration_proof(&sent, ""));
        assert!(!identity.verify_registration_proof(&sent, "not-a-proof"));
        // A proof for a different nonce is a replay, not an answer.
        assert!(!identity.verify_registration_proof(
            &sent,
            &registration_proof(&key, "office-a", &issued.fingerprint(), &nonce())
        ));
        // And the material the controller stores about this office answers
        // nothing: the digest is not the key any more.
        assert!(!identity.verify_registration_proof(&sent, &issued.secret_digest()));
    }

    #[test]
    fn an_office_accepts_only_its_own_controller_call_header() {
        let key = controller();
        let issued = mint_for("office-a", &key);
        let identity = OfficeJoinIdentity::from_token(issued.clone());

        assert!(identity.verify_controller_header(&controller_call_header(
            &key,
            "office-a",
            &issued.fingerprint()
        )));
        // A header for another office, or under another controller's key, or
        // for a rotated membership, is not ours.
        assert!(!identity.verify_controller_header(&controller_call_header(
            &key,
            "office-b",
            &issued.fingerprint()
        )));
        assert!(!identity.verify_controller_header(&controller_call_header(
            &controller(),
            "office-a",
            &issued.fingerprint()
        )));
        assert!(!identity.verify_controller_header(&controller_call_header(
            &key,
            "office-a",
            &mint_for("office-a", &key).fingerprint()
        )));
        // The office's own token is not a controller header: the two
        // directions do not share a scheme word, so neither replays as the
        // other.
        assert!(!identity.verify_controller_header(&issued.render()));
        assert!(!identity.verify_controller_header(""));
    }

    #[test]
    fn the_old_shared_secret_shape_refuses_and_names_the_fix() {
        let root = temp_root("legacy-secret");
        let err =
            resolve_office_identity(&root, Some("join-token")).expect_err("legacy shape refuses");
        assert_eq!(err.code, "CLUSTER_JOIN_TOKEN_MALFORMED");
        assert!(
            err.message.contains("Mint one on the controller"),
            "{}",
            err.message
        );
        assert!(
            !office_token_path(&root).exists(),
            "a refused token must not be stored"
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_controller_key_is_created_once_and_reused_for_the_life_of_the_instance() {
        let root = temp_root("controller-key");
        let first = resolve_controller_signing_key(&root).expect("first start");
        let path = controller_signing_key_path(&root);
        assert!(path.is_file());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(&path).expect("meta").permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "the signing key is not a world-readable file");
        }
        let second = resolve_controller_signing_key(&root).expect("second start");
        assert_eq!(
            first.verify_key(),
            second.verify_key(),
            "regenerating would invalidate every token this controller ever issued"
        );

        // A damaged key refuses rather than silently minting a new identity.
        fs::write(&path, b"not a key").expect("damage");
        let err = resolve_controller_signing_key(&root).expect_err("damaged key");
        assert_eq!(err.code, "CLUSTER_CONTROLLER_KEY_UNUSABLE");
        assert!(
            err.message.contains("re-mint every office's token"),
            "{err:?}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// A data adapter that lets a test commit something *between* another
    /// caller's read and its write.
    ///
    /// The window `verify_office_token` used to leave open is exactly one
    /// instruction wide in wall-clock terms and permanently open in logical
    /// terms, because offices heartbeat every ten seconds. Racing threads would
    /// reproduce it most of the time; firing the interleave from inside the
    /// read reproduces it every time, which is what a regression test owes.
    struct InterleavingAdapter {
        inner: Arc<dyn DataAdapter>,
        /// Run once, on the way out of the first `get_office_join_token`.
        on_first_read: Box<dyn Fn(&dyn DataAdapter) + Send + Sync>,
        fired: std::sync::atomic::AtomicBool,
    }

    impl DataAdapter for InterleavingAdapter {
        fn get_office_join_token(
            &self,
            office_id: &str,
        ) -> Result<Option<PlatformOfficeJoinToken>, PlatformError> {
            let record = self.inner.get_office_join_token(office_id)?;
            if record.is_some() && !self.fired.swap(true, std::sync::atomic::Ordering::SeqCst) {
                (self.on_first_read)(self.inner.as_ref());
            }
            Ok(record)
        }

        // The office methods this decorator has to carry: they have default
        // trait bodies that refuse, so delegating them is not optional.
        fn put_office_join_token(
            &self,
            token: &PlatformOfficeJoinToken,
        ) -> Result<(), PlatformError> {
            self.inner.put_office_join_token(token)
        }
        fn touch_office_join_token(
            &self,
            office_id: &str,
            secret_digest: &str,
            last_used_at: i64,
        ) -> Result<bool, PlatformError> {
            self.inner
                .touch_office_join_token(office_id, secret_digest, last_used_at)
        }
        fn list_office_join_tokens(&self) -> Result<Vec<PlatformOfficeJoinToken>, PlatformError> {
            self.inner.list_office_join_tokens()
        }
        fn get_platform_office(
            &self,
            office_id: &str,
        ) -> Result<Option<PlatformOffice>, PlatformError> {
            self.inner.get_platform_office(office_id)
        }
        fn put_platform_office(&self, office: &PlatformOffice) -> Result<(), PlatformError> {
            self.inner.put_platform_office(office)
        }

        fn id(&self) -> &'static str {
            self.inner.id()
        }
        fn get_user_auth(&self, owner: &str) -> Result<Option<StoredUser>, PlatformError> {
            self.inner.get_user_auth(owner)
        }
        fn put_user(&self, user: &StoredUser) -> Result<(), PlatformError> {
            self.inner.put_user(user)
        }
        fn list_users(&self) -> Result<Vec<PlatformUser>, PlatformError> {
            self.inner.list_users()
        }
        fn get_project(
            &self,
            owner: &str,
            project: &str,
        ) -> Result<Option<PlatformProject>, PlatformError> {
            self.inner.get_project(owner, project)
        }
        fn put_project(&self, project: &PlatformProject) -> Result<(), PlatformError> {
            self.inner.put_project(project)
        }
        fn list_projects(&self, owner: &str) -> Result<Vec<PlatformProject>, PlatformError> {
            self.inner.list_projects(owner)
        }
        fn delete_project(&self, owner: &str, project: &str) -> Result<(), PlatformError> {
            self.inner.delete_project(owner, project)
        }
        fn get_project_credential(
            &self,
            owner: &str,
            project: &str,
            credential_id: &str,
        ) -> Result<Option<ProjectCredential>, PlatformError> {
            self.inner
                .get_project_credential(owner, project, credential_id)
        }
        fn put_project_credential(
            &self,
            credential: &ProjectCredential,
        ) -> Result<(), PlatformError> {
            self.inner.put_project_credential(credential)
        }
        fn list_project_credentials(
            &self,
            owner: &str,
            project: &str,
        ) -> Result<Vec<ProjectCredential>, PlatformError> {
            self.inner.list_project_credentials(owner, project)
        }
        fn delete_project_credential(
            &self,
            owner: &str,
            project: &str,
            credential_id: &str,
        ) -> Result<(), PlatformError> {
            self.inner
                .delete_project_credential(owner, project, credential_id)
        }
        fn get_project_db_connection(
            &self,
            owner: &str,
            project: &str,
            connection_slug: &str,
        ) -> Result<Option<ProjectDbConnection>, PlatformError> {
            self.inner
                .get_project_db_connection(owner, project, connection_slug)
        }
        fn put_project_db_connection(
            &self,
            connection: &ProjectDbConnection,
        ) -> Result<(), PlatformError> {
            self.inner.put_project_db_connection(connection)
        }
        fn list_project_db_connections(
            &self,
            owner: &str,
            project: &str,
        ) -> Result<Vec<ProjectDbConnection>, PlatformError> {
            self.inner.list_project_db_connections(owner, project)
        }
        fn delete_project_db_connection(
            &self,
            owner: &str,
            project: &str,
            connection_slug: &str,
        ) -> Result<(), PlatformError> {
            self.inner
                .delete_project_db_connection(owner, project, connection_slug)
        }
        fn put_pipeline_meta(&self, meta: &PipelineMeta) -> Result<(), PlatformError> {
            self.inner.put_pipeline_meta(meta)
        }
        fn delete_pipeline_meta(
            &self,
            owner: &str,
            project: &str,
            file_rel_path: &str,
        ) -> Result<(), PlatformError> {
            self.inner
                .delete_pipeline_meta(owner, project, file_rel_path)
        }
        fn list_pipeline_meta(
            &self,
            owner: &str,
            project: &str,
        ) -> Result<Vec<PipelineMeta>, PlatformError> {
            self.inner.list_pipeline_meta(owner, project)
        }
        fn put_project_policy(&self, policy: &ProjectPolicy) -> Result<(), PlatformError> {
            self.inner.put_project_policy(policy)
        }
        fn list_project_policies(
            &self,
            owner: &str,
            project: &str,
        ) -> Result<Vec<ProjectPolicy>, PlatformError> {
            self.inner.list_project_policies(owner, project)
        }
        fn put_project_policy_binding(
            &self,
            binding: &ProjectPolicyBinding,
        ) -> Result<(), PlatformError> {
            self.inner.put_project_policy_binding(binding)
        }
        fn list_project_policy_bindings(
            &self,
            owner: &str,
            project: &str,
        ) -> Result<Vec<ProjectPolicyBinding>, PlatformError> {
            self.inner.list_project_policy_bindings(owner, project)
        }
        fn delete_project_policy(
            &self,
            owner: &str,
            project: &str,
            policy_id: &str,
        ) -> Result<(), PlatformError> {
            self.inner.delete_project_policy(owner, project, policy_id)
        }
        fn delete_project_policy_binding(
            &self,
            owner: &str,
            project: &str,
            subject_id: &str,
        ) -> Result<(), PlatformError> {
            self.inner
                .delete_project_policy_binding(owner, project, subject_id)
        }
        fn get_project_member(
            &self,
            owner: &str,
            project: &str,
            user_id: &str,
        ) -> Result<Option<ProjectMember>, PlatformError> {
            self.inner.get_project_member(owner, project, user_id)
        }
        fn put_project_member(&self, member: &ProjectMember) -> Result<(), PlatformError> {
            self.inner.put_project_member(member)
        }
        fn list_project_members(
            &self,
            owner: &str,
            project: &str,
        ) -> Result<Vec<ProjectMember>, PlatformError> {
            self.inner.list_project_members(owner, project)
        }
        fn delete_project_member(
            &self,
            owner: &str,
            project: &str,
            user_id: &str,
        ) -> Result<(), PlatformError> {
            self.inner.delete_project_member(owner, project, user_id)
        }
        fn get_project_invite(
            &self,
            owner: &str,
            project: &str,
            invite_id: &str,
        ) -> Result<Option<ProjectInvite>, PlatformError> {
            self.inner.get_project_invite(owner, project, invite_id)
        }
        fn put_project_invite(&self, invite: &ProjectInvite) -> Result<(), PlatformError> {
            self.inner.put_project_invite(invite)
        }
        fn list_project_invites(
            &self,
            owner: &str,
            project: &str,
        ) -> Result<Vec<ProjectInvite>, PlatformError> {
            self.inner.list_project_invites(owner, project)
        }
        fn delete_project_invite(
            &self,
            owner: &str,
            project: &str,
            invite_id: &str,
        ) -> Result<(), PlatformError> {
            self.inner.delete_project_invite(owner, project, invite_id)
        }
        fn get_worker_registry_record(
            &self,
            node_id: &str,
        ) -> Result<Option<WorkerRegistryRecord>, PlatformError> {
            self.inner.get_worker_registry_record(node_id)
        }
        fn put_worker_registry_record(
            &self,
            record: &WorkerRegistryRecord,
        ) -> Result<(), PlatformError> {
            self.inner.put_worker_registry_record(record)
        }
        fn list_worker_registry_records(&self) -> Result<Vec<WorkerRegistryRecord>, PlatformError> {
            self.inner.list_worker_registry_records()
        }
        fn delete_worker_registry_record(&self, node_id: &str) -> Result<(), PlatformError> {
            self.inner.delete_worker_registry_record(node_id)
        }
        fn get_project_runtime_placement(
            &self,
            owner: &str,
            project: &str,
        ) -> Result<Option<ProjectRuntimePlacement>, PlatformError> {
            self.inner.get_project_runtime_placement(owner, project)
        }
        fn put_project_runtime_placement(
            &self,
            placement: &ProjectRuntimePlacement,
        ) -> Result<(), PlatformError> {
            self.inner.put_project_runtime_placement(placement)
        }
        fn list_project_runtime_placements(
            &self,
        ) -> Result<Vec<ProjectRuntimePlacement>, PlatformError> {
            self.inner.list_project_runtime_placements()
        }
        fn delete_project_runtime_placement(
            &self,
            owner: &str,
            project: &str,
        ) -> Result<(), PlatformError> {
            self.inner.delete_project_runtime_placement(owner, project)
        }
        fn list_all_mcp_sessions(&self) -> Result<Vec<McpSession>, PlatformError> {
            self.inner.list_all_mcp_sessions()
        }
        fn put_mcp_session(&self, session: &McpSession) -> Result<(), PlatformError> {
            self.inner.put_mcp_session(session)
        }
        fn delete_mcp_session(&self, token: &str) -> Result<(), PlatformError> {
            self.inner.delete_mcp_session(token)
        }
    }

    fn interleaved(
        inner: Arc<dyn DataAdapter>,
        on_first_read: impl Fn(&dyn DataAdapter) + Send + Sync + 'static,
    ) -> Arc<dyn DataAdapter> {
        Arc::new(InterleavingAdapter {
            inner,
            on_first_read: Box::new(on_first_read),
            fired: std::sync::atomic::AtomicBool::new(false),
        })
    }

    fn seed_token(data: &dyn DataAdapter, token: &JoinToken) {
        // The office row first: `office_join_tokens` has a foreign key to it,
        // which is minting's rule ("the office record comes first") expressed
        // in the schema.
        let now = now_ts();
        data.put_platform_office(&PlatformOffice {
            office_id: token.office_id.clone(),
            office_slug: token.office_id.clone(),
            label: token.office_id.clone(),
            office_kind: "office".to_string(),
            base_url: String::new(),
            status: "planned".to_string(),
            created_at: now,
            updated_at: now,
        })
        .expect("seed office");
        data.put_office_join_token(&PlatformOfficeJoinToken {
            office_id: token.office_id.clone(),
            secret_digest: token.secret_digest(),
            status: JOIN_TOKEN_STATUS_ACTIVE.to_string(),
            note: String::new(),
            created_at: now_ts(),
            last_used_at: 0,
        })
        .expect("seed");
    }

    fn sqlite_at(root: &Path) -> Arc<dyn DataAdapter> {
        crate::platform::adapters::data::build_data_adapter(
            crate::platform::model::DataAdapterKind::Sqlite,
            root,
        )
        .expect("adapter")
    }

    /// FIX 1's regression: a revoke that commits between the read and the write
    /// of `verify_office_token` used to be silently undone, un-revoking the
    /// office on the next heartbeat ten seconds later.
    #[test]
    fn a_revoke_landing_mid_verification_is_not_undone_by_the_use_record() {
        let root = temp_root("lost-update-revoke");
        let inner = sqlite_at(&root);
        let token = mint_for("office-a", &controller());
        seed_token(inner.as_ref(), &token);

        let office_id = token.office_id.clone();
        let data = interleaved(inner.clone(), move |db| {
            // The operator's revoke, committing while a heartbeat is in flight.
            let mut record = db
                .get_office_join_token(&office_id)
                .expect("read")
                .expect("row");
            record.status = JOIN_TOKEN_STATUS_REVOKED.to_string();
            db.put_office_join_token(&record).expect("revoke");
        });
        let service = ClusterJoinTokenService::new(
            data,
            ClusterRole::Master,
            None,
            Some(Arc::new(controller())),
        );

        let err = service
            .verify_office_token(&token.render())
            .expect_err("a token revoked mid-verification must not verify");
        assert_eq!(err.code, "CLUSTER_JOIN_TOKEN_STALE");
        assert_eq!(
            inner
                .get_office_join_token("office-a")
                .expect("read")
                .expect("row")
                .status,
            JOIN_TOKEN_STATUS_REVOKED,
            "recording a use must never write `status` back"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// The same window, for the rotate half: the superseded digest must not be
    /// restored, which would invalidate the token just issued to that office.
    #[test]
    fn a_rotation_landing_mid_verification_is_not_undone_by_the_use_record() {
        let root = temp_root("lost-update-rotate");
        let inner = sqlite_at(&root);
        let key = controller();
        let old = mint_for("office-a", &key);
        let fresh = mint_for("office-a", &key);
        seed_token(inner.as_ref(), &old);

        let replacement = fresh.secret_digest();
        let office_id = old.office_id.clone();
        let data = interleaved(inner.clone(), move |db| {
            let mut record = db
                .get_office_join_token(&office_id)
                .expect("read")
                .expect("row");
            record.secret_digest = replacement.clone();
            db.put_office_join_token(&record).expect("rotate");
        });
        let service =
            ClusterJoinTokenService::new(data, ClusterRole::Master, None, Some(Arc::new(key)));

        let err = service
            .verify_office_token(&old.render())
            .expect_err("the superseded token must not verify");
        assert_eq!(err.code, "CLUSTER_JOIN_TOKEN_STALE");
        assert_eq!(
            inner
                .get_office_join_token("office-a")
                .expect("read")
                .expect("row")
                .secret_digest,
            fresh.secret_digest(),
            "recording a use must never write `secret_digest` back"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// `kinds/office-topology/README.md`, Rejections: "An office without a
    /// `base_url` — an office nothing can reach is not an office."
    #[test]
    fn minting_refuses_an_office_with_no_base_url() {
        let root = temp_root("mint-no-base-url");
        let data = sqlite_at(&root);
        let service = ClusterJoinTokenService::new(
            data.clone(),
            ClusterRole::Master,
            None,
            Some(Arc::new(controller())),
        );

        for base_url in ["", "   ", "///"] {
            let err = service
                .mint(&ClusterJoinTokenMintRequest {
                    office_id: "office-a".to_string(),
                    base_url: base_url.to_string(),
                    ..Default::default()
                })
                .expect_err("an unreachable office must not be minted");
            assert_eq!(err.code, "CLUSTER_OFFICE_BASE_URL_REQUIRED");
        }
        // And nothing was created on the way to the refusal.
        assert!(
            data.get_platform_office("office-a")
                .expect("read")
                .is_none()
        );
        assert!(
            data.get_office_join_token("office-a")
                .expect("read")
                .is_none()
        );

        // With an address, minting works and the address is recorded.
        let minted = service
            .mint(&ClusterJoinTokenMintRequest {
                office_id: "office-a".to_string(),
                base_url: "https://office-a.example.com/".to_string(),
                ..Default::default()
            })
            .expect("mint");
        assert_eq!(minted.office.base_url, "https://office-a.example.com");
        let _ = fs::remove_dir_all(&root);
    }

    /// The ordinary path still records the use, so the fix is not "stop
    /// writing".
    #[test]
    fn an_uncontested_verification_records_the_use() {
        let root = temp_root("touch");
        let data = sqlite_at(&root);
        let key = controller();
        let token = mint_for("office-a", &key);
        seed_token(data.as_ref(), &token);
        let service = ClusterJoinTokenService::new(
            data.clone(),
            ClusterRole::Master,
            None,
            Some(Arc::new(key)),
        );

        let verified = service
            .verify_office_token(&token.render())
            .expect("verify");
        assert!(verified.record.last_used_at > 0);
        assert_eq!(
            data.get_office_join_token("office-a")
                .expect("read")
                .expect("row")
                .last_used_at,
            verified.record.last_used_at
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// FIX 3's direction rule, at the layer that owns it.
    #[test]
    fn each_half_of_the_credential_authenticates_one_direction_only() {
        let root = temp_root("directions");
        let key = Arc::new(controller());
        let data = sqlite_at(&root);
        let token = mint_for("office-a", &key);
        seed_token(data.as_ref(), &token);

        let controller_side = ClusterJoinTokenService::new(
            data.clone(),
            ClusterRole::Master,
            None,
            Some(key.clone()),
        );
        let office_side = ClusterJoinTokenService::new(
            data.clone(),
            ClusterRole::Worker,
            Some(OfficeJoinIdentity::from_token(token.clone())),
            // Handed one on purpose: an office must drop it.
            Some(key.clone()),
        );
        assert!(
            office_side.controller_verify_key().is_none(),
            "an office must not hold a signing key even if one is handed to it"
        );

        let call_header = controller_side
            .controller_call_header_for("office-a")
            .expect("header");

        // The office accepts its controller's call and refuses a join token.
        assert!(office_side.controller_call_authenticates(&call_header));
        assert!(!office_side.controller_call_authenticates(&token.render()));
        // The controller accepts a join token and refuses its own call header
        // — this is the short-circuit that used to make any office's token a
        // project-owner credential on the controller.
        assert!(!controller_side.controller_call_authenticates(&call_header));
        assert!(!controller_side.controller_call_authenticates(&token.render()));
        assert!(controller_side.verify_office_token(&token.render()).is_ok());
        // And an office never verifies a join token at all.
        let err = office_side
            .verify_office_token(&token.render())
            .expect_err("an office is not a controller");
        assert_eq!(err.code, "CLUSTER_JOIN_TOKEN_NOT_A_CONTROLLER");
        let _ = fs::remove_dir_all(&root);
    }
}
