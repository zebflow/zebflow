//! Content-addressed script cache for platform-managed external RWE assets.
//!
//! Design:
//!
//! - disk is the durable source of truth (`{hash}.blob`)
//! - memory is a bounded hot cache (LRU by bytes)
//! - callers can persist `CompiledScript` outputs without holding all content
//!   in RAM when many pages/templates exist

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::rwe::CompiledScript;

/// Configuration for [`RenderScriptCache`].
#[derive(Debug, Clone)]
pub struct ScriptCacheConfig {
    /// Cache root directory where script blobs are stored.
    pub root: PathBuf,
    /// Max in-memory bytes for hot script bodies.
    pub memory_budget_bytes: usize,
}

impl ScriptCacheConfig {
    pub fn new(root: PathBuf, memory_budget_bytes: usize) -> Self {
        Self {
            root,
            memory_budget_bytes,
        }
    }
}

/// Metadata returned after storing one script artifact.
#[derive(Debug, Clone)]
pub struct CachedScriptRef {
    pub content_hash: String,
    pub content_type: String,
    pub size_bytes: usize,
    pub file_path: PathBuf,
}

#[derive(Default)]
struct CacheState {
    bytes_used: usize,
    by_hash: HashMap<String, Arc<String>>,
    lru: VecDeque<String>,
}

/// Bounded in-memory + disk-backed script cache.
pub struct RenderScriptCache {
    root: PathBuf,
    memory_budget_bytes: usize,
    state: Mutex<CacheState>,
}

impl RenderScriptCache {
    /// Creates cache and ensures disk root exists.
    pub fn new(config: ScriptCacheConfig) -> io::Result<Self> {
        fs::create_dir_all(&config.root)?;
        Ok(Self {
            root: config.root,
            memory_budget_bytes: config.memory_budget_bytes,
            state: Mutex::new(CacheState::default()),
        })
    }

