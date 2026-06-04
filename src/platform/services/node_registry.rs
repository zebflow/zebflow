//! Runtime registry for installed composite and WASM node packages.
//!
//! Scans `repo/nodes/` for installed packages, validates manifests, and provides
//! merged node catalogs. Uses the same ArcSwap pattern as `PipelineRuntimeService`.
//!
//! Also loads official composite nodes embedded in the binary via
//! `PLATFORM_COMPOSITE_NODE_ASSETS`.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use arc_swap::ArcSwap;

use crate::pipeline::{NodeDefinition, PipelineGraph};
use crate::platform::error::PlatformError;
use crate::platform::model::{
    CredentialTypeDef, InstalledNodePackage, MultiNodePackageDefinition, NodePackageManifest,
    NodePackageSource, slug_segment,
};
use crate::platform::services::ProjectService;
use crate::platform::web::embedded::{
    PLATFORM_COMPOSITE_NODE_ASSETS, platform_composite_node_asset,
};

/// An official composite node embedded in the binary.
struct EmbeddedCompositeNode {
    manifest: NodePackageManifest,
    /// V1: pre-resolved pipeline bytes.
    pipeline_json: Option<&'static [u8]>,
    /// Package slug for resolving function pipelines from embedded assets.
    package_slug: String,
    icon_svg: Option<&'static [u8]>,
}

/// Runtime registry of installed node packages across all projects.
pub struct NodeRegistryService {
    projects: Arc<ProjectService>,
    /// Key = `"{owner}/{project}/{kind}"`.
    inner: ArcSwap<HashMap<String, InstalledNodePackage>>,
    /// Official composite nodes embedded in the binary, keyed by kind.
    embedded_composites: HashMap<String, EmbeddedCompositeNode>,
}

/// Explodes a multi-node `definition.json` into individual `NodePackageManifest` entries.
///
/// Each node entry gets its own manifest with the shared credentials and functions map.
fn explode_multi_node_package(
    pkg: &MultiNodePackageDefinition,
    slug: &str,
) -> Vec<NodePackageManifest> {
    let mut manifests = Vec::new();
    for node in &pkg.nodes {
        let definition = crate::pipeline::NodeDefinition {
            kind: node.kind.clone(),
            title: node.title.clone(),
            description: node.description.clone(),
            input_pins: node.definition.input_pins.clone(),
            output_pins: node.definition.output_pins.clone(),
            config_schema: node.definition.config_schema.clone(),
            fields: node.definition.fields.clone(),
            layout: node.definition.layout.clone(),
            dsl_flags: node.definition.dsl_flags.clone(),
            ui_category: node.ui_category.clone(),
            ui_category_label: node.ui_category_label.clone(),
            ..Default::default()
        };
        manifests.push(NodePackageManifest {
            source: NodePackageSource::Composite,
            version: pkg.version.clone(),
            definition,
            credentials: pkg.credentials.clone(),
            runtime: None,
            wasm_runtime: None,
            functions: pkg.functions.clone(),
            main_function: node.main.clone(),
            trigger: node.trigger.clone(),
            lifecycle: node.lifecycle.clone(),
            package_slug: slug.to_string(),
        });
    }
    manifests
}

fn registry_key(owner: &str, project: &str, kind: &str) -> String {
    format!("{}/{}/{}", owner, project, kind)
}

/// Derive a directory slug from a node kind.
///
/// `"n.c.telegram.send"` → `"telegram-send"` (strip `n.c.` / `n.wasm.` prefix, dots → dashes).
fn slug_from_kind(kind: &str) -> String {
    let stripped = kind
        .strip_prefix("n.c.")
        .or_else(|| kind.strip_prefix("n.wasm."))
        .unwrap_or(kind);
    stripped.replace('.', "-")
}

impl NodeRegistryService {
    pub fn new(projects: Arc<ProjectService>) -> Self {
        let embedded_composites = Self::load_embedded_composites();
        Self {
            projects,
            inner: ArcSwap::new(Arc::new(HashMap::new())),
            embedded_composites,
        }
    }

