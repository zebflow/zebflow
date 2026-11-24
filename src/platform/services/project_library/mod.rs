//! A project's own RWE library (`rwe-library-manifest/README.md` § Project
//! libraries): one `.tar.gz` a maintainer uploads, built elsewhere from a
//! `LibraryRecipe`, checked file by file, installed for this project only and
//! locked as `direct.file`.

mod archive;
mod scan;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::Serialize;
use sha2::{Digest, Sha256};

use crate::contracts::kinds::{DependencyLockArtifactSpec, DependencyLockSource, RweLibraryManifestContract};
use crate::platform::error::PlatformError;
use crate::platform::services::PlatformService;
use crate::platform::services::library::{RweLibraryManifestSpec, RweLibraryVersionSpec};

/// The largest upload read, compressed.
pub const MAX_UPLOAD_BYTES: usize = 64 * 1024 * 1024;

/// What an upload installed.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct InstalledProjectLibrary {
    pub name: String,
    pub version: String,
    pub entries: Vec<String>,
    pub files: usize,
    pub bytes: u64,
    /// `false` when the same version with the same files was already there.
    pub changed: bool,
}

pub(super) fn refuse(message: impl Into<String>) -> PlatformError {
    PlatformError::new("PROJECT_LIBRARY_INVALID", message)
}

fn digest(bytes: &[u8]) -> String {
    format!("sha256:{:x}", Sha256::digest(bytes))
}

/// The directory a project library is installed in, under `data/hub/`.
pub fn package_dir_name(name: &str) -> String {
    format!("rwe-libraries/project.{name}")
}

/// `x.y.z` (a `-pre` part sorts below its release) as something comparable.
pub(crate) fn version_order(version: &str) -> Option<(u64, u64, u64, bool, String)> {
    let (core, pre) = match version.split_once('-') {
        Some((core, pre)) => (core, pre.to_string()),
        None => (version, String::new()),
    };
    let mut parts = core.split('.').map(|part| part.parse::<u64>().ok());
    let (a, b, c) = (parts.next()??, parts.next()??, parts.next()??);
    if parts.next().is_some() {
        return None;
    }
    Some((a, b, c, pre.is_empty(), pre))
}

/// The one version an upload carries, checked against the files it holds:
/// every listed file present with its size and digest, nothing unlisted but
/// the manifest and its notices.
fn checked_release<'a>(manifest: &'a RweLibraryManifestSpec, files: &BTreeMap<String, Vec<u8>>) -> Result<(&'a String, &'a RweLibraryVersionSpec), PlatformError> {
    if !crate::contracts::kinds::is_project_library_name(&manifest.name) {
        return Err(refuse(format!(
            "'{}' is not a project library name: one lowercase word, letters, digits and '-', never zeb…",
            manifest.name
        )));
    }
    let mut versions = manifest.versions.iter();
    let (Some((key, release)), None) = (versions.next(), versions.next()) else {
        return Err(refuse("an upload carries exactly one version"));
    };
    if version_order(key).is_none() || release.source != "offline" || release.files.is_empty() {
        return Err(refuse(format!(
            "version '{key}' must be x.y.z, source offline, and list its files (build it with zeb-bundler)"
        )));
    }
    for (path, record) in &release.files {
        let bytes = files.get(path).ok_or_else(|| refuse(format!("'{path}' is listed but not in the upload")))?;
        if bytes.len() as u64 != record.size_bytes || digest(bytes) != record.integrity {
            return Err(refuse(format!("'{path}' does not match the size and digest the manifest lists")));
        }
    }
    for path in files.keys() {
        let known = path == "manifest.json" || release.files.contains_key(path) || manifest.license.notices.contains(path);
        if !known {
            return Err(refuse(format!("'{path}' is in the upload but not listed in the manifest")));
        }
    }
    Ok((key, release))
}

/// Writes `files` into a fresh folder beside `target`, then swaps it in; the
/// old copy comes back if `commit` fails.
fn swap_in(target: &Path, files: &BTreeMap<String, Vec<u8>>, commit: impl FnOnce() -> Result<(), PlatformError>) -> Result<(), PlatformError> {
    let parent = target.parent().ok_or_else(|| refuse("no install folder"))?;
    std::fs::create_dir_all(parent).map_err(|err| PlatformError::new("PROJECT_LIBRARY_WRITE", err.to_string()))?;
    let stamp = uuid::Uuid::new_v4().simple().to_string();
    let incoming: PathBuf = parent.join(format!(".incoming-{stamp}"));
    let outgoing: PathBuf = parent.join(format!(".outgoing-{stamp}"));
    let write = || -> std::io::Result<()> {
        for (path, bytes) in files {
            let abs = incoming.join(path);
            if let Some(dir) = abs.parent() {
                std::fs::create_dir_all(dir)?;
            }
            std::fs::write(abs, bytes)?;
        }
        Ok(())
    };
    if let Err(err) = write() {
        let _ = std::fs::remove_dir_all(&incoming);
        return Err(PlatformError::new("PROJECT_LIBRARY_WRITE", err.to_string()));
    }
    let had_old = target.exists();
    if had_old {
        std::fs::rename(target, &outgoing).map_err(|err| PlatformError::new("PROJECT_LIBRARY_WRITE", err.to_string()))?;
    }
    if let Err(err) = std::fs::rename(&incoming, target) {
        if had_old {
            let _ = std::fs::rename(&outgoing, target);
        }
        let _ = std::fs::remove_dir_all(&incoming);
        return Err(PlatformError::new("PROJECT_LIBRARY_WRITE", err.to_string()));
    }
    match commit() {
        Ok(()) => {
            let _ = std::fs::remove_dir_all(&outgoing);
            Ok(())
        }
        Err(err) => {
            let _ = std::fs::remove_dir_all(target);
            if had_old {
                let _ = std::fs::rename(&outgoing, target);
            }
            Err(err)
        }
    }
}

