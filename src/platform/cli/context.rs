//! The client's record of which instance it talks to, as whom, and where.
//!
//! `interface.md` §5 records why this file exists at all: a Zebflow project
//! lives at `<data-root>/users/{owner}/{project}/` inside an instance, so
//! nothing about the shell's working directory identifies one. Context is
//! therefore stated once and stored, and resolution is
//! `explicit flag -> stored default -> error` with no step in between that
//! guesses.
//!
//! What is stored is the session token the server issued, never the password
//! that obtained it. The token expires, is revoked by `zeb logout`, and buys an
//! attacker who reads the file one account's session rather than the password
//! it was probably reused from.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Directory the client keeps its own state in, under the `~/.zebflow` base
/// the rest of the product already uses for machine-local material.
const CLIENT_DIR: &str = "client";

/// One person's client context. Every field is optional in the file, so a
/// context written by an older binary still loads.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientContext {
    /// Base URL of the instance, without a trailing slash.
    #[serde(default)]
    pub instance: String,
    /// Session token issued by `POST /login`. Not the password.
    #[serde(default)]
    pub token: String,
    /// Account the token authenticated as, and the default `--owner`.
    #[serde(default)]
    pub owner: String,
    /// Default `--project`. Set by `zeb use`, never inferred.
    #[serde(default)]
    pub project: String,
}

impl ClientContext {
    /// True when nothing has been stored yet, which is what separates "no
    /// context" from "a context that names an instance but no credential".
    pub fn is_empty(&self) -> bool {
        self == &Self::default()
    }
}

/// Where the context file lives, or an error naming why no home directory was
/// found — a guessed fallback would write a credential somewhere the user did
/// not choose.
pub fn default_context_path() -> Result<PathBuf, io::Error> {
    let home = dirs_next::home_dir().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::NotFound,
            "no home directory: cannot locate the Zebflow client context",
        )
    })?;
    Ok(home.join(".zebflow").join(CLIENT_DIR).join("context.json"))
}

/// Reads the stored context, treating a missing file as an empty one.
///
/// A file readable by anyone but its owner is reported on stderr rather than
/// refused: the token inside is already exposed, and refusing to read it would
/// only stop the user from running the `zeb logout` that revokes it.
pub fn load(path: &Path) -> Result<ClientContext, io::Error> {
    let raw = match fs::read_to_string(path) {
        Ok(value) => value,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(ClientContext::default()),
        Err(err) => return Err(err),
    };
    if let Some(mode) = group_or_world_permissions(path) {
        eprintln!(
            "{}: warning: {} is mode {:04o} and holds a session token; \
             anyone who can read it can act as this account until `{} logout`.",
            super::program(),
            path.display(),
            mode,
            super::program()
        );
    }
    serde_json::from_str(&raw).map_err(|err| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("invalid client context '{}': {err}", path.display()),
        )
    })
}

/// Writes the context with owner-only permissions.
///
/// The directory is created 0700 and the file 0600, the same shape
/// `platform::services::bootstrap` uses for the generated superadmin password.
/// The mode is re-applied on every write, so a file someone loosened is
/// tightened again the next time the client touches it.
pub fn save(path: &Path, context: &ClientContext) -> Result<(), io::Error> {
    if let Some(parent) = path.parent() {
        create_private_dir(parent)?;
    }
    let body = serde_json::to_string_pretty(context)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))?;
    write_private_file(path, body.as_bytes())
}

/// Removes the stored context, treating an absent file as already removed.
pub fn clear(path: &Path) -> Result<(), io::Error> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err),
    }
}

/// Resolves one context value: explicit flag, then stored default, then error.
///
/// The error names the command that would set the missing value, because a
/// person who has not read `interface.md` needs the next step and not the rule.
pub fn resolve(
    flag: Option<&str>,
    stored: &str,
    what: &str,
    hint: &str,
) -> Result<String, io::Error> {
    if let Some(value) = flag.map(str::trim).filter(|value| !value.is_empty()) {
        return Ok(value.to_string());
    }
    let stored = stored.trim();
    if !stored.is_empty() {
        return Ok(stored.to_string());
    }
    Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("no {what} set; {hint}"),
    ))
}

/// Strips a trailing slash so stored and typed instance URLs compare equal.
pub fn normalize_instance_url(raw: &str) -> String {
    raw.trim().trim_end_matches('/').to_string()
}

fn create_private_dir(path: &Path) -> Result<(), io::Error> {
    fs::create_dir_all(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

fn write_private_file(path: &Path, bytes: &[u8]) -> Result<(), io::Error> {
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    {
        use std::io::Write as _;
        let mut file = options.open(path)?;
        file.write_all(bytes)?;
        file.flush()?;
    }
    #[cfg(unix)]
    {
        // `OpenOptions::mode` only applies to a file this call created, so an
        // existing file keeps whatever mode it was given. Set it again.
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    }
    Ok(())
}

/// Returns the file mode when it grants any group or other access.
#[cfg(unix)]
fn group_or_world_permissions(path: &Path) -> Option<u32> {
    use std::os::unix::fs::PermissionsExt;
    let mode = fs::metadata(path).ok()?.permissions().mode() & 0o777;
    (mode & 0o077 != 0).then_some(mode)
}

#[cfg(not(unix))]
fn group_or_world_permissions(_path: &Path) -> Option<u32> {
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_context_file_loads_as_empty() {
        let dir = tempfile::tempdir().expect("temp dir");
        let context = load(&dir.path().join("context.json")).expect("load");
        assert!(context.is_empty());
    }

    #[test]
    fn saved_context_round_trips_with_owner_only_permissions() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("client").join("context.json");
        let context = ClientContext {
            instance: "http://localhost:10610".to_string(),
            token: "tok".to_string(),
            owner: "superadmin".to_string(),
            project: "default".to_string(),
        };
        save(&path, &context).expect("save");
        assert_eq!(load(&path).expect("load"), context);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let file_mode = fs::metadata(&path).expect("meta").permissions().mode() & 0o777;
            assert_eq!(file_mode, 0o600);
            let dir_mode = fs::metadata(path.parent().expect("parent"))
                .expect("meta")
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(dir_mode, 0o700);
        }
    }

    #[cfg(unix)]
    #[test]
    fn rewriting_a_loosened_context_file_tightens_it_again() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("context.json");
        save(&path, &ClientContext::default()).expect("save");
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).expect("loosen");
        assert_eq!(group_or_world_permissions(&path), Some(0o644));

        save(&path, &ClientContext::default()).expect("resave");
        assert_eq!(group_or_world_permissions(&path), None);
    }

    #[test]
    fn resolution_order_is_flag_then_stored_then_error() {
        assert_eq!(
            resolve(Some("flag"), "stored", "owner", "run zebflow use").expect("flag wins"),
            "flag"
        );
        assert_eq!(
            resolve(None, "stored", "owner", "run zebflow use").expect("stored next"),
            "stored"
        );
        let err = resolve(
            Some("  "),
            "",
            "owner",
            "run `zebflow use <owner>/<project>`",
        )
        .expect_err("no source");
        assert!(err.to_string().contains("no owner set"), "{err}");
        assert!(err.to_string().contains("zebflow use"), "{err}");
    }

    #[test]
    fn instance_urls_normalize_to_one_spelling() {
        assert_eq!(
            normalize_instance_url(" http://localhost:10610/ "),
            "http://localhost:10610"
        );
    }
}
