use std::fs;
use std::path::{Component, Path, PathBuf};

use super::error::ZebFsError;
use super::model::{ZebFsEntry, ZebFsEntryKind, ZebFsObject, ZebFsStat};

/// Local filesystem-backed ZebFS implementation.
#[derive(Debug, Clone)]
pub struct LocalZebFs {
    root: PathBuf,
}

impl LocalZebFs {
    /// Creates a local ZebFS rooted at a project artifact directory.
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    /// Returns the physical root for this backend.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Resolves a ZebFS object path to the backend-relative path and local
    /// filesystem path without reading the object bytes.
    ///
    /// This is for engines such as DataFusion, GDAL, and map serving that can
    /// stream from a file path themselves. Callers must still check metadata
    /// when existence is required.
    pub fn resolve_object_path(&self, path: &str) -> Result<(String, PathBuf), ZebFsError> {
        let rel = normalize_object_path(path)?;
        let abs = self.abs_path(&rel)?;
        Ok((rel, abs))
    }

    /// Writes one object, creating parent directories as needed.
    pub fn put(&self, path: &str, bytes: &[u8]) -> Result<ZebFsStat, ZebFsError> {
        let rel = normalize_object_path(path)?;
        let abs = self.abs_path(&rel)?;
        if let Some(parent) = abs.parent() {
            fs::create_dir_all(parent)?;
        }

        // Local fast path with atomic same-directory replace.
        let tmp_name = format!(
            ".{}.{}.tmp",
            abs.file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("object"),
            std::process::id()
        );
        let tmp = abs.with_file_name(tmp_name);
        fs::write(&tmp, bytes)?;
        fs::rename(&tmp, &abs)?;
        self.head(&rel)
    }

    /// Reads one object into memory.
    pub fn get(&self, path: &str) -> Result<ZebFsObject, ZebFsError> {
        let rel = normalize_object_path(path)?;
        let abs = self.abs_path(&rel)?;
        if !abs.is_file() {
            return Err(ZebFsError::new("ZEBFS_NOT_FOUND", "object not found"));
        }
        let bytes = fs::read(&abs)?;
        let stat = self.head(&rel)?;
        Ok(ZebFsObject {
            path: rel,
            bytes,
            stat,
        })
    }

    /// Reads metadata for one object or prefix.
    pub fn head(&self, path: &str) -> Result<ZebFsStat, ZebFsError> {
        let rel = normalize_object_path(path)?;
        let abs = self.abs_path(&rel)?;
        let metadata = fs::metadata(&abs)
            .map_err(|_| ZebFsError::new("ZEBFS_NOT_FOUND", "object not found"))?;
        let kind = if metadata.is_dir() {
            ZebFsEntryKind::Prefix
        } else {
            ZebFsEntryKind::Object
        };
        Ok(ZebFsStat {
            path: rel,
            size: metadata.len(),
            modified: metadata.modified().ok(),
            kind,
        })
    }

    /// Lists immediate children under a prefix.
    pub fn list(&self, prefix: &str) -> Result<Vec<ZebFsEntry>, ZebFsError> {
        let rel = normalize_optional_prefix(prefix)?;
        let dir = if rel.is_empty() {
            self.root.clone()
        } else {
            self.abs_path(&rel)?
        };

        if !dir.exists() {
            return Ok(Vec::new());
        }
        if !dir.is_dir() {
            return Err(ZebFsError::new("ZEBFS_NOT_PREFIX", "path is not a prefix"));
        }

        let mut out = Vec::new();
        let mut entries = fs::read_dir(&dir)?.flatten().collect::<Vec<_>>();
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let name = entry.file_name().to_string_lossy().into_owned();
            let entry_rel = if rel.is_empty() {
                name.clone()
            } else {
                format!("{rel}/{name}")
            };
            let metadata = match entry.metadata() {
                Ok(metadata) => metadata,
                Err(_) => continue,
            };
            out.push(ZebFsEntry {
                name,
                path: entry_rel,
                size: metadata.len(),
                modified: metadata.modified().ok(),
                kind: if metadata.is_dir() {
                    ZebFsEntryKind::Prefix
                } else {
                    ZebFsEntryKind::Object
                },
            });
        }
        Ok(out)
    }

    /// Creates one prefix directory.
    pub fn create_prefix(&self, prefix: &str) -> Result<ZebFsStat, ZebFsError> {
        let rel = normalize_object_path(prefix)?;
        let abs = self.abs_path(&rel)?;
        fs::create_dir_all(abs)?;
        self.head(&rel)
    }

    /// Deletes one object or prefix tree.
    pub fn delete(&self, path: &str) -> Result<(), ZebFsError> {
        let rel = normalize_object_path(path)?;
        let abs = self.abs_path(&rel)?;
        if !abs.exists() {
            return Ok(());
        }
        if abs.is_dir() {
            fs::remove_dir_all(abs)?;
        } else {
            fs::remove_file(abs)?;
        }
        Ok(())
    }

    /// Copies one object.
    pub fn copy(&self, from: &str, to: &str) -> Result<ZebFsStat, ZebFsError> {
        let from_rel = normalize_object_path(from)?;
        let to_rel = normalize_object_path(to)?;
        let from_abs = self.abs_path(&from_rel)?;
        let to_abs = self.abs_path(&to_rel)?;
        if !from_abs.is_file() {
            return Err(ZebFsError::new(
                "ZEBFS_NOT_FOUND",
                "source object not found",
            ));
        }
        if let Some(parent) = to_abs.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(from_abs, to_abs)?;
        self.head(&to_rel)
    }

    fn abs_path(&self, normalized_rel: &str) -> Result<PathBuf, ZebFsError> {
        let abs = self.root.join(normalized_rel);
        if !abs.starts_with(&self.root) {
            return Err(ZebFsError::new("ZEBFS_INVALID_PATH", "invalid object path"));
        }
        Ok(abs)
    }
}

/// Normalizes a required object path.
pub fn normalize_object_path(path: &str) -> Result<String, ZebFsError> {
    let normalized = normalize_path(path)?;
    if normalized.is_empty() {
        return Err(ZebFsError::new(
            "ZEBFS_INVALID_PATH",
            "path must not be empty",
        ));
    }
    Ok(normalized)
}

fn normalize_optional_prefix(path: &str) -> Result<String, ZebFsError> {
    normalize_path(path)
}

fn normalize_path(path: &str) -> Result<String, ZebFsError> {
    let raw = path.trim().trim_start_matches('/').replace('\\', "/");
    if raw.contains('\0') {
        return Err(ZebFsError::new(
            "ZEBFS_INVALID_PATH",
            "path contains null byte",
        ));
    }

    let mut parts = Vec::new();
    for component in Path::new(&raw).components() {
        match component {
            Component::Normal(part) => {
                let part = part.to_string_lossy();
                if part.is_empty() {
                    continue;
                }
                parts.push(part.into_owned());
            }
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(ZebFsError::new("ZEBFS_INVALID_PATH", "path escapes root"));
            }
        }
    }
    Ok(parts.join("/"))
}
