//! Overrides (`rwe-library-manifest/README.md` § Overrides): a built-in
//! `zeb/*` library replaced, in one project's pages, by an entry of a library
//! uploaded to that project.
//!
//! The replacement must export every name the built-in library exports. Both
//! are read the same way, without running either: the entry file and every
//! file its `export *` lines pass on, inside its own package. New names are
//! allowed; a missing one refuses the override and names it. Then every page
//! and component that imports the library is checked with the override in
//! place. What a name does at run time is the uploader's to keep: removing the
//! override brings the built-in library back.

use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::contracts::kinds::DependencyLockSource;
use crate::platform::error::PlatformError;
use crate::platform::model::ZebflowJsonLibraryOverride;
use crate::platform::services::PlatformService;
use crate::platform::services::library::RweLibraryVersionSpec;

/// What setting an override checked.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct OverrideReport {
    /// The built-in library replaced, e.g. `zeb/threejs`.
    pub library: String,
    /// The uploaded library and entry standing in for it.
    pub with: String,
    pub entry: String,
    pub version: String,
    /// Names the built-in library exports; the override exports all of them.
    pub built_in_names: usize,
    /// Names the override exports beyond those.
    pub added_names: usize,
    /// Pages and components that import the library, each checked.
    pub files_checked: Vec<String>,
}

fn refuse(message: impl Into<String>) -> PlatformError {
    PlatformError::new("PROJECT_LIBRARY_OVERRIDE", message)
}

/// `rel` resolved against the folder of `from`, inside one package.
fn join(from: &str, rel: &str) -> Option<String> {
    let mut parts: Vec<&str> = from.split('/').collect();
    parts.pop();
    for part in rel.split('/') {
        match part {
            "." | "" => {}
            ".." => {
                parts.pop()?;
            }
            other => parts.push(other),
        }
    }
    Some(parts.join("/"))
}

/// Every name `entry` exports, following its `export * from "./…"` lines
/// inside the package `read` gives files of.
fn names_of(entry: &str, read: &dyn Fn(&str) -> Option<String>) -> Result<BTreeSet<String>, String> {
    let (mut names, mut seen, mut todo) = (BTreeSet::new(), BTreeSet::new(), vec![entry.to_string()]);
    while let Some(path) = todo.pop() {
        if !seen.insert(path.clone()) {
            continue;
        }
        let text = read(&path).ok_or_else(|| format!("'{path}' is not in the library"))?;
        let (own, stars) = crate::rwe::core::browser_modules::export_surface(&text).map_err(|err| format!("'{path}' could not be read: {err}"))?;
        names.extend(own);
        for star in stars {
            let inside = (star.starts_with("./") || star.starts_with("../")).then(|| join(&path, &star)).flatten();
            let next = inside.ok_or_else(|| format!("'{path}' passes on every name of '{star}', which is outside the library"))?;
            todo.push(next);
        }
    }
    Ok(names)
}

/// The names a built-in library's bundle exports.
pub fn built_in_names(library: &str) -> Result<BTreeSet<String>, PlatformError> {
    let Some(url) = crate::rwe::core::render::zeb_bundle_url(library) else {
        let replaceable: Vec<&str> = crate::rwe::core::render::ZEB_BUNDLES.iter().map(|(name, _)| *name).collect();
        return Err(refuse(format!("'{library}' is not a built-in library an override can replace; those are {}", replaceable.join(", "))));
    };
    let entry = url.trim_start_matches("/assets/libraries/");
    let read = |path: &str| crate::platform::web::embedded::platform_library_asset(path).map(|bytes| String::from_utf8_lossy(bytes).into_owned());
    names_of(entry, &read).map_err(|err| PlatformError::new("PLATFORM_LIBRARY_ASSET", format!("the built-in {library} could not be read: {err}")))
}

