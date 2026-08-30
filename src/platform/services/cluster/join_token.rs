//! Per-office join tokens: minting, verification, revocation, and the office's
//! own stored identity.
//!
//! `offices.md` §8 states the whole agreement: the token is "per-office, issued
//! by the controller, revocable for one office alone", and it carries proof in
//! both directions. Four things follow, and this module owns all four.
//!
//! 1. **Minting creates the office record first.** An operator mints for a
//!    *planned* office, so the controller knows who holds what before anything
//!    is presented. That is GitLab's modern runner flow, and it is what turns
//!    possession into a record rather than into membership.
//! 2. **The secret is stored hashed.** The controller keeps `sha256(secret)`
//!    and never the secret.
//! 3. **Verification is constant-time and per-office.** Flipping one record's
//!    status locks out exactly one office, by construction.
//! 4. **The office verifies the controller.** It sends a nonce and refuses a
//!    response whose HMAC proof does not verify.

use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::infra::cluster::config::ClusterRole;
use crate::infra::cluster::security::join_token::{
    self, JoinToken, JoinTokenError, controller_call_header, digests_match,
    parse_controller_call_header, registration_proof, secret_digest,
};
use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    ClusterJoinTokenMintRequest, ClusterMintedJoinToken, JOIN_TOKEN_STATUS_ACTIVE,
    JOIN_TOKEN_STATUS_REVOKED, PlatformOffice, PlatformOfficeJoinToken, now_ts, slug_segment,
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

/// What an office holds after it has been issued a token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OfficeJoinIdentity {
    /// Office this token names.
    pub office_id: String,
    /// The token as presented to the controller.
    pub token: String,
    /// `sha256(secret)` — the key of every proof exchanged with the controller.
    pub secret_digest: String,
}

impl OfficeJoinIdentity {
    fn from_token(token: JoinToken) -> Self {
        Self {
            office_id: token.office_id.clone(),
            secret_digest: token.secret_digest(),
            token: token.render(),
        }
    }

    /// Whether `proof` proves the responder holds this office's secret.
    pub fn verify_registration_proof(&self, nonce: &str, proof: &str) -> bool {
        let expected = registration_proof(&self.secret_digest, &self.office_id, nonce);
        digests_match(&expected, proof)
    }

    /// Whether a presented controller-call header is really from the controller.
    pub fn verify_controller_header(&self, raw: &str) -> bool {
        let Some((office_id, _)) = parse_controller_call_header(raw) else {
            return false;
        };
        if office_id != self.office_id {
            return false;
        }
        let expected = controller_call_header(&self.secret_digest, &self.office_id);
        digests_match(&expected, raw)
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
}

impl ClusterJoinTokenService {
    /// Build the service for one process.
    pub fn new(
        data: Arc<dyn DataAdapter>,
        role: ClusterRole,
        identity: Option<OfficeJoinIdentity>,
    ) -> Self {
        Self {
            data,
            role,
            identity,
        }
    }

    /// This office's own issued token, when it has one.
    pub fn office_identity(&self) -> Option<&OfficeJoinIdentity> {
        self.identity.as_ref()
    }

    /// Mint a token for one office, creating the office record if needed.
    pub fn mint(
        &self,
        request: &ClusterJoinTokenMintRequest,
    ) -> Result<ClusterMintedJoinToken, PlatformError> {
        let office_id = normalize_office_id(&request.office_id)?;
        let now = now_ts();

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
                if !request.base_url.trim().is_empty() {
                    existing.base_url = request.base_url.trim_end_matches('/').to_string();
                }
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
                base_url: request.base_url.trim_end_matches('/').to_string(),
                // Planned, not online: nothing has registered yet, and saying
                // "online" here would put an unreachable office in the
                // directory as a healthy one.
                status: "planned".to_string(),
                created_at: now,
                updated_at: now,
            },
        };
        self.data.put_platform_office(&office)?;

