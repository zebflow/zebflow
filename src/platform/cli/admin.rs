//! Offline account recovery: `zeb admin reset-password <owner>`.
//!
//! Group 3 maintenance (`interface.md` §6): it goes straight at the data
//! directory because it exists for the moment no credential works any more —
//! the operator lost the password, so logging in to change it is not
//! available. The Grafana `admin reset-admin-password` shape, with one
//! difference: the new password is random and printed once, and the account is
//! marked `generated` again, so the next browser login is forced through the
//! change-password screen exactly like a first boot.

use std::io;
use std::path::Path;
use std::sync::Arc;

use crate::platform::adapters::data::build_data_adapter;
use crate::platform::boot;
use crate::platform::model::PlatformConfig;
use crate::platform::services::UserService;

/// The file whose presence means an instance exists at a data root. Kept in
/// sync with `local.rs`; an absent catalog means there is nothing to reset and
/// opening the adapter would create an empty one.
const CATALOG_REL: &str = "platform/catalog.db";

/// `zeb admin <subcommand>` dispatch.
pub async fn run(args: &[String]) -> Result<(), io::Error> {
    match args.first().map(String::as_str) {
        Some("reset-password") => run_reset_password(&args[1..]).await,
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("usage: {} admin reset-password <owner>", super::program()),
        )),
    }
}

/// `zeb admin reset-password <owner>` — offline, against the catalog.
async fn run_reset_password(args: &[String]) -> Result<(), io::Error> {
    let [owner] = args else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("usage: {} admin reset-password <owner>", super::program()),
        ));
    };

    // Refuse to write behind a running server's back: it verifies logins from
    // its own open connection to the catalog and holds every live session in
    // memory. The probe only sees a server on this machine's configured
    // address — one listening elsewhere on an unusual port is not detected,
    // which is the residual risk the help text names.
    let url = boot::local_instance_url();
    if super::local::listening(&url).await {
        return Err(io::Error::other(format!(
            "a server is answering at {url} and owns this data root. Stop it first, or change \
             the password through its web UI instead ({url}/account/password)."
        )));
    }

    let config =
        boot::load_platform_config(crate::infra::cluster::config::ClusterRole::Standalone)?;
    let data_root = config.data_root.clone();
    if !data_root.join(CATALOG_REL).exists() {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!(
                "no instance at {} (missing {CATALOG_REL}); nothing to reset",
                data_root.display()
            ),
        ));
    }

    let password = reset_against(&data_root, owner).map_err(io::Error::other)?;

    println!("New password for {owner}: {password}");
    println!(
        "Printed once, not stored. The account is marked `generated` again, so the next \
         browser login is forced to choose a new password. Sessions live only in a running \
         server's memory, and none is running here."
    );
    Ok(())
}

/// Opens the catalog directly and rotates the credential.
fn reset_against(data_root: &Path, owner: &str) -> Result<String, String> {
    let data = build_data_adapter(PlatformConfig::default().data_adapter, data_root)
        .map_err(|err| err.message)?;
    let users = Arc::new(UserService::new(data));
    users
        .reset_password_generated(owner)
        .map_err(|err| err.message)
}
