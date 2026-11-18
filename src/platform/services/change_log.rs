//! The change record (`project.md` § Change record): one line for every change
//! Zebflow makes in a project's `repo/`, naming who made it and through what.
//!
//! Who is not passed down through every writer. The entry that knows it — the
//! web layer for a signed-in person, the MCP handler for a key — opens a
//! [`scope`], and a writer deep below reads it with [`current_actor`]. Work
//! that leaves the task (a blocking thread, a spawned task) carries it across
//! with [`carry`].

use std::future::Future;
use std::path::Path;
use std::sync::Arc;

use sha2::{Digest, Sha256};

use crate::platform::adapters::data::DataAdapter;
use crate::platform::error::PlatformError;
use crate::platform::model::{ProjectChange, now_ts, slug_segment};

/// Lines kept per project; older ones are dropped.
pub const KEEP_PER_PROJECT: usize = 20_000;
/// A file above this is recorded without its hashes.
const HASH_MAX_BYTES: u64 = 8 * 1024 * 1024;
/// How often, in lines, a project's record is trimmed back to [`KEEP_PER_PROJECT`].
const PRUNE_EVERY: i64 = 500;

/// Who is changing the project, and through what.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChangeActor {
    pub person: String,
    pub via: String,
}

impl ChangeActor {
    /// A signed-in person in a browser, or calling the API with their session.
    pub fn studio(person: &str) -> Self {
        Self { person: slug_segment(person), via: "studio".to_string() }
    }

    /// An agent holding one of the person's MCP keys.
    pub fn mcp(person: &str, key_name: &str) -> Self {
        Self { person: slug_segment(person), via: format!("mcp · {key_name}") }
    }

    /// Nobody asked: a pipeline run, a start-up repair.
    pub fn system() -> Self {
        Self { person: String::new(), via: "system".to_string() }
    }
}

tokio::task_local! {
    static ACTOR: ChangeActor;
}

/// Runs `work` with `actor` as the one changing things.
pub async fn scope<F: Future>(actor: ChangeActor, work: F) -> F::Output {
    ACTOR.scope(actor, work).await
}

/// The actor in scope here, if any — to hand to work that runs elsewhere.
pub fn carried() -> Option<ChangeActor> {
    ACTOR.try_with(Clone::clone).ok()
}

/// Runs blocking `work` under an actor taken with [`carried`] on the task that
/// started it.
pub fn carry<R>(actor: Option<ChangeActor>, work: impl FnOnce() -> R) -> R {
    match actor {
        Some(actor) => ACTOR.sync_scope(actor, work),
        None => work(),
    }
}

/// Who is changing things now: the actor in scope, or `system`.
pub fn current_actor() -> ChangeActor {
    carried().unwrap_or_else(ChangeActor::system)
}

/// `sha256:<hex>` of some bytes.
pub fn digest(bytes: &[u8]) -> String {
    format!("sha256:{}", hex::encode(Sha256::digest(bytes)))
}

/// The digest of a file as it is on disk; none for a folder, a missing file,
/// or one above 8 MB.
pub fn file_digest(path: &Path) -> Option<String> {
    let meta = std::fs::metadata(path).ok()?;
    if !meta.is_file() || meta.len() > HASH_MAX_BYTES {
        return None;
    }
    std::fs::read(path).ok().map(|bytes| digest(&bytes))
}

/// The digest of bytes about to be written; none above 8 MB, like [`file_digest`].
pub fn bytes_digest(bytes: &[u8]) -> Option<String> {
    (bytes.len() as u64 <= HASH_MAX_BYTES).then(|| digest(bytes))
}

/// One change, before it is stamped with who and when.
#[derive(Debug, Clone, Default)]
pub struct Change {
    pub action: String,
    pub path: String,
    pub to_path: Option<String>,
    pub before: Option<String>,
    pub after: Option<String>,
}

impl Change {
    /// A file written: `create` when nothing was there, `edit` otherwise.
    pub fn write(path: &str, before: Option<String>, existed: bool, after: Option<String>) -> Self {
        Self {
            action: if existed { "edit" } else { "create" }.to_string(),
            path: path.to_string(),
            before,
            after,
            ..Self::default()
        }
    }

    pub fn create_folder(path: &str) -> Self {
        Self { action: "create".to_string(), path: path.to_string(), ..Self::default() }
    }

    pub fn delete(path: &str, before: Option<String>) -> Self {
        Self { action: "delete".to_string(), path: path.to_string(), before, ..Self::default() }
    }

