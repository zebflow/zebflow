//! The data-root layout version — `platform/layout.json`.
//!
//! `instance-directory.md` says migrations key off this version, never off
//! path-shape heuristics ("does `data/runtime` exist"). This module is the one
//! reader and the one writer of the file, and [`open_data_root`] is called
//! exactly once per process, before any adapter opens anything inside the
//! root.
//!
//! ## Deliberately beneath the envelope
//!
//! This file is *not* a registered contract kind, and that is a decision, not
//! an omission. A kind exists so a document can leave the instance and be
//! validated by strangers — `zebflow-repository.json` became one the day other
//! people started publishing it. `layout.json` never leaves the instance: it
//! is the instance's own bookkeeping, read before anything else in the root
//! can be assumed to have its current shape — including whatever stores the
//! envelope machinery itself relies on. The oldest and the newest binary must
//! both be able to read it with zero machinery between them, so it stays a
//! bare `{ "version": N }` under the STORE tier, one integer and a timestamp.
//!
//! ## The four cases on open
//!
//! ```text
//! no file, empty root      -> fresh instance: stamp the current version
//! no file, populated root  -> pre-versioned root: the version-1 shape is
//!                             reached by the resident first-touch migrations
//!                             (see below); stamp the file
//! file, older version      -> run the migrations between, then restamp
//! file, newer version      -> refuse: an old binary must not touch a newer root
//! ```
//!
//! ## Version 1 has no step function here
//!
//! The moves that produce the version-1 tree — `data/sekejap`, `data/local.db`
//! and `data/runtime` into their tiers, `data/nodes` into `data/hub/nodes`,
//! the row-level `kv_durable.db` drain, chat history — ship as transparent
//! first-touch migrations (`migrate_tier_entry` and friends) at the sites that
//! touch the content. They stay where they are: they already handle both a
//! whole pre-versioned root and a single straggler project restored from an
//! old backup, which a boot-time step keyed off this file could not. From
//! version 2 on, a relocation registers a step in [`MIGRATIONS`] instead.

use std::io;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::infra::io::durable::atomic_write;
use crate::platform::error::PlatformError;

/// The layout version this binary produces and understands.
pub const CURRENT_LAYOUT_VERSION: u32 = 1;

/// Where the version lives inside a data root. STORE tier.
pub const LAYOUT_FILE_REL: &str = "platform/layout.json";

/// The version steps between two layout versions, in order.
///
/// `(target_version, step)`: the step transforms a root of shape
/// `target_version - 1` into shape `target_version`. Version 1 is absent by
/// design — see the module documentation.
type MigrationStep = fn(&Path) -> Result<(), PlatformError>;
const MIGRATIONS: &[(u32, MigrationStep)] = &[];

/// The file, exactly as stored: a version and when the file was first written.
#[derive(Debug, Serialize, Deserialize)]
struct LayoutFile {
    version: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    created_at: Option<i64>,
}

/// Opens one data root: verifies its layout version, runs any pending
/// migrations, and stamps the current version. Returns the version the root
/// had before this call (equal to [`CURRENT_LAYOUT_VERSION`] afterwards).
///
/// A root stamped with a *newer* version refuses here, before a single adapter
/// opens: an old binary writing into a tree it does not understand is how a
/// newer instance gets silently corrupted.
pub fn open_data_root(data_root: &Path) -> Result<u32, PlatformError> {
    let path = layout_file_path(data_root);
    let recorded = read_layout_file(&path)?;

    let (found_version, created_at) = match recorded {
        Some(file) if file.version > CURRENT_LAYOUT_VERSION => {
            return Err(PlatformError::new(
                "PLATFORM_LAYOUT_NEWER",
                format!(
                    "data root '{}' has layout version {}, but this binary understands \
                     up to version {}; refusing to touch a newer root — run a newer \
                     Zebflow binary against it instead",
                    data_root.display(),
                    file.version,
                    CURRENT_LAYOUT_VERSION
                ),
            ));
        }
        Some(file) => (file.version, file.created_at),
        // No file. An empty root is a fresh instance and starts at the current
        // version; a populated one predates the version file and is brought to
        // version 1 by the resident first-touch migrations, so its pending
        // steps here are those *after* 1.
        None if root_is_populated(data_root)? => (1, None),
        None => (CURRENT_LAYOUT_VERSION, None),
    };

    for (target, step) in MIGRATIONS {
        if *target > found_version && *target <= CURRENT_LAYOUT_VERSION {
            step(data_root)?;
        }
    }

    if found_version != CURRENT_LAYOUT_VERSION || !path.is_file() {
        let stamped = LayoutFile {
            version: CURRENT_LAYOUT_VERSION,
            created_at: Some(created_at.unwrap_or_else(crate::platform::model::now_ts)),
        };
        atomic_write(&path, serde_json::to_string_pretty(&stamped)?.as_bytes())?;
    }

    Ok(found_version)
}

