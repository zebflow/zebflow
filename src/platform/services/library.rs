//! In-memory registry of embedded `zeb/*` frontend library manifests.
//!
//! Built once at startup from [`PLATFORM_LIBRARY_ASSETS`]. Each `manifest.json`
//! embedded in the binary is parsed into a [`LibraryManifest`] and stored in
//! insertion order for stable listing.

use crate::platform::web::embedded::PLATFORM_LIBRARY_ASSETS;

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
    pub fn from_embedded() -> Self {
        let mut manifests = Vec::new();
        for asset in PLATFORM_LIBRARY_ASSETS {
            if !asset.path.ends_with("/manifest.json") {
                continue;
            }
            let Ok(text) = std::str::from_utf8(asset.bytes) else { continue };
            let Ok(json) = serde_json::from_str::<serde_json::Value>(text) else { continue };

            let name = json["name"].as_str().unwrap_or("").to_string();
            if name.is_empty() {
                continue;
            }

            let description = json["description"].as_str().unwrap_or("").to_string();

            let exports: Vec<String> = json["exports"]
                .as_array()
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            let versions: Vec<LibraryVersion> = json["versions"]
                .as_object()
                .map(|obj| {
                    obj.iter()
                        .map(|(key, v)| LibraryVersion {
                            key: key.clone(),
                            entry: v["entry"].as_str().unwrap_or("").to_string(),
                            source: v["source"].as_str().unwrap_or("online").to_string(),
                            package_version: v["package_version"]
                                .as_str()
                                .unwrap_or("")
                                .to_string(),
                            size_bytes: v["size_bytes"].as_u64().unwrap_or(0),
                            integrity: v["integrity"].as_str().unwrap_or("").to_string(),
                            registry_url: v["registry_url"].as_str().map(|s| s.to_string()),
                        })
                        .collect()
                })
                .unwrap_or_default();

            manifests.push(LibraryManifest {
                name,
                description,
                exports,
                versions,
            });
        }
        Self { manifests }
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
}
