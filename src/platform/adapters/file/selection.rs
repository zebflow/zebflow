//! Which credential reaches a project's bucket: `data/store/files-backend.json`.
//!
//! `spec.files.backend` in `repo/zebflow.yaml` declares the *word* — it is
//! committed, git-synced and portable, so it says what kind of store the
//! project keeps its files in. The credential that reaches that store is this
//! instance's, and a different instance restoring the same repository will
//! select its own; so the selection lives in the STORE tier beside
//! `addressing.json`, never in the repository.
//!
//! ```json
//! { "credential_id": "seaweed-local" }
//! ```

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::infra::io::durable::atomic_write;
use crate::platform::error::PlatformError;

/// The document's name under `data/store/`.
pub const FILES_BACKEND_FILE: &str = "files-backend.json";

/// The instance's selection for a project declaring an object-store backend.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FilesBackendSelection {
    /// The id of a credential of kind `s3` in this project; absent when none
    /// was chosen.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credential_id: Option<String>,
}

/// The selection, or the default when the document was never written.
pub fn read_selection(store_dir: &Path) -> Result<FilesBackendSelection, PlatformError> {
    let path = store_dir.join(FILES_BACKEND_FILE);
    let bytes = match std::fs::read(&path) {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Ok(FilesBackendSelection::default());
        }
        Err(err) => {
            return Err(PlatformError::new(
                "PROJECT_FILES_BACKEND_READ",
                format!("{}: {err}", path.display()),
            ));
        }
    };
    serde_json::from_slice(&bytes).map_err(|err| {
        PlatformError::new(
            "PROJECT_FILES_BACKEND_INVALID",
            format!("{}: {err}", path.display()),
        )
    })
}

/// Persists the selection atomically, creating the store tier if needed.
pub fn write_selection(
    store_dir: &Path,
    selection: &FilesBackendSelection,
) -> Result<(), PlatformError> {
    std::fs::create_dir_all(store_dir).map_err(|err| {
        PlatformError::new(
            "PROJECT_FILES_BACKEND_WRITE",
            format!("{}: {err}", store_dir.display()),
        )
    })?;
    let path = store_dir.join(FILES_BACKEND_FILE);
    let mut bytes = serde_json::to_vec_pretty(selection)
        .map_err(|err| PlatformError::new("PROJECT_FILES_BACKEND_WRITE", err.to_string()))?;
    bytes.push(b'\n');
    atomic_write(&path, &bytes).map_err(|err| {
        PlatformError::new(
            "PROJECT_FILES_BACKEND_WRITE",
            format!("{}: {err}", path.display()),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_absent_document_is_the_default_and_a_written_one_reads_back() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(read_selection(dir.path()).unwrap(), FilesBackendSelection::default());
        let selection = FilesBackendSelection {
            credential_id: Some("seaweed-local".into()),
        };
        write_selection(dir.path(), &selection).unwrap();
        assert_eq!(read_selection(dir.path()).unwrap(), selection);
        assert!(dir.path().join(FILES_BACKEND_FILE).is_file());
    }

    #[test]
    fn a_document_with_a_field_this_build_does_not_know_is_refused_by_name() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join(FILES_BACKEND_FILE),
            br#"{"credential_id":"a","bucket":"b"}"#,
        )
        .unwrap();
        let err = read_selection(dir.path()).unwrap_err();
        assert_eq!(err.code, "PROJECT_FILES_BACKEND_INVALID");
        assert!(err.message.contains("bucket"), "{}", err.message);
    }
}
