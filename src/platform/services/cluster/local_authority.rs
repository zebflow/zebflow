//! Whether this office's own accounts may open its front door.
//!
//! `offices.md` §4 lists exactly one thing an office gives up when it joins:
//! "Local accounts are disabled for login while joined; the controller's
//! identity is the normal door." §5 adds that they are *disabled, not deleted*.
//! §6 gives the way back — a command on the office's host re-enables local
//! authority, with no quorum and no controller. §7 makes leaving a separate
//! act that restores everything.
//!
//! Those three sentences are one rule, and this module is where it is stated:
//!
//! ```text
//! local login is refused  <=>  this data root holds an office join token
//!                              AND no break-glass row matches that token
//! ```
//!
//! ## Where the state lives, and why a restart cannot forget it
//!
//! Both halves are on disk, in the office's own data root, and neither is a
//! process argument.
//!
//! **Joined** is `platform/office-join-token` existing. That file is already
//! the office's membership — it is written on the first join and read on every
//! later start ([`super::join_token::resolve_office_identity`]) — so the login
//! gate reads the same thing membership itself reads. It deliberately does
//! *not* read [`crate::infra::cluster::config::ClusterRole`], which comes from
//! which server-mode word was typed: a joined office restarted as plain `zeb`
//! would otherwise forget it was joined and open local login again, which is
//! precisely the silent forgetting §4 must not permit. Starting in a different
//! mode is not leaving; §7 says what leaving is.
//!
//! **Broken out** is a row in `office_local_authority`, and it names the
//! *fingerprint* of the token that was in force when the break-glass happened.
//! That is what stops the state forgetting in the other direction. A flag that
//! merely said "local authority is on" would survive a later re-join and leave
//! a joined office with local login open and nobody aware of it. Matching on
//! the fingerprint means a re-issued token — a different secret, a different
//! fingerprint — does not match the old row, so the new join is disabled again
//! by construction rather than by somebody remembering to clear something.
//!
//! Both are read on every attempt rather than cached at boot, so nothing
//! depends on restart ordering.
//!
//! ## Fail closed
//!
//! A token file that exists but cannot be read or parsed counts as joined. An
//! office whose membership record got truncated must not fall *open*; the
//! refusal names the file, and §6's command is on the same host as the file.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use rand::RngExt as _;

use crate::infra::cluster::security::{JoinToken, token_fingerprint};
use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    LOCAL_AUTHORITY_EVENT_BREAK_GLASS, LOCAL_AUTHORITY_EVENT_DETACH,
    PlatformOfficeLocalAuthorityEvent, now_ts,
};

use super::join_token::office_token_path;

/// Error code a refused local login carries.
pub const LOCAL_LOGIN_DISABLED_CODE: &str = "OFFICE_LOCAL_LOGIN_DISABLED";

/// What this data root's join-token file says about membership.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OfficeJoinFile {
    /// No token file. This instance has joined nobody.
    Absent,
    /// A usable token, naming its office.
    Held {
        /// Office this instance is joined as.
        office_id: String,
        /// `sha256(sha256(secret))` — see [`PlatformOfficeLocalAuthorityEvent`].
        fingerprint: String,
    },
    /// A token file that exists and cannot be used.
    ///
    /// Treated as joined everywhere it matters. The alternative is an office
    /// that opens local login because its membership record was damaged.
    Unreadable {
        /// Why it could not be read.
        reason: String,
    },
}

impl OfficeJoinFile {
    /// Whether this instance is an office at all.
    pub fn is_joined(&self) -> bool {
        !matches!(self, Self::Absent)
    }

    /// The office id when one is legible.
    pub fn office_id(&self) -> &str {
        match self {
            Self::Held { office_id, .. } => office_id,
            _ => "",
        }
    }

    /// The token fingerprint when one is legible.
    pub fn fingerprint(&self) -> &str {
        match self {
            Self::Held { fingerprint, .. } => fingerprint,
            _ => "",
        }
    }
}