/// `<data-root>/platform/layout.json`.
pub fn layout_file_path(data_root: &Path) -> PathBuf {
    data_root.join(LAYOUT_FILE_REL)
}

/// Reads and parses the file; absent is `None`, malformed refuses.
///
/// A file that exists but cannot be read as `{ "version": N }` is not treated
/// as absent — pretending an unreadable version file is a fresh root would
/// stamp the current version over whatever the file was trying to say.
fn read_layout_file(path: &Path) -> Result<Option<LayoutFile>, PlatformError> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(PlatformError::new(
                "PLATFORM_LAYOUT_READ",
                format!("failed reading '{}': {error}", path.display()),
            ));
        }
    };
    serde_json::from_slice::<LayoutFile>(&bytes)
        .map(Some)
        .map_err(|error| {
            PlatformError::new(
                "PLATFORM_LAYOUT_READ",
                format!(
                    "'{}' is not a layout version file ({error}); refusing to guess \
                     this root's version — restore or repair the file by hand",
                    path.display()
                ),
            )
        })
}

/// Whether the root holds anything at all.
///
/// Any entry counts: the version file is the first thing written on the fresh
/// path, so a root with content but no file predates the file, whatever that
/// content is.
fn root_is_populated(data_root: &Path) -> Result<bool, PlatformError> {
    match std::fs::read_dir(data_root) {
        Ok(mut entries) => Ok(entries.next().is_some()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(PlatformError::new(
            "PLATFORM_LAYOUT_READ",
            format!("failed reading '{}': {error}", data_root.display()),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fresh_root_is_stamped_with_the_current_version() {
        let root = tempfile::tempdir().expect("root");
        let found = open_data_root(root.path()).expect("open");
        assert_eq!(found, CURRENT_LAYOUT_VERSION);
        let stored: serde_json::Value =
            serde_json::from_slice(&std::fs::read(layout_file_path(root.path())).unwrap())
                .expect("valid json");
        assert_eq!(stored["version"], CURRENT_LAYOUT_VERSION);
        assert!(stored["created_at"].is_i64());
    }

    #[test]
    fn a_populated_root_without_a_file_is_a_pre_versioned_root() {
        let root = tempfile::tempdir().expect("root");
        std::fs::create_dir_all(root.path().join("users/owner/project/data")).unwrap();
        let found = open_data_root(root.path()).expect("open");
        // Found at version 1: the version-1 shape is the resident first-touch
        // migrations' job, not this module's.
        assert_eq!(found, 1);
        assert!(layout_file_path(root.path()).is_file());
    }

    #[test]
    fn a_second_open_is_a_no_op_and_keeps_created_at() {
        let root = tempfile::tempdir().expect("root");
        open_data_root(root.path()).expect("first open");
        let first = std::fs::read(layout_file_path(root.path())).unwrap();
        open_data_root(root.path()).expect("second open");
        assert_eq!(first, std::fs::read(layout_file_path(root.path())).unwrap());
    }

    #[test]
    fn a_newer_root_refuses() {
        let root = tempfile::tempdir().expect("root");
        atomic_write(
            &layout_file_path(root.path()),
            format!("{{\"version\":{}}}", CURRENT_LAYOUT_VERSION + 1).as_bytes(),
        )
        .unwrap();
        let error = open_data_root(root.path()).expect_err("must refuse");
        assert_eq!(error.code, "PLATFORM_LAYOUT_NEWER");
        // And the refusal changed nothing: the newer stamp is still there.
        let stored: serde_json::Value =
            serde_json::from_slice(&std::fs::read(layout_file_path(root.path())).unwrap()).unwrap();
        assert_eq!(stored["version"], CURRENT_LAYOUT_VERSION + 1);
    }

    #[test]
    fn a_malformed_file_refuses_rather_than_guessing() {
        let root = tempfile::tempdir().expect("root");
        atomic_write(&layout_file_path(root.path()), b"not json").unwrap();
        let error = open_data_root(root.path()).expect_err("must refuse");
        assert_eq!(error.code, "PLATFORM_LAYOUT_READ");
    }
}
