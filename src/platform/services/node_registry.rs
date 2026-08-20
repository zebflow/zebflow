//! Runtime registry for installed composite and WASM node packages.
//!
//! Scans `data/nodes/` for installed packages, validates manifests, and provides
//! merged node catalogs. Uses the same ArcSwap pattern as `PipelineRuntimeService`.
//!
//! Also loads official composite nodes embedded in the binary via
//! `PLATFORM_COMPOSITE_NODE_ASSETS`.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::contracts::kinds::INSTALLED_NODE_KIND_PREFIX;
use crate::contracts::kinds::{
    BundleScope, DependencyLockNodeBundleSpec, DependencyLockSource, MAX_NODE_BUNDLE_FILES,
    NodeBundleContract, decode_pipeline_graph, normalize_node_bundle, validate_bundle_namespace,
    validate_node_definition_spec,
};
use crate::contracts::{ContractMetadata, decode_contract};
use crate::infra::io::durable::{atomic_write, directory_tree_sha256};
use crate::pipeline::{NodeDefinition, PipelineGraph};
use crate::platform::error::PlatformError;
use crate::platform::model::{
    CredentialTypeDef, InstalledNodePackage, MultiNodePackageDefinition, NodePackageManifest,
    slug_segment,
};
use crate::platform::services::ProjectService;
use crate::platform::services::dependency_lock::DependencyLockService;
use crate::platform::web::embedded::{
    PLATFORM_COMPOSITE_NODE_ASSETS, platform_composite_node_asset,
};
use arc_swap::ArcSwap;

/// An official composite node embedded in the binary.
struct EmbeddedCompositeNode {
    manifest: NodePackageManifest,
    /// Package slug for resolving function pipelines from embedded assets.
    package_slug: String,
    icon_svg: Option<&'static [u8]>,
}

/// Runtime registry of installed node packages across all projects.
pub struct NodeRegistryService {
    projects: Arc<ProjectService>,
    dependency_lock: Arc<DependencyLockService>,
    /// Key = `"{owner}/{project}/{kind}"`.
    inner: ArcSwap<HashMap<String, InstalledNodePackage>>,
    /// Official composite nodes embedded in the binary, keyed by kind.
    embedded_composites: HashMap<String, EmbeddedCompositeNode>,
}

fn registry_key(owner: &str, project: &str, kind: &str) -> String {
    format!("{}/{}/{}", owner, project, kind)
}

impl NodeRegistryService {
    pub fn new(projects: Arc<ProjectService>, dependency_lock: Arc<DependencyLockService>) -> Self {
        let embedded_composites = Self::load_embedded_composites();
        Self {
            projects,
            dependency_lock,
            inner: ArcSwap::new(Arc::new(HashMap::new())),
            embedded_composites,
        }
    }

    /// Loads official composite nodes from `PLATFORM_COMPOSITE_NODE_ASSETS`.
    ///
    /// Every package uses `{slug}/definition.json`, including one-node bundles.
    fn load_embedded_composites() -> HashMap<String, EmbeddedCompositeNode> {
        let mut result = HashMap::new();
        let builtin_kinds: HashSet<String> = crate::pipeline::nodes::builtin_node_definitions()
            .iter()
            .map(|d| d.kind.clone())
            .collect();

        // 1. Scan for multi-node packages: `{slug}/definition.json`.
        let multi_defs: Vec<(&str, &'static [u8])> = PLATFORM_COMPOSITE_NODE_ASSETS
            .iter()
            .filter(|a| a.path.ends_with("/definition.json"))
            .map(|a| {
                let slug = a.path.trim_end_matches("/definition.json");
                (slug, a.bytes)
            })
            .collect();

        for (slug, def_bytes) in multi_defs {
            let document =
                decode_contract::<NodeBundleContract>(def_bytes).unwrap_or_else(|error| {
                    panic!("embedded node bundle '{slug}' is invalid: {error}")
                });
            let pkg_def = document.spec;
            // Bundles baked into the binary are curated by Zebflow, so their
            // kinds live in `n.*` rather than the third-party namespace.
            validate_bundle_namespace(&pkg_def, BundleScope::Platform).unwrap_or_else(|error| {
                panic!("embedded node bundle '{slug}' has an invalid namespace: {error}")
            });
            let manifests = normalize_node_bundle(&pkg_def)
                .expect("decoded embedded node bundle must normalize");
            for manifest in manifests {
                let kind = manifest.definition.kind.clone();
                validate_manifest(&manifest, &builtin_kinds).unwrap_or_else(|error| {
                    panic!("embedded node bundle '{slug}' has invalid '{kind}': {error}")
                });
                let icon_svg = {
                    let node_entry = pkg_def.nodes.iter().find(|node| node.kind == kind);
                    let icon_path = node_entry
                        .and_then(|node| {
                            (!node.icon.is_empty()).then(|| format!("{}/{}", slug, node.icon))
                        })
                        .unwrap_or_else(|| format!("{}/{}", slug, pkg_def.icon));
                    platform_composite_node_asset(&icon_path)
                };
                if result
                    .insert(
                        kind,
                        EmbeddedCompositeNode {
                            manifest,
                            package_slug: slug.to_string(),
                            icon_svg,
                        },
                    )
                    .is_some()
                {
                    panic!("duplicate official node kind in embedded bundles");
                }
            }
        }

        result
    }

    /// Returns definitions for official composite nodes embedded in the binary.
    ///
    /// This is used by public docs/help surfaces that do not have a project context
    /// but still need to advertise platform-bundled composites.
    pub fn embedded_official_definitions() -> Vec<NodeDefinition> {
        let mut defs = Self::load_embedded_composites()
            .into_values()
            .map(|embedded| embedded.manifest.definition)
            .collect::<Vec<_>>();
        defs.sort_by(|a, b| a.kind.cmp(&b.kind));
        defs
    }

    /// Returns manifests for official composite nodes embedded in the binary.
    pub fn embedded_official_manifests() -> Vec<NodePackageManifest> {
        let mut manifests = Self::load_embedded_composites()
            .into_values()
            .map(|embedded| embedded.manifest)
            .collect::<Vec<_>>();
        manifests.sort_by(|a, b| a.definition.kind.cmp(&b.definition.kind));
        manifests
    }

    /// Scans `data/nodes/` for a project and rebuilds its registry entries.
    pub fn refresh_project(&self, owner: &str, project: &str) -> Result<(), PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);

        let layout = self.projects.project_layout(&owner, &project)?;
        let nodes_dir = &layout.data_nodes_dir;

        // Installed bundles may not replace any official node kind.
        let mut official_kinds: HashSet<String> =
            crate::pipeline::nodes::builtin_node_definitions()
                .iter()
                .map(|d| d.kind.clone())
                .collect();
        official_kinds.extend(self.embedded_composites.keys().cloned());

        let mut new_entries: Vec<(String, InstalledNodePackage)> = Vec::new();
        let mut discovered_bundles = BTreeMap::new();
        let mut discovered_kinds = HashSet::new();