    /// Loads official composite nodes from `PLATFORM_COMPOSITE_NODE_ASSETS`.
    ///
    /// Supports both formats:
    /// - V1: `{slug}/node.json` + `{slug}/pipeline.zf.json` (single node)
    /// - Multi-node: `{slug}/definition.json` + `{slug}/functions/*.zf.json` (N nodes)
    fn load_embedded_composites() -> HashMap<String, EmbeddedCompositeNode> {
        let mut result = HashMap::new();

        // 1. Scan for multi-node packages: `{slug}/definition.json`.
        let multi_defs: Vec<(&str, &'static [u8])> = PLATFORM_COMPOSITE_NODE_ASSETS
            .iter()
            .filter(|a| a.path.ends_with("/definition.json"))
            .map(|a| {
                let slug = a.path.trim_end_matches("/definition.json");
                (slug, a.bytes)
            })
            .collect();

        let mut multi_slugs = HashSet::new();
        for (slug, def_bytes) in multi_defs {
            match serde_json::from_slice::<MultiNodePackageDefinition>(def_bytes) {
                Ok(pkg_def) => {
                    multi_slugs.insert(slug.to_string());
                    let manifests = explode_multi_node_package(&pkg_def, slug);
                    for manifest in manifests {
                        let kind = manifest.definition.kind.clone();
                        // Resolve per-node icon from embedded assets.
                        let icon_svg = {
                            // Try per-node icon first (from the node entry's icon field).
                            let node_entry = pkg_def.nodes.iter().find(|n| n.kind == kind);
                            let icon_path = node_entry
                                .and_then(|n| {
                                    if n.icon.is_empty() {
                                        None
                                    } else {
                                        Some(format!("{}/{}", slug, n.icon))
                                    }
                                })
                                .unwrap_or_else(|| format!("{}/{}", slug, pkg_def.icon));
                            platform_composite_node_asset(&icon_path)
                        };
                        result.insert(
                            kind,
                            EmbeddedCompositeNode {
                                manifest,
                                pipeline_json: None,
                                package_slug: slug.to_string(),
                                icon_svg,
                            },
                        );
                    }
                }
                Err(e) => {
                    eprintln!(
                        "node_registry: embedded multi-node package '{}': invalid definition.json: {}",
                        slug, e
                    );
                }
            }
        }

        // 2. Scan for V1 single-node packages: `{slug}/node.json`.
        let v1_manifests: Vec<(&str, &'static [u8])> = PLATFORM_COMPOSITE_NODE_ASSETS
            .iter()
            .filter(|a| a.path.ends_with("/node.json"))
            .map(|a| {
                let slug = a.path.trim_end_matches("/node.json");
                (slug, a.bytes)
            })
            .collect();

        for (slug, manifest_bytes) in v1_manifests {
            // Skip if this slug was already loaded as a multi-node package.
            if multi_slugs.contains(slug) {
                continue;
            }
            match serde_json::from_slice::<NodePackageManifest>(manifest_bytes) {
                Ok(manifest) => {
                    let pipeline_path = format!("{}/pipeline.zf.json", slug);
                    let pipeline_json = match platform_composite_node_asset(&pipeline_path) {
                        Some(b) => b,
                        None => {
                            eprintln!(
                                "node_registry: embedded composite '{}': missing pipeline.zf.json",
                                slug
                            );
                            continue;
                        }
                    };
                    let icon_path = format!("{}/icon.svg", slug);
                    let icon_svg = platform_composite_node_asset(&icon_path);
                    let kind = manifest.definition.kind.clone();
                    result.insert(
                        kind,
                        EmbeddedCompositeNode {
                            manifest,
                            pipeline_json: Some(pipeline_json),
                            package_slug: slug.to_string(),
                            icon_svg,
                        },
                    );
                }
                Err(e) => {
                    eprintln!(
                        "node_registry: embedded composite '{}': invalid node.json: {}",
                        slug, e
                    );
                }
            }
        }
        result
    }

    /// Scans `repo/nodes/` for a project and rebuilds its registry entries.
    pub fn refresh_project(&self, owner: &str, project: &str) -> Result<(), PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);

        let layout = self.projects.project_layout(&owner, &project)?;
        let nodes_dir = &layout.repo_nodes_dir;

        // Collect builtin kinds to reject collisions.
        let builtin_kinds: HashSet<String> = crate::pipeline::nodes::builtin_node_definitions()
            .iter()
            .map(|d| d.kind.clone())
            .collect();

        let mut new_entries: Vec<(String, InstalledNodePackage)> = Vec::new();

        if nodes_dir.is_dir() {
            let entries = std::fs::read_dir(nodes_dir).map_err(|e| {
                PlatformError::new(
                    "NODE_REGISTRY_SCAN",
                    format!("failed reading {}: {}", nodes_dir.display(), e),
                )
            })?;

            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let slug = entry
                    .file_name()
                    .to_string_lossy()
                    .to_string();

                // Check for multi-node definition.json first, then v1 node.json.
                let definition_path = path.join("definition.json");
                let manifest_path = path.join("node.json");

                if definition_path.is_file() {
                    // Multi-node package.
                    match parse_multi_node_definition(&definition_path, &slug, &builtin_kinds) {
                        Ok(manifests) => {
                            for manifest in manifests {
                                let has_icon = !manifest.package_slug.is_empty();
                                let kind = manifest.definition.kind.clone();
                                let key = registry_key(&owner, &project, &kind);
                                new_entries.push((
                                    key,
                                    InstalledNodePackage {
                                        slug: slug.clone(),
                                        owner: owner.clone(),
                                        project: project.clone(),
                                        manifest,
                                        package_dir: path.display().to_string(),
                                        has_icon,
                                    },
                                ));
                            }
                        }
                        Err(e) => {
                            eprintln!(
                                "node_registry: skipping multi-node package '{}' in {}/{}: {}",
                                slug, owner, project, e.message
                            );
                        }
                    }
                } else if manifest_path.is_file() {
                    // V1 single-node package.
                    match parse_and_validate_manifest(&manifest_path, &builtin_kinds) {
                        Ok(manifest) => {
                            let has_icon = path.join("icon.svg").is_file();
                            let kind = manifest.definition.kind.clone();
                            let key = registry_key(&owner, &project, &kind);
                            new_entries.push((
                                key,
                                InstalledNodePackage {
                                    slug,
                                    owner: owner.clone(),
                                    project: project.clone(),
                                    manifest,
                                    package_dir: path.display().to_string(),
                                    has_icon,
                                },
                            ));
                        }
                        Err(e) => {
                            eprintln!(
                                "node_registry: skipping package '{}' in {}/{}: {}",
                                slug, owner, project, e.message
                            );
                        }
                    }
                }
            }
        }