    /// Stores one script in disk CAS and hot memory cache.
    pub fn store(&self, script: &CompiledScript) -> io::Result<CachedScriptRef> {
        let hash = script.content_hash.trim();
        if hash.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "compiled script content_hash must not be empty",
            ));
        }
        self.rotate_global_by_script_id(&script.id, hash)?;
        let path = self.path_for_hash(hash);
        if !path.exists() {
            self.write_blob_atomically(&path, script.content.as_bytes())?;
        }
        self.remember(hash.to_string(), Arc::new(script.content.clone()));
        Ok(CachedScriptRef {
            content_hash: hash.to_string(),
            content_type: script.content_type.clone(),
            size_bytes: script.content.len(),
            file_path: path,
        })
    }

    /// Stores one script in project-scoped disk CAS and hot memory cache.
    pub fn store_scoped(
        &self,
        owner: &str,
        project: &str,
        script: &CompiledScript,
    ) -> io::Result<CachedScriptRef> {
        let hash = script.content_hash.trim();
        if hash.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "compiled script content_hash must not be empty",
            ));
        }
        let owner = sanitize_scope_segment(owner)?;
        let project = sanitize_scope_segment(project)?;
        self.rotate_scoped_by_script_id(&owner, &project, &script.id, hash)?;
        let path = self.path_for_scope_hash(&owner, &project, hash)?;
        if !path.exists() {
            self.write_blob_atomically(&path, script.content.as_bytes())?;
        }
        let cache_key = scoped_cache_key(&owner, &project, hash)?;
        self.remember(cache_key, Arc::new(script.content.clone()));
        Ok(CachedScriptRef {
            content_hash: hash.to_string(),
            content_type: script.content_type.clone(),
            size_bytes: script.content.len(),
            file_path: path,
        })
    }

    /// Retrieves one script by content hash.
    ///
    /// Order:
    ///
    /// 1. in-memory hot cache
    /// 2. disk CAS blob load + hot-cache promote
    pub fn get(&self, content_hash: &str) -> io::Result<Option<String>> {
        let hash = content_hash.trim();
        if hash.is_empty() {
            return Ok(None);
        }
        if let Some(hit) = self.get_hot(hash) {
            return Ok(Some((*hit).clone()));
        }
        let path = self.path_for_hash(hash);
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(path)?;
        self.remember(hash.to_string(), Arc::new(content.clone()));
        Ok(Some(content))
    }

    /// Retrieves one project-scoped script by content hash.
    pub fn get_scoped(
        &self,
        owner: &str,
        project: &str,
        content_hash: &str,
    ) -> io::Result<Option<String>> {
        let hash = content_hash.trim();
        if hash.is_empty() {
            return Ok(None);
        }
        let cache_key = scoped_cache_key(owner, project, hash)?;
        if let Some(hit) = self.get_hot(&cache_key) {
            return Ok(Some((*hit).clone()));
        }
        let path = self.path_for_scope_hash(owner, project, hash)?;
        if !path.exists() {
            return Ok(None);
        }
        let content = fs::read_to_string(path)?;
        self.remember(cache_key, Arc::new(content.clone()));
        Ok(Some(content))
    }

    /// Returns blob path for one content hash.
    pub fn path_for_hash(&self, content_hash: &str) -> PathBuf {
        self.root.join(format!("{}.blob", content_hash.trim()))
    }

    /// Returns project-scoped blob path for one content hash.
    pub fn path_for_scope_hash(
        &self,
        owner: &str,
        project: &str,
        content_hash: &str,
    ) -> io::Result<PathBuf> {
        let owner = sanitize_scope_segment(owner)?;
        let project = sanitize_scope_segment(project)?;
        Ok(self
            .root
            .join("projects")
            .join(owner)
            .join(project)
            .join(format!("{}.blob", content_hash.trim())))
    }

    fn get_hot(&self, content_hash: &str) -> Option<Arc<String>> {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let hit = state.by_hash.get(content_hash).cloned();
        if hit.is_some() {
            touch_lru(&mut state.lru, content_hash);
        }
        hit
    }

    fn remember(&self, hash: String, content: Arc<String>) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        let size = content.len();

        if let Some(prev) = state.by_hash.insert(hash.clone(), content) {
            state.bytes_used = state.bytes_used.saturating_sub(prev.len());
        }
        state.bytes_used += size;
        touch_lru(&mut state.lru, &hash);

        while state.bytes_used > self.memory_budget_bytes {
            let Some(oldest) = state.lru.pop_front() else {
                break;
            };
            if oldest == hash {
                // Keep most recently inserted entry even when budget is tiny.
                state.lru.push_back(oldest);
                break;
            }
            if let Some(evicted) = state.by_hash.remove(&oldest) {
                state.bytes_used = state.bytes_used.saturating_sub(evicted.len());
            }
        }
    }

    fn forget_hot(&self, key: &str) {
        let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(prev) = state.by_hash.remove(key) {
            state.bytes_used = state.bytes_used.saturating_sub(prev.len());
        }
        if let Some(idx) = state.lru.iter().position(|item| item == key) {
            let _ = state.lru.remove(idx);
        }
    }

    fn write_blob_atomically(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        let parent = path
            .parent()
            .ok_or_else(|| io::Error::other("invalid cache path"))?;
        fs::create_dir_all(parent)?;

        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        let tmp = parent.join(format!(".tmp-{}-{}", std::process::id(), now));
        fs::write(&tmp, bytes)?;
        fs::rename(tmp, path)?;
        Ok(())
    }

    fn rotate_global_by_script_id(&self, script_id: &str, next_hash: &str) -> io::Result<()> {
        let index_path = self.global_script_index_path(script_id)?;
        if let Some(prev_hash) = self.read_hash_index(&index_path)? {
            let prev_hash = prev_hash.trim();
            if !prev_hash.is_empty() && prev_hash != next_hash {
                let old_key = prev_hash.to_string();
                let old_path = self.path_for_hash(prev_hash);
                if !self.hash_is_referenced_elsewhere(
                    index_path.parent(),
                    index_path.file_name(),
                    prev_hash,
                )? {
                    let _ = fs::remove_file(old_path);
                }
                self.forget_hot(&old_key);
            }
        }
        self.write_hash_index(&index_path, next_hash)?;
        Ok(())
    }

    fn rotate_scoped_by_script_id(
        &self,
        owner: &str,
        project: &str,
        script_id: &str,
        next_hash: &str,
    ) -> io::Result<()> {
        let scope_root = self.root.join("projects").join(owner).join(project);
        let index_path = self.scoped_script_index_path(owner, project, script_id)?;
        if let Some(prev_hash) = self.read_hash_index(&index_path)? {
            let prev_hash = prev_hash.trim();
            if !prev_hash.is_empty() && prev_hash != next_hash {
                let old_key = scoped_cache_key(owner, project, prev_hash)?;
                let old_path = scope_root.join(format!("{prev_hash}.blob"));
                if !self.hash_is_referenced_elsewhere(
                    index_path.parent(),
                    index_path.file_name(),
                    prev_hash,
                )? {
                    let _ = fs::remove_file(old_path);
                }
                self.forget_hot(&old_key);
            }
        }
        self.write_hash_index(&index_path, next_hash)?;
        Ok(())
    }

    fn global_script_index_path(&self, script_id: &str) -> io::Result<PathBuf> {
        let file = sanitize_script_id(script_id)?;
        Ok(self.root.join("index").join(format!("{file}.hash")))
    }

    fn scoped_script_index_path(
        &self,
        owner: &str,
        project: &str,
        script_id: &str,
    ) -> io::Result<PathBuf> {
        let owner = sanitize_scope_segment(owner)?;
        let project = sanitize_scope_segment(project)?;
        let file = sanitize_script_id(script_id)?;
        Ok(self
            .root
            .join("projects")
            .join(owner)
            .join(project)
            .join("index")
            .join(format!("{file}.hash")))
    }

    fn read_hash_index(&self, index_path: &Path) -> io::Result<Option<String>> {
        if !index_path.exists() {
            return Ok(None);
        }
        let raw = fs::read_to_string(index_path)?;
        let value = raw.trim().to_string();
        if value.is_empty() {
            return Ok(None);
        }
        Ok(Some(value))
    }

    fn write_hash_index(&self, index_path: &Path, hash: &str) -> io::Result<()> {
        self.write_blob_atomically(index_path, hash.as_bytes())
    }

    fn hash_is_referenced_elsewhere(
        &self,
        index_dir: Option<&Path>,
        exclude_file_name: Option<&std::ffi::OsStr>,
        hash: &str,
    ) -> io::Result<bool> {
        let Some(index_dir) = index_dir else {
            return Ok(false);
        };
        if !index_dir.exists() {
            return Ok(false);
        }

        for entry in fs::read_dir(index_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("hash") {
                continue;
            }
            if let (Some(exclude), Some(name)) = (exclude_file_name, path.file_name())
                && name == exclude
            {
                continue;
            }
            let value = fs::read_to_string(&path).unwrap_or_default();
            if value.trim() == hash {
                return Ok(true);
            }
        }
        Ok(false)
    }
}

