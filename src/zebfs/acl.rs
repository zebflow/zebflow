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
    /// `public_execute` only: the full origins the path runs on.
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

/// One `serve` origin in its normal form: `https://host/` or
/// `http://host:port/` — scheme, lower-case host, optional port, nothing else.
/// Wildcards, paths, queries and user info are refused.
pub fn normalize_serve_origin(raw: &str) -> Result<String, ZebFsError> {
    let refuse = |why: &str| {
        Err(ZebFsError::new(
            "ZEBFS_ACL_SERVE_ORIGIN",
            format!("serve origin '{}' {why}", raw.trim()),
        ))
    };
    let value = raw.trim();
    let (scheme, rest) = match value.split_once("://") {
        Some((scheme, rest)) if scheme == "https" || scheme == "http" => (scheme, rest),
        _ => return refuse("must start with https:// or http://"),
    };
    let authority = rest.strip_suffix('/').unwrap_or(rest);
    if authority.is_empty() {
        return refuse("has no host");
    }
    if authority.contains(['/', '?', '#', '@', '*', ' ']) {
        return refuse("must be a bare origin: no path, query, user or wildcard");
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
    Ok(match port {
        Some(port) => format!("{scheme}://{host}:{port}/"),
        None => format!("{scheme}://{host}/"),
    })
}

/// The host of a normal-form origin, without port.
pub fn serve_origin_host(origin: &str) -> &str {
    let rest = origin.split_once("://").map(|(_, rest)| rest).unwrap_or(origin);
    let authority = rest.trim_end_matches('/');
    authority.rsplit_once(':').map(|(host, _)| host).unwrap_or(authority)
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
        // One address runs one folder: a second folder naming the same host
        // would leave the gateway to guess which one answers.
        for origin in &origins {
            let host = serve_origin_host(origin);
            if let Some((taken_by, _)) = self.rules.iter().find(|(key, rule)| {
                **key != normalized
                    && rule.access == ZebFsAccess::PublicExecute
                    && rule.serve.iter().any(|other| serve_origin_host(other) == host)
            }) {
                return Err(ZebFsError::new(
                    "ZEBFS_ACL_SERVE_TAKEN",
                    format!("'{host}' already runs '{taken_by}'; one address runs one folder"),
                ));
            }
        }
        let rule = ZebFsAclRule::new(access, scope, origins);
        rule.validate()?;
        self.rules.insert(normalized.clone(), rule);
        Ok(normalized)
    }

    /// Every rule well-formed. A document that fails is refused whole.
    pub fn validate(&self) -> Result<(), ZebFsError> {
        for (path, rule) in &self.rules {
            if normalize_acl_target(path)? != *path {
                return Err(ZebFsError::new(
                    "ZEBFS_ACL_PATH",
                    format!("rule key '{path}' is not a normal store path"),
                ));
            }
            rule.validate()?;
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

    /// Removes every `serve` origin whose host is not in `hosts`. A rule left
    /// with none becomes private. Answers the paths that became private.
    pub fn retain_hosts(&mut self, hosts: &[String]) -> Vec<String> {
        let mut demoted = Vec::new();
        for (path, rule) in self.rules.iter_mut() {
            if rule.access != ZebFsAccess::PublicExecute {
                continue;
            }
            rule.serve
                .retain(|origin| hosts.iter().any(|host| host == serve_origin_host(origin)));
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
        for bad in ["*.site.example", "https://*.site.example/", "https://site.example/path", "ftp://site.example", "https://u@site.example"] {
            assert!(normalize_serve_origin(bad).is_err(), "{bad} must be refused");
        }
        assert_eq!(normalize_serve_origin("http://dev.site.localhost:10610").unwrap(), "http://dev.site.localhost:10610/");
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
        assert!(manifest.retain_hosts(&["b.example".to_string()]).is_empty());
        assert_eq!(manifest.rules["site"].serve, vec!["https://b.example/".to_string()]);
        assert_eq!(manifest.retain_hosts(&[]), vec!["site".to_string()]);
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
