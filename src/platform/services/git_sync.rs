//! Git between a project's `repo/` and its remote: fetch, sync, push, and a
//! conflict that is resolved in the Studio instead of "locally".
//!
//! The rules (docs/contracts/project.md, "Git"):
//!
//! - **Commit and push are separate acts.** A commit is local and always
//!   succeeds or fails on its own. Pushing is explicit, or the `push` flag on
//!   a commit, which is commit → sync → push and stops at the first step that
//!   cannot proceed.
//! - **Nothing is lost on a connection error.** Commits that did not reach
//!   the remote are counted (`ahead`) and shown; the next push takes them.
//! - **A conflict is a state, not a dead end.** `sync` rebases the local
//!   commits on the remote branch; when a file conflicts the rebase is left
//!   in progress and the state names the files. Each one is resolved with
//!   `mine`, `theirs`, or content written by hand, then `continue` replays
//!   the rest; `abort` puts everything back. During a rebase git's "ours" is
//!   the remote side and "theirs" the local commit, which this module hides:
//!   `mine` always means what the project had.
//! - **The remote is `origin`, without secrets.** The bare URL is stored in
//!   the repository's config; the token travels as an `http.extraheader` on
//!   the one command that needs it, so it never lands in `.git/config` or a
//!   log line.

use std::path::{Path, PathBuf};
use std::process::Command;

use base64::Engine as _;
use serde::{Deserialize, Serialize};

use crate::platform::error::PlatformError;
use crate::platform::services::platform::PlatformService;

/// Where the project stands against its remote. `git/status` carries it, so
/// does every sync verb's answer.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct SyncState {
    /// The checked-out branch.
    pub branch: String,
    /// Whether a remote URL is configured for the project.
    pub remote_configured: bool,
    /// The remote branch the project syncs with (`main` unless set).
    pub remote_branch: String,
    /// Local commits the remote does not have. Unknown before the first fetch.
    pub ahead: Option<u32>,
    /// Remote commits the project does not have. Unknown before the first fetch.
    pub behind: Option<u32>,
    /// A rebase is in progress: `conflicts` are the files still unmerged.
    pub rebase_in_progress: bool,
    pub conflicts: Vec<String>,
    /// Files the working tree has changed and not committed.
    pub dirty: u32,
}

impl SyncState {
    /// One word for the panel: `clean`, `unpushed`, `behind`, `diverged`,
    /// `conflict`, `unknown` (no fetch yet), `no-remote`.
    pub fn word(&self) -> &'static str {
        if !self.remote_configured {
            return "no-remote";
        }
        if self.rebase_in_progress {
            return "conflict";
        }
        match (self.ahead, self.behind) {
            (Some(0), Some(0)) => "clean",
            (Some(a), Some(0)) if a > 0 => "unpushed",
            (Some(0), Some(b)) if b > 0 => "behind",
            (Some(_), Some(_)) => "diverged",
            _ => "unknown",
        }
    }
}

/// How one conflicted file is settled.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Resolution {
    /// Keep what this project had (git's "theirs" during a rebase).
    Mine,
    /// Take the remote's version (git's "ours" during a rebase).
    Theirs,
    /// Write this content as the resolution — or, with no content, stage the
    /// file as it was saved in the editor.
    Content,
}

pub struct GitSyncService {
    platform: std::sync::Arc<PlatformService>,
}

/// The outcome of a verb that may stop at a conflict.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "outcome", rename_all = "lowercase")]
pub enum SyncOutcome {
    Ok { state: SyncState },
    Conflict { state: SyncState },
}

impl GitSyncService {
    pub fn new(platform: std::sync::Arc<PlatformService>) -> Self {
        Self { platform }
    }

    fn repo_dir(&self, owner: &str, project: &str) -> Result<PathBuf, PlatformError> {
        Ok(self.platform.file.ensure_project_layout(owner, project)?.repo_dir)
    }

    fn remote(&self, owner: &str, project: &str) -> Result<(String, String, String), PlatformError> {
        let cfg = self.platform.zebflow_cfg.read_or_default(owner, project)?;
        let r = &cfg.configs.git.remote;
        let branch = if r.branch.trim().is_empty() { "main".to_string() } else { r.branch.trim().to_string() };
        Ok((r.repo_url.trim().to_string(), r.credential_id.trim().to_string(), branch))
    }

