//! A local working folder for engines that only speak file paths.
//!
//! tar, zip, pdfium, GDAL and DataFusion read and write paths, but a project's
//! files live in its one active store — local disk or a bucket. A node that
//! needs a path pulls the objects it reads into a scratch folder, runs the
//! engine there, and pushes what the engine wrote back into the store. The
//! folder is removed when the scratch is dropped, so nothing outside the store
//! survives the node, and a project in a bucket works the same as one on disk.

use std::path::{Path, PathBuf};

use crate::pipeline::PipelineError;
use crate::zebfs::{ZebFs, ZebFsEntryKind};

pub struct StoreScratch {
    root: PathBuf,
    code: &'static str,
}

impl StoreScratch {
    /// A fresh, empty folder under the system temporary directory. `code` is
    /// the error code every failure here is reported under.
    pub fn new(code: &'static str) -> Result<Self, PipelineError> {
        let root = std::env::temp_dir()
            .join("zebflow-scratch")
            .join(uuid::Uuid::new_v4().to_string());
        std::fs::create_dir_all(&root)
            .map_err(|err| PipelineError::new(code, format!("create scratch folder: {err}")))?;
        Ok(Self { root, code })
    }

    pub fn path(&self) -> &Path {
        &self.root
    }

    /// Where `rel` lives inside the scratch folder.
    pub fn local(&self, rel: &str) -> PathBuf {
        self.root.join(rel.trim_start_matches('/'))
    }

    fn err(&self, message: String) -> PipelineError {
        PipelineError::new(self.code, message)
    }

    /// Copies one object, or every object under a prefix, from the store into
    /// the scratch folder at the same relative path. Answers the local path.
    pub fn pull(&self, store: &ZebFs, rel: &str) -> Result<PathBuf, PipelineError> {
        let rel = rel.trim_matches('/');
        match store.get(rel) {
            Ok(object) => {
                self.write_local(rel, &object.bytes)?;
                return Ok(self.local(rel));
            }
            Err(err) if err.code == "ZEBFS_NOT_FOUND" => {}
            Err(err) => return Err(self.err(format!("read '{rel}': {err}"))),
        }
        let mut found = false;
        self.pull_prefix(store, rel, &mut found)?;
        if !found {
            return Err(self.err(format!("source path not found: {rel}")));
        }
        Ok(self.local(rel))
    }

    /// Like [`Self::pull`], plus every object beside it sharing its stem:
    /// a shapefile's `.shx`, `.dbf` and `.prj` travel with its `.shp`.
    pub fn pull_with_siblings(&self, store: &ZebFs, rel: &str) -> Result<PathBuf, PipelineError> {
        let local = self.pull(store, rel)?;
        let rel = rel.trim_matches('/');
        let (parent, name) = match rel.rsplit_once('/') {
            Some((parent, name)) => (parent, name),
            None => ("", rel),
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
                let object = store
                    .get(&entry.path)
                    .map_err(|err| self.err(format!("read '{}': {err}", entry.path)))?;
                self.write_local(&entry.path, &object.bytes)?;
            }
        }
        Ok(local)
    }

    fn pull_prefix(&self, store: &ZebFs, prefix: &str, found: &mut bool) -> Result<(), PipelineError> {
        let entries = match store.list(prefix) {
            Ok(entries) => entries,
            Err(err) if err.code == "ZEBFS_NOT_FOUND" || err.code == "ZEBFS_NOT_PREFIX" => {
                return Ok(());
            }
            Err(err) => return Err(self.err(format!("list '{prefix}': {err}"))),
        };
        if !entries.is_empty() || store.head(prefix).is_ok() {
            *found = true;
            std::fs::create_dir_all(self.local(prefix))
                .map_err(|err| self.err(format!("create '{prefix}': {err}")))?;
        }
        for entry in entries {
            match entry.kind {
                ZebFsEntryKind::Prefix => self.pull_prefix(store, &entry.path, found)?,
                ZebFsEntryKind::Object => {
                    let object = store
                        .get(&entry.path)
                        .map_err(|err| self.err(format!("read '{}': {err}", entry.path)))?;
                    self.write_local(&entry.path, &object.bytes)?;
                }
            }
        }
        Ok(())
    }

    fn write_local(&self, rel: &str, bytes: &[u8]) -> Result<(), PipelineError> {
        let path = self.local(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|err| self.err(format!("create '{}': {err}", parent.display())))?;
        }
        std::fs::write(&path, bytes).map_err(|err| self.err(format!("write scratch '{rel}': {err}")))
    }

    /// Stores one file the engine wrote at `rel` in the project's store.
    pub fn push_file(&self, store: &ZebFs, local: &Path, rel: &str) -> Result<Vec<u8>, PipelineError> {
        let bytes = std::fs::read(local)
            .map_err(|err| self.err(format!("read '{}': {err}", local.display())))?;
        store
            .put(rel, &bytes)
            .map_err(|err| self.err(format!("write '{rel}': {err}")))?;
        Ok(bytes)
    }

    /// Stores every file under `local_dir` in the project's store beneath
    /// `rel_prefix`, keeping their relative layout. Answers the store paths.
    pub fn push_tree(&self, store: &ZebFs, local_dir: &Path, rel_prefix: &str) -> Result<Vec<String>, PipelineError> {
        let mut written = Vec::new();
        self.push_tree_inner(store, local_dir, local_dir, rel_prefix.trim_matches('/'), &mut written)?;
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
}
