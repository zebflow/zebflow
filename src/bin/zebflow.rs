//! Unified Zebflow binary, built twice as `zeb` and `zebflow`.
//!
//! Current behavior:
//!
//! - `zeb` or `zeb standalone` starts the current all-in-one server
//! - `zeb controller` starts the control-plane oriented server
//! - `zeb office` starts the execution-plane oriented server
//! - `zeb run <owner>/<project>` serves one installed project's public route
//! - `zeb install|remove|list|status|login|use|logout` talk to a running
//!   instance over its HTTP API (`platform::cli`)
//! - `zeb k8s cluster ...` manages file-based Kubernetes manifest folders
//!
//! `master` and `worker` are deprecated spellings of `controller` and `office`;
//! they still resolve to the same roles and warn.
//!
//! Nothing here writes the program's own name: `distribution.md` §Binary name
//! ships two, so every message reads argv[0] through `cli::program()`.
//!
//! The goal is one binary that still runs comfortably on a laptop or Raspberry Pi while also
//! growing into controller/office and Kubernetes deployments.

use std::io;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;

use zebflow::infra::cluster::config::{ClusterRole, ClusterSettings};
use zebflow::infra::health::{
    HealthState, spawn_main_runtime_heartbeat, start_dedicated_health_server,
};
use zebflow::platform::cli;
use zebflow::platform::services::PlatformService;
use zebflow::platform::services::project::webhook_triggers_from_source;
use zebflow::platform::services::{
    DependencyLockService, LibraryService, ProjectConfigurationService, ProjectService,
};
use zebflow::platform::web;
use zebflow::platform::{DataAdapterKind, FileAdapterKind, PlatformConfig, build_router};
use zebflow::provision::k8s as k8s_provision;
use zebflow::version::APP_VERSION;

/// Resolves when SIGTERM or Ctrl-C arrives, allowing axum's graceful shutdown
/// to drain in-flight requests.
async fn shutdown_signal(health_state: Option<Arc<HealthState>>) {
    use tokio::signal;

    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    if let Some(state) = health_state {
        state.mark_shutdown_requested();
    }
    eprintln!("Zebflow: graceful shutdown initiated; draining in-flight requests...");
}

fn configured_host() -> String {
    std::env::var("ZEBFLOW_PLATFORM_HOST").unwrap_or_else(|_| "127.0.0.1".to_string())
}

fn configured_port() -> u16 {
    std::env::var("ZEBFLOW_PLATFORM_PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(10610)
}

fn configured_health_addr(default_host: &str) -> Result<Option<SocketAddr>, io::Error> {
    let Some(raw_port) = std::env::var("ZEBFLOW_HEALTH_PORT").ok() else {
        return Ok(None);
    };
    let trimmed = raw_port.trim();
    if trimmed.is_empty() || trimmed == "0" {
        return Ok(None);
    }
    let port = trimmed.parse::<u16>().map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("invalid ZEBFLOW_HEALTH_PORT '{trimmed}': {err}"),
        )
    })?;
    let host = std::env::var("ZEBFLOW_HEALTH_HOST").unwrap_or_else(|_| default_host.to_string());
    let addr = format!("{host}:{port}")
        .parse::<SocketAddr>()
        .map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("invalid health listen address {host}:{port}: {err}"),
            )
        })?;
    Ok(Some(addr))
}

fn maybe_start_dedicated_health_server(
    host: &str,
) -> Result<Option<(Arc<HealthState>, std::thread::JoinHandle<()>, SocketAddr)>, io::Error> {
    let Some(addr) = configured_health_addr(host)? else {
        return Ok(None);
    };
    let state = HealthState::new();
    let handle = start_dedicated_health_server(addr, state.clone())?;
    spawn_main_runtime_heartbeat(state.clone());
    Ok(Some((state, handle, addr)))
}

fn default_advertise_url(host: &str, port: u16) -> String {
    let host = if host == "0.0.0.0" { "127.0.0.1" } else { host };
    format!("http://{host}:{port}")
}

fn display_mode(role: ClusterRole) -> &'static str {
    match role {
        ClusterRole::Standalone => "standalone (controller + office)",
        ClusterRole::Master => "controller",
        ClusterRole::Worker => "office",
    }
}