        if nodes_dir.is_dir() {
            let entries = std::fs::read_dir(nodes_dir).map_err(|e| {
                PlatformError::new(
                    "NODE_REGISTRY_SCAN",
                    format!("failed reading {}: {}", nodes_dir.display(), e),
                )
            })?;

            for entry in entries {
                let entry = entry.map_err(|error| {
                    PlatformError::new(
                        "NODE_REGISTRY_SCAN",
                        format!("failed reading entry in {}: {error}", nodes_dir.display()),
                    )
                })?;
                let path = entry.path();
                let file_type = entry.file_type().map_err(|error| {
                    PlatformError::new(
                        "NODE_REGISTRY_SCAN",
                        format!("failed reading type of {}: {error}", path.display()),
                    )
                })?;
                if file_type.is_symlink() {
                    return Err(PlatformError::new(
                        "NODE_BUNDLE_PATH",
                        format!(
                            "node bundle directory must not be a symlink: {}",
                            path.display()
                        ),
                    ));
                }
                if !file_type.is_dir() {
                    continue;
                }
                let slug = entry.file_name().to_string_lossy().to_string();

                let definition_path = path.join("definition.json");
                if !definition_path.is_file() {
                    return Err(PlatformError::new(
                        "NODE_BUNDLE_MANIFEST_MISSING",
                        format!(
                            "node bundle '{}' must contain definition.json",
                            path.display()
                        ),
                    ));
                }

                let (package, manifests) =
                    parse_multi_node_definition(&definition_path, &official_kinds)?;
                validate_bundle_namespace(&package, BundleScope::Project).map_err(|error| {
                    PlatformError::new("NODE_BUNDLE_NAMESPACE", error.to_string())
                })?;
                verify_bundle_artifacts(&path, &package)?;
                let bundle =
                    project_bundle_lock_from_multi_node(nodes_dir, &path, &definition_path)?;
                if discovered_bundles
                    .insert(bundle.0.clone(), bundle.1)
                    .is_some()
                {
                    return Err(PlatformError::new(
                        "NODE_BUNDLE_COLLISION",
                        format!("duplicate project node bundle '{}'", bundle.0),
                    ));
                }

                for manifest in manifests {
                    let kind = manifest.definition.kind.clone();
                    if !discovered_kinds.insert(kind.clone()) {
                        return Err(PlatformError::new(
                            "NODE_KIND_COLLISION",
                            format!("node kind '{}' is provided by more than one bundle", kind),
                        ));
                    }
                    let icon_rel_path = package
                        .nodes
                        .iter()
                        .find(|node| node.kind == kind)
                        .map(|node| node.icon.as_str())
                        .filter(|icon| !icon.is_empty())
                        .or_else(|| (!package.icon.is_empty()).then_some(package.icon.as_str()))
                        .filter(|icon| path.join(icon).is_file())
                        .map(str::to_string);
                    let key = registry_key(&owner, &project, &kind);
                    new_entries.push((
                        key,
                        InstalledNodePackage {
                            slug: slug.clone(),
                            owner: owner.clone(),
                            project: project.clone(),
                            manifest,
                            package_dir: path.display().to_string(),
                            icon_rel_path,
                        },
                    ));
                }
            }
        }

        self.dependency_lock.record_discovered_node_bundles(
            &owner,
            &project,
            discovered_bundles.into_iter().collect(),
        )?;

        // Publish the complete validated registry in one atomic pointer swap.
        let guard = self.inner.load();
        let mut map = (**guard).clone();
        let prefix = format!("{}/{}/", owner, project);
        map.retain(|k, _| !k.starts_with(&prefix));
        for (key, pkg) in new_entries {
            map.insert(key, pkg);
        }
        self.inner.store(Arc::new(map));

