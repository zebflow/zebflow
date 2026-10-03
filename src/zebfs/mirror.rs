//! A bounded local mirror of store objects, for engines that read a path.
//!
//! GDAL, DataFusion and the map service open files by path and seek in them,
//! so a tile request cannot read a bucket object through `get`. A directory
//! store already has a path and answers it in place; a bucket object is
//! streamed once into the mirror and answered from there until the object
//! changes. The mirror is a cache: anything in it can be deleted and is
//! fetched again, and it never holds more than its cap beyond the one object
//! a request is reading.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant, SystemTime};

use sha2::{Digest, Sha256};

use super::error::ZebFsError;
use super::store::ZebFs;

/// How long a checked object is trusted without asking the store again.
const RECHECK_AFTER: Duration = Duration::from_secs(30);

/// One mirror directory, usually a project's `data/cache/zebfs-mirror/`.
pub struct Mirror {
    root: PathBuf,
    cap_bytes: u64,
}

impl Mirror {
    pub fn new(root: PathBuf, cap_bytes: u64) -> Self {
        Self { root, cap_bytes }
    }

    /// A local path holding the bytes of `key` in `store`. `store_id` names
    /// the store, so one key in two stores never shares an entry.
    pub fn path(&self, store: &ZebFs, store_id: &str, key: &str) -> Result<PathBuf, ZebFsError> {
        if let Some(local) = store.local_path(key)? {
            return if local.is_file() {
                Ok(local)
            } else {
                Err(ZebFsError::new("ZEBFS_NOT_FOUND", format!("'{key}' is not in the store")))
            };
        }
        let entry = self.entry_path(store_id, key);
        let lock = entry_lock(&entry);
        let _held = lock.lock().unwrap_or_else(|poisoned| poisoned.into_inner());

        if entry.is_file() && recently_checked(&entry) {
            touch(&entry);
            return Ok(entry);
        }
        let stat = store.head(key)?;
        let stamp = stamp_of(stat.size, stat.modified);
        if entry.is_file() && std::fs::read_to_string(stamp_path(&entry)).ok().as_deref() == Some(stamp.as_str()) {
            mark_checked(&entry);
            touch(&entry);
            return Ok(entry);
        }

        std::fs::create_dir_all(&self.root)?;
        let partial = entry.with_extension(format!("partial-{}", uuid::Uuid::new_v4()));
        if let Err(err) = store.get_to_file(key, &partial) {
            let _ = std::fs::remove_file(&partial);
            return Err(err);
        }
        std::fs::rename(&partial, &entry)?;
        std::fs::write(stamp_path(&entry), &stamp)?;
        mark_checked(&entry);
        self.evict_beyond_cap(&entry);
        Ok(entry)
    }

    /// Where an object's copy lives: a digest of store and key, keeping the
    /// key's extension so engines that sniff by name still see `.parquet`.
    fn entry_path(&self, store_id: &str, key: &str) -> PathBuf {
        let digest = hex::encode(Sha256::digest(format!("{store_id}\n{key}").as_bytes()));
        let ext = Path::new(key)
            .extension()
            .and_then(|e| e.to_str())
            .filter(|e| e.len() <= 16 && e.bytes().all(|b| b.is_ascii_alphanumeric()))
            .map(|e| format!(".{}", e.to_ascii_lowercase()))
            .unwrap_or_default();
        self.root.join(format!("{digest}{ext}"))
    }

    /// Removes the least recently read copies until the mirror fits its cap.
    /// `keep` is the copy just written for the current request.
    fn evict_beyond_cap(&self, keep: &Path) {
        let Ok(dir) = std::fs::read_dir(&self.root) else { return };
        let mut copies: Vec<(PathBuf, u64, SystemTime)> = dir
            .flatten()
            .filter_map(|entry| {
                let path = entry.path();
                let name = path.file_name()?.to_str()?;
                if name.ends_with(".stamp") || name.contains(".partial-") {
                    return None;
                }
                let meta = entry.metadata().ok()?;
                Some((path, meta.len(), meta.modified().ok()?))
            })
            .collect();
        let mut total: u64 = copies.iter().map(|(_, size, _)| size).sum();
        copies.sort_by_key(|(_, _, modified)| *modified);
        for (path, size, _) in copies {
            if total <= self.cap_bytes {
                break;
            }
            if path == keep {
                continue;
            }
            if std::fs::remove_file(&path).is_ok() {
                let _ = std::fs::remove_file(stamp_path(&path));
                total = total.saturating_sub(size);
            }
        }
    }
}

fn stamp_of(size: u64, modified: Option<SystemTime>) -> String {
    let modified = modified
        .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{size}:{modified}")
}

fn stamp_path(entry: &Path) -> PathBuf {
    let mut name = entry.as_os_str().to_os_string();
    name.push(".stamp");
    PathBuf::from(name)
}

/// Marks a copy as just read, which is what eviction orders by.
fn touch(entry: &Path) {
    if let Ok(file) = std::fs::File::options().append(true).open(entry) {
        let _ = file.set_modified(SystemTime::now());
    }
}

fn entry_lock(entry: &Path) -> Arc<Mutex<()>> {
    static LOCKS: OnceLock<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>> = OnceLock::new();
    let mut locks = LOCKS.get_or_init(Default::default).lock().unwrap_or_else(|p| p.into_inner());
    locks.entry(entry.to_path_buf()).or_default().clone()
}