    pub fn moved(from: &str, to: &str) -> Self {
        Self {
            action: "move".to_string(),
            path: from.to_string(),
            to_path: Some(to.to_string()),
            ..Self::default()
        }
    }

    /// A step that touches many files, such as `git sync` or `import`.
    pub fn step(action: &str) -> Self {
        Self { action: action.to_string(), ..Self::default() }
    }

    /// A write whose bytes did not change is not a change.
    fn is_noop(&self) -> bool {
        matches!(self.action.as_str(), "edit")
            && self.before.is_some()
            && self.before == self.after
    }
}

/// `(owner, project, repository-relative path)` of a file under
/// `{users_root}/{owner}/{project}/repo/`; none for anything outside a repo.
pub fn repo_relative(users_root: &Path, path: &Path) -> Option<(String, String, String)> {
    let rest = path.strip_prefix(users_root).ok()?;
    let mut parts = rest.components();
    let owner = parts.next()?.as_os_str().to_str()?.to_string();
    let project = parts.next()?.as_os_str().to_str()?.to_string();
    if parts.next()?.as_os_str() != "repo" {
        return None;
    }
    let rel = parts.as_path().to_string_lossy().replace('\\', "/");
    Some((owner, project, rel))
}

/// Where a service records, once the platform attaches the record. A service
/// built on its own — in a unit test — records nothing.
#[derive(Default)]
pub struct ChangeSink(std::sync::OnceLock<Arc<ChangeLog>>);

impl ChangeSink {
    pub fn attach(&self, log: Arc<ChangeLog>) {
        let _ = self.0.set(log);
    }

    pub fn record(&self, owner: &str, project: &str, change: Change) {
        if let Some(log) = self.0.get() {
            log.record(owner, project, change);
        }
    }

    /// A file written at an absolute path inside some project's `repo/`.
    pub fn record_file_write(&self, users_root: &Path, path: &Path, before: Option<String>, existed: bool) {
        if self.0.get().is_none() {
            return;
        }
        if let Some((owner, project, rel)) = repo_relative(users_root, path) {
            self.record(&owner, &project, Change::write(&rel, before, existed, file_digest(path)));
        }
    }
}

/// The record of each data root this process serves, for writers that know
/// only the data root (plain functions below the services). Weak, so a
/// platform dropped in a test takes its record with it.
static BY_DATA_ROOT: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<std::path::PathBuf, std::sync::Weak<ChangeLog>>>> =
    std::sync::OnceLock::new();

/// Makes `log` the record for everything under `data_root`.
pub fn register(data_root: &Path, log: &Arc<ChangeLog>) {
    let map = BY_DATA_ROOT.get_or_init(Default::default);
    map.lock()
        .unwrap_or_else(|e| e.into_inner())
        .insert(data_root.to_path_buf(), Arc::downgrade(log));
}

fn registered(data_root: &Path) -> Option<Arc<ChangeLog>> {
    BY_DATA_ROOT
        .get()?
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .get(data_root)
        .and_then(std::sync::Weak::upgrade)
}

/// Records a change for a writer that knows only the data root.
pub fn record_at(data_root: &Path, owner: &str, project: &str, change: Change) {
    if let Some(log) = registered(data_root) {
        log.record(owner, project, change);
    }
}

/// Records a file written at an absolute path, for a writer that knows only
/// the data root. `before` and `existed` are taken before the write.
pub fn record_file_at(data_root: &Path, path: &Path, before: Option<String>, existed: bool) {
    let Some(log) = registered(data_root) else { return };
    if let Some((owner, project, rel)) = repo_relative(&data_root.join("users"), path) {
        log.record(&owner, &project, Change::write(&rel, before, existed, file_digest(path)));
    }
}

/// Records a file or folder removed at an absolute path.
pub fn record_removal_at(data_root: &Path, path: &Path, before: Option<String>) {
    let Some(log) = registered(data_root) else { return };
    if let Some((owner, project, rel)) = repo_relative(&data_root.join("users"), path) {
        log.record(&owner, &project, Change::delete(&rel, before));
    }
}

/// The record itself, written through to the platform catalog.
pub struct ChangeLog {
    data: Arc<dyn DataAdapter>,
}

impl ChangeLog {
    pub fn new(data: Arc<dyn DataAdapter>) -> Self {
        Self { data }
    }