/// Reads and writes one office's local-authority state.
///
/// Deliberately separate from [`super::ClusterJoinTokenService`], which is the
/// *controller relationship*: minting, verification, vouches. This is the
/// office's statement about its own accounts, and §6 exists precisely so that
/// statement never needs the controller.
#[derive(Clone)]
pub struct OfficeLocalAuthorityService {
    data: Arc<dyn DataAdapter>,
    data_root: PathBuf,
}

impl OfficeLocalAuthorityService {
    /// Build the service for one data root.
    pub fn new(data: Arc<dyn DataAdapter>, data_root: PathBuf) -> Self {
        Self { data, data_root }
    }

    /// What the token file says right now.
    pub fn join_file(&self) -> OfficeJoinFile {
        read_join_file(&self.data_root)
    }

    /// The break-glass row in force against the token held right now, if any.
    ///
    /// "In force" is fingerprint equality and nothing else. A row from an
    /// earlier membership is history, not permission.
    pub fn break_glass_in_force(
        &self,
    ) -> Result<Option<PlatformOfficeLocalAuthorityEvent>, PlatformError> {
        let join = self.join_file();
        let fingerprint = join.fingerprint();
        if fingerprint.is_empty() {
            // An unreadable token file has no fingerprint to match, so nothing
            // is in force against it and the office stays closed. That is the
            // fail-closed half stated once, here.
            return Ok(None);
        }
        // Asked as a question, never as a scan of a window. It used to read the
        // newest 200 rows and look for a match, so an office past
        // that many local-authority events silently re-locked its own front
        // door: the break-glass row scrolled out of the window, nothing had
        // been rotated, nobody had acted, and §6's "never locked out of itself"
        // stopped being true. The paginated read below is still the display
        // API; it is only the *decision* that must not be windowed.
        self.data.find_office_break_glass(fingerprint)
    }

    /// Whether a local password may open a session on this instance.
    pub fn local_login_allowed(&self) -> Result<bool, PlatformError> {
        if !self.join_file().is_joined() {
            return Ok(true);
        }
        Ok(self.break_glass_in_force()?.is_some())
    }

    /// The refusal a local login meets while this office is joined.
    ///
    /// It says three things and no fourth: that this instance is joined, that
    /// the controller is the way in, and that host access re-enables local
    /// authority. It says nothing whatever about the identifier or the
    /// password — the gate runs before either is looked at, so the same words
    /// come back for a real account, a wrong password, and a name that has
    /// never existed here.
    pub fn refusal(&self) -> PlatformError {
        let join = self.join_file();
        let office = match &join {
            OfficeJoinFile::Held { office_id, .. } => format!(" as office '{office_id}'"),
            _ => String::new(),
        };
        let damaged = match &join {
            OfficeJoinFile::Unreadable { reason } => format!(
                " This office's token file at '{}' is unreadable ({reason}); it is treated as \
                 joined rather than opened.",
                office_token_path(&self.data_root).display()
            ),
            _ => String::new(),
        };
        PlatformError::new(
            LOCAL_LOGIN_DISABLED_CODE,
            format!(
                "local password login is disabled on this instance. It has joined a controller\
                 {office}, and while it is joined the controller's identity is the way in: open \
                 this office from the controller's directory, or ask it for a vouch.{damaged} \
                 Local authority can be re-enabled from this host, with no controller and no \
                 quorum, by stopping this server and running `zeb admin break-glass`; \
                 `zeb admin detach` leaves the controller for good and keeps every project, its \
                 data, its files, and its shelf."
            ),
        )
    }