fn touch_lru(lru: &mut VecDeque<String>, hash: &str) {
    if let Some(idx) = lru.iter().position(|h| h == hash) {
        let _ = lru.remove(idx);
    }
    lru.push_back(hash.to_string());
}

fn sanitize_scope_segment(raw: &str) -> io::Result<String> {
    let segment = raw.trim();
    if segment.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "scope segment must not be empty",
        ));
    }
    if !segment
        .bytes()
        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "scope segment contains unsupported characters",
        ));
    }
    Ok(segment.to_string())
}

fn scoped_cache_key(owner: &str, project: &str, hash: &str) -> io::Result<String> {
    let owner = sanitize_scope_segment(owner)?;
    let project = sanitize_scope_segment(project)?;
    Ok(format!("{owner}/{project}/{hash}"))
}

fn sanitize_script_id(raw: &str) -> io::Result<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "script id must not be empty",
        ));
    }

    let mut out = String::with_capacity(trimmed.len());
    for ch in trimmed.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push_str("script");
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rwe::CompiledScriptScope;

    fn tmp_root(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos();
        std::env::temp_dir().join(format!("zebflow-rwe-script-cache-{label}-{nanos}"))
    }

    fn script(hash: &str, content: &str) -> CompiledScript {
        script_with_id("page", hash, content)
    }

    fn script_with_id(id: &str, hash: &str, content: &str) -> CompiledScript {
        CompiledScript {
            id: id.to_string(),
            scope: CompiledScriptScope::Page,
            content_type: "text/javascript; charset=utf-8".to_string(),
            content: content.to_string(),
            content_hash: hash.to_string(),
            suggested_file_name: "page.js".to_string(),
        }
    }

    #[test]
    fn script_cache_persists_to_disk_and_reads_back() {
        let root = tmp_root("disk");
        let cache = RenderScriptCache::new(ScriptCacheConfig::new(root.clone(), 8))
            .expect("create script cache");

        let entry = script("h1", "console.log('one')");
        let stored = cache.store(&entry).expect("store");
        assert!(stored.file_path.exists());

        let loaded = cache.get("h1").expect("get").expect("content exists");
        assert_eq!(loaded, "console.log('one')");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn script_cache_evicts_hot_entries_by_budget() {
        let root = tmp_root("evict");
        let cache = RenderScriptCache::new(ScriptCacheConfig::new(root.clone(), 10))
            .expect("create script cache");

        let _ = cache
            .store(&script_with_id("page-a", "a", "1234567890"))
            .expect("store a");
        let _ = cache
            .store(&script_with_id("page-b", "b", "abcdefghij"))
            .expect("store b");

        // `a` should still be available from disk even when evicted from hot cache.
        let a = cache.get("a").expect("read a").expect("a exists");
        assert_eq!(a, "1234567890");

        let b = cache.get("b").expect("read b").expect("b exists");
        assert_eq!(b, "abcdefghij");

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn scoped_store_rotates_old_hash_for_same_script_id() {
        let root = tmp_root("rotate");
        let cache = RenderScriptCache::new(ScriptCacheConfig::new(root.clone(), 128))
            .expect("create script cache");

        let mut v1 = script("h1", "console.log('one')");
        v1.id = "rwe.page".to_string();
        cache
            .store_scoped("superadmin", "default", &v1)
            .expect("store scoped v1");

        let old_path = cache
            .path_for_scope_hash("superadmin", "default", "h1")
            .expect("old path");
        assert!(old_path.exists());

        let mut v2 = script("h2", "console.log('two')");
        v2.id = "rwe.page".to_string();
        cache
            .store_scoped("superadmin", "default", &v2)
            .expect("store scoped v2");

        assert!(
            !old_path.exists(),
            "old scoped hash blob should be deleted after script content changed"
        );
        let new_path = cache
            .path_for_scope_hash("superadmin", "default", "h2")
            .expect("new path");
        assert!(new_path.exists(), "new scoped hash blob should exist");

        let loaded = cache
            .get_scoped("superadmin", "default", "h2")
            .expect("get scoped")
            .expect("exists");
        assert_eq!(loaded, "console.log('two')");

        let _ = fs::remove_dir_all(root);
    }
}