fn checked() -> &'static Mutex<HashMap<PathBuf, Instant>> {
    static CHECKED: OnceLock<Mutex<HashMap<PathBuf, Instant>>> = OnceLock::new();
    CHECKED.get_or_init(Default::default)
}

fn recently_checked(entry: &Path) -> bool {
    checked()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .get(entry)
        .is_some_and(|at| at.elapsed() < RECHECK_AFTER)
}

fn mark_checked(entry: &Path) {
    checked().lock().unwrap_or_else(|p| p.into_inner()).insert(entry.to_path_buf(), Instant::now());
}

#[cfg(test)]
mod tests {
    use super::Mirror;
    use crate::zebfs::{LocalZebFs, ZebFs};

    #[test]
    fn a_directory_store_answers_in_place_and_nothing_is_copied() {
        let files = tempfile::tempdir().unwrap();
        let cache = tempfile::tempdir().unwrap();
        let store = ZebFs::Local(LocalZebFs::new(files.path().to_path_buf()));
        store.put("maps/a.parquet", b"abc").unwrap();
        let mirror = Mirror::new(cache.path().join("m"), 10);
        let path = mirror.path(&store, "local", "maps/a.parquet").unwrap();
        assert!(path.starts_with(files.path()));
        assert!(!cache.path().join("m").exists());
        assert!(mirror.path(&store, "local", "maps/missing.parquet").is_err());
        assert!(mirror.path(&store, "local", "../escape").is_err());
    }

    #[test]
    fn a_bucket_object_is_fetched_once_and_again_when_it_changes() {
        let (port, objects) = crate::zebfs::s3::tests::fake_s3();
        let store = ZebFs::S3(crate::zebfs::s3::tests::store(port, "p"));
        store.put("maps/a.parquet", b"first").unwrap();
        let cache = tempfile::tempdir().unwrap();
        let mirror = Mirror::new(cache.path().to_path_buf(), 1024);

        let path = mirror.path(&store, "s3-a", "maps/a.parquet").unwrap();
        assert!(path.starts_with(cache.path()));
        assert_eq!(std::fs::read(&path).unwrap(), b"first");

        // Changed in the bucket: a fresh check sees the new size and fetches.
        objects.lock().unwrap().insert("p/maps/a.parquet".into(), b"second!".to_vec());
        super::checked().lock().unwrap().clear();
        let again = mirror.path(&store, "s3-a", "maps/a.parquet").unwrap();
        assert_eq!(again, path);
        assert_eq!(std::fs::read(&again).unwrap(), b"second!");

        // Gone from the bucket: the copy is not answered as if it were there.
        objects.lock().unwrap().clear();
        super::checked().lock().unwrap().clear();
        assert!(mirror.path(&store, "s3-a", "maps/a.parquet").is_err());
    }

    #[test]
    fn the_mirror_stays_within_its_cap() {
        let (port, _objects) = crate::zebfs::s3::tests::fake_s3();
        let store = ZebFs::S3(crate::zebfs::s3::tests::store(port, "p"));
        let cache = tempfile::tempdir().unwrap();
        let mirror = Mirror::new(cache.path().to_path_buf(), 10);
        for name in ["a", "b", "c"] {
            store.put(&format!("{name}.bin"), b"123456").unwrap();
            mirror.path(&store, "s3", &format!("{name}.bin")).unwrap();
        }
        let copies = std::fs::read_dir(cache.path())
            .unwrap()
            .flatten()
            .filter(|e| e.path().extension().is_some_and(|x| x == "bin"))
            .count();
        assert_eq!(copies, 1, "two copies of six bytes do not fit in ten");
    }

    #[test]
    fn entries_keep_the_extension_and_differ_per_store() {
        let mirror = Mirror::new("/tmp/m".into(), 10);
        let a = mirror.entry_path("local", "x/a.parquet");
        let b = mirror.entry_path("s3-other", "x/a.parquet");
        assert_ne!(a, b);
        assert_eq!(a.extension().unwrap(), "parquet");
        assert_eq!(mirror.entry_path("s", "noext").extension(), None);
    }

    #[test]
    fn eviction_drops_the_oldest_copies_but_never_the_one_in_use() {
        let dir = tempfile::tempdir().unwrap();
        let mirror = Mirror::new(dir.path().to_path_buf(), 5);
        for (name, age) in [("old.bin", 300), ("mid.bin", 200), ("new.bin", 100)] {
            let path = dir.path().join(name);
            std::fs::write(&path, b"1234").unwrap();
            std::fs::write(super::stamp_path(&path), "4:0").unwrap();
            let file = std::fs::File::options().append(true).open(&path).unwrap();
            file.set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(age)).unwrap();
        }
        mirror.evict_beyond_cap(&dir.path().join("old.bin"));
        assert!(dir.path().join("old.bin").exists(), "the copy being read stays");
        assert!(!dir.path().join("mid.bin").exists(), "oldest beside it goes first");
        assert!(!dir.path().join("new.bin").exists(), "and on until the mirror fits");
        assert!(!dir.path().join("mid.bin.stamp").exists());
    }
}
