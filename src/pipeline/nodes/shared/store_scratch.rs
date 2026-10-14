//! A local working folder for engines that only speak file paths.
//!
//! tar, zip, pdfium, GDAL and DataFusion read and write paths, but a project's
//! files live in its one active store — local disk or a bucket. A node that
//! needs a path pulls the objects it reads into a scratch folder, runs the
//! engine there, and pushes what the engine wrote back into the store. The
//! folder is removed when the scratch is dropped, so nothing outside the store
//! survives the node, and a project in a bucket works the same as one on disk.

use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};

use crate::pipeline::PipelineError;
use crate::zebfs::{ZebFs, ZebFsEntryKind};

pub const MAX_SCRATCH_BYTES: u64 = 256 * 1024 * 1024;
pub const MAX_SCRATCH_FILES: usize = 2_000;

pub struct StoreScratch {
    root: PathBuf,
    code: &'static str,
    bytes_used: AtomicU64,
    files_used: AtomicUsize,
    max_bytes: u64,
    max_files: usize,
}

impl StoreScratch {
    /// A fresh, empty folder under the system temporary directory. `code` is
    /// the error code every failure here is reported under.
    pub fn new(code: &'static str) -> Result<Self, PipelineError> {
        Self::with_limits(code, MAX_SCRATCH_BYTES, MAX_SCRATCH_FILES)
    }

    pub fn with_limits(
        code: &'static str,
        max_bytes: u64,
        max_files: usize,
    ) -> Result<Self, PipelineError> {
        let root = std::env::temp_dir()
            .join("zebflow-scratch")
            .join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(&root)
            .map_err(|err| PipelineError::new(code, format!("create scratch folder: {err}")))?;
        Ok(Self {
            root,
            code,
            bytes_used: AtomicU64::new(0),
            files_used: AtomicUsize::new(0),
            max_bytes,
            max_files,
        })
    }

    pub fn path(&self) -> &Path {
        &self.root
    }

    fn validate_rel_allow_empty(&self, rel: &str, allow_empty: bool) -> Result<String, PipelineError> {
        if rel.contains('\\') || rel.contains('\0') {
            return Err(self.err(format!("invalid scratch path '{rel}'")));
        }
        let trimmed = rel.trim_matches('/');
        if trimmed.is_empty() {
            if allow_empty {
                return Ok(String::new());
            }
            return Err(self.err("scratch path must not be empty".to_string()));
        }
        for component in Path::new(trimmed).components() {
            match component {
                Component::Normal(seg) => {
                    let s = seg.to_string_lossy();
                    if s == "." || s == ".." {
                        return Err(self.err(format!("invalid scratch path segment in '{rel}'")));
                    }
                }
                _ => {
                    return Err(self.err(format!("invalid scratch path '{rel}'")));
                }
            }
        }
        Ok(trimmed.to_string())
    }

    fn checked_local(&self, rel: &str, allow_empty: bool) -> Result<PathBuf, PipelineError> {
        let clean = self.validate_rel_allow_empty(rel, allow_empty)?;
        if clean.is_empty() {
            Ok(self.root.clone())
        } else {
            Ok(self.root.join(clean))
        }
    }

    /// Where `rel` lives inside the scratch folder.
    /// Rejects parent-dir and root components so returned paths always stay under `self.root`.
    pub fn local(&self, rel: &str) -> PathBuf {
        let mut out = self.root.clone();
        for seg in rel.replace('\\', "/").split('/') {
            if !seg.is_empty() && seg != "." && seg != ".." && !seg.contains('\0') {
                out.push(seg);
            }
        }
        out
    }

    fn err(&self, message: String) -> PipelineError {
        PipelineError::new(self.code, message)
    }

    fn account_file(&self, size: u64) -> Result<(), PipelineError> {
        let prev_files = self.files_used.fetch_add(1, Ordering::Relaxed);
        if prev_files + 1 > self.max_files {
            return Err(self.err(format!(
                "scratch file count exceeds limit of {} files",
                self.max_files
            )));
        }
        let mut current = self.bytes_used.load(Ordering::Relaxed);
        loop {
            let next = current.checked_add(size).ok_or_else(|| {
                self.err("scratch byte accounting overflow".to_string())
            })?;
            if next > self.max_bytes {
                return Err(self.err(format!(
                    "scratch byte usage ({next} bytes) exceeds limit of {} bytes",
                    self.max_bytes
                )));
            }
            match self.bytes_used.compare_exchange_weak(
                current,
                next,
                Ordering::Relaxed,
                Ordering::Relaxed,
            ) {
                Ok(_) => return Ok(()),
                Err(actual) => current = actual,
            }
        }
    }