fn top_level_help() -> String {
    format!(
        "Zebflow {version}

Usage:
  {zeb} [standalone]
  {zeb} install <ref> [--repo <repository-id>] [--yes]
  {zeb} remove <owner>/<project>
  {zeb} list
  {zeb} status
  {zeb} login <instance-url> [--user <name>] [--password <pw>]
  {zeb} use <owner>/<project>
  {zeb} logout
  {zeb} project install <ref>
  {zeb} project remove <owner>/<project>
  {zeb} project list
  {zeb} run <owner>/<project>
  {zeb} project config migrate <owner> <project>
  {zeb} project lock migrate <owner> <project>
  {zeb} project pipelines migrate <owner> <project>
  {zeb} controller
  {zeb} office
  {zeb} k8s cluster <command> ...
  {zeb} help
  {zeb} --help
  {zeb} --version

Runtime Modes:
  standalone   Start the combined controller + office server (default)
  run          Serve one installed project's public route (install it first)
  controller   Start the control-plane oriented server
  office       Start the execution-plane oriented server

  `master` and `worker` are deprecated spellings of `controller` and `office`.
  They still start the same role, print a warning, and will be removed.

{client_help}

Project Maintenance (offline: these run against a data directory with no server,
because they are what you run when the server will not start):
  {zeb} project config migrate <owner> <project>
               Explicitly migrate repo/zebflow.json to repo/zebflow.yaml
  {zeb} project lock migrate <owner> <project>
               Explicitly migrate the pre-v1 repo/zeb.lock format
  {zeb} project pipelines migrate <owner> <project>
               Rewrite persisted pipeline ids to the source-relative form

Kubernetes:
  {zeb} k8s cluster init <path>
  {zeb} k8s cluster add-office <path> <office-id>
  {zeb} k8s cluster set-controller <path> <office-id>
  {zeb} k8s cluster set-namespace <path> <namespace>
  {zeb} k8s cluster set-resource-suffix <path> <suffix>
  {zeb} k8s cluster set-image <path> <image>
  {zeb} k8s cluster use-secret <path> <secret-name>
  {zeb} k8s cluster set-replicas <path> <replicas>
  {zeb} k8s cluster enable-precreate-pvcs <path>
  {zeb} k8s cluster disable-precreate-pvcs <path>
  {zeb} k8s cluster enable-auto-update <path>
  {zeb} k8s cluster disable-auto-update <path>
  {zeb} k8s cluster render-copy-jobs <source-path> <target-path> <output-file>
  {zeb} k8s cluster describe <path>
  {zeb} k8s cluster validate <path>

Environment - process shape (where this process listens and stores):
  ZEBFLOW_PLATFORM_HOST              Listen host (default: 127.0.0.1)
  ZEBFLOW_PLATFORM_PORT              Listen port (default: 10610)
  ZEBFLOW_PLATFORM_DATA_DIR          Data root override
  ZEBFLOW_PLATFORM_BASE_URL          External base URL used for OAuth redirect and MCP session
                                     URLs (default: derived from the request headers)
  ZEBFLOW_HEALTH_PORT                Optional dedicated liveness port, e.g. 10611
  ZEBFLOW_HEALTH_HOST                Dedicated liveness host (default: ZEBFLOW_PLATFORM_HOST)

Environment - first-boot bootstrap (server state created on first start; not CLI context):
  ZEBFLOW_PLATFORM_DEFAULT_OWNER     Owner account created on first boot (default: superadmin)
  ZEBFLOW_PLATFORM_DEFAULT_PROJECT   Project created for that owner on first boot
                                     (default: default)
  ZEBFLOW_PLATFORM_DEFAULT_PASSWORD  Initial password for that owner; when unset a random one is
                                     generated into <data-dir>/.bootstrap/superadmin-password
  ZEBFLOW_PLATFORM_ALLOW_INSECURE_DEFAULT_PASSWORD
                                     Set to 1 to permit the literal password 'secret'
                                     (disposable local development only)

Environment - cluster membership (controller and office):
  ZEBFLOW_CLUSTER_JOIN_TOKEN         Shared internal cluster token; required by controller
                                     and office
  ZEBFLOW_CLUSTER_MASTER_URL         Controller base URL an office registers with; required
                                     by office
  ZEBFLOW_CLUSTER_ADVERTISE_URL      Base URL this node advertises to the control plane
                                     (default: this process's own listen URL)
  ZEBFLOW_CLUSTER_NODE_ID            Stable node id (default: the role name)
  ZEBFLOW_CLUSTER_NODE_LABEL         Human-readable node label (default: the node id)
  A controller or office missing any variable marked required above refuses to start and
  names every missing one at once.

Environment - sessions and tokens:
  ZEBFLOW_COOKIE_SECURE              Force the Secure attribute on session cookies
                                     (default: on unless the listen host is loopback)
  ZEBFLOW_SECRET_ROTATION_EPOCH      Unix timestamp; invalidate older platform-issued tokens

Environment - hub:
  ZEBFLOW_HUB_DEFAULT_BASE_URL       Default platform hub API URL
                                     (default: https://hub.zebflow.com/api)
  ZEBFLOW_HUB_ALLOW_LOCALHOST_REMOTE Set to 1 to allow localhost hub remotes (local development
                                     only; production must leave this unset)

Environment - rendering engine (advanced):
  ZEBFLOW_PLATFORM_RWE_ENGINE_ID     Reactive web engine id for the platform admin UI
  ZEBFLOW_RWE_ENGINE_ID              Reactive web engine id for project pipeline rendering
  ZEBFLOW_RWE_PREWARM                Set to 0 to disable post-compile SSR warmup

Use `{zeb} k8s --help` for the file-based Kubernetes cluster manager.",
        zeb = cli::program(),
        version = APP_VERSION,
        client_help = cli::help_section()
    )
}

fn print_top_level_help() {
    println!("{}", top_level_help());
}

fn print_version() {
    println!("{APP_VERSION}");
}

/// Routes the `project` noun group.
///
/// `install` and `list` reach a running instance over HTTP; `config`, `lock`
/// and `pipelines` run offline against a data directory, because they are what
/// you run when the server will not start. `interface.md` §6 draws that line.
async fn project_command(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    match args.first().map(String::as_str) {
        Some("install") => cli::run_install(&args[1..]).await.map_err(Into::into),
        Some("remove") => cli::run_remove(&args[1..]).await.map_err(Into::into),
        Some("list") => cli::run_list(&args[1..]).await.map_err(Into::into),
        Some("config") | Some("lock") | Some("pipelines") => project_maintenance(args),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "usage: {zeb} project <install|remove|list> ...\n       \
                 {zeb} project <config|lock|pipelines> migrate <owner> <project>",
                zeb = cli::program()
            ),
        )
        .into()),
    }
}

