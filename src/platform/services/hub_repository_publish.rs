//! Publishing into a Hub-format repository kept in git: `github.com/zebflow/hub`,
//! or any repository laid out the same way (`HubRepositoryIndex`).
//!
//! A static repository has no server to accept a package, so publishing to one
//! is a git push. The repository is cloned shallow into a temporary folder, the
//! release is written beside the ones already there, the index is updated and
//! checked against its contract, and one commit is pushed. Whoever holds a
//! token the git host accepts for that repository can publish: that is the
//! whole permission model, the same one as pushing by hand.
//!
//! The token reaches git only as an HTTP header in the environment of the
//! command that needs it (`git_http_auth_env`), never in a URL, a file or an
//! argument, and every message git prints is scrubbed of it before it is
//! returned.

use std::fs;
use std::path::Path;

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::contracts::ContractMetadata;
use crate::contracts::kinds::{
    HUB_REPOSITORY_INDEX_FILE, HubRepositoryIndexPackage, HubRepositoryIndexRelease, HubRepositoryIndexSpec,
    decode_hub_repository_index, encode_hub_repository_index, is_hub_package_id, is_hub_package_version,
};
use crate::platform::error::PlatformError;
use crate::platform::model::ResolvedGitIdentity;
use crate::platform::services::git_sync::{checked_remote_url, git_command, git_http_auth_env, scrub_secrets};
use crate::platform::services::project_library::version_order;

/// The official Hub repository, the one every Zebflow reads first.
pub const OFFICIAL_HUB_GIT_URL: &str = "https://github.com/zebflow/hub.git";
/// The branch the official Hub is read from.
pub const DEFAULT_HUB_BRANCH: &str = "main";

/// What a repository that has no index yet calls itself.
const NEW_INDEX_NAME: &str = "zebflow-hub";

/// Where a release goes, and the credential that lets it.
pub struct RepositoryTarget<'a> {
    pub repo_url: &'a str,
    pub branch: &'a str,
    /// A token the git host accepts for writing to this repository.
    pub token: &'a str,
    /// Who the commit names: the platform user who publishes.
    pub author: &'a ResolvedGitIdentity,
}

/// One reviewed, sealed release and the presentation that travels with it.
pub struct RepositoryRelease {
    pub package_id: String,
    pub version: String,
    pub asset_kind: String,
    pub title: String,
    pub description: String,
    pub tags: Vec<String>,
    /// The encoded release document: these are the bytes the index pins.
    pub document: Vec<u8>,
    /// Bytes the document names by digest rather than carries.
    pub artifacts: Vec<Vec<u8>>,
    /// The cover as `(file name, bytes)`, e.g. `("cover.webp", …)`.
    pub cover: Option<(String, Vec<u8>)>,
    /// The package page, in Markdown.
    pub readme: Option<String>,
}

/// What a publish did.
#[derive(Debug, Clone, Serialize)]
pub struct RepositoryPublishReport {
    pub repository: String,
    pub branch: String,
    pub package_id: String,
    pub version: String,
    /// The pushed commit; empty when nothing changed.
    pub commit: String,
    /// The repository files this publish wrote, index included.
    pub files: Vec<String>,
    /// `false` when this exact release was already there.
    pub changed: bool,
}

