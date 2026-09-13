//! The blessed source tree, enumerated from the binary.
//!
//! `blessed/` is the curated content the release itself carries: RWE library
//! bundles (`blessed/rwe-libraries/`), which the hub seeder publishes into
//! the local hub (`services/hub-local/`) at every boot, check-first, as the
//! reserved `zebflow` publisher, through the same publish gates every other
//! package passes; and the `zeb/ui` source library
//! (`blessed/source-libraries/`), which is not a hub package — the compiler
//! inlines it into pages, and the catalog clones from it. This module only
//! enumerates; it publishes nothing.
//!
//! `blessed/nodes/` and `blessed/pipelines/` are deliberately empty today:
//! foundation composites in `src/pipeline/nodes/bundled/` are part of the
//! `n.*` node set the binary provides, not hub packages.

use serde::Deserialize;

use crate::platform::error::PlatformError;
use crate::platform::web::embedded::PLATFORM_LIBRARY_ASSETS;

/// The one publisher id the seeder publishes as. Reserved: any publish under
/// this id through the public publish surface is refused
/// (`distribution.md` §1b).
pub const RESERVED_HUB_PUBLISHER_ID: &str = "zebflow";

/// The `package.yaml` publish metadata carried beside each blessed package.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BlessedPackageMeta {
    pub id: String,
    pub version: String,
    pub title: String,
    pub description: String,
}

/// One file a blessed package carries, path relative to the package root.
#[derive(Debug, Clone)]
pub struct BlessedFile {
    pub rel_path: String,
    pub bytes: &'static [u8],
}

/// One blessed package, ready for the seeder to publish.
#[derive(Debug, Clone)]
pub struct BlessedPackage {
    /// Canonical package id, always under the reserved publisher:
    /// `zebflow.deckgl`, `zebflow.ui-primitives`.
    pub package_id: String,
    pub version: String,
    pub title: String,
    pub description: String,
    /// `rwe_library` or `template_bundle`.
    pub asset_kind: &'static str,
    /// Where in the blessed tree this package came from, recorded as the
    /// seeded release's `source_ref`.
    pub source_ref: String,
    pub files: Vec<BlessedFile>,
}

fn parse_meta(source_ref: &str, bytes: &[u8]) -> Result<BlessedPackageMeta, PlatformError> {
    let meta: BlessedPackageMeta = serde_yaml_ng::from_slice(bytes).map_err(|err| {
        PlatformError::new(
            "BLESSED_PACKAGE_META",
            format!("invalid package.yaml in {source_ref}: {err}"),
        )
    })?;
    if meta.id.trim() != meta.id
        || !meta
            .id
            .strip_prefix(&format!("{RESERVED_HUB_PUBLISHER_ID}."))
            .is_some_and(|slug| !slug.is_empty())
    {
        return Err(PlatformError::new(
            "BLESSED_PACKAGE_META",
            format!(
                "package id '{}' in {source_ref} must use the reserved '{RESERVED_HUB_PUBLISHER_ID}.' prefix",
                meta.id
            ),
        ));
    }
    if meta.version.trim().is_empty() {
        return Err(PlatformError::new(
            "BLESSED_PACKAGE_META",
            format!("package.yaml in {source_ref} declares no version"),
        ));
    }
    Ok(meta)
}

/// Every blessed RWE library package, from the embedded library asset table.
///
/// The embedded table's paths are `zeb/{name}/...`; a package is one `{name}`
/// directory, its `package.yaml` is the publish metadata, and every other file
/// becomes a package entry with the `zeb/{name}/` prefix stripped — the
/// installed copy at `data/hub/rwe-libraries/{name}/` mirrors the package
/// root, `manifest.json` included.
fn blessed_rwe_library_packages() -> Result<Vec<BlessedPackage>, PlatformError> {
    let mut names: Vec<&'static str> = Vec::new();
    for asset in PLATFORM_LIBRARY_ASSETS {
        if let Some(name) = asset
            .path
            .strip_prefix("zeb/")
            .and_then(|rest| rest.strip_suffix("/package.yaml"))
            && !name.contains('/')
        {
            names.push(name);
        }
    }
    names.sort_unstable();

    let mut packages = Vec::with_capacity(names.len());
    for name in names {
        let source_ref = format!("blessed/rwe-libraries/{name}");
        let prefix = format!("zeb/{name}/");
        let mut meta = None;
        let mut files = Vec::new();
        for asset in PLATFORM_LIBRARY_ASSETS {
            let Some(rest) = asset.path.strip_prefix(&prefix) else {
                continue;
            };
            if rest == "package.yaml" {
                meta = Some(parse_meta(&source_ref, asset.bytes)?);
                continue;
            }
            files.push(BlessedFile {
                rel_path: rest.to_string(),
                bytes: asset.bytes,
            });
        }
        let meta = meta.ok_or_else(|| {
            PlatformError::new(
                "BLESSED_PACKAGE_META",
                format!("{source_ref} has no package.yaml"),
            )
        })?;
        files.sort_by(|a, b| a.rel_path.cmp(&b.rel_path));
        packages.push(BlessedPackage {
            package_id: meta.id,
            version: meta.version,
            title: meta.title,
            description: meta.description,
            asset_kind: "rwe_library",
            source_ref,
            files,
        });
    }
    Ok(packages)
}