fn project_maintenance(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() != 4 || args[1] != "migrate" {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "usage: {} project <config|lock|pipelines> migrate <owner> <project>",
                cli::program()
            ),
        )
        .into());
    }
    let mut data_root = PlatformConfig::default().data_root;
    if let Ok(path) = std::env::var("ZEBFLOW_PLATFORM_DATA_DIR") {
        data_root = path.into();
    }
    match args[0].as_str() {
        "config" => {
            let service = ProjectConfigurationService::new(data_root.join("users"));
            let result = service.migrate_legacy_json(&args[2], &args[3])?;
            println!("Project configuration migrated ({})", result.source_format);
            println!("Canonical: {}", result.canonical_path.display());
            println!("Recovery: {}", result.recovery_path.display());
            Ok(())
        }
        "lock" => {
            let library = Arc::new(LibraryService::from_embedded()?);
            let service =
                DependencyLockService::with_library_service(data_root.join("users"), library);
            let result = service.migrate_legacy(&args[2], &args[3])?;
            println!("Dependency lock migrated ({})", result.source_format);
            println!("Canonical: {}", result.canonical_path.display());
            println!("Recovery: {}", result.recovery_path.display());
            Ok(())
        }
        "pipelines" => {
            let result = pipeline_identity_service(&data_root)?
                .migrate_pipeline_identity(&args[2], &args[3])?;
            println!(
                "Pipeline identity migrated ({} converted, {} already source-relative)",
                result.rewrites.len(),
                result.already_canonical
            );
            for rewrite in &result.rewrites {
                println!("  {} -> {}", rewrite.from, rewrite.to);
            }
            if !result.bootstrap_activate.is_empty() {
                println!("spec.bootstrap.activate: {:?}", result.bootstrap_activate);
            }
            match result.recovery_path {
                Some(path) => println!("Recovery: {}", path.display()),
                None => println!("Nothing to convert; no recovery copy written."),
            }
            Ok(())
        }
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "usage: {} project <config|lock|pipelines> migrate <owner> <project>",
                cli::program()
            ),
        )
        .into()),
    }
}

