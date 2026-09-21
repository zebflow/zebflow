//! Platform composition root for adapters + services.

use std::sync::Arc;

use crate::infra::cluster::config::ClusterRole;
use crate::infra::execution::runner::RunnerCapabilities;
use crate::infra::io::state::{DynStateBus, MemStateBus};
use crate::infra::mem::MemHub;
use crate::infra::transport::ws::WsHub;
use crate::language::DenoSandboxEngine;
use crate::platform::adapters::data::{DataAdapter, build_data_adapter, build_hub_data_adapter};
use crate::platform::adapters::file::{FileAdapter, build_file_adapter};
use crate::platform::adapters::project_data::{ProjectDataFactory, build_project_data_factory};
use crate::platform::error::PlatformError;
use crate::platform::model::{
    CreateProjectRequest, CreateUserRequest, PipelineInvocationEntry, PlatformConfig,
    PlatformOffice, PlatformOfficeNode, now_ts,
};
use crate::platform::services::bootstrap::resolve_superadmin_password;
use crate::platform::services::{
    AssistantConfigService, AuthService, AuthorizationService, ClusterBootstrapService,
    ClusterJoinTokenService, ClusterPlacementService, ClusterRegistryService,
    ClusterRuntimeSyncService, CredentialService, DbConnectionService, DbRuntimeService,
    DependencyLockService, GitIdentityService, HubService, LibraryService, McpSessionService,
    NodeRegistryService, OfficeLocalAuthorityService, PipelineHitsService, PipelineRuntimeService,
    ProjectConfigurationService, ProjectInviteService, ProjectMembershipService,
    ProjectOperationService, ProjectService, ProjectTransferService, UserService,
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
    /// Per-office join token minting, verification, and this office's identity.
    pub cluster_join_tokens: Arc<ClusterJoinTokenService>,
    /// Whether this office's own accounts may open its front door
    /// (`offices.md` §4 login term, §6 break-glass, §7 detach).
    pub local_authority: Arc<OfficeLocalAuthorityService>,
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
    /// Where each project answers: hosts, routes, surfaces (`addressing.md`).
    pub addressing: Arc<crate::platform::services::addressing::AddressingService>,
    /// Portable project configuration (`zebflow.yaml`) service.
    pub zebflow_cfg: Arc<ProjectConfigurationService>,
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
    /// Fonts: the bundled default set and each project's `static/fonts/`.
    pub fonts: Arc<crate::platform::services::FontService>,
    /// Platform-level asset hub service.
    pub hub: Arc<HubService>,
    /// Read/write service for per-project `repo/zeb.lock`.
    pub dependency_lock: Arc<DependencyLockService>,
}

