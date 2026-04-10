//! Project migration plan and execution models.
//!
//! These models make migration a first-class architecture concept rather than an ad hoc operator
//! procedure. The first real implementation should use the `clone_first` strategy.

use serde::{Deserialize, Serialize};

use crate::infra::execution::placement::ProjectRuntimeProfile;

/// Current migration plan schema version.
pub const PROJECT_MIGRATION_PLAN_SCHEMA_VERSION: u32 = 1;
/// Current migration execution schema version.
pub const PROJECT_MIGRATION_EXECUTION_SCHEMA_VERSION: u32 = 1;

/// Conceptual migration strategy.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MigrationStrategy {
    CloneFirst,
    LiveHandoff,
}

/// Runtime data handling strategy during migration.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DataMigrationStrategy {
    RepoOnly,
    RepoAndRuntimeSnapshot,
    FullProject,
}

/// Secret remapping strategy during migration.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SecretsMigrationStrategy {
    RemapManually,
    ReuseByName,
    ReferencesOnly,
}

/// Traffic cutover strategy.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MigrationCutoverStrategy {
    Manual,
    Immediate,
    Planned,
}

/// Endpoint categories involved in a migration.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MigrationEndpointKind {
    LocalNode,
    Worker,
    Cluster,
    DedicatedRuntime,
    StandaloneInstall,
    IndependentPlatform,
}

/// Source or target endpoint for a migration.
#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq, Eq)]
pub struct MigrationEndpoint {
    /// Endpoint kind.
    pub kind: MigrationEndpointKind,
    /// Human-readable label.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub label: String,
    /// Optional platform identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub platform_id: Option<String>,
    /// Optional runner or worker identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runner_id: Option<String>,
    /// Optional network address or base URL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
}

impl Default for MigrationStrategy {
    fn default() -> Self {
        Self::CloneFirst
    }
}

impl Default for DataMigrationStrategy {
    fn default() -> Self {
        Self::RepoAndRuntimeSnapshot
    }
}

impl Default for SecretsMigrationStrategy {
    fn default() -> Self {
        Self::RemapManually
    }
}

impl Default for MigrationCutoverStrategy {
    fn default() -> Self {
        Self::Manual
    }
}

impl Default for MigrationEndpointKind {
    fn default() -> Self {
        Self::LocalNode
    }
}

/// Versioned migration plan.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectMigrationPlan {
    /// Plan schema version.
    #[serde(default = "default_project_migration_plan_schema_version")]
    pub schema_version: u32,
    /// Stable migration identifier.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub migration_id: String,
    /// Migration strategy.
    #[serde(default)]
    pub strategy: MigrationStrategy,
    /// Where the project is moving from.
    #[serde(default)]
    pub source: MigrationEndpoint,
    /// Where the project is moving to.
    #[serde(default)]
    pub target: MigrationEndpoint,
    /// Bundle identifier selected for the migration.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub bundle_id: String,
    /// Optional runtime snapshot identifier.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_snapshot_id: Option<String>,
    /// Desired runtime profile on the destination.
    #[serde(default)]
    pub desired_runtime: ProjectRuntimeProfile,
    /// Runtime data strategy.
    #[serde(default)]
    pub data_strategy: DataMigrationStrategy,
    /// Secret remapping strategy.
    #[serde(default)]
    pub secrets_strategy: SecretsMigrationStrategy,
    /// Cutover strategy.
    #[serde(default)]
    pub cutover_strategy: MigrationCutoverStrategy,
    /// Optional rollback window in seconds.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rollback_window_secs: Option<u64>,
    /// Optional freeform notes.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub notes: String,
}

const fn default_project_migration_plan_schema_version() -> u32 {
    PROJECT_MIGRATION_PLAN_SCHEMA_VERSION
}

impl Default for ProjectMigrationPlan {
    fn default() -> Self {
        Self {
            schema_version: PROJECT_MIGRATION_PLAN_SCHEMA_VERSION,
            migration_id: String::new(),
            strategy: MigrationStrategy::default(),
            source: MigrationEndpoint::default(),
            target: MigrationEndpoint::default(),
            bundle_id: String::new(),
            runtime_snapshot_id: None,
            desired_runtime: ProjectRuntimeProfile::default(),
            data_strategy: DataMigrationStrategy::default(),
            secrets_strategy: SecretsMigrationStrategy::default(),
            cutover_strategy: MigrationCutoverStrategy::default(),
            rollback_window_secs: None,
            notes: String::new(),
        }
    }
}

/// Current lifecycle state of a migration execution.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MigrationStatus {
    Draft,
    Planned,
    Syncing,
    AwaitingSecrets,
    Warming,
    ReadyForCutover,
    CuttingOver,
    RunningRollbackWindow,
    Completed,
    RolledBack,
    Failed,
}

impl Default for MigrationStatus {
    fn default() -> Self {
        Self::Draft
    }
}

/// Runtime execution record for one migration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MigrationExecution {
    /// Execution schema version.
    #[serde(default = "default_project_migration_execution_schema_version")]
    pub schema_version: u32,
    /// Stable migration identifier.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub migration_id: String,
    /// Current lifecycle state.
    #[serde(default)]
    pub status: MigrationStatus,
    /// Human-readable current step.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub current_step: String,
    /// Unix timestamp when the execution started.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<i64>,
    /// Unix timestamp of the most recent update.
    #[serde(default)]
    pub updated_at: i64,
    /// Unix timestamp when cutover happened.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cutover_at: Option<i64>,
    /// Unix timestamp when execution finished.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<i64>,
    /// Most recent human-readable error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}

const fn default_project_migration_execution_schema_version() -> u32 {
    PROJECT_MIGRATION_EXECUTION_SCHEMA_VERSION
}

impl Default for MigrationExecution {
    fn default() -> Self {
        Self {
            schema_version: PROJECT_MIGRATION_EXECUTION_SCHEMA_VERSION,
            migration_id: String::new(),
            status: MigrationStatus::Draft,
            current_step: String::new(),
            started_at: None,
            updated_at: 0,
            cutover_at: None,
            finished_at: None,
            last_error: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        MigrationStatus, MigrationStrategy, PROJECT_MIGRATION_EXECUTION_SCHEMA_VERSION,
        PROJECT_MIGRATION_PLAN_SCHEMA_VERSION, ProjectMigrationPlan,
    };

    #[test]
    fn migration_defaults_are_clone_first_and_versioned() {
        let plan = ProjectMigrationPlan::default();
        assert_eq!(plan.schema_version, PROJECT_MIGRATION_PLAN_SCHEMA_VERSION);
        assert_eq!(plan.strategy, MigrationStrategy::CloneFirst);

        let execution = super::MigrationExecution::default();
        assert_eq!(
            execution.schema_version,
            PROJECT_MIGRATION_EXECUTION_SCHEMA_VERSION
        );
        assert_eq!(execution.status, MigrationStatus::Draft);
    }
}