/// Builds only the services the pipeline identity migration needs.
///
/// The migration touches the platform catalog and `zebflow.yaml`, so it needs
/// a real data adapter, but it must not start a server or run bootstrap.
fn pipeline_identity_service(
    data_root: &Path,
) -> Result<ProjectService, Box<dyn std::error::Error>> {
    let users_root = data_root.join("users");
    let data = zebflow::platform::adapters::data::build_data_adapter(
        PlatformConfig::default().data_adapter,
        data_root,
    )?;
    let configs = Arc::new(ProjectConfigurationService::new(users_root.clone()));
    let file = zebflow::platform::adapters::file::build_file_adapter(
        FileAdapterKind::Filesystem,
        data_root.to_path_buf(),
        configs.clone(),
    );
    file.initialize()?;
    Ok(ProjectService::new(
        data,
        file,
        zebflow::platform::adapters::project_data::build_project_data_factory(data_root),
        configs,
        Arc::new(DependencyLockService::new(users_root)),
    ))
}

/// Load the platform configuration for the requested runtime role from environment variables.
fn load_platform_config_with_default_password(
    role: ClusterRole,
    default_password_fallback: Option<&str>,
) -> Result<PlatformConfig, io::Error> {
    let mut config = PlatformConfig::default();
    let host = configured_host();
    let port = configured_port();

    if let Ok(path) = std::env::var("ZEBFLOW_PLATFORM_DATA_DIR") {
        config.data_root = path.into();
    }
    if let Ok(owner) = std::env::var("ZEBFLOW_PLATFORM_DEFAULT_OWNER") {
        config.default_owner = owner;
    }
    config.default_password = std::env::var("ZEBFLOW_PLATFORM_DEFAULT_PASSWORD")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| default_password_fallback.map(ToString::to_string))
        .unwrap_or_default();
    if let Ok(project) = std::env::var("ZEBFLOW_PLATFORM_DEFAULT_PROJECT") {
        config.default_project = project;
    }
    if let Ok(value) = std::env::var("ZEBFLOW_SECRET_ROTATION_EPOCH") {
        config.secret_rotation_epoch = value.trim().parse::<i64>().map_err(|err| {
            io::Error::other(format!(
                "invalid ZEBFLOW_SECRET_ROTATION_EPOCH '{}': {err}",
                value.trim()
            ))
        })?;
    }
    config.cluster = ClusterSettings::from_env(role, &default_advertise_url(&host, port));
    // Refused here as well as in `PlatformService::from_config`, so `zeb office`
    // names every missing variable before it opens the data root.
    config
        .cluster
        .validate()
        .map_err(|err| io::Error::other(err.to_string()))?;

    config.data_adapter = DataAdapterKind::Sqlite;
    config.file_adapter = FileAdapterKind::Filesystem;

    let allow_insecure_default = std::env::var("ZEBFLOW_PLATFORM_ALLOW_INSECURE_DEFAULT_PASSWORD")
        .ok()
        .map(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false);
    if role != ClusterRole::Worker
        && config.default_password.trim() == "secret"
        && !allow_insecure_default
    {
        return Err(io::Error::other(
            "refusing insecure ZEBFLOW_PLATFORM_DEFAULT_PASSWORD=secret; choose a strong password or set ZEBFLOW_PLATFORM_ALLOW_INSECURE_DEFAULT_PASSWORD=1 only for disposable local development",
        ));
    }

    Ok(config)
}

fn load_platform_config(role: ClusterRole) -> Result<PlatformConfig, io::Error> {
    load_platform_config_with_default_password(role, None)
}

/// Run the requested Zebflow server role.
async fn run_server(role: ClusterRole) -> Result<(), Box<dyn std::error::Error>> {
    let host = configured_host();
    let port = configured_port();
    let health = maybe_start_dedicated_health_server(&host)?;

    let config = load_platform_config(role)?;
    let app = build_router(config).await.map_err(io::Error::other)?;

    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    println!("Zebflow v{APP_VERSION} listening on http://{addr}");
    if let Some((_, _, health_addr)) = &health {
        println!("Dedicated liveness: http://{health_addr}/health/runtime");
    }
    println!("Mode: {}", display_mode(role));
    println!("Flow: /login -> /home -> /projects/{{owner}}/{{project}}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(
            health.as_ref().map(|(state, _, _)| state.clone()),
        ))
        .await?;

    eprintln!("Zebflow: shutdown complete.");
    Ok(())
}

