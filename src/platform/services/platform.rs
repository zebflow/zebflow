//! Platform composition root for adapters + services.

use std::sync::Arc;

use crate::infra::execution::runner::RunnerCapabilities;
use crate::infra::io::state::{DynStateBus, MemStateBus};
use crate::infra::mem::MemHub;
use crate::infra::transport::ws::WsHub;
use crate::platform::adapters::data::{DataAdapter, build_data_adapter, build_hub_data_adapter};
use crate::platform::adapters::file::{FileAdapter, build_file_adapter};
use crate::platform::adapters::project_data::{ProjectDataFactory, build_project_data_factory};
use crate::platform::error::PlatformError;
use crate::platform::model::{
    CreateProjectRequest, CreateUserRequest, PipelineInvocationEntry, PlatformConfig,
    PlatformOffice, PlatformOfficeNode, now_ts,
};
use crate::platform::services::{
    AssistantConfigService, AuthService, AuthorizationService, ClusterBootstrapService,
    ClusterPlacementService, ClusterRegistryService, ClusterRuntimeSyncService, CredentialService,
    DbConnectionService, DbRuntimeService, GitIdentityService, HubService, LibraryService,
    McpSessionService, NodeRegistryService, PipelineHitsService, PipelineRuntimeService,
    ProjectInviteService, ProjectMembershipService, ProjectOperationService, ProjectService,
    ProjectTransferService, UserService, ZebLockService, ZebflowJsonService,
};

/// Main platform service graph, created once per process.
#[derive(Clone)]
pub struct PlatformService {
    /// Effective config.
    pub config: PlatformConfig,
    /// Metadata backend.
    pub data: Arc<dyn DataAdapter>,
    /// File/project backend.
    pub file: Arc<dyn FileAdapter>,
    /// Project runtime data factory.
    pub project_data: Arc<dyn ProjectDataFactory>,
    /// User domain service.
    pub users: Arc<UserService>,
    /// Auth domain service.
    pub auth: Arc<AuthService>,
    /// User-bound git author identity resolution.
    pub git_identity: Arc<GitIdentityService>,
    /// Project-level authorization service shared by REST/MCP/assistant.
    pub authz: Arc<AuthorizationService>,
    /// Project-sharing membership service.
    pub project_members: Arc<ProjectMembershipService>,
    /// Project-sharing invite service.
    pub project_invites: Arc<ProjectInviteService>,
    /// Cluster role/bootstrap service.
    pub cluster_bootstrap: Arc<ClusterBootstrapService>,
    /// Worker registry service.
    pub cluster_registry: Arc<ClusterRegistryService>,
    /// Project placement service.
    pub cluster_placement: Arc<ClusterPlacementService>,
    /// Project runtime bundle sync service.
    pub cluster_runtime_sync: Arc<ClusterRuntimeSyncService>,
    /// Project credential management service.
    pub credentials: Arc<CredentialService>,
    /// Project assistant config service.
    pub assistant_configs: Arc<AssistantConfigService>,
    /// Layer 2 project config (zebflow.json) service.
    pub zebflow_cfg: Arc<ZebflowJsonService>,
    /// Project DB connection management service.
    pub db_connections: Arc<DbConnectionService>,
    /// Project DB runtime service (kind-dispatched describe/query).
    pub db_runtime: Arc<DbRuntimeService>,
    /// Project domain service.
    pub projects: Arc<ProjectService>,
    /// Durable controller-side operation log for portability and sync actions.
    pub project_operations: Arc<ProjectOperationService>,
    /// Archive-based export/import service for project portability.
    pub project_transfer: Arc<ProjectTransferService>,
    /// Active production pipeline registry compiled from activated snapshots.
    pub pipeline_runtime: Arc<PipelineRuntimeService>,
    /// Runtime registry of installed composite/WASM node packages.
    pub node_registry: Arc<NodeRegistryService>,
    /// Lightweight execution hit/error counters per pipeline.
    pub pipeline_hits: Arc<PipelineHitsService>,
    /// MCP session management (in-memory tokens for project-scoped remote control).
    pub mcp_sessions: Arc<McpSessionService>,
    /// WebSocket hub — real-time room management for WS pipelines.
    pub ws_hub: Arc<WsHub>,
    /// In-memory KV + pub/sub hub for n.kv.* pipeline nodes.
    pub mem_hub: Arc<MemHub>,
    /// Shared state-bus seam currently backed by the same in-process mem hub.
    pub state_bus: DynStateBus,
    /// In-memory registry of embedded `zeb/*` library manifests.
    pub library: Arc<LibraryService>,
    /// Platform-level asset hub service.
    pub hub: Arc<HubService>,
    /// Read/write service for per-project `repo/zeb.lock`.
    pub zeb_lock: Arc<ZebLockService>,
}