    /// Writes one line, stamped with the actor in scope and the time. A failure
    /// to record never fails the change itself; it is reported on stderr.
    pub fn record(&self, owner: &str, project: &str, change: Change) {
        if change.is_noop() {
            return;
        }
        let actor = current_actor();
        let line = ProjectChange {
            id: 0,
            owner: slug_segment(owner),
            project: slug_segment(project),
            at: now_ts(),
            person: actor.person,
            via: actor.via,
            action: change.action,
            path: change.path,
            to_path: change.to_path,
            before: change.before,
            after: change.after,
        };
        match self.data.put_project_change(&line) {
            Ok(id) if id % PRUNE_EVERY == 0 => {
                if let Err(err) = self.data.prune_project_changes(&line.owner, &line.project, KEEP_PER_PROJECT) {
                    eprintln!("warning: trimming the change record of {}/{}: {}", line.owner, line.project, err.message);
                }
            }
            Ok(_) => {}
            Err(err) => eprintln!(
                "warning: recording a change in {}/{} ({} {}): {}",
                line.owner, line.project, line.action, line.path, err.message
            ),
        }
    }

    /// Newest first. `path` keeps the lines about that file or anything under
    /// that folder; `before_id` pages back from a line already seen.
    pub fn list(
        &self,
        owner: &str,
        project: &str,
        path: Option<&str>,
        before_id: Option<i64>,
        limit: usize,
    ) -> Result<Vec<ProjectChange>, PlatformError> {
        let path = path.map(|p| p.trim().trim_matches('/')).filter(|p| !p.is_empty());
        self.data.list_project_changes(
            &slug_segment(owner),
            &slug_segment(project),
            path,
            before_id,
            limit.clamp(1, 500),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::model::PlatformConfig;
    use crate::platform::services::PlatformService;

    fn platform() -> (tempfile::TempDir, PlatformService) {
        let dir = tempfile::tempdir().expect("tempdir");
        let platform = PlatformService::from_config(PlatformConfig {
            data_root: dir.path().to_path_buf(),
            ..PlatformConfig::default()
        })
        .expect("platform");
        (dir, platform)
    }

    #[tokio::test]
    async fn a_line_names_the_actor_in_scope_and_system_without_one() {
        let (_dir, platform) = platform();
        let log = &platform.changes;
        log.record("superadmin", "lab", Change::step("git sync"));
        scope(ChangeActor::mcp("dev-one", "laptop"), async {
            log.record("superadmin", "lab", Change::write("pages/a.tsx", None, false, Some(digest(b"a"))));
        })
        .await;
        // Carried into a blocking thread, as the web layer does.
        let actor = scope(ChangeActor::studio("superadmin"), async { carried() }).await;
        let log2 = platform.changes.clone();
        tokio::task::spawn_blocking(move || {
            carry(actor, || log2.record("superadmin", "lab", Change::delete("old.md", None)))
        })
        .await
        .expect("blocking");

        let lines = log.list("superadmin", "lab", None, None, 10).expect("list");
        let seen: Vec<(&str, &str, &str, &str)> = lines
            .iter()
            .map(|l| (l.person.as_str(), l.via.as_str(), l.action.as_str(), l.path.as_str()))
            .collect();
        assert_eq!(
            seen,
            vec![
                ("superadmin", "studio", "delete", "old.md"),
                ("dev-one", "mcp · laptop", "create", "pages/a.tsx"),
                ("", "system", "git sync", ""),
            ],
            "newest first"
        );
    }

    #[test]
    fn an_unchanged_write_is_not_a_change_and_a_path_filter_keeps_its_folder() {
        let (_dir, platform) = platform();
        let log = &platform.changes;
        let same = Some(digest(b"x"));
        log.record("superadmin", "lab", Change::write("pages/a.tsx", same.clone(), true, same));
        assert!(log.list("superadmin", "lab", None, None, 10).expect("list").is_empty());

        log.record("superadmin", "lab", Change::write("pages/a.tsx", None, false, Some(digest(b"a"))));
        log.record("superadmin", "lab", Change::write("pages-old/b.tsx", None, false, None));
        log.record("superadmin", "lab", Change::moved("pages/a.tsx", "pages/c.tsx"));
        let under: Vec<String> = log
            .list("superadmin", "lab", Some("pages/"), None, 10)
            .expect("list")
            .into_iter()
            .map(|l| l.path)
            .collect();
        assert_eq!(under, vec!["pages/a.tsx", "pages/a.tsx"], "a folder is not a prefix of its neighbour");
        let first = log.list("superadmin", "lab", None, None, 1).expect("page");
        let rest = log.list("superadmin", "lab", None, Some(first[0].id), 10).expect("next page");
        assert_eq!(rest.len(), 2);
    }
}
