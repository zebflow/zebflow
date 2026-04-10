//! Project runtime bundle sync.
//!
//! This layer is responsible for copying the repo-owned runtime shape of a project from a master
//! node to a worker node without coupling the rest of the system to a specific transport.

pub mod apply;
pub mod bundle;
pub mod migration;
pub mod snapshot;

pub use apply::ProjectBundleApplyReport;
pub use bundle::{
    ProjectBootstrapPlan, ProjectBundleContents, ProjectBundleFile, ProjectBundleIdentity,
    ProjectRuntimeBundle, SecretBindingKind, SecretBindingRequirement, SecretBindingsManifest,
};
pub use migration::{
    DataMigrationStrategy, MigrationCutoverStrategy, MigrationEndpoint, MigrationEndpointKind,
    MigrationExecution, MigrationStatus, MigrationStrategy, ProjectMigrationPlan,
    SecretsMigrationStrategy,
};
pub use snapshot::{
    ProjectRuntimeSnapshot, RuntimeSnapshotFormat, RuntimeSnapshotPart, RuntimeSnapshotScope,
};