    /// `offices.md` §6 — re-enable local authority on this office.
    ///
    /// `owner` is the local account whose password was rotated in the same act,
    /// or empty when none was: the record says who was let in, not only that
    /// somebody was.
    ///
    /// Deliberately **not** a detach. §6 says break-glass "re-enables local
    /// authority" and §7 makes leaving a separate act; the only reading that
    /// keeps both sentences true is that break-glass changes one thing — who
    /// may open the door — and leaves membership alone. An office repaired at
    /// 3am is still the controller's office in the morning, still placed, still
    /// vouched for, and still reachable from the directory. Making break-glass
    /// detach would mean every emergency login silently dissolved a
    /// relationship, which is the accident §7 opens by ruling out.
    pub fn break_glass(
        &self,
        owner: &str,
        detail: &str,
    ) -> Result<PlatformOfficeLocalAuthorityEvent, PlatformError> {
        let join = self.join_file();
        if !join.is_joined() {
            return Err(PlatformError::new(
                "OFFICE_NOT_JOINED",
                format!(
                    "this instance holds no join token at '{}', so its local authority was never \
                     disabled and there is nothing to break out of. Local login already works.",
                    office_token_path(&self.data_root).display()
                ),
            ));
        }
        let OfficeJoinFile::Held {
            office_id,
            fingerprint,
        } = join
        else {
            return Err(PlatformError::new(
                "OFFICE_JOIN_TOKEN_UNREADABLE",
                format!(
                    "the join token at '{}' cannot be read, so this office cannot say which \
                     membership is being broken out of. Repair or remove the file — \
                     `zeb admin detach` removes it and leaves the controller.",
                    office_token_path(&self.data_root).display()
                ),
            ));
        };
        let entry = PlatformOfficeLocalAuthorityEvent {
            event_id: random_event_id(),
            office_id,
            event: LOCAL_AUTHORITY_EVENT_BREAK_GLASS.to_string(),
            join_fingerprint: fingerprint,
            owner: owner.trim().to_string(),
            detail: detail.trim().to_string(),
            acted_at: now_ts(),
            reported_at: 0,
        };
        self.data.put_office_local_authority_event(&entry)?;
        Ok(entry)
    }

    /// `offices.md` §7 — leave the controller.
    ///
    /// Removing the token file is the whole act, and that is the point: nothing
    /// else is touched, because nothing else was ever the controller's.
    /// Projects, `data/`, `files/`, the blessed shelf, and the public surface
    /// are the office's own under §4 whether it is joined or not, so a detach
    /// that deleted or moved any of them would be inventing a cost the contract
    /// says does not exist. Local accounts come back live because they were
    /// disabled, never deleted (§5), and the gate above stops applying the
    /// moment the file is gone.
    ///
    /// The detach row is appended for the record. It is never reported: a
    /// detach gives up the credential that would authenticate the report, and
    /// the controller learns the same fact from the office ceasing to
    /// heartbeat.
    pub fn detach(&self, detail: &str) -> Result<PlatformOfficeLocalAuthorityEvent, PlatformError> {
        let join = self.join_file();
        if !join.is_joined() {
            return Err(PlatformError::new(
                "OFFICE_NOT_JOINED",
                format!(
                    "this instance holds no join token at '{}'. It has joined nobody, so there \
                     is nothing to leave.",
                    office_token_path(&self.data_root).display()
                ),
            ));
        }
        let entry = PlatformOfficeLocalAuthorityEvent {
            event_id: random_event_id(),
            office_id: join.office_id().to_string(),
            event: LOCAL_AUTHORITY_EVENT_DETACH.to_string(),
            join_fingerprint: join.fingerprint().to_string(),
            owner: String::new(),
            detail: detail.trim().to_string(),
            acted_at: now_ts(),
            reported_at: 0,
        };
        // Recorded before the token goes, so a failure between the two leaves a
        // row describing an act that did not finish rather than an act that
        // finished with no row.
        self.data.put_office_local_authority_event(&entry)?;
        let path = office_token_path(&self.data_root);
        std::fs::remove_file(&path).map_err(|err| {
            PlatformError::new(
                "OFFICE_DETACH_FAILED",
                format!("failed removing '{}': {err}", path.display()),
            )
        })?;
        Ok(entry)
    }

    /// This office's local-authority log, newest first.
    pub fn list(
        &self,
        limit: usize,
    ) -> Result<Vec<PlatformOfficeLocalAuthorityEvent>, PlatformError> {
        self.data
            .list_office_local_authority_events(limit.clamp(1, 500))
    }