impl PlatformService {
    /// Builds platform from config and runs bootstrap initialization.
    pub fn from_config(config: PlatformConfig) -> Result<Self, PlatformError> {
        std::fs::create_dir_all(&config.data_root)?;
        let data = build_data_adapter(config.data_adapter, &config.data_root)?;
        let file = build_file_adapter(config.file_adapter, config.data_root.clone());
        let project_data = build_project_data_factory(&config.data_root);
        file.initialize()?;

        let zebflow_cfg = Arc::new(ZebflowJsonService::new(config.data_root.join("users")));
        let zeb_lock = Arc::new(ZebLockService::new(config.data_root.join("users")));
        let library = Arc::new(LibraryService::from_embedded());
        let users = Arc::new(UserService::new(data.clone()));
        let projects = Arc::new(ProjectService::new(
            data.clone(),
            file.clone(),
            project_data.clone(),
            zebflow_cfg.clone(),
            zeb_lock.clone(),
        ));
        let auth = Arc::new(AuthService::new(users.clone()));
        let git_identity = Arc::new(GitIdentityService::new(users.clone()));
        let authz = Arc::new(AuthorizationService::new(data.clone()));
        let project_members = Arc::new(ProjectMembershipService::new(data.clone(), authz.clone()));
        let project_invites = Arc::new(ProjectInviteService::new(data.clone()));
        let credentials = Arc::new(CredentialService::new(data.clone(), reqwest::Client::new()));
        let assistant_configs = Arc::new(AssistantConfigService::new(
            data.clone(),
            zebflow_cfg.clone(),
        ));
        let db_connections = Arc::new(DbConnectionService::new(data.clone()));
        let db_runtime = Arc::new(DbRuntimeService::new(
            db_connections.clone(),
            credentials.clone(),
            config.data_root.clone(),
        ));
        let hub_data = build_hub_data_adapter(
            config.data_adapter,
            &config
                .data_root
                .join("services")
                .join(crate::platform::services::hub::DEFAULT_HUB_SERVICE_INSTANCE_ID)
                .join("hub.db"),
        )?;
        let hub = Arc::new(HubService::new(
            data.clone(),
            hub_data,
            projects.clone(),
            config.data_root.clone(),
        ));
        let project_operations = Arc::new(ProjectOperationService::new(data.clone()));
        let project_transfer = Arc::new(ProjectTransferService::new(
            file.clone(),
            zebflow_cfg.clone(),
            config.data_root.join("platform").join("project-operations"),
        ));
        let pipeline_runtime = Arc::new(PipelineRuntimeService::new(projects.clone()));
        let node_registry = Arc::new(NodeRegistryService::new(projects.clone()));
        let pipeline_hits = Arc::new(PipelineHitsService::new(10));
        let mcp_sessions = Arc::new(McpSessionService::new(
            data.clone(),
            config.secret_rotation_epoch,
        ));
        let ws_hub = Arc::new(WsHub::new());
        let mem_hub = Arc::new(MemHub::new());
        let state_bus: DynStateBus = Arc::new(MemStateBus::from_hub_with_durable(
            (*mem_hub).clone(),
            config.data_root.clone(),
        ));
        let cluster_bootstrap = Arc::new(ClusterBootstrapService::new(config.cluster.clone()));
        let cluster_registry = Arc::new(ClusterRegistryService::new(data.clone()));
        let cluster_placement = Arc::new(ClusterPlacementService::new(data.clone()));
        let cluster_runtime_sync = Arc::new(ClusterRuntimeSyncService::new(
            data.clone(),
            file.clone(),
            projects.clone(),
            zebflow_cfg.clone(),
            pipeline_runtime.clone(),
        ));

        let svc = Self {
            config,
            data,
            file,
            project_data,
            users,
            auth,
            git_identity,
            authz,
            project_members,
            project_invites,
            cluster_bootstrap,
            cluster_registry,
            cluster_placement,
            cluster_runtime_sync,
            credentials,
            assistant_configs,
            zebflow_cfg,
            db_connections,
            db_runtime,
            projects,
            project_operations,
            project_transfer,
            pipeline_runtime,
            node_registry,
            pipeline_hits,
            mcp_sessions,
            ws_hub,
            mem_hub,
            state_bus,
            library,
            hub,
            zeb_lock,
        };
        svc.bootstrap_local_office()?;
        if !svc.cluster_bootstrap.is_worker() {
            svc.bootstrap_defaults()?;
        }
        // Reload active pipelines for every project across all users.
        if let Ok(users) = svc.data.list_users() {
            for user in &users {
                if let Ok(projects) = svc.projects.list_projects(&user.owner) {
                    for project in &projects {
                        let _ = svc
                            .node_registry
                            .refresh_project(&user.owner, &project.project);
                        if let Err(e) = svc
                            .pipeline_runtime
                            .refresh_project(&user.owner, &project.project)
                        {
                            eprintln!(
                                "⚠ pipeline bootstrap {}/{}: {}",
                                user.owner, project.project, e.message
                            );
                        }
                    }
                }
            }
        }
        Ok(svc)
    }

