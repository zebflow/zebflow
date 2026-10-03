//! Platform-owned persistence for a project's ZebFS exposure rules.
//!
//! The document lives in the project's store tier, `data/store/zebfs-acl.json`,
//! beside `addressing.json` — never inside the file backend. A bucket may be
//! written by tools outside Zebflow, and whoever writes the bucket must not be
//! able to grant exposure (`docs/contracts/kinds/zebfs-acl/README.md`).

use std::path::{Path, PathBuf};

use crate::contracts::kinds::ZebFsAclContract;
use crate::contracts::{ContractMetadata, decode_contract, encode_contract};
use crate::zebfs::acl::{ZebFsAclManifest, ZebFsAclRule};
use crate::zebfs::{ZebFsAccess, ZebFsAclScope, ZebFsError};

/// The document's name under the project's `data/store/`.
pub const ACL_DOCUMENT: &str = "zebfs-acl.json";

fn document_path(store_dir: &Path) -> PathBuf {
    store_dir.join(ACL_DOCUMENT)
}

/// The project's rules. No document means no rules: everything private. Only
/// the enveloped shape is read; anything else is refused.
pub fn read(store_dir: &Path) -> Result<ZebFsAclManifest, ZebFsError> {
    let bytes = match std::fs::read(document_path(store_dir)) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ZebFsAclManifest::default());
        }
        Err(err) => return Err(ZebFsError::new("ZEBFS_ACL_READ", err.to_string())),
    };
    let manifest = decode_contract::<ZebFsAclContract>(&bytes)
        .map_err(|err| ZebFsError::new("ZEBFS_ACL_READ", format!("{} ({})", err, err.category())))?
        .spec;
    manifest
        .validate()
        .map_err(|err| ZebFsError::new("ZEBFS_ACL_READ", err.message))?;
    Ok(manifest)
}

pub fn write(store_dir: &Path, manifest: &ZebFsAclManifest) -> Result<(), ZebFsError> {
    let bytes = encode_contract::<ZebFsAclContract>(
        ContractMetadata::named("access-control"),
        manifest.clone(),
    )
    .map_err(|err| ZebFsError::new("ZEBFS_ACL_WRITE", format!("{} ({})", err, err.category())))?;
    std::fs::create_dir_all(store_dir)
        .map_err(|err| ZebFsError::new("ZEBFS_ACL_WRITE", err.to_string()))?;
    let path = document_path(store_dir);
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, bytes).map_err(|err| ZebFsError::new("ZEBFS_ACL_WRITE", err.to_string()))?;
    std::fs::rename(&tmp, &path).map_err(|err| ZebFsError::new("ZEBFS_ACL_WRITE", err.to_string()))
}

/// How exposed `path` is. An unreadable document means private: a
/// permissions file may only fail in the safe direction.
pub fn effective_access(store_dir: &Path, path: &str) -> Result<ZebFsAccess, ZebFsError> {
    match read(store_dir) {
        Ok(manifest) => manifest.effective_access(path),
        Err(err) if err.code == "ZEBFS_ACL_READ" => Ok(ZebFsAccess::Private),
        Err(err) => Err(err),
    }
}

/// The rule deciding `path`, with the path it was set on.
pub fn effective_rule(store_dir: &Path, path: &str) -> Result<Option<(String, ZebFsAclRule)>, ZebFsError> {
    let manifest = match read(store_dir) {
        Ok(manifest) => manifest,
        Err(err) if err.code == "ZEBFS_ACL_READ" => return Ok(None),
        Err(err) => return Err(err),
    };
    Ok(manifest
        .effective_rule(path)?
        .map(|(key, rule)| (key.to_string(), rule.clone())))
}

/// Readable without sign-in on the project's file host.
pub fn is_public_read(store_dir: &Path, path: &str) -> Result<bool, ZebFsError> {
    Ok(effective_access(store_dir, path)?.is_exposed())
}

pub fn set_access(
    store_dir: &Path,
    path: &str,
    access: ZebFsAccess,
    scope: ZebFsAclScope,
    serve: &[String],
) -> Result<String, ZebFsError> {
    set_access_as(store_dir, path, access, scope, serve, None)
}

