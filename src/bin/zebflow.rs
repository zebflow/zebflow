//! Unified Zebflow binary.
//!
//! Current behavior:
//!
//! - `zebflow` or `zebflow standalone` starts the current all-in-one server
//! - `zebflow master` or `zebflow controller` starts the control-plane oriented server
//! - `zebflow worker` or `zebflow office` starts the execution-plane oriented server
//! - `zebflow k8s cluster ...` manages file-based Kubernetes manifest folders
//!
//! The goal is one binary that still runs comfortably on a laptop or Raspberry Pi while also
//! growing into controller/office and Kubernetes deployments.

use std::io;
use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

use base64::Engine as _;
use reqwest::Url;
use serde_json::Value;
use zebflow::infra::cluster::config::ClusterRole;
use zebflow::infra::execution::sync::ProjectBootstrapPlan;
use zebflow::infra::health::{
    HealthState, spawn_main_runtime_heartbeat, start_dedicated_health_server,
};
use zebflow::platform::model::CreateProjectRequest;
use zebflow::platform::services::PlatformService;
use zebflow::platform::services::project::{
    derive_trigger_kind_from_source, webhook_triggers_from_source,
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
  zebflow [standalone]
  zebflow run <project-or-hub-asset-url> [--owner <owner>] [--project <project>]
  zebflow controller
  zebflow office
  zebflow k8s cluster <command> ...
  zebflow help
  zebflow --help
  zebflow --version

Runtime Modes:
  standalone   Start the combined controller + office server (default)
  run          Materialize one app project if needed, then serve its public route
  controller   Start the control-plane oriented server
  office       Start the execution-plane oriented server

Kubernetes:
  zebflow k8s cluster init <path>
  zebflow k8s cluster add-office <path> <office-id>
  zebflow k8s cluster set-controller <path> <office-id>
  zebflow k8s cluster set-namespace <path> <namespace>
  zebflow k8s cluster set-resource-suffix <path> <suffix>
  zebflow k8s cluster set-image <path> <image>
  zebflow k8s cluster use-secret <path> <secret-name>
  zebflow k8s cluster set-replicas <path> <replicas>
  zebflow k8s cluster enable-precreate-pvcs <path>
  zebflow k8s cluster disable-precreate-pvcs <path>
  zebflow k8s cluster enable-auto-update <path>
  zebflow k8s cluster disable-auto-update <path>
  zebflow k8s cluster render-copy-jobs <source-path> <target-path> <output-file>
  zebflow k8s cluster describe <path>
  zebflow k8s cluster validate <path>

Environment:
  ZEBFLOW_PLATFORM_DEFAULT_PASSWORD  Required for standalone/controller bootstrap
  ZEBFLOW_PLATFORM_HOST              Listen host (default: 127.0.0.1)
  ZEBFLOW_PLATFORM_PORT              Listen port (default: 10610)
  ZEBFLOW_HEALTH_PORT                Optional dedicated liveness port, e.g. 10611
  ZEBFLOW_HEALTH_HOST                Dedicated liveness host (default: ZEBFLOW_PLATFORM_HOST)
  ZEBFLOW_PLATFORM_DATA_DIR          Data root override
  ZEBFLOW_SECRET_ROTATION_EPOCH      Unix timestamp; invalidate older platform-issued tokens
  ZEBFLOW_HUB_DEFAULT_BASE_URL          Default platform hub API URL

Use `zebflow k8s --help` for the file-based Kubernetes cluster manager.",
        version = APP_VERSION
    )
}

fn print_top_level_help() {
    println!("{}", top_level_help());
}