/// The entry of `release` that stands in for `library`, and how many names it
/// adds. A named entry must have the whole interface. With none named, the one
/// entry that has it is chosen; when none does, the closest is reported, and
/// when several do, the uploader names one.
fn choose(
    library: &str,
    name: &str,
    release: &RweLibraryVersionSpec,
    entry: Option<&str>,
    read: &dyn Fn(&str) -> Option<String>,
) -> Result<(String, usize), PlatformError> {
    let built_in = built_in_names(library)?;
    let names = |entry: &str, file: &str| names_of(file, read).map_err(|err| refuse(format!("{name}/{entry}: {err}")));
    if let Some(entry) = entry {
        let spec = release.entries.get(entry).ok_or_else(|| {
            let listed: Vec<&str> = release.entries.keys().map(String::as_str).collect();
            refuse(format!("'{name}' has no entry '{entry}'; its entries are {}", listed.join(", ")))
        })?;
        let added = compare(library, &format!("{name}/{entry}"), &built_in, &names(entry, &spec.file)?)?;
        return Ok((entry.to_string(), added));
    }
    let mut fitting = Vec::new();
    let mut closest: Option<(usize, PlatformError)> = None;
    for (key, spec) in &release.entries {
        let found = match names(key, &spec.file) {
            Ok(found) => found,
            Err(err) => {
                closest.get_or_insert((usize::MAX, err));
                continue;
            }
        };
        match compare(library, &format!("{name}/{key}"), &built_in, &found) {
            Ok(added) => fitting.push((key.clone(), added)),
            Err(err) => {
                let missing = built_in.difference(&found).count();
                if closest.as_ref().is_none_or(|(least, _)| missing < *least) {
                    closest = Some((missing, err));
                }
            }
        }
    }
    match fitting.len() {
        1 => Ok(fitting.remove(0)),
        0 => Err(closest.map(|(_, err)| err).unwrap_or_else(|| refuse(format!("'{name}' has no entries")))),
        _ => {
            let keys: Vec<&str> = fitting.iter().map(|(key, _)| key.as_str()).collect();
            Err(refuse(format!("several entries of '{name}' export every name of {library}: {}; name the one that replaces it", keys.join(", "))))
        }
    }
}

/// Refuses unless `names` holds every name of `built_in`. Returns how many it adds.
fn compare(library: &str, with: &str, built_in: &BTreeSet<String>, names: &BTreeSet<String>) -> Result<usize, PlatformError> {
    let missing: Vec<&String> = built_in.difference(names).collect();
    if !missing.is_empty() {
        let shown: Vec<&str> = missing.iter().take(20).map(|name| name.as_str()).collect();
        let more = if missing.len() > shown.len() { format!(" and {} more", missing.len() - shown.len()) } else { String::new() };
        return Err(refuse(format!(
            "{with} cannot replace {library}: it does not export {} of the {} names {library} exports: {}{more}",
            missing.len(),
            built_in.len(),
            shown.join(", ")
        )));
    }
    Ok(names.difference(built_in).count())
}

/// Checks the entry of an upload that is about to be installed, from its own
/// files, before anything is written.
pub(crate) fn check_upload(
    library: &str,
    name: &str,
    release: &RweLibraryVersionSpec,
    files: &BTreeMap<String, Vec<u8>>,
    entry: Option<&str>,
) -> Result<String, PlatformError> {
    let read = |path: &str| files.get(path).map(|bytes| String::from_utf8_lossy(bytes).into_owned());
    Ok(choose(library, name, release, entry, &read)?.0)
}

/// A new version of a library that already stands in for a built-in one must
/// still do so: it keeps the entry and every name, or the upload is refused.
pub(crate) fn recheck_upload(
    platform: &PlatformService,
    owner: &str,
    project: &str,
    name: &str,
    release: &RweLibraryVersionSpec,
    files: &BTreeMap<String, Vec<u8>>,
) -> Result<(), PlatformError> {
    for (library, with) in current(platform, owner, project)?.iter().filter(|(_, with)| with.library == name) {
        check_upload(library, name, release, files, Some(&with.entry))
            .map_err(|err| refuse(format!("this version would break the override of {library}: {}", err.message)))?;
    }
    Ok(())
}

/// The project's overrides, as `zebflow.yaml` holds them.
pub fn current(platform: &PlatformService, owner: &str, project: &str) -> Result<BTreeMap<String, ZebflowJsonLibraryOverride>, PlatformError> {
    Ok(platform.zebflow_cfg.read_or_default(owner, project)?.configs.rwe.library_overrides.clone())
}