/// `zeb run <owner>/<project>` — which project to serve, and nothing about
/// where it comes from.
///
/// `run` used to accept a hub asset URL and materialise it, which was a second
/// materialisation path that skipped the review every other channel runs
/// (`distribution.md` §3). It serves only what is already installed now, so
/// `install` is the one way a project comes into existence.
#[derive(Debug, Clone)]
struct RunRequest {
    owner: Option<String>,
    project: String,
}

fn parse_run_request(args: &[String]) -> Result<RunRequest, io::Error> {
    let Some(target) = args.first() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "usage: {} run <owner>/<project>\n\nA project has to be installed before it can \
                 be served: `{} install <ref>`.",
                cli::program(),
                cli::program()
            ),
        ));
    };
    let mut owner_flag = None;
    let mut project_flag = None;
    let mut index = 1;
    while index < args.len() {
        match args[index].as_str() {
            "--owner" => {
                index += 1;
                let Some(value) = args.get(index) else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "missing value for --owner",
                    ));
                };
                owner_flag = Some(value.clone());
            }
            "--project" => {
                index += 1;
                let Some(value) = args.get(index) else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "missing value for --project",
                    ));
                };
                project_flag = Some(value.clone());
            }
            other => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unknown run flag '{other}'"),
                ));
            }
        }
        index += 1;
    }
    // `<owner>/<project>` is the same reference shape `use` and `remove` take.
    // The flags predate it and still work, but a flag that disagrees with the
    // reference is refused rather than ranked: no reading of
    // `zeb run a/b --owner c` is obviously the right one.
    let (reference_owner, project) = match target.split_once('/') {
        Some((left, right)) => (Some(left.trim().to_string()), right.trim().to_string()),
        None => (None, target.trim().to_string()),
    };
    if project.is_empty() || project.contains('/') {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("'{target}' is not <owner>/<project>"),
        ));
    }
    if let Some(flag) = owner_flag.as_deref()
        && reference_owner
            .as_deref()
            .is_some_and(|named| named != flag.trim())
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("'{target}' and --owner name different owners; give the owner once"),
        ));
    }
    if let Some(flag) = project_flag.as_deref()
        && flag.trim() != project
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("'{target}' and --project name different projects; give it once"),
        ));
    }

    Ok(RunRequest {
        owner: reference_owner.or(owner_flag),
        project,
    })
}

fn choose_public_app_path(
    platform: &PlatformService,
    owner: &str,
    project: &str,
) -> Result<String, io::Error> {
    let metas = platform
        .projects
        .list_pipeline_meta_rows(owner, project)
        .map_err(|err| io::Error::other(err.to_string()))?;
    let mut preferred_root = None::<String>;
    let mut preferred_get = None::<String>;
    let mut fallback = None::<String>;

    for meta in metas {
        let source = platform
            .projects
            .read_pipeline_source(owner, project, &meta.file_rel_path)
            .map_err(|err| io::Error::other(err.to_string()))?;
        if let Some(triggers) = webhook_triggers_from_source(&source) {
            for trigger in triggers {
                if trigger.method == "GET" && trigger.path == "/" {
                    preferred_root = Some(trigger.path);
                    break;
                }
                if preferred_get.is_none() && trigger.method == "GET" {
                    preferred_get = Some(trigger.path.clone());
                }
                if fallback.is_none() {
                    fallback = Some(trigger.path.clone());
                }
            }
        }
        if preferred_root.is_some() {
            break;
        }
    }

    preferred_root
        .or(preferred_get)
        .or(fallback)
        .ok_or_else(|| io::Error::other("no webhook-triggered public route found for project"))
}

