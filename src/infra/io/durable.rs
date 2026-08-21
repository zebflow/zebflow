//! Durable local-file replacement primitives.
//!
//! Contract identity and validation belong to `contracts`. This
//! independent module only guarantees synchronized same-directory replacement
//! so a failed write cannot partially overwrite the last valid file.

use std::fs::File;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

/// Atomically replaces `path` with `bytes` and synchronizes the durable rename.
pub fn atomic_write(path: &Path, bytes: &[u8]) -> io::Result<()> {
    atomic_write_with(path, |file| file.write_all(bytes))
}

/// Atomically installs an already-filled temporary file at `path`.
///
/// A writer that produces its bytes a chunk at a time -- a size-bounded
/// download, for one -- never holds them all at once and so cannot use
/// [`atomic_write`]. It fills a temporary file in the destination directory and
/// finishes here, keeping the same guarantee: `path` is either the previous
/// file or the complete new one, never a partial write.
pub fn durable_persist(temporary: tempfile::NamedTempFile, path: &Path) -> io::Result<()> {
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    sync_directory(path.parent().unwrap_or_else(|| Path::new(".")))
}

/// Removes one file and synchronizes its parent directory.
///
/// A missing file is already in the requested state and succeeds.
pub fn durable_remove_file(path: &Path) -> io::Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => sync_directory(path.parent().unwrap_or_else(|| Path::new("."))),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

/// Computes a deterministic SHA-256 digest for one directory tree.
///
/// Paths, lengths, and file bytes are hashed in lexical relative-path order.
/// Symbolic links are rejected so the digest cannot escape the selected root.
pub fn directory_tree_sha256(root: &Path) -> io::Result<String> {
    fn collect(root: &Path, current: &Path, files: &mut Vec<PathBuf>) -> io::Result<()> {
        for entry in std::fs::read_dir(current)? {
            let entry = entry?;
            let path = entry.path();
            let file_type = entry.file_type()?;
            if file_type.is_symlink() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("symbolic link is not allowed: {}", path.display()),
                ));
            }
            if file_type.is_dir() {
                collect(root, &path, files)?;
            } else if file_type.is_file() {
                path.strip_prefix(root).map_err(|_| {
                    io::Error::new(io::ErrorKind::InvalidData, "file escaped directory root")
                })?;
                files.push(path);
            }
        }
        Ok(())
    }

    let mut files = Vec::new();
    collect(root, root, &mut files)?;
    files.sort_by(|left, right| {
        left.strip_prefix(root)
            .unwrap_or(left)
            .cmp(right.strip_prefix(root).unwrap_or(right))
    });

    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 64 * 1024];
    for path in files {
        let relative = path
            .strip_prefix(root)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid tree path"))?
            .to_string_lossy()
            .replace('\\', "/");
        let size = std::fs::metadata(&path)?.len();
        hasher.update((relative.len() as u64).to_le_bytes());
        hasher.update(relative.as_bytes());
        hasher.update(size.to_le_bytes());
        let mut file = File::open(&path)?;
        loop {
            let read = file.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn atomic_write_with<F>(path: &Path, write: F) -> io::Result<()>
where
    F: FnOnce(&mut File) -> io::Result<()>,
{
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;

    let prefix = format!(
        ".{}.zebflow-write-",
        path.file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("state")
    );
    let mut temporary = tempfile::Builder::new()
        .prefix(&prefix)
        .tempfile_in(parent)?;

    write(temporary.as_file_mut())?;
    temporary.as_file_mut().flush()?;
    temporary.as_file().sync_all()?;
    temporary.persist(path).map_err(|error| error.error)?;
    sync_directory(parent)?;
    Ok(())
}

#[cfg(unix)]
fn sync_directory(path: &Path) -> io::Result<()> {
    File::open(path)?.sync_all()
}

#[cfg(not(unix))]
fn sync_directory(_path: &Path) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn failed_write_keeps_previous_file_and_removes_temporary_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        atomic_write(&path, b"previous").unwrap();

        let result = atomic_write_with(&path, |file| {
            file.write_all(b"partial")?;
            Err(io::Error::other("injected write failure"))
        });
        assert!(result.is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"previous");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }

    #[test]
    fn durable_remove_is_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");
        atomic_write(&path, b"state").unwrap();
        durable_remove_file(&path).unwrap();
        durable_remove_file(&path).unwrap();
        assert!(!path.exists());
    }

    #[test]
    fn directory_digest_is_order_independent_and_content_sensitive() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(first.path().join("nested")).unwrap();
        std::fs::create_dir_all(second.path().join("nested")).unwrap();
        std::fs::write(first.path().join("b.txt"), b"b").unwrap();
        std::fs::write(first.path().join("nested/a.txt"), b"a").unwrap();
        std::fs::write(second.path().join("nested/a.txt"), b"a").unwrap();
        std::fs::write(second.path().join("b.txt"), b"b").unwrap();

        assert_eq!(
            directory_tree_sha256(first.path()).unwrap(),
            directory_tree_sha256(second.path()).unwrap()
        );
        std::fs::write(second.path().join("b.txt"), b"changed").unwrap();
        assert_ne!(
            directory_tree_sha256(first.path()).unwrap(),
            directory_tree_sha256(second.path()).unwrap()
        );
    }
}
