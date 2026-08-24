//! Registered contract adapters. Runtime models remain in their owning modules.

mod database_schema;
mod dependency_lock;
mod hub_package;
mod hub_repository_index;
mod library_manifest;
mod map_publish_manifest;
mod node;
mod pipeline;
mod project_bundle;
mod project_configuration;
mod runtime_bundle;
mod zebfs_acl;

pub use database_schema::DatabaseSchemaContract;
pub use dependency_lock::{
    DEPENDENCY_LOCK_BACKUP_FILE, DEPENDENCY_LOCK_FILE, DependencyLockArtifactSpec,
    DependencyLockContract, DependencyLockNodeBundleSpec, DependencyLockNodesSpec,
    DependencyLockRweSpec, DependencyLockSource, DependencyLockSpec,
    MAX_DEPENDENCY_LOCK_BUNDLE_DEFINITIONS, MAX_DEPENDENCY_LOCK_BYTES,
    MAX_DEPENDENCY_LOCK_LIBRARIES, MAX_DEPENDENCY_LOCK_NODE_BUNDLES, decode_dependency_lock,
    encode_dependency_lock,
};
pub use hub_package::{
    HUB_PACKAGE_FILE_ENCODINGS, HubPackageArtifactRef, HubPackageContract, HubPackageFile,
    HubPackageFileSupply, HubPackageInitialDataStep, HubPackageInitialization, HubPackageLayout,
    HubPackageSpec, MAX_HUB_PACKAGE_ACTIVE_PIPELINES, MAX_HUB_PACKAGE_BYTES,
    MAX_HUB_PACKAGE_CARRIED_FILE_BYTES, MAX_HUB_PACKAGE_DESCRIPTION_BYTES, MAX_HUB_PACKAGE_FILES,
    MAX_HUB_PACKAGE_INITIAL_DATA_STEPS, MAX_HUB_PACKAGE_LIBRARIES,
    MAX_HUB_PACKAGE_REFERENCED_FILE_BYTES, MAX_HUB_PACKAGE_TEXT_BYTES, decode_hub_package,
    encode_hub_package,
};
pub use hub_repository_index::{
    HUB_REPOSITORY_INDEX_FILE, HubRepositoryIndexContract, HubRepositoryIndexPackage,
    HubRepositoryIndexRelease, HubRepositoryIndexSpec, MAX_HUB_REPOSITORY_INDEX_BYTES,
    MAX_HUB_REPOSITORY_INDEX_PACKAGES, MAX_HUB_REPOSITORY_INDEX_RELEASES,
    decode_hub_repository_index, encode_hub_repository_index,
};
pub use library_manifest::LibraryManifestContract;
pub use map_publish_manifest::{MapPublishManifestContract, MapserverLayerRecord};
pub use node::{
    BUNDLE_TRIGGER_TYPES, BundleScope, INSTALLED_NODE_KIND_PREFIX, MAX_NODE_BUNDLE_BYTES,
    MAX_NODE_BUNDLE_CREDENTIALS, MAX_NODE_BUNDLE_FILES, MAX_NODE_BUNDLE_FUNCTIONS,
    MAX_NODE_BUNDLE_HOSTS, MAX_NODE_BUNDLE_MODULES, MAX_NODE_BUNDLE_NODES,
    MAX_NODE_DEFINITION_BYTES, NodeBundleContract, NodeDefinitionContract, WASM_JSON_ABI_V1,
    decode_node_bundle, decode_node_definition, encode_node_bundle, encode_node_definition,
    normalize_node_bundle, package_kind_namespace, package_kind_token, validate_bundle_namespace,
    validate_node_definition_spec, validate_normalized_node_definition,
};
pub use pipeline::{
    MAX_PIPELINE_EDGES, MAX_PIPELINE_NODES, MAX_PIPELINE_SOURCE_BYTES, PipelineContract,
    PipelineEdgeSpec, PipelineInvocationRetentionSpec, PipelineMetadataSpec, PipelineNodeSpec,
    PipelineSettingsSpec, PipelineSpec, decode_pipeline_graph, encode_pipeline_graph,
    validate_pipeline_activation, validate_pipeline_graph,
};
pub use project_bundle::ProjectBundleContract;
pub use project_configuration::{
    LEGACY_PROJECT_CONFIGURATION_FILE, PROJECT_CONFIGURATION_BACKUP_FILE,
    PROJECT_CONFIGURATION_FILE, ProjectConfigurationContract, ProjectConfigurationSpec,
    ProjectInitialDataDirSpec, ProjectLayoutSpec, decode_legacy_project_configuration,
};
pub use runtime_bundle::RuntimeBundleContract;
pub use zebfs_acl::ZebFsAclContract;
