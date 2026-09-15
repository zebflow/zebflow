//! Axum web layer for Zebflow platform flows, rendered via RWE templates.
//!
//! Pipeline webhook ingress is normalized in [`build_webhook_ingress_input`].
//! Keep the wire-to-pipeline payload contract there in sync with
//! `src/pipeline/nodes/basic/trigger/mod.rs` and
//! `src/pipeline/nodes/basic/file_ref.rs`: user body data lives under
//! `input.body`, request context at root, and multipart files under
//! `input.files` as FileRef metadata.

pub(crate) mod embedded;
mod webhook_url;

use std::collections::HashMap;
use std::convert::Infallible;
use std::fs;
use std::path::{Path as FsPath, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use axum::body::{Body, Bytes};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Form, Multipart, Path, Query, State};
use axum::http::{
    HeaderMap, HeaderName, HeaderValue, Method, StatusCode, Uri, header::CACHE_CONTROL,
    header::CONTENT_DISPOSITION, header::CONTENT_TYPE, header::HOST, header::LOCATION,
    header::SET_COOKIE,
};
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::routing::{any, delete, get, patch, post, put};
use axum::{Json, Router};
use rand::RngExt as _;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::automaton::infra::assistant_config::load_project_assistant_llm;
use crate::contracts::kinds::decode_pipeline_graph;
use crate::infra::cluster::registry::WorkerRegistryRecord;
use crate::infra::execution::placement::{ProjectRuntimeMode, ProjectRuntimePlacementTarget};
use crate::infra::mem::subscriber::KvSubscriber;
use crate::infra::scheduler::PipelineScheduler;
use crate::infra::ws_client::WsClientManager;
use crate::language::{DenoSandboxEngine, LanguageEngine, NoopLanguageEngine};
use crate::pipeline::model::{ExecuteOptions, ExecutionBus};
use crate::pipeline::{BasicPipelineEngine, PipelineContext, PipelineEngine, PipelineGraph};
use crate::platform::error::PlatformError;
use crate::platform::model::NodePackageManifest;
use crate::platform::model::ResolvedProjectLayout;
use crate::platform::model::{
    ChangePasswordRequest,
    ClusterJoinTokenMintRequest,
    ClusterWorkerHeartbeatRequest,
    ClusterWorkerRegisterRequest,
    ClusterWorkerRegisterResponse,
    CreateHubTokenRequest,
    CreateProjectRequest,
    CreateSimpleTableRequest,
    CreateUserRequest,
    DeletePipelineRequest,
    DescribeProjectDbConnectionRequest,
    ExecutePipelineRequest,
    GitCommitRequest,
    IDENTITY_WRITE_ACTION_CREATED,
    IDENTITY_WRITE_ACTION_LINKED,
    LoginRequest,
    McpSessionCreateRequest,
    McpSessionToggleRequest,
    PipelineExecuteTrigger,
    PipelineInvocationEntry,
    PipelineLocateRequest,
    PlatformOfficeIdentityWrite,
    PlatformOfficeLocalAuthorityEvent,
    ProjectAccessSubject,
    ProjectCapability,
    ProjectOperationKind,
    ProjectTransferArtifactKind,
    QueryProjectDbConnectionRequest,
    RepoTreeScope,
    TemplateCompileRequest,
    TemplateCompileResponse,
    TemplateDiagnostic,
    TestProjectDbConnectionRequest,
    UpdateSettingsSectionRequest,
    UpdateSimpleTableRequest,
    UpdateUserSettingsRequest,
    UpsertPipelineDefinitionRequest,
    UpsertProjectAssistantConfigRequest,
    UpsertProjectCredentialRequest,
    UpsertProjectDbConnectionRequest,
    now_ts,
    slug_segment,
};
use crate::platform::sekejap;
use crate::platform::services::PlatformService;
use crate::platform::services::cluster::local_authority::LOCAL_LOGIN_DISABLED_CODE;
use crate::platform::services::hub::{HubProjectBundlePublishOptions, RemoteHubPublishRequest};
use crate::platform::services::node_registry::NodeRegistryService;
use crate::rwe::{
    CompiledScript, CompiledTemplate, ReactiveWebEngine, ReactiveWebOptions, RenderContext,
    RenderScriptCache, ScriptCacheConfig, TemplateOptions, TemplateSource,
    resolve_engine_or_default,
};
use crate::version::APP_VERSION;
use embedded::{
    PLATFORM_TEMPLATE_ASSETS, hub_catalogue_asset, platform_library_asset,
    platform_node_icon_asset,
};
use crate::platform::db::sql_ddl::SqlDialect;

/// Platform login path — used for unauthenticated page redirects and frontend 401 handling.
const LOGIN_PATH: &str = "/login";
/// The change-password screen a `generated`-credential browser session is
/// forced to until the password is chosen.
const ACCOUNT_PASSWORD_PATH: &str = "/account/password";
/// Platform home path — redirect target after successful login.
const HOME_PATH: &str = "/home";
const SESSION_COOKIE_NAME: &str = "zebflow_session";
const SESSION_TTL_SECS: i64 = 86_400;
/// Shared internal auth header for the first controller/office control-plane slice.
const INTERNAL_CLUSTER_TOKEN_HEADER: &str = "x-zebflow-cluster-token";
const PUBLIC_FS_PROXY_HEADER: &str = "x-zebflow-public-fs-proxy";

const BRAND_LOGO_SVG: &[u8] = include_bytes!("assets/branding/logo.svg");
const BRAND_LOGO_PNG: &[u8] = include_bytes!("assets/branding/logo.png");
const BRAND_FAVICON_SVG: &[u8] = include_bytes!("assets/branding/favicon.svg");
const BRAND_FAVICON_ICO: &[u8] = include_bytes!("assets/branding/favicon.ico");
const BRAND_FAVICON_16_PNG: &[u8] = include_bytes!("assets/branding/favicon-16.png");
const BRAND_FAVICON_32_PNG: &[u8] = include_bytes!("assets/branding/favicon-32.png");
const BRAND_APPLE_TOUCH_ICON_PNG: &[u8] = include_bytes!("assets/branding/apple-touch-icon.png");
/// Global tokens + shared UI; studio rules are concatenated from `pages/project-studio/styles.css` (one HTTP stylesheet).
const PLATFORM_MAIN_CSS: &str = concat!(
    include_str!("templates/styles/fonts.css"),
    "\n\n",
    include_str!("templates/styles/main.css"),
    "\n\n",
    include_str!("templates/pages/project-studio/styles.css"),
);
const PLATFORM_DB_SUITE_CSS: &str = include_str!("templates/styles/db-suite.css");
/// Technology marks for the database pages, inlined as data URIs.
const PLATFORM_DEVICONS_CSS: &str = include_str!("templates/styles/devicons.css");
const PLATFORM_DB_CONNECTIONS_CSS: &str = include_str!("templates/styles/db-connections.css");

fn header_first_value<'a>(headers: &'a HeaderMap, name: &str) -> Option<&'a str> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(',').next())
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn platform_base_url(headers: &HeaderMap) -> String {
    if let Ok(value) = std::env::var("ZEBFLOW_PLATFORM_BASE_URL") {
        let value = value.trim().trim_end_matches('/');
        if !value.is_empty() {
            return value.to_string();
        }
    }
    let proto = header_first_value(headers, "x-forwarded-proto").unwrap_or("http");
    let host = header_first_value(headers, "x-forwarded-host")
        .or_else(|| headers.get(HOST).and_then(|value| value.to_str().ok()))
        .unwrap_or("localhost:10610")
        .trim();
    format!("{}://{}", proto, host)
        .trim_end_matches('/')
        .to_string()
}

/// In debug builds, set to true when source templates change.
/// The SSE endpoint consumes this flag and broadcasts a reload to all browser tabs.
#[cfg(debug_assertions)]
static DEV_DIRTY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// Absolute path to platform template sources — resolved at compile time.
/// Used in debug builds to watch for file changes and serve CSS directly from disk.
#[cfg(debug_assertions)]
const TEMPLATE_SOURCE_DIR: &str =
    concat!(env!("CARGO_MANIFEST_DIR"), "/src/platform/web/templates");

/// Canonical page registry: (BTreeMap key, RWE template id, relative path from template root).
/// Used by build_frontend() at startup and by render_page() in debug builds for per-request
/// recompilation.
const PAGE_DEFS: &[(&str, &str, &str)] = &[
    ("platform-login", "platform.login", "pages/login/page.tsx"),
    ("platform-home", "platform.home", "pages/home/page.tsx"),
    (
        "platform-profile",
        "platform.profile",
        "pages/profile/page.tsx",
    ),
    (
        "platform-account-password",
        "platform.account_password",
        "pages/account-password/page.tsx",
    ),
    ("platform-hub", "platform.hub", "pages/hub/page.tsx"),
    (
        "platform-project-pipelines",
        "platform.project.pipelines",
        "pages/project-studio/pipelines/page.tsx",
    ),
    (
        "platform-project-editor",
        "platform.project.editor",
        "pages/project-studio/pipelines/registry/page.tsx",
    ),
    (
        "platform-project-section",
        "platform.project.section",
        "pages/project-studio/files/page.tsx",
    ),
    (
        "platform-project-dashboard",
        "platform.project.dashboard",
        "pages/project-studio/dashboard/page.tsx",
    ),
    (
        "platform-project-settings",
        "platform.project.settings",
        "pages/project-studio/settings/page.tsx",
    ),
    (
        "platform-project-infrastructure",
        "platform.project.infrastructure",
        "pages/project-studio/infrastructure/page.tsx",
    ),
    (
        "platform-project-credentials",
        "platform.project.credentials",
        "pages/project-studio/credentials/page.tsx",
    ),
    (
        "platform-project-tables",
        "platform.project.tables",
        "pages/project-studio/connections/page.tsx",
    ),
    (
        "platform-project-table-connection",
        "platform.project.table_connection",
        "pages/project-studio/connections/db/connection/page.tsx",
    ),
    (
        "platform-project-table-connection-mapserver",
        "platform.project.table_connection.mapserver",
        "pages/project-studio/connections/db/mapserver/page.tsx",
    ),
    (
        "platform-project-hub",
        "platform.project.hub",
        "pages/project-studio/hub/page.tsx",
    ),
    (
        "platform-design-system",
        "platform.dev.design_system",
        "pages/dev/design-system/page.tsx",
    ),
    (
        "platform-design-system-ui",
        "platform.dev.design_system_ui",
        "pages/dev/design-system/ui/page.tsx",
    ),
    (
        "platform-project-settings-clone-ui-preview",
        "platform.project.settings.clone_ui_preview",
        "pages/project-studio/settings/clone/ui/preview/page.tsx",
    ),
];

/// Shared frontend render bundle (compiled templates + engines).
#[derive(Clone)]
struct PlatformFrontend {
    rwe: Arc<dyn ReactiveWebEngine>,
    language: Arc<dyn LanguageEngine>,
    // In debug builds render_page() recompiles on every request, so this cache is unused.
    #[cfg_attr(debug_assertions, allow(dead_code))]
    pages: Arc<std::collections::BTreeMap<&'static str, CompiledTemplate>>,
    /// Template root on disk — used in debug builds for per-request recompilation.
    #[cfg_attr(not(debug_assertions), allow(dead_code))]
    template_root: PathBuf,
    /// Compile options — cloned per request in debug builds.
    #[cfg_attr(not(debug_assertions), allow(dead_code))]
    options: ReactiveWebOptions,
}

#[derive(Clone)]
struct PlatformWebSession {
    owner: String,
    expires_at: i64,
    /// Whether this session began with a controller vouch rather than a password.
    ///
    /// The forced change-password screen exists to stop a *generated* credential
    /// being used before a person replaces it. A vouched session used no
    /// credential at all, and bouncing it there would dead-end the door on
    /// exactly `offices.md` §5's ordinary case — a fresh office, whose local
    /// account is still on its generated password and whose operator has no way
    /// to type it from another host.
    vouched: bool,
}

/// Shared app state used by platform routes.
#[derive(Clone)]
pub struct PlatformAppState {
    /// Platform service graph.
    pub platform: Arc<PlatformService>,
    /// Shared outbound HTTP client for cluster/runtime proxy flows.
    http_client: reqwest::Client,
    frontend: PlatformFrontend,
    render_script_cache: Option<Arc<RenderScriptCache>>,
    /// Shared template compile cache — keyed by markup content hash, reused across requests.
    template_cache: crate::pipeline::engines::basic::TemplateCache,
    /// Background cron scheduler — kept alive for the lifetime of the process.
    scheduler: Arc<PipelineScheduler>,
    /// Background KV subscribe listener service.
    kv_subscriber: Arc<KvSubscriber>,
    /// Background WS client connection service.
    ws_client_manager: Arc<WsClientManager>,
    /// Live preview toggle registry. Key: "{owner}/{project}/{file_rel}".
    preview_registry: Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
    /// Server-side platform web sessions keyed by random cookie token.
    sessions: Arc<std::sync::Mutex<HashMap<String, PlatformWebSession>>>,
    /// Per-process random secret for signing OAuth2 state parameters.
    oauth_state_secret: [u8; 32],
}

/// Builds Zebflow platform router.
pub async fn router(platform: Arc<PlatformService>) -> Router {
    let frontend = build_frontend(&platform.config.data_root, platform.library.source_roots()).unwrap_or_else(|err| {
        panic!("failed building platform frontend templates: {err}");
    });

    // In debug builds, watch the template source directory for changes.
    // Copies changed files to the materialized temp dir and signals the SSE reload endpoint.
    #[cfg(debug_assertions)]
    {
        let watcher_template_root = frontend.template_root.clone();
        tokio::spawn(dev_template_watcher(watcher_template_root));
    }

    let render_script_cache = build_render_script_cache(&platform.config.data_root);

    // Build a BasicPipelineEngine for the scheduler + mem subscriber.
    //
    // One engine serves every project here, so there is no single project to
    // scope the script sandbox to. The default sandbox has no local fetch root
    // and refuses a local fetch by name; it does not fall back to this
    // process's directory.
    let sched_engine = Arc::new(
        BasicPipelineEngine::new(
            Arc::new(DenoSandboxEngine::default()),
            crate::rwe::resolve_engine_or_default(None),
            Some(platform.credentials.clone()),
        )
        .with_platform(platform.clone())
        .with_ws_hub(platform.ws_hub.clone())
        .with_state_bus(platform.state_bus.clone())
        .with_data_root(platform.config.data_root.clone()),
    );

    let scheduler = PipelineScheduler::start(
        platform.pipeline_runtime.clone(),
        sched_engine.clone(),
        platform.pipeline_hits.clone(),
        platform.data.clone(),
        platform.zebflow_cfg.clone(),
    )
    .await
    .unwrap_or_else(|e| {
        panic!("failed starting pipeline scheduler: {e}");
    });

    println!("✅ Pipeline scheduler started");

    let kv_subscriber = Arc::new(KvSubscriber::new(
        platform.state_bus.clone(),
        platform.pipeline_runtime.clone(),
        sched_engine.clone(),
        platform.pipeline_hits.clone(),
        platform.data.clone(),
        platform.zebflow_cfg.clone(),
    ));
    kv_subscriber.register_all().await;
    println!("✅ KV subscriber started");

    let sessions: Arc<std::sync::Mutex<HashMap<String, PlatformWebSession>>> =
        Arc::new(std::sync::Mutex::new(HashMap::new()));

    let ws_client_manager = Arc::new(WsClientManager::new(
        platform.pipeline_runtime.clone(),
        sched_engine,
        platform.pipeline_hits.clone(),
        platform.data.clone(),
        platform.zebflow_cfg.clone(),
    ));
    ws_client_manager.register_all().await;
    println!("✅ WS client manager started");

    let template_cache = crate::pipeline::engines::basic::new_template_cache();
    let mcp_service =
        crate::platform::mcp::build_mcp_service(platform.clone(), template_cache.clone());

    let router = Router::new()
        // Liveness/readiness probes — no auth, always fast.
        .route("/health", get(health_handler))
        .route("/ready", get(ready_handler))
        // OAuth discovery endpoints (RFC 9728 / MCP 2025-03-26 spec) — no auth required.
        .route(
            "/.well-known/oauth-protected-resource",
            get(oauth_protected_resource_handler),
        )
        .route(
            "/.well-known/oauth-authorization-server",
            get(oauth_authorization_server_handler),
        )
        // OAuth2 credential callback — unauthenticated (provider redirects browser here).
        .route("/oauth/callback", get(oauth2_callback_handler))
        .route("/", get(root_redirect))
        .route("/favicon.ico", get(favicon_ico_asset))
        .route("/favicon.svg", get(favicon_svg_asset))
        .route("/favicon-16.png", get(favicon_16_asset))
        .route("/favicon-32.png", get(favicon_32_asset))
        .route("/apple-touch-icon.png", get(apple_touch_icon_asset))
        .route("/assets/branding/{asset}", get(branding_asset))
        .route("/assets/platform/{*asset}", get(platform_asset))
        .route("/assets/node-icons/{*path}", get(node_icon_asset))
        .route("/assets/rwe/scripts/{hash}", get(rwe_script_asset))
        .route(
            "/static/{owner}/{project}/_rwe/scripts/{hash}",
            get(project_rwe_script_asset),
        )
        // `_rwe/` is reserved: compiled bundles and RWE libraries are machinery,
        // not files the author wrote, and they are content-hashed so they can be
        // cached forever. Keeping them under the same prefix means one proxy
        // rule, and the origin -- not the proxy -- sets Cache-Control.
        .route(
            "/static/{owner}/{project}/_rwe/lib/{*path}",
            get(project_scoped_library_asset),
        )
        .route("/static/{owner}/{project}/{*path}", get(project_asset))
        .route("/assets/libraries/{*path}", get(library_asset))
        .route(
            "/p/{owner}/{project}/assets/{*path}",
            get(project_static_asset),
        )
        .route(
            "/files/{owner}/{project}/{*path}",
            get(project_legacy_file_serve),
        )
        .route("/fs/{owner}/{project}/{*path}", get(project_fs_serve))
        .route("/login", get(login_page).post(login_submit))
        .route("/logout", post(logout_submit))
        .route("/home", get(home_page))
        .route("/profile", get(profile_page))
        .route("/account/password", get(account_password_page))
        .route("/hub", get(platform_hub_page))
        .route("/dev/design-system", get(design_system_page))
        .route("/dev/design-system/ui", get(design_system_ui_page))
        .route("/docs/node", get(docs_node_contract))
        .route("/docs/operation", get(docs_operation_contract))
        .route("/home/projects/create", post(home_create_project_submit))
        .route("/home/projects/clone", post(home_clone_project_submit))
        .route("/projects/{owner}/{project}", get(project_root_page))
        .route(
            "/projects/{owner}/{project}/pipelines/{tab}",
            get(project_pipelines_page),
        )
        .route(
            "/projects/{owner}/{project}/dashboard",
            get(project_dashboard_page),
        )
        .route(
            "/projects/{owner}/{project}/hub",
            get(project_hub_page),
        )
        .route(
            "/projects/{owner}/{project}/hub/{tab}",
            get(project_hub_tab_page),
        )
        .route(
            "/projects/{owner}/{project}/credentials",
            get(project_credentials_page),
        )
        .route(
            "/projects/{owner}/{project}/db/connections",
            get(project_db_connections_page),
        )
        .route(
            "/projects/{owner}/{project}/db/{db_kind}/{connection}",
            get(project_db_suite_redirect_page),
        )
        .route(
            "/projects/{owner}/{project}/db/{db_kind}/{connection}/{tab}",
            get(project_db_suite_page),
        )
        .route("/projects/{owner}/{project}/files", get(project_files_page))
        .route(
            "/projects/{owner}/{project}/files/{tab}",
            get(project_files_tab_page),
        )
        .route("/projects/{owner}/{project}/todo", get(project_todo_page))
        .route(
            "/projects/{owner}/{project}/settings/clone/ui/preview",
            get(project_settings_clone_ui_preview_page),
        )
        .route(
            "/projects/{owner}/{project}/settings",
            get(project_settings_page),
        )
        .route(
            "/projects/{owner}/{project}/settings/{tab}",
            get(project_settings_tab_page),
        )
        .route(
            "/projects/{owner}/{project}/infrastructure",
            get(project_infrastructure_page),
        )
        .route(
            "/projects/{owner}/{project}/editor",
            get(project_editor_page),
        )
        .route("/api/meta", get(api_meta))
        .route("/api/system/info", get(api_system_info))
        .route(
            "/api/platform/db/collections",
            get(api_admin_db_list_collections),
        )
        .route("/api/platform/db/query", post(api_admin_db_query))
        .route(
            "/api/platform/db/node/{slug}",
            get(api_admin_db_get_node).delete(api_admin_db_delete_node),
        )
        .route(
            "/api/platform/credentials/keyring",
            get(api_admin_credential_keyring),
        )
        .route(
            "/api/platform/credentials/rotate",
            post(api_admin_credential_rotate),
        )
        .route(
            "/api/platform/credentials/rekey",
            post(api_admin_credential_rekey),
        )
        .route(
            "/api/platform/credentials/reencrypt",
            post(api_admin_credential_reencrypt),
        )
        .route(
            "/api/projects/{owner}/{project}/nodes",
            get(api_list_node_definitions),
        )
        .route(
            "/api/projects/{owner}/{project}/nodes/by-kind/{kind}",
            get(api_get_node_definition),
        )
        .route(
            "/api/projects/{owner}/{project}/nodes/install/review",
            post(api_review_local_node_bundle),
        )
        .route(
            "/api/projects/{owner}/{project}/nodes/install",
            post(api_install_local_node_bundle),
        )
        .route(
            "/api/projects/{owner}/{project}/nodes/uninstall/{kind}",
            delete(api_uninstall_node_package),
        )
        .route(
            "/api/projects/{owner}/{project}/nodes/icon/{kind}",
            get(api_node_icon),
        )
        // The roster is an instance resource — it lives under the instance
        // scope, not beside `/api/users/{owner}/…`, which is one person's own
        // space. Same shape as GitLab's: the path names the resource, the
        // guard names the audience.
        .route(
            "/api/platform/users",
            get(api_list_users).post(api_create_user),
        )
        .route("/api/platform/users/{owner}", delete(api_delete_user))
        .route("/api/profile", get(api_get_profile).put(api_update_profile))
        .route("/api/profile/password", post(api_change_password))
        .route("/api/platform/cluster/workers", get(api_cluster_workers))
        .route(
            "/api/platform/cluster/join-tokens",
            get(api_cluster_join_tokens).post(api_cluster_mint_join_token),
        )
        .route(
            "/api/platform/cluster/join-tokens/{office_id}/revoke",
            post(api_cluster_revoke_join_token),
        )
        // `offices.md` §2's third verb. The controller mints; the office
        // spends. The two halves are registered on one router because one
        // binary is both roles, and each half refuses in the role it is not.
        .route(
            "/api/platform/cluster/offices/{office_id}/vouch",
            post(api_cluster_mint_office_vouch),
        )
        .route(
            "/cluster/offices/{office_id}/open",
            get(cluster_open_office_redirect),
        )
        .route("/office/vouch", get(office_vouch_redeem))
        // Deliberately NOT under /api/platform: redemption is called by a
        // browser that has no session yet — the vouch in the body is the whole
        // credential. Everything under the instance scope requires a session.
        .route("/api/office/vouch", post(api_office_vouch_redeem))
        .route(
            "/api/platform/office/identity-writes",
            get(api_office_identity_writes),
        )
        // `offices.md` §6's record, read by the office's own operator, and the
        // controller's copy of what its offices reported. Two routes rather
        // than one because they answer two different questions on two
        // different instances: "what happened here" and "what did my offices
        // tell me".
        .route(
            "/api/platform/office/local-authority",
            get(api_office_local_authority),
        )
        .route(
            "/api/platform/cluster/office-break-glass",
            get(api_cluster_office_break_glass),
        )
        .route(
            "/api/internal/cluster/offices/break-glass",
            post(api_internal_cluster_report_break_glass),
        )
        .route(
            "/api/internal/cluster/workers/register",
            post(api_internal_cluster_register_worker),
        )
        .route(
            "/api/internal/cluster/workers/heartbeat",
            post(api_internal_cluster_worker_heartbeat),
        )
        .route(
            "/api/internal/runtime/execute/{owner}/{project}",
            post(api_internal_runtime_execute_pipeline),
        )
        .route(
            "/api/internal/runtime/webhook/{owner}/{project}",
            any(api_internal_runtime_webhook_root),
        )
        .route(
            "/api/internal/runtime/webhook/{owner}/{project}/",
            any(api_internal_runtime_webhook_root),
        )
        .route(
            "/api/internal/runtime/webhook/{owner}/{project}/{*tail}",
            any(api_internal_runtime_webhook),
        )
        .route(
            "/api/internal/project-transfer/{owner}/{project}/export/{kind}",
            post(api_internal_project_transfer_export),
        )
        .route(
            "/api/internal/project-transfer/{owner}/{project}/import/{kind}",
            post(api_internal_project_transfer_import),
        )
        .route(
            "/api/users/{owner}/projects",
            get(api_list_projects).post(api_create_project),
        )
        .route(
            "/api/users/{owner}/hub/repositories",
            get(api_list_platform_hub_repositories)
                .post(api_upsert_platform_hub_repository),
        )
        .route(
            "/api/platform/hub/repositories",
            get(api_list_platform_hub_sources).post(api_upsert_platform_hub_source),
        )
        .route(
            "/api/platform/hub/service",
            get(api_get_platform_hub_service).post(api_configure_platform_hub_service),
        )
        .route(
            "/api/platform/hub/publishers",
            get(api_list_platform_hub_publishers).post(api_upsert_platform_hub_publisher),
        )
        .route(
            "/api/platform/hub/tokens",
            get(api_list_platform_hub_tokens).post(api_create_platform_hub_token),
        )
        .route(
            "/api/platform/hub/tokens/{token_id}",
            delete(api_delete_platform_hub_token),
        )
        .route(
            "/api/platform/hub/grants",
            get(api_list_platform_hub_grants).post(api_upsert_platform_hub_grant),
        )
        .route(
            "/api/platform/hub/grants/{grant_id}",
            delete(api_delete_platform_hub_grant),
        )
        .route(
            "/api/platform/hub/publishers/{publisher_id}",
            delete(api_delete_platform_hub_publisher),
        )
        .route(
            "/api/users/{owner}/hub/repositories/{repository_id}",
            delete(api_delete_platform_hub_repository),
        )
        .route(
            "/api/platform/hub/repositories/{repository_id}",
            delete(api_delete_platform_hub_source),
        )
        .route(
            "/api/users/{owner}/hub/assets",
            get(api_list_platform_hub_assets),
        )
        .route(
            "/api/platform/hub/assets",
            get(api_list_platform_hub_apps),
        )
        .route(
            "/api/users/{owner}/hub/install",
            post(api_install_platform_hub_project),
        )
        .route(
            "/api/users/{owner}/hub/install/review",
            post(api_review_platform_hub_project),
        )
        .route(
            "/api/platform/hub/install",
            post(api_install_platform_hub_app),
        )
        .route(
            "/api/platform/hub/install/review",
            post(api_review_platform_hub_app),
        )
        .route(
            "/api/platform/transfer/import",
            post(api_platform_transfer_import)
                .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024 * 1024)),
        )
        .route(
            "/api/users/{owner}/projects/{project}",
            delete(api_delete_project),
        )
        .route(
            "/api/projects/{owner}/{project}/pipelines/registry",
            get(api_pipeline_registry),
        )
        .route(
            "/api/projects/{owner}/{project}/pipelines",
            get(api_list_pipelines),
        )
        .route(
            "/api/projects/{owner}/{project}/pipelines/by-id",
            get(api_get_pipeline_by_id),
        )
        .route(
            "/api/projects/{owner}/{project}/pipelines/definition",
            post(api_upsert_pipeline_definition).delete(api_delete_pipeline_definition),
        )
        .route(
            "/api/projects/{owner}/{project}/pipelines/lock-toggle",
            post(api_pipeline_lock_toggle),
        )
        .route(
            "/api/projects/{owner}/{project}/templates/lock-toggle",
            post(api_template_lock_toggle),
        )
        .route(
            "/api/projects/{owner}/{project}/git/status",
            get(api_repo_git_status),
        )
        .route(
            "/api/projects/{owner}/{project}/git/health",
            get(api_repo_git_health),
        )
        .route(
            "/api/projects/{owner}/{project}/git/repair",
            post(api_repo_git_repair),
        )
        .route(
            "/api/projects/{owner}/{project}/git/commit",
            post(api_git_commit),
        )
        .route(
            "/api/projects/{owner}/{project}/git/remote",
            get(api_git_get_remote).put(api_git_put_remote),
        )
        .route(
            "/api/projects/{owner}/{project}/git/branches",
            get(api_git_list_branches).post(api_git_checkout_branch),
        )
        .route(
            "/api/projects/{owner}/{project}/pipelines/activate",
            post(api_activate_pipeline_definition),
        )
        .route(
            "/api/projects/{owner}/{project}/pipelines/deactivate",
            post(api_deactivate_pipeline_definition),
        )
        .route(
            "/api/projects/{owner}/{project}/pipelines/execute",
            post(api_execute_pipeline),
        )
        .route(
            "/api/projects/{owner}/{project}/pipelines/dsl",
            post(api_execute_pipeline_dsl),
        )
        .route(
            "/api/projects/{owner}/{project}/pipelines/hits",
            get(api_pipeline_hits),
        )
        .route(
            "/api/projects/{owner}/{project}/pipelines/invocations",
            get(api_pipeline_invocations),
        )
        // One repository file surface. `kinds/project-configuration/layout.md`
        // §8 decides what a repository may hold from the path alone, so there
        // is one tree and one door rather than a source root and a docs root
        // with a rule each.
        .route(
            "/api/projects/{owner}/{project}/repo",
            get(api_repo_tree),
        )
        .route(
            "/api/projects/{owner}/{project}/repo/file",
            get(api_repo_read).put(api_repo_write).delete(api_repo_delete),
        )
        .route(
            "/api/projects/{owner}/{project}/repo/folder",
            post(api_repo_create_folder),
        )
        .route(
            "/api/projects/{owner}/{project}/repo/move",
            post(api_repo_move),
        )
        .route(
            "/api/projects/{owner}/{project}/repo/search",
            get(api_repo_search),
        )
        .route(
            "/api/projects/{owner}/{project}/templates/outline",
            get(api_template_outline),
        )
        .route(
            "/api/projects/{owner}/{project}/templates/git-status",
            get(api_template_git_status),
        )
        .route(
            "/api/projects/{owner}/{project}/templates/diagnostics",
            post(api_template_diagnostics),
        )
        .route(
            "/api/projects/{owner}/{project}/files/list",
            get(api_files_list),
        )
        .route(
            "/api/projects/{owner}/{project}/files/mkdir",
            post(api_files_mkdir),
        )
        .route(
            "/api/projects/{owner}/{project}/files/upload",
            post(api_files_upload)
                .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024 * 1024)),
        )
        .route(
            "/api/projects/{owner}/{project}/files/rm",
            post(api_files_rm),
        )
        .route(
            "/api/projects/{owner}/{project}/files/access",
            put(api_files_access).post(api_files_access_form),
        )
        .route(
            "/api/projects/{owner}/{project}/credential-types",
            get(api_list_credential_types),
        )
        .route(
            "/api/projects/{owner}/{project}/credentials",
            get(api_list_credentials).post(api_upsert_credential),
        )
        .route(
            "/api/projects/{owner}/{project}/credentials/{credential_id}",
            get(api_get_credential)
                .put(api_upsert_credential_by_path)
                .delete(api_delete_credential),
        )
        .route(
            "/api/projects/{owner}/{project}/credentials/{credential_id}/oauth/authorize",
            get(api_oauth2_authorize),
        )
        .route(
            "/api/projects/{owner}/{project}/assistant/config",
            get(api_get_project_assistant_config).put(api_upsert_project_assistant_config),
        )
        .route(
            "/api/projects/{owner}/{project}/settings/logs/invocations",
            get(api_project_invocation_log_stats).delete(api_clear_project_invocation_logs),
        )
        .route(
            "/api/projects/{owner}/{project}/settings/addressing/check",
            post(api_check_addressing_host),
        )
        .route(
            "/api/projects/{owner}/{project}/settings/{section}",
            get(api_get_settings_section).put(api_upsert_settings_section),
        )
        .route(
            "/api/projects/{owner}/{project}/rwe/libraries",
            get(api_list_rwe_libraries),
        )
        .route(
            "/api/projects/{owner}/{project}/rwe/libraries/enable",
            post(api_enable_rwe_library),
        )
        .route(
            "/api/projects/{owner}/{project}/rwe/libraries/remove",
            delete(api_remove_rwe_library),
        )
        .route(
            "/api/projects/{owner}/{project}/dependencies",
            get(api_project_dependency_status).post(api_repair_project_dependencies),
        )
        .route(
            "/api/projects/{owner}/{project}/rwe/cache/clear",
            post(api_rwe_cache_clear),
        )
        .route(
            "/api/projects/{owner}/{project}/editor/completion-catalog",
            get(api_editor_completion_catalog),
        )
        .route(
            "/api/projects/{owner}/{project}/assistant/chat",
            post(api_project_assistant_chat),
        )
        .route(
            "/api/projects/{owner}/{project}/db/connections",
            get(api_list_db_connections).post(api_upsert_db_connection),
        )
        .route(
            "/api/projects/{owner}/{project}/tables",
            get(api_list_simple_tables).post(api_create_simple_table),
        )
        .route(
            "/api/projects/{owner}/{project}/tables/schema/export",
            get(api_export_sekejap_schema),
        )
        .route(
            "/api/projects/{owner}/{project}/tables/schema/sync",
            post(api_sync_sekejap_schema),
        )
        .route(
            "/api/projects/{owner}/{project}/db/sekejap/maintenance/health",
            get(api_sekejap_project_health),
        )
        .route(
            "/api/projects/{owner}/{project}/db/sekejap/maintenance/sync",
            post(api_sekejap_project_sync),
        )
        .route(
            "/api/projects/{owner}/{project}/db/sekejap/maintenance/compact",
            post(api_sekejap_project_compact),
        )
        .route(
            "/api/projects/{owner}/{project}/tables/{table}",
            put(api_update_simple_table).delete(api_delete_simple_table),
        )
        .route(
            "/api/projects/{owner}/{project}/db/connections/{connection_id}/describe",
            get(api_describe_db_connection),
        )
        .route(
            "/api/projects/{owner}/{project}/db/connections/{connection_id}/schemas",
            get(api_list_db_connection_schemas),
        )
        .route(
            "/api/projects/{owner}/{project}/db/connections/{connection_id}/tables",
            get(api_list_db_connection_tables).post(api_create_db_connection_table),
        )
        .route(
            "/api/projects/{owner}/{project}/db/connections/{connection_id}/tables/{table}",
            delete(api_drop_db_connection_table).put(api_alter_db_connection_table),
        )
        .route(
            "/api/projects/{owner}/{project}/db/connections/{connection_id}/tables/{table}/rows",
            post(api_insert_db_connection_row),
        )
        .route(
            "/api/projects/{owner}/{project}/db/connections/{connection_id}/functions",
            get(api_list_db_connection_functions),
        )
        .route(
            "/api/projects/{owner}/{project}/db/connections/{connection_id}/query",
            post(api_query_db_connection),
        )
        .route(
            "/api/projects/{owner}/{project}/db/connections/{connection_id}/table-preview",
            get(api_preview_db_connection_table),
        )
        .route(
            "/api/projects/{owner}/{project}/db/connections/{connection_slug}",
            get(api_get_db_connection)
                .put(api_upsert_db_connection_by_path)
                .delete(api_delete_db_connection),
        )
        .route(
            "/api/projects/{owner}/{project}/db/connections/test",
            post(api_test_db_connection),
        )
        .route(
            "/api/projects/{owner}/{project}/hub/assets",
            get(api_list_hub_assets),
        )
        .route(
            "/api/projects/{owner}/{project}/members",
            get(api_list_project_members).post(api_upsert_project_member),
        )
        .route(
            "/api/projects/{owner}/{project}/members/{user_id}",
            delete(api_remove_project_member),
        )
        .route(
            "/api/projects/{owner}/{project}/invites",
            get(api_list_project_invites).post(api_create_project_invite),
        )
        .route(
            "/api/projects/{owner}/{project}/invites/{invite_id}",
            delete(api_revoke_project_invite),
        )
        // Answering is session-scoped: the invitee is not yet a member, so a
        // project-scoped capability check would refuse the very person the
        // invitation is for.
        .route("/api/invites", get(api_list_my_invites))
        .route(
            "/api/invites/{owner}/{project}/{invite_id}/{answer}",
            post(api_answer_my_invite),
        )
        .route(
            "/api/projects/{owner}/{project}/hub/access",
            get(api_list_project_hub_access),
        )
        .route(
            "/api/projects/{owner}/{project}/hub/assets/{package_id}",
            delete(api_delete_hub_asset),
        )
        .route(
            "/api/projects/{owner}/{project}/hub/assets/{package_id}/presentation",
            patch(api_update_hub_asset_presentation),
        )
        .route(
            "/api/projects/{owner}/{project}/help",
            get(api_get_project_help),
        )
        .route(
            "/api/projects/{owner}/{project}/hub/remote/assets",
            get(api_list_remote_hub_assets),
        )
        .route(
            "/api/projects/{owner}/{project}/hub/remote/assets/{package_id}/{version}",
            get(api_get_remote_hub_asset),
        )
        .route(
            "/api/projects/{owner}/{project}/hub/remote/assets/{package_id}/{version}/artifact",
            get(api_get_remote_hub_artifact),
        )
        .route(
            "/api/projects/{owner}/{project}/hub/remote/assets/{package_id}/{version}/artifacts/{sha256}",
            get(api_get_remote_hub_referenced_artifact),
        )
        .route(
            "/api/hub/remote/assets",
            get(api_list_public_hub_assets).post(api_public_remote_publish_hub_asset),
        )
        .route(
            "/api/hub/remote/assets/{package_id}",
            delete(api_public_delete_hub_asset),
        )
        .route(
            "/api/hub/remote/assets/{package_id}/media/{media_name}",
            get(api_get_public_hub_media),
        )
        .route(
            "/api/hub/remote/assets/{package_id}/{version}",
            get(api_get_public_hub_asset),
        )
        .route(
            "/api/hub/remote/assets/{package_id}/{version}/artifact",
            get(api_get_public_hub_artifact),
        )
        .route(
            "/api/hub/remote/assets/{package_id}/{version}/artifacts/{sha256}",
            get(api_get_public_hub_referenced_artifact),
        )
        .route(
            "/api/projects/{owner}/{project}/hub/assets/mine",
            get(api_list_my_hub_assets),
        )
        .route(
            "/api/projects/{owner}/{project}/hub/publish-sources",
            get(api_list_hub_publish_sources),
        )
        .route(
            "/api/projects/{owner}/{project}/hub/publish-preview",
            get(api_preview_hub_publish_source),
        )
        .route(
            "/api/projects/{owner}/{project}/hub/assets/preview",
            get(api_preview_hub_publish_source),
        )
        .route(
            "/api/projects/{owner}/{project}/hub/assets/publish-review",
            post(api_review_hub_publish_asset),
        )
        .route(
            "/api/projects/{owner}/{project}/hub/assets/{package_id}/{version}/add",
            post(api_install_hub_asset),
        )
        .route(
            "/api/projects/{owner}/{project}/hub/assets/{package_id}/{version}/review",
            post(api_review_hub_asset),
        )
        .route(
            "/api/projects/{owner}/{project}/hub/remote/assets/publish",
            post(api_remote_publish_hub_asset),
        )
        .route(
            "/api/projects/{owner}/{project}/hub/tokens",
            get(api_list_hub_tokens).post(api_create_hub_token),
        )
        .route(
            "/api/projects/{owner}/{project}/hub/publishers",
            get(api_list_hub_publishers).post(api_upsert_hub_publisher),
        )
        .route(
            "/api/projects/{owner}/{project}/hub/repositories",
            get(api_list_hub_repositories).post(api_upsert_hub_repository),
        )
        .route(
            "/api/projects/{owner}/{project}/hub/repositories/{repository_id}/packs/{package_id}/{version}/add",
            post(api_install_remote_hub_pack),
        )
        .route(
            "/api/projects/{owner}/{project}/hub/repositories/{repository_id}/packs/{package_id}/{version}/review",
            post(api_review_remote_hub_pack),
        )
        .route(
            "/api/projects/{owner}/{project}/hub/repositories/{repository_id}",
            delete(api_delete_hub_repository),
        )
        .route(
            "/api/projects/{owner}/{project}/hub/producer",
            post(api_set_hub_producer_mode),
        )
        .route(
            "/api/projects/{owner}/{project}/hub/tokens/{token_id}",
            delete(api_delete_hub_token),
        )
        .route(
            "/api/projects/{owner}/{project}/hub/publishers/{publisher_id}",
            delete(api_delete_hub_publisher),
        )
        .route(
            "/api/projects/{owner}/{project}/agent-docs",
            get(api_list_agent_docs),
        )
        .route(
            "/api/projects/{owner}/{project}/agent-docs/file",
            get(api_read_agent_doc).put(api_upsert_agent_doc_file),
        )
        .route(
            "/api/projects/{owner}/{project}/mcp/session",
            get(api_get_mcp_session)
                .post(api_create_mcp_session)
                .put(api_toggle_mcp_session)
                .delete(api_revoke_mcp_session),
        )
        .route(
            "/api/projects/{owner}/{project}/mcp/session/reset-token",
            post(api_reset_mcp_session_token),
        )
        .route(
            "/api/projects/{owner}/{project}/assets",
            get(api_list_assets)
                .post(api_upload_asset)
                .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024 * 1024)),
        )
        .route(
            "/api/projects/{owner}/{project}/assets/{*path}",
            delete(api_delete_asset),
        )
        .route(
            "/api/projects/{owner}/{project}/mapserver/{instance}/sources",
            get(api_mapserver_sources_list),
        )
        .route(
            "/api/projects/{owner}/{project}/mapserver/{instance}/layers",
            get(api_mapserver_layers_list).post(api_mapserver_layers_publish),
        )
        .route(
            "/api/projects/{owner}/{project}/mapserver/{instance}/layers/{layer_id}",
            delete(api_mapserver_layers_delete),
        )
        .route(
            "/api/projects/{owner}/{project}/mapserver/{instance}/layers/{layer_id}/stats",
            get(api_mapserver_layer_stats),
        )
        .route(
            "/api/projects/{owner}/{project}/install/catalog/ui",
            get(api_list_ui_catalog),
        )
        .route(
            "/api/projects/{owner}/{project}/install/ui/review",
            post(api_review_ui_components),
        )
        .route(
            "/api/projects/{owner}/{project}/install/ui",
            post(api_install_ui_components),
        )
        .route(
            "/api/projects/{owner}/{project}/reindex",
            post(api_reindex_project),
        )
        .nest("/api/projects/{owner}/{project}/mcp", mcp_service)
        .route(
            "/wh/{owner}/{project}",
            any(public_webhook_ingress_root)
                .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024 * 1024)),
        )
        .route(
            "/wh/{owner}/{project}/",
            any(public_webhook_ingress_root)
                .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024 * 1024)),
        )
        .route(
            "/wh/{owner}/{project}/{*tail}",
            any(public_webhook_ingress)
                .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024 * 1024)),
        )
        .route("/ms/{owner}/{project}", get(public_mapserver_ingress_root))
        .route("/ms/{owner}/{project}/", get(public_mapserver_ingress_root))
        .route("/ms/{owner}/{project}/{*tail}", get(public_mapserver_ingress))
        .route(
            "/ws/{owner}/{project}/rooms/{room_id}",
            get(ws_room_handler),
        )
        .route(
            "/api/projects/{owner}/{project}/preview/toggle",
            post(api_preview_toggle),
        )
        .route(
            "/api/projects/{owner}/{project}/preview/status",
            get(api_preview_status),
        )
        .route(
            "/api/projects/{owner}/{project}/runtime",
            get(api_project_runtime_status),
        )
        .route(
            "/api/projects/{owner}/{project}/transfer/operations",
            get(api_project_transfer_operations),
        )
        .route(
            "/api/projects/{owner}/{project}/transfer/export/{kind}",
            post(api_project_transfer_export),
        )
        .route(
            "/api/projects/{owner}/{project}/transfer/import/{kind}",
            post(api_project_transfer_import)
                .layer(axum::extract::DefaultBodyLimit::max(1024 * 1024 * 1024)),
        )
        .route(
            "/api/projects/{owner}/{project}/transfer/rollback",
            post(api_project_transfer_rollback),
        )
        .route(
            "/api/projects/{owner}/{project}/transfer/download/{operation_id}",
            get(api_project_transfer_download),
        )
        .route(
            "/api/projects/{owner}/{project}/transfer/owner",
            post(api_transfer_project_owner),
        )
        .route("/preview/{owner}/{project}", get(preview_page))
        .route("/ws/preview/{owner}/{project}", get(ws_preview_handler));

    let app_state = PlatformAppState {
        platform,
        http_client: reqwest::Client::new(),
        frontend,
        render_script_cache,
        template_cache,
        scheduler,
        kv_subscriber,
        ws_client_manager,
        preview_registry: Arc::new(std::sync::Mutex::new(std::collections::HashSet::new())),
        sessions,
        oauth_state_secret: {
            let mut bytes = [0u8; 32];
            rand::rng().fill(&mut bytes);
            bytes
        },
    };

    if app_state.platform.cluster_bootstrap.is_worker() {
        tokio::spawn(cluster_worker_registration_loop(app_state.clone()));
    }

    // Forced password change (Grafana's rule): while an account's credential
    // is still the platform-generated one, every browser page request is
    // redirected to the change-password screen. Applied as a layer because the
    // page handlers resolve their sessions individually; the gate never
    // touches /api, /wh, /ws, /mcp, or asset paths, so the CLI's zero-ceremony
    // window (authenticating with the generated password) keeps working.
    let router = router.layer(axum::middleware::from_fn_with_state(
        app_state.clone(),
        credential_change_gate,
    ));

    let router = router.with_state(app_state.clone());

    // Debug-only: SSE reload endpoint added after with_state because it needs no app state.
    #[cfg(debug_assertions)]
    let router = router.route("/__dev/reload", get(dev_reload_sse));

    // Addressing (`docs/contracts/addressing.md`): a request on a project's
    // host is rewritten to the platform form before it is routed. The gate
    // must run before routing, so it wraps the whole router rather than
    // sitting on it as a route layer.
    match app_state.platform.addressing.rebuild_index() {
        Ok(hosts) => println!("✅ Addressing index built ({hosts} custom hosts)"),
        Err(err) => eprintln!("warning: addressing index: {err}"),
    }
    Router::new()
        .fallback_service(router)
        .layer(axum::middleware::from_fn_with_state(app_state, addressing_gate))
}

/// The host a request arrived on, when it is a project's.
#[derive(Debug, Clone)]
pub struct ProjectHost {
    pub host: String,
    pub owner: String,
    pub project: String,
    pub dev_host: bool,
}

/// Rewrites a request on a project host to the platform form it is served
/// as (`northside.superadmin.localhost/book` → `/wh/superadmin/northside/book`,
/// `/_files/a.jpg` → `/files/superadmin/northside/a.jpg`), refuses a surface
/// the project switched off, and leaves every other request alone.
async fn addressing_gate(
    State(state): State<PlatformAppState>,
    mut request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let path = request.uri().path().to_string();
    let host = request
        .headers()
        .get(axum::http::header::HOST)
        .and_then(|v| v.to_str().ok())
        .map(str::to_string)
        .or_else(|| request.uri().authority().map(|a| a.to_string()));
    if let Some(host) = host
        && let Some(resolution) = state.platform.addressing.resolve(&host, &path)
    {
        // The platform forms for this very project stay valid on its host:
        // pages emit `/static/{o}/{p}/_rwe/…` and the Studio's API lives at
        // `/api/projects/{o}/{p}/…`; neither is an app path.
        let own_prefixes = [
            format!("/wh/{}/{}", resolution.owner, resolution.project),
            format!("/ws/{}/{}", resolution.owner, resolution.project),
            format!("/files/{}/{}", resolution.owner, resolution.project),
            format!("/static/{}/{}", resolution.owner, resolution.project),
            format!("/ms/{}/{}", resolution.owner, resolution.project),
            format!("/fs/{}/{}", resolution.owner, resolution.project),
            format!("/api/projects/{}/{}", resolution.owner, resolution.project),
        ];
        let platform_form = path.starts_with("/assets/")
            || own_prefixes
                .iter()
                .any(|p| path == *p || path.starts_with(&format!("{p}/")));
        if !platform_form {
            if resolution.rest.is_empty() {
                return (
                    StatusCode::NOT_FOUND,
                    format!(
                        "{} is switched off for this project (Settings → Addressing)",
                        resolution.surface.title()
                    ),
                )
                    .into_response();
            }
            let query = request
                .uri()
                .query()
                .map(|q| format!("?{q}"))
                .unwrap_or_default();
            let target = format!("{}{}", resolution.platform_path(), query);
            if let Ok(path_and_query) = target.parse::<axum::http::uri::PathAndQuery>() {
                let mut parts = request.uri().clone().into_parts();
                parts.path_and_query = Some(path_and_query);
                if let Ok(uri) = Uri::from_parts(parts) {
                    *request.uri_mut() = uri;
                }
            }
        }
        request.extensions_mut().insert(ProjectHost {
            host: resolution.host.clone(),
            owner: resolution.owner.clone(),
            project: resolution.project.clone(),
            dev_host: resolution.dev_host,
        });
        let mut response = next.run(request).await;
        // The proof a proxy is wired right: Settings → Addressing → Verify
        // reads this back through the public host.
        if let Ok(value) = HeaderValue::from_str(&format!("{}/{}", resolution.owner, resolution.project)) {
            response.headers_mut().insert("x-zebflow-project", value);
        }
        return response;
    }
    if state.platform.addressing.platform_form_disabled(&path) {
        return (StatusCode::NOT_FOUND, "this surface is switched off for the project (Settings → Addressing)").into_response();
    }
    next.run(request).await
}

fn build_render_script_cache(data_root: &FsPath) -> Option<Arc<RenderScriptCache>> {
    let root = data_root.join("platform").join("rwe-script-cache");
    let cfg = ScriptCacheConfig::new(root, 8 * 1024 * 1024);
    match RenderScriptCache::new(cfg) {
        Ok(cache) => Some(Arc::new(cache)),
        Err(err) => {
            eprintln!("warning: failed creating RWE script cache: {err}");
            None
        }
    }
}

fn build_frontend(
    data_root: &FsPath,
    library_roots: std::collections::BTreeMap<String, PathBuf>,
) -> Result<PlatformFrontend, PlatformError> {
    let rwe_engine_id = std::env::var("ZEBFLOW_PLATFORM_RWE_ENGINE_ID").ok();
    let rwe: Arc<dyn ReactiveWebEngine> = resolve_engine_or_default(rwe_engine_id.as_deref());
    let language: Arc<dyn LanguageEngine> = Arc::new(NoopLanguageEngine);
    let template_root = materialize_platform_template_root(data_root)?;

    let options = ReactiveWebOptions {
        load_scripts: vec!["/assets/platform/*".to_string()],
        allow_list: crate::rwe::ResourceAllowList {
            scripts: vec!["/assets/platform/*".to_string()],
            urls: vec!["/assets/platform/*".to_string()],
            ..Default::default()
        },
        templates: TemplateOptions {
            template_root: Some(template_root.clone()),
            // The gallery at /dev/design-system renders `zeb/ui` next to the
            // studio primitives; nothing else in the studio imports it.
            library_roots: library_roots.clone(),
            style_entries: Vec::new(),
        },
        processors: vec!["tailwind".to_string()],
        ..Default::default()
    };

    let mut pages = std::collections::BTreeMap::new();

    for &(key, id, rel_path) in PAGE_DEFS {
        pages.insert(
            key,
            compile_page(
                rwe.as_ref(),
                language.as_ref(),
                id,
                &template_root,
                rel_path,
                options.clone(),
            )?,
        );
    }

    Ok(PlatformFrontend {
        rwe,
        language,
        pages: Arc::new(pages),
        template_root,
        options,
    })
}

// ---------------------------------------------------------------------------
// Debug-only hot-reload infrastructure
// ---------------------------------------------------------------------------

/// Watches `TEMPLATE_SOURCE_DIR` for file changes every 500 ms.
/// On any change: copies the modified file(s) to the materialized temp dir,
/// re-runs prepare_template_root (idempotent import rewrite), then sets DEV_DIRTY.
#[cfg(debug_assertions)]
async fn dev_template_watcher(temp_root: PathBuf) {
    use std::collections::HashMap;
    use std::sync::atomic::Ordering;

    let source_dir = FsPath::new(TEMPLATE_SOURCE_DIR);
    let mut known: HashMap<PathBuf, std::time::SystemTime> = HashMap::new();
    dev_collect_mtimes(source_dir, &mut known);

    let mut interval = tokio::time::interval(Duration::from_millis(500));
    interval.tick().await; // skip first immediate tick

    loop {
        interval.tick().await;

        let mut current: HashMap<PathBuf, std::time::SystemTime> = HashMap::new();
        dev_collect_mtimes(source_dir, &mut current);

        let changed: Vec<PathBuf> = current
            .iter()
            .filter(|(p, mtime)| known.get(*p) != Some(mtime))
            .map(|(p, _)| p.clone())
            .collect();

        if changed.is_empty() {
            continue;
        }

        // Copy each changed source file into the matching temp-dir slot.
        for src in &changed {
            let Ok(rel) = src.strip_prefix(source_dir) else {
                continue;
            };
            let dest = temp_root.join(rel);
            if let Some(parent) = dest.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::copy(src, &dest);
        }

        // Re-run import rewriting so any newly copied files get @/ → absolute paths.
        let _ = crate::rwe::core::prepare_template_root(&temp_root);

        DEV_DIRTY.store(true, Ordering::Relaxed);
        known = current;
        println!(
            "🔄 [dev] {} template file(s) changed — signalling reload",
            changed.len()
        );
    }
}

#[cfg(debug_assertions)]
fn dev_collect_mtimes(
    dir: &FsPath,
    out: &mut std::collections::HashMap<PathBuf, std::time::SystemTime>,
) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let p = entry.path();
        if p.is_dir() {
            dev_collect_mtimes(&p, out);
        } else if let Ok(meta) = fs::metadata(&p) {
            if let Ok(mtime) = meta.modified() {
                out.insert(p, mtime);
            }
        }
    }
}

/// SSE endpoint — sends `data: reload` whenever DEV_DIRTY is set.
/// Browser pages subscribe via `new EventSource('/__dev/reload')`.
#[cfg(debug_assertions)]
async fn dev_reload_sse() -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    use std::sync::atomic::Ordering;

    let stream = async_stream::stream! {
        let mut interval = tokio::time::interval(Duration::from_millis(250));
        loop {
            interval.tick().await;
            if DEV_DIRTY.swap(false, Ordering::Relaxed) {
                yield Ok::<Event, Infallible>(Event::default().data("reload"));
            }
        }
    };

    Sse::new(stream).keep_alive(
        KeepAlive::new()
            .interval(Duration::from_secs(15))
            .text("keep-alive"),
    )
}

// ---------------------------------------------------------------------------
// Page compilation
// ---------------------------------------------------------------------------

fn compile_page(
    rwe: &dyn ReactiveWebEngine,
    language: &dyn LanguageEngine,
    id: &str,
    template_root: &FsPath,
    relative_path: &str,
    options: ReactiveWebOptions,
) -> Result<CompiledTemplate, PlatformError> {
    let page_path = template_root.join(relative_path);
    #[cfg(debug_assertions)]
    if !page_path.exists() {
        materialize_platform_template_root(template_root)?;
    }
    let markup = fs::read_to_string(&page_path).map_err(|err| {
        PlatformError::new(
            "PLATFORM_RWE_SOURCE_READ",
            format!("failed reading '{}': {err}", page_path.display()),
        )
    })?;
    rwe.compile_template(
        &TemplateSource {
            id: id.to_string(),
            source_path: Some(page_path),
            markup,
        },
        language,
        &options,
    )
    .map_err(|e| PlatformError::new("PLATFORM_RWE_COMPILE", e.to_string()))
}

fn materialize_platform_template_root(_data_root: &FsPath) -> Result<PathBuf, PlatformError> {
    // Write to the OS temp dir under a version-scoped subdirectory.
    // This keeps embedded platform templates out of the user-visible data directory.
    // Debug builds always re-extract so template changes are picked up by cargo watch.
    // Release builds skip extraction when sentinel exists (same version = same bytes).
    let root = std::env::temp_dir()
        .join("zebflow-platform")
        .join(env!("CARGO_PKG_VERSION"));
    let sentinel = root.join(".materialized");
    let needs_extract = cfg!(debug_assertions) || !sentinel.exists();
    if needs_extract {
        // Build set of asset paths that belong to the current binary.
        let asset_paths: std::collections::HashSet<PathBuf> = PLATFORM_TEMPLATE_ASSETS
            .iter()
            .map(|a| root.join(a.path))
            .collect();

        // In debug mode, remove any existing files that are no longer embedded
        // (e.g. stale files left by a previous binary build).
        if cfg!(debug_assertions) && root.exists() {
            fn collect_files(dir: &FsPath, out: &mut Vec<PathBuf>) {
                let Ok(rd) = fs::read_dir(dir) else { return };
                for entry in rd.flatten() {
                    let p = entry.path();
                    if p.is_dir() {
                        collect_files(&p, out);
                    } else {
                        out.push(p);
                    }
                }
            }
            let mut existing = Vec::new();
            collect_files(&root, &mut existing);
            // Another process's in-flight `.tmp` is not stale; sweeping it would
            // fail that process's rename.
            let in_flight = |p: &PathBuf| p.extension().is_some_and(|e| e == "tmp");
            for stale in existing
                .into_iter()
                .filter(|p| !asset_paths.contains(p) && !in_flight(p))
            {
                let _ = fs::remove_file(&stale);
            }
        }

        for asset in PLATFORM_TEMPLATE_ASSETS {
            let full = root.join(asset.path);
            if let Some(parent) = full.parent() {
                fs::create_dir_all(parent)?;
            }
            // Atomic: a second process (another dev server, a parallel test)
            // compiling from this shared root must never read a half-written file.
            crate::rwe::core::write_atomic(&full, asset.bytes)?;
        }
        crate::rwe::core::prepare_template_root(&root)
            .map_err(|e| PlatformError::new("PLATFORM_TEMPLATE_REWRITE", e.message))?;
        if !cfg!(debug_assertions) {
            fs::write(&sentinel, b"")
                .map_err(|e| PlatformError::new("PLATFORM_TEMPLATE_SENTINEL", e.to_string()))?;
        }
    }
    Ok(root)
}

fn render_page(
    state: &PlatformAppState,
    page: &'static str,
    route: &str,
    input: Value,
) -> Result<String, PlatformError> {
    // In debug builds: recompile from disk on every request so file edits are
    // visible after a browser refresh without a cargo rebuild.
    #[cfg(debug_assertions)]
    let _compiled_owned: CompiledTemplate;

    let compiled: &CompiledTemplate;

    #[cfg(debug_assertions)]
    {
        let &(_, id, rel_path) = PAGE_DEFS
            .iter()
            .find(|&&(key, _, _)| key == page)
            .ok_or_else(|| PlatformError::new("PLATFORM_RWE_PAGE_MISSING", page))?;
        _compiled_owned = compile_page(
            state.frontend.rwe.as_ref(),
            state.frontend.language.as_ref(),
            id,
            &state.frontend.template_root,
            rel_path,
            state.frontend.options.clone(),
        )?;
        compiled = &_compiled_owned;
    }

    #[cfg(not(debug_assertions))]
    {
        compiled = state
            .frontend
            .pages
            .get(page)
            .ok_or_else(|| PlatformError::new("PLATFORM_RWE_PAGE_MISSING", page))?;
    }

    let out = state
        .frontend
        .rwe
        .render(
            compiled,
            input,
            state.frontend.language.as_ref(),
            &RenderContext {
                route: route.to_string(),
                request_id: format!("zebflow-{page}"),
                metadata: json!({"zebflow": true}),
                enabled_libraries: Vec::new(),
            },
        )
        .map_err(|e| PlatformError::new("PLATFORM_RWE_RENDER", e.to_string()))?;

    let mut html = out.html;

    // Ensure UTF-8 meta is present early in the document so browsers don't
    // fall back to Latin-1 when no DOCTYPE or explicit encoding is in the fragment.
    html = ensure_meta_charset(html);

    if let Some(css) = out.hydration_payload.get("css").and_then(Value::as_str) {
        html = crate::rwe::core::render::insert_engine_styles(&html, css);
    }

    // All platform pages depend on shared design tokens and reset CSS.
    html = ensure_stylesheet_link(html, "/assets/platform/main.css");
    html = ensure_favicon_links(html);
    // DB suite pages require dedicated layout rules.
    if html.contains("data-db-suite=\"true\"") {
        html = ensure_stylesheet_link(html, "/assets/platform/db-suite.css");
    }
    // Any page that uses devicon- classes gets the icon font CSS injected.
    if html.contains("devicon-") {
        html = ensure_stylesheet_link(html, "/assets/platform/devicons.css");
    }

    let final_html = externalize_rwe_scripts(state, html.as_str(), &out.compiled_scripts, None)?;

    // In debug builds: inject a tiny SSE listener that auto-reloads the page
    // whenever platform template sources change on disk.
    #[cfg(debug_assertions)]
    let final_html = {
        inject_before_body_end(
            &final_html,
            "<script>(function(){new EventSource('/__dev/reload').onmessage=function(){location.reload()}})();</script>",
        )
    };

    Ok(final_html)
}

fn ensure_meta_charset(mut html: String) -> String {
    if html.contains("<meta charset") || html.contains("<meta http-equiv=\"Content-Type\"") {
        return html;
    }
    let tag = "<meta charset=\"utf-8\">";
    if let Some(pos) = html.find("<head>") {
        html.insert_str(pos + "<head>".len(), tag);
    } else if let Some(pos) = html.find("</head>") {
        html.insert_str(pos, tag);
    } else {
        html = format!("{tag}{html}");
    }
    html
}

fn ensure_stylesheet_link(mut html: String, href: &str) -> String {
    if html.contains(href) {
        return html;
    }
    let link = format!("<link rel=\"stylesheet\" href=\"{href}\">");
    if let Some(pos) = html.find("</head>") {
        html.insert_str(pos, &link);
        return html;
    }
    format!("{link}{html}")
}

fn ensure_favicon_links(mut html: String) -> String {
    if html.contains("/favicon.svg") || html.contains("/apple-touch-icon.png") {
        return html;
    }
    let links = concat!(
        "<link rel=\"icon\" href=\"/favicon.svg\" type=\"image/svg+xml\">",
        "<link rel=\"icon\" href=\"/favicon-32.png\" sizes=\"32x32\" type=\"image/png\">",
        "<link rel=\"icon\" href=\"/favicon-16.png\" sizes=\"16x16\" type=\"image/png\">",
        "<link rel=\"apple-touch-icon\" href=\"/apple-touch-icon.png\" sizes=\"180x180\">",
    );
    if let Some(pos) = html.find("</head>") {
        html.insert_str(pos, links);
        return html;
    }
    format!("{links}{html}")
}

fn insert_project_theme_block(mut html: String, style_block: &str) -> String {
    let pos = html
        .find("<style data-rwe-style")
        .or_else(|| html.find("<style data-rwe-tw"))
        .or_else(|| html.find("</head>"));
    if let Some(pos) = pos {
        html.insert_str(pos, style_block);
        return html;
    }
    format!("{style_block}{html}")
}

fn externalize_rwe_scripts(
    state: &PlatformAppState,
    html: &str,
    compiled_scripts: &[CompiledScript],
    project_scope: Option<(&str, &str)>,
) -> Result<String, PlatformError> {
    // Read deployment_asset_base up front — needed on all return paths, not
    // just when scripts are present.  When set, every `/static/{owner}/{project}/`
    // occurrence in the final HTML is rewritten to `{base}/`, covering scripts,
    // uploaded files, images, and library chunks alike.
    let deployment_asset_base = match project_scope {
        Some((owner, project)) => state
            .platform
            .zebflow_cfg
            .read_or_default(owner, project)
            .map(|config| config.configs.rwe.deployment_asset_base)?,
        None => None,
    };

    // A project whose lock pins installed RWE libraries must load them from
    // its own installed copies at `data/hub/rwe-libraries/` — every accepted
    // lock source resolves there — and only the project-scoped asset route
    // can serve those. When a deployment asset base is set the existing
    // `{base}/libraries/` rewrite already lands on that route, so this only
    // applies without one.
    let hub_library_base = match (project_scope, &deployment_asset_base) {
        (Some((owner, project)), None) => state
            .platform
            .dependency_lock
            .read(owner, project)
            .ok()
            .filter(|lock| !lock.rwe.libraries.is_empty())
            .map(|_| format!("/static/{owner}/{project}/_rwe/lib/")),
        _ => None,
    };

    // Rewrite remaining `/static/{owner}/{project}/` occurrences that were not
    // handled by the script-tag logic below (images, uploads, library chunks).
    let rewrite_assets = |s: String| -> String {
        let s = match (project_scope, &deployment_asset_base) {
            (Some((owner, project)), Some(base)) => {
                let from = format!("/assets/{}/{}/", owner, project);
                let to = format!("{}/", base.trim_end_matches('/'));
                s.replace(&from, &to)
            }
            _ => s,
        };
        match &hub_library_base {
            Some(to) => s.replace("/assets/libraries/", to),
            None => s,
        }
    };

    let Some(cache) = &state.render_script_cache else {
        return Ok(rewrite_assets(html.to_string()));
    };
    if compiled_scripts.is_empty() {
        return Ok(rewrite_assets(html.to_string()));
    }

    let mut script_tags = String::new();
    for script in compiled_scripts {
        if script.content_hash.trim().is_empty() {
            continue;
        }

        // When `deployment_asset_base` is set, the HTML script-tag src is already
        // rewritten to `{base}/rwe/scripts/{hash}`. But the compiled script *content*
        // has `/assets/libraries/` paths baked in at render time (preact bundle,
        // zeb/* library imports). Those paths resolve against the page origin at
        // runtime — they must be rewritten to point to the same CDN base.
        // We do a content-patch here, store under the new hash, and use the new
        // hash for the script tag src so CAS integrity is maintained.
        let maybe_patched: Option<CompiledScript> = {
            const LIB_FROM: &str = "/assets/libraries/";
            let lib_to = match &deployment_asset_base {
                Some(base) => Some(format!("{}/_rwe/lib/", base.trim_end_matches('/'))),
                // Same content-patch for hub-installed libraries: the compiled
                // script must import from the project-scoped route that serves
                // the installed copy.
                None => hub_library_base.clone(),
            };
            match lib_to {
                Some(lib_to) if script.content.contains(LIB_FROM) => {
                    use sha2::{Digest, Sha256};
                    let new_content = script.content.replace(LIB_FROM, &lib_to);
                    let new_hash = hex::encode(Sha256::digest(new_content.as_bytes()));
                    Some(CompiledScript {
                        content: new_content,
                        content_hash: new_hash,
                        ..script.clone()
                    })
                }
                _ => None,
            }
        };
        let script = maybe_patched.as_ref().unwrap_or(script);

        let store_res = match project_scope {
            Some((owner, project)) => cache.store_scoped(owner, project, script),
            None => cache.store(script),
        };
        if store_res.is_err() {
            continue;
        }
        let role = if script.id == "runtime" {
            "runtime"
        } else {
            "page"
        };
        let src = if let Some(ref base) = deployment_asset_base {
            format!("{base}/_rwe/scripts/{}", script.content_hash)
        } else {
            match project_scope {
                Some((owner, project)) => {
                    format!(
                        "/static/{owner}/{project}/_rwe/scripts/{}",
                        script.content_hash
                    )
                }
                None => format!("/assets/rwe/scripts/{}", script.content_hash),
            }
        };
        script_tags.push_str(&format!(
            "<script type=\"module\" defer data-rwe-external=\"{}\" src=\"{}\"></script>",
            role, src
        ));
    }
    if script_tags.is_empty() {
        return Ok(rewrite_assets(html.to_string()));
    }

    let stripped = strip_inline_runtime_bundle(html);
    Ok(rewrite_assets(inject_before_body_end(
        &stripped,
        &script_tags,
    )))
}

fn strip_inline_runtime_bundle(html: &str) -> String {
    let marker = "<script data-rwe-runtime=";
    let Some(start) = html.find(marker) else {
        return html.to_string();
    };
    let Some(end_rel) = html[start..].find("</script>") else {
        return html.to_string();
    };
    let end = start + end_rel + "</script>".len();
    let mut out = String::with_capacity(html.len());
    out.push_str(&html[..start]);
    out.push_str(&html[end..]);
    out
}

fn inject_before_body_end(html: &str, chunk: &str) -> String {
    if let Some(idx) = html.rfind("</body>") {
        let mut out = String::with_capacity(html.len() + chunk.len());
        out.push_str(&html[..idx]);
        out.push_str(chunk);
        out.push_str(&html[idx..]);
        out
    } else {
        let mut out = String::with_capacity(html.len() + chunk.len());
        out.push_str(html);
        out.push_str(chunk);
        out
    }
}

// ---------------------------------------------------------------------------
// Liveness + readiness probes (no auth required — used by K8s probes)
// ---------------------------------------------------------------------------

/// GET /health — liveness probe: always 200 if the process is alive.
async fn health_handler() -> impl IntoResponse {
    Json(json!({"status": "ok", "version": APP_VERSION}))
}

/// GET /ready — readiness probe: 200 if V8 worker pool is alive, 503 otherwise.
async fn ready_handler() -> impl IntoResponse {
    use crate::rwe::core::deno_worker;
    if deno_worker::is_pool_ready() {
        (StatusCode::OK, Json(json!({"status": "ready"}))).into_response()
    } else {
        (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({"status": "not_ready"})),
        )
            .into_response()
    }
}

// ---------------------------------------------------------------------------
// OAuth discovery endpoints (RFC 9728 / MCP 2025-03-26 spec)
// No auth required — these are public discovery documents.
// ---------------------------------------------------------------------------

/// GET /.well-known/oauth-protected-resource
async fn oauth_protected_resource_handler(headers: HeaderMap) -> impl IntoResponse {
    let host = headers
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("localhost");
    let scheme = if host.starts_with("localhost") || host.starts_with("127.") {
        "http"
    } else {
        "https"
    };
    Json(json!({
        "resource": format!("{}://{}", scheme, host),
        "bearer_methods_supported": ["header"]
    }))
}

/// GET /.well-known/oauth-authorization-server
async fn oauth_authorization_server_handler(headers: HeaderMap) -> impl IntoResponse {
    let host = headers
        .get("host")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("localhost");
    let scheme = if host.starts_with("localhost") || host.starts_with("127.") {
        "http"
    } else {
        "https"
    };
    Json(json!({
        "issuer": format!("{}://{}", scheme, host),
        "token_endpoint": format!("{}://{}/api/mcp/token", scheme, host),
        "response_types_supported": ["token"]
    }))
}

async fn root_redirect() -> Redirect {
    Redirect::to(LOGIN_PATH)
}

#[derive(Debug, Clone, Default, Deserialize)]
struct PipelineRegistryQuery {
    #[serde(rename = "type")]
    editor_type: Option<String>,
    path: Option<String>,
    file: Option<String>,
    line: Option<u32>,
    scope: Option<String>,
    id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PipelineRegistryScope {
    Path,
    Project,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct PipelineListQuery {
    path: Option<String>,
    recursive: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct PipelineByIdQuery {
    id: Option<String>,
    include_source: Option<bool>,
    include_active_source: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct UnifiedEditorQuery {
    #[serde(rename = "type")]
    editor_type: Option<String>,
    path: Option<String>,
    file: Option<String>,
    line: Option<u32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct TemplatePathQuery {
    path: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct DocPathQuery {
    path: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct DbSuiteQuery {
    table: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct DbDescribeQuery {
    scope: Option<String>,
    schema: Option<String>,
    /// Table whose columns are wanted. Required by `scope=columns`, which is
    /// unreachable without it.
    table: Option<String>,
    include_system: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct DbObjectListQuery {
    schema: Option<String>,
    include_system: Option<bool>,
}

#[derive(Debug, Clone, Default, Deserialize)]
struct DbTablePreviewQuery {
    table: Option<String>,
    limit: Option<usize>,
}

#[derive(Debug, Clone, serde::Serialize, Deserialize)]
#[serde(default)]
struct AssistantChatRequest {
    message: String,
    history: Vec<AssistantChatMessage>,
    use_high_model: bool,
    current_page: Option<String>,
    client_time: Option<String>,
}

impl Default for AssistantChatRequest {
    fn default() -> Self {
        Self {
            message: String::new(),
            history: Vec::new(),
            use_high_model: false,
            current_page: None,
            client_time: None,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, Deserialize)]
struct AssistantChatMessage {
    role: String,
    content: String,
}

async fn branding_asset(Path(asset): Path<String>) -> Response {
    match asset.as_str() {
        "logo.svg" => asset_response("image/svg+xml; charset=utf-8", BRAND_LOGO_SVG),
        "logo.png" => asset_response("image/png", BRAND_LOGO_PNG),
        "favicon.svg" => asset_response("image/svg+xml; charset=utf-8", BRAND_FAVICON_SVG),
        "favicon.ico" => asset_response("image/x-icon", BRAND_FAVICON_ICO),
        "favicon-16.png" => asset_response("image/png", BRAND_FAVICON_16_PNG),
        "favicon-32.png" => asset_response("image/png", BRAND_FAVICON_32_PNG),
        "apple-touch-icon.png" => asset_response("image/png", BRAND_APPLE_TOUCH_ICON_PNG),
        _ => (StatusCode::NOT_FOUND, "asset not found").into_response(),
    }
}

async fn favicon_ico_asset() -> Response {
    asset_response("image/x-icon", BRAND_FAVICON_ICO)
}

async fn favicon_svg_asset() -> Response {
    asset_response("image/svg+xml; charset=utf-8", BRAND_FAVICON_SVG)
}

async fn favicon_16_asset() -> Response {
    asset_response("image/png", BRAND_FAVICON_16_PNG)
}

async fn favicon_32_asset() -> Response {
    asset_response("image/png", BRAND_FAVICON_32_PNG)
}




// Vendored web fonts. Embedded rather than read from disk so an install that
// never had a network still draws in the faces the Studio was designed in.
// One file per family and subset — they are variable fonts, so each carries
// every weight.
const FONT_HANKENGROTESK_CYRILLIC_EXT_WOFF2: &[u8] = include_bytes!("templates/styles/fonts/HankenGrotesk-cyrillic-ext.woff2");
const FONT_HANKENGROTESK_LATIN_EXT_WOFF2: &[u8] = include_bytes!("templates/styles/fonts/HankenGrotesk-latin-ext.woff2");
const FONT_HANKENGROTESK_LATIN_WOFF2: &[u8] = include_bytes!("templates/styles/fonts/HankenGrotesk-latin.woff2");
const FONT_HANKENGROTESK_VIETNAMESE_WOFF2: &[u8] = include_bytes!("templates/styles/fonts/HankenGrotesk-vietnamese.woff2");
const FONT_JETBRAINSMONO_CYRILLIC_EXT_WOFF2: &[u8] = include_bytes!("templates/styles/fonts/JetBrainsMono-cyrillic-ext.woff2");
const FONT_JETBRAINSMONO_CYRILLIC_WOFF2: &[u8] = include_bytes!("templates/styles/fonts/JetBrainsMono-cyrillic.woff2");
const FONT_JETBRAINSMONO_GREEK_WOFF2: &[u8] = include_bytes!("templates/styles/fonts/JetBrainsMono-greek.woff2");
const FONT_JETBRAINSMONO_LATIN_EXT_WOFF2: &[u8] = include_bytes!("templates/styles/fonts/JetBrainsMono-latin-ext.woff2");
const FONT_JETBRAINSMONO_LATIN_WOFF2: &[u8] = include_bytes!("templates/styles/fonts/JetBrainsMono-latin.woff2");
const FONT_JETBRAINSMONO_VIETNAMESE_WOFF2: &[u8] = include_bytes!("templates/styles/fonts/JetBrainsMono-vietnamese.woff2");
const FONT_SPACEGROTESK_LATIN_EXT_WOFF2: &[u8] = include_bytes!("templates/styles/fonts/SpaceGrotesk-latin-ext.woff2");
const FONT_SPACEGROTESK_LATIN_WOFF2: &[u8] = include_bytes!("templates/styles/fonts/SpaceGrotesk-latin.woff2");
const FONT_SPACEGROTESK_VIETNAMESE_WOFF2: &[u8] = include_bytes!("templates/styles/fonts/SpaceGrotesk-vietnamese.woff2");

async fn apple_touch_icon_asset() -> Response {
    asset_response("image/png", BRAND_APPLE_TOUCH_ICON_PNG)
}

async fn platform_asset(Path(asset): Path<String>) -> Response {
    // In debug builds, serve CSS directly from source so edits are visible without rebuild.
    #[cfg(debug_assertions)]
    match asset.as_str() {
        "main.css" => {
            let css = [
                fs::read_to_string(format!("{TEMPLATE_SOURCE_DIR}/styles/fonts.css")),
                fs::read_to_string(format!("{TEMPLATE_SOURCE_DIR}/styles/main.css")),
                fs::read_to_string(format!(
                    "{TEMPLATE_SOURCE_DIR}/pages/project-studio/styles.css"
                )),
            ]
            .map(|r| r.unwrap_or_default())
            .join("\n\n");
            return asset_response("text/css; charset=utf-8", css.as_bytes());
        }
        "db-suite.css" => {
            let css = fs::read_to_string(format!("{TEMPLATE_SOURCE_DIR}/styles/db-suite.css"))
                .unwrap_or_default();
            return asset_response("text/css; charset=utf-8", css.as_bytes());
        }
        "db-connections.css" => {
            let css =
                fs::read_to_string(format!("{TEMPLATE_SOURCE_DIR}/styles/db-connections.css"))
                    .unwrap_or_default();
            return asset_response("text/css; charset=utf-8", css.as_bytes());
        }
        _ => {}
    }

    match asset.as_str() {
        "fonts/HankenGrotesk-cyrillic-ext.woff2" => asset_response("font/woff2", FONT_HANKENGROTESK_CYRILLIC_EXT_WOFF2),
        "fonts/HankenGrotesk-latin-ext.woff2" => asset_response("font/woff2", FONT_HANKENGROTESK_LATIN_EXT_WOFF2),
        "fonts/HankenGrotesk-latin.woff2" => asset_response("font/woff2", FONT_HANKENGROTESK_LATIN_WOFF2),
        "fonts/HankenGrotesk-vietnamese.woff2" => asset_response("font/woff2", FONT_HANKENGROTESK_VIETNAMESE_WOFF2),
        "fonts/JetBrainsMono-cyrillic-ext.woff2" => asset_response("font/woff2", FONT_JETBRAINSMONO_CYRILLIC_EXT_WOFF2),
        "fonts/JetBrainsMono-cyrillic.woff2" => asset_response("font/woff2", FONT_JETBRAINSMONO_CYRILLIC_WOFF2),
        "fonts/JetBrainsMono-greek.woff2" => asset_response("font/woff2", FONT_JETBRAINSMONO_GREEK_WOFF2),
        "fonts/JetBrainsMono-latin-ext.woff2" => asset_response("font/woff2", FONT_JETBRAINSMONO_LATIN_EXT_WOFF2),
        "fonts/JetBrainsMono-latin.woff2" => asset_response("font/woff2", FONT_JETBRAINSMONO_LATIN_WOFF2),
        "fonts/JetBrainsMono-vietnamese.woff2" => asset_response("font/woff2", FONT_JETBRAINSMONO_VIETNAMESE_WOFF2),
        "fonts/SpaceGrotesk-latin-ext.woff2" => asset_response("font/woff2", FONT_SPACEGROTESK_LATIN_EXT_WOFF2),
        "fonts/SpaceGrotesk-latin.woff2" => asset_response("font/woff2", FONT_SPACEGROTESK_LATIN_WOFF2),
        "fonts/SpaceGrotesk-vietnamese.woff2" => asset_response("font/woff2", FONT_SPACEGROTESK_VIETNAMESE_WOFF2),
        "main.css" => asset_response("text/css; charset=utf-8", PLATFORM_MAIN_CSS.as_bytes()),
        "db-suite.css" => {
            asset_response("text/css; charset=utf-8", PLATFORM_DB_SUITE_CSS.as_bytes())
        }
        "db-connections.css" => asset_response(
            "text/css; charset=utf-8",
            PLATFORM_DB_CONNECTIONS_CSS.as_bytes(),
        ),
        "devicons.css" => {
            asset_response("text/css; charset=utf-8", PLATFORM_DEVICONS_CSS.as_bytes())
        }
        _ => (StatusCode::NOT_FOUND, "asset not found").into_response(),
    }
}

async fn node_icon_asset(Path(path): Path<String>) -> Response {
    let normalized = path.trim_start_matches('/').replace('\\', "/");
    #[cfg(debug_assertions)]
    {
        let source_path = FsPath::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/platform/web/assets/node-icons")
            .join(&normalized);
        if source_path.is_file() {
            match fs::read(&source_path) {
                Ok(bytes) => {
                    return asset_response(content_type_for_path(FsPath::new(&normalized)), &bytes);
                }
                Err(_) => return (StatusCode::NOT_FOUND, "asset not found").into_response(),
            }
        }
    }

    match platform_node_icon_asset(&normalized) {
        Some(bytes) => asset_response(content_type_for_path(FsPath::new(&normalized)), bytes),
        None => (StatusCode::NOT_FOUND, "asset not found").into_response(),
    }
}

async fn library_asset(Path(path): Path<String>) -> Response {
    let normalized = path.trim_start_matches('/').replace('\\', "/");
    match platform_library_asset(&normalized) {
        Some(bytes) => asset_response(content_type_for_path(FsPath::new(&normalized)), bytes),
        None => (StatusCode::NOT_FOUND, "asset not found").into_response(),
    }
}

/// Serves platform library assets via the project-scoped asset path.
/// Used when `deployment_asset_base` is set and a CDN/nginx rule maps
/// `/base/` → `/static/{owner}/{project}/` — library bundle requests
/// (e.g. `/static/{owner}/{project}/_rwe/lib/zeb/preact/...`) are routed here
/// so a single nginx rule covers both project assets and library bundles.
async fn project_scoped_library_asset(
    State(state): State<PlatformAppState>,
    Path((owner, project, path)): Path<(String, String, String)>,
) -> Response {
    let normalized = path.trim_start_matches('/').replace('\\', "/");
    // A library the project installed is served from its installed copy: the
    // request names the library (`zeb/{name}/{rest}`), the lock entry names
    // where its package landed (`rwe-libraries/{package_id}/…`), and the rest
    // of the path resolves inside that package directory. Anything not
    // locked falls back to the embedded bytes, so the platform's own pages
    // keep working unchanged.
    if !normalized.split('/').any(|segment| segment == "..")
        && let Some(rest) = normalized.strip_prefix("zeb/")
        && let Some((lib_slug, lib_rest)) = rest.split_once('/')
    {
        let library_name = format!("zeb/{lib_slug}");
        // The lock's digest is checked before a single installed byte is
        // served: "Integrity validation happens before an artifact becomes
        // available to RWE or the node registry"
        // (`kinds/dependency-lock/README.md`). A drifted install is refused
        // out loud rather than quietly answered with embedded bytes, because
        // silently serving a different library than the one the lock pins is
        // exactly the substitution the digest exists to catch.
        let install_root = match state.platform.dependency_lock.verified_rwe_library_root(
            &owner,
            &project,
            &library_name,
        ) {
            Ok(root) => root,
            Err(err) if err.code == "PLATFORM_DEPENDENCY_INTEGRITY" => {
                return (StatusCode::CONFLICT, err.message).into_response();
            }
            // Missing bytes or an unreadable lock are not an integrity
            // failure; the embedded copy still answers, as it did before any
            // library was installed.
            Err(_) => None,
        };
        if let Some(install_root) = install_root {
            let installed = install_root.join(lib_rest);
            if installed.starts_with(&install_root)
                && installed.is_file()
                && let Ok(bytes) = fs::read(&installed)
            {
                return asset_response(content_type_for_path(FsPath::new(&normalized)), &bytes);
            }
        }
    }
    // Not installed: answer with the hub's catalogue copy, which is what an
    // install would have given this project. The Studio's vendored copies are
    // deliberately not consulted here — they are the platform's pin, not a
    // default for somebody else's project.
    match hub_catalogue_asset(&normalized) {
        Some(bytes) => asset_response(content_type_for_path(FsPath::new(&normalized)), bytes),
        None => (StatusCode::NOT_FOUND, "asset not found").into_response(),
    }
}

async fn rwe_script_asset(
    State(state): State<PlatformAppState>,
    Path(hash): Path<String>,
) -> Response {
    if !hash
        .bytes()
        .all(|ch| ch.is_ascii_hexdigit() || ch == b'-' || ch == b'_')
    {
        return (StatusCode::BAD_REQUEST, "invalid script hash").into_response();
    }
    let Some(cache) = &state.render_script_cache else {
        return (StatusCode::NOT_FOUND, "script cache unavailable").into_response();
    };
    let Some(content) = cache.get(&hash).ok().flatten() else {
        return (StatusCode::NOT_FOUND, "script not found").into_response();
    };

    let mut resp = Response::new(Body::from(content));
    *resp.status_mut() = StatusCode::OK;
    resp.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("text/javascript; charset=utf-8"),
    );
    resp.headers_mut().insert(
        CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    resp
}

async fn project_rwe_script_asset(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    uri: Uri,
    Path((owner, project, hash)): Path<(String, String, String)>,
) -> Response {
    if !hash
        .bytes()
        .all(|ch| ch.is_ascii_hexdigit() || ch == b'-' || ch == b'_')
    {
        return (StatusCode::BAD_REQUEST, "invalid script hash").into_response();
    }
    let valid_segment = |value: &str| {
        value
            .bytes()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == b'-' || ch == b'_')
    };
    if !valid_segment(&owner) || !valid_segment(&project) {
        return (StatusCode::BAD_REQUEST, "invalid project scope").into_response();
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::GET,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }
    let Some(cache) = &state.render_script_cache else {
        return (StatusCode::NOT_FOUND, "script cache unavailable").into_response();
    };

    let scoped = cache.get_scoped(&owner, &project, &hash).ok().flatten();
    let global = cache.get(&hash).ok().flatten();
    let Some(content) = scoped.or(global) else {
        return (StatusCode::NOT_FOUND, "script not found").into_response();
    };

    let mut resp = Response::new(Body::from(content));
    *resp.status_mut() = StatusCode::OK;
    resp.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("text/javascript; charset=utf-8"),
    );
    resp.headers_mut().insert(
        CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=31536000, immutable"),
    );
    resp
}

async fn project_asset(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    uri: Uri,
    Path((owner, project, path)): Path<(String, String, String)>,
) -> Response {
    let valid_segment = |value: &str| {
        value
            .bytes()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == b'-' || ch == b'_')
    };
    if !valid_segment(&owner) || !valid_segment(&project) {
        return (StatusCode::BAD_REQUEST, "invalid project scope").into_response();
    }

    let normalized = path.trim_start_matches('/').replace('\\', "/");
    if normalized.is_empty() || normalized.contains("..") {
        return (StatusCode::BAD_REQUEST, "invalid asset path").into_response();
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::GET,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }

    let layout = match state.platform.file.ensure_project_layout(&owner, &project) {
        Ok(layout) => layout,
        Err(err) => return internal_error(err),
    };
    let root = layout.data_cache_dir().join("web-assets");
    let abs = root.join(&normalized);
    if !abs.starts_with(&root) {
        return (StatusCode::BAD_REQUEST, "invalid asset path").into_response();
    }
    // Try compiled web-assets first; fall back to repo/pipelines/assets/.
    let (serve_bytes, serve_path) = if abs.is_file() {
        match std::fs::read(&abs) {
            Ok(b) => (b, abs),
            Err(err) => {
                return internal_error(PlatformError::new("PLATFORM_ASSET_READ", err.to_string()));
            }
        }
    } else {
        let static_root = layout.repo_static_dir();
        let static_abs = static_root.join(&normalized);
        if !static_abs.starts_with(&static_root) || !static_abs.is_file() {
            return (StatusCode::NOT_FOUND, "asset not found").into_response();
        }
        match std::fs::read(&static_abs) {
            Ok(b) => (b, static_abs),
            Err(err) => {
                return internal_error(PlatformError::new(
                    "PLATFORM_STATIC_ASSET_READ",
                    err.to_string(),
                ));
            }
        }
    };
    let mut resp = Response::new(Body::from(serve_bytes));
    *resp.status_mut() = StatusCode::OK;
    if let Ok(v) = HeaderValue::from_str(content_type_for_path(&serve_path)) {
        resp.headers_mut().insert(CONTENT_TYPE, v);
    }
    // An authored file keeps its name when its contents change, so it may not be
    // cached as immutable -- editing `logo.png` would otherwise never reach a
    // browser that had seen the old one. The content-hashed machinery that can
    // be cached forever lives under the reserved `_rwe/` prefix and is served by
    // its own handlers.
    resp.headers_mut().insert(
        CACHE_CONTROL,
        HeaderValue::from_static("public, max-age=0, must-revalidate"),
    );
    resp
}

/// Serves static project assets from `repo/pipelines/assets/{*path}`.
/// Route: GET /p/{owner}/{project}/assets/{*path}
async fn project_static_asset(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    uri: Uri,
    Path((owner, project, path)): Path<(String, String, String)>,
) -> Response {
    let valid_segment = |value: &str| {
        value
            .bytes()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == b'-' || ch == b'_')
    };
    if !valid_segment(&owner) || !valid_segment(&project) {
        return (StatusCode::BAD_REQUEST, "invalid project scope").into_response();
    }

    let normalized = path.trim_start_matches('/').replace('\\', "/");
    if normalized.is_empty() || normalized.contains("..") {
        return (StatusCode::BAD_REQUEST, "invalid asset path").into_response();
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::GET,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }

    let layout = match state.platform.file.ensure_project_layout(&owner, &project) {
        Ok(layout) => layout,
        Err(err) => return internal_error(err),
    };
    let assets_root = layout.repo_static_dir();
    let abs = assets_root.join(&normalized);
    if !abs.starts_with(&assets_root) {
        return (StatusCode::BAD_REQUEST, "invalid asset path").into_response();
    }
    if !abs.is_file() {
        return (StatusCode::NOT_FOUND, "asset not found").into_response();
    }
    let bytes = match std::fs::read(&abs) {
        Ok(bytes) => bytes,
        Err(err) => {
            return internal_error(PlatformError::new(
                "PLATFORM_STATIC_ASSET_READ",
                err.to_string(),
            ));
        }
    };
    let mut resp = Response::new(Body::from(bytes));
    *resp.status_mut() = StatusCode::OK;
    if let Ok(v) = HeaderValue::from_str(content_type_for_path(&abs)) {
        resp.headers_mut().insert(CONTENT_TYPE, v);
    }
    resp
}

/// Legacy route: GET /files/{owner}/{project}/{*path}
///
/// Keeps the older public/private convention:
/// - `public/*` can be read anonymously;
/// - every other path requires project `FilesRead`.
///
/// The newer `/fs/{owner}/{project}/{*path}` route remains private-by-default.
async fn project_legacy_file_serve(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, path)): Path<(String, String, String)>,
    uri: Uri,
) -> Response {
    let valid_segment = |value: &str| {
        value
            .bytes()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == b'-' || ch == b'_')
    };
    if !valid_segment(&owner) || !valid_segment(&project) {
        return (StatusCode::BAD_REQUEST, "invalid project scope").into_response();
    }

    let normalized = path.trim_start_matches('/').replace('\\', "/");
    if normalized.is_empty() || normalized.contains("..") {
        return (StatusCode::BAD_REQUEST, "invalid object path").into_response();
    }

    if !normalized.starts_with("public/")
        && let Err(response) = require_project_api_capability(
            &state,
            &headers,
            &owner,
            &project,
            ProjectCapability::FilesRead,
        )
    {
        return response;
    }

    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::GET,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }

    let layout = match state.platform.file.ensure_project_layout(&owner, &project) {
        Ok(layout) => layout,
        Err(err) => return internal_error(err),
    };
    let zebfs = layout.open_files();
    let (_, abs_path) = match zebfs.resolve_object_path(&normalized) {
        Ok(resolved) => resolved,
        Err(err) if err.code == "ZEBFS_INVALID_PATH" => {
            return (StatusCode::BAD_REQUEST, "invalid object path").into_response();
        }
        Err(err) => return internal_error(PlatformError::new(err.code, err.message)),
    };
    if let Ok(abs_canonical) = std::fs::canonicalize(&abs_path) {
        let root_canonical =
            std::fs::canonicalize(&layout.files_dir).unwrap_or_else(|_| layout.files_dir.clone());
        if !abs_canonical.starts_with(&root_canonical) {
            return (StatusCode::BAD_REQUEST, "invalid object path").into_response();
        }
    }
    let object = match zebfs.get(&normalized) {
        Ok(object) => object,
        Err(err) if err.code == "ZEBFS_NOT_FOUND" => {
            return (StatusCode::NOT_FOUND, "object not found").into_response();
        }
        Err(err) if err.code == "ZEBFS_INVALID_PATH" => {
            return (StatusCode::BAD_REQUEST, "invalid object path").into_response();
        }
        Err(err) => return internal_error(PlatformError::new(err.code, err.message)),
    };

    let mut resp = Response::new(Body::from(object.bytes));
    *resp.status_mut() = StatusCode::OK;
    let content_type = content_type_for_path(FsPath::new(&object.path));
    if let Ok(v) = HeaderValue::from_str(content_type) {
        resp.headers_mut().insert(CONTENT_TYPE, v);
    }
    if normalized.starts_with("public/") {
        resp.headers_mut().insert(
            CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=300"),
        );
    } else if let Ok(v) = HeaderValue::from_str("no-cache") {
        resp.headers_mut().insert(CACHE_CONTROL, v);
    }
    resp
}

/// Route: GET /fs/{owner}/{project}/{*path}
/// Serves one object from the project default ZebFS namespace.
async fn project_fs_serve(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Query(params): Query<std::collections::HashMap<String, String>>,
    Path((owner, project, path)): Path<(String, String, String)>,
    uri: Uri,
) -> Response {
    let valid_segment = |value: &str| {
        value
            .bytes()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == b'-' || ch == b'_')
    };
    if !valid_segment(&owner) || !valid_segment(&project) {
        return (StatusCode::BAD_REQUEST, "invalid project scope").into_response();
    }

    let public_proxy_request = is_controller_call(&state, &headers)
        && headers
            .get(PUBLIC_FS_PROXY_HEADER)
            .and_then(|value| value.to_str().ok())
            .map(|value| value == "1")
            .unwrap_or(false);
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        match require_project_api_capability(
            &state,
            &headers,
            &owner,
            &project,
            ProjectCapability::FilesRead,
        ) {
            Ok(_) => {
                return match forward_project_api_request_to_worker(
                    &state,
                    &uri,
                    &Method::GET,
                    &headers,
                    Bytes::new(),
                    &worker_id,
                )
                .await
                {
                    Ok(response) => response,
                    Err(err) => internal_error(err),
                };
            }
            Err(_) => {
                let mut public_headers = headers.clone();
                public_headers.insert(PUBLIC_FS_PROXY_HEADER, HeaderValue::from_static("1"));
                return match forward_project_api_request_to_worker(
                    &state,
                    &uri,
                    &Method::GET,
                    &public_headers,
                    Bytes::new(),
                    &worker_id,
                )
                .await
                {
                    Ok(response) => response,
                    Err(err) => internal_error(err),
                };
            }
        }
    }

    let layout = match state.platform.file.ensure_project_layout(&owner, &project) {
        Ok(layout) => layout,
        Err(err) => return internal_error(err),
    };
    let zebfs = layout.open_files();
    let is_public = match crate::platform::services::zebfs_acl::is_public_read(&zebfs, &path) {
        Ok(value) => value,
        Err(err) if err.code == "ZEBFS_INVALID_PATH" || err.code == "ZEBFS_RESERVED_PATH" => {
            return (StatusCode::BAD_REQUEST, "invalid object path").into_response();
        }
        Err(err) => return internal_error(PlatformError::new(err.code, err.message)),
    };
    if !is_public {
        if public_proxy_request {
            return StatusCode::UNAUTHORIZED.into_response();
        }
        if let Err(response) = require_project_api_capability(
            &state,
            &headers,
            &owner,
            &project,
            ProjectCapability::FilesRead,
        ) {
            return response;
        }
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::GET,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }

    let object = match zebfs.get(&path) {
        Ok(object) => object,
        Err(err) if err.code == "ZEBFS_NOT_FOUND" => {
            return (StatusCode::NOT_FOUND, "object not found").into_response();
        }
        Err(err) if err.code == "ZEBFS_INVALID_PATH" => {
            return (StatusCode::BAD_REQUEST, "invalid object path").into_response();
        }
        Err(err) => return internal_error(PlatformError::new(err.code, err.message)),
    };

    let mut resp = Response::new(Body::from(object.bytes));
    *resp.status_mut() = StatusCode::OK;
    let content_type = content_type_for_path(FsPath::new(&object.path));
    if let Ok(v) = HeaderValue::from_str(content_type) {
        resp.headers_mut().insert(CONTENT_TYPE, v);
    }
    let download_requested = query_flag_enabled(params.get("download").map(String::as_str));
    if let Some(disposition) =
        file_content_disposition(FsPath::new(&object.path), content_type, download_requested)
    {
        resp.headers_mut().insert(CONTENT_DISPOSITION, disposition);
    }
    if is_public {
        resp.headers_mut().insert(
            axum::http::header::CACHE_CONTROL,
            HeaderValue::from_static("public, max-age=300"),
        );
    } else if let Ok(v) = HeaderValue::from_str("no-cache") {
        resp.headers_mut()
            .insert(axum::http::header::CACHE_CONTROL, v);
    }
    resp
}

fn asset_response(content_type: &'static str, bytes: &[u8]) -> Response {
    let mut resp = Response::new(Body::from(bytes.to_vec()));
    *resp.status_mut() = StatusCode::OK;
    if let Ok(v) = HeaderValue::from_str(content_type) {
        resp.headers_mut().insert(CONTENT_TYPE, v);
    }
    resp
}

fn query_flag_enabled(value: Option<&str>) -> bool {
    matches!(
        value.map(str::trim),
        Some("1")
            | Some("true")
            | Some("TRUE")
            | Some("yes")
            | Some("YES")
            | Some("on")
            | Some("ON")
    )
}

fn file_content_disposition(
    path: &FsPath,
    content_type: &'static str,
    download_requested: bool,
) -> Option<HeaderValue> {
    let file_name = path.file_name()?.to_string_lossy().replace('"', "");
    let disposition_kind = if download_requested {
        "attachment"
    } else if is_inline_file_content_type(content_type) {
        "inline"
    } else {
        return None;
    };
    HeaderValue::from_str(&format!("{disposition_kind}; filename=\"{file_name}\"")).ok()
}

fn is_inline_file_content_type(content_type: &str) -> bool {
    matches!(
        content_type,
        "text/javascript; charset=utf-8"
            | "application/json; charset=utf-8"
            | "application/geo+json; charset=utf-8"
            | "application/xml; charset=utf-8"
            | "text/css; charset=utf-8"
            | "image/svg+xml; charset=utf-8"
            | "image/png"
            | "image/jpeg"
            | "image/gif"
            | "image/webp"
            | "image/x-icon"
            | "application/pdf"
            | "video/mp4"
            | "audio/mpeg"
            | "text/plain; charset=utf-8"
            | "text/csv; charset=utf-8"
            | "text/markdown; charset=utf-8"
            | "application/yaml; charset=utf-8"
    )
}

async fn login_page(State(state): State<PlatformAppState>) -> Response {
    match render_login_page(&state, None, StatusCode::OK) {
        Ok(resp) => resp,
        Err(err) => internal_error(err),
    }
}

fn render_login_page(
    state: &PlatformAppState,
    error: Option<&str>,
    status: StatusCode,
) -> Result<Response, PlatformError> {
    let html = render_page(
        state,
        "platform-login",
        "/login",
        json!({
            "seo": {
                "title": "Zebflow Platform Login",
                "description": "Login page for Zebflow platform"
            },
            "error": error.unwrap_or(""),
            "default_identifier": state.platform.config.default_owner,
            "app_version": APP_VERSION,
        }),
    )?;
    Ok((status, Html(html)).into_response())
}

async fn login_submit(
    State(state): State<PlatformAppState>,
    Form(req): Form<LoginRequest>,
) -> Response {
    match state.platform.auth.login(&req.identifier, &req.password) {
        Ok(Some(session)) => {
            // Grafana's rule: while the credential is still the generated one,
            // the browser lands on the change-password screen instead of home.
            // The session cookie is issued either way, so API/CLI callers that
            // POST /login for the Set-Cookie header are unaffected.
            let target = match state.platform.users.must_change_password(&session.owner) {
                Ok(true) => ACCOUNT_PASSWORD_PATH,
                _ => HOME_PATH,
            };
            let mut resp = Redirect::to(target).into_response();
            let token = issue_session(&state, &session.owner);
            let cookie = session_cookie_header(&token, SESSION_TTL_SECS);
            if let Ok(v) = HeaderValue::from_str(&cookie) {
                resp.headers_mut().insert(SET_COOKIE, v);
            }
            resp
        }
        Ok(None) => {
            match render_login_page(
                &state,
                Some("invalid credentials"),
                StatusCode::UNAUTHORIZED,
            ) {
                Ok(resp) => resp,
                Err(err) => internal_error(err),
            }
        }
        // `offices.md` §4: while joined, this door is closed and the
        // controller's is open. The refusal is shown in full on the login page
        // rather than reduced to a status code, because the operator standing
        // in front of it needs three facts and a next action, and it carries
        // nothing about the identifier or the password — the gate ran before
        // either was read.
        Err(err) if err.code == LOCAL_LOGIN_DISABLED_CODE => {
            match render_login_page(&state, Some(&err.message), StatusCode::FORBIDDEN) {
                Ok(resp) => resp,
                Err(err) => internal_error(err),
            }
        }
        Err(err) => internal_error(err),
    }
}

async fn logout_submit(State(state): State<PlatformAppState>, headers: HeaderMap) -> Response {
    if let Some(token) = session_token(&headers)
        && let Ok(mut sessions) = state.sessions.lock()
    {
        sessions.remove(&token);
    }
    let mut resp = Redirect::to(LOGIN_PATH).into_response();
    if let Ok(v) = HeaderValue::from_str(&session_cookie_header("", 0)) {
        resp.headers_mut().insert(SET_COOKIE, v);
    }
    resp
}

async fn home_page(State(state): State<PlatformAppState>, headers: HeaderMap) -> Response {
    let Some(owner) = session_owner(&state, &headers) else {
        return Redirect::to(LOGIN_PATH).into_response();
    };
    if let Some(source_owner) = platform_hub_source_owner(&state)
        && let Err(err) = state
            .platform
            .hub
            .ensure_default_platform_repository(&source_owner)
    {
        return internal_error(err);
    }

    match state.platform.projects.list_projects(&owner) {
        Ok(items) => {
            let known_workers = state
                .platform
                .cluster_registry
                .snapshot()
                .map(|snapshot| snapshot.workers)
                .unwrap_or_default();
            let all_projects = state
                .platform
                .data
                .list_users()
                .unwrap_or_default()
                .into_iter()
                .flat_map(|user| {
                    state
                        .platform
                        .projects
                        .list_projects(&user.owner)
                        .unwrap_or_default()
                        .into_iter()
                        .map(move |project| (user.owner.clone(), project.project.clone()))
                })
                .collect::<Vec<_>>();
            let mut hosted_projects: std::collections::BTreeMap<String, Vec<String>> =
                std::collections::BTreeMap::new();
            let local_office_id = state.platform.cluster_bootstrap.node_id();
            for (project_owner, project_slug) in &all_projects {
                let placement = state
                    .platform
                    .cluster_placement
                    .get(project_owner, project_slug)
                    .ok()
                    .flatten();
                let target_key = match placement.as_ref() {
                    Some(value) if value.target == ProjectRuntimePlacementTarget::Worker => value
                        .worker_id
                        .clone()
                        .unwrap_or_else(|| "__dangling__".to_string()),
                    _ => local_office_id.clone(),
                };
                hosted_projects
                    .entry(target_key)
                    .or_default()
                    .push(format!("{project_owner}/{project_slug}"));
            }
            let now = chrono::Utc::now().timestamp();
            let mut offices = vec![json!({
                "id": local_office_id,
                "label": state.platform.cluster_bootstrap.node_label(),
                "role": if state.platform.cluster_bootstrap.is_master() {
                    "Managing office"
                } else if state.platform.cluster_bootstrap.is_worker() {
                    "Office"
                } else {
                    "Self-managed office"
                },
                "availability": "online",
                "resource_state": "local runtime available",
                "address": state.platform.cluster_bootstrap.advertise_url().map(str::to_string),
                "version": APP_VERSION,
                "last_seen": "Current process",
                "hosted_project_count": hosted_projects
                    .get(&state.platform.cluster_bootstrap.node_id())
                    .map(|items| items.len())
                    .unwrap_or(0usize),
                "hosted_projects": hosted_projects
                    .get(&state.platform.cluster_bootstrap.node_id())
                    .cloned()
                    .unwrap_or_default(),
                "capabilities": vec!["resident".to_string()],
            })];
            let known_worker_ids = known_workers
                .iter()
                .map(|worker| worker.node_id.clone())
                .collect::<std::collections::BTreeSet<_>>();
            // `offices.md` §2's vouch verb, made one action. The link is
            // emitted only for a controller and only for an operator who could
            // mint anyway, so the card never offers a door the viewer cannot
            // open. The local office is excluded because the viewer is already
            // standing in it.
            let can_vouch =
                state.platform.cluster_bootstrap.is_master() && is_superadmin_owner(&state, &owner);
            for worker in known_workers {
                let worker_id = worker.node_id.clone();
                let worker_label = worker.label.clone();
                let worker_status = worker.status.clone();
                let worker_base_url = worker.base_url.clone();
                let heartbeat_age = if worker.last_heartbeat_at > 0 {
                    now.saturating_sub(worker.last_heartbeat_at)
                } else {
                    i64::MAX
                };
                let availability = if heartbeat_age > 45 {
                    "offline"
                } else {
                    "online"
                };
                let mut capabilities = Vec::new();
                if worker.capabilities.supports_resident {
                    capabilities.push("resident".to_string());
                }
                if worker.capabilities.supports_k8s_job {
                    capabilities.push("k8s-job".to_string());
                }
                if worker.capabilities.supports_spark_submit {
                    capabilities.push("spark-submit".to_string());
                }
                capabilities.extend(
                    worker
                        .capabilities
                        .tags
                        .iter()
                        .filter(|tag| !tag.starts_with("app_version:"))
                        .cloned(),
                );
                let version = worker
                    .capabilities
                    .tags
                    .iter()
                    .find_map(|tag| tag.strip_prefix("app_version:"))
                    .map(str::to_string);
                let last_seen = if heartbeat_age == i64::MAX {
                    "No heartbeat yet".to_string()
                } else if heartbeat_age <= 1 {
                    "just now".to_string()
                } else {
                    format!("{heartbeat_age}s ago")
                };
                offices.push(json!({
                    "id": worker_id,
                    "label": worker_label,
                    "role": "Joined office",
                    "availability": availability,
                    "resource_state": if worker_status.trim().is_empty() {
                        if availability == "online" {
                            "healthy".to_string()
                        } else {
                            "heartbeat stale".to_string()
                        }
                    } else {
                        worker_status
                    },
                    "address": if worker_base_url.trim().is_empty() { None::<String> } else { Some(worker_base_url) },
                    "version": version,
                    "last_seen": last_seen,
                    "hosted_project_count": hosted_projects.get(&worker_id).map(|items| items.len()).unwrap_or(0usize),
                    "hosted_projects": hosted_projects.get(&worker_id).cloned().unwrap_or_default(),
                    "capabilities": capabilities,
                    "open_url": if can_vouch {
                        Some(format!("/cluster/offices/{worker_id}/open"))
                    } else {
                        None
                    },
                }));
            }
            for (office_id, projects) in &hosted_projects {
                if office_id == "__dangling__"
                    || office_id == &state.platform.cluster_bootstrap.node_id()
                    || known_worker_ids.contains(office_id)
                {
                    continue;
                }
                offices.push(json!({
                    "id": office_id,
                    "label": office_id,
                    "role": "Missing office",
                    "availability": "dangling",
                    "resource_state": "project placement exists but office is not registered",
                    "address": serde_json::Value::Null,
                    "version": serde_json::Value::Null,
                    "last_seen": "No active registration",
                    "hosted_project_count": projects.len(),
                    "hosted_projects": projects,
                    "capabilities": Vec::<String>::new(),
                }));
            }
            let runtime_targets = state
                .platform
                .cluster_registry
                .runtime_target_options()
                .unwrap_or_else(|_| {
                    vec![crate::platform::model::ClusterRuntimeTargetOption {
                        value: "local".to_string(),
                        label: "Local office".to_string(),
                        description: "Run inside the current self-controlled office.".to_string(),
                    }]
                });
            let projects = match items
                .into_iter()
                .map(|item| home_project_card_json(&state, &owner, &item))
                .collect::<Result<Vec<_>, _>>()
            {
                Ok(projects) => projects,
                Err(err) => return internal_error(err),
            };
            match render_page(
                &state,
                "platform-home",
                "/home",
                json!({
                    "seo": {
                        "title": "Zebflow Platform Home",
                        "description": "Project list"
                    },
                    "owner": owner,
                    "projects": projects,
                    "hub_api": {
                        "service": "/api/platform/hub/service",
                        "repositories": format!("/api/users/{}/hub/repositories", owner),
                        "assets": format!("/api/users/{}/hub/assets", owner),
                        "install": format!("/api/users/{}/hub/install", owner),
                        "install_review": format!("/api/users/{}/hub/install/review", owner),
                    },
                    "offices": offices,
                    "runtime_targets": runtime_targets,
                    "app_version": APP_VERSION,
                }),
            ) {
                Ok(html) => Html(html).into_response(),
                Err(err) => internal_error(err),
            }
        }
        Err(err) => internal_error(err),
    }
}

async fn platform_hub_page(State(state): State<PlatformAppState>, headers: HeaderMap) -> Response {
    let Some(owner) = session_owner(&state, &headers) else {
        return Redirect::to(LOGIN_PATH).into_response();
    };
    let is_superadmin = is_superadmin_owner(&state, &owner);
    let service = match state.platform.hub.get_default_service_instance() {
        Ok(service) => service,
        Err(err) => return internal_error(err),
    };
    let (source_owner, source_rows) = match visible_platform_hub_sources(&state, &owner) {
        Ok(value) => value,
        Err(err) => return internal_error(err),
    };
    let repositories = source_rows
        .clone()
        .into_iter()
        .map(platform_hub_repository_json)
        .collect::<Vec<_>>();
    let remote_apps = state
        .platform
        .hub
        .fetch_platform_remote_app_rows_from_repositories(source_rows)
        .await
        .items;
    let local_packages = match state.platform.hub.list_asset_packages() {
        Ok(items) => items
            .into_iter()
            .filter(|item| item.visibility == "public" || item.visibility == "unlisted")
            .map(|item| {
                json!({
                    "package_id": item.package_id,
                    "title": item.title,
                    "description": item.description,
                    "asset_kind": item.asset_kind,
                    "publisher_id": item.publisher_id,
                    "publisher_display_name": item.publisher_display_name,
                    "visibility": item.visibility,
                    "updated_at": item.updated_at,
                })
            })
            .collect::<Vec<_>>(),
        Err(_) => Vec::new(),
    };
    let projects = match state.platform.projects.list_projects(&owner) {
        Ok(items) => match items
            .into_iter()
            .map(|item| {
                is_project_hub_producer_enabled(&state, &owner, &item.project).map(
                    |producer_enabled| {
                        json!({
                            "owner": item.owner,
                            "project": item.project,
                            "title": item.title,
                            "hub_href": format!("/projects/{owner}/{}/hub", item.project),
                            "producer_enabled": producer_enabled,
                        })
                    },
                )
            })
            .collect::<Result<Vec<_>, _>>()
        {
            Ok(projects) => projects,
            Err(err) => return internal_error(err),
        },
        Err(err) => return internal_error(err),
    };
    let publishers = if is_superadmin {
        match state
            .platform
            .hub
            .list_publishers(&source_owner, "platform")
        {
            Ok(items) => items
                .into_iter()
                .map(hub_publisher_json)
                .collect::<Vec<_>>(),
            Err(_) => Vec::new(),
        }
    } else {
        Vec::new()
    };
    let tokens = if is_superadmin {
        match state.platform.hub.list_all_tokens() {
            Ok(items) => items.into_iter().map(hub_token_json).collect::<Vec<_>>(),
            Err(_) => Vec::new(),
        }
    } else {
        Vec::new()
    };
    let grants = if is_superadmin {
        match state.platform.hub.list_access_grants(&source_owner) {
            Ok(items) => items
                .into_iter()
                .map(hub_access_grant_json)
                .collect::<Vec<_>>(),
            Err(_) => Vec::new(),
        }
    } else {
        Vec::new()
    };
    let offices = platform_office_rows(&state);
    match render_page(
        &state,
        "platform-hub",
        "/hub",
        json!({
            "seo": {
                "title": "Zebflow Hub",
                "description": "Platform hub management and app explorer."
            },
            "owner": owner,
            "source_owner": source_owner,
            "is_superadmin": is_superadmin,
            "service": service,
            "repositories": repositories,
            "remote_apps": remote_apps,
            "local_packages": local_packages,
            "projects": projects,
            "publishers": publishers,
            "tokens": tokens,
            "grants": grants,
            "offices": offices,
            "hub_api": {
                "service": "/api/platform/hub/service",
                "repositories": "/api/platform/hub/repositories",
                "grants": "/api/platform/hub/grants",
                "assets": "/api/platform/hub/assets",
                "install": "/api/platform/hub/install",
                "install_review": "/api/platform/hub/install/review",
                "publishers": "/api/platform/hub/publishers",
                "tokens": "/api/platform/hub/tokens"
            },
            "app_version": APP_VERSION,
        }),
    ) {
        Ok(html) => Html(html).into_response(),
        Err(err) => internal_error(err),
    }
}

fn home_project_card_json(
    state: &PlatformAppState,
    owner: &str,
    item: &crate::platform::model::PlatformProject,
) -> Result<Value, PlatformError> {
    let item_owner = if item.owner.trim().is_empty() {
        owner.to_string()
    } else {
        item.owner.clone()
    };
    let placement = state
        .platform
        .cluster_placement
        .get(&item_owner, &item.project)
        .ok()
        .flatten();
    let runtime_mode = match placement.as_ref() {
        Some(value) => value.mode.to_string(),
        None => state
            .platform
            .zebflow_cfg
            .get_runtime_profile(&item_owner, &item.project)?
            .mode
            .to_string(),
    };
    let runtime_summary = state
        .platform
        .cluster_placement
        .describe(placement.as_ref());
    let (office_label, office_url) = match placement.as_ref() {
        Some(value) if value.target == ProjectRuntimePlacementTarget::Worker => {
            if let Some(worker_id) = value.worker_id.as_deref() {
                match state.platform.cluster_registry.get_worker(worker_id) {
                    Ok(Some(worker)) => (
                        worker.label,
                        if worker.base_url.trim().is_empty() {
                            None
                        } else {
                            Some(worker.base_url)
                        },
                    ),
                    _ => (worker_id.to_string(), None),
                }
            } else {
                ("Remote office".to_string(), None)
            }
        }
        _ => (
            state.platform.cluster_bootstrap.node_label(),
            state
                .platform
                .cluster_bootstrap
                .advertise_url()
                .map(str::to_string),
        ),
    };
    let hub_cfg = state
        .platform
        .zebflow_cfg
        .get_hub_distribution(&item_owner, &item.project)?;
    let open_app_path = resolve_project_hub_entry_path(
        &item_owner,
        &item.project,
        &hub_cfg.entry_url,
        hub_cfg.as_app,
    );
    Ok(json!({
        "owner": item_owner,
        "project": item.project,
        "title": item.title,
        "path": format!("/projects/{}/{}", item_owner, item.project),
        "edit_path": format!("/projects/{}/{}", item_owner, item.project),
        "open_app_path": open_app_path,
        "is_app": hub_cfg.as_app,
        "runtime_mode": runtime_mode,
        "runtime_summary": runtime_summary,
        "office_label": office_label,
        "office_url": office_url,
    }))
}

fn platform_office_rows(state: &PlatformAppState) -> Vec<Value> {
    let mut rows = vec![json!({
        "id": state.platform.cluster_bootstrap.node_id(),
        "label": state.platform.cluster_bootstrap.node_label(),
    })];
    let workers = state
        .platform
        .cluster_registry
        .snapshot()
        .map(|snapshot| snapshot.workers)
        .unwrap_or_default();
    for worker in workers {
        rows.push(json!({
            "id": worker.node_id,
            "label": worker.label,
        }));
    }
    rows
}

fn resolve_project_hub_entry_path(
    owner: &str,
    project: &str,
    entry_url: &str,
    as_app: bool,
) -> Option<String> {
    if !as_app {
        return None;
    }
    let entry = entry_url.trim();
    if entry.is_empty() {
        return None;
    }
    if entry.starts_with("http://") || entry.starts_with("https://") {
        return Some(entry.to_string());
    }
    let suffix = if entry.starts_with('/') {
        entry.to_string()
    } else {
        format!("/{entry}")
    };
    Some(format!("/wh/{owner}/{project}{suffix}"))
}

fn is_project_hub_producer_enabled(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
) -> Result<bool, PlatformError> {
    let cfg = state
        .platform
        .zebflow_cfg
        .get_hub_distribution(owner, project)?;
    Ok(cfg.producer_enabled && owner == state.platform.config.default_owner)
}

fn can_manage_project_hub_producer(state: &PlatformAppState, owner: &str) -> bool {
    owner == state.platform.config.default_owner
}

fn require_project_hub_producer(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
) -> Result<(), Response> {
    if is_project_hub_producer_enabled(state, owner, project).map_err(internal_error)? {
        Ok(())
    } else {
        Err((
            StatusCode::FORBIDDEN,
            Json(json!({
                "ok": false,
                "error": "Hub producer mode is disabled for this project",
                "code": "HUB_PRODUCER_DISABLED"
            })),
        )
            .into_response())
    }
}

async fn profile_page(State(state): State<PlatformAppState>, headers: HeaderMap) -> Response {
    let Some(owner) = session_owner(&state, &headers) else {
        return Redirect::to(LOGIN_PATH).into_response();
    };
    let user = match state.platform.users.get_user(&owner) {
        Ok(Some(user)) => user,
        Ok(None) => return Redirect::to(LOGIN_PATH).into_response(),
        Err(err) => return internal_error(err),
    };
    let resolved = state
        .platform
        .git_identity
        .resolve_for_actor(Some(&owner), &owner);
    match render_page(
        &state,
        "platform-profile",
        "/profile",
        json!({
            "seo": {
                "title": "Profile",
                "description": "User profile and git identity"
            },
            "owner": owner,
            "user": user,
            "effective_git_identity": resolved,
            "profile_api": "/api/profile",
            "app_version": APP_VERSION,
        }),
    ) {
        Ok(html) => Html(html).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn account_password_page(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
) -> Response {
    let Some(owner) = session_owner(&state, &headers) else {
        return Redirect::to(LOGIN_PATH).into_response();
    };
    let forced = state
        .platform
        .users
        .must_change_password(&owner)
        .unwrap_or(false);
    match render_page(
        &state,
        "platform-account-password",
        ACCOUNT_PASSWORD_PATH,
        json!({
            "seo": {
                "title": "Change Password",
                "description": "Choose a new password for this account"
            },
            "owner": owner,
            "forced": forced,
            "password_api": "/api/profile/password",
            "app_version": APP_VERSION,
        }),
    ) {
        Ok(html) => Html(html).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_change_password(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Json(req): Json<ChangePasswordRequest>,
) -> Response {
    let Some(owner) = session_owner(&state, &headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"ok": false, "error": "login required"})),
        )
            .into_response();
    };
    if let Err(err) =
        state
            .platform
            .users
            .change_password(&owner, &req.current_password, &req.new_password)
    {
        let status = match err.code {
            "PLATFORM_PASSWORD_INCORRECT" => StatusCode::FORBIDDEN,
            "PLATFORM_PASSWORD_INVALID" | "PLATFORM_PASSWORD_INSECURE" => StatusCode::BAD_REQUEST,
            "PLATFORM_USER_NOT_FOUND" => StatusCode::NOT_FOUND,
            _ => return internal_error(err),
        };
        return (
            status,
            Json(json!({"ok": false, "error": {"code": err.code, "message": err.message}})),
        )
            .into_response();
    }

    // The password is chosen now, so the bootstrap cleartext no longer opens
    // anything and its zero-ceremony window closes: delete it. Only the
    // first-boot owner ever had one.
    if owner == state.platform.config.default_owner {
        match crate::platform::services::bootstrap::remove_generated_password_file(
            &state.platform.config.data_root,
        ) {
            Ok(true) => eprintln!("Removed the generated bootstrap password file."),
            Ok(false) => {}
            Err(err) => eprintln!("warning: failed removing the bootstrap password file: {err}"),
        }
    }

    // Every other session of this user is now on a dead password; end them.
    // The session that just proved it knows the new password stays.
    let current_token = session_token(&headers);
    if let Ok(mut sessions) = state.sessions.lock() {
        sessions.retain(|token, session| {
            session.owner != owner || Some(token.as_str()) == current_token.as_deref()
        });
    }

    Json(json!({"ok": true})).into_response()
}

async fn design_system_page(State(state): State<PlatformAppState>, headers: HeaderMap) -> Response {
    let Some(owner) = session_owner(&state, &headers) else {
        return Redirect::to(LOGIN_PATH).into_response();
    };
    // The primitives that read a repository (RepoTree, FolderPicker) are shown
    // against the viewer's first project, so the gallery renders real rows
    // rather than an error state.
    let project = state
        .platform
        .projects
        .list_projects(&owner)
        .ok()
        .and_then(|items| items.into_iter().next())
        .map(|item| item.project);
    match render_page(
        &state,
        "platform-design-system",
        "/dev/design-system",
        json!({
            "owner": owner,
            "project": project,
            "seo": {
                "title": "Design System · Zebflow",
                "description": "Platform UI reference for platform developers and agents",
            },
        }),
    ) {
        Ok(html) => Html(html).into_response(),
        Err(err) => internal_error(err),
    }
}

/// The `zeb/ui` gallery is its own page: the studio kit and `zeb/ui` export
/// the same names (`Button`, `Card`, `Dialog`…), and the compiler's flat
/// bundle declares each exported name once per page. Two kits, two pages.
async fn design_system_ui_page(State(state): State<PlatformAppState>, headers: HeaderMap) -> Response {
    let Some(owner) = session_owner(&state, &headers) else {
        return Redirect::to(LOGIN_PATH).into_response();
    };
    // The primitives that read a repository (RepoTree, FolderPicker) are shown
    // against the viewer's first project, so the gallery renders real rows
    // rather than an error state.
    let project = state
        .platform
        .projects
        .list_projects(&owner)
        .ok()
        .and_then(|items| items.into_iter().next())
        .map(|item| item.project);
    match render_page(
        &state,
        "platform-design-system-ui",
        "/dev/design-system/ui",
        json!({
            "owner": owner,
            "project": project,
            "seo": {
                "title": "zeb/ui · Zebflow",
                "description": "The component library project pages import — every component, every variant",
            },
        }),
    ) {
        Ok(html) => Html(html).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn home_create_project_submit(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Form(req): Form<CreateProjectRequest>,
) -> Response {
    let Some(owner) = session_owner(&state, &headers) else {
        return Redirect::to(LOGIN_PATH).into_response();
    };

    match state
        .platform
        .projects
        .create_or_update_project(&owner, &req)
    {
        Ok((project, _layout)) => {
            // A person made this project, so it gets the starter README and
            // samples. An import, an install and a clone do not.
            if let Err(err) = state
                .platform
                .projects
                .write_starter_files(&owner, &project.project)
            {
                return internal_error(err);
            }
            match finalize_project_runtime_setup(&state, &owner, &project.project, &req.runtime)
                .await
            {
                Ok(_) => Redirect::to(HOME_PATH).into_response(),
                Err(err) => internal_error(err),
            }
        }
        Err(err) => internal_error(err),
    }
}

fn default_branch_main() -> String {
    "main".to_string()
}

/// Form payload for cloning a remote git repository into a new project.
#[derive(serde::Deserialize)]
struct CloneProjectFormRequest {
    project: String,
    #[serde(default)]
    title: String,
    provider: String,
    #[serde(default)]
    instance_url: String,
    repo_url: String,
    username: String,
    token: String,
    #[serde(default)]
    git_name: String,
    #[serde(default)]
    git_email: String,
    /// Which remote branch to clone from. Defaults to "main".
    #[serde(default = "default_branch_main")]
    remote_branch: String,
    /// Local branch name after clone. Empty means same as remote_branch.
    #[serde(default)]
    local_branch: String,
    /// Optional runtime mode for the cloned project.
    #[serde(default)]
    runtime_mode: Option<ProjectRuntimeMode>,
    /// Optional office id to pin the cloned project to after creation.
    #[serde(default)]
    placement_worker_id: Option<String>,
}

async fn home_clone_project_submit(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Form(req): Form<CloneProjectFormRequest>,
) -> Response {
    let Some(owner) = session_owner(&state, &headers) else {
        return Redirect::to(LOGIN_PATH).into_response();
    };

    let project = crate::platform::model::slug_segment(&req.project);
    if project.is_empty() {
        return (StatusCode::BAD_REQUEST, "Project slug is required").into_response();
    }

    // Build clone target path: data_root/users/{owner}/{project}/repo
    let repo_dir = state
        .platform
        .config
        .data_root
        .join("users")
        .join(crate::platform::model::slug_segment(&owner))
        .join(&project)
        .join("repo");

    // Build authenticated clone URL
    let auth_url = match build_git_clone_auth_url(
        &req.provider,
        &req.instance_url,
        &req.repo_url,
        &req.username,
        &req.token,
    ) {
        Ok(u) => u,
        Err(e) => return (StatusCode::BAD_REQUEST, e).into_response(),
    };

    // Ensure parent dir exists before cloning
    if let Some(parent) = repo_dir.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return internal_error(crate::platform::error::PlatformError::new(
                "CLONE_MKDIR",
                e.to_string(),
            ));
        }
    }

    // Run git clone (with explicit -b if a remote branch is specified)
    let remote_branch = {
        let b = req.remote_branch.trim().to_string();
        if b.is_empty() { "main".to_string() } else { b }
    };
    let mut clone_cmd = std::process::Command::new("git");
    clone_cmd.arg("clone");
    if !remote_branch.is_empty() {
        clone_cmd.arg("-b").arg(&remote_branch);
    }
    clone_cmd.arg("--").arg(&auth_url).arg(&repo_dir);
    let clone_out = clone_cmd.output();

    match clone_out {
        Err(e) => {
            return internal_error(crate::platform::error::PlatformError::new(
                "CLONE_SPAWN",
                e.to_string(),
            ));
        }
        Ok(out) if !out.status.success() => {
            let stderr =
                scrub_token_from_str(String::from_utf8_lossy(&out.stderr).as_ref(), &req.token);
            let msg = format!("git clone failed: {stderr}");
            return (StatusCode::BAD_REQUEST, msg).into_response();
        }
        Ok(_) => {}
    }

    // Compute effective local branch name; rename if it differs from the remote branch.
    let effective_local = {
        let lb = req.local_branch.trim().to_string();
        if lb.is_empty() {
            remote_branch.clone()
        } else {
            lb
        }
    };
    if effective_local != remote_branch {
        let _ = std::process::Command::new("git")
            .arg("-C")
            .arg(&repo_dir)
            .arg("branch")
            .arg("-m")
            .arg(&remote_branch)
            .arg(&effective_local)
            .output();
    }

    // Detect whether the cloned repo has any commits (empty remote repos have no HEAD)
    let has_commits = std::process::Command::new("git")
        .arg("-C")
        .arg(&repo_dir)
        .arg("rev-parse")
        .arg("HEAD")
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    // Create project record + layout dirs (git init is skipped since .git already exists)
    let title = if req.title.trim().is_empty() {
        project.clone()
    } else {
        req.title.clone()
    };
    let create_req = crate::platform::model::CreateProjectRequest {
        project: project.clone(),
        title: Some(title.clone()),
        local_branch: None, // branch already set by git clone + rename above
        runtime: crate::platform::model::ProjectRuntimeSelectionRequest {
            runtime_mode: req.runtime_mode,
            placement_worker_id: req.placement_worker_id.clone(),
        },
    };
    if let Err(e) = state
        .platform
        .projects
        .create_or_update_project(&owner, &create_req)
    {
        return internal_error(e);
    }

    // Save git remote config in zebflow.yaml.
    // Git author identity is resolved from the acting platform user profile.
    let cred_id = format!("{}-origin", req.provider);
    let resolved_git = state
        .platform
        .git_identity
        .resolve_for_actor(Some(&owner), &project);
    let git_name = if req.git_name.trim().is_empty() {
        resolved_git.name.clone()
    } else {
        req.git_name.trim().to_string()
    };
    let git_email = if req.git_email.trim().is_empty() {
        resolved_git.email.clone()
    } else {
        req.git_email.trim().to_string()
    };
    if let Err(err) = state.platform.zebflow_cfg.update(&owner, &project, |cfg| {
        cfg.configs.git.remote.credential_id = cred_id.clone();
        cfg.configs.git.remote.repo_url = req.repo_url.clone();
        cfg.configs.git.remote.branch = effective_local.clone();
    }) {
        return internal_error(err);
    }

    // Save credential record so git push can look it up later
    let cred_title = format!("{} origin", req.provider);
    let secret = serde_json::json!({
        "username": req.username,
        "token": req.token,
        "git_name": git_name.clone(),
        "git_email": git_email.clone(),
        "repo_url": req.repo_url,
        "instance_url": req.instance_url,
    });
    let upsert_req = UpsertProjectCredentialRequest {
        credential_id: cred_id.clone(),
        title: cred_title,
        kind: req.provider.clone(),
        secret,
        notes: format!("Auto-created on project clone from {}", req.repo_url),
    };
    let _ = state
        .platform
        .credentials
        .upsert_project_credential(&owner, &project, &upsert_req);

    // Empty repo: create initial commit + push so the remote gets a default branch
    if !has_commits {
        let project_slug = crate::platform::model::slug_segment(&project);
        let identity_fallback_name = if git_name.is_empty() {
            project_slug.clone()
        } else {
            git_name.clone()
        };
        let identity_fallback_email = if git_email.is_empty() {
            format!("{project_slug}@zebflow.local")
        } else {
            git_email.clone()
        };

        // Stage everything scaffolded by create_or_update_project
        let _ = std::process::Command::new("git")
            .arg("-C")
            .arg(&repo_dir)
            .arg("add")
            .arg("-A")
            .output();

        let commit_out = std::process::Command::new("git")
            .arg("-c")
            .arg(format!("user.name={identity_fallback_name}"))
            .arg("-c")
            .arg(format!("user.email={identity_fallback_email}"))
            .arg("-C")
            .arg(&repo_dir)
            .arg("commit")
            .arg("-m")
            .arg("Initial commit")
            .output();

        if commit_out.map(|o| o.status.success()).unwrap_or(false) {
            // Push to establish the default branch on the remote
            let push_out = std::process::Command::new("git")
                .arg("-C")
                .arg(&repo_dir)
                .arg("push")
                .arg("--set-upstream")
                .arg(&auth_url)
                .arg(format!("HEAD:{}", effective_local))
                .output();
            if let Ok(ref o) = push_out {
                if !o.status.success() {
                    let stderr =
                        scrub_token_from_str(&String::from_utf8_lossy(&o.stderr), &req.token);
                    return (
                        StatusCode::BAD_REQUEST,
                        format!("Initial push to empty repo failed: {stderr}"),
                    )
                        .into_response();
                }
            }
        }
    }

    if let Err(err) =
        finalize_project_runtime_setup(&state, &owner, &project, &create_req.runtime).await
    {
        return internal_error(err);
    }

    Redirect::to(HOME_PATH).into_response()
}

/// Builds an authenticated HTTPS git clone URL for the given provider.
/// Returns an error string on malformed input.
fn build_git_clone_auth_url(
    _provider: &str,
    instance_url: &str,
    repo_url: &str,
    username: &str,
    token: &str,
) -> Result<String, String> {
    let enc_user = percent_encode_userinfo(username);
    let enc_token = percent_encode_userinfo(token);

    // Determine base host from repo_url if possible, otherwise fall back to instance_url
    let base = if repo_url.starts_with("http://") || repo_url.starts_with("https://") {
        repo_url.to_string()
    } else if !instance_url.is_empty() {
        // Treat repo_url as a path relative to instance
        let base = instance_url.trim_end_matches('/');
        let path = repo_url.trim_start_matches('/');
        format!("{}/{}", base, path)
    } else {
        return Err(format!("Invalid repo URL: {repo_url}"));
    };

    // Insert credentials into the URL
    if let Some(after_scheme) = base.strip_prefix("https://") {
        Ok(format!(
            "https://{}:{}@{}",
            enc_user, enc_token, after_scheme
        ))
    } else if let Some(after_scheme) = base.strip_prefix("http://") {
        Ok(format!(
            "http://{}:{}@{}",
            enc_user, enc_token, after_scheme
        ))
    } else {
        Err(format!(
            "Repo URL must start with https:// or http://: {base}"
        ))
    }
}

/// Percent-encodes characters not allowed in userinfo (RFC 3986).
fn percent_encode_userinfo(s: &str) -> String {
    s.chars()
        .flat_map(|c| {
            if c.is_ascii_alphanumeric()
                || matches!(
                    c,
                    '-' | '.' | '_' | '~' | '!' | '$' | '\'' | '(' | ')' | '*' | '+' | ',' | ';'
                )
            {
                vec![c as u8]
            } else {
                format!("%{:02X}", c as u32).into_bytes()
            }
        })
        .map(|b| b as char)
        .collect()
}

/// Replaces occurrences of `token` in `s` with `***` (for safe error messages).
fn scrub_token_from_str(s: &str, token: &str) -> String {
    if token.is_empty() {
        return s.to_string();
    }
    s.replace(token, "***")
}

/// The header a controller presents when it calls one of *its* offices.
///
/// Derived per office from that office's own token digest, so it is scoped to
/// one office and to one direction (`offices.md` §8). There is no
/// process-wide value any more: the shared environment secret was the defect —
/// one secret for every office meant revoking one office meant rotating all of
/// them.
fn cluster_call_header_for_office(
    state: &PlatformAppState,
    office_id: &str,
) -> Result<String, PlatformError> {
    state
        .platform
        .cluster_join_tokens
        .controller_call_header_for(office_id)
}

/// The office a registry record belongs to, falling back to its node id.
fn worker_office_id(worker: &WorkerRegistryRecord) -> &str {
    if worker.office_id.trim().is_empty() {
        worker.node_id.as_str()
    } else {
        worker.office_id.as_str()
    }
}

/// Whether this request is **this office's controller** calling it.
///
/// One direction, named after the direction. It replaces a role-switching
/// `has_valid_cluster_token`, which on a controller returned true for *any*
/// active office's join token — and every project-capability check
/// short-circuited on it, so one office's token authorised acting as any owner
/// on the controller, including exporting and overwriting other users'
/// projects. `offices.md` §8 makes a token "revocable for one office alone" and
/// §2 gives the controller three verbs; a token that was a universal
/// project-owner credential was neither.
///
/// The office→controller half — registration, heartbeat, break-glass reporting
/// — is [`require_registering_office`], which checks the token *and* that it
/// names the office the request claims to be. The two halves are different
/// credentials for different directions and are no longer reachable through
/// one name.
///
/// On a controller or a standalone instance this is always `false`: nothing
/// calls *into* a controller with a controller-call header, and a controller
/// that accepted one would be accepting a value it mints itself.
fn is_controller_call(state: &PlatformAppState, headers: &HeaderMap) -> bool {
    let Some(presented) = headers
        .get(INTERNAL_CLUSTER_TOKEN_HEADER)
        .and_then(|value| value.to_str().ok())
    else {
        return false;
    };
    state
        .platform
        .cluster_join_tokens
        .controller_call_authenticates(presented)
}

/// Gate a route that only this office's controller may call.
fn require_controller_call(state: &PlatformAppState, headers: &HeaderMap) -> Result<(), Response> {
    if is_controller_call(state, headers) {
        return Ok(());
    }
    Err((
        StatusCode::UNAUTHORIZED,
        Json(json!({
            "ok": false,
            "error": {
                "code": "CLUSTER_TOKEN_INVALID",
                "message": "this route is reachable only by this office's controller, \
                            proving itself with the header it derives from this office's \
                            join token (`offices.md` §8)"
            }
        })),
    )
        .into_response())
}

/// Finish creating or reconfiguring a project on **this** office.
///
/// Placement onto another office is refused rather than faked. It used to be
/// implemented by the master-to-worker copier, which is retired
/// (`stability-matrix.md` row 12): a project is created on the office that will
/// run it, and moves between offices as a `ProjectBundle`. Remote placement
/// itself is recorded as unbuilt in `offices.md` §9, so the code says so.
async fn finalize_project_runtime_setup(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
    selection: &crate::platform::model::ProjectRuntimeSelectionRequest,
) -> Result<crate::infra::execution::placement::ProjectRuntimePlacement, PlatformError> {
    if selection
        .placement_worker_id
        .as_deref()
        .map(str::trim)
        .is_some_and(|value| !value.is_empty() && value != "local")
    {
        return Err(PlatformError::new(
            "CLUSTER_REMOTE_PLACEMENT_UNBUILT",
            "placing a project on another office is not built: a project is created on the \
             office that will run it, and moves between offices as a ProjectBundle export \
             and import (offices.md section 9, stability-matrix.md row 12)",
        ));
    }
    if let Some(mode) = selection.runtime_mode {
        state.platform.zebflow_cfg.update(owner, project, |cfg| {
            cfg.configs.runtime.mode = mode;
        })?;
    }
    state
        .platform
        .cluster_runtime_sync
        .refresh_local_repo_state(owner, project)?;
    let runtime_profile = state
        .platform
        .zebflow_cfg
        .get_runtime_profile(owner, project)?;
    let placement = state.platform.cluster_placement.assign_for_project(
        owner,
        project,
        &runtime_profile,
        selection,
    )?;
    Ok(placement)
}

fn cluster_runtime_summary(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
) -> Result<Value, PlatformError> {
    let placement = state.platform.cluster_placement.get(owner, project)?;
    let workers = state.platform.cluster_registry.snapshot()?.workers;
    Ok(json!({
        "placement": placement,
        "workers": workers,
        "summary": state.platform.cluster_placement.describe(placement.as_ref()),
    }))
}

async fn reqwest_response_to_axum(response: reqwest::Response) -> Response {
    let status = response.status();
    let headers = response.headers().clone();
    let body = response.bytes().await.unwrap_or_default();
    let mut axum_response = Response::new(Body::from(body));
    *axum_response.status_mut() =
        StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    for (name, value) in headers.iter() {
        if name.as_str().eq_ignore_ascii_case("transfer-encoding")
            || name.as_str().eq_ignore_ascii_case("content-length")
            || name.as_str().eq_ignore_ascii_case("connection")
        {
            continue;
        }
        axum_response
            .headers_mut()
            .insert(name.clone(), value.clone());
    }
    axum_response
}

async fn forward_runtime_execute_to_worker(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
    req: &ExecutePipelineRequest,
    worker_id: &str,
) -> Result<Response, PlatformError> {
    let worker = state
        .platform
        .cluster_registry
        .get_worker(worker_id)?
        .ok_or_else(|| {
            PlatformError::new(
                "CLUSTER_WORKER_UNKNOWN",
                format!("office '{}' not found", worker_id),
            )
        })?;
    let token = cluster_call_header_for_office(state, worker_office_id(&worker))?;
    let url = format!(
        "{}/api/internal/runtime/execute/{}/{}",
        worker.base_url.trim_end_matches('/'),
        owner,
        project
    );
    let response = state
        .http_client
        .post(url)
        .header(INTERNAL_CLUSTER_TOKEN_HEADER, token)
        .json(req)
        .send()
        .await
        .map_err(|err| PlatformError::new("CLUSTER_WORKER_PROXY", err.to_string()))?;
    Ok(reqwest_response_to_axum(response).await)
}

async fn forward_runtime_webhook_to_worker(
    state: &PlatformAppState,
    method: &Method,
    uri: &Uri,
    headers: &HeaderMap,
    body: &Bytes,
    worker_id: &str,
) -> Result<Response, PlatformError> {
    let worker = state
        .platform
        .cluster_registry
        .get_worker(worker_id)?
        .ok_or_else(|| {
            PlatformError::new(
                "CLUSTER_WORKER_UNKNOWN",
                format!("office '{}' not found", worker_id),
            )
        })?;
    let token = cluster_call_header_for_office(state, worker_office_id(&worker))?;
    let url = format!(
        "{}{}",
        worker.base_url.trim_end_matches('/'),
        webhook_url::worker_path_and_query(uri)?
    );
    let reqwest_method =
        reqwest::Method::from_bytes(method.as_str().as_bytes()).map_err(|err| {
            PlatformError::new(
                "CLUSTER_WORKER_PROXY",
                format!("unsupported HTTP method '{}': {err}", method.as_str()),
            )
        })?;
    let mut request = state
        .http_client
        .request(reqwest_method, url)
        .header(INTERNAL_CLUSTER_TOKEN_HEADER, token);
    for (name, value) in headers.iter() {
        if name.as_str().eq_ignore_ascii_case("host")
            || name.as_str().eq_ignore_ascii_case("content-length")
            || name
                .as_str()
                .eq_ignore_ascii_case(INTERNAL_CLUSTER_TOKEN_HEADER)
        {
            continue;
        }
        request = request.header(name, value);
    }
    let response = request
        .body(body.clone())
        .send()
        .await
        .map_err(|err| PlatformError::new("CLUSTER_WORKER_PROXY", err.to_string()))?;
    Ok(reqwest_response_to_axum(response).await)
}

fn parse_project_transfer_kind(raw: &str) -> Result<ProjectTransferArtifactKind, Response> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "bundle" => Ok(ProjectTransferArtifactKind::Bundle),
        "files" => Ok(ProjectTransferArtifactKind::Files),
        "full" => Ok(ProjectTransferArtifactKind::Full),
        _ => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "ok": false,
                "error": {
                    "code": "PROJECT_TRANSFER_KIND_INVALID",
                    "message": "kind must be 'bundle', 'files', or 'full'"
                }
            })),
        )
            .into_response()),
    }
}

fn remote_project_worker_id(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
) -> Result<Option<String>, PlatformError> {
    let placement = state.platform.cluster_placement.get(owner, project)?;
    let worker_id = match placement {
        Some(record) if record.target == ProjectRuntimePlacementTarget::Worker => record.worker_id,
        _ => None,
    };
    if worker_id.as_deref() == local_office_id(state).as_deref() {
        return Ok(None);
    }
    Ok(worker_id)
}

async fn forward_project_api_request_to_worker(
    state: &PlatformAppState,
    uri: &Uri,
    method: &Method,
    headers: &HeaderMap,
    body: Bytes,
    worker_id: &str,
) -> Result<Response, PlatformError> {
    let worker = state
        .platform
        .cluster_registry
        .get_worker(worker_id)?
        .ok_or_else(|| {
            PlatformError::new(
                "CLUSTER_WORKER_UNKNOWN",
                format!("office '{}' not found", worker_id),
            )
        })?;
    let token = cluster_call_header_for_office(state, worker_office_id(&worker))?;
    let mut url = format!("{}{}", worker.base_url.trim_end_matches('/'), uri.path());
    if let Some(query) = uri.query() {
        url.push('?');
        url.push_str(query);
    }
    let reqwest_method =
        reqwest::Method::from_bytes(method.as_str().as_bytes()).map_err(|err| {
            PlatformError::new(
                "CLUSTER_WORKER_PROXY",
                format!("unsupported HTTP method '{}': {err}", method.as_str()),
            )
        })?;
    let mut request = state
        .http_client
        .request(reqwest_method, url)
        .header(INTERNAL_CLUSTER_TOKEN_HEADER, token);
    for (name, value) in headers.iter() {
        if name.as_str().eq_ignore_ascii_case("host")
            || name.as_str().eq_ignore_ascii_case("content-length")
            || name.as_str().eq_ignore_ascii_case("cookie")
            || name
                .as_str()
                .eq_ignore_ascii_case(INTERNAL_CLUSTER_TOKEN_HEADER)
        {
            continue;
        }
        request = request.header(name, value);
    }
    let response = request
        .body(body)
        .send()
        .await
        .map_err(|err| PlatformError::new("CLUSTER_WORKER_PROXY", err.to_string()))?;
    Ok(reqwest_response_to_axum(response).await)
}

async fn forward_project_json_request_to_worker<T: serde::Serialize>(
    state: &PlatformAppState,
    uri: &Uri,
    headers: &HeaderMap,
    method: Method,
    payload: &T,
    worker_id: &str,
) -> Result<Response, PlatformError> {
    forward_project_api_request_to_worker(
        state,
        uri,
        &method,
        headers,
        Bytes::from(serde_json::to_vec(payload)?),
        worker_id,
    )
    .await
}

async fn maybe_forward_project_api_to_worker(
    state: &PlatformAppState,
    uri: &Uri,
    method: &Method,
    headers: &HeaderMap,
    body: Bytes,
    owner: &str,
    project: &str,
) -> Result<Option<Response>, PlatformError> {
    let Some(worker_id) = remote_project_worker_id(state, owner, project)? else {
        return Ok(None);
    };
    forward_project_api_request_to_worker(state, uri, method, headers, body, &worker_id)
        .await
        .map(Some)
}

async fn maybe_forward_project_json_to_worker<T: serde::Serialize>(
    state: &PlatformAppState,
    uri: &Uri,
    headers: &HeaderMap,
    method: Method,
    payload: &T,
    owner: &str,
    project: &str,
) -> Result<Option<Response>, PlatformError> {
    let Some(worker_id) = remote_project_worker_id(state, owner, project)? else {
        return Ok(None);
    };
    forward_project_json_request_to_worker(state, uri, headers, method, payload, &worker_id)
        .await
        .map(Some)
}

async fn maybe_forward_project_page_to_worker(
    state: &PlatformAppState,
    headers: &HeaderMap,
    uri: &Uri,
    owner: &str,
    project: &str,
) -> Result<Option<Response>, PlatformError> {
    let Some(worker_id) = remote_project_worker_id(state, owner, project)? else {
        return Ok(None);
    };
    forward_project_page_request_to_worker(state, uri, headers, owner, project, &worker_id)
        .await
        .map(Some)
}

fn worker_websocket_url(worker_base_url: &str, uri: &Uri) -> String {
    let base = worker_base_url.trim_end_matches('/');
    let base = if let Some(rest) = base.strip_prefix("https://") {
        format!("wss://{rest}")
    } else if let Some(rest) = base.strip_prefix("http://") {
        format!("ws://{rest}")
    } else {
        base.to_string()
    };
    let path_and_query = uri
        .path_and_query()
        .map(|value| value.as_str())
        .unwrap_or("/");
    format!("{base}{path_and_query}")
}

fn forwarded_websocket_headers(headers: &HeaderMap) -> Vec<(String, String)> {
    fn skip_header(name: &HeaderName) -> bool {
        matches!(
            name.as_str().to_ascii_lowercase().as_str(),
            "host"
                | "connection"
                | "upgrade"
                | "sec-websocket-key"
                | "sec-websocket-version"
                | "sec-websocket-extensions"
                | "sec-websocket-protocol"
                | "content-length"
        )
    }

    headers
        .iter()
        .filter(|(name, _)| !skip_header(name))
        .filter_map(|(name, value)| {
            value
                .to_str()
                .ok()
                .map(|value| (name.as_str().to_string(), value.to_string()))
        })
        .collect()
}

async fn proxy_websocket_to_worker(
    mut client_socket: WebSocket,
    worker_url: String,
    headers: Vec<(String, String)>,
) {
    use futures::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::Message as WorkerMessage;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;

    let mut request = match worker_url.into_client_request() {
        Ok(request) => request,
        Err(_) => {
            let _ = client_socket.send(Message::Close(None)).await;
            return;
        }
    };
    for (name, value) in headers {
        let Ok(name) =
            tokio_tungstenite::tungstenite::http::header::HeaderName::from_bytes(name.as_bytes())
        else {
            continue;
        };
        let Ok(value) = tokio_tungstenite::tungstenite::http::header::HeaderValue::from_str(&value)
        else {
            continue;
        };
        request.headers_mut().insert(name, value);
    }

    let (worker_socket, _) = match tokio_tungstenite::connect_async(request).await {
        Ok(pair) => pair,
        Err(_) => {
            let _ = client_socket.send(Message::Close(None)).await;
            return;
        }
    };

    let (mut client_tx, mut client_rx) = client_socket.split();
    let (mut worker_tx, mut worker_rx) = worker_socket.split();

    let client_to_worker = async {
        while let Some(message) = client_rx.next().await {
            let Ok(message) = message else {
                break;
            };
            let Some(message) = axum_ws_to_worker_ws(message) else {
                continue;
            };
            let is_close = matches!(message, WorkerMessage::Close(_));
            if worker_tx.send(message).await.is_err() || is_close {
                break;
            }
        }
    };

    let worker_to_client = async {
        while let Some(message) = worker_rx.next().await {
            let Ok(message) = message else {
                break;
            };
            let Some(message) = worker_ws_to_axum_ws(message) else {
                continue;
            };
            let is_close = matches!(message, Message::Close(_));
            if client_tx.send(message).await.is_err() || is_close {
                break;
            }
        }
    };

    tokio::select! {
        _ = client_to_worker => {}
        _ = worker_to_client => {}
    }
}

fn axum_ws_to_worker_ws(message: Message) -> Option<tokio_tungstenite::tungstenite::Message> {
    use tokio_tungstenite::tungstenite::Message as WorkerMessage;
    match message {
        Message::Text(text) => Some(WorkerMessage::Text(text.to_string())),
        Message::Binary(bytes) => Some(WorkerMessage::Binary(bytes.to_vec())),
        Message::Ping(bytes) => Some(WorkerMessage::Ping(bytes.to_vec())),
        Message::Pong(bytes) => Some(WorkerMessage::Pong(bytes.to_vec())),
        Message::Close(_) => Some(WorkerMessage::Close(None)),
    }
}

fn worker_ws_to_axum_ws(message: tokio_tungstenite::tungstenite::Message) -> Option<Message> {
    use tokio_tungstenite::tungstenite::Message as WorkerMessage;
    match message {
        WorkerMessage::Text(text) => Some(Message::Text(text.into())),
        WorkerMessage::Binary(bytes) => Some(Message::Binary(bytes.into())),
        WorkerMessage::Ping(bytes) => Some(Message::Ping(bytes.into())),
        WorkerMessage::Pong(bytes) => Some(Message::Pong(bytes.into())),
        WorkerMessage::Close(_) => Some(Message::Close(None)),
        WorkerMessage::Frame(_) => None,
    }
}

async fn forward_project_page_request_to_worker(
    state: &PlatformAppState,
    uri: &Uri,
    headers: &HeaderMap,
    owner: &str,
    project: &str,
    worker_id: &str,
) -> Result<Response, PlatformError> {
    let worker = state
        .platform
        .cluster_registry
        .get_worker(worker_id)?
        .ok_or_else(|| {
            PlatformError::new(
                "CLUSTER_WORKER_UNKNOWN",
                format!("office '{}' not found", worker_id),
            )
        })?;
    let token = cluster_call_header_for_office(state, worker_office_id(&worker))?;
    let mut url = format!("{}{}", worker.base_url.trim_end_matches('/'), uri.path());
    if let Some(query) = uri.query() {
        url.push('?');
        url.push_str(query);
    }

    let mut request = state
        .http_client
        .get(url)
        .header(INTERNAL_CLUSTER_TOKEN_HEADER, token);
    for (name, value) in headers.iter() {
        if name.as_str().eq_ignore_ascii_case("host")
            || name.as_str().eq_ignore_ascii_case("content-length")
            || name.as_str().eq_ignore_ascii_case("cookie")
            || name
                .as_str()
                .eq_ignore_ascii_case(INTERNAL_CLUSTER_TOKEN_HEADER)
        {
            continue;
        }
        request = request.header(name, value);
    }

    let response = request
        .send()
        .await
        .map_err(|err| PlatformError::new("CLUSTER_WORKER_PROXY", err.to_string()))?;
    let status = response.status();
    let headers = response.headers().clone();
    let is_html = headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|value| value.to_ascii_lowercase().starts_with("text/html"));
    let body = response.bytes().await.unwrap_or_default();
    let body = if is_html {
        let html = String::from_utf8_lossy(&body);
        Bytes::from(html.replace(
            "/assets/rwe/scripts/",
            &format!("/static/{owner}/{project}/_rwe/scripts/"),
        ))
    } else {
        body
    };

    let mut axum_response = Response::new(Body::from(body));
    *axum_response.status_mut() =
        StatusCode::from_u16(status.as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    for (name, value) in headers.iter() {
        if name.as_str().eq_ignore_ascii_case("transfer-encoding")
            || name.as_str().eq_ignore_ascii_case("content-length")
            || name.as_str().eq_ignore_ascii_case("connection")
        {
            continue;
        }
        axum_response
            .headers_mut()
            .insert(name.clone(), value.clone());
    }
    Ok(axum_response)
}

/// Send this office's unreported break-glass records to its controller.
///
/// Best effort by construction. The local record is the authority and is
/// already durable; this only adds an acknowledgement timestamp to it. A
/// controller that is down, slow, or older than this office costs one silent
/// retry fifteen seconds later and never a lost row.
async fn report_break_glass_to_controller(
    state: &PlatformAppState,
    master_url: &str,
    token: &str,
    node_id: &str,
) {
    let pending = match state.platform.local_authority.unreported_break_glass() {
        Ok(pending) if !pending.is_empty() => pending,
        Ok(_) => return,
        Err(err) => {
            eprintln!(
                "Zebflow office: cannot read the local-authority log: {}",
                err.message
            );
            return;
        }
    };
    let url = format!(
        "{}/api/internal/cluster/offices/break-glass",
        master_url.trim_end_matches('/')
    );
    let body = json!({ "office_id": node_id, "events": pending });
    let response = state
        .http_client
        .post(&url)
        .header(INTERNAL_CLUSTER_TOKEN_HEADER, token)
        .json(&body)
        .send()
        .await;
    let accepted = match response {
        Ok(response) if response.status().is_success() => response
            .json::<Value>()
            .await
            .ok()
            .and_then(|value| {
                value
                    .get("accepted")
                    .and_then(Value::as_array)
                    .map(|items| {
                        items
                            .iter()
                            .filter_map(Value::as_str)
                            .map(str::to_string)
                            .collect::<Vec<_>>()
                    })
            })
            .unwrap_or_default(),
        Ok(response) => {
            eprintln!(
                "Zebflow office: controller refused a break-glass report with {}",
                response.status()
            );
            return;
        }
        Err(err) => {
            eprintln!("Zebflow office: break-glass report failed: {err}");
            return;
        }
    };
    let now = now_ts();
    for event_id in accepted {
        if let Err(err) = state.platform.local_authority.mark_reported(&event_id, now) {
            eprintln!(
                "Zebflow office: break-glass {event_id} was reported but not marked: {}",
                err.message
            );
        }
    }
}

async fn cluster_worker_registration_loop(state: PlatformAppState) {
    let bootstrap = state.platform.cluster_bootstrap.clone();
    let identity = state
        .platform
        .cluster_join_tokens
        .office_identity()
        .cloned();
    let (Some(master_url), Some(identity), Some(base_url)) = (
        bootstrap.master_url().map(str::to_string),
        identity,
        bootstrap.advertise_url().map(str::to_string),
    ) else {
        // Unreachable through `zebflow office`, which refuses to start in this
        // state. Still guarded, and reporting every missing name at once, because
        // an embedder can build this router with settings no CLI check saw.
        eprintln!(
            "Zebflow office: registration loop disabled; missing {}",
            bootstrap.settings().missing_required_env(false).join(", ")
        );
        return;
    };
    let token = identity.token.clone();
    // The id this process claims to be, which is the token's office unless the
    // operator named a different one. The controller refuses the mismatch
    // rather than resolving it: a token minted for one office presented by
    // another is exactly what the office id inside the token exists to catch.
    let node_id = bootstrap.node_id();
    let label = bootstrap.node_label();
    let register_url = format!(
        "{}/api/internal/cluster/workers/register",
        master_url.trim_end_matches('/')
    );
    let heartbeat_url = format!(
        "{}/api/internal/cluster/workers/heartbeat",
        master_url.trim_end_matches('/')
    );
    let capabilities = crate::infra::execution::runner::RunnerCapabilities {
        tags: vec![format!("app_version:{APP_VERSION}")],
        supports_resident: true,
        ..Default::default()
    };
    loop {
        let nonce = crate::platform::services::cluster::join_token::nonce();
        let register_request = ClusterWorkerRegisterRequest {
            node_id: node_id.clone(),
            label: label.clone(),
            base_url: base_url.clone(),
            capabilities: capabilities.clone(),
            nonce: nonce.clone(),
        };
        let registered = state
            .http_client
            .post(&register_url)
            .header(INTERNAL_CLUSTER_TOKEN_HEADER, &token)
            .json(&register_request)
            .send()
            .await;
        let response = match registered {
            Ok(response) => response,
            Err(err) => {
                eprintln!("Zebflow office register failed: {err}");
                tokio::time::sleep(Duration::from_secs(10)).await;
                continue;
            }
        };
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        if !status.is_success() {
            // A revoked or wrong token lands here. The office keeps serving
            // its own projects — `offices.md` §4 keeps execution local — but
            // it never proceeds to heartbeat as a member it is not.
            eprintln!(
                "Zebflow office register refused with {status}: {}",
                body.trim()
            );
            tokio::time::sleep(Duration::from_secs(10)).await;
            continue;
        }
        let proof = serde_json::from_str::<Value>(&body)
            .ok()
            .and_then(|value| {
                value
                    .get("proof")
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })
            .unwrap_or_default();
        if !identity.verify_registration_proof(&nonce, &proof) {
            // The other half of §8: the office verifies the controller. A host
            // that answers this base URL without holding this office's secret
            // cannot produce the proof, and is not treated as the controller.
            eprintln!(
                "Zebflow office: refusing '{master_url}' — its registration response did not \
                 prove it holds this office's join token. Not registering."
            );
            tokio::time::sleep(Duration::from_secs(10)).await;
            continue;
        }
        // `offices.md` §6: "Its use is recorded locally and reported to the
        // controller on reconnect." This is the reconnect, and it is here
        // rather than before the proof check on purpose — a break-glass record
        // says who let themselves into this office, and it is not handed to a
        // host that has not yet proved it is this office's controller.
        //
        // It is a re-send of everything still unacknowledged rather than a
        // queue drained once, so an office offline for a month reports on its
        // first successful registration and an office that never comes back
        // keeps the record locally and says so. Nothing is deleted after
        // reporting; only a timestamp is added.
        report_break_glass_to_controller(&state, &master_url, &token, &node_id).await;

        let heartbeat_request = ClusterWorkerHeartbeatRequest {
            node_id: node_id.clone(),
            status: "online".to_string(),
            base_url: base_url.clone(),
            capabilities: capabilities.clone(),
        };
        match state
            .http_client
            .post(&heartbeat_url)
            .header(INTERNAL_CLUSTER_TOKEN_HEADER, &token)
            .json(&heartbeat_request)
            .send()
            .await
        {
            Ok(response) if !response.status().is_success() => {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                // Revocation mid-session arrives here. Nothing already running
                // is killed (§4 keeps execution); the office simply stops being
                // a member and the controller stops hearing from it.
                eprintln!(
                    "Zebflow office heartbeat refused with {status}: {}",
                    body.trim()
                );
            }
            Ok(_) => {}
            Err(err) => eprintln!("Zebflow office heartbeat failed: {err}"),
        }
        tokio::time::sleep(Duration::from_secs(15)).await;
    }
}

async fn project_root_page(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    uri: Uri,
    Path((owner, project)): Path<(String, String)>,
    Query(query): Query<PipelineRegistryQuery>,
) -> Response {
    render_project_pipelines_with_tab(
        state,
        headers,
        uri,
        owner,
        project,
        "registry",
        query.path.as_deref(),
        None,
        query.editor_type.as_deref(),
        query.file.as_deref(),
        query.line,
    )
    .await
}

async fn project_pipelines_page(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    uri: Uri,
    Path((owner, project, tab)): Path<(String, String, String)>,
    Query(query): Query<PipelineRegistryQuery>,
) -> Response {
    render_project_pipelines_with_tab(
        state,
        headers,
        uri,
        owner,
        project,
        &tab,
        query.path.as_deref(),
        query.id.as_deref(),
        query.editor_type.as_deref(),
        query.file.as_deref(),
        query.line,
    )
    .await
}

async fn render_project_pipelines_with_tab(
    state: PlatformAppState,
    headers: HeaderMap,
    uri: Uri,
    owner: String,
    project: String,
    tab: &str,
    registry_path: Option<&str>,
    editor_id: Option<&str>,
    registry_type: Option<&str>,
    registry_file: Option<&str>,
    registry_line: Option<u32>,
) -> Response {
    if let Err(response) = require_project_page_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesRead,
    ) {
        return response;
    }
    match maybe_forward_project_page_to_worker(&state, &headers, &uri, &owner, &project).await {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }

    let repo_layout = match state.platform.file.ensure_project_layout(&owner, &project) {
        Ok(layout) => layout.repo_layout,
        Err(err) => return internal_error(err),
    };

    let is_registry = tab == "registry";
    let is_editor = tab == "editor";

    // Registry tab now delegates to the unified editor with nav_sub="registry"
    if is_registry {
        let query = UnifiedEditorQuery {
            editor_type: registry_type.map(str::to_string),
            path: registry_path.map(str::to_string),
            file: registry_file.map(str::to_string),
            line: registry_line,
        };
        return render_project_editor(state, headers, uri, owner, project, query, "registry").await;
    }

    let tab_payload = if is_registry {
        Some((
            "registry",
            "Pipeline Registry",
            "Browse pipelines by project path.",
            Vec::new(),
        ))
    } else if is_editor {
        Some((
            "editor",
            "Pipeline Editor",
            "Create and edit pipeline graph + node configuration.",
            Vec::new(),
        ))
    } else {
        pipeline_tab_payload(tab)
    };
    let Some((tab_key, tab_title, tab_desc, items)) = tab_payload else {
        return (
            StatusCode::NOT_FOUND,
            Html("pipeline tab not found".to_string()),
        )
            .into_response();
    };

    match state.platform.projects.get_project(&owner, &project) {
        Ok(Some(info)) => {
            let nav = nav_classes(&owner, &project, "pipelines", Some(tab_key));
            let route = format!("/projects/{owner}/{project}/pipelines/{tab_key}");
            let editor_base = format!("/projects/{owner}/{project}/pipelines/registry");
            let route_base = format!("/projects/{owner}/{project}/pipelines/registry");
            let pipeline_items = if is_registry || is_editor {
                items
            } else {
                let trigger_filter = match tab_key {
                    "webhooks" => Some("webhook"),
                    "schedules" => Some("schedule"),
                    "manual" => Some("manual"),
                    "functions" => Some("function"),
                    _ => None,
                };
                match state
                    .platform
                    .projects
                    .list_pipeline_meta_rows(&owner, &project)
                {
                    Ok(rows) => rows
                        .into_iter()
                        .filter(|meta| {
                            trigger_filter
                                .map(|wanted| meta.trigger_kind.eq_ignore_ascii_case(wanted))
                                .unwrap_or(true)
                        })
                        .map(|meta| {
                            let file_rel_path = meta.file_rel_path.clone();
                            let virtual_path = crate::platform::model::normalize_virtual_path(
                                &meta.virtual_path,
                            );
                            let (webhook_path, webhook_method) = if tab_key == "webhooks" {
                                match state.platform.projects.read_pipeline_source(
                                    &owner,
                                    &project,
                                    &file_rel_path,
                                ) {
                                    Ok(source) => crate::platform::services::project::first_webhook_trigger_from_source(&source)
                                        .map_or((None, None), |(path, method)| {
                                            (Some(path), Some(method))
                                        }),
                                    Err(_) => (None, None),
                                }
                            } else {
                                (None, None)
                            };
                            json!({
                                "name": meta.name,
                                "title": meta.title,
                                "description": meta.description,
                                "trigger_kind": meta.trigger_kind,
                                "virtual_path": virtual_path,
                                "file_rel_path": file_rel_path,
                                "editor_href": format!("{editor_base}?type=pipeline&path={virtual_path}&file={file_rel_path}"),
                                "webhook_path": webhook_path.unwrap_or_else(|| "/".to_string()),
                                "webhook_method": webhook_method.unwrap_or_else(|| "GET".to_string()),
                            })
                        })
                        .collect::<Vec<_>>(),
                    Err(err) => return internal_error(err),
                }
            };

            let registry_git_map: std::collections::HashMap<String, String> = state
                .platform
                .projects
                .list_repo_git_status(&owner, &project)
                .unwrap_or_default()
                .into_iter()
                .map(|item| (item.rel_path, item.code))
                .collect();

            let registry = if is_registry {
                let current_registry_path = registry_path.unwrap_or("/");
                match state.platform.projects.list_pipeline_registry(
                    &owner,
                    &project,
                    current_registry_path,
                    &route_base,
                    &editor_base,
                ) {
                    Ok(mut listing) => {
                        for item in &mut listing.pipelines {
                            // Git keys are repository-relative; identity is not.
                            item.git_status = registry_git_map
                                .get(&repo_layout.source_rel(&item.file_rel_path))
                                .cloned();
                        }
                        for item in &mut listing.files {
                            item.git_status = registry_git_map.get(&item.rel_path).cloned();
                        }
                        let current_path = listing.current_path;
                        let breadcrumbs = listing.breadcrumbs;
                        let folders = listing.folders;
                        let template_files: Vec<Value> = listing
                            .files
                            .into_iter()
                            .map(|f| {
                                let template_path =
                                    template_display_path(&repo_layout, &f.rel_path);
                                let git_status = registry_git_map.get(&f.rel_path).cloned();
                                json!({
                                    "name": f.name,
                                    "rel_path": template_path,
                                    "kind": f.kind,
                                    "edit_href": f.edit_href,
                                    "git_status": git_status,
                                })
                            })
                            .collect();
                        let pipelines = listing
                            .pipelines
                            .into_iter()
                            .map(|item| {
                                let file_id = item.file_rel_path.clone();
                                json!({
                                    "name": item.name,
                                    "title": item.title,
                                    "description": item.description,
                                    "trigger_kind": item.trigger_kind,
                                    "file_rel_path": item.file_rel_path,
                                    "is_active": item.is_active,
                                    "has_draft": item.has_draft,
                                    "git_status": item.git_status,
                                    "edit_href": format!("/projects/{owner}/{project}/editor?type=pipeline&path={current_path}&file={file_id}")
                                })
                            })
                            .collect::<Vec<_>>();
                        let has_folders = !folders.is_empty();
                        let has_pipelines = !pipelines.is_empty();
                        let has_files = !template_files.is_empty();
                        let docs_items = state
                            .platform
                            .projects
                            .list_repo_tree(&owner, &project, &RepoTreeScope::all())
                            .map(|listing| listing.items)
                            .unwrap_or_default()
                            .into_iter()
                            .filter(|item| item.file_kind == "doc")
                            .map(|item| {
                                let encoded = item.rel_path.replace(' ', "%20");
                                let href = format!(
                                    "/projects/{owner}/{project}/editor?type=file&file={}",
                                    encoded
                                );
                                json!({
                                    "name": item.name,
                                    "path": item.rel_path,
                                    "href": href
                                })
                            })
                            .collect::<Vec<_>>();
                        json!({
                            "current_path": current_path,
                            "editor_href": format!("{editor_base}?path={current_path}"),
                            "breadcrumbs": breadcrumbs,
                            "folders": folders,
                            "pipelines": pipelines,
                            "files": template_files,
                            "docs": docs_items,
                            "has_folders": has_folders,
                            "has_pipelines": has_pipelines,
                            "has_files": has_files,
                            "api": {
                                "delete": format!("/api/projects/{owner}/{project}/pipelines/definition"),
                                "delete_template": format!("/api/projects/{owner}/{project}/templates/file"),
                                "git_status": format!("/api/projects/{owner}/{project}/templates/git-status"),
                                "git_commit": format!("/api/projects/{owner}/{project}/git/commit"),
                            }
                        })
                    }
                    Err(err) => return internal_error(err),
                }
            } else {
                json!({
                    "current_path": "/",
                    "breadcrumbs": [],
                    "folders": [],
                    "pipelines": [],
                    "files": []
                })
            };

            let editor_payload = if is_editor {
                let all_rows = match state
                    .platform
                    .projects
                    .list_pipeline_meta_rows(&owner, &project)
                {
                    Ok(rows) => rows,
                    Err(err) => return internal_error(err),
                };

                let wanted_id = editor_id
                    .map(str::trim)
                    .filter(|raw| !raw.is_empty())
                    .map(str::to_string)
                    .or_else(|| all_rows.first().map(|meta| meta.file_rel_path.clone()));

                let selected_any = wanted_id
                    .as_deref()
                    .and_then(|id| all_rows.iter().find(|row| row.file_rel_path == id))
                    .cloned()
                    .or_else(|| all_rows.first().cloned());

                let scope_path = registry_path
                    .map(crate::platform::model::normalize_virtual_path)
                    .unwrap_or_else(|| {
                        selected_any
                            .as_ref()
                            .map(|meta| {
                                crate::platform::model::normalize_virtual_path(&meta.virtual_path)
                            })
                            .unwrap_or_else(|| "/".to_string())
                    });

                let rows = all_rows
                    .iter()
                    .filter(|meta| {
                        crate::platform::model::normalize_virtual_path(&meta.virtual_path)
                            == scope_path
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                let project_config =
                    match state.platform.zebflow_cfg.read_or_default(&owner, &project) {
                        Ok(config) => config,
                        Err(err) => return internal_error(err),
                    };
                let pipeline_logging_defaults = pipeline_logging_defaults(&project_config);

                let selected = wanted_id
                    .as_deref()
                    .and_then(|id| rows.iter().find(|row| row.file_rel_path == id))
                    .cloned()
                    .or_else(|| rows.first().cloned());
                let selected_id_effective =
                    selected.as_ref().map(|meta| meta.file_rel_path.clone());

                let mut lock_map = std::collections::HashMap::new();
                for meta in &rows {
                    let locked = match state.platform.projects.read_pipeline_source(
                        &owner,
                        &project,
                        &meta.file_rel_path,
                    ) {
                        Ok(source) => pipeline_source_is_locked(&source),
                        Err(_) => false,
                    };
                    lock_map.insert(meta.file_rel_path.clone(), locked);
                }

                let (source, graph_json, parse_error, hit_stats, selected_locked) =
                    if let Some(meta) = &selected {
                        let source = match state.platform.projects.read_pipeline_source(
                            &owner,
                            &project,
                            &meta.file_rel_path,
                        ) {
                            Ok(source) => source,
                            Err(err) => return internal_error(err),
                        };
                        let (graph_json, parse_error) = match serde_json::from_str::<Value>(&source)
                        {
                            Ok(value) => (value, Value::Null),
                            Err(err) => (
                                Value::Null,
                                Value::String(format!("pipeline JSON parse error: {err}")),
                            ),
                        };
                        let stats =
                            state
                                .platform
                                .pipeline_hits
                                .get(&owner, &project, &meta.file_rel_path);
                        let locked = lock_map
                            .get(&meta.file_rel_path)
                            .copied()
                            .unwrap_or_else(|| pipeline_source_is_locked(&source));
                        (
                            Value::String(source),
                            graph_json,
                            parse_error,
                            json!(stats),
                            Value::Bool(locked),
                        )
                    } else {
                        (
                            Value::Null,
                            Value::Null,
                            Value::Null,
                            Value::Null,
                            Value::Bool(false),
                        )
                    };

                let node_catalog = state
                    .platform
                    .node_registry
                    .merged_definitions(&owner, &project)
                    .into_iter()
                    .map(|def| {
                        json!({
                            "kind": def.kind,
                            "title": def.title,
                            "description": def.description,
                            "input_pins": def.input_pins,
                            "output_pins": def.output_pins,
                            "input_schema": def.input_schema,
                            "output_schema": def.output_schema
                        })
                    })
                    .collect::<Vec<_>>();

                let mut folder_counts = std::collections::BTreeMap::<String, usize>::new();
                for meta in &all_rows {
                    let vpath = crate::platform::model::normalize_virtual_path(&meta.virtual_path);
                    *folder_counts.entry(vpath).or_insert(0) += 1;
                }
                // Also count template files so folder badges reflect all items, not just pipelines.
                // Skip .zf.json files — they are pipeline definitions already counted above.
                if let Ok(workspace) = state.platform.projects.list_repo_tree(&owner, &project, &RepoTreeScope::all()) {
                    for item in &workspace.items {
                        if item.kind == "file" && !item.rel_path.ends_with(".zf.json") {
                            let parent = std::path::Path::new(&item.rel_path)
                                .parent()
                                .and_then(|p| p.to_str())
                                .unwrap_or("");
                            let vpath = crate::platform::model::normalize_virtual_path(parent);
                            *folder_counts.entry(vpath).or_insert(0) += 1;
                        }
                    }
                }
                let scope_folders = folder_counts
                    .into_iter()
                    .map(|(vpath, count)| {
                        json!({
                            "virtual_path": vpath,
                            "count": count,
                            "href": format!("{editor_base}?path={vpath}")
                        })
                    })
                    .collect::<Vec<_>>();

                let mut scope_hierarchy = vec![json!({
                    "name": "root",
                    "virtual_path": "/",
                    "href": format!("{editor_base}?path=/")
                })];
                if scope_path != "/" {
                    let mut accum = String::new();
                    for seg in scope_path.trim_start_matches('/').split('/') {
                        if seg.trim().is_empty() {
                            continue;
                        }
                        accum.push('/');
                        accum.push_str(seg);
                        scope_hierarchy.push(json!({
                            "name": seg,
                            "virtual_path": accum,
                            "href": format!("{editor_base}?path={accum}")
                        }));
                    }
                }

                let pipelines = rows
                    .iter()
                    .map(|meta| {
                        let file_id = meta.file_rel_path.clone();
                        let is_active = meta
                            .active_hash
                            .as_deref()
                            .map(|hash| hash == meta.hash)
                            .unwrap_or(false);
                        let has_draft = meta
                            .active_hash
                            .as_deref()
                            .map(|hash| hash != meta.hash)
                            .unwrap_or(false);
                        let locked = lock_map.get(&file_id).copied().unwrap_or(false);
                        json!({
                            "id": file_id,
                            "name": meta.name,
                            "title": meta.title,
                            "description": meta.description,
                            "trigger_kind": meta.trigger_kind,
                            "virtual_path": meta.virtual_path,
                            "file_rel_path": meta.file_rel_path,
                            "is_active": is_active,
                            "has_draft": has_draft,
                            "is_locked": locked,
                            "status_label": if is_active { "active" } else if has_draft { "draft" } else { "inactive" },
                            "editor_href": format!("{editor_base}?type=pipeline&path={scope_path}&file={file_id}")
                        })
                    })
                    .collect::<Vec<_>>();

                // Template/script files at the current scope folder
                let editor_template_files: Vec<Value> =
                    match state.platform.projects.list_pipeline_registry(
                        &owner,
                        &project,
                        &scope_path,
                        &route_base,
                        &editor_base,
                    ) {
                        Ok(listing) => listing
                            .files
                            .into_iter()
                            .map(|f| {
                                let template_path =
                                    template_display_path(&repo_layout, &f.rel_path);
                                let git_status = registry_git_map.get(&f.rel_path).cloned();
                                json!({
                                    "name": f.name,
                                    "rel_path": template_path,
                                    "kind": f.kind,
                                    "template_path": template_path,
                                    "git_status": git_status,
                                })
                            })
                            .collect(),
                        Err(_) => Vec::new(),
                    };

                json!({
                    "scope_path": scope_path,
                    "scope_hierarchy": scope_hierarchy,
                    "scope_folders": scope_folders,
                    "selected_id": selected_id_effective,
                    "selected_locked": selected_locked,
                    "selected_meta": selected,
                    "selected_source": source,
                    "selected_graph": graph_json,
                    "parse_error": parse_error,
                    "hits": hit_stats,
                    "logging_defaults": pipeline_logging_defaults,
                    "pipelines": pipelines,
                    "template_files": editor_template_files,
                    "nodes": node_catalog,
                    "api": {
                        "registry": format!("/api/projects/{owner}/{project}/pipelines/registry"),
                        "list": format!("/api/projects/{owner}/{project}/pipelines"),
                        "by_id": format!("/api/projects/{owner}/{project}/pipelines/by-id"),
                        "definition": format!("/api/projects/{owner}/{project}/pipelines/definition"),
                        "activate": format!("/api/projects/{owner}/{project}/pipelines/activate"),
                        "deactivate": format!("/api/projects/{owner}/{project}/pipelines/deactivate"),
                        "execute": format!("/api/projects/{owner}/{project}/pipelines/execute"),
                        "hits": format!("/api/projects/{owner}/{project}/pipelines/hits"),
                        "invocations": format!("/api/projects/{owner}/{project}/pipelines/invocations"),
                        "nodes": format!("/api/projects/{owner}/{project}/nodes"),
                        "credentials": format!("/api/projects/{owner}/{project}/credentials"),
                        // The repository tree. `/templates/workspace` was removed and this kept
                // pointing at it, so both readers 404'd and silently showed nothing.
                "templates_workspace": format!("/api/projects/{owner}/{project}/repo"),
                        "template_file": format!("/api/projects/{owner}/{project}/templates/file"),
                        "template_save": format!("/api/projects/{owner}/{project}/templates/file"),
                        "template_outline": format!("/api/projects/{owner}/{project}/templates/outline"),
                    },
                    "graphui": {
                        "runtime_src": "/assets/libraries/zeb/graphui/0.1/runtime/graphui.bundle.mjs",
                        "package_label": "zeb/graphui@0.1"
                    }
                })
            } else {
                Value::Null
            };
            let input = json!({
                "seo": {
                    "title": format!("{} - Pipelines", info.title),
                    "description": "Pipeline management"
                },
                "owner": info.owner,
                "project": info.project,
                "title": info.title,
                "project_href": format!("/projects/{owner}/{project}"),
                "current_menu": format!("Pipelines / {tab_title}"),
                "page_title": tab_title,
                "page_subtitle": tab_desc,
                "pipeline_items": pipeline_items,
                "is_registry": is_registry,
                "is_editor": is_editor,
                "is_non_registry": !is_registry,
                "is_webhooks": tab_key == "webhooks",
                "registry": registry,
                "editor": editor_payload,
                "nav": nav,
            });

            match render_page(&state, "platform-project-pipelines", &route, input) {
                Ok(html) => Html(html).into_response(),
                Err(err) => internal_error(err),
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, Html("project not found".to_string())).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn project_editor_page(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    uri: Uri,
    Path((owner, project)): Path<(String, String)>,
    Query(query): Query<UnifiedEditorQuery>,
) -> Response {
    render_project_editor(state, headers, uri, owner, project, query, "editor").await
}

/// Repository-relative path as the template editor names it.
///
/// The editor addresses templates relative to the source root, so a listing
/// entry that carries a repository-relative path is shown with that root
/// removed.
fn template_display_path(layout: &ResolvedProjectLayout, rel_path: &str) -> String {
    layout
        .strip_source(rel_path)
        .unwrap_or(rel_path)
        .to_string()
}

/// The children of one scope in the repository tree.
///
/// The sidebar used to merge two listings — the pipeline registry, scoped
/// source-relative, and a `docs` overlay bolted onto the same namespace, which
/// meant a real folder named `docs` inside the source root collided with the
/// fake one. One tree, one scope, and `docs` is a directory like any other.
fn repo_scope_children<'a>(
    items: &'a [crate::platform::model::TemplateTreeItem],
    scope: &str,
) -> (
    Vec<&'a crate::platform::model::TemplateTreeItem>,
    Vec<&'a crate::platform::model::TemplateTreeItem>,
) {
    let prefix = scope.trim_matches('/');
    let mut folders = Vec::new();
    let mut files = Vec::new();
    for item in items {
        let parent = item
            .rel_path
            .rsplit_once('/')
            .map(|(dir, _)| dir)
            .unwrap_or("");
        if parent != prefix {
            continue;
        }
        if item.kind == "folder" {
            folders.push(item);
        } else {
            files.push(item);
        }
    }
    (folders, files)
}

/// The scope a repository path sits in, as a leading-slash repository path.
fn repo_scope_of(rel_path: &str) -> String {
    match rel_path.trim_matches('/').rsplit_once('/') {
        Some((dir, _)) if !dir.is_empty() => format!("/{dir}"),
        _ => "/".to_string(),
    }
}

async fn render_project_editor(
    state: PlatformAppState,
    headers: HeaderMap,
    uri: Uri,
    owner: String,
    project: String,
    query: UnifiedEditorQuery,
    nav_sub: &str,
) -> Response {
    if let Err(response) = require_project_page_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesRead,
    ) {
        return response;
    }
    match maybe_forward_project_page_to_worker(&state, &headers, &uri, &owner, &project).await {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }

    let project_info = match state.platform.projects.get_project(&owner, &project) {
        Ok(Some(info)) => info,
        Ok(None) => {
            return (StatusCode::NOT_FOUND, Html("project not found".to_string())).into_response();
        }
        Err(err) => return internal_error(err),
    };

    let repo_layout = match state.platform.file.ensure_project_layout(&owner, &project) {
        Ok(layout) => layout.repo_layout,
        Err(err) => return internal_error(err),
    };

    let editor_base = if nav_sub == "registry" {
        format!("/projects/{owner}/{project}/pipelines/registry")
    } else {
        format!("/projects/{owner}/{project}/editor")
    };
    let route_base = format!("/projects/{owner}/{project}/pipelines/registry");
    let route = editor_base.clone();

    let editor_type = query.editor_type.as_deref().unwrap_or("").to_string();
    let file_param = query
        .file
        .as_deref()
        .filter(|s| !s.trim().is_empty())
        .map(str::to_string);

    // Determine scope path: from ?path param, or derived from file, or root
    let scope_path = if let Some(path) = query.path.as_deref().filter(|s| !s.trim().is_empty()) {
        crate::platform::model::normalize_virtual_path(path)
    } else if let Some(ref file) = file_param {
        if editor_type == "doc" {
            repo_scope_of(file)
        } else {
            repo_scope_of(file)
        }
    } else {
        "/".to_string()
    };

    // One tree over `repo/`. Every folder in the sidebar is a real directory,
    // so a `.md` beside `zebflow.yaml` is as reachable as one inside `docs/` --
    // which is what the extension allowlist has always permitted.
    let repo_tree = match state.platform.projects.list_repo_tree(&owner, &project, &RepoTreeScope::all()) {
        Ok(listing) => listing.items,
        Err(err) => return internal_error(err),
    };
    let (scope_folders_here, scope_files_here) = repo_scope_children(&repo_tree, &scope_path);

    // Git status map for git indicators
    let git_map: std::collections::HashMap<String, String> = state
        .platform
        .projects
        .list_repo_git_status(&owner, &project)
        .unwrap_or_default()
        .into_iter()
        .map(|item| (item.rel_path, item.code))
        .collect();

    // The registry still answers for pipelines, which carry identity, activation
    // state and a trigger kind that a directory walk cannot know. Its folder and
    // file duties are the repository tree's now.
    let listing = match state.platform.projects.list_pipeline_registry(
        &owner,
        &project,
        &crate::platform::model::normalize_virtual_path(
            repo_layout
                .strip_source(scope_path.trim_start_matches('/'))
                .unwrap_or(""),
        ),
        &route_base,
        &editor_base,
    ) {
        Ok(l) => l,
        Err(err) => return internal_error(err),
    };

    // All pipeline rows (for scope folder map)
    let all_rows = match state
        .platform
        .projects
        .list_pipeline_meta_rows(&owner, &project)
    {
        Ok(rows) => rows,
        Err(err) => return internal_error(err),
    };

    // Scope hierarchy breadcrumbs
    let mut scope_hierarchy = vec![json!({
        "name": "root",
        "href": format!("{editor_base}?path=/")
    })];
    if scope_path != "/" {
        let mut accum = String::new();
        for seg in scope_path.trim_start_matches('/').split('/') {
            if seg.trim().is_empty() {
                continue;
            }
            accum.push('/');
            accum.push_str(seg);
            scope_hierarchy.push(json!({
                "name": seg,
                "href": format!("{editor_base}?path={accum}")
            }));
        }
    }

    // Every folder in the repository, with how many files it holds, for the
    // sidebar accordion. Counted off the one tree so the accordion and the
    // folder view cannot disagree — the road this replaces counted pipelines
    // by their source-relative virtual path and templates by a second walk.
    let scope_folders = repo_tree
        .iter()
        .filter(|item| item.kind == "folder")
        .map(|folder| {
            let prefix = format!("{}/", folder.rel_path);
            let count = repo_tree
                .iter()
                .filter(|child| child.kind == "file" && child.rel_path.starts_with(&prefix))
                .count();
            json!({
                "virtual_path": format!("/{}", folder.rel_path),
                "count": count,
                "href": format!("{editor_base}?path=/{}", folder.rel_path),
            })
        })
        .collect::<Vec<_>>();

    // Locked templates list (for both sidebar indicators and lock toggle)
    let locked_templates = state
        .platform
        .zebflow_cfg
        .get_locked_templates(&owner, &project)
        .unwrap_or_default();

    // Pipeline lock map for sidebar indicators
    let pipeline_lock_map: std::collections::HashMap<String, bool> = {
        let mut m = std::collections::HashMap::new();
        for item in &listing.pipelines {
            let locked = match state.platform.projects.read_pipeline_source(
                &owner,
                &project,
                &item.file_rel_path,
            ) {
                Ok(source) => pipeline_source_is_locked(&source),
                Err(_) => false,
            };
            m.insert(item.file_rel_path.clone(), locked);
        }
        m
    };

    // Sidebar pipelines — pipelines at current scope with new editor URLs
    let sidebar_pipelines = listing
        .pipelines
        .iter()
        .map(|item| {
            let file_id = item.file_rel_path.clone();
            let is_selected = file_param.as_deref() == Some(file_id.as_str());
            let is_locked = pipeline_lock_map.get(&file_id).copied().unwrap_or(false);
            json!({
                "id": file_id,
                "name": item.name,
                "title": item.title,
                "trigger_kind": item.trigger_kind,
                "virtual_path": scope_path,
                "is_active": item.is_active,
                "has_draft": item.has_draft,
                "is_selected": is_selected,
                "is_locked": is_locked,
                "status_label": if item.is_active { "active" } else if item.has_draft { "draft" } else { "inactive" },
                "editor_href": format!("{editor_base}?type=pipeline&path={scope_path}&file={file_id}"),
                "git_status": item.git_status,
            })
        })
        .collect::<Vec<_>>();

    // Sidebar files — every non-pipeline file at this scope, one editor type.
    // `type=doc` and `type=template` were the same operation reached by two
    // roads; the path here is repository-relative, so nothing has to be
    // un-prefixed on the way back.
    let sidebar_template_files: Vec<Value> = scope_files_here
        .iter()
        .filter(|item| {
            !item
                .rel_path
                .ends_with(crate::platform::model::PIPELINE_DEFINITION_EXTENSION)
        })
        .map(|item| {
            let git_status = git_map.get(&item.rel_path).cloned();
            let is_selected = file_param.as_deref() == Some(item.rel_path.as_str());
            json!({
                "name": item.name,
                "rel_path": item.rel_path,
                "template_path": item.rel_path,
                "kind": item.file_kind,
                "git_status": git_status,
                "is_selected": is_selected,
                "editor_href": format!(
                    "{editor_base}?type=file&path={scope_path}&file={}",
                    item.rel_path
                ),
            })
        })
        .collect();

    // Child folders — real directories at this scope. Nothing is injected here
    // any more; `docs/` shows up because it exists on disk.
    let child_folders: Vec<Value> = scope_folders_here
        .iter()
        .map(|item| {
            let virtual_path = format!("/{}", item.rel_path);
            json!({
                "name": item.name,
                "virtual_path": virtual_path,
                "href": format!("{editor_base}?path={virtual_path}"),
                "count": repo_tree
                    .iter()
                    .filter(|child| {
                        child.kind == "file"
                            && child
                                .rel_path
                                .starts_with(&format!("{}/", item.rel_path))
                    })
                    .count(),
                "is_protected": item.is_protected,
            })
        })
        .collect();

    let sidebar = json!({
        "scope_path": scope_path,
        "scope_hierarchy": scope_hierarchy,
        "scope_folders": scope_folders,
        "child_folders": child_folders,
        "pipelines": sidebar_pipelines,
        "template_files": sidebar_template_files,
    });

    // Determine effective editor type
    let effective_type = match (editor_type.as_str(), file_param.as_deref()) {
        ("pipeline", Some(_)) => "pipeline",
        // `template` and `doc` both land here: one editor, one repository path.
        // Older links carrying either word keep working.
        ("file" | "template" | "doc", Some(_)) => "file",
        _ => "folder",
    };

    // Pipeline payload
    let pipeline_payload = if effective_type == "pipeline" {
        let file = file_param.as_deref().unwrap_or("");
        let meta = all_rows.iter().find(|r| r.file_rel_path == file).cloned();

        let (source, graph_json, parse_error, hit_stats, selected_locked) =
            if let Some(ref meta) = meta {
                let source = match state.platform.projects.read_pipeline_source(
                    &owner,
                    &project,
                    &meta.file_rel_path,
                ) {
                    Ok(s) => s,
                    Err(err) => return internal_error(err),
                };
                let locked = pipeline_source_is_locked(&source);
                let (graph_json, parse_error) = match serde_json::from_str::<Value>(&source) {
                    Ok(v) => (v, Value::Null),
                    Err(err) => (
                        Value::Null,
                        Value::String(format!("pipeline JSON parse error: {err}")),
                    ),
                };
                let stats = state
                    .platform
                    .pipeline_hits
                    .get(&owner, &project, &meta.file_rel_path);
                (
                    Value::String(source),
                    graph_json,
                    parse_error,
                    json!(stats),
                    Value::Bool(locked),
                )
            } else {
                (
                    Value::Null,
                    Value::Null,
                    Value::Null,
                    Value::Null,
                    Value::Bool(false),
                )
            };

        let node_catalog = state
            .platform
            .node_registry
            .merged_definitions(&owner, &project)
            .into_iter()
            .map(|def| {
                json!({
                    "kind": def.kind,
                    "title": def.title,
                    "description": def.description,
                    "input_pins": def.input_pins,
                    "output_pins": def.output_pins,
                    "input_schema": def.input_schema,
                    "output_schema": def.output_schema
                })
            })
            .collect::<Vec<_>>();

        // The unified editor needs the same effective logging defaults as the
        // legacy registry payload, including independently inherited capture fields.
        let project_config = match state.platform.zebflow_cfg.read_or_default(&owner, &project) {
            Ok(config) => config,
            Err(err) => return internal_error(err),
        };
        json!({
            "logging_defaults": pipeline_logging_defaults(&project_config),
            "selected_id": file,
            "selected_meta": meta,
            "selected_source": source,
            "selected_graph": graph_json,
            "parse_error": parse_error,
            "hits": hit_stats,
            "selected_locked": selected_locked,
            "nodes": node_catalog,
            "api": {
                "by_id": format!("/api/projects/{owner}/{project}/pipelines/by-id"),
                "definition": format!("/api/projects/{owner}/{project}/pipelines/definition"),
                "activate": format!("/api/projects/{owner}/{project}/pipelines/activate"),
                "deactivate": format!("/api/projects/{owner}/{project}/pipelines/deactivate"),
                "execute": format!("/api/projects/{owner}/{project}/pipelines/execute"),
                "hits": format!("/api/projects/{owner}/{project}/pipelines/hits"),
                "invocations": format!("/api/projects/{owner}/{project}/pipelines/invocations"),
                "nodes": format!("/api/projects/{owner}/{project}/nodes"),
                "credentials": format!("/api/projects/{owner}/{project}/credentials"),
                // The repository tree. `/templates/workspace` was removed and this kept
                // pointing at it, so both readers 404'd and silently showed nothing.
                "templates_workspace": format!("/api/projects/{owner}/{project}/repo"),
                "template_file": format!("/api/projects/{owner}/{project}/templates/file"),
                "template_save": format!("/api/projects/{owner}/{project}/templates/file"),
                "template_outline": format!("/api/projects/{owner}/{project}/templates/outline"),
            },
            "graphui": {
                "runtime_src": "/assets/libraries/zeb/graphui/0.1/runtime/graphui.bundle.mjs",
                "package_label": "zeb/graphui@0.1",
            }
        })
    } else {
        Value::Null
    };

    // Template payload
    // One file payload. `template` and `doc` were the same editor reached by two
    // roads with two APIs; the path is repository-relative and the API is the
    // one repository door.
    let template_payload = if effective_type == "file" {
        let file = file_param.as_deref().unwrap_or("");
        let file_data = match state
            .platform
            .projects
            .read_repo_file(&owner, &project, file)
        {
            Ok(d) => d,
            Err(_) => crate::platform::model::TemplateFilePayload {
                rel_path: file.to_string(),
                name: file.rsplit('/').next().unwrap_or(file).to_string(),
                file_kind: "doc".to_string(),
                content: String::new(),
                line_count: 0,
                is_protected: false,
            },
        };
        json!({
            "name": file_data.name,
            "rel_path": file_data.rel_path,
            "file_kind": file_data.file_kind,
            "content": file_data.content,
            "line_count": file_data.line_count,
            "is_protected": file_data.is_protected,
            "api": {
                "file": format!("/api/projects/{owner}/{project}/repo/file"),
                "save": format!("/api/projects/{owner}/{project}/repo/file"),
                "outline": format!("/api/projects/{owner}/{project}/templates/outline"),
            }
        })
    } else {
        Value::Null
    };

    let doc_payload = Value::Null;

    // Folder view payload — reuse sidebar data
    let folder_payload = if effective_type == "folder" {
        json!({
            "child_folders": child_folders,
            "pipelines": sidebar_pipelines,
            "template_files": sidebar_template_files,
        })
    } else {
        Value::Null
    };

    let selected_template_locked = if effective_type == "file" {
        let file = file_param.as_deref().unwrap_or("");
        crate::platform::services::project_config::is_template_path_locked(&locked_templates, file)
    } else {
        false
    };

    let (seo_title, current_menu) = if nav_sub == "registry" {
        (
            format!("{} - Pipelines", project_info.title),
            "Pipelines / Registry",
        )
    } else {
        (
            format!("{} - Editor", project_info.title),
            "Pipelines / Editor",
        )
    };

    let input = json!({
        "seo": {
            "title": seo_title,
            "description": "Unified pipeline and template editor"
        },
        "owner": project_info.owner,
        "project": project_info.project,
        "title": project_info.title,
        "project_href": format!("/projects/{owner}/{project}"),
        "current_menu": current_menu,
        "editor_base": editor_base,
        "editor_type": effective_type,
        "selected_file": file_param,
        "selected_line": query.line,
        "sidebar": sidebar,
        "pipeline": pipeline_payload,
        "template": template_payload,
        "doc": doc_payload,
        "folder": folder_payload,
        "locked_templates": locked_templates,
        "selected_template_locked": selected_template_locked,
        "assets": {
            "api": format!("/api/projects/{owner}/{project}/assets"),
        },
        "nav": nav_classes(&owner, &project, "pipelines", Some(nav_sub)),
    });

    match render_page(&state, "platform-project-editor", &route, input) {
        Ok(html) => Html(html).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn project_dashboard_page(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    uri: Uri,
    Path((owner, project)): Path<(String, String)>,
) -> Response {
    if let Err(response) = require_project_page_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::ProjectRead,
    ) {
        return response;
    }
    match maybe_forward_project_page_to_worker(&state, &headers, &uri, &owner, &project).await {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    match state.platform.projects.get_project(&owner, &project) {
        Ok(Some(info)) => {
            let nav = nav_classes(&owner, &project, "dashboard", None);
            let route = format!("/projects/{owner}/{project}/dashboard");
            let input = json!({
                "seo": {
                    "title": format!("{} - Dashboard", info.title),
                    "description": "Runtime health, resource usage, and installed capabilities."
                },
                "owner": info.owner,
                "project": info.project,
                "title": info.title,
                "project_href": format!("/projects/{owner}/{project}"),
                "nav": nav,
                "api": {
                    "system_info": "/api/system/info"
                }
            });
            match render_page(&state, "platform-project-dashboard", &route, input) {
                Ok(html) => Html(html).into_response(),
                Err(err) => internal_error(err),
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, Html("project not found".to_string())).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn project_hub_page(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    uri: Uri,
    Path((owner, project)): Path<(String, String)>,
) -> Response {
    project_hub_tab_page(
        State(state),
        headers,
        uri,
        Path((owner, project, "packs".to_string())),
    )
    .await
}

async fn project_hub_tab_page(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    uri: Uri,
    Path((owner, project, raw_tab)): Path<(String, String, String)>,
) -> Response {
    if let Err(response) = require_project_page_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesRead,
    ) {
        return response;
    }
    match maybe_forward_project_page_to_worker(&state, &headers, &uri, &owner, &project).await {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    match state.platform.projects.get_project(&owner, &project) {
        Ok(Some(info)) => {
            let tab = normalize_hub_tab(&raw_tab);
            let route = if tab == "packs" {
                format!("/projects/{owner}/{project}/hub")
            } else {
                format!("/projects/{owner}/{project}/hub/{tab}")
            };
            let nav = nav_classes(&owner, &project, "hub", None);
            if let Err(err) = state
                .platform
                .hub
                .ensure_default_project_repository(&owner, &project)
            {
                return internal_error(err);
            }
            let assets = match hub_asset_rows(&state, &owner, &project, false) {
                Ok(items) => items,
                Err(err) => return internal_error(err),
            };
            let my_assets = match hub_asset_rows(&state, &owner, &project, true) {
                Ok(items) => items,
                Err(err) => return internal_error(err),
            };
            let publish_sources =
                match hub_publish_sources(&state, &owner, &project, "pipeline_with_dependencies") {
                    Ok(items) => items,
                    Err(err) => return internal_error(err),
                };
            let sekejap_schema =
                match sekejap::export_schema(&state.platform.config.data_root, &owner, &project) {
                    Ok(schema) => json!({
                        "available": !schema.tables.is_empty(),
                        "table_count": schema.tables.len(),
                    }),
                    Err(_) => json!({
                        "available": false,
                        "table_count": 0,
                    }),
                };
            let sqlite_schema = match crate::platform::sqlite_schema::has_schema(
                &state.platform.config.data_root,
                &owner,
                &project,
            ) {
                Ok(available) => json!({ "available": available }),
                Err(_) => json!({ "available": false }),
            };
            let initial_data = match state
                .platform
                .hub
                .list_project_initial_data(&owner, &project)
            {
                Ok(items) => json!({
                    "available": !items.is_empty(),
                    "items": items,
                }),
                Err(_) => json!({
                    "available": false,
                    "items": [],
                }),
            };
            let rwe_libraries = state
                .platform
                .zebflow_cfg
                .get_rwe_libraries(&owner, &project)
                .unwrap_or_default()
                .into_iter()
                .map(|(name, entry)| {
                    json!({
                        "name": name,
                        "version": entry.version,
                        "source": entry.source,
                        "enabled": true,
                    })
                })
                .collect::<Vec<_>>();
            let installed = match installed_artifacts_payload(&state, &owner, &project) {
                Ok(payload) => payload,
                Err(err) => return internal_error(err),
            };

            let input = json!({
                "seo": {
                    "title": format!("{} - Hub", info.title),
                    "description": "Embedded hub for pipelines and reusable project materials."
                },
                "owner": info.owner,
                "project": info.project,
                "title": info.title,
                "project_href": format!("/projects/{owner}/{project}"),
                "nav": nav,
                "hub_tabs": hub_tab_items(&owner, &project, tab),
                "installed": installed,
                "hub_producer": {
                    "enabled": true,
                },
                "tab_flags": {
                    "packs": tab == "packs",
                    "my_packs": tab == "my-packs",
                    "publish": tab == "publish",
                    "nodes": tab == "nodes",
                    "dependencies": tab == "dependencies",
                },
                "assets": assets,
                "my_assets": my_assets,
                "publish_sources": publish_sources,
                "publish_options": {
                    "sekejap_schema": sekejap_schema,
                    "sqlite_schema": sqlite_schema,
                    "initial_data": initial_data,
                    "libraries": rwe_libraries,
                },
                "hub_api": {
                    "assets": format!("/api/projects/{owner}/{project}/hub/assets"),
                    "my_assets": format!("/api/projects/{owner}/{project}/hub/assets/mine"),
                    "publish_sources": format!("/api/projects/{owner}/{project}/hub/publish-sources"),
                    "publish_preview": format!("/api/projects/{owner}/{project}/hub/publish-preview"),
                    "publish_asset": format!("/api/projects/{owner}/{project}/hub/remote/assets/publish"),
                    "publish_review": format!("/api/projects/{owner}/{project}/hub/assets/publish-review"),
                    "upload": format!("/api/projects/{owner}/{project}/files/upload"),
                    "access": format!("/api/projects/{owner}/{project}/hub/access"),
                    "repositories": format!("/api/projects/{owner}/{project}/hub/repositories"),
                    "node_bundle_review": format!("/api/projects/{owner}/{project}/nodes/install/review"),
                    "node_bundle_install": format!("/api/projects/{owner}/{project}/nodes/install"),
                    // Repairing a package whose bytes no longer match the lock.
                    "dependencies": format!("/api/projects/{owner}/{project}/dependencies"),
                    // Undoing an install: the lock entry and the bytes.
                    "libraries_remove": format!("/api/projects/{owner}/{project}/rwe/libraries/remove"),
                    // Which libraries this project actually has, re-read after a write.
                    "libraries": format!("/api/projects/{owner}/{project}/rwe/libraries"),
                }
            });
            match render_page(&state, "platform-project-hub", &route, input) {
                Ok(html) => Html(html).into_response(),
                Err(err) => internal_error(err),
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, Html("project not found".to_string())).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn project_credentials_page(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    uri: Uri,
    Path((owner, project)): Path<(String, String)>,
) -> Response {
    if let Err(response) = require_project_page_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::CredentialsRead,
    ) {
        return response;
    }
    match maybe_forward_project_page_to_worker(&state, &headers, &uri, &owner, &project).await {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }

    match state.platform.projects.get_project(&owner, &project) {
        Ok(Some(info)) => {
            let nav = nav_classes(&owner, &project, "credentials", None);
            let route = format!("/projects/{owner}/{project}/credentials");
            let input = json!({
                "seo": {
                    "title": format!("{} - Credentials", info.title),
                    "description": "Credential catalog and secret payload management"
                },
                "owner": info.owner,
                "project": info.project,
                "title": info.title,
                "project_href": format!("/projects/{owner}/{project}"),
                "credentials": {
                    "api": {
                        "list": format!("/api/projects/{owner}/{project}/credentials"),
                        "item_base": format!("/api/projects/{owner}/{project}/credentials"),
                    }
                },
                "nav": nav,
            });
            match render_page(&state, "platform-project-credentials", &route, input) {
                Ok(html) => Html(html).into_response(),
                Err(err) => internal_error(err),
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, Html("project not found".to_string())).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn project_db_connections_page(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    uri: Uri,
    Path((owner, project)): Path<(String, String)>,
) -> Response {
    if let Err(response) = require_project_page_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesRead,
    ) {
        return response;
    }
    match maybe_forward_project_page_to_worker(&state, &headers, &uri, &owner, &project).await {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }

    match state.platform.projects.get_project(&owner, &project) {
        Ok(Some(info)) => {
            let nav = nav_classes(&owner, &project, "databases", Some("connections"));
            let route = format!("/projects/{owner}/{project}/db/connections");
            let connections = match state
                .platform
                .db_connections
                .list_project_connections(&owner, &project)
            {
                Ok(items) => items,
                Err(err) => return internal_error(err),
            };
            let external_connection_cards = connections
                .iter()
                .map(|item| {
                    json!({
                        "connection_id": item.connection_id,
                        "slug": item.connection_slug,
                        "name": item.connection_label,
                        "kind": item.database_kind,
                        "icon_class": db_connection_icon_class(&item.database_kind),
                        "credential_id": item.credential_id,
                        "updated_at": item.updated_at,
                        "description": "Credential-backed external database connection.",
                        "path": format!(
                            "/projects/{owner}/{project}/db/{}/{}/tables",
                            item.database_kind,
                            item.connection_slug
                        )
                    })
                })
                .collect::<Vec<_>>();
            let mut connection_cards = vec![json!({
                "connection_id": "builtin:mapserver:default-mapserver",
                "slug": "default-mapserver",
                "name": "Default Mapserver",
                "kind": "mapserver",
                "icon_class": "i-devicon-mapserver",
                "credential_id": "",
                "updated_at": "",
                "description": "Published GeoJSON-backed spatial layers for map serving.",
                "path": format!("/projects/{owner}/{project}/db/mapserver/default-mapserver/layers")
            })];
            connection_cards.extend(external_connection_cards);
            let input = json!({
                "seo": {
                    "title": format!("{} - Databases", info.title),
                    "description": "Project database connections"
                },
                "owner": info.owner,
                "project": info.project,
                "title": info.title,
                "project_href": format!("/projects/{owner}/{project}"),
                "connections": connection_cards,
                "db_connections": {
                    "api": {
                        "list": format!("/api/projects/{owner}/{project}/db/connections"),
                        "item_base": format!("/api/projects/{owner}/{project}/db/connections"),
                        "test": format!("/api/projects/{owner}/{project}/db/connections/test"),
                        "credentials_list": format!("/api/projects/{owner}/{project}/credentials"),
                    }
                },
                "nav": nav,
            });
            match render_page(&state, "platform-project-tables", &route, input) {
                Ok(html) => Html(html).into_response(),
                Err(err) => internal_error(err),
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, Html("project not found".to_string())).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn project_db_suite_redirect_page(
    Path((owner, project, db_kind, connection)): Path<(String, String, String, String)>,
) -> Response {
    Redirect::to(&format!(
        "/projects/{owner}/{project}/db/{db_kind}/{connection}/tables"
    ))
    .into_response()
}

async fn project_db_suite_page(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    uri: Uri,
    Path((owner, project, db_kind, connection, tab)): Path<(
        String,
        String,
        String,
        String,
        String,
    )>,
    Query(query): Query<DbSuiteQuery>,
) -> Response {
    if let Err(response) = require_project_page_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesRead,
    ) {
        return response;
    }
    match maybe_forward_project_page_to_worker(&state, &headers, &uri, &owner, &project).await {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }

    let tab_key = match tab.as_str() {
        "tables" | "query" | "graph" | "schema" | "mart" | "maintenance" => tab,
        "layers" | "publish" | "test" => tab,
        _ => {
            return (StatusCode::NOT_FOUND, Html("db tab not found".to_string())).into_response();
        }
    };

    match state.platform.projects.get_project(&owner, &project) {
        Ok(Some(info)) => {
            if db_kind == "mapserver" {
                if connection != "default-mapserver" {
                    return (
                        StatusCode::NOT_FOUND,
                        Html("mapserver instance not found".to_string()),
                    )
                        .into_response();
                }
                if tab_key != "layers" && tab_key != "publish" && tab_key != "test" {
                    return (
                        StatusCode::NOT_FOUND,
                        Html("mapserver tab not found".to_string()),
                    )
                        .into_response();
                }
                let nav = nav_classes(&owner, &project, "databases", Some("connections"));
                let route =
                    format!("/projects/{owner}/{project}/db/{db_kind}/{connection}/{tab_key}");
                let base = format!("/projects/{owner}/{project}/db/{db_kind}/{connection}");
                let suite_tabs = vec![
                    json!({
                        "label": "Layers",
                        "href": format!("{base}/layers"),
                        "classes": if tab_key == "layers" { "is-active" } else { "" },
                    }),
                    json!({
                        "label": "Publish",
                        "href": format!("{base}/publish"),
                        "classes": if tab_key == "publish" { "is-active" } else { "" },
                    }),
                    json!({
                        "label": "Test",
                        "href": format!("{base}/test"),
                        "classes": if tab_key == "test" { "is-active" } else { "" },
                    }),
                ];
                let layers = match read_mapserver_layers(&state, &owner, &project, &connection) {
                    Ok(items) => items,
                    Err(err) => return internal_error(err),
                };
                let sources = match list_mapserver_source_files(&state, &owner, &project) {
                    Ok(items) => items,
                    Err(err) => return internal_error(err),
                };
                let input = json!({
                    "seo": {
                        "title": format!("{} - Mapserver {}", info.title, connection),
                        "description": "Published GeoJSON-backed spatial layers"
                    },
                    "owner": info.owner,
                    "project": info.project,
                    "title": info.title,
                    "project_href": format!("/projects/{owner}/{project}"),
                    "connection": {
                        "id": format!("builtin:mapserver:{connection}"),
                        "name": "Default Mapserver",
                        "kind": db_kind,
                        "slug": connection,
                        "icon_class": "i-devicon-mapserver",
                        "credential_id": "",
                    },
                    "suite_tabs": suite_tabs,
                    "tab_flags": {
                        "layers": tab_key == "layers",
                        "publish": tab_key == "publish",
                        "test": tab_key == "test",
                    },
                    "layers": layers,
                    "sources": sources,
                    "mapserver_api": {
                        "sources": format!("/api/projects/{owner}/{project}/mapserver/{connection}/sources"),
                        "layers": format!("/api/projects/{owner}/{project}/mapserver/{connection}/layers"),
                        "upload": format!("/api/projects/{owner}/{project}/files/upload?path=mapserver"),
                        "list_files": format!("/api/projects/{owner}/{project}/files/list?path=mapserver"),
                        "base_public": format!("/ms/{owner}/{project}"),
                    },
                    "nav": nav,
                });
                match render_page(
                    &state,
                    "platform-project-table-connection-mapserver",
                    &route,
                    input,
                ) {
                    Ok(html) => return Html(html).into_response(),
                    Err(err) => return internal_error(err),
                }
            }
            let Some(connection_info) = (match state.platform.db_connections.get_project_connection(
                &owner,
                &project,
                &connection,
            ) {
                Ok(item) => item,
                Err(err) => return internal_error(err),
            }) else {
                return (
                    StatusCode::NOT_FOUND,
                    Html("db connection not found".to_string()),
                )
                    .into_response();
            };
            if connection_info.database_kind != db_kind {
                return (
                    StatusCode::NOT_FOUND,
                    Html("db connection not found".to_string()),
                )
                    .into_response();
            }
            let capabilities = state
                .platform
                .db_runtime
                .capabilities_for_kind(&connection_info.database_kind);
            if tab_key == "maintenance" && !capabilities.maintenance {
                return (StatusCode::NOT_FOUND, Html("db tab not found".to_string()))
                    .into_response();
            }
            let nav = nav_classes(&owner, &project, "databases", Some("connections"));
            let route = format!("/projects/{owner}/{project}/db/{db_kind}/{connection}/{tab_key}");
            // One page for every engine. It renders its panels from the
            // driver's declared capabilities, so a new engine needs a driver
            // and no page at all.
            let table_page_key = "platform-project-table-connection";

            let requested = query.table.unwrap_or_default();
            let selected_table = requested.trim().to_lowercase().replace(
                |c: char| !c.is_ascii_alphanumeric() && c != '_' && c != '-' && c != '.',
                "-",
            );
            let query_example = match connection_info.database_kind.as_str() {
                "sekejap" => "SHOW TABLES".to_string(),
                _ => "-- Write SQL and click Run Query.".to_string(),
            };

            let table_query = if selected_table.is_empty() {
                String::new()
            } else {
                format!("?table={selected_table}")
            };
            let base = format!("/projects/{owner}/{project}/db/{db_kind}/{connection}");
            let mut suite_tabs = vec![
                json!({
                    "label": "Tables",
                    "href": format!("{base}/tables{table_query}"),
                    "classes": if tab_key == "tables" { "is-active" } else { "" },
                }),
                json!({
                    "label": "Query",
                    "href": format!("{base}/query{table_query}"),
                    "classes": if tab_key == "query" { "is-active" } else { "" },
                }),
                json!({
                    "label": "Graph",
                    "href": format!("{base}/graph{table_query}"),
                    "classes": if tab_key == "graph" { "is-active" } else { "" },
                }),
                json!({
                    "label": "Schema",
                    "href": format!("{base}/schema{table_query}"),
                    "classes": if tab_key == "schema" { "is-active" } else { "" },
                }),
                json!({
                    "label": "Mart",
                    "href": format!("{base}/mart{table_query}"),
                    "classes": if tab_key == "mart" { "is-active" } else { "" },
                }),
            ];
            if capabilities.maintenance {
                suite_tabs.push(json!({
                    "label": "Maintenance",
                    "href": format!("{base}/maintenance{table_query}"),
                    "classes": if tab_key == "maintenance" { "is-active" } else { "" },
                }));
            }

            // Every URL here is absent unless the engine declares the matching
            // capability, so the page hides a surface by finding no address for
            // it rather than by testing a driver name.
            let db_types = state
                .platform
                .db_runtime
                .type_catalog_for_kind(&connection_info.database_kind);
            let mut db_schema_api = serde_json::Map::new();
            if capabilities.create_table || capabilities.drop_table {
                // Table definition is scoped to the connection, so it reaches
                // the engine the user is actually looking at.
                db_schema_api.insert(
                    "tables".to_string(),
                    json!(format!(
                        "/api/projects/{owner}/{project}/db/connections/{}/tables",
                        connection_info.connection_id
                    )),
                );
            }
            if capabilities.edit_table_properties {
                // Editing attributes and index kinds is scoped to the
                // connection, like creation, so it reaches the right engine.
                db_schema_api.insert(
                    "properties".to_string(),
                    json!(format!(
                        "/api/projects/{owner}/{project}/db/connections/{}/tables",
                        connection_info.connection_id
                    )),
                );
                db_schema_api.insert(
                    "schema_sync".to_string(),
                    json!(format!("/api/projects/{owner}/{project}/tables/schema/sync")),
                );
            }
            if capabilities.maintenance {
                db_schema_api.insert(
                    "maintenance".to_string(),
                    json!(format!(
                        "/api/projects/{owner}/{project}/db/{}/maintenance",
                        connection_info.database_kind
                    )),
                );
            }

            let input = json!({
                "seo": {
                    "title": format!("{} - DB {} / {}", info.title, db_kind, connection),
                    "description": "Database suite"
                },
                "owner": info.owner,
                "project": info.project,
                "title": info.title,
                "project_href": format!("/projects/{owner}/{project}"),
                "connection": {
                    "id": connection_info.connection_id,
                    "name": connection_info.connection_label,
                    "kind": db_kind,
                    "slug": connection,
                    "icon_class": db_connection_icon_class(&connection_info.database_kind),
                    "credential_id": connection_info.credential_id,
                },
                "db_runtime_api": {
                    "describe": format!("/api/projects/{owner}/{project}/db/connections/{}/describe", connection_info.connection_id),
                    "schemas": format!("/api/projects/{owner}/{project}/db/connections/{}/schemas", connection_info.connection_id),
                    "tables": format!("/api/projects/{owner}/{project}/db/connections/{}/tables", connection_info.connection_id),
                    "functions": format!("/api/projects/{owner}/{project}/db/connections/{}/functions", connection_info.connection_id),
                    "preview": format!("/api/projects/{owner}/{project}/db/connections/{}/table-preview", connection_info.connection_id),
                    "query": format!("/api/projects/{owner}/{project}/db/connections/{}/query", connection_info.connection_id),
                },
                // The page renders its panels from this and never from the kind
                // name, so an engine gains a panel by declaring the capability
                // in its driver.
                "capabilities": capabilities,
                // The engine's own column types, so the picker offers what this
                // database actually has rather than a generic set.
                "db_types": db_types,
                // Schema definition still travels an engine-owned route. The URL
                // is supplied only when the engine supports the operation, so
                // the page tests for the URL rather than for a driver name.
                "db_schema_api": db_schema_api,
                "suite_tabs": suite_tabs,
                "object_groups": Vec::<Value>::new(),
                "tables": Vec::<Value>::new(),
                "table_summary": "Loading tables...",
                "preview": {
                    "columns": Vec::<String>::new(),
                    "rows": Vec::<Vec<String>>::new(),
                    "empty": true,
                },
                "query_example": query_example,
                "schema_text": "{}",
                "tab_flags": {
                    "tables": tab_key == "tables",
                    "query": tab_key == "query",
                    "graph": tab_key == "graph",
                    "schema": tab_key == "schema",
                    "mart": tab_key == "mart",
                    "maintenance": tab_key == "maintenance",
                },
                "nav": nav,
            });
            match render_page(&state, table_page_key, &route, input) {
                Ok(html) => Html(html).into_response(),
                Err(err) => internal_error(err),
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, Html("project not found".to_string())).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn project_files_page(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    uri: Uri,
    Path((owner, project)): Path<(String, String)>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Response {
    let browse_path = params.get("path").cloned().unwrap_or_default();
    render_files_page(
        state,
        headers,
        uri,
        owner,
        project,
        "storages",
        None,
        browse_path,
    )
    .await
}

async fn project_files_tab_page(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    uri: Uri,
    Path((owner, project, tab)): Path<(String, String, String)>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Response {
    let selected_storage = slug_segment(tab.trim());
    let selected_storage = if selected_storage.is_empty() {
        None
    } else {
        Some(selected_storage)
    };
    let browse_path = params.get("path").cloned().unwrap_or_default();
    render_files_page(
        state,
        headers,
        uri,
        owner,
        project,
        "explorer",
        selected_storage,
        browse_path,
    )
    .await
}

async fn render_files_page(
    state: PlatformAppState,
    headers: HeaderMap,
    uri: Uri,
    owner: String,
    project: String,
    tab: &str,
    selected_storage: Option<String>,
    browse_path: String,
) -> Response {
    if let Err(response) = require_project_page_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::FilesRead,
    ) {
        return response;
    }
    match maybe_forward_project_page_to_worker(&state, &headers, &uri, &owner, &project).await {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    match state.platform.projects.get_project(&owner, &project) {
        Ok(Some(info)) => {
            let nav = nav_classes(&owner, &project, "files", None);
            let route = format!("/projects/{owner}/{project}/files");
            let selected_storage = selected_storage.unwrap_or_else(|| "default".to_string());
            let active_tab = if tab == "explorer" {
                "explorer"
            } else {
                "storages"
            };

            // Build file listing for the requested browse_path
            let (folders, files, rel_path) = if active_tab == "explorer" {
                match state.platform.file.ensure_project_layout(&owner, &project) {
                    Ok(layout) => {
                        let rel = browse_path.trim().trim_start_matches('/').to_string();
                        let zebfs = layout.open_files();
                        let mut fds: Vec<serde_json::Value> = Vec::new();
                        let mut fls: Vec<serde_json::Value> = Vec::new();
                        let rel_path = match zebfs.list(&rel) {
                            Ok(entries) => {
                                for entry in entries {
                                    let name = entry.name;
                                    let path = entry.path;
                                    let access =
                                        crate::platform::services::zebfs_acl::effective_access(
                                            &zebfs, &path,
                                        )
                                        .map(|value| value.as_str())
                                        .unwrap_or("private");
                                    if matches!(entry.kind, crate::zebfs::ZebFsEntryKind::Prefix) {
                                        fds.push(json!({
                                            "name": name,
                                            "path": path,
                                            "access": access,
                                            "public": access == "public_read",
                                            "protected": false,
                                        }));
                                    } else {
                                        let modified = entry
                                            .modified
                                            .and_then(|t| {
                                                t.duration_since(std::time::UNIX_EPOCH).ok()
                                            })
                                            .map(|d| d.as_secs())
                                            .unwrap_or(0);
                                        fls.push(json!({
                                            "name": name,
                                            "path": path.clone(),
                                            "size": entry.size,
                                            "modified": modified,
                                            "access": access,
                                            "public": access == "public_read",
                                            "url": format!("/fs/{owner}/{project}/{path}"),
                                        }));
                                    }
                                }
                                rel
                            }
                            Err(_) => String::new(),
                        };
                        (fds, fls, rel_path)
                    }
                    Err(_) => (vec![], vec![], String::new()),
                }
            } else {
                (vec![], vec![], String::new())
            };

            let zebflow_cfg = match state.platform.zebflow_cfg.read_or_default(&owner, &project) {
                Ok(config) => config,
                Err(err) => return internal_error(err),
            };
            // Already validated by the reader, so an unknown word never reaches
            // the page.
            let file_backend = match zebflow_cfg.configs.files.effective_backend() {
                Ok(backend) => backend,
                Err(err) => return internal_error(PlatformError::new(err.code, err.message)),
            };

            let input = json!({
                "seo": {
                    "title": format!("{} - Files", info.title),
                    "description": "Project file storage — public and private."
                },
                "owner": info.owner,
                "project": info.project,
                "title": info.title,
                "project_href": format!("/projects/{owner}/{project}"),
                "current_menu": "Files",
                "active_tab": active_tab,
                "selected_storage": selected_storage,
                // Where this project's files actually live. Configured here
                // rather than in Settings: it is a fact about the store you are
                // looking at, and it was previously shown in a place you could
                // not act on it from.
                "storage": {
                    "backend": file_backend.as_str(),
                    "backend_label": file_backend.label(),
                    "declared": zebflow_cfg.configs.files.backend.is_some(),
                    "field": "spec.files.backend",
                    "accepted": crate::zebfs::FILE_BACKENDS,
                },
                "storages": [
                    {
                        "name": "default",
                        "backend": file_backend.label(),
                        "namespace": format!("{owner}/{project}"),
                        "tags": ["default"],
                        "open_href": format!("/projects/{owner}/{project}/files/default")
                    }
                ],
                "api": {
                    "list":  format!("/api/projects/{owner}/{project}/files/list"),
                    "mkdir": format!("/api/projects/{owner}/{project}/files/mkdir"),
                    "upload": format!("/api/projects/{owner}/{project}/files/upload"),
                    "rm":    format!("/api/projects/{owner}/{project}/files/rm"),
                    "access": format!("/api/projects/{owner}/{project}/files/access"),
                },
                "browser": {
                    "path": rel_path,
                    "folders": folders,
                    "files": files,
                },
                "nav": nav,
            });
            match render_page(&state, "platform-project-section", &route, input) {
                Ok(html) => Html(html).into_response(),
                Err(err) => internal_error(err),
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, Html("project not found".to_string())).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn project_todo_page(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    uri: Uri,
    Path((owner, project)): Path<(String, String)>,
) -> Response {
    render_section_page(
        state,
        headers,
        uri,
        owner,
        project,
        "todo",
        ProjectCapability::ProjectRead,
        "Todo",
        "Collaborative notes and task lists for project delivery.",
        vec![
            json!({"title":"Backlog","description":"Track pending improvements and fixes."}),
            json!({"title":"Sprint Tasks","description":"Focus tasks tied to current release cycle."}),
        ],
    )
    .await
}

async fn project_settings_page(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    uri: Uri,
    Path((owner, project)): Path<(String, String)>,
) -> Response {
    render_settings_tab_page(state, headers, uri, owner, project, "general".to_string()).await
}

async fn project_settings_tab_page(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    uri: Uri,
    Path((owner, project, tab)): Path<(String, String, String)>,
) -> Response {
    render_settings_tab_page(state, headers, uri, owner, project, tab).await
}

async fn project_infrastructure_page(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    uri: Uri,
    Path((owner, project)): Path<(String, String)>,
) -> Response {
    if let Err(response) = require_project_page_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::SettingsRead,
    ) {
        return response;
    }
    match maybe_forward_project_page_to_worker(&state, &headers, &uri, &owner, &project).await {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }

    match state.platform.projects.get_project(&owner, &project) {
        Ok(Some(info)) => {
            let nav = nav_classes(&owner, &project, "settings", None);
            let runtime = match cluster_runtime_summary(&state, &owner, &project) {
                Ok(runtime) => runtime,
                Err(err) => return internal_error(err),
            };
            match render_page(
                &state,
                "platform-project-infrastructure",
                &format!("/projects/{owner}/{project}/infrastructure"),
                json!({
                    "seo": {
                        "title": format!("{} - Infrastructure", info.title),
                        "description": "Cluster runtime control"
                    },
                    "project": {
                        "owner": owner,
                        "project": project,
                        "title": info.title,
                    },
                    "nav": nav,
                    "runtime": runtime,
                }),
            ) {
                Ok(html) => Html(html).into_response(),
                Err(err) => internal_error(err),
            }
        }
        Ok(None) => Redirect::to(HOME_PATH).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn render_settings_tab_page(
    state: PlatformAppState,
    headers: HeaderMap,
    uri: Uri,
    owner: String,
    project: String,
    raw_tab: String,
) -> Response {
    if let Err(response) = require_project_page_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::SettingsRead,
    ) {
        return response;
    }
    match maybe_forward_project_page_to_worker(&state, &headers, &uri, &owner, &project).await {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }

    // Libraries, nodes and dependencies moved to the Hub, which is where they
    // are installed. Send the old address to the new one rather than quietly
    // rendering General — a bookmark landing on the wrong page with no
    // explanation is worse than either a redirect or a refusal.
    if let Some(moved) = settings_tab_moved_to_hub(&raw_tab) {
        let target = if moved.is_empty() {
            format!("/projects/{owner}/{project}/hub")
        } else {
            format!("/projects/{owner}/{project}/hub/{moved}")
        };
        return Redirect::to(&target).into_response();
    }
    if let Some(moved) = settings_tab_moved_to_feature(&raw_tab) {
        return Redirect::to(&format!("/projects/{owner}/{project}/{moved}")).into_response();
    }

    let tab = normalize_settings_tab(&raw_tab);
    let tab_title = settings_tab_title(tab);
    let tab_subtitle = settings_tab_subtitle(tab);

    match state.platform.projects.get_project(&owner, &project) {
        Ok(Some(info)) => {
            let nav = nav_classes(&owner, &project, "settings", None);
            let route = if tab == "general" {
                format!("/projects/{owner}/{project}/settings")
            } else {
                format!("/projects/{owner}/{project}/settings/{tab}")
            };

            let tabs = settings_tab_items(&owner, &project, tab);
            let general_cards = settings_general_cards(&owner, &project);
            let policy_cards = settings_policy_cards();

            let assistant_config = match state
                .platform
                .assistant_configs
                .get_project_assistant_config(&owner, &project)
            {
                Ok(config) => config,
                Err(err) => return internal_error(err),
            };

            let assistant_credentials = match state
                .platform
                .credentials
                .list_project_credentials(&owner, &project)
            {
                Ok(items) => items
                    .into_iter()
                    .filter(|item| item.kind == "openai" || item.kind == "openrouter")
                    .map(|item| {
                        json!({
                            "credential_id": item.credential_id,
                            "title": item.title,
                            "kind": item.kind
                        })
                    })
                    .collect::<Vec<_>>(),
                Err(err) => return internal_error(err),
            };

            let mcp_session = state
                .platform
                .mcp_sessions
                .get_for_project(&owner, &project);
            let transfer_operations = state
                .platform
                .project_operations
                .list(&owner, &project, 10)
                .unwrap_or_default();

            let zebflow_cfg = match state.platform.zebflow_cfg.read_or_default(&owner, &project) {
                Ok(config) => config,
                Err(err) => return internal_error(err),
            };
            let project_configuration_exists = state
                .platform
                .zebflow_cfg
                .canonical_exists(&owner, &project);
            // The declaration is already validated by the reader above, so an
            // unknown word never reaches the page.
            let file_backend = match zebflow_cfg.configs.files.effective_backend() {
                Ok(backend) => backend,
                Err(err) => return internal_error(PlatformError::new(err.code, err.message)),
            };

            let input = json!({
                "seo": {
                    "title": format!("{} - Settings / {}", info.title, tab_title),
                    "description": tab_subtitle
                },
                "owner": info.owner,
                "project": info.project,
                "title": info.title,
                "project_href": format!("/projects/{owner}/{project}"),
                "current_menu": "Settings",
                "settings_tabs": tabs,
                "active_tab": tab,
                "tab_flags": {
                    "general": tab == "general",
                    "git": tab == "git",
                    "addressing": tab == "addressing",
                    "members": tab == "members",
                    "policy": tab == "policy",
                    "automatons": tab == "automatons",
                    "logs": tab == "logs"
                },
                "page_title": tab_title,
                "page_subtitle": tab_subtitle,
                "cards_general": general_cards,
                "cards_policy": policy_cards,
                "assistant": {
                    "api": {
                        "config": format!("/api/projects/{owner}/{project}/assistant/config")
                    },
                    "config": assistant_config,
                    "credentials": assistant_credentials
                },
                "project_configuration": {
                    "api_version": crate::contracts::CONTRACT_API_VERSION,
                    "kind": crate::contracts::ContractKind::ProjectConfiguration.as_str(),
                    "path": crate::contracts::kinds::PROJECT_CONFIGURATION_FILE,
                    "metadata_name": project,
                    "status": if project_configuration_exists { "valid" } else { "missing" },
                    "valid": project_configuration_exists
                },
                "profile": {
                    "api": format!("/api/projects/{owner}/{project}/settings/profile"),
                    "config": zebflow_cfg.metadata
                },
                "rwe": {
                    "api": format!("/api/projects/{owner}/{project}/settings/rwe"),
                    "config": zebflow_cfg.configs.rwe
                },
                "logging": {
                    "api": format!("/api/projects/{owner}/{project}/settings/logging"),
                    "invocations_api": format!("/api/projects/{owner}/{project}/settings/logs/invocations"),
                    "config": zebflow_cfg.configs.pipelines.logging
                },
                "addressing": {
                    "api": format!("/api/projects/{owner}/{project}/settings/addressing"),
                    "check_api": format!("/api/projects/{owner}/{project}/settings/addressing/check"),
                    "data": if tab == "addressing" {
                        match addressing_section_json(&state, &owner, &project, &zebflow_cfg) {
                            Ok(data) => data,
                            Err(err) => return internal_error(err),
                        }
                    } else { Value::Null }
                },
                "members": {
                    "members_api": format!("/api/projects/{owner}/{project}/members"),
                    "invites_api": format!("/api/projects/{owner}/{project}/invites"),
                    // Which roles this session may hand out. The ceiling is
                    // enforced server-side either way; offering a role the
                    // reader cannot grant only invites a refusal they could
                    // have been spared.
                    "grantable_roles": grantable_role_options(&state, &headers, &owner, &project),
                },
                "git": {
                    "remote_api": format!("/api/projects/{owner}/{project}/git/remote"),
                    "config": zebflow_cfg.configs.git.remote,
                    "health_api": format!("/api/projects/{owner}/{project}/git/health"),
                    "repair_api": format!("/api/projects/{owner}/{project}/git/repair")
                },
                "distribution": {
                    "api": format!("/api/projects/{owner}/{project}/settings/distribution"),
                    "config": zebflow_cfg.distribution.hub
                },
                "assets": {
                    "api": format!("/api/projects/{owner}/{project}/assets"),
                    "settings_api": format!("/api/projects/{owner}/{project}/settings/assets"),
                    "config": {
                        "max_asset_size_mb": zebflow_cfg.configs.files.uploads.effective_max_asset_size_mb(),
                        "max_file_size_mb": zebflow_cfg.configs.files.uploads.effective_max_file_size_mb(),
                        "webhook_body_max_mb": zebflow_cfg.configs.files.uploads.effective_webhook_body_max_mb(),
                        "pipeline_node_timeout_secs": zebflow_cfg.configs.pipelines.effective_node_timeout_secs()
                    }
                },
                "files": {
                    // The native store this project's files live in. The panel
                    // shows which one is active; it is not a writer, because
                    // there is one backend to choose from today.
                    "backend": file_backend.as_str(),
                    "backend_label": file_backend.label(),
                    "declared": zebflow_cfg.configs.files.backend.is_some(),
                    "field": "spec.files.backend",
                    "accepted": crate::zebfs::FILE_BACKENDS,
                },
                "reindex_api": format!("/api/projects/{owner}/{project}/reindex"),
                "transfer": {
                    "api": {
                        "operations": format!("/api/projects/{owner}/{project}/transfer/operations"),
                        "export_bundle": format!("/api/projects/{owner}/{project}/transfer/export/bundle"),
                        "export_files": format!("/api/projects/{owner}/{project}/transfer/export/files"),
                        "export_full": format!("/api/projects/{owner}/{project}/transfer/export/full"),
                        "import_bundle": format!("/api/projects/{owner}/{project}/transfer/import/bundle"),
                        "import_files": format!("/api/projects/{owner}/{project}/transfer/import/files"),
                        "import_full": format!("/api/projects/{owner}/{project}/transfer/import/full"),
                        "rollback": format!("/api/projects/{owner}/{project}/transfer/rollback"),
                    },
                    "operations": transfer_operations,
                },
                "mcp": {
                    "active": mcp_session.as_ref().map(|session| session.enabled).unwrap_or(false),
                    "status_label": if mcp_session.as_ref().map(|session| session.enabled).unwrap_or(false) { "active" } else { "inactive" },
                    "capabilities": mcp_session
                        .as_ref()
                        .map(|session| session.capabilities.iter().map(|cap| cap.key()).collect::<Vec<_>>())
                        .unwrap_or_default()
                },
                "nav": nav,
            });
            match render_page(&state, "platform-project-settings", &route, input) {
                Ok(html) => Html(html).into_response(),
                Err(err) => internal_error(err),
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, Html("project not found".to_string())).into_response(),
        Err(err) => internal_error(err),
    }
}

/// Settings tabs that now live in the Hub, and where each went.
fn settings_tab_moved_to_hub(raw: &str) -> Option<&'static str> {
    match raw.trim().to_ascii_lowercase().as_str() {
        // Libraries went twice: first to a hub tab, then into the catalogue
        // itself — a library is a package, so it is browsed, installed and
        // removed where every other package is.
        "libraries" => Some(""),
        "nodes" => Some("nodes"),
        "dependencies" => Some("dependencies"),
        _ => None,
    }
}

/// Settings tabs that moved to the feature they configure.
///
/// `files` is the storage backend, which now sits with the Files page — a fact
/// about the store you are looking at belongs where you are looking at it.
fn settings_tab_moved_to_feature(raw: &str) -> Option<&'static str> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "files" => Some("files"),
        _ => None,
    }
}

fn normalize_settings_tab(raw: &str) -> &'static str {
    match raw.trim().to_ascii_lowercase().as_str() {
        "" | "general" => "general",
        "git" => "git",
        "addressing" => "addressing",
        "members" => "members",
        "policy" => "policy",
        "automatons" => "automatons",
        "logs" => "logs",
        _ => "general",
    }
}


/// The roles this session may grant in one project, weakest first.
///
/// Mirrors the server-side ceiling rather than replacing it: `upsert_member`
/// and `create_invite` both refuse a role above the actor's own. This exists so
/// the dropdown does not offer a choice that will be refused.
fn grantable_role_options(
    state: &PlatformAppState,
    headers: &HeaderMap,
    owner: &str,
    project: &str,
) -> Vec<Value> {
    let Some(actor) = session_owner(state, headers) else {
        return Vec::new();
    };
    let actor_role = state
        .platform
        .project_members
        .role_of_public(owner, project, &actor)
        .unwrap_or(crate::platform::model::ProjectAccessRolePreset::Guest);
    crate::platform::services::access::roles::ROLE_LADDER
        .iter()
        .filter(|role| crate::platform::services::access::roles::can_grant(actor_role, **role))
        .map(|role| json!({ "key": role.key(), "title": role.title() }))
        .collect()
}

fn settings_tab_title(tab: &str) -> &'static str {
    match tab {
        "git" => "Git",
        "addressing" => "Addressing",
        "members" => "Members",
        "policy" => "Policy",
        "automatons" => "Automatons",
        "logs" => "Logs",
        _ => "General",
    }
}

fn settings_tab_subtitle(tab: &str) -> &'static str {
    match tab {
        "git" => "The remote this project pushes to, the branch it works on, and the health of its repository.",
        "addressing" => "Where this project answers: its dev host, the domains you add, what each host serves, and the web server config to paste.",
        "members" => "Who works on this project, what each of them may do, and who has been invited.",
        "policy" => "Capability boundaries, runtime constraints, and session controls.",
        "automatons" => "Assistant and automation runtime configuration per project.",
        "logs" => "Project-owned invocation history, storage size, retention, and cleanup.",
        _ => "Core project defaults and shared runtime switches.",
    }
}

fn settings_tab_items(owner: &str, project: &str, active: &str) -> Vec<Value> {
    let base = format!("/projects/{owner}/{project}/settings");
    let entries = [
        ("general", "General"),
        ("git", "Git"),
        ("addressing", "Addressing"),
        ("members", "Members"),
        ("policy", "Policy"),
        ("automatons", "Automatons"),
        ("logs", "Logs"),
    ];
    entries
        .iter()
        .map(|(key, label)| {
            json!({
                "key": *key,
                "label": *label,
                "href": if *key == "general" { base.clone() } else { format!("{base}/{key}") },
                "classes": if *key == active { "is-active" } else { "" }
            })
        })
        .collect::<Vec<_>>()
}

/// Where the settings that are not here went.
///
/// Configuration that belongs to a feature now lives with that feature, but
/// people will look in Settings first for a long time. These are signposts, not
/// duplicates — each one points at the single place the thing is actually
/// configured. (The card that used to sit here described connected services and
/// linked to the libraries tab, which was neither.)
fn settings_general_cards(owner: &str, project: &str) -> Vec<Value> {
    vec![
        json!({
            "title": "Libraries",
            "description": "Web libraries are hub packages: browse, install and remove them in the catalogue.",
            "href": format!("/projects/{owner}/{project}/hub"),
            "tag": "Hub"
        }),
        json!({
            "title": "Nodes",
            "description": "Pipeline node contracts available to this project, and installing more.",
            "href": format!("/projects/{owner}/{project}/hub/nodes"),
            "tag": "Hub"
        }),
        json!({
            "title": "Dependencies",
            "description": "Exact library and node resolutions recorded in zeb.lock.",
            "href": format!("/projects/{owner}/{project}/hub/dependencies"),
            "tag": "Hub"
        }),
        json!({
            "title": "File storage",
            "description": "The backend this project keeps its files in. Shown with the files themselves.",
            "href": format!("/projects/{owner}/{project}/files"),
            "tag": "Files"
        }),
        json!({
            "title": "Databases",
            "description": "Connections, credentials, and the engines this project reads and writes.",
            "href": format!("/projects/{owner}/{project}/db/connections"),
            "tag": "Data"
        }),
    ]
}

fn settings_policy_cards() -> Vec<Value> {
    vec![
        json!({
            "title":"Capability Gate",
            "description":"Subject capability checks enforced across REST, MCP, and assistant channels.",
            "href":"#",
            "tag":"Access"
        }),
        json!({
            "title":"Request Boundary",
            "description":"Input validation, payload size bounds, and deterministic error contracts.",
            "href":"#",
            "tag":"Runtime"
        }),
        json!({
            "title":"Session Scope",
            "description":"Project-scoped session constraints for remote control and internal assistant execution.",
            "href":"#",
            "tag":"Session"
        }),
    ]
}

fn node_group_rank(kind: &str) -> u8 {
    if kind.starts_with("n.trigger.") {
        0
    } else if kind == "n.script" || kind.starts_with("n.script.") {
        1
    } else if kind.starts_with("n.logic.") {
        2
    } else if kind.starts_with("n.ai.") {
        3
    } else {
        4
    }
}

fn node_group_prefix(kind: &str) -> &'static str {
    if kind.starts_with("n.trigger.") {
        "n.trigger"
    } else if kind == "n.script" || kind.starts_with("n.script.") {
        "n.script"
    } else if kind.starts_with("n.logic.") {
        "n.logic"
    } else if kind.starts_with("n.ai.") {
        "n.ai"
    } else {
        ""
    }
}

fn settings_nodes() -> (usize, Vec<Value>) {
    let mut defs = crate::pipeline::nodes::builtin_node_definitions();
    defs.extend(NodeRegistryService::embedded_official_definitions());
    defs.sort_by(|a, b| {
        node_group_rank(&a.kind)
            .cmp(&node_group_rank(&b.kind))
            .then_with(|| a.kind.cmp(&b.kind))
    });
    let total = defs.len();
    let mut groups: Vec<Value> = Vec::new();
    let mut current_prefix = String::new();
    let mut current_nodes: Vec<Value> = Vec::new();
    for def in defs {
        let prefix = node_group_prefix(&def.kind).to_string();
        if prefix != current_prefix && !current_nodes.is_empty() {
            groups.push(json!({
                "prefix": current_prefix,
                "nodes": current_nodes.drain(..).collect::<Vec<_>>()
            }));
        }
        current_prefix = prefix;
        current_nodes.push(json!({
            "kind": def.kind,
            "title": def.title,
            "description": def.description,
            "ai_registered": def.ai_tool.registered,
            "source": "built-in"
        }));
    }
    if !current_nodes.is_empty() {
        groups.push(json!({ "prefix": current_prefix, "nodes": current_nodes }));
    }
    (total, groups)
}

async fn render_section_page(
    state: PlatformAppState,
    headers: HeaderMap,
    uri: Uri,
    owner: String,
    project: String,
    section_key: &str,
    capability: ProjectCapability,
    section_title: &str,
    section_desc: &str,
    cards: Vec<Value>,
) -> Response {
    if let Err(response) =
        require_project_page_capability(&state, &headers, &owner, &project, capability)
    {
        return response;
    }
    match maybe_forward_project_page_to_worker(&state, &headers, &uri, &owner, &project).await {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }

    match state.platform.projects.get_project(&owner, &project) {
        Ok(Some(info)) => {
            let nav = nav_classes(&owner, &project, section_key, None);
            let route = format!("/projects/{owner}/{project}/{section_key}");
            let input = json!({
                "seo": {
                    "title": format!("{} - {}", info.title, section_title),
                    "description": section_desc
                },
                "owner": info.owner,
                "project": info.project,
                "title": info.title,
                "project_href": format!("/projects/{owner}/{project}"),
                "current_menu": section_title,
                "page_title": section_title,
                "page_subtitle": section_desc,
                "cards": cards,
                "nav": nav,
            });
            match render_page(&state, "platform-project-section", &route, input) {
                Ok(html) => Html(html).into_response(),
                Err(err) => internal_error(err),
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, Html("project not found".to_string())).into_response(),
        Err(err) => internal_error(err),
    }
}

fn pipeline_tab_payload(
    tab: &str,
) -> Option<(&'static str, &'static str, &'static str, Vec<Value>)> {
    match tab {
        "webhooks" => Some((
            "webhooks",
            "Webhook Pipelines",
            "Inbound HTTP triggers mapped to project pipelines.",
            Vec::new(),
        )),
        "schedules" => Some((
            "schedules",
            "Schedule Pipelines",
            "Cron-based and interval-based recurring jobs.",
            Vec::new(),
        )),
        "manual" => Some((
            "manual",
            "Manual Pipelines",
            "Pipelines triggered explicitly from API/UI manual execute requests.",
            Vec::new(),
        )),
        "functions" => Some((
            "functions",
            "Function Pipelines",
            "Callable in-house functions for reuse across workflows.",
            Vec::new(),
        )),
        _ => None,
    }
}

fn project_nav_map(owner: &str, project: &str) -> String {
    let b = format!("/projects/{owner}/{project}");
    let pb = format!("{b}/pipelines");
    format!(
        "  - Pipelines › Registry: {pb}/registry?path=/\n\
           - Pipelines › Webhooks: {pb}/webhooks\n\
           - Pipelines › Schedules: {pb}/schedules\n\
           - Pipelines › Manual: {pb}/manual\n\
           - Pipelines › Functions: {pb}/functions\n\
           - Dashboard: {b}/dashboard\n\
           - Credentials: {b}/credentials\n\
           - Databases / Tables (lists all connections): {b}/db/connections\n\
           - Files: {b}/files\n\
           - Todo: {b}/todo\n\
           - Settings: {b}/settings\n\
         \n\
         DB connection sub-pages (substitute actual db_kind and connection_id):\n\
           - {b}/db/{{db_kind}}/{{connection_id}}/tables  — browse tables\n\
           - {b}/db/{{db_kind}}/{{connection_id}}/query   — run SQL / query UI\n\
           - {b}/db/{{db_kind}}/{{connection_id}}/schema  — schema explorer"
    )
}

fn nav_classes(owner: &str, project: &str, main: &str, pipeline_sub: Option<&str>) -> Value {
    let pipelines_base = format!("/projects/{owner}/{project}/pipelines");

    json!({
        "title": "Project Menu",
        "links": {
            "pipelines_registry": format!("{pipelines_base}/registry?path=/"),
            "pipelines_editor": format!("/projects/{owner}/{project}/editor"),
            "pipelines_webhooks": format!("{pipelines_base}/webhooks"),
            "pipelines_schedules": format!("{pipelines_base}/schedules"),
            "pipelines_manual": format!("{pipelines_base}/manual"),
            "pipelines_functions": format!("{pipelines_base}/functions"),
            "hub": format!("/projects/{owner}/{project}/hub"),
            "dashboard": format!("/projects/{owner}/{project}/dashboard"),
            "credentials": format!("/projects/{owner}/{project}/credentials"),
            "db_connections": format!("/projects/{owner}/{project}/db/connections"),
            "tables_connections": format!("/projects/{owner}/{project}/db/connections"),
            "files": format!("/projects/{owner}/{project}/files"),
            "todo": format!("/projects/{owner}/{project}/todo"),
            "settings": format!("/projects/{owner}/{project}/settings"),
        },
        "classes": {
            "pipelines": if main == "pipelines" { "is-active" } else { "" },
            "hub": if main == "hub" { "is-active" } else { "" },
            "dashboard": if main == "dashboard" { "is-active" } else { "" },
            "credentials": if main == "credentials" { "is-active" } else { "" },
            "databases": if main == "databases" { "is-active" } else { "" },
            "tables": if main == "databases" { "is-active" } else { "" },
            "files": if main == "files" { "is-active" } else { "" },
            "todo": if main == "todo" { "is-active" } else { "" },
            "settings": if main == "settings" { "is-active" } else { "" },
            "pipeline_registry": if pipeline_sub == Some("registry") { "is-active" } else { "" },
            "pipeline_editor": if pipeline_sub == Some("editor") { "is-active" } else { "" },
            "pipeline_webhooks": if pipeline_sub == Some("webhooks") { "is-active" } else { "" },
            "pipeline_schedules": if pipeline_sub == Some("schedules") { "is-active" } else { "" },
            "pipeline_manual": if pipeline_sub == Some("manual") { "is-active" } else { "" },
            "pipeline_functions": if pipeline_sub == Some("functions") { "is-active" } else { "" },
            "db_connections": if main == "databases" { "is-active" } else { "" },
            "table_connections": if main == "databases" { "is-active" } else { "" },
        }
    })
}

/// What this project has installed: libraries, nodes, and the lock over them.
///
/// These three answer one question — "what is in this project that came from
/// somewhere else" — and the Hub is where that question is asked, because the
/// Hub is where the installing happens. Settings used to own all three, which
/// meant the inventory lived in one place and the install button in another.
fn installed_artifacts_payload(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
) -> Result<Value, PlatformError> {
    let (node_count, node_groups) = settings_nodes();

    // Merge the embedded manifests with this project's enabled state.
    let rwe_libs = state
        .platform
        .zebflow_cfg
        .get_rwe_libraries(owner, project)
        .unwrap_or_default();
    let libraries_available = state
        .platform
        .library
        .list()
        .map(|m| {
            let enabled_entry = rwe_libs.get(&m.name);
            json!({
                "name": m.name,
                "description": m.description,
                "packed_version": m.packed_version(),
                "packed_kind": m.packed_kind(),
                "enabled": enabled_entry.is_some(),
                "installed_version": enabled_entry.map(|e| e.version.clone()),
                "source": enabled_entry.map(|e| e.source.clone())
            })
        })
        .collect::<Vec<_>>();
    let dependency_status = state
        .platform
        .dependency_lock
        .status(owner, project, &rwe_libs)?;

    Ok(json!({
        "libraries_available": libraries_available,
        "libraries_api": format!("/api/projects/{owner}/{project}/rwe/libraries"),
        "dependencies": {
            "api": format!("/api/projects/{owner}/{project}/dependencies"),
            "status": dependency_status
        },
        "node_count": node_count,
        "node_groups": node_groups,
        "nodes_install_api": format!("/api/projects/{owner}/{project}/nodes/install"),
    }))
}

fn normalize_hub_tab(raw: &str) -> &'static str {
    match raw.trim() {
        "" | "packs" | "assets" => "packs",
        "my-packs" => "my-packs",
        "publish" => "publish",
        "nodes" => "nodes",
        "dependencies" => "dependencies",
        // "libraries" is deliberately absent: the tab was a second front-end
        // for the same install the Browse rows perform, still speaking the
        // retired `zeb/*` names. Old addresses fall through to Browse.
        _ => "packs",
    }
}

fn hub_tab_items(owner: &str, project: &str, active: &str) -> Vec<Value> {
    let base = format!("/projects/{owner}/{project}/hub");
    let tabs = vec!["packs", "my-packs", "publish", "nodes", "dependencies"];
    tabs.into_iter()
        .map(|tab| {
            let label = match tab {
                "packs" => "Browse",
                "my-packs" => "Published",
                "publish" => "Publish",
                "nodes" => "Nodes",
                "dependencies" => "Dependencies",
                _ => tab,
            };
            let href = if tab == "packs" {
                base.clone()
            } else {
                format!("{base}/{tab}")
            };
            json!({
                "label": label,
                "href": href,
                "classes": if active == tab { "is-active" } else { "" }
            })
        })
        .collect()
}

fn hub_publish_sources(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
    source_type: &str,
) -> Result<Vec<Value>, PlatformError> {
    let mut rows = state
        .platform
        .hub
        .list_publish_sources(owner, project, source_type)?
        .into_iter()
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(rows
        .into_iter()
        .filter_map(|item| serde_json::to_value(item).ok())
        .collect())
}

fn hub_asset_rows(
    state: &PlatformAppState,
    owner: &str,
    _project: &str,
    only_mine: bool,
) -> Result<Vec<Value>, PlatformError> {
    // "Mine" is what this owner published, which lives in the Public Hub
    // store; the plain listing is the blessed shelf a project installs from —
    // the same set `hub_search` offers over MCP.
    let packages: Vec<(crate::platform::model::HubAssetPackage, String)> = if only_mine {
        let packages = match state.platform.hub.list_asset_packages_by_owner(owner) {
            Ok(items) => items,
            Err(err) if err.code == "HUB_SERVICE_DISABLED" => Vec::new(),
            Err(err) => return Err(err),
        };
        let mut out = Vec::new();
        for package in packages {
            let latest_version = latest_installable_hub_version(
                state
                    .platform
                    .hub
                    .list_public_asset_versions(&package.package_id)?,
            );
            // Every version retracted means nothing here can be installed. The
            // row stays in the store so its coordinates remain taken, but a
            // listing that offers a package with no installable release is
            // offering nothing.
            if latest_version.is_empty() {
                continue;
            }
            out.push((package, latest_version));
        }
        out
    } else {
        state.platform.hub.installable_packages(owner)?
    };
    let mut rows = Vec::new();
    for (package, latest_version) in packages {
        let (summary, gallery) = hub_package_gallery_projection(&package);
        rows.push(json!({
            "package_id": package.package_id,
            "publisher_owner": package.publisher_owner,
            "publisher_id": package.publisher_id,
            "publisher_display_name": package.publisher_display_name,
            "publisher_url": package.publisher_url,
            "publisher_email": package.publisher_email,
            "asset_kind": package.asset_kind,
            "title": package.title,
            "description": package.description,
            "summary": summary,
            "image_url": package.image_url,
            "gallery": gallery,
            "visibility": package.visibility,
            "tags": package.tags,
            "latest_version": latest_version,
            "retracted": hub_retraction_json(package.retracted_at, &package.retracted_reason),
            "updated_at": package.updated_at,
            "source": "local",
            "repository_id": "",
            "repository_title": "Local",
        }));
    }
    Ok(rows)
}

fn public_hub_asset_item_json(
    state: &PlatformAppState,
    package: crate::platform::model::HubAssetPackage,
) -> Value {
    let latest_version = latest_installable_hub_version(
        state
            .platform
            .hub
            .list_public_asset_versions(&package.package_id)
            .unwrap_or_default(),
    );
    let (summary, gallery) = hub_package_gallery_projection(&package);
    json!({
        "package_id": package.package_id,
        "publisher_id": package.publisher_id,
        "publisher_display_name": package.publisher_display_name,
        "publisher_url": package.publisher_url,
        "asset_kind": package.asset_kind,
        "title": package.title,
        "description": package.description,
        "summary": summary,
        "image_url": package.image_url,
        "gallery": gallery,
        "visibility": package.visibility,
        "tags": package.tags,
        "latest_version": latest_version,
        "retracted": hub_retraction_json(package.retracted_at, &package.retracted_reason),
        "updated_at": package.updated_at,
        "service_instance_id": crate::platform::services::hub::DEFAULT_HUB_SERVICE_INSTANCE_ID,
    })
}

/// The retraction marker any coordinate-carrying response shows, or `null`.
///
/// Retraction keeps the coordinates and destroys the bytes, so a listing that
/// simply dropped the row would be describing a deletion that did not happen.
fn hub_retraction_json(retracted_at: Option<i64>, reason: &str) -> Value {
    match retracted_at {
        Some(retracted_at) => json!({"retracted_at": retracted_at, "reason": reason}),
        None => Value::Null,
    }
}

/// The newest release that still has bytes.
///
/// A retracted release stays listed and explorable, but it is not the version a
/// listing should offer as the one to take.
fn latest_installable_hub_version(
    versions: Vec<crate::platform::model::HubAssetVersion>,
) -> String {
    versions
        .into_iter()
        .find(|item| item.retracted_at.is_none())
        .map(|item| item.version)
        .unwrap_or_default()
}

/// A listing reads presentation from the mutable package row.
///
/// It never opens a release document to do it: presentation lives beside the
/// releases so that correcting it costs an update rather than a version bump,
/// and a listing that parsed the immutable artifact would undo that.
fn hub_package_gallery_projection(
    package: &crate::platform::model::HubAssetPackage,
) -> (String, Value) {
    let trimmed = package.summary.trim();
    let summary = if trimmed.is_empty() {
        package.description.clone()
    } else {
        trimmed.to_string()
    };
    // A package with nothing to show reports no gallery at all, rather than an
    // empty one the listing would have to special-case.
    let gallery = if package.gallery.cover.is_some() || !package.gallery.items.is_empty() {
        serde_json::to_value(&package.gallery).unwrap_or(Value::Null)
    } else {
        Value::Null
    };
    (summary, gallery)
}

fn public_hub_version_json(
    package: &crate::platform::model::HubAssetPackage,
    version: &crate::platform::model::HubAssetVersion,
) -> Value {
    json!({
        "package_id": version.package_id,
        "version": version.version,
        "publisher_id": package.publisher_id,
        "publisher_display_name": package.publisher_display_name,
        "publisher_url": package.publisher_url,
        "asset_kind": package.asset_kind,
        "title": package.title,
        "description": package.description,
        "image_url": package.image_url,
        "visibility": package.visibility,
        "tags": package.tags,
        "source_kind": version.source_kind,
        "artifact_sha256": version.artifact_sha256,
        "retracted": hub_retraction_json(version.retracted_at, &version.retracted_reason),
        "created_at": version.created_at,
        "manifest": {
            "asset_kind": package.asset_kind,
            "title": package.title,
            "description": package.description,
            "image_url": package.image_url,
            "visibility": package.visibility,
            "tags": package.tags,
            "publisher_id": package.publisher_id,
            "publisher_display_name": package.publisher_display_name,
            "publisher_url": package.publisher_url,
        },
    })
}

/// The detail body for one release, whether it still has bytes or not.
///
/// A retracted release keeps its coordinate and loses its artifact, so detail
/// describes it with a null `artifact` and a marker saying why: a project that
/// pinned the coordinate needs to read what happened, and a refusal here would
/// hide it. The artifact and install routes are where retraction refuses.
fn hub_asset_detail_response(
    state: &PlatformAppState,
    package: &crate::platform::model::HubAssetPackage,
    package_id: &str,
    version: &str,
) -> Response {
    match state.platform.hub.get_asset_version(package_id, version) {
        Ok(Some(version_row)) if version_row.retracted_at.is_some() => {
            return Json(json!({
                "ok": true,
                "version": public_hub_version_json(package, &version_row),
                "presentation": public_hub_presentation_json(package),
                "artifact": Value::Null,
            }))
            .into_response();
        }
        Ok(_) => {}
        Err(err) => return hub_api_error(err),
    }
    match state
        .platform
        .hub
        .get_asset_version_artifact(package_id, version)
    {
        Ok((version_row, artifact)) => Json(json!({
            "ok": true,
            "version": public_hub_version_json(package, &version_row),
            "presentation": public_hub_presentation_json(package),
            "artifact": public_hub_artifact_json(artifact),
        }))
        .into_response(),
        Err(err) => hub_api_error(err),
    }
}

/// The release document as the public detail route serves it.
///
/// Only `files` is withheld: provenance, publisher identity, and presentation
/// are no longer carried by a release, so there is nothing else left to strip.
fn public_hub_artifact_json(mut artifact: Value) -> Value {
    if let Some(object) = artifact.get_mut("spec").and_then(Value::as_object_mut) {
        object.remove("files");
    }
    artifact
}

/// Presentation for one package, read from the mutable row beside its releases.
///
/// Media is named and sized but its bytes are not inlined: they are fetched
/// from `/media/{name}`, which resolves the digest in the artifact store.
fn public_hub_presentation_json(package: &crate::platform::model::HubAssetPackage) -> Value {
    let (summary, gallery) = hub_package_gallery_projection(package);
    json!({
        "summary": summary,
        "description_md": package.description_md,
        "image_url": package.image_url,
        "gallery": gallery,
        "media": package
            .media
            .iter()
            .map(|item| json!({
                "name": item.name,
                "role": item.role,
                "content_type": item.content_type,
                "size_bytes": item.size_bytes,
            }))
            .collect::<Vec<_>>(),
    })
}

fn raw_hub_artifact_response_json(
    package: &crate::platform::model::HubAssetPackage,
    version_row: &crate::platform::model::HubAssetVersion,
    artifact: Value,
    artifact_size_bytes: u64,
) -> Value {
    json!({
        "ok": true,
        "version": public_hub_version_json(package, version_row),
        "artifact_sha256": version_row.artifact_sha256,
        "artifact_size_bytes": artifact_size_bytes,
        "artifact": artifact,
    })
}

fn require_hub_service_enabled(state: &PlatformAppState) -> Result<(), Response> {
    match state.platform.hub.get_default_service_instance() {
        Ok(Some(service)) if service.enabled => Ok(()),
        Ok(_) => Err((
            StatusCode::NOT_FOUND,
            Json(json!({
                "ok": false,
                "error": {
                    "code": "HUB_SERVICE_DISABLED",
                    "message": "hub service is not enabled"
                }
            })),
        )
            .into_response()),
        Err(err) => Err(internal_error(err)),
    }
}

fn hub_token_rows(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
) -> Result<Vec<Value>, PlatformError> {
    Ok(state
        .platform
        .hub
        .list_tokens(owner, project)?
        .into_iter()
        .map(hub_token_json)
        .collect())
}

fn bearer_token_from_headers(headers: &HeaderMap) -> Option<String> {
    let value = headers.get(axum::http::header::AUTHORIZATION)?;
    let value = value.to_str().ok()?.trim();
    let token = value.strip_prefix("Bearer ")?.trim();
    if token.is_empty() {
        None
    } else {
        Some(token.to_string())
    }
}

/// Strip `https?://user:token@` from git stderr before returning to the client.
fn redact_auth_urls(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(proto_end) = rest.find("://") {
        let before = &rest[..proto_end + 3];
        let after = &rest[proto_end + 3..];
        if let Some(at_pos) = after.find('@') {
            // Only redact if there's no whitespace between :// and @ (i.e. it's really a URL)
            if !after[..at_pos].contains(|c: char| c.is_whitespace()) {
                out.push_str(before);
                out.push_str("[redacted]@");
                rest = &after[at_pos + 1..];
                continue;
            }
        }
        out.push_str(before);
        rest = after;
    }
    out.push_str(rest);
    out
}

fn content_type_for_path(path: &FsPath) -> &'static str {
    match path.extension().and_then(|ext| ext.to_str()) {
        Some("mjs") | Some("js") => "text/javascript; charset=utf-8",
        Some("json") => "application/json; charset=utf-8",
        Some("geojson") => "application/geo+json; charset=utf-8",
        Some("xml") => "application/xml; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("svg") => "image/svg+xml; charset=utf-8",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("gif") => "image/gif",
        Some("webp") => "image/webp",
        Some("ico") => "image/x-icon",
        Some("pdf") => "application/pdf",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        Some("ttf") => "font/ttf",
        Some("mp4") => "video/mp4",
        Some("mp3") => "audio/mpeg",
        Some("csv") => "text/csv; charset=utf-8",
        Some("md") => "text/markdown; charset=utf-8",
        Some("yaml") | Some("yml") => "application/yaml; charset=utf-8",
        Some("txt") => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

fn db_connection_icon_class(database_kind: &str) -> &'static str {
    match database_kind {
        "postgresql" => "devicon-postgresql-plain colored",
        "mysql" => "devicon-mysql-original colored",
        "sqlite" => "devicon-sqlite-plain colored",
        "redis" => "devicon-redis-plain colored",
        "mongodb" => "devicon-mongodb-plain colored",
        "qdrant" => "zf-icon-default-db",
        _ => "zf-icon-default-db",
    }
}

fn template_kind_from_rel(rel: &str) -> &'static str {
    if rel.ends_with(".css") {
        "style"
    } else if rel.ends_with(".ts") {
        "script"
    } else if rel.contains("/pages/") || rel.starts_with("pages/") {
        "page"
    } else {
        "component"
    }
}

async fn api_meta(State(state): State<PlatformAppState>) -> Response {
    Json(json!({
        "ok": true,
        "data_adapter": state.platform.data.id(),
        "file_adapter": state.platform.file.id(),
        "project_data_factory": state.platform.project_data.id(),
        "project_data_engines": state.platform.project_data.enabled_engines(),
    }))
    .into_response()
}

// ── System info endpoint ─────────────────────────────────────────────────────

async fn api_system_info(State(state): State<PlatformAppState>, headers: HeaderMap) -> Response {
    // Require a logged-in session (not necessarily superadmin)
    if session_owner(&state, &headers).is_none() {
        return StatusCode::UNAUTHORIZED.into_response();
    }

    let info = tokio::task::spawn_blocking(collect_system_info)
        .await
        .unwrap_or_else(|_| json!({"ok": false, "error": "collection failed"}));

    Json(info).into_response()
}

fn collect_system_info() -> serde_json::Value {
    use sysinfo::{Disks, Pid, System};

    let mut sys = System::new_all();
    sys.refresh_all();

    // Two-pass CPU sampling for accurate usage (sysinfo requirement)
    sys.refresh_cpu_usage();
    std::thread::sleep(std::time::Duration::from_millis(250));
    sys.refresh_cpu_usage();

    // ── OS ──
    let os_name = System::name().unwrap_or_else(|| "Unknown".into());
    let os_version = System::os_version().unwrap_or_default();
    let kernel = System::kernel_version().unwrap_or_default();
    let hostname = System::host_name().unwrap_or_default();
    let arch = std::env::consts::ARCH;

    // Detect environment variant (Raspberry Pi, WSL, etc.)
    let os_variant = detect_os_variant();

    // ── CPU ──
    let cpu_count = sys.cpus().len();
    let cpu_brand = sys
        .cpus()
        .first()
        .map(|c| c.brand().to_string())
        .unwrap_or_default();
    let cpu_usage = sys.global_cpu_usage();

    // ── Memory ──
    let total_mem = sys.total_memory();
    let used_mem = sys.used_memory();
    let avail_mem = sys.available_memory();
    let mem_pct = if total_mem > 0 {
        used_mem as f64 / total_mem as f64 * 100.0
    } else {
        0.0
    };

    // ── Disk (aggregate all mounts) ──
    let disks = Disks::new_with_refreshed_list();
    let (total_disk, avail_disk) = disks.iter().fold((0u64, 0u64), |(t, a), d| {
        (t + d.total_space(), a + d.available_space())
    });
    let used_disk = total_disk.saturating_sub(avail_disk);
    let disk_pct = if total_disk > 0 {
        used_disk as f64 / total_disk as f64 * 100.0
    } else {
        0.0
    };

    // ── Current process ──
    let pid = Pid::from_u32(std::process::id());
    let (proc_cpu, proc_mem, proc_virt, proc_threads) = sys
        .process(pid)
        .map(|p| {
            (
                p.cpu_usage(),
                p.memory(),
                p.virtual_memory(),
                p.tasks().map(|t| t.len()).unwrap_or(0),
            )
        })
        .unwrap_or((0.0, 0, 0, 0));

    // Process uptime via process start_time vs system boot
    let proc_uptime = sys.process(pid).map(|p| p.run_time()).unwrap_or(0);

    // ── Capabilities ──
    let caps = collect_capabilities();

    json!({
        "ok": true,
        "system": {
            "os": {
                "name": os_name,
                "version": os_version,
                "kernel": kernel,
                "arch": arch,
                "hostname": hostname,
                "variant": os_variant,
                "container": detect_container_context(),
            },
            "cpu": {
                "cores": cpu_count,
                "brand": cpu_brand,
                "usage_pct": (cpu_usage * 10.0).round() / 10.0,
            },
            "memory": {
                "total_bytes": total_mem,
                "used_bytes": used_mem,
                "available_bytes": avail_mem,
                "usage_pct": (mem_pct * 10.0).round() / 10.0,
            },
            "disk": {
                "total_bytes": total_disk,
                "used_bytes": used_disk,
                "available_bytes": avail_disk,
                "usage_pct": (disk_pct * 10.0).round() / 10.0,
            }
        },
        "process": {
            "pid": std::process::id(),
            "cpu_pct": (proc_cpu * 10.0).round() / 10.0,
            "memory_bytes": proc_mem,
            "virtual_memory_bytes": proc_virt,
            "threads": proc_threads,
            "uptime_seconds": proc_uptime,
        },
        "capabilities": caps,
    })
}

fn detect_os_variant() -> &'static str {
    // Raspberry Pi: /proc/cpuinfo contains "Raspberry Pi"
    #[cfg(target_os = "linux")]
    if let Ok(info) = std::fs::read_to_string("/proc/cpuinfo") {
        if info.contains("Raspberry Pi") || info.contains("BCM") {
            return "raspberry-pi";
        }
    }

    // WSL: /proc/version contains "microsoft" or "WSL"
    #[cfg(target_os = "linux")]
    if let Ok(ver) = std::fs::read_to_string("/proc/version") {
        let lower = ver.to_lowercase();
        if lower.contains("microsoft") || lower.contains("wsl") {
            return "wsl";
        }
    }

    #[cfg(target_os = "macos")]
    {
        "macos"
    }
    #[cfg(target_os = "windows")]
    {
        "windows"
    }
    #[cfg(target_os = "linux")]
    {
        "linux"
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
    {
        "unknown"
    }
}

fn collect_capabilities() -> serde_json::Value {
    let home = dirs_next::home_dir().unwrap_or_default();
    let zf_base = home.join(".zebflow");

    // Python (system)
    let python_info = probe_python_system();

    // Python (managed by zebflow)
    let managed_python = zf_base
        .join("engines")
        .join("python")
        .join("bin")
        .join("python3");
    let python_managed_available = managed_python.exists();

    // Lightpanda
    let lp_path = zf_base.join("browsers").join("lightpanda");
    let lp_installed = lp_path.exists();
    let lp_version = if lp_installed {
        probe_binary_version(&lp_path, "--version")
    } else {
        None
    };

    // Chromium (chromiumoxide fetcher puts it in a subdirectory)
    let chromium_dir = zf_base.join("browsers").join("chromium");
    let chromium_installed = chromium_dir.exists()
        && chromium_dir
            .read_dir()
            .map(|mut d| d.next().is_some())
            .unwrap_or(false);

    // Ollama (managed)
    let ollama_path = zf_base.join("engines").join("ollama");
    let ollama_installed = ollama_path.exists();
    let ollama_version = if ollama_installed {
        probe_binary_version(&ollama_path, "--version")
    } else {
        // Also check system PATH
        probe_binary_version(std::path::Path::new("ollama"), "--version")
    };

    // SearXNG: check if running on default port
    let searxng_installed = std::net::TcpStream::connect("127.0.0.1:8888").is_ok();

    // vips (libvips CLI)
    let vips_version = probe_binary_version(std::path::Path::new("vips"), "--version");

    // Security / pentest tools (system PATH only — no managed path)
    let nmap_version = probe_binary_version(std::path::Path::new("nmap"), "--version");
    let nuclei_version = probe_binary_version(std::path::Path::new("nuclei"), "-version");
    let httpx_version = probe_binary_version(std::path::Path::new("httpx"), "-version");
    let trivy_version = probe_binary_version(std::path::Path::new("trivy"), "--version");
    let masscan_version = probe_binary_version(std::path::Path::new("masscan"), "--version");
    let ffuf_version = probe_binary_version(std::path::Path::new("ffuf"), "-V");
    let sqlmap_version = probe_binary_version(std::path::Path::new("sqlmap"), "--version");
    let nikto_version = probe_binary_version(std::path::Path::new("nikto"), "--version");

    json!({
        "python": python_info,
        "python_managed": {
            "available": python_managed_available,
            "path": if python_managed_available { Some(managed_python.to_string_lossy().to_string()) } else { None },
        },
        "lightpanda": {
            "installed": lp_installed,
            "version": lp_version,
        },
        "chromium": {
            "installed": chromium_installed,
        },
        "ollama": {
            "installed": ollama_installed || ollama_version.is_some(),
            "version": ollama_version,
        },
        "searxng": {
            "installed": searxng_installed,
        },
        "vips": {
            "installed": vips_version.is_some(),
            "version": vips_version,
        },
        "security": {
            "nmap":    { "installed": nmap_version.is_some(),    "version": nmap_version    },
            "nuclei":  { "installed": nuclei_version.is_some(),  "version": nuclei_version  },
            "httpx":   { "installed": httpx_version.is_some(),   "version": httpx_version   },
            "trivy":   { "installed": trivy_version.is_some(),   "version": trivy_version   },
            "masscan": { "installed": masscan_version.is_some(), "version": masscan_version },
            "ffuf":    { "installed": ffuf_version.is_some(),    "version": ffuf_version    },
            "sqlmap":  { "installed": sqlmap_version.is_some(),  "version": sqlmap_version  },
            "nikto":   { "installed": nikto_version.is_some(),   "version": nikto_version   },
        },
    })
}

fn detect_container_context() -> serde_json::Value {
    let in_docker = std::path::Path::new("/.dockerenv").exists();

    #[cfg(target_os = "linux")]
    let host_gateway: Option<String> =
        std::fs::read_to_string("/proc/net/route")
            .ok()
            .and_then(|content| {
                content.lines().skip(1).find_map(|line| {
                    let cols: Vec<&str> = line.split_whitespace().collect();
                    // Default route: Destination == 00000000
                    if cols.len() >= 3 && cols[1] == "00000000" {
                        u32::from_str_radix(cols[2], 16).ok().map(|hex| {
                            let b = hex.to_le_bytes();
                            format!("{}.{}.{}.{}", b[0], b[1], b[2], b[3])
                        })
                    } else {
                        None
                    }
                })
            });

    #[cfg(not(target_os = "linux"))]
    let host_gateway: Option<String> = None;

    json!({
        "in_docker": in_docker,
        "host_gateway": host_gateway,
    })
}

fn probe_python_system() -> serde_json::Value {
    // Try python3 first, then python
    for cmd in &["python3", "python"] {
        if let Ok(out) = std::process::Command::new(cmd).arg("--version").output() {
            if out.status.success() {
                let raw = format!(
                    "{}{}",
                    String::from_utf8_lossy(&out.stdout),
                    String::from_utf8_lossy(&out.stderr)
                );
                let version = raw.trim().trim_start_matches("Python ").to_string();
                return json!({ "available": true, "version": version, "cmd": cmd });
            }
        }
    }
    json!({ "available": false })
}

fn probe_binary_version(path: &std::path::Path, flag: &str) -> Option<String> {
    std::process::Command::new(path)
        .arg(flag)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| {
            let raw = format!(
                "{}{}",
                String::from_utf8_lossy(&o.stdout),
                String::from_utf8_lossy(&o.stderr)
            );
            raw.lines().next().unwrap_or("").trim().to_string()
        })
}

fn node_credential_requirements_from_manifest(
    item: &crate::pipeline::NodeContractItem,
    manifest: &NodePackageManifest,
) -> Vec<crate::pipeline::NodeCredentialRequirement> {
    let required_keys = item
        .config_schema
        .get("required")
        .and_then(|value| value.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|value| value.as_str().map(str::to_string))
                .collect::<std::collections::HashSet<_>>()
        })
        .unwrap_or_default();

    manifest
        .credentials
        .iter()
        .filter(|credential| !credential.kind.trim().is_empty())
        .map(|credential| {
            let config_key = credential.config_key.clone();
            let mut placeholder_names = credential.placeholders.keys().cloned().collect::<Vec<_>>();
            placeholder_names.sort();
            let required = !config_key.is_empty()
                && (required_keys.contains(&config_key)
                    || item
                        .dsl_flags
                        .iter()
                        .any(|flag| flag.config_key == config_key && flag.required));
            crate::pipeline::NodeCredentialRequirement {
                kind: credential.kind.clone(),
                title: credential.title.clone(),
                description: credential.description.clone(),
                config_key,
                required,
                placeholder_names,
            }
        })
        .collect()
}

fn apply_manifest_metadata_to_node_contract(
    item: &mut crate::pipeline::NodeContractItem,
    manifest: Option<&NodePackageManifest>,
) {
    if let Some(manifest) = manifest {
        // The manifest is the only authority on how a node is implemented.
        item.source = manifest.source.as_str().to_string();
        item.credential_requirements = node_credential_requirements_from_manifest(item, manifest);
    }
}

/// Public, machine-readable node contract extracted from native + official composite registry.
async fn docs_node_contract() -> Response {
    let mut items = crate::pipeline::nodes::builtin_node_definitions()
        .into_iter()
        .map(|def| {
            let mut item = crate::pipeline::NodeContractItem::from(def);
            item.tier = "official".to_string();
            item
        })
        .collect::<Vec<_>>();
    for manifest in NodeRegistryService::embedded_official_manifests() {
        let mut item = crate::pipeline::NodeContractItem::from(manifest.definition.clone());
        item.tier = "official".to_string();
        apply_manifest_metadata_to_node_contract(&mut item, Some(&manifest));
        items.push(item);
    }
    items.sort_by(|a, b| a.kind.cmp(&b.kind));
    Json(crate::pipeline::NodeContractDocument {
        ok: true,
        schema_version: crate::contracts::CONTRACT_API_VERSION,
        source: "pipeline::nodes + embedded_official_composites",
        items,
    })
    .into_response()
}

/// Public, machine-readable operation contract extracted from `platform::operations`.
async fn docs_operation_contract() -> Response {
    Json(crate::platform::operations::OperationContractDocument {
        ok: true,
        schema_version: "0.1",
        source: "platform::operations::OPERATIONS",
        items: crate::platform::operations::operation_contract_items(),
    })
    .into_response()
}

async fn api_list_node_definitions(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::SettingsRead,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::GET,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    let defs = state
        .platform
        .node_registry
        .merged_definitions(&owner, &project);

    let items: Vec<crate::pipeline::NodeContractItem> = defs
        .into_iter()
        .map(|def| {
            let kind = def.kind.clone();
            let mut item = crate::pipeline::NodeContractItem::from(def);

            // Set tier: official (native or embedded composite) vs community (installed).
            item.tier = if state.platform.node_registry.is_official(&kind) {
                "official".to_string()
            } else {
                "community".to_string()
            };
            let manifest = state
                .platform
                .node_registry
                .get_manifest(&owner, &project, &kind);
            apply_manifest_metadata_to_node_contract(&mut item, manifest.as_ref());

            // Resolve icon URL + content hash for cache-busting.
            let icon_bytes: Option<Vec<u8>> = state
                .platform
                .node_registry
                .load_icon_any(&owner, &project, &kind)
                .or_else(|| {
                    let path = format!("zebflow/{}.svg", kind);
                    platform_node_icon_asset(&path).map(|b| b.to_vec())
                });

            if let Some(bytes) = icon_bytes {
                item.icon_url = format!("/api/projects/{}/{}/nodes/icon/{}", owner, project, kind);
                use sha2::{Digest, Sha256};
                let hash = Sha256::digest(&bytes);
                item.icon_hash = format!(
                    "{:02x}{:02x}{:02x}{:02x}",
                    hash[0], hash[1], hash[2], hash[3]
                );
            }

            item
        })
        .collect();

    // Nodes the project describes but cannot run: their interface travelled with
    // the project while the bundle did not. They appear in the catalog so a
    // graph stays readable, marked unavailable so nothing tries to execute them.
    let mut items = items;
    for definition in state
        .platform
        .node_registry
        .unavailable_interface_definitions(&owner, &project)
    {
        let mut item = crate::pipeline::NodeContractItem::from(definition);
        item.available = false;
        item.tier = "community".to_string();
        items.push(item);
    }
    items.sort_by(|a, b| a.kind.cmp(&b.kind));

    Json(json!({
        "ok": true,
        "items": items
    }))
    .into_response()
}

async fn api_get_node_definition(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, kind)): Path<(String, String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::SettingsRead,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::GET,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    let merged = state
        .platform
        .node_registry
        .merged_definitions(&owner, &project);
    match merged.into_iter().find(|d| d.kind == kind) {
        Some(def) => {
            let mut item = crate::pipeline::NodeContractItem::from(def);
            item.tier = if state.platform.node_registry.is_official(&kind) {
                "official".to_string()
            } else {
                "community".to_string()
            };
            let manifest = state
                .platform
                .node_registry
                .get_manifest(&owner, &project, &kind);
            apply_manifest_metadata_to_node_contract(&mut item, manifest.as_ref());
            Json(json!({ "ok": true, "item": item })).into_response()
        }
        None => Json(json!({
            "ok": false,
            "error": format!("node kind '{}' not found", kind)
        }))
        .into_response(),
    }
}

// ── Node package install/uninstall/icon ─────────────────────────────────────

/// Body for installing a node bundle that is not published to a Hub.
#[derive(Debug, Deserialize)]
struct LocalNodeBundleRequest {
    /// Package identifier used for the install root and lock provenance.
    package_id: String,
    /// Release version recorded in `zeb.lock`.
    version: String,
    /// Optional install folder override.
    #[serde(default)]
    target_folder: String,
    /// The bundle itself, as a `HubPackage` envelope.
    artifact: serde_json::Value,
}

/// Reports what a locally supplied node bundle would do, before installing it.
async fn api_review_local_node_bundle(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Json(req): Json<LocalNodeBundleRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::SettingsWrite,
    ) {
        return response;
    }
    match state.platform.hub.review_local_node_bundle(
        &owner,
        &project,
        &req.package_id,
        &req.version,
        &req.target_folder,
        req.artifact,
    ) {
        Ok(review) => Json(json!({"ok": true, "review": review})).into_response(),
        Err(err) => Json(json!({"ok": false, "error": format!("{}: {}", err.code, err.message)}))
            .into_response(),
    }
}

/// Installs a locally supplied node bundle.
///
/// The same review runs again here, so a violation refuses the install even if
/// the caller never asked for a review.
async fn api_install_local_node_bundle(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Json(req): Json<LocalNodeBundleRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::SettingsWrite,
    ) {
        return response;
    }
    match state.platform.hub.install_local_node_bundle(
        &owner,
        &project,
        &req.package_id,
        &req.version,
        &req.target_folder,
        req.artifact,
    ) {
        Ok(result) => Json(json!({"ok": true, "result": result})).into_response(),
        Err(err) => Json(json!({"ok": false, "error": format!("{}: {}", err.code, err.message)}))
            .into_response(),
    }
}

async fn api_uninstall_node_package(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, kind)): Path<(String, String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::SettingsWrite,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::DELETE,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    // Reject uninstalling official nodes (native + embedded composites).
    if state.platform.node_registry.is_official(&kind) {
        return Json(json!({
            "ok": false,
            "error": "cannot uninstall official node kinds"
        }))
        .into_response();
    }
    match state
        .platform
        .node_registry
        .uninstall_package(&owner, &project, &kind)
    {
        Ok(()) => Json(json!({"ok": true})).into_response(),
        Err(e) => Json(json!({
            "ok": false,
            "error": format!("{}: {}", e.code, e.message)
        }))
        .into_response(),
    }
}

async fn api_node_icon(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, kind)): Path<(String, String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::SettingsRead,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::GET,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    // 1. Check installed + embedded composite node packages first.
    if let Some(svg_bytes) = state
        .platform
        .node_registry
        .load_icon_any(&owner, &project, &kind)
    {
        return (
            [
                (CONTENT_TYPE, "image/svg+xml"),
                (CACHE_CONTROL, "public, max-age=31536000, immutable"),
            ],
            svg_bytes,
        )
            .into_response();
    }

    // 2. Fall back to embedded builtin icon.
    let icon_path = format!("zebflow/{}.svg", kind);
    if let Some(bytes) = platform_node_icon_asset(&icon_path) {
        return (
            [
                (CONTENT_TYPE, "image/svg+xml"),
                (CACHE_CONTROL, "public, max-age=31536000, immutable"),
            ],
            bytes.to_vec(),
        )
            .into_response();
    }

    StatusCode::NOT_FOUND.into_response()
}

// ── Admin DB endpoints ──────────────────────────────────────────────────────

fn require_superadmin(state: &PlatformAppState, headers: &HeaderMap) -> Result<(), Response> {
    let Some(owner) = session_owner(state, headers) else {
        return Err(StatusCode::UNAUTHORIZED.into_response());
    };
    if !is_superadmin_owner(state, &owner) {
        return Err(StatusCode::FORBIDDEN.into_response());
    }
    Ok(())
}

fn is_superadmin_owner(state: &PlatformAppState, owner: &str) -> bool {
    state
        .platform
        .users
        .get_user(owner)
        .ok()
        .flatten()
        .map(|u| u.role == "superadmin")
        .unwrap_or(false)
}

fn platform_hub_source_owner(state: &PlatformAppState) -> Option<String> {
    state
        .platform
        .users
        .list_users()
        .ok()?
        .into_iter()
        .find(|user| user.role == "superadmin")
        .map(|user| user.owner)
}

fn visible_platform_hub_sources(
    state: &PlatformAppState,
    session_owner: &str,
) -> Result<(String, Vec<crate::platform::model::PlatformHubRepository>), PlatformError> {
    let source_owner =
        platform_hub_source_owner(state).unwrap_or_else(|| slug_segment(session_owner));
    state
        .platform
        .hub
        .ensure_default_platform_repository(&source_owner)?;
    let mut items = state
        .platform
        .hub
        .list_platform_repositories(&source_owner)?;
    if !is_superadmin_owner(state, session_owner) {
        items.retain(|item| item.visibility == "public");
    }
    Ok((source_owner, items))
}

fn require_owner_or_superadmin(
    state: &PlatformAppState,
    headers: &HeaderMap,
    owner: &str,
) -> Result<String, Response> {
    let Some(session) = session_owner(state, headers) else {
        return Err(StatusCode::UNAUTHORIZED.into_response());
    };
    let session_slug = crate::platform::model::slug_segment(&session);
    let owner_slug = crate::platform::model::slug_segment(owner);
    if session_slug == owner_slug || is_superadmin_owner(state, &session) {
        return Ok(session);
    }
    Err(StatusCode::FORBIDDEN.into_response())
}

async fn api_admin_db_list_collections(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
) -> Response {
    if let Err(r) = require_superadmin(&state, &headers) {
        return r;
    }
    match state.platform.data.admin_list_collections() {
        Ok(collections) => Json(json!({
            "ok": true,
            "collections": collections.into_iter().map(|(name, count)| json!({"name": name, "count": count})).collect::<Vec<_>>()
        }))
        .into_response(),
        Err(err) => internal_error(err),
    }
}

// ── Credential encryption keyring ───────────────────────────────────────────
//
// `rotate` and `rekey` are HTTP and not CLI, and the Credential contract is
// what decides it: both are "two operations, **both online**". `interface.md`
// §3a freezes the CLI at server modes, project install, and the `admin` noun,
// and §3's Group 3 says what that noun is for — "offline maintenance run when
// the server will not start". An operation that requires a running instance is
// not that, and minting a cluster join token reached the same conclusion for
// the same reason.
//
// Superadmin, instance-wide: these act on the keyring every project's
// credentials sit under, so no project scopes them.

async fn api_admin_credential_keyring(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
) -> Response {
    if let Err(r) = require_superadmin(&state, &headers) {
        return r;
    }
    match state.platform.data.credential_keyring_report() {
        Ok(report) => Json(json!({"ok": true, "keyring": report})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_admin_credential_rotate(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
) -> Response {
    if let Err(r) = require_superadmin(&state, &headers) {
        return r;
    }
    match state.platform.data.rotate_credential_key() {
        Ok(report) => Json(json!({
            "ok": true,
            "keyring": report,
            "note": concat!(
                "New writes use the new generation. Nothing was re-encrypted; earlier ",
                "generations stay for reads. POST /api/platform/credentials/reencrypt when you ",
                "want an older one to stop being needed."
            )
        }))
        .into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_admin_credential_rekey(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
) -> Response {
    if let Err(r) = require_superadmin(&state, &headers) {
        return r;
    }
    match state.platform.data.rekey_credential_keyring() {
        Ok(report) => Json(json!({
            "ok": true,
            "keyring": report,
            "note": concat!(
                "The instance key file was replaced. No credential changed. Back up the new ",
                "key file: the old one no longer opens anything."
            )
        }))
        .into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_admin_credential_reencrypt(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
) -> Response {
    if let Err(r) = require_superadmin(&state, &headers) {
        return r;
    }
    match state.platform.data.reencrypt_project_credentials() {
        Ok(report) => Json(json!({"ok": true, "sweep": report})).into_response(),
        Err(err) => internal_error(err),
    }
}

#[derive(serde::Deserialize)]
struct AdminDbQueryRequest {
    pipeline: serde_json::Value,
}

async fn api_admin_db_query(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Json(req): Json<AdminDbQueryRequest>,
) -> Response {
    if let Err(r) = require_superadmin(&state, &headers) {
        return r;
    }
    let q = json!({"pipeline": req.pipeline}).to_string();
    match state.platform.data.admin_raw_query(&q) {
        Ok(rows) => {
            let count = rows.len();
            Json(json!({"ok": true, "rows": rows, "count": count})).into_response()
        }
        Err(err) => internal_error(err),
    }
}

async fn api_admin_db_get_node(
    State(state): State<PlatformAppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Response {
    if let Err(r) = require_superadmin(&state, &headers) {
        return r;
    }
    match state.platform.data.admin_get_node(&slug) {
        Ok(Some(raw)) => {
            let v: serde_json::Value =
                serde_json::from_str(&raw).unwrap_or(serde_json::Value::String(raw));
            Json(json!({"ok": true, "slug": slug, "node": v})).into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({"ok": false, "error": {"code": "NODE_NOT_FOUND", "message": format!("Node not found: {slug}")}})),
        )
            .into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_admin_db_delete_node(
    State(state): State<PlatformAppState>,
    Path(slug): Path<String>,
    headers: HeaderMap,
) -> Response {
    if let Err(r) = require_superadmin(&state, &headers) {
        return r;
    }
    match state.platform.data.admin_delete_node(&slug) {
        Ok(deleted) => Json(json!({"ok": true, "deleted": deleted, "slug": slug})).into_response(),
        Err(err) => internal_error(err),
    }
}

// ────────────────────────────────────────────────────────────────────────────

async fn api_list_users(State(state): State<PlatformAppState>, headers: HeaderMap) -> Response {
    if let Err(response) = require_superadmin(&state, &headers) {
        return response;
    }
    match state.platform.users.list_users() {
        Ok(items) => Json(json!({"ok": true, "items": items})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_create_user(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Json(req): Json<CreateUserRequest>,
) -> Response {
    if let Err(response) = require_superadmin(&state, &headers) {
        return response;
    }
    match state.platform.users.create_user(&req) {
        Ok(user) => Json(json!({"ok": true, "user": user})).into_response(),
        Err(err) if err.code == "PLATFORM_USER_EXISTS" => (
            StatusCode::CONFLICT,
            Json(json!({"ok": false, "error": {"code": err.code, "message": err.message}})),
        )
            .into_response(),
        Err(err) => internal_error(err),
    }
}

/// Request body for `DELETE /api/platform/users/{owner}`.
#[derive(serde::Deserialize)]
struct DeleteUserRequest {
    /// Must exactly match the user slug as a confirmation.
    username: String,
    /// The *caller's* password — re-verified before deletion.
    password: String,
    /// Where the deleted user's projects go. `None` purges them instead,
    /// project by project, with every refusal `delete_project` makes.
    #[serde(default)]
    successor: Option<String>,
}

/// `DELETE /api/platform/users/{owner}` — removes a person from the instance.
///
/// n8n's shape: you cannot delete a person without deciding what happens to
/// what they owned. `successor` transfers every project through the same
/// machinery as the transfer-owner route; `null` purges them, and purging
/// refuses any project that other people are members of — deletion is never
/// the path by which someone else's work disappears.
///
/// Refusals, each a distinct answer:
/// - not superadmin → 403; wrong typed-back name → 400; wrong password → 401
/// - deleting yourself → 400. One rule, and it guarantees a superadmin
///   survives every deletion: the caller.
/// - published hub versions → 409 naming the packages. `package@version` is
///   immutable and someone else's `zeb.lock` points at it, so the publisher
///   record has to outlive the person. Transfer the publisher first.
/// - purge of a project with other members → 409. Transfer instead.
/// - purge of a project hosting a hub authority → 409, from `delete_project`.
async fn api_delete_user(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path(owner): Path<String>,
    Json(req): Json<DeleteUserRequest>,
) -> Response {
    if let Err(response) = require_superadmin(&state, &headers) {
        return response;
    }
    let Some(caller) = session_owner(&state, &headers) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let target = crate::platform::model::slug_segment(&owner);
    let confirmed = crate::platform::model::slug_segment(&req.username);
    if confirmed != target || target.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": "Username does not match"})),
        )
            .into_response();
    }
    if caller == target {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "ok": false,
                "error": "You cannot delete your own account. Another superadmin has to do it.",
            })),
        )
            .into_response();
    }
    match state.platform.users.authenticate(&caller, &req.password) {
        Ok(true) => {}
        Ok(false) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"ok": false, "error": "Incorrect password"})),
            )
                .into_response();
        }
        Err(e) => return internal_error(e),
    }
    let target_user = match state.platform.data.get_user_auth(&target) {
        Ok(Some(user)) => user,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"ok": false, "error": "User not found"})),
            )
                .into_response();
        }
        Err(e) => return internal_error(e),
    };
    let target_user_id = target_user.profile.user_id.clone();

    // A published version is immutable and other installations point at it,
    // so its publisher record must outlive the publisher.
    match state.platform.data.list_hub_asset_packages() {
        Ok(packages) => {
            let published: Vec<String> = packages
                .into_iter()
                .filter(|p| p.publisher_owner == target)
                .map(|p| p.package_id)
                .collect();
            if !published.is_empty() {
                return (
                    StatusCode::CONFLICT,
                    Json(json!({
                        "ok": false,
                        "error": format!(
                            "{target} published hub packages that other installations depend on: \
                             {}. Transfer the publisher identity first.",
                            published.join(", ")
                        ),
                        "packages": published,
                    })),
                )
                    .into_response();
            }
        }
        Err(e) => return internal_error(e),
    }

    let owned = match state.platform.data.list_projects(&target) {
        Ok(projects) => projects,
        Err(e) => return internal_error(e),
    };

    if let Some(successor) = req.successor.as_deref() {
        let successor = crate::platform::model::slug_segment(successor);
        if successor == target || successor.is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok": false, "error": "successor must be a different user"})),
            )
                .into_response();
        }
        for project in &owned {
            if let Err(err) = state.platform.projects.transfer_project_owner(
                &target,
                &project.project,
                &successor,
            ) {
                let status = match err.code {
                    "PLATFORM_TRANSFER_CONFLICT" => StatusCode::CONFLICT,
                    "PLATFORM_TRANSFER_INVALID" => StatusCode::BAD_REQUEST,
                    _ => return internal_error(err),
                };
                return (
                    status,
                    Json(json!({
                        "ok": false,
                        "error": format!("transferring {}: {}", project.project, err.message),
                    })),
                )
                    .into_response();
            }
            let _ = state
                .platform
                .pipeline_runtime
                .refresh_project(&successor, &project.project);
            let _ = state
                .platform
                .mcp_sessions
                .revoke_for_project(&target, &project.project);
        }
    } else {
        // Purging: refuse first, delete second, so a refusal halfway through
        // never leaves the account half-gone.
        for project in &owned {
            match state
                .platform
                .data
                .list_project_members(&target, &project.project)
            {
                Ok(members) => {
                    if members.iter().any(|m| m.user_id != target) {
                        return (
                            StatusCode::CONFLICT,
                            Json(json!({
                                "ok": false,
                                "error": format!(
                                    "project {} has other members; transfer it with \
                                     `successor` instead of purging their work",
                                    project.project
                                ),
                            })),
                        )
                            .into_response();
                    }
                }
                Err(e) => return internal_error(e),
            }
        }
        for project in &owned {
            if let Err(e) = state
                .platform
                .data
                .delete_project(&target, &project.project)
            {
                if e.code == "PLATFORM_PROJECT_HOSTS_HUB_AUTHORITY" {
                    return (
                        StatusCode::CONFLICT,
                        Json(json!({"ok": false, "error": e.message})),
                    )
                        .into_response();
                }
                return internal_error(e);
            }
            let project_root = state
                .platform
                .config
                .data_root
                .join("users")
                .join(&target)
                .join(&project.project);
            if project_root.exists() {
                if let Err(e) = std::fs::remove_dir_all(&project_root) {
                    eprintln!("WARN: Failed to remove project dir {project_root:?}: {e}");
                }
            }
            let _ = state
                .platform
                .mcp_sessions
                .revoke_for_project(&target, &project.project);
        }
    }

    if let Err(e) = state.platform.data.delete_user(&target, &target_user_id) {
        return internal_error(e);
    }

    // Their directory goes too. Transferred projects have already moved out,
    // purged ones are already deleted — what remains is an empty shell that
    // would otherwise read as a half-deleted account forever.
    let user_root = state.platform.config.data_root.join("users").join(&target);
    if user_root.exists() {
        if let Err(e) = std::fs::remove_dir_all(&user_root) {
            eprintln!("WARN: Failed to remove user dir {user_root:?}: {e}");
        }
    }

    // Their live browser sessions die with the account.
    state
        .sessions
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .retain(|_, session| session.owner != target);
    // Transferred projects moved on disk, and template cache dependency paths
    // are absolute, so they point into the old owner's directory.
    if let Ok(mut cache) = state.template_cache.write() {
        cache.clear();
    }

    Json(json!({
        "ok": true,
        "projects": owned.len(),
        "resolution": if req.successor.is_some() { "transferred" } else { "purged" },
    }))
    .into_response()
}

async fn api_get_profile(State(state): State<PlatformAppState>, headers: HeaderMap) -> Response {
    let Some(owner) = session_owner(&state, &headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"ok": false, "error": "login required"})),
        )
            .into_response();
    };
    match state.platform.users.get_user(&owner) {
        Ok(Some(user)) => {
            let identity = state
                .platform
                .git_identity
                .resolve_for_actor(Some(&owner), &owner);
            Json(json!({"ok": true, "user": user, "effective_git_identity": identity}))
                .into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({"ok": false, "error": "user not found"})),
        )
            .into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_update_profile(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Json(req): Json<UpdateUserSettingsRequest>,
) -> Response {
    let Some(owner) = session_owner(&state, &headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"ok": false, "error": "login required"})),
        )
            .into_response();
    };
    match state.platform.users.update_user_settings(&owner, &req) {
        Ok(user) => {
            let identity = state
                .platform
                .git_identity
                .resolve_for_actor(Some(&owner), &owner);
            Json(json!({"ok": true, "user": user, "effective_git_identity": identity}))
                .into_response()
        }
        Err(err) if err.code == "PLATFORM_USER_NOT_FOUND" => (
            StatusCode::NOT_FOUND,
            Json(json!({"ok": false, "error": {"code": err.code, "message": err.message}})),
        )
            .into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_list_projects(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path(owner): Path<String>,
) -> Response {
    if let Err(response) = require_owner_or_superadmin(&state, &headers, &owner) {
        return response;
    }
    match state.platform.projects.list_projects(&owner) {
        Ok(items) => Json(json!({"ok": true, "items": items})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_create_project(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path(owner): Path<String>,
    Json(req): Json<CreateProjectRequest>,
) -> Response {
    if let Err(response) = require_owner_or_superadmin(&state, &headers, &owner) {
        return response;
    }
    let runtime = req.runtime.clone();
    match state
        .platform
        .projects
        .create_or_update_project(&owner, &req)
    {
        Ok((project, layout)) => {
            if let Err(err) = state
                .platform
                .projects
                .write_starter_files(&owner, &project.project)
            {
                return internal_error(err);
            }
            match finalize_project_runtime_setup(&state, &owner, &project.project, &runtime).await
            {
                Ok(placement) => Json(
                    json!({"ok": true, "project": project, "layout": layout, "placement": placement}),
                )
                .into_response(),
                Err(err) => internal_error(err),
            }
        }
        Err(err) => internal_error(err),
    }
}

async fn api_list_platform_hub_repositories(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path(owner): Path<String>,
) -> Response {
    let Some(session) = session_owner(&state, &headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"ok": false, "error": "login required"})),
        )
            .into_response();
    };
    if session != owner {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"ok": false, "error": "forbidden"})),
        )
            .into_response();
    }
    if let Err(err) = state
        .platform
        .hub
        .ensure_default_platform_repository(&owner)
    {
        return internal_error(err);
    }
    match state.platform.hub.list_platform_repositories(&owner) {
        Ok(items) => Json(json!({
            "ok": true,
            "items": items.into_iter().map(platform_hub_repository_json).collect::<Vec<_>>()
        }))
        .into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_list_platform_hub_sources(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
) -> Response {
    let Some(session) = session_owner(&state, &headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"ok": false, "error": "login required"})),
        )
            .into_response();
    };
    match visible_platform_hub_sources(&state, &session) {
        Ok((_source_owner, items)) => Json(json!({
            "ok": true,
            "items": items.into_iter().map(platform_hub_repository_json).collect::<Vec<_>>()
        }))
        .into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_get_platform_hub_service(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = require_superadmin(&state, &headers) {
        return response;
    }
    match state.platform.hub.get_default_service_instance() {
        Ok(service) => Json(json!({"ok": true, "service": service})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_configure_platform_hub_service(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Json(req): Json<ConfigurePlatformHubServiceRequest>,
) -> Response {
    let Some(session_owner) = session_owner(&state, &headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"ok": false, "error": "login required"})),
        )
            .into_response();
    };
    if !is_superadmin_owner(&state, &session_owner) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"ok": false, "error": "superadmin required"})),
        )
            .into_response();
    }
    match state
        .platform
        .users
        .authenticate(&session_owner, &req.password)
    {
        Ok(true) => {}
        Ok(false) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"ok": false, "error": "Incorrect password"})),
            )
                .into_response();
        }
        Err(err) => return internal_error(err),
    }
    match state.platform.hub.ensure_default_service_instance(
        &req.host_office_id,
        &req.public_base_url,
        req.enabled,
    ) {
        Ok(service) => Json(json!({"ok": true, "service": service})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_upsert_platform_hub_source(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Json(req): Json<UpsertHubRepositoryRequest>,
) -> Response {
    if let Err(response) = require_superadmin(&state, &headers) {
        return response;
    }
    let Some(source_owner) = platform_hub_source_owner(&state) else {
        return internal_error(PlatformError::new(
            "PLATFORM_USER_NOT_FOUND",
            "superadmin user not found",
        ));
    };
    match state.platform.hub.upsert_platform_repository(
        &source_owner,
        &req.repository_id,
        &req.title,
        &req.base_url,
        &req.remote_owner,
        &req.remote_project,
        &req.read_token,
        &req.kind,
        req.priority,
        &req.visibility,
        req.enabled,
    ) {
        Ok(repository) => Json(json!({
            "ok": true,
            "repository": platform_hub_repository_json(repository)
        }))
        .into_response(),
        Err(err) => hub_api_error(err),
    }
}

async fn api_upsert_platform_hub_repository(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path(owner): Path<String>,
    Json(req): Json<UpsertHubRepositoryRequest>,
) -> Response {
    let Some(session) = session_owner(&state, &headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"ok": false, "error": "login required"})),
        )
            .into_response();
    };
    if session != owner {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"ok": false, "error": "forbidden"})),
        )
            .into_response();
    }
    match state.platform.hub.upsert_platform_repository(
        &owner,
        &req.repository_id,
        &req.title,
        &req.base_url,
        &req.remote_owner,
        &req.remote_project,
        &req.read_token,
        &req.kind,
        req.priority,
        &req.visibility,
        req.enabled,
    ) {
        Ok(repository) => Json(json!({
            "ok": true,
            "repository": platform_hub_repository_json(repository)
        }))
        .into_response(),
        Err(err) => hub_api_error(err),
    }
}

async fn api_delete_platform_hub_source(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path(repository_id): Path<String>,
) -> Response {
    if let Err(response) = require_superadmin(&state, &headers) {
        return response;
    }
    let Some(source_owner) = platform_hub_source_owner(&state) else {
        return internal_error(PlatformError::new(
            "PLATFORM_USER_NOT_FOUND",
            "superadmin user not found",
        ));
    };
    match state
        .platform
        .hub
        .delete_platform_repository(&source_owner, &repository_id)
    {
        Ok(()) => Json(json!({"ok": true})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_delete_platform_hub_repository(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, repository_id)): Path<(String, String)>,
) -> Response {
    let Some(session) = session_owner(&state, &headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"ok": false, "error": "login required"})),
        )
            .into_response();
    };
    if session != owner {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"ok": false, "error": "forbidden"})),
        )
            .into_response();
    }
    match state
        .platform
        .hub
        .delete_platform_repository(&owner, &repository_id)
    {
        Ok(()) => Json(json!({"ok": true})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_list_platform_hub_apps(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
) -> Response {
    let Some(session) = session_owner(&state, &headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"ok": false, "error": "login required"})),
        )
            .into_response();
    };
    let (_source_owner, sources) = match visible_platform_hub_sources(&state, &session) {
        Ok(value) => value,
        Err(err) => return internal_error(err),
    };
    // Every source is named in the answer, in resolution order, whether or not
    // it answered. A client that only saw the rows could not tell "nobody
    // publishes it" from "the source that does is unreachable", and one error
    // naming both sources is what `zeb install` prints when neither has it.
    let listing = state
        .platform
        .hub
        .fetch_platform_remote_app_rows_from_repositories(sources)
        .await;
    Json(json!({
        "ok": true,
        "items": listing.items,
        "sources": listing.sources,
    }))
    .into_response()
}

async fn api_list_platform_hub_assets(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path(owner): Path<String>,
) -> Response {
    let Some(session) = session_owner(&state, &headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"ok": false, "error": "login required"})),
        )
            .into_response();
    };
    if session != owner {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"ok": false, "error": "forbidden"})),
        )
            .into_response();
    }
    if let Err(err) = state
        .platform
        .hub
        .ensure_default_platform_repository(&owner)
    {
        return internal_error(err);
    }
    match state
        .platform
        .hub
        .fetch_platform_remote_app_rows(&owner)
        .await
    {
        Ok(listing) => Json(json!({
            "ok": true,
            "items": listing.items,
            "sources": listing.sources,
        }))
        .into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_install_platform_hub_app(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Json(req): Json<InstallPlatformHubProjectRequest>,
) -> Response {
    let Some(session) = session_owner(&state, &headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"ok": false, "error": "login required"})),
        )
            .into_response();
    };
    let (source_owner, sources) = match visible_platform_hub_sources(&state, &session) {
        Ok(value) => value,
        Err(err) => return internal_error(err),
    };
    if !sources
        .iter()
        .any(|item| item.repository_id == req.repository_id && item.enabled)
    {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"ok": false, "error": "hub source is not visible"})),
        )
            .into_response();
    }
    match state
        .platform
        .hub
        .install_remote_project_from_platform_source(
            &source_owner,
            &session,
            &req.repository_id,
            &req.package_id,
            &req.version,
            req.scope(),
        )
        .await
    {
        Ok(result) => {
            let installed_owner = result.owner.clone();
            let installed_project = result.project.clone();
            // What the scope left out travels with the answer, so a partial
            // install is never reported as a whole one.
            let install = serde_json::to_value(&result).unwrap_or(Value::Null);
            if let Err(err) = state
                .platform
                .cluster_runtime_sync
                .refresh_local_repo_state(&installed_owner, &installed_project)
            {
                return internal_error(err);
            }
            match state
                .platform
                .projects
                .get_project(&installed_owner, &installed_project)
            {
                Ok(Some(project)) => match home_project_card_json(&state, &session, &project) {
                    Ok(project) => Json(json!({
                        "ok": true,
                        "owner": installed_owner,
                        "project": project,
                        "install": install,
                    }))
                    .into_response(),
                    Err(err) => internal_error(err),
                },
                Ok(None) => Json(json!({
                    "ok": true,
                    "owner": installed_owner,
                    "project_slug": installed_project,
                    "install": install,
                }))
                .into_response(),
                Err(err) => internal_error(err),
            }
        }
        Err(err) => hub_api_error(err),
    }
}

/// Reports what installing a platform-scope project bundle would do.
///
/// Same auth and same visibility check as the install it precedes, because a
/// review that reads a repository the caller cannot install from would disclose
/// a package they are not allowed to see.
async fn api_review_platform_hub_app(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Json(req): Json<InstallPlatformHubProjectRequest>,
) -> Response {
    let Some(session) = session_owner(&state, &headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"ok": false, "error": "login required"})),
        )
            .into_response();
    };
    let (source_owner, sources) = match visible_platform_hub_sources(&state, &session) {
        Ok(value) => value,
        Err(err) => return internal_error(err),
    };
    if !sources
        .iter()
        .any(|item| item.repository_id == req.repository_id && item.enabled)
    {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"ok": false, "error": "hub source is not visible"})),
        )
            .into_response();
    }
    match state
        .platform
        .hub
        .review_remote_project_from_platform_source(
            &source_owner,
            &session,
            &req.repository_id,
            &req.package_id,
            &req.version,
            req.scope(),
        )
        .await
    {
        Ok(review) => Json(json!({"ok": true, "review": review})).into_response(),
        Err(err) => hub_api_error(err),
    }
}

/// Reports what installing a project bundle into `owner`'s account would do.
async fn api_review_platform_hub_project(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path(owner): Path<String>,
    Json(req): Json<InstallPlatformHubProjectRequest>,
) -> Response {
    let Some(session) = session_owner(&state, &headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"ok": false, "error": "login required"})),
        )
            .into_response();
    };
    if session != owner {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"ok": false, "error": "forbidden"})),
        )
            .into_response();
    }
    match state
        .platform
        .hub
        .review_remote_project_from_platform_repository(
            &owner,
            &req.repository_id,
            &req.package_id,
            &req.version,
            req.scope(),
        )
        .await
    {
        Ok(review) => Json(json!({"ok": true, "review": review})).into_response(),
        Err(err) => hub_api_error(err),
    }
}

async fn api_install_platform_hub_project(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path(owner): Path<String>,
    Json(req): Json<InstallPlatformHubProjectRequest>,
) -> Response {
    let Some(session) = session_owner(&state, &headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"ok": false, "error": "login required"})),
        )
            .into_response();
    };
    if session != owner {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"ok": false, "error": "forbidden"})),
        )
            .into_response();
    }
    match state
        .platform
        .hub
        .install_remote_project_from_platform_repository(
            &owner,
            &req.repository_id,
            &req.package_id,
            &req.version,
            req.scope(),
        )
        .await
    {
        Ok(result) => {
            let installed_owner = result.owner.clone();
            let installed_project = result.project.clone();
            let install = serde_json::to_value(&result).unwrap_or(Value::Null);
            if let Err(err) = state
                .platform
                .cluster_runtime_sync
                .refresh_local_repo_state(&installed_owner, &installed_project)
            {
                return internal_error(err);
            }
            match state
                .platform
                .projects
                .get_project(&installed_owner, &installed_project)
            {
                Ok(Some(project)) => match home_project_card_json(&state, &owner, &project) {
                    Ok(project) => Json(json!({
                        "ok": true,
                        "project": project,
                        "install": install,
                    }))
                    .into_response(),
                    Err(err) => internal_error(err),
                },
                Ok(None) => internal_error(PlatformError::new(
                    "HUB_INSTALL",
                    "installed project missing after install",
                )),
                Err(err) => internal_error(err),
            }
        }
        Err(err) => hub_api_error(err),
    }
}

async fn api_list_platform_hub_publishers(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = require_superadmin(&state, &headers) {
        return response;
    }
    let Some(source_owner) = platform_hub_source_owner(&state) else {
        return internal_error(PlatformError::new(
            "PLATFORM_USER_NOT_FOUND",
            "superadmin user not found",
        ));
    };
    match state
        .platform
        .hub
        .list_publishers(&source_owner, "platform")
    {
        Ok(items) => Json(json!({
            "ok": true,
            "items": items.into_iter().map(hub_publisher_json).collect::<Vec<_>>()
        }))
        .into_response(),
        Err(err) if err.code == "HUB_SERVICE_DISABLED" => {
            Json(json!({"ok": true, "items": []})).into_response()
        }
        Err(err) => internal_error(err),
    }
}

async fn api_upsert_platform_hub_publisher(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Json(req): Json<UpsertHubPublisherRequest>,
) -> Response {
    if let Err(response) = require_superadmin(&state, &headers) {
        return response;
    }
    let Some(source_owner) = platform_hub_source_owner(&state) else {
        return internal_error(PlatformError::new(
            "PLATFORM_USER_NOT_FOUND",
            "superadmin user not found",
        ));
    };
    match state.platform.hub.upsert_publisher(
        &source_owner,
        "platform",
        &req.publisher_id,
        &req.display_name,
        &req.publisher_url,
        &req.email,
        &req.description,
        &req.icon_url,
        &req.website_url,
        req.enabled,
        req.can_read,
        req.can_publish,
        req.can_manage,
        req.max_packages,
        req.max_package_bytes,
        req.max_media_files,
        req.max_image_bytes,
    ) {
        Ok(item) => {
            Json(json!({"ok": true, "publisher": hub_publisher_json(item)})).into_response()
        }
        Err(err) => internal_error(err),
    }
}

async fn api_delete_platform_hub_publisher(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path(publisher_id): Path<String>,
) -> Response {
    if let Err(response) = require_superadmin(&state, &headers) {
        return response;
    }
    let Some(source_owner) = platform_hub_source_owner(&state) else {
        return internal_error(PlatformError::new(
            "PLATFORM_USER_NOT_FOUND",
            "superadmin user not found",
        ));
    };
    match state
        .platform
        .hub
        .delete_publisher(&source_owner, "platform", &publisher_id)
    {
        Ok(()) => Json(json!({"ok": true})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_list_platform_hub_tokens(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = require_superadmin(&state, &headers) {
        return response;
    }
    match state.platform.hub.list_all_tokens() {
        Ok(items) => Json(json!({
            "ok": true,
            "items": items.into_iter().map(hub_token_json).collect::<Vec<_>>()
        }))
        .into_response(),
        Err(err) if err.code == "HUB_SERVICE_DISABLED" => {
            Json(json!({"ok": true, "items": []})).into_response()
        }
        Err(err) => internal_error(err),
    }
}

async fn api_create_platform_hub_token(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Json(req): Json<CreatePlatformHubTokenRequest>,
) -> Response {
    if let Err(response) = require_superadmin(&state, &headers) {
        return response;
    }
    let Some(source_owner) = platform_hub_source_owner(&state) else {
        return internal_error(PlatformError::new(
            "PLATFORM_USER_NOT_FOUND",
            "superadmin user not found",
        ));
    };
    let owner = if req.owner.trim().is_empty() {
        source_owner.clone()
    } else {
        req.owner
    };
    let project = if req.project.trim().is_empty() {
        "platform".to_string()
    } else {
        req.project
    };
    match state.platform.hub.create_token(
        &owner,
        &project,
        &CreateHubTokenRequest {
            publisher_id: req.publisher_id,
            title: req.title,
            scopes: req.scopes,
            expires_at: req.expires_at,
        },
    ) {
        Ok((token, token_value)) => {
            Json(json!({"ok": true, "token": hub_token_json(token), "token_value": token_value}))
                .into_response()
        }
        Err(err) => internal_error(err),
    }
}

async fn api_delete_platform_hub_token(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path(token_id): Path<String>,
) -> Response {
    if let Err(response) = require_superadmin(&state, &headers) {
        return response;
    }
    match state.platform.hub.revoke_token_any(&token_id) {
        Ok(()) => Json(json!({"ok": true})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_list_platform_hub_grants(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = require_superadmin(&state, &headers) {
        return response;
    }
    let Some(source_owner) = platform_hub_source_owner(&state) else {
        return internal_error(PlatformError::new(
            "PLATFORM_USER_NOT_FOUND",
            "superadmin user not found",
        ));
    };
    match state.platform.hub.list_access_grants(&source_owner) {
        Ok(items) => Json(json!({
            "ok": true,
            "items": items.into_iter().map(hub_access_grant_json).collect::<Vec<_>>()
        }))
        .into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_upsert_platform_hub_grant(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Json(req): Json<UpsertPlatformHubGrantRequest>,
) -> Response {
    if let Err(response) = require_superadmin(&state, &headers) {
        return response;
    }
    let Some(source_owner) = platform_hub_source_owner(&state) else {
        return internal_error(PlatformError::new(
            "PLATFORM_USER_NOT_FOUND",
            "superadmin user not found",
        ));
    };
    match state.platform.hub.upsert_access_grant(
        &source_owner,
        &req.repository_id,
        &req.grant_scope,
        &req.target_owner,
        &req.target_project,
        req.can_read,
        req.can_publish,
        req.can_manage,
        req.enabled,
    ) {
        Ok(grant) => Json(json!({
            "ok": true,
            "grant": hub_access_grant_json(grant)
        }))
        .into_response(),
        Err(err) => hub_api_error(err),
    }
}

async fn api_delete_platform_hub_grant(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path(grant_id): Path<String>,
) -> Response {
    if let Err(response) = require_superadmin(&state, &headers) {
        return response;
    }
    match state.platform.hub.delete_access_grant(&grant_id) {
        Ok(()) => Json(json!({"ok": true})).into_response(),
        Err(err) => hub_api_error(err),
    }
}

async fn api_cluster_workers(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = require_superadmin(&state, &headers) {
        return response;
    }
    match state.platform.cluster_registry.snapshot() {
        Ok(snapshot) => Json(json!({ "ok": true, "workers": snapshot.workers })).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_project_runtime_status(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::SettingsRead,
    ) {
        return response;
    }
    let placement = match state.platform.cluster_placement.get(&owner, &project) {
        Ok(value) => value,
        Err(err) => return internal_error(err),
    };
    let workers = match state.platform.cluster_registry.snapshot() {
        Ok(snapshot) => snapshot.workers,
        Err(err) => return internal_error(err),
    };
    Json(json!({
        "ok": true,
        "placement": placement,
        "summary": state.platform.cluster_placement.describe(placement.as_ref()),
        "workers": workers,
    }))
    .into_response()
}

/// Verify the join token an office presented, and that it named *this* office.
///
/// Two checks, not one. The token says which office is presenting; the request
/// says which office it claims to be. A token minted for office A presented by
/// office B passes the first and fails the second, which is the whole point of
/// putting the office id inside the token (`offices.md` §8).
fn require_registering_office(
    state: &PlatformAppState,
    headers: &HeaderMap,
    node_id: &str,
) -> Result<String, Response> {
    let presented = headers
        .get(INTERNAL_CLUSTER_TOKEN_HEADER)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    let verified = state
        .platform
        .cluster_join_tokens
        .verify_office_token(presented)
        .map_err(|err| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "ok": false,
                    "error": { "code": err.code, "message": err.message }
                })),
            )
                .into_response()
        })?;
    let claimed = slug_segment(node_id);
    if claimed != verified.office_id {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({
                "ok": false,
                "error": {
                    "code": "CLUSTER_JOIN_TOKEN_OFFICE_MISMATCH",
                    "message": format!(
                        "this join token was issued to office '{}', but the request registers \
                         office '{}'. Mint a token for '{}' on the controller \
                         (POST /api/platform/cluster/join-tokens).",
                        verified.office_id, claimed, claimed
                    )
                }
            })),
        )
            .into_response());
    }
    Ok(verified.office_id)
}

async fn api_internal_cluster_register_worker(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Json(req): Json<ClusterWorkerRegisterRequest>,
) -> Response {
    let office_id = match require_registering_office(&state, &headers, &req.node_id) {
        Ok(office_id) => office_id,
        Err(response) => return response,
    };
    match state.platform.cluster_registry.register_worker(&req) {
        Ok(worker) => Json(ClusterWorkerRegisterResponse {
            ok: true,
            worker,
            // The office's half of the mutual proof: an HMAC over the nonce it
            // sent, keyed by this office's stored secret digest. A host that
            // never held the token cannot produce it, which is how the office
            // tells a real controller from anything that answers the URL.
            proof: state
                .platform
                .cluster_join_tokens
                .registration_proof_for(&office_id, &req.nonce)
                .unwrap_or_default(),
        })
        .into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_internal_cluster_worker_heartbeat(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Json(req): Json<ClusterWorkerHeartbeatRequest>,
) -> Response {
    // Checked on every heartbeat, not only at registration: revoking one
    // office's token has to stop that office, and an already-registered office
    // that kept beating on a revoked token would make revocation cosmetic.
    if let Err(response) = require_registering_office(&state, &headers, &req.node_id) {
        return response;
    }
    match state.platform.cluster_registry.heartbeat(&req) {
        Ok(heartbeat) => Json(heartbeat).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_cluster_join_tokens(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = require_superadmin(&state, &headers) {
        return response;
    }
    match state.platform.cluster_join_tokens.list() {
        Ok(tokens) => Json(json!({ "ok": true, "tokens": tokens })).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_cluster_mint_join_token(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Json(req): Json<ClusterJoinTokenMintRequest>,
) -> Response {
    if let Err(response) = require_superadmin(&state, &headers) {
        return response;
    }
    match state.platform.cluster_join_tokens.mint(&req) {
        Ok(minted) => (
            StatusCode::CREATED,
            Json(json!({
                "ok": true,
                "office": minted.office,
                "record": minted.record,
                "token": minted.token,
                "note": "This token is shown once. The controller stores only its digest."
            })),
        )
            .into_response(),
        Err(err) => cluster_join_token_error(err),
    }
}

async fn api_cluster_revoke_join_token(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path(office_id): Path<String>,
) -> Response {
    if let Err(response) = require_superadmin(&state, &headers) {
        return response;
    }
    match state.platform.cluster_join_tokens.revoke(&office_id) {
        Ok(record) => Json(json!({ "ok": true, "record": record })).into_response(),
        Err(err) => cluster_join_token_error(err),
    }
}

fn cluster_join_token_error(err: PlatformError) -> Response {
    let status = match err.code {
        "CLUSTER_JOIN_TOKEN_UNKNOWN" => StatusCode::NOT_FOUND,
        "CLUSTER_JOIN_TOKEN_EXISTS" => StatusCode::CONFLICT,
        "CLUSTER_JOIN_TOKEN_OFFICE_INVALID"
        | "CLUSTER_JOIN_TOKEN_MALFORMED"
        // A mint naming no address is a bad request, not a server fault
        // (`kinds/office-topology/README.md`, Rejections).
        | "CLUSTER_OFFICE_BASE_URL_REQUIRED" => StatusCode::BAD_REQUEST,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        status,
        Json(json!({
            "ok": false,
            "error": { "code": err.code, "message": err.message }
        })),
    )
        .into_response()
}

/// `POST /api/platform/cluster/offices/{office_id}/vouch` — mint one (controller).
///
/// The vouch names the *session owner*, never a value from the request. An
/// operator minting a vouch for an arbitrary identity would be impersonation
/// with a signature on it, and `offices.md` §2 asks for "one identity the
/// offices accept", not for a way to name any of them.
async fn api_cluster_mint_office_vouch(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path(office_id): Path<String>,
) -> Response {
    let Some(owner) = session_owner(&state, &headers) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    if !is_superadmin_owner(&state, &owner) {
        return StatusCode::FORBIDDEN.into_response();
    }
    match state
        .platform
        .cluster_join_tokens
        .mint_vouch(&office_id, &owner, now_ts())
    {
        Ok(vouch) => Json(json!({ "ok": true, "vouch": vouch })).into_response(),
        Err(err) => cluster_vouch_error(err),
    }
}

/// `GET /cluster/offices/{office_id}/open` — the operator's one action.
///
/// Mint and redirect, so reaching an office from the controller's directory is
/// a link rather than a copy-paste of a credential. A `GET` that mints is fine
/// here because minting writes nothing: it is a read of the controller's own
/// record plus an HMAC, and the worst a forged navigation achieves is landing
/// the operator on an office they already administer.
///
/// The vouch does travel in the URL. Short life and single use are what bound
/// that, which is why both exist rather than either alone.
async fn cluster_open_office_redirect(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path(office_id): Path<String>,
) -> Response {
    let Some(owner) = session_owner(&state, &headers) else {
        return Redirect::to(LOGIN_PATH).into_response();
    };
    if !is_superadmin_owner(&state, &owner) {
        return StatusCode::FORBIDDEN.into_response();
    }
    match state
        .platform
        .cluster_join_tokens
        .mint_vouch(&office_id, &owner, now_ts())
    {
        Ok(vouch) if !vouch.redeem_url.is_empty() => {
            Redirect::to(&vouch.redeem_url).into_response()
        }
        Ok(vouch) => cluster_vouch_error(PlatformError::new(
            "CLUSTER_OFFICE_ADDRESS_UNKNOWN",
            format!(
                "office '{}' has no base URL recorded, so there is nowhere to send you. \
                 An office's base URL is where the public reaches its projects; set it when \
                 minting its join token, or let the office advertise one by registering.",
                vouch.office_id
            ),
        )),
        Err(err) => cluster_vouch_error(err),
    }
}

/// One vouch spent, an account resolved, and the write logged.
///
/// The order matters. The vouch is verified and *spent* before any account is
/// touched, so a replay cannot reach the identity write at all. The write is
/// logged before the session is issued, so an operator reading the office's own
/// log sees every arrival, including one whose response never reached the
/// browser.
fn redeem_vouch_into_local_principal(
    state: &PlatformAppState,
    raw: &str,
) -> Result<(String, PlatformOfficeIdentityWrite), PlatformError> {
    let accepted = state
        .platform
        .cluster_join_tokens
        .redeem_vouch(raw, now_ts())?;
    let owner = slug_segment(&accepted.identity);
    if owner.is_empty() {
        return Err(PlatformError::new(
            "CLUSTER_VOUCH_IDENTITY_INVALID",
            "the vouch names an identity that is not a usable owner slug",
        ));
    }

    // `offices.md` §5's owner-mapping rule is about *joining* an office that
    // already holds projects — the moment when two account databases would
    // otherwise be merged by guess. This is a different moment: the office has
    // already accepted this controller as its authority, and nothing is being
    // merged. §4 makes the controller's identity "the normal door", and a door
    // that opens only for people already inside is not a door — the ordinary
    // case in §5 is a *fresh* office, which by definition holds nobody. §8 then
    // presumes the write outright: "Anything the controller pushes into an
    // office's accounts is logged", which only parses if it can push. §9 says
    // so in as many words — inserting an administrator is "accepted and logged
    // today", with the co-signature that would prevent it named as unbuilt.
    if let Some(existing) = state.platform.users.get_user(&owner)? {
        // An existing local account keeps the role the office gave it. The
        // controller vouches for *who* somebody is; what they may do here is
        // this office's own statement, and silently promoting a local member
        // because a vouch arrived would be the escalation §9 worries about.
        let write = state.platform.cluster_join_tokens.record_identity_write(
            &owner,
            IDENTITY_WRITE_ACTION_LINKED,
            &existing.role,
            "controller-vouch",
            &format!(
                "a controller vouch opened a session as the existing local account '{owner}'; \
                 its role '{}' was left unchanged",
                existing.role
            ),
        )?;
        return Ok((owner, write));
    }

    // The created account gets no usable password. A 32-byte random one is
    // hashed and discarded unread, so the account cannot be reached by local
    // login even after §6's break-glass re-enables local authority — a
    // controller-created principal must not also be a local backdoor with a
    // credential somebody could guess was default.
    //
    // `zeb admin break-glass` now carries the second half of that promise: it
    // refuses to name an account this log records as `created`, so the one
    // command that reopens local authority cannot also hand that account a
    // password in the same keystroke.
    let mut throwaway = [0u8; 32];
    rand::rng().fill(&mut throwaway);
    state.platform.users.create_user(&CreateUserRequest {
        owner: owner.clone(),
        password: hex::encode(throwaway),
        // §2's vouch verb is "one identity the offices accept **for
        // administration**", so an identity that exists only because the
        // controller vouched for it is created able to administer. §9 records
        // this as today's accepted behaviour and names the co-signature that
        // would constrain it as not built.
        role: "superadmin".to_string(),
        git_name: owner.clone(),
        git_email: String::new(),
    })?;
    let write = state.platform.cluster_join_tokens.record_identity_write(
        &owner,
        IDENTITY_WRITE_ACTION_CREATED,
        "superadmin",
        "controller-vouch",
        &format!(
            "a controller vouch created the local account '{owner}' with role 'superadmin'. \
             It has no usable password: the account is reachable by vouch only, never by \
             local login."
        ),
    )?;
    Ok((owner, write))
}

/// `GET /office/vouch?v=…` — spend a vouch and land in this office.
///
/// Deliberately reachable without a session: it *is* the way a session begins
/// here. On success the office issues its own ordinary session cookie — the
/// same one `POST /login` issues — so every downstream handler is unchanged and
/// none of them needs to know a vouch happened.
async fn office_vouch_redeem(
    State(state): State<PlatformAppState>,
    Query(params): Query<OfficeVouchRedeemQuery>,
) -> Response {
    match redeem_vouch_into_local_principal(&state, &params.v) {
        Ok((owner, _write)) => {
            let mut resp = Redirect::to(HOME_PATH).into_response();
            let token = issue_vouched_session(&state, &owner);
            if let Ok(value) = HeaderValue::from_str(&session_cookie_header_same_site(
                &token,
                SESSION_TTL_SECS,
                "Lax",
            )) {
                resp.headers_mut().insert(SET_COOKIE, value);
            }
            resp
        }
        Err(err) => cluster_vouch_error(err),
    }
}

/// `POST /api/office/vouch` — the same redemption for a non-browser client.
async fn api_office_vouch_redeem(
    State(state): State<PlatformAppState>,
    Json(req): Json<OfficeVouchRedeemRequest>,
) -> Response {
    match redeem_vouch_into_local_principal(&state, &req.vouch) {
        Ok((owner, write)) => {
            let mut resp = Json(json!({
                "ok": true,
                "owner": owner,
                "action": write.action,
                "identity_write": write,
            }))
            .into_response();
            let token = issue_vouched_session(&state, &owner);
            if let Ok(value) =
                HeaderValue::from_str(&session_cookie_header(&token, SESSION_TTL_SECS))
            {
                resp.headers_mut().insert(SET_COOKIE, value);
            }
            resp
        }
        Err(err) => cluster_vouch_error(err),
    }
}

/// `GET /api/platform/office/identity-writes` — `offices.md` §8's log, read locally.
///
/// Gated on this office's *own* superadmin session, not on a controller
/// header. That is the term's whole point: the record of what the controller
/// put into these accounts has to be readable by the office's operator with the
/// controller absent, or the controller is auditing itself.
async fn api_office_identity_writes(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = require_superadmin(&state, &headers) {
        return response;
    }
    match state.platform.cluster_join_tokens.list_identity_writes(200) {
        Ok(writes) => Json(json!({ "ok": true, "writes": writes })).into_response(),
        Err(err) => internal_error(err),
    }
}

/// `GET /api/platform/office/local-authority` — `offices.md` §6's record, read locally.
///
/// Gated on this office's *own* superadmin, exactly like the identity-write
/// log beside it, and for a stronger version of the same reason: the record of
/// somebody letting themselves in with host access is worth nothing if the only
/// party who can read it is the one the record was supposed to reach. §6 says
/// the use is "recorded locally *and* reported to the controller", in that
/// order, and this route is the first half.
///
/// It also answers the state question, not only the history one: `joined`,
/// `local_login_allowed`, and which recorded act is currently in force.
async fn api_office_local_authority(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = require_superadmin(&state, &headers) {
        return response;
    }
    let authority = &state.platform.local_authority;
    let join = authority.join_file();
    let in_force = match authority.break_glass_in_force() {
        Ok(value) => value,
        Err(err) => return internal_error(err),
    };
    let allowed = match authority.local_login_allowed() {
        Ok(value) => value,
        Err(err) => return internal_error(err),
    };
    match authority.list(200) {
        Ok(events) => Json(json!({
            "ok": true,
            "joined": join.is_joined(),
            "office_id": join.office_id(),
            "local_login_allowed": allowed,
            "break_glass_in_force": in_force,
            "events": events,
        }))
        .into_response(),
        Err(err) => internal_error(err),
    }
}

/// `GET /api/platform/cluster/office-break-glass` — what this controller has been told.
///
/// The controller's copy is deliberately read-only and deliberately partial: it
/// holds what offices managed to report, which is not the same set as what
/// happened. An office that broke the glass and never came back is invisible
/// here and fully visible on itself, and that asymmetry is the honest one —
/// §6 requires the office to be repairable without the controller, so the
/// controller cannot also be the authority on whether it was.
async fn api_cluster_office_break_glass(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
) -> Response {
    if let Err(response) = require_superadmin(&state, &headers) {
        return response;
    }
    match state.platform.local_authority.list(500) {
        Ok(events) => Json(json!({ "ok": true, "events": events })).into_response(),
        Err(err) => internal_error(err),
    }
}

/// `POST /api/internal/cluster/offices/break-glass` — §6's report, arriving.
///
/// Authenticated by the same join token a registration carries, and by the same
/// check: the office id is taken from the *token*, never from the body, so one
/// office cannot file a break-glass against another's name.
async fn api_internal_cluster_report_break_glass(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Json(req): Json<OfficeBreakGlassReportRequest>,
) -> Response {
    let office_id = match require_registering_office(&state, &headers, &req.office_id) {
        Ok(office_id) => office_id,
        Err(response) => return response,
    };
    let received_at = now_ts();
    let mut accepted = Vec::new();
    for event in &req.events {
        match state
            .platform
            .local_authority
            .accept_report(event, &office_id, received_at)
        {
            Ok(stored) => accepted.push(stored.event_id),
            Err(err) => return internal_error(err),
        }
    }
    Json(json!({ "ok": true, "office_id": office_id, "accepted": accepted })).into_response()
}

/// One office's unreported break-glass acts, on their way to the controller.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct OfficeBreakGlassReportRequest {
    /// Office filing the report. Checked against the token, never trusted.
    #[serde(default)]
    office_id: String,
    /// The rows, exactly as the office recorded them.
    #[serde(default)]
    events: Vec<PlatformOfficeLocalAuthorityEvent>,
}

/// `?v=` — the vouch as it arrives in a redirect from the controller.
#[derive(Debug, Clone, Deserialize)]
struct OfficeVouchRedeemQuery {
    /// The rendered vouch.
    #[serde(default)]
    v: String,
}

/// The same value posted as JSON by a client that is not a browser.
#[derive(Debug, Clone, Deserialize)]
struct OfficeVouchRedeemRequest {
    /// The rendered vouch.
    #[serde(default)]
    vouch: String,
}

fn cluster_vouch_error(err: PlatformError) -> Response {
    let status = match err.code {
        // Spent, stale, forged, or addressed elsewhere are all "this vouch does
        // not open this door", which is a 401 and not a 400: the caller may
        // retry with a fresh one.
        "CLUSTER_VOUCH_INVALID"
        | "CLUSTER_VOUCH_EXPIRED"
        | "CLUSTER_VOUCH_ALREADY_REDEEMED"
        | "CLUSTER_VOUCH_OFFICE_MISMATCH" => StatusCode::UNAUTHORIZED,
        "CLUSTER_VOUCH_MALFORMED"
        | "CLUSTER_VOUCH_IDENTITY_INVALID"
        | "CLUSTER_JOIN_TOKEN_OFFICE_INVALID" => StatusCode::BAD_REQUEST,
        "CLUSTER_VOUCH_NOT_AN_OFFICE" | "CLUSTER_VOUCH_NOT_A_CONTROLLER" => {
            StatusCode::NOT_IMPLEMENTED
        }
        "CLUSTER_JOIN_TOKEN_UNKNOWN" | "CLUSTER_OFFICE_ADDRESS_UNKNOWN" => StatusCode::NOT_FOUND,
        "CLUSTER_JOIN_TOKEN_REVOKED" => StatusCode::FORBIDDEN,
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        status,
        Json(json!({
            "ok": false,
            "error": { "code": err.code, "message": err.message }
        })),
    )
        .into_response()
}

async fn api_internal_runtime_execute_pipeline(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Json(req): Json<ExecutePipelineRequest>,
) -> Response {
    if let Err(response) = require_controller_call(&state, &headers) {
        return response;
    }
    execute_pipeline_local(&state, &owner, &project, &req).await
}

async fn api_internal_runtime_webhook(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, tail)): Path<(String, String, String)>,
    method: Method,
    uri: Uri,
    body: Bytes,
) -> Response {
    if let Err(response) = require_controller_call(&state, &headers) {
        return response;
    }
    public_webhook_ingress(
        State(state),
        Path((owner, project, tail)),
        method,
        uri,
        headers,
        body,
    )
    .await
}

async fn api_internal_runtime_webhook_root(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    method: Method,
    uri: Uri,
    body: Bytes,
) -> Response {
    if let Err(response) = require_controller_call(&state, &headers) {
        return response;
    }
    public_webhook_ingress_root(
        State(state),
        Path((owner, project)),
        method,
        uri,
        headers,
        body,
    )
    .await
}

fn local_office_id(state: &PlatformAppState) -> Option<String> {
    let node_id = state.platform.cluster_bootstrap.node_id();
    if node_id.trim().is_empty() {
        None
    } else {
        Some(node_id)
    }
}

fn project_transfer_operation_kind(
    kind: ProjectTransferArtifactKind,
    is_import: bool,
) -> ProjectOperationKind {
    match (is_import, kind) {
        (false, ProjectTransferArtifactKind::Bundle) => ProjectOperationKind::ExportBundle,
        (false, ProjectTransferArtifactKind::Files) => ProjectOperationKind::ExportFiles,
        (false, ProjectTransferArtifactKind::Full) => ProjectOperationKind::ExportFull,
        (true, ProjectTransferArtifactKind::Bundle) => ProjectOperationKind::ImportBundle,
        (true, ProjectTransferArtifactKind::Files) => ProjectOperationKind::ImportFiles,
        (true, ProjectTransferArtifactKind::Full) => ProjectOperationKind::ImportFull,
    }
}

fn refresh_local_project_workspace(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
) -> Result<(), PlatformError> {
    state
        .platform
        .node_registry
        .refresh_project(owner, project)?;
    state
        .platform
        .cluster_runtime_sync
        .refresh_local_repo_state(owner, project)?;
    state
        .platform
        .pipeline_runtime
        .refresh_project(owner, project)?;
    Ok(())
}

async fn api_internal_project_transfer_export(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, raw_kind)): Path<(String, String, String)>,
) -> Response {
    if let Err(response) = require_controller_call(&state, &headers) {
        return response;
    }
    let kind = match parse_project_transfer_kind(&raw_kind) {
        Ok(kind) => kind,
        Err(response) => return response,
    };
    let op_id = format!(
        "internal-export-{}-{}",
        kind.key(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );
    let output_path = state.platform.project_transfer.artifact_path(&op_id, kind);
    let export = state.platform.project_transfer.export_project(
        &owner,
        &project,
        kind.classes(),
        local_office_id(&state).as_deref(),
        &output_path,
    );
    match export {
        Ok(_) => {}
        Err(err) => return internal_error(err),
    }
    let bytes = match fs::read(&output_path) {
        Ok(bytes) => bytes,
        Err(err) => return internal_error(err.into()),
    };
    let _ = fs::remove_dir_all(state.platform.project_transfer.operation_dir(&op_id));
    let mut response = Response::new(Body::from(bytes));
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("application/x-tar"));
    if let Ok(value) = HeaderValue::from_str(kind.archive_name()) {
        response
            .headers_mut()
            .insert(HeaderName::from_static("x-zebflow-transfer-archive"), value);
    }
    response
}

async fn api_internal_project_transfer_import(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, raw_kind)): Path<(String, String, String)>,
    body: Bytes,
) -> Response {
    if let Err(response) = require_controller_call(&state, &headers) {
        return response;
    }
    let kind = match parse_project_transfer_kind(&raw_kind) {
        Ok(kind) => kind,
        Err(response) => return response,
    };
    let op_id = format!(
        "internal-import-{}-{}",
        kind.key(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );
    let archive_path = state.platform.project_transfer.artifact_path(&op_id, kind);
    if let Some(parent) = archive_path.parent()
        && let Err(err) = fs::create_dir_all(parent)
    {
        return internal_error(err.into());
    }
    if let Err(err) = fs::write(&archive_path, &body) {
        return internal_error(err.into());
    }
    let import = state.platform.project_transfer.import_project(
        &owner,
        &project,
        Some(kind.classes()),
        &archive_path,
    );
    let outcome = match import {
        Ok(outcome) => outcome,
        Err(err) => return internal_error(err),
    };
    if let Err(err) = refresh_local_project_workspace(&state, &owner, &project) {
        return internal_error(err);
    }
    let _ = fs::remove_dir_all(state.platform.project_transfer.operation_dir(&op_id));
    let dependencies = post_import_dependency_report(&state, &owner, &project);
    Json(json!({"ok": true, "import": outcome, "dependencies": dependencies})).into_response()
}

/// The dependency report an import owes its caller
/// (`kinds/project-bundle/README.md`).
///
/// "Import verifies function targets resolve in the carried repo and reports
/// misses through the dependency report as its fifth family — report, not
/// refuse." The whole report travels, not just the fifth family: an import
/// that lands a repo whose node bundles or libraries did not travel has the
/// same problem, and the caller wants to see it in the same place. Producing
/// the report can itself fail; that failure is reported too, and never turns a
/// completed import into an error.
fn post_import_dependency_report(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
) -> serde_json::Value {
    let requested = match state.platform.zebflow_cfg.get_rwe_libraries(owner, project) {
        Ok(value) => value,
        Err(err) => return json!({ "ok": false, "error": err.message }),
    };
    match state
        .platform
        .dependency_lock
        .status(owner, project, &requested)
    {
        Ok(report) => json!(report),
        Err(err) => json!({ "ok": false, "error": err.message }),
    }
}

async fn api_project_transfer_operations(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::SettingsRead,
    ) {
        return response;
    }
    match state.platform.project_operations.list(&owner, &project, 10) {
        Ok(items) => Json(json!({"ok": true, "items": items})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_project_transfer_export(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, raw_kind)): Path<(String, String, String)>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::SettingsWrite,
    ) {
        return response;
    }
    let kind = match parse_project_transfer_kind(&raw_kind) {
        Ok(kind) => kind,
        Err(response) => return response,
    };
    let remote_worker_id = match remote_project_worker_id(&state, &owner, &project) {
        Ok(worker_id) => worker_id,
        Err(err) => return internal_error(err),
    };
    let mut operation = match state.platform.project_operations.create(
        &owner,
        &project,
        project_transfer_operation_kind(kind, false),
        remote_worker_id.clone().or_else(|| local_office_id(&state)),
        None,
    ) {
        Ok(record) => record,
        Err(err) => return internal_error(err),
    };
    operation = match state
        .platform
        .project_operations
        .mark_running(&operation, "preparing export")
    {
        Ok(record) => record,
        Err(err) => return internal_error(err),
    };
    let artifact_path = state
        .platform
        .project_transfer
        .artifact_path(&operation.operation_id, kind);
    let export_result = if let Some(worker_id) = remote_worker_id.as_deref() {
        let worker = match state.platform.cluster_registry.get_worker(worker_id) {
            Ok(Some(worker)) => worker,
            Ok(None) => {
                let _ = state.platform.project_operations.mark_failed(
                    &operation,
                    "locating office",
                    format!("office '{}' is not registered", worker_id),
                );
                return (
                    StatusCode::BAD_GATEWAY,
                    Json(json!({"ok": false, "error": {"code":"CLUSTER_WORKER_UNKNOWN","message":"office not registered"}})),
                )
                    .into_response();
            }
            Err(err) => return internal_error(err),
        };
        let token = match cluster_call_header_for_office(&state, worker_office_id(&worker)) {
            Ok(token) => token,
            Err(err) => return internal_error(err),
        };
        let url = format!(
            "{}/api/internal/project-transfer/{}/{}/export/{}",
            worker.base_url.trim_end_matches('/'),
            owner,
            project,
            kind.key()
        );
        match state
            .http_client
            .post(url)
            .header(INTERNAL_CLUSTER_TOKEN_HEADER, token)
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => match response.bytes().await {
                Ok(bytes) => fs::write(&artifact_path, &bytes).map_err(PlatformError::from),
                Err(err) => Err(PlatformError::new(
                    "PROJECT_TRANSFER_EXPORT",
                    err.to_string(),
                )),
            },
            Ok(response) => {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                Err(PlatformError::new(
                    "PROJECT_TRANSFER_EXPORT",
                    format!("remote office export failed with {status}: {body}"),
                ))
            }
            Err(err) => Err(PlatformError::new(
                "PROJECT_TRANSFER_EXPORT",
                err.to_string(),
            )),
        }
    } else {
        match state.platform.project_transfer.export_project(
            &owner,
            &project,
            kind.classes(),
            local_office_id(&state).as_deref(),
            &artifact_path,
        ) {
            Ok(_) => Ok(()),
            Err(err) => Err(err),
        }
    };
    if let Err(err) = export_result {
        let _ = state.platform.project_operations.mark_failed(
            &operation,
            "export failed",
            err.message.clone(),
        );
        return internal_error(err);
    }
    let sha256 = match state.platform.project_transfer.sha256_hex(&artifact_path) {
        Ok(value) => value,
        Err(err) => {
            let _ = state.platform.project_operations.mark_failed(
                &operation,
                "hashing archive",
                err.message.clone(),
            );
            return internal_error(err);
        }
    };
    let artifact_bytes = fs::metadata(&artifact_path).ok().map(|meta| meta.len());
    let operation = match state.platform.project_operations.mark_completed(
        &operation,
        "export completed",
        Some(
            state
                .platform
                .project_transfer
                .artifact_rel_path(&operation.operation_id, kind),
        ),
        Some(sha256),
        artifact_bytes,
    ) {
        Ok(record) => record,
        Err(err) => return internal_error(err),
    };
    Json(json!({
        "ok": true,
        "operation": operation,
        "download_url": format!(
            "/api/projects/{}/{}/transfer/download/{}",
            owner, project, operation.operation_id
        )
    }))
    .into_response()
}

async fn api_project_transfer_import(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, raw_kind)): Path<(String, String, String)>,
    mut multipart: Multipart,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::SettingsWrite,
    ) {
        return response;
    }
    let kind = match parse_project_transfer_kind(&raw_kind) {
        Ok(kind) => kind,
        Err(response) => return response,
    };
    let remote_worker_id = match remote_project_worker_id(&state, &owner, &project) {
        Ok(worker_id) => worker_id,
        Err(err) => return internal_error(err),
    };
    let mut operation = match state.platform.project_operations.create(
        &owner,
        &project,
        project_transfer_operation_kind(kind, true),
        None,
        remote_worker_id.clone().or_else(|| local_office_id(&state)),
    ) {
        Ok(record) => record,
        Err(err) => return internal_error(err),
    };
    operation = match state
        .platform
        .project_operations
        .mark_running(&operation, "receiving upload")
    {
        Ok(record) => record,
        Err(err) => return internal_error(err),
    };
    let field = match multipart.next_field().await {
        Ok(Some(field)) => field,
        Ok(None) => {
            let _ = state.platform.project_operations.mark_failed(
                &operation,
                "receiving upload",
                "no archive file in multipart body",
            );
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok": false, "error": {"code":"PROJECT_TRANSFER_UPLOAD_MISSING","message":"no archive file in multipart body"}})),
            )
                .into_response();
        }
        Err(err) => {
            let _ = state.platform.project_operations.mark_failed(
                &operation,
                "receiving upload",
                err.to_string(),
            );
            return internal_error(PlatformError::new(
                "PROJECT_TRANSFER_UPLOAD",
                err.to_string(),
            ));
        }
    };
    let bytes = match field.bytes().await {
        Ok(bytes) => bytes,
        Err(err) => {
            let _ = state.platform.project_operations.mark_failed(
                &operation,
                "reading upload",
                err.to_string(),
            );
            return internal_error(PlatformError::new(
                "PROJECT_TRANSFER_UPLOAD",
                err.to_string(),
            ));
        }
    };
    let archive_path = state
        .platform
        .project_transfer
        .artifact_path(&operation.operation_id, kind);
    if let Some(parent) = archive_path.parent()
        && let Err(err) = fs::create_dir_all(parent)
    {
        let _ = state.platform.project_operations.mark_failed(
            &operation,
            "staging upload",
            err.to_string(),
        );
        return internal_error(err.into());
    }
    if let Err(err) = fs::write(&archive_path, &bytes) {
        let _ = state.platform.project_operations.mark_failed(
            &operation,
            "staging upload",
            err.to_string(),
        );
        return internal_error(err.into());
    }
    let import_result = if let Some(worker_id) = remote_worker_id.as_deref() {
        let worker = match state.platform.cluster_registry.get_worker(worker_id) {
            Ok(Some(worker)) => worker,
            Ok(None) => {
                let _ = state.platform.project_operations.mark_failed(
                    &operation,
                    "locating office",
                    format!("office '{}' is not registered", worker_id),
                );
                return (
                    StatusCode::BAD_GATEWAY,
                    Json(json!({"ok": false, "error": {"code":"CLUSTER_WORKER_UNKNOWN","message":"office not registered"}})),
                )
                    .into_response();
            }
            Err(err) => return internal_error(err),
        };
        let token = match cluster_call_header_for_office(&state, worker_office_id(&worker)) {
            Ok(token) => token,
            Err(err) => return internal_error(err),
        };
        let url = format!(
            "{}/api/internal/project-transfer/{}/{}/import/{}",
            worker.base_url.trim_end_matches('/'),
            owner,
            project,
            kind.key()
        );
        match state
            .http_client
            .post(url)
            .header(INTERNAL_CLUSTER_TOKEN_HEADER, token)
            .header(CONTENT_TYPE, "application/x-tar")
            .body(bytes.clone())
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => Ok(None),
            Ok(response) => {
                let status = response.status();
                let body = response.text().await.unwrap_or_default();
                Err(PlatformError::new(
                    "PROJECT_TRANSFER_IMPORT",
                    format!("remote office import failed with {status}: {body}"),
                ))
            }
            Err(err) => Err(PlatformError::new(
                "PROJECT_TRANSFER_IMPORT",
                err.to_string(),
            )),
        }
    } else {
        match state.platform.project_transfer.import_project(
            &owner,
            &project,
            Some(kind.classes()),
            &archive_path,
        ) {
            Ok(outcome) => {
                refresh_local_project_workspace(&state, &owner, &project).map(|()| Some(outcome))
            }
            Err(err) => Err(err),
        }
    };
    let outcome = match import_result {
        Ok(outcome) => outcome,
        Err(err) => {
            let _ = state.platform.project_operations.mark_failed(
                &operation,
                "import failed",
                err.message.clone(),
            );
            return internal_error(err);
        }
    };
    // Provenance is recorded and shown, never a gate; the recovery copies are
    // named so rollback — the reverse swap — can find them.
    let step = match &outcome {
        Some(outcome) => format!(
            "import completed from '{}'; recovery: {}",
            outcome.provenance,
            outcome
                .recovery
                .iter()
                .map(|swap| swap.recovery_dir.clone())
                .collect::<Vec<_>>()
                .join(", ")
        ),
        None => "import completed".to_string(),
    };
    let operation = match state.platform.project_operations.mark_completed(
        &operation,
        step,
        None,
        None,
        Some(bytes.len() as u64),
    ) {
        Ok(record) => record,
        Err(err) => return internal_error(err),
    };
    let dependencies = post_import_dependency_report(&state, &owner, &project);
    Json(json!({
        "ok": true,
        "operation": operation,
        "import": outcome,
        "dependencies": dependencies,
    }))
    .into_response()
}

/// Request body for `POST /api/projects/{owner}/{project}/transfer/rollback`.
#[derive(serde::Deserialize)]
struct ProjectTransferRollbackRequest {
    /// The recovery swaps an import returned — rollback is their reverse.
    entries: Vec<crate::platform::services::project_transfer::ProjectImportRecoverySwap>,
}

/// `POST /api/projects/{owner}/{project}/transfer/rollback`
///
/// The reverse swap of `kinds/project-bundle/README.md`: each named recovery
/// copy under `data/recovery/` moves back over its class, and the displaced
/// imported bytes become a recovery copy of their own.
async fn api_project_transfer_rollback(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Json(body): Json<ProjectTransferRollbackRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::SettingsWrite,
    ) {
        return response;
    }
    if body.entries.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": {"code":"PROJECT_TRANSFER_ROLLBACK","message":"entries must name at least one recovery swap"}})),
        )
            .into_response();
    }
    // The recovery copies live where the import ran. Rolling back a
    // remote-placed project is the same unbuilt remote wiring as directing a
    // platform import at a remote office — recorded, not invented here.
    match remote_project_worker_id(&state, &owner, &project) {
        Ok(None) => {}
        Ok(Some(worker_id)) => {
            return (
                StatusCode::BAD_GATEWAY,
                Json(json!({"ok": false, "error": {"code":"PROJECT_TRANSFER_ROLLBACK_REMOTE","message": format!("project is placed on office '{worker_id}'; remote rollback is not built")}})),
            )
                .into_response();
        }
        Err(err) => return internal_error(err),
    }
    let mut operation = match state.platform.project_operations.create(
        &owner,
        &project,
        ProjectOperationKind::RollbackImport,
        None,
        local_office_id(&state),
    ) {
        Ok(record) => record,
        Err(err) => return internal_error(err),
    };
    operation = match state
        .platform
        .project_operations
        .mark_running(&operation, "restoring recovery copies")
    {
        Ok(record) => record,
        Err(err) => return internal_error(err),
    };
    let displaced =
        match state
            .platform
            .project_transfer
            .rollback_import(&owner, &project, &body.entries)
        {
            Ok(displaced) => displaced,
            Err(err) => {
                let _ = state.platform.project_operations.mark_failed(
                    &operation,
                    "rollback failed",
                    err.message.clone(),
                );
                return internal_error(err);
            }
        };
    if let Err(err) = refresh_local_project_workspace(&state, &owner, &project) {
        let _ = state.platform.project_operations.mark_failed(
            &operation,
            "refreshing workspace",
            err.message.clone(),
        );
        return internal_error(err);
    }
    let step = format!(
        "rollback completed; displaced imports: {}",
        displaced
            .iter()
            .map(|swap| swap.recovery_dir.clone())
            .collect::<Vec<_>>()
            .join(", ")
    );
    let operation = match state
        .platform
        .project_operations
        .mark_completed(&operation, step, None, None, None)
    {
        Ok(record) => record,
        Err(err) => return internal_error(err),
    };
    Json(json!({"ok": true, "operation": operation, "displaced": displaced})).into_response()
}

/// `POST /api/platform/transfer/import` — platform-scope import: creates a
/// project from a `ProjectBundle` archive.
///
/// Superadmin only, like every platform-scope install. The importer supplies
/// `owner` and `project`; the office is the one this import runs on —
/// directing an import at a remote office is the same unbuilt wiring as
/// remote `hub-public` provisioning (`stability-matrix.md` row 14) and is
/// not invented here.
///
/// Any bundle carrying `repo` qualifies. A bundle without `store` is
/// auto-initiated: the repo's declared schema and initial-data steps replay
/// into the fresh store. A carried store snapshot wins and nothing replays.
async fn api_platform_transfer_import(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    mut multipart: Multipart,
) -> Response {
    if let Err(response) = require_superadmin(&state, &headers) {
        return response;
    }
    let mut owner = String::new();
    let mut project = String::new();
    let mut archive: Option<Bytes> = None;
    loop {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => break,
            Err(err) => {
                return internal_error(PlatformError::new(
                    "PROJECT_TRANSFER_UPLOAD",
                    err.to_string(),
                ));
            }
        };
        let name = field.name().unwrap_or_default().to_string();
        match name.as_str() {
            "owner" => owner = field.text().await.unwrap_or_default(),
            "project" => project = field.text().await.unwrap_or_default(),
            "archive" => match field.bytes().await {
                Ok(bytes) => archive = Some(bytes),
                Err(err) => {
                    return internal_error(PlatformError::new(
                        "PROJECT_TRANSFER_UPLOAD",
                        err.to_string(),
                    ));
                }
            },
            _ => {}
        }
    }
    let owner = crate::platform::model::slug_segment(&owner);
    let project = crate::platform::model::slug_segment(&project);
    if owner.is_empty() || project.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": {"code":"PROJECT_TRANSFER_TARGET","message":"multipart fields 'owner' and 'project' are required"}})),
        )
            .into_response();
    }
    let Some(archive) = archive else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": {"code":"PROJECT_TRANSFER_UPLOAD_MISSING","message":"no 'archive' file in multipart body"}})),
        )
            .into_response();
    };
    match state.platform.users.get_user(&owner) {
        Ok(Some(_)) => {}
        Ok(None) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok": false, "error": {"code":"PROJECT_TRANSFER_TARGET","message":"owner does not exist"}})),
            )
                .into_response();
        }
        Err(err) => return internal_error(err),
    }
    match state.platform.projects.get_project(&owner, &project) {
        Ok(None) => {}
        Ok(Some(_)) => {
            return (
                StatusCode::CONFLICT,
                Json(json!({"ok": false, "error": {"code":"PROJECT_EXISTS","message":"target project already exists; import inside the project to replace its classes"}})),
            )
                .into_response();
        }
        Err(err) => return internal_error(err),
    }

    // Stage into the EPHEMERAL tier and verify the whole archive before any
    // project exists: a refused archive leaves nothing behind.
    let staging_root = state.platform.config.data_root.join("tmp").join("transfer");
    if let Err(err) = fs::create_dir_all(&staging_root) {
        return internal_error(err.into());
    }
    let upload_dir = match tempfile::Builder::new()
        .prefix("platform-import-")
        .tempdir_in(&staging_root)
    {
        Ok(dir) => dir,
        Err(err) => return internal_error(err.into()),
    };
    let archive_path = upload_dir
        .path()
        .join(ProjectTransferArtifactKind::Bundle.archive_name());
    if let Err(err) = fs::write(&archive_path, &archive) {
        return internal_error(err.into());
    }
    let staging = match state.platform.project_transfer.stage_archive(&archive_path) {
        Ok(staging) => staging,
        Err(err) => return internal_error(err),
    };
    if !staging.carries(crate::contracts::kinds::ProjectBundleClass::Repo) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": {"code":"PROJECT_TRANSFER_CLASSES","message":"a platform import creates a project, so the archive must carry the repo class"}})),
        )
            .into_response();
    }
    let store_carried = staging.carries(crate::contracts::kinds::ProjectBundleClass::Store);
    let archive_sha256 = match state.platform.project_transfer.sha256_hex(&archive_path) {
        Ok(value) => value,
        Err(err) => return internal_error(err),
    };

    if let Err(err) = state.platform.projects.create_or_update_project(
        &owner,
        &crate::platform::model::CreateProjectRequest {
            project: project.clone(),
            title: None,
            local_branch: None,
            runtime: Default::default(),
        },
    ) {
        return internal_error(err);
    }
    let mut operation = match state.platform.project_operations.create(
        &owner,
        &project,
        ProjectOperationKind::PlatformImport,
        None,
        local_office_id(&state),
    ) {
        Ok(record) => record,
        Err(err) => return internal_error(err),
    };
    operation = match state
        .platform
        .project_operations
        .mark_running(&operation, "materializing classes")
    {
        Ok(record) => record,
        Err(err) => return internal_error(err),
    };
    let outcome = match state
        .platform
        .project_transfer
        .import_staged(&owner, &project, staging)
    {
        Ok(outcome) => outcome,
        Err(err) => {
            let _ = state.platform.project_operations.mark_failed(
                &operation,
                "materializing classes",
                err.message.clone(),
            );
            return internal_error(err);
        }
    };
    // The catalog has no rows for the imported repo's pipelines yet; the
    // reindex walk registers them, exactly as crash recovery does.
    let reindex = match reindex_project_repo_files(&state, &owner, &project) {
        Ok(report) => report,
        Err(err) => json!({ "errors": [err.message] }),
    };
    // Auto-initiation per the schema/initial-data rule: a carried store
    // snapshot already contains its applied state and nothing replays.
    let initial_data_steps = if store_carried {
        Vec::new()
    } else {
        match state.platform.hub.initiate_project_store(&owner, &project) {
            Ok(steps) => steps.into_iter().map(|step| step.path).collect(),
            Err(err) => {
                let _ = state.platform.project_operations.mark_failed(
                    &operation,
                    "auto-initiating store",
                    err.message.clone(),
                );
                return internal_error(err);
            }
        }
    };
    if let Err(err) = refresh_local_project_workspace(&state, &owner, &project) {
        let _ = state.platform.project_operations.mark_failed(
            &operation,
            "refreshing workspace",
            err.message.clone(),
        );
        return internal_error(err);
    }
    let step = format!("platform import completed from '{}'", outcome.provenance);
    let operation = match state.platform.project_operations.mark_completed(
        &operation,
        step,
        None,
        Some(archive_sha256),
        Some(archive.len() as u64),
    ) {
        Ok(record) => record,
        Err(err) => return internal_error(err),
    };
    let dependencies = post_import_dependency_report(&state, &owner, &project);
    Json(json!({
        "ok": true,
        "owner": owner,
        "project": project,
        "operation": operation,
        "import": outcome,
        "store_auto_initiated": !store_carried,
        "initial_data_replayed": initial_data_steps,
        "pipelines_indexed": reindex,
        "dependencies": dependencies,
    }))
    .into_response()
}

async fn api_project_transfer_download(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, operation_id)): Path<(String, String, String)>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::SettingsWrite,
    ) {
        return response;
    }
    let record = match state
        .platform
        .project_operations
        .get(&owner, &project, &operation_id)
    {
        Ok(Some(record)) => record,
        Ok(None) => return StatusCode::NOT_FOUND.into_response(),
        Err(err) => return internal_error(err),
    };
    let Some(rel_path) = record.artifact_rel_path.as_deref() else {
        return StatusCode::NOT_FOUND.into_response();
    };
    let path = state
        .platform
        .config
        .data_root
        .join("platform")
        .join("project-operations")
        .join(rel_path);
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) => return internal_error(err.into()),
    };
    let archive_name = match record.kind {
        ProjectOperationKind::ExportBundle | ProjectOperationKind::ImportBundle => {
            ProjectTransferArtifactKind::Bundle.archive_name()
        }
        ProjectOperationKind::ExportFiles | ProjectOperationKind::ImportFiles => {
            ProjectTransferArtifactKind::Files.archive_name()
        }
        ProjectOperationKind::ExportFull | ProjectOperationKind::ImportFull => {
            ProjectTransferArtifactKind::Full.archive_name()
        }
        // Platform imports and rollbacks record no downloadable artifact; the
        // missing artifact_rel_path already returned 404 above.
        ProjectOperationKind::PlatformImport | ProjectOperationKind::RollbackImport => {
            ProjectTransferArtifactKind::Bundle.archive_name()
        }
    };
    let mut response = Response::new(Body::from(bytes));
    response
        .headers_mut()
        .insert(CONTENT_TYPE, HeaderValue::from_static("application/x-tar"));
    if let Ok(disposition) =
        HeaderValue::from_str(&format!("attachment; filename=\"{archive_name}\""))
    {
        response
            .headers_mut()
            .insert(HeaderName::from_static("content-disposition"), disposition);
    }
    response
}

/// `POST /api/projects/{owner}/{project}/transfer/owner`
///
/// Re-keys all project-scoped catalog records and moves the project directory
/// tree to the new owner. Superadmin-only.
async fn api_transfer_project_owner(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Json(body): Json<serde_json::Value>,
) -> Response {
    if let Err(response) = require_superadmin(&state, &headers) {
        return response;
    }
    let new_owner = match body.get("new_owner").and_then(|v| v.as_str()) {
        Some(v) if !v.trim().is_empty() => v.trim().to_string(),
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok": false, "error": "new_owner is required"})),
            )
                .into_response();
        }
    };
    if let Err(err) = state
        .platform
        .projects
        .transfer_project_owner(&owner, &project, &new_owner)
    {
        return match err.code {
            "PLATFORM_TRANSFER_INVALID" => (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok": false, "error": err.message})),
            )
                .into_response(),
            "PLATFORM_TRANSFER_NOT_FOUND" => (
                StatusCode::NOT_FOUND,
                Json(json!({"ok": false, "error": err.message})),
            )
                .into_response(),
            "PLATFORM_TRANSFER_CONFLICT" => (
                StatusCode::CONFLICT,
                Json(json!({"ok": false, "error": err.message})),
            )
                .into_response(),
            _ => internal_error(err),
        };
    }

    // Refresh pipeline runtime under new owner.
    let _ = state
        .platform
        .pipeline_runtime
        .refresh_project(&new_owner, &project);

    // Clear template cache — dependency paths contain absolute owner paths.
    if let Ok(mut cache) = state.template_cache.write() {
        cache.clear();
    }

    // Revoke in-memory MCP sessions for the old owner/project.
    let _ = state
        .platform
        .mcp_sessions
        .revoke_for_project(&owner, &project);

    Json(json!({
        "ok": true,
        "old_owner": owner,
        "new_owner": new_owner,
        "project": project,
    }))
    .into_response()
}

/// Request body for `DELETE /api/users/{owner}/projects/{project}`.
#[derive(serde::Deserialize)]
struct DeleteProjectRequest {
    /// Must exactly match the project slug as a confirmation.
    project_name: String,
    /// Authenticated user's password — re-verified before deletion.
    password: String,
}

/// `DELETE /api/users/{owner}/projects/{project}` — irreversibly deletes a project.
///
/// Requires:
/// - The caller is authenticated as `{owner}` (session cookie).
/// - `project_name` in the body matches the project slug.
/// - `password` in the body authenticates the current user.
///
/// Deletes:
/// - The project row and every platform-DB row keyed to it: policies and their
///   bindings, members, invites, runtime placement, operations, credentials,
///   database connections, hub repositories, pipeline metadata and
///   invocations, and MCP sessions.
/// - Entire `data_root/users/{owner}/{project}/` directory tree from disk.
///
/// Refuses with 409 when the project hosts a hub authority: that would cascade
/// into a published catalogue, which has to be a deliberate act of its own.
async fn api_delete_project(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Json(req): Json<DeleteProjectRequest>,
) -> Response {
    // Deleting a project asks the policy system, like everything else.
    //
    // It used to compare the session's name against the namespace owner. That
    // is outside the policy system entirely: no binding could grant it, no role
    // could describe it, and it was the reason Owner and Maintainer resolved to
    // the same capabilities — the one thing separating them was not a
    // capability at all.
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::ProjectDelete,
    ) {
        return response;
    }
    let owner_slug = crate::platform::model::slug_segment(&owner);
    let project_slug = crate::platform::model::slug_segment(&project);

    // Confirm project name matches
    let confirmed_slug = crate::platform::model::slug_segment(&req.project_name);
    if confirmed_slug != project_slug || project_slug.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": "Project name does not match"})),
        )
            .into_response();
    }

    // Re-verify password
    match state
        .platform
        .users
        .authenticate(&owner_slug, &req.password)
    {
        Ok(true) => {}
        Ok(false) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"ok": false, "error": "Incorrect password"})),
            )
                .into_response();
        }
        Err(e) => return internal_error(e),
    }

    // Verify project exists
    match state.platform.data.get_project(&owner_slug, &project_slug) {
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"ok": false, "error": "Project not found"})),
            )
                .into_response();
        }
        Err(e) => return internal_error(e),
        Ok(Some(_)) => {}
    }

    // Delete metadata from the platform DB. A project that hosts a hub
    // authority is refused rather than cascaded, and that refusal is a
    // conflict the caller can act on, not a server fault.
    if let Err(e) = state
        .platform
        .data
        .delete_project(&owner_slug, &project_slug)
    {
        if e.code == "PLATFORM_PROJECT_HOSTS_HUB_AUTHORITY" {
            return (
                StatusCode::CONFLICT,
                Json(json!({"ok": false, "error": e.message})),
            )
                .into_response();
        }
        return internal_error(e);
    }

    // Delete the entire project directory from disk
    let project_root = state
        .platform
        .config
        .data_root
        .join("users")
        .join(&owner_slug)
        .join(&project_slug);

    if project_root.exists() {
        if let Err(e) = std::fs::remove_dir_all(&project_root) {
            // DB record already gone — log but don't fail the response
            eprintln!("WARN: Failed to remove project dir {:?}: {e}", project_root);
        }
    }

    Json(json!({"ok": true})).into_response()
}

async fn api_pipeline_registry(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Query(query): Query<PipelineRegistryQuery>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesRead,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::GET,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }
    let scope = match resolve_pipeline_registry_scope(&query) {
        Ok(scope) => scope,
        Err(response) => return response,
    };
    let base_route = format!("/projects/{owner}/{project}/pipelines/registry");
    let editor_base = format!("/projects/{owner}/{project}/pipelines/registry");
    // Git reports repository-relative paths, while pipeline identity is
    // source-relative, so the root is added back for those lookups.
    let repo_layout = match state.platform.projects.project_layout(&owner, &project) {
        Ok(layout) => layout.repo_layout,
        Err(err) => return internal_error(err),
    };
    let git_map: std::collections::HashMap<String, String> = state
        .platform
        .projects
        .list_repo_git_status(&owner, &project)
        .unwrap_or_default()
        .into_iter()
        .map(|item| (item.rel_path, item.code))
        .collect();
    let pipeline_git_status = |file_rel_path: &str| -> Option<String> {
        git_map.get(&repo_layout.source_rel(file_rel_path)).cloned()
    };
    match scope {
        PipelineRegistryScope::Path => {
            let current_path = query.path.as_deref().unwrap_or("/");
            match state.platform.projects.list_pipeline_registry(
                &owner,
                &project,
                current_path,
                &base_route,
                &editor_base,
            ) {
                Ok(mut listing) => {
                    for item in &mut listing.pipelines {
                        item.git_status = pipeline_git_status(&item.file_rel_path);
                    }
                    for item in &mut listing.files {
                        item.git_status = git_map.get(&item.rel_path).cloned();
                    }
                    Json(json!({"ok": true, "scope": "path", "listing": listing})).into_response()
                }
                Err(err) => internal_error(err),
            }
        }
        PipelineRegistryScope::Project => match state
            .platform
            .projects
            .list_pipeline_meta_rows(&owner, &project)
        {
            Ok(rows) => {
                let items = rows
                    .into_iter()
                    .map(|meta| {
                        let is_active = meta
                            .active_hash
                            .as_deref()
                            .map(|h| !h.is_empty() && h == meta.hash)
                            .unwrap_or(false);
                        let has_draft = meta
                            .active_hash
                            .as_deref()
                            .map(|h| !h.is_empty() && h != meta.hash)
                            .unwrap_or(false);
                        let git_status = pipeline_git_status(&meta.file_rel_path);
                        let file_rel_path = meta.file_rel_path.clone();
                        json!({
                            "id": file_rel_path,
                            "name": meta.name,
                            "title": meta.title,
                            "description": meta.description,
                            "trigger_kind": meta.trigger_kind,
                            "virtual_path": meta.virtual_path,
                            "file_rel_path": meta.file_rel_path,
                            "is_active": is_active,
                            "has_draft": has_draft,
                            "git_status": git_status,
                        })
                    })
                    .collect::<Vec<_>>();
                let count = items.len();
                Json(json!({
                    "ok": true,
                    "scope": "project",
                    "items": items,
                    "count": count
                }))
                .into_response()
            }
            Err(err) => internal_error(err),
        },
    }
}

async fn api_list_pipelines(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Query(query): Query<PipelineListQuery>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesRead,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::GET,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }
    let base_path =
        crate::platform::model::normalize_virtual_path(query.path.as_deref().unwrap_or("/"));
    let recursive = query.recursive.unwrap_or(false);
    match state
        .platform
        .projects
        .list_pipeline_meta_rows(&owner, &project)
    {
        Ok(rows) => {
            let items = rows
                .into_iter()
                .filter(|meta| pipeline_path_matches(&base_path, &meta.virtual_path, recursive))
                .map(|meta| {
                    json!({
                        "id": meta.file_rel_path,
                        "meta": meta
                    })
                })
                .collect::<Vec<_>>();
            Json(json!({
                "ok": true,
                "path": base_path,
                "recursive": recursive,
                "items": items
            }))
            .into_response()
        }
        Err(err) => internal_error(err),
    }
}

async fn api_get_pipeline_by_id(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Query(query): Query<PipelineByIdQuery>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesRead,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::GET,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }
    let Some(file_id) = query
        .id
        .as_deref()
        .map(str::trim)
        .filter(|id| !id.is_empty())
    else {
        return (
            StatusCode::BAD_REQUEST,
            Json(
                json!({"ok": false, "error": {"code":"PLATFORM_PIPELINE_ID_MISSING", "message":"query.id is required"}}),
            ),
        )
            .into_response();
    };

    let meta = match state
        .platform
        .projects
        .get_pipeline_meta_by_file_id(&owner, &project, file_id)
    {
        Ok(Some(meta)) => meta,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(
                    json!({"ok": false, "error": {"code":"PLATFORM_PIPELINE_MISSING", "message":"pipeline not found"}}),
                ),
            )
                .into_response();
        }
        Err(err) => return internal_error(err),
    };

    let include_source = query.include_source.unwrap_or(true);
    let include_active_source = query.include_active_source.unwrap_or(false);
    let source = if include_source {
        match state
            .platform
            .projects
            .read_pipeline_source(&owner, &project, &meta.file_rel_path)
        {
            Ok(source) => Some(source),
            Err(err) => return internal_error(err),
        }
    } else {
        None
    };
    let active_source = if include_active_source {
        match state
            .platform
            .projects
            .read_active_pipeline_source(&owner, &project, &meta)
        {
            Ok(source) => Some(source),
            Err(err) if err.code == "PLATFORM_PIPELINE_NOT_ACTIVE" => None,
            Err(err) => return internal_error(err),
        }
    } else {
        None
    };
    let locked = if let Some(source_text) = source.as_deref() {
        pipeline_source_is_locked(source_text)
    } else {
        match state
            .platform
            .projects
            .read_pipeline_source(&owner, &project, &meta.file_rel_path)
        {
            Ok(source_text) => pipeline_source_is_locked(&source_text),
            Err(_) => false,
        }
    };

    Json(json!({
        "ok": true,
        "id": meta.file_rel_path,
        "meta": meta,
        "locked": locked,
        "source": source,
        "active_source": active_source,
        "hits": state
            .platform
            .pipeline_hits
            .get(&owner, &project, &meta.file_rel_path)
    }))
    .into_response()
}

async fn api_upsert_pipeline_definition(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Json(req): Json<UpsertPipelineDefinitionRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesWrite,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_json_request_to_worker(
            &state,
            &uri,
            &headers,
            Method::POST,
            &req,
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }

    let graph = match decode_pipeline_graph(req.source.as_bytes()) {
        Ok(document) => document.spec,
        Err(err) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({
                    "ok": false,
                    "error": {
                        "code": err.violation_code().unwrap_or("PLATFORM_PIPELINE_PARSE"),
                        "message": format!("failed parsing pipeline source: {err} ({})", err.category())
                    }
                })),
            )
                .into_response();
        }
    };

    let self_file_rel_path = req.file_rel_path.clone();
    // Conflict check: reject if any active pipeline already owns the same webhook path.
    if let Ok(conflicts) = state.platform.projects.check_webhook_path_conflict(
        &owner,
        &project,
        &graph,
        &self_file_rel_path,
    ) && !conflicts.is_empty()
    {
        let msg = format!(
            "{} {} is already registered by pipeline '{}'",
            conflicts[0].method, conflicts[0].path, conflicts[0].pipeline_name
        );
        return (
            StatusCode::CONFLICT,
            Json(json!({
                "ok": false,
                "error": {
                    "code": "PLATFORM_PIPELINE_WEBHOOK_CONFLICT",
                    "message": msg,
                    "conflicts": conflicts
                }
            })),
        )
            .into_response();
    }

    // Derive trigger_kind from the actual graph entry nodes so that it stays correct
    // even when the user changes the trigger node in the visual editor after creation.
    let trigger_kind =
        crate::platform::services::project::derive_trigger_kind_from_source(&req.source)
            .unwrap_or_else(|| req.trigger_kind.clone());

    match state.platform.projects.upsert_pipeline_definition(
        &owner,
        &project,
        &req.file_rel_path,
        &req.title,
        &req.description,
        &trigger_kind,
        &req.source,
    ) {
        Ok(meta) => {
            // The set of third-party nodes this project depends on may have
            // changed, so refresh the interfaces it carries in repo/nodes.
            if let Err(err) = state
                .platform
                .node_registry
                .sync_project_node_interfaces(&owner, &project)
            {
                eprintln!(
                    "node_interfaces: sync failed for {owner}/{project}: {}",
                    err.message
                );
            }
            Json(json!({"ok": true, "meta": meta})).into_response()
        }
        Err(err) => internal_error(err),
    }
}

async fn api_delete_pipeline_definition(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Json(req): Json<DeletePipelineRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesWrite,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_json_request_to_worker(
            &state,
            &uri,
            &headers,
            Method::DELETE,
            &req,
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }
    match state
        .platform
        .projects
        .delete_pipeline(&owner, &project, &req.file_rel_path)
    {
        Ok(()) => {
            state
                .platform
                .pipeline_runtime
                .evict(&owner, &project, &req.file_rel_path);
            Json(json!({"ok": true})).into_response()
        }
        Err(err) => internal_error(err),
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
struct PipelineLockToggleRequest {
    file_rel_path: String,
    locked: bool,
}

#[derive(serde::Deserialize, serde::Serialize)]
struct TemplateLockToggleRequest {
    rel_path: String,
    locked: bool,
}

async fn api_pipeline_lock_toggle(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Json(req): Json<PipelineLockToggleRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesWrite,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_json_request_to_worker(
            &state,
            &uri,
            &headers,
            Method::POST,
            &req,
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }
    let owner_slug = crate::platform::model::slug_segment(&owner);
    let project_slug = crate::platform::model::slug_segment(&project);
    let layout = match state
        .platform
        .file
        .ensure_project_layout(&owner_slug, &project_slug)
    {
        Ok(l) => l,
        Err(err) => return internal_error(err),
    };
    // Identity is source-relative; git and the filesystem both want the
    // repository-relative form. Both come from the project service's one
    // resolver, which applies the identity rule and refuses a path that leaves
    // the source root -- this handler used to derive them itself, so a body
    // naming `../../` reached another owner's project under this caller's
    // authorisation.
    let (pipeline_repo_rel, pipeline_path) = match state
        .platform
        .projects
        .resolve_pipeline_paths(&layout, &req.file_rel_path)
    {
        Ok(paths) => paths,
        Err(err) => return internal_error(err),
    };
    let source = match std::fs::read(&pipeline_path) {
        Ok(s) => s,
        Err(_) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"ok": false, "error": "pipeline not found"})),
            )
                .into_response();
        }
    };
    let mut graph = match crate::contracts::kinds::decode_pipeline_graph(&source) {
        Ok(document) => document.spec,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok": false, "error": e.to_string()})),
            )
                .into_response();
        }
    };
    graph.metadata.get_or_insert_default().locked = req.locked;
    let serialized = match crate::contracts::kinds::encode_pipeline_graph(graph) {
        Ok(s) => s,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"ok": false, "error": e.to_string()})),
            )
                .into_response();
        }
    };
    if let Err(e) = crate::infra::io::durable::atomic_write(&pipeline_path, &serialized) {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"ok": false, "error": e.to_string()})),
        )
            .into_response();
    }
    // Auto-commit
    let name = req
        .file_rel_path
        .rsplit('/')
        .next()
        .unwrap_or(&req.file_rel_path)
        .replace(".zf.json", "");
    let verb = if req.locked { "lock" } else { "unlock" };
    let commit_msg = format!("{verb}: pipeline {name}");
    let actor_user = session_owner(&state, &headers);
    let identity_args = git_identity_args(&state, actor_user.as_deref(), &project_slug);
    let _ = {
        let mut add_cmd = std::process::Command::new("git");
        add_cmd
            .arg("-C")
            .arg(&layout.repo_dir)
            .arg("add")
            .arg("--")
            .arg(&pipeline_repo_rel);
        add_cmd.output()
    };
    let _ = {
        let mut commit_cmd = std::process::Command::new("git");
        for arg in &identity_args {
            commit_cmd.arg(arg);
        }
        commit_cmd
            .arg("-C")
            .arg(&layout.repo_dir)
            .arg("commit")
            .arg("-m")
            .arg(&commit_msg);
        commit_cmd.output()
    };
    Json(json!({"ok": true, "is_locked": req.locked})).into_response()
}

async fn api_template_lock_toggle(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Json(req): Json<TemplateLockToggleRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesWrite,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_json_request_to_worker(
            &state,
            &uri,
            &headers,
            Method::POST,
            &req,
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }
    if let Err(err) =
        state
            .platform
            .zebflow_cfg
            .set_template_locked(&owner, &project, &req.rel_path, req.locked)
    {
        return internal_error(err);
    }
    // Auto-commit
    let owner_slug = crate::platform::model::slug_segment(&owner);
    let project_slug = crate::platform::model::slug_segment(&project);
    let layout = match state
        .platform
        .file
        .ensure_project_layout(&owner_slug, &project_slug)
    {
        Ok(l) => l,
        Err(err) => return internal_error(err),
    };
    let verb = if req.locked { "lock" } else { "unlock" };
    let commit_msg = format!("{verb}: template {}", req.rel_path);
    let actor_user = session_owner(&state, &headers);
    let identity_args = git_identity_args(&state, actor_user.as_deref(), &project_slug);
    let _ = {
        let mut add_cmd = std::process::Command::new("git");
        add_cmd
            .arg("-C")
            .arg(&layout.repo_dir)
            .arg("add")
            .arg("--")
            .arg(crate::contracts::kinds::PROJECT_CONFIGURATION_FILE);
        add_cmd.output()
    };
    let _ = {
        let mut commit_cmd = std::process::Command::new("git");
        for arg in &identity_args {
            commit_cmd.arg(arg);
        }
        commit_cmd
            .arg("-C")
            .arg(&layout.repo_dir)
            .arg("commit")
            .arg("-m")
            .arg(&commit_msg);
        commit_cmd.output()
    };
    Json(json!({"ok": true, "is_locked": req.locked})).into_response()
}

async fn api_repo_git_status(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TemplatesRead,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::GET,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }
    match state
        .platform
        .projects
        .list_repo_git_status(&owner, &project)
    {
        Ok(items) => {
            let branch = state
                .platform
                .projects
                .get_repo_git_branch(&owner, &project)
                .unwrap_or_default();
            Json(json!({ "branch": branch, "files": items })).into_response()
        }
        Err(err) => internal_error(err),
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
struct GitRepairRequest {
    #[serde(default)]
    mode: Option<crate::platform::services::project::ProjectGitRepairMode>,
}

async fn api_repo_git_health(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TemplatesRead,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::GET,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }
    match state
        .platform
        .projects
        .get_repo_git_health(&owner, &project)
    {
        Ok(health) => Json(json!({ "ok": true, "health": health })).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_repo_git_repair(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Json(req): Json<GitRepairRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesWrite,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_json_request_to_worker(
            &state,
            &uri,
            &headers,
            Method::POST,
            &req,
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }
    let mode = req
        .mode
        .unwrap_or(crate::platform::services::project::ProjectGitRepairMode::Repair);
    match state
        .platform
        .projects
        .repair_repo_git(&owner, &project, mode)
    {
        Ok(health) => Json(json!({ "ok": true, "health": health })).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_git_list_branches(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TemplatesRead,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::GET,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }
    let current = state
        .platform
        .projects
        .get_repo_git_branch(&owner, &project)
        .unwrap_or_default();
    match state
        .platform
        .projects
        .list_repo_git_local_branches(&owner, &project)
    {
        Ok(branches) => Json(json!({ "current": current, "branches": branches })).into_response(),
        Err(err) => internal_error(err),
    }
}

#[derive(serde::Deserialize, serde::Serialize)]
struct GitCheckoutRequest {
    branch: String,
    #[serde(default)]
    create: bool,
}

async fn api_git_checkout_branch(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Json(req): Json<GitCheckoutRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesWrite,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_json_request_to_worker(
            &state,
            &uri,
            &headers,
            Method::POST,
            &req,
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }
    match state.platform.projects.checkout_repo_git_branch(
        &owner,
        &project,
        &req.branch,
        req.create,
    ) {
        Ok(()) => Json(json!({ "ok": true })).into_response(),
        Err(err) => (
            StatusCode::BAD_REQUEST,
            Json(json!({ "ok": false, "error": err.to_string() })),
        )
            .into_response(),
    }
}

/// Returns `-c user.name=… -c user.email=…` args for git.
/// Prefers the acting user's git profile, then a generated fallback identity.
fn git_identity_args(
    state: &PlatformAppState,
    actor_user: Option<&str>,
    project_slug: &str,
) -> Vec<String> {
    let resolved = state
        .platform
        .git_identity
        .resolve_for_actor(actor_user, project_slug);
    vec![
        "-c".to_string(),
        format!("user.name={}", resolved.name),
        "-c".to_string(),
        format!("user.email={}", resolved.email),
    ]
}

fn git_failure_message(output: &std::process::Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    match (stderr.is_empty(), stdout.is_empty()) {
        (false, false) => format!("{stderr}\n{stdout}"),
        (false, true) => stderr,
        (true, false) => stdout,
        (true, true) => format!("git exited with status {}", output.status),
    }
}

async fn api_git_commit(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Json(req): Json<GitCommitRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesWrite,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_json_request_to_worker(
            &state,
            &uri,
            &headers,
            Method::POST,
            &req,
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }
    if req.files.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": "no files specified"})),
        )
            .into_response();
    }
    if req.message.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": "commit message is required"})),
        )
            .into_response();
    }
    let owner_slug = crate::platform::model::slug_segment(&owner);
    let project_slug = crate::platform::model::slug_segment(&project);
    let layout = match state
        .platform
        .file
        .ensure_project_layout(&owner_slug, &project_slug)
    {
        Ok(l) => l,
        Err(err) => return internal_error(err),
    };
    // git add <files>
    let mut add_cmd = std::process::Command::new("git");
    add_cmd.arg("-C").arg(&layout.repo_dir).arg("add").arg("--");
    for f in &req.files {
        add_cmd.arg(f);
    }
    let add_out = match add_cmd.output() {
        Ok(o) => o,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"ok": false, "error": e.to_string()})),
            )
                .into_response();
        }
    };
    if !add_out.status.success() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"ok": false, "error": git_failure_message(&add_out)})),
        )
            .into_response();
    }
    // git commit -m <message>
    let actor_user = session_owner(&state, &headers);
    let identity_args = git_identity_args(&state, actor_user.as_deref(), &project_slug);
    let mut commit_cmd = std::process::Command::new("git");
    for arg in &identity_args {
        commit_cmd.arg(arg);
    }
    commit_cmd
        .arg("-C")
        .arg(&layout.repo_dir)
        .arg("commit")
        .arg("-m")
        .arg(req.message.trim());
    let commit_out = match commit_cmd.output() {
        Ok(o) => o,
        Err(e) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"ok": false, "error": e.to_string()})),
            )
                .into_response();
        }
    };
    if !commit_out.status.success() {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"ok": false, "error": git_failure_message(&commit_out)})),
        )
            .into_response();
    }
    // optional push
    if req.push {
        // Resolve push target: prefer explicit request fields, fall back to zebflow.yaml remote
        let zebflow_cfg = match state
            .platform
            .zebflow_cfg
            .read_or_default(&owner_slug, &project_slug)
        {
            Ok(config) => config,
            Err(err) => return internal_error(err),
        };
        let (cred_id, repo_url, branch) = {
            let remote_cfg = &zebflow_cfg.configs.git.remote;
            let cid = req
                .credential_id
                .clone()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| remote_cfg.credential_id.clone());
            let url = req
                .repo_url
                .clone()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| remote_cfg.repo_url.clone());
            let br = req
                .branch
                .clone()
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| {
                    if remote_cfg.branch.is_empty() {
                        "main".to_string()
                    } else {
                        remote_cfg.branch.clone()
                    }
                });
            (cid, url, br)
        };

        if cred_id.is_empty() || repo_url.is_empty() {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok": false, "error": "No remote configured. Connect a remote in the Git panel first."})),
            )
                .into_response();
        }

        // Build authenticated push URL
        let auth_url = {
            let cred = state
                .platform
                .credentials
                .get_project_credential(&owner, &project, &cred_id)
                .ok()
                .flatten();
            let mut url_opt: Option<String> = None;
            if let Some(c) = cred {
                let username = c.secret["username"].as_str().unwrap_or("");
                let token = c.secret["token"].as_str().unwrap_or("");
                if !username.is_empty() && !token.is_empty() {
                    if let Ok(mut parsed) = reqwest::Url::parse(&repo_url) {
                        let _ = parsed.set_username(username);
                        let _ = parsed.set_password(Some(token));
                        url_opt = Some(parsed.to_string());
                    }
                }
            }
            match url_opt {
                Some(u) => u,
                None => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"ok": false, "error": "Could not build authenticated push URL — check credential username/token."})),
                    )
                        .into_response();
                }
            }
        };

        // Pull --rebase before push to handle diverged remote
        let mut pull_cmd = std::process::Command::new("git");
        for arg in &identity_args {
            pull_cmd.arg(arg);
        }
        let pull_out = pull_cmd
            .arg("-C")
            .arg(&layout.repo_dir)
            .arg("pull")
            .arg("--rebase")
            .arg(&auth_url)
            .arg(&branch)
            .output();
        match pull_out {
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"ok": false, "error": format!("pull --rebase failed: {}", e)})),
                )
                    .into_response();
            }
            Ok(ref o) if !o.status.success() => {
                // Abort the rebase so the repo is not left mid-rebase
                let _ = std::process::Command::new("git")
                    .arg("-C")
                    .arg(&layout.repo_dir)
                    .arg("rebase")
                    .arg("--abort")
                    .output();
                let raw = git_failure_message(o);
                let safe = redact_auth_urls(&raw);
                return (
                    StatusCode::CONFLICT,
                    Json(json!({"ok": false, "error": format!("Rebase conflict — resolve locally and try again: {safe}")})),
                )
                    .into_response();
            }
            Ok(_) => {}
        }

        // Push --set-upstream so future pushes work without tracking config
        let push_out = std::process::Command::new("git")
            .arg("-C")
            .arg(&layout.repo_dir)
            .arg("push")
            .arg("--set-upstream")
            .arg(&auth_url)
            .arg(format!("HEAD:{}", branch))
            .output();
        match push_out {
            Err(e) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"ok": false, "error": e.to_string()})),
                )
                    .into_response();
            }
            Ok(ref o) if !o.status.success() => {
                let raw = git_failure_message(o);
                let safe = redact_auth_urls(&raw);
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"ok": false, "error": safe})),
                )
                    .into_response();
            }
            Ok(_) => {}
        }
    }
    Json(json!({"ok": true})).into_response()
}

/// Request body for `PUT /api/projects/{owner}/{project}/git/remote`.
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(deny_unknown_fields)]
struct GitRemoteRequest {
    #[serde(default)]
    credential_id: String,
    #[serde(default)]
    repo_url: String,
    #[serde(default)]
    branch: String,
}

/// `GET /api/projects/{owner}/{project}/git/remote` — returns saved remote config.
async fn api_git_get_remote(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    if let Err(r) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::SettingsRead,
    ) {
        return r;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::GET,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }
    let cfg = match state.platform.zebflow_cfg.read_or_default(&owner, &project) {
        Ok(config) => config,
        Err(err) => return internal_error(err),
    };
    Json(json!({
        "credential_id": cfg.configs.git.remote.credential_id,
        "repo_url": cfg.configs.git.remote.repo_url,
        "branch": cfg.configs.git.remote.branch,
    }))
    .into_response()
}

/// `PUT /api/projects/{owner}/{project}/git/remote` — saves remote config.
async fn api_git_put_remote(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Json(req): Json<GitRemoteRequest>,
) -> Response {
    if let Err(r) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::SettingsWrite,
    ) {
        return r;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_json_request_to_worker(
            &state,
            &uri,
            &headers,
            Method::PUT,
            &req,
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }
    let branch = if req.branch.trim().is_empty() {
        "main".to_string()
    } else {
        req.branch.trim().to_string()
    };
    match state.platform.zebflow_cfg.update(&owner, &project, |cfg| {
        cfg.configs.git.remote.credential_id = req.credential_id.trim().to_string();
        cfg.configs.git.remote.repo_url = req.repo_url.trim().to_string();
        cfg.configs.git.remote.branch = branch;
    }) {
        Ok(_) => Json(json!({"ok": true})).into_response(),
        Err(e) => internal_error(e),
    }
}

async fn api_activate_pipeline_definition(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Json(req): Json<PipelineLocateRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesWrite,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_json_request_to_worker(
            &state,
            &uri,
            &headers,
            Method::POST,
            &req,
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }

    if let Ok(Some(meta)) =
        state
            .platform
            .projects
            .get_pipeline_meta_by_file_id(&owner, &project, &req.file_rel_path)
    {
        if let Ok(source) =
            state
                .platform
                .projects
                .read_pipeline_source(&owner, &project, &meta.file_rel_path)
        {
            if let Ok(graph) = decode_pipeline_graph(source.as_bytes()).map(|value| value.spec) {
                if let Ok(conflicts) = state.platform.projects.check_webhook_path_conflict(
                    &owner,
                    &project,
                    &graph,
                    &meta.file_rel_path,
                ) {
                    if !conflicts.is_empty() {
                        let msg = format!(
                            "{} {} is already registered by pipeline '{}'",
                            conflicts[0].method, conflicts[0].path, conflicts[0].pipeline_name
                        );
                        return (
                            StatusCode::CONFLICT,
                            Json(json!({
                                "ok": false,
                                "error": {
                                    "code": "PLATFORM_PIPELINE_WEBHOOK_CONFLICT",
                                    "message": msg,
                                    "conflicts": conflicts
                                }
                            })),
                        )
                            .into_response();
                    }
                }
            }
        }
    }

    match state
        .platform
        .projects
        .activate_pipeline_definition(&owner, &project, &req.file_rel_path)
    {
        Ok(meta) => {
            if let Err(err) = state.platform.pipeline_runtime.refresh_pipeline(
                &owner,
                &project,
                &req.file_rel_path,
            ) {
                return internal_error(err);
            }
            state
                .scheduler
                .sync_pipeline(&owner, &project, &req.file_rel_path)
                .await;
            state
                .kv_subscriber
                .sync_pipeline(&owner, &project, &req.file_rel_path)
                .await;
            state
                .ws_client_manager
                .sync_pipeline(&owner, &project, &req.file_rel_path)
                .await;
            // Run composite lifecycle hooks (on_activate).
            run_composite_lifecycle_hooks(
                &state,
                &owner,
                &project,
                &req.file_rel_path,
                "on_activate",
            )
            .await;
            Json(json!({"ok": true, "meta": meta})).into_response()
        }
        Err(err) => internal_error(err),
    }
}

async fn api_deactivate_pipeline_definition(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Json(req): Json<PipelineLocateRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesWrite,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_json_request_to_worker(
            &state,
            &uri,
            &headers,
            Method::POST,
            &req,
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }

    match state.platform.projects.deactivate_pipeline_definition(
        &owner,
        &project,
        &req.file_rel_path,
    ) {
        Ok(meta) => {
            // Run composite lifecycle hooks (on_deactivate) before eviction.
            run_composite_lifecycle_hooks(
                &state,
                &owner,
                &project,
                &req.file_rel_path,
                "on_deactivate",
            )
            .await;
            state
                .platform
                .pipeline_runtime
                .evict(&owner, &project, &req.file_rel_path);
            state
                .scheduler
                .sync_pipeline(&owner, &project, &req.file_rel_path)
                .await;
            state
                .kv_subscriber
                .sync_pipeline(&owner, &project, &req.file_rel_path)
                .await;
            state
                .ws_client_manager
                .sync_pipeline(&owner, &project, &req.file_rel_path)
                .await;
            Json(json!({"ok": true, "meta": meta})).into_response()
        }
        Err(err) => internal_error(err),
    }
}

/// Runs one WASM lifecycle hook for an installed node.
///
/// Failures are logged and never block activation, matching the composite hook
/// behavior.
async fn run_wasm_lifecycle_hook(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
    file_rel_path: &str,
    hook: &str,
    node: &crate::pipeline::model::PipelineNode,
) {
    let Some(installed) = state
        .platform
        .node_registry
        .get_by_kind(owner, project, &node.kind)
    else {
        return;
    };
    let lifecycle = match installed.manifest.lifecycle.as_ref() {
        Some(lifecycle) => lifecycle,
        None => return,
    };
    let binding = match hook {
        "on_activate" => lifecycle.on_activate.as_ref(),
        "on_deactivate" => lifecycle.on_deactivate.as_ref(),
        _ => None,
    };
    let Some(binding) = binding else {
        return;
    };
    let (Some(module_name), Some(export)) = (binding.module.as_deref(), binding.export.as_deref())
    else {
        return;
    };
    let Some(module_spec) = installed.manifest.modules.get(module_name) else {
        eprintln!(
            "wasm_lifecycle: {hook} for '{}' references undeclared module '{module_name}'",
            node.kind
        );
        return;
    };

    let public_url = std::env::var("ZEBFLOW_PLATFORM_BASE_URL").unwrap_or_else(|_| {
        let port = std::env::var("ZEBFLOW_PORT").unwrap_or_else(|_| "10611".to_string());
        format!("http://localhost:{}", port)
    });
    let payload = serde_json::json!({
        "node_id": node.id,
        "node_kind": node.kind,
        "config": node.config,
        "hook": hook,
        "pipeline": file_rel_path,
        "owner": owner,
        "project": project,
        "platform": { "public_url": public_url },
    });
    let metadata = serde_json::json!({ "owner": owner, "project": project });

    match crate::pipeline::engines::wasm_host::run_wasm_export(
        &node.kind,
        &installed.package_dir,
        module_spec,
        export,
        &node.config,
        &payload,
        &metadata,
    ) {
        Ok(output) => eprintln!(
            "wasm_lifecycle: {hook} for '{}' in '{}' OK: {}",
            node.kind, file_rel_path, output
        ),
        Err(err) => eprintln!(
            "wasm_lifecycle: {hook} for '{}' in '{}' failed: {}",
            node.kind, file_rel_path, err.message
        ),
    }
}

/// Runs lifecycle hooks (on_activate or on_deactivate) for every installed node
/// found in a pipeline graph.
///
/// Scans the pipeline graph nodes, checks each against the node registry for lifecycle
/// hooks, and executes the corresponding composite function or WASM export.
async fn run_composite_lifecycle_hooks(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
    file_rel_path: &str,
    hook: &str, // "on_activate" or "on_deactivate"
) {
    // Read the pipeline source to get the graph.
    let source = match state
        .platform
        .projects
        .read_pipeline_source(owner, project, file_rel_path)
    {
        Ok(s) => s,
        Err(_) => return,
    };
    let graph = match decode_pipeline_graph(source.as_bytes()) {
        Ok(document) => document.spec,
        Err(_) => return,
    };

    // Check each node for composite lifecycle hooks.
    for node in &graph.nodes {
        // No namespace test: a curated bundle node and a third-party one both
        // have lifecycle hooks, and the registry returns nothing for native
        // kinds, so the manifest lookup below is the filter.
        let manifest = match state
            .platform
            .node_registry
            .get_manifest(owner, project, &node.kind)
        {
            Some(m) => m,
            None => continue,
        };
        let lifecycle = match &manifest.lifecycle {
            Some(lc) => lc,
            None => continue,
        };
        let binding = match hook {
            "on_activate" => lifecycle.on_activate.as_ref(),
            "on_deactivate" => lifecycle.on_deactivate.as_ref(),
            _ => None,
        };
        let binding = match binding {
            Some(b) => b,
            None => continue,
        };

        // A WASM hook runs its declared export directly. Composite and WASM
        // hooks receive the same payload.
        if binding.module.is_some() {
            run_wasm_lifecycle_hook(state, owner, project, file_rel_path, hook, node).await;
            continue;
        }

        let function_name = match binding.function.as_deref() {
            Some(f) => f,
            None => continue,
        };

        // Load the lifecycle function pipeline.
        let function_graph = match state.platform.node_registry.load_composite_function(
            owner,
            project,
            &node.kind,
            Some(function_name),
        ) {
            Ok(g) => g,
            Err(e) => {
                eprintln!(
                    "composite_lifecycle: failed loading {} for '{}': {}",
                    hook, node.kind, e.message
                );
                continue;
            }
        };

        // Resolve credential placeholders from the composite node's config.
        let placeholder_map = crate::pipeline::build_composite_placeholder_map(
            &node.kind,
            &node.config,
            owner,
            project,
            &state.platform,
        );

        // Resolve public URL for platform context.
        let public_url = std::env::var("ZEBFLOW_PLATFORM_BASE_URL").unwrap_or_else(|_| {
            let port = std::env::var("ZEBFLOW_PORT").unwrap_or_else(|_| "10611".to_string());
            format!("http://localhost:{}", port)
        });

        // Build execution context with node config, placeholders, and platform info.
        let ctx = crate::pipeline::PipelineContext {
            owner: owner.to_string(),
            project: project.to_string(),
            pipeline: function_graph.id.clone(),
            request_id: format!(
                "lifecycle-{}-{}-{}",
                hook,
                node.kind,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            ),
            route: Default::default(),
            input: serde_json::json!({
                "node_id": node.id,
                "node_kind": node.kind,
                "config": node.config,
                "hook": hook,
                "pipeline": file_rel_path,
                "owner": owner,
                "project": project,
                "platform": {
                    "public_url": public_url,
                },
            }),
            trigger: None,
            placeholder: if placeholder_map.is_empty() {
                None
            } else {
                Some(serde_json::json!(placeholder_map))
            },
        };

        // Build engine with credential support. A lifecycle hook is the
        // bundle's own function pipeline, so it runs under the same declared
        // hosts as any other node the bundle provides.
        let engine = crate::pipeline::BasicPipelineEngine::new(
            std::sync::Arc::new(state.platform.project_sandbox(owner, project)),
            crate::rwe::resolve_engine_or_default(None),
            Some(state.platform.credentials.clone()),
        )
        .with_platform(state.platform.clone())
        .with_ws_hub(state.platform.ws_hub.clone())
        .with_state_bus(state.platform.state_bus.clone())
        .with_data_root(state.platform.config.data_root.clone())
        .with_bundle_egress(Some(std::sync::Arc::new(
            crate::pipeline::security::BundleEgress::extend(
                None,
                &manifest.package,
                &manifest.hosts,
            ),
        )));

        // Execute — fire and forget (log errors but don't block activation).
        match engine.execute_async(&function_graph, &ctx).await {
            Ok(output) => {
                eprintln!(
                    "composite_lifecycle: {} for '{}' in '{}' OK: {}",
                    hook,
                    node.kind,
                    file_rel_path,
                    serde_json::to_string(&output.value).unwrap_or_default()
                );
            }
            Err(e) => {
                eprintln!(
                    "composite_lifecycle: {} for '{}' in '{}' FAILED: {}",
                    hook, node.kind, file_rel_path, e.message
                );
            }
        }
    }
}

async fn api_execute_pipeline(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Json(req): Json<ExecutePipelineRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesExecute,
    ) {
        return response;
    }
    if let Ok(Some(placement)) = state.platform.cluster_placement.get(&owner, &project) {
        if placement.target == ProjectRuntimePlacementTarget::Worker {
            if let Some(worker_id) = placement.worker_id.as_deref() {
                return match forward_runtime_execute_to_worker(
                    &state, &owner, &project, &req, worker_id,
                )
                .await
                {
                    Ok(response) => response,
                    Err(err) => internal_error(err),
                };
            }
        }
    }
    execute_pipeline_local(&state, &owner, &project, &req).await
}

async fn execute_pipeline_local(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
    req: &ExecutePipelineRequest,
) -> Response {
    let exec_start = std::time::Instant::now();
    let project_cfg = match state.platform.zebflow_cfg.read_or_default(owner, project) {
        Ok(config) => config,
        Err(err) => return internal_error(err),
    };
    let project_retention = resolve_invocation_retention(&project_cfg, None);
    let request_id = format!(
        "pipeline-exec-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );

    let meta = match state.platform.projects.get_pipeline_meta_by_file_id(
        owner,
        project,
        &req.file_rel_path,
    ) {
        Ok(Some(meta)) => meta,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(
                    json!({"ok": false, "error": {"code":"PLATFORM_PIPELINE_MISSING", "message":"pipeline not found"}}),
                ),
            )
                .into_response();
        }
        Err(err) => return internal_error(err),
    };

    let source = match state
        .platform
        .projects
        .read_active_pipeline_source(owner, project, &meta)
    {
        Ok(source) => source,
        Err(err) if err.code == "PLATFORM_PIPELINE_NOT_ACTIVE" => {
            state.platform.pipeline_hits.record_failure(
                owner,
                project,
                &meta.file_rel_path,
                "api.execute",
                err.code,
                "pipeline must be activated before execution",
            );
            let _ = state.platform.data.log_pipeline_invocation(
                owner,
                project,
                &meta.file_rel_path,
                &PipelineInvocationEntry {
                    run_id: request_id.clone(),
                    at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64,
                    duration_ms: exec_start.elapsed().as_millis() as u64,
                    status: "error".to_string(),
                    trigger: "manual".to_string(),
                    error: Some("pipeline must be activated before execution".to_string()),
                    trace: vec![],
                },
                project_retention.max_invocations,
                project_retention.max_age_secs,
            );
            return (
                StatusCode::CONFLICT,
                Json(
                    json!({"ok": false, "error": {"code": err.code, "message":"pipeline must be activated before execution"}}),
                ),
            )
                .into_response();
        }
        Err(err) => return internal_error(err),
    };

    let mut graph = match decode_pipeline_graph(source.as_bytes()) {
        Ok(document) => document.spec,
        Err(err) => {
            state.platform.pipeline_hits.record_failure(
                owner,
                project,
                &meta.file_rel_path,
                "api.execute",
                "PLATFORM_PIPELINE_PARSE",
                &err.to_string(),
            );
            let _ = state.platform.data.log_pipeline_invocation(
                owner,
                project,
                &meta.file_rel_path,
                &PipelineInvocationEntry {
                    run_id: request_id.clone(),
                    at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64,
                    duration_ms: exec_start.elapsed().as_millis() as u64,
                    status: "error".to_string(),
                    trigger: "manual".to_string(),
                    error: Some(err.to_string()),
                    trace: vec![],
                },
                project_retention.max_invocations,
                project_retention.max_age_secs,
            );
            return (
                StatusCode::BAD_REQUEST,
                Json(
                    json!({"ok": false, "error": {"code":"PLATFORM_PIPELINE_PARSE", "message": err.to_string()}}),
                ),
            )
                .into_response();
        }
    };
    let retention = resolve_invocation_retention(&project_cfg, Some(&graph));
    if let Err(err) = hydrate_template_markup(state, owner, project, &mut graph) {
        state.platform.pipeline_hits.record_failure(
            owner,
            project,
            &meta.file_rel_path,
            "api.execute",
            err.code,
            &err.message,
        );
        let _ = state.platform.data.log_pipeline_invocation(
            owner,
            project,
            &meta.file_rel_path,
            &PipelineInvocationEntry {
                run_id: request_id.clone(),
                at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64,
                duration_ms: exec_start.elapsed().as_millis() as u64,
                status: "error".to_string(),
                trigger: "manual".to_string(),
                error: Some(err.message.clone()),
                trace: vec![],
            },
            retention.max_invocations,
            retention.max_age_secs,
        );
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": {"code": err.code, "message": err.message}})),
        )
            .into_response();
    }
    if let Err(err) = apply_rwe_project_options(state, owner, project, &mut graph) {
        return internal_error(err);
    }

    if let Err(message) = validate_execute_trigger(&graph, &req) {
        state.platform.pipeline_hits.record_failure(
            owner,
            project,
            &meta.file_rel_path,
            "api.execute",
            "PLATFORM_PIPELINE_TRIGGER_MISMATCH",
            &message,
        );
        let _ = state.platform.data.log_pipeline_invocation(
            owner,
            project,
            &meta.file_rel_path,
            &PipelineInvocationEntry {
                run_id: request_id.clone(),
                at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64,
                duration_ms: exec_start.elapsed().as_millis() as u64,
                status: "error".to_string(),
                trigger: "manual".to_string(),
                error: Some(message.clone()),
                trace: vec![],
            },
            retention.max_invocations,
            retention.max_age_secs,
        );
        return (
            StatusCode::BAD_REQUEST,
            Json(
                json!({"ok": false, "error": {"code":"PLATFORM_PIPELINE_TRIGGER_MISMATCH", "message": message}}),
            ),
        )
            .into_response();
    }

    let credentials = state.platform.credentials.clone();
    let graph_for_run = graph.clone();
    let ctx = PipelineContext {
        owner: owner.to_string(),
        project: project.to_string(),
        pipeline: graph.id.clone(),
        request_id: request_id.clone(),
        route: Default::default(),
        input: req.input.clone(),
        trigger: None,
        placeholder: None,
    };
    let engine = BasicPipelineEngine::new(
        Arc::new(state.platform.project_sandbox(owner, project)),
        state.frontend.rwe.clone(),
        Some(credentials),
    )
    .with_platform(state.platform.clone())
    .with_template_cache(state.template_cache.clone())
    .with_project_layout(state.platform.projects.project_layout(owner, project).ok())
    .with_ws_hub(state.platform.ws_hub.clone())
    .with_ws_client_manager(state.ws_client_manager.clone())
    .with_state_bus(state.platform.state_bus.clone())
    .with_data_root(state.platform.config.data_root.clone());
    match engine.execute_async(&graph_for_run, &ctx).await {
        Ok(output) => {
            state
                .platform
                .pipeline_hits
                .record_success(owner, project, &meta.file_rel_path);
            let _ = state.platform.data.log_pipeline_invocation(
                owner,
                project,
                &meta.file_rel_path,
                &PipelineInvocationEntry {
                    run_id: request_id.clone(),
                    at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64,
                    duration_ms: exec_start.elapsed().as_millis() as u64,
                    status: "ok".to_string(),
                    trigger: "manual".to_string(),
                    error: None,
                    trace: output.node_trace.clone(),
                },
                retention.max_invocations,
                retention.max_age_secs,
            );
            Json(json!({
                "ok": true,
                "run_id": request_id,
                "meta": meta,
                "output": output.value,
                "trace": output.trace
            }))
            .into_response()
        }
        Err(err) => {
            state.platform.pipeline_hits.record_failure(
                owner,
                project,
                &meta.file_rel_path,
                "api.execute",
                err.code,
                &err.message,
            );
            let _ = state.platform.data.log_pipeline_invocation(
                owner,
                project,
                &meta.file_rel_path,
                &PipelineInvocationEntry {
                    run_id: request_id.clone(),
                    at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64,
                    duration_ms: exec_start.elapsed().as_millis() as u64,
                    status: "error".to_string(),
                    trigger: "manual".to_string(),
                    error: Some(err.message.clone()),
                    trace: err.node_trace.clone(),
                },
                retention.max_invocations,
                retention.max_age_secs,
            );
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok": false, "error": {"code": err.code, "message": err.message}})),
            )
                .into_response()
        }
    }
}

/// POST /api/projects/{owner}/{project}/pipelines/dsl
async fn api_execute_pipeline_dsl(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Json(req): Json<crate::platform::shell::DslRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesExecute,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_json_request_to_worker(
            &state,
            &uri,
            &headers,
            Method::POST,
            &req,
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }

    let executor = crate::platform::shell::executor::DslExecutor::new(
        state.platform.clone(),
        &owner,
        &project,
    );
    let output = executor.execute_dsl(&req.dsl).await;
    let navigate = crate::platform::interaction::InteractionEngine::new(&owner, &project)
        .match_dsl(&req.dsl, output.ok);
    Json(json!({ "ok": output.ok, "lines": output.lines, "navigate": navigate })).into_response()
}

async fn api_pipeline_hits(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesRead,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::GET,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }

    match state
        .platform
        .projects
        .list_pipeline_meta_rows(&owner, &project)
    {
        Ok(rows) => {
            let items = rows
                .into_iter()
                .map(|meta| {
                    let file_id = meta.file_rel_path.clone();
                    json!({
                        "id": file_id,
                        "name": meta.name,
                        "virtual_path": meta.virtual_path,
                        "file_rel_path": meta.file_rel_path,
                        "stats": state
                            .platform
                            .pipeline_hits
                            .get(&owner, &project, &meta.file_rel_path)
                    })
                })
                .collect::<Vec<_>>();
            Json(json!({
                "ok": true,
                "items": items,
                "count": items.len()
            }))
            .into_response()
        }
        Err(err) => internal_error(err),
    }
}

/// GET /api/projects/{owner}/{project}/pipelines/invocations?pipeline=<file_rel_path>
async fn api_pipeline_invocations(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesRead,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::GET,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }

    let Some(file_rel_path) = params.get("pipeline") else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": {"code": "MISSING_PARAM", "message": "missing ?pipeline= query parameter"}})),
        )
            .into_response();
    };

    let project_cfg = match state.platform.zebflow_cfg.read_or_default(&owner, &project) {
        Ok(config) => config,
        Err(err) => return internal_error(err),
    };
    let retention =
        match state
            .platform
            .projects
            .get_pipeline_meta_by_file_id(&owner, &project, file_rel_path)
        {
            Ok(Some(meta)) => {
                let source = state
                    .platform
                    .projects
                    .read_active_pipeline_source(&owner, &project, &meta)
                    .or_else(|_| {
                        state.platform.projects.read_pipeline_source(
                            &owner,
                            &project,
                            file_rel_path,
                        )
                    })
                    .ok();
                let graph = source
                    .as_deref()
                    .and_then(|raw| decode_pipeline_graph(raw.as_bytes()).ok())
                    .map(|document| document.spec);
                resolve_invocation_retention(&project_cfg, graph.as_ref())
            }
            _ => resolve_invocation_retention(&project_cfg, None),
        };

    match state.platform.data.get_pipeline_invocations(
        &owner,
        &project,
        file_rel_path,
        retention.max_age_secs,
    ) {
        Ok(entries) => Json(json!({
            "ok": true,
            "file_rel_path": file_rel_path,
            "entries": entries,
            "count": entries.len(),
        }))
        .into_response(),
        Err(err) => internal_error(err),
    }
}

#[derive(Debug, Deserialize)]
struct RepoPathQuery {
    path: Option<String>,
}

/// Query for `GET /repo`.
///
/// `depth=1` is one folder's children, which is what a sidebar asks for as the
/// reader opens it. Omitting `depth` walks the whole subtree, which is what the
/// pipeline pages want. `fields=path` answers file paths and nothing else, for
/// the quick-open palette.
#[derive(Debug, Deserialize)]
struct RepoTreeQuery {
    path: Option<String>,
    depth: Option<usize>,
    fields: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RepoMoveRequest {
    from_path: String,
    to_path: String,
}

fn repo_path_param(query: &RepoPathQuery) -> Result<String, Response> {
    query
        .path
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok": false, "error": {"code": "PLATFORM_REPO_PATH", "message": "missing repository path"}})),
            )
                .into_response()
        })
}

#[derive(Debug, Deserialize)]
struct RepoSearchQuery {
    q: Option<String>,
    #[serde(default)]
    context: Option<usize>,
}

async fn api_repo_search(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Query(query): Query<RepoSearchQuery>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TemplatesRead,
    ) {
        return response;
    }
    let Some(pattern) = query
        .q
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Json(json!({"ok": true, "matches": []})).into_response();
    };
    match state.platform.projects.search_repo_files(
        &owner,
        &project,
        pattern,
        query.context.unwrap_or(0),
    ) {
        Ok(hits) => {
            let matches = hits
                .into_iter()
                .map(|(rel_path, line, block)| {
                    json!({ "rel_path": rel_path, "line": line, "block": block })
                })
                .collect::<Vec<_>>();
            Json(json!({"ok": true, "matches": matches})).into_response()
        }
        Err(err) => internal_error(err),
    }
}

async fn api_repo_tree(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Query(query): Query<RepoTreeQuery>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TemplatesRead,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::GET,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }
    if query.fields.as_deref() == Some("path") {
        return match state.platform.projects.list_repo_paths(&owner, &project) {
            Ok(paths) => Json(serde_json::json!({ "paths": paths })).into_response(),
            Err(err) => internal_error(err),
        };
    }

    let scope = RepoTreeScope {
        path: query.path.unwrap_or_default().trim_matches('/').to_string(),
        depth: query.depth,
    };
    match state.platform.projects.list_repo_tree(&owner, &project, &scope) {
        Ok(listing) => Json(listing).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_repo_read(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Query(query): Query<RepoPathQuery>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TemplatesRead,
    ) {
        return response;
    }
    let path = match repo_path_param(&query) {
        Ok(path) => path,
        Err(response) => return response,
    };
    match state
        .platform
        .projects
        .read_repo_file(&owner, &project, &path)
    {
        Ok(payload) => Json(json!({"ok": true, "file": payload})).into_response(),
        Err(err) => internal_error(err),
    }
}

/// Drop any compiled page whose bundle inlined this file.
///
/// The compiler flattens every component a page imports into one bundle, so a
/// page keeps serving its old copy of a component until the entry holding it is
/// evicted. MCP's `template_write` has always done this; the repo file API never
/// did, so writing a component through `PUT /repo/file` left the browser served
/// from cache indefinitely — not until a timeout, but until someone restarted
/// the server or pressed Clear Template Cache.
///
/// Silent on failure by design: a path that resolves to nothing was not a
/// template, and refusing the write over it would be worse than the stale entry.
fn evict_cached_pages_for(state: &PlatformAppState, owner: &str, project: &str, rel_path: &str) {
    if let Ok(abs) = state
        .platform
        .projects
        .resolve_template_abs_path(owner, project, rel_path)
    {
        crate::pipeline::engines::basic::evict_template_cache_by_path(
            &state.template_cache,
            &abs.to_string_lossy(),
        );
    }
}

async fn api_repo_write(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Query(query): Query<RepoPathQuery>,
    body: Bytes,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TemplatesWrite,
    ) {
        return response;
    }
    let path = match repo_path_param(&query) {
        Ok(path) => path,
        Err(response) => return response,
    };
    let content = String::from_utf8(body.to_vec()).unwrap_or_default();
    match state
        .platform
        .projects
        .write_repo_file(&owner, &project, &path, &content)
    {
        Ok(payload) => {
            evict_cached_pages_for(&state, &owner, &project, &path);
            Json(json!({"ok": true, "file": payload})).into_response()
        }
        Err(err) => internal_error(err),
    }
}

async fn api_repo_delete(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Query(query): Query<RepoPathQuery>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TemplatesDelete,
    ) {
        return response;
    }
    let path = match repo_path_param(&query) {
        Ok(path) => path,
        Err(response) => return response,
    };
    match state
        .platform
        .projects
        .delete_repo_entry(&owner, &project, &path)
    {
        Ok(()) => {
            evict_cached_pages_for(&state, &owner, &project, &path);
            Json(json!({"ok": true})).into_response()
        }
        Err(err) => internal_error(err),
    }
}

async fn api_repo_create_folder(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Json(query): Json<RepoPathQuery>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TemplatesCreate,
    ) {
        return response;
    }
    let path = match repo_path_param(&query) {
        Ok(path) => path,
        Err(response) => return response,
    };
    match state
        .platform
        .projects
        .create_repo_folder(&owner, &project, &path)
    {
        Ok(rel) => Json(json!({"ok": true, "path": rel})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_repo_move(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Json(req): Json<RepoMoveRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TemplatesMove,
    ) {
        return response;
    }
    match state
        .platform
        .projects
        .move_repo_entry(&owner, &project, &req.from_path, &req.to_path)
    {
        Ok(rel) => Json(json!({"ok": true, "path": rel})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_template_outline(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Query(query): Query<TemplatePathQuery>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TemplatesRead,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::GET,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }
    let Some(path) = query.path.as_deref() else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"missing path"})),
        )
            .into_response();
    };
    match state
        .platform
        .projects
        .read_repo_file_text(&owner, &project, path)
    {
        Ok(content) => {
            let outline =
                crate::platform::services::tsx_outline::extract_outline(&content, Some(path));
            Json(json!({
                "ok": true,
                "rel_path": path,
                "outline": outline,
            }))
            .into_response()
        }
        Err(err) => internal_error(err),
    }
}

async fn api_template_git_status(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TemplatesRead,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::GET,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }
    match state
        .platform
        .projects
        .list_template_git_status(&owner, &project)
    {
        Ok(items) => Json(items).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_template_diagnostics(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Json(req): Json<TemplateCompileRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TemplatesDiagnostics,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_json_request_to_worker(
            &state,
            &uri,
            &headers,
            Method::POST,
            &req,
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }

    let owner = crate::platform::model::slug_segment(&owner);
    let project = crate::platform::model::slug_segment(&project);
    let layout = match state.platform.file.ensure_project_layout(&owner, &project) {
        Ok(layout) => layout,
        Err(err) => return internal_error(err),
    };

    let response = compile_template_buffer(&state, &layout.repo_source_dir(), &req);
    Json(response).into_response()
}

// ── File browser API ──────────────────────────────────────────────────────────

/// GET /api/projects/{owner}/{project}/files/list?path=uploads
/// Returns folders and files at a given path inside files_dir.
async fn api_files_list(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Response {
    if let Err(r) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::FilesRead,
    ) {
        return r;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::GET,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }
    let layout = match state.platform.file.ensure_project_layout(&owner, &project) {
        Ok(l) => l,
        Err(e) => return internal_error(e),
    };
    let rel = params
        .get("path")
        .map(|s| s.trim().trim_start_matches('/'))
        .unwrap_or("")
        .to_string();
    let zebfs = layout.open_files();

    let mut folders: Vec<serde_json::Value> = Vec::new();
    let mut files: Vec<serde_json::Value> = Vec::new();
    let entries = match zebfs.list(&rel) {
        Ok(entries) => entries,
        Err(err) if err.code == "ZEBFS_INVALID_PATH" => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error":"invalid path"})),
            )
                .into_response();
        }
        Err(err) => return internal_error(PlatformError::new(err.code, err.message)),
    };
    for entry in entries {
        let name = entry.name;
        let path = entry.path;
        let access = crate::platform::services::zebfs_acl::effective_access(&zebfs, &path)
            .map(|value| value.as_str())
            .unwrap_or("private");
        if matches!(entry.kind, crate::zebfs::ZebFsEntryKind::Prefix) {
            folders.push(json!({
                "name": name,
                "path": path,
                "access": access,
                "public": access == "public_read",
                "protected": false,
            }));
        } else {
            let modified = entry
                .modified
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            files.push(json!({
                "name": name,
                "path": path.clone(),
                "size": entry.size,
                "modified": modified,
                "access": access,
                "public": access == "public_read",
                "url": format!("/fs/{owner}/{project}/{}", path),
            }));
        }
    }

    Json(json!({ "path": rel, "folders": folders, "files": files })).into_response()
}

/// POST /api/projects/{owner}/{project}/files/mkdir  { "path": "photos" }
async fn api_files_mkdir(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Json(body): Json<serde_json::Value>,
) -> Response {
    if let Err(r) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::FilesWrite,
    ) {
        return r;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_json_request_to_worker(
            &state,
            &uri,
            &headers,
            Method::POST,
            &body,
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }
    let layout = match state.platform.file.ensure_project_layout(&owner, &project) {
        Ok(l) => l,
        Err(e) => return internal_error(e),
    };

    let path_str = body
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .trim_start_matches('/')
        .to_string();
    let path_str = match crate::zebfs::normalize_object_path(&path_str) {
        Ok(path) => path,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error":"invalid path"})),
            )
                .into_response();
        }
    };
    if path_str.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"path is required"})),
        )
            .into_response();
    }

    let zebfs = layout.open_files();
    match zebfs.create_prefix(&path_str) {
        Ok(_) => Json(json!({ "ok": true, "path": path_str })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

/// `POST /api/projects/{owner}/{project}/files/upload?path=uploads`
/// Upload a file into project file storage via multipart.
async fn api_files_upload(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Query(params): Query<std::collections::HashMap<String, String>>,
    mut multipart: Multipart,
) -> Response {
    if let Err(r) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::FilesWrite,
    ) {
        return r;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"ok": false, "error": "no file field in multipart body"})),
                )
                    .into_response();
            }
            Err(err) => return internal_error(PlatformError::new("FILES_UPLOAD", err.to_string())),
        };
        let raw_filename = field
            .file_name()
            .map(|value| value.to_string())
            .unwrap_or_else(|| "upload.bin".to_string());
        let content_type = field.content_type().map(|value| value.to_string());
        let bytes = match field.bytes().await {
            Ok(bytes) => bytes,
            Err(err) => return internal_error(PlatformError::new("FILES_UPLOAD", err.to_string())),
        };
        let worker = match state.platform.cluster_registry.get_worker(&worker_id) {
            Ok(Some(worker)) => worker,
            Ok(None) => {
                return (
                    StatusCode::BAD_GATEWAY,
                    Json(json!({"ok": false, "error": "office not registered"})),
                )
                    .into_response();
            }
            Err(err) => return internal_error(err),
        };
        let token = match cluster_call_header_for_office(&state, worker_office_id(&worker)) {
            Ok(token) => token,
            Err(err) => return internal_error(err),
        };
        let mut url = format!("{}{}", worker.base_url.trim_end_matches('/'), uri.path());
        if let Some(query) = uri.query() {
            url.push('?');
            url.push_str(query);
        }
        let part = match content_type {
            Some(content_type) => reqwest::multipart::Part::bytes(bytes.to_vec())
                .file_name(raw_filename.clone())
                .mime_str(&content_type)
                .unwrap_or_else(|_| {
                    reqwest::multipart::Part::bytes(bytes.to_vec()).file_name(raw_filename)
                }),
            None => reqwest::multipart::Part::bytes(bytes.to_vec()).file_name(raw_filename),
        };
        let form = reqwest::multipart::Form::new().part("file", part);
        let response = match state
            .http_client
            .post(url)
            .header(INTERNAL_CLUSTER_TOKEN_HEADER, token)
            .multipart(form)
            .send()
            .await
        {
            Ok(response) => response,
            Err(err) => return internal_error(PlatformError::new("FILES_UPLOAD", err.to_string())),
        };
        return reqwest_response_to_axum(response).await;
    }

    let layout = match state.platform.file.ensure_project_layout(&owner, &project) {
        Ok(l) => l,
        Err(e) => return internal_error(e),
    };
    let project_cfg = match state.platform.zebflow_cfg.read_or_default(&owner, &project) {
        Ok(config) => config,
        Err(err) => return internal_error(err),
    };
    let max_file_size_mb = project_cfg
        .configs
        .files
        .uploads
        .effective_max_file_size_mb();
    let max_bytes = (max_file_size_mb as u64) * 1024 * 1024;

    let rel = params
        .get("path")
        .map(|s| s.trim().trim_start_matches('/'))
        .unwrap_or("uploads")
        .to_string();
    let rel = match crate::zebfs::normalize_object_path(&rel) {
        Ok(path) => path,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok": false, "error": "invalid path"})),
            )
                .into_response();
        }
    };
    if rel.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": "path is required"})),
        )
            .into_response();
    }

    let field = match multipart.next_field().await {
        Ok(Some(field)) => field,
        Ok(None) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok": false, "error": "no file field in multipart body"})),
            )
                .into_response();
        }
        Err(err) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok": false, "error": err.to_string()})),
            )
                .into_response();
        }
    };

    let raw_filename = field
        .file_name()
        .map(|s| s.to_string())
        .or_else(|| field.name().map(|s| s.to_string()))
        .unwrap_or_else(|| "upload.bin".to_string());
    let content_type = field
        .content_type()
        .map(|value| value.to_string())
        .unwrap_or_else(|| "application/octet-stream".to_string());
    let filename = match sanitize_asset_filename(&raw_filename) {
        Some(name) => name,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok": false, "error": "invalid filename"})),
            )
                .into_response();
        }
    };

    let bytes = match field.bytes().await {
        Ok(bytes) => bytes,
        Err(err) => return internal_error(PlatformError::new("FILES_UPLOAD", err.to_string())),
    };
    if bytes.len() as u64 > max_bytes {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(json!({"ok": false, "error": format!("file size {} bytes exceeds limit of {} MB", bytes.len(), max_file_size_mb)})),
        )
            .into_response();
    }
    let entry_rel = format!("{rel}/{filename}");
    let zebfs = layout.open_files();
    if let Err(err) = zebfs.put(&entry_rel, bytes.as_ref()) {
        return internal_error(PlatformError::new("FILES_UPLOAD_WRITE", err.to_string()));
    }

    // An upload answers with the FileRef the pipeline nodes would receive, so
    // it is held to the same contract: eleven fields, `sha256` and `kind`
    // included — a FileRef missing its digest is invalid by its own document
    // (`kinds/file-ref/README.md`). `ok` is the HTTP envelope, not a field of
    // the ref.
    Json(json!({
        "ok": true,
        "__zf_type": crate::pipeline::nodes::basic::file_ref::FILE_REF_TYPE,
        "backend": crate::pipeline::nodes::basic::file_ref::BACKEND_ZEBFS,
        "ref": entry_rel,
        "filename": filename,
        "mime": content_type,
        "kind": crate::pipeline::nodes::basic::file_ref::infer_kind(&content_type, &filename),
        "size": bytes.len(),
        "sha256": format!("sha256:{:x}", <sha2::Sha256 as sha2::Digest>::digest(bytes.as_ref())),
        "lifecycle": crate::pipeline::nodes::basic::file_ref::LIFECYCLE_DURABLE,
        "origin": "project.files.upload",
        "trust": "user"
    }))
    .into_response()
}

/// POST /api/projects/{owner}/{project}/files/rm  { "path": "uploads/abc.jpg" }
async fn api_files_rm(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Json(body): Json<serde_json::Value>,
) -> Response {
    if let Err(r) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::FilesWrite,
    ) {
        return r;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_json_request_to_worker(
            &state,
            &uri,
            &headers,
            Method::POST,
            &body,
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }
    let layout = match state.platform.file.ensure_project_layout(&owner, &project) {
        Ok(l) => l,
        Err(e) => return internal_error(e),
    };

    let path_str = body
        .get("path")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim()
        .trim_start_matches('/')
        .to_string();
    let path_str = match crate::zebfs::normalize_object_path(&path_str) {
        Ok(path) => path,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"error":"invalid path"})),
            )
                .into_response();
        }
    };
    if path_str.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"error":"path is required"})),
        )
            .into_response();
    }

    let zebfs = layout.open_files();
    match zebfs.delete(&path_str) {
        Ok(_) => Json(json!({ "ok": true })).into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error": e.to_string()})),
        )
            .into_response(),
    }
}

#[derive(Deserialize, Serialize)]
struct FileAccessRequest {
    path: String,
    access: String,
    #[serde(default)]
    scope: Option<String>,
}

#[derive(Deserialize)]
struct FileAccessFormRequest {
    path: String,
    access: String,
    #[serde(default)]
    scope: Option<String>,
    #[serde(default)]
    return_to: Option<String>,
}

#[derive(Serialize)]
struct FileAccessResponse {
    ok: bool,
    path: String,
    access: String,
    public: bool,
    scope: String,
    url: String,
}

fn safe_files_return_to(owner: &str, project: &str, return_to: Option<&str>) -> String {
    let prefix = format!("/projects/{owner}/{project}/files");
    match return_to {
        Some(value) if value.starts_with(&prefix) => value.to_string(),
        _ => format!("{prefix}/default"),
    }
}

fn set_project_file_access(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
    req: &FileAccessRequest,
) -> Result<FileAccessResponse, Response> {
    let access = crate::zebfs::ZebFsAccess::parse(&req.access).ok_or_else(|| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": "access must be private or public_read"})),
        )
            .into_response()
    })?;
    let scope = match req.scope.as_deref() {
        Some(value) => crate::zebfs::ZebFsAclScope::parse(value).ok_or_else(|| {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok": false, "error": "scope must be object or prefix"})),
            )
                .into_response()
        })?,
        None => crate::zebfs::ZebFsAclScope::Object,
    };
    let layout = state
        .platform
        .file
        .ensure_project_layout(owner, project)
        .map_err(internal_error)?;
    let zebfs = layout.open_files();
    let path = crate::platform::services::zebfs_acl::set_access(&zebfs, &req.path, access, scope)
        .map_err(|err| {
        if err.code == "ZEBFS_INVALID_PATH" || err.code == "ZEBFS_RESERVED_PATH" {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok": false, "error": "invalid path"})),
            )
                .into_response()
        } else {
            internal_error(PlatformError::new(err.code, err.message))
        }
    })?;
    Ok(FileAccessResponse {
        ok: true,
        path: path.clone(),
        access: access.as_str().to_string(),
        public: access == crate::zebfs::ZebFsAccess::PublicRead,
        scope: scope.as_str().to_string(),
        url: format!("/fs/{owner}/{project}/{path}"),
    })
}

/// PUT /api/projects/{owner}/{project}/files/access
/// Body: { "path": "uploads/image.png", "access": "public_read", "scope": "object|prefix" }
async fn api_files_access(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Json(req): Json<FileAccessRequest>,
) -> Response {
    if let Err(r) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::FilesWrite,
    ) {
        return r;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_json_request_to_worker(
            &state,
            &uri,
            &headers,
            Method::PUT,
            &req,
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }

    match set_project_file_access(&state, &owner, &project, &req) {
        Ok(response) => Json(response).into_response(),
        Err(response) => response,
    }
}

/// POST /api/projects/{owner}/{project}/files/access
/// Form fallback for file browser controls when client hydration is unavailable.
async fn api_files_access_form(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Form(form): Form<FileAccessFormRequest>,
) -> Response {
    let return_to = safe_files_return_to(&owner, &project, form.return_to.as_deref());
    if let Err(r) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::FilesWrite,
    ) {
        return r;
    }

    let req = FileAccessRequest {
        path: form.path,
        access: form.access,
        scope: form.scope,
    };
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        let mut worker_headers = headers.clone();
        worker_headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));
        let response = match forward_project_json_request_to_worker(
            &state,
            &uri,
            &worker_headers,
            Method::PUT,
            &req,
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => return internal_error(err),
        };
        if response.status().is_success() {
            return Redirect::to(&return_to).into_response();
        }
        return response;
    }

    match set_project_file_access(&state, &owner, &project, &req) {
        Ok(_) => Redirect::to(&return_to).into_response(),
        Err(response) => response,
    }
}

async fn api_mapserver_sources_list(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, _instance)): Path<(String, String, String)>,
    uri: Uri,
) -> Response {
    if let Err(r) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::FilesRead,
    ) {
        return r;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::GET,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }
    match list_mapserver_source_files(&state, &owner, &project) {
        Ok(items) => Json(json!({ "items": items })).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_mapserver_layers_list(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, instance)): Path<(String, String, String)>,
    uri: Uri,
) -> Response {
    if let Err(r) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::FilesRead,
    ) {
        return r;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::GET,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }
    match read_mapserver_layers(&state, &owner, &project, &instance) {
        Ok(items) => Json(json!({ "items": items })).into_response(),
        Err(err) => internal_error(err),
    }
}

/// Operator view of a layer's column statistics.
///
/// The public `/ms/.../stats` endpoint reports only the columns named in
/// `allowed_properties` (contract `MapPublishManifest`). Choosing which columns
/// to expose needs the full list, so the publish UI reads it here, behind the
/// project's own read capability.
async fn api_mapserver_layer_stats(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, instance, layer_id)): Path<(String, String, String, String)>,
    uri: Uri,
) -> Response {
    if let Err(r) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::FilesRead,
    ) {
        return r;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::GET,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }
    let items = match read_mapserver_layers(&state, &owner, &project, &instance) {
        Ok(items) => items,
        Err(err) => return internal_error(err),
    };
    let Some(item) = items.into_iter().find(|item| item.layer_id == layer_id) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"ok": false, "error": "map layer not found"})),
        )
            .into_response();
    };
    let source_path = item.source_path.clone();
    let manifest = match mapserver_record_to_manifest(&state, &owner, &project, item) {
        Ok(manifest) => manifest,
        Err(err) => return internal_error(err),
    };
    // An artifact-backed layer has no single file to scan — its `source_ref` is
    // the generated chunk manifest. The columns the operator is choosing from
    // are the ones in the GeoJSON it was built from, so read them there. The
    // public endpoint deliberately does not do this: that scan is bounded work
    // an unauthenticated caller must not be able to ask for.
    let manifest = if manifest.source_kind
        == crate::mapserver::publish::manifest::SourceKind::GeoJsonArtifact
    {
        match state.platform.file.ensure_project_layout(&owner, &project) {
            Ok(layout) => crate::mapserver::publish::manifest::PublishedLayerManifest {
                source_kind: crate::mapserver::publish::manifest::SourceKind::GeoJsonFile,
                source_ref: layout
                    .files_dir
                    .join(source_path.trim_start_matches('/'))
                    .display()
                    .to_string(),
                ..manifest
            },
            Err(err) => return internal_error(err),
        }
    } else {
        manifest
    };
    match crate::mapserver::resolve::stats::compute_layer_stats(
        &manifest,
        crate::mapserver::resolve::stats::StatsAudience::Operator,
    ) {
        Ok(stats) => Json(stats).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"ok": false, "error": format!("stats computation failed: {err}")})),
        )
            .into_response(),
    }
}

async fn api_mapserver_layers_publish(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, instance)): Path<(String, String, String)>,
    uri: Uri,
    Json(body): Json<serde_json::Value>,
) -> Response {
    if let Err(r) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::FilesWrite,
    ) {
        return r;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_json_request_to_worker(
            &state,
            &uri,
            &headers,
            Method::POST,
            &body,
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }
    let layer_id = body
        .get("layer_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("");
    let path = body
        .get("path")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("");
    let source_path = body
        .get("source_path")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("");
    if layer_id.is_empty() || path.is_empty() || source_path.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": "layer_id, path, and source_path are required"})),
        )
            .into_response();
    }
    let source_path = match crate::zebfs::normalize_object_path(source_path) {
        Ok(path) if path.starts_with("mapserver/") => path,
        _ => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok": false, "error": "source_path must be under mapserver/"})),
            )
                .into_response();
        }
    };
    let layout = match state.platform.file.ensure_project_layout(&owner, &project) {
        Ok(layout) => layout,
        Err(err) => return internal_error(err),
    };
    let source_abs_path = layout.files_dir.join(&source_path);
    if !source_abs_path.exists() || !source_abs_path.is_file() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": "source_path file not found"})),
        )
            .into_response();
    }
    let allowed_properties = body
        .get("allowed_properties")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(ToString::to_string))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let min_zoom = body
        .get("min_zoom")
        .and_then(Value::as_u64)
        .and_then(|v| u8::try_from(v).ok());
    let max_zoom = body
        .get("max_zoom")
        .and_then(Value::as_u64)
        .and_then(|v| u8::try_from(v).ok());
    if let (Some(min_zoom), Some(max_zoom)) = (min_zoom, max_zoom) {
        if min_zoom > max_zoom {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok": false, "error": "min_zoom must be <= max_zoom"})),
            )
                .into_response();
        }
    }
    let artifact_root = match mapserver_artifacts_root(&state, &owner, &project, &instance) {
        Ok(path) => path,
        Err(err) => return internal_error(err),
    };
    let safe_layer_id = layer_id
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    let artifact_abs_dir = artifact_root.join(&safe_layer_id);
    let artifact_rel_dir = format!("mapserver-artifacts/{instance}/{safe_layer_id}");
    let build = match crate::mapserver::publish::build::build_geojson_artifact(
        &source_abs_path,
        layer_id,
        &artifact_abs_dir,
        &artifact_rel_dir,
    ) {
        Ok(build) => build,
        Err(err) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok": false, "error": err})),
            )
                .into_response();
        }
    };
    let mut items = match read_mapserver_layers(&state, &owner, &project, &instance) {
        Ok(items) => items,
        Err(err) => return internal_error(err),
    };
    let record = MapserverLayerRecord {
        layer_id: layer_id.to_string(),
        // Contract `MapPublishManifest`: stored without a leading slash, the
        // same shape `n.ms.publish` writes.
        path: crate::mapserver::publish::registry::normalize_layer_path(path).to_string(),
        source_path,
        source_kind: "geojson_artifact".to_string(),
        artifact_manifest_path: Some(build.manifest_rel_path.clone()),
        mode: "features".to_string(),
        min_zoom,
        max_zoom,
        bbox_required: body
            .get("bbox_required")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        max_features: body
            .get("max_features")
            .and_then(Value::as_u64)
            .map(|v| v as usize)
            .unwrap_or(1000),
        allowed_properties,
        feature_count: Some(build.feature_count),
        chunk_count: Some(build.chunk_count),
        style: None,
        filter: None,
        function_slug: None,
        cache_ttl_secs: None,
    };
    items.retain(|item| item.layer_id != record.layer_id);
    items.push(record.clone());
    items.sort_by(|a, b| a.layer_id.cmp(&b.layer_id));
    if let Err(err) = write_mapserver_layers(&state, &owner, &project, &instance, &items) {
        return internal_error(err);
    }
    Json(json!({
        "ok": true,
        "item": record,
        "public_url": format!("/ms/{owner}/{project}/{}", record.path)
    }))
    .into_response()
}

async fn api_mapserver_layers_delete(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, instance, layer_id)): Path<(String, String, String, String)>,
    uri: Uri,
) -> Response {
    if let Err(r) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::FilesWrite,
    ) {
        return r;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::DELETE,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }
    let mut items = match read_mapserver_layers(&state, &owner, &project, &instance) {
        Ok(items) => items,
        Err(err) => return internal_error(err),
    };
    let removed = items.iter().find(|item| item.layer_id == layer_id).cloned();
    let before = items.len();
    items.retain(|item| item.layer_id != layer_id);
    if before == items.len() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"ok": false, "error": "layer not found"})),
        )
            .into_response();
    }
    if let Err(err) = write_mapserver_layers(&state, &owner, &project, &instance, &items) {
        return internal_error(err);
    }
    if let Some(removed) = removed {
        if let Some(artifact_rel) = removed.artifact_manifest_path {
            if let Ok(layout) = state.platform.file.ensure_project_layout(&owner, &project) {
                // Move a pre-tier artifact tree to its cache home first so the
                // cleanup hits the tree wherever it actually lives.
                let _ = layout.ensure_mapserver_artifacts_home();
                let artifact_manifest = layout.resolve_mapserver_artifact_path(&artifact_rel);
                if let Some(dir) = artifact_manifest.parent() {
                    let _ = std::fs::remove_dir_all(dir);
                }
            }
        }
    }
    Json(json!({"ok": true})).into_response()
}

async fn api_list_credential_types(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::CredentialsRead,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::GET,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    // Built-in types + custom types from installed composite/WASM packages.
    let mut types = crate::platform::services::credential::builtin_credential_types();
    for ct in state
        .platform
        .node_registry
        .all_package_credential_types(&owner, &project)
    {
        if !types.iter().any(|t| t.kind == ct.kind) {
            types.push(ct);
        }
    }
    Json(json!({"ok": true, "types": types})).into_response()
}

async fn api_list_credentials(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::CredentialsRead,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::GET,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    match state
        .platform
        .credentials
        .list_project_credentials(&owner, &project)
    {
        Ok(items) => Json(json!({"ok": true, "items": items})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_get_credential(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, credential_id)): Path<(String, String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::CredentialsRead,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::GET,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    match state
        .platform
        .credentials
        .get_project_credential(&owner, &project, &credential_id)
    {
        Ok(Some(credential)) => Json(json!({"ok": true, "credential": credential})).into_response(),
        Ok(None) => (StatusCode::NOT_FOUND, Json(json!({"ok": false, "error": {"code":"PLATFORM_CREDENTIAL_MISSING","message":"credential not found"}}))).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_upsert_credential(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Json(req): Json<UpsertProjectCredentialRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::CredentialsWrite,
    ) {
        return response;
    }
    match maybe_forward_project_json_to_worker(
        &state,
        &uri,
        &headers,
        Method::POST,
        &req,
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    match state
        .platform
        .credentials
        .upsert_project_credential(&owner, &project, &req)
    {
        Ok(credential) => Json(json!({"ok": true, "credential": credential})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_upsert_credential_by_path(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, credential_id)): Path<(String, String, String)>,
    uri: Uri,
    Json(mut req): Json<UpsertProjectCredentialRequest>,
) -> Response {
    req.credential_id = credential_id;
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::CredentialsWrite,
    ) {
        return response;
    }
    match maybe_forward_project_json_to_worker(
        &state,
        &uri,
        &headers,
        Method::PUT,
        &req,
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    match state
        .platform
        .credentials
        .upsert_project_credential(&owner, &project, &req)
    {
        Ok(credential) => Json(json!({"ok": true, "credential": credential})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_delete_credential(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, credential_id)): Path<(String, String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::CredentialsWrite,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::DELETE,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    match state
        .platform
        .credentials
        .delete_project_credential(&owner, &project, &credential_id)
    {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => internal_error(err),
    }
}

// ── OAuth2 credential flow ────────────────────────────────────────────────────

/// Initiate OAuth2 authorization — returns the provider's authorize URL.
async fn api_oauth2_authorize(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, credential_id)): Path<(String, String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::CredentialsWrite,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::GET,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    let credential =
        match state
            .platform
            .credentials
            .get_project_credential(&owner, &project, &credential_id)
        {
            Ok(Some(c)) => c,
            Ok(None) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(serde_json::json!({"error": "credential not found"})),
                )
                    .into_response();
            }
            Err(err) => return internal_error(err),
        };
    if credential.kind != "oauth2" {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "credential is not oauth2"})),
        )
            .into_response();
    }
    let secret = &credential.secret;
    let authorize_url = secret
        .get("authorize_url")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let client_id = secret
        .get("client_id")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let scopes = secret
        .get("scopes")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();

    if authorize_url.is_empty() || client_id.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "authorize_url and client_id are required"})),
        )
            .into_response();
    }

    // redirect_uri is stored per-credential (each project/domain can differ).
    // Falls back to derived URL for dev convenience.
    let redirect_uri = secret
        .get("redirect_uri")
        .and_then(serde_json::Value::as_str)
        .filter(|v| !v.trim().is_empty())
        .map(|v| v.to_string())
        .unwrap_or_else(|| {
            let base = platform_base_url(&headers);
            format!("{base}/oauth/callback")
        });
    let oauth_state = sign_oauth_state(&state.oauth_state_secret, &owner, &project, &credential_id);

    let mut url = format!(
        "{authorize_url}?client_id={}&redirect_uri={}&response_type=code&state={}",
        url_query_encode(client_id),
        url_query_encode(&redirect_uri),
        url_query_encode(&oauth_state),
    );
    if !scopes.is_empty() {
        url.push_str(&format!("&scope={}", url_query_encode(scopes)));
    }
    // Request offline access for refresh tokens (Google-specific but harmless elsewhere)
    url.push_str("&access_type=offline&prompt=consent");

    Json(serde_json::json!({ "redirect_url": url })).into_response()
}

#[derive(serde::Deserialize)]
struct OAuthCallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    #[allow(dead_code)]
    error_description: Option<String>,
}

/// OAuth2 callback handler — unauthenticated. Provider redirects browser here.
async fn oauth2_callback_handler(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    uri: Uri,
    axum::extract::Query(query): axum::extract::Query<OAuthCallbackQuery>,
) -> Response {
    // Provider error (user denied access)
    if query.error.is_some() {
        return axum::response::Redirect::to("/home?oauth=denied").into_response();
    }

    let Some(oauth_state) = &query.state else {
        return axum::response::Redirect::to("/home?oauth=error").into_response();
    };
    let Some(code) = &query.code else {
        return axum::response::Redirect::to("/home?oauth=error").into_response();
    };

    let (owner, project, credential_id) =
        match verify_oauth_state(&state.oauth_state_secret, oauth_state) {
            Ok(v) => v,
            Err(_) => {
                return axum::response::Redirect::to("/home?oauth=error").into_response();
            }
        };

    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::GET,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(_) => {
            return axum::response::Redirect::to("/home?oauth=error").into_response();
        }
    }

    // Read redirect_uri from the credential's secret (per-credential, per-domain).
    let redirect_uri =
        match state
            .platform
            .credentials
            .get_project_credential(&owner, &project, &credential_id)
        {
            Ok(Some(cred)) => cred
                .secret
                .get("redirect_uri")
                .and_then(serde_json::Value::as_str)
                .filter(|v| !v.trim().is_empty())
                .map(|v| v.to_string())
                .unwrap_or_else(|| {
                    let base = platform_base_url(&headers);
                    format!("{base}/oauth/callback")
                }),
            _ => {
                let base = platform_base_url(&headers);
                format!("{base}/oauth/callback")
            }
        };

    match state
        .platform
        .credentials
        .exchange_oauth2_code(&owner, &project, &credential_id, code, &redirect_uri)
        .await
    {
        Ok(()) => {
            let target = format!("/projects/{owner}/{project}/credentials?oauth=success");
            axum::response::Redirect::to(&target).into_response()
        }
        Err(_err) => {
            let target = format!("/projects/{owner}/{project}/credentials?oauth=error");
            axum::response::Redirect::to(&target).into_response()
        }
    }
}

fn sign_oauth_state(secret: &[u8; 32], owner: &str, project: &str, credential_id: &str) -> String {
    use base64::Engine as _;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let now = crate::platform::model::now_ts();
    let payload = serde_json::json!({"o": owner, "p": project, "c": credential_id, "t": now});
    let payload_b64 =
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(payload.to_string().as_bytes());

    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret).expect("hmac key size");
    mac.update(payload_b64.as_bytes());
    let sig = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());

    format!("{payload_b64}.{sig}")
}

fn verify_oauth_state(secret: &[u8; 32], state: &str) -> Result<(String, String, String), String> {
    use base64::Engine as _;
    use hmac::{Hmac, Mac};
    use sha2::Sha256;

    let parts: Vec<&str> = state.splitn(2, '.').collect();
    if parts.len() != 2 {
        return Err("invalid state format".to_string());
    }
    let (payload_b64, sig_b64) = (parts[0], parts[1]);

    // Verify HMAC
    type HmacSha256 = Hmac<Sha256>;
    let mut mac = HmacSha256::new_from_slice(secret).expect("hmac key size");
    mac.update(payload_b64.as_bytes());
    let sig_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(sig_b64)
        .map_err(|e| format!("bad sig encoding: {e}"))?;
    mac.verify_slice(&sig_bytes)
        .map_err(|_| "invalid signature".to_string())?;

    // Decode payload
    let payload_bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload_b64)
        .map_err(|e| format!("bad payload encoding: {e}"))?;
    let payload: serde_json::Value =
        serde_json::from_slice(&payload_bytes).map_err(|e| format!("bad payload json: {e}"))?;

    let owner = payload
        .get("o")
        .and_then(serde_json::Value::as_str)
        .ok_or("missing owner")?
        .to_string();
    let project = payload
        .get("p")
        .and_then(serde_json::Value::as_str)
        .ok_or("missing project")?
        .to_string();
    let credential_id = payload
        .get("c")
        .and_then(serde_json::Value::as_str)
        .ok_or("missing credential_id")?
        .to_string();
    let timestamp = payload
        .get("t")
        .and_then(serde_json::Value::as_i64)
        .ok_or("missing timestamp")?;

    // Reject states older than 10 minutes
    let now = crate::platform::model::now_ts();
    if (now - timestamp).abs() > 600 {
        return Err("state expired".to_string());
    }

    Ok((owner, project, credential_id))
}

async fn api_get_project_assistant_config(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::SettingsRead,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::GET,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    match state
        .platform
        .assistant_configs
        .get_project_assistant_config(&owner, &project)
    {
        Ok(config) => Json(json!({"ok": true, "config": config})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_upsert_project_assistant_config(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Json(req): Json<UpsertProjectAssistantConfigRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::SettingsWrite,
    ) {
        return response;
    }
    match maybe_forward_project_json_to_worker(
        &state,
        &uri,
        &headers,
        Method::PUT,
        &req,
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    match state
        .platform
        .assistant_configs
        .upsert_project_assistant_config(&owner, &project, &req)
    {
        Ok(config) => Json(json!({"ok": true, "config": config})).into_response(),
        Err(err) => internal_error(err),
    }
}

/// `GET /api/projects/{owner}/{project}/settings/{section}` — read one zebflow.yaml section.
///
/// Supported sections: `profile`, `rwe`, `logging`, `assets`, and `distribution`.
async fn api_get_settings_section(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, section)): Path<(String, String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::SettingsRead,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::GET,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }
    let cfg = match state.platform.zebflow_cfg.read_or_default(&owner, &project) {
        Ok(config) => config,
        Err(err) => return internal_error(err),
    };
    match section.as_str() {
        "profile" => Json(json!({
            "ok": true,
            "section": "profile",
            "data": cfg.metadata
        }))
        .into_response(),
        "rwe" => {
            Json(json!({"ok": true, "section": "rwe", "data": cfg.configs.rwe})).into_response()
        }
        "logging" => {
            Json(json!({"ok": true, "section": "logging", "data": cfg.configs.pipelines.logging}))
                .into_response()
        }
        "assets" => Json(json!({
            "ok": true,
            "section": "assets",
            "data": {
                "max_asset_size_mb": cfg.configs.files.uploads.effective_max_asset_size_mb(),
                "max_file_size_mb": cfg.configs.files.uploads.effective_max_file_size_mb(),
                "webhook_body_max_mb": cfg.configs.files.uploads.effective_webhook_body_max_mb(),
                "pipeline_node_timeout_secs": cfg.configs.pipelines.effective_node_timeout_secs()
            }
        }))
        .into_response(),
        "distribution" => Json(json!({
            "ok": true,
            "section": "distribution",
            "data": cfg.distribution.hub
        }))
        .into_response(),
        "addressing" => match addressing_section_json(&state, &owner, &project, &cfg) {
            Ok(data) => Json(json!({ "ok": true, "section": "addressing", "data": data })).into_response(),
            Err(err) => internal_error(err),
        },
        _ => (
            StatusCode::NOT_FOUND,
            Json(json!({"ok": false, "error": format!("unknown settings section '{section}'")})),
        )
            .into_response(),
    }
}

/// The Addressing section as the Studio and the API read it: what the
/// operator set, the dev host every project has, and the proxy configs
/// generated from it (`docs/contracts/addressing.md` §6).
fn addressing_section_json(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
    cfg: &crate::platform::model::ZebflowJson,
) -> Result<Value, PlatformError> {
    use crate::platform::services::addressing::{AddressingService, ProxyFacts, Surface};
    let addressing = state.platform.addressing.read(owner, project)?;
    let dev_host = AddressingService::dev_host(owner, project);
    let port = crate::platform::boot::configured_port();
    let upstream = format!("127.0.0.1:{port}");
    let upload_mb = cfg.configs.files.uploads.effective_max_file_size_mb().max(cfg.configs.files.uploads.effective_webhook_body_max_mb());
    let configs: Vec<Value> = crate::platform::services::addressing::server_configs(&ProxyFacts {
        hosts: &addressing.hosts,
        upstream: &upstream,
        upload_mb,
        owner,
        project,
    })
    .into_iter()
    .map(|(id, title, text)| json!({ "id": id, "title": title, "text": text }))
    .collect();
    let surfaces: Vec<Value> = Surface::ALL
        .iter()
        .map(|s| {
            json!({
                "key": s.key(),
                "title": s.title(),
                "enabled": addressing.is_enabled(*s),
                "default_path": s.default_path(),
                "platform_path": s.platform_prefix(owner, project),
            })
        })
        .collect();
    Ok(json!({
        "config": addressing,
        "dev_host": dev_host,
        "dev_url": format!("http://{dev_host}:{port}/"),
        "neutral_url": format!("/wh/{owner}/{project}"),
        "upstream": upstream,
        "upload_mb": upload_mb,
        "surfaces": surfaces,
        "configs": configs,
    }))
}

#[derive(Debug, Deserialize)]
struct AddressingCheckRequest {
    host: String,
}

/// Does the world reach this project at `host`? Two facts, checked from the
/// instance: what DNS says the host is, and what answers at it — verified by
/// the `x-zebflow-project` header the addressing gate puts on every response
/// it served for a project host. A proxy that drops the Host header shows up
/// here as "answers, but not this project".
async fn api_check_addressing_host(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Json(req): Json<AddressingCheckRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::SettingsRead,
    ) {
        return response;
    }
    let host = crate::platform::services::addressing::normalize_host(&req.host);
    if host.is_empty() {
        return (StatusCode::BAD_REQUEST, Json(json!({"ok": false, "error": "host is required"}))).into_response();
    }
    let expected = format!("{}/{}", slug_segment(&owner), slug_segment(&project));
    let dns_host = host.clone();
    let resolved: Vec<String> = tokio::task::spawn_blocking(move || {
        use std::net::ToSocketAddrs;
        (dns_host.as_str(), 443u16)
            .to_socket_addrs()
            .map(|addrs| addrs.map(|a| a.ip().to_string()).collect::<Vec<_>>())
            .unwrap_or_default()
    })
    .await
    .unwrap_or_default();
    let mut verify = json!({ "ok": false, "tried": [] });
    if !resolved.is_empty() {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(8))
            .build();
        if let Ok(client) = client {
            let mut tried = Vec::new();
            for scheme in ["https", "http"] {
                let url = format!("{scheme}://{host}/");
                match client.get(&url).send().await {
                    Ok(resp) => {
                        let served = resp
                            .headers()
                            .get("x-zebflow-project")
                            .and_then(|v| v.to_str().ok())
                            .unwrap_or("")
                            .to_string();
                        let ok = served == expected;
                        tried.push(json!({ "url": url, "status": resp.status().as_u16(), "project": served }));
                        if ok {
                            verify = json!({ "ok": true, "url": format!("{scheme}://{host}/"), "status": resp.status().as_u16(), "tried": tried });
                            break;
                        }
                        if scheme == "http" {
                            verify = json!({ "ok": false, "tried": tried, "reason": if served.is_empty() { "answers, but not from Zebflow — is the proxy passing the Host header?" } else { "answers for another project" } });
                        }
                    }
                    Err(err) => {
                        tried.push(json!({ "url": url, "error": err.to_string() }));
                        if scheme == "http" {
                            verify = json!({ "ok": false, "tried": tried, "reason": "nothing answered — the proxy is not up yet, or a firewall is in the way" });
                        }
                    }
                }
            }
        }
    } else {
        verify = json!({ "ok": false, "tried": [], "reason": "the name does not resolve yet" });
    }
    Json(json!({
        "ok": true,
        "host": host,
        "expected_project": expected,
        "dns": { "resolved": resolved, "ok": !resolved.is_empty() },
        "verify": verify,
    }))
    .into_response()
}

async fn api_project_invocation_log_stats(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::SettingsRead,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::GET,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }
    match state
        .platform
        .data
        .get_pipeline_invocation_log_stats(&owner, &project)
    {
        Ok(stats) => Json(json!({"ok": true, "stats": stats})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_clear_project_invocation_logs(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::SettingsWrite,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::DELETE,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }
    let pipeline = params
        .get("pipeline")
        .map(|value| value.trim())
        .filter(|value| !value.is_empty());
    match state
        .platform
        .data
        .clear_pipeline_invocation_logs(&owner, &project, pipeline)
    {
        Ok(deleted) => Json(json!({"ok": true, "deleted": deleted})).into_response(),
        Err(err) => internal_error(err),
    }
}

fn optional_bounded_settings_u64(
    data: &serde_json::Value,
    field: &str,
    minimum: u64,
    maximum: u64,
) -> Result<Option<u64>, Response> {
    let Some(value) = data.get(field) else {
        return Ok(None);
    };
    let Some(value) = value.as_u64() else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "ok": false,
                "error": format!("{field} must be an integer between {minimum} and {maximum}")
            })),
        )
            .into_response());
    };
    if !(minimum..=maximum).contains(&value) {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "ok": false,
                "error": format!("{field} must be between {minimum} and {maximum}")
            })),
        )
            .into_response());
    }
    Ok(Some(value))
}

fn reject_unknown_settings_fields(
    data: &serde_json::Value,
    allowed: &[&str],
) -> Result<(), Response> {
    let Some(object) = data.as_object() else {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": "settings data must be an object"})),
        )
            .into_response());
    };
    if let Some(field) = object
        .keys()
        .find(|field| !allowed.contains(&field.as_str()))
    {
        return Err((
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": format!("unknown settings field '{field}'")})),
        )
            .into_response());
    }
    Ok(())
}

/// `PUT /api/projects/{owner}/{project}/settings/{section}` — write one zebflow.yaml section
/// and commit the change.
///
/// Body: `{ "commit_message": "...", "data": { ...section fields } }`.
/// After writing, stages `zebflow.yaml` and runs `git commit` in the project repo.
/// Returns `{ ok, section, data, committed, git_error? }`.
async fn api_upsert_settings_section(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, section)): Path<(String, String, String)>,
    uri: Uri,
    Json(req): Json<UpdateSettingsSectionRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::SettingsWrite,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_json_request_to_worker(
            &state,
            &uri,
            &headers,
            Method::PUT,
            &req,
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }

    // Sections stored in `zebflow.yaml` are committed with a message;
    // `addressing` is instance configuration and has nothing to commit.
    if section != "addressing" && req.commit_message.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": "commit_message must not be empty"})),
        )
            .into_response();
    }

    let section_data = match section.as_str() {
        "profile" => {
            #[derive(serde::Deserialize)]
            #[serde(deny_unknown_fields)]
            struct ProfilePayload {
                #[serde(default)]
                title: String,
                #[serde(default)]
                description: String,
            }
            let payload: ProfilePayload = match serde_json::from_value(req.data.clone()) {
                Ok(payload) => payload,
                Err(err) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"ok": false, "error": err.to_string()})),
                    )
                        .into_response();
                }
            };
            match state.platform.zebflow_cfg.update(&owner, &project, |cfg| {
                cfg.metadata.title = payload.title.trim().to_string();
                cfg.metadata.description = payload.description.trim().to_string();
            }) {
                Ok(cfg) => json!(cfg.metadata),
                Err(err) => return internal_error(err),
            }
        }
        "rwe" => {
            #[derive(serde::Deserialize)]
            #[serde(deny_unknown_fields)]
            struct RwePayload {
                #[serde(default)]
                allow_list: Vec<String>,
                #[serde(default)]
                minify_html: bool,
                #[serde(default = "crate::platform::model::default_rwe_strict_mode")]
                strict_mode: bool,
                #[serde(default)]
                deployment_asset_base: Option<String>,
            }
            let payload: RwePayload = match serde_json::from_value(req.data.clone()) {
                Ok(p) => p,
                Err(e) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"ok": false, "error": e.to_string()})),
                    )
                        .into_response();
                }
            };
            let updated = state.platform.zebflow_cfg.update(&owner, &project, |cfg| {
                cfg.configs.rwe.allow_list = payload.allow_list;
                cfg.configs.rwe.minify_html = payload.minify_html;
                cfg.configs.rwe.strict_mode = payload.strict_mode;
                cfg.configs.rwe.deployment_asset_base =
                    payload.deployment_asset_base.and_then(|value| {
                        let value = value.trim();
                        (!value.is_empty()).then(|| value.to_string())
                    });
            });
            match updated {
                Ok(cfg) => json!(cfg.configs.rwe),
                Err(err) => return internal_error(err),
            }
        }
        "logging" => {
            if let Err(response) = reject_unknown_settings_fields(
                &req.data,
                &["max_invocations", "trace_capture"],
            ) {
                return response;
            }
            let max_inv = match optional_bounded_settings_u64(&req.data, "max_invocations", 1, 1000)
            {
                Ok(value) => value.map(|value| value as u32),
                Err(response) => return response,
            };
            // A present object replaces capture overrides; null resets them to
            // engine defaults. An absent field preserves the existing setting.
            let capture = match req.data.get("trace_capture") {
                None => None,
                Some(value) => {
                    let parsed = serde_json::from_value::<Option<
                        crate::pipeline::trace_capture::TraceCaptureSettings,
                    >>(value.clone());
                    match parsed.and_then(|capture| {
                        if let Some(settings) = &capture {
                            settings
                                .validate()
                                .map_err(<serde_json::Error as serde::de::Error>::custom)?;
                        }
                        Ok(capture)
                    }) {
                        Ok(capture) => Some(capture),
                        Err(error) => {
                            return (
                                StatusCode::BAD_REQUEST,
                                Json(json!({"ok": false, "error": format!("trace_capture: {error}")})),
                            )
                                .into_response();
                        }
                    }
                }
            };
            match state.platform.zebflow_cfg.update(&owner, &project, |cfg| {
                if req.data.get("max_invocations").is_some() {
                    cfg.configs.pipelines.logging.max_invocations = max_inv;
                }
                if let Some(capture) = capture {
                    cfg.configs.pipelines.logging.trace_capture = capture;
                }
            }) {
                Ok(cfg) => json!(cfg.configs.pipelines.logging),
                Err(err) => return internal_error(err),
            }
        }
        "assets" => {
            if let Err(response) = reject_unknown_settings_fields(
                &req.data,
                &[
                    "max_asset_size_mb",
                    "max_file_size_mb",
                    "webhook_body_max_mb",
                    "pipeline_node_timeout_secs",
                ],
            ) {
                return response;
            }
            let upload_maximum = crate::platform::model::MAX_UPLOAD_SIZE_MB as u64;
            let max_mb = match optional_bounded_settings_u64(
                &req.data,
                "max_asset_size_mb",
                5,
                upload_maximum,
            ) {
                Ok(value) => value.map(|value| value as u32),
                Err(response) => return response,
            };
            let max_file_size_mb = match optional_bounded_settings_u64(
                &req.data,
                "max_file_size_mb",
                5,
                upload_maximum,
            ) {
                Ok(value) => value.map(|value| value as u32),
                Err(response) => return response,
            };
            let webhook_body_max_mb = match optional_bounded_settings_u64(
                &req.data,
                "webhook_body_max_mb",
                100,
                upload_maximum,
            ) {
                Ok(value) => value.map(|value| value as u32),
                Err(response) => return response,
            };
            let node_timeout_secs = match optional_bounded_settings_u64(
                &req.data,
                "pipeline_node_timeout_secs",
                5,
                3600,
            ) {
                Ok(value) => value,
                Err(response) => return response,
            };
            let updated = state.platform.zebflow_cfg.update(&owner, &project, |cfg| {
                if let Some(value) = max_mb {
                    cfg.configs.files.uploads.max_asset_size_mb = value;
                }
                if let Some(value) = max_file_size_mb {
                    cfg.configs.files.uploads.max_file_size_mb = Some(value);
                }
                if let Some(value) = webhook_body_max_mb {
                    cfg.configs.files.uploads.webhook_body_max_mb = value;
                }
                if let Some(value) = node_timeout_secs {
                    cfg.configs.pipelines.node_timeout_secs = Some(value);
                }
            });
            match updated {
                Ok(cfg) => json!({
                    "max_asset_size_mb": cfg.configs.files.uploads.effective_max_asset_size_mb(),
                    "max_file_size_mb": cfg.configs.files.uploads.effective_max_file_size_mb(),
                    "webhook_body_max_mb": cfg.configs.files.uploads.effective_webhook_body_max_mb(),
                    "pipeline_node_timeout_secs": cfg.configs.pipelines.effective_node_timeout_secs()
                }),
                Err(err) => return internal_error(err),
            }
        }
        "addressing" => {
            // Instance configuration, not `zebflow.yaml`: nothing to commit.
            let payload: crate::platform::services::addressing::ProjectAddressing =
                match serde_json::from_value(req.data.clone()) {
                    Ok(payload) => payload,
                    Err(err) => {
                        return (
                            StatusCode::BAD_REQUEST,
                            Json(json!({"ok": false, "error": err.to_string()})),
                        )
                            .into_response();
                    }
                };
            return match state.platform.addressing.write(&owner, &project, payload) {
                Ok(_) => {
                    let cfg = match state.platform.zebflow_cfg.read_or_default(&owner, &project) {
                        Ok(config) => config,
                        Err(err) => return internal_error(err),
                    };
                    match addressing_section_json(&state, &owner, &project, &cfg) {
                        Ok(data) => Json(json!({ "ok": true, "section": "addressing", "data": data })).into_response(),
                        Err(err) => internal_error(err),
                    }
                }
                Err(err) if err.code.starts_with("ADDRESSING_") => (
                    StatusCode::BAD_REQUEST,
                    Json(json!({ "ok": false, "error": { "code": err.code, "message": err.message } })),
                )
                    .into_response(),
                Err(err) => internal_error(err),
            };
        }
        "distribution" => {
            #[derive(serde::Deserialize)]
            #[serde(deny_unknown_fields)]
            struct DistributionPayload {
                #[serde(default)]
                entry_url: String,
                #[serde(default)]
                as_app: bool,
            }
            let payload: DistributionPayload = match serde_json::from_value(req.data.clone()) {
                Ok(payload) => payload,
                Err(err) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        Json(json!({"ok": false, "error": err.to_string()})),
                    )
                        .into_response();
                }
            };
            match state.platform.zebflow_cfg.update(&owner, &project, |cfg| {
                cfg.distribution.hub.entry_url = payload.entry_url.trim().to_string();
                cfg.distribution.hub.as_app = payload.as_app;
            }) {
                Ok(cfg) => json!(cfg.distribution.hub),
                Err(err) => return internal_error(err),
            }
        }
        _ => {
            return (
                StatusCode::NOT_FOUND,
                Json(
                    json!({"ok": false, "error": format!("unknown settings section '{section}'")}),
                ),
            )
                .into_response();
        }
    };

    // Git: stage zebflow.yaml and commit with the user-provided message.
    // Uses the acting platform user's Git identity when committing settings changes.
    // Failure is non-fatal — settings are already saved; we report the git outcome.
    let (committed, git_error) = {
        let owner_slug = crate::platform::model::slug_segment(&owner);
        let project_slug = crate::platform::model::slug_segment(&project);
        let actor_user = session_owner(&state, &headers);
        let identity_args = git_identity_args(&state, actor_user.as_deref(), &project_slug);
        match state
            .platform
            .file
            .ensure_project_layout(&owner_slug, &project_slug)
        {
            Err(_) => (false, Some("could not resolve project layout".to_string())),
            Ok(layout) => {
                let add_ok = std::process::Command::new("git")
                    .arg("-C")
                    .arg(&layout.repo_dir)
                    .arg("add")
                    .arg(crate::contracts::kinds::PROJECT_CONFIGURATION_FILE)
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false);

                if !add_ok {
                    (false, Some("git add failed".to_string()))
                } else {
                    let mut commit_cmd = std::process::Command::new("git");
                    for arg in &identity_args {
                        commit_cmd.arg(arg);
                    }
                    commit_cmd
                        .arg("-C")
                        .arg(&layout.repo_dir)
                        .arg("commit")
                        .arg("-m")
                        .arg(req.commit_message.trim());
                    let commit_out = commit_cmd.output();
                    match commit_out {
                        Err(e) => (false, Some(e.to_string())),
                        Ok(o) => {
                            if o.status.success() {
                                (true, None)
                            } else {
                                let msg = String::from_utf8_lossy(&o.stderr).to_string();
                                // "nothing to commit" is not an error — settings were already saved.
                                if msg.contains("nothing to commit") {
                                    (false, None)
                                } else {
                                    (false, Some(msg.trim().to_string()))
                                }
                            }
                        }
                    }
                }
            }
        }
    };

    let mut resp = json!({
        "ok": true,
        "section": section,
        "data": section_data,
        "committed": committed
    });
    if let Some(err) = git_error {
        resp["git_error"] = json!(err);
    }
    Json(resp).into_response()
}

// ─── RWE Library API ─────────────────────────────────────────────────────────

async fn api_project_dependency_status(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::LibrariesRead,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::GET,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    let requested = match state
        .platform
        .zebflow_cfg
        .get_rwe_libraries(&owner, &project)
    {
        Ok(value) => value,
        Err(err) => return internal_error(err),
    };
    match state
        .platform
        .dependency_lock
        .status(&owner, &project, &requested)
    {
        Ok(report) => Json(json!({"ok": true, "report": report})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_repair_project_dependencies(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::LibrariesInstall,
    ) {
        return response;
    }
    match maybe_forward_project_json_to_worker(
        &state,
        &uri,
        &headers,
        Method::POST,
        &json!({}),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    let requested = match state
        .platform
        .zebflow_cfg
        .get_rwe_libraries(&owner, &project)
    {
        Ok(value) => value,
        Err(err) => return internal_error(err),
    };
    // A lock the canonical reader refuses — a dead source word included — is
    // regenerated here from requested state: the invalid bytes move to
    // `data/recovery/`, an empty lock takes their place, and each requested
    // library reinstalls from the blessed shelf, which rewrites its entry
    // with full provenance (`kinds/dependency-lock/README.md`).
    let mut regenerated: Vec<String> = Vec::new();
    if state
        .platform
        .dependency_lock
        .read(&owner, &project)
        .is_err()
    {
        match state
            .platform
            .dependency_lock
            .quarantine_invalid(&owner, &project)
        {
            Ok(Some(recovery)) => {
                regenerated.push(format!("invalid lock moved to {}", recovery.display()));
                for name in requested.keys() {
                    let Some(slug) = name.strip_prefix("zeb/").filter(|slug| !slug.is_empty())
                    else {
                        continue;
                    };
                    let package_id = format!("zebflow.{slug}");
                    let version = match state.platform.hub.latest_live_asset_version(&package_id) {
                        Ok(Some(version)) => version,
                        _ => continue,
                    };
                    match state.platform.hub.install_asset(
                        &owner,
                        &project,
                        &package_id,
                        &version,
                        "",
                    ) {
                        Ok(_) => regenerated.push(format!("reinstalled {package_id}@{version}")),
                        Err(err) => {
                            regenerated.push(format!("{package_id}: {}", err.message));
                        }
                    }
                }
            }
            Ok(None) => {}
            Err(err) => return internal_error(err),
        }
    }
    if let Err(err) = state
        .platform
        .dependency_lock
        .repair_rwe_libraries(&owner, &project, &requested)
    {
        return internal_error(err);
    }
    if let Err(err) = state
        .platform
        .node_registry
        .refresh_project(&owner, &project)
    {
        return internal_error(err);
    }
    match state
        .platform
        .dependency_lock
        .status(&owner, &project, &requested)
    {
        Ok(report) => {
            Json(json!({"ok": true, "report": report, "regenerated": regenerated})).into_response()
        }
        Err(err) => internal_error(err),
    }
}

/// `GET /api/projects/{owner}/{project}/rwe/libraries` — list all available
/// libraries merged with per-project enabled state.
async fn api_list_rwe_libraries(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::LibrariesRead,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::GET,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    let rwe_libs = state
        .platform
        .zebflow_cfg
        .get_rwe_libraries(&owner, &project)
        .unwrap_or_default();
    let items = state
        .platform
        .library
        .list()
        .map(|m| {
            let enabled_entry = rwe_libs.get(&m.name);
            json!({
                "name": m.name,
                "description": m.description,
                "packed_version": m.packed_version(),
                "packed_kind": m.packed_kind(),
                "enabled": enabled_entry.is_some(),
                "installed_version": enabled_entry.map(|e| e.version.clone()),
                "source": enabled_entry.map(|e| e.source.clone())
            })
        })
        .collect::<Vec<_>>();
    Json(items).into_response()
}

/// Request body for `POST /api/projects/{owner}/{project}/rwe/libraries/enable`.
#[derive(serde::Deserialize, serde::Serialize)]
struct EnableRweLibraryRequest {
    name: String,
    version: String,
    source: String,
}

/// `POST /api/projects/{owner}/{project}/rwe/libraries/enable`
async fn api_enable_rwe_library(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Json(req): Json<EnableRweLibraryRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::LibrariesInstall,
    ) {
        return response;
    }
    match maybe_forward_project_json_to_worker(
        &state,
        &uri,
        &headers,
        Method::POST,
        &req,
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    if req.name.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": "library name must not be empty"})),
        )
            .into_response();
    }
    // Enabling a library IS the local-hub install path: the blessed shelf
    // carries the seeded `zebflow.*` packages, the install copies the
    // package's bytes to `data/hub/rwe-libraries/{package_id}/`, and the lock
    // records `source: hub.local` with the installed digest. The binary's
    // embedded tree is only the seed — never a source a lock can name
    // (`distribution.md` §2) — so the retired embedded enable path is gone
    // and every requested source resolves through the shelf.
    let name = req.name.trim();
    let Some(slug) = name.strip_prefix("zeb/").filter(|slug| !slug.is_empty()) else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "ok": false,
                "error": format!(
                    "library '{name}' is not a blessed zeb/* library; install its \
                     rwe_library package from a hub instead"
                )
            })),
        )
            .into_response();
    };
    let package_id = format!("zebflow.{slug}");
    let version = match req.version.trim() {
        "" => match state.platform.hub.latest_live_asset_version(&package_id) {
            Ok(Some(version)) => version,
            Ok(None) => {
                return (
                    StatusCode::NOT_FOUND,
                    Json(json!({
                        "ok": false,
                        "error": format!("the local hub has no live release of '{package_id}'")
                    })),
                )
                    .into_response();
            }
            Err(err) => return internal_error(err),
        },
        explicit => explicit.to_string(),
    };
    match state
        .platform
        .hub
        .install_asset(&owner, &project, &package_id, &version, "")
    {
        Ok(result) => {
            let actor_user = session_owner(&state, &headers);
            let _ = rwe_library_git_commit(
                &state,
                actor_user.as_deref(),
                &owner,
                &project,
                &format!("chore(rwe): install library {package_id}@{version} from hub"),
            );
            Json(json!({"ok": true, "install": result})).into_response()
        }
        Err(err) => internal_error(err),
    }
}

/// Query params for `DELETE /api/projects/{owner}/{project}/rwe/libraries/remove`.
#[derive(serde::Deserialize)]
struct RemoveRweLibraryQuery {
    name: String,
}

/// `DELETE /api/projects/{owner}/{project}/rwe/libraries/remove?name=zeb%2Fthreejs`
async fn api_remove_rwe_library(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Query(params): Query<RemoveRweLibraryQuery>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::LibrariesRemove,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::DELETE,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    if params.name.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": "library name must not be empty"})),
        )
            .into_response();
    }
    if let Err(err) = state.platform.dependency_lock.remove_rwe_library(
        state.platform.zebflow_cfg.as_ref(),
        &owner,
        &project,
        params.name.trim(),
    ) {
        return internal_error(err);
    }
    // Git commit (best-effort).
    let actor_user = session_owner(&state, &headers);
    let _ = rwe_library_git_commit(
        &state,
        actor_user.as_deref(),
        &owner,
        &project,
        &format!("chore(rwe): remove library {}", params.name.trim()),
    );
    Json(json!({"ok": true})).into_response()
}

/// `POST /api/projects/{owner}/{project}/rwe/cache/clear` — flush the shared
/// template compile cache for the running process. Useful after editing an
/// imported component whose hash is not part of the entry-page hash key.
async fn api_rwe_cache_clear(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::SettingsWrite,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::POST,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    state
        .template_cache
        .write()
        .unwrap_or_else(|e| e.into_inner())
        .clear();
    Json(json!({"ok": true, "cleared": true})).into_response()
}

/// Stages `zebflow.yaml` and `zeb.lock`, then commits with the given message.
/// Best-effort: errors are logged but not propagated to the caller.
fn rwe_library_git_commit(
    state: &PlatformAppState,
    actor_user: Option<&str>,
    owner: &str,
    project: &str,
    message: &str,
) -> Result<(), ()> {
    let owner_slug = crate::platform::model::slug_segment(owner);
    let project_slug = crate::platform::model::slug_segment(project);
    let layout = state
        .platform
        .file
        .ensure_project_layout(&owner_slug, &project_slug)
        .map_err(|_| ())?;
    let add_ok = std::process::Command::new("git")
        .arg("-C")
        .arg(&layout.repo_dir)
        .arg("add")
        .arg(crate::contracts::kinds::PROJECT_CONFIGURATION_FILE)
        .arg("zeb.lock")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !add_ok {
        return Err(());
    }
    let identity_args = git_identity_args(state, actor_user, &project_slug);
    let mut cmd = std::process::Command::new("git");
    for arg in &identity_args {
        cmd.arg(arg);
    }
    cmd.arg("-C")
        .arg(&layout.repo_dir)
        .arg("commit")
        .arg("-m")
        .arg(message);
    cmd.output().map(|_| ()).map_err(|_| ())
}

/// Merges project-level RWE settings (`zebflow.yaml -> rwe`) into each `n.web.response`
/// node's `config.options` before pipeline execution.
///
/// Also parses the node-level `--load-scripts` comma-separated string and injects it
/// as a proper `Vec<String>` into `options.load_scripts`.  Called immediately after
/// `hydrate_template_markup` in the webhook and manual-execute paths.
fn apply_rwe_project_options(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
    graph: &mut PipelineGraph,
) -> Result<(), PlatformError> {
    let cfg = state.platform.zebflow_cfg.read_or_default(owner, project)?;
    let rwe = &cfg.configs.rwe;

    // Resolve template root so @/ alias imports work in user project templates.
    let template_root_str = state
        .platform
        .projects
        .get_project_template_root(owner, project)
        .ok()
        .map(|p| p.display().to_string());

    for node in &mut graph.nodes {
        if node.kind != "n.web.response" {
            continue;
        }

        // Parse node-level load_scripts (comma-separated string from DSL flag).
        let node_load_scripts: Vec<String> = node
            .config
            .get("load_scripts")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();

        let options = json!({
            "minify_html": rwe.minify_html,
            "strict_mode": rwe.strict_mode,
            "allow_list": {
                "urls": rwe.allow_list,
                "scripts": [],
                "css": []
            },
            "load_scripts": node_load_scripts,
            "templates": {
                "template_root": template_root_str
            }
        });

        if let Some(map) = node.config.as_object_mut() {
            map.insert("options".to_string(), options);
        }
    }
    Ok(())
}

/// Resolves the chat history file at its `store`-tier home, migrating a
/// pre-tier copy out of `data/cache/` the first time it is touched
/// (`project-directory.md` §5). A file present at both paths is refused
/// rather than guessed, per `migrate_tier_entry`.
fn chat_history_file(
    layout: &crate::platform::model::ProjectFileLayout,
) -> std::io::Result<std::path::PathBuf> {
    let new_path = layout.data_store_chat_history_file();
    crate::infra::io::durable::migrate_tier_entry(
        &layout.data_cache_dir().join("chat_history.json"),
        &new_path,
    )?;
    Ok(new_path)
}

/// Load up to `max_pairs * 2` chat messages from the project's store tier.
fn load_chat_history(
    file: &Arc<dyn crate::platform::adapters::file::FileAdapter>,
    owner: &str,
    project: &str,
) -> Vec<Value> {
    let layout = match file.ensure_project_layout(owner, project) {
        Ok(l) => l,
        Err(_) => return vec![],
    };
    let path = match chat_history_file(&layout) {
        Ok(path) => path,
        Err(err) => {
            eprintln!("WARN: chat history tier migration refused for {owner}/{project}: {err}");
            return vec![];
        }
    };
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return vec![],
    };
    serde_json::from_str::<Vec<Value>>(&content).unwrap_or_default()
}

/// Append a user+assistant exchange and persist to disk, keeping the last `max_pairs` pairs.
fn save_chat_history(
    file: &Arc<dyn crate::platform::adapters::file::FileAdapter>,
    owner: &str,
    project: &str,
    user_msg: &str,
    assistant_msg: &str,
    max_pairs: usize,
) {
    let layout = match file.ensure_project_layout(owner, project) {
        Ok(l) => l,
        Err(_) => return,
    };
    let path = match chat_history_file(&layout) {
        Ok(path) => path,
        Err(err) => {
            // A refused migration must not fork history by writing to either
            // path; surface it and write nothing.
            eprintln!("WARN: chat history tier migration refused for {owner}/{project}: {err}");
            return;
        }
    };
    let mut history: Vec<Value> = std::fs::read_to_string(&path)
        .ok()
        .and_then(|c| serde_json::from_str(&c).ok())
        .unwrap_or_default();
    history.push(json!({"role": "user", "content": user_msg}));
    history.push(json!({"role": "assistant", "content": assistant_msg}));
    // Keep last max_pairs pairs = max_pairs * 2 messages
    let keep = max_pairs * 2;
    if history.len() > keep {
        history.drain(0..history.len() - keep);
    }
    if let Ok(json) = serde_json::to_string(&history) {
        std::fs::create_dir_all(layout.data_store_dir()).ok();
        std::fs::write(&path, json).ok();
    }
}

async fn api_project_assistant_chat(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Json(req): Json<AssistantChatRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::ProjectRead,
    ) {
        return response;
    }
    match maybe_forward_project_json_to_worker(
        &state,
        &uri,
        &headers,
        Method::POST,
        &req,
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }

    let message = req.message.trim().to_string();
    if message.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "ok": false,
                "error": {
                    "code": "ASSISTANT_MESSAGE_INVALID",
                    "message": "message must not be empty"
                }
            })),
        )
            .into_response();
    }

    let bundle = match load_project_assistant_llm(
        state.platform.data.as_ref(),
        state.platform.assistant_configs.as_ref(),
        &owner,
        &project,
    ) {
        Ok(bundle) => bundle,
        Err(err) => {
            let status = match err.code {
                "ASSISTANT_NOT_CONFIGURED"
                | "ASSISTANT_DISABLED"
                | "ASSISTANT_NO_LLM"
                | "ASSISTANT_CREDENTIAL_MISSING"
                | "ASSISTANT_CREDENTIAL_INVALID" => StatusCode::BAD_REQUEST,
                _ => StatusCode::INTERNAL_SERVER_ERROR,
            };
            return (
                status,
                Json(json!({
                    "ok": false,
                    "error": { "code": err.code, "message": err.message }
                })),
            )
                .into_response();
        }
    };

    let tools = crate::platform::services::AssistantPlatformTools::new(
        state.platform.clone(),
        &owner,
        &project,
    );

    let tool_defs = crate::platform::services::AssistantPlatformTools::tool_defs();

    let mut messages: Vec<Value> = Vec::new();
    {
        let skills_text = crate::platform::help::format_for_system_prompt();
        let page_context = req
            .current_page
            .as_deref()
            .filter(|p| !p.is_empty())
            .map(|p| format!("\nCurrently viewing: {p}"))
            .unwrap_or_default();
        let time_context = req
            .client_time
            .as_deref()
            .filter(|t| !t.is_empty())
            .map(|t| format!("\nUser local time: {t}"))
            .unwrap_or_default();
        let nav_map = project_nav_map(&owner, &project);

        // Load agent docs for system prompt
        state
            .platform
            .projects
            .ensure_agent_docs_defaults(&owner, &project)
            .ok();
        let memory = state
            .platform
            .projects
            .read_agent_doc(&owner, &project, "MEMORY.md")
            .unwrap_or_default();
        let soul = state
            .platform
            .projects
            .read_agent_doc(&owner, &project, "SOUL.md")
            .unwrap_or_default();
        let agents = state
            .platform
            .projects
            .read_agent_doc(&owner, &project, "AGENTS.md")
            .unwrap_or_default();
        let readme = state
            .platform
            .projects
            .read_repo_file_text(&owner, &project, "README.md")
            .unwrap_or_default();
        let readme_section = if readme.trim().is_empty() {
            String::new()
        } else {
            format!("\n\n## Project README\n{readme}")
        };

        let system = format!(
            "You are the Zebflow project operator — an autonomous assistant for project {owner}/{project}.{page_context}{time_context}\n\n\
             ## Your Memory\n{memory}\n\n\
             ## Your Soul\n{soul}\n\n\
             ## Project Context\n{agents}{readme_section}\n\n\
             ## Your Tools\n\
             You have 33 native tools covering every project management operation:\n\
             - **Orientation**: `start_here` — call at the start of every session for a live project overview\n\
             - **Pipelines**: `pipeline_list`, `pipeline_get`, `pipeline_register`, `pipeline_describe`, `pipeline_patch`, `pipeline_activate`, `pipeline_deactivate`, `pipeline_execute`, `pipeline_run`\n\
             - **Files**: `file_list`, `file_read`, `file_write`, `file_edit`, `file_search` — every file in the repository, whatever its extension or folder\n\
             - **Agent docs**: `docs_agent_list`, `docs_agent_read`, `docs_agent_write`\n\
             - **Database**: `connection_list`, `connection_describe` — then use `pipeline_run` with `pg.query` or `n.sqlite.query` nodes to execute queries\n\
             - **Credentials**: `credential_list`\n\
             - **Git**: `git_command` — subcommands: status, log, diff, add, commit\n\
             - **UI Components**: `list_ui_catalog`, `install_ui_components`\n\
             - **Knowledge**: `help_pipeline`, `help_web_engine`, `help_examples`, `help_nodes`, `help_search`, `skill_list`, `skill_read`\n\n\
             ## Workflow\n\
             1. Call `start_here` to orient yourself when starting a new task\n\
             2. Use `connection_describe` before writing any SQL queries\n\
             3. Use `pipeline_run` with pipe-chained nodes to execute one-off queries or test pipelines\n\
             4. After registering or patching pipelines, always call `pipeline_activate` to make them live\n\
             5. Always commit changes with `git_command subcommand=add` then `git_command subcommand=commit`\n\n\
             Available pages:\n\
             {nav_map}\n\n\
             ## Zebflow Knowledge\n\n{skills_text}"
        );
        messages.push(json!({"role": "system", "content": system}));
    }

    // Server-side history takes precedence; fall back to client-sent history if empty.
    let server_history = load_chat_history(&state.platform.file, &owner, &project);
    let history_source: Box<dyn Iterator<Item = Value>> = if server_history.is_empty() {
        // Fall back to client-sent history (first session or no persistence yet)
        Box::new(
            req.history
                .into_iter()
                .filter(|item| !item.content.trim().is_empty())
                .filter_map(|item| match item.role.as_str() {
                    "user" | "assistant" => {
                        Some(json!({"role": item.role, "content": item.content.trim()}))
                    }
                    _ => None,
                })
                .take(20),
        )
    } else {
        Box::new(server_history.into_iter().take(20))
    };
    for item in history_source {
        messages.push(item);
    }
    messages.push(json!({"role": "user", "content": message}));

    let llm = if req.use_high_model {
        bundle.high.clone()
    } else {
        bundle.general.clone()
    };

    let max_steps = bundle.max_steps;
    let chat_history_pairs_for_save = bundle.chat_history_pairs;
    let model_tier = if req.use_high_model {
        "high"
    } else {
        "general"
    };

    // Channel for streaming step events to SSE
    let (step_tx, mut step_rx) = tokio::sync::mpsc::unbounded_channel::<AssistantStepEvent>();

    // Spawn the agentic loop as a background task
    let loop_task = tokio::spawn(async move {
        run_assistant_loop(llm, &tools, tool_defs, messages, max_steps, &step_tx).await
    });

    // Collect all SSE events: start + step events + message + done
    let owner_clone = owner.clone();
    let project_clone = project.clone();
    let file_for_history = state.platform.file.clone();
    let message_for_history = message.clone();

    let sse_stream = async_stream::stream! {
        // start event
        yield Ok::<Event, Infallible>(
            Event::default().event("start").data(
                json!({
                    "ok": true,
                    "owner": owner_clone,
                    "project": project_clone,
                    "model_tier": model_tier,
                    "max_steps": max_steps,
                })
                .to_string(),
            ),
        );

        // Drain step events while loop is running
        let mut budget_exhausted = false;
        let mut steps_taken: u32 = 0;
        let final_content;

        loop {
            match step_rx.recv().await {
                Some(AssistantStepEvent::ToolCall { step, tool, args, thought }) => {
                    steps_taken = step;
                    yield Ok(Event::default().event("tool_call").data(
                        json!({
                            "step": step,
                            "tool": tool,
                            "args": args,
                            "thought": thought,
                        }).to_string()
                    ));
                }
                Some(AssistantStepEvent::ToolResult { step, tool, result_preview }) => {
                    steps_taken = step;
                    yield Ok(Event::default().event("tool_result").data(
                        json!({
                            "step": step,
                            "tool": tool,
                            "result_preview": result_preview,
                        }).to_string()
                    ));
                }
                Some(AssistantStepEvent::Navigate { url, label }) => {
                    yield Ok(Event::default().event("navigate").data(
                        json!({ "url": url, "label": label }).to_string()
                    ));
                }
                Some(AssistantStepEvent::InteractionSequence { id, label, steps }) => {
                    yield Ok(Event::default().event("interaction_sequence").data(
                        json!({ "id": id, "label": label, "steps": steps }).to_string()
                    ));
                }
                Some(AssistantStepEvent::BudgetExhausted) => {
                    budget_exhausted = true;
                }
                Some(AssistantStepEvent::Done(content)) => {
                    final_content = content;
                    break;
                }
                None => {
                    // Channel closed (loop task ended without Done — shouldn't happen)
                    final_content = String::new();
                    break;
                }
            }
        }

        // Await loop task to make sure it's fully done
        let _ = loop_task.await;

        // Persist this exchange to server-side chat history (last 10 pairs)
        if !final_content.is_empty() {
            save_chat_history(
                &file_for_history,
                &owner_clone,
                &project_clone,
                &message_for_history,
                &final_content,
                chat_history_pairs_for_save as usize,
            );
        }

        let content_html = crate::rwe::processors::markdown::render_markdown_fragment(&final_content);
        yield Ok(Event::default().event("message").data(
            json!({"role":"assistant","content": final_content, "content_html": content_html}).to_string()
        ));
        yield Ok(Event::default().event("done").data(
            json!({"ok": true, "steps_taken": steps_taken, "budget_exhausted": budget_exhausted}).to_string()
        ));
    };

    Sse::new(sse_stream)
        .keep_alive(
            KeepAlive::new()
                .interval(Duration::from_secs(15))
                .text("keep-alive"),
        )
        .into_response()
}

/// Events emitted by the assistant agentic loop.
enum AssistantStepEvent {
    ToolCall {
        step: u32,
        tool: String,
        args: Value,
        thought: String,
    },
    ToolResult {
        step: u32,
        tool: String,
        result_preview: String,
    },
    Navigate {
        url: String,
        label: String,
    },
    InteractionSequence {
        id: String,
        label: String,
        steps: Value,
    },
    BudgetExhausted,
    Done(String),
}

async fn run_assistant_loop(
    llm: std::sync::Arc<dyn crate::automaton::infra::llm_interface::LlmCall>,
    tools: &crate::platform::services::AssistantPlatformTools,
    tool_defs: Vec<crate::automaton::infra::llm_interface::ToolDef>,
    mut messages: Vec<Value>,
    max_steps: u32,
    step_tx: &tokio::sync::mpsc::UnboundedSender<AssistantStepEvent>,
) -> String {
    use crate::automaton::infra::llm_interface::CallResult;

    for step in 1..=max_steps {
        let (result, _usage) = match llm.call_with_tools(messages.clone(), &tool_defs).await {
            Ok(r) => r,
            Err(err) => {
                let content = format!("(LLM error: {err})");
                let _ = step_tx.send(AssistantStepEvent::Done(content.clone()));
                return content;
            }
        };

        match result {
            CallResult::Text(content) => {
                let _ = step_tx.send(AssistantStepEvent::Done(content.clone()));
                return content;
            }
            CallResult::ToolCalls(calls) => {
                // Append the assistant's tool_calls message to history
                let tool_calls_json: Vec<Value> = calls
                    .iter()
                    .map(|tc| {
                        json!({
                            "id": tc.id,
                            "type": "function",
                            "function": { "name": tc.name, "arguments": tc.arguments }
                        })
                    })
                    .collect();
                messages.push(json!({
                    "role": "assistant",
                    "content": null,
                    "tool_calls": tool_calls_json
                }));

                // Execute each call and append tool result messages
                for tc in &calls {
                    let args: Value = serde_json::from_str(&tc.arguments).unwrap_or(json!({}));

                    let _ = step_tx.send(AssistantStepEvent::ToolCall {
                        step,
                        tool: tc.name.clone(),
                        args: args.clone(),
                        thought: String::new(),
                    });

                    let tool_result = tools.run_async(&tc.name, &args).await;
                    let result_str = tool_result.text.clone();

                    // Side-effect events for interactive tools
                    if let Some(seq) = tool_result.interaction {
                        let id = seq["id"].as_str().unwrap_or(&tc.id).to_string();
                        let label = seq["label"].as_str().unwrap_or(&tc.name).to_string();
                        let steps = seq["steps"].clone();
                        let _ = step_tx.send(AssistantStepEvent::InteractionSequence {
                            id,
                            label,
                            steps,
                        });
                    } else if let Some(url) = tool_result.navigate {
                        let _ = step_tx.send(AssistantStepEvent::Navigate {
                            label: url.clone(),
                            url,
                        });
                    }

                    let result_preview = result_str.chars().take(500).collect::<String>();

                    let _ = step_tx.send(AssistantStepEvent::ToolResult {
                        step,
                        tool: tc.name.clone(),
                        result_preview,
                    });

                    messages.push(json!({
                        "role": "tool",
                        "tool_call_id": tc.id,
                        "content": result_str
                    }));
                }
            }
        }
    }

    // Budget exhausted
    let _ = step_tx.send(AssistantStepEvent::BudgetExhausted);
    let content = "(max steps reached)".to_string();
    let _ = step_tx.send(AssistantStepEvent::Done(content.clone()));
    content
}

async fn api_list_db_connections(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesRead,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::GET,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    match state
        .platform
        .db_connections
        .list_project_connections(&owner, &project)
    {
        Ok(items) => {
            let mut rows = vec![json!({
                "connection_id": "builtin:mapserver:default-mapserver",
                "connection_slug": "default-mapserver",
                "connection_label": "Default Mapserver",
                "database_kind": "mapserver",
                "credential_id": "",
                "config_json": "{}",
                "created_at": 0,
                "updated_at": 0,
                "path": format!("/projects/{owner}/{project}/db/mapserver/default-mapserver/layers"),
                "builtin": true
            })];
            rows.extend(
                items
                    .into_iter()
                    .filter_map(|item| serde_json::to_value(item).ok()),
            );
            Json(json!({"ok": true, "items": rows})).into_response()
        }
        Err(err) => internal_error(err),
    }
}

#[derive(Debug, Deserialize, serde::Serialize)]
struct PublishHubAssetRequest {
    source_type: String,
    source_ref: String,
    package_id: String,
    version: String,
    #[serde(default)]
    publisher_token: String,
    #[serde(default)]
    title: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    image_file_path: String,
    #[serde(default)]
    visibility: String,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    include_sekejap_schema: bool,
    #[serde(default)]
    include_sqlite_schema: bool,
    #[serde(default)]
    include_libraries: Vec<String>,
    #[serde(default)]
    include_initial_data: bool,
    #[serde(default)]
    initial_data_paths: Vec<String>,
}

/// A presentation edit as the route receives it.
///
/// Absent is not empty here. Presentation was split out of the release so a
/// typo costs an update rather than a version bump, and a request that blanked
/// every field it did not mention would put that cost straight back.
#[derive(Debug, Default, Deserialize, serde::Serialize)]
struct UpdateHubPresentationRequest {
    #[serde(default)]
    publisher_token: String,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    description_md: Option<String>,
    #[serde(default)]
    image_file_path: Option<String>,
    #[serde(default)]
    gallery: Option<crate::platform::model::HubAssetGallery>,
}

/// Why a publisher withdrew a package, carried on the retraction request.
#[derive(Debug, Default, Deserialize, serde::Serialize)]
struct RetractHubAssetRequest {
    #[serde(default)]
    reason: String,
}

#[derive(Debug, Deserialize)]
struct HubPublishQuery {
    #[serde(default)]
    source_type: String,
    #[serde(default)]
    source_ref: String,
}

fn project_bundle_publish_options(req: &PublishHubAssetRequest) -> HubProjectBundlePublishOptions {
    HubProjectBundlePublishOptions {
        include_sekejap_schema: req.include_sekejap_schema,
        include_sqlite_schema: req.include_sqlite_schema,
        include_libraries: req.include_libraries.clone(),
        include_initial_data: req.include_initial_data,
        initial_data_paths: req.initial_data_paths.clone(),
    }
}

#[derive(Debug, Deserialize, serde::Serialize)]
struct InstallHubAssetRequest {
    #[serde(default)]
    target_folder: String,
}

fn normalized_hub_target_folder(body: Option<&Json<InstallHubAssetRequest>>) -> String {
    body.map(|Json(req)| req.target_folder.trim().to_string())
        .unwrap_or_default()
}

#[derive(Debug, Deserialize, serde::Serialize)]
struct UpsertHubRepositoryRequest {
    repository_id: String,
    title: String,
    base_url: String,
    #[serde(default)]
    remote_owner: String,
    #[serde(default)]
    remote_project: String,
    #[serde(default)]
    read_token: String,
    /// Which channel serves this source: `api` or `static`. Absent means `api`.
    #[serde(default)]
    kind: String,
    /// Where it sits in resolution order. Absent leaves an existing row's
    /// priority alone, so editing a title cannot silently reorder resolution.
    #[serde(default)]
    priority: Option<i64>,
    #[serde(default = "default_public_visibility")]
    visibility: String,
    #[serde(default = "default_true")]
    enabled: bool,
}

fn default_public_visibility() -> String {
    "public".to_string()
}

#[derive(Debug, Deserialize)]
struct CreatePlatformHubTokenRequest {
    #[serde(default)]
    owner: String,
    #[serde(default)]
    project: String,
    publisher_id: String,
    title: String,
    #[serde(default)]
    scopes: Vec<String>,
    #[serde(default)]
    expires_at: Option<i64>,
}

#[derive(Debug, Deserialize, serde::Serialize)]
struct UpsertPlatformHubGrantRequest {
    repository_id: String,
    #[serde(default = "default_selected_project_grant_scope")]
    grant_scope: String,
    #[serde(default)]
    target_owner: String,
    #[serde(default)]
    target_project: String,
    #[serde(default = "default_true")]
    can_read: bool,
    #[serde(default)]
    can_publish: bool,
    #[serde(default)]
    can_manage: bool,
    #[serde(default = "default_true")]
    enabled: bool,
}

fn default_selected_project_grant_scope() -> String {
    "selected_project".to_string()
}

#[derive(Debug, Deserialize, serde::Serialize)]
struct UpsertHubPublisherRequest {
    publisher_id: String,
    #[serde(default)]
    display_name: String,
    #[serde(default)]
    publisher_url: String,
    #[serde(default)]
    email: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    icon_url: String,
    #[serde(default)]
    website_url: String,
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default = "default_true")]
    can_read: bool,
    #[serde(default = "default_true")]
    can_publish: bool,
    #[serde(default)]
    can_manage: bool,
    #[serde(default)]
    max_packages: i64,
    #[serde(default)]
    max_package_bytes: i64,
    #[serde(default)]
    max_media_files: i64,
    #[serde(default)]
    max_image_bytes: i64,
}

#[derive(Debug, Deserialize)]
struct InstallPlatformHubProjectRequest {
    repository_id: String,
    package_id: String,
    version: String,
    /// Which parts of the bundle to install. Each flag defaults to true, so a
    /// body that names none of them installs the whole package as before.
    #[serde(default = "default_true")]
    include_code: bool,
    #[serde(default = "default_true")]
    include_schema: bool,
    #[serde(default = "default_true")]
    execute_schema: bool,
}

impl InstallPlatformHubProjectRequest {
    fn scope(&self) -> crate::platform::services::hub::HubInstallScope {
        crate::platform::services::hub::HubInstallScope {
            include_code: self.include_code,
            include_schema: self.include_schema,
            execute_schema: self.execute_schema,
        }
    }
}

#[derive(Debug, Deserialize)]
struct ConfigurePlatformHubServiceRequest {
    host_office_id: String,
    #[serde(default)]
    public_base_url: String,
    password: String,
    enabled: bool,
}

#[derive(Debug, Deserialize, serde::Serialize)]
struct SetHubProducerRequest {
    project_name: String,
    password: String,
    enabled: bool,
}

fn project_hub_repository_json(item: crate::platform::model::ProjectHubRepository) -> Value {
    json!({
        "owner": item.owner,
        "project": item.project,
        "repository_id": item.repository_id,
        "title": item.title,
        "base_url": item.base_url,
        "remote_owner": item.remote_owner,
        "remote_project": item.remote_project,
        "read_token": "",
        "has_read_token": !item.read_token.trim().is_empty(),
        "enabled": item.enabled,
        "created_at": item.created_at,
        "updated_at": item.updated_at,
    })
}

fn project_hub_repository_json_with_scope(
    item: crate::platform::model::ProjectHubRepository,
    source_scope: &str,
) -> Value {
    let mut value = project_hub_repository_json(item);
    if let Some(map) = value.as_object_mut() {
        map.insert("source_scope".to_string(), json!(source_scope));
        map.insert(
            "editable".to_string(),
            json!(source_scope == "project_local"),
        );
    }
    value
}

fn platform_hub_repository_json(item: crate::platform::model::PlatformHubRepository) -> Value {
    json!({
        "owner": item.owner,
        "repository_id": item.repository_id,
        "title": item.title,
        "base_url": item.base_url,
        "remote_owner": item.remote_owner,
        "remote_project": item.remote_project,
        "read_token": "",
        "has_read_token": !item.read_token.trim().is_empty(),
        "kind": if item.kind.trim().is_empty() { "api" } else { item.kind.as_str() },
        "priority": item.priority,
        "visibility": item.visibility,
        "enabled": item.enabled,
        "created_at": item.created_at,
        "updated_at": item.updated_at,
    })
}

fn hub_access_grant_json(item: crate::platform::model::HubAccessGrant) -> Value {
    json!({
        "grant_id": item.grant_id,
        "source_owner": item.source_owner,
        "source_id": item.source_id,
        "repository_id": item.repository_id,
        "grant_scope": item.grant_scope,
        "target_owner": item.target_owner,
        "target_project": item.target_project,
        "can_read": item.can_read,
        "can_publish": item.can_publish,
        "can_manage": item.can_manage,
        "enabled": item.enabled,
        "created_at": item.created_at,
        "updated_at": item.updated_at,
    })
}

fn hub_api_error(err: PlatformError) -> Response {
    let status = if err.code == "FW_EGRESS_DENIED"
        || err.code == "FW_EGRESS_URL_INVALID"
        || err.code == "FW_EGRESS_DNS"
        || err.code == "HUB_REPOSITORY_INVALID"
        || err.code == "HUB_ACCESS_GRANT_INVALID"
        || err.code == "HUB_PACKAGE_INVALID"
        || err.code == "HUB_ASSET_KIND_INVALID"
        || err.code == "HUB_MEDIA_INVALID"
        || err.code == "HUB_GALLERY_INVALID"
        || err.code == "HUB_INSTALL_SCOPE_INVALID"
        // A digest that is not 64 hexadecimal digits is a malformed request,
        // and it is refused before it can become a path or a URL segment.
        || err.code == "HUB_ARTIFACT_INVALID"
        // A refused publish is a fault in the package the caller sent, and a
        // publisher who reads 500 fixes nothing.
        || err.code == "HUB_PUBLISH_REFUSED"
        || err.code == "HUB_REMOTE_PUBLISH_REFUSED"
    {
        StatusCode::BAD_REQUEST
    } else if err.code == "HUB_ASSET_MISSING"
        || err.code == "HUB_MEDIA_MISSING"
        || err.code == "HUB_REPOSITORY_MISSING"
        // A digest this release does not reference, or whose bytes this hub
        // does not hold, is a missing thing and not a server fault.
        || err.code == "HUB_ARTIFACT_MISSING"
    {
        StatusCode::NOT_FOUND
    } else if err.code == "HUB_TOKEN_INVALID"
        || err.code == "HUB_TOKEN_REVOKED"
        || err.code == "HUB_TOKEN_EXPIRED"
    {
        StatusCode::UNAUTHORIZED
    } else if err.code == "HUB_TOKEN_FORBIDDEN" || err.code == "HUB_PACKAGE_FORBIDDEN" {
        StatusCode::FORBIDDEN
    } else if err.code == "HUB_VERSION_EXISTS" {
        // Releases are immutable: the version is already there.
        StatusCode::CONFLICT
    } else if err.code == "HUB_VERSION_RETRACTED" {
        // The coordinate is still real; its bytes are deliberately gone.
        StatusCode::GONE
    } else if err.code == "HUB_TOKEN_SCOPE_INVALID" {
        StatusCode::BAD_REQUEST
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    };
    (
        status,
        Json(json!({"ok": false, "error": {"code": err.code, "message": err.message}})),
    )
        .into_response()
}

fn authenticate_hub_delete_token(
    state: &PlatformAppState,
    token_value: &str,
) -> Result<crate::platform::model::HubToken, PlatformError> {
    match state
        .platform
        .hub
        .authenticate_token(token_value, "hub:manage")
    {
        Ok(token) => Ok(token),
        Err(manage_err) if manage_err.code == "HUB_TOKEN_FORBIDDEN" => state
            .platform
            .hub
            .authenticate_token(token_value, "hub:publish"),
        Err(err) => Err(err),
    }
}

fn authenticate_project_hub_publish_token(
    state: &PlatformAppState,
    headers: &HeaderMap,
    owner: &str,
    project: &str,
    explicit_token: &str,
) -> Result<crate::platform::model::HubToken, Response> {
    let token_value = explicit_token.trim().to_string();
    let token_value = if token_value.is_empty() {
        bearer_token_from_headers(headers).unwrap_or_default()
    } else {
        token_value
    };
    if token_value.is_empty() {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "ok": false,
                "error": "publisher token is required",
                "code": "HUB_TOKEN_REQUIRED"
            })),
        )
            .into_response());
    }
    let token = match state
        .platform
        .hub
        .authenticate_token(&token_value, "hub:publish")
    {
        Ok(token) => token,
        Err(err) => {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({"ok": false, "error": err.message, "code": err.code})),
            )
                .into_response());
        }
    };
    if token.owner != owner || token.project != project {
        return Err((
            StatusCode::FORBIDDEN,
            Json(json!({
                "ok": false,
                "error": "publisher token does not belong to this project",
                "code": "HUB_TOKEN_FORBIDDEN"
            })),
        )
            .into_response());
    }
    Ok(token)
}

fn hub_token_json(item: crate::platform::model::HubToken) -> Value {
    json!({
        "token_id": item.token_id,
        "owner": item.owner,
        "project": item.project,
        "publisher_id": item.publisher_id,
        "publisher_display_name": item.publisher_display_name,
        "publisher_url": item.publisher_url,
        "publisher_email": item.publisher_email,
        "title": item.title,
        "scopes": item.scopes,
        "expires_at": item.expires_at,
        "last_used_at": item.last_used_at,
        "revoked_at": item.revoked_at,
        "created_at": item.created_at,
        "updated_at": item.updated_at,
    })
}

fn hub_publisher_json(item: crate::platform::model::HubPublisher) -> Value {
    json!({
        "owner": item.owner,
        "project": item.project,
        "publisher_id": item.publisher_id,
        "display_name": item.display_name,
        "publisher_url": item.publisher_url,
        "email": item.email,
        "description": item.description,
        "icon_url": item.icon_url,
        "website_url": item.website_url,
        "enabled": item.enabled,
        "can_read": item.can_read,
        "can_publish": item.can_publish,
        "can_manage": item.can_manage,
        "max_packages": item.max_packages,
        "max_package_bytes": item.max_package_bytes,
        "max_media_files": item.max_media_files,
        "max_image_bytes": item.max_image_bytes,
        "created_at": item.created_at,
        "updated_at": item.updated_at,
    })
}

fn default_true() -> bool {
    true
}

async fn api_list_hub_assets(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesRead,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::GET,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    let mut items = match hub_asset_rows(&state, &owner, &project, false) {
        Ok(items) => items,
        Err(err) => return internal_error(err),
    };
    match state
        .platform
        .hub
        .fetch_remote_pack_rows(&owner, &project)
        .await
    {
        Ok(remote_rows) => {
            items.extend(
                remote_rows
                    .into_iter()
                    .filter_map(|item| serde_json::to_value(item).ok()),
            );
            Json(json!({"ok": true, "items": items})).into_response()
        }
        Err(err) => internal_error(err),
    }
}

async fn api_list_project_hub_access(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesRead,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::GET,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    if let Err(err) = state
        .platform
        .hub
        .ensure_default_project_repository(&owner, &project)
    {
        return internal_error(err);
    }
    let local_ids = match state.platform.hub.list_repositories(&owner, &project) {
        Ok(items) => items
            .into_iter()
            .map(|item| item.repository_id)
            .collect::<std::collections::BTreeSet<_>>(),
        Err(err) => return internal_error(err),
    };
    match state
        .platform
        .hub
        .list_effective_repositories(&owner, &project)
    {
        Ok(items) => Json(json!({
            "ok": true,
            "repositories": items
                .into_iter()
                .map(|item| {
                    let scope = if local_ids.contains(&item.repository_id) {
                        "project_local"
                    } else {
                        "platform_grant"
                    };
                    project_hub_repository_json_with_scope(item, scope)
                })
                .collect::<Vec<_>>()
        }))
        .into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_get_project_help(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesRead,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::GET,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    let ops =
        crate::platform::services::ops::PlatformOps::new(state.platform.clone(), &owner, &project);
    Json(json!({
        "ok": true,
        "sections": ops.help_dialog_sections(),
    }))
    .into_response()
}

async fn api_list_remote_hub_assets(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::GET,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    if let Err(response) = require_hub_service_enabled(&state) {
        return response;
    }
    if let Err(response) = require_project_hub_producer(&state, &owner, &project) {
        return response;
    }
    let token = bearer_token_from_headers(&headers);
    let requester = if let Some(token_value) = token {
        match state
            .platform
            .hub
            .authenticate_token(&token_value, "hub:read")
        {
            Ok(token) => {
                if token.owner != owner || token.project != project {
                    return (
                        StatusCode::UNAUTHORIZED,
                        Json(json!({"ok": false, "error": "token does not belong to this hub", "code": "HUB_TOKEN_FORBIDDEN"})),
                    )
                        .into_response();
                }
                Some(token.owner)
            }
            Err(err) => {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({"ok": false, "error": err.message, "code": err.code})),
                )
                    .into_response();
            }
        }
    } else {
        None
    };
    match state.platform.hub.list_public_asset_packages() {
        Ok(packages) => {
            let mut items = Vec::new();
            for package in packages {
                if package.visibility == "private"
                    && requester.as_deref() != Some(package.publisher_owner.as_str())
                {
                    continue;
                }
                let latest_version = state
                    .platform
                    .hub
                    .list_public_asset_versions(&package.package_id)
                    .ok()
                    .and_then(|items| items.into_iter().next().map(|v| v.version))
                    .unwrap_or_default();
                let mut item = public_hub_asset_item_json(&state, package);
                if let Some(object) = item.as_object_mut() {
                    object.insert("latest_version".to_string(), json!(latest_version));
                }
                items.push(item);
            }
            Json(json!({"ok": true, "items": items})).into_response()
        }
        Err(err) => internal_error(err),
    }
}

async fn api_get_remote_hub_asset(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, package_id, version)): Path<(String, String, String, String)>,
    uri: Uri,
) -> Response {
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::GET,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    if let Err(response) = require_hub_service_enabled(&state) {
        return response;
    }
    if let Err(response) = require_project_hub_producer(&state, &owner, &project) {
        return response;
    }
    let token = bearer_token_from_headers(&headers);
    let requester = if let Some(token_value) = token {
        match state
            .platform
            .hub
            .authenticate_token(&token_value, "hub:read")
        {
            Ok(token) => {
                if token.owner != owner || token.project != project {
                    return (
                        StatusCode::UNAUTHORIZED,
                        Json(json!({"ok": false, "error": "token does not belong to this hub", "code": "HUB_TOKEN_FORBIDDEN"})),
                    )
                        .into_response();
                }
                Some(token.owner)
            }
            Err(err) => {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({"ok": false, "error": err.message, "code": err.code})),
                )
                    .into_response();
            }
        }
    } else {
        None
    };
    let Some(package) = state
        .platform
        .hub
        .list_public_asset_packages()
        .ok()
        .and_then(|items| items.into_iter().find(|item| item.package_id == package_id))
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"ok": false, "error": "package not found"})),
        )
            .into_response();
    };
    if package.visibility == "private"
        && requester.as_deref() != Some(package.publisher_owner.as_str())
    {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"ok": false, "error": "private package"})),
        )
            .into_response();
    }
    hub_asset_detail_response(&state, &package, &package_id, &version)
}

async fn api_get_remote_hub_artifact(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, package_id, version)): Path<(String, String, String, String)>,
    uri: Uri,
) -> Response {
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::GET,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    if let Err(response) = require_hub_service_enabled(&state) {
        return response;
    }
    if let Err(response) = require_project_hub_producer(&state, &owner, &project) {
        return response;
    }
    let token = bearer_token_from_headers(&headers);
    let requester = if let Some(token_value) = token {
        match state
            .platform
            .hub
            .authenticate_token(&token_value, "hub:read")
        {
            Ok(token) => {
                if token.owner != owner || token.project != project {
                    return (
                        StatusCode::UNAUTHORIZED,
                        Json(json!({"ok": false, "error": "token does not belong to this hub", "code": "HUB_TOKEN_FORBIDDEN"})),
                    )
                        .into_response();
                }
                Some(token.owner)
            }
            Err(err) => {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({"ok": false, "error": err.message, "code": err.code})),
                )
                    .into_response();
            }
        }
    } else {
        None
    };
    let Some(package) = state
        .platform
        .hub
        .list_public_asset_packages()
        .ok()
        .and_then(|items| items.into_iter().find(|item| item.package_id == package_id))
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"ok": false, "error": "package not found"})),
        )
            .into_response();
    };
    if package.visibility == "private"
        && requester.as_deref() != Some(package.publisher_owner.as_str())
    {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"ok": false, "error": "private package"})),
        )
            .into_response();
    }
    match state
        .platform
        .hub
        .get_asset_version_install_artifact(&package_id, &version)
    {
        Ok((version_row, artifact, artifact_size_bytes)) => Json(raw_hub_artifact_response_json(
            &package,
            &version_row,
            artifact,
            artifact_size_bytes,
        ))
        .into_response(),
        Err(err) => internal_error(err),
    }
}

/// Serves one artifact a release references: the HTTP half of the artifact
/// channel.
///
/// A referenced file's bytes never travel in the release document, so a hub
/// that serves releases has to serve their artifacts too or every remote
/// install of one refuses. The response is the raw bytes and nothing else: the
/// installer already knows the digest it wants and verifies what it gets.
async fn api_get_public_hub_referenced_artifact(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((package_id, version, sha256)): Path<(String, String, String)>,
) -> Response {
    if let Err(response) = require_hub_service_enabled(&state) {
        return response;
    }
    let requester = match hub_read_token_owner(&state, &headers) {
        Ok(requester) => requester,
        Err(response) => return response,
    };
    hub_referenced_artifact_response(&state, requester, &package_id, &version, &sha256)
}

/// The same artifact, on a project-scoped producer hub.
async fn api_get_remote_hub_referenced_artifact(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, package_id, version, sha256)): Path<(
        String,
        String,
        String,
        String,
        String,
    )>,
    uri: Uri,
) -> Response {
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::GET,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    if let Err(response) = require_hub_service_enabled(&state) {
        return response;
    }
    if let Err(response) = require_project_hub_producer(&state, &owner, &project) {
        return response;
    }
    let requester = match hub_read_token_owner(&state, &headers) {
        Ok(requester) => requester,
        Err(response) => return response,
    };
    hub_referenced_artifact_response(&state, requester, &package_id, &version, &sha256)
}

/// Who is asking, if they presented a hub read token at all.
fn hub_read_token_owner(
    state: &PlatformAppState,
    headers: &HeaderMap,
) -> Result<Option<String>, Response> {
    let Some(token_value) = bearer_token_from_headers(headers) else {
        return Ok(None);
    };
    match state
        .platform
        .hub
        .authenticate_token(&token_value, "hub:read")
    {
        Ok(token) => Ok(Some(token.owner)),
        Err(err) => Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({"ok": false, "error": err.message, "code": err.code})),
        )
            .into_response()),
    }
}

/// The release's own visibility decides who may read its artifacts, so the two
/// routes above answer the same way about the same bytes.
fn hub_referenced_artifact_response(
    state: &PlatformAppState,
    requester: Option<String>,
    package_id: &str,
    version: &str,
    sha256: &str,
) -> Response {
    let Some(package) = state
        .platform
        .hub
        .list_public_asset_packages()
        .ok()
        .and_then(|items| items.into_iter().find(|item| item.package_id == package_id))
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"ok": false, "error": "package not found"})),
        )
            .into_response();
    };
    if package.visibility == "private"
        && requester.as_deref() != Some(package.publisher_owner.as_str())
    {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"ok": false, "error": "private package"})),
        )
            .into_response();
    }
    match state
        .platform
        .hub
        .get_release_referenced_artifact(package_id, version, sha256)
    {
        Ok((_, media_type, bytes)) => {
            let mut response_headers = HeaderMap::new();
            if let Ok(value) = HeaderValue::from_str(&media_type) {
                response_headers.insert(CONTENT_TYPE, value);
            }
            // Content-addressed bytes cannot change under their own name.
            response_headers.insert(
                CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=31536000, immutable"),
            );
            (StatusCode::OK, response_headers, bytes).into_response()
        }
        Err(err) => hub_api_error(err),
    }
}

async fn api_list_public_hub_assets(State(state): State<PlatformAppState>) -> Response {
    if let Err(response) = require_hub_service_enabled(&state) {
        return response;
    }
    match state.platform.hub.list_public_asset_packages() {
        Ok(packages) => {
            let items = packages
                .into_iter()
                .filter(|package| package.visibility == "public")
                .map(|package| public_hub_asset_item_json(&state, package))
                .collect::<Vec<_>>();
            Json(json!({"ok": true, "items": items})).into_response()
        }
        Err(err) => internal_error(err),
    }
}

async fn api_get_public_hub_asset(
    State(state): State<PlatformAppState>,
    Path((package_id, version)): Path<(String, String)>,
) -> Response {
    if let Err(response) = require_hub_service_enabled(&state) {
        return response;
    }
    let Some(package) = state
        .platform
        .hub
        .list_public_asset_packages()
        .ok()
        .and_then(|items| items.into_iter().find(|item| item.package_id == package_id))
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"ok": false, "error": "package not found"})),
        )
            .into_response();
    };
    if package.visibility == "private" {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"ok": false, "error": "private package"})),
        )
            .into_response();
    }
    hub_asset_detail_response(&state, &package, &package_id, &version)
}

async fn api_get_public_hub_media(
    State(state): State<PlatformAppState>,
    Path((package_id, media_name)): Path<(String, String)>,
) -> Response {
    if let Err(response) = require_hub_service_enabled(&state) {
        return response;
    }
    match state
        .platform
        .hub
        .get_latest_asset_media(&package_id, &media_name)
    {
        Ok((package, media, bytes)) => {
            if package.visibility != "public" {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({"ok": false, "error": "private package"})),
                )
                    .into_response();
            }
            let mut headers = HeaderMap::new();
            if let Ok(value) = HeaderValue::from_str(&media.content_type) {
                headers.insert(CONTENT_TYPE, value);
            }
            headers.insert(
                CACHE_CONTROL,
                HeaderValue::from_static("public, max-age=300"),
            );
            (StatusCode::OK, headers, bytes).into_response()
        }
        Err(err) => hub_api_error(err),
    }
}

async fn api_get_public_hub_artifact(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((package_id, version)): Path<(String, String)>,
) -> Response {
    if let Err(response) = require_hub_service_enabled(&state) {
        return response;
    }
    let token = bearer_token_from_headers(&headers);
    let requester = if let Some(token_value) = token {
        match state
            .platform
            .hub
            .authenticate_token(&token_value, "hub:read")
        {
            Ok(token) => Some(token.owner),
            Err(err) => {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({"ok": false, "error": err.message, "code": err.code})),
                )
                    .into_response();
            }
        }
    } else {
        None
    };
    let Some(package) = state
        .platform
        .hub
        .list_public_asset_packages()
        .ok()
        .and_then(|items| items.into_iter().find(|item| item.package_id == package_id))
    else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"ok": false, "error": "package not found"})),
        )
            .into_response();
    };
    if package.visibility == "private"
        && requester.as_deref() != Some(package.publisher_owner.as_str())
    {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"ok": false, "error": "private package"})),
        )
            .into_response();
    }
    match state
        .platform
        .hub
        .get_asset_version_install_artifact(&package_id, &version)
    {
        Ok((version_row, artifact, artifact_size_bytes)) => Json(raw_hub_artifact_response_json(
            &package,
            &version_row,
            artifact,
            artifact_size_bytes,
        ))
        .into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_list_my_hub_assets(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesRead,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::GET,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    match hub_asset_rows(&state, &owner, &project, true) {
        Ok(items) => Json(json!({"ok": true, "items": items})).into_response(),
        Err(err) => internal_error(err),
    }
}

/// Retract every release of one package: `DELETE .../hub/assets/{package_id}`.
///
/// The method is still DELETE because that is what a publisher means, but the
/// act is retraction: the rows survive with a marker, the artifacts do not, and
/// the coordinates can never be published again.
async fn api_delete_hub_asset(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, package_id)): Path<(String, String, String)>,
    uri: Uri,
    body: Option<Json<RetractHubAssetRequest>>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesWrite,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::DELETE,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    let Some(token_value) = bearer_token_from_headers(&headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "ok": false,
                "error": "publisher token is required",
                "code": "HUB_TOKEN_REQUIRED"
            })),
        )
            .into_response();
    };
    let token = match authenticate_hub_delete_token(&state, &token_value) {
        Ok(token) => token,
        Err(err) => return hub_api_error(err),
    };
    if !token.scopes.iter().any(|scope| scope == "hub:manage")
        && (token.owner != owner || token.project != project)
    {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({
                "ok": false,
                "error": "publisher token does not belong to this project",
                "code": "HUB_TOKEN_FORBIDDEN"
            })),
        )
            .into_response();
    }
    let reason = body.map(|Json(req)| req.reason).unwrap_or_default();
    match state
        .platform
        .hub
        .retract_asset_package(&token, &package_id, &reason)
    {
        Ok(retracted_versions) => {
            Json(json!({"ok": true, "retracted": true, "retracted_versions": retracted_versions}))
                .into_response()
        }
        Err(err) => hub_api_error(err),
    }
}

async fn api_list_hub_publish_sources(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Query(query): Query<HubPublishQuery>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesRead,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::GET,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }
    match hub_publish_sources(
        &state,
        &owner,
        &project,
        if query.source_type.trim().is_empty() {
            "pipeline_with_dependencies"
        } else {
            &query.source_type
        },
    ) {
        Ok(items) => Json(json!({"ok": true, "items": items})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_preview_hub_publish_source(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Query(query): Query<HubPublishQuery>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesRead,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::GET,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }
    match state.platform.hub.preview_publish_source(
        &owner,
        &project,
        &query.source_type,
        &query.source_ref,
    ) {
        Ok(preview) => Json(json!({"ok": true, "preview": preview})).into_response(),
        Err(err) => internal_error(err),
    }
}

/// The project-source half of the ONE publish route: resolves a project
/// source through the shared publish core, landing in the Public Hub store.
fn publish_project_source_to_public_hub(
    state: &PlatformAppState,
    headers: &HeaderMap,
    owner: &str,
    project: &str,
    req: PublishHubAssetRequest,
) -> Response {
    if let Err(response) = require_project_api_capability(
        state,
        headers,
        owner,
        project,
        ProjectCapability::PipelinesWrite,
    ) {
        return response;
    }
    let token = match authenticate_project_hub_publish_token(
        state,
        headers,
        owner,
        project,
        &req.publisher_token,
    ) {
        Ok(token) => token,
        Err(response) => return response,
    };
    if req.source_type == "project_files" {
        let requested = match state.platform.zebflow_cfg.get_rwe_libraries(owner, project) {
            Ok(value) => value,
            Err(err) => return internal_error(err),
        };
        let dependencies = match state
            .platform
            .dependency_lock
            .status(owner, project, &requested)
        {
            Ok(report) => report,
            Err(err) => return internal_error(err),
        };
        if !dependencies.ok {
            return (
                StatusCode::CONFLICT,
                Json(json!({
                    "ok": false,
                    "error": {
                        "code": "HUB_PUBLISH_DEPENDENCIES",
                        "message": "project bundle has unresolved dependencies"
                    },
                    "dependencies": dependencies
                })),
            )
                .into_response();
        }
    }
    match state.platform.hub.publish_asset(
        owner,
        project,
        &token.owner,
        &token.publisher_id,
        &token.publisher_display_name,
        &token.publisher_url,
        &token.publisher_email,
        owner,
        project,
        &req.source_type,
        &req.source_ref,
        &req.package_id,
        &req.version,
        &req.title,
        &req.description,
        &req.image_file_path,
        &req.visibility,
        project_bundle_publish_options(&req),
        req.tags,
    ) {
        Ok((package, version)) => {
            Json(json!({"ok": true, "package": package, "version": version})).into_response()
        }
        Err(err) => internal_error(err),
    }
}

async fn api_review_hub_publish_asset(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Json(req): Json<PublishHubAssetRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesWrite,
    ) {
        return response;
    }
    match maybe_forward_project_json_to_worker(
        &state,
        &uri,
        &headers,
        Method::POST,
        &req,
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    let token = match authenticate_project_hub_publish_token(
        &state,
        &headers,
        &owner,
        &project,
        &req.publisher_token,
    ) {
        Ok(token) => token,
        Err(response) => return response,
    };
    match state.platform.hub.review_publish_asset(
        &owner,
        &project,
        &token.publisher_id,
        &req.source_type,
        &req.source_ref,
        &req.package_id,
        &req.version,
        &req.title,
        &req.description,
        &req.image_file_path,
        &req.visibility,
        project_bundle_publish_options(&req),
        req.tags,
    ) {
        Ok(review) => Json(json!({"ok": true, "review": review})).into_response(),
        Err(err) => hub_api_error(err),
    }
}

/// Edit the mutable half of a published package, touching no release.
///
/// `PATCH .../hub/assets/{package_id}/presentation`, publisher-scoped with the
/// same token as publish. This is the endpoint the presentation split was made
/// for: without it, presentation is mutable in the design and immutable in
/// practice, and fixing a description costs a version bump.
async fn api_update_hub_asset_presentation(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, package_id)): Path<(String, String, String)>,
    uri: Uri,
    Json(req): Json<UpdateHubPresentationRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesWrite,
    ) {
        return response;
    }
    match maybe_forward_project_json_to_worker(
        &state,
        &uri,
        &headers,
        Method::PATCH,
        &req,
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    let token = match authenticate_project_hub_publish_token(
        &state,
        &headers,
        &owner,
        &project,
        &req.publisher_token,
    ) {
        Ok(token) => token,
        Err(response) => return response,
    };
    match state.platform.hub.update_asset_presentation(
        &token,
        &owner,
        &project,
        &package_id,
        &crate::platform::services::hub::HubPresentationUpdate {
            summary: req.summary,
            description_md: req.description_md,
            image_file_path: req.image_file_path,
            gallery: req.gallery,
        },
    ) {
        Ok(package) => Json(json!({
            "ok": true,
            "presentation": public_hub_presentation_json(&package),
        }))
        .into_response(),
        Err(err) => hub_api_error(err),
    }
}

async fn api_install_hub_asset(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, package_id, version)): Path<(String, String, String, String)>,
    uri: Uri,
    body: Option<Json<InstallHubAssetRequest>>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesWrite,
    ) {
        return response;
    }
    let forward_body = body
        .as_ref()
        .map(|Json(req)| json!({"target_folder": req.target_folder}))
        .unwrap_or_else(|| json!({}));
    match maybe_forward_project_json_to_worker(
        &state,
        &uri,
        &headers,
        Method::POST,
        &forward_body,
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    let target_folder = normalized_hub_target_folder(body.as_ref());
    match state
        .platform
        .hub
        .install_asset(&owner, &project, &package_id, &version, &target_folder)
    {
        Ok(result) => Json(json!({"ok": true, "target_folder": target_folder, "result": result}))
            .into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_review_hub_asset(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, package_id, version)): Path<(String, String, String, String)>,
    uri: Uri,
    body: Option<Json<InstallHubAssetRequest>>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesWrite,
    ) {
        return response;
    }
    let forward_body = body
        .as_ref()
        .map(|Json(req)| json!({"target_folder": req.target_folder}))
        .unwrap_or_else(|| json!({}));
    match maybe_forward_project_json_to_worker(
        &state,
        &uri,
        &headers,
        Method::POST,
        &forward_body,
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    let target_folder = normalized_hub_target_folder(body.as_ref());
    match state.platform.hub.review_asset_install(
        &owner,
        &project,
        &package_id,
        &version,
        &target_folder,
    ) {
        Ok(review) => Json(json!({"ok": true, "review": review})).into_response(),
        Err(err) => hub_api_error(err),
    }
}

/// `POST .../hub/remote/assets/publish` — the ONE publish route.
///
/// Publishing lands in the Public Hub store, whichever form the request
/// takes (`distribution.md` §1b). A body carrying `artifact` is a pre-built
/// release pushed over HTTP; a body naming a project source is resolved
/// in-process through the same publish core. The retired local-write route
/// (`POST .../hub/assets/publish`) is gone — no publisher token can reach
/// the blessed shelf.
async fn api_remote_publish_hub_asset(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Json(body): Json<Value>,
) -> Response {
    match maybe_forward_project_json_to_worker(
        &state,
        &uri,
        &headers,
        Method::POST,
        &body,
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    if body.get("artifact").is_none() {
        // Project-source form: the Studio publish flow. The publisher token
        // rides in the body or the Authorization header.
        let req: PublishHubAssetRequest = match serde_json::from_value(body) {
            Ok(req) => req,
            Err(err) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"ok": false, "error": err.to_string()})),
                )
                    .into_response();
            }
        };
        return publish_project_source_to_public_hub(&state, &headers, &owner, &project, req);
    }
    let req: RemoteHubPublishRequest = match serde_json::from_value(body) {
        Ok(req) => req,
        Err(err) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok": false, "error": err.to_string()})),
            )
                .into_response();
        }
    };
    let Some(token_value) = bearer_token_from_headers(&headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"ok": false, "error": "missing bearer token"})),
        )
            .into_response();
    };
    let token = match state
        .platform
        .hub
        .authenticate_token(&token_value, "hub:publish")
    {
        Ok(token) => token,
        Err(err) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"ok": false, "error": err.message, "code": err.code})),
            )
                .into_response();
        }
    };
    if token.owner != owner || token.project != project {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"ok": false, "error": "token does not belong to this hub", "code": "HUB_TOKEN_FORBIDDEN"})),
        )
            .into_response();
    }
    match state
        .platform
        .hub
        .import_remote_asset(&owner, &project, &token, &req)
    {
        Ok((package, version)) => {
            Json(json!({"ok": true, "package": package, "version": version})).into_response()
        }
        Err(err) => hub_api_error(err),
    }
}

async fn api_public_remote_publish_hub_asset(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Json(req): Json<RemoteHubPublishRequest>,
) -> Response {
    let Some(token_value) = bearer_token_from_headers(&headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"ok": false, "error": "missing bearer token"})),
        )
            .into_response();
    };
    let token = match state
        .platform
        .hub
        .authenticate_token(&token_value, "hub:publish")
    {
        Ok(token) => token,
        Err(err) => {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"ok": false, "error": err.message, "code": err.code})),
            )
                .into_response();
        }
    };
    match state
        .platform
        .hub
        .import_remote_asset(&token.owner, &token.project, &token, &req)
    {
        Ok((package, version)) => {
            Json(json!({"ok": true, "package": package, "version": version})).into_response()
        }
        Err(err) => hub_api_error(err),
    }
}

/// Retract every release of one package: `DELETE /api/hub/remote/assets/{id}`.
///
/// See `api_delete_hub_asset`: DELETE withdraws the bytes, never the coordinates.
async fn api_public_delete_hub_asset(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path(package_id): Path<String>,
    body: Option<Json<RetractHubAssetRequest>>,
) -> Response {
    let Some(token_value) = bearer_token_from_headers(&headers) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(
                json!({"ok": false, "error": "missing bearer token", "code": "HUB_TOKEN_REQUIRED"}),
            ),
        )
            .into_response();
    };
    let token = match authenticate_hub_delete_token(&state, &token_value) {
        Ok(token) => token,
        Err(err) => return hub_api_error(err),
    };
    let reason = body.map(|Json(req)| req.reason).unwrap_or_default();
    match state
        .platform
        .hub
        .retract_asset_package(&token, &package_id, &reason)
    {
        Ok(retracted_versions) => {
            Json(json!({"ok": true, "retracted": true, "retracted_versions": retracted_versions}))
                .into_response()
        }
        Err(err) => hub_api_error(err),
    }
}

async fn api_list_hub_tokens(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesWrite,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::GET,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    if let Err(response) = require_project_hub_producer(&state, &owner, &project) {
        return response;
    }
    match hub_token_rows(&state, &owner, &project) {
        Ok(items) => Json(json!({"ok": true, "items": items})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_create_hub_token(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Json(req): Json<CreateHubTokenRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesWrite,
    ) {
        return response;
    }
    match maybe_forward_project_json_to_worker(
        &state,
        &uri,
        &headers,
        Method::POST,
        &req,
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    if let Err(response) = require_project_hub_producer(&state, &owner, &project) {
        return response;
    }
    match state.platform.hub.create_token(&owner, &project, &req) {
        Ok((token, token_value)) => {
            Json(json!({"ok": true, "token": hub_token_json(token), "token_value": token_value}))
                .into_response()
        }
        Err(err) => internal_error(err),
    }
}

async fn api_delete_hub_token(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, token_id)): Path<(String, String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesWrite,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::DELETE,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    if let Err(response) = require_project_hub_producer(&state, &owner, &project) {
        return response;
    }
    match state.platform.hub.revoke_token(&owner, &project, &token_id) {
        Ok(()) => Json(json!({"ok": true})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_list_hub_publishers(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesWrite,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::GET,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    if let Err(response) = require_project_hub_producer(&state, &owner, &project) {
        return response;
    }
    match state.platform.hub.list_publishers(&owner, &project) {
        Ok(items) => Json(json!({
            "ok": true,
            "items": items.into_iter().map(hub_publisher_json).collect::<Vec<_>>()
        }))
        .into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_upsert_hub_publisher(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Json(req): Json<UpsertHubPublisherRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesWrite,
    ) {
        return response;
    }
    match maybe_forward_project_json_to_worker(
        &state,
        &uri,
        &headers,
        Method::POST,
        &req,
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    if let Err(response) = require_project_hub_producer(&state, &owner, &project) {
        return response;
    }
    match state.platform.hub.upsert_publisher(
        &owner,
        &project,
        &req.publisher_id,
        &req.display_name,
        &req.publisher_url,
        &req.email,
        &req.description,
        &req.icon_url,
        &req.website_url,
        req.enabled,
        req.can_read,
        req.can_publish,
        req.can_manage,
        req.max_packages,
        req.max_package_bytes,
        req.max_media_files,
        req.max_image_bytes,
    ) {
        Ok(item) => {
            Json(json!({"ok": true, "publisher": hub_publisher_json(item)})).into_response()
        }
        Err(err) => internal_error(err),
    }
}

async fn api_delete_hub_publisher(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, publisher_id)): Path<(String, String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesWrite,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::DELETE,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    if let Err(response) = require_project_hub_producer(&state, &owner, &project) {
        return response;
    }
    match state
        .platform
        .hub
        .delete_publisher(&owner, &project, &publisher_id)
    {
        Ok(()) => Json(json!({"ok": true})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_set_hub_producer_mode(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Json(req): Json<SetHubProducerRequest>,
) -> Response {
    let internal_cluster_call = is_controller_call(&state, &headers);
    let Some(session_owner) = session_owner(&state, &headers).or_else(|| {
        if internal_cluster_call {
            Some(owner.clone())
        } else {
            None
        }
    }) else {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"ok": false, "error": "Not authenticated"})),
        )
            .into_response();
    };
    let owner_slug = crate::platform::model::slug_segment(&owner);
    let project_slug = crate::platform::model::slug_segment(&project);
    if crate::platform::model::slug_segment(&session_owner) != owner_slug {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"ok": false, "error": "Forbidden"})),
        )
            .into_response();
    }
    if !can_manage_project_hub_producer(&state, &owner_slug) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"ok": false, "error": "Only curated superadmin projects can host a hub producer"})),
        )
            .into_response();
    }
    if crate::platform::model::slug_segment(&req.project_name) != project_slug
        || project_slug.is_empty()
    {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": "Project name does not match"})),
        )
            .into_response();
    }
    if !internal_cluster_call {
        match state
            .platform
            .users
            .authenticate(&owner_slug, &req.password)
        {
            Ok(true) => {}
            Ok(false) => {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({"ok": false, "error": "Incorrect password"})),
                )
                    .into_response();
            }
            Err(err) => return internal_error(err),
        }
        match maybe_forward_project_json_to_worker(
            &state,
            &uri,
            &headers,
            Method::POST,
            &req,
            &owner,
            &project,
        )
        .await
        {
            Ok(Some(response)) => return response,
            Ok(None) => {}
            Err(err) => return internal_error(err),
        }
    }
    let mut cfg = match state
        .platform
        .zebflow_cfg
        .get_hub_distribution(&owner_slug, &project_slug)
    {
        Ok(config) => config,
        Err(err) => return internal_error(err),
    };
    cfg.producer_enabled = req.enabled;
    match state
        .platform
        .zebflow_cfg
        .set_hub_distribution(&owner_slug, &project_slug, cfg.clone())
    {
        Ok(()) => {
            match state
                .platform
                .hub
                .set_authority_enabled(&owner_slug, &project_slug, req.enabled)
            {
                Ok(authority) => {
                    Json(json!({"ok": true, "hub": cfg, "authority": authority})).into_response()
                }
                Err(err) => internal_error(err),
            }
        }
        Err(err) => internal_error(err),
    }
}

async fn api_list_hub_repositories(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesRead,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::GET,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    if let Err(err) = state
        .platform
        .hub
        .ensure_default_project_repository(&owner, &project)
    {
        return internal_error(err);
    }
    match state
        .platform
        .hub
        .list_effective_repositories(&owner, &project)
    {
        Ok(items) => Json(json!({
            "ok": true,
            "items": items.into_iter().map(project_hub_repository_json).collect::<Vec<_>>()
        }))
        .into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_upsert_hub_repository(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Json(req): Json<UpsertHubRepositoryRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesWrite,
    ) {
        return response;
    }
    match maybe_forward_project_json_to_worker(
        &state,
        &uri,
        &headers,
        Method::POST,
        &req,
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    match state.platform.hub.upsert_repository(
        &owner,
        &project,
        &req.repository_id,
        &req.title,
        &req.base_url,
        &req.remote_owner,
        &req.remote_project,
        &req.read_token,
        req.enabled,
    ) {
        Ok(item) => Json(json!({
            "ok": true,
            "repository": project_hub_repository_json(item)
        }))
        .into_response(),
        Err(err) => hub_api_error(err),
    }
}

async fn api_delete_hub_repository(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, repository_id)): Path<(String, String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesWrite,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::DELETE,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    match state
        .platform
        .hub
        .delete_repository(&owner, &project, &repository_id)
    {
        Ok(()) => Json(json!({"ok": true})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_install_remote_hub_pack(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, repository_id, package_id, version)): Path<(
        String,
        String,
        String,
        String,
        String,
    )>,
    uri: Uri,
    body: Option<Json<InstallHubAssetRequest>>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesWrite,
    ) {
        return response;
    }
    let forward_body = body
        .as_ref()
        .map(|Json(req)| json!({"target_folder": req.target_folder}))
        .unwrap_or_else(|| json!({}));
    match maybe_forward_project_json_to_worker(
        &state,
        &uri,
        &headers,
        Method::POST,
        &forward_body,
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    let target_folder = normalized_hub_target_folder(body.as_ref());
    match state
        .platform
        .hub
        .install_remote_pack_from_repository(
            &owner,
            &project,
            &repository_id,
            &package_id,
            &version,
            &target_folder,
        )
        .await
    {
        Ok(result) => Json(json!({"ok": true, "target_folder": target_folder, "result": result}))
            .into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_review_remote_hub_pack(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, repository_id, package_id, version)): Path<(
        String,
        String,
        String,
        String,
        String,
    )>,
    uri: Uri,
    body: Option<Json<InstallHubAssetRequest>>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesWrite,
    ) {
        return response;
    }
    let forward_body = body
        .as_ref()
        .map(|Json(req)| json!({"target_folder": req.target_folder}))
        .unwrap_or_else(|| json!({}));
    match maybe_forward_project_json_to_worker(
        &state,
        &uri,
        &headers,
        Method::POST,
        &forward_body,
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    let target_folder = normalized_hub_target_folder(body.as_ref());
    match state
        .platform
        .hub
        .review_remote_pack_from_repository(
            &owner,
            &project,
            &repository_id,
            &package_id,
            &version,
            &target_folder,
        )
        .await
    {
        Ok(review) => Json(json!({"ok": true, "review": review})).into_response(),
        Err(err) => hub_api_error(err),
    }
}

async fn api_list_simple_tables(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesRead,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::GET,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }

    let data_root = state.platform.config.data_root.clone();
    match run_platform_blocking(move || sekejap::list_tables(&data_root, &owner, &project)).await {
        Ok(items) => Json(json!({"ok": true, "items": items})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_create_simple_table(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Json(req): Json<CreateSimpleTableRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesWrite,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_json_request_to_worker(
            &state,
            &uri,
            &headers,
            Method::POST,
            &req,
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }

    let data_root = state.platform.config.data_root.clone();
    match run_platform_blocking(move || sekejap::create_table(&data_root, &owner, &project, &req))
        .await
    {
        Ok(table) => Json(json!({"ok": true, "table": table})).into_response(),
        Err(err)
            if err.code == "PLATFORM_SEKEJAP_TABLE_INVALID"
                || err.code == "PLATFORM_SEKEJAP_TABLE_EXISTS" =>
        {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok": false, "error": {"code": err.code, "message": err.message}})),
            )
                .into_response()
        }
        Err(err) => internal_error(err),
    }
}

async fn api_export_sekejap_schema(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesRead,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::GET,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }

    let filename = format!("{project}-sekejap-schema.json");
    let data_root = state.platform.config.data_root.clone();
    let export =
        match run_platform_blocking(move || sekejap::export_schema(&data_root, &owner, &project))
            .await
        {
            Ok(export) => export,
            Err(err) => return internal_error(err),
        };
    let body = match sekejap::encode_schema_export(export) {
        Ok(value) => value,
        Err(err) => {
            return internal_error(PlatformError::new("SEKEJAP_SCHEMA_EXPORT", err.to_string()));
        }
    };
    (
        [
            (
                CONTENT_TYPE,
                HeaderValue::from_static("application/json; charset=utf-8"),
            ),
            (
                CONTENT_DISPOSITION,
                HeaderValue::from_str(&format!("attachment; filename=\"{filename}\""))
                    .unwrap_or_else(|_| HeaderValue::from_static("attachment")),
            ),
        ],
        body,
    )
        .into_response()
}

async fn api_sync_sekejap_schema(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesWrite,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::POST,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }

    let data_root = state.platform.config.data_root.clone();
    match run_platform_blocking(move || sekejap::sync_schema_to_repo(&data_root, &owner, &project))
        .await
    {
        Ok(report) => Json(json!({"ok": true, "sync": report})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_sekejap_project_health(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesRead,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::GET,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }

    let data_root = state.platform.config.data_root.clone();
    match run_platform_blocking(move || sekejap::project_health(&data_root, &owner, &project)).await
    {
        Ok(health) => Json(json!({"ok": true, "health": health})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_sekejap_project_sync(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesWrite,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::POST,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }

    let data_root = state.platform.config.data_root.clone();
    match run_platform_blocking(move || sekejap::sync_project(&data_root, &owner, &project)).await {
        Ok(sync) => Json(json!({"ok": true, "sync": sync})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_sekejap_project_compact(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesWrite,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::POST,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }

    let data_root = state.platform.config.data_root.clone();
    match run_platform_blocking(move || sekejap::compact_project(&data_root, &owner, &project))
        .await
    {
        Ok(compact) => Json(json!({"ok": true, "compact": compact})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_update_simple_table(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, table)): Path<(String, String, String)>,
    uri: Uri,
    Json(req): Json<UpdateSimpleTableRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesWrite,
    ) {
        return response;
    }
    match maybe_forward_project_json_to_worker(
        &state,
        &uri,
        &headers,
        Method::PUT,
        &req,
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }

    let data_root = state.platform.config.data_root.clone();
    match run_platform_blocking(move || {
        sekejap::update_table(&data_root, &owner, &project, &table, &req)
    })
    .await
    {
        Ok(updated) => Json(json!({"ok": true, "table": updated})).into_response(),
        Err(err)
            if err.code == "PLATFORM_SEKEJAP_TABLE_INVALID"
                || err.code == "PLATFORM_SEKEJAP_TABLE_NOT_FOUND" =>
        {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok": false, "error": {"code": err.code, "message": err.message}})),
            )
                .into_response()
        }
        Err(err) => internal_error(err),
    }
}

async fn api_delete_simple_table(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, table)): Path<(String, String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesWrite,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::DELETE,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }

    let data_root = state.platform.config.data_root.clone();
    match run_platform_blocking(move || sekejap::delete_table(&data_root, &owner, &project, &table))
        .await
    {
        Ok(()) => Json(json!({"ok": true})).into_response(),
        Err(err)
            if err.code == "PLATFORM_SEKEJAP_TABLE_INVALID"
                || err.code == "PLATFORM_SEKEJAP_TABLE_NOT_FOUND" =>
        {
            (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok": false, "error": {"code": err.code, "message": err.message}})),
            )
                .into_response()
        }
        Err(err) => internal_error(err),
    }
}

async fn api_get_db_connection(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, connection_slug)): Path<(String, String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesRead,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::GET,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    match state
        .platform
        .db_connections
        .get_project_connection(&owner, &project, &connection_slug)
    {
        Ok(Some(connection)) => Json(json!({"ok": true, "connection": connection})).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({"ok": false, "error": {"code":"PLATFORM_DB_CONNECTION_MISSING","message":"db connection not found"}})),
        )
            .into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_upsert_db_connection(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Json(req): Json<UpsertProjectDbConnectionRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesWrite,
    ) {
        return response;
    }
    match maybe_forward_project_json_to_worker(
        &state,
        &uri,
        &headers,
        Method::POST,
        &req,
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    match state
        .platform
        .db_connections
        .upsert_project_connection(&owner, &project, &req)
    {
        Ok(connection) => Json(json!({"ok": true, "connection": connection})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_upsert_db_connection_by_path(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, connection_slug)): Path<(String, String, String)>,
    uri: Uri,
    Json(mut req): Json<UpsertProjectDbConnectionRequest>,
) -> Response {
    req.connection_slug = connection_slug;
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesWrite,
    ) {
        return response;
    }
    match maybe_forward_project_json_to_worker(
        &state,
        &uri,
        &headers,
        Method::PUT,
        &req,
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    match state
        .platform
        .db_connections
        .upsert_project_connection(&owner, &project, &req)
    {
        Ok(connection) => Json(json!({"ok": true, "connection": connection})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_delete_db_connection(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, connection_slug)): Path<(String, String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesWrite,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::DELETE,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    match state.platform.db_connections.delete_project_connection(
        &owner,
        &project,
        &connection_slug,
    ) {
        Ok(()) => StatusCode::NO_CONTENT.into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_test_db_connection(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Json(req): Json<TestProjectDbConnectionRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesRead,
    ) {
        return response;
    }
    match maybe_forward_project_json_to_worker(
        &state,
        &uri,
        &headers,
        Method::POST,
        &req,
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    match state
        .platform
        .db_connections
        .test_project_connection(&owner, &project, &req)
        .await
    {
        Ok(result) => Json(json!({"ok": true, "result": result})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_describe_db_connection(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, connection_id)): Path<(String, String, String)>,
    uri: Uri,
    Query(query): Query<DbDescribeQuery>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesRead,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::GET,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    let req = DescribeProjectDbConnectionRequest {
        scope: query.scope,
        schema: query.schema,
        table: query.table,
        include_system: query.include_system,
    };
    match state
        .platform
        .db_runtime
        .describe_connection(&owner, &project, &connection_id, &req)
        .await
    {
        Ok(result) => Json(json!({"ok": true, "result": result})).into_response(),
        Err(err) => internal_error(err),
    }
}

/// POST /api/projects/{owner}/{project}/db/connections/{connection_id}/tables
async fn api_create_db_connection_table(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, connection_id)): Path<(String, String, String)>,
    Json(req): Json<CreateSimpleTableRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesWrite,
    ) {
        return response;
    }
    match state
        .platform
        .db_runtime
        .create_table(&owner, &project, &connection_id, &req)
        .await
    {
        Ok(table) => Json(json!({"ok": true, "table": table})).into_response(),
        Err(err) => db_ddl_error(err),
    }
}

/// DELETE /api/projects/{owner}/{project}/db/connections/{connection_id}/tables/{table}
async fn api_drop_db_connection_table(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, connection_id, table)): Path<(String, String, String, String)>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesWrite,
    ) {
        return response;
    }
    match state
        .platform
        .db_runtime
        .drop_table(&owner, &project, &connection_id, &table)
        .await
    {
        Ok(()) => Json(json!({"ok": true})).into_response(),
        Err(err) => db_ddl_error(err),
    }
}

/// PUT /api/projects/{owner}/{project}/db/connections/{connection_id}/tables/{table}
async fn api_alter_db_connection_table(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, connection_id, table)): Path<(String, String, String, String)>,
    Json(req): Json<UpdateSimpleTableRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesWrite,
    ) {
        return response;
    }
    match state
        .platform
        .db_runtime
        .alter_table(&owner, &project, &connection_id, &table, &req)
        .await
    {
        Ok(table) => Json(json!({"ok": true, "table": table})).into_response(),
        Err(err) => db_ddl_error(err),
    }
}

/// POST /api/projects/{owner}/{project}/db/connections/{connection_id}/tables/{table}/rows
///
/// Inserts one row with the values in the body. The statement differs by
/// engine, so the driver writes it.
async fn api_insert_db_connection_row(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, connection_id, table)): Path<(String, String, String, String)>,
    Json(values): Json<serde_json::Map<String, serde_json::Value>>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesWrite,
    ) {
        return response;
    }
    match state
        .platform
        .db_runtime
        .insert_row(&owner, &project, &connection_id, &table, &values)
        .await
    {
        Ok(identity) => Json(json!({"ok": true, "identity": identity})).into_response(),
        Err(err) => db_ddl_error(err),
    }
}

/// A refused definition is the caller's mistake, not a server fault, so it
/// answers 400 rather than 500.
fn db_ddl_error(err: PlatformError) -> Response {
    let caller_error = matches!(
        &*err.code,
        "PLATFORM_DB_DDL_UNSUPPORTED"
            | "PLATFORM_DB_DDL_NAME"
            | "PLATFORM_DB_DDL_KIND"
            | "PLATFORM_DB_DDL_INDEX"
            | "PLATFORM_DB_DDL_FAILED"
            | "PLATFORM_DB_ROW_UNSUPPORTED"
            | "PLATFORM_DB_ROW_FAILED"
            | "PLATFORM_DB_DDL_TYPE_CHANGE"
    );
    if caller_error {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": {"code": err.code, "message": err.message}})),
        )
            .into_response();
    }
    internal_error(err)
}

async fn api_list_db_connection_schemas(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, connection_id)): Path<(String, String, String)>,
    uri: Uri,
    Query(query): Query<DbObjectListQuery>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesRead,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::GET,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    let req = DescribeProjectDbConnectionRequest {
        scope: Some("schemas".to_string()),
        schema: query.schema,
        table: None,
        include_system: query.include_system,
    };
    match state
        .platform
        .db_runtime
        .describe_connection(&owner, &project, &connection_id, &req)
        .await
    {
        Ok(result) => Json(json!({"ok": true, "result": result})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_list_db_connection_tables(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, connection_id)): Path<(String, String, String)>,
    uri: Uri,
    Query(query): Query<DbObjectListQuery>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesRead,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::GET,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    let req = DescribeProjectDbConnectionRequest {
        scope: Some("tables".to_string()),
        schema: query.schema,
        table: None,
        include_system: query.include_system,
    };
    match state
        .platform
        .db_runtime
        .describe_connection(&owner, &project, &connection_id, &req)
        .await
    {
        Ok(result) => Json(json!({"ok": true, "result": result})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_list_db_connection_functions(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, connection_id)): Path<(String, String, String)>,
    uri: Uri,
    Query(query): Query<DbObjectListQuery>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesRead,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::GET,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    let req = DescribeProjectDbConnectionRequest {
        scope: Some("functions".to_string()),
        schema: query.schema,
        table: None,
        include_system: query.include_system,
    };
    match state
        .platform
        .db_runtime
        .describe_connection(&owner, &project, &connection_id, &req)
        .await
    {
        Ok(result) => Json(json!({"ok": true, "result": result})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_preview_db_connection_table(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, connection_id)): Path<(String, String, String)>,
    uri: Uri,
    Query(query): Query<DbTablePreviewQuery>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TablesRead,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::GET,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    let table = query.table.unwrap_or_default();
    if table.trim().is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": {"code":"PLATFORM_DB_QUERY_INVALID","message":"query.table is required"}})),
        )
            .into_response();
    }
    let limit = query.limit.unwrap_or(120).clamp(1, 5000);
    let database_kind = match state
        .platform
        .db_connections
        .list_project_connections(&owner, &project)
    {
        Ok(items) => items
            .into_iter()
            .find(|item| item.connection_id == connection_id)
            .map(|item| item.database_kind),
        Err(err) => return internal_error(err),
    };
    let Some(database_kind) = database_kind else {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"ok": false, "error": {"code":"PLATFORM_DB_CONNECTION_MISSING","message":"connection not found"}})),
        )
            .into_response();
    };
    let table_name = table.split('.').next_back().unwrap_or(&table).trim();
    let dialect = state.platform.db_runtime.sql_dialect_for_kind(&database_kind);
    let sql = build_table_preview_sql(dialect, &table, table_name, limit);
    let req = QueryProjectDbConnectionRequest {
        table: Some(table_name.to_string()),
        sql,
        limit: Some(limit),
        read_only: Some(true),
        ..Default::default()
    };
    match state
        .platform
        .db_runtime
        .query_connection(&owner, &project, &connection_id, &req)
        .await
    {
        Ok(result) => Json(json!({"ok": true, "result": result})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_query_db_connection(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, connection_id)): Path<(String, String, String)>,
    uri: Uri,
    Json(mut req): Json<QueryProjectDbConnectionRequest>,
) -> Response {
    // What the statement does decides which capability it needs. The request
    // carries `read_only`, and taking the caller's word for it meant
    // `{"sql": "DELETE FROM posts", "read_only": false}` ran for anyone who
    // could read a table — the role ladder already separated Reporter from
    // Maintainer, and this endpoint simply never asked.
    let writes = crate::platform::db::statement::statement_writes(&req.sql);
    let needed = crate::platform::db::statement::capability_for_statement(&req.sql);
    if let Err(response) =
        require_project_api_capability(&state, &headers, &owner, &project, needed)
    {
        return response;
    }
    // A caller may ask for a stricter run than its capabilities require. It may
    // not ask for a looser one, so the flag is narrowed rather than trusted.
    if !writes {
        req.read_only = Some(true);
    }
    match maybe_forward_project_json_to_worker(
        &state,
        &uri,
        &headers,
        Method::POST,
        &req,
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    match state
        .platform
        .db_runtime
        .query_connection(&owner, &project, &connection_id, &req)
        .await
    {
        Ok(result) => Json(json!({"ok": true, "result": result})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_list_agent_docs(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::ProjectRead,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::GET,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    match state.platform.projects.list_agent_docs(&owner, &project) {
        Ok(items) => Json(json!({"ok": true, "items": items})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_read_agent_doc(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Query(query): Query<DocPathQuery>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::ProjectRead,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::GET,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    let Some(name) = query
        .path
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": {"code":"PLATFORM_AGENT_DOC_INVALID","message":"missing name query param"}})),
        )
            .into_response();
    };
    match state
        .platform
        .projects
        .read_agent_doc(&owner, &project, name)
    {
        Ok(content) => {
            Json(json!({"ok": true, "doc": {"name": name, "content": content}})).into_response()
        }
        Err(err) if err.code == "PLATFORM_AGENT_DOC_INVALID" => (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": {"code": err.code, "message": err.message}})),
        )
            .into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_upsert_agent_doc_file(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Query(query): Query<DocPathQuery>,
    body: Bytes,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::FilesWrite,
    ) {
        return response;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::PUT,
        &headers,
        body.clone(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    let Some(name) = query
        .path
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    else {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": {"code":"PLATFORM_AGENT_DOC_INVALID","message":"missing name query param"}})),
        )
            .into_response();
    };
    // Only user-editable docs can be written via REST (not MEMORY.md)
    if name == "MEMORY.md" {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"ok": false, "error": {"code":"PLATFORM_AGENT_DOC_READONLY","message":"MEMORY.md is managed by the assistant and cannot be written via REST"}})),
        )
            .into_response();
    }
    let content = String::from_utf8(body.to_vec()).unwrap_or_default();
    match state
        .platform
        .projects
        .upsert_agent_doc(&owner, &project, name, &content)
    {
        Ok(_) => Json(json!({"ok": true, "name": name})).into_response(),
        Err(err) if err.code == "PLATFORM_AGENT_DOC_INVALID" => (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": {"code": err.code, "message": err.message}})),
        )
            .into_response(),
        Err(err) => internal_error(err),
    }
}

// ── Webhook auth helper ──────────────────────────────────────────────────────

/// Auth failure kinds returned by [`verify_webhook_auth`].
enum AuthError {
    /// 401 — not authenticated or token invalid. Carries optional redirect URL from credential.
    Unauthenticated {
        message: String,
        redirect_url: Option<String>,
    },
    /// 403 — authenticated but role check failed. Carries optional redirect URL from credential.
    Forbidden {
        message: String,
        redirect_url: Option<String>,
    },
    /// 500 — server-side misconfiguration.
    Internal(String),
}

/// Returns `true` when the request is a browser page navigation (not a fetch/XHR call).
///
/// Primary signal: `Sec-Fetch-Mode: navigate` + `Sec-Fetch-Dest: document` (Fetch Metadata spec).
/// Fallback: `Accept` header contains `text/html` (for older browsers without Sec-Fetch support).
fn is_page_navigation(headers: &HeaderMap) -> bool {
    let mode = headers
        .get("sec-fetch-mode")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let dest = headers
        .get("sec-fetch-dest")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !mode.is_empty() || !dest.is_empty() {
        // Sec-Fetch headers present — use them as authoritative signal.
        return mode == "navigate" && dest == "document";
    }

    // Fallback: Accept header contains text/html (old browsers, curl with explicit Accept).
    headers
        .get("accept")
        .and_then(|v| v.to_str().ok())
        .map(|a| a.contains("text/html"))
        .unwrap_or(false)
}

/// Returns `true` when the request explicitly asks for an SSE event stream.
///
/// Used by the webhook ingress to decide between the normal synchronous response path
/// and the streaming path that forwards [`Signal`] values as SSE messages.
fn wants_event_stream(headers: &HeaderMap) -> bool {
    headers
        .get("accept")
        .and_then(|v| v.to_str().ok())
        .map(|accept| accept.contains("text/event-stream"))
        .unwrap_or(false)
}

/// Verifies the auth requirement of a webhook trigger spec.
///
/// Returns:
/// - `Ok(Some(claims))` — JWT auth passed; claims to inject as `payload.auth`
/// - `Ok(None)` — HMAC / API key auth passed; no claims to inject
/// - `Err(AuthError)` — auth failed
fn verify_webhook_auth(
    headers: &HeaderMap,
    body: &Bytes,
    auth_type: &str,
    auth_credential: &str,
    required_roles: &[String],
    credentials: &crate::platform::services::CredentialService,
    owner: &str,
    project: &str,
) -> Result<Option<Value>, AuthError> {
    if auth_type.is_empty() || auth_type == "none" {
        return Ok(None);
    }
    if auth_credential.is_empty() {
        return Err(AuthError::Internal(
            "auth_type set but auth_credential is empty".to_string(),
        ));
    }

    let credential = credentials
        .get_project_credential(owner, project, auth_credential)
        .map_err(|e| AuthError::Internal(e.message))?
        .ok_or_else(|| {
            AuthError::Internal(format!("auth credential '{auth_credential}' not found"))
        })?;

    // Read redirect URLs from credential secret — used when building AuthError responses.
    let auth_redirect = credential
        .secret
        .get("auth_redirect")
        .and_then(|v| v.as_str())
        .map(ToString::to_string);
    let auth_forbidden_redirect = credential
        .secret
        .get("auth_forbidden_redirect")
        .and_then(|v| v.as_str())
        .map(ToString::to_string);

    match auth_type {
        "jwt" => {
            use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};

            // Check Authorization: Bearer header first, then Cookie: <cookie_name>.
            //
            // The name comes from the credential, beside `auth_redirect`, because
            // an application's session cookie is the application's to name. Both
            // branches of this used to return `zebflow_session` — the platform's
            // own Studio cookie — so a project's JWT auth read the admin session
            // instead of its own, and an app either had to clobber that name or
            // could not use webhook JWT auth at all. Default unchanged for
            // anything already relying on it.
            let cookie_name = credential
                .secret
                .get("cookie_name")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|v| !v.is_empty())
                .unwrap_or("zebflow_session");
            let token = headers
                .get("Authorization")
                .and_then(|h| h.to_str().ok())
                .and_then(|h| h.strip_prefix("Bearer "))
                .map(ToString::to_string)
                .or_else(|| {
                    let cookie = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
                    cookie.split(';').map(str::trim).find_map(|part| {
                        part.strip_prefix(&format!("{cookie_name}="))
                            .map(ToString::to_string)
                    })
                })
                .ok_or_else(|| AuthError::Unauthenticated {
                    message: "missing Authorization: Bearer <token> or session cookie".to_string(),
                    redirect_url: auth_redirect.clone(),
                })?;

            let algo_str = credential
                .secret
                .get("algorithm")
                .and_then(|v| v.as_str())
                .unwrap_or("HS256");
            let algorithm = match algo_str.to_ascii_uppercase().as_str() {
                "HS256" => Algorithm::HS256,
                "HS384" => Algorithm::HS384,
                "HS512" => Algorithm::HS512,
                "RS256" => Algorithm::RS256,
                "RS384" => Algorithm::RS384,
                "RS512" => Algorithm::RS512,
                other => {
                    return Err(AuthError::Internal(format!(
                        "unsupported JWT algorithm '{other}'"
                    )));
                }
            };

            let decoding_key = match algorithm {
                Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512 => {
                    let secret = credential
                        .secret
                        .get("secret")
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            AuthError::Internal(
                                "jwt_signing_key credential missing 'secret' field".to_string(),
                            )
                        })?;
                    DecodingKey::from_secret(secret.as_bytes())
                }
                _ => {
                    let pem = credential
                        .secret
                        .get("public_key")
                        .or_else(|| credential.secret.get("private_key"))
                        .and_then(|v| v.as_str())
                        .ok_or_else(|| {
                            AuthError::Internal(
                                "jwt_signing_key credential missing 'public_key'".to_string(),
                            )
                        })?;
                    DecodingKey::from_rsa_pem(pem.as_bytes())
                        .map_err(|e| AuthError::Internal(e.to_string()))?
                }
            };

            let mut validation = Validation::new(algorithm);
            validation.validate_exp = true;

            let token_data = decode::<Value>(&token, &decoding_key, &validation).map_err(|e| {
                AuthError::Unauthenticated {
                    message: format!("JWT invalid: {e}"),
                    redirect_url: auth_redirect.clone(),
                }
            })?;

            let claims = token_data.claims;

            // Role check — only applies when the trigger specifies required roles.
            // The claim is meant to be an array of strings; a string is read as
            // one role or a comma-separated list, because `--claim "roles=admin"`
            // and `--claim "roles=['admin']"` are what people actually write, and
            // both used to authorise nobody.
            if !required_roles.is_empty() {
                let user_roles = token_roles(claims.get("roles"));
                let authorized = required_roles
                    .iter()
                    .any(|r| user_roles.iter().any(|u| u == r));
                if !authorized {
                    return Err(AuthError::Forbidden {
                        message: format!("roles {:?} are not permitted for this route", user_roles),
                        redirect_url: auth_forbidden_redirect.clone(),
                    });
                }
            }

            Ok(Some(claims))
        }

        "hmac" => {
            use hmac::{Hmac, Mac};

            // Read configurable fields from credential secret.
            let provider = credential
                .secret
                .get("provider")
                .and_then(|v| v.as_str())
                .unwrap_or("generic");
            let sig_header_name = credential
                .secret
                .get("signature_header")
                .and_then(|v| v.as_str())
                .unwrap_or("X-Hub-Signature-256");
            let sig_prefix = credential
                .secret
                .get("signature_prefix")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let sig_encoding = credential
                .secret
                .get("signature_encoding")
                .and_then(|v| v.as_str())
                .unwrap_or("hex");
            let algorithm = credential
                .secret
                .get("algorithm")
                .and_then(|v| v.as_str())
                .unwrap_or("sha256");
            let replay_tolerance = credential
                .secret
                .get("replay_tolerance")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);

            let secret = credential
                .secret
                .get("secret")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    AuthError::Internal("hmac credential missing 'secret' field".to_string())
                })?;

            // Get signature value from the configured header.
            let sig_raw = headers
                .get(sig_header_name)
                .and_then(|h| h.to_str().ok())
                .ok_or_else(|| AuthError::Unauthenticated {
                    message: format!("missing HMAC signature header '{sig_header_name}'"),
                    redirect_url: None,
                })?;

            // Provider-specific parsing: extract expected signature bytes + message to sign.
            let (expected_sig_str, message_to_sign): (String, Vec<u8>) = match provider {
                // Stripe: header = "t=1234567890,v1=<hex_sig>", signs "{t}.{body}"
                "stripe" => {
                    let parts: std::collections::HashMap<&str, &str> = sig_raw
                        .split(',')
                        .filter_map(|p| p.split_once('='))
                        .collect();
                    let timestamp = parts.get("t").ok_or_else(|| AuthError::Unauthenticated {
                        message: "Stripe signature missing 't' field".to_string(),
                        redirect_url: None,
                    })?;
                    let sig = parts.get("v1").ok_or_else(|| AuthError::Unauthenticated {
                        message: "Stripe signature missing 'v1' field".to_string(),
                        redirect_url: None,
                    })?;

                    if replay_tolerance > 0 {
                        let ts: i64 =
                            timestamp.parse().map_err(|_| AuthError::Unauthenticated {
                                message: "Stripe timestamp invalid".to_string(),
                                redirect_url: None,
                            })?;
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs() as i64;
                        if (now - ts).unsigned_abs() > replay_tolerance {
                            return Err(AuthError::Unauthenticated {
                                message: format!("replay tolerance exceeded ({replay_tolerance}s)"),
                                redirect_url: None,
                            });
                        }
                    }

                    let body_str = std::str::from_utf8(body.as_ref()).unwrap_or("");
                    let msg = format!("{timestamp}.{body_str}");
                    (sig.to_string(), msg.into_bytes())
                }
                // Slack: timestamp in X-Slack-Request-Timestamp, signs "v0:{ts}:{body}"
                "slack" => {
                    let timestamp = headers
                        .get("X-Slack-Request-Timestamp")
                        .and_then(|h| h.to_str().ok())
                        .ok_or_else(|| AuthError::Unauthenticated {
                            message: "missing X-Slack-Request-Timestamp header".to_string(),
                            redirect_url: None,
                        })?;

                    if replay_tolerance > 0 {
                        let ts: i64 =
                            timestamp.parse().map_err(|_| AuthError::Unauthenticated {
                                message: "Slack timestamp invalid".to_string(),
                                redirect_url: None,
                            })?;
                        let now = std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs() as i64;
                        if (now - ts).unsigned_abs() > replay_tolerance {
                            return Err(AuthError::Unauthenticated {
                                message: format!("replay tolerance exceeded ({replay_tolerance}s)"),
                                redirect_url: None,
                            });
                        }
                    }

                    let sig = if !sig_prefix.is_empty() {
                        sig_raw.strip_prefix(sig_prefix).unwrap_or(sig_raw)
                    } else {
                        sig_raw
                    };
                    let body_str = std::str::from_utf8(body.as_ref()).unwrap_or("");
                    let msg = format!("v0:{timestamp}:{body_str}");
                    (sig.to_string(), msg.into_bytes())
                }
                // Generic / GitHub / Shopify: strip prefix, sign raw body.
                _ => {
                    let sig = if !sig_prefix.is_empty() {
                        sig_raw.strip_prefix(sig_prefix).unwrap_or(sig_raw)
                    } else {
                        sig_raw
                    };
                    (sig.to_string(), body.to_vec())
                }
            };

            // Decode expected signature from hex or base64.
            let expected_bytes = match sig_encoding {
                "base64" => {
                    use base64::Engine as _;
                    base64::engine::general_purpose::STANDARD
                        .decode(expected_sig_str.as_bytes())
                        .map_err(|e| AuthError::Unauthenticated {
                            message: format!("invalid base64 in signature: {e}"),
                            redirect_url: None,
                        })?
                }
                _ => hex::decode(&expected_sig_str).map_err(|e| AuthError::Unauthenticated {
                    message: format!("invalid hex in signature: {e}"),
                    redirect_url: None,
                })?,
            };

            // Compute HMAC and verify using constant-time comparison.
            match algorithm {
                "sha1" => {
                    type HmacSha1 = Hmac<sha1::Sha1>;
                    let mut mac = HmacSha1::new_from_slice(secret.as_bytes())
                        .map_err(|e| AuthError::Internal(e.to_string()))?;
                    mac.update(&message_to_sign);
                    mac.verify_slice(&expected_bytes)
                        .map_err(|_| AuthError::Unauthenticated {
                            message: "HMAC signature mismatch".to_string(),
                            redirect_url: None,
                        })?;
                }
                _ => {
                    type HmacSha256 = Hmac<sha2::Sha256>;
                    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
                        .map_err(|e| AuthError::Internal(e.to_string()))?;
                    mac.update(&message_to_sign);
                    mac.verify_slice(&expected_bytes)
                        .map_err(|_| AuthError::Unauthenticated {
                            message: "HMAC signature mismatch".to_string(),
                            redirect_url: None,
                        })?;
                }
            }

            Ok(None)
        }

        "api_key" => {
            let provided = headers
                .get("X-API-Key")
                .or_else(|| headers.get("x-api-key"))
                .and_then(|h| h.to_str().ok())
                .or_else(|| {
                    headers
                        .get("Authorization")
                        .and_then(|h| h.to_str().ok())
                        .and_then(|h| h.strip_prefix("ApiKey "))
                })
                .ok_or_else(|| AuthError::Unauthenticated {
                    message: "missing API key (X-API-Key or Authorization: ApiKey <key>)"
                        .to_string(),
                    redirect_url: None,
                })?;

            let stored = credential
                .secret
                .get("key")
                .and_then(|v| v.as_str())
                .ok_or_else(|| {
                    AuthError::Internal("api_key credential missing 'key' field".to_string())
                })?;

            if provided != stored {
                return Err(AuthError::Unauthenticated {
                    message: "invalid API key".to_string(),
                    redirect_url: None,
                });
            }

            Ok(None)
        }

        other => Err(AuthError::Internal(format!(
            "unknown auth_type '{other}'. Valid: jwt, hmac, api_key, none"
        ))),
    }
}

// ── Weberror dispatch helper ─────────────────────────────────────────────────

/// Finds and runs the best matching weberror pipeline.
///
/// Returns a rendered response (HTML or JSON with the error status code) if a
/// matching pipeline is found and executes successfully, `None` otherwise.
async fn dispatch_weberror(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
    error_code: u16,
    error_payload: Value,
) -> Option<Response> {
    use crate::pipeline::nodes::basic::trigger::weberror::match_specificity;

    // Find the most specific matching weberror pipeline.
    let mut best: Option<(
        u8,
        crate::platform::services::pipeline_runtime::CompiledPipeline,
    )> = None;
    for compiled in state.platform.pipeline_runtime.list_project(owner, project) {
        for trigger in &compiled.weberror_triggers {
            if let Some(spec) = match_specificity(&trigger.code, error_code) {
                if best.as_ref().map_or(true, |(s, _)| spec > *s) {
                    best = Some((spec, compiled.clone()));
                    break;
                }
            }
        }
    }
    let (_, compiled) = best?;

    let credentials = state.platform.credentials.clone();
    let engine = BasicPipelineEngine::new(
        Arc::new(state.platform.project_sandbox(owner, project)),
        state.frontend.rwe.clone(),
        Some(credentials),
    )
    .with_platform(state.platform.clone())
    .with_template_cache(state.template_cache.clone())
    .with_project_layout(state.platform.projects.project_layout(owner, project).ok())
    .with_ws_hub(state.platform.ws_hub.clone())
    .with_ws_client_manager(state.ws_client_manager.clone())
    .with_state_bus(state.platform.state_bus.clone())
    .with_data_root(state.platform.config.data_root.clone());

    let ctx = PipelineContext {
        owner: owner.to_string(),
        project: project.to_string(),
        pipeline: compiled.graph.id.clone(),
        request_id: format!("weberror-{error_code}"),
        route: Default::default(),
        input: error_payload,
        trigger: None,
        placeholder: None,
    };

    let output = engine.execute_async(&compiled.graph, &ctx).await.ok()?;

    let status = StatusCode::from_u16(error_code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);

    // Prefer rendered HTML output.
    if let Some(html) = output.value.get("html").and_then(Value::as_str) {
        let mut html = html.to_string();
        if let Some(css) = output
            .value
            .get("hydration_payload")
            .and_then(|hp| hp.get("css"))
            .and_then(Value::as_str)
        {
            html = crate::rwe::core::render::insert_engine_styles(&html, css);
        }
        let scripts = output
            .value
            .get("compiled_scripts")
            .cloned()
            .and_then(|v| serde_json::from_value::<Vec<CompiledScript>>(v).ok())
            .unwrap_or_default();
        let externalized =
            match externalize_rwe_scripts(state, &html, &scripts, Some((owner, project))) {
                Ok(html) => html,
                Err(err) => return Some(internal_error(err)),
            };
        return Some((status, Html(externalized)).into_response());
    }

    // JSON fallback.
    Some((status, Json(output.value)).into_response())
}

// ────────────────────────────────────────────────────────────────────────────

async fn public_webhook_ingress(
    State(state): State<PlatformAppState>,
    Path((owner, project, tail)): Path<(String, String, String)>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    let owner = crate::platform::model::slug_segment(&owner);
    let project = crate::platform::model::slug_segment(&project);
    let path = format!("/{}", tail.trim_start_matches('/'));
    let method_key = method.as_str().to_ascii_uppercase();
    let exec_start = std::time::Instant::now();
    let project_cfg = match state.platform.zebflow_cfg.read_or_default(&owner, &project) {
        Ok(config) => config,
        Err(err) => return internal_error(err),
    };
    let webhook_body_max_mb = project_cfg
        .configs
        .files
        .uploads
        .effective_webhook_body_max_mb();
    let webhook_body_limit_bytes = (webhook_body_max_mb as usize) * 1024 * 1024;
    if body.len() > webhook_body_limit_bytes {
        return (
            StatusCode::PAYLOAD_TOO_LARGE,
            Json(json!({
                "ok": false,
                "error": format!(
                    "webhook body {} bytes exceeds limit of {} MB",
                    body.len(),
                    webhook_body_max_mb
                )
            })),
        )
            .into_response();
    }
    let request_id = format!(
        "webhook-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis()
    );

    if let Ok(Some(placement)) = state.platform.cluster_placement.get(&owner, &project) {
        if placement.target == ProjectRuntimePlacementTarget::Worker {
            if let Some(worker_id) = placement.worker_id.as_deref() {
                return match forward_runtime_webhook_to_worker(
                    &state, &method, &uri, &headers, &body, worker_id,
                )
                .await
                {
                    Ok(response) => response,
                    Err(err) => internal_error(err),
                };
            }
        }
    }

    struct Candidate {
        compiled: crate::platform::services::pipeline_runtime::CompiledPipeline,
        path_params: serde_json::Map<String, Value>,
        static_segments: usize,
        dynamic_segments: usize,
        total_segments: usize,
        auth_type: String,
        auth_credential: String,
        auth_required_role: Vec<String>,
        auth_optional: bool,
    }

    let mut candidates = Vec::<Candidate>::new();
    for compiled in state
        .platform
        .pipeline_runtime
        .list_project(&owner, &project)
    {
        for trigger in &compiled.webhook_triggers {
            if !trigger.method.eq_ignore_ascii_case(&method_key) {
                continue;
            }
            let Some(path_match) = match_webhook_path(&trigger.path, &path) else {
                continue;
            };
            candidates.push(Candidate {
                auth_type: trigger.auth_type.clone(),
                auth_credential: trigger.auth_credential.clone(),
                auth_required_role: trigger.auth_required_role.clone(),
                auth_optional: trigger.auth_optional,
                compiled: compiled.clone(),
                path_params: path_match.params,
                static_segments: path_match.static_segments,
                dynamic_segments: path_match.dynamic_segments,
                total_segments: path_match.total_segments,
            });
        }
    }

    candidates.sort_by(|a, b| {
        b.static_segments
            .cmp(&a.static_segments)
            .then(a.dynamic_segments.cmp(&b.dynamic_segments))
            .then(b.total_segments.cmp(&a.total_segments))
            .then(a.compiled.file_rel_path.cmp(&b.compiled.file_rel_path))
    });

    let Some(selected) = candidates.into_iter().next() else {
        if let Some(err_resp) = dispatch_weberror(
            &state,
            &owner,
            &project,
            404,
            json!({"error": "not found", "path": path, "method": method_key}),
        )
        .await
        {
            return err_resp;
        }
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"ok": false, "error": "not found"})),
        )
            .into_response();
    };
    let retention = resolve_invocation_retention(&project_cfg, Some(&selected.compiled.graph));
    // Verify trigger-level auth before executing the pipeline.
    let auth_claims = if is_controller_call(&state, &headers) {
        Ok(None)
    } else {
        let verified = verify_webhook_auth(
            &headers,
            &body,
            &selected.auth_type,
            &selected.auth_credential,
            &selected.auth_required_role,
            &state.platform.credentials,
            &owner,
            &project,
        );
        // `--auth-optional`: the route is public and only wants to know who is
        // signed in. A missing, expired or role-less token is a guest, not a
        // refusal; the page reads `input.auth` and decides. A misconfigured
        // credential is still an error — that is the operator's, not the visitor's.
        match verified {
            Err(AuthError::Unauthenticated { .. }) | Err(AuthError::Forbidden { .. })
                if selected.auth_optional =>
            {
                Ok(None)
            }
            other => other,
        }
    };
    let auth_claims = match auth_claims {
        Ok(claims) => claims,
        Err(auth_err) => {
            let (status, msg, redirect_url) = match auth_err {
                AuthError::Unauthenticated {
                    message,
                    redirect_url,
                } => (StatusCode::UNAUTHORIZED, message, redirect_url),
                AuthError::Forbidden {
                    message,
                    redirect_url,
                } => (StatusCode::FORBIDDEN, message, redirect_url),
                AuthError::Internal(message) => (StatusCode::INTERNAL_SERVER_ERROR, message, None),
            };

            if is_page_navigation(&headers) {
                if let Some(url) = redirect_url {
                    // Carry the page the visitor wanted: a member who opens
                    // the WA group's event link, signs in, and lands back on
                    // that event, not on a generic home. `next` is the name
                    // every login form already reads (Django, Rails' return_to);
                    // a redirect that already carries a query gets `&next=`.
                    let wanted = {
                        let mut wanted = path.clone();
                        if let Some(q) = uri.query().filter(|q| !q.is_empty()) {
                            wanted.push('?');
                            wanted.push_str(q);
                        }
                        wanted
                    };
                    let sep = if url.contains('?') { '&' } else { '?' };
                    let url = format!("{url}{sep}next={}", url_query_encode(&wanted));
                    return axum::response::Redirect::to(&url).into_response();
                }
            }

            if let Some(err_resp) = dispatch_weberror(
                &state,
                &owner,
                &project,
                status.as_u16(),
                json!({"error": msg, "path": path, "method": method_key}),
            )
            .await
            {
                return err_resp;
            }
            return (status, Json(json!({"ok": false, "error": msg}))).into_response();
        }
    };

    if let Ok(Some(placement)) = state.platform.cluster_placement.get(&owner, &project) {
        if placement.target == ProjectRuntimePlacementTarget::Worker {
            if let Some(worker_id) = placement.worker_id.as_deref() {
                return match forward_runtime_webhook_to_worker(
                    &state, &method, &uri, &headers, &body, worker_id,
                )
                .await
                {
                    Ok(response) => response,
                    Err(err) => internal_error(err),
                };
            }
        }
    }

    let mut graph = selected.compiled.graph.clone();
    if let Err(err) = hydrate_template_markup(&state, &owner, &project, &mut graph) {
        state.platform.pipeline_hits.record_failure(
            &owner,
            &project,
            &selected.compiled.file_rel_path,
            "webhook.ingress",
            err.code,
            &err.message,
        );
        let _ = state.platform.data.log_pipeline_invocation(
            &owner,
            &project,
            &selected.compiled.file_rel_path,
            &PipelineInvocationEntry {
                run_id: request_id.clone(),
                at: std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64,
                duration_ms: exec_start.elapsed().as_millis() as u64,
                status: "error".to_string(),
                trigger: "webhook".to_string(),
                error: Some(err.message.clone()),
                trace: vec![],
            },
            retention.max_invocations,
            retention.max_age_secs,
        );
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": {"code": err.code, "message": err.message}})),
        )
            .into_response();
    }
    if let Err(err) = apply_rwe_project_options(&state, &owner, &project, &mut graph) {
        return internal_error(err);
    }

    let mut input = match build_webhook_ingress_input(
        &state.platform,
        &owner,
        &project,
        &request_id,
        &method,
        &uri,
        &headers,
        &body,
        &path,
        &selected.path_params,
    )
    .await
    {
        Ok(input) => input,
        Err(err) => {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"ok": false, "error": {"code": err.code, "message": err.message}})),
            )
                .into_response();
        }
    };
    // Inject JWT claims as `auth` field when auth_type == "jwt".
    if let Some(claims) = auth_claims {
        if let Value::Object(ref mut map) = input {
            map.insert("auth".to_string(), claims);
        }
    }

    // Build immutable trigger snapshot — available to every node via metadata.
    let trigger = json!({
        "auth": input.get("auth").cloned().unwrap_or(Value::Null),
        "params": input.get("params").cloned().unwrap_or(json!({})),
        "query": input.get("query").cloned().unwrap_or(json!({})),
        // Preserve the original query for SSR URL hooks; the legacy query map
        // cannot represent repeated keys or distinguish encoded values.
        "search": uri.query().map(|query| format!("?{query}")).unwrap_or_default(),
        // Page URL context is separate from the pipeline's relative route.
        "pathname": webhook_url::pathname(&uri),
        "headers": safe_headers(&headers),
    });

    let credentials = state.platform.credentials.clone();
    let graph_for_run = graph.clone();
    let ctx = PipelineContext {
        owner: owner.clone(),
        project: project.clone(),
        pipeline: graph.id.clone(),
        request_id: request_id.clone(),
        route: path.clone(),
        input: input.clone(),
        trigger: Some(trigger),
        placeholder: None,
    };
    let file_rel_path = selected.compiled.file_rel_path.clone();
    let engine = BasicPipelineEngine::new(
        Arc::new(state.platform.project_sandbox(&owner, &project)),
        state.frontend.rwe.clone(),
        Some(credentials),
    )
    .with_platform(state.platform.clone())
    .with_template_cache(state.template_cache.clone())
    .with_project_layout(
        state
            .platform
            .projects
            .project_layout(&owner, &project)
            .ok(),
    )
    .with_ws_hub(state.platform.ws_hub.clone())
    .with_ws_client_manager(state.ws_client_manager.clone())
    .with_state_bus(state.platform.state_bus.clone())
    .with_data_root(state.platform.config.data_root.clone());

    // ── SSE streaming path (ExecutionBus) ──────────────────────────────────────
    // When the caller explicitly sends `Accept: text/event-stream`, create an
    // ExecutionBus, subscribe an SSE receiver, and forward Signal values as SSE
    // `event: signal` messages.  The final result is `event: done` / `event: error`.
    // Normal webhook behavior is completely unchanged — this branch returns early.
    if wants_event_stream(&headers) {
        let bus = std::sync::Arc::new(ExecutionBus::new(256));
        let mut signal_rx = bus.subscribe();
        let options = ExecuteOptions { bus: Some(bus) };

        let platform_sse = state.platform.clone();
        let file_rel_path_sse = file_rel_path.clone();
        let request_id_sse = request_id.clone();
        let owner_sse = owner.clone();
        let project_sse = project.clone();

        // One-shot channel for the final pipeline result.
        let (result_tx, mut result_rx) = tokio::sync::oneshot::channel();

        tokio::spawn(async move {
            let result = engine
                .execute_with_options_async(&graph_for_run, &ctx, &options)
                .await;
            let _ = result_tx.send(result);
        });

        let stream = async_stream::stream! {
            // Drain signals until the bus closes or the result arrives.
            loop {
                tokio::select! {
                    biased;
                    sig = signal_rx.recv() => {
                        match sig {
                            Ok(signal) => {
                                let payload = serde_json::to_string(&signal)
                                    .unwrap_or_else(|_| "{}".to_string());
                                yield Ok::<_, Infallible>(Event::default()
                                    .event("signal")
                                    .data(payload));
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        }
                    }
                    result = &mut result_rx => {
                        match result {
                            Ok(Ok(output)) => {
                                let done = json!({ "ok": true, "value": output.value });
                                yield Ok(Event::default().event("done").data(done.to_string()));
                            }
                            Ok(Err(err)) => {
                                let error = json!({
                                    "ok": false,
                                    "error": { "code": err.code, "message": err.message }
                                });
                                yield Ok(Event::default().event("error").data(error.to_string()));
                            }
                            Err(_) => {}
                        }
                        return;
                    }
                }
            }

            // Bus closed — drain remaining signals.
            loop {
                match signal_rx.try_recv() {
                    Ok(signal) => {
                        let payload = serde_json::to_string(&signal)
                            .unwrap_or_else(|_| "{}".to_string());
                        yield Ok::<_, Infallible>(Event::default()
                            .event("signal")
                            .data(payload));
                    }
                    Err(_) => break,
                }
            }

            // Collect the final pipeline result.
            match result_rx.await {
                Ok(Ok(output)) => {
                    let elapsed_ms = exec_start.elapsed().as_millis() as u64;
                    platform_sse.pipeline_hits.record_success(
                        &owner_sse, &project_sse, &file_rel_path_sse,
                    );
                    let _ = platform_sse.data.log_pipeline_invocation(
                        &owner_sse, &project_sse, &file_rel_path_sse,
                        &PipelineInvocationEntry {
                            run_id: request_id_sse.clone(),
                            at: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default().as_secs() as i64,
                            duration_ms: elapsed_ms,
                            status: "ok".to_string(),
                            trigger: "webhook".to_string(),
                            error: None,
                            trace: output.node_trace.clone(),
                        },
                        retention.max_invocations,
                        retention.max_age_secs,
                    );
                    let done = json!({ "ok": true, "value": output.value });
                    yield Ok(Event::default().event("done").data(done.to_string()));
                }
                Ok(Err(err)) => {
                    let elapsed_ms = exec_start.elapsed().as_millis() as u64;
                    platform_sse.pipeline_hits.record_failure(
                        &owner_sse, &project_sse, &file_rel_path_sse,
                        "webhook.ingress", err.code, &err.message,
                    );
                    let _ = platform_sse.data.log_pipeline_invocation(
                        &owner_sse, &project_sse, &file_rel_path_sse,
                        &PipelineInvocationEntry {
                            run_id: request_id_sse.clone(),
                            at: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap_or_default().as_secs() as i64,
                            duration_ms: elapsed_ms,
                            status: "error".to_string(),
                            trigger: "webhook".to_string(),
                            error: Some(err.message.clone()),
                            trace: err.node_trace.clone(),
                        },
                        retention.max_invocations,
                        retention.max_age_secs,
                    );
                    let error = json!({
                        "ok": false,
                        "error": { "code": err.code, "message": err.message }
                    });
                    yield Ok(Event::default().event("error").data(error.to_string()));
                }
                Err(_) => {}
            }
        };

        return Sse::new(stream)
            .keep_alive(KeepAlive::default())
            .into_response();
    }

    let output = match engine.execute_async(&graph_for_run, &ctx).await {
        Ok(output) => output,
        Err(err) => {
            state.platform.pipeline_hits.record_failure(
                &owner,
                &project,
                &file_rel_path,
                "webhook.ingress",
                err.code,
                &err.message,
            );
            let _ = state.platform.data.log_pipeline_invocation(
                &owner,
                &project,
                &file_rel_path,
                &PipelineInvocationEntry {
                    run_id: request_id.clone(),
                    at: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs() as i64,
                    duration_ms: exec_start.elapsed().as_millis() as u64,
                    status: "error".to_string(),
                    trigger: "webhook".to_string(),
                    error: Some(err.message.clone()),
                    trace: err.node_trace.clone(),
                },
                retention.max_invocations,
                retention.max_age_secs,
            );
            if let Some(err_resp) = dispatch_weberror(
                &state,
                &owner,
                &project,
                500,
                json!({"error": err.message, "code": err.code}),
            )
            .await
            {
                return err_resp;
            }
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"ok": false, "error": {"code": err.code, "message": err.message}})),
            )
                .into_response();
        }
    };
    state
        .platform
        .pipeline_hits
        .record_success(&owner, &project, &file_rel_path);
    let _ = state.platform.data.log_pipeline_invocation(
        &owner,
        &project,
        &file_rel_path,
        &PipelineInvocationEntry {
            run_id: request_id.clone(),
            at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
            duration_ms: exec_start.elapsed().as_millis() as u64,
            status: "ok".to_string(),
            trigger: "webhook".to_string(),
            error: None,
            trace: output.node_trace.clone(),
        },
        retention.max_invocations,
        retention.max_age_secs,
    );

    // ── n.web.response — explicit response envelope ───────────────────────────
    // When the pipeline ends with n.web.response, its output carries a __zf_response
    // envelope that fully controls the HTTP response. Handle it before legacy magic keys.
    if let Some(resp_cfg) = output.value.get("__zf_response").cloned() {
        let status = resp_cfg
            .get("status")
            .and_then(Value::as_u64)
            .and_then(|c| StatusCode::from_u16(c as u16).ok())
            .unwrap_or(StatusCode::OK);

        // Build helpers for cookie + extra headers
        let zf_cookie = build_zf_cookie(&resp_cfg);
        let zf_headers = build_zf_headers(&resp_cfg);

        // HTML (template mode)
        if let Some(html) = resp_cfg.get("html").and_then(Value::as_str) {
            let mut html = html.to_string();
            // Inject project styles/main.css as inline <style> so CSS vars are available.
            if let Ok(tmpl_root) = state
                .platform
                .projects
                .get_project_template_root(&owner, &project)
            {
                let main_css_path = tmpl_root.join("styles").join("main.css");
                if let Ok(project_css) = std::fs::read_to_string(&main_css_path) {
                    if !project_css.trim().is_empty() {
                        let style_block =
                            format!("<style data-project-theme>{project_css}</style>");
                        html = insert_project_theme_block(html, &style_block);
                    }
                }
            }
            if let Some(css) = resp_cfg
                .get("hydration_payload")
                .and_then(|hp| hp.get("css"))
                .and_then(Value::as_str)
            {
                html = crate::rwe::core::render::insert_engine_styles(&html, css);
            }
            let scripts = resp_cfg
                .get("compiled_scripts")
                .cloned()
                .and_then(|v| serde_json::from_value::<Vec<CompiledScript>>(v).ok())
                .unwrap_or_default();
            let externalized =
                match externalize_rwe_scripts(&state, &html, &scripts, Some((&owner, &project))) {
                    Ok(html) => html,
                    Err(err) => return internal_error(err),
                };
            let mut resp = Html(externalized).into_response();
            *resp.status_mut() = status;
            apply_zf_extras(&mut resp, &zf_cookie, &zf_headers);
            return resp;
        }

        // Redirect
        if let Some(loc) = resp_cfg.get("location").and_then(Value::as_str) {
            let mut resp = (status, "").into_response();
            if let Ok(v) = HeaderValue::from_str(loc) {
                resp.headers_mut().insert(LOCATION, v);
            }
            apply_zf_extras(&mut resp, &zf_cookie, &zf_headers);
            return resp;
        }

        // Plain text message
        if let Some(msg) = resp_cfg.get("message").and_then(Value::as_str) {
            let mut resp = (status, msg.to_string()).into_response();
            apply_zf_extras(&mut resp, &zf_cookie, &zf_headers);
            return resp;
        }

        // Explicit body path
        if let Some(body) = resp_cfg.get("body") {
            if !body.is_null() {
                let mut resp = (status, Json(body.clone())).into_response();
                apply_zf_extras(&mut resp, &zf_cookie, &zf_headers);
                return resp;
            }
        }

        // Default: JSON response — pipeline output minus __zf_response
        let mut out = output.value.clone();
        if let Value::Object(ref mut map) = out {
            map.remove("__zf_response");
        }
        let mut resp = (status, Json(out)).into_response();
        apply_zf_extras(&mut resp, &zf_cookie, &zf_headers);
        return resp;
    }

    // ── _set_cookie convention (legacy) ──────────────────────────────────────
    // If the pipeline output contains `_set_cookie`, build the Set-Cookie header string.
    // Applied to the final response regardless of response type.
    let set_cookie_header: Option<String> = output.value.get("_set_cookie").and_then(|sc| {
        let name = sc.get("name")?.as_str()?;
        let value = sc.get("value")?.as_str()?;
        let max_age = sc.get("max_age").and_then(Value::as_i64).unwrap_or(900);
        let path = sc.get("path").and_then(Value::as_str).unwrap_or("/");
        let same_site = sc.get("same_site").and_then(Value::as_str).unwrap_or("Lax");
        let http_only = sc.get("http_only").and_then(Value::as_bool).unwrap_or(true);
        let secure = sc.get("secure").and_then(Value::as_bool).unwrap_or(false);
        let mut parts = vec![
            format!("{name}={value}"),
            format!("Path={path}"),
            format!("Max-Age={max_age}"),
            format!("SameSite={same_site}"),
        ];
        if http_only {
            parts.push("HttpOnly".to_string());
        }
        // The spec parsed `secure`; the header must say it or the flag was a lie.
        if secure {
            parts.push("Secure".to_string());
        }
        Some(parts.join("; "))
    });

    // ── HTML response (rendered template output) ─────────────────────────────
    if let Some(html) = output.value.get("html").and_then(Value::as_str) {
        let mut html = html.to_string();
        // Re-inject Tailwind CSS from hydration_payload (extracted by RWE engine).
        if let Some(css) = output
            .value
            .get("hydration_payload")
            .and_then(|hp| hp.get("css"))
            .and_then(Value::as_str)
        {
            html = crate::rwe::core::render::insert_engine_styles(&html, css);
        }
        let scripts = output
            .value
            .get("compiled_scripts")
            .cloned()
            .and_then(|value| serde_json::from_value::<Vec<CompiledScript>>(value).ok())
            .unwrap_or_default();
        let externalized =
            match externalize_rwe_scripts(&state, &html, &scripts, Some((&owner, &project))) {
                Ok(html) => html,
                Err(err) => return internal_error(err),
            };
        let mut resp = Html(externalized).into_response();
        if let Some(ref cookie) = set_cookie_header {
            if let Ok(v) = HeaderValue::from_str(cookie) {
                resp.headers_mut().insert(SET_COOKIE, v);
            }
        }
        return resp;
    }

    // ── _status convention ────────────────────────────────────────────────────
    // If the pipeline output contains `_status`, use it as the HTTP status code.
    // For 4xx/5xx codes, try dispatching a weberror pipeline for a custom error page.
    if let Some(code) = output.value.get("_status").and_then(Value::as_u64) {
        let status = StatusCode::from_u16(code as u16).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
        if code >= 400 {
            // Build error payload — strip _status from body before forwarding.
            let mut error_body = output.value.clone();
            if let Value::Object(ref mut map) = error_body {
                map.remove("_status");
                map.remove("_set_cookie");
            }
            if let Some(err_resp) =
                dispatch_weberror(&state, &owner, &project, code as u16, error_body.clone()).await
            {
                return err_resp;
            }
            return (status, Json(error_body)).into_response();
        }
        let mut body = output.value.clone();
        if let Value::Object(ref mut map) = body {
            map.remove("_status");
            map.remove("_set_cookie");
        }
        let mut resp = (status, Json(body)).into_response();
        if let Some(ref cookie) = set_cookie_header {
            if let Ok(v) = HeaderValue::from_str(cookie) {
                resp.headers_mut().insert(SET_COOKIE, v);
            }
        }
        return resp;
    }

    let mut out_body = output.value.clone();
    if let Value::Object(ref mut map) = out_body {
        map.remove("_set_cookie");
    }
    let mut resp = Json(out_body).into_response();
    if let Some(ref cookie) = set_cookie_header {
        if let Ok(v) = HeaderValue::from_str(cookie) {
            resp.headers_mut().insert(SET_COOKIE, v);
        }
    }
    resp
}

async fn public_webhook_ingress_root(
    State(state): State<PlatformAppState>,
    Path((owner, project)): Path<(String, String)>,
    method: Method,
    uri: Uri,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    public_webhook_ingress(
        State(state),
        Path((owner, project, String::new())),
        method,
        uri,
        headers,
        body,
    )
    .await
}

async fn public_mapserver_ingress(
    State(state): State<PlatformAppState>,
    Path((owner, project, tail)): Path<(String, String, String)>,
    uri: Uri,
) -> Response {
    let owner = crate::platform::model::slug_segment(&owner);
    let project = crate::platform::model::slug_segment(&project);
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::GET,
            &HeaderMap::new(),
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }
    let raw_path = format!("/{}", tail.trim_start_matches('/'));

    let params = uri
        .query()
        .map(|raw| serde_urlencoded::from_str::<std::collections::HashMap<String, String>>(raw))
        .transpose();
    let params = match params {
        Ok(Some(map)) => map,
        Ok(None) => std::collections::HashMap::new(),
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok": false, "error": "invalid query string"})),
            )
                .into_response();
        }
    };

    // ── Z/X/Y tile URL detection ──────────────────────────────────────
    // Detect /{z}/{x}/{y}.{ext} suffix (e.g. /10/825/565.mvt)
    struct ZxyTileMatch {
        z: u8,
        x: u32,
        y: u32,
        ext: String,        // "mvt", "pbf", or "png"
        layer_path: String, // path with z/x/y stripped
    }

    fn parse_zxy_tile_url(path: &str) -> Option<ZxyTileMatch> {
        let trimmed = path.trim_end_matches('/');
        // Split from the right to get /{z}/{x}/{y}.{ext}
        let segments: Vec<&str> = trimmed.rsplitn(4, '/').collect();
        if segments.len() < 4 {
            return None;
        }
        // segments[0] = "{y}.{ext}", segments[1] = "{x}", segments[2] = "{z}", segments[3] = rest
        let y_ext = segments[0];
        let x_str = segments[1];
        let z_str = segments[2];
        let layer_rest = segments[3];

        // Parse {y}.{ext}
        let dot_pos = y_ext.rfind('.')?;
        let y_str = &y_ext[..dot_pos];
        let ext = &y_ext[dot_pos + 1..];

        // Validate extension
        let ext_lower = ext.to_ascii_lowercase();
        if !matches!(ext_lower.as_str(), "mvt" | "pbf" | "png") {
            return None;
        }

        let z: u8 = z_str.parse().ok()?;
        let x: u32 = x_str.parse().ok()?;
        let y: u32 = y_str.parse().ok()?;

        // Validate bounds
        let max_tile = 1u32.checked_shl(z as u32)?;
        if x >= max_tile || y >= max_tile {
            return None;
        }

        // Reconstruct layer path (everything before /{z}/...)
        let layer_path = if layer_rest.is_empty() {
            "/".to_string()
        } else {
            layer_rest.to_string()
        };

        Some(ZxyTileMatch {
            z,
            x,
            y,
            ext: ext_lower,
            layer_path,
        })
    }

    let zxy_match = parse_zxy_tile_url(&raw_path);

    // ── /stats suffix detection ───────────────────────────────────────
    let is_stats_request =
        zxy_match.is_none() && raw_path.trim_end_matches('/').ends_with("/stats");

    // Determine the layer path for manifest lookup
    let path = if let Some(ref zm) = zxy_match {
        zm.layer_path.clone()
    } else if is_stats_request {
        let stripped = raw_path.trim_end_matches('/');
        stripped[..stripped.len() - "/stats".len()].to_string()
    } else {
        raw_path.clone()
    };

    let manifest = match resolve_mapserver_manifest_for_path(&state, &owner, &project, &path) {
        Ok(Some(m)) => m,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({"ok": false, "error": "map layer not found"})),
            )
                .into_response();
        }
        Err(err) => return internal_error(err),
    };
    // ── Stats endpoint: early return ─────────────────────────────────
    if is_stats_request {
        match crate::mapserver::resolve::stats::compute_layer_stats(
            &manifest,
            crate::mapserver::resolve::stats::StatsAudience::Public,
        ) {
            Ok(stats_json) => {
                let mut resp = (StatusCode::OK, Json(stats_json)).into_response();
                resp.headers_mut().insert(
                    HeaderName::from_static("access-control-allow-origin"),
                    HeaderValue::from_static("*"),
                );
                return resp;
            }
            Err(err) => {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({"ok": false, "error": format!("stats computation failed: {err}")})),
                )
                    .into_response();
            }
        }
    }

    let layer_id = manifest.layer_id;
    let layer_path = manifest.path;
    let source_kind = manifest.source_kind;
    let source_path = manifest.source_ref;
    let mode = manifest.mode;
    let min_zoom = manifest.min_zoom;
    let max_zoom = manifest.max_zoom;
    let bbox_required = manifest.bbox_required;
    let max_features = manifest.max_features;
    let allowed_properties = manifest.allowed_properties;
    let layer_style_json = manifest.style;
    let layer_default_filter = manifest.filter;
    let function_slug = manifest.function_slug;
    let cache_ttl_secs = manifest.cache_ttl_secs;

    if !mode.eq_ignore_ascii_case("features") {
        return (
            StatusCode::NOT_IMPLEMENTED,
            Json(json!({"ok": false, "error": "only features mode is supported in v1"})),
        )
            .into_response();
    }

    // ── Point query detection (GetFeatureInfo) ────────────────────────
    // ?lat=...&lon=... constructs a bbox around the point and returns features
    let point_query_lat = params
        .get("lat")
        .or_else(|| params.get("LAT"))
        .or_else(|| params.get("y"))
        .or_else(|| params.get("Y"))
        .and_then(|s| s.parse::<f64>().ok());
    let point_query_lon = params
        .get("lon")
        .or_else(|| params.get("LON"))
        .or_else(|| params.get("x"))
        .or_else(|| params.get("X"))
        .and_then(|s| s.parse::<f64>().ok());

    // ── Z/X/Y tile: compute bbox and override format flags ───────────
    let (is_tile_request, is_mvt_request, bbox_override, zoom_override) =
        if let Some(ref zm) = zxy_match {
            let tile_bbox = crate::mapserver::resolve::tile::tile_to_bbox(zm.z, zm.x, zm.y);
            let is_png = zm.ext == "png";
            let is_mvt = zm.ext == "mvt" || zm.ext == "pbf";
            (is_png, is_mvt, Some(tile_bbox), Some(zm.z))
        } else {
            (false, false, None, None)
        };

    // WMS clients send uppercase params (FORMAT, REQUEST, BBOX, WIDTH, HEIGHT).
    // Support both cases for compatibility.
    let format_val = params.get("format").or_else(|| params.get("FORMAT"));

    // Detect tile request: format=image/png or REQUEST=GetMap (or z/x/y override)
    let is_tile_request = is_tile_request
        || format_val
            .map(|f| f.eq_ignore_ascii_case("image/png") || f.eq_ignore_ascii_case("png"))
            .unwrap_or(false)
        || params
            .get("REQUEST")
            .or(params.get("request"))
            .map(|r| r.eq_ignore_ascii_case("GetMap"))
            .unwrap_or(false);

    // Detect MVT vector tile request: format=mvt or format=pbf (or z/x/y override)
    let is_mvt_request = is_mvt_request
        || format_val
            .map(|f| {
                f.eq_ignore_ascii_case("mvt")
                    || f.eq_ignore_ascii_case("pbf")
                    || f.eq_ignore_ascii_case("application/vnd.mapbox-vector-tile")
            })
            .unwrap_or(false);

    // Point query: build bbox from lat/lon + tolerance
    let point_query_bbox = if let (Some(lat), Some(lon)) = (point_query_lat, point_query_lon) {
        let tolerance: f64 = params
            .get("tolerance")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.001); // ~111m at equator
        Some([
            lon - tolerance,
            lat - tolerance,
            lon + tolerance,
            lat + tolerance,
        ])
    } else {
        None
    };

    // bbox priority: z/x/y tile > point query > URL ?bbox= param
    let bbox = if let Some(tb) = bbox_override {
        Some(tb)
    } else if let Some(pq) = point_query_bbox {
        Some(pq)
    } else {
        match crate::mapserver::infra::http::parse_bbox_param(&params) {
            Ok(v) => v,
            Err(err) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"ok": false, "error": err})),
                )
                    .into_response();
            }
        }
    };
    let limit = match crate::mapserver::infra::http::parse_limit_param(&params, max_features) {
        Ok(v) => v.min(max_features), // Cap at max_features for GeoJSON feature download
        Err(err) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok": false, "error": err})),
            )
                .into_response();
        }
    };
    // For tile rendering (PNG/MVT), bbox already constrains the spatial extent.
    // max_features is designed for GeoJSON feature download (unbounded response size).
    // Tile rendering should not be capped by max_features — use a generous per-tile
    // safety limit instead (same as how GeoServer/MapServer WMS ignores maxFeatures).
    let tile_limit: usize = if is_tile_request || is_mvt_request {
        params
            .get("limit")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(500_000)
            .min(500_000)
    } else {
        limit
    };
    // zoom priority: z/x/y tile > URL ?zoom= param
    let zoom = zoom_override.or_else(|| {
        params
            .get("zoom")
            .or_else(|| params.get("ZOOM"))
            .and_then(|s| s.parse::<u8>().ok())
    });
    let style_param = params.get("style").or_else(|| params.get("STYLE")).cloned();

    let manifest = crate::mapserver::publish::registry::manifest_from_runtime(
        layer_id,
        layer_path,
        source_kind,
        source_path,
        mode,
        min_zoom,
        max_zoom,
        bbox_required,
        max_features,
        allowed_properties,
        layer_style_json,
        layer_default_filter,
        function_slug,
        cache_ttl_secs,
    );

    let filter_param = params.get("filter").cloned();
    // Merge: URL filter param > manifest default filter
    let effective_filter: Option<String> = filter_param.or_else(|| manifest.filter.clone());
    // Validate filter syntax early (return 400 on parse error)
    if let Some(ref f) = effective_filter {
        if let Err(e) = crate::mapserver::resolve::filter_dsl::parse_filter(f) {
            return (
                axum::http::StatusCode::BAD_REQUEST,
                format!("invalid filter: {e}"),
            )
                .into_response();
        }
    }
    let request = crate::mapserver::resolve::ResolveRequest {
        layer_id: manifest.layer_id.clone(),
        bbox,
        zoom,
        limit: Some(limit),
        filter: effective_filter.clone(),
    };
    // Tile request uses tile_limit (not capped by max_features)
    let tile_request = crate::mapserver::resolve::ResolveRequest {
        layer_id: manifest.layer_id.clone(),
        bbox,
        zoom,
        limit: Some(tile_limit),
        filter: effective_filter.clone(),
    };

    // ── GeoJsonFunction: preload features from function pipeline ──────
    let render_format = if is_mvt_request {
        "mvt"
    } else if is_tile_request {
        "png"
    } else {
        "geojson"
    };
    let preloaded_features: Option<serde_json::Value> = if manifest.source_kind
        == crate::mapserver::publish::manifest::SourceKind::GeoJsonFunction
    {
        let slug = manifest.function_slug.as_deref().unwrap_or("");
        if slug.is_empty() {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"ok": false, "error": "geojson_function layer has no function_slug"})),
            )
                .into_response();
        }
        let cache_key =
            crate::mapserver::resolve::function_cache::cache_key(&owner, &project, slug);
        if let Some(cached) = crate::mapserver::resolve::function_cache::get_cached(&cache_key) {
            Some(cached)
        } else {
            let fn_input = serde_json::json!({
                "layer": {
                    "id": manifest.layer_id,
                    "path": manifest.path,
                },
                "request": {
                    "bbox": bbox,
                    "zoom": zoom,
                    "format": render_format,
                    "filter": effective_filter,
                    "limit": limit,
                },
                "owner": owner,
                "project": project,
            });
            match state
                .platform
                .execute_function_pipeline(&owner, &project, slug, fn_input)
                .await
            {
                Ok(result) => {
                    let ttl = manifest.cache_ttl_secs.unwrap_or(60);
                    crate::mapserver::resolve::function_cache::put_cached(&cache_key, &result, ttl);
                    Some(result)
                }
                Err(e) => {
                    return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({"ok": false, "error": format!("function pipeline error: {}", e.message)})),
                        )
                            .into_response();
                }
            }
        }
    } else {
        None
    };

    // ── MVT vector tile branch ─────────────────────────────────────────
    if is_mvt_request {
        let tile_bbox = match bbox {
            Some(b) => b,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"ok": false, "error": "bbox is required for MVT requests"})),
                )
                    .into_response();
            }
        };
        let mvt_layer_name = manifest.layer_id.clone();

        // MVT cache key (format="mvt", width/height fixed at 4096 extent)
        let mvt_key = crate::mapserver::resolve::tile_cache::tile_cache_key(
            &manifest.layer_id,
            &tile_bbox,
            4096,
            4096,
            zoom,
            &manifest.source_ref,
            None,
            effective_filter.as_deref(),
            "mvt",
            &manifest.allowed_properties,
        );
        if let Some(cached_mvt) = crate::mapserver::resolve::tile_cache::get_tile(&mvt_key) {
            let mut resp = (StatusCode::OK, cached_mvt).into_response();
            resp.headers_mut().insert(
                CONTENT_TYPE,
                HeaderValue::from_static("application/vnd.mapbox-vector-tile"),
            );
            resp.headers_mut().insert(
                HeaderName::from_static("content-encoding"),
                HeaderValue::from_static("gzip"),
            );
            resp.headers_mut().insert(
                HeaderName::from_static("access-control-allow-origin"),
                HeaderValue::from_static("*"),
            );
            return resp;
        }

        // ── Direct GeoParquet → MVT fast path ─────────────────────────────
        if manifest.source_kind == crate::mapserver::publish::manifest::SourceKind::GeoParquet {
            let source_path = std::path::Path::new(&manifest.source_ref);
            if source_path.exists() {
                let parsed_filter = effective_filter
                    .as_deref()
                    .and_then(|f| crate::mapserver::resolve::filter_dsl::parse_filter(f).ok());
                if let Ok(mvt_bytes) =
                    crate::mapserver::resolve::geoparquet_direct::render_mvt_direct(
                        source_path,
                        tile_bbox,
                        &mvt_layer_name,
                        zoom,
                        parsed_filter.as_ref(),
                        &manifest.allowed_properties,
                        tile_limit,
                    )
                {
                    crate::mapserver::resolve::tile_cache::put_tile(
                        mvt_key,
                        mvt_bytes.clone(),
                        &manifest.source_ref,
                    );
                    let mut resp = (StatusCode::OK, mvt_bytes).into_response();
                    resp.headers_mut().insert(
                        CONTENT_TYPE,
                        HeaderValue::from_static("application/vnd.mapbox-vector-tile"),
                    );
                    resp.headers_mut().insert(
                        HeaderName::from_static("content-encoding"),
                        HeaderValue::from_static("gzip"),
                    );
                    resp.headers_mut().insert(
                        HeaderName::from_static("access-control-allow-origin"),
                        HeaderValue::from_static("*"),
                    );
                    return resp;
                }
                // Direct path failed — fall through to GeoJSON → MVT
            }
        }

        // ── GeoJSON → MVT fallback ────────────────────────────────────────
        let resolved = match crate::mapserver::resolve::resolve_features(
            &manifest,
            &tile_request,
            preloaded_features.as_ref(),
        ) {
            Ok(v) => v,
            Err(err) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"ok": false, "error": err})),
                )
                    .into_response();
            }
        };

        let mvt_bytes = crate::mapserver::resolve::mvt::features_to_mvt_gz(
            &mvt_layer_name,
            &resolved.features,
            &tile_bbox,
            &manifest.allowed_properties,
        );

        crate::mapserver::resolve::tile_cache::put_tile(
            mvt_key,
            mvt_bytes.clone(),
            &manifest.source_ref,
        );

        let mut resp = (StatusCode::OK, mvt_bytes).into_response();
        resp.headers_mut().insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/vnd.mapbox-vector-tile"),
        );
        resp.headers_mut().insert(
            HeaderName::from_static("content-encoding"),
            HeaderValue::from_static("gzip"),
        );
        resp.headers_mut().insert(
            HeaderName::from_static("access-control-allow-origin"),
            HeaderValue::from_static("*"),
        );
        return resp;
    }

    // ── Tile rendering branch ───────────────────────────────────────────
    if is_tile_request {
        let tile_bbox = match bbox {
            Some(b) => b,
            None => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"ok": false, "error": "bbox is required for tile requests"})),
                )
                    .into_response();
            }
        };
        let tile_width = params
            .get("width")
            .or_else(|| params.get("WIDTH"))
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(256)
            .min(1024)
            .max(1);
        let tile_height = params
            .get("height")
            .or_else(|| params.get("HEIGHT"))
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(256)
            .min(1024)
            .max(1);

        // ── Style resolution ─────────────────────────────────────────────
        // Priority: ?style= URL param > manifest DSL string > manifest JSON > defaults
        let style_dsl_str: Option<String> = style_param.clone().or_else(|| {
            manifest
                .style
                .as_ref()
                .and_then(|v| v.as_str().map(|s| s.to_string()))
        });

        // Base layer style (uniform fallback)
        let style_config: Option<crate::mapserver::resolve::style::LayerStyleConfig> =
            if style_dsl_str.is_none() {
                manifest
                    .style
                    .as_ref()
                    .and_then(|v| serde_json::from_value(v.clone()).ok())
            } else {
                None
            };
        let style =
            crate::mapserver::resolve::style::resolve_layer_style(style_config.as_ref(), zoom);

        // Resolve DSL → ResolvedStyle for per-feature data-driven styling
        let resolved_style: Option<crate::mapserver::resolve::style_dsl::ResolvedStyle> =
            style_dsl_str.as_deref().and_then(|dsl| {
                let def = crate::mapserver::resolve::style_dsl::parse_style_dsl(dsl).ok()?;
                let source_path = std::path::Path::new(&manifest.source_ref);
                let stats = if def.needs_stats() {
                    def.field_name().and_then(|fname| {
                        crate::mapserver::resolve::geoparquet_direct::get_field_stats(
                            source_path,
                            fname,
                        )
                    })
                } else {
                    None
                };
                let distinct = if def.needs_distinct() {
                    def.field_name().and_then(|fname| {
                        crate::mapserver::resolve::geoparquet_direct::get_field_distinct(
                            source_path,
                            fname,
                            50,
                        )
                    })
                } else {
                    None
                };
                crate::mapserver::resolve::style_dsl::resolve_style(
                    &def,
                    stats.as_ref(),
                    distinct.as_ref(),
                    zoom,
                )
                .ok()
            });

        // A raster tile is pixels: no column of the source leaves the server on
        // this branch, so `allowed_properties` has nothing to prune here. Data
        // driven styling still has to read the column it colours by, and making
        // that column public first would force the operator to leak it to get a
        // choropleth. The render path therefore resolves with exactly that one
        // field added and nothing else.
        let render_manifest = {
            let mut render_manifest = manifest.clone();
            if let Some(field) = resolved_style
                .as_ref()
                .and_then(crate::mapserver::resolve::style_dsl::ResolvedStyle::field_name)
                && !crate::mapserver::resolve::property_is_public(
                    &render_manifest.allowed_properties,
                    field,
                )
            {
                render_manifest.allowed_properties.push(field.to_string());
            }
            render_manifest
        };

        // If resolved style is Uniform, override the base LayerStyle
        let style =
            if let Some(crate::mapserver::resolve::style_dsl::ResolvedStyle::Uniform(ref fs)) =
                resolved_style
            {
                crate::mapserver::resolve::style::LayerStyle {
                    fill_color: fs.fill_color,
                    stroke_color: fs.stroke_color,
                    stroke_width: fs.stroke_width,
                    point_radius: fs.point_radius,
                    point_color: fs.point_color,
                }
            } else {
                style
            };

        // Check tile cache (style hash included in key)
        let tile_key = crate::mapserver::resolve::tile_cache::tile_cache_key(
            &manifest.layer_id,
            &tile_bbox,
            tile_width,
            tile_height,
            zoom,
            &manifest.source_ref,
            style_dsl_str.as_deref(),
            effective_filter.as_deref(),
            "png",
            &manifest.allowed_properties,
        );
        if let Some(cached_png) = crate::mapserver::resolve::tile_cache::get_tile(&tile_key) {
            let mut resp = (StatusCode::OK, cached_png).into_response();
            resp.headers_mut()
                .insert(CONTENT_TYPE, HeaderValue::from_static("image/png"));
            resp.headers_mut().insert(
                HeaderName::from_static("access-control-allow-origin"),
                HeaderValue::from_static("*"),
            );
            return resp;
        }

        // ── Direct parquet fast path (bypasses DataFusion entirely) ─────
        if manifest.source_kind == crate::mapserver::publish::manifest::SourceKind::GeoParquet {
            let source_path = std::path::Path::new(&manifest.source_ref);
            if source_path.exists() {
                let parsed_filter = effective_filter
                    .as_deref()
                    .and_then(|f| crate::mapserver::resolve::filter_dsl::parse_filter(f).ok());
                if let Ok(png_bytes) =
                    crate::mapserver::resolve::geoparquet_direct::render_tile_direct(
                        source_path,
                        tile_bbox,
                        tile_width,
                        tile_height,
                        zoom,
                        &style,
                        resolved_style.as_ref(),
                        parsed_filter.as_ref(),
                        tile_limit,
                    )
                {
                    crate::mapserver::resolve::tile_cache::put_tile(
                        tile_key,
                        png_bytes.clone(),
                        &manifest.source_ref,
                    );
                    let mut resp = (StatusCode::OK, png_bytes).into_response();
                    resp.headers_mut()
                        .insert(CONTENT_TYPE, HeaderValue::from_static("image/png"));
                    resp.headers_mut().insert(
                        HeaderName::from_static("access-control-allow-origin"),
                        HeaderValue::from_static("*"),
                    );
                    return resp;
                }
                // Direct path failed — fall through to DataFusion path
            }
        }

        // ── Metatile pixmap cache: zoom >= 8 and standard 256×256 tiles ──
        // Non-blocking: if a metatile pixmap is cached, slice from it (very fast).
        // Otherwise, render the individual tile normally and spawn a background
        // task to render+cache the metatile pixmap for future requests.
        let use_metatile =
            zoom.map(|z| z >= 8).unwrap_or(false) && tile_width == 256 && tile_height == 256;

        if use_metatile {
            const META_GRID: u32 = 4;
            let meta_info =
                crate::mapserver::resolve::tile::compute_metatile_info(&tile_bbox, META_GRID);
            let meta_key = crate::mapserver::resolve::tile_cache::metatile_cache_key(
                &manifest.layer_id,
                &meta_info.bbox,
                zoom,
                &manifest.source_ref,
                style_dsl_str.as_deref(),
                effective_filter.as_deref(),
            );

            // Fast path: metatile pixmap already cached — slice our tile from it
            if let Some(png_bytes) = crate::mapserver::resolve::tile_cache::get_metatile_tile(
                &meta_key,
                &tile_bbox,
                tile_width,
                tile_height,
            ) {
                crate::mapserver::resolve::tile_cache::put_tile(
                    tile_key,
                    png_bytes.clone(),
                    &manifest.source_ref,
                );
                let mut resp = (StatusCode::OK, png_bytes).into_response();
                resp.headers_mut()
                    .insert(CONTENT_TYPE, HeaderValue::from_static("image/png"));
                resp.headers_mut().insert(
                    HeaderName::from_static("access-control-allow-origin"),
                    HeaderValue::from_static("*"),
                );
                return resp;
            }

            // No cached pixmap — spawn background metatile render (non-blocking)
            if crate::mapserver::resolve::tile_cache::try_claim_metatile(&meta_key) {
                let bg_manifest = render_manifest.clone();
                let bg_style = style.clone();
                let bg_resolved_style = resolved_style.clone();
                let bg_meta_key = meta_key.clone();
                let bg_meta_bbox = meta_info.bbox;
                let bg_filter = effective_filter.clone();
                let bg_preloaded = preloaded_features.clone();
                tokio::task::spawn_blocking(move || {
                    // Delay so individual tile queries get priority on the DataFusion mutex
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    let meta_request = crate::mapserver::resolve::ResolveRequest {
                        layer_id: bg_manifest.layer_id.clone(),
                        bbox: Some(bg_meta_bbox),
                        zoom,
                        limit: Some(tile_limit),
                        filter: bg_filter,
                    };
                    let meta_resolved = match crate::mapserver::resolve::resolve_features(
                        &bg_manifest,
                        &meta_request,
                        bg_preloaded.as_ref(),
                    ) {
                        Ok(v) => v,
                        Err(_) => {
                            crate::mapserver::resolve::tile_cache::release_metatile(&bg_meta_key);
                            return;
                        }
                    };
                    let meta_tile_req = crate::mapserver::resolve::tile::TileRenderRequest {
                        bbox: bg_meta_bbox,
                        width: tile_width * META_GRID,
                        height: tile_height * META_GRID,
                        zoom,
                    };
                    if let Ok(pm) = crate::mapserver::resolve::tile::render_features_to_pixmap(
                        &meta_resolved.features,
                        &meta_tile_req,
                        &bg_style,
                        bg_resolved_style.as_ref(),
                    ) {
                        crate::mapserver::resolve::tile_cache::put_metatile_pixmap(
                            bg_meta_key.clone(),
                            pm.data().to_vec(),
                            pm.width(),
                            pm.height(),
                            bg_meta_bbox,
                        );
                    }
                    crate::mapserver::resolve::tile_cache::release_metatile(&bg_meta_key);
                });
            }
            // Fall through to individual tile rendering below
        }

        // ── Individual tile rendering ──────────────────────────────────────
        let resolved = match crate::mapserver::resolve::resolve_features(
            &render_manifest,
            &tile_request,
            preloaded_features.as_ref(),
        ) {
            Ok(v) => v,
            Err(err) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"ok": false, "error": err})),
                )
                    .into_response();
            }
        };

        let tile_request = crate::mapserver::resolve::tile::TileRenderRequest {
            bbox: tile_bbox,
            width: tile_width,
            height: tile_height,
            zoom,
        };
        let png_bytes = match crate::mapserver::resolve::tile::render_features_to_png(
            &resolved.features,
            &tile_request,
            &style,
            resolved_style.as_ref(),
        ) {
            Ok(bytes) => bytes,
            Err(err) => {
                return internal_error(PlatformError::new(
                    "MAPSERVER_TILE_RENDER",
                    format!("tile rendering failed: {err}"),
                ));
            }
        };

        crate::mapserver::resolve::tile_cache::put_tile(
            tile_key,
            png_bytes.clone(),
            &manifest.source_ref,
        );

        let mut resp = (StatusCode::OK, png_bytes).into_response();
        resp.headers_mut()
            .insert(CONTENT_TYPE, HeaderValue::from_static("image/png"));
        resp.headers_mut().insert(
            HeaderName::from_static("access-control-allow-origin"),
            HeaderValue::from_static("*"),
        );
        return resp;
    }

    // ── Point query (GetFeatureInfo) branch ──────────────────────────
    if let (Some(lat), Some(lon)) = (point_query_lat, point_query_lon) {
        let pq_limit = params
            .get("limit")
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(10)
            .min(100);
        let tolerance: f64 = params
            .get("tolerance")
            .and_then(|s| s.parse::<f64>().ok())
            .unwrap_or(0.001);
        let pq_request = crate::mapserver::resolve::ResolveRequest {
            layer_id: manifest.layer_id.clone(),
            bbox: Some([
                lon - tolerance,
                lat - tolerance,
                lon + tolerance,
                lat + tolerance,
            ]),
            zoom: None,
            limit: Some(pq_limit),
            filter: effective_filter.clone(),
        };
        let resolved = match crate::mapserver::resolve::resolve_features(
            &manifest,
            &pq_request,
            preloaded_features.as_ref(),
        ) {
            Ok(v) => v,
            Err(err) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"ok": false, "error": err})),
                )
                    .into_response();
            }
        };
        let payload = json!({
            "type": "FeatureCollection",
            "layer": resolved.layer,
            "count": resolved.count,
            "truncated": resolved.truncated,
            "query": {
                "lat": lat,
                "lon": lon,
                "tolerance": tolerance,
            },
            "features": resolved.features
        });
        let body = match serde_json::to_vec(&payload) {
            Ok(body) => body,
            Err(err) => {
                return internal_error(PlatformError::new(
                    "MAPSERVER_RESPONSE_SERIALIZE",
                    format!("failed serializing point query payload: {err}"),
                ));
            }
        };
        let mut resp = (StatusCode::OK, body).into_response();
        resp.headers_mut().insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/geo+json; charset=utf-8"),
        );
        resp.headers_mut().insert(
            HeaderName::from_static("access-control-allow-origin"),
            HeaderValue::from_static("*"),
        );
        return resp;
    }

    // ── GeoJSON features branch ─────────────────────────────────────────
    let response_cache_key =
        crate::mapserver::resolve::cache::response_cache_key(&manifest, &request);
    if let Some(body) = crate::mapserver::resolve::cache::get_response_bytes(&response_cache_key) {
        let mut resp = (StatusCode::OK, body).into_response();
        resp.headers_mut().insert(
            CONTENT_TYPE,
            HeaderValue::from_static("application/geo+json; charset=utf-8"),
        );
        resp.headers_mut().insert(
            HeaderName::from_static("access-control-allow-origin"),
            HeaderValue::from_static("*"),
        );
        return resp;
    }
    let resolved = match crate::mapserver::resolve::resolve_features(
        &manifest,
        &request,
        preloaded_features.as_ref(),
    ) {
        Ok(v) => v,
        Err(err) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok": false, "error": err})),
            )
                .into_response();
        }
    };
    let payload = json!({
        "type": "FeatureCollection",
        "layer": resolved.layer,
        "count": resolved.count,
        "truncated": resolved.truncated,
        "features": resolved.features
    });
    let body = match serde_json::to_vec(&payload) {
        Ok(body) => body,
        Err(err) => {
            return internal_error(PlatformError::new(
                "MAPSERVER_RESPONSE_SERIALIZE",
                format!("failed serializing mapserver payload: {err}"),
            ));
        }
    };
    crate::mapserver::resolve::cache::put_response_bytes(
        response_cache_key,
        body.clone(),
        &manifest.source_ref,
    );
    let mut resp = (StatusCode::OK, body).into_response();
    resp.headers_mut().insert(
        CONTENT_TYPE,
        HeaderValue::from_static("application/geo+json; charset=utf-8"),
    );
    resp.headers_mut().insert(
        HeaderName::from_static("access-control-allow-origin"),
        HeaderValue::from_static("*"),
    );
    resp
}

async fn public_mapserver_ingress_root(
    State(state): State<PlatformAppState>,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    public_mapserver_ingress(State(state), Path((owner, project, String::new())), uri).await
}

use crate::contracts::kinds::MapserverLayerRecord;

fn mapserver_layers_manifest_path(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
    instance: &str,
) -> Result<std::path::PathBuf, PlatformError> {
    let layout = state.platform.file.ensure_project_layout(owner, project)?;
    Ok(crate::mapserver::publish::registry::layers_manifest_path(
        &layout.files_dir,
        instance,
    ))
}

/// Resolves the per-instance artifact root at its `cache`-tier home,
/// migrating a pre-tier `files/mapserver/.artifacts` tree the first time it
/// is touched (`instance-directory.md` rule 8). Both-paths-present refuses.
fn mapserver_artifacts_root(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
    instance: &str,
) -> Result<std::path::PathBuf, PlatformError> {
    let layout = state.platform.file.ensure_project_layout(owner, project)?;
    let root = layout
        .ensure_mapserver_artifacts_home()
        .map_err(|err| PlatformError::new("MAPSERVER_ARTIFACT_TIER_MIGRATE", err.to_string()))?;
    Ok(root.join(instance))
}

fn read_mapserver_layers(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
    instance: &str,
) -> Result<Vec<MapserverLayerRecord>, PlatformError> {
    let path = mapserver_layers_manifest_path(state, owner, project, instance)?;
    crate::mapserver::publish::registry::read_layers(&path).map_err(|err| {
        PlatformError::new("MAPSERVER_PARSE", format!("{} ({})", err, err.category()))
    })
}

fn write_mapserver_layers(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
    instance: &str,
    items: &[MapserverLayerRecord],
) -> Result<(), PlatformError> {
    let path = mapserver_layers_manifest_path(state, owner, project, instance)?;
    crate::mapserver::publish::registry::write_layers(&path, instance, items).map_err(|err| {
        PlatformError::new("MAPSERVER_WRITE", format!("{} ({})", err, err.category()))
    })
}

fn list_mapserver_source_files(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
) -> Result<Vec<serde_json::Value>, PlatformError> {
    let layout = state.platform.file.ensure_project_layout(owner, project)?;
    let dir = layout.files_dir.join("mapserver");
    std::fs::create_dir_all(&dir)
        .map_err(|err| PlatformError::new("MAPSERVER_LIST", err.to_string()))?;
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        let mut entries: Vec<_> = entries.flatten().collect();
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.ends_with(".layers.json") {
                continue;
            }
            let lower = name.to_ascii_lowercase();
            if !(lower.ends_with(".geojson")
                || lower.ends_with(".json")
                || lower.ends_with(".parquet"))
            {
                continue;
            }
            let rel = format!("mapserver/{name}");
            let meta = path.metadata().ok();
            out.push(json!({
                "name": name,
                "path": rel,
                "size": meta.as_ref().map(|m| m.len()).unwrap_or(0),
                "url": format!("/fs/{owner}/{project}/{rel}")
            }));
        }
    }
    Ok(out)
}

fn resolve_mapserver_manifest_for_path(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
    path: &str,
) -> Result<Option<crate::mapserver::publish::manifest::PublishedLayerManifest>, PlatformError> {
    let layers = read_mapserver_layers(
        state,
        owner,
        project,
        crate::mapserver::publish::registry::DEFAULT_INSTANCE,
    )?;
    let normalized = crate::mapserver::publish::registry::normalize_layer_path(path);
    let layer = layers.into_iter().find(|item| {
        crate::mapserver::publish::registry::normalize_layer_path(&item.path) == normalized
    });
    let Some(layer) = layer else {
        return Ok(None);
    };
    mapserver_record_to_manifest(state, owner, project, layer).map(Some)
}

fn mapserver_record_to_manifest(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
    item: MapserverLayerRecord,
) -> Result<crate::mapserver::publish::manifest::PublishedLayerManifest, PlatformError> {
    let layout = state.platform.file.ensure_project_layout(owner, project)?;
    Ok({
        let (source_kind, source_ref) = if let Some(artifact_rel) =
            item.artifact_manifest_path.clone()
        {
            // Serving may be the first touch after the tier move: migrate
            // a pre-tier `.artifacts` tree before resolving into it. A
            // both-paths refusal surfaces later as a missing manifest
            // rather than silently serving the stale copy.
            if let Err(err) = layout.ensure_mapserver_artifacts_home() {
                eprintln!(
                    "WARN: mapserver artifact tier migration refused for {owner}/{project}: {err}"
                );
            }
            (
                crate::mapserver::publish::manifest::SourceKind::GeoJsonArtifact,
                layout
                    .resolve_mapserver_artifact_path(&artifact_rel)
                    .display()
                    .to_string(),
            )
        } else if item.source_kind == "geoparquet" {
            (
                crate::mapserver::publish::manifest::SourceKind::GeoParquet,
                layout
                    .files_dir
                    .join(item.source_path.trim_start_matches('/'))
                    .display()
                    .to_string(),
            )
        } else if item.source_kind == "geojson_function" {
            (
                crate::mapserver::publish::manifest::SourceKind::GeoJsonFunction,
                String::new(),
            )
        } else {
            (
                crate::mapserver::publish::manifest::SourceKind::GeoJsonFile,
                layout
                    .files_dir
                    .join(item.source_path.trim_start_matches('/'))
                    .display()
                    .to_string(),
            )
        };
        crate::mapserver::publish::registry::manifest_from_runtime(
            item.layer_id,
            item.path,
            source_kind,
            source_ref,
            item.mode,
            item.min_zoom,
            item.max_zoom,
            item.bbox_required,
            item.max_features,
            item.allowed_properties,
            item.style,
            item.filter,
            item.function_slug,
            item.cache_ttl_secs,
        )
    })
}

/// Extract a safe subset of request headers for the trigger snapshot.
/// Excludes Authorization and Cookie to avoid leaking credentials downstream.
/// The roles a token carries: an array of strings, or a string holding one
/// role or a comma-separated list, with stray quotes and brackets removed.
fn token_roles(claim: Option<&Value>) -> Vec<String> {
    let clean = |s: &str| s.trim_matches(|c: char| c.is_whitespace() || matches!(c, '[' | ']' | '\'' | '"')).to_string();
    match claim {
        Some(Value::Array(items)) => items
            .iter()
            .filter_map(|v| v.as_str())
            .map(clean)
            .filter(|s| !s.is_empty())
            .collect(),
        Some(Value::String(s)) => s
            .split(',')
            .map(clean)
            .filter(|s| !s.is_empty())
            .collect(),
        _ => Vec::new(),
    }
}

fn safe_headers(headers: &HeaderMap) -> Value {
    const SAFE: &[&str] = &[
        // Where the request arrived (`addressing.md`): a sitemap or an e-mail
        // link needs an absolute URL, and the project must not store one.
        "host",
        "x-forwarded-host",
        "x-forwarded-proto",
        "content-type",
        "accept",
        "user-agent",
        "x-forwarded-for",
        "x-real-ip",
        "referer",
        "origin",
        "sec-fetch-mode",
        "sec-fetch-dest",
    ];
    let map: serde_json::Map<String, Value> = SAFE
        .iter()
        .filter_map(|k| {
            headers
                .get(*k)
                .and_then(|v| v.to_str().ok())
                .map(|v| (k.to_string(), Value::String(v.to_string())))
        })
        .collect();
    Value::Object(map)
}

/// Build the structured webhook payload.
///
/// User-submitted data lives under `input.body` — never at root.
/// Server request context (`query`, `params`, `path`, `method`) lives at root.
/// This prevents collisions when user body contains fields like "query" or "path".
fn build_structured_payload(
    body: Value,
    query: &Value,
    params: &Value,
    path: &str,
    method: &str,
    files: Option<Value>,
) -> Value {
    let mut obj = serde_json::Map::new();
    obj.insert("body".to_string(), body);
    obj.insert("query".to_string(), query.clone());
    obj.insert("params".to_string(), params.clone());
    obj.insert("path".to_string(), Value::String(path.to_string()));
    obj.insert("method".to_string(), Value::String(method.to_string()));
    if let Some(f) = files {
        obj.insert("files".to_string(), f);
    }
    Value::Object(obj)
}

async fn build_webhook_ingress_input(
    platform: &Arc<PlatformService>,
    owner: &str,
    project: &str,
    request_id: &str,
    method: &Method,
    uri: &Uri,
    headers: &HeaderMap,
    body: &Bytes,
    path: &str,
    path_params: &serde_json::Map<String, Value>,
) -> Result<Value, crate::pipeline::PipelineError> {
    let query = parse_query_to_json(uri.query());
    let params = Value::Object(path_params.clone());
    let method_str = method.as_str();

    // GET with no body: body is null, query params at root
    if method == Method::GET && body.is_empty() {
        return Ok(build_structured_payload(
            Value::Null,
            &query,
            &params,
            path,
            method_str,
            None,
        ));
    }

    // Non-GET with empty body
    if body.is_empty() {
        return Ok(build_structured_payload(
            Value::Null,
            &query,
            &params,
            path,
            method_str,
            None,
        ));
    }

    let content_type = headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let content_type_lc = content_type.to_ascii_lowercase();

    // JSON body → user data under .body
    if content_type_lc.contains("application/json")
        && let Ok(parsed) = serde_json::from_slice::<Value>(body)
    {
        return Ok(build_structured_payload(
            parsed, &query, &params, path, method_str, None,
        ));
    }

    // Form-urlencoded → form fields under .body
    if content_type_lc.contains("application/x-www-form-urlencoded") {
        if let Ok(fields) = serde_urlencoded::from_bytes::<Vec<(String, String)>>(body) {
            let mut form_obj = serde_json::Map::new();
            for (k, v) in fields {
                form_obj.insert(k, Value::String(v));
            }
            return Ok(build_structured_payload(
                Value::Object(form_obj),
                &query,
                &params,
                path,
                method_str,
                None,
            ));
        }
    }

    // Multipart → text fields under .body, files at root .files.
    // Repeated names and common frontend array names (`field[]`, `field[0]`)
    // become JSON arrays so dot paths like `files.photos.0` are stable.
    if content_type_lc.contains("multipart/form-data") {
        if let Ok(boundary) = multer::parse_boundary(&content_type) {
            let body_clone = body.clone();
            let stream = futures::stream::once(async move { Ok::<_, std::io::Error>(body_clone) });
            let mut mp = multer::Multipart::new(stream, boundary);
            let mut text_fields = serde_json::Map::new();
            let mut files = serde_json::Map::new();
            while let Ok(Some(mut field)) = mp.next_field().await {
                let name = field.name().unwrap_or("").to_string();
                let filename = field.file_name().map(|s| s.to_string());
                let field_ct = field.content_type().map(|ct| ct.to_string());
                let mut data: Vec<u8> = Vec::new();
                while let Ok(Some(chunk)) = field.chunk().await {
                    data.extend_from_slice(&chunk);
                }
                if let Some(filename) = filename {
                    let mime = field_ct
                        .as_deref()
                        .filter(|value| !value.trim().is_empty())
                        .unwrap_or("application/octet-stream");
                    let file_ref = crate::pipeline::nodes::basic::file_ref::write_tmp_file_ref(
                        platform,
                        crate::pipeline::nodes::basic::file_ref::FileRefInput {
                            owner,
                            project,
                            request_id,
                            bytes: &data,
                            filename: Some(&filename),
                            mime: Some(mime),
                            origin: "webhook",
                            trust: "untrusted",
                        },
                    )?;
                    insert_multipart_value(&mut files, &name, file_ref);
                } else {
                    let text = String::from_utf8_lossy(&data).to_string();
                    insert_multipart_value(&mut text_fields, &name, Value::String(text));
                }
            }
            let files_val = if files.is_empty() {
                None
            } else {
                Some(Value::Object(files))
            };
            return Ok(build_structured_payload(
                Value::Object(text_fields),
                &query,
                &params,
                path,
                method_str,
                files_val,
            ));
        }
    }

    // Raw text / unknown content type
    let body_text = String::from_utf8_lossy(body).to_string();
    Ok(build_structured_payload(
        Value::String(body_text),
        &query,
        &params,
        path,
        method_str,
        None,
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct MultipartValueKey {
    base: String,
    index: Option<usize>,
    force_array: bool,
}

fn parse_multipart_value_key(raw: &str) -> MultipartValueKey {
    let name = raw.trim();
    if let Some(base) = name.strip_suffix("[]").filter(|base| !base.is_empty()) {
        return MultipartValueKey {
            base: base.to_string(),
            index: None,
            force_array: true,
        };
    }
    if let Some(open) = name.rfind('[')
        && name.ends_with(']')
        && open > 0
    {
        let inner = &name[open + 1..name.len() - 1];
        if !inner.is_empty()
            && inner.chars().all(|ch| ch.is_ascii_digit())
            && let Ok(index) = inner.parse::<usize>()
        {
            return MultipartValueKey {
                base: name[..open].to_string(),
                index: Some(index),
                force_array: true,
            };
        }
    }
    MultipartValueKey {
        base: name.to_string(),
        index: None,
        force_array: false,
    }
}

fn insert_multipart_value(map: &mut serde_json::Map<String, Value>, raw_name: &str, value: Value) {
    let key = parse_multipart_value_key(raw_name);
    if key.base.is_empty() {
        return;
    }

    if let Some(index) = key.index {
        let entry = map
            .entry(key.base)
            .or_insert_with(|| Value::Array(Vec::new()));
        if !entry.is_array() {
            let existing = std::mem::take(entry);
            *entry = Value::Array(vec![existing]);
        }
        if let Some(items) = entry.as_array_mut() {
            if items.len() <= index {
                items.resize(index + 1, Value::Null);
            }
            items[index] = value;
        }
        return;
    }

    if key.force_array {
        let entry = map
            .entry(key.base)
            .or_insert_with(|| Value::Array(Vec::new()));
        if !entry.is_array() {
            let existing = std::mem::take(entry);
            *entry = Value::Array(vec![existing]);
        }
        if let Some(items) = entry.as_array_mut() {
            items.push(value);
        }
        return;
    }

    match map.get_mut(&key.base) {
        Some(existing) if existing.is_array() => {
            if let Some(items) = existing.as_array_mut() {
                items.push(value);
            }
        }
        Some(existing) => {
            let first = std::mem::take(existing);
            *existing = Value::Array(vec![first, value]);
        }
        None => {
            map.insert(key.base, value);
        }
    }
}

#[cfg(test)]
mod webhook_ingress_tests {
    use serde_json::{Map, json};

    use super::{insert_multipart_value, parse_multipart_value_key};

    #[test]
    fn multipart_repeated_plain_fields_become_arrays() {
        let mut values = Map::new();
        insert_multipart_value(&mut values, "photos", json!("a"));
        insert_multipart_value(&mut values, "photos", json!("b"));

        assert_eq!(values.get("photos"), Some(&json!(["a", "b"])));
    }

    #[test]
    fn multipart_bracket_fields_become_arrays() {
        let mut values = Map::new();
        insert_multipart_value(&mut values, "photos[]", json!("a"));
        insert_multipart_value(&mut values, "photos[]", json!("b"));

        assert_eq!(values.get("photos"), Some(&json!(["a", "b"])));
    }

    #[test]
    fn multipart_indexed_fields_keep_index_order() {
        let mut values = Map::new();
        insert_multipart_value(&mut values, "photos[1]", json!("b"));
        insert_multipart_value(&mut values, "photos[0]", json!("a"));

        assert_eq!(values.get("photos"), Some(&json!(["a", "b"])));
    }

    #[test]
    fn multipart_non_numeric_brackets_remain_literal() {
        let key = parse_multipart_value_key("meta[name]");

        assert_eq!(key.base, "meta[name]");
        assert_eq!(key.index, None);
        assert!(!key.force_array);
    }
}

fn parse_query_to_json(raw_query: Option<&str>) -> Value {
    let mut map = serde_json::Map::new();
    let Some(raw_query) = raw_query else {
        return Value::Object(map);
    };
    for pair in raw_query.split('&') {
        if pair.is_empty() {
            continue;
        }
        let mut split = pair.splitn(2, '=');
        let key = split.next().unwrap_or_default().trim();
        if key.is_empty() {
            continue;
        }
        let value = split.next().unwrap_or_default().trim();
        map.insert(key.to_string(), Value::String(value.to_string()));
    }
    Value::Object(map)
}

fn quote_sql_identifier_path(raw: &str, dialect: SqlDialect) -> String {
    raw.split('.')
        .filter(|part| !part.trim().is_empty())
        .map(|part| dialect.quote(part.trim()))
        .collect::<Vec<_>>()
        .join(".")
}

/// The statement the studio runs to show the first rows of a table.
///
/// The dialect comes from the driver rather than from a match on the engine's
/// name, because the quoting character differs — MySQL rejects the double
/// quotes PostgreSQL requires. An engine with no SQL dialect is addressed by
/// its bare table name.
fn build_table_preview_sql(
    dialect: Option<SqlDialect>,
    raw_table: &str,
    bare_table: &str,
    limit: usize,
) -> String {
    match dialect {
        Some(dialect) => format!(
            "SELECT * FROM {} LIMIT {}",
            quote_sql_identifier_path(raw_table, dialect),
            limit
        ),
        None => format!("SELECT * FROM {} LIMIT {}", bare_table.trim(), limit),
    }
}

#[cfg(test)]
mod preview_sql_tests {
    use std::path::Path;

    use super::{
        build_table_preview_sql, content_type_for_path, file_content_disposition,
        query_flag_enabled,
    };
    use crate::platform::db::sql_ddl::SqlDialect;

    #[test]
    fn builds_unquoted_preview_sql_for_an_engine_with_no_dialect() {
        assert_eq!(
            build_table_preview_sql(None, "default.posts", "posts", 120),
            "SELECT * FROM posts LIMIT 120"
        );
    }

    #[test]
    fn each_sql_dialect_quotes_the_way_its_engine_requires() {
        assert_eq!(
            build_table_preview_sql(Some(SqlDialect::Postgres), "public.posts", "posts", 50),
            "SELECT * FROM \"public\".\"posts\" LIMIT 50"
        );
        // MySQL rejects double quotes around identifiers by default.
        assert_eq!(
            build_table_preview_sql(Some(SqlDialect::MySql), "zebflow.orders", "orders", 10),
            "SELECT * FROM `zebflow`.`orders` LIMIT 10"
        );
    }

    #[test]
    fn serves_geojson_with_inline_preview_headers_by_default() {
        let path = Path::new("public/v2x-map/roads.geojson");
        let content_type = content_type_for_path(path);
        assert_eq!(content_type, "application/geo+json; charset=utf-8");
        let disposition =
            file_content_disposition(path, content_type, false).expect("inline disposition");
        assert_eq!(
            disposition.to_str().unwrap(),
            "inline; filename=\"roads.geojson\""
        );
    }

    #[test]
    fn supports_explicit_download_flag_for_known_file_types() {
        let path = Path::new("public/v2x-map/roads.geojson");
        let content_type = content_type_for_path(path);
        let disposition =
            file_content_disposition(path, content_type, true).expect("attachment disposition");
        assert_eq!(
            disposition.to_str().unwrap(),
            "attachment; filename=\"roads.geojson\""
        );
        assert!(query_flag_enabled(Some("1")));
        assert!(query_flag_enabled(Some("true")));
        assert!(query_flag_enabled(Some("yes")));
        assert!(!query_flag_enabled(Some("0")));
        assert!(!query_flag_enabled(None));
    }
}

#[derive(Debug)]
struct WebhookPathMatch {
    params: serde_json::Map<String, Value>,
    static_segments: usize,
    dynamic_segments: usize,
    total_segments: usize,
}

fn match_webhook_path(pattern: &str, actual: &str) -> Option<WebhookPathMatch> {
    let normalized_pattern = normalize_webhook_path(pattern);
    let normalized_actual = normalize_webhook_path(actual);

    let pattern_segments = split_webhook_segments(&normalized_pattern);
    let actual_segments = split_webhook_segments(&normalized_actual);
    if pattern_segments.len() != actual_segments.len() {
        return None;
    }

    let mut params = serde_json::Map::new();
    let mut static_segments = 0usize;
    let mut dynamic_segments = 0usize;

    for (pattern_seg, actual_seg) in pattern_segments.iter().zip(actual_segments.iter()) {
        if let Some(name) = path_param_name(pattern_seg) {
            dynamic_segments += 1;
            params.insert(name.to_string(), Value::String((*actual_seg).to_string()));
            continue;
        }
        if pattern_seg == actual_seg {
            static_segments += 1;
            continue;
        }
        return None;
    }

    Some(WebhookPathMatch {
        params,
        static_segments,
        dynamic_segments,
        total_segments: pattern_segments.len(),
    })
}

fn normalize_webhook_path(raw: &str) -> String {
    let raw = raw.trim();
    if raw.is_empty() || raw == "/" {
        return "/".to_string();
    }
    let mut out = String::from("/");
    out.push_str(raw.trim_matches('/'));
    out
}

fn split_webhook_segments(path: &str) -> Vec<&str> {
    if path == "/" {
        return Vec::new();
    }
    path.trim_start_matches('/')
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>()
}

fn path_param_name(segment: &str) -> Option<&str> {
    // Support {name} style
    if let Some(inner) = segment.strip_prefix('{').and_then(|s| s.strip_suffix('}')) {
        let name = inner.trim();
        if !name.is_empty() && !name.contains('/') && !name.contains('{') && !name.contains('}') {
            return Some(name);
        }
    }
    // Support :name style
    if let Some(name) = segment.strip_prefix(':') {
        let name = name.trim();
        if !name.is_empty() && !name.contains('/') && !name.contains(':') {
            return Some(name);
        }
    }
    None
}

fn pipeline_path_matches(base_path: &str, candidate_path: &str, recursive: bool) -> bool {
    let base = crate::platform::model::normalize_virtual_path(base_path);
    let candidate = crate::platform::model::normalize_virtual_path(candidate_path);
    if !recursive {
        return candidate == base;
    }
    if base == "/" {
        return true;
    }
    candidate == base || candidate.starts_with(&(base + "/"))
}

fn pipeline_source_is_locked(source: &str) -> bool {
    let Ok(value) = serde_json::from_str::<Value>(source) else {
        return false;
    };
    value
        .get("metadata")
        .and_then(|metadata| metadata.get("locked"))
        .and_then(Value::as_bool)
        .or_else(|| value.get("locked").and_then(Value::as_bool))
        .unwrap_or(false)
}

#[derive(Debug, Clone, Copy)]
struct EffectivePipelineInvocationRetention {
    max_invocations: usize,
    max_age_secs: Option<i64>,
}

/// Both registry/editor entry routes expose the same resolved logging defaults.
/// These are display hints; execution resolves overrides from its own snapshot.
fn pipeline_logging_defaults(config: &crate::platform::model::ZebflowJson) -> Value {
    let logging = &config.configs.pipelines.logging;
    json!({
        "max_invocations": logging.effective_max_invocations(),
        "trace_capture": logging.trace_capture.as_ref().cloned().unwrap_or_default().resolve(None),
    })
}

fn resolve_invocation_retention(
    project_cfg: &crate::platform::model::ZebflowJson,
    graph: Option<&crate::pipeline::PipelineGraph>,
) -> EffectivePipelineInvocationRetention {
    let project_max_invocations = project_cfg
        .configs
        .pipelines
        .logging
        .effective_max_invocations();
    let pipeline_retention = graph
        .and_then(|graph| graph.metadata.as_ref())
        .and_then(|metadata| metadata.settings.invocation_retention.as_ref());
    let max_invocations = pipeline_retention
        .and_then(|retention| retention.max_invocations)
        .map(|value| value.max(1) as usize)
        .unwrap_or(project_max_invocations);
    let max_age_secs = pipeline_retention
        .and_then(|retention| retention.max_age_secs)
        .map(|value| value.max(1) as i64);
    EffectivePipelineInvocationRetention {
        max_invocations,
        max_age_secs,
    }
}

fn resolve_pipeline_registry_scope(
    query: &PipelineRegistryQuery,
) -> Result<PipelineRegistryScope, Response> {
    match query
        .scope
        .as_deref()
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("path") => Ok(PipelineRegistryScope::Path),
        Some("project") => Ok(PipelineRegistryScope::Project),
        Some(_) => Err((
            StatusCode::BAD_REQUEST,
            Json(json!({
                "ok": false,
                "error": {
                    "code": "PLATFORM_PIPELINE_REGISTRY_SCOPE_INVALID",
                    "message": "query.scope must be 'path' or 'project'"
                }
            })),
        )
            .into_response()),
        None => {
            if query.path.is_some() {
                Ok(PipelineRegistryScope::Path)
            } else {
                Ok(PipelineRegistryScope::Project)
            }
        }
    }
}

/// Whether the forced password change intercepts this path.
///
/// Deliberately a list of the browser-facing platform UI pages rather than a
/// catch-all: API, webhook, WS, MCP, asset, file-serve, and public ingress
/// paths must keep answering to the generated credential, because the CLI's
/// zero-ceremony local install authenticates with it before any human has
/// logged in. The change screen itself and login/logout stay reachable or the
/// redirect would loop.
fn forced_change_applies_to(path: &str) -> bool {
    path == "/home"
        || path == "/profile"
        || path == "/hub"
        || path.starts_with("/home/")
        || path.starts_with("/projects/")
        || path.starts_with("/dev/")
        || path.starts_with("/docs/")
        || path.starts_with("/preview/")
}

/// Redirects browser page requests to the change-password screen while the
/// session owner's credential is still `generated`.
async fn credential_change_gate(
    State(state): State<PlatformAppState>,
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    if request.method() == Method::GET
        && forced_change_applies_to(request.uri().path())
        && !session_is_vouched(&state, request.headers())
        && let Some(owner) = session_owner(&state, request.headers())
        && state
            .platform
            .users
            .must_change_password(&owner)
            .unwrap_or(false)
    {
        return Redirect::to(ACCOUNT_PASSWORD_PATH).into_response();
    }
    next.run(request).await
}

/// Whether the presented session began with a vouch.
fn session_is_vouched(state: &PlatformAppState, headers: &HeaderMap) -> bool {
    let Some(token) = session_token(headers) else {
        return false;
    };
    state
        .sessions
        .lock()
        .ok()
        .and_then(|sessions| sessions.get(&token).map(|session| session.vouched))
        .unwrap_or(false)
}

fn random_session_token() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill(&mut bytes);
    hex::encode(bytes)
}

fn issue_session(state: &PlatformAppState, owner: &str) -> String {
    issue_session_inner(state, owner, false)
}

/// A session that began with a vouch (`offices.md` §2) rather than a password.
fn issue_vouched_session(state: &PlatformAppState, owner: &str) -> String {
    issue_session_inner(state, owner, true)
}

fn issue_session_inner(state: &PlatformAppState, owner: &str, vouched: bool) -> String {
    let token = random_session_token();
    let expires_at = crate::platform::model::now_ts() + SESSION_TTL_SECS;
    if let Ok(mut sessions) = state.sessions.lock() {
        sessions.insert(
            token.clone(),
            PlatformWebSession {
                owner: owner.to_string(),
                expires_at,
                vouched,
            },
        );
    }
    token
}

fn session_cookie_header(token: &str, max_age: i64) -> String {
    session_cookie_header_same_site(token, max_age, "Strict")
}

/// `SameSite` is a parameter for exactly one caller: the browser vouch landing.
///
/// `offices.md` §3 makes the office a different host from its controller, so
/// the "Open office" hand-off is cross-site *by construction*. Under
/// `SameSite=Strict` the browser sets the cookie and then withholds it from the
/// redirect that follows, and the operator lands on the office's login page
/// holding a session it will not send. `Lax` is the narrowest attribute that
/// survives a top-level GET hand-off, and it is what every SSO callback uses
/// for the same reason. A locally-authenticated session stays `Strict`.
fn session_cookie_header_same_site(token: &str, max_age: i64, same_site: &str) -> String {
    let secure = match std::env::var("ZEBFLOW_COOKIE_SECURE") {
        Ok(value) => matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        ),
        Err(_) => !matches!(
            std::env::var("ZEBFLOW_PLATFORM_HOST")
                .unwrap_or_else(|_| "127.0.0.1".to_string())
                .as_str(),
            "127.0.0.1" | "localhost" | "::1"
        ),
    };
    let secure_attr = if secure { "; Secure" } else { "" };
    format!(
        "{SESSION_COOKIE_NAME}={token}; Path=/; HttpOnly; SameSite={same_site}; Max-Age={max_age}{secure_attr}",
    )
}

fn session_token(headers: &HeaderMap) -> Option<String> {
    let cookie = headers.get(axum::http::header::COOKIE)?.to_str().ok()?;
    cookie.split(';').map(str::trim).find_map(|part| {
        part.strip_prefix(&format!("{SESSION_COOKIE_NAME}="))
            .map(ToString::to_string)
    })
}

fn session_owner(state: &PlatformAppState, headers: &HeaderMap) -> Option<String> {
    let token = session_token(headers)?;
    let now = crate::platform::model::now_ts();
    let owner = {
        let mut sessions = state.sessions.lock().ok()?;
        let Some(session) = sessions.get(&token) else {
            return None;
        };
        if session.expires_at <= now {
            sessions.remove(&token);
            return None;
        }
        session.owner.clone()
    };
    if state
        .platform
        .users
        .get_user(&owner)
        .ok()
        .flatten()
        .is_none()
    {
        if let Ok(mut sessions) = state.sessions.lock() {
            sessions.remove(&token);
        }
        return None;
    }
    Some(owner)
}

/// Page-capability gate. See [`require_project_api_capability`] for the rule
/// the cluster branch follows; the only difference here is that a refusal is a
/// redirect to the login page rather than a status code.
fn require_project_page_capability(
    state: &PlatformAppState,
    headers: &HeaderMap,
    owner: &str,
    project: &str,
    capability: ProjectCapability,
) -> Result<ProjectAccessSubject, Response> {
    if is_controller_call(state, headers) {
        return Ok(ProjectAccessSubject::user(owner));
    }
    let Some(session_owner) = session_owner(state, headers) else {
        return Err(Redirect::to(LOGIN_PATH).into_response());
    };
    let subject = ProjectAccessSubject::user(&session_owner);
    match state
        .platform
        .authz
        .ensure_project_capability(&subject, owner, project, capability)
    {
        Ok(()) => Ok(subject),
        Err(err) if err.code == "PLATFORM_PROJECT_MISSING" => {
            Err((StatusCode::NOT_FOUND, Html("project not found".to_string())).into_response())
        }
        Err(err) if err.code == "PLATFORM_AUTHZ_FORBIDDEN" => {
            Err((StatusCode::FORBIDDEN, Html("forbidden".to_string())).into_response())
        }
        Err(err) => Err(internal_error(err)),
    }
}

/// API-capability gate for one project route.
///
/// The first branch is a **one-directional** trust and nothing wider. It fires
/// only on an office, only for a header that office's own controller signed
/// against that office's current token — so it means "the controller I joined
/// is forwarding a request it already authorised", which is exactly the
/// arrangement `offices.md` §4 describes when it makes the controller's
/// identity "the normal door" of a joined office. The owner comes from the URL
/// because the controller has already decided who is acting; the office has no
/// session of its own to consult, by §4's own login term.
///
/// It used to fire on a controller too, for any active office's join token, and
/// that was the hole: an office could export or overwrite any user's project on
/// its own controller through `/api/internal/project-transfer/...` or any
/// project route, which made §8's "revocable for one office alone" false and
/// §2's three verbs a fiction. [`is_controller_call`] answers `false` on a
/// controller now, always.

// ── Project membership and invitations ───────────────────────────────────────
//
// The services behind these have existed for a while and nothing could reach
// them: `MembersRead` and `MembersWrite` guarded no route, so a project had
// exactly one member — whoever created it — and no way to gain another.
//
// Joining is invited and accepted, never done to someone. A member's git
// identity ends up on commits made in the project, so consent is the point
// rather than a formality, and `invited_by` plus the accepted timestamp says
// who asked and who agreed.

async fn api_list_project_members(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::MembersRead,
    ) {
        return response;
    }
    match state.platform.project_members.list_members(&owner, &project) {
        Ok(items) => Json(json!({"ok": true, "items": items})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_upsert_project_member(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Json(req): Json<crate::platform::model::UpsertProjectMemberRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::MembersWrite,
    ) {
        return response;
    }
    let Some(actor) = session_owner(&state, &headers) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    match state
        .platform
        .project_members
        .upsert_member(&actor, &owner, &project, &req)
    {
        Ok(member) => Json(json!({"ok": true, "member": member})).into_response(),
        Err(err) if err.code == "PLATFORM_MEMBER_ROLE_ABOVE_ACTOR" => (
            StatusCode::FORBIDDEN,
            Json(json!({"ok": false, "error": {"code": err.code, "message": err.message}})),
        )
            .into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_remove_project_member(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, user_id)): Path<(String, String, String)>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::MembersWrite,
    ) {
        return response;
    }
    match state
        .platform
        .project_members
        .remove_member(&owner, &project, &user_id)
    {
        Ok(()) => Json(json!({"ok": true})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_list_project_invites(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::MembersRead,
    ) {
        return response;
    }
    match state.platform.project_invites.list_invites(&owner, &project) {
        Ok(items) => Json(json!({"ok": true, "items": items})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_create_project_invite(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Json(req): Json<crate::platform::model::CreateProjectInviteRequest>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::MembersWrite,
    ) {
        return response;
    }
    let Some(actor) = session_owner(&state, &headers) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    // The ceiling applies to the invitation, not only to the membership it
    // becomes — otherwise a Maintainer invites someone as Owner and the refusal
    // arrives days later, at acceptance, addressed to the wrong person.
    let actor_role = match state
        .platform
        .project_members
        .role_of_public(&owner, &project, &actor)
    {
        Ok(role) => role,
        Err(err) => return internal_error(err),
    };
    if !crate::platform::services::access::roles::can_grant(actor_role, req.role_preset) {
        return (
            StatusCode::FORBIDDEN,
            Json(json!({"ok": false, "error": {
                "code": "PLATFORM_MEMBER_ROLE_ABOVE_ACTOR",
                "message": format!("a {} cannot invite someone as {}",
                    actor_role.title(), req.role_preset.title())
            }})),
        )
            .into_response();
    }
    match state
        .platform
        .project_invites
        .create_invite(&actor, &owner, &project, &req)
    {
        Ok(invite) => Json(json!({"ok": true, "invite": invite})).into_response(),
        Err(err) if err.code == "PLATFORM_INVITE_USER_MISSING" => (
            StatusCode::NOT_FOUND,
            Json(json!({"ok": false, "error": {"code": err.code, "message": err.message}})),
        )
            .into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_revoke_project_invite(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, invite_id)): Path<(String, String, String)>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::MembersWrite,
    ) {
        return response;
    }
    match state
        .platform
        .project_invites
        .revoke_invite(&owner, &project, &invite_id)
    {
        Ok(()) => Json(json!({"ok": true})).into_response(),
        Err(err) => internal_error(err),
    }
}

/// `GET /api/invites` — what this session has been asked to join.
///
/// Session-scoped, not project-scoped: someone deciding whether to join a
/// project cannot be asked to prove they are already in it.
async fn api_list_my_invites(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
) -> Response {
    let Some(actor) = session_owner(&state, &headers) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    match state.platform.project_invites.list_invites_for_user(&actor) {
        Ok(items) => Json(json!({"ok": true, "items": items})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_answer_my_invite(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, invite_id, answer)): Path<(String, String, String, String)>,
) -> Response {
    let Some(actor) = session_owner(&state, &headers) else {
        return StatusCode::UNAUTHORIZED.into_response();
    };
    let invites = &state.platform.project_invites;
    // Declining is one step. Accepting is two — become a member, then record
    // the answer — and they are done in that order deliberately.
    //
    // Marking the invite first is how the first version of this lost one: the
    // membership write failed, the invite was already Accepted, and the person
    // could neither join nor accept again. A member without a recorded answer
    // is recoverable, because the invite still reads pending and accepting is
    // idempotent. An accepted invite without a member is not.
    if answer == "accept" {
        let invite = match invites.peek_answerable(&actor, &owner, &project, &invite_id) {
            Ok(invite) => invite,
            Err(err) => return invite_answer_error(err),
        };
        let member_request = crate::platform::model::UpsertProjectMemberRequest {
            user_id: invite.target_user.clone(),
            role_preset: invite.role_preset,
            custom_policy_ids: invite.custom_policy_ids.clone(),
            mcp_capability_ceiling: invite.mcp_capability_ceiling.clone(),
        };
        let member = match state.platform.project_members.upsert_member(
            &invite.invited_by,
            &owner,
            &project,
            &member_request,
        ) {
            Ok(member) => member,
            Err(err) => return internal_error(err),
        };
        return match invites.accept_invite(&actor, &owner, &project, &invite_id) {
            Ok(invite) => {
                Json(json!({"ok": true, "invite": invite, "member": member})).into_response()
            }
            Err(err) => internal_error(err),
        };
    }

    let result = match answer.as_str() {
        "decline" => invites.decline_invite(&actor, &owner, &project, &invite_id),
        _ => return StatusCode::NOT_FOUND.into_response(),
    };
    let invite = match result {
        Ok(invite) => invite,
        Err(err) if err.code == "PLATFORM_INVITE_NOT_FOUND" => {
            return StatusCode::NOT_FOUND.into_response();
        }
        Err(err)
            if err.code == "PLATFORM_INVITE_NOT_PENDING"
                || err.code == "PLATFORM_INVITE_EXPIRED" =>
        {
            return (
                StatusCode::CONFLICT,
                Json(json!({"ok": false, "error": {"code": err.code, "message": err.message}})),
            )
                .into_response();
        }
        Err(err) => return internal_error(err),
    };

    Json(json!({"ok": true, "invite": invite})).into_response()
}

/// The HTTP answer for a refusal to accept or decline.
fn invite_answer_error(err: crate::platform::error::PlatformError) -> Response {
    match &*err.code {
        "PLATFORM_INVITE_NOT_FOUND" => StatusCode::NOT_FOUND.into_response(),
        "PLATFORM_INVITE_NOT_PENDING" | "PLATFORM_INVITE_EXPIRED" => (
            StatusCode::CONFLICT,
            Json(json!({"ok": false, "error": {"code": err.code, "message": err.message}})),
        )
            .into_response(),
        _ => internal_error(err),
    }
}

fn require_project_api_capability(
    state: &PlatformAppState,
    headers: &HeaderMap,
    owner: &str,
    project: &str,
    capability: ProjectCapability,
) -> Result<ProjectAccessSubject, Response> {
    if is_controller_call(state, headers) {
        return Ok(ProjectAccessSubject::user(owner));
    }
    let Some(session_owner) = session_owner(state, headers) else {
        return Err(StatusCode::UNAUTHORIZED.into_response());
    };
    let subject = ProjectAccessSubject::user(&session_owner);
    match state
        .platform
        .authz
        .ensure_project_capability(&subject, owner, project, capability)
    {
        Ok(()) => Ok(subject),
        Err(err) if err.code == "PLATFORM_PROJECT_MISSING" => {
            Err(StatusCode::NOT_FOUND.into_response())
        }
        Err(err) if err.code == "PLATFORM_AUTHZ_FORBIDDEN" => {
            Err(StatusCode::FORBIDDEN.into_response())
        }
        Err(err) => Err(internal_error(err)),
    }
}

fn canonical_pipeline_node_kind(kind: &str) -> &str {
    if let Some(stripped) = kind.strip_prefix("x.n.") {
        return match stripped {
            "trigger.webhook" => "n.trigger.webhook",
            "trigger.schedule" => "n.trigger.schedule",
            "trigger.manual" => "n.trigger.manual",
            _ => kind,
        };
    }
    kind
}

fn validate_execute_trigger(
    graph: &PipelineGraph,
    req: &ExecutePipelineRequest,
) -> Result<(), String> {
    match req.trigger {
        PipelineExecuteTrigger::Webhook => {
            let wanted_path = req.webhook_path.as_deref().unwrap_or("/").trim();
            let wanted_method = req
                .webhook_method
                .as_deref()
                .unwrap_or("POST")
                .trim()
                .to_uppercase();
            let matched = crate::platform::services::project::webhook_triggers_from_graph(graph)
                .iter()
                .any(|trigger| trigger.path == wanted_path && trigger.method == wanted_method);
            if matched {
                Ok(())
            } else {
                Err(format!(
                    "no webhook trigger matched path='{}' method='{}'",
                    wanted_path, wanted_method
                ))
            }
        }
        PipelineExecuteTrigger::Schedule => {
            let wanted_cron = req.schedule_cron.as_deref().map(str::trim);
            let matched = graph.nodes.iter().any(|node| {
                if canonical_pipeline_node_kind(&node.kind) != "n.trigger.schedule" {
                    return false;
                }
                match wanted_cron {
                    Some(cron) => {
                        node.config
                            .get("cron")
                            .and_then(Value::as_str)
                            .map(str::trim)
                            == Some(cron)
                    }
                    None => true,
                }
            });
            if matched {
                Ok(())
            } else if let Some(cron) = wanted_cron {
                Err(format!("no schedule trigger matched cron='{}'", cron))
            } else {
                Err("pipeline has no schedule trigger".to_string())
            }
        }
        PipelineExecuteTrigger::Manual => {
            let matched = graph
                .nodes
                .iter()
                .any(|node| canonical_pipeline_node_kind(&node.kind) == "n.trigger.manual");
            if matched {
                Ok(())
            } else {
                Err("pipeline has no manual trigger".to_string())
            }
        }
    }
}

fn hydrate_template_markup(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
    graph: &mut PipelineGraph,
) -> Result<(), PlatformError> {
    for node in &mut graph.nodes {
        if node.kind != "n.web.response" {
            continue;
        }

        let has_markup = node
            .config
            .get("markup")
            .and_then(Value::as_str)
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        if has_markup {
            continue;
        }

        let template_rel = node
            .config
            .get("template")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|s| !s.is_empty());

        let Some(template_rel) = template_rel else {
            continue;
        };

        let markup = state
            .platform
            .projects
            .read_repo_file_text(owner, project, template_rel)?;
        if let Some(map) = node.config.as_object_mut() {
            map.insert("markup".to_string(), Value::String(markup));
        }
    }
    Ok(())
}

/// Build a Set-Cookie string from a `__zf_response` envelope's `set_cookie` field.
fn build_zf_cookie(resp_cfg: &Value) -> Option<String> {
    let sc = resp_cfg.get("set_cookie")?;
    let name = sc.get("name")?.as_str()?;
    let value = sc.get("value")?.as_str()?;
    let max_age = sc.get("max_age").and_then(Value::as_i64).unwrap_or(900);
    let path = sc.get("path").and_then(Value::as_str).unwrap_or("/");
    let same_site = sc.get("same_site").and_then(Value::as_str).unwrap_or("Lax");
    let http_only = sc.get("http_only").and_then(Value::as_bool).unwrap_or(true);
    let secure = sc.get("secure").and_then(Value::as_bool).unwrap_or(false);

    let mut parts = vec![
        format!("{name}={value}"),
        format!("Path={path}"),
        format!("Max-Age={max_age}"),
        format!("SameSite={same_site}"),
    ];
    if http_only {
        parts.push("HttpOnly".to_string());
    }
    if secure {
        parts.push("Secure".to_string());
    }
    Some(parts.join("; "))
}

/// Build extra headers list from a `__zf_response` envelope's `headers` field.
fn build_zf_headers(resp_cfg: &Value) -> Vec<(String, String)> {
    resp_cfg
        .get("headers")
        .and_then(Value::as_object)
        .map(|m| {
            m.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

/// Apply cookie and extra headers from a `__zf_response` envelope to an axum Response.
fn apply_zf_extras(
    resp: &mut axum::response::Response,
    cookie: &Option<String>,
    headers: &[(String, String)],
) {
    if let Some(c) = cookie {
        if let Ok(v) = HeaderValue::from_str(c) {
            resp.headers_mut().insert(SET_COOKIE, v);
        }
    }
    for (k, v) in headers {
        if let (Ok(name), Ok(val)) = (
            HeaderName::from_bytes(k.as_bytes()),
            HeaderValue::from_str(v),
        ) {
            // A header the pipeline declared replaces the one the response type
            // picked for itself. Appending instead left an SVG served through
            // `--message` carrying both `text/plain; charset=utf-8` and
            // `image/svg+xml`, and the browser reads the first one it sees.
            //
            // `set-cookie` is the exception the HTTP spec makes: repetition is
            // how you send more than one cookie, so those still stack.
            if name == SET_COOKIE {
                resp.headers_mut().append(name, val);
            } else {
                resp.headers_mut().insert(name, val);
            }
        }
    }
}

fn compile_template_buffer(
    state: &PlatformAppState,
    template_root: &FsPath,
    req: &TemplateCompileRequest,
) -> TemplateCompileResponse {
    let rel = req.rel_path.trim();
    if rel.is_empty() {
        return TemplateCompileResponse {
            ok: false,
            diagnostics: vec![TemplateDiagnostic {
                code: "template_path_missing".to_string(),
                message: "template path must not be empty".to_string(),
                severity: "error".to_string(),
                from: Some(0),
                to: Some(1),
            }],
        };
    }

    let kind = template_kind_from_rel(rel);
    if kind == "script" || kind == "style" {
        return TemplateCompileResponse {
            ok: true,
            diagnostics: Vec::new(),
        };
    }

    let options = ReactiveWebOptions {
        load_scripts: vec!["/assets/platform/*".to_string()],
        allow_list: crate::rwe::ResourceAllowList {
            scripts: vec!["/assets/platform/*".to_string()],
            urls: vec!["/assets/platform/*".to_string()],
            ..Default::default()
        },
        templates: TemplateOptions {
            template_root: Some(template_root.to_path_buf()),
            // The editor checks a buffer with the same resolution a render
            // uses, or `zeb/ui/button` would fail here and work in the page.
            library_roots: state.platform.library.source_roots(),
            style_entries: Vec::new(),
        },
        processors: vec!["tailwind".to_string()],
        ..Default::default()
    };

    let source = TemplateSource {
        id: format!("platform.editor.{}", rel.replace('/', ".")),
        source_path: Some(template_root.join(rel)),
        markup: req.content.clone(),
    };

    match state
        .frontend
        .rwe
        .compile_template(&source, state.frontend.language.as_ref(), &options)
    {
        Ok(compiled) => TemplateCompileResponse {
            ok: true,
            diagnostics: compiled
                .diagnostics
                .into_iter()
                .map(|diag| TemplateDiagnostic {
                    code: diag.code,
                    message: diag.message,
                    severity: "warning".to_string(),
                    from: None,
                    to: None,
                })
                .collect(),
        },
        Err(err) => TemplateCompileResponse {
            ok: false,
            diagnostics: vec![TemplateDiagnostic {
                code: err.code.to_string(),
                message: err.message,
                severity: "error".to_string(),
                from: Some(0),
                to: Some(1),
            }],
        },
    }
}

async fn api_get_mcp_session(
    State(state): State<PlatformAppState>,
    Path((owner, project)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    if let Err(resp) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::McpSessionCreate,
    ) {
        return resp;
    }

    let base_url = platform_base_url(&headers);

    match state
        .platform
        .mcp_sessions
        .get_for_project(&owner, &project)
    {
        Some(session) => {
            let mcp_url = format!(
                "{}/api/projects/{}/{}/mcp",
                base_url.trim_end_matches('/'),
                owner,
                project
            );
            Json(json!({
                "ok": true,
                "session": {
                    "active": session.enabled,
                    "enabled": session.enabled,
                    "token": session.token,
                    "mcp_url": mcp_url,
                    "capabilities": session.capabilities.iter().map(|c| c.key()).collect::<Vec<_>>(),
                    "created_at": session.created_at,
                    "auto_reset_seconds": session.auto_reset_seconds,
                    "rotation_epoch": state.platform.mcp_sessions.min_created_at(),
                }
            }))
            .into_response()
        }
        None => Json(json!({
            "ok": true,
            "session": {
                "active": false,
                "enabled": false,
                "token": null,
                "mcp_url": null,
                "capabilities": Vec::<String>::new(),
                "created_at": null,
                "auto_reset_seconds": null,
                "rotation_epoch": state.platform.mcp_sessions.min_created_at(),
            }
        }))
        .into_response(),
    }
}

async fn api_create_mcp_session(
    State(state): State<PlatformAppState>,
    Path((owner, project)): Path<(String, String)>,
    headers: HeaderMap,
    Json(req): Json<McpSessionCreateRequest>,
) -> Response {
    if let Err(resp) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::McpSessionCreate,
    ) {
        return resp;
    }

    let capabilities: Vec<ProjectCapability> = req
        .capabilities
        .iter()
        .filter_map(|key| ProjectCapability::from_key(key))
        .collect();

    if capabilities.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "ok": false,
                "error": {"code": "INVALID_REQUEST", "message": "At least one valid capability must be specified"}
            })),
        )
            .into_response();
    }

    let base_url = platform_base_url(&headers);

    match state.platform.mcp_sessions.create(
        &owner,
        &project,
        capabilities,
        &base_url,
        req.auto_reset_seconds,
    ) {
        Ok(response) => Json(json!({"ok": true, "session": response})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_toggle_mcp_session(
    State(state): State<PlatformAppState>,
    Path((owner, project)): Path<(String, String)>,
    headers: HeaderMap,
    Json(req): Json<McpSessionToggleRequest>,
) -> Response {
    if let Err(resp) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::McpSessionCreate,
    ) {
        return resp;
    }

    if req.enabled
        && state
            .platform
            .mcp_sessions
            .get_for_project(&owner, &project)
            .is_none()
    {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "ok": false,
                "error": {
                    "code": "MCP_SESSION_NOT_FOUND",
                    "message": "Create an MCP session before enabling it"
                }
            })),
        )
            .into_response();
    }

    match state
        .platform
        .mcp_sessions
        .set_enabled(&owner, &project, req.enabled)
    {
        Ok(()) => Json(json!({"ok": true})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_reset_mcp_session_token(
    State(state): State<PlatformAppState>,
    Path((owner, project)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    if let Err(resp) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::McpSessionCreate,
    ) {
        return resp;
    }

    let base_url = platform_base_url(&headers);

    match state
        .platform
        .mcp_sessions
        .reset_token(&owner, &project, &base_url)
    {
        Ok(response) => Json(json!({"ok": true, "session": response})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn api_revoke_mcp_session(
    State(state): State<PlatformAppState>,
    Path((owner, project)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    if let Err(resp) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::McpSessionRevoke,
    ) {
        return resp;
    }

    match state
        .platform
        .mcp_sessions
        .revoke_for_project(&owner, &project)
    {
        Ok(()) => Json(json!({"ok": true})).into_response(),
        Err(err) => internal_error(err),
    }
}

async fn run_platform_blocking<T, F>(f: F) -> Result<T, PlatformError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, PlatformError> + Send + 'static,
{
    tokio::task::spawn_blocking(f).await.map_err(|err| {
        PlatformError::new(
            "PLATFORM_BLOCKING_TASK_JOIN",
            format!("blocking platform task failed: {err}"),
        )
    })?
}

fn internal_error(err: PlatformError) -> Response {
    let status = match &*err.code {
        "PLATFORM_PIPELINE_LOCKED" | "PLATFORM_TEMPLATE_LOCKED" => StatusCode::LOCKED,
        "HUB_PUBLISHER_MISSING"
        | "HUB_ASSET_MISSING"
        | "HUB_PUBLISH_SOURCE_MISSING"
        | "HUB_SERVICE_DISABLED"
        | "MCP_SESSION_NOT_FOUND" => StatusCode::NOT_FOUND,
        "HUB_TOKEN_FORBIDDEN"
        | "HUB_PUBLISHER_FORBIDDEN"
        | "HUB_PUBLISHER_SCOPE_DENIED"
        | "HUB_PACKAGE_FORBIDDEN" => StatusCode::FORBIDDEN,
        "HUB_PUBLISHER_DISABLED" => StatusCode::CONFLICT,
        "HUB_PUBLISHER_QUOTA_EXCEEDED" => StatusCode::CONFLICT,
        // A published release is immutable, so a republish is a conflict with
        // what is already there, not a malformed request.
        "HUB_VERSION_EXISTS" => StatusCode::CONFLICT,
        // Retraction destroyed the bytes and kept the coordinate, which is what
        // 410 says and 404 would not.
        "HUB_VERSION_RETRACTED" => StatusCode::GONE,
        "HUB_TOKEN_INVALID"
        | "HUB_ARTIFACT_PATH_INVALID"
        | "HUB_PUBLISH_INVALID"
        | "HUB_REMOTE_INVALID"
        | "HUB_REMOTE_HASH_MISSING"
        | "HUB_REMOTE_HASH_MISMATCH"
        | "HUB_REPOSITORY_INVALID"
        | "HUB_TOKEN_SCOPE_INVALID"
        | "HUB_PUBLISH_EMPTY" => StatusCode::BAD_REQUEST,
        "HUB_ARTIFACT_TOO_LARGE" => StatusCode::PAYLOAD_TOO_LARGE,
        // A pipeline the author wrote wrongly is a 4xx, and saying 500 tells
        // them the server broke and invites a retry that must fail identically.
        // This is the transport half of the refused/failed split in the NodeIO
        // contract: refused is the caller's to fix, failed is ours.
        "PLATFORM_PIPELINE_INVALID"
        | "PLATFORM_PIPELINE_PARSE"
        | "PLATFORM_PIPELINE_SCRIPT_INVALID"
        | "PLATFORM_PIPELINE_PATH"
        | "PLATFORM_PIPELINE_ID_MISSING"
        | "PLATFORM_PIPELINE_META_INJECTED"
        | "PLATFORM_PIPELINE_TRIGGER_MISMATCH"
        | "PLATFORM_PIPELINE_REGISTRY_SCOPE_INVALID" => StatusCode::BAD_REQUEST,
        // Someone else already owns that webhook path: a conflict with existing
        // state rather than a malformed request.
        "PLATFORM_PIPELINE_WEBHOOK_CONFLICT" => StatusCode::CONFLICT,
        "PLATFORM_PIPELINE_MISSING" | "PLATFORM_PIPELINE_ACTIVE_SNAPSHOT_MISSING" => {
            StatusCode::NOT_FOUND
        }
        _ => StatusCode::INTERNAL_SERVER_ERROR,
    };
    (
        status,
        Json(json!({"ok": false, "error": {"code": err.code, "message": err.message}})),
    )
        .into_response()
}

// ---- WebSocket handlers ---------------------------------------------------

/// Upgrade handler for application room WebSocket connections.
///
/// URL: `GET /ws/{owner}/{project}/rooms/{room_id}`
///
/// After upgrade, the handler:
/// 1. Subscribes to the room broadcast channel
/// 2. Sends an initial `joined` message with current room state
/// 3. Forwards broadcasts from the room to the client
/// 4. Dispatches inbound `{event, payload}` messages to matching WS pipelines
async fn ws_room_handler(
    ws: WebSocketUpgrade,
    Path((owner, project, room_id)): Path<(String, String, String)>,
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    uri: Uri,
) -> Response {
    match remote_project_worker_id(&state, &owner, &project) {
        Ok(Some(worker_id)) => {
            let worker = match state.platform.cluster_registry.get_worker(&worker_id) {
                Ok(Some(worker)) => worker,
                Ok(None) => {
                    return (
                        StatusCode::BAD_GATEWAY,
                        Json(json!({"ok": false, "error": "office not registered"})),
                    )
                        .into_response();
                }
                Err(err) => return internal_error(err),
            };
            let worker_url = worker_websocket_url(&worker.base_url, &uri);
            let proxy_headers = forwarded_websocket_headers(&headers);
            return ws
                .on_upgrade(move |socket| {
                    proxy_websocket_to_worker(socket, worker_url, proxy_headers)
                })
                .into_response();
        }
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }

    let session_id = format!(
        "ws-{:016x}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    // Capture connection headers for per-trigger auth checks on incoming messages.
    ws.on_upgrade(move |socket| {
        handle_ws_room(socket, owner, project, room_id, session_id, headers, state)
    })
    .into_response()
}

async fn handle_ws_room(
    mut socket: WebSocket,
    owner: String,
    project: String,
    room_id: String,
    session_id: String,
    connection_headers: HeaderMap,
    state: PlatformAppState,
) {
    let room_key = format!("{}/{}/{}", owner, project, room_id);
    let room = state.platform.ws_hub.get_or_create_room(&room_key);

    // Subscribe BEFORE reading state — avoids missing a patch that arrives between the two.
    let mut broadcast_rx = room.subscribe();

    // Join session (auto-decrements count on drop).
    let _guard = room.join_session();

    // Snapshot current state for the initial message.
    let current_state = room.get_state();

    // Send joined message.
    let joined = serde_json::json!({
        "type": "joined",
        "session_id": session_id,
        "room": room_id,
        "state": current_state,
    })
    .to_string();
    if socket.send(Message::Text(joined.into())).await.is_err() {
        return;
    }

    // Main loop: interleave incoming WS messages and room broadcasts.
    loop {
        tokio::select! {
            // Forward room broadcasts to this client.
            broadcast = broadcast_rx.recv() => {
                match broadcast {
                    Ok(msg) => {
                        if socket.send(Message::Text(msg.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        // Fell behind — skip missed messages and continue.
                        continue;
                    }
                }
            }

            // Process messages from this client.
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        let text_str = text.as_str();
                        if let Ok(val) = serde_json::from_str::<serde_json::Value>(text_str) {
                            let event = val
                                .get("event")
                                .and_then(|v| v.as_str())
                                .unwrap_or("message")
                                .to_string();
                            let payload = val
                                .get("payload")
                                .cloned()
                                .unwrap_or(serde_json::json!({}));
                            // Dispatch to matching WS pipelines (non-blocking — fires in background).
                            ws_dispatch_event(
                                &owner, &project, &room_id, &session_id,
                                &event, payload, &connection_headers, &state,
                            ).await;
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
        }
    }

    // Clean up room if now empty.
    state.platform.ws_hub.remove_room(&room_key);
}

/// Dispatch an incoming WS event to all matching `n.trigger.ws` pipelines.
///
/// Auth is checked per-trigger using the connection's initial HTTP headers
/// (which carry cookies set by the login pipeline). Same mechanism as webhook
/// auth — `verify_webhook_auth` is reused.
async fn ws_dispatch_event(
    owner: &str,
    project: &str,
    room_id: &str,
    session_id: &str,
    event: &str,
    payload: Value,
    connection_headers: &HeaderMap,
    state: &PlatformAppState,
) {
    let pipelines = state.platform.pipeline_runtime.list_project(owner, project);

    // Collect (compiled_pipeline, matched_trigger) pairs so we can access the
    // trigger's auth fields.
    struct Match {
        compiled: crate::platform::services::pipeline_runtime::CompiledPipeline,
        auth_type: String,
        auth_credential: String,
        auth_required_role: Vec<String>,
    }

    let matching: Vec<Match> = pipelines
        .into_iter()
        .filter_map(|p| {
            let trigger = p.ws_triggers.iter().find(|t| {
                let room_match = t.room.is_empty() || t.room == room_id;
                let event_match = t.event.is_empty() || t.event == event;
                room_match && event_match
            })?;
            Some(Match {
                auth_type: trigger.auth_type.clone(),
                auth_credential: trigger.auth_credential.clone(),
                auth_required_role: trigger.auth_required_role.clone(),
                compiled: p,
            })
        })
        .collect();

    for m in matching {
        // Per-trigger auth check using the connection's initial HTTP headers.
        let auth_claims = if m.auth_type.is_empty() || m.auth_type == "none" {
            // No auth configured — open trigger.
            Ok(None)
        } else if is_controller_call(state, connection_headers) {
            // Cluster-internal traffic bypasses project-level auth.
            Ok(None)
        } else {
            verify_webhook_auth(
                connection_headers,
                &axum::body::Bytes::new(), // WS messages don't have a body for HMAC — use empty.
                &m.auth_type,
                &m.auth_credential,
                &m.auth_required_role,
                &state.platform.credentials,
                owner,
                project,
            )
        };

        let auth_claims = match auth_claims {
            Ok(claims) => claims,
            Err(_) => {
                // Auth failed — silently skip this trigger (same as a 401 on webhook).
                continue;
            }
        };

        let mut input = json!({
            "room_id": room_id,
            "session_id": session_id,
            "event": event,
            "payload": payload,
        });
        // Inject JWT claims as `auth` field (same as webhook).
        if let Some(claims) = &auth_claims {
            if let Value::Object(ref mut map) = input {
                map.insert("auth".to_string(), claims.clone());
            }
        }

        let trigger = json!({
            "auth": auth_claims.clone().unwrap_or(Value::Null),
        });

        let ctx = PipelineContext {
            owner: owner.to_string(),
            project: project.to_string(),
            pipeline: m.compiled.graph.id.clone(),
            request_id: format!(
                "ws-{}-{}",
                session_id,
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_millis()
            ),
            route: Default::default(),
            input,
            trigger: Some(trigger),
            placeholder: None,
        };
        let graph = m.compiled.graph.clone();
        let credentials = state.platform.credentials.clone();
        let rwe = state.frontend.rwe.clone();
        let ws_hub = state.platform.ws_hub.clone();
        let ws_client_mgr = state.ws_client_manager.clone();
        let state_bus = state.platform.state_bus.clone();
        let data_root = state.platform.config.data_root.clone();
        let platform_clone = state.platform.clone();
        let sandbox = state.platform.project_sandbox(owner, project);
        tokio::spawn(async move {
            let engine = crate::pipeline::BasicPipelineEngine::new(
                std::sync::Arc::new(sandbox),
                rwe,
                Some(credentials),
            )
            .with_platform(platform_clone)
            .with_ws_hub(ws_hub)
            .with_ws_client_manager(ws_client_mgr)
            .with_state_bus(state_bus)
            .with_data_root(data_root);
            let _ = engine.execute_async(&graph, &ctx).await;
        });
    }
}

// ── Editor API handlers ────────────────────────────────────────────────────────

fn editor_class_completion(label: &str, section: &str, detail: &str) -> Value {
    json!({
        "label": label,
        "type": "constant",
        "detail": detail,
        "section": { "name": section },
    })
}

fn editor_push_class_completions(
    completions: &mut Vec<Value>,
    labels: &[&str],
    section: &str,
    detail: &str,
) {
    for label in labels {
        completions.push(editor_class_completion(label, section, detail));
    }
}

fn editor_push_tailwind_spacing_completions(completions: &mut Vec<Value>) {
    let steps = [
        "0", "px", "0.5", "1", "1.5", "2", "2.5", "3", "3.5", "4", "5", "6", "7", "8", "9", "10",
        "11", "12", "14", "16", "20", "24", "28", "32", "36", "40", "44", "48", "52", "56", "60",
        "64", "72", "80", "96",
    ];
    let positive_prefixes = [
        "p", "px", "py", "pt", "pr", "pb", "pl", "gap", "gap-x", "gap-y", "space-x", "space-y",
    ];
    let margin_prefixes = ["m", "mx", "my", "mt", "mr", "mb", "ml"];

    for prefix in positive_prefixes {
        for step in steps {
            completions.push(editor_class_completion(
                &format!("{prefix}-{step}"),
                "Spacing",
                "Tailwind spacing",
            ));
        }
    }
    for prefix in margin_prefixes {
        completions.push(editor_class_completion(
            &format!("{prefix}-auto"),
            "Spacing",
            "Tailwind spacing",
        ));
        for step in steps {
            completions.push(editor_class_completion(
                &format!("{prefix}-{step}"),
                "Spacing",
                "Tailwind spacing",
            ));
            if step != "0" {
                completions.push(editor_class_completion(
                    &format!("-{prefix}-{step}"),
                    "Spacing",
                    "Tailwind spacing",
                ));
            }
        }
    }
}

fn editor_completion_catalog() -> Value {
    let layout = [
        "block",
        "inline",
        "inline-block",
        "flex",
        "inline-flex",
        "grid",
        "hidden",
        "relative",
        "absolute",
        "fixed",
        "sticky",
        "inset-0",
        "w-full",
        "w-screen",
        "h-full",
        "h-screen",
        "min-w-0",
        "min-h-0",
        "min-h-screen",
        "max-w-sm",
        "max-w-md",
        "max-w-lg",
        "max-w-xl",
        "max-w-2xl",
        "max-w-3xl",
        "max-w-4xl",
        "max-w-5xl",
        "max-w-6xl",
        "max-w-7xl",
        "max-w-none",
        "overflow-hidden",
        "overflow-auto",
        "overflow-x-auto",
        "overflow-y-auto",
        "truncate",
        "whitespace-nowrap",
    ];
    let flex_grid = [
        "flex-row",
        "flex-col",
        "flex-wrap",
        "flex-nowrap",
        "flex-1",
        "shrink-0",
        "grow",
        "grow-0",
        "items-start",
        "items-center",
        "items-end",
        "items-baseline",
        "items-stretch",
        "justify-start",
        "justify-center",
        "justify-end",
        "justify-between",
        "justify-around",
        "justify-evenly",
        "place-items-center",
        "grid-cols-1",
        "grid-cols-2",
        "grid-cols-3",
        "grid-cols-4",
        "grid-cols-6",
        "grid-cols-12",
        "gap-1",
        "gap-1.5",
        "gap-2",
        "gap-3",
        "gap-4",
        "gap-5",
        "gap-6",
        "gap-8",
    ];
    let visual = [
        "rounded",
        "rounded-sm",
        "rounded-md",
        "rounded-lg",
        "rounded-xl",
        "rounded-full",
        "border",
        "border-0",
        "border-t",
        "border-b",
        "shadow",
        "shadow-sm",
        "shadow-md",
        "shadow-lg",
        "ring-1",
        "ring-2",
        "opacity-50",
        "opacity-60",
        "opacity-70",
        "opacity-100",
        "transition",
        "transition-colors",
        "duration-150",
        "duration-200",
    ];
    let tokens = [
        "bg-background",
        "bg-card",
        "bg-popover",
        "bg-muted",
        "bg-accent",
        "bg-primary",
        "bg-secondary",
        "bg-destructive",
        "bg-success",
        "bg-warning",
        "bg-info",
        "text-foreground",
        "text-muted-foreground",
        "text-card-foreground",
        "text-primary",
        "text-primary-foreground",
        "text-destructive",
        "text-success",
        "text-warning",
        "text-info",
        "border-border",
        "border-input",
        "border-primary",
        "ring-ring",
    ];
    let typography = [
        "text-xs",
        "text-sm",
        "text-base",
        "text-lg",
        "text-xl",
        "text-2xl",
        "text-3xl",
        "text-4xl",
        "font-sans",
        "font-mono",
        "font-display",
        "font-normal",
        "font-medium",
        "font-semibold",
        "font-bold",
        "font-black",
        "leading-none",
        "leading-tight",
        "leading-snug",
        "leading-normal",
        "tracking-wide",
        "uppercase",
    ];
    let variants = [
        "hover:bg-muted",
        "hover:bg-accent",
        "hover:text-foreground",
        "hover:text-primary",
        "focus:outline-none",
        "focus:ring-2",
        "disabled:opacity-50",
        "disabled:pointer-events-none",
        "sm:flex",
        "md:flex",
        "lg:flex",
        "sm:grid-cols-2",
        "md:grid-cols-2",
        "lg:grid-cols-2",
        "lg:grid-cols-3",
    ];

    let mut completions = Vec::new();
    editor_push_class_completions(&mut completions, &layout, "Layout", "Tailwind layout");
    editor_push_class_completions(
        &mut completions,
        &flex_grid,
        "Flex/Grid",
        "Tailwind flex/grid",
    );
    editor_push_tailwind_spacing_completions(&mut completions);
    editor_push_class_completions(&mut completions, &visual, "Visual", "Tailwind visual");
    editor_push_class_completions(&mut completions, &tokens, "Zebflow Tokens", "Zebflow token");
    editor_push_class_completions(
        &mut completions,
        &typography,
        "Typography",
        "Tailwind typography",
    );
    editor_push_class_completions(&mut completions, &variants, "Variants", "Tailwind variant");

    json!({
        "version": "zebflow-editor-catalog-v1",
        "class_completions": completions,
        "variants": ["hover", "focus", "active", "disabled", "sm", "md", "lg", "xl", "dark", "group-hover", "peer-focus"],
        "components": [],
    })
}

async fn api_editor_completion_catalog(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    if let Err(r) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesRead,
    ) {
        return r;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::GET,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }

    Json(json!({ "ok": true, "catalog": editor_completion_catalog() })).into_response()
}

// ── Catalog API handlers ───────────────────────────────────────────────────────

async fn api_list_ui_catalog(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    if let Err(r) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesRead,
    ) {
        return r;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::GET,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    let layout = match state.platform.file.ensure_project_layout(&owner, &project) {
        Ok(l) => l,
        Err(e) => return internal_error(e),
    };
    let shared_ui_dir = layout.repo_source_dir().join("shared").join("ui");
    let entries = crate::platform::catalog::CatalogService::list_ui_with_presence(&shared_ui_dir);
    Json(json!({ "ok": true, "components": entries })).into_response()
}

async fn api_review_ui_components(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Json(req): Json<crate::platform::catalog::InstallUiRequest>,
) -> Response {
    if let Err(r) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesRead,
    ) {
        return r;
    }
    match maybe_forward_project_json_to_worker(
        &state,
        &uri,
        &headers,
        Method::POST,
        &req,
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    let layout = match state.platform.file.ensure_project_layout(&owner, &project) {
        Ok(l) => l,
        Err(e) => return internal_error(e),
    };
    let shared_ui_dir = layout.repo_source_dir().join("shared").join("ui");
    let review = crate::platform::catalog::CatalogService::review_ui(
        &layout.repo_layout,
        &req.names,
        &shared_ui_dir,
        req.overwrite,
    );
    Json(json!({ "ok": true, "review": review })).into_response()
}

async fn api_install_ui_components(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Json(req): Json<crate::platform::catalog::InstallUiRequest>,
) -> Response {
    if let Err(r) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesWrite,
    ) {
        return r;
    }
    match maybe_forward_project_json_to_worker(
        &state,
        &uri,
        &headers,
        Method::POST,
        &req,
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }
    let layout = match state.platform.file.ensure_project_layout(&owner, &project) {
        Ok(l) => l,
        Err(e) => return internal_error(e),
    };
    let shared_ui_dir = layout.repo_source_dir().join("shared").join("ui");
    match crate::platform::catalog::CatalogService::install_ui_reviewed(
        &layout.repo_layout,
        &req.names,
        &shared_ui_dir,
        req.overwrite,
    ) {
        Ok(report) => Json(json!({ "ok": true, "report": report })).into_response(),
        Err(e) => Json(json!({ "ok": false, "error": e })).into_response(),
    }
}

/// `POST /api/projects/{owner}/{project}/reindex` — scan repo files on disk and re-register
/// them into the catalog. Useful after a catalog DB wipe or crash recovery where `repo/`
/// files survive but their metadata rows are lost.
///
/// Returns `{ ok, pipelines, templates, assets, errors }`.
async fn api_reindex_project(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
) -> Response {
    if let Err(r) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::PipelinesWrite,
    ) {
        return r;
    }
    match maybe_forward_project_api_to_worker(
        &state,
        &uri,
        &Method::POST,
        &headers,
        Bytes::new(),
        &owner,
        &project,
    )
    .await
    {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }

    let report = match reindex_project_repo_files(&state, &owner, &project) {
        Ok(report) => report,
        Err(err) => return internal_error(err),
    };
    let mut body = report;
    body["ok"] = json!(true);
    Json(body).into_response()
}

/// Scans `repo/` on disk and re-registers every found pipeline into the
/// catalog. Shared by explicit reindex and platform import, where a freshly
/// materialized repo has no catalog rows yet.
fn reindex_project_repo_files(
    state: &PlatformAppState,
    owner: &str,
    project: &str,
) -> Result<serde_json::Value, PlatformError> {
    let owner_slug = crate::platform::model::slug_segment(owner);
    let project_slug = crate::platform::model::slug_segment(project);

    let layout = state
        .platform
        .file
        .ensure_project_layout(&owner_slug, &project_slug)?;

    let repo_root = layout.repo_source_dir();
    let assets_root = layout.repo_static_dir();
    let mut pipelines_indexed: usize = 0;
    let mut templates_indexed: usize = 0;
    let mut assets_indexed: usize = 0;
    let mut errors: Vec<String> = Vec::new();

    // Iterative directory walk using an explicit stack.
    let mut stack = vec![repo_root.clone()];
    while let Some(dir) = stack.pop() {
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let fname = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if fname.starts_with('.') {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else if path.is_file() {
                let rel = match path.strip_prefix(&repo_root) {
                    Ok(r) => r.to_string_lossy().replace('\\', "/"),
                    Err(_) => continue,
                };
                if path
                    .parent()
                    .is_some_and(|dir| dir.starts_with(&assets_root))
                {
                    assets_indexed += 1;
                } else if rel.ends_with(".zf.json") {
                    match std::fs::read_to_string(&path) {
                        Ok(source) => {
                            let file_rel_path = layout.repo_layout.source_rel(&rel);
                            // Preserve description stored inside the graph JSON
                            let graph_description = decode_pipeline_graph(source.as_bytes())
                                .ok()
                                .and_then(|document| document.spec.description)
                                .unwrap_or_default();
                            let reindex_trigger_kind =
                                crate::platform::services::project::derive_trigger_kind_from_source(&source)
                                    .unwrap_or_default();
                            match state.platform.projects.upsert_pipeline_definition(
                                &owner_slug,
                                &project_slug,
                                &file_rel_path,
                                "",
                                &graph_description,
                                &reindex_trigger_kind,
                                &source,
                            ) {
                                Ok(_) => pipelines_indexed += 1,
                                Err(e) => errors.push(format!("{rel}: {e}")),
                            }
                        }
                        Err(e) => errors.push(format!("{rel}: read error: {e}")),
                    }
                } else if rel.ends_with(".tsx") || rel.ends_with(".ts") {
                    templates_indexed += 1;
                }
            }
        }
    }

    Ok(json!({
        "pipelines": pipelines_indexed,
        "templates": templates_indexed,
        "assets": assets_indexed,
        "errors": errors
    }))
}

async fn project_settings_clone_ui_preview_page(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    uri: Uri,
    Path((owner, project)): Path<(String, String)>,
) -> Response {
    if let Err(response) = require_project_page_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::SettingsRead,
    ) {
        return response;
    }
    match maybe_forward_project_page_to_worker(&state, &headers, &uri, &owner, &project).await {
        Ok(Some(response)) => return response,
        Ok(None) => {}
        Err(err) => return internal_error(err),
    }

    match state.platform.projects.get_project(&owner, &project) {
        Ok(Some(info)) => {
            let nav = nav_classes(&owner, &project, "settings", None);
            let route = format!("/projects/{owner}/{project}/settings/clone/ui/preview");
            let input = json!({
                "seo": {
                    "title": format!("{} - UI catalog preview", info.title),
                    "description": "shadcn-compatible Zeb React components for this project",
                },
                "owner": info.owner,
                "project": info.project,
                "title": info.title,
                "project_href": format!("/projects/{owner}/{project}"),
                "components": crate::platform::catalog::CatalogService::list_ui(),
                "nav": nav,
            });
            match render_page(
                &state,
                "platform-project-settings-clone-ui-preview",
                &route,
                input,
            ) {
                Ok(html) => Html(html).into_response(),
                Err(err) => internal_error(err),
            }
        }
        Ok(None) => (StatusCode::NOT_FOUND, Html("project not found".to_string())).into_response(),
        Err(err) => internal_error(err),
    }
}

// ─── Asset Management API ────────────────────────────────────────────────────

/// Sanitize an asset filename: allow `[A-Za-z0-9._-]`, reject `..` and `/`,
/// reject leading dot, max 200 chars. Returns `None` if invalid.
fn sanitize_asset_filename(raw: &str) -> Option<String> {
    let name = raw.trim();
    if name.is_empty() || name.len() > 200 {
        return None;
    }
    if name.starts_with('.') || name.contains("..") || name.contains('/') || name.contains('\\') {
        return None;
    }
    // Replace spaces and invalid chars with '-', then collapse consecutive dashes.
    let replaced: String = name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '_' || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect();
    // Collapse consecutive dashes (e.g. "foo--bar" → "foo-bar")
    let mut sanitized = String::with_capacity(replaced.len());
    let mut prev_dash = false;
    for c in replaced.chars() {
        if c == '-' {
            if !prev_dash {
                sanitized.push(c);
            }
            prev_dash = true;
        } else {
            sanitized.push(c);
            prev_dash = false;
        }
    }
    // Strip leading/trailing dashes
    let sanitized = sanitized.trim_matches('-').to_string();
    if sanitized.is_empty() {
        return None;
    }
    Some(sanitized)
}

/// Sanitize a single subfolder segment: `[A-Za-z0-9_-]` only, no slashes or dots.
fn sanitize_subfolder(s: &str) -> Option<String> {
    let s = s.trim_matches('/');
    if s.is_empty() {
        return None;
    }
    if s.bytes()
        .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_')
    {
        Some(s.to_string())
    } else {
        None
    }
}

/// Sanitize a multi-segment asset subpath (e.g. `images/logo.png`).
/// Each segment must pass `[A-Za-z0-9._-]` rules and not start with `.`.
fn sanitize_asset_subpath(path: &str) -> Option<std::path::PathBuf> {
    let trimmed = path.trim_matches('/');
    if trimmed.is_empty() || trimmed.contains("..") {
        return None;
    }
    let mut result = std::path::PathBuf::new();
    for part in trimmed.split('/') {
        if part.is_empty() || part.starts_with('.') {
            return None;
        }
        if !part
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || c == b'.' || c == b'-' || c == b'_')
        {
            return None;
        }
        result.push(part);
    }
    if result.components().count() == 0 {
        return None;
    }
    Some(result)
}

/// Query params shared by asset list/upload endpoints.
#[derive(serde::Deserialize)]
struct AssetQuery {
    subfolder: Option<String>,
}

/// `GET /api/projects/{owner}/{project}/assets` — list files in `repo/pipelines/assets/`.
async fn api_list_assets(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Query(query): Query<AssetQuery>,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TemplatesRead,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::GET,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }

    let owner_slug = crate::platform::model::slug_segment(&owner);
    let project_slug = crate::platform::model::slug_segment(&project);
    let layout = match state
        .platform
        .file
        .ensure_project_layout(&owner_slug, &project_slug)
    {
        Ok(l) => l,
        Err(err) => return internal_error(err),
    };

    let subfolder = query
        .subfolder
        .as_deref()
        .and_then(sanitize_subfolder)
        .unwrap_or_default();
    let assets_dir = if subfolder.is_empty() {
        layout.repo_static_dir()
    } else {
        layout.repo_static_dir().join(&subfolder)
    };
    if let Err(e) = std::fs::create_dir_all(&assets_dir) {
        return internal_error(PlatformError::new("ASSET_DIR_CREATE", e.to_string()));
    }

    let entries = match std::fs::read_dir(&assets_dir) {
        Ok(rd) => rd,
        Err(e) => return internal_error(PlatformError::new("ASSET_DIR_READ", e.to_string())),
    };

    let mut files: Vec<Value> = Vec::new();
    for entry in entries.flatten() {
        let meta = match entry.metadata() {
            Ok(m) => m,
            Err(_) => continue,
        };
        if !meta.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().to_string();
        let size_bytes = meta.len();
        let url = if subfolder.is_empty() {
            format!("/assets/{owner_slug}/{project_slug}/{name}")
        } else {
            format!("/assets/{owner_slug}/{project_slug}/{subfolder}/{name}")
        };
        files.push(json!({ "name": name, "size_bytes": size_bytes, "url": url }));
    }

    files.sort_by(|a, b| {
        a["name"]
            .as_str()
            .unwrap_or("")
            .cmp(b["name"].as_str().unwrap_or(""))
    });

    Json(json!({ "ok": true, "files": files })).into_response()
}

/// `POST /api/projects/{owner}/{project}/assets` — upload a file via multipart.
async fn api_upload_asset(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Query(query): Query<AssetQuery>,
    mut multipart: Multipart,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TemplatesWrite,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        let field = match multipart.next_field().await {
            Ok(Some(field)) => field,
            Ok(None) => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({"ok": false, "error": "no file field in multipart body"})),
                )
                    .into_response();
            }
            Err(err) => return internal_error(PlatformError::new("ASSET_UPLOAD", err.to_string())),
        };
        let raw_filename = field
            .file_name()
            .map(|value| value.to_string())
            .unwrap_or_else(|| "upload.bin".to_string());
        let content_type = field.content_type().map(|value| value.to_string());
        let bytes = match field.bytes().await {
            Ok(bytes) => bytes,
            Err(err) => return internal_error(PlatformError::new("ASSET_UPLOAD", err.to_string())),
        };
        let worker = match state.platform.cluster_registry.get_worker(&worker_id) {
            Ok(Some(worker)) => worker,
            Ok(None) => {
                return (
                    StatusCode::BAD_GATEWAY,
                    Json(json!({"ok": false, "error": "office not registered"})),
                )
                    .into_response();
            }
            Err(err) => return internal_error(err),
        };
        let token = match cluster_call_header_for_office(&state, worker_office_id(&worker)) {
            Ok(token) => token,
            Err(err) => return internal_error(err),
        };
        let mut url = format!("{}{}", worker.base_url.trim_end_matches('/'), uri.path());
        if let Some(query) = uri.query() {
            url.push('?');
            url.push_str(query);
        }
        let part = match content_type {
            Some(content_type) => reqwest::multipart::Part::bytes(bytes.to_vec())
                .file_name(raw_filename.clone())
                .mime_str(&content_type)
                .unwrap_or_else(|_| {
                    reqwest::multipart::Part::bytes(bytes.to_vec()).file_name(raw_filename)
                }),
            None => reqwest::multipart::Part::bytes(bytes.to_vec()).file_name(raw_filename),
        };
        let form = reqwest::multipart::Form::new().part("file", part);
        let response = match state
            .http_client
            .post(url)
            .header(INTERNAL_CLUSTER_TOKEN_HEADER, token)
            .multipart(form)
            .send()
            .await
        {
            Ok(response) => response,
            Err(err) => return internal_error(PlatformError::new("ASSET_UPLOAD", err.to_string())),
        };
        return reqwest_response_to_axum(response).await;
    }

    let owner_slug = crate::platform::model::slug_segment(&owner);
    let project_slug = crate::platform::model::slug_segment(&project);
    let layout = match state
        .platform
        .file
        .ensure_project_layout(&owner_slug, &project_slug)
    {
        Ok(l) => l,
        Err(err) => return internal_error(err),
    };

    let cfg = match state
        .platform
        .zebflow_cfg
        .read_or_default(&owner_slug, &project_slug)
    {
        Ok(config) => config,
        Err(err) => return internal_error(err),
    };
    let max_asset_size_mb = cfg.configs.files.uploads.effective_max_asset_size_mb();
    let max_bytes = (max_asset_size_mb as u64) * 1024 * 1024;

    let subfolder = query
        .subfolder
        .as_deref()
        .and_then(sanitize_subfolder)
        .unwrap_or_default();
    let assets_dir = if subfolder.is_empty() {
        layout.repo_static_dir()
    } else {
        layout.repo_static_dir().join(&subfolder)
    };
    if let Err(e) = std::fs::create_dir_all(&assets_dir) {
        return internal_error(PlatformError::new("ASSET_DIR_CREATE", e.to_string()));
    }

    let field = match multipart.next_field().await {
        Ok(Some(f)) => f,
        Ok(None) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok": false, "error": "no file field in multipart body"})),
            )
                .into_response();
        }
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok": false, "error": e.to_string()})),
            )
                .into_response();
        }
    };

    let raw_filename = field
        .file_name()
        .map(|s| s.to_string())
        .or_else(|| field.name().map(|s| s.to_string()))
        .unwrap_or_default();

    let filename = match sanitize_asset_filename(&raw_filename) {
        Some(n) => n,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok": false, "error": format!("invalid filename '{raw_filename}': only [A-Za-z0-9._-] allowed, no leading dot, max 200 chars")})),
            ).into_response();
        }
    };

    let bytes = match field.bytes().await {
        Ok(b) => b,
        Err(e) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok": false, "error": e.to_string()})),
            )
                .into_response();
        }
    };

    if bytes.len() as u64 > max_bytes {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": format!("file size {} bytes exceeds limit of {} MB", bytes.len(), max_asset_size_mb)})),
        ).into_response();
    }

    let dest = assets_dir.join(&filename);
    if let Err(e) = std::fs::write(&dest, &bytes) {
        return internal_error(PlatformError::new("ASSET_WRITE", e.to_string()));
    }

    let url = if subfolder.is_empty() {
        format!("/assets/{owner_slug}/{project_slug}/{filename}")
    } else {
        format!("/assets/{owner_slug}/{project_slug}/{subfolder}/{filename}")
    };
    Json(json!({ "ok": true, "name": filename, "size_bytes": bytes.len(), "url": url }))
        .into_response()
}

/// `DELETE /api/projects/{owner}/{project}/assets/{*path}` — delete an asset file.
async fn api_delete_asset(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project, path)): Path<(String, String, String)>,
    uri: Uri,
) -> Response {
    if let Err(response) = require_project_api_capability(
        &state,
        &headers,
        &owner,
        &project,
        ProjectCapability::TemplatesDelete,
    ) {
        return response;
    }
    if let Ok(Some(worker_id)) = remote_project_worker_id(&state, &owner, &project) {
        return match forward_project_api_request_to_worker(
            &state,
            &uri,
            &Method::DELETE,
            &headers,
            Bytes::new(),
            &worker_id,
        )
        .await
        {
            Ok(response) => response,
            Err(err) => internal_error(err),
        };
    }

    let subpath = match sanitize_asset_subpath(path.trim_start_matches('/')) {
        Some(p) => p,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok": false, "error": "invalid asset path"})),
            )
                .into_response();
        }
    };

    let owner_slug = crate::platform::model::slug_segment(&owner);
    let project_slug = crate::platform::model::slug_segment(&project);
    let layout = match state
        .platform
        .file
        .ensure_project_layout(&owner_slug, &project_slug)
    {
        Ok(l) => l,
        Err(err) => return internal_error(err),
    };

    let assets_dir = layout.repo_static_dir();
    let abs = assets_dir.join(&subpath);

    if !abs.starts_with(&assets_dir) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({"ok": false, "error": "invalid asset path"})),
        )
            .into_response();
    }

    if !abs.is_file() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({"ok": false, "error": "asset not found"})),
        )
            .into_response();
    }

    if let Err(e) = std::fs::remove_file(&abs) {
        return internal_error(PlatformError::new("ASSET_DELETE", e.to_string()));
    }

    Json(json!({ "ok": true })).into_response()
}

// ── Live Preview ──────────────────────────────────────────────────────────────

#[derive(Deserialize)]
struct PreviewQuery {
    file: Option<String>,
}

#[derive(Deserialize, serde::Serialize)]
struct PreviewToggleBody {
    active: bool,
    file: String,
}

fn preview_key(owner: &str, project: &str, file: &str) -> String {
    format!("{owner}/{project}/{file}")
}

/// Sanitize a preview file path: must not escape the templates dir.
fn sanitize_preview_file(raw: &str) -> Option<String> {
    let trimmed = raw.trim().trim_start_matches('/');
    if trimmed.is_empty() || trimmed.contains("..") {
        return None;
    }
    Some(trimmed.to_string())
}

/// `POST /api/projects/{owner}/{project}/preview/toggle`
async fn api_preview_toggle(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Json(body): Json<PreviewToggleBody>,
) -> Response {
    // Being logged in is not the same as being allowed near this project.
    // Asking only `session_owner(...).is_none()` let any account toggle preview
    // on another owner's private file and then read the rendered page, while
    // every other project route refused them.
    let internal_cluster_call = is_controller_call(&state, &headers);
    if !internal_cluster_call {
        if let Err(response) = require_project_api_capability(
            &state,
            &headers,
            &owner,
            &project,
            ProjectCapability::TemplatesWrite,
        ) {
            return response;
        }
    }
    let file = match sanitize_preview_file(&body.file) {
        Some(f) => f,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({"ok": false, "error": "invalid file"})),
            )
                .into_response();
        }
    };
    if !internal_cluster_call {
        match maybe_forward_project_json_to_worker(
            &state,
            &uri,
            &headers,
            Method::POST,
            &body,
            &owner,
            &project,
        )
        .await
        {
            Ok(Some(response)) => return response,
            Ok(None) => {}
            Err(err) => return internal_error(err),
        }
    }
    let key = preview_key(&owner, &project, &file);
    let mut reg = state
        .preview_registry
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if body.active {
        reg.insert(key);
    } else {
        reg.remove(&key);
    }
    Json(json!({ "ok": true, "active": body.active })).into_response()
}

/// `GET /api/projects/{owner}/{project}/preview/status?file=...`
async fn api_preview_status(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Query(q): Query<PreviewQuery>,
) -> Response {
    // Being logged in is not the same as being allowed near this project.
    // Asking only `session_owner(...).is_none()` let any account toggle preview
    // on another owner's private file and then read the rendered page, while
    // every other project route refused them.
    let internal_cluster_call = is_controller_call(&state, &headers);
    if !internal_cluster_call {
        if let Err(response) = require_project_api_capability(
            &state,
            &headers,
            &owner,
            &project,
            ProjectCapability::TemplatesRead,
        ) {
            return response;
        }
    }
    if !internal_cluster_call {
        match maybe_forward_project_api_to_worker(
            &state,
            &uri,
            &Method::GET,
            &headers,
            Bytes::new(),
            &owner,
            &project,
        )
        .await
        {
            Ok(Some(response)) => return response,
            Ok(None) => {}
            Err(err) => return internal_error(err),
        }
    }
    let file = q
        .file
        .as_deref()
        .and_then(sanitize_preview_file)
        .unwrap_or_default();
    let key = preview_key(&owner, &project, &file);
    let active = state
        .preview_registry
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .contains(&key);
    Json(json!({ "active": active })).into_response()
}

/// `GET /preview/{owner}/{project}?file=...`
/// Auth + toggle gated. Compiles the TSX on the fly and returns full HTML with WS reload client.
async fn preview_page(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    uri: Uri,
    Query(q): Query<PreviewQuery>,
) -> Response {
    // Being logged in is not the same as being allowed near this project.
    // Asking only `session_owner(...).is_none()` let any account toggle preview
    // on another owner's private file and then read the rendered page, while
    // every other project route refused them.
    let internal_cluster_call = is_controller_call(&state, &headers);
    if !internal_cluster_call {
        if session_owner(&state, &headers).is_none() {
            // No session at all is a sign-in problem, not a refusal.
            return Redirect::to(LOGIN_PATH).into_response();
        }
        if let Err(response) = require_project_api_capability(
            &state,
            &headers,
            &owner,
            &project,
            ProjectCapability::TemplatesRead,
        ) {
            return response;
        }
    }

    let file = match q.file.as_deref().and_then(sanitize_preview_file) {
        Some(f) => f,
        None => {
            return (
                StatusCode::BAD_REQUEST,
                Html("<p>Missing ?file= param</p>".to_string()),
            )
                .into_response();
        }
    };

    if !internal_cluster_call {
        match maybe_forward_project_page_to_worker(&state, &headers, &uri, &owner, &project).await {
            Ok(Some(response)) => return response,
            Ok(None) => {}
            Err(err) => return internal_error(err),
        }
    }

    let key = preview_key(&owner, &project, &file);
    let active = state
        .preview_registry
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .contains(&key);
    if !active {
        return (
            StatusCode::FORBIDDEN,
            Html("<html><body style='font-family:sans-serif;padding:2rem'><h2>Preview not active</h2><p>Enable Live Preview in the registry editor first.</p></body></html>".to_string()),
        ).into_response();
    }

    let owner_slug = crate::platform::model::slug_segment(&owner);
    let project_slug = crate::platform::model::slug_segment(&project);
    let layout = match state
        .platform
        .file
        .ensure_project_layout(&owner_slug, &project_slug)
    {
        Ok(l) => l,
        Err(err) => return internal_error(err),
    };

    let abs_path = layout.repo_source_dir().join(&file);
    let markup = match fs::read_to_string(&abs_path) {
        Ok(m) => m,
        Err(e) => {
            return (
                StatusCode::NOT_FOUND,
                Html(format!("<p>Cannot read file: {e}</p>")),
            )
                .into_response();
        }
    };

    let options = crate::rwe::ReactiveWebOptions {
        templates: crate::rwe::TemplateOptions {
            template_root: Some(layout.repo_source_dir()),
            library_roots: state.platform.library.source_roots(),
            style_entries: Vec::new(),
        },
        processors: vec!["tailwind".to_string()],
        ..Default::default()
    };

    let compiled = match state.frontend.rwe.compile_template(
        &crate::rwe::TemplateSource {
            id: format!("preview/{owner}/{project}/{file}"),
            source_path: Some(abs_path),
            markup,
        },
        state.frontend.language.as_ref(),
        &options,
    ) {
        Ok(c) => c,
        Err(e) => return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Html(format!("<html><body style='font-family:monospace;padding:1rem;white-space:pre-wrap'><b>Compile error:</b>\n{e}</body></html>")),
        ).into_response(),
    };

    let ctx = crate::rwe::RenderContext {
        route: format!("/preview/{owner}/{project}"),
        request_id: "preview".to_string(),
        metadata: json!({}),
        enabled_libraries: Vec::new(),
    };

    let out = match state.frontend.rwe.render(&compiled, json!({}), state.frontend.language.as_ref(), &ctx) {
        Ok(out) => out,
        Err(e) => return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Html(format!("<html><body style='font-family:monospace;padding:1rem;white-space:pre-wrap'><b>Render error:</b>\n{e}</body></html>")),
        ).into_response(),
    };

    // Build final HTML the same way `render_page_html` does.
    let mut html = ensure_meta_charset(out.html);

    // Inject Tailwind CSS extracted by the RWE engine.
    if let Some(css) = out.hydration_payload.get("css").and_then(Value::as_str) {
        html = crate::rwe::core::render::insert_engine_styles(&html, css);
    }

    // Platform design tokens + reset.
    html = ensure_stylesheet_link(html, "/assets/platform/main.css");

    // Handle compiled JS scripts.
    html = match externalize_rwe_scripts(&state, &html, &out.compiled_scripts, None) {
        Ok(html) => html,
        Err(err) => return internal_error(err),
    };

    // Inject WS live-reload client just before </body>
    let ws_url = format!(
        "/ws/preview/{owner}/{project}?file={}",
        urlencoding_encode(&file)
    );
    let ws_script = format!(
        r#"<script>
(function(){{
  var url = (location.protocol === 'https:' ? 'wss' : 'ws') + '://' + location.host + '{ws_url}';
  function connect() {{
    var ws = new WebSocket(url);
    ws.onmessage = function(e) {{
      try {{ if (JSON.parse(e.data).type === 'reload') location.reload(); }} catch(_) {{}}
    }};
    ws.onclose = function() {{ setTimeout(connect, 2000); }};
  }}
  connect();
}})();
</script></body>"#
    );
    let html = if html.contains("</body>") {
        html.replacen("</body>", &ws_script, 1)
    } else {
        format!("{html}{ws_script}")
    };

    (
        StatusCode::OK,
        [(CONTENT_TYPE, "text/html; charset=utf-8")],
        html,
    )
        .into_response()
}

fn urlencoding_encode(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' | '/' => c.to_string(),
            ' ' => "%20".to_string(),
            c => format!("%{:02X}", c as u32),
        })
        .collect()
}

fn url_query_encode(s: &str) -> String {
    let mut out = String::new();
    for byte in s.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char);
            }
            b' ' => out.push_str("%20"),
            byte => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// `GET /ws/preview/{owner}/{project}?file=...`
/// WebSocket: polls file mtime every second, sends {"type":"reload"} on change.
/// On disconnect: removes the file from the preview registry.
async fn ws_preview_handler(
    State(state): State<PlatformAppState>,
    headers: HeaderMap,
    Path((owner, project)): Path<(String, String)>,
    Query(q): Query<PreviewQuery>,
    ws: WebSocketUpgrade,
    uri: Uri,
) -> Response {
    // Being logged in is not the same as being allowed near this project.
    // Asking only `session_owner(...).is_none()` let any account toggle preview
    // on another owner's private file and then read the rendered page, while
    // every other project route refused them.
    let internal_cluster_call = is_controller_call(&state, &headers);
    if !internal_cluster_call {
        if let Err(response) = require_project_api_capability(
            &state,
            &headers,
            &owner,
            &project,
            ProjectCapability::TemplatesRead,
        ) {
            return response;
        }
    }

    if !internal_cluster_call {
        match remote_project_worker_id(&state, &owner, &project) {
            Ok(Some(worker_id)) => {
                let worker = match state.platform.cluster_registry.get_worker(&worker_id) {
                    Ok(Some(worker)) => worker,
                    Ok(None) => {
                        return (
                            StatusCode::BAD_GATEWAY,
                            Json(json!({"ok": false, "error": "office not registered"})),
                        )
                            .into_response();
                    }
                    Err(err) => return internal_error(err),
                };
                let token = match cluster_call_header_for_office(&state, worker_office_id(&worker))
                {
                    Ok(token) => token,
                    Err(err) => return internal_error(err),
                };
                let worker_url = worker_websocket_url(&worker.base_url, &uri);
                let proxy_headers = vec![(INTERNAL_CLUSTER_TOKEN_HEADER.to_string(), token)];
                return ws
                    .on_upgrade(move |socket| {
                        proxy_websocket_to_worker(socket, worker_url, proxy_headers)
                    })
                    .into_response();
            }
            Ok(None) => {}
            Err(err) => return internal_error(err),
        }
    }

    let file = match q.file.as_deref().and_then(sanitize_preview_file) {
        Some(f) => f,
        None => return (StatusCode::BAD_REQUEST, "missing file param").into_response(),
    };

    let key = preview_key(&owner, &project, &file);
    {
        let reg = state
            .preview_registry
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if !reg.contains(&key) {
            return (StatusCode::FORBIDDEN, "preview not active").into_response();
        }
    }

    let owner_slug = crate::platform::model::slug_segment(&owner);
    let project_slug = crate::platform::model::slug_segment(&project);
    let layout = match state
        .platform
        .file
        .ensure_project_layout(&owner_slug, &project_slug)
    {
        Ok(l) => l,
        Err(err) => return internal_error(err),
    };

    let abs_path = layout.repo_source_dir().join(&file);
    let registry = state.preview_registry.clone();

    ws.on_upgrade(move |socket| async move {
        handle_preview_ws(socket, abs_path, key, registry).await;
    })
}

async fn handle_preview_ws(
    mut socket: WebSocket,
    path: PathBuf,
    _key: String,
    _registry: Arc<std::sync::Mutex<std::collections::HashSet<String>>>,
) {
    let mut last_mtime = get_mtime(&path);
    let mut interval = tokio::time::interval(Duration::from_secs(1));

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let mtime = get_mtime(&path);
                if mtime != last_mtime {
                    last_mtime = mtime;
                    let msg = Message::Text(r#"{"type":"reload"}"#.to_string().into());
                    if socket.send(msg).await.is_err() {
                        break;
                    }
                }
            }
            msg = socket.recv() => {
                match msg {
                    Some(Ok(_)) => {} // ignore client messages
                    _ => break,      // disconnect
                }
            }
        }
    }

    // WS disconnected (tab reload or close) — do NOT remove from registry.
    // Preview toggle state is managed only via POST /preview/toggle.
}

fn get_mtime(path: &FsPath) -> Option<std::time::SystemTime> {
    fs::metadata(path).ok().and_then(|m| m.modified().ok())
}

#[cfg(test)]
mod webhook_sse_tests {
    use axum::http::{HeaderMap, HeaderValue, StatusCode};

    use super::{
        optional_bounded_settings_u64, reject_unknown_settings_fields, wants_event_stream,
    };

    #[test]
    fn project_setting_numbers_are_rejected_instead_of_clamped() {
        let below = optional_bounded_settings_u64(
            &serde_json::json!({"node_timeout_secs": 1}),
            "node_timeout_secs",
            5,
            3600,
        )
        .unwrap_err();
        assert_eq!(below.status(), StatusCode::BAD_REQUEST);

        let wrong_type = optional_bounded_settings_u64(
            &serde_json::json!({"node_timeout_secs": "30"}),
            "node_timeout_secs",
            5,
            3600,
        )
        .unwrap_err();
        assert_eq!(wrong_type.status(), StatusCode::BAD_REQUEST);

        assert_eq!(
            optional_bounded_settings_u64(
                &serde_json::json!({"node_timeout_secs": 30}),
                "node_timeout_secs",
                5,
                3600,
            )
            .unwrap(),
            Some(30)
        );
    }

    #[test]
    fn project_setting_payloads_reject_unknown_fields() {
        assert!(
            reject_unknown_settings_fields(
                &serde_json::json!({"max_invocations": 20}),
                &["max_invocations"],
            )
            .is_ok()
        );
        let response = reject_unknown_settings_fields(
            &serde_json::json!({"max_invocation": 20}),
            &["max_invocations"],
        )
        .unwrap_err();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    // ── wants_event_stream detection ──────────────────────────────────────────

    #[test]
    fn detects_event_stream_accept_header() {
        let mut headers = HeaderMap::new();
        headers.insert("accept", HeaderValue::from_static("text/event-stream"));
        assert!(wants_event_stream(&headers));
    }

    #[test]
    fn detects_event_stream_among_multiple_types() {
        let mut headers = HeaderMap::new();
        headers.insert(
            "accept",
            HeaderValue::from_static("application/json, text/event-stream"),
        );
        assert!(wants_event_stream(&headers));
    }

    #[test]
    fn normal_json_request_is_not_event_stream() {
        let mut headers = HeaderMap::new();
        headers.insert("accept", HeaderValue::from_static("application/json"));
        assert!(!wants_event_stream(&headers));
    }

    #[test]
    fn missing_accept_header_is_not_event_stream() {
        let headers = HeaderMap::new();
        assert!(!wants_event_stream(&headers));
    }

    #[test]
    fn html_accept_is_not_event_stream() {
        let mut headers = HeaderMap::new();
        headers.insert("accept", HeaderValue::from_static("text/html"));
        assert!(!wants_event_stream(&headers));
    }

    // ── Signal SSE serialization ────────────────────────────────────────────

    #[test]
    fn signal_serializes_to_expected_json_shape() {
        use crate::pipeline::model::Signal;

        let sig = Signal {
            kind: "thinking".to_string(),
            message: "LLM reasoning about query".to_string(),
            node_id: "n0".to_string(),
            node_kind: "n.ai.agent".to_string(),
            data: None,
            at: "00:00:03".to_string(),
        };
        let s = serde_json::to_string(&sig).unwrap();
        assert!(s.contains("\"kind\":\"thinking\""));
        assert!(s.contains("\"message\":\"LLM reasoning about query\""));
        assert!(s.contains("\"node_id\":\"n0\""));
        assert!(s.contains("\"at\":\"00:00:03\""));
    }

    #[test]
    fn done_event_envelope_shape() {
        let output_value = serde_json::json!({"rows": [1, 2, 3]});
        let done = serde_json::json!({ "ok": true, "value": output_value });
        assert_eq!(done["ok"], true);
        assert_eq!(done["value"]["rows"][0], 1);
    }

    #[test]
    fn error_event_envelope_shape() {
        let error = serde_json::json!({
            "ok": false,
            "error": { "code": "PG_QUERY_FAIL", "message": "connection refused" }
        });
        assert_eq!(error["ok"], false);
        assert_eq!(error["error"]["code"], "PG_QUERY_FAIL");
        assert_eq!(error["error"]["message"], "connection refused");
    }

    // ── SSE stream integration (channel-level, no HTTP server) ────────────────

    #[tokio::test]
    async fn signals_forwarded_then_done() {
        use crate::pipeline::model::{ExecutionBus, Signal};
        use futures::StreamExt;

        let bus = std::sync::Arc::new(ExecutionBus::new(64));
        let mut signal_rx = bus.subscribe();
        let (result_tx, mut result_rx) =
            tokio::sync::oneshot::channel::<Result<serde_json::Value, String>>();

        // Simulate: 2 signals, then result.
        bus.emit(Signal {
            kind: "query".to_string(),
            message: "running SQL".to_string(),
            node_id: "n0".to_string(),
            node_kind: "n.db.query".to_string(),
            data: None,
            at: "00:00:01".to_string(),
        });
        bus.emit(Signal {
            kind: "render".to_string(),
            message: "rendering template".to_string(),
            node_id: "n1".to_string(),
            node_kind: "n.web.response".to_string(),
            data: None,
            at: "00:00:02".to_string(),
        });
        drop(bus); // no more senders
        let _ = result_tx.send(Ok(serde_json::json!({"html": "<h1>hi</h1>"})));

        // Build the stream exactly as the production SSE code does.
        let stream = async_stream::stream! {
            loop {
                tokio::select! {
                    biased;
                    sig = signal_rx.recv() => {
                        match sig {
                            Ok(s) => {
                                yield ("signal", serde_json::to_value(&s).unwrap());
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                            Err(_) => break,
                        }
                    }
                    result = &mut result_rx => {
                        match result {
                            Ok(Ok(val)) => yield ("done", serde_json::json!({"ok": true, "value": val})),
                            Ok(Err(msg)) => yield ("error", serde_json::json!({"ok": false, "error": msg})),
                            Err(_) => {}
                        }
                        return;
                    }
                }
            }
            match result_rx.await {
                Ok(Ok(val)) => yield ("done", serde_json::json!({"ok": true, "value": val})),
                Ok(Err(msg)) => yield ("error", serde_json::json!({"ok": false, "error": msg})),
                Err(_) => {}
            }
        };

        tokio::pin!(stream);
        let mut collected = Vec::new();
        while let Some(item) = stream.next().await {
            collected.push(item);
        }

        assert_eq!(collected.len(), 3);
        assert_eq!(collected[0].0, "signal");
        assert_eq!(collected[0].1["kind"], "query");
        assert_eq!(collected[1].0, "signal");
        assert_eq!(collected[1].1["kind"], "render");
        assert_eq!(collected[2].0, "done");
        assert_eq!(collected[2].1["ok"], true);
        assert_eq!(collected[2].1["value"]["html"], "<h1>hi</h1>");
    }

    #[tokio::test]
    async fn error_result_emits_error_event() {
        use crate::pipeline::model::ExecutionBus;
        use futures::StreamExt;

        let bus = std::sync::Arc::new(ExecutionBus::new(64));
        let mut signal_rx = bus.subscribe();
        let (result_tx, mut result_rx) =
            tokio::sync::oneshot::channel::<Result<serde_json::Value, String>>();

        drop(bus);
        let _ = result_tx.send(Err("connection refused".to_string()));

        let stream = async_stream::stream! {
            loop {
                tokio::select! {
                    biased;
                    sig = signal_rx.recv() => {
                        match sig {
                            Ok(s) => {
                                yield ("signal", serde_json::to_value(&s).unwrap());
                            }
                            Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                            Err(_) => break,
                        }
                    }
                    result = &mut result_rx => {
                        match result {
                            Ok(Ok(val)) => yield ("done", serde_json::json!({"ok": true, "value": val})),
                            Ok(Err(msg)) => yield ("error", serde_json::json!({"ok": false, "error": msg})),
                            Err(_) => {}
                        }
                        return;
                    }
                }
            }
            match result_rx.await {
                Ok(Ok(val)) => yield ("done", serde_json::json!({"ok": true, "value": val})),
                Ok(Err(msg)) => yield ("error", serde_json::json!({"ok": false, "error": msg})),
                Err(_) => {}
            }
        };

        tokio::pin!(stream);
        let mut collected = Vec::new();
        while let Some(item) = stream.next().await {
            collected.push(item);
        }

        assert_eq!(collected.len(), 1);
        assert_eq!(collected[0].0, "error");
        assert_eq!(collected[0].1["ok"], false);
        assert!(
            collected[0].1["error"]
                .as_str()
                .unwrap()
                .contains("connection refused")
        );
    }
}

#[cfg(test)]
mod response_header_tests {
    use axum::response::IntoResponse;

    use super::apply_zf_extras;

    /// A pipeline that declares `content-type` means that content type, not a
    /// second opinion alongside the one the body picked.
    #[test]
    fn a_declared_header_replaces_the_one_the_body_chose() {
        let mut resp = (
            axum::http::StatusCode::OK,
            "<svg xmlns=\"http://www.w3.org/2000/svg\"/>".to_string(),
        )
            .into_response();
        // What axum picks for a String body.
        assert_eq!(resp.headers().get("content-type").unwrap(), "text/plain; charset=utf-8");

        apply_zf_extras(
            &mut resp,
            &None,
            &[("content-type".to_string(), "image/svg+xml".to_string())],
        );

        let seen: Vec<_> = resp.headers().get_all("content-type").iter().collect();
        assert_eq!(seen.len(), 1, "one content-type, not two: {seen:?}");
        assert_eq!(seen[0], "image/svg+xml");
    }

    /// Repetition is how HTTP sends more than one cookie, so these still stack.
    #[test]
    fn every_declared_cookie_survives() {
        let mut resp = (axum::http::StatusCode::OK, "ok".to_string()).into_response();
        apply_zf_extras(
            &mut resp,
            &Some("session=abc; Path=/".to_string()),
            &[("set-cookie".to_string(), "theme=dark; Path=/".to_string())],
        );

        let seen: Vec<_> = resp
            .headers()
            .get_all("set-cookie")
            .iter()
            .map(|v| v.to_str().unwrap().to_string())
            .collect();
        assert_eq!(seen.len(), 2, "both cookies reach the browser: {seen:?}");
        assert!(seen.iter().any(|c| c.starts_with("session=")));
        assert!(seen.iter().any(|c| c.starts_with("theme=")));
    }
}

#[cfg(test)]
mod offline_tests {
    /// Nothing the Studio serves may send the browser to another host.
    ///
    /// Zebflow is meant to run on a machine with no route to the internet — a
    /// Raspberry Pi on a bench, an air-gapped server. Every remote asset breaks
    /// that quietly: the page still answers 200, the browser waits, and the
    /// user gets a Studio drawn in fallback fonts with no database logos.
    ///
    /// Three were found this way. `styles/main.css` opened with an @import of
    /// fonts.googleapis.com, on every page, blocking first paint. `devicons.css`
    /// looked vendored and was a 942-byte file whose second line @imported
    /// jsdelivr. Both are now local.
    ///
    /// Comments are stripped before scanning, so a URL may still be *named* in
    /// prose explaining why it is gone.
    #[test]
    fn no_platform_stylesheet_sends_the_browser_to_another_host() {
        let sheets = [
            ("styles/main.css", include_str!("templates/styles/main.css")),
            ("styles/fonts.css", include_str!("templates/styles/fonts.css")),
            ("styles/db-suite.css", include_str!("templates/styles/db-suite.css")),
            (
                "styles/db-connections.css",
                include_str!("templates/styles/db-connections.css"),
            ),
            (
                "project-studio/styles.css",
                include_str!("templates/pages/project-studio/styles.css"),
            ),
        ];
        for (name, css) in sheets {
            let code = strip_css_comments(css);
            let remote: Vec<_> = code
                .match_indices("http")
                .map(|(i, _)| {
                    code[i..].chars().take(60).collect::<String>()
                })
                .filter(|s| s.starts_with("http://") || s.starts_with("https://"))
                .collect();
            assert!(
                remote.is_empty(),
                "{name} points the browser at another host: {remote:?}"
            );
        }
    }

    /// The font faces the Studio asks for must all be on this machine.
    #[test]
    fn every_font_face_names_a_file_that_ships() {
        let css = include_str!("templates/styles/fonts.css");
        let mut named = 0;
        for (index, _) in css.match_indices("url(/assets/platform/fonts/") {
            let rest = &css[index + "url(/assets/platform/fonts/".len()..];
            let file: String = rest.chars().take_while(|c| *c != ')').collect();
            let path = format!("{}/templates/styles/fonts/{file}", env!("CARGO_MANIFEST_DIR"));
            let path = path.replace("/templates/", "/src/platform/web/templates/");
            assert!(
                std::path::Path::new(&path).is_file(),
                "fonts.css names '{file}', which is not in the package"
            );
            named += 1;
        }
        assert!(named >= 13, "only {named} font files referenced — the scan stopped matching");
    }

    fn strip_css_comments(css: &str) -> String {
        let mut out = String::with_capacity(css.len());
        let bytes = css.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i..].starts_with(b"/*") {
                match css[i + 2..].find("*/") {
                    Some(end) => i += 2 + end + 2,
                    None => break,
                }
            } else {
                out.push(bytes[i] as char);
                i += 1;
            }
        }
        out
    }
}
