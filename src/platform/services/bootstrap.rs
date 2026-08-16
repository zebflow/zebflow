//! Secure first-run bootstrap helpers.
//!
//! This module owns generated platform bootstrap credentials. Generated
//! passwords are stored under the platform data root instead of being written
//! to process logs.

use std::fs::{self, OpenOptions};
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};

use rand::RngExt as _;

use crate::platform::error::PlatformError;

const GENERATED_PASSWORD_BYTES: usize = 24;
const BOOTSTRAP_DIR: &str = ".bootstrap";
const SUPERADMIN_PASSWORD_FILE: &str = "superadmin-password";

/// Initial superadmin password resolved from host configuration or generated
/// for a first-run installation.
pub struct BootstrapPassword {
    /// Password used to create the initial account.
    pub value: String,
    /// Secure file containing the generated password, when no password was
    /// supplied by the host.
    pub generated_path: Option<PathBuf>,
}

/// Resolves the initial superadmin password.
///
/// A non-empty configured password is used directly. Otherwise, Zebflow
/// creates a cryptographically random password and persists it in a file that
/// is readable only by the process owner on Unix systems. `create_new` makes
/// concurrent or retried bootstrap attempts reuse one stable credential.
pub fn resolve_superadmin_password(
    data_root: &Path,
    configured: &str,
) -> Result<BootstrapPassword, PlatformError> {
    if !configured.trim().is_empty() {
        return Ok(BootstrapPassword {
            value: configured.to_string(),
            generated_path: None,
        });
    }

    let bootstrap_dir = data_root.join(BOOTSTRAP_DIR);
    create_private_dir(&bootstrap_dir)?;
    let password_path = bootstrap_dir.join(SUPERADMIN_PASSWORD_FILE);
    let generated = random_password();

    match create_private_file(&password_path) {
        Ok(mut file) => {
            file.write_all(generated.as_bytes()).map_err(|err| {
                PlatformError::new(
                    "PLATFORM_BOOTSTRAP_PASSWORD_WRITE",
                    format!(
                        "failed writing generated superadmin password '{}': {err}",
                        password_path.display()
                    ),
                )
            })?;
            file.write_all(b"\n").map_err(|err| {
                PlatformError::new(
                    "PLATFORM_BOOTSTRAP_PASSWORD_WRITE",
                    format!(
                        "failed finishing generated superadmin password '{}': {err}",
                        password_path.display()
                    ),
                )
            })?;
            file.sync_all().map_err(|err| {
                PlatformError::new(
                    "PLATFORM_BOOTSTRAP_PASSWORD_WRITE",
                    format!(
                        "failed syncing generated superadmin password '{}': {err}",
                        password_path.display()
                    ),
                )
            })?;
            Ok(BootstrapPassword {
                value: generated,
                generated_path: Some(password_path),
            })
        }
        Err(err) if err.kind() == ErrorKind::AlreadyExists => {
            let existing = fs::read_to_string(&password_path).map_err(|read_err| {
                PlatformError::new(
                    "PLATFORM_BOOTSTRAP_PASSWORD_READ",
                    format!(
                        "failed reading generated superadmin password '{}': {read_err}",
                        password_path.display()
                    ),
                )
            })?;
            let existing = existing.trim();
            if existing.is_empty() {
                return Err(PlatformError::new(
                    "PLATFORM_BOOTSTRAP_PASSWORD_INVALID",
                    format!(
                        "generated superadmin password file '{}' is empty",
                        password_path.display()
                    ),
                ));
            }
            Ok(BootstrapPassword {
                value: existing.to_string(),
                generated_path: Some(password_path),
            })
        }
        Err(err) => Err(PlatformError::new(
            "PLATFORM_BOOTSTRAP_PASSWORD_WRITE",
            format!(
                "failed creating generated superadmin password '{}': {err}",
                password_path.display()
            ),
        )),
    }
}

fn random_password() -> String {
    let mut bytes = [0u8; GENERATED_PASSWORD_BYTES];
    rand::rng().fill(&mut bytes);
    hex::encode(bytes)
}

fn create_private_dir(path: &Path) -> Result<(), PlatformError> {
    fs::create_dir_all(path).map_err(|err| {
        PlatformError::new(
            "PLATFORM_BOOTSTRAP_PASSWORD_WRITE",
            format!(
                "failed creating bootstrap directory '{}': {err}",
                path.display()
            ),
        )
    })?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|err| {
            PlatformError::new(
                "PLATFORM_BOOTSTRAP_PASSWORD_WRITE",
                format!(
                    "failed securing bootstrap directory '{}': {err}",
                    path.display()
                ),
            )
        })?;
    }
    Ok(())
}

fn create_private_file(path: &Path) -> std::io::Result<std::fs::File> {
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options.open(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn configured_password_does_not_create_bootstrap_file() {
        let root = tempfile::tempdir().expect("temp dir");
        let password = resolve_superadmin_password(root.path(), "provided-password")
            .expect("configured password");

        assert_eq!(password.value, "provided-password");
        assert!(password.generated_path.is_none());
        assert!(!root.path().join(BOOTSTRAP_DIR).exists());
    }

    #[test]
    fn generated_password_is_random_and_stable_for_retried_bootstrap() {
        let root = tempfile::tempdir().expect("temp dir");
        let first = resolve_superadmin_password(root.path(), "").expect("generated password");
        let second = resolve_superadmin_password(root.path(), "").expect("reloaded password");

        assert_eq!(first.value.len(), GENERATED_PASSWORD_BYTES * 2);
        assert!(first.value.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert_eq!(first.value, second.value);
        assert_eq!(first.generated_path, second.generated_path);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(first.generated_path.expect("password path"))
                .expect("password metadata")
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(mode, 0o600);
        }
    }
}
