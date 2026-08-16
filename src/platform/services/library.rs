//! In-memory registry of embedded `zeb/*` frontend library manifests.
//!
//! Built once at startup from [`PLATFORM_LIBRARY_ASSETS`]. Each `manifest.json`
//! embedded in the binary is parsed into a [`LibraryManifest`] and stored in
//! insertion order for stable listing.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::contracts::decode_contract;
use crate::contracts::kinds::LibraryManifestContract;
use crate::contracts::kinds::{DependencyLockArtifactSpec, DependencyLockSource};
use crate::platform::error::PlatformError;
use crate::platform::web::embedded::{PLATFORM_LIBRARY_ASSETS, platform_library_asset};

/// Strict source form stored in each library `manifest.json` contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibraryManifestSpec {
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub exports: Vec<String>,
    pub versions: BTreeMap<String, LibraryVersionSpec>,
}

/// Strict source form for one library release.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LibraryVersionSpec {
    pub entry: String,
    pub source: String,
    pub package_version: String,
    pub size_bytes: u64,
    #[serde(default)]
    pub size_gzip_bytes: Option<u64>,
    #[serde(default)]
    pub integrity: String,
    #[serde(default)]
    pub registry_url: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

/// One available version entry from a library manifest.
#[derive(Debug, Clone)]
pub struct LibraryVersion {
    /// Version key, e.g. `"r183"`.
    pub key: String,
    /// Relative path to the bundle within the library dir, e.g. `"r183/bundle.min.mjs"`.
    pub entry: String,
    /// `"offline"` (embedded in binary) or `"online"` (download required).
    pub source: String,
    /// Upstream package version, e.g. `"0.183.2"`.
    pub package_version: String,
    /// Bundle size in bytes.
    pub size_bytes: u64,
    /// sha256 integrity hash of the bundle file.
    pub integrity: String,
    /// Download URL for online versions.
    pub registry_url: Option<String>,
}

/// Parsed manifest for one embedded `zeb/*` library.
#[derive(Debug, Clone)]
pub struct LibraryManifest {
    /// Stable library name, e.g. `"zeb/threejs"`.
    pub name: String,
    /// Human-readable description shown in the settings UI.
    pub description: String,
    /// Exported symbol names (used by the RWE compiler for import resolution).
    pub exports: Vec<String>,
    /// All available versions, in manifest order.
    pub versions: Vec<LibraryVersion>,
}

impl LibraryManifest {
    /// Returns the default offline version, if any (first version with source == "offline").
    pub fn default_offline_version(&self) -> Option<&LibraryVersion> {
        self.versions.iter().find(|v| v.source == "offline")
    }

    /// Returns a version by key.
    pub fn version(&self, key: &str) -> Option<&LibraryVersion> {
        self.versions.iter().find(|v| v.key == key)
    }

    /// `packed_version` — key of the first offline version, or first version key.
    pub fn packed_version(&self) -> &str {
        self.default_offline_version()
            .or_else(|| self.versions.first())
            .map(|v| v.key.as_str())
            .unwrap_or("unknown")
    }

    /// `packed_kind` — `"full"` if offline bundle exists, else `"online"`.
    pub fn packed_kind(&self) -> &str {
        if self.versions.iter().any(|v| v.source == "offline") {
            "full"
        } else {
            "online"
        }
    }
}

/// In-memory ordered registry of all embedded library manifests.
pub struct LibraryService {
    manifests: Vec<LibraryManifest>,
}

impl LibraryService {
    /// Scans [`PLATFORM_LIBRARY_ASSETS`] for `manifest.json` files and builds
    /// the registry. Called once at platform startup.
    pub fn from_embedded() -> Result<Self, PlatformError> {
        let mut manifests = Vec::new();
        for asset in PLATFORM_LIBRARY_ASSETS {
            if !asset.path.ends_with("/manifest.json") {
                continue;
            }
            let document =
                decode_contract::<LibraryManifestContract>(asset.bytes).map_err(|err| {
                    PlatformError::new(
                        "PLATFORM_LIBRARY_MANIFEST",
                        format!("invalid embedded library manifest '{}': {err}", asset.path),
                    )
                })?;
            let spec = document.spec;
            let versions = spec
                .versions
                .into_iter()
                .map(|(key, version)| LibraryVersion {
                    key,
                    entry: version.entry,
                    source: version.source,
                    package_version: version.package_version,
                    size_bytes: version.size_bytes,
                    integrity: version.integrity,
                    registry_url: version.registry_url,
                })
                .collect();

            manifests.push(LibraryManifest {
                name: spec.name,
                description: spec.description,
                exports: spec.exports,
                versions,
            });
        }
        Ok(Self { manifests })
    }

    /// Returns an iterator over all registered manifests in insertion order.
    pub fn list(&self) -> impl Iterator<Item = &LibraryManifest> {
        self.manifests.iter()
    }