    /// `-c http.extraheader=…` for the one command that talks to the remote;
    /// nothing for a remote that needs no token (a `file://` test remote).
    fn auth_args(&self, owner: &str, project: &str, credential_id: &str, repo_url: &str) -> Vec<String> {
        if credential_id.is_empty() || !repo_url.starts_with("http") {
            return Vec::new();
        }
        let Ok(Some(cred)) = self.platform.credentials.get_project_credential(owner, project, credential_id) else {
            return Vec::new();
        };
        let username = cred.secret["username"].as_str().unwrap_or("");
        let token = cred.secret["token"].as_str().unwrap_or("");
        if token.is_empty() {
            return Vec::new();
        }
        let user = if username.is_empty() { "x-access-token" } else { username };
        let basic = base64::engine::general_purpose::STANDARD.encode(format!("{user}:{token}"));
        vec!["-c".to_string(), format!("http.extraheader=AUTHORIZATION: basic {basic}")]
    }

    fn identity_args(&self, actor: Option<&str>, project: &str) -> Vec<String> {
        let id = self.platform.git_identity.resolve_for_actor(actor, project);
        vec!["-c".into(), format!("user.name={}", id.name), "-c".into(), format!("user.email={}", id.email)]
    }

    /// `origin` carries the bare URL. Set on every remote-facing verb, so a
    /// project configured before this module existed gets it lazily.
    fn ensure_origin(&self, repo: &Path, repo_url: &str) -> Result<(), PlatformError> {
        if repo_url.is_empty() {
            return Err(PlatformError::new("GIT_NO_REMOTE", "No remote configured. Connect a remote in the Git panel first."));
        }
        let current = run(repo, &[], &["remote", "get-url", "origin"]).ok();
        match current {
            Some(url) if url.trim() == repo_url => Ok(()),
            Some(_) => run(repo, &[], &["remote", "set-url", "origin", repo_url]).map(|_| ()),
            None => run(repo, &[], &["remote", "add", "origin", repo_url]).map(|_| ()),
        }
    }

    /// The state without touching the network: counts against the last
    /// fetched `origin/<branch>`, the rebase directory, the working tree.
    pub fn state(&self, owner: &str, project: &str) -> Result<SyncState, PlatformError> {
        let repo = self.repo_dir(owner, project)?;
        let (repo_url, _, remote_branch) = self.remote(owner, project)?;
        let branch = run(&repo, &[], &["rev-parse", "--abbrev-ref", "HEAD"]).map(|s| s.trim().to_string()).unwrap_or_else(|_| "HEAD".into());
        let rebase_in_progress = repo.join(".git/rebase-merge").exists() || repo.join(".git/rebase-apply").exists();
        let conflicts: Vec<String> = run(&repo, &[], &["diff", "--name-only", "--diff-filter=U"])
            .map(|s| s.lines().filter(|l| !l.trim().is_empty()).map(|l| l.trim().to_string()).collect())
            .unwrap_or_default();
        let dirty = run(&repo, &[], &["status", "--porcelain=v1"]).map(|s| s.lines().filter(|l| !l.trim().is_empty()).count() as u32).unwrap_or(0);
        let (ahead, behind) = if repo_url.is_empty() {
            (None, None)
        } else {
            match run(&repo, &[], &["rev-list", "--left-right", "--count", &format!("HEAD...origin/{remote_branch}")]) {
                Ok(s) => {
                    let mut it = s.split_whitespace();
                    (it.next().and_then(|a| a.parse().ok()), it.next().and_then(|b| b.parse().ok()))
                }
                Err(_) => (None, None),
            }
        };
        Ok(SyncState { branch, remote_configured: !repo_url.is_empty(), remote_branch, ahead, behind, rebase_in_progress, conflicts, dirty })
    }

    /// Bring `origin/<branch>` up to date. The only verb that reads the remote
    /// without writing anything local.
    pub fn fetch(&self, owner: &str, project: &str) -> Result<SyncState, PlatformError> {
        let repo = self.repo_dir(owner, project)?;
        let (repo_url, cred, remote_branch) = self.remote(owner, project)?;
        self.ensure_origin(&repo, &repo_url)?;
        let auth = self.auth_args(owner, project, &cred, &repo_url);
        match run(&repo, &auth, &["fetch", "origin", &remote_branch]) {
            Ok(_) => {}
            // An empty remote has no branch yet: nothing to fetch, nothing to
            // rebase on; the first push creates it. Every other failure is real.
            Err(e) if e.message.contains("couldn't find remote ref") => {}
            Err(e) => return Err(e.with_code("GIT_FETCH")),
        }
        self.state(owner, project)
    }

