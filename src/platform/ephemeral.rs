//! The EPHEMERAL tier — `<data-root>/run/` and `<data-root>/tmp/`.
//!
//! `instance-directory.md` rule 3: wiped on startup, never backed up, never
//! exported. `run/` holds locks, pids, and sockets; `tmp/` holds staging,
//! upload chunks, and scratch. This is the one tier where deletion is the
//! contract: anything a crash leaves behind here dies at the next boot.
//!
//! [`prepare_ephemeral_tier`] runs once per process start, from
//! `PlatformService::from_config`, after the layout gate
//! ([`crate::platform::layout::open_data_root`]) and before any adapter or
//! service opens anything in the root — so nothing can hold an ephemeral file
//! open while it is being wiped, and nothing serves before the wipe is done.
//!
//! ## What "wipe" means, exactly
//!
//! - The wipe removes the *contents* of `run/` and `tmp/`, never the
//!   directories themselves; both exist when this returns.
//! - A `run` or `tmp` that is a **symlink refuses boot**. Deleting through a
//!   symlink would wipe a tree outside the root this contract covers, and
//!   writing through one would put ephemeral state somewhere no future boot
//!   wipes. Nothing here ever follows a symlink: entries that are symlinks
//!   are removed as links, targets untouched.
//! - An error on one entry (a busy socket, a permission oddity) logs and
//!   continues — a leftover file must not stop boot. Only the tier being
//!   structurally wrong (symlinked, unreadable, uncreatable) refuses.

use std::path::Path;

use crate::platform::error::PlatformError;

/// The EPHEMERAL directories at the data root, wiped on startup.
pub const EPHEMERAL_DIRS: &[&str] = &["run", "tmp"];

/// Wipes the contents of `<data-root>/run/` and `<data-root>/tmp/` and
/// guarantees both directories exist. Called once per process start, before
/// anything serves.
pub fn prepare_ephemeral_tier(data_root: &Path) -> Result<(), PlatformError> {
    for name in EPHEMERAL_DIRS {
        let dir = data_root.join(name);
        match std::fs::symlink_metadata(&dir) {
            Ok(meta) if meta.file_type().is_symlink() => {
                return Err(PlatformError::new(
                    "PLATFORM_EPHEMERAL_SYMLINK",
                    format!(
                        "'{}' is a symlink; the EPHEMERAL tier is wiped on every boot \
                         and wiping through a symlink would delete outside the data \
                         root — replace it with a real directory",
                        dir.display()
                    ),
                ));
            }
            Ok(meta) if meta.is_dir() => wipe_directory_contents(&dir)?,
            Ok(_) => {
                // A plain file squatting on the tier's name. It is inside the
                // ephemeral namespace, so deleting it is the contract.
                std::fs::remove_file(&dir).map_err(|error| {
                    PlatformError::new(
                        "PLATFORM_EPHEMERAL_WIPE",
                        format!("failed removing stray file '{}': {error}", dir.display()),
                    )
                })?;
                std::fs::create_dir_all(&dir)?;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                std::fs::create_dir_all(&dir)?;
            }
            Err(error) => {
                return Err(PlatformError::new(
                    "PLATFORM_EPHEMERAL_WIPE",
                    format!("failed inspecting '{}': {error}", dir.display()),
                ));
            }
        }
    }
    Ok(())
}

/// Removes every entry inside `dir`, leaving `dir` itself in place.
///
/// Entry-level failures log and continue; only a directory that cannot be
/// enumerated at all is an error. Symlink entries are removed as links —
/// `entry.file_type()` does not follow them, and `std::fs::remove_dir_all`
/// removes symlinks inside a subtree without following them either.
fn wipe_directory_contents(dir: &Path) -> Result<(), PlatformError> {
    let entries = std::fs::read_dir(dir).map_err(|error| {
        PlatformError::new(
            "PLATFORM_EPHEMERAL_WIPE",
            format!("failed reading '{}': {error}", dir.display()),
        )
    })?;
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                eprintln!(
                    "warning: ephemeral wipe skipped an entry in '{}': {error}",
                    dir.display()
                );
                continue;
            }
        };
        let path = entry.path();
        let result = match entry.file_type() {
            Ok(file_type) if file_type.is_dir() => std::fs::remove_dir_all(&path),
            Ok(_) => std::fs::remove_file(&path),
            Err(error) => Err(error),
        };
        if let Err(error) = result {
            eprintln!(
                "warning: ephemeral wipe left '{}' behind: {error}",
                path.display()
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_root_gets_both_directories() {
        let root = tempfile::tempdir().expect("root");
        prepare_ephemeral_tier(root.path()).expect("prepare");
        assert!(root.path().join("run").is_dir());
        assert!(root.path().join("tmp").is_dir());
    }

    #[test]
    fn the_wipe_removes_contents_but_keeps_the_directories() {
        let root = tempfile::tempdir().expect("root");
        let run = root.path().join("run");
        let tmp = root.path().join("tmp");
        std::fs::create_dir_all(run.join("locks")).unwrap();
        std::fs::write(run.join("server.pid"), b"123").unwrap();
        std::fs::create_dir_all(tmp.join("transfer/export-bundle-x")).unwrap();
        std::fs::write(tmp.join("transfer/export-bundle-x/manifest.json"), b"{}").unwrap();
        std::fs::write(tmp.join("chunk.bin"), b"bytes").unwrap();

        prepare_ephemeral_tier(root.path()).expect("prepare");

        assert!(run.is_dir() && tmp.is_dir());
        assert_eq!(std::fs::read_dir(&run).unwrap().count(), 0);
        assert_eq!(std::fs::read_dir(&tmp).unwrap().count(), 0);
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_tier_directory_refuses() {
        let root = tempfile::tempdir().expect("root");
        let elsewhere = tempfile::tempdir().expect("elsewhere");
        let precious = elsewhere.path().join("precious.txt");
        std::fs::write(&precious, b"keep me").unwrap();
        std::os::unix::fs::symlink(elsewhere.path(), root.path().join("tmp")).unwrap();

        let error = prepare_ephemeral_tier(root.path()).expect_err("must refuse");
        assert_eq!(error.code, "PLATFORM_EPHEMERAL_SYMLINK");
        // And nothing behind the link was touched.
        assert_eq!(std::fs::read(&precious).unwrap(), b"keep me");
    }

    #[cfg(unix)]
    #[test]
    fn a_symlink_entry_inside_the_tier_is_removed_without_following_it() {
        let root = tempfile::tempdir().expect("root");
        let elsewhere = tempfile::tempdir().expect("elsewhere");
        let precious = elsewhere.path().join("precious.txt");
        std::fs::write(&precious, b"keep me").unwrap();
        let tmp = root.path().join("tmp");
        std::fs::create_dir_all(&tmp).unwrap();
        std::os::unix::fs::symlink(elsewhere.path(), tmp.join("escape")).unwrap();

        prepare_ephemeral_tier(root.path()).expect("prepare");

        assert_eq!(std::fs::read_dir(&tmp).unwrap().count(), 0);
        assert_eq!(std::fs::read(&precious).unwrap(), b"keep me");
    }

    #[test]
    fn a_plain_file_on_the_tier_name_is_replaced_by_the_directory() {
        let root = tempfile::tempdir().expect("root");
        std::fs::write(root.path().join("run"), b"not a dir").unwrap();
        prepare_ephemeral_tier(root.path()).expect("prepare");
        assert!(root.path().join("run").is_dir());
    }
}