async fn run_project(req: RunRequest) -> Result<(), Box<dyn std::error::Error>> {
    let config = load_platform_config(ClusterRole::Standalone)?;
    let host = configured_host();
    let port = configured_port();
    let health = maybe_start_dedicated_health_server(&host)?;

    // No fallback to ZEBFLOW_PLATFORM_DEFAULT_OWNER: §3a of `interface.md` says
    // that variable names what the server bootstraps, not who a command acts
    // for, and nothing resolves an owner from it.
    let Some(owner) = req.owner.clone() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "run needs an owner; write `{zeb} run <owner>/{project}` or pass `--owner <name>`",
                zeb = cli::program(),
                project = req.project
            ),
        )
        .into());
    };
    let project = req.project.clone();

    let platform = Arc::new(PlatformService::from_config(config)?);
    let exists = platform
        .projects
        .list_projects(&owner)?
        .into_iter()
        .any(|item| item.project == project);
    if !exists {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "project '{owner}/{project}' is not installed in this data root; install it \
                 first with `{} install <ref>`",
                cli::program()
            ),
        )
        .into());
    }
    platform
        .cluster_runtime_sync
        .refresh_local_repo_state(&owner, &project)?;

    let public_path = choose_public_app_path(platform.as_ref(), &owner, &project)?;
    let app_url = format!(
        "http://{}:{}/wh/{owner}/{project}{}",
        if host == "0.0.0.0" {
            "127.0.0.1"
        } else {
            host.as_str()
        },
        port,
        public_path
    );

    let app = web::router(platform).await;
    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    println!("Zebflow app running for {owner}/{project}");
    if let Some((_, _, health_addr)) = &health {
        println!("Dedicated liveness: http://{health_addr}/health/runtime");
    }
    println!("Public route: {app_url}");
    println!("Mode: standalone app runtime");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(
            health.as_ref().map(|(state, _, _)| state.clone()),
        ))
        .await?;

    eprintln!("Zebflow: shutdown complete.");
    Ok(())
}

#[tokio::main]
pub async fn main() {
    if let Err(err) = run().await {
        // Displayed, not debugged: returning the error from `main` prints its
        // `Debug` form, which turns a refusal a person is meant to act on into
        // `Custom { kind: NotFound, error: "..." }`.
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut argv = std::env::args();
    // Recorded before anything can print: `zeb` and `zebflow` are one binary,
    // and every usage line and hint has to name the one that was typed.
    cli::set_program_name(argv.next().as_deref());
    let mut args = argv;
    let mode = args.next();

    match mode.as_deref() {
        None => run_server(ClusterRole::Standalone).await,
        Some("run") => run_project(parse_run_request(&args.collect::<Vec<_>>())?).await,
        Some("project") => project_command(&args.collect::<Vec<_>>()).await,
        // Group 2 aliases, each naming exactly one canonical command
        // (interface.md §4). `install` never means the project-scope verb,
        // whatever context is stored (§7).
        Some("install") => cli::run_install(&args.collect::<Vec<_>>())
            .await
            .map_err(Into::into),
        Some("remove") => cli::run_remove(&args.collect::<Vec<_>>())
            .await
            .map_err(Into::into),
        Some("list") => cli::run_list(&args.collect::<Vec<_>>())
            .await
            .map_err(Into::into),
        Some("status") => cli::run_status(&args.collect::<Vec<_>>())
            .await
            .map_err(Into::into),
        Some("login") => cli::run_login(&args.collect::<Vec<_>>())
            .await
            .map_err(Into::into),
        Some("use") => cli::run_use(&args.collect::<Vec<_>>()).map_err(Into::into),
        Some("logout") => cli::run_logout(&args.collect::<Vec<_>>()).map_err(Into::into),
        Some("help") | Some("--help") | Some("-h") => {
            print_top_level_help();
            Ok(())
        }
        Some("version") | Some("--version") | Some("-V") => {
            print_version();
            Ok(())
        }
        Some("k8s") => k8s_provision::run_cli(&args.collect::<Vec<_>>()),
        Some(other) => match ClusterRole::from_mode_arg(other) {
            Some(mode) => {
                if let Some(canonical) = mode.deprecated_alias_of {
                    eprintln!(
                        "Zebflow: `{zeb} {other}` is a deprecated spelling of `{zeb} {canonical}` and will be removed; use `{zeb} {canonical}`.",
                        zeb = cli::program()
                    );
                }
                run_server(mode.role).await
            }
            None => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!(
                    "unknown {} mode '{other}'\n\n{}",
                    cli::program(),
                    top_level_help()
                ),
            )
            .into()),
        },
    }
}