/// Writes `release` into the repository `target` names and pushes it.
///
/// A release is immutable, as on every other channel: the same version with
/// the same bytes is answered as unchanged, and the same version with other
/// bytes is refused, because every `zeb.lock` that pinned it would now name
/// something else.
pub fn publish_release(
    data_root: &Path,
    target: &RepositoryTarget,
    release: &RepositoryRelease,
) -> Result<RepositoryPublishReport, PlatformError> {
    if !is_hub_package_id(&release.package_id) || !is_hub_package_version(&release.version) {
        return Err(refused(format!(
            "'{}' {} is not a package id and version a repository can hold",
            release.package_id, release.version
        )));
    }
    let url = checked_remote_url(target.repo_url, data_root)?;
    let branch = checked_branch(target.branch)?;
    let git = Git::new(target.token, target.author);
    let work = tempfile::tempdir().map_err(|err| PlatformError::new("HUB_REPOSITORY_PUBLISH", err.to_string()))?;
    let dir = work.path().join("hub");
    if git.remote_has_branch(work.path(), &url, branch)? {
        git.run(work.path(), &["clone", "--quiet", "--depth", "1", "--single-branch", "--branch", branch, &url, "hub"])?;
    } else {
        // An empty repository, or a branch nobody pushed yet: this publish
        // makes its first commit.
        fs::create_dir(&dir)?;
        git.run(&dir, &["init", "--quiet", "--initial-branch", branch])?;
        git.run(&dir, &["remote", "add", "origin", &url])?;
    }

    let (metadata, mut spec) = read_index(&dir)?;
    let digest = sha256_hex(&release.document);
    if let Some(existing) = find_release(&spec, &release.package_id, &release.version) {
        if existing.sha256 == digest {
            return Ok(report(&url, branch, release, String::new(), Vec::new(), false));
        }
        return Err(refused(format!(
            "{} {} is already published with other contents; publish a new version",
            release.package_id, release.version
        )));
    }

    let package_dir = format!("packages/{}", release.package_id);
    let document_path = format!("{package_dir}/{}/package.json", release.version);
    let mut files = vec![document_path.clone()];
    write_file(&dir, &document_path, &release.document)?;
    for bytes in &release.artifacts {
        let path = format!("artifacts/{}", sha256_hex(bytes));
        write_file(&dir, &path, bytes)?;
        files.push(path);
    }
    let cover = match &release.cover {
        Some((name, bytes)) => {
            let path = format!("{package_dir}/media/{name}");
            write_file(&dir, &path, bytes)?;
            files.push(path.clone());
            Some(path)
        }
        None => None,
    };
    let readme = match &release.readme {
        Some(text) => {
            let path = format!("{package_dir}/README.md");
            write_file(&dir, &path, text.as_bytes())?;
            files.push(path.clone());
            Some(path)
        }
        None => None,
    };
    upsert_entry(
        &mut spec,
        release,
        HubRepositoryIndexRelease {
            version: release.version.clone(),
            path: document_path,
            sha256: digest,
            size_bytes: release.document.len(),
        },
        cover,
        readme,
    );
    let index = encode_hub_repository_index(metadata, spec)
        .map_err(|err| refused(format!("the updated index is not valid: {err}")))?;
    write_file(&dir, HUB_REPOSITORY_INDEX_FILE, &index)?;
    files.push(HUB_REPOSITORY_INDEX_FILE.to_string());

    git.run(&dir, &["add", "--all"])?;
    let message = format!("Publish {} {}", release.package_id, release.version);
    git.run(&dir, &["commit", "--quiet", "--message", &message])?;
    git.run(&dir, &["push", "--quiet", "origin", &format!("HEAD:refs/heads/{branch}")])?;
    let commit = git.run(&dir, &["rev-parse", "HEAD"])?.trim().to_string();
    Ok(report(&url, branch, release, commit, files, true))
}

/// The index already there, or a new one for a repository that has none.
fn read_index(dir: &Path) -> Result<(ContractMetadata, HubRepositoryIndexSpec), PlatformError> {
    let path = dir.join(HUB_REPOSITORY_INDEX_FILE);
    if !path.is_file() {
        return Ok((ContractMetadata::named(NEW_INDEX_NAME), HubRepositoryIndexSpec::default()));
    }
    let document = decode_hub_repository_index(&fs::read(&path)?)
        .map_err(|err| refused(format!("the repository's {HUB_REPOSITORY_INDEX_FILE} is not valid: {err}")))?;
    Ok((document.metadata, document.spec))
}

fn find_release<'a>(spec: &'a HubRepositoryIndexSpec, package_id: &str, version: &str) -> Option<&'a HubRepositoryIndexRelease> {
    spec.find_release(package_id, Some(version)).map(|(_, release)| release)
}