/// [`set_access`] recording the setter's project role on the rule.
pub fn set_access_as(
    store_dir: &Path,
    path: &str,
    access: ZebFsAccess,
    scope: ZebFsAclScope,
    serve: &[String],
    role: Option<&str>,
) -> Result<String, ZebFsError> {
    let mut manifest = read(store_dir)?;
    let normalized = manifest.set_rule(path, access, scope, serve)?;
    if let Some(rule) = manifest.rules.get_mut(&normalized) {
        rule.updated_by_role = role.map(str::to_string);
    }
    write(store_dir, &manifest)?;
    Ok(normalized)
}

/// The rule set on exactly `path`, if any.
pub fn own_rule(store_dir: &Path, path: &str) -> Result<Option<ZebFsAclRule>, ZebFsError> {
    let normalized = crate::zebfs::normalize_object_path(path)?;
    Ok(read(store_dir)?.rules.get(&normalized).cloned())
}

/// Whether `path` is inside a folder that runs as a site (or is one): writing
/// there changes a live site.
pub fn inside_execute(store_dir: &Path, path: &str) -> Result<bool, ZebFsError> {
    execute_relation(store_dir, path, false)
}

/// Whether `path`, anything beneath it, or anything above it runs as a site:
/// deleting or moving it changes a live site.
pub fn touches_execute(store_dir: &Path, path: &str) -> Result<bool, ZebFsError> {
    execute_relation(store_dir, path, true)
}

fn execute_relation(store_dir: &Path, path: &str, beneath_too: bool) -> Result<bool, ZebFsError> {
    let Ok(normalized) = crate::zebfs::normalize_object_path(path) else {
        return Ok(false);
    };
    let manifest = match read(store_dir) {
        Ok(manifest) => manifest,
        Err(err) if err.code == "ZEBFS_ACL_READ" => return Ok(false),
        Err(err) => return Err(err),
    };
    Ok(manifest.rules.iter().any(|(key, rule)| {
        rule.access == ZebFsAccess::PublicExecute
            && (key == &normalized
                || normalized.starts_with(&format!("{key}/"))
                || (beneath_too && key.starts_with(&format!("{normalized}/"))))
    }))
}

/// A path was deleted: its rules, and the rules beneath it, go with it.
pub fn forget(store_dir: &Path, path: &str) -> Result<usize, ZebFsError> {
    let mut manifest = read(store_dir)?;
    let removed = manifest.forget(path);
    if removed > 0 {
        write(store_dir, &manifest)?;
    }
    Ok(removed)
}

/// The project's hosts changed: origins on hosts it no longer has leave every
/// `serve`, and a rule left with none becomes private. Answers those paths.
pub fn retain_hosts(store_dir: &Path, hosts: &[String]) -> Result<Vec<String>, ZebFsError> {
    let mut manifest = read(store_dir)?;
    let before = serde_json::to_vec(&manifest.rules).unwrap_or_default();
    let demoted = manifest.retain_hosts(hosts);
    if serde_json::to_vec(&manifest.rules).unwrap_or_default() != before {
        write(store_dir, &manifest)?;
    }
    Ok(demoted)
}

/// One exposed rule and what it currently exposes.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExposedRule {
    pub path: String,
    pub access: &'static str,
    pub scope: &'static str,
    pub serve: Vec<String>,
    pub files: u64,
    pub bytes: u64,
}

/// Every exposed rule with the objects it decides today, for the Files page:
/// exposure is never quiet. An object counts under the rule that decides it,
/// so a private rule nested in a public folder takes its objects out.
pub fn exposure(store_dir: &Path, store: &crate::zebfs::ZebFs) -> Result<Vec<ExposedRule>, ZebFsError> {
    let manifest = read(store_dir)?;
    let mut out = Vec::new();
    for (path, rule) in &manifest.rules {
        if !rule.access.is_exposed() {
            continue;
        }
        let mut files = 0u64;
        let mut bytes = 0u64;
        let mut count = |key: &str, size: u64| {
            if manifest
                .effective_rule(key)
                .ok()
                .flatten()
                .is_some_and(|(decider, _)| decider == path)
            {
                files += 1;
                bytes += size;
            }
        };
        match rule.scope {
            ZebFsAclScope::Object => {
                if let Ok(stat) = store.head(path)
                    && stat.kind == crate::zebfs::ZebFsEntryKind::Object
                {
                    count(path, stat.size);
                }
            }
            ZebFsAclScope::Prefix => walk(store, path, &mut count),
        }
        out.push(ExposedRule {
            path: path.clone(),
            access: rule.access.as_str(),
            scope: rule.scope.as_str(),
            serve: rule.serve.clone(),
            files,
            bytes,
        });
    }
    Ok(out)
}

