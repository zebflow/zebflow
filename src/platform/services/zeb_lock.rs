//! Service for reading and writing `repo/zeb.lock` (git-tracked library lock file).

use std::path::PathBuf;

use crate::platform::error::PlatformError;
use crate::platform::model::{ZebLock, ZebLockEntry, slug_segment};

/// Reads and writes `{users_root}/{owner}/{project}/repo/zeb.lock`.
pub struct ZebLockService {
    users_root: PathBuf,
}

impl ZebLockService {
    /// Creates service rooted at `{data_root}/users`.
    pub fn new(users_root: PathBuf) -> Self {
        Self { users_root }
    }

    fn lock_path(&self, owner: &str, project: &str) -> PathBuf {
        self.users_root
            .join(slug_segment(owner))
            .join(slug_segment(project))
            .join("repo")
            .join("zeb.lock")
    }

    /// Reads `zeb.lock`, returning an empty lock if the file is missing or invalid.
    pub fn read(&self, owner: &str, project: &str) -> Result<ZebLock, PlatformError> {
        let path = self.lock_path(owner, project);
        let Ok(raw) = std::fs::read_to_string(&path) else {
            return Ok(ZebLock::default());
        };
        Ok(serde_json::from_str(&raw).unwrap_or_default())
    }

    /// Writes `zeb.lock` as pretty JSON.
    pub fn write(&self, owner: &str, project: &str, lock: &ZebLock) -> Result<(), PlatformError> {
        let path = self.lock_path(owner, project);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let serialized = serde_json::to_string_pretty(lock)
            .map_err(|e| PlatformError::new("ZEB_LOCK_SERIALIZE", e.to_string()))?;
        std::fs::write(&path, serialized)
            .map_err(|e| PlatformError::new("ZEB_LOCK_WRITE", e.to_string()))?;
        Ok(())
    }

    /// Writes `zeb.lock` only if the file does not already exist.
    pub fn write_if_missing(
        &self,
        owner: &str,
        project: &str,
        default: &ZebLock,
    ) -> Result<(), PlatformError> {
        let path = self.lock_path(owner, project);
        if path.exists() {
            return Ok(());
        }
        self.write(owner, project, default)
    }

    /// Adds or updates one library entry in `zeb.lock`.
    pub fn add_entry(
        &self,
        owner: &str,
        project: &str,
        name: &str,
        entry: ZebLockEntry,
    ) -> Result<(), PlatformError> {
        let mut lock = self.read(owner, project)?;
        lock.libraries.insert(name.to_string(), entry);
        self.write(owner, project, &lock)
    }

    /// Removes one library entry from `zeb.lock`. No-op if not present.
    pub fn remove_entry(
        &self,
        owner: &str,
        project: &str,
        name: &str,
    ) -> Result<(), PlatformError> {
        let mut lock = self.read(owner, project)?;
        lock.libraries.remove(name);
        self.write(owner, project, &lock)
    }
}