        // Swap: remove old entries for this project, add new ones.
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
    /// For multi-node packages, resolves the `main` function name to a pipeline file.
    /// For v1 packages, uses `runtime.pipeline` directly.
    ///
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
                format!("composite node '{}' not installed in {}/{}", kind, owner, project),
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
            .or_else(|| embedded.manifest.main_function.clone());

        if let Some(fn_name) = &fn_name {
            // Multi-node: resolve function name → file path → embedded asset.
            let fn_path = embedded.manifest.functions.get(fn_name.as_str()).ok_or_else(|| {
                PlatformError::new(
                    "NODE_COMPOSITE_FUNCTION_NOT_FOUND",
                    format!("composite '{}': function '{}' not in functions map", kind, fn_name),
                )
            })?;
            let asset_path = format!("{}/{}", embedded.package_slug, fn_path);
            let bytes = platform_composite_node_asset(&asset_path).ok_or_else(|| {
                PlatformError::new(
                    "NODE_COMPOSITE_PIPELINE_READ",
                    format!("embedded composite '{}': missing asset '{}'", kind, asset_path),
                )
            })?;
            let graph: PipelineGraph = serde_json::from_slice(bytes).map_err(|e| {
                PlatformError::new(
                    "NODE_COMPOSITE_PIPELINE_PARSE",
                    format!("embedded composite '{}' function '{}': {}", kind, fn_name, e),
                )
            })?;
            Ok(graph)
        } else if let Some(pipeline_json) = embedded.pipeline_json {
            // V1: single pre-resolved pipeline.
            let graph: PipelineGraph =
                serde_json::from_slice(pipeline_json).map_err(|e| {
                    PlatformError::new(
                        "NODE_COMPOSITE_PIPELINE_PARSE",
                        format!("embedded composite '{}': {}", kind, e),
                    )
                })?;
            Ok(graph)
        } else if let Some(runtime) = &embedded.manifest.runtime {
            // V1 fallback via runtime.pipeline.
            let asset_path = format!("{}/{}", embedded.package_slug, runtime.pipeline);
            let bytes = platform_composite_node_asset(&asset_path).ok_or_else(|| {
                PlatformError::new(
                    "NODE_COMPOSITE_PIPELINE_READ",
                    format!("embedded composite '{}': missing '{}'", kind, asset_path),
                )
            })?;
            let graph: PipelineGraph = serde_json::from_slice(bytes).map_err(|e| {
                PlatformError::new(
                    "NODE_COMPOSITE_PIPELINE_PARSE",
                    format!("embedded composite '{}': {}", kind, e),
                )
            })?;
            Ok(graph)
        } else {
            Err(PlatformError::new(
                "NODE_COMPOSITE_NO_RUNTIME",
                format!("composite '{}' has no main function or runtime pipeline", kind),
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
            .or_else(|| pkg.manifest.main_function.clone());

        let pipeline_path = if let Some(fn_name) = &fn_name {
            // Multi-node: resolve function name → file path.
            let fn_path = pkg.manifest.functions.get(fn_name.as_str()).ok_or_else(|| {
                PlatformError::new(
                    "NODE_COMPOSITE_FUNCTION_NOT_FOUND",
                    format!("node '{}': function '{}' not in functions map", kind, fn_name),
                )
            })?;
            std::path::Path::new(&pkg.package_dir).join(fn_path)
        } else if let Some(runtime) = &pkg.manifest.runtime {
            // V1: runtime.pipeline.
            std::path::Path::new(&pkg.package_dir).join(&runtime.pipeline)
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
        let graph: PipelineGraph = serde_json::from_str(&source).map_err(|e| {
            PlatformError::new(
                "NODE_COMPOSITE_PIPELINE_PARSE",
                format!("failed parsing '{}': {}", pipeline_path.display(), e),
            )
        })?;
        Ok(graph)
    }

    /// Returns the icon SVG bytes for an installed node, if present.
    pub fn load_icon(
        &self,
        owner: &str,
        project: &str,
        kind: &str,
    ) -> Option<Vec<u8>> {
        let pkg = self.get_by_kind(owner, project, kind)?;
        if !pkg.has_icon {
            return None;
        }
        let icon_path = std::path::Path::new(&pkg.package_dir).join("icon.svg");
        std::fs::read(&icon_path).ok()
    }

    /// Returns icon bytes for a node kind from any registry source:
    /// installed packages first, then embedded official composites.
    ///
    /// Builtin native node icons (from `PLATFORM_NODE_ICON_ASSETS`) are handled
    /// separately in the API layer.
    pub fn load_icon_any(
        &self,
        owner: &str,
        project: &str,
        kind: &str,
    ) -> Option<Vec<u8>> {
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
        // Native nodes (non-composite, non-wasm) are always official.
        if !kind.starts_with("n.c.") && !kind.starts_with("n.wasm.") {
            return true;
        }
        // Embedded composites are official.
        self.embedded_composites.contains_key(kind)
    }

    /// Installs a node package into the project.
    pub fn install_package(
        &self,
        owner: &str,
        project: &str,
        manifest: &NodePackageManifest,
        pipeline_source: Option<&str>,
        icon_svg: Option<&str>,
    ) -> Result<InstalledNodePackage, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);

        // Validate.
        let builtin_kinds: HashSet<String> = crate::pipeline::nodes::builtin_node_definitions()
            .iter()
            .map(|d| d.kind.clone())
            .collect();
        validate_manifest(manifest, &builtin_kinds)?;

        let slug = slug_from_kind(&manifest.definition.kind);
        let layout = self.projects.project_layout(&owner, &project)?;
        let pkg_dir = layout.repo_nodes_dir.join(&slug);

        // Create directory.
        std::fs::create_dir_all(&pkg_dir).map_err(|e| {
            PlatformError::new(
                "NODE_INSTALL_DIR",
                format!("failed creating {}: {}", pkg_dir.display(), e),
            )
        })?;

        // Write node.json.
        let manifest_json = serde_json::to_string_pretty(manifest).map_err(|e| {
            PlatformError::new("NODE_INSTALL_SERIALIZE", e.to_string())
        })?;
        std::fs::write(pkg_dir.join("node.json"), &manifest_json).map_err(|e| {
            PlatformError::new(
                "NODE_INSTALL_WRITE",
                format!("failed writing node.json: {}", e),
            )
        })?;

        // Write pipeline.zf.json for composite.
        if let Some(source) = pipeline_source {
            let filename = manifest
                .runtime
                .as_ref()
                .map(|r| r.pipeline.as_str())
                .unwrap_or("pipeline.zf.json");
            std::fs::write(pkg_dir.join(filename), source).map_err(|e| {
                PlatformError::new(
                    "NODE_INSTALL_WRITE",
                    format!("failed writing {}: {}", filename, e),
                )
            })?;
        }

        // Write icon.svg.
        let has_icon = if let Some(svg) = icon_svg {
            std::fs::write(pkg_dir.join("icon.svg"), svg).map_err(|e| {
                PlatformError::new(
                    "NODE_INSTALL_WRITE",
                    format!("failed writing icon.svg: {}", e),
                )
            })?;
            true
        } else {
            false
        };

        // Refresh registry.
        self.refresh_project(&owner, &project)?;

        Ok(InstalledNodePackage {
            slug,
            owner: owner.clone(),
            project: project.clone(),
            manifest: manifest.clone(),
            package_dir: pkg_dir.display().to_string(),
            has_icon,
        })
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
        Ok(())
    }
}

/// Parse a multi-node `definition.json` from disk and explode into manifests.
fn parse_multi_node_definition(
    def_path: &std::path::Path,
    slug: &str,
    builtin_kinds: &HashSet<String>,
) -> Result<Vec<NodePackageManifest>, PlatformError> {
    let raw = std::fs::read_to_string(def_path).map_err(|e| {
        PlatformError::new(
            "NODE_MANIFEST_READ",
            format!("failed reading {}: {}", def_path.display(), e),
        )
    })?;
    let pkg_def: MultiNodePackageDefinition = serde_json::from_str(&raw).map_err(|e| {
        PlatformError::new(
            "NODE_MANIFEST_PARSE",
            format!("invalid definition.json at {}: {}", def_path.display(), e),
        )
    })?;
    let manifests = explode_multi_node_package(&pkg_def, slug);
    for manifest in &manifests {
        validate_manifest(manifest, builtin_kinds)?;
    }
    Ok(manifests)
}

/// Parse and validate a `node.json` manifest from disk.
fn parse_and_validate_manifest(
    manifest_path: &std::path::Path,
    builtin_kinds: &HashSet<String>,
) -> Result<NodePackageManifest, PlatformError> {
    let raw = std::fs::read_to_string(manifest_path).map_err(|e| {
        PlatformError::new(
            "NODE_MANIFEST_READ",
            format!("failed reading {}: {}", manifest_path.display(), e),
        )
    })?;
    let manifest: NodePackageManifest = serde_json::from_str(&raw).map_err(|e| {
        PlatformError::new(
            "NODE_MANIFEST_PARSE",
            format!(
                "invalid node.json at {}: {}",
                manifest_path.display(),
                e
            ),
        )
    })?;
    validate_manifest(&manifest, builtin_kinds)?;
    Ok(manifest)
}

/// Validate a node package manifest against namespace and collision rules.
fn validate_manifest(
    manifest: &NodePackageManifest,
    builtin_kinds: &HashSet<String>,
) -> Result<(), PlatformError> {
    let kind = &manifest.definition.kind;

    // Must have non-empty kind and title.
    if kind.is_empty() {
        return Err(PlatformError::new(
            "NODE_MANIFEST_INVALID",
            "node definition kind must not be empty",
        ));
    }
    if manifest.definition.title.is_empty() {
        return Err(PlatformError::new(
            "NODE_MANIFEST_INVALID",
            format!("node '{}' definition title must not be empty", kind),
        ));
    }

    // Namespace rules.
    match manifest.source {
        NodePackageSource::Composite => {
            if !kind.starts_with("n.c.") {
                return Err(PlatformError::new(
                    "NODE_NAMESPACE_VIOLATION",
                    format!(
                        "composite node kind '{}' must start with 'n.c.'",
                        kind
                    ),
                ));
            }
            // Must have runtime.pipeline (v1) or main_function/trigger (multi-node).
            if manifest.runtime.is_none()
                && manifest.main_function.is_none()
                && manifest.trigger.is_none()
            {
                return Err(PlatformError::new(
                    "NODE_MANIFEST_INVALID",
                    format!(
                        "composite node '{}' must have runtime.pipeline, main function, or trigger",
                        kind
                    ),
                ));
            }
        }
        NodePackageSource::Wasm => {
            if !kind.starts_with("n.wasm.") {
                return Err(PlatformError::new(
                    "NODE_NAMESPACE_VIOLATION",
                    format!(
                        "WASM node kind '{}' must start with 'n.wasm.'",
                        kind
                    ),
                ));
            }
        }
    }

    // Collision check.
    if builtin_kinds.contains(kind) {
        return Err(PlatformError::new(
            "NODE_KIND_COLLISION",
            format!(
                "node kind '{}' collides with a built-in native node",
                kind
            ),
        ));
    }

    Ok(())
}