        Ok(())
    }

    /// Returns all installed node packages for a project.
    pub fn list_installed(&self, owner: &str, project: &str) -> Vec<InstalledNodePackage> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let prefix = format!("{}/{}/", owner, project);
        self.inner
            .load()
            .values()
            .filter(|pkg| {
                registry_key(&pkg.owner, &pkg.project, &pkg.manifest.definition.kind)
                    .starts_with(&prefix)
            })
            .cloned()
            .collect()
    }

    /// Returns a single installed package by node kind.
    pub fn get_by_kind(
        &self,
        owner: &str,
        project: &str,
        kind: &str,
    ) -> Option<InstalledNodePackage> {
        let key = registry_key(&slug_segment(owner), &slug_segment(project), kind);
        self.inner.load().get(&key).cloned()
    }

    /// Returns merged node definitions: builtin + embedded composites + installed, sorted by kind.
    pub fn merged_definitions(&self, owner: &str, project: &str) -> Vec<NodeDefinition> {
        let mut defs = crate::pipeline::nodes::builtin_node_definitions();
        let builtin_kinds: HashSet<String> = defs.iter().map(|d| d.kind.clone()).collect();

        // Add embedded official composites.
        for embedded in self.embedded_composites.values() {
            if !builtin_kinds.contains(&embedded.manifest.definition.kind) {
                defs.push(embedded.manifest.definition.clone());
            }
        }
        let official_kinds: HashSet<String> = defs.iter().map(|d| d.kind.clone()).collect();

        // Add installed (community) nodes — cannot override official.
        for pkg in self.list_installed(owner, project) {
            if !official_kinds.contains(&pkg.manifest.definition.kind) {
                defs.push(pkg.manifest.definition);
            }
        }
        defs.sort_by(|a, b| a.kind.cmp(&b.kind));
        defs
    }

    /// Returns custom credential types from all installed packages for a project.
    /// Returns credential types from all node packages: embedded composites + installed.
    /// Deduplicates by kind (first occurrence wins — embedded before installed).
    pub fn all_package_credential_types(
        &self,
        owner: &str,
        project: &str,
    ) -> Vec<CredentialTypeDef> {
        let mut types = Vec::new();
        let mut seen = HashSet::new();
        // 1. Embedded official composites.
        for embedded in self.embedded_composites.values() {
            for ct in &embedded.manifest.credentials {
                if seen.insert(ct.kind.clone()) {
                    types.push(ct.clone());
                }
            }
        }
        // 2. Installed packages (community).
        for pkg in self.list_installed(owner, project) {
            for ct in pkg.manifest.credentials {
                if seen.insert(ct.kind.clone()) {
                    types.push(ct);
                }
            }
        }
        types
    }

    /// Loads the inner function pipeline graph for a composite node.
    ///
    /// Resolves the node's `main` function name to a pipeline file.
    /// Checks embedded official composites first, then falls back to installed packages.
    pub fn load_composite_pipeline(
        &self,
        owner: &str,
        project: &str,
        kind: &str,
    ) -> Result<PipelineGraph, PlatformError> {
        self.load_composite_function(owner, project, kind, None)
    }

    /// Loads a specific function pipeline for a composite node.
    ///
    /// If `function_name` is None, loads the main/default function.
    /// If `function_name` is Some, loads that specific function (e.g. lifecycle hooks).
    pub fn load_composite_function(
        &self,
        owner: &str,
        project: &str,
        kind: &str,
        function_name: Option<&str>,
    ) -> Result<PipelineGraph, PlatformError> {
        // 1. Check embedded official composites.
        if let Some(embedded) = self.embedded_composites.get(kind) {
            return self.load_embedded_function(embedded, kind, function_name);
        }

        // 2. Check installed packages.
        let pkg = self.get_by_kind(owner, project, kind).ok_or_else(|| {
            PlatformError::new(
                "NODE_COMPOSITE_NOT_FOUND",
                format!(
                    "composite node '{}' not installed in {}/{}",
                    kind, owner, project
                ),
            )
        })?;
        self.load_installed_function(&pkg, kind, function_name)
    }

    /// Load a function pipeline from an embedded composite.
    fn load_embedded_function(
        &self,
        embedded: &EmbeddedCompositeNode,
        kind: &str,
        function_name: Option<&str>,
    ) -> Result<PipelineGraph, PlatformError> {
        // Determine which function to load.
        let fn_name = function_name
            .map(|s| s.to_string())
            .or_else(|| embedded.manifest.main_function().map(str::to_string));

        if let Some(fn_name) = &fn_name {
            // Multi-node: resolve function name → file path → embedded asset.
            let fn_path = embedded
                .manifest
                .functions
                .get(fn_name.as_str())
                .ok_or_else(|| {
                    PlatformError::new(
                        "NODE_COMPOSITE_FUNCTION_NOT_FOUND",
                        format!(
                            "composite '{}': function '{}' not in functions map",
                            kind, fn_name
                        ),
                    )
                })?;
            let asset_path = format!("{}/{}", embedded.package_slug, fn_path);
            let bytes = platform_composite_node_asset(&asset_path).ok_or_else(|| {
                PlatformError::new(
                    "NODE_COMPOSITE_PIPELINE_READ",
                    format!(
                        "embedded composite '{}': missing asset '{}'",
                        kind, asset_path
                    ),
                )
            })?;
            let graph = decode_pipeline_graph(bytes).map_err(|e| {
                PlatformError::new(
                    "NODE_COMPOSITE_PIPELINE_PARSE",
                    format!(
                        "embedded composite '{}' function '{}': {}",
                        kind, fn_name, e
                    ),
                )
            })?;
            Ok(graph.spec)
        } else {
            Err(PlatformError::new(
                "NODE_COMPOSITE_NO_RUNTIME",
                format!(
                    "composite '{}' has no main function or runtime pipeline",
                    kind
                ),
            ))
        }
    }

    /// Load a function pipeline from an installed (disk-based) package.
    fn load_installed_function(
        &self,
        pkg: &InstalledNodePackage,
        kind: &str,
        function_name: Option<&str>,
    ) -> Result<PipelineGraph, PlatformError> {
        let fn_name = function_name
            .map(|s| s.to_string())
            .or_else(|| pkg.manifest.main_function().map(str::to_string));

        let pipeline_path = if let Some(fn_name) = &fn_name {
            // Multi-node: resolve function name → file path.
            let fn_path = pkg
                .manifest
                .functions
                .get(fn_name.as_str())
                .ok_or_else(|| {
                    PlatformError::new(
                        "NODE_COMPOSITE_FUNCTION_NOT_FOUND",
                        format!(
                            "node '{}': function '{}' not in functions map",
                            kind, fn_name
                        ),
                    )
                })?;
            std::path::Path::new(&pkg.package_dir).join(fn_path)
        } else {
            return Err(PlatformError::new(
                "NODE_COMPOSITE_NO_RUNTIME",
                format!("node '{}' has no main function or runtime pipeline", kind),
            ));
        };

        let source = std::fs::read_to_string(&pipeline_path).map_err(|e| {
            PlatformError::new(
                "NODE_COMPOSITE_PIPELINE_READ",
                format!("failed reading '{}': {}", pipeline_path.display(), e),
            )
        })?;
        let graph = decode_pipeline_graph(source.as_bytes()).map_err(|e| {
            PlatformError::new(
                "NODE_COMPOSITE_PIPELINE_PARSE",
                format!("failed parsing '{}': {}", pipeline_path.display(), e),
            )
        })?;
        Ok(graph.spec)
    }

    /// Returns the icon SVG bytes for an installed node, if present.
    pub fn load_icon(&self, owner: &str, project: &str, kind: &str) -> Option<Vec<u8>> {
        let pkg = self.get_by_kind(owner, project, kind)?;
        let icon_path = std::path::Path::new(&pkg.package_dir).join(pkg.icon_rel_path?);
        std::fs::read(&icon_path).ok()
    }

    /// Returns icon bytes for a node kind from any registry source:
    /// installed packages first, then embedded official composites.
    ///
    /// Builtin native node icons (from `PLATFORM_NODE_ICON_ASSETS`) are handled
    /// separately in the API layer.
    pub fn load_icon_any(&self, owner: &str, project: &str, kind: &str) -> Option<Vec<u8>> {
        // 1. Installed package (project-level).
        if let Some(bytes) = self.load_icon(owner, project, kind) {
            return Some(bytes);
        }
        // 2. Embedded official composite.
        if let Some(embedded) = self.embedded_composites.get(kind) {
            return embedded.icon_svg.map(|b| b.to_vec());
        }
        None
    }

    /// Returns the manifest for a composite node kind from any source:
    /// embedded official composites first, then installed packages.
    pub fn get_manifest(
        &self,
        owner: &str,
        project: &str,
        kind: &str,
    ) -> Option<NodePackageManifest> {
        // 1. Embedded official composite.
        if let Some(embedded) = self.embedded_composites.get(kind) {
            return Some(embedded.manifest.clone());
        }
        // 2. Installed package.
        self.get_by_kind(owner, project, kind)
            .map(|pkg| pkg.manifest)
    }

    /// Returns true if a node kind is official (native or embedded composite).
    ///
    /// Official nodes cannot be uninstalled.
    pub fn is_official(&self, kind: &str) -> bool {
        // Anything outside the installed namespace is native, so it is official.
        if !kind.starts_with(crate::contracts::kinds::INSTALLED_NODE_KIND_PREFIX) {
            return true;
        }
        // Embedded composites are official.
        self.embedded_composites.contains_key(kind)
    }

    /// Uninstalls a node package from the project.
    pub fn uninstall_package(
        &self,
        owner: &str,
        project: &str,
        kind: &str,
    ) -> Result<(), PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);

        let pkg = self.get_by_kind(&owner, &project, kind).ok_or_else(|| {
            PlatformError::new(
                "NODE_UNINSTALL_NOT_FOUND",
                format!("node '{}' not installed in {}/{}", kind, owner, project),
            )
        })?;

        let pkg_dir = std::path::Path::new(&pkg.package_dir);
        if pkg_dir.exists() {
            std::fs::remove_dir_all(pkg_dir).map_err(|e| {
                PlatformError::new(
                    "NODE_UNINSTALL_REMOVE",
                    format!("failed removing {}: {}", pkg_dir.display(), e),
                )
            })?;
        }

        self.refresh_project(&owner, &project)?;
        self.dependency_lock
            .remove_node_bundle_providing(&owner, &project, kind)?;
        Ok(())
    }
}