    /// Copies one object, or every object under a prefix, from the store into
    /// the scratch folder at the same relative path. Answers the local path.
    pub fn pull(&self, store: &ZebFs, rel: &str) -> Result<PathBuf, PipelineError> {
        let rel = self.validate_rel_allow_empty(rel, false)?;
        let local_path = self.checked_local(&rel, false)?;
        match store.head(&rel) {
            Ok(stat) if stat.kind == ZebFsEntryKind::Object => {
                self.fetch(store, &rel)?;
                return Ok(local_path);
            }
            Ok(_) => {}
            Err(err) if err.code == "ZEBFS_NOT_FOUND" => {}
            Err(err) => return Err(self.err(format!("read '{rel}': {err}"))),
        }
        let mut found = false;
        self.pull_prefix(store, &rel, &mut found)?;
        if !found {
            return Err(self.err(format!("source path not found: {rel}")));
        }
        Ok(local_path)
    }

    /// Like [`Self::pull`], plus every object beside it sharing its stem:
    /// a shapefile's `.shx`, `.dbf` and `.prj` travel with its `.shp`.
    pub fn pull_with_siblings(&self, store: &ZebFs, rel: &str) -> Result<PathBuf, PipelineError> {
        let local = self.pull(store, rel)?;
        let rel = self.validate_rel_allow_empty(rel, false)?;
        let (parent, name) = match rel.rsplit_once('/') {
            Some((parent, name)) => (parent, name),
            None => ("", rel.as_str()),
        };
        let stem = name.split('.').next().unwrap_or(name);
        if stem.is_empty() || !local.is_file() {
            return Ok(local);
        }
        let siblings = store
            .list(parent)
            .map_err(|err| self.err(format!("list '{parent}': {err}")))?;
        for entry in siblings {
            if entry.kind == ZebFsEntryKind::Object
                && entry.name != name
                && entry.name.starts_with(&format!("{stem}."))
            {
                self.fetch(store, &entry.path)?;
            }
        }
        Ok(local)
    }

    fn pull_prefix(&self, store: &ZebFs, prefix: &str, found: &mut bool) -> Result<(), PipelineError> {
        let prefix = self.validate_rel_allow_empty(prefix, true)?;
        let entries = match store.list(&prefix) {
            Ok(entries) => entries,
            Err(err) if err.code == "ZEBFS_NOT_FOUND" || err.code == "ZEBFS_NOT_PREFIX" => {
                return Ok(());
            }
            Err(err) => return Err(self.err(format!("list '{prefix}': {err}"))),
        };
        if !entries.is_empty() || store.head(&prefix).is_ok() {
            *found = true;
            let local_dir = self.checked_local(&prefix, true)?;
            std::fs::create_dir_all(local_dir)
                .map_err(|err| self.err(format!("create '{prefix}': {err}")))?;
        }
        for entry in entries {
            match entry.kind {
                ZebFsEntryKind::Prefix => self.pull_prefix(store, &entry.path, found)?,
                ZebFsEntryKind::Object => self.fetch(store, &entry.path)?,
            }
        }
        Ok(())
    }