    /// Break-glass rows this office has not yet had acknowledged.
    ///
    /// Detach rows are excluded by [`PlatformOfficeLocalAuthorityEvent::is_break_glass`]
    /// rather than by a second flag, because §6 is the sentence that asks for a
    /// report and §7 is not.
    pub fn unreported_break_glass(
        &self,
    ) -> Result<Vec<PlatformOfficeLocalAuthorityEvent>, PlatformError> {
        // Same rule as `break_glass_in_force`: what to report is a decision, so
        // it is a query. Windowed, an office with a long local-authority
        // history would have dropped its oldest unreported acts on the floor —
        // and §6 says the record is reported "on reconnect", with no clause
        // about how much else happened in between.
        self.data.list_unreported_office_break_glass()
    }

    /// Record that the controller acknowledged one break-glass.
    pub fn mark_reported(&self, event_id: &str, reported_at: i64) -> Result<(), PlatformError> {
        self.data
            .mark_office_local_authority_reported(event_id, reported_at)
    }

    /// Store a copy of an office's report. Controller side.
    ///
    /// The row is written with the office's own `event_id`, so re-sends
    /// converge on one row rather than accumulating duplicates, and it is
    /// stamped `reported_at` at arrival because from the controller's side that
    /// is when it learned.
    pub fn accept_report(
        &self,
        entry: &PlatformOfficeLocalAuthorityEvent,
        office_id: &str,
        received_at: i64,
    ) -> Result<PlatformOfficeLocalAuthorityEvent, PlatformError> {
        let stored = PlatformOfficeLocalAuthorityEvent {
            event_id: entry.event_id.trim().to_string(),
            // The office the *token* named, never the one the body claimed:
            // the body is the reporting office's own text and the office id is
            // the one thing on it that must not be self-asserted.
            office_id: office_id.to_string(),
            event: LOCAL_AUTHORITY_EVENT_BREAK_GLASS.to_string(),
            join_fingerprint: entry.join_fingerprint.trim().to_string(),
            owner: entry.owner.trim().to_string(),
            detail: entry.detail.trim().to_string(),
            acted_at: entry.acted_at,
            reported_at: received_at,
        };
        if stored.event_id.is_empty() {
            return Err(PlatformError::new(
                "OFFICE_BREAK_GLASS_REPORT_INVALID",
                "a reported break-glass must carry the event id the office recorded",
            ));
        }
        self.data.put_office_local_authority_event(&stored)?;
        Ok(stored)
    }
}

/// `sha256(secret_digest)` — which membership a row is about.
///
/// The same value every controller proof is bound to
/// ([`crate::infra::cluster::security::token_fingerprint`]), stated once there
/// and re-exported here so the login gate and the proof scheme can never drift
/// into two different notions of "which token".
pub fn join_fingerprint(secret_digest: &str) -> String {
    token_fingerprint(secret_digest)
}

/// Read this data root's join-token file without failing open.
pub fn read_join_file(data_root: &Path) -> OfficeJoinFile {
    let path = office_token_path(data_root);
    let raw = match std::fs::read_to_string(&path) {
        Ok(value) => value,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return OfficeJoinFile::Absent,
        Err(err) => {
            return OfficeJoinFile::Unreadable {
                reason: err.to_string(),
            };
        }
    };
    if raw.trim().is_empty() {
        // An empty file is not membership. It is what a truncated write leaves
        // behind, and `resolve_office_identity` already reads it as "no token",
        // so the two agree.
        return OfficeJoinFile::Absent;
    }
    match JoinToken::parse(&raw) {
        Ok(token) => OfficeJoinFile::Held {
            office_id: token.office_id.clone(),
            fingerprint: join_fingerprint(&token.secret_digest()),
        },
        Err(err) => OfficeJoinFile::Unreadable {
            reason: err.to_string(),
        },
    }
}