/// Verifies that every artifact a bundle declares actually exists on disk.
///
/// The contract validator can only check that a path is *shaped* safely. Without
/// this, a bundle referencing a missing function or module publishes cleanly and
/// fails much later, at run time, far from the cause.
fn verify_bundle_artifacts(
    package_dir: &Path,
    package: &MultiNodePackageDefinition,
) -> Result<(), PlatformError> {
    let missing = |label: &str, relative: &str| {
        PlatformError::new(
            "NODE_BUNDLE_ARTIFACT_MISSING",
            format!(
                "node bundle '{}' declares {label} '{relative}' which is not a file in {}",
                package.package,
                package_dir.display()
            ),
        )
    };
    let resolve = |relative: &str| -> Result<PathBuf, PlatformError> {
        let path = Path::new(relative);
        if path.is_absolute()
            || path
                .components()
                .any(|part| !matches!(part, std::path::Component::Normal(_)))
        {
            return Err(PlatformError::new(
                "NODE_BUNDLE_PATH",
                format!(
                    "node bundle '{}' declares unsafe path '{relative}'",
                    package.package
                ),
            ));
        }
        Ok(package_dir.join(path))
    };

    for (name, relative) in &package.functions {
        let path = resolve(relative)?;
        if !path.is_file() {
            return Err(missing(&format!("function '{name}'"), relative));
        }
        // A composite function is a Pipeline document. Decoding it here keeps a
        // malformed function from reaching the registry and failing at run time.
        let source = std::fs::read(&path).map_err(|error| {
            PlatformError::new(
                "NODE_BUNDLE_ARTIFACT_UNREADABLE",
                format!(
                    "node bundle '{}' cannot read function '{name}' at '{relative}': {error}",
                    package.package
                ),
            )
        })?;
        decode_pipeline_graph(&source).map_err(|error| {
            PlatformError::new(
                "NODE_BUNDLE_FUNCTION_INVALID",
                format!(
                    "node bundle '{}' function '{name}' is not a valid pipeline: {} ({})",
                    package.package,
                    error,
                    error.category()
                ),
            )
        })?;
    }
    for (name, module) in &package.modules {
        if !resolve(&module.path)?.is_file() {
            return Err(missing(&format!("module '{name}'"), &module.path));
        }
    }
    if !package.icon.is_empty() && !resolve(&package.icon)?.is_file() {
        return Err(missing("package icon", &package.icon));
    }
    for node in &package.nodes {
        if !node.icon.is_empty() && !resolve(&node.icon)?.is_file() {
            return Err(missing(&format!("icon for '{}'", node.kind), &node.icon));
        }
    }

    let files = count_package_files(package_dir)?;
    if files > MAX_NODE_BUNDLE_FILES {
        return Err(PlatformError::new(
            "NODE_BUNDLE_FILE_LIMIT",
            format!(
                "node bundle '{}' contains {files} files; the limit is {MAX_NODE_BUNDLE_FILES}",
                package.package
            ),
        ));
    }
    Ok(())
}

/// Counts regular files under a package directory, rejecting symlinks.
fn count_package_files(root: &Path) -> Result<usize, PlatformError> {
    let mut total = 0usize;
    let mut stack = vec![root.to_path_buf()];
    while let Some(current) = stack.pop() {
        let entries = std::fs::read_dir(&current).map_err(|error| {
            PlatformError::new(
                "NODE_BUNDLE_SCAN",
                format!("failed reading {}: {error}", current.display()),
            )
        })?;
        for entry in entries {
            let entry = entry.map_err(|error| {
                PlatformError::new(
                    "NODE_BUNDLE_SCAN",
                    format!("failed reading entry in {}: {error}", current.display()),
                )
            })?;
            let file_type = entry.file_type().map_err(|error| {
                PlatformError::new(
                    "NODE_BUNDLE_SCAN",
                    format!("failed reading type of {}: {error}", entry.path().display()),
                )
            })?;
            if file_type.is_symlink() {
                return Err(PlatformError::new(
                    "NODE_BUNDLE_PATH",
                    format!(
                        "node bundle must not contain a symlink: {}",
                        entry.path().display()
                    ),
                ));
            }
            if file_type.is_dir() {
                stack.push(entry.path());
            } else if file_type.is_file() {
                total += 1;
                if total > MAX_NODE_BUNDLE_FILES {
                    return Ok(total);
                }
            }
        }
    }
    Ok(total)
}

fn project_bundle_lock_from_multi_node(
    nodes_dir: &Path,
    package_dir: &Path,
    definition_path: &Path,
) -> Result<(String, DependencyLockNodeBundleSpec), PlatformError> {
    let raw = std::fs::read(definition_path).map_err(|error| {
        PlatformError::new(
            "NODE_MANIFEST_READ",
            format!("failed reading {}: {error}", definition_path.display()),
        )
    })?;
    let document = decode_contract::<NodeBundleContract>(&raw).map_err(|error| {
        PlatformError::new(
            "NODE_MANIFEST_PARSE",
            format!("{} ({})", error, error.category()),
        )
    })?;
    let mut definitions = document
        .spec
        .nodes
        .iter()
        .map(|node| node.kind.clone())
        .collect::<Vec<_>>();
    definitions.sort();
    definitions.dedup();
    let package_name = document.spec.package.clone();
    let key = format!("project/{package_name}");
    Ok((
        key,
        DependencyLockNodeBundleSpec {
            version: document.spec.version,
            source: DependencyLockSource::Project,
            source_id: format!("project/{package_name}"),
            entry: node_manifest_entry(nodes_dir, definition_path)?,
            integrity: directory_tree_sha256(package_dir)
                .map_err(|error| PlatformError::new("NODE_BUNDLE_HASH", error.to_string()))?,
            definitions,
        },
    ))
}

fn node_manifest_entry(nodes_dir: &Path, manifest_path: &Path) -> Result<String, PlatformError> {
    let relative = manifest_path.strip_prefix(nodes_dir).map_err(|_| {
        PlatformError::new(
            "NODE_BUNDLE_PATH",
            "node bundle manifest escaped the project nodes directory",
        )
    })?;
    Ok(format!(
        "nodes/{}",
        relative.to_string_lossy().replace('\\', "/")
    ))
}

/// Parse a multi-node `definition.json` from disk and explode into manifests.
fn parse_multi_node_definition(
    def_path: &std::path::Path,
    builtin_kinds: &HashSet<String>,
) -> Result<(MultiNodePackageDefinition, Vec<NodePackageManifest>), PlatformError> {
    let raw = std::fs::read_to_string(def_path).map_err(|e| {
        PlatformError::new(
            "NODE_MANIFEST_READ",
            format!("failed reading {}: {}", def_path.display(), e),
        )
    })?;
    let pkg_def = decode_contract::<NodeBundleContract>(raw.as_bytes()).map_err(|e| {
        PlatformError::new(
            "NODE_MANIFEST_PARSE",
            format!(
                "invalid definition.json at {}: {} ({})",
                def_path.display(),
                e,
                e.category()
            ),
        )
    })?;
    let package = pkg_def.spec;
    let manifests = normalize_node_bundle(&package).map_err(|e| {
        PlatformError::new(
            "NODE_MANIFEST_PARSE",
            format!("invalid definition.json at {}: {}", def_path.display(), e),
        )
    })?;
    for manifest in &manifests {
        validate_manifest(manifest, builtin_kinds)?;
    }
    Ok((package, manifests))
}