/// Installs an uploaded library into one project: refused whole unless every
/// file is listed, present and matches; a name already installed is replaced
/// only by a higher version, or by the same version with the same files.
pub fn install_upload(platform: &PlatformService, owner: &str, project: &str, upload: &[u8]) -> Result<InstalledProjectLibrary, PlatformError> {
    if upload.len() > MAX_UPLOAD_BYTES {
        return Err(refuse(format!("the upload is larger than {} MiB", MAX_UPLOAD_BYTES / 1024 / 1024)));
    }
    let files = archive::read_tar_gz(upload)?;
    let manifest_bytes = files.get("manifest.json").ok_or_else(|| refuse("the upload has no manifest.json at its root"))?;
    let manifest = crate::contracts::decode_contract::<RweLibraryManifestContract>(manifest_bytes)
        .map_err(|err| refuse(format!("manifest.json is not a valid library manifest: {err}")))?
        .spec;
    let (version, release) = checked_release(&manifest, &files)?;
    scan::scan(&files, &manifest.license.notices)?;
    let lock = platform.dependency_lock.read(owner, project)?;
    let installed = lock.rwe.libraries.get(&manifest.name);
    if let Some(current) = installed {
        if current.source != DependencyLockSource::DirectFile {
            return Err(refuse(format!("'{}' is already a library from another source; remove it first", manifest.name)));
        }
        let (old, new) = (version_order(&current.version), version_order(version));
        let same = current.version == *version && current.integrity == release.integrity;
        if same {
            return Ok(summary(&manifest.name, version, release, false));
        }
        if !matches!((old, new), (Some(old), Some(new)) if new > old) {
            return Err(refuse(format!(
                "'{}' {} is installed; an upload replaces it only with a higher version (this is {version})",
                manifest.name, current.version
            )));
        }
    }
    let dir_name = package_dir_name(&manifest.name);
    let target = platform.dependency_lock.node_root(owner, project).join(&dir_name);
    swap_in(&target, &files, || {
        platform.dependency_lock.enable_rwe_library(
            &platform.projects.configuration_service(),
            owner,
            project,
            &manifest.name,
            DependencyLockArtifactSpec {
                version: version.clone(),
                source: DependencyLockSource::DirectFile,
                source_id: format!("upload:{}", release.recipe.clone().unwrap_or_else(|| release.integrity.clone())),
                entry: format!("{dir_name}/{}", release.entry),
                integrity: release.integrity.clone(),
            },
        )
    })?;
    Ok(summary(&manifest.name, version, release, true))
}

fn summary(name: &str, version: &str, release: &RweLibraryVersionSpec, changed: bool) -> InstalledProjectLibrary {
    InstalledProjectLibrary {
        name: name.to_string(),
        version: version.to_string(),
        entries: release.entries.keys().cloned().collect(),
        files: release.files.len(),
        bytes: release.files.values().map(|file| file.size_bytes).sum(),
        changed,
    }
}

/// The installed manifest of a project library, read from its folder.
pub fn installed_manifest(platform: &PlatformService, owner: &str, project: &str, name: &str) -> Result<Option<RweLibraryManifestSpec>, PlatformError> {
    let path = platform.dependency_lock.node_root(owner, project).join(package_dir_name(name)).join("manifest.json");
    let Ok(bytes) = std::fs::read(&path) else { return Ok(None) };
    crate::contracts::decode_contract::<RweLibraryManifestContract>(&bytes)
        .map(|document| Some(document.spec))
        .map_err(|err| PlatformError::new("PLATFORM_DEPENDENCY_INTEGRITY", format!("installed library '{name}' has an invalid manifest: {err}")))
}

