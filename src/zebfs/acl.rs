//! ZebFS exposure rules (`docs/contracts/kinds/zebfs-acl/README.md`).
//!
//! Every backend stays private; this manifest is the only place a path is
//! exposed. Three levels: `private` (the default), `public_read` (readable,
//! never run as a page) and `public_execute` (served as a site, scripts
//! running, only on the origins in `serve`).

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use super::error::ZebFsError;
use super::local::normalize_object_path;

pub const ACL_MANIFEST_PATH: &str = ".zebfs/acl.json";
pub const ACL_RESERVED_PREFIX: &str = ".zebfs";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ZebFsAccess {
    Private,
    PublicRead,
    PublicExecute,
}

impl ZebFsAccess {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "private" => Some(Self::Private),
            "public_read" => Some(Self::PublicRead),
            "public_execute" => Some(Self::PublicExecute),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::PublicRead => "public_read",
            Self::PublicExecute => "public_execute",
        }
    }

    /// Readable without sign-in. A `public_execute` path is readable on the
    /// file host too, as `public_read` — sandboxed there like everything else.
    pub fn is_exposed(self) -> bool {
        !matches!(self, Self::Private)
    }
}

impl Default for ZebFsAccess {
    fn default() -> Self {
        Self::Private
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ZebFsAclScope {
    Object,
    Prefix,
}

impl ZebFsAclScope {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim() {
            "object" => Some(Self::Object),
            "prefix" => Some(Self::Prefix),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Object => "object",
            Self::Prefix => "prefix",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZebFsAclRule {
    pub access: ZebFsAccess,
    pub scope: ZebFsAclScope,
    #[serde(default)]
    pub updated_at: u64,
    /// `public_execute` only: where the path runs: a full origin, which gives
    /// the folder the whole host, or an origin and a mount path
    /// (`https://example.com/docs/`), which gives it that path on a host whose
    /// other paths stay the project's pages.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub serve: Vec<String>,
    /// The setter's project role when the rule was set (`guest` … `owner`).
    /// A rule may be changed only by that role or a higher one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub updated_by_role: Option<String>,
}

impl ZebFsAclRule {
    pub fn new(access: ZebFsAccess, scope: ZebFsAclScope, serve: Vec<String>) -> Self {
        Self {
            access,
            scope,
            updated_at: now_secs(),
            serve,
            updated_by_role: None,
        }
    }

    fn validate(&self) -> Result<(), ZebFsError> {
        if let Some(role) = &self.updated_by_role
            && !["guest", "reporter", "developer", "maintainer", "owner"].contains(&role.as_str())
        {
            return Err(ZebFsError::new(
                "ZEBFS_ACL_ROLE",
                format!("updated_by_role '{role}' is not a project role"),
            ));
        }
        match (self.access, self.serve.is_empty()) {
            (ZebFsAccess::PublicExecute, true) => Err(ZebFsError::new(
                "ZEBFS_ACL_SERVE_REQUIRED",
                "public_execute needs at least one origin in serve",
            )),
            (ZebFsAccess::PublicExecute, false) => {
                for origin in &self.serve {
                    if normalize_serve_origin(origin)? != *origin {
                        return Err(ZebFsError::new(
                            "ZEBFS_ACL_SERVE_ORIGIN",
                            format!("serve origin '{origin}' is not in its normal form"),
                        ));
                    }
                }
                Ok(())
            }
            (_, false) => Err(ZebFsError::new(
                "ZEBFS_ACL_SERVE_UNEXPECTED",
                "serve is only for public_execute",
            )),
            (_, true) => Ok(()),
        }
    }
}

/// First path segments a folder can never mount at: the platform forms every
/// project host answers whatever its routes say (`/_…` and `/.well-known/`
/// are out already, since a mount segment holds only `[a-z0-9-]`).
const RESERVED_MOUNTS: &[&str] = &["api", "assets", "mcp", "ms", "static", "wh", "ws"];

/// One `serve` entry in its normal form: `https://host/`, `http://host:port/`,
/// or either followed by a mount path, `https://host/docs/`. Scheme, lower-case
/// host, optional port, then path segments of lower-case letters, digits and
/// `-`, ending in `/`. Wildcards, queries, user info and other characters are
/// refused, and so is a mount at a reserved platform path.
pub fn normalize_serve_origin(raw: &str) -> Result<String, ZebFsError> {
    let refuse = |why: &str| {
        Err(ZebFsError::new(
            "ZEBFS_ACL_SERVE_ORIGIN",
            format!("serve address '{}' {why}", raw.trim()),
        ))
    };
    let value = raw.trim();
    let (scheme, rest) = match value.split_once("://") {
        Some((scheme, rest)) if scheme == "https" || scheme == "http" => (scheme, rest),
        _ => return refuse("must start with https:// or http://"),
    };
    let (authority, path) = match rest.find('/') {
        Some(at) => (&rest[..at], &rest[at..]),
        None => (rest, "/"),
    };
    if authority.is_empty() {
        return refuse("has no host");
    }
    if authority.contains(['?', '#', '@', '*', ' ']) || path.contains(['?', '#', '*']) {
        return refuse("must be an origin and at most a path: no query, user or wildcard");
    }
    let (host, port) = match authority.rsplit_once(':') {
        Some((host, port)) => {
            if port.is_empty() || !port.bytes().all(|b| b.is_ascii_digit()) {
                return refuse("has an invalid port");
            }
            (host, Some(port))
        }
        None => (authority, None),
    };
    if host.is_empty()
        || !host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.')
    {
        return refuse("has an invalid host");
    }
    let host = host.to_ascii_lowercase();
    // Exactly one leading and at most one trailing `/` around the segments,
    // so `//docs/` or `/a//b/` is an empty segment, never quietly `/docs/`.
    let inner = path.strip_prefix('/').unwrap_or(path);
    let inner = inner.strip_suffix('/').unwrap_or(inner);
    let segments: Vec<&str> = if inner.is_empty() { Vec::new() } else { inner.split('/').collect() };
    if segments.iter().any(|s| s.is_empty()) {
        return refuse("has an empty path segment");
    }
    if let Some(bad) = segments
        .iter()
        .find(|s| !s.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-'))
    {
        return refuse(&format!("has a path segment '{bad}': lower-case letters, digits and - only"));
    }
    if let Some(first) = segments.first()
        && RESERVED_MOUNTS.contains(first)
    {
        return refuse(&format!("mounts at /{first}/, which is the platform's on every project host"));
    }
    let mount = if segments.is_empty() { "/".to_string() } else { format!("/{}/", segments.join("/")) };
    Ok(match port {
        Some(port) => format!("{scheme}://{host}:{port}{mount}"),
        None => format!("{scheme}://{host}{mount}"),
    })
}

/// The origin of a normal-form entry: `scheme://host[:port]`, no slash.
pub fn serve_origin(entry: &str) -> &str {
    let after_scheme = entry.find("://").map(|at| at + 3).unwrap_or(0);
    match entry[after_scheme..].find('/') {
        Some(at) => &entry[..after_scheme + at],
        None => entry,
    }
}

/// Where a normal-form entry mounts its folder: `/` for the whole host, or a
/// path such as `/docs/`.
pub fn serve_mount_path(entry: &str) -> &str {
    let origin = serve_origin(entry);
    let mount = &entry[origin.len()..];
    if mount.is_empty() { "/" } else { mount }
}

/// The host of a normal-form entry, without port or path.
pub fn serve_origin_host(entry: &str) -> &str {
    let origin = serve_origin(entry);
    let authority = origin.split_once("://").map(|(_, rest)| rest).unwrap_or(origin);
    authority.rsplit_once(':').map(|(host, _)| host).unwrap_or(authority)
}

/// The port a normal-form entry names, if any.
pub fn serve_origin_port(entry: &str) -> Option<&str> {
    let origin = serve_origin(entry);
    let authority = origin.split_once("://").map(|(_, rest)| rest).unwrap_or(origin);
    authority.rsplit_once(':').map(|(_, port)| port)
}

/// Whether two mounts on one host would both claim a request: the same path,
/// one inside the other, or either the whole host.
fn mounts_overlap(a: &str, b: &str) -> bool {
    a == "/" || b == "/" || a.starts_with(b) || b.starts_with(a)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ZebFsAclManifest {
    #[serde(default)]
    pub rules: BTreeMap<String, ZebFsAclRule>,
}

impl Default for ZebFsAclManifest {
    fn default() -> Self {
        Self {
            rules: BTreeMap::new(),
        }
    }
}

impl ZebFsAclManifest {
    pub fn set_rule(
        &mut self,
        path: &str,
        access: ZebFsAccess,
        scope: ZebFsAclScope,
        serve: &[String],
    ) -> Result<String, ZebFsError> {
        let normalized = normalize_acl_target(path)?;
        let mut origins = Vec::new();
        for origin in serve {
            let origin = normalize_serve_origin(origin)?;
            if !origins.contains(&origin) {
                origins.push(origin);
            }
        }
        // One folder has one mount path: its pages link with one base.
        if let Some(first) = origins.first()
            && let Some(other) = origins.iter().find(|o| serve_mount_path(o) != serve_mount_path(first))
        {
            return Err(ZebFsError::new(
                "ZEBFS_ACL_SERVE_PATHS",
                format!(
                    "'{first}' and '{other}' mount the folder at different paths; one folder's pages link with one base"
                ),
            ));
        }
        // One address runs one folder: a second folder naming the same host
        // and an overlapping path would leave the gateway to guess which one
        // answers. A whole-host mount overlaps every path on that host.
        for origin in &origins {
            let host = serve_origin_host(origin);
            let mount = serve_mount_path(origin);
            if let Some((taken_by, taken)) = self.rules.iter().find_map(|(key, rule)| {
                if *key == normalized || rule.access != ZebFsAccess::PublicExecute {
                    return None;
                }
                rule.serve
                    .iter()
                    .find(|other| serve_origin_host(other) == host && mounts_overlap(serve_mount_path(other), mount))
                    .map(|other| (key.clone(), other.clone()))
            }) {
                return Err(ZebFsError::new(
                    "ZEBFS_ACL_SERVE_TAKEN",
                    format!("'{origin}' overlaps '{taken}', which runs '{taken_by}'; one address runs one folder"),
                ));
            }
        }
        let rule = ZebFsAclRule::new(access, scope, origins);
        rule.validate()?;
        self.rules.insert(normalized.clone(), rule);
        Ok(normalized)
    }

    /// Every rule well-formed, and the rules agree with each other as
    /// `set_rule` keeps them: one mount per rule, no overlapping mounts on one
    /// host. A document that fails is refused whole, so everything is private.
    pub fn validate(&self) -> Result<(), ZebFsError> {
        for (path, rule) in &self.rules {
            if normalize_acl_target(path)? != *path {
                return Err(ZebFsError::new(
                    "ZEBFS_ACL_PATH",
                    format!("rule key '{path}' is not a normal store path"),
                ));
            }
            rule.validate()?;
            if let Some(first) = rule.serve.first()
                && rule.serve.iter().any(|entry| serve_mount_path(entry) != serve_mount_path(first))
            {
                return Err(ZebFsError::new("ZEBFS_ACL_SERVE_PATHS", format!("'{path}' mounts at more than one path")));
            }
        }
        let entries: Vec<(&String, &String)> = self
            .rules
            .iter()
            .filter(|(_, rule)| rule.access == ZebFsAccess::PublicExecute)
            .flat_map(|(path, rule)| rule.serve.iter().map(move |entry| (path, entry)))
            .collect();
        for (i, (path, entry)) in entries.iter().enumerate() {
            for (other_path, other) in &entries[i + 1..] {
                if path != other_path
                    && serve_origin_host(entry) == serve_origin_host(other)
                    && mounts_overlap(serve_mount_path(entry), serve_mount_path(other))
                {
                    return Err(ZebFsError::new(
                        "ZEBFS_ACL_SERVE_TAKEN",
                        format!("'{entry}' of '{path}' overlaps '{other}' of '{other_path}'"),
                    ));
                }
            }
        }
        Ok(())
    }

    /// Deleting a path deletes the rules on it and beneath it, so a new object
    /// of the same name starts private. Answers how many rules went.
    pub fn forget(&mut self, path: &str) -> usize {
        let Ok(normalized) = normalize_object_path(path) else {
            return 0;
        };
        let before = self.rules.len();
        self.rules.retain(|key, _| {
            !(key == &normalized
                || key
                    .strip_prefix(&normalized)
                    .is_some_and(|tail| tail.starts_with('/')))
        });
        before - self.rules.len()
    }

    /// Removes every `serve` entry whose host the project no longer has: a
    /// whole-host entry needs a site host, an entry with a mount path a site
    /// host or a section host (a section host's other paths stay its
    /// section's). A rule left with none becomes private. Answers the paths
    /// that became private.
    pub fn retain_hosts(&mut self, site_hosts: &[String], section_hosts: &[String]) -> Vec<String> {
        let mut demoted = Vec::new();
        for (path, rule) in self.rules.iter_mut() {
            if rule.access != ZebFsAccess::PublicExecute {
                continue;
            }
            rule.serve.retain(|entry| {
                let host = serve_origin_host(entry);
                site_hosts.iter().any(|known| known == host)
                    || (serve_mount_path(entry) != "/" && section_hosts.iter().any(|known| known == host))
            });
            if rule.serve.is_empty() {
                rule.access = ZebFsAccess::Private;
                rule.updated_at = now_secs();
                demoted.push(path.clone());
            }
        }
        demoted
    }

    /// The rule that decides `path`: longest match, whole segments.
    pub fn effective_rule(&self, path: &str) -> Result<Option<(&str, &ZebFsAclRule)>, ZebFsError> {
        let normalized = normalize_acl_target(path)?;
        let mut best: Option<(&str, &ZebFsAclRule)> = None;
        for (rule_path, rule) in &self.rules {
            let matches = match rule.scope {
                ZebFsAclScope::Object => normalized == *rule_path,
                ZebFsAclScope::Prefix => {
                    normalized == *rule_path
                        || normalized
                            .strip_prefix(rule_path.as_str())
                            .map(|tail| tail.starts_with('/'))
                            .unwrap_or(false)
                }
            };
            if matches && best.is_none_or(|(best_path, _)| rule_path.len() >= best_path.len()) {
                best = Some((rule_path.as_str(), rule));
            }
        }
        Ok(best)
    }

    pub fn effective_access(&self, path: &str) -> Result<ZebFsAccess, ZebFsError> {
        let normalized = normalize_acl_target(path)?;
        let mut best: Option<(usize, ZebFsAccess)> = None;
        for (rule_path, rule) in &self.rules {
            let matches = match rule.scope {
                ZebFsAclScope::Object => normalized == *rule_path,
                ZebFsAclScope::Prefix => {
                    normalized == *rule_path
                        || normalized
                            .strip_prefix(rule_path)
                            .map(|tail| tail.starts_with('/'))
                            .unwrap_or(false)
                }
            };
            if matches {
                let score = rule_path.len();
                if best
                    .map(|(best_score, _)| score >= best_score)
                    .unwrap_or(true)
                {
                    best = Some((score, rule.access));
                }
            }
        }
        Ok(best.map(|(_, access)| access).unwrap_or_default())
    }
}

pub fn is_reserved_acl_path(path: &str) -> bool {
    path == ACL_RESERVED_PREFIX
        || path
            .strip_prefix(ACL_RESERVED_PREFIX)
            .map(|tail| tail.starts_with('/'))
            .unwrap_or(false)
}

fn normalize_acl_target(path: &str) -> Result<String, ZebFsError> {
    let normalized = normalize_object_path(path)?;
    if is_reserved_acl_path(&normalized) {
        return Err(ZebFsError::new(
            "ZEBFS_RESERVED_PATH",
            "path is reserved for ZebFS metadata",
        ));
    }
    Ok(normalized)
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|value| value.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn acl_defaults_to_private_and_supports_object_rules() {
        let mut manifest = ZebFsAclManifest::default();
        assert_eq!(
            manifest.effective_access("uploads/a.txt").unwrap(),
            ZebFsAccess::Private
        );

        manifest
            .set_rule(
                "uploads/a.txt",
                ZebFsAccess::PublicRead,
                ZebFsAclScope::Object,
                &[],
            )
            .unwrap();

        assert_eq!(
            manifest.effective_access("uploads/a.txt").unwrap(),
            ZebFsAccess::PublicRead
        );
        assert_eq!(
            manifest.effective_access("uploads/b.txt").unwrap(),
            ZebFsAccess::Private
        );
    }

    #[test]
    fn prefix_rules_apply_to_descendants_and_specific_rules_win() {
        let mut manifest = ZebFsAclManifest::default();
        manifest
            .set_rule(
                "public-folder",
                ZebFsAccess::PublicRead,
                ZebFsAclScope::Prefix,
                &[],
            )
            .unwrap();
        manifest
            .set_rule(
                "public-folder/private.txt",
                ZebFsAccess::Private,
                ZebFsAclScope::Object,
                &[],
            )
            .unwrap();

        assert_eq!(
            manifest.effective_access("public-folder").unwrap(),
            ZebFsAccess::PublicRead
        );
        assert_eq!(
            manifest
                .effective_access("public-folder/nested/a.txt")
                .unwrap(),
            ZebFsAccess::PublicRead
        );
        assert_eq!(
            manifest
                .effective_access("public-folder/private.txt")
                .unwrap(),
            ZebFsAccess::Private
        );
        assert_eq!(
            manifest.effective_access("public-folderish/a.txt").unwrap(),
            ZebFsAccess::Private
        );
    }

    #[test]
    fn acl_reserved_metadata_path_is_rejected() {
        let mut manifest = ZebFsAclManifest::default();
        assert!(
            manifest
                .set_rule(
                    ".zebfs/acl.json",
                    ZebFsAccess::PublicRead,
                    ZebFsAclScope::Object,
                    &[]
                )
                .is_err()
        );
        assert!(manifest.effective_access(".zebfs/acl.json").is_err());
    }

    #[test]
    fn public_execute_needs_normal_origins_and_serve_is_only_for_it() {
        let mut manifest = ZebFsAclManifest::default();
        let serve = vec!["https://Site.Example".to_string(), "https://site.example/".to_string()];
        manifest
            .set_rule("site", ZebFsAccess::PublicExecute, ZebFsAclScope::Prefix, &serve)
            .unwrap();
        assert_eq!(manifest.rules["site"].serve, vec!["https://site.example/".to_string()]);
        assert!(manifest.set_rule("x", ZebFsAccess::PublicExecute, ZebFsAclScope::Prefix, &[]).is_err());
        assert!(manifest
            .set_rule("x", ZebFsAccess::PublicRead, ZebFsAclScope::Prefix, &serve)
            .is_err());
        for bad in ["*.site.example", "https://*.site.example/", "ftp://site.example", "https://u@site.example", "https://site.example/Docs/", "https://site.example/a//b/", "https://site.example//docs/", "https://site.example/a?b", "https://site.example/api/", "https://site.example/static/x/", "https://site.example/.well-known/", "https://site.example/_x/", "https://site.example/a%2fb/"] {
            assert!(normalize_serve_origin(bad).is_err(), "{bad} must be refused");
        }
        assert_eq!(normalize_serve_origin("http://dev.site.localhost:10610").unwrap(), "http://dev.site.localhost:10610/");
        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn a_folder_can_mount_at_a_path_and_mounts_on_one_host_never_overlap() {
        assert_eq!(normalize_serve_origin("https://Site.Example/docs").unwrap(), "https://site.example/docs/");
        assert_eq!(normalize_serve_origin("http://d.localhost:10610/learn/v1/").unwrap(), "http://d.localhost:10610/learn/v1/");
        let entry = "http://d.localhost:10610/learn/v1/";
        assert_eq!(serve_origin(entry), "http://d.localhost:10610");
        assert_eq!(serve_mount_path(entry), "/learn/v1/");
        assert_eq!(serve_origin_host(entry), "d.localhost");
        assert_eq!(serve_origin_port(entry), Some("10610"));
        assert_eq!(serve_mount_path("https://site.example/"), "/");

        let mut manifest = ZebFsAclManifest::default();
        let docs = vec!["https://site.example/docs/".to_string()];
        manifest.set_rule("docs-out", ZebFsAccess::PublicExecute, ZebFsAclScope::Prefix, &docs).unwrap();
        // Another path on the same host is fine; an overlapping one, or the whole host, is not.
        manifest
            .set_rule("learn-out", ZebFsAccess::PublicExecute, ZebFsAclScope::Prefix, &["https://site.example/learn/".to_string()])
            .unwrap();
        for taken in ["https://site.example/docs/", "https://site.example/docs/v1/", "https://site.example/"] {
            let err = manifest
                .set_rule("other", ZebFsAccess::PublicExecute, ZebFsAclScope::Prefix, &[taken.to_string()])
                .unwrap_err();
            assert_eq!(err.code, "ZEBFS_ACL_SERVE_TAKEN", "{taken}");
        }
        // One folder, one base path.
        let mixed = vec!["https://a.example/docs/".to_string(), "https://b.example/".to_string()];
        assert_eq!(
            manifest.set_rule("mixed", ZebFsAccess::PublicExecute, ZebFsAclScope::Prefix, &mixed).unwrap_err().code,
            "ZEBFS_ACL_SERVE_PATHS"
        );
        // Removing the host removes its mounts too.
        // A mount may stand on a section host; the whole host may not.
        assert!(manifest.retain_hosts(&[], &["site.example".to_string()]).is_empty());
        assert_eq!(manifest.retain_hosts(&[], &[]), vec!["docs-out".to_string(), "learn-out".to_string()]);
        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn deleting_a_path_forgets_its_rules_and_a_removed_host_demotes() {
        let mut manifest = ZebFsAclManifest::default();
        manifest.set_rule("covers/a.jpg", ZebFsAccess::PublicRead, ZebFsAclScope::Object, &[]).unwrap();
        manifest.set_rule("covers/old", ZebFsAccess::PublicRead, ZebFsAclScope::Prefix, &[]).unwrap();
        manifest.set_rule("coversx", ZebFsAccess::PublicRead, ZebFsAclScope::Prefix, &[]).unwrap();
        assert_eq!(manifest.forget("covers"), 2);
        assert!(manifest.rules.contains_key("coversx"));

        let serve = vec!["https://a.example/".to_string(), "https://b.example/".to_string()];
        manifest.set_rule("site", ZebFsAccess::PublicExecute, ZebFsAclScope::Prefix, &serve).unwrap();
        assert!(manifest.retain_hosts(&["b.example".to_string()], &[]).is_empty());
        assert_eq!(manifest.rules["site"].serve, vec!["https://b.example/".to_string()]);
        assert_eq!(manifest.retain_hosts(&[], &["a.example".to_string()]), vec!["site".to_string()], "a whole-host entry needs a site host");
        assert_eq!(manifest.rules["site"].access, ZebFsAccess::Private);
        assert!(manifest.validate().is_ok());
    }

    #[test]
    fn old_aliases_are_refused() {
        for alias in ["public", "public-read", "PUBLIC_READ"] {
            assert!(ZebFsAccess::parse(alias).is_none(), "{alias}");
        }
        for alias in ["file", "folder", "directory"] {
            assert!(ZebFsAclScope::parse(alias).is_none(), "{alias}");
        }
    }

    #[test]
    fn one_address_runs_one_folder() {
        let mut manifest = ZebFsAclManifest::default();
        let serve = vec!["https://site.example/".to_string()];
        manifest.set_rule("site", ZebFsAccess::PublicExecute, ZebFsAclScope::Prefix, &serve).unwrap();
        let err = manifest
            .set_rule("other", ZebFsAccess::PublicExecute, ZebFsAclScope::Prefix, &serve)
            .unwrap_err();
        assert_eq!(err.code, "ZEBFS_ACL_SERVE_TAKEN");
        // Re-setting the same folder is fine.
        manifest.set_rule("site", ZebFsAccess::PublicExecute, ZebFsAclScope::Prefix, &serve).unwrap();
    }
}
