//! ZebFS access-control metadata.
//!
//! The ACL manifest is backend-neutral: local filesystem and future object-store
//! backends can both store/read the same small JSON manifest while keeping object
//! bytes in their native storage layer.

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
}

impl ZebFsAccess {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "private" => Some(Self::Private),
            "public" | "public_read" | "public-read" => Some(Self::PublicRead),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::PublicRead => "public_read",
        }
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
        match value.trim().to_ascii_lowercase().as_str() {
            "object" | "file" => Some(Self::Object),
            "prefix" | "folder" | "directory" => Some(Self::Prefix),
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
pub struct ZebFsAclRule {
    pub access: ZebFsAccess,
    pub scope: ZebFsAclScope,
    #[serde(default)]
    pub updated_at: u64,
}

impl ZebFsAclRule {
    pub fn new(access: ZebFsAccess, scope: ZebFsAclScope) -> Self {
        Self {
            access,
            scope,
            updated_at: now_secs(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ZebFsAclManifest {
    #[serde(default = "acl_manifest_version")]
    pub version: u32,
    #[serde(default)]
    pub rules: BTreeMap<String, ZebFsAclRule>,
}

impl Default for ZebFsAclManifest {
    fn default() -> Self {
        Self {
            version: acl_manifest_version(),
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
    ) -> Result<String, ZebFsError> {
        let normalized = normalize_acl_target(path)?;
        self.rules
            .insert(normalized.clone(), ZebFsAclRule::new(access, scope));
        Ok(normalized)
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

fn acl_manifest_version() -> u32 {
    1
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
            )
            .unwrap();
        manifest
            .set_rule(
                "public-folder/private.txt",
                ZebFsAccess::Private,
                ZebFsAclScope::Object,
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
                    ZebFsAclScope::Object
                )
                .is_err()
        );
        assert!(manifest.effective_access(".zebfs/acl.json").is_err());
    }
}