/// Validate a node package manifest against namespace and collision rules.
fn validate_manifest(
    manifest: &NodePackageManifest,
    builtin_kinds: &HashSet<String>,
) -> Result<(), PlatformError> {
    let kind = &manifest.definition.kind;
    if let Err(error) = validate_node_definition_spec(manifest) {
        return Err(PlatformError::new(
            "NODE_DEFINITION_INCOMPLETE",
            format!("node '{}' definition invalid: {}", kind, error),
        ));
    }

    // Collision check.
    if builtin_kinds.contains(kind) {
        return Err(PlatformError::new(
            "NODE_KIND_COLLISION",
            format!("node kind '{}' collides with a built-in native node", kind),
        ));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;
    use std::path::Path;
    use std::sync::Arc;

    use crate::contracts::{ContractMetadata, encode_contract};
    use crate::platform::adapters::data::build_data_adapter;
    use crate::platform::adapters::file::{FileAdapter, FilesystemFileAdapter};
    use crate::platform::adapters::project_data::build_project_data_factory;
    use crate::platform::model::{
        CreateProjectRequest, DataAdapterKind, PlatformUser, PlatformUserLocalAuth,
        ProjectRuntimeSelectionRequest, StoredUser, now_ts,
    };
    use crate::platform::services::dependency_lock::DependencyLockService;
    use crate::platform::services::project_config::ProjectConfigurationService;

    fn make_registry(root: &Path) -> super::NodeRegistryService {
        let data = build_data_adapter(DataAdapterKind::Sqlite, root).expect("sqlite adapter");
        let configs = Arc::new(ProjectConfigurationService::new(root.join("users")));
        let file = Arc::new(FilesystemFileAdapter::new(
            root.join("users"),
            Arc::clone(&configs),
        ));
        file.initialize().expect("file adapter init");
        let now = now_ts();
        let user_id = "usr_node_registry_test".to_string();
        data.put_user(&StoredUser {
            profile: PlatformUser {
                user_id: user_id.clone(),
                owner: "superadmin".into(),
                role: "superadmin".into(),
                git_name: "Superadmin".into(),
                git_email: String::new(),
                created_at: now,
                updated_at: now,
            },
            auth: PlatformUserLocalAuth {
                user_id,
                password_hash: String::new(),
                password_alg: "sha256".into(),
                password_updated_at: now,
            },
        })
        .expect("seed owner");
        let dependency_lock = Arc::new(DependencyLockService::new(root.join("users")));
        let projects = Arc::new(crate::platform::services::ProjectService::new(
            data,
            file,
            build_project_data_factory(root),
            configs,
            dependency_lock.clone(),
        ));
        projects
            .create_or_update_project(
                "superadmin",
                &CreateProjectRequest {
                    project: "default".into(),
                    title: Some("Default".into()),
                    local_branch: None,
                    runtime: ProjectRuntimeSelectionRequest::default(),
                },
            )
            .expect("create project");
        super::NodeRegistryService::new(projects, dependency_lock)
    }

    fn write_composite_bundle(root: &Path, directory: &str, package: &str, kind: &str, icon: &str) {
        let package_dir = root
            .join("users/superadmin/default/data/nodes")
            .join(directory);
        std::fs::create_dir_all(package_dir.join("functions")).expect("package dirs");
        let spec: crate::platform::model::MultiNodePackageDefinition =
            serde_json::from_value(serde_json::json!({
                "package": package,
                "version": "1.0.0",
                "title": "Test Bundle",
                "description": "A complete test bundle for registry behavior.",
                "icon": icon,
                "functions": {"main": "functions/main.zf.json"},
                "nodes": [{
                    "kind": kind,
                    "title": "Test Node",
                    "description": "Return the incoming test payload.",
                    "icon": icon,
                    "run": { "function": "main" },
                    "definition": {
                        "config_schema": {},
                        "input_schema": {"type": "object"},
                        "output_schema": {"type": "object"},
                        "input_pins": ["in"],
                        "output_pins": ["out"]
                    }
                }]
            }))
            .expect("bundle spec");
        let mut metadata = ContractMetadata::named(package);
        metadata.version = Some("1.0.0".into());
        let bytes = encode_contract::<crate::contracts::kinds::NodeBundleContract>(metadata, spec)
            .expect("bundle contract");
        std::fs::write(package_dir.join("definition.json"), bytes).expect("definition");
        std::fs::write(
            package_dir.join("functions/main.zf.json"),
            include_bytes!("../../../tests/fixtures/contracts/pipeline/v1-complete.json"),
        )
        .expect("function");
        if !icon.is_empty() {
            let icon_path = package_dir.join(icon);
            std::fs::create_dir_all(icon_path.parent().expect("icon parent")).expect("icon dirs");
            std::fs::write(icon_path, b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>")
                .expect("icon");
        }
    }

    /// A node that exposes a credential's config key must declare that
    /// credential kind, or the UI silently offers the wrong provider list.
    #[test]
    fn embedded_official_nodes_declare_every_credential_they_expose() {
        use crate::contracts::decode_contract;
        use crate::contracts::kinds::NodeBundleContract;
        use crate::platform::web::embedded::PLATFORM_COMPOSITE_NODE_ASSETS;

        for (slug, bytes) in PLATFORM_COMPOSITE_NODE_ASSETS
            .iter()
            .filter(|asset| asset.path.ends_with("/definition.json"))
            .map(|asset| (asset.path, asset.bytes))
        {
            let package = decode_contract::<NodeBundleContract>(bytes)
                .unwrap_or_else(|error| panic!("{slug} decodes: {error}"))
                .spec;
            for node in &package.nodes {
                let properties = node
                    .definition
                    .config_schema
                    .get("properties")
                    .and_then(serde_json::Value::as_object);
                let expected: Vec<&str> = package
                    .credentials
                    .iter()
                    .filter(|credential| {
                        properties.is_some_and(|values| values.contains_key(&credential.config_key))
                    })
                    .map(|credential| credential.kind.as_str())
                    .collect();
                for kind in expected {
                    assert!(
                        node.uses_credentials.iter().any(|used| used == kind),
                        "{slug}: node '{}' exposes a config key for credential '{kind}' but does not declare it",
                        node.kind
                    );
                }
            }
        }
    }

    #[test]
    fn embedded_official_manifests_have_complete_node_contracts() {
        let builtin_kinds: HashSet<String> = crate::pipeline::nodes::builtin_node_definitions()
            .iter()
            .map(|def| def.kind.clone())
            .collect();
        for manifest in super::NodeRegistryService::embedded_official_manifests() {
            super::validate_manifest(&manifest, &builtin_kinds)
                .unwrap_or_else(|err| panic!("{}: {}", manifest.definition.kind, err.message));
        }
    }

    #[test]
    fn multi_node_definition_can_describe_wasm_nodes() {
        let raw = r#"
{
  "apiVersion": "zebflow.com/v1",
  "kind": "NodeBundle",
  "metadata": { "name": "wasm-test-add", "version": "0.1.0" },
  "spec": {
  "package": "wasm-test-add",
  "version": "0.1.0",
  "title": "WASM Test Add",
  "description": "Tiny local WASM node package smoke test.",
  "icon": "icon.svg",
  "modules": {
    "core": { "path": "module.wasm", "abi": "zebflow-wasm-json-v1" }
  },
  "nodes": [
    {
      "kind": "n.x.wasm_test_add.add",
      "title": "WASM Test Add",
      "description": "Run a tiny WASM addition function for local node package smoke tests.",
      "icon": "icon.svg",
      "ui_category": "wasm.test",
      "ui_category_label": "WASM Test",
      "run": { "module": "core", "export": "zebflow_add" },
      "definition": {
        "input_pins": ["in"],
        "output_pins": ["out", "error"],
        "config_schema": {
          "type": "object",
          "properties": {
            "a": { "type": "integer" },
            "b": { "type": "integer" }
          }
        },
        "dsl_flags": [
          {
            "flag": "--a",
            "config_key": "a",
            "kind": "scalar",
            "required": false,
            "description": "Left integer operand."
          },
          {
            "flag": "--b",
            "config_key": "b",
            "kind": "scalar",
            "required": false,
            "description": "Right integer operand."
          }
        ],
        "fields": [
          {
            "name": "a",
            "label": "A",
            "type": "number",
            "help": "Left integer operand."
          },
          {
            "name": "b",
            "label": "B",
            "type": "number",
            "help": "Right integer operand."
          }
        ],
        "layout": ["a", "b"]
      }
    }
  ]}
}
"#;
        let pkg = crate::contracts::decode_contract::<crate::contracts::kinds::NodeBundleContract>(
            raw.as_bytes(),
        )
        .expect("definition")
        .spec;
        let manifests = crate::contracts::kinds::normalize_node_bundle(&pkg)
            .expect("valid bundle must normalize");
        assert_eq!(manifests.len(), 1);
        let manifest = &manifests[0];
        assert_eq!(
            manifest.source,
            crate::platform::model::NodePackageSource::Wasm
        );
        assert_eq!(manifest.definition.kind, "n.x.wasm_test_add.add");
        let (module, export) = manifest.wasm_target().expect("wasm target");
        assert_eq!(module.path, "module.wasm");
        assert_eq!(export, "zebflow_add");

        let builtin_kinds: HashSet<String> = crate::pipeline::nodes::builtin_node_definitions()
            .iter()
            .map(|def| def.kind.clone())
            .collect();
        super::validate_manifest(manifest, &builtin_kinds).expect("valid wasm manifest");
    }

    #[test]
    fn refresh_is_fail_closed_and_keeps_previous_registry() {
        let temp = tempfile::tempdir().expect("temp dir");
        let registry = make_registry(temp.path());
        write_composite_bundle(
            temp.path(),
            "alpha",
            "alpha",
            "n.x.alpha.thing",
            "icons/alpha.svg",
        );
        registry
            .refresh_project("superadmin", "default")
            .expect("initial refresh");
        assert!(
            registry
                .get_by_kind("superadmin", "default", "n.x.alpha.thing")
                .is_some()
        );

        let invalid = temp
            .path()
            .join("users/superadmin/default/data/nodes/invalid");
        std::fs::create_dir_all(invalid).expect("invalid package dir");
        let error = registry
            .refresh_project("superadmin", "default")
            .expect_err("invalid package must fail refresh");
        assert_eq!(error.code, "NODE_BUNDLE_MANIFEST_MISSING");
        assert!(
            registry
                .get_by_kind("superadmin", "default", "n.x.alpha.thing")
                .is_some(),
            "failed refresh must not publish a partial registry"
        );
    }

    /// Two bundles cannot claim one kind, because a kind names its package.
    ///
    /// Ownership is structural rather than discovered at refresh time, so this
    /// asserts the disk boundary still refuses a bundle that tries to escape
    /// its namespace instead of publishing a package that owns a foreign kind.
    #[test]
    fn refresh_rejects_a_bundle_claiming_a_foreign_kind() {
        let temp = tempfile::tempdir().expect("temp dir");
        let registry = make_registry(temp.path());
        write_composite_bundle(temp.path(), "one", "one", "n.x.one.same", "");
        registry
            .refresh_project("superadmin", "default")
            .expect("owned kind refreshes");

        // Hand-write a second bundle that claims the first package's kind. The
        // contract rejects it, so the refresh fails closed.
        let package_dir = temp
            .path()
            .join("users/superadmin/default/data/nodes")
            .join("two");
        let stolen = std::fs::read_to_string(
            temp.path()
                .join("users/superadmin/default/data/nodes/one/definition.json"),
        )
        .expect("source bundle")
        .replace("\"package\": \"one\"", "\"package\": \"two\"")
        .replace("\"name\": \"one\"", "\"name\": \"two\"");
        std::fs::create_dir_all(package_dir.join("functions")).expect("package dirs");
        std::fs::write(package_dir.join("definition.json"), stolen).expect("definition");
        std::fs::write(
            package_dir.join("functions/main.zf.json"),
            include_bytes!("../../../tests/fixtures/contracts/pipeline/v1-complete.json"),
        )
        .expect("function");

        let error = registry
            .refresh_project("superadmin", "default")
            .expect_err("a foreign kind must fail refresh");
        assert_eq!(error.code, "NODE_BUNDLE_NAMESPACE");
        // The previous valid registry stays active.
        assert!(
            registry
                .get_by_kind("superadmin", "default", "n.x.one.same")
                .is_some()
        );
    }

    /// A declared artifact that is not on disk must fail the refresh, not
    /// publish and fail later at run time.
    #[test]
    fn refresh_rejects_a_bundle_with_a_missing_artifact() {
        let temp = tempfile::tempdir().expect("temp dir");
        let registry = make_registry(temp.path());
        write_composite_bundle(temp.path(), "gap", "gap", "n.x.gap.thing", "");
        registry
            .refresh_project("superadmin", "default")
            .expect("complete bundle refreshes");

        let function = temp
            .path()
            .join("users/superadmin/default/data/nodes/gap/functions/main.zf.json");
        std::fs::remove_file(&function).expect("remove declared function");

        let error = registry
            .refresh_project("superadmin", "default")
            .expect_err("a missing declared artifact must fail refresh");
        assert_eq!(error.code, "NODE_BUNDLE_ARTIFACT_MISSING");
        // The previous valid registry stays active.
        assert!(
            registry
                .get_by_kind("superadmin", "default", "n.x.gap.thing")
                .is_some()
        );
    }

    /// A composite function is a Pipeline document. A malformed one must fail
    /// the refresh rather than reach the registry and fail at run time.
    #[test]
    fn refresh_rejects_a_bundle_with_an_invalid_function_pipeline() {
        let temp = tempfile::tempdir().expect("temp dir");
        let registry = make_registry(temp.path());
        write_composite_bundle(temp.path(), "broken", "broken", "n.x.broken.thing", "");
        registry
            .refresh_project("superadmin", "default")
            .expect("valid bundle refreshes");

        std::fs::write(
            temp.path()
                .join("users/superadmin/default/data/nodes/broken/functions/main.zf.json"),
            b"{\"not\":\"a pipeline\"}",
        )
        .expect("corrupt the function");

        let error = registry
            .refresh_project("superadmin", "default")
            .expect_err("an invalid function must fail refresh");
        assert_eq!(error.code, "NODE_BUNDLE_FUNCTION_INVALID");
        assert!(
            registry
                .get_by_kind("superadmin", "default", "n.x.broken.thing")
                .is_some(),
            "the previous valid registry stays active"
        );
    }

    /// The interface is what makes a graph survive a bundle it cannot obtain.
    #[test]
    fn third_party_interfaces_are_written_pruned_and_kept_when_the_bundle_goes() {
        let temp = tempfile::tempdir().expect("temp dir");
        let registry = make_registry(temp.path());
        write_composite_bundle(temp.path(), "acme", "acme", "n.x.acme.thing", "");
        registry
            .refresh_project("superadmin", "default")
            .expect("refresh");

        let project = temp.path().join("users/superadmin/default");
        let pipelines = project.join("repo/pipelines");
        std::fs::create_dir_all(&pipelines).expect("pipelines dir");
        // Build from the golden Pipeline so the graph really decodes; the scan
        // skips undecodable files, which would make this pass vacuously.
        let mut graph: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../../../tests/fixtures/contracts/pipeline/v1-complete.json"
        ))
        .expect("golden pipeline");
        let last = graph["spec"]["nodes"]
            .as_array_mut()
            .expect("nodes")
            .last_mut()
            .expect("at least one node");
        last["kind"] = serde_json::json!("n.x.acme.thing");
        std::fs::write(
            pipelines.join("uses-acme.zf.json"),
            serde_json::to_vec_pretty(&graph).expect("graph"),
        )
        .expect("write pipeline");

        registry
            .sync_project_node_interfaces("superadmin", "default")
            .expect("sync");

        let interface = project.join("repo/nodes/n.x.acme.thing.json");
        assert!(interface.is_file(), "the interface is materialized");
        let document = crate::contracts::kinds::decode_node_definition(
            &std::fs::read(&interface).expect("read interface"),
        )
        .expect("interface is a valid NodeDefinition");
        assert_eq!(document.spec.kind, "n.x.acme.thing");
        assert_eq!(document.spec.output_pins, vec!["out".to_string()]);

        // Curated nodes carry no portability risk, so they get no interface.
        assert!(!project.join("repo/nodes/n.trigger.manual.json").exists());

        // Losing the bundle must not lose the interface: it is now the only
        // description of what the node was.
        std::fs::remove_dir_all(project.join("data/nodes/acme")).expect("remove bundle");
        registry
            .refresh_project("superadmin", "default")
            .expect("refresh without the bundle");
        registry
            .sync_project_node_interfaces("superadmin", "default")
            .expect("sync without the bundle");
        assert!(interface.is_file(), "the interface survives the bundle");

        // The interface is now the only description of the node, so it must
        // resolve even though nothing can run it.
        let unavailable = registry.unavailable_interface_definitions("superadmin", "default");
        assert_eq!(unavailable.len(), 1);
        assert_eq!(unavailable[0].kind, "n.x.acme.thing");
        assert_eq!(unavailable[0].output_pins, vec!["out".to_string()]);

        // Dropping the reference prunes it.
        std::fs::remove_file(pipelines.join("uses-acme.zf.json")).expect("remove pipeline");
        registry
            .sync_project_node_interfaces("superadmin", "default")
            .expect("sync after removing the reference");
        assert!(!interface.exists(), "an unreferenced interface is pruned");
    }

    /// While the bundle is installed the registry is the better answer, so the
    /// interface must not produce a duplicate catalog entry.
    #[test]
    fn an_installed_bundle_is_not_also_reported_as_unavailable() {
        let temp = tempfile::tempdir().expect("temp dir");
        let registry = make_registry(temp.path());
        write_composite_bundle(temp.path(), "acme", "acme", "n.x.acme.thing", "");
        registry
            .refresh_project("superadmin", "default")
            .expect("refresh");

        let interfaces = temp.path().join("users/superadmin/default/repo/nodes");
        std::fs::create_dir_all(&interfaces).expect("interfaces dir");
        let definition = registry
            .get_manifest("superadmin", "default", "n.x.acme.thing")
            .expect("installed")
            .definition;
        let bytes = crate::contracts::kinds::encode_node_definition(
            crate::contracts::ContractMetadata::named("n.x.acme.thing"),
            definition,
        )
        .expect("encode");
        std::fs::write(interfaces.join("n.x.acme.thing.json"), bytes).expect("write interface");

        assert!(
            registry
                .unavailable_interface_definitions("superadmin", "default")
                .is_empty(),
            "an installed kind is served by the registry, not by its interface"
        );
    }

    /// Install spans files, the lock, and the registry, so a killed process can
    /// stop between them. This constructs the state each interruption would
    /// leave and asserts the next refresh either recovers or fails closed.
    ///
    /// The contract calls install recoverable rather than atomic; this is what
    /// that promise means in practice.
    #[test]
    fn refresh_recovers_or_fails_closed_after_an_interrupted_install() {
        let temp = tempfile::tempdir().expect("temp dir");
        let registry = make_registry(temp.path());
        let nodes_dir = temp.path().join("users/superadmin/default/data/nodes");

        // Interrupted after the files landed but before the lock was written.
        // Nothing references the bundle yet, so the refresh adopts and pins it.
        write_composite_bundle(temp.path(), "landed", "landed", "n.x.landed.thing", "");
        registry
            .refresh_project("superadmin", "default")
            .expect("a complete package with no lock entry is adopted");
        assert!(
            registry
                .get_by_kind("superadmin", "default", "n.x.landed.thing")
                .is_some()
        );
        let lock = registry
            .dependency_lock
            .read("superadmin", "default")
            .expect("lock");
        assert!(
            lock.nodes
                .bundles
                .values()
                .any(|bundle| bundle.definitions.iter().any(|k| k == "n.x.landed.thing")),
            "the lock self-heals to describe what is on disk"
        );

        // Interrupted before definition.json was written, leaving a directory
        // with artifacts but no manifest.
        let partial = nodes_dir.join("partial");
        std::fs::create_dir_all(partial.join("functions")).expect("partial dirs");
        std::fs::write(partial.join("functions/main.zf.json"), b"{}").expect("orphan artifact");
        let error = registry
            .refresh_project("superadmin", "default")
            .expect_err("a package without a manifest must fail refresh");
        assert_eq!(error.code, "NODE_BUNDLE_MANIFEST_MISSING");
        assert!(
            registry
                .get_by_kind("superadmin", "default", "n.x.landed.thing")
                .is_some(),
            "the previously published registry stays active"
        );

        // Removing the half-written package is the recovery, and it restores a
        // clean refresh without touching the bundle that did land.
        std::fs::remove_dir_all(&partial).expect("clean up the partial package");
        registry
            .refresh_project("superadmin", "default")
            .expect("refresh recovers once the partial package is gone");
        assert!(
            registry
                .get_by_kind("superadmin", "default", "n.x.landed.thing")
                .is_some()
        );
    }

    /// Uninstall must remove only what the selected kind's package owns.
    #[test]
    fn uninstall_removes_only_the_owning_bundle() {
        let temp = tempfile::tempdir().expect("temp dir");
        let registry = make_registry(temp.path());
        write_composite_bundle(temp.path(), "alpha", "alpha", "n.x.alpha.thing", "");
        write_composite_bundle(temp.path(), "beta", "beta", "n.x.beta.thing", "");
        registry
            .refresh_project("superadmin", "default")
            .expect("refresh");

        registry
            .uninstall_package("superadmin", "default", "n.x.alpha.thing")
            .expect("uninstall alpha");

        let nodes_dir = temp.path().join("users/superadmin/default/data/nodes");
        assert!(!nodes_dir.join("alpha").exists(), "owned files are removed");
        assert!(
            nodes_dir.join("beta").exists(),
            "an unrelated bundle must survive"
        );
        assert!(
            registry
                .get_by_kind("superadmin", "default", "n.x.alpha.thing")
                .is_none()
        );
        assert!(
            registry
                .get_by_kind("superadmin", "default", "n.x.beta.thing")
                .is_some()
        );

        let lock = registry
            .dependency_lock
            .read("superadmin", "default")
            .expect("lock");
        assert!(
            !lock
                .nodes
                .bundles
                .values()
                .any(|bundle| bundle.definitions.iter().any(|k| k == "n.x.alpha.thing"))
        );
        assert!(
            lock.nodes
                .bundles
                .values()
                .any(|bundle| bundle.definitions.iter().any(|k| k == "n.x.beta.thing")),
            "the surviving bundle keeps its lock entry"
        );
    }

    #[test]
    fn installed_node_uses_its_declared_icon() {
        let temp = tempfile::tempdir().expect("temp dir");
        let registry = make_registry(temp.path());
        write_composite_bundle(
            temp.path(),
            "icon-test",
            "icon-test",
            "n.x.icon_test.badge",
            "icons/custom.svg",
        );
        registry
            .refresh_project("superadmin", "default")
            .expect("refresh");
        let icon = registry
            .load_icon("superadmin", "default", "n.x.icon_test.badge")
            .expect("declared icon");
        assert_eq!(icon, b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>");
    }
}

// ── Third-party node interfaces ──────────────────────────────────────────────

/// Collects every third-party node kind a project's pipelines reference.
fn collect_referenced_third_party_kinds(
    current: &Path,
    kinds: &mut BTreeSet<String>,
) -> Result<(), PlatformError> {
    let entries = match std::fs::read_dir(current) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(PlatformError::new(
                "NODE_INTERFACE_SCAN",
                format!("failed reading {}: {error}", current.display()),
            ));
        }
    };
    for entry in entries {
        let entry = entry.map_err(|error| {
            PlatformError::new(
                "NODE_INTERFACE_SCAN",
                format!("failed reading entry in {}: {error}", current.display()),
            )
        })?;
        let path = entry.path();
        if path.is_dir() {
            collect_referenced_third_party_kinds(&path, kinds)?;
            continue;
        }
        if !path.extension().is_some_and(|ext| ext == "json") {
            continue;
        }
        let Ok(source) = std::fs::read(&path) else {
            continue;
        };
        // A pipeline that no longer decodes is the pipeline contract's problem,
        // not this scan's, so an unreadable file is skipped rather than fatal.
        let Ok(document) = decode_pipeline_graph(&source) else {
            continue;
        };
        for node in document.spec.nodes {
            if node.kind.starts_with(INSTALLED_NODE_KIND_PREFIX) {
                kinds.insert(node.kind);
            }
        }
    }
    Ok(())
}