        let token = JoinToken::mint(office_id.clone());
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
    pub fn verify_office_token(&self, raw: &str) -> Result<VerifiedOffice, PlatformError> {
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
        if !digests_match(&record.secret_digest, &presented.secret_digest()) {
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
        record.last_used_at = now_ts();
        self.data.put_office_join_token(&record)?;
        Ok(VerifiedOffice { office_id, record })
    }

    /// The controller's proof that it holds this office's secret.
    pub fn registration_proof_for(&self, office_id: &str, nonce: &str) -> Option<String> {
        if nonce.trim().is_empty() {
            return None;
        }
        let record = self.data.get_office_join_token(office_id).ok().flatten()?;
        Some(registration_proof(&record.secret_digest, office_id, nonce))
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
        Ok(controller_call_header(&record.secret_digest, &office_id))
    }

    /// Whether a presented internal-cluster header authenticates a peer.
    ///
    /// The two roles check different things, because the two directions carry
    /// different halves of the credential. A standalone instance has no peer,
    /// so nothing authenticates by this route at all.
    pub fn header_authenticates_peer(&self, raw: &str) -> bool {
        match self.role {
            ClusterRole::Master => self.verify_office_token(raw).is_ok(),
            ClusterRole::Worker => self
                .identity
                .as_ref()
                .is_some_and(|identity| identity.verify_controller_header(raw)),
            ClusterRole::Standalone => false,
        }
    }
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
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            PlatformError::new(
                "CLUSTER_JOIN_TOKEN_STORE_WRITE",
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
        PlatformError::new(
            "CLUSTER_JOIN_TOKEN_STORE_WRITE",
            format!("failed creating '{}': {err}", path.display()),
        )
    })?;
    file.write_all(value.as_bytes())
        .and_then(|()| file.write_all(b"\n"))
        .and_then(|()| file.sync_all())
        .map_err(|err| {
            PlatformError::new(
                "CLUSTER_JOIN_TOKEN_STORE_WRITE",
                format!("failed writing '{}': {err}", path.display()),
            )
        })?;
    // An existing file keeps its old mode through `create`, so tighten after
    // the fact as well as at open time.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|err| {
            PlatformError::new(
                "CLUSTER_JOIN_TOKEN_STORE_WRITE",
                format!("failed securing '{}': {err}", path.display()),
            )
        })?;
    }
    Ok(())
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

    #[test]
    fn a_first_join_stores_the_supplied_token_privately() {
        let root = temp_root("first-join");
        let minted = JoinToken::mint("office-a");
        let identity = resolve_office_identity(&root, Some(&minted.render()))
            .expect("first join")
            .expect("identity");
        assert_eq!(identity.office_id, "office-a");
        assert_eq!(identity.secret_digest, minted.secret_digest());

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
        let stored = JoinToken::mint("office-a");
        resolve_office_identity(&root, Some(&stored.render())).expect("first join");

        let other = JoinToken::mint("office-b");
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
        let stored = JoinToken::mint("office-a");
        resolve_office_identity(&root, Some(&stored.render())).expect("first join");
        let again = resolve_office_identity(&root, Some(&stored.render()))
            .expect("same token")
            .expect("identity");
        assert_eq!(again.office_id, "office-a");
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn an_office_refuses_a_response_that_does_not_prove_the_controller() {
        let issued = JoinToken::mint("office-a");
        let identity = OfficeJoinIdentity::from_token(issued.clone());
        let sent = nonce();

        let honest = registration_proof(&issued.secret_digest(), "office-a", &sent);
        assert!(identity.verify_registration_proof(&sent, &honest));

        // Anything that does not hold this office's secret.
        let impostor = JoinToken::mint("office-a");
        assert!(!identity.verify_registration_proof(
            &sent,
            &registration_proof(&impostor.secret_digest(), "office-a", &sent)
        ));
        assert!(!identity.verify_registration_proof(&sent, ""));
        assert!(!identity.verify_registration_proof(&sent, "not-a-proof"));
        // A proof for a different nonce is a replay, not an answer.
        assert!(!identity.verify_registration_proof(
            &sent,
            &registration_proof(&issued.secret_digest(), "office-a", &nonce())
        ));
    }

    #[test]
    fn an_office_accepts_only_its_own_controller_call_header() {
        let issued = JoinToken::mint("office-a");
        let identity = OfficeJoinIdentity::from_token(issued.clone());

        assert!(identity.verify_controller_header(&controller_call_header(
            &issued.secret_digest(),
            "office-a"
        )));
        // A header for another office, or under another secret, is not ours.
        assert!(!identity.verify_controller_header(&controller_call_header(
            &issued.secret_digest(),
            "office-b"
        )));
        assert!(!identity.verify_controller_header(&controller_call_header(
            &JoinToken::mint("office-a").secret_digest(),
            "office-a"
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
}