fn walk(store: &crate::zebfs::ZebFs, prefix: &str, count: &mut impl FnMut(&str, u64)) {
    let Ok(entries) = store.list(prefix) else {
        return;
    };
    for entry in entries {
        match entry.kind {
            crate::zebfs::ZebFsEntryKind::Prefix => walk(store, &entry.path, count),
            crate::zebfs::ZebFsEntryKind::Object => count(&entry.path, entry.size),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rules_roundtrip_in_the_store_tier_and_a_future_document_reads_as_private() {
        let root = tempfile::tempdir().unwrap();
        let store = root.path();
        set_access(store, "covers", ZebFsAccess::PublicRead, ZebFsAclScope::Prefix, &[]).unwrap();
        assert!(is_public_read(store, "covers/a.jpg").unwrap());
        assert!(!is_public_read(store, "uploads/a.jpg").unwrap());
        assert!(store.join(ACL_DOCUMENT).is_file());

        std::fs::write(
            store.join(ACL_DOCUMENT),
            br#"{"apiVersion":"zebflow.com/v9","kind":"ZebFsAcl","metadata":{"name":"a"},"spec":{"rules":{}}}"#,
        )
        .unwrap();
        assert_eq!(read(store).unwrap_err().code, "ZEBFS_ACL_READ");
        assert!(!is_public_read(store, "covers/a.jpg").unwrap());
    }

    /// No backward compatibility: the bare pre-contract shape is refused, and
    /// so is garbage or an envelope with an unknown field.
    #[test]
    fn only_the_enveloped_document_is_read() {
        let root = tempfile::tempdir().unwrap();
        let store = root.path();
        for bytes in [
            &br#"{"version":1,"rules":{"shared":{"access":"public_read","scope":"prefix","updated_at":1}}}"#[..],
            &br#"{"rules":{"a":{"access":"nope"}}}"#[..],
            &b"not json at all"[..],
            &br#"{"apiVersion":"zebflow.com/v1","kind":"ZebFsAcl","metadata":{"name":"a"},"spec":{"rules":{}},"extra":1}"#[..],
            &br#"{"apiVersion":"zebflow.com/v1","kind":"ZebFsAcl","metadata":{"name":"a"},"spec":{"rules":{"site":{"access":"public_execute","scope":"prefix","updated_at":1}}}}"#[..],
        ] {
            std::fs::write(store.join(ACL_DOCUMENT), bytes).unwrap();
            assert_eq!(read(store).unwrap_err().code, "ZEBFS_ACL_READ");
        }
    }

    #[test]
    fn deleting_and_host_removal_update_the_document() {
        let root = tempfile::tempdir().unwrap();
        let store = root.path();
        set_access(store, "site", ZebFsAccess::PublicExecute, ZebFsAclScope::Prefix, &["https://site.example".to_string()]).unwrap();
        set_access(store, "covers/a.jpg", ZebFsAccess::PublicRead, ZebFsAclScope::Object, &[]).unwrap();
        assert_eq!(effective_access(store, "site/index.html").unwrap(), ZebFsAccess::PublicExecute);
        assert_eq!(forget(store, "covers/a.jpg").unwrap(), 1);
        assert!(!is_public_read(store, "covers/a.jpg").unwrap());
        assert_eq!(retain_hosts(store, &["other.example".to_string()]).unwrap(), vec!["site".to_string()]);
        assert_eq!(effective_access(store, "site/index.html").unwrap(), ZebFsAccess::Private);
    }

    #[test]
    fn exposure_counts_what_each_rule_decides() {
        let root = tempfile::tempdir().unwrap();
        let files = tempfile::tempdir().unwrap();
        let store = crate::zebfs::ZebFs::Local(crate::zebfs::LocalZebFs::new(files.path().to_path_buf()));
        store.put("covers/a.jpg", b"aaaa").unwrap();
        store.put("covers/draft/b.jpg", b"bb").unwrap();
        store.put("uploads/c.jpg", b"c").unwrap();
        let dir = root.path();
        set_access(dir, "covers", ZebFsAccess::PublicRead, ZebFsAclScope::Prefix, &[]).unwrap();
        set_access(dir, "covers/draft", ZebFsAccess::Private, ZebFsAclScope::Prefix, &[]).unwrap();
        let exposed = exposure(dir, &store).unwrap();
        assert_eq!(exposed.len(), 1);
        assert_eq!((exposed[0].path.as_str(), exposed[0].files, exposed[0].bytes), ("covers", 1, 4));
    }
}
