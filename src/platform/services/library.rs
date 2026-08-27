//! In-memory registry of embedded `zeb/*` frontend library manifests.
//!
//! Built once at startup from [`PLATFORM_LIBRARY_ASSETS`]. Each `manifest.json`
//! embedded in the binary is parsed into a [`RweLibraryManifest`] and stored in
//! insertion order for stable listing.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::contracts::decode_contract;
use crate::contracts::kinds::RweLibraryManifestContract;
use crate::platform::error::PlatformError;
use crate::platform::web::embedded::PLATFORM_LIBRARY_ASSETS;

/// Strict source form stored in each library `manifest.json` contract.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RweLibraryManifestSpec {
    pub name: String,
    pub description: String,
    pub exports: Vec<String>,
    pub versions: BTreeMap<String, RweLibraryVersionSpec>,
}

/// Strict source form for one library release.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RweLibraryVersionSpec {
    pub entry: String,
    pub source: String,
    pub package_version: String,
    pub size_bytes: u64,
    pub integrity: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

/// One available version entry from a library manifest.
#[derive(Debug, Clone)]
pub struct LibraryVersion {
    /// Version key, e.g. `"r183"`.
    pub key: String,
    /// Relative path to the bundle within the library dir, e.g. `"r183/bundle.min.mjs"`.
    pub entry: String,
    /// `"offline"` (bytes embedded in this binary) or `"hub"` (installed copy).
    pub source: String,
    /// Upstream package version, e.g. `"0.183.2"`.
    pub package_version: String,
    /// Bundle size in bytes.
    pub size_bytes: u64,
    /// sha256 integrity hash of the bundle file.
    pub integrity: String,
}

/// Parsed manifest for one embedded `zeb/*` library.
#[derive(Debug, Clone)]
pub struct RweLibraryManifest {
    /// Stable library name, e.g. `"zeb/threejs"`.
    pub name: String,
    /// Human-readable description shown in the settings UI.
    pub description: String,
    /// Exported symbol names (used by the RWE compiler for import resolution).
    pub exports: Vec<String>,
    /// All available versions, in manifest order.
    pub versions: Vec<LibraryVersion>,
}

impl RweLibraryManifest {
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

    /// `packed_kind` — `"full"` if an offline bundle is embedded in this
    /// binary, else `"hub"` (only an installed copy can serve it).
    pub fn packed_kind(&self) -> &str {
        if self.versions.iter().any(|v| v.source == "offline") {
            "full"
        } else {
            "hub"
        }
    }
}

/// In-memory ordered registry of all embedded library manifests.
pub struct LibraryService {
    manifests: Vec<RweLibraryManifest>,
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
                decode_contract::<RweLibraryManifestContract>(asset.bytes).map_err(|err| {
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
                })
                .collect();

            manifests.push(RweLibraryManifest {
                name: spec.name,
                description: spec.description,
                exports: spec.exports,
                versions,
            });
        }
        Ok(Self { manifests })
    }

    /// Returns an iterator over all registered manifests in insertion order.
    pub fn list(&self) -> impl Iterator<Item = &RweLibraryManifest> {
        self.manifests.iter()
    }

    /// Returns the manifest for one library by name, or `None` if not registered.
    pub fn get(&self, name: &str) -> Option<&RweLibraryManifest> {
        self.manifests.iter().find(|m| m.name == name)
    }

    /// Returns true if a library exports a given symbol name.
    pub fn find_library_for_symbol(&self, symbol: &str) -> Option<&RweLibraryManifest> {
        self.manifests
            .iter()
            .find(|m| m.exports.iter().any(|e| e == symbol))
    }
}

#[cfg(test)]
mod tests {
    use sha2::{Digest, Sha256};

    use super::*;
    use crate::platform::web::embedded::platform_library_asset;

    #[test]
    fn every_embedded_library_uses_the_canonical_contract() {
        let service = LibraryService::from_embedded().expect("embedded library contracts");
        assert_eq!(service.list().count(), 12);
        assert!(service.get("zeb/deckgl").is_some());
    }

    #[test]
    fn legacy_library_manifest_shape_is_rejected() {
        let raw = br#"{"name":"zeb/example","description":"example","exports":[],"versions":{}}"#;
        let err = decode_contract::<RweLibraryManifestContract>(raw).unwrap_err();
        assert_eq!(err.category(), "invalid");
    }

    /// The checked-in integrity and size values are honest: every embedded
    /// manifest's declared digest and decoded size are recomputed here from
    /// the bytes this binary actually embeds.
    #[test]
    fn every_embedded_manifest_declares_the_digest_and_size_of_its_real_bytes() {
        let service = LibraryService::from_embedded().expect("embedded library contracts");
        let mut releases = 0usize;
        for manifest in service.list() {
            for version in &manifest.versions {
                let asset_path = format!("{}/{}", manifest.name, version.entry);
                let bytes = platform_library_asset(&asset_path)
                    .unwrap_or_else(|| panic!("embedded bundle '{asset_path}' exists"));
                assert_eq!(
                    version.integrity,
                    format!("sha256:{:x}", Sha256::digest(bytes)),
                    "{} '{}' integrity matches its embedded bytes",
                    manifest.name,
                    version.key
                );
                assert_eq!(
                    version.size_bytes,
                    bytes.len() as u64,
                    "{} '{}' size_bytes matches its embedded bytes",
                    manifest.name,
                    version.key
                );
                releases += 1;
            }
        }
        assert!(releases >= 12, "every library carries at least one release");
    }

    /// The blessed documents are canonical: decoding one and re-encoding it
    /// through the one canonical writer reproduces the file byte for byte.
    #[test]
    fn every_embedded_manifest_is_canonical_byte_for_byte() {
        for asset in PLATFORM_LIBRARY_ASSETS {
            if !asset.path.ends_with("/manifest.json") {
                continue;
            }
            let document = decode_contract::<RweLibraryManifestContract>(asset.bytes)
                .unwrap_or_else(|err| panic!("'{}' decodes: {err}", asset.path));
            let encoded = crate::contracts::encode_contract::<RweLibraryManifestContract>(
                document.metadata,
                document.spec,
            )
            .expect("re-encode");
            assert_eq!(
                encoded, asset.bytes,
                "'{}' is written in canonical form",
                asset.path
            );
        }
    }
}