/// Whether a repository file is a library recipe's own source: it lies in a
/// folder (or below one) holding a `library.yaml` of kind `LibraryRecipe`.
/// zeb-bundler compiles that code, not the page compiler, so the page checks
/// (borrowed bindings, a default export) do not apply to it.
pub fn is_recipe_source(platform: &PlatformService, owner: &str, project: &str, rel_path: &str) -> bool {
    let mut dir = Path::new(rel_path.trim_start_matches('/')).parent();
    while let Some(folder) = dir {
        let recipe = folder.join("library.yaml");
        let recipe = recipe.to_string_lossy();
        if let Ok(text) = platform.projects.read_repo_file_text(owner, project, &recipe)
            && text.lines().any(|line| line.trim() == "kind: LibraryRecipe")
        {
            return true;
        }
        dir = folder.parent().filter(|parent| !parent.as_os_str().is_empty());
    }
    false
}

/// Every entry of every library uploaded to this project, as the compiler
/// takes them: import specifier (`site-a-lib/world`) → the address its module
/// loads from (`/assets/libraries/site-a-lib/0.1.0/runtime/world.mjs`, which a
/// project page rewrites to its own host). A library whose installed copy
/// cannot be read is left out, so its import is refused by name rather than
/// loaded from nothing.
pub fn runtime_libraries(platform: &PlatformService, owner: &str, project: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let Ok(lock) = platform.dependency_lock.read(owner, project) else { return out };
    for (name, locked) in &lock.rwe.libraries {
        if locked.source != DependencyLockSource::DirectFile || !crate::contracts::kinds::is_project_library_name(name) {
            continue;
        }
        let Ok(Some(manifest)) = installed_manifest(platform, owner, project, name) else { continue };
        let Some(release) = manifest.versions.get(&locked.version) else { continue };
        for (subpath, entry) in &release.entries {
            out.insert(format!("{name}/{subpath}"), format!("/assets/libraries/{name}/{}", entry.file));
        }
    }
    out
}

/// The project's browser module folders (`rwe.browser_modules` in
/// `zebflow.yaml`), as the page compiler takes them. Read through the
/// configuration cache, so a change applies on the next compile.
pub fn browser_modules(platform: &PlatformService, owner: &str, project: &str) -> Vec<crate::rwe::core::browser_modules::ModuleFolder> {
    platform
        .zebflow_cfg
        .read_or_default(owner, project)
        .map(|config| {
            config
                .configs
                .rwe
                .browser_modules
                .iter()
                .map(|folder| crate::rwe::core::browser_modules::ModuleFolder { path: folder.path.clone(), entries: folder.entries.clone() })
                .collect()
        })
        .unwrap_or_default()
}

/// Files already checked, by path: served again without re-hashing while
/// the expected digest, the length and the modification time are unchanged.
static VERIFIED: std::sync::LazyLock<std::sync::Mutex<std::collections::HashMap<PathBuf, (String, u64, Option<std::time::SystemTime>)>>> =
    std::sync::LazyLock::new(Default::default);

/// One file of a project library, checked against its own digest before it is
/// served. `None` when `name` is not a library uploaded to this project or
/// the file is not one it lists; an error when the installed copy drifted
/// from what the lock and manifest pin.
pub fn verified_file(platform: &PlatformService, owner: &str, project: &str, name: &str, rel: &str) -> Result<Option<Vec<u8>>, PlatformError> {
    let drift = |message: String| PlatformError::new("PLATFORM_DEPENDENCY_INTEGRITY", message);
    let lock = platform.dependency_lock.read(owner, project)?;
    let Some(locked) = lock.rwe.libraries.get(name).filter(|entry| entry.source == DependencyLockSource::DirectFile) else {
        return Ok(None);
    };
    let Some(manifest) = installed_manifest(platform, owner, project, name)? else {
        return Err(drift(format!("library '{name}' is locked but its files are missing")));
    };
    // The manifest is tied to the lock: the same version, the same main entry.
    let Some(release) = manifest.versions.get(&locked.version).filter(|release| release.integrity == locked.integrity) else {
        return Err(drift(format!("library '{name}' does not match the version zeb.lock pins")));
    };
    let Some(record) = release.files.get(rel) else { return Ok(None) };
    let path = platform.dependency_lock.node_root(owner, project).join(package_dir_name(name)).join(rel);
    let metadata = std::fs::metadata(&path).map_err(|err| drift(format!("'{name}/{rel}' is missing: {err}")))?;
    let stamp = (record.integrity.clone(), metadata.len(), metadata.modified().ok());
    let bytes = std::fs::read(&path).map_err(|err| drift(format!("'{name}/{rel}' is unreadable: {err}")))?;
    let known = VERIFIED.lock().unwrap_or_else(|err| err.into_inner()).get(&path) == Some(&stamp);
    if !known {
        if bytes.len() as u64 != record.size_bytes || digest(&bytes) != record.integrity {
            return Err(drift(format!("'{name}/{rel}' does not match its digest; it was changed after upload")));
        }
        VERIFIED.lock().unwrap_or_else(|err| err.into_inner()).insert(path, stamp);
    }
    Ok(Some(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_higher_version_sorts_higher() {
        assert!(version_order("0.2.0") > version_order("0.1.9"));
        assert!(version_order("1.0.0") > version_order("1.0.0-rc.1"));
        assert!(version_order("0.1").is_none() && version_order("x.1.2").is_none());
    }
}