/// Adds the release to its package's entry, or makes the entry. The package's
/// card follows the newest publish; `latest_version` moves only forward, so a
/// fix published for an older line never becomes what a bare reference means.
fn upsert_entry(
    spec: &mut HubRepositoryIndexSpec,
    release: &RepositoryRelease,
    entry: HubRepositoryIndexRelease,
    cover: Option<String>,
    readme: Option<String>,
) {
    let position = spec.packages.iter().position(|item| item.package_id == release.package_id);
    let package = match position {
        Some(index) => &mut spec.packages[index],
        None => {
            spec.packages.push(HubRepositoryIndexPackage {
                package_id: release.package_id.clone(),
                latest_version: release.version.clone(),
                ..HubRepositoryIndexPackage::default()
            });
            spec.packages.last_mut().expect("just pushed")
        }
    };
    package.asset_kind = release.asset_kind.clone();
    package.title = release.title.clone();
    package.description = release.description.clone();
    package.tags = release.tags.clone();
    if let Some(path) = cover {
        package.cover = path;
    }
    if let Some(path) = readme {
        package.readme = path;
    }
    if is_newer(&release.version, &package.latest_version) {
        package.latest_version = release.version.clone();
    }
    package.releases.insert(0, entry);
}

/// Whether `candidate` sorts after `current`. A version that is not `x.y.z`
/// cannot be ordered, so it wins: the publish that wrote it meant it.
fn is_newer(candidate: &str, current: &str) -> bool {
    match (version_order(candidate), version_order(current)) {
        (Some(candidate), Some(current)) => candidate > current,
        _ => candidate != current,
    }
}

fn report(url: &str, branch: &str, release: &RepositoryRelease, commit: String, files: Vec<String>, changed: bool) -> RepositoryPublishReport {
    RepositoryPublishReport {
        repository: url.to_string(),
        branch: branch.to_string(),
        package_id: release.package_id.clone(),
        version: release.version.clone(),
        commit,
        files,
        changed,
    }
}

/// A branch name git accepts that is never read as an option or a path step.
fn checked_branch(branch: &str) -> Result<&str, PlatformError> {
    let branch = branch.trim();
    let ok = !branch.is_empty()
        && branch.len() <= 100
        && !branch.starts_with(['-', '/', '.'])
        && !branch.ends_with(['/', '.'])
        && !branch.contains("..")
        && !branch.contains("//")
        && branch.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/'));
    if ok { Ok(branch) } else { Err(refused(format!("'{branch}' is not a branch name this can push to"))) }
}

/// `rel` is built here from a checked package id, version and file name, so it
/// never leaves `dir`.
fn write_file(dir: &Path, rel: &str, bytes: &[u8]) -> Result<(), PlatformError> {
    let path = dir.join(rel);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)?;
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn refused(message: impl Into<String>) -> PlatformError {
    PlatformError::new("HUB_REPOSITORY_PUBLISH", message)
}

/// Runs git with the token as a header on every command, and scrubs it from
/// whatever git says.
struct Git<'a> {
    token: &'a str,
    author: &'a ResolvedGitIdentity,
    auth: Vec<(&'static str, String)>,
}

impl<'a> Git<'a> {
    fn new(token: &'a str, author: &'a ResolvedGitIdentity) -> Self {
        Self { token, author, auth: git_http_auth_env("", token) }
    }

    fn run(&self, cwd: &Path, args: &[&str]) -> Result<String, PlatformError> {
        let mut cmd = git_command();
        cmd.arg("-c").arg(format!("user.name={}", self.author.name));
        cmd.arg("-c").arg(format!("user.email={}", self.author.email));
        cmd.arg("-C").arg(cwd).args(args);
        for (key, value) in &self.auth {
            cmd.env(key, value);
        }
        let out = cmd.output().map_err(|err| PlatformError::new("GIT_EXEC", err.to_string()))?;
        if out.status.success() {
            return Ok(String::from_utf8_lossy(&out.stdout).to_string());
        }
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let said = if stderr.is_empty() { String::from_utf8_lossy(&out.stdout).trim().to_string() } else { stderr };
        Err(PlatformError::new("GIT_FAILED", scrub_secrets(&said, &[self.token])))
    }

