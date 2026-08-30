//! Where an instance lives, and what the environment says about it.
//!
//! This module is the one place that turns environment variables into a
//! [`PlatformConfig`]. It used to live in `src/bin/zebflow.rs`, which was fine
//! while only a server mode ever opened a data root. `interface.md` §6 now
//! says the CLI opens one itself when no server owns it, so the reading has to
//! be shared: a client that resolved the data root differently from the server
//! would install into a tree the server never serves.
//!
//! ## The data root is not a property of the working directory
//!
//! `interface.md` §5 says there is no current directory: a project lives at
//! `<data-root>/users/{owner}/{project}/` inside an instance, and nothing about
//! where a shell is standing identifies one. The default data root used to be
//! `.zebflow-platform-data` *relative to the process's working directory*,
//! which contradicted that in the most direct way available — an installed
//! binary created an instance wherever the user happened to be, and a `cd`
//! lost the apps in it.
//!
//! So there are exactly two cases, and no third that consults the working
//! directory:
//!
//! ```text
//! ZEBFLOW_PLATFORM_DATA_DIR set  ->  use it
//! otherwise                      ->  the OS user-data path
//! ```
//!
//! Both cases produce a `PathBuf` that goes into the same layout code, so the
//! tree inside the directory is identical either way; only its parent differs.

use std::io;
use std::path::PathBuf;

use crate::infra::cluster::config::settings::ENV_JOIN_TOKEN;
use crate::infra::cluster::config::{ClusterRole, ClusterSettings};
use crate::platform::model::{DataAdapterKind, FileAdapterKind, PlatformConfig};

/// The variable that names an explicit data root.
pub const DATA_DIR_VAR: &str = "ZEBFLOW_PLATFORM_DATA_DIR";

/// Last resort when the OS reports no user-data directory at all.
///
/// This is not a third case. It is what is left on a machine with no `HOME`
/// and no `%LOCALAPPDATA%`, where there is nowhere else to put an instance;
/// such a host should set [`DATA_DIR_VAR`] explicitly.
const HOMELESS_FALLBACK: &str = ".zebflow-platform-data";

/// Instance directory name inside the OS user-data path.
///
/// macOS and Windows capitalise application directories; XDG does not. This is
/// the convention the `directories` crate encodes, followed here without taking
/// the dependency, because `dirs-next` — which already ships in this binary for
/// `~/.zebflow/client/context.json` — provides the same base paths.
#[cfg(any(target_os = "macos", target_os = "windows"))]
const INSTANCE_DIR: &str = "Zebflow";
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const INSTANCE_DIR: &str = "zebflow";

/// The data root: the explicit one when it is set, the OS user-data path
/// otherwise.
///
/// `dirs_next::data_local_dir()` is `$XDG_DATA_HOME` or `~/.local/share` on
/// Linux, `~/Library/Application Support` on macOS, and `%LOCALAPPDATA%` on
/// Windows.
pub fn default_data_root() -> PathBuf {
    data_root_from(|name| std::env::var(name).ok())
}

/// The same two cases against an arbitrary variable lookup, so the rule can be
/// tested without mutating the process environment — the shape
/// `ClusterSettings::from_lookup` already uses.
///
/// Blank is unset: an exported-but-empty variable is a misconfiguration, not a
/// request to use the current directory (`interface.md` §3a says the same of
/// the cluster variables).
pub fn data_root_from(lookup: impl Fn(&str) -> Option<String>) -> PathBuf {
    lookup(DATA_DIR_VAR)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(os_data_root)
}

/// The OS user-data path for this platform.
pub fn os_data_root() -> PathBuf {
    match dirs_next::data_local_dir() {
        Some(base) => base.join(INSTANCE_DIR),
        None => PathBuf::from(HOMELESS_FALLBACK),
    }
}

/// Listen host for a server mode.
pub fn configured_host() -> String {
    std::env::var("ZEBFLOW_PLATFORM_HOST").unwrap_or_else(|_| "127.0.0.1".to_string())
}