fn random_event_id() -> String {
    let mut bytes = [0u8; 16];
    rand::rng().fill(&mut bytes);
    hex::encode(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::adapters::data::build_data_adapter;
    use crate::platform::model::DataAdapterKind;

    fn temp_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "zebflow-local-authority-{name}-{}-{}",
            std::process::id(),
            now_ts()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("temp root");
        root
    }

    fn service(root: &Path) -> OfficeLocalAuthorityService {
        let data = build_data_adapter(DataAdapterKind::Sqlite, root).expect("adapter");
        OfficeLocalAuthorityService::new(data, root.to_path_buf())
    }

    /// A token as a controller would mint it, with a real verification key.
    ///
    /// The key is not used here — this module never verifies a controller —
    /// but a token without one is not a token, so the fixture mints one.
    fn minted_token(office_id: &str) -> JoinToken {
        let (_, key) =
            crate::infra::cluster::security::ControllerSigningKey::generate().expect("key");
        JoinToken::mint(office_id, key.verify_key())
    }

    fn write_token(root: &Path, token: &JoinToken) {
        let path = office_token_path(root);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("parent");
        std::fs::write(&path, format!("{}\n", token.render())).expect("write token");
    }

    #[test]
    fn a_standalone_instance_allows_local_login_and_has_nothing_to_break_out_of() {
        let root = temp_root("standalone");
        let svc = service(&root);
        assert_eq!(svc.join_file(), OfficeJoinFile::Absent);
        assert!(svc.local_login_allowed().expect("allowed"));
        let err = svc
            .break_glass("", "t")
            .expect_err("nothing to break out of");
        assert_eq!(err.code, "OFFICE_NOT_JOINED");
        let err = svc.detach("t").expect_err("nothing to leave");
        assert_eq!(err.code, "OFFICE_NOT_JOINED");
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_joined_office_refuses_local_login_until_break_glass_and_the_refusal_is_actionable() {
        let root = temp_root("joined");
        let token = minted_token("office-a");
        write_token(&root, &token);
        let svc = service(&root);

        assert!(!svc.local_login_allowed().expect("disabled while joined"));
        let refusal = svc.refusal();
        assert_eq!(refusal.code, LOCAL_LOGIN_DISABLED_CODE);
        // The three things §4 and §6 require an operator to be told.
        assert!(
            refusal.message.contains("joined a controller"),
            "{refusal:?}"
        );
        assert!(refusal.message.contains("office-a"), "{refusal:?}");
        assert!(refusal.message.contains("vouch"), "{refusal:?}");
        assert!(
            refusal.message.contains("zeb admin break-glass"),
            "{refusal:?}"
        );
        // And nothing about whether any account or password was otherwise good.
        for leak in ["password is", "no such", "unknown user", "incorrect"] {
            assert!(!refusal.message.contains(leak), "{leak} in {refusal:?}");
        }

        svc.break_glass("", "test").expect("break glass");
        assert!(
            svc.local_login_allowed()
                .expect("allowed after break glass")
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn break_glass_does_not_detach_and_detach_is_a_separate_act() {
        let root = temp_root("break-glass-keeps-membership");
        let token = minted_token("office-a");
        write_token(&root, &token);
        let svc = service(&root);

        svc.break_glass("", "test").expect("break glass");
        // §6 re-enables local authority. §7 is what leaves. The token is still
        // here, so this office is still its controller's office.
        assert!(office_token_path(&root).is_file());
        assert!(svc.join_file().is_joined());

        svc.detach("test").expect("detach");
        assert!(!office_token_path(&root).is_file());
        assert!(!svc.join_file().is_joined());
        assert!(svc.local_login_allowed().expect("local login live"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_break_glass_does_not_survive_into_a_later_join() {
        let root = temp_root("refingerprint");
        write_token(&root, &minted_token("office-a"));
        let svc = service(&root);
        svc.break_glass("", "first").expect("break glass");
        assert!(svc.local_login_allowed().expect("allowed"));

        // Re-issued: same office, new secret. The old row is history and does
        // not authorise the new membership.
        write_token(&root, &minted_token("office-a"));
        assert!(
            !svc.local_login_allowed().expect("disabled again"),
            "a re-issued token must not inherit an earlier break-glass"
        );
        assert!(svc.break_glass_in_force().expect("query").is_none());
        // The record of the earlier act is still there. Nothing is erased.
        assert_eq!(svc.list(50).expect("log").len(), 1);
        let _ = std::fs::remove_dir_all(root);
    }

    /// FIX 2's regression. The gate used to read the newest 200
    /// local-authority rows and scan them for a matching fingerprint, so an
    /// office that accumulated more than that after breaking the glass locked
    /// its own front door again — no rotation, no act by anybody, and no way
    /// in but to break the glass a second time. §6 says an office "is never
    /// locked out of itself".
    #[test]
    fn a_break_glass_stays_in_force_however_long_the_log_grows() {
        let root = temp_root("deep-log");
        let token = minted_token("office-a");
        write_token(&root, &token);
        let svc = service(&root);

        svc.break_glass("", "the act that opened the door")
            .expect("break glass");
        assert!(svc.local_login_allowed().expect("open"));

        // Everything that happens afterwards, at more than double the window
        // the old scan could see. Detach rows against other fingerprints are
        // ordinary history — an office may be re-issued and re-joined many
        // times over its life.
        let data = svc.data.clone();
        for index in 0..500 {
            data.put_office_local_authority_event(&PlatformOfficeLocalAuthorityEvent {
                event_id: format!("later-{index}"),
                office_id: "office-a".to_string(),
                event: LOCAL_AUTHORITY_EVENT_DETACH.to_string(),
                join_fingerprint: format!("other-fingerprint-{index}"),
                owner: String::new(),
                detail: "history".to_string(),
                acted_at: now_ts() + 1 + index,
                reported_at: 0,
            })
            .expect("later row");
        }

        assert!(
            svc.break_glass_in_force().expect("query").is_some(),
            "the row that opened the door must not scroll out of a window"
        );
        assert!(
            svc.local_login_allowed().expect("still open"),
            "an office must not re-lock itself because its log got long"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    /// The same rule for the other decision this module makes: what to report.
    #[test]
    fn an_unreported_break_glass_survives_a_long_log() {
        let root = temp_root("deep-log-reporting");
        write_token(&root, &minted_token("office-a"));
        let svc = service(&root);
        let event = svc
            .break_glass("", "the act to report")
            .expect("break glass");

        let data = svc.data.clone();
        for index in 0..500 {
            data.put_office_local_authority_event(&PlatformOfficeLocalAuthorityEvent {
                event_id: format!("later-{index}"),
                office_id: "office-a".to_string(),
                event: LOCAL_AUTHORITY_EVENT_DETACH.to_string(),
                join_fingerprint: format!("other-fingerprint-{index}"),
                owner: String::new(),
                detail: "history".to_string(),
                acted_at: now_ts() + 1 + index,
                reported_at: 0,
            })
            .expect("later row");
        }

        let unreported = svc.unreported_break_glass().expect("unreported");
        assert_eq!(
            unreported.len(),
            1,
            "an office offline for a long time still reports on its first reconnect"
        );
        assert_eq!(unreported[0].event_id, event.event_id);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn a_damaged_token_file_fails_closed() {
        let root = temp_root("damaged");
        let path = office_token_path(&root);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("parent");
        std::fs::write(&path, "not-a-token\n").expect("write");
        let svc = service(&root);
        assert!(matches!(svc.join_file(), OfficeJoinFile::Unreadable { .. }));
        assert!(
            !svc.local_login_allowed().expect("closed"),
            "a damaged membership record must not open local login"
        );
        assert!(svc.refusal().message.contains("unreadable"));
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn only_a_break_glass_is_ever_reported() {
        let root = temp_root("reporting");
        write_token(&root, &minted_token("office-a"));
        let svc = service(&root);
        let event = svc.break_glass("", "test").expect("break glass");
        assert_eq!(svc.unreported_break_glass().expect("unreported").len(), 1);

        svc.mark_reported(&event.event_id, 4242).expect("mark");
        assert!(svc.unreported_break_glass().expect("unreported").is_empty());

        svc.detach("test").expect("detach");
        assert!(
            svc.unreported_break_glass().expect("unreported").is_empty(),
            "a detach gives up the credential that would report it"
        );
        let _ = std::fs::remove_dir_all(root);
    }
}