    fn bootstrap_local_office(&self) -> Result<(), PlatformError> {
        let office_id = self.cluster_bootstrap.node_id();
        if office_id.trim().is_empty() {
            return Ok(());
        }
        let now = now_ts();
        let office_kind = if self.cluster_bootstrap.is_standalone() {
            "standalone"
        } else if self.cluster_bootstrap.is_master() {
            "controller"
        } else {
            "office"
        };
        let base_url = self
            .cluster_bootstrap
            .advertise_url()
            .unwrap_or_default()
            .trim_end_matches('/')
            .to_string();
        self.data.put_platform_office(&PlatformOffice {
            office_id: office_id.clone(),
            office_slug: office_id.clone(),
            label: self.cluster_bootstrap.node_label(),
            office_kind: office_kind.to_string(),
            base_url: base_url.clone(),
            status: "online".to_string(),
            created_at: now,
            updated_at: now,
        })?;
        self.data.put_platform_office_node(&PlatformOfficeNode {
            office_id,
            node_id: self.cluster_bootstrap.node_id(),
            label: self.cluster_bootstrap.node_label(),
            base_url,
            status: "online".to_string(),
            capabilities: RunnerCapabilities::default(),
            registered_at: now,
            last_heartbeat_at: now,
        })?;
        Ok(())
    }

