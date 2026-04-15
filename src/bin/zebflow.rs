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

use zebflow::infra::cluster::config::ClusterRole;
use zebflow::platform::{DataAdapterKind, FileAdapterKind, PlatformConfig, build_router};
use zebflow::provision::k8s as k8s_provision;
use zebflow::version::APP_VERSION;

/// Resolves when SIGTERM or Ctrl-C arrives, allowing axum's graceful shutdown
/// to drain in-flight requests.
async fn shutdown_signal() {
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
  zebflow controller
  zebflow office
  zebflow k8s cluster <command> ...
  zebflow help
  zebflow --help
  zebflow --version

Runtime Modes:
  standalone   Start the combined controller + office server (default)
  controller   Start the control-plane oriented server
  office       Start the execution-plane oriented server

Kubernetes:
  zebflow k8s cluster init <path>
  zebflow k8s cluster add-office <path> <office-id>
  zebflow k8s cluster set-controller <path> <office-id>
  zebflow k8s cluster set-image <path> <image>
  zebflow k8s cluster enable-auto-update <path>
  zebflow k8s cluster disable-auto-update <path>
  zebflow k8s cluster describe <path>
  zebflow k8s cluster validate <path>

Environment:
  ZEBFLOW_PLATFORM_DEFAULT_PASSWORD  Required for standalone/controller bootstrap
  ZEBFLOW_PLATFORM_HOST              Listen host (default: 127.0.0.1)
  ZEBFLOW_PLATFORM_PORT              Listen port (default: 10610)
  ZEBFLOW_PLATFORM_DATA_DIR          Data root override

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
fn load_platform_config(role: ClusterRole) -> Result<PlatformConfig, io::Error> {
    let mut config = PlatformConfig::default();
    let host = configured_host();
    let port = configured_port();

    if let Ok(path) = std::env::var("ZEBFLOW_PLATFORM_DATA_DIR") {
        config.data_root = path.into();
    }
    if let Ok(owner) = std::env::var("ZEBFLOW_PLATFORM_DEFAULT_OWNER") {
        config.default_owner = owner;
    }
    config.default_password =
        std::env::var("ZEBFLOW_PLATFORM_DEFAULT_PASSWORD").unwrap_or_default();
    if let Ok(project) = std::env::var("ZEBFLOW_PLATFORM_DEFAULT_PROJECT") {
        config.default_project = project;
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

    Ok(config)
}

/// Run the requested Zebflow server role.
async fn run_server(role: ClusterRole) -> Result<(), Box<dyn std::error::Error>> {
    let config = load_platform_config(role)?;
    let app = build_router(config).await.map_err(io::Error::other)?;

    let host = configured_host();
    let port = configured_port();

    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    println!("Zebflow listening on http://{}", addr);
    println!("Mode: {}", display_mode(role));
    println!("Flow: /login -> /home -> /projects/{{owner}}/{{project}}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
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