/// Listen port for a server mode.
pub fn configured_port() -> u16 {
    std::env::var("ZEBFLOW_PLATFORM_PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(10610)
}

/// The base URL a node advertises when it does not name one.
pub fn default_advertise_url(host: &str, port: u16) -> String {
    let host = if host == "0.0.0.0" { "127.0.0.1" } else { host };
    format!("http://{host}:{port}")
}

/// The URL a server on this machine listens on, or would listen on.
///
/// The client uses it to ask whether an instance is already running here before
/// deciding whether to open the data root itself.
pub fn local_instance_url() -> String {
    default_advertise_url(&configured_host(), configured_port())
}

/// Loads the platform configuration for one runtime role from the environment.
pub fn load_platform_config(role: ClusterRole) -> Result<PlatformConfig, io::Error> {
    let mut config = PlatformConfig::default();
    let host = configured_host();
    let port = configured_port();

    config.data_root = default_data_root();
    if let Ok(owner) = std::env::var("ZEBFLOW_PLATFORM_DEFAULT_OWNER") {
        config.default_owner = owner;
    }
    config.default_password = std::env::var("ZEBFLOW_PLATFORM_DEFAULT_PASSWORD")
        .ok()
        .filter(|value| !value.trim().is_empty())
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
    // Shape-checked here for the same reason: an unusable token should refuse
    // before the data root is opened, not after. There is no migration from
    // the old shared secret (`offices.md` §8) — a value that is not
    // `zfjoin1:` shaped names the mint command in its refusal.
    if let Some(token) = config.cluster.join_token.as_deref() {
        crate::infra::cluster::security::JoinToken::parse(token)
            .map_err(|err| io::Error::other(format!("{ENV_JOIN_TOKEN}: {err}")))?;
    }

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
        && crate::platform::services::bootstrap::is_insecure_password(&config.default_password)
        && !allow_insecure_default
    {
        return Err(io::Error::other(
            "refusing insecure ZEBFLOW_PLATFORM_DEFAULT_PASSWORD=secret; choose a strong password or set ZEBFLOW_PLATFORM_ALLOW_INSECURE_DEFAULT_PASSWORD=1 only for disposable local development",
        ));
    }

    Ok(config)
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::Path;

    use super::*;
    use crate::platform::services::PlatformService;

    #[test]
    fn the_data_root_has_exactly_two_cases() {
        assert_eq!(
            data_root_from(|_| Some("/srv/zebflow".to_string())),
            PathBuf::from("/srv/zebflow")
        );
        // Nothing consults the working directory, so an unset variable and a
        // blank one both land on the OS path rather than on `./something`.
        assert_eq!(data_root_from(|_| None), os_data_root());
        assert_eq!(data_root_from(|_| Some("   ".to_string())), os_data_root());
    }

    #[test]
    fn the_os_data_root_is_absolute_and_named_for_the_product() {
        // A relative root is the bug this module exists to remove: it makes an
        // instance a property of the directory the binary was run from.
        let root = os_data_root();
        if root == PathBuf::from(HOMELESS_FALLBACK) {
            return; // no home directory on this host; nothing to assert
        }
        assert!(root.is_absolute(), "{}", root.display());
        assert_eq!(
            root.file_name().and_then(|name| name.to_str()),
            Some(INSTANCE_DIR)
        );
    }

    /// Every directory under `root`, as a path relative to it.
    fn tree(root: &Path) -> BTreeSet<PathBuf> {
        fn walk(base: &Path, dir: &Path, out: &mut BTreeSet<PathBuf>) {
            let Ok(entries) = std::fs::read_dir(dir) else {
                return;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Ok(rel) = path.strip_prefix(base) {
                        out.insert(rel.to_path_buf());
                    }
                    walk(base, &path, out);
                }
            }
        }
        let mut out = BTreeSet::new();
        walk(root, root, &mut out);
        out
    }

    #[test]
    fn the_auto_and_explicit_roots_produce_the_same_tree() {
        // The contract: only the parent path differs between the two cases.
        // Both are a `PathBuf` handed to the same layout code, and nothing
        // downstream may branch on which case produced it. Two roots standing
        // in for the two cases must therefore hold identical trees.
        let explicit = tempfile::tempdir().expect("explicit root");
        let automatic = tempfile::tempdir().expect("automatic root");

        for root in [explicit.path(), automatic.path()] {
            let config = PlatformConfig {
                data_root: root.to_path_buf(),
                default_password: "tree-shape-check".to_string(),
                ..PlatformConfig::default()
            };
            PlatformService::from_config(config).expect("open the data root");
        }

        assert_eq!(
            tree(explicit.path()),
            tree(automatic.path()),
            "the two data-root cases produced different trees"
        );
        // And the tree is the one `interface.md` §5 describes, not an empty set
        // two runs happen to agree on.
        let shape = tree(explicit.path());
        for expected in [
            "platform",
            "services",
            "users/superadmin/default/repo",
            "users/superadmin/default/data",
            "users/superadmin/default/files",
        ] {
            assert!(
                shape.contains(&PathBuf::from(expected)),
                "missing {expected} in {shape:?}"
            );
        }
    }
}