    /// Fetch, then rebase the local commits on the remote branch. A conflict
    /// leaves the rebase in progress and answers `Conflict` with the files.
    pub fn sync(&self, owner: &str, project: &str, actor: Option<&str>) -> Result<SyncOutcome, PlatformError> {
        let repo = self.repo_dir(owner, project)?;
        let before = self.state(owner, project)?;
        if before.rebase_in_progress {
            return Ok(SyncOutcome::Conflict { state: before });
        }
        let fetched = self.fetch(owner, project)?;
        // Nothing to pull (or no remote branch yet): the sync is done.
        if fetched.behind == Some(0) || fetched.behind.is_none() {
            return Ok(SyncOutcome::Ok { state: fetched });
        }
        let identity = self.identity_args(actor, project);
        let (_, _, remote_branch) = self.remote(owner, project)?;
        // A project always has uncommitted files (its scaffold, a draft page);
        // `--autostash` sets them aside for the rebase and brings them back.
        match run_env(&repo, &identity, &["rebase", "--autostash", &format!("origin/{remote_branch}")], &[("GIT_EDITOR", "true")]) {
            Ok(_) => Ok(SyncOutcome::Ok { state: self.state(owner, project)? }),
            Err(err) => {
                let state = self.state(owner, project)?;
                if state.rebase_in_progress && !state.conflicts.is_empty() {
                    Ok(SyncOutcome::Conflict { state })
                } else {
                    let _ = run(&repo, &[], &["rebase", "--abort"]);
                    Err(err.with_code("GIT_REBASE"))
                }
            }
        }
    }

    /// Push the branch. Refuses while a rebase is in progress or the project
    /// is behind, and says so, because a forced push is not a thing here.
    pub fn push(&self, owner: &str, project: &str) -> Result<SyncState, PlatformError> {
        let repo = self.repo_dir(owner, project)?;
        let (repo_url, cred, remote_branch) = self.remote(owner, project)?;
        self.ensure_origin(&repo, &repo_url)?;
        let state = self.state(owner, project)?;
        if state.rebase_in_progress {
            return Err(PlatformError::new("GIT_CONFLICT", "a rebase is in progress — resolve the conflicts and continue, or abort"));
        }
        let auth = self.auth_args(owner, project, &cred, &repo_url);
        match run(&repo, &auth, &["push", "--set-upstream", "origin", &format!("HEAD:{remote_branch}")]) {
            Ok(_) => self.fetch(owner, project),
            Err(err) => {
                let msg = err.message.to_lowercase();
                if msg.contains("rejected") || msg.contains("non-fast-forward") || msg.contains("fetch first") {
                    Err(PlatformError::new("GIT_BEHIND", "the remote has commits this project does not — sync first, then push"))
                } else {
                    Err(err.with_code("GIT_PUSH"))
                }
            }
        }
    }

    /// Settle one conflicted file and stage it.
    pub fn resolve(&self, owner: &str, project: &str, rel_path: &str, how: Resolution, content: Option<&str>) -> Result<SyncState, PlatformError> {
        let repo = self.repo_dir(owner, project)?;
        let state = self.state(owner, project)?;
        let rel = rel_path.trim().trim_start_matches('/');
        if !state.rebase_in_progress {
            return Err(PlatformError::new("GIT_NO_CONFLICT", "no rebase in progress"));
        }
        if !state.conflicts.iter().any(|c| c == rel) {
            return Err(PlatformError::new("GIT_NO_CONFLICT", format!("'{rel}' is not a conflicted file")));
        }
        match how {
            // During a rebase, git's "theirs" is the commit being replayed — the project's own.
            Resolution::Mine => { run(&repo, &[], &["checkout", "--theirs", "--", rel])?; }
            Resolution::Theirs => { run(&repo, &[], &["checkout", "--ours", "--", rel])?; }
            Resolution::Content => {
                let abs = repo.join(rel);
                if !abs.starts_with(&repo) {
                    return Err(PlatformError::new("GIT_RESOLVE", "path leaves the repository"));
                }
                if let Some(text) = content {
                    std::fs::write(&abs, text)?;
                }
                // A file still carrying markers is not resolved, whoever wrote it.
                let saved = std::fs::read_to_string(&abs).unwrap_or_default();
                if saved.lines().any(|l| l.starts_with("<<<<<<< ") || l.starts_with(">>>>>>> ") || l == "=======") {
                    return Err(PlatformError::new("GIT_RESOLVE", format!("'{rel}' still has conflict markers — finish editing it first")));
                }
            }
        }
        run(&repo, &[], &["add", "--", rel])?;
        self.state(owner, project)
    }

