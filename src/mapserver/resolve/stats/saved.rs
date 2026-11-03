//! A layer's saved statistics: one small JSON file, written whole or not at
//! all, that names the version of the source file it describes.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// The saved file's name inside the layer's cache folder.
pub const STATS_FILE_NAME: &str = "stats.json";
/// The saved file's format. A file of another version is computed again.
pub const SAVED_VERSION: u32 = 2;

/// Which version of a source file statistics describe: its size and a digest
/// of its last bytes, where a Parquet file keeps its footer (schema, row
/// groups and their offsets). Not its modification time: a bucket's mirror
/// copy is marked as used on every read by moving that time, which would make
/// saved statistics look stale on every request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Fingerprint {
    pub size: u64,
    pub tail_sha256: String,
}

/// How many trailing bytes the fingerprint reads.
const TAIL_BYTES: u64 = 64 * 1024;

impl Fingerprint {
    /// The fingerprint of the file at `path`, or `None` when it is not a
    /// readable file.
    pub fn of(path: &Path) -> Option<Self> {
        use sha2::{Digest, Sha256};
        use std::io::{Read, Seek, SeekFrom};
        let mut file = std::fs::File::open(path).ok()?;
        let meta = file.metadata().ok()?;
        if !meta.is_file() {
            return None;
        }
        let size = meta.len();
        let tail = size.min(TAIL_BYTES);
        file.seek(SeekFrom::Start(size - tail)).ok()?;
        let mut bytes = Vec::with_capacity(tail as usize);
        file.take(tail).read_to_end(&mut bytes).ok()?;
        Some(Self {
            size,
            tail_sha256: hex::encode(Sha256::digest(&bytes)),
        })
    }
}

/// One column's statistics, in the shape `/stats` answers.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColumnStats {
    pub name: String,
    pub data_type: String,
    pub null_count: u64,
    /// Distinct non-null values: exact when `cardinality_exact`, else an estimate.
    pub cardinality: u64,
    pub cardinality_exact: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
    /// The most frequent values with their exact counts, most frequent first.
    pub top_values: Vec<(String, u64)>,
    /// Why `top_values` is empty when it was not computed:
    /// `too_many_distinct` or `unsupported_type`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub top_values_skipped: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub histogram: Option<Histogram>,
}

/// Equal-width bins from `min` to `max`; the last bin includes `max`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Histogram {
    pub min: f64,
    pub max: f64,
    pub counts: Vec<u64>,
}

/// The statistics of a whole layer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct LayerStats {
    pub row_count: u64,
    /// `[min x, min y, max x, max y]`, or `None` when the file does not say.
    pub extent: Option<[f64; 4]>,
    pub columns: Vec<ColumnStats>,
}

/// How a computation ended.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum Outcome {
    Ready(LayerStats),
    Failed { error: String },
}

/// The saved file.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SavedStats {
    pub version: u32,
    pub source: Fingerprint,
    /// UTC, RFC 3339.
    pub computed_at: String,
    #[serde(flatten)]
    pub outcome: Outcome,
}

impl SavedStats {
    pub fn new(source: Fingerprint, result: Result<LayerStats, String>) -> Self {
        Self {
            version: SAVED_VERSION,
            source,
            computed_at: now_rfc3339(),
            outcome: match result {
                Ok(stats) => Outcome::Ready(stats),
                Err(error) => Outcome::Failed { error },
            },
        }
    }
}

/// The current time, UTC, to the second.
pub fn now_rfc3339() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// The saved statistics at `path`, or `None` when there are none, or they
/// cannot be read, or they are of another format version.
pub fn read(path: &Path) -> Option<SavedStats> {
    let bytes = std::fs::read(path).ok()?;
    let saved: SavedStats = serde_json::from_slice(&bytes).ok()?;
    (saved.version == SAVED_VERSION).then_some(saved)
}

/// Writes `saved` to `path` through a temporary file in the same folder and
/// a rename, so a reader sees the old file or the new one, never half of one.
pub fn write(path: &Path, saved: &SavedStats) -> Result<(), String> {
    let dir = path
        .parent()
        .ok_or_else(|| format!("'{}' has no folder", path.display()))?;
    std::fs::create_dir_all(dir).map_err(|e| format!("create '{}': {e}", dir.display()))?;
    let mut tmp = tempfile::NamedTempFile::new_in(dir)
        .map_err(|e| format!("temporary file in '{}': {e}", dir.display()))?;
    serde_json::to_writer(&mut tmp, saved).map_err(|e| format!("encode statistics: {e}"))?;
    tmp.as_file()
        .sync_all()
        .map_err(|e| format!("flush statistics: {e}"))?;
    tmp.persist(path)
        .map_err(|e| format!("save '{}': {}", path.display(), e.error))?;
    Ok(())
}
