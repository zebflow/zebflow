//! Service for reading and writing `repo/zeb.lock` (git-tracked library lock file).

use std::path::PathBuf;

use crate::infra::io::durable::{
    JsonContract, JsonContractField, JsonContractValue, read_optional_versioned_json,
    write_atomic_json,
};
use crate::platform::error::PlatformError;
use crate::platform::model::{ZebLock, ZebLockEntry, slug_segment};

const ZEB_LOCK_FIELDS: &[JsonContractField] = &[JsonContractField {
    name: "version",
    expected: JsonContractValue::U64(1),
}];
const ZEB_LOCK_CONTRACT: JsonContract = JsonContract {
    name: "zeb.lock",
    fields: ZEB_LOCK_FIELDS,
};

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

    /// Reads `zeb.lock`, returning an empty current-version lock only when missing.
    ///
    /// Malformed and unsupported lock files are rejected. They are never replaced
    /// with defaults because doing so would silently discard dependency pins.
    pub fn read(&self, owner: &str, project: &str) -> Result<ZebLock, PlatformError> {
        let path = self.lock_path(owner, project);
        read_optional_versioned_json(&path, ZEB_LOCK_CONTRACT)
            .map(|lock| lock.unwrap_or_default())
            .map_err(|err| {
                PlatformError::new("ZEB_LOCK_READ", format!("{} ({})", err, err.category()))
            })
    }

    /// Validates and durably replaces `zeb.lock` as pretty JSON.
    pub fn write(&self, owner: &str, project: &str, lock: &ZebLock) -> Result<(), PlatformError> {
        let path = self.lock_path(owner, project);
        write_atomic_json(&path, lock, ZEB_LOCK_CONTRACT).map_err(|err| {
            PlatformError::new("ZEB_LOCK_WRITE", format!("{} ({})", err, err.category()))
        })
    }

    /// Writes `zeb.lock` only if the file does not already exist.
    pub fn write_if_missing(
        &self,
        owner: &str,
        project: &str,
        default: &ZebLock,
    ) -> Result<(), PlatformError> {
        let path = self.lock_path(owner, project);
        if path.try_exists().map_err(PlatformError::from)? {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_and_future_lock_files_are_rejected() {
        let root = tempfile::tempdir().unwrap();
        let service = ZebLockService::new(root.path().join("users"));
        let path = service.lock_path("owner", "project");
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();

        std::fs::write(&path, b"not-json").unwrap();
        assert_eq!(
            service.read("owner", "project").unwrap_err().code,
            "ZEB_LOCK_READ"
        );

        std::fs::write(&path, br#"{"version":2,"libraries":{}}"#).unwrap();
        assert_eq!(
            service.read("owner", "project").unwrap_err().code,
            "ZEB_LOCK_READ"
        );
    }

    #[test]
    fn lock_write_is_strict_and_roundtrips() {
        let root = tempfile::tempdir().unwrap();
        let service = ZebLockService::new(root.path().join("users"));
        service
            .write("owner", "project", &ZebLock::default())
            .unwrap();
        assert_eq!(service.read("owner", "project").unwrap().version, 1);

        let mut unsupported = ZebLock::default();
        unsupported.version = 2;
        assert!(service.write("owner", "project", &unsupported).is_err());
        assert_eq!(service.read("owner", "project").unwrap().version, 1);
    }
}
