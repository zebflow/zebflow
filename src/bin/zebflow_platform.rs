//! Zebflow platform runnable server.
//!
//! Run:
//! `cargo run -p zebflow --bin zebflow_platform`

use std::net::SocketAddr;

use zebflow::platform::{DataAdapterKind, FileAdapterKind, PlatformConfig, build_router};

/// Resolves when SIGTERM or Ctrl-C arrives, allowing axum's graceful shutdown
/// to drain in-flight requests (max 15 s is K8s default termination grace period).
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

    eprintln!("Zebflow: graceful shutdown initiated — draining in-flight requests…");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut config = PlatformConfig::default();

    if let Ok(path) = std::env::var("ZEBFLOW_PLATFORM_DATA_DIR") {
        config.data_root = path.into();
    }
    if let Ok(owner) = std::env::var("ZEBFLOW_PLATFORM_DEFAULT_OWNER") {
        config.default_owner = owner;
    }
    config.default_password = std::env::var("ZEBFLOW_PLATFORM_DEFAULT_PASSWORD").map_err(|_| {
        std::io::Error::other(
            "missing ZEBFLOW_PLATFORM_DEFAULT_PASSWORD for initial superadmin bootstrap",
        )
    })?;
    if let Ok(project) = std::env::var("ZEBFLOW_PLATFORM_DEFAULT_PROJECT") {
        config.default_project = project;
    }

    config.data_adapter = DataAdapterKind::Sekejap;
    config.file_adapter = FileAdapterKind::Filesystem;

    let app = build_router(config).await.map_err(std::io::Error::other)?;

    let host = std::env::var("ZEBFLOW_PLATFORM_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("ZEBFLOW_PLATFORM_PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(10610);

    let addr: SocketAddr = format!("{host}:{port}").parse()?;
    let listener = tokio::net::TcpListener::bind(addr).await?;

    println!("Zebflow platform listening on http://{}", addr);
    println!("Flow: /login -> /home -> /projects/{{owner}}/{{project}}");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    eprintln!("Zebflow: shutdown complete.");
    Ok(())
}
