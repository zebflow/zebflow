//! This machine's own instance, opened by the client when nothing else has.
//!
//! `interface.md` §6 draws the transport line at **who owns the state**. A
//! running server owns an in-memory node registry that a disk write behind its
//! back would invalidate, so the client speaks HTTP to it. When nothing is
//! listening there is nothing to keep coherent, and this module is what happens
//! instead: the command opens the data root itself, does the work, and exits.
//! It does not start a daemon, because a background process nobody asked for is
//! not a side effect an install may have — `zeb run` and the Group 1 modes are
//! the commands whose stated job is to serve.
//!
//! §5 adds the first-use half. A machine with `zebflow` newly installed has no
//! instance and no stored context, and `zeb install <ref>` on it has to work
//! without a separate provisioning step. Opening the data root bootstraps it —
//! `PlatformService::from_config` already creates the superadmin account and
//! writes its generated password 0600 — and this module then reads that file
//! and logs in through `POST /login` like any other client. The file is mode
//! 0600 and owned by the same user, so being able to read it *is* the check.
//! No second authentication mechanism is invented, and a file that cannot be
//! read produces a login failure, which is the correct outcome.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::infra::cluster::config::ClusterRole;
use crate::platform::boot;
use crate::platform::build_router;

use super::client::Instance;
use super::context::{self, ClientContext};

/// Where first boot leaves the generated superadmin password.
const BOOTSTRAP_PASSWORD_REL: &str = ".bootstrap/superadmin-password";

/// The file whose presence means an instance already exists at a data root.
const CATALOG_REL: &str = "platform/catalog.db";

/// How long to wait for a local server to answer before concluding that none
/// is running. Loopback either answers quickly or is not there.
const PROBE_TIMEOUT: Duration = Duration::from_millis(1500);

/// Opens this machine's instance, provisioning and authenticating as needed.
///
/// Three states, two transports:
///
/// ```text
/// a server is listening here     -> HTTP to it, exactly as before
/// nothing listening, instance    -> open the data root in this process
/// nothing listening, no instance -> create it, then open it in this process
/// ```
///
/// The one-time provisioning notice is printed here rather than returned,
/// because it belongs before whatever the command goes on to print.
///
/// `stored` is updated in place when a sign-in happens here, so a caller that
/// read the context before this ran still resolves `--owner` from what was just
/// written rather than from the empty context it started with.
pub async fn open(stored: &mut ClientContext) -> Result<Instance, io::Error> {
    let config = boot::load_platform_config(ClusterRole::Standalone)?;
    let data_root = config.data_root.clone();
    let owner = config.default_owner.clone();
    let configured_password = config.default_password.clone();
    let url = boot::local_instance_url();
    let existed = data_root.join(CATALOG_REL).exists();

    let instance = if listening(&url).await {
        Instance::new(&url, stored.token.trim())?
    } else {
        // Opening the data root is what bootstraps it, so this one call covers
        // both "an instance is already here" and "there is not one yet".
        let router = build_router(config)
            .await
            .map_err(|err| io::Error::other(err.message))?;
        Instance::local(&url, stored.token.trim(), router)
    };

    let instance = authenticated(
        instance,
        stored,
        &url,
        &owner,
        &data_root,
        &configured_password,
    )
    .await?;

    if !existed {
        announce_provisioning(&data_root, &owner);
    }
    Ok(instance)
}

/// Returns the instance carrying a credential that works.
///
/// A stored token is checked rather than trusted: sessions live in a server's
/// memory, so one written by an earlier in-process command is not valid in this
/// one. When it does not answer, the local bootstrap password buys a new token
/// through the ordinary login route and the context store records it.
async fn authenticated(
    instance: Instance,
    stored: &mut ClientContext,
    url: &str,
    owner: &str,
    data_root: &Path,
    configured_password: &str,
) -> Result<Instance, io::Error> {
    let usable = !stored.token.trim().is_empty()
        && stored.instance == url
        && instance.get_json("/api/profile").await.is_ok();
    if usable {
        return Ok(instance);
    }
    let password = superadmin_password(data_root, configured_password)?;
    let token = instance.authenticate(owner, &password).await?;
    stored.instance = url.to_string();
    stored.token = token.clone();
    stored.owner = owner.to_string();
    remember(url, &token, owner)?;
    Ok(instance.with_token(&token))
}

/// Says where the instance is and what its credential is, once, on the run
/// that created it.
fn announce_provisioning(data_root: &Path, owner: &str) {
    println!(
        "Created a new Zebflow instance at {} and signed in as {owner}.",
        data_root.display()
    );
    let password_path = data_root.join(BOOTSTRAP_PASSWORD_REL);
    if password_path.is_file() {
        // Named, not printed: the password is written 0600 precisely so it does
        // not land in a terminal scrollback or a CI log, and nobody has to go
        // and read it to continue -- the sign-in already happened.
        println!(
            "Its generated password is in {} (mode 0600). No command changes it yet: set \
             ZEBFLOW_PLATFORM_DEFAULT_PASSWORD before an instance's first start to choose one.",
            password_path.display()
        );
    }
}

/// Whether anything answers the liveness route at this machine's own URL.
async fn listening(url: &str) -> bool {
    let Ok(client) = reqwest::Client::builder().timeout(PROBE_TIMEOUT).build() else {
        return false;
    };
    client
        .get(format!("{url}/health"))
        .send()
        .await
        .map(|response| response.status().is_success())
        .unwrap_or(false)
}

/// The password to sign in with: the configured one when the host named one,
/// and otherwise the generated file first boot wrote.
fn superadmin_password(data_root: &Path, configured: &str) -> Result<String, io::Error> {
    if !configured.trim().is_empty() {
        return Ok(configured.to_string());
    }
    let path: PathBuf = data_root.join(BOOTSTRAP_PASSWORD_REL);
    let raw = fs::read_to_string(&path).map_err(|err| {
        io::Error::new(
            err.kind(),
            format!(
                "cannot read the generated password at {}: {err}. Sign in with `{} login {}` \
                 instead.",
                path.display(),
                super::program(),
                boot::local_instance_url()
            ),
        )
    })?;
    let password = raw.trim().to_string();
    if password.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("the generated password at {} is empty", path.display()),
        ));
    }
    Ok(password)
}

/// Stores the token in the ordinary client context, so the next command needs
/// no provisioning at all and `zeb logout` can revoke what this created.
fn remember(url: &str, token: &str, owner: &str) -> Result<(), io::Error> {
    let path = context::default_context_path()?;
    let mut stored = context::load(&path)?;
    stored.instance = url.to_string();
    stored.token = token.to_string();
    stored.owner = owner.to_string();
    context::save(&path, &stored)
}