fn print_version() {
    println!("{APP_VERSION}");
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
    config.cluster.role = role;
    config.cluster.node_id = std::env::var("ZEBFLOW_CLUSTER_NODE_ID").ok();
    config.cluster.node_label = std::env::var("ZEBFLOW_CLUSTER_NODE_LABEL").ok();
    config.cluster.master_url = std::env::var("ZEBFLOW_CLUSTER_MASTER_URL").ok();
    config.cluster.advertise_url = std::env::var("ZEBFLOW_CLUSTER_ADVERTISE_URL")
        .ok()
        .or_else(|| Some(default_advertise_url(&host, port)));
    config.cluster.join_token = std::env::var("ZEBFLOW_CLUSTER_JOIN_TOKEN").ok();

    config.data_adapter = DataAdapterKind::Sqlite;
    config.file_adapter = FileAdapterKind::Filesystem;

    if role != ClusterRole::Worker && config.default_password.trim().is_empty() {
        return Err(io::Error::other(
            "missing ZEBFLOW_PLATFORM_DEFAULT_PASSWORD for initial superadmin bootstrap",
        ));
    }
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

#[derive(Debug, Clone)]
struct RunRequest {
    target: String,
    owner: Option<String>,
    project: Option<String>,
}

#[derive(Debug, Clone)]
struct RemoteAssetRef {
    url: String,
    package_id: String,
    version: String,
}

fn parse_run_request(args: &[String]) -> Result<RunRequest, io::Error> {
    let Some(target) = args.first() else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "missing run target",
        ));
    };
    let mut owner = None;
    let mut project = None;
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
                owner = Some(value.clone());
            }
            "--project" => {
                index += 1;
                let Some(value) = args.get(index) else {
                    return Err(io::Error::new(
                        io::ErrorKind::InvalidInput,
                        "missing value for --project",
                    ));
                };
                project = Some(value.clone());
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
    Ok(RunRequest {
        target: target.clone(),
        owner,
        project,
    })
}

fn parse_remote_asset_ref(raw: &str) -> Option<RemoteAssetRef> {
    let url = Url::parse(raw).ok()?;
    let segments = url
        .path_segments()?
        .filter(|segment| !segment.is_empty())
        .collect::<Vec<_>>();
    let remote_idx = segments
        .windows(2)
        .position(|window| window == ["remote", "assets"])?;
    let package_id = segments.get(remote_idx + 2)?.to_string();
    let version = segments.get(remote_idx + 3)?.to_string();
    Some(RemoteAssetRef {
        url: raw.to_string(),
        package_id,
        version,
    })
}

fn sanitize_project_slug(raw: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in raw.chars() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            out.push(lower);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    out.trim_matches('-').to_string()
}

fn repo_rel_path(raw: &str) -> Result<PathBuf, io::Error> {
    let path = Path::new(raw);
    if path.is_absolute() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("absolute repo path is not allowed: {raw}"),
        ));
    }
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(part) => out.push(part),
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("repo path must not escape project root: {raw}"),
                ));
            }
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("unsupported repo path component in '{raw}'"),
                ));
            }
        }
    }
    if out.as_os_str().is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "repo path must not be empty",
        ));
    }
    Ok(out)
}

fn decode_artifact_entry_bytes(entry: &Value) -> Result<Vec<u8>, io::Error> {
    let encoding = entry
        .get("encoding")
        .and_then(Value::as_str)
        .unwrap_or("text");
    let content = entry
        .get("content")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if encoding == "base64" {
        return base64::engine::general_purpose::STANDARD
            .decode(content)
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()));
    }
    Ok(content.as_bytes().to_vec())
}

