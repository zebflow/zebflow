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
//! - **Every git runs hardened** ([`git_command`]). A repository's own
//!   config was written by whoever wrote the repository, so the settings
//!   that make git start a program are pinned on the command line, where it
//!   cannot override them; a ref from a request is never read as an option
//!   (`--end-of-options`); and a `file://` remote never reaches into the data
//!   root, where every other project's repository lives.

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

    /// `GIT_CONFIG_*` environment variables for the one command that talks to
    /// the remote; nothing for a remote that needs no credential (e.g. a
    /// `file://` test remote). Passing the authorization header via environment
    /// variables instead of `-c http.extraheader=…` keeps the credential out of
    /// the child process's `argv` (`/proc/<pid>/cmdline`).
    fn auth_env(&self, owner: &str, project: &str, credential_id: &str, repo_url: &str) -> Vec<(&'static str, String)> {
        if credential_id.is_empty() || !repo_url.starts_with("http") {
            return Vec::new();
        }
        let Ok(Some(cred)) = self.platform.credentials.get_project_credential(owner, project, credential_id) else {
            return Vec::new();
        };
        if !git_credential_matches_remote(&cred.kind, &cred.secret, repo_url) {
            return Vec::new();
        }
        let username = cred.secret["username"].as_str().unwrap_or("");
        let token = cred.secret["token"].as_str().unwrap_or("");
        let Some(header) = git_http_basic_auth_header(username, token) else {
            return Vec::new();
        };
        vec![
            ("GIT_CONFIG_COUNT", "1".to_string()),
            ("GIT_CONFIG_KEY_0", "http.extraheader".to_string()),
            ("GIT_CONFIG_VALUE_0", header),
        ]
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
        refuse_remote_in_data_root(repo_url, &self.platform.config.data_root)?;
        let current = run(repo, &[], &["remote", "get-url", "origin"]).ok();
        match current {
            Some(url) if url.trim() == repo_url => Ok(()),
            Some(_) => run(repo, &[], &["remote", "set-url", "--end-of-options", "origin", repo_url]).map(|_| ()),
            None => run(repo, &[], &["remote", "add", "--end-of-options", "origin", repo_url]).map(|_| ()),
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
            match run(&repo, &[], &["rev-list", "--left-right", "--count", "--end-of-options", &format!("HEAD...origin/{remote_branch}")]) {
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
        let auth = self.auth_env(owner, project, &cred, &repo_url);
        let auth_refs: Vec<(&str, &str)> = auth.iter().map(|(k, v)| (*k, v.as_str())).collect();
        match run_env(&repo, &[], &["fetch", "--end-of-options", "origin", &remote_branch], &auth_refs) {
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
        match run_env(&repo, &identity, &["rebase", "--autostash", "--end-of-options", &format!("origin/{remote_branch}")], &[("GIT_EDITOR", "true")]) {
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
        let auth = self.auth_env(owner, project, &cred, &repo_url);
        let auth_refs: Vec<(&str, &str)> = auth.iter().map(|(k, v)| (*k, v.as_str())).collect();
        match run_env(&repo, &[], &["push", "--set-upstream", "--end-of-options", "origin", &format!("HEAD:{remote_branch}")], &auth_refs) {
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
        let (rel, abs) = self.platform.projects.validate_git_resolve_path(owner, project, rel_path)?;
        let repo = self.repo_dir(owner, project)?;
        let state = self.state(owner, project)?;
        if !state.rebase_in_progress {
            return Err(PlatformError::new("GIT_NO_CONFLICT", "no rebase in progress"));
        }
        if !state.conflicts.iter().any(|c| c == &rel) {
            return Err(PlatformError::new("GIT_NO_CONFLICT", format!("'{rel}' is not a conflicted file")));
        }
        match how {
            // During a rebase, git's "theirs" is the commit being replayed — the project's own.
            Resolution::Mine => { run(&repo, &[], &["checkout", "--theirs", "--", &rel])?; }
            Resolution::Theirs => { run(&repo, &[], &["checkout", "--ours", "--", &rel])?; }
            Resolution::Content => {
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
        run(&repo, &[], &["add", "--", &rel])?;
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

/// The settings pinned on every git this platform runs. Command-line `-c`
/// outranks every config file, so a `.git/config` planted by a clone, a
/// bundle or an import cannot name a hook, a file-system monitor or an
/// `ext::` transport to run. `core.symlinks=false` makes a clone, a sync or
/// a checkout write a committed symlink as a plain file holding its target:
/// a project's files are served without a login, and a link would serve
/// whatever it points at.
const HARDENED_CONFIG: &[&str] = &[
    "core.hooksPath=/dev/null",
    "core.fsmonitor=false",
    "core.symlinks=false",
    "protocol.ext.allow=never",
];

/// A `git` process with [`HARDENED_CONFIG`] applied and no terminal prompt.
/// Every git the platform starts is built here; add arguments after it.
pub fn git_command() -> Command {
    let mut cmd = Command::new("git");
    for setting in HARDENED_CONFIG {
        cmd.arg("-c").arg(setting);
    }
    cmd.env("GIT_TERMINAL_PROMPT", "0");
    cmd
}

/// Validates that `cred` is a git credential (`github`, `gitlab`, `git`) and,
/// when a provider host is bound or implied, that `repo_url`'s host matches it.
pub fn git_credential_matches_remote(kind: &str, secret: &serde_json::Value, repo_url: &str) -> bool {
    if !matches!(kind, "github" | "gitlab" | "git") {
        return false;
    }
    let Ok(parsed_remote) = reqwest::Url::parse(repo_url) else {
        return false;
    };
    let Some(remote_host) = parsed_remote.host_str().map(|h| h.to_ascii_lowercase()) else {
        return false;
    };
    let bound_url = secret
        .get("repo_url")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .or_else(|| {
            secret
                .get("url")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
        })
        .or_else(|| {
            secret
                .get("instance_url")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
        });
    if let Some(bound) = bound_url {
        let expected_host = reqwest::Url::parse(bound)
            .ok()
            .and_then(|u| u.host_str().map(|h| h.to_ascii_lowercase()))
            .unwrap_or_else(|| bound.trim_end_matches('/').to_ascii_lowercase());
        return !expected_host.is_empty() && remote_host == expected_host;
    }
    match kind {
        "github" => remote_host == "github.com",
        "gitlab" => remote_host == "gitlab.com",
        "git" => true,
        _ => false,
    }
}

fn normalize_lexical_path(path: &Path) -> PathBuf {
    use std::path::Component;
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

fn resolve_existing_ancestor(path: &Path) -> PathBuf {
    let lex = normalize_lexical_path(path);
    let mut cur: &Path = &lex;
    let mut suffix = Vec::new();
    while !cur.as_os_str().is_empty() {
        if let Ok(canon) = std::fs::canonicalize(cur) {
            let mut rebuilt = canon;
            for part in suffix.into_iter().rev() {
                rebuilt.push(part);
            }
            return rebuilt;
        }
        if let Some(name) = cur.file_name() {
            suffix.push(name.to_os_string());
        } else {
            break;
        }
        cur = match cur.parent() {
            Some(p) => p,
            None => break,
        };
    }
    lex
}

/// Validates a Git remote URL:
/// - Refuses bare filesystem paths, SCP-style `user@host:path`, `ssh://`, `git://`, or leading `-`.
/// - For `http://` and `https://`, refuses localhost, internal domains, private/loopback/link-local IP literals, and any DNS resolution to a blocked IP.
/// - For `file://`, requires an absolute path outside `data_root`.
pub fn refuse_remote_in_data_root(repo_url: &str, data_root: &Path) -> Result<(), PlatformError> {
    let repo_url = repo_url.trim();
    if repo_url.is_empty() || repo_url.starts_with('-') {
        return Err(PlatformError::new(
            "GIT_REMOTE_REFUSED",
            "remote URL is empty or invalid",
        ));
    }
    if repo_url.starts_with("http://") || repo_url.starts_with("https://") {
        let parsed = reqwest::Url::parse(repo_url).map_err(|e| {
            PlatformError::new("GIT_REMOTE_REFUSED", format!("invalid remote URL: {e}"))
        })?;
        let Some(host) = parsed.host_str() else {
            return Err(PlatformError::new(
                "GIT_REMOTE_REFUSED",
                "remote URL must include a host",
            ));
        };
        let lower = host.trim_start_matches('[').trim_end_matches(']').trim_end_matches('.').to_ascii_lowercase();
        if lower == "localhost"
            || lower.ends_with(".localhost")
            || lower.ends_with(".local")
            || lower.ends_with(".internal")
            || lower.ends_with(".cluster.local")
        {
            return Err(PlatformError::new(
                "GIT_REMOTE_REFUSED",
                format!("remote URL host '{host}' targets a local or internal domain"),
            ));
        }
        if let Ok(ip) = lower.parse::<std::net::IpAddr>() {
            crate::pipeline::security::validate_outbound_http_url(repo_url, "git.remote")
                .map_err(|e| PlatformError::new("GIT_REMOTE_REFUSED", e.message))?;
            let _ = ip;
            return Ok(());
        }
        if let Some(port) = parsed.port_or_known_default() {
            if let Ok(addrs) = std::net::ToSocketAddrs::to_socket_addrs(&(host, port)) {
                for addr in addrs {
                    let probe = format!("{}://{}:{}/", parsed.scheme(), addr.ip(), port);
                    crate::pipeline::security::validate_outbound_http_url(&probe, "git.remote")
                        .map_err(|e| PlatformError::new("GIT_REMOTE_REFUSED", e.message))?;
                }
            }
        }
        return Ok(());
    }
    let Some(rest) = repo_url.strip_prefix("file://") else {
        return Err(PlatformError::new(
            "GIT_REMOTE_REFUSED",
            "remote URL must use https://, http://, or an absolute file:// path outside the data root",
        ));
    };
    let rest = rest.strip_prefix("localhost").unwrap_or(rest);
    if !rest.starts_with('/') {
        return Err(PlatformError::new(
            "GIT_REMOTE_REFUSED",
            "a file:// remote must be an absolute path",
        ));
    }
    let target_lex = normalize_lexical_path(Path::new(rest));
    let root_lex = normalize_lexical_path(data_root);
    let target_resolved = resolve_existing_ancestor(Path::new(rest));
    let root_resolved = resolve_existing_ancestor(data_root);
    if target_resolved.starts_with(&root_resolved)
        || root_resolved.starts_with(&target_resolved)
        || target_lex.starts_with(&root_lex)
        || root_lex.starts_with(&target_lex)
    {
        return Err(PlatformError::new(
            "GIT_REMOTE_REFUSED",
            "a file:// remote cannot point inside the platform's data root",
        ));
    }
    Ok(())
}

fn run(repo: &Path, pre: &[String], args: &[&str]) -> Result<String, PlatformError> {
    run_env(repo, pre, args, &[])
}

fn run_env(repo: &Path, pre: &[String], args: &[&str], env: &[(&str, &str)]) -> Result<String, PlatformError> {
    let mut cmd = git_command();
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

/// Resolves a remote HTTP(S) git URL from `instance_url` and `repo_url` with any
/// embedded `username:password@` userinfo stripped so credentials never land in
/// `.git/config` or `zebflow.yaml`.
pub fn resolve_clean_git_remote_url(instance_url: &str, repo_url: &str) -> Result<String, String> {
    let trimmed_repo = repo_url.trim();
    let trimmed_instance = instance_url.trim();
    let base = if trimmed_repo.starts_with("http://") || trimmed_repo.starts_with("https://") {
        trimmed_repo.to_string()
    } else if !trimmed_instance.is_empty() {
        format!(
            "{}/{}",
            trimmed_instance.trim_end_matches('/'),
            trimmed_repo.trim_start_matches('/')
        )
    } else {
        return Err(format!("Invalid repo URL: {trimmed_repo}"));
    };
    let mut parsed =
        reqwest::Url::parse(&base).map_err(|e| format!("Invalid repo URL: {e}"))?;
    if !matches!(parsed.scheme(), "http" | "https") {
        return Err(format!(
            "Repo URL must start with https:// or http://: {trimmed_repo}"
        ));
    }
    if parsed.host_str().is_none() {
        return Err("Repo URL must include a host".to_string());
    }
    let _ = parsed.set_username("");
    let _ = parsed.set_password(None);
    parsed.set_query(None);
    parsed.set_fragment(None);
    Ok(parsed.to_string())
}

/// Builds an HTTP Basic `AUTHORIZATION` header value for a git operation without
/// embedding the token into the remote URL.
pub fn git_http_basic_auth_header(username: &str, token: &str) -> Option<String> {
    let token = token.trim();
    if token.is_empty() {
        return None;
    }
    let user = if username.trim().is_empty() {
        "x-access-token"
    } else {
        username.trim()
    };
    let basic = base64::engine::general_purpose::STANDARD.encode(format!("{user}:{token}"));
    Some(format!("AUTHORIZATION: basic {basic}"))
}

/// Ensures `remote.origin.url` in `repo_dir/.git/config` is set to the bare,
/// credential-free URL after a clone or remote update.
pub fn sanitize_cloned_repo_origin(
    repo_dir: &Path,
    clean_remote_url: &str,
) -> Result<(), PlatformError> {
    let parsed = reqwest::Url::parse(clean_remote_url)
        .map_err(|e| PlatformError::new("GIT_REMOTE_REFUSED", format!("invalid remote URL: {e}")))?;
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(PlatformError::new(
            "GIT_REMOTE_REFUSED",
            "remote origin URL must not contain embedded credentials",
        ));
    }
    let current = run(repo_dir, &[], &["remote", "get-url", "origin"]).ok();
    match current {
        Some(_) => run(
            repo_dir,
            &[],
            &["remote", "set-url", "--end-of-options", "origin", clean_remote_url],
        )
        .map(|_| ()),
        None => run(
            repo_dir,
            &[],
            &["remote", "add", "--end-of-options", "origin", clean_remote_url],
        )
        .map(|_| ()),
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
    use serde_json::json;

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
    fn a_file_remote_never_reaches_into_the_data_root() {
        let root = std::env::temp_dir().join(format!("zf-git-root-{}", std::process::id()));
        std::fs::create_dir_all(root.join("tenants/a/b/repo")).unwrap();
        let inside = format!("file://{}", root.join("tenants/a/b/repo").display());
        let dotted = format!("file://{}/tenants/../tenants/a/b/repo", root.display());
        assert!(refuse_remote_in_data_root(&inside, &root).is_err());
        assert!(refuse_remote_in_data_root(&dotted, &root).is_err());
        assert!(refuse_remote_in_data_root("file:///srv/remotes/site-a.git", &root).is_ok());
        assert!(refuse_remote_in_data_root("https://git.example.com/a.git", &root).is_ok());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn git_sync_refuses_bare_and_internal_remotes_checks_credential_and_locks() {
        let root = std::env::temp_dir().join(format!("zf-git-guard-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();

        for bad_remote in [
            "../../other-owner/other-project/repo",
            "/srv/remotes/bare-path-without-scheme",
            "git@github.com:org/repo.git",
            "ssh://git.example.com/repo.git",
            "--upload-pack=touch /tmp/pwn",
            "http://127.0.0.1:8080/repo.git",
            "http://169.254.169.254/latest/meta-data",
            "http://localhost/repo.git",
            "https://api.svc.cluster.local/repo.git",
        ] {
            assert!(
                refuse_remote_in_data_root(bad_remote, &root).is_err(),
                "expected remote '{bad_remote}' to be refused"
            );
        }

        // Credential kind and host binding:
        assert!(!git_credential_matches_remote(
            "jwt_signing_key",
            &json!({ "token": uuid::Uuid::new_v4().to_string() }),
            "https://github.com/org/repo.git"
        ));
        assert!(git_credential_matches_remote(
            "github",
            &json!({ "token": uuid::Uuid::new_v4().to_string() }),
            "https://github.com/org/repo.git"
        ));
        assert!(
            !git_credential_matches_remote(
                "github",
                &json!({ "token": uuid::Uuid::new_v4().to_string() }),
                "https://evil.example.com/org/repo.git"
            ),
            "github credential must not send token to arbitrary third-party host"
        );
        assert!(
            !git_credential_matches_remote(
                "gitlab",
                &json!({ "token": uuid::Uuid::new_v4().to_string(), "url": "https://gitlab.example.com" }),
                "https://evil.example.com/org/repo.git"
            ),
            "gitlab credential bound to gitlab.example.com must not authenticate to evil.example.com"
        );

        // Lock, .git, and machine-owned path checks in validate_git_resolve_path:
        let p = crate::pipeline::nodes::shared::test_platform::test_platform();
        assert!(p.projects.validate_git_resolve_path("superadmin", "default", ".git/config").is_err());
        assert!(p.projects.validate_git_resolve_path("superadmin", "default", "zebflow.yaml").is_err());
        assert!(p.projects.validate_git_resolve_path("superadmin", "default", "zeb.lock").is_err());
        p.zebflow_cfg
            .set_template_locked("superadmin", "default", "pages/home.tsx", true)
            .expect("lock template");
        let err = p
            .projects
            .validate_git_resolve_path("superadmin", "default", "pages/home.tsx")
            .expect_err("locked template must not be resolvable via git resolve");
        assert_eq!(err.code, "PLATFORM_TEMPLATE_LOCKED");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_planted_hook_never_runs() {
        let repo = std::env::temp_dir().join(format!("zf-git-hook-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&repo);
        std::fs::create_dir_all(&repo).unwrap();
        assert!(git_command().arg("init").arg("-q").arg(&repo).status().unwrap().success());
        let marker = repo.join("hook-ran");
        let hook = repo.join(".git/hooks/pre-commit");
        std::fs::write(&hook, format!("#!/bin/sh\ntouch '{}'\n", marker.display())).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&hook, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let ok = git_command()
            .args(["-c", "user.name=a", "-c", "user.email=a@example.com", "-C"])
            .arg(&repo)
            .args(["commit", "-q", "--allow-empty", "-m", "one"])
            .status()
            .unwrap()
            .success();
        assert!(ok);
        assert!(!marker.exists(), "the repository's own hook ran");
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn secrets_never_survive_an_error_message() {
        assert_eq!(scrub_secrets("fatal: https://user:ghp_abc@github.com/x/y failed"), "fatal: https://[redacted]@github.com/x/y failed");
        assert_eq!(scrub_secrets("header AUTHORIZATION: basic dXNlcjp0b2tlbg== rejected"), "header AUTHORIZATION: basic [redacted] rejected");
    }

    #[test]
    fn git_clone_strips_credentials_from_origin_url_and_refuses_existing_project() {
        let secret_val = uuid::Uuid::new_v4().to_string();
        let embedded = format!("https://oauth2:{secret_val}@git.example.com/org/site-a.git");
        let clean = resolve_clean_git_remote_url("", &embedded).expect("clean URL");
        assert_eq!(clean, "https://git.example.com/org/site-a.git");
        assert!(!clean.contains(&secret_val));

        let repo = std::env::temp_dir().join(format!("zf-git-origin-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&repo);
        std::fs::create_dir_all(&repo).unwrap();
        assert!(git_command().arg("init").arg("-q").arg(&repo).status().unwrap().success());
        assert!(
            git_command()
                .arg("-C")
                .arg(&repo)
                .args(["remote", "add", "origin", &embedded])
                .status()
                .unwrap()
                .success()
        );

        // Passing a URL with embedded credentials to sanitize_cloned_repo_origin is refused:
        assert!(sanitize_cloned_repo_origin(&repo, &embedded).is_err());

        // Sanitizing with the clean URL removes the credential from .git/config:
        sanitize_cloned_repo_origin(&repo, &clean).expect("sanitize origin");
        let git_config = std::fs::read_to_string(repo.join(".git/config")).expect("read .git/config");
        assert!(
            !git_config.contains(&secret_val),
            ".git/config must not contain the provider secret after clone sanitization"
        );
        assert!(git_config.contains("https://git.example.com/org/site-a.git"));
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn git_sync_auth_env_passes_http_header_via_git_config_env_not_argv() {
        let p = crate::pipeline::nodes::shared::test_platform::test_platform();
        let cred_val = uuid::Uuid::new_v4().to_string();
        p.credentials
            .upsert_project_credential(
                "superadmin",
                "default",
                &crate::platform::model::UpsertProjectCredentialRequest {
                    credential_id: "git-origin".to_string(),
                    title: "GitHub Origin".to_string(),
                    kind: "github".to_string(),
                    secret: json!({
                        "username": "ci-bot",
                        "token": cred_val,
                        "repo_url": "https://github.com/org/site-a.git"
                    }),
                    notes: String::new(),
                },
            )
            .expect("upsert credential");

        let env = GitSyncService::new((*p).clone()).auth_env(
            "superadmin",
            "default",
            "git-origin",
            "https://github.com/org/site-a.git",
        );
        assert_eq!(env.len(), 3);
        assert_eq!(env[0], ("GIT_CONFIG_COUNT", "1".to_string()));
        assert_eq!(env[1], ("GIT_CONFIG_KEY_0", "http.extraheader".to_string()));
        assert!(env[2].1.starts_with("AUTHORIZATION: basic "));
    }
}