/// Every package the binary blesses, in seeding order.
pub fn blessed_packages() -> Result<Vec<BlessedPackage>, PlatformError> {
    // `zeb/ui` is not a hub package yet: it ships with the platform as source
    // the compiler inlines, and a project clones a component from it through
    // the catalog. It joins the hub when runtime libraries resolve from
    // `zeb.lock` rather than from the binary.
    blessed_rwe_library_packages()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The seeder packages embedded bytes, not arbitrary files sitting next to
    /// them in the source tree. Required license notices must survive that edge.
    #[test]
    fn every_blessed_library_embeds_its_declared_license_notices() {
        for package in blessed_rwe_library_packages().expect("blessed libraries") {
            let manifest = package
                .files
                .iter()
                .find(|file| file.rel_path == "manifest.json")
                .expect("every library has a manifest");
            let document = crate::contracts::decode_contract::<
                crate::contracts::kinds::RweLibraryManifestContract,
            >(manifest.bytes)
            .expect("valid library manifest");
            for notice in &document.spec.license.notices {
                assert!(
                    package
                        .files
                        .iter()
                        .any(|file| file.rel_path == *notice && !file.bytes.is_empty()),
                    "{} declares notice {notice}, but the seeded package omits its bytes",
                    package.package_id
                );
            }
            if package.package_id == "zebflow.deckgl" {
                assert!(
                    package
                        .files
                        .iter()
                        .any(|file| file.rel_path == "MODIFICATIONS")
                );
            }
        }
    }

    #[test]
    fn every_blessed_package_carries_reserved_metadata_and_files() {
        let packages = blessed_packages().expect("blessed packages enumerate");
        // 11 installable libraries. The six UI template sets that used to
        // sit beside them were a copy of the same files `zeb/ui` now ships as
        // source; a project imports those without installing and clones one
        // through the catalog.
        //
        // Zeb React is not among them and has no package here at all: it is the
        // engine every template imports, not something a project chooses to
        // install. It used to sit in blessed/rwe-libraries/react/ with only a
        // library.json, excluded from the hub purely because it lacked a
        // manifest.json — an exclusion by missing file, which the next person to
        // notice would have "fixed" by adding one.
        assert_eq!(packages.len(), 11);
        assert!(
            !packages
                .iter()
                .any(|p| matches!(p.package_id.as_str(), "zebflow.preact" | "zebflow.react"))
        );
        for package in &packages {
            assert!(
                package.package_id.starts_with("zebflow."),
                "{} is outside the reserved publisher",
                package.package_id
            );
            assert!(!package.files.is_empty(), "{} is empty", package.package_id);
            assert!(
                package
                    .files
                    .iter()
                    .all(|file| file.rel_path != "package.yaml"),
                "{} carries its publish metadata as content",
                package.package_id
            );
        }
        let deckgl = packages
            .iter()
            .find(|package| package.package_id == "zebflow.deckgl")
            .expect("deckgl is blessed");
        assert_eq!(deckgl.asset_kind, "rwe_library");
        assert!(
            deckgl
                .files
                .iter()
                .any(|file| file.rel_path == "manifest.json")
        );
        assert!(
            deckgl
                .files
                .iter()
                .any(|file| file.rel_path == "0.1/runtime/deckgl.bundle.mjs")
        );
        // No UI template package: `zeb/ui` is source, not a hub asset.
        assert!(
            !packages.iter().any(|p| p.package_id.starts_with("zebflow.ui")),
            "zeb/ui ships as source the compiler inlines, not as a hub package"
        );
        assert!(packages.iter().all(|p| p.asset_kind == "rwe_library"));
    }
}