    /// Execute an active function pipeline by slug and return its output value.
    ///
    /// Called from `n.function.call` nodes during pipeline execution.
    /// The slug is matched against active pipelines that have an `n.trigger.function` entry node.
    pub async fn execute_function_pipeline(
        &self,
        owner: &str,
        project: &str,
        slug: &str,
        input: serde_json::Value,
    ) -> Result<serde_json::Value, crate::pipeline::PipelineError> {
        use crate::pipeline::PipelineEngine;
        use crate::platform::services::project::name_from_file_rel_path;

        const FUNCTION_TRIGGER_KIND: &str = "n.trigger.function";

        // Find the active function pipeline by slug.
        let compiled = self
            .pipeline_runtime
            .list_project(owner, project)
            .into_iter()
            .find(|c| {
                name_from_file_rel_path(&c.file_rel_path) == slug
                    && c.graph
                        .nodes
                        .iter()
                        .any(|n| n.kind == FUNCTION_TRIGGER_KIND)
            })
            .ok_or_else(|| {
                crate::pipeline::PipelineError::new(
                    "FW_FUNCTION_NOT_FOUND",
                    format!(
                        "function pipeline '{}' not found or not active in {}/{}",
                        slug, owner, project
                    ),
                )
            })?;

        let trigger_config = compiled
            .graph
            .nodes
            .iter()
            .find(|n| n.kind == FUNCTION_TRIGGER_KIND)
            .map(|n| n.config.clone())
            .unwrap_or_default();
        let input_schema =
            crate::pipeline::nodes::basic::trigger::function::input_schema_from_config(
                &trigger_config,
            );
        if let Err(err_payload) =
            crate::pipeline::nodes::basic::trigger::function::validate_function_input(
                &input_schema,
                &input,
            )
        {
            return Err(crate::pipeline::PipelineError::new(
                "FW_FUNCTION_INPUT_INVALID",
                serde_json::to_string(&err_payload).unwrap_or_else(|_| err_payload.to_string()),
            ));
        }

        let ctx = crate::pipeline::PipelineContext {
            owner: owner.to_string(),
            project: project.to_string(),
            pipeline: compiled.graph.id.clone(),
            request_id: format!(
                "fn-call-{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            ),
            route: Default::default(),
            input,
            trigger: None,
            placeholder: None,
        };

        let engine = crate::pipeline::BasicPipelineEngine::new(
            std::sync::Arc::new(crate::language::DenoSandboxEngine::default()),
            crate::rwe::resolve_engine_or_default(None),
            Some(self.credentials.clone()),
        )
        .with_platform(std::sync::Arc::new(self.clone()))
        .with_ws_hub(self.ws_hub.clone())
        .with_state_bus(self.state_bus.clone())
        .with_data_root(self.config.data_root.clone());

        let file_rel_path = compiled.file_rel_path.clone();

        // Retention settings for invocation log.
        let project_cfg = self.zebflow_cfg.read_or_default(owner, project);
        let max_invocations = project_cfg
            .configs
            .pipelines
            .logging
            .effective_max_invocations();
        let max_age_secs: Option<i64> = compiled
            .graph
            .metadata
            .as_ref()
            .and_then(|m| m.settings.invocation_retention.as_ref())
            .and_then(|r| r.max_age_secs)
            .map(|v| v.max(1) as i64);
        let effective_max = compiled
            .graph
            .metadata
            .as_ref()
            .and_then(|m| m.settings.invocation_retention.as_ref())
            .and_then(|r| r.max_invocations)
            .map(|v| v.max(1) as usize)
            .unwrap_or(max_invocations);

        let exec_start = std::time::Instant::now();
        let at = crate::platform::model::now_ts();

        match engine.execute_async(&compiled.graph, &ctx).await {
            Ok(output) => {
                let duration_ms = exec_start.elapsed().as_millis() as u64;
                self.pipeline_hits
                    .record_success(owner, project, &file_rel_path);
                let _ = self.data.log_pipeline_invocation(
                    owner,
                    project,
                    &file_rel_path,
                    &PipelineInvocationEntry {
                        run_id: ctx.request_id.clone(),
                        at,
                        duration_ms,
                        status: "ok".to_string(),
                        trigger: "function.call".to_string(),
                        error: None,
                        trace: output.node_trace,
                    },
                    effective_max,
                    max_age_secs,
                );
                Ok(output.value)
            }
            Err(e) => {
                let duration_ms = exec_start.elapsed().as_millis() as u64;
                self.pipeline_hits.record_failure(
                    owner,
                    project,
                    &file_rel_path,
                    "function.call",
                    &e.code,
                    &e.message,
                );
                let _ = self.data.log_pipeline_invocation(
                    owner,
                    project,
                    &file_rel_path,
                    &PipelineInvocationEntry {
                        run_id: ctx.request_id.clone(),
                        at,
                        duration_ms,
                        status: "error".to_string(),
                        trigger: "function.call".to_string(),
                        error: Some(e.message.clone()),
                        trace: e.node_trace.clone(),
                    },
                    effective_max,
                    max_age_secs,
                );
                Err(e)
            }
        }
    }

    /// Creates default superadmin + default project if missing.
    pub fn bootstrap_defaults(&self) -> Result<(), PlatformError> {
        if self.config.default_password.trim().is_empty() {
            return Err(PlatformError::new(
                "PLATFORM_BOOTSTRAP_PASSWORD_MISSING",
                "default superadmin password is missing; set ZEBFLOW_PLATFORM_DEFAULT_PASSWORD or provide PlatformConfig.default_password",
            ));
        }

        self.users.create_or_update_user(&CreateUserRequest {
            owner: self.config.default_owner.clone(),
            password: self.config.default_password.clone(),
            role: "superadmin".to_string(),
            git_name: String::new(),
            git_email: String::new(),
        })?;

        self.projects.create_or_update_project(
            &self.config.default_owner,
            &CreateProjectRequest {
                project: self.config.default_project.clone(),
                title: Some("Default".to_string()),
                local_branch: None,
                runtime: Default::default(),
            },
        )?;
        Ok(())
    }
}