    /// Streams one object into the scratch folder at its own relative path.
    fn fetch(&self, store: &ZebFs, rel: &str) -> Result<(), PipelineError> {
        let rel = self.validate_rel_allow_empty(rel, false)?;
        let path = self.checked_local(&rel, false)?;
        if let Ok(stat) = store.head(&rel) {
            if stat.size > self.max_bytes {
                return Err(self.err(format!(
                    "object '{rel}' ({} bytes) exceeds scratch limit of {} bytes",
                    stat.size, self.max_bytes
                )));
            }
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| self.err(format!("create '{}': {err}", parent.display())))?;
        }
        let stat = store
            .get_to_file(&rel, &path)
            .map_err(|err| self.err(format!("read '{rel}': {err}")))?;
        if let Err(err) = self.account_file(stat.size) {
            let _ = std::fs::remove_file(&path);
            return Err(err);
        }
        Ok(())
    }

    /// Streams one file the engine wrote into the project's store at `rel`.
    pub fn push_file(&self, store: &ZebFs, local: &Path, rel: &str) -> Result<(), PipelineError> {
        let rel = self.validate_rel_allow_empty(rel, false)?;
        let meta = std::fs::symlink_metadata(local)
            .map_err(|err| self.err(format!("inspect '{}': {err}", local.display())))?;
        if meta.file_type().is_symlink() {
            return Err(self.err(format!("refusing to push symlink '{}'", local.display())));
        }
        self.account_file(meta.len())?;
        store
            .put_from_file(&rel, local)
            .map(|_| ())
            .map_err(|err| self.err(format!("write '{rel}': {err}")))
    }

    /// [`Self::push_tree`], answering a durable FileRef for every file.
    pub fn push_tree_refs(
        &self,
        store: &super::project_store::NodeStore,
        local_dir: &Path,
        rel_prefix: &str,
        origin: &str,
        trust: &str,
    ) -> Result<Vec<serde_json::Value>, PipelineError> {
        let prefix = self.validate_rel_allow_empty(rel_prefix, true)?;
        let written = self.push_tree(&store.fs, local_dir, &prefix)?;
        let mut refs = Vec::with_capacity(written.len());
        for rel in written {
            let inner = if prefix.is_empty() { rel.as_str() } else { rel.strip_prefix(&format!("{prefix}/")).unwrap_or(&rel) };
            let filename = rel.rsplit('/').next().unwrap_or(&rel).to_string();
            let mime = super::file_ref::mime_for_filename(&filename);
            refs.push(store.file_ref_from_file(&rel, &filename, mime, &local_dir.join(inner), origin, trust, self.code)?);
        }
        Ok(refs)
    }

    /// Stores every file under `local_dir` in the project's store beneath
    /// `rel_prefix`, keeping their relative layout. Answers the store paths.
    pub fn push_tree(&self, store: &ZebFs, local_dir: &Path, rel_prefix: &str) -> Result<Vec<String>, PipelineError> {
        let prefix = self.validate_rel_allow_empty(rel_prefix, true)?;
        let mut written = Vec::new();
        self.push_tree_inner(store, local_dir, local_dir, &prefix, &mut written)?;
        written.sort();
        Ok(written)
    }

    fn push_tree_inner(
        &self,
        store: &ZebFs,
        base: &Path,
        dir: &Path,
        rel_prefix: &str,
        written: &mut Vec<String>,
    ) -> Result<(), PipelineError> {
        let entries = std::fs::read_dir(dir)
            .map_err(|err| self.err(format!("read '{}': {err}", dir.display())))?;
        for entry in entries.flatten() {
            let path = entry.path();
            let file_type = entry
                .file_type()
                .map_err(|err| self.err(format!("inspect '{}': {err}", path.display())))?;
            // Links never leave the scratch folder for the store.
            if file_type.is_symlink() {
                continue;
            }
            if file_type.is_dir() {
                self.push_tree_inner(store, base, &path, rel_prefix, written)?;
            } else if file_type.is_file() {
                let inner = path
                    .strip_prefix(base)
                    .map_err(|err| self.err(format!("scratch path: {err}")))?
                    .to_string_lossy()
                    .replace('\\', "/");
                let rel = if rel_prefix.is_empty() {
                    inner
                } else {
                    format!("{rel_prefix}/{inner}")
                };
                self.push_file(store, &path, &rel)?;
                written.push(rel);
            }
        }
        Ok(())
    }
}

impl Drop for StoreScratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[cfg(test)]
mod tests {
    use super::StoreScratch;
    use crate::zebfs::{LocalZebFs, ZebFs};

    #[test]
    fn pulls_a_prefix_and_pushes_a_tree_back_through_the_store() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = ZebFs::Local(LocalZebFs::new(temp.path().to_path_buf()));
        store.put("docs/a.txt", b"a").expect("put a");
        store.put("docs/inner/b.txt", b"b").expect("put b");

        let scratch = StoreScratch::new("TEST").expect("scratch");
        let local = scratch.pull(&store, "docs").expect("pull");
        assert_eq!(std::fs::read(local.join("inner/b.txt")).unwrap(), b"b");

        let written = scratch.push_tree(&store, &local, "copy").expect("push");
        assert_eq!(written, vec!["copy/a.txt".to_string(), "copy/inner/b.txt".to_string()]);
        assert_eq!(store.get("copy/inner/b.txt").unwrap().bytes, b"b");

        let root = scratch.path().to_path_buf();
        drop(scratch);
        assert!(!root.exists());
    }

    #[test]
    fn a_missing_source_is_reported() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = ZebFs::Local(LocalZebFs::new(temp.path().to_path_buf()));
        let scratch = StoreScratch::new("TEST").expect("scratch");
        assert!(scratch.pull(&store, "nothing/here").is_err());
    }

    #[test]
    fn rejects_parent_traversal_and_enforces_byte_and_file_caps() {
        let temp = tempfile::tempdir().expect("tempdir");
        let store = ZebFs::Local(LocalZebFs::new(temp.path().to_path_buf()));
        store.put("docs/a.txt", b"hello").expect("put a");
        store.put("docs/b.txt", b"world!").expect("put b");

        let scratch = StoreScratch::with_limits("TEST", 8, 10).expect("scratch");
        assert!(scratch.pull(&store, "../escape").is_err());
        assert!(scratch.pull(&store, "docs/../escape").is_err());
        assert!(scratch.local("../../etc/passwd").starts_with(scratch.path()));

        // Pulling both files (5 + 6 = 11 bytes) exceeds the 8-byte cap.
        let err = scratch.pull(&store, "docs").expect_err("should exceed byte cap");
        assert!(err.message.contains("exceeds limit"), "{err:?}");

        // File-count cap enforcement.
        let scratch_files = StoreScratch::with_limits("TEST", 1024, 1).expect("scratch");
        let err = scratch_files
            .pull(&store, "docs")
            .expect_err("should exceed file count cap");
        assert!(err.message.contains("file count exceeds limit"), "{err:?}");
    }
}