    /// Returns the manifest for one library by name, or `None` if not registered.
    pub fn get(&self, name: &str) -> Option<&LibraryManifest> {
        self.manifests.iter().find(|m| m.name == name)
    }

    /// Returns true if a library exports a given symbol name.
    pub fn find_library_for_symbol(&self, symbol: &str) -> Option<&LibraryManifest> {
        self.manifests
            .iter()
            .find(|m| m.exports.iter().any(|e| e == symbol))
    }

    /// Resolves one exact library release into a verified dependency-lock entry.
    ///
    /// Offline releases are hashed from the bytes embedded in this binary. An
    /// optional manifest digest must match those bytes. Online releases cannot
    /// be pinned unless their manifest already provides a valid digest.
    pub fn resolve_lock_entry(
        &self,
        name: &str,
        version: &str,
        requested_source: &str,
    ) -> Result<DependencyLockArtifactSpec, PlatformError> {
        let manifest = self.get(name).ok_or_else(|| {
            PlatformError::new(
                "PLATFORM_LIBRARY_NOT_FOUND",
                format!("library '{name}' is not registered"),
            )
        })?;
        let resolved = manifest.version(version).ok_or_else(|| {
            PlatformError::new(
                "PLATFORM_LIBRARY_VERSION_NOT_FOUND",
                format!("library '{name}' has no version '{version}'"),
            )
        })?;
        if requested_source != resolved.source {
            return Err(PlatformError::new(
                "PLATFORM_LIBRARY_SOURCE_MISMATCH",
                format!(
                    "library '{name}' version '{version}' uses source '{}', not '{requested_source}'",
                    resolved.source
                ),
            ));
        }

        let integrity = if resolved.source == "offline" {
            let asset_path = format!("{name}/{}", resolved.entry);
            let bytes = platform_library_asset(&asset_path).ok_or_else(|| {
                PlatformError::new(
                    "PLATFORM_LIBRARY_ASSET_MISSING",
                    format!("embedded library asset '{asset_path}' is missing"),
                )
            })?;
            let computed = format!("sha256:{:x}", Sha256::digest(bytes));
            if !resolved.integrity.is_empty() && resolved.integrity != computed {
                return Err(PlatformError::new(
                    "PLATFORM_LIBRARY_INTEGRITY",
                    format!(
                        "library '{name}' version '{version}' manifest digest does not match its embedded bytes"
                    ),
                ));
            }
            computed
        } else {
            validate_sha256(&resolved.integrity).map_err(|message| {
                PlatformError::new(
                    "PLATFORM_LIBRARY_INTEGRITY",
                    format!("library '{name}' version '{version}' {message}"),
                )
            })?;
            resolved.integrity.clone()
        };

        let source = match resolved.source.as_str() {
            "offline" => DependencyLockSource::Embedded,
            unsupported => {
                return Err(PlatformError::new(
                    "PLATFORM_LIBRARY_SOURCE_UNSUPPORTED",
                    format!(
                        "library '{name}' version '{version}' uses unsupported durable source '{unsupported}'"
                    ),
                ));
            }
        };

        Ok(DependencyLockArtifactSpec {
            version: resolved.key.clone(),
            source,
            source_id: format!("zebflow/{name}"),
            entry: resolved.entry.clone(),
            integrity,
        })
    }
}

fn validate_sha256(value: &str) -> Result<(), &'static str> {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return Err("must declare integrity as sha256:<hex>");
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("must declare exactly 64 lowercase hexadecimal digest characters");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_embedded_library_uses_the_canonical_contract() {
        let service = LibraryService::from_embedded().expect("embedded library contracts");
        assert_eq!(service.list().count(), 12);
        assert!(service.get("zeb/deckgl").is_some());
    }

    #[test]
    fn legacy_library_manifest_shape_is_rejected() {
        let raw = br#"{"name":"zeb/example","description":"example","exports":[],"versions":{}}"#;
        let err = decode_contract::<LibraryManifestContract>(raw).unwrap_err();
        assert_eq!(err.category(), "invalid");
    }

    #[test]
    fn offline_lock_entries_use_the_digest_of_embedded_bytes() {
        let service = LibraryService::from_embedded().expect("embedded library contracts");
        let entry = service
            .resolve_lock_entry("zeb/deckgl", "full-9.x", "offline")
            .expect("resolved offline release");
        let bytes = platform_library_asset("zeb/deckgl/0.1/runtime/deckgl.bundle.mjs")
            .expect("embedded deckgl bundle");
        assert_eq!(
            entry.integrity,
            format!("sha256:{:x}", Sha256::digest(bytes))
        );
    }

    #[test]
    fn lock_resolution_rejects_source_substitution() {
        let service = LibraryService::from_embedded().expect("embedded library contracts");
        let error = service
            .resolve_lock_entry("zeb/deckgl", "full-9.x", "online")
            .unwrap_err();
        assert_eq!(error.code, "PLATFORM_LIBRARY_SOURCE_MISMATCH");
    }
}