impl NodeRegistryService {
    /// Materializes the interface of every third-party node the project uses.
    ///
    /// `zeb.lock` says which bundle a project needs and how to verify it, but a
    /// private or local source cannot always be re-obtained. Writing the frozen
    /// `NodeDefinition` into `repo/nodes/` means the graph still states what the
    /// node's pins, fields, and configuration are, so it stays readable and can
    /// be reimplemented instead of failing opaquely.
    ///
    /// Curated `n.*` nodes are omitted: the platform guarantees them, so they
    /// carry no portability risk.
    pub fn sync_project_node_interfaces(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<(), PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.projects.project_layout(&owner, &project)?;

        let mut referenced = BTreeSet::new();
        collect_referenced_third_party_kinds(&layout.repo_source_dir(), &mut referenced)?;

        let dir = &layout.repo_node_interfaces_dir();
        let mut expected = HashSet::new();
        for kind in &referenced {
            let Some(definition) = self.definition_for_kind(&owner, &project, kind) else {
                // The bundle is not installed here, so there is nothing to
                // write. Any interface already on disk is left alone; it is the
                // only remaining description of the node.
                expected.insert(interface_file_name(kind));
                continue;
            };
            let bytes = crate::contracts::kinds::encode_node_definition(
                ContractMetadata::named(kind),
                definition,
            )
            .map_err(|error| {
                PlatformError::new(
                    "NODE_INTERFACE_ENCODE",
                    format!("failed encoding interface for '{kind}': {error}"),
                )
            })?;
            let file_name = interface_file_name(kind);
            std::fs::create_dir_all(dir).map_err(|error| {
                PlatformError::new(
                    "NODE_INTERFACE_WRITE",
                    format!("failed creating {}: {error}", dir.display()),
                )
            })?;
            atomic_write(&dir.join(&file_name), &bytes)?;
            expected.insert(file_name);
        }

        // Drop interfaces for nodes the project no longer uses.
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if name.starts_with(INSTALLED_NODE_KIND_PREFIX)
                    && name.ends_with(".json")
                    && !expected.contains(&name)
                {
                    let _ = std::fs::remove_file(entry.path());
                }
            }
        }
        Ok(())
    }

    /// Returns the definition for one kind from any tier the project can see.
    fn definition_for_kind(
        &self,
        owner: &str,
        project: &str,
        kind: &str,
    ) -> Option<NodeDefinition> {
        self.get_manifest(owner, project, kind)
            .map(|manifest| manifest.definition)
    }
}