/// Makes `name` (an uploaded library, at `entry`) stand in for `library` in
/// this project's pages: refused, and nothing changed, unless it exports every
/// name `library` does and every file importing `library` still checks.
pub fn set(platform: &PlatformService, owner: &str, project: &str, library: &str, name: &str, entry: Option<&str>) -> Result<OverrideReport, PlatformError> {
    let built_in = built_in_names(library)?;
    let lock = platform.dependency_lock.read(owner, project)?;
    let Some(locked) = lock.rwe.libraries.get(name).filter(|locked| locked.source == DependencyLockSource::DirectFile) else {
        return Err(refuse(format!("'{name}' is not a library uploaded to this project; upload it first")));
    };
    let manifest = super::installed_manifest(platform, owner, project, name)?
        .ok_or_else(|| refuse(format!("'{name}' is locked but its files are missing; upload it again")))?;
    let release = manifest
        .versions
        .get(&locked.version)
        .ok_or_else(|| refuse(format!("'{name}' does not hold the version zeb.lock pins; upload it again")))?;
    let root = platform.dependency_lock.node_root(owner, project).join(super::package_dir_name(name));
    let read = |path: &str| release.files.contains_key(path).then(|| std::fs::read_to_string(root.join(path)).ok()).flatten();
    let (entry, added_names) = choose(library, name, release, entry, &read)?;

    let previous = current(platform, owner, project)?.get(library).cloned();
    let chosen = ZebflowJsonLibraryOverride { library: name.to_string(), entry: entry.clone() };
    write(platform, owner, project, library, Some(chosen))?;
    let (files_checked, failed) = check_importers(platform, owner, project, library)?;
    if !failed.is_empty() {
        write(platform, owner, project, library, previous)?;
        return Err(refuse(format!(
            "{name}/{entry} exports every name of {library}, but with it {} no longer check: {}",
            if failed.len() == 1 { "this file does" } else { "these files do" },
            failed.join("; ")
        )));
    }
    Ok(OverrideReport {
        library: library.to_string(),
        with: name.to_string(),
        entry,
        version: locked.version.clone(),
        built_in_names: built_in.len(),
        added_names,
        files_checked,
    })
}

/// Stops overriding `library`: its pages load the built-in one again.
pub fn remove(platform: &PlatformService, owner: &str, project: &str, library: &str) -> Result<bool, PlatformError> {
    let had = current(platform, owner, project)?.contains_key(library);
    if had {
        write(platform, owner, project, library, None)?;
    }
    Ok(had)
}

/// Drops every override an uploaded library stood in for, when it is removed.
pub fn forget_library(platform: &PlatformService, owner: &str, project: &str, name: &str) -> Result<Vec<String>, PlatformError> {
    let gone: Vec<String> = current(platform, owner, project)?
        .into_iter()
        .filter(|(_, with)| with.library == name)
        .map(|(library, _)| library)
        .collect();
    for library in &gone {
        write(platform, owner, project, library, None)?;
    }
    Ok(gone)
}

fn write(platform: &PlatformService, owner: &str, project: &str, library: &str, with: Option<ZebflowJsonLibraryOverride>) -> Result<(), PlatformError> {
    platform.zebflow_cfg.update(owner, project, |cfg| {
        let overrides = &mut cfg.configs.rwe.library_overrides;
        match with {
            Some(with) => {
                overrides.insert(library.to_string(), with);
            }
            None => {
                overrides.remove(library);
            }
        }
    })?;
    Ok(())
}