fn fallback_pipeline_title(file_rel_path: &str) -> String {
    Path::new(file_rel_path)
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("Imported Pipeline")
        .replace(".zf", "")
        .replace('-', " ")
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

async fn install_remote_project_asset(
    platform: &PlatformService,
    owner: &str,
    project: &str,
    remote: &RemoteAssetRef,
) -> Result<(), Box<dyn std::error::Error>> {
    let response = reqwest::Client::new().get(&remote.url).send().await?;
    if !response.status().is_success() {
        return Err(io::Error::other(format!(
            "remote hub fetch failed with {}",
            response.status()
        ))
        .into());
    }
    let payload: Value = response.json().await?;
    let artifact = payload
        .get("artifact")
        .cloned()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "missing artifact payload"))?;
    let files = artifact
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                "artifact.files must be an array",
            )
        })?;

    platform.projects.create_or_update_project(
        owner,
        &CreateProjectRequest {
            project: project.to_string(),
            title: Some(remote.package_id.replace('-', " ")),
            local_branch: None,
            runtime: Default::default(),
        },
    )?;
    let layout = platform.projects.project_layout(owner, project)?;

    let mut activated = Vec::<String>::new();
    for entry in files {
        let rel_path_raw = entry
            .get("rel_path")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    "artifact entry missing rel_path",
                )
            })?;
        let rel_path = repo_rel_path(rel_path_raw)?;
        let dest_abs = layout.repo_dir.join(&rel_path);
        if let Some(parent) = dest_abs.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let bytes = decode_artifact_entry_bytes(entry)?;
        std::fs::write(&dest_abs, bytes)?;

        if rel_path
            .extension()
            .and_then(|value| value.to_str())
            .map(|value| value.eq_ignore_ascii_case("json"))
            .unwrap_or(false)
            && rel_path
                .to_str()
                .map(|value| value.starts_with("pipelines/") && value.ends_with(".zf.json"))
                .unwrap_or(false)
        {
            let rel_string = rel_path.to_string_lossy().to_string();
            let source = std::fs::read_to_string(&dest_abs)?;
            let title = fallback_pipeline_title(&rel_string);
            let trigger_kind =
                derive_trigger_kind_from_source(&source).unwrap_or_else(|| "webhook".to_string());
            let meta = platform.projects.upsert_pipeline_definition(
                owner,
                project,
                &rel_string,
                &title,
                "",
                &trigger_kind,
                &source,
            )?;
            activated.push(meta.file_rel_path);
        }
    }

    if !activated.is_empty() {
        platform.zebflow_cfg.set_bootstrap(
            owner,
            project,
            ProjectBootstrapPlan {
                activate: activated.clone(),
            },
        )?;
        platform
            .cluster_runtime_sync
            .refresh_local_repo_state(owner, project)?;
    }

    Ok(())
}

async fn run_project(req: RunRequest) -> Result<(), Box<dyn std::error::Error>> {
    let config = load_platform_config(ClusterRole::Standalone)?;
    let host = configured_host();
    let port = configured_port();
    let health = maybe_start_dedicated_health_server(&host)?;

    let remote = parse_remote_asset_ref(&req.target);
    let owner = req
        .owner
        .clone()
        .unwrap_or_else(|| config.default_owner.clone());
    let project = req.project.clone().unwrap_or_else(|| {
        remote
            .as_ref()
            .map(|item| sanitize_project_slug(&item.package_id))
            .unwrap_or_else(|| sanitize_project_slug(&req.target))
    });
    if project.trim().is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "empty project slug").into());
    }

    let platform = Arc::new(PlatformService::from_config(config)?);
    if let Some(remote) = remote {
        println!(
            "Installing {}@{} from remote hub asset...",
            remote.package_id, remote.version
        );
        install_remote_project_asset(platform.as_ref(), &owner, &project, &remote).await?;
    } else {
        let exists = platform
            .projects
            .list_projects(&owner)?
            .into_iter()
            .any(|item| item.project == project);
        if !exists {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("project '{owner}/{project}' is not installed"),
            )
            .into());
        }
        platform
            .cluster_runtime_sync
            .refresh_local_repo_state(&owner, &project)?;
    }

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
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let mode = args.next();

    match mode.as_deref() {
        None | Some("standalone") => run_server(ClusterRole::Standalone).await,
        Some("run") => run_project(parse_run_request(&args.collect::<Vec<_>>())?).await,
        Some("help") | Some("--help") | Some("-h") => {
            print_top_level_help();
            Ok(())
        }
        Some("version") | Some("--version") | Some("-V") => {
            print_version();
            Ok(())
        }
        Some("master") | Some("controller") => run_server(ClusterRole::Master).await,
        Some("worker") | Some("office") => run_server(ClusterRole::Worker).await,
        Some("k8s") => k8s_provision::run_cli(&args.collect::<Vec<_>>()),
        Some(other) => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("unknown zebflow mode '{other}'\n\n{}", top_level_help()),
        )
        .into()),
    }
}