    fn remote_has_branch(&self, cwd: &Path, url: &str, branch: &str) -> Result<bool, PlatformError> {
        let heads = self.run(cwd, &["ls-remote", "--heads", url, &format!("refs/heads/{branch}")])?;
        Ok(!heads.trim().is_empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bare_remote(root: &Path) -> String {
        let remote = root.join("remote.git");
        let status = git_command().args(["init", "--quiet", "--bare"]).arg(&remote).status().expect("git");
        assert!(status.success());
        format!("file://{}", remote.display())
    }

    fn release(version: &str, body: &str) -> RepositoryRelease {
        RepositoryRelease {
            package_id: "zebflow.realtime-game".to_string(),
            version: version.to_string(),
            asset_kind: "project_bundle".to_string(),
            title: "Real-Time Game".to_string(),
            description: "A shared board, live.".to_string(),
            tags: vec!["realtime".to_string(), "games".to_string()],
            document: format!("{{\"release\":\"{body}\"}}").into_bytes(),
            artifacts: vec![b"large file".to_vec()],
            cover: Some(("cover.webp".to_string(), b"RIFFwebp".to_vec())),
            readme: Some("# Real-Time Game\n".to_string()),
        }
    }

    /// The platform user who publishes, as the route resolves them.
    fn publisher() -> ResolvedGitIdentity {
        ResolvedGitIdentity {
            name: "Publisher".to_string(),
            email: "publisher@example.com".to_string(),
            source: crate::platform::model::GitIdentitySource::UserProfile,
        }
    }

    fn read_back(root: &Path, url: &str) -> HubRepositoryIndexSpec {
        let check = root.join("check");
        let _ = fs::remove_dir_all(&check);
        let status = git_command().args(["clone", "--quiet", "--branch", "main", url]).arg(&check).status().expect("git");
        assert!(status.success());
        decode_hub_repository_index(&fs::read(check.join(HUB_REPOSITORY_INDEX_FILE)).unwrap()).expect("index").spec
    }

    #[test]
    fn a_first_publish_makes_the_index_and_pushes_every_file() {
        let (root, data) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let url = bare_remote(root.path());
        let author = publisher();
        let target = RepositoryTarget { repo_url: &url, branch: "main", token: "", author: &author };
        let report = publish_release(data.path(), &target, &release("1.0.0", "one")).expect("publish");
        assert!(report.changed && !report.commit.is_empty());
        assert!(report.files.contains(&"packages/zebflow.realtime-game/1.0.0/package.json".to_string()));
        assert!(report.files.contains(&format!("artifacts/{}", sha256_hex(b"large file"))));

        let spec = read_back(root.path(), &url);
        let package = &spec.packages[0];
        assert_eq!(package.latest_version, "1.0.0");
        assert_eq!(package.cover, "packages/zebflow.realtime-game/media/cover.webp");
        assert_eq!(package.readme, "packages/zebflow.realtime-game/README.md");
        assert_eq!(package.tags, ["realtime", "games"]);
        assert_eq!(package.releases[0].sha256, sha256_hex(b"{\"release\":\"one\"}"));

        // The commit names the platform user who published, author and committer.
        let who = git_command()
            .args(["-C"]).arg(root.path().join("check"))
            .args(["log", "-1", "--format=%an <%ae> | %cn <%ce>"])
            .output()
            .expect("git log");
        assert_eq!(String::from_utf8_lossy(&who.stdout).trim(), "Publisher <publisher@example.com> | Publisher <publisher@example.com>");
    }

    #[test]
    fn a_published_version_is_immutable() {
        let (root, data) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let url = bare_remote(root.path());
        let author = publisher();
        let target = RepositoryTarget { repo_url: &url, branch: "main", token: "", author: &author };
        publish_release(data.path(), &target, &release("1.0.0", "one")).expect("publish");
        let again = publish_release(data.path(), &target, &release("1.0.0", "one")).expect("same bytes");
        assert!(!again.changed);
        let error = publish_release(data.path(), &target, &release("1.0.0", "two")).expect_err("other bytes");
        assert!(error.message.contains("already published"), "{}", error.message);
    }

    #[test]
    fn latest_moves_only_forward() {
        let (root, data) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        let url = bare_remote(root.path());
        let author = publisher();
        let target = RepositoryTarget { repo_url: &url, branch: "main", token: "", author: &author };
        publish_release(data.path(), &target, &release("1.1.0", "b")).expect("1.1.0");
        publish_release(data.path(), &target, &release("1.0.1", "a")).expect("1.0.1");
        let spec = read_back(root.path(), &url);
        assert_eq!(spec.packages[0].latest_version, "1.1.0");
        assert_eq!(spec.packages[0].releases.len(), 2);
    }

    #[test]
    fn a_branch_that_could_be_an_option_or_a_path_is_refused() {
        for branch in ["", "-x", "../main", "a..b", "a b", "main/"] {
            checked_branch(branch).expect_err(branch);
        }
        checked_branch("publish/realtime-game").expect("a path-like branch");
    }
}