fn interface_file_name(kind: &str) -> String {
    format!("{kind}.json")
}

impl NodeRegistryService {
    /// Definitions carried as interfaces whose bundle is not installed here.
    ///
    /// These are the nodes a project can describe but not run. Returning them
    /// keeps a graph readable on an instance that cannot obtain the bundle,
    /// which is the entire reason the interface travels with the project.
    pub fn unavailable_interface_definitions(
        &self,
        owner: &str,
        project: &str,
    ) -> Vec<NodeDefinition> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let Ok(layout) = self.projects.project_layout(&owner, &project) else {
            return Vec::new();
        };
        let Ok(entries) = std::fs::read_dir(&layout.repo_node_interfaces_dir()) else {
            return Vec::new();
        };

        let mut definitions = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.extension().is_some_and(|ext| ext == "json") {
                continue;
            }
            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            let Ok(document) = crate::contracts::kinds::decode_node_definition(&bytes) else {
                continue;
            };
            // An interface only matters while its bundle is absent. Once the
            // bundle is installed the registry is the better answer.
            if self
                .get_manifest(&owner, &project, &document.spec.kind)
                .is_none()
            {
                definitions.push(document.spec);
            }
        }
        definitions.sort_by(|a, b| a.kind.cmp(&b.kind));
        definitions
    }
}