/// Every page and component under the source folder that imports `library`,
/// checked as Studio checks a save: the files checked, and those that failed
/// with their first problem.
fn check_importers(platform: &PlatformService, owner: &str, project: &str, library: &str) -> Result<(Vec<String>, Vec<String>), PlatformError> {
    let layout = platform.file.ensure_project_layout(owner, project)?;
    let quoted = [format!("\"{library}\""), format!("'{library}'")];
    let (mut checked, mut failed) = (Vec::new(), Vec::new());
    let mut todo = vec![layout.repo_source_dir()];
    while let Some(dir) = todo.pop() {
        let Ok(items) = std::fs::read_dir(&dir) else { continue };
        for item in items.flatten() {
            let path = item.path();
            let file_name = item.file_name().to_string_lossy().to_string();
            if file_name.starts_with('.') || file_name == "node_modules" {
                continue;
            }
            if path.is_dir() {
                todo.push(path);
                continue;
            }
            if !(file_name.ends_with(".tsx") || file_name.ends_with(".ts") || file_name.ends_with(".jsx")) {
                continue;
            }
            let Ok(content) = std::fs::read_to_string(&path) else { continue };
            if !quoted.iter().any(|q| content.contains(q.as_str())) {
                continue;
            }
            let rel = path.strip_prefix(&layout.repo_dir).unwrap_or(&path).to_string_lossy().replace('\\', "/");
            let result = crate::platform::services::template_check::check(platform, owner, project, &rel, &content)?;
            if !result.ok {
                let first = result.diagnostics.first().map(|d| d.message.clone()).unwrap_or_default();
                failed.push(format!("{rel}: {first}"));
            }
            checked.push(rel);
        }
    }
    checked.sort();
    Ok((checked, failed))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn package(files: &[(&str, &str)]) -> impl Fn(&str) -> Option<String> + use<> {
        let files: BTreeMap<String, String> = files.iter().map(|(p, t)| (p.to_string(), t.to_string())).collect();
        move |path: &str| files.get(path).cloned()
    }

    #[test]
    fn names_follow_star_exports_inside_the_package_only() {
        let read = package(&[
            ("runtime/main.mjs", "export * from './core.mjs'; export * as math from './math.mjs'; export const REVISION = '188'; export default 1;"),
            ("runtime/core.mjs", "function a(){} class B {} export { a as Scene, B as Mesh }; export * from '../shared/x.mjs';"),
            ("shared/x.mjs", "export let { Color, Vector3: V3 } = {}; export function clamp(){}"),
            ("runtime/math.mjs", "export const pi = 3;"),
        ]);
        let names = names_of("runtime/main.mjs", &read).expect("names");
        let expected = ["Color", "Mesh", "REVISION", "Scene", "V3", "clamp", "default", "math"];
        assert_eq!(names.iter().map(String::as_str).collect::<Vec<_>>(), expected);

        let outside = package(&[("main.mjs", "export * from 'three';")]);
        assert!(names_of("main.mjs", &outside).unwrap_err().contains("outside the library"));
        let missing = package(&[("main.mjs", "export * from './gone.mjs';")]);
        assert!(names_of("main.mjs", &missing).unwrap_err().contains("not in the library"));
        let cycle = package(&[("a.mjs", "export * from './b.mjs'; export const x = 1;"), ("b.mjs", "export * from './a.mjs';")]);
        assert_eq!(names_of("a.mjs", &cycle).expect("a cycle ends").len(), 1);
    }

    #[test]
    fn every_built_in_name_is_required_and_new_ones_are_allowed() {
        let built_in: BTreeSet<String> = ["Mesh", "Scene"].iter().map(|s| s.to_string()).collect();
        let more: BTreeSet<String> = ["Mesh", "Scene", "Fog"].iter().map(|s| s.to_string()).collect();
        assert_eq!(compare("zeb/threejs", "next/three", &built_in, &more).expect("a superset"), 1);
        let fewer: BTreeSet<String> = ["Scene"].iter().map(|s| s.to_string()).collect();
        let err = compare("zeb/threejs", "next/three", &built_in, &fewer).unwrap_err();
        assert_eq!(err.code, "PROJECT_LIBRARY_OVERRIDE");
        assert!(err.message.contains("does not export 1 of the 2 names") && err.message.contains("Mesh"), "{}", err.message);
    }

    #[test]
    fn every_built_in_bundle_can_be_read() {
        for (library, _) in crate::rwe::core::render::ZEB_BUNDLES {
            let names = built_in_names(library).unwrap_or_else(|err| panic!("{library}: {}", err.message));
            assert!(!names.is_empty(), "{library} exports nothing");
        }
        assert!(built_in_names("zeb/react").unwrap_err().message.contains("zeb/threejs"));
    }

    /// With no entry named, the one that has the whole interface is chosen.
    #[test]
    fn the_entry_with_the_whole_interface_is_chosen() {
        let names: Vec<String> = built_in_names("zeb/rooms").expect("names").into_iter().collect();
        let all = names.iter().filter(|n| n.as_str() != "default").map(|n| format!("export const {n} = 1;")).collect::<Vec<_>>().join("\n")
            + if names.iter().any(|n| n == "default") { "\nexport default 1;" } else { "" };
        let release = |entries: &[(&str, &str)]| -> RweLibraryVersionSpec {
            serde_json::from_value(serde_json::json!({
                "entry": "a.mjs", "source": "offline", "package_version": "1.0.0", "size_bytes": 1, "integrity": "sha256:0",
                "entries": entries.iter().map(|(key, file)| (key.to_string(), serde_json::json!({"file": file, "exports": ["x"]}))).collect::<serde_json::Map<_, _>>(),
            }))
            .expect("release")
        };
        let read = package(&[("full.mjs", all.as_str()), ("again.mjs", "export * from './full.mjs';"), ("part.mjs", "export const nothing = 1;")]);

        let one = release(&[("helpers", "part.mjs"), ("rooms", "full.mjs")]);
        assert_eq!(choose("zeb/rooms", "next", &one, None, &read).expect("chosen").0, "rooms");
        let two = release(&[("a", "full.mjs"), ("b", "again.mjs")]);
        assert!(choose("zeb/rooms", "next", &two, None, &read).unwrap_err().message.contains("several entries of 'next'"));
        assert_eq!(choose("zeb/rooms", "next", &two, Some("b"), &read).expect("named").0, "b");
        let none = release(&[("helpers", "part.mjs")]);
        assert!(choose("zeb/rooms", "next", &none, None, &read).unwrap_err().message.contains("next/helpers cannot replace zeb/rooms"));
    }

    #[test]
    fn a_path_joins_inside_its_package() {
        assert_eq!(join("runtime/main.mjs", "./core.mjs").as_deref(), Some("runtime/core.mjs"));
        assert_eq!(join("runtime/main.mjs", "../shared/x.mjs").as_deref(), Some("shared/x.mjs"));
        assert_eq!(join("main.mjs", "../x.mjs"), None);
    }
}