impl PlatformService {
    /// Builds platform from config and runs bootstrap initialization.
    pub fn from_config(mut config: PlatformConfig) -> Result<Self, PlatformError> {
        // A process asked for a role it cannot perform refuses here, before it
        // creates a data root or binds a port. An office that cannot join a
        // controller would otherwise serve traffic and report itself healthy
        // while belonging to no cluster.
        // A joined office restarts on what it stored: `interface.md` §5 says
        // the variable is for a first join only, so its absence is not a
        // missing credential when the credential is already on disk.
        let stored_office_token =
            crate::platform::services::cluster::join_token::office_token_path(&config.data_root)
                .metadata()
                .map(|meta| meta.len() > 0)
                .unwrap_or(false);
        config
            .cluster
            .validate(stored_office_token)
            .map_err(|err| PlatformError::new("CLUSTER_CONFIG_INCOMPLETE", err.to_string()))?;
        std::fs::create_dir_all(&config.data_root)?;
        // The layout version gate runs before any adapter opens anything in
        // the root: a newer-versioned root refuses here, a pre-versioned one
        // is stamped (`platform/layout.json`, `instance-directory.md`).
        crate::platform::layout::open_data_root(&config.data_root)?;
        // The EPHEMERAL tier (`run/`, `tmp/`) is wiped and recreated next,
        // still before any adapter opens: deletion is that tier's contract
        // (`instance-directory.md` rule 3), and it must finish before
        // anything can hold an ephemeral file open or begin serving.
        crate::platform::ephemeral::prepare_ephemeral_tier(&config.data_root)?;
        let data = build_data_adapter(config.data_adapter, &config.data_root)?;
        // The configuration service is built first: the file adapter resolves
        // every project directory through the layout that service reads.
        let zebflow_cfg = Arc::new(ProjectConfigurationService::new(
            config.data_root.join("users"),
        ));
        let file = build_file_adapter(
            config.file_adapter,
            config.data_root.clone(),
            zebflow_cfg.clone(),
        );
        let project_data = build_project_data_factory(&config.data_root);
        file.initialize()?;

        let library = Arc::new(LibraryService::from_embedded()?);
        let dependency_lock = Arc::new(
            DependencyLockService::new(config.data_root.join("users"))
                .with_project_configs(zebflow_cfg.clone()),
        );
        let users = Arc::new(UserService::new(data.clone()));
        let projects = Arc::new(ProjectService::new(
            data.clone(),
            file.clone(),
            project_data.clone(),
            zebflow_cfg.clone(),
            dependency_lock.clone(),
        ));
        // Built before `auth`, because `auth` is gated on it. Read from disk
        // on every attempt rather than captured here, so a break-glass or a
        // detach performed against this data root takes effect without anything
        // in this graph having to be told.
        let local_authority = Arc::new(OfficeLocalAuthorityService::new(
            data.clone(),
            config.data_root.clone(),
        ));
        let auth = Arc::new(AuthService::new(users.clone(), local_authority.clone()));
        let git_identity = Arc::new(GitIdentityService::new(users.clone()));
        let authz = Arc::new(AuthorizationService::new(data.clone()));
        let project_members = Arc::new(ProjectMembershipService::new(data.clone(), authz.clone()));
        let project_invites = Arc::new(ProjectInviteService::new(data.clone()));
        let credentials = Arc::new(CredentialService::new(data.clone(), reqwest::Client::new()));
        let addressing = Arc::new(crate::platform::services::addressing::AddressingService::new(
            data.clone(),
            file.clone(),
        ));
        let fonts = Arc::new(crate::platform::services::FontService::new(file.clone()));
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
                .join(crate::platform::services::hub::LOCAL_HUB_STORE_DIR)
                .join("hub.db"),
        )?;
        let node_registry = Arc::new(NodeRegistryService::new(
            projects.clone(),
            dependency_lock.clone(),
        ));
        let hub = Arc::new(HubService::new(
            data.clone(),
            hub_data,
            config.data_adapter,
            projects.clone(),
            node_registry.clone(),
            dependency_lock.clone(),
            config.data_root.clone(),
        ));
        let project_operations = Arc::new(ProjectOperationService::new(data.clone()));
        let project_transfer = Arc::new(ProjectTransferService::new(
            file.clone(),
            config.data_root.clone(),
            config.data_root.join("platform").join("project-operations"),
        ));
        let pipeline_runtime = Arc::new(PipelineRuntimeService::new(
            projects.clone(),
            node_registry.clone(),
        ));
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
        // Reconciled before anything serves: a token supplied by environment
        // and a token already stored that disagree is a refusal to start
        // (`offices.md` §8; the Credential contract states the same rule for
        // encryption keys), never a silent overwrite of one by the other.
        let office_identity = if config.cluster.role == ClusterRole::Worker {
            crate::platform::services::cluster::join_token::resolve_office_identity(
                &config.data_root,
                config
                    .cluster
                    .join_token
                    .as_deref()
                    .filter(|value| !value.trim().is_empty()),
            )?
        } else {
            None
        };
        // The token names the office, so an office that was not told a node id
        // takes the one it was issued rather than the generic role word. An
        // explicitly configured id is left alone and checked at the door: the
        // controller refuses a registration whose claimed office is not the
        // one inside the token.
        if let Some(identity) = office_identity.as_ref()
            && config
                .cluster
                .node_id
                .as_deref()
                .is_none_or(|value| value.trim().is_empty())
        {
            config.cluster.node_id = Some(identity.office_id.clone());
        }
        let cluster_bootstrap = Arc::new(ClusterBootstrapService::new(config.cluster.clone()));
        // The controller's private signing key: generated on its first start,
        // read on every later one, and never held by an office. Every office
        // was issued its public half inside a join token, so this file is the
        // one thing on a controller whose loss costs a re-mint everywhere —
        // hence load-or-create, never regenerate-on-error.
        let controller_signing_key = if cluster_bootstrap.role() == ClusterRole::Worker {
            None
        } else {
            Some(Arc::new(
                crate::platform::services::cluster::join_token::resolve_controller_signing_key(
                    &config.data_root,
                )?,
            ))
        };
        let cluster_join_tokens = Arc::new(ClusterJoinTokenService::new(
            data.clone(),
            cluster_bootstrap.role(),
            office_identity,
            controller_signing_key,
        ));
        let cluster_registry = Arc::new(ClusterRegistryService::new(data.clone()));
        let cluster_placement = Arc::new(ClusterPlacementService::new(data.clone()));
        let cluster_runtime_sync = Arc::new(ClusterRuntimeSyncService::new(
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
            cluster_join_tokens,
            local_authority,
            cluster_registry,
            cluster_placement,
            cluster_runtime_sync,
            credentials,
            assistant_configs,
            addressing,
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
            fonts,
            hub,
            dependency_lock,
        };
        svc.bootstrap_local_office()?;
        // Role does not change what an office is made of. `offices.md` §4 keeps
        // institutions on both sides of a join — "every office seeds its own
        // blessed shelf and data root, joined or not; identical bytes for the
        // same release" — and §6 and §7 need the local account to exist even
        // while the controller is the normal door: break-glass "re-enables
        // local authority" on an account that is "disabled, not merely
        // unknown", and a detached office "is a complete instance the moment it
        // leaves". Neither is possible on an office that never created one.
        //
        // Owed, and recorded in `stability-matrix.md` row 14d: while joined the
        // local account must be disabled for login (§4). That machinery — the
        // disable, the break-glass re-enable, and the owner mapping — does not
        // exist yet, so a joined office's local account is live exactly as a
        // standalone office's is.
        svc.bootstrap_defaults()?;
        // Seed the local hub with the blessed content this binary carries
        // (`distribution.md` §1b: runs at every boot, check-first, as the
        // reserved `zebflow` publisher — the seed is the shelf's only
        // writer). The seed touches `services/hub-local/` only — the
        // Public Hub service and its store stay exactly as the operator
        // left them. Idempotent — already-published coordinates are
        // skipped — and never fatal: a refused package is reported and
        // retried next boot rather than keeping the instance down.
        match svc.hub.seed_blessed_catalog() {
            Ok(report) => {
                if !report.published.is_empty() {
                    println!("hub: seeded {}", report.published.join(", "));
                }
                if !report.retired.is_empty() {
                    println!("hub: retired {}", report.retired.join(", "));
                }
                for error in &report.errors {
                    eprintln!("⚠ hub seed: {error}");
                }
            }
            Err(error) => eprintln!("⚠ hub seed failed: {}", error.message),
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

    /// Builds the script sandbox for one project's pipeline run.
    ///
    /// The sandbox's local fetch root is that project's own file storage, so a
    /// script reads the project it belongs to rather than the directory the
    /// server process was started in. A project whose layout cannot be resolved
    /// gets a sandbox with no root, which refuses a local fetch instead of
    /// falling back to one.
    pub fn project_sandbox(&self, owner: &str, project: &str) -> DenoSandboxEngine {
        match self.file.ensure_project_layout(owner, project) {
            Ok(layout) => DenoSandboxEngine::for_project(layout.files_dir),
            Err(_) => DenoSandboxEngine::default(),
        }
    }

    /// Opens the native file storage backend this project declared.
    ///
    /// This is the seam. `spec.files.backend` in `repo/zebflow.yaml` names the
    /// store that owns the project's bytes, and this is the one place that
    /// declaration becomes an implementation, so a second backend is added by
    /// changing `zebfs::backend::open` rather than by finding every caller that
    /// once constructed `LocalZebFs` directly.
    ///
    /// It answers only "where does this project keep its files". An outside
    /// bucket a pipeline reads from is a connection with a credential, chosen
    /// per pipeline, and its bytes still land in the store returned here.
    pub fn project_zebfs(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<crate::zebfs::LocalZebFs, PlatformError> {
        Ok(self
            .file
            .ensure_project_layout(owner, project)?
            .open_files())
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
            std::sync::Arc::new(self.project_sandbox(owner, project)),
            crate::rwe::resolve_engine_or_default(None),
            Some(self.credentials.clone()),
        )
        .with_platform(std::sync::Arc::new(self.clone()))
        .with_ws_hub(self.ws_hub.clone())
        .with_state_bus(self.state_bus.clone())
        .with_data_root(self.config.data_root.clone());

        let file_rel_path = compiled.file_rel_path.clone();

        // Retention settings for invocation log.
        let project_cfg = self
            .zebflow_cfg
            .read_or_default(owner, project)
            .map_err(|err| crate::pipeline::PipelineError::new(err.code, err.message))?;
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
        if self.users.get_user(&self.config.default_owner)?.is_none() {
            let password =
                resolve_superadmin_password(&self.config.data_root, &self.config.default_password)?;
            self.users.create_user(&CreateUserRequest {
                owner: self.config.default_owner.clone(),
                password: password.value,
                role: "superadmin".to_string(),
                git_name: String::new(),
                git_email: String::new(),
            })?;
            if let Some(path) = password.generated_path {
                // Generated, not chosen: browser logins are forced through the
                // change-password screen until a person replaces it. A host
                // that set ZEBFLOW_PLATFORM_DEFAULT_PASSWORD skips this branch
                // entirely — no bootstrap file, credential stays `chosen`.
                self.users
                    .mark_credential_generated(&self.config.default_owner)?;
                eprintln!(
                    "Generated initial superadmin password at {}. Read it now and store it securely.",
                    path.display()
                );
            }
        }

        self.projects.create_or_update_project(
            &self.config.default_owner,
            &CreateProjectRequest {
                project: self.config.default_project.clone(),
                title: Some("Default".to_string()),
                local_branch: None,
                runtime: Default::default(),
            },
        )?;
        // First boot is a person's new project, so it gets the starter files.
        // An import, a hub install and a clone do not — they arrive with their
        // own content and a sample would be a stray file in it.
        self.projects
            .write_starter_files(&self.config.default_owner, &self.config.default_project)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platform::model::PlatformConfig;

    #[test]
    fn a_project_sandbox_fetches_inside_that_project() {
        let data_root = tempfile::tempdir().expect("temp data root");
        let platform = PlatformService::from_config(PlatformConfig {
            data_root: data_root.path().to_path_buf(),
            default_password: "secret".to_string(),
            default_project: "sandbox_root".to_string(),
            ..Default::default()
        })
        .expect("platform");

        let compiled = platform
            .project_sandbox("superadmin", "sandbox_root")
            .compile_script("return 1;", None)
            .expect("compile");

        let layout = platform
            .file
            .ensure_project_layout("superadmin", "sandbox_root")
            .expect("project layout");
        assert_eq!(
            compiled.resolved_config.local_fetch_root,
            layout.files_dir.display().to_string()
        );
    }
}