    /// Replay the rest after every conflict is staged. Stops at the next one.
    pub fn continue_rebase(&self, owner: &str, project: &str, actor: Option<&str>) -> Result<SyncOutcome, PlatformError> {
        let repo = self.repo_dir(owner, project)?;
        let state = self.state(owner, project)?;
        if !state.rebase_in_progress {
            return Err(PlatformError::new("GIT_NO_CONFLICT", "no rebase in progress"));
        }
        if !state.conflicts.is_empty() {
            return Err(PlatformError::new("GIT_CONFLICT", format!("{} file(s) still conflicted: {}", state.conflicts.len(), state.conflicts.join(", "))));
        }
        let identity = self.identity_args(actor, project);
        match run_env(&repo, &identity, &["rebase", "--continue"], &[("GIT_EDITOR", "true")]) {
            Ok(_) => Ok(SyncOutcome::Ok { state: self.state(owner, project)? }),
            Err(err) => {
                let state = self.state(owner, project)?;
                if state.rebase_in_progress && !state.conflicts.is_empty() {
                    Ok(SyncOutcome::Conflict { state })
                } else {
                    Err(err.with_code("GIT_REBASE"))
                }
            }
        }
    }

    /// Put everything back as it was before `sync`.
    pub fn abort(&self, owner: &str, project: &str) -> Result<SyncState, PlatformError> {
        let repo = self.repo_dir(owner, project)?;
        if self.state(owner, project)?.rebase_in_progress {
            run(&repo, &[], &["rebase", "--abort"])?;
        }
        self.state(owner, project)
    }
}

fn run(repo: &Path, pre: &[String], args: &[&str]) -> Result<String, PlatformError> {
    run_env(repo, pre, args, &[])
}

fn run_env(repo: &Path, pre: &[String], args: &[&str], env: &[(&str, &str)]) -> Result<String, PlatformError> {
    let mut cmd = Command::new("git");
    for p in pre {
        cmd.arg(p);
    }
    cmd.arg("-C").arg(repo);
    for a in args {
        cmd.arg(a);
    }
    for (k, v) in env {
        cmd.env(k, v);
    }
    let out = cmd.output().map_err(|e| PlatformError::new("GIT_EXEC", e.to_string()))?;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    if out.status.success() {
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let msg = if stderr.is_empty() { stdout.trim().to_string() } else { stderr };
        Err(PlatformError::new("GIT_FAILED", scrub_secrets(&msg)))
    }
}

/// A token never reaches a log line or a panel, whichever command leaked it.
fn scrub_secrets(text: &str) -> String {
    let re = regex::Regex::new(r"(?i)(authorization: basic )[A-Za-z0-9+/=]+|://[^/\s:]+:[^@\s]+@").unwrap();
    re.replace_all(text, |c: &regex::Captures| {
        if c.get(1).is_some() { format!("{}[redacted]", &c[1]) } else { "://[redacted]@".to_string() }
    })
    .to_string()
}

trait WithCode {
    fn with_code(self, code: &'static str) -> Self;
}
impl WithCode for PlatformError {
    fn with_code(mut self, code: &'static str) -> Self {
        if self.code == "GIT_FAILED" {
            self.code = code;
        }
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_state_word_reads_the_counts() {
        let mut s = SyncState { remote_configured: true, ahead: Some(0), behind: Some(0), ..Default::default() };
        assert_eq!(s.word(), "clean");
        s.ahead = Some(2);
        assert_eq!(s.word(), "unpushed");
        s.behind = Some(1);
        assert_eq!(s.word(), "diverged");
        s.ahead = Some(0);
        assert_eq!(s.word(), "behind");
        s.rebase_in_progress = true;
        assert_eq!(s.word(), "conflict");
        s.remote_configured = false;
        assert_eq!(s.word(), "no-remote");
        let fresh = SyncState { remote_configured: true, ..Default::default() };
        assert_eq!(fresh.word(), "unknown");
    }

    #[test]
    fn secrets_never_survive_an_error_message() {
        assert_eq!(scrub_secrets("fatal: https://user:ghp_abc@github.com/x/y failed"), "fatal: https://[redacted]@github.com/x/y failed");
        assert_eq!(scrub_secrets("header AUTHORIZATION: basic dXNlcjp0b2tlbg== rejected"), "header AUTHORIZATION: basic [redacted] rejected");
    }
}
