//! The project's embedded multimodel store, on sekejap 0.17.
//!
//! One `Db` per project directory, pooled by path. sekejap locks internally
//! and is `Send + Sync`, so the pool hands out `Arc<Db>` and nothing here
//! wraps it in a lock of its own. Every call that changes rows commits
//! before it returns; a batch (`bulk_insert`) takes one transaction so the
//! records and their edges land under one barrier or not at all.
//!
//! What this module owns, and sekejap does not: the project directory and
//! the pool, the schema document mirrored into the repository, the managed
//! table shape the Studio edits (`SimpleTableDefinition`), and the SQL the
//! Studio's own actions emit. The dialect itself is sekejap's
//! (`docs/lang/QL_CONTRACT.md` in that repository); the help page
//! `db/sekejap` is the short form for pipeline authors.
//!
//! A store written by sekejap 0.16 cannot be opened by 0.17 — the on-disk
//! format changed whole. `ensure_project_dir` recognises the old files and
//! refuses by name when they hold rows, so nothing is ever opened halfway.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Instant;

use sekejap::{Db, FieldKind, IndexFamily};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::contracts::kinds::DatabaseSchemaContract;
use crate::contracts::{ContractMetadata, decode_contract, encode_contract};
use crate::infra::io::durable::atomic_write;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    CollectionAttribute, CreateSimpleTableRequest, DbObjectNode, DbQueryColumn,
    ProjectDbConnectionQueryResult, QueryProjectDbConnectionRequest, ResolvedProjectLayout,
    SCHEMA_DOCUMENT_FILE, SimpleTableDefinition, UpdateSimpleTableRequest, slug_segment,
};

// ── the pool ─────────────────────────────────────────────────────────────

type DbPool = HashMap<PathBuf, Arc<Db>>;
type MaintenancePool = HashMap<PathBuf, SekejapAutoMaintenanceState>;

static POOL: OnceLock<Mutex<DbPool>> = OnceLock::new();
static MAINTENANCE: OnceLock<Mutex<MaintenancePool>> = OnceLock::new();

const AUTO_CHECKPOINT_WRITE_UNITS: usize = 10_000;
const AUTO_CHECKPOINT_WAL_BYTES: u64 = 64 * 1024 * 1024;

/// The files a sekejap 0.16 store left behind. Any of them beside no `data`
/// file means the directory is a 0.16 store, and one that holds rows is
/// refused rather than overwritten.
const LEGACY_FILES: [&str; 5] = [
    "wal.log",
    "snapshot.json",
    "payloads.bin",
    "gin.bin",
    "edge_meta.bin",
];

#[derive(Debug, Default)]
struct SekejapAutoMaintenanceState {
    write_units_since_checkpoint: usize,
}

fn pool() -> &'static Mutex<DbPool> {
    POOL.get_or_init(|| Mutex::new(HashMap::new()))
}

fn maintenance_pool() -> &'static Mutex<MaintenancePool> {
    MAINTENANCE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Any sekejap failure as a platform error under one code.
fn store_error(code: &'static str, err: impl std::fmt::Display) -> PlatformError {
    PlatformError::new(code, err.to_string())
}

fn get_db(data_root: &Path, owner: &str, project: &str) -> Result<Arc<Db>, PlatformError> {
    let dir = ensure_project_dir(data_root, owner, project)?;
    let mut map = pool().lock().unwrap_or_else(|e| e.into_inner());
    if let Some(db) = map.get(&dir) {
        return Ok(Arc::clone(db));
    }
    // SERVICE mode: one writer, parallel readers on a published snapshot.
    // Zebflow is a long-running server answering many requests at once,
    // which is the shape this mode is for; single mode would put every read
    // and write through one mutex. The publish interval starts at zero, so
    // a commit is visible to the next reader.
    let db = Db::open_service(&dir).map_err(|err| {
        PlatformError::new(
            "PLATFORM_SEKEJAP_OPEN",
            format!("failed to open sekejap store at {}: {err}", dir.display()),
        )
    })?;
    let arc = Arc::new(db);
    map.insert(dir, Arc::clone(&arc));
    Ok(arc)
}

/// Fold the committed WAL into the data file.
///
/// In service mode `Db::checkpoint` is always DEFERRED: the published read
/// view holds a reader slot for its whole life, and a slot out defers the
/// fold (`docs/dist/OPS_CONTRACT.md` §1). So the fold is done the one way it
/// can be: the pooled service handle is closed, which releases the view;
/// the store is opened for a moment in single mode, where a checkpoint
/// folds; and the service is reopened for the next caller.
///
/// A handle a request is still using cannot be closed — `Arc::try_unwrap`
/// says so — and then this answers `Ok(false)`: deferred, not failed, and
/// the next call tries again. That is what a low-traffic maintenance window
/// is for.
fn fold_wal(dir: &Path) -> Result<bool, PlatformError> {
    let taken = {
        let mut map = pool().lock().unwrap_or_else(|e| e.into_inner());
        map.remove(dir)
    };
    if let Some(shared) = taken {
        match Arc::try_unwrap(shared) {
            Ok(db) => db
                .close()
                .map_err(|e| store_error("PLATFORM_SEKEJAP_COMPACT", e))?,
            Err(shared) => {
                // Somebody is mid-call on it. Put it back and defer.
                pool()
                    .lock()
                    .unwrap_or_else(|e| e.into_inner())
                    .insert(dir.to_path_buf(), shared);
                return Ok(false);
            }
        }
    }
    let single = Db::open(dir).map_err(|e| store_error("PLATFORM_SEKEJAP_COMPACT", e))?;
    let folded = single
        .checkpoint()
        .map_err(|e| store_error("PLATFORM_SEKEJAP_COMPACT", e))?;
    single
        .close()
        .map_err(|e| store_error("PLATFORM_SEKEJAP_COMPACT", e))?;
    Ok(folded)
}

/// Record project write pressure and checkpoint the WAL when it is large
/// enough.
///
/// Best-effort on purpose: the write already committed, so maintenance must
/// not turn an accepted mutation into a failed pipeline result. An explicit
/// `compact_project` still reports its failure to the caller.
fn record_project_write(data_root: &Path, owner: &str, project: &str, write_units: usize) {
    if write_units == 0 {
        return;
    }
    let Ok(dir) = ensure_project_dir(data_root, owner, project) else {
        return;
    };
    let Ok(db) = get_db(data_root, owner, project) else {
        return;
    };
    let wal_bytes = db.storage().map(|s| s.wal_bytes).unwrap_or(0);
    let should_checkpoint = {
        let mut map = maintenance_pool().lock().unwrap_or_else(|e| e.into_inner());
        let state = map.entry(dir.clone()).or_default();
        state.write_units_since_checkpoint =
            state.write_units_since_checkpoint.saturating_add(write_units);
        state.write_units_since_checkpoint >= AUTO_CHECKPOINT_WRITE_UNITS
            || wal_bytes >= AUTO_CHECKPOINT_WAL_BYTES
    };
    if !should_checkpoint {
        return;
    }
    // The handle this function holds is one of the clones `fold_wal` has to
    // see gone, so it is released first.
    drop(db);
    if matches!(fold_wal(&dir), Ok(true)) {
        let mut map = maintenance_pool().lock().unwrap_or_else(|e| e.into_inner());
        map.entry(dir).or_default().write_units_since_checkpoint = 0;
    }
}

/// Cheap project-scoped store health: what is on disk and how many rows and
/// edges the store holds.
///
/// Row counts come from sekejap's live per-collection record where the
/// database keeps one, and from a walk where it does not; edges are always a
/// walk. Small stores answer in microseconds; a store with millions of rows
/// and no live record pays for the walk, which is what the name of
/// `Db::scan_count_rows` says.
pub fn project_health(
    data_root: &Path,
    owner: &str,
    project: &str,
) -> Result<SekejapProjectHealth, PlatformError> {
    let dir = ensure_project_dir(data_root, owner, project)?;
    let started = Instant::now();
    let db = get_db(data_root, owner, project)?;
    let mut node_count = 0u64;
    for name in db
        .collections()
        .map_err(|e| store_error("PLATFORM_SEKEJAP_HEALTH", e))?
    {
        node_count += db
            .count_rows(&name)
            .map_err(|e| store_error("PLATFORM_SEKEJAP_HEALTH", e))?;
    }
    let edge_count = db
        .scan_count_edges()
        .map_err(|e| store_error("PLATFORM_SEKEJAP_HEALTH", e))?;
    let storage = db
        .storage()
        .map_err(|e| store_error("PLATFORM_SEKEJAP_HEALTH", e))?;

    Ok(SekejapProjectHealth {
        owner: owner.to_string(),
        project: project.to_string(),
        root: dir.to_string_lossy().to_string(),
        node_count: node_count as usize,
        edge_count: edge_count as usize,
        wal_bytes: storage.wal_bytes,
        data_bytes: storage.data_bytes,
        duration_ms: started.elapsed().as_millis() as u64,
    })
}

/// Make the newest commit visible to every reader.
///
/// On sekejap 0.17 every commit is already durable when the call returns,
/// so there is no WAL to force. What is left of the old "sync" is
/// publication, which in the single-handle mode this module opens is
/// already the case too. The operation stays so a caller that scheduled it
/// keeps a report to read.
pub fn sync_project(
    data_root: &Path,
    owner: &str,
    project: &str,
) -> Result<SekejapMaintenanceReport, PlatformError> {
    let started = Instant::now();
    let before = project_health(data_root, owner, project)?;
    let db = get_db(data_root, owner, project)?;
    db.publish()
        .map_err(|e| store_error("PLATFORM_SEKEJAP_SYNC", e))?;
    let after = project_health(data_root, owner, project)?;
    Ok(SekejapMaintenanceReport {
        operation: "sync".to_string(),
        before,
        after,
        duration_ms: started.elapsed().as_millis() as u64,
    })
}

/// Fold the committed WAL into the data file.
///
/// The WAL checkpoint primitive, for explicit admin actions, low-traffic
/// scheduled maintenance, and graceful shutdown. See [`fold_wal`] for why
/// it closes and reopens the handle; a fold deferred because a request
/// still holds the handle is reported as `compact (deferred)`, not failed.
pub fn compact_project(
    data_root: &Path,
    owner: &str,
    project: &str,
) -> Result<SekejapMaintenanceReport, PlatformError> {
    let started = Instant::now();
    let before = project_health(data_root, owner, project)?;
    let dir = ensure_project_dir(data_root, owner, project)?;
    let folded = fold_wal(&dir)?;
    let after = project_health(data_root, owner, project)?;
    Ok(SekejapMaintenanceReport {
        operation: if folded {
            "compact".to_string()
        } else {
            "compact (deferred)".to_string()
        },
        before,
        after,
        duration_ms: started.elapsed().as_millis() as u64,
    })
}

pub const BUILTIN_CONNECTION_SLUG: &str = "default-multimodel";
pub const BUILTIN_CONNECTION_LABEL: &str = "Default Multimodel Store";
pub const DB_KIND: &str = "sekejap";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SekejapTableSchemaExport {
    pub table: String,
    pub collection: String,
    #[serde(default)]
    pub attributes: Vec<CollectionAttribute>,
    #[serde(default)]
    pub hash_indexed_fields: Vec<String>,
    #[serde(default)]
    pub range_indexed_fields: Vec<String>,
    #[serde(default)]
    pub fulltext_fields: Vec<String>,
    #[serde(default)]
    pub vector_fields: Vec<String>,
    #[serde(default)]
    pub spatial_fields: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SekejapSchemaExport {
    pub database: String,
    pub connection_slug: String,
    pub tables: Vec<SekejapTableSchemaExport>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SekejapSchemaSyncReport {
    pub changed: bool,
    pub root: String,
    pub files_written: Vec<String>,
    pub files_removed: Vec<String>,
    pub table_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SekejapSchemaApplyReport {
    pub tables_created: Vec<String>,
    pub tables_skipped: Vec<String>,
    pub table_count: usize,
}

/// What the store occupies and holds. `node_count` is rows across every
/// collection; the name is kept because every reader of this report — the
/// maintenance panel, the ops routes — already reads it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SekejapProjectHealth {
    pub owner: String,
    pub project: String,
    pub root: String,
    pub node_count: usize,
    pub edge_count: usize,
    pub wal_bytes: u64,
    pub data_bytes: u64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SekejapMaintenanceReport {
    pub operation: String,
    pub before: SekejapProjectHealth,
    pub after: SekejapProjectHealth,
    pub duration_ms: u64,
}

/// Drops the pooled handle for one project store.
///
/// A `ProjectBundle` import swaps `data/store/` as a unit
/// (`kinds/project-bundle/README.md`), so a `Db` opened against the
/// displaced directory must not keep serving its bytes. Callers still
/// holding a cloned `Arc` finish their in-flight call on the old handle; the
/// next `get_db` reopens from the swapped-in directory.
pub fn evict_project_pool(data_root: &Path, owner: &str, project: &str) {
    let dir = project_dir(data_root, owner, project);
    pool()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&dir);
    maintenance_pool()
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .remove(&dir);
}

pub fn project_dir(data_root: &Path, owner: &str, project: &str) -> PathBuf {
    data_root
        .join("users")
        .join(slug_segment(owner))
        .join(slug_segment(project))
        .join("data")
        .join("store")
        .join("sekejap")
}

fn repo_dir(data_root: &Path, owner: &str, project: &str) -> PathBuf {
    data_root
        .join("users")
        .join(slug_segment(owner))
        .join(slug_segment(project))
        .join("repo")
}

fn repo_schema_dir(data_root: &Path, owner: &str, project: &str) -> PathBuf {
    repo_dir(data_root, owner, project).join(repo_layout().schema)
}

/// The layout the exported schema documents are placed by.
///
/// This module reaches the repository through `data_root` rather than through
/// a `ProjectFileLayout`, so it resolves the same layout rather than repeating
/// the directory it names. Every writer in this module is reached from a node
/// or handler that has no layout in hand, so honoring a declared
/// `spec.layout.schema` here is a separate change from this one.
fn repo_layout() -> ResolvedProjectLayout {
    ResolvedProjectLayout::platform_default()
}

pub fn ensure_project_dir(
    data_root: &Path,
    owner: &str,
    project: &str,
) -> Result<PathBuf, PlatformError> {
    let dir = project_dir(data_root, owner, project);
    migrate_legacy_sekejap_dir(data_root, owner, project, &dir)?;
    std::fs::create_dir_all(&dir)?;
    refuse_or_retire_legacy_store(&dir)?;
    // `tables.json` was a hand-kept mirror of the table list from before
    // sekejap could be asked directly. `live_tables` reads the database now,
    // so the file is only stale weight; drop it the first time a project is
    // touched.
    match std::fs::remove_file(dir.join("tables.json")) {
        Ok(()) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
        Err(err) => return Err(err.into()),
    }
    Ok(dir)
}

/// Moves a pre-tier `data/sekejap` into `data/store/sekejap`
/// (`project-directory.md` §5), once.
///
/// This is the one chokepoint every store access already runs through
/// (`get_db`, `record_project_write`, health and maintenance all call
/// `ensure_project_dir`), independent of whether anything has resolved a full
/// `ProjectFileLayout` for the request. See
/// [`crate::infra::io::durable::migrate_tier_entry`] for the atomicity and
/// idempotency this relies on.
fn migrate_legacy_sekejap_dir(
    data_root: &Path,
    owner: &str,
    project: &str,
    new_dir: &Path,
) -> Result<(), PlatformError> {
    let old_dir = data_root
        .join("users")
        .join(slug_segment(owner))
        .join(slug_segment(project))
        .join("data")
        .join("sekejap");
    crate::infra::io::durable::migrate_tier_entry(&old_dir, new_dir)
        .map_err(|err| PlatformError::new("PLATFORM_DATA_TIER_MIGRATE", err.to_string()))
}

/// A directory sekejap 0.16 wrote, met by 0.17.
///
/// 0.17 recognises its own store by a `data` file. A directory that has none
/// but carries the 0.16 files is one of two things. Empty — the header-only
/// WAL every fresh 0.16 store wrote, no payload, no snapshot — and the old
/// files are moved aside into `legacy-0.16/` so the directory can be created
/// into. Holding rows — and it is refused by name, because the only honest
/// migration is to read it with 0.16 and write it with 0.17, and nothing
/// here can do the first half.
fn refuse_or_retire_legacy_store(dir: &Path) -> Result<(), PlatformError> {
    if dir.join("data").exists() {
        return Ok(());
    }
    let present: Vec<&str> = LEGACY_FILES
        .iter()
        .copied()
        .filter(|name| dir.join(name).exists())
        .collect();
    if present.is_empty() {
        return Ok(());
    }
    let size = |name: &str| std::fs::metadata(dir.join(name)).map(|m| m.len()).unwrap_or(0);
    // A 0.16 WAL is eight header bytes when nothing was ever written.
    let holds_rows = size("wal.log") > 8 || size("payloads.bin") > 0 || size("snapshot.json") > 0;
    if holds_rows {
        return Err(PlatformError::new(
            "PLATFORM_SEKEJAP_LEGACY_STORE",
            format!(
                "{} holds a sekejap 0.16 store with rows, and sekejap 0.17 cannot read that format. \
                 Export it with a 0.16 build and re-import, or move the directory aside to start empty.",
                dir.display()
            ),
        ));
    }
    let retired = dir.join("legacy-0.16");
    std::fs::create_dir_all(&retired)?;
    for name in present {
        std::fs::rename(dir.join(name), retired.join(name))?;
    }
    for name in ["db.lock"] {
        let _ = std::fs::remove_file(dir.join(name));
    }
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            let is_vector_sidecar = path
                .file_name()
                .and_then(|n| n.to_str())
                .is_some_and(|n| n.starts_with("vectors_") && n.ends_with(".bin"));
            if is_vector_sidecar {
                let _ = std::fs::rename(&path, retired.join(entry.file_name()));
            }
        }
    }
    Ok(())
}

// ── attribute kinds ──────────────────────────────────────────────────────

/// The kind a Studio attribute declares, split from the one parameter a
/// kind can carry: `vector(384)`. Everything else has none.
fn parse_kind(raw: &str) -> (String, Option<usize>) {
    let raw = raw.trim().to_ascii_lowercase();
    if let Some(open) = raw.find('(') {
        let base = raw[..open].trim().to_string();
        let inner = raw[open + 1..].trim_end_matches(')').trim();
        return (base, inner.parse::<usize>().ok().filter(|n| *n > 0));
    }
    (raw, None)
}

fn map_declared_kind(kind: &str) -> String {
    let (base, dims) = parse_kind(kind);
    match base.as_str() {
        "number" | "real" | "integer" | "int" | "float" => "number".to_string(),
        "boolean" | "bool" => "boolean".to_string(),
        "json" | "jsonb" => "json".to_string(),
        "vector" => match dims {
            Some(n) => format!("vector({n})"),
            None => "vector".to_string(),
        },
        "geo" | "geometry" | "point" => "geo".to_string(),
        _ => "string".to_string(),
    }
}

/// The SQL type one attribute kind becomes. A vector without its dimension
/// is refused: `VECTOR(n)` needs `n`, and there is no default that would not
/// be a guess about someone's embedding model.
fn map_field_type(kind: &str) -> Result<String, PlatformError> {
    let (base, dims) = parse_kind(kind);
    Ok(match base.as_str() {
        "number" | "real" | "integer" | "int" | "float" => "REAL".to_string(),
        "boolean" | "bool" => "BOOLEAN".to_string(),
        "json" | "jsonb" => "JSONB".to_string(),
        "vector" => match dims {
            Some(n) => format!("VECTOR({n})"),
            None => {
                return Err(PlatformError::new(
                    "PLATFORM_SEKEJAP_VECTOR_DIMENSION",
                    "a vector attribute needs its dimension: write the kind as `vector(384)` (the length of the embeddings that will be stored)",
                ));
            }
        },
        "geo" | "geometry" => "GEOMETRY".to_string(),
        "point" => "GEOMETRY(Point,4326)".to_string(),
        _ => "TEXT".to_string(),
    })
}

/// One declared field back into the attribute kind the Studio shows.
fn field_kind_to_attribute(field: &sekejap::Field) -> String {
    match &field.kind {
        FieldKind::Text => "string".to_string(),
        FieldKind::Int | FieldKind::Real => "number".to_string(),
        FieldKind::Bool => "boolean".to_string(),
        FieldKind::Json => "json".to_string(),
        FieldKind::Geo | FieldKind::Point => "geo".to_string(),
        FieldKind::Vector(n) => format!("vector({n})"),
    }
}

/// Which of the Studio's index kinds one catalog index is. A scalar index
/// answers equality and range alike, so it is both `hash` and `range`.
fn index_family_kinds(family: &IndexFamily) -> &'static [&'static str] {
    match family {
        IndexFamily::Scalar => &["hash", "range"],
        IndexFamily::Text => &["fulltext"],
        IndexFamily::SpatialPoint | IndexFamily::SpatialGeometry => &["spatial"],
        IndexFamily::ExactVector | IndexFamily::QuantizedVector | IndexFamily::VamanaGraph => {
            &["vector"]
        }
        #[allow(unreachable_patterns)]
        _ => &[],
    }
}

fn collect_index_fields(
    attrs: &[CollectionAttribute],
    explicit: &[String],
    index_kind: &str,
) -> Vec<String> {
    let mut out = BTreeSet::new();
    for item in explicit {
        let field = slug_segment(item);
        if !field.is_empty() {
            out.insert(field);
        }
    }
    for attr in attrs {
        if attr.index_types.iter().any(|item| item == index_kind) {
            out.insert(attr.name.clone());
        }
    }
    out.into_iter().collect()
}

fn normalize_attributes(attributes: &[CollectionAttribute]) -> Vec<CollectionAttribute> {
    let mut attrs = Vec::new();
    let mut seen = BTreeSet::new();
    for attr in attributes {
        let name = slug_segment(&attr.name);
        if name.is_empty() || name == "_key" || !seen.insert(name.clone()) {
            continue;
        }
        let kind = map_declared_kind(&attr.kind);
        let mut index_types = Vec::new();
        for item in &attr.index_types {
            let key = slug_segment(item);
            if key.is_empty() || index_types.iter().any(|existing| existing == &key) {
                continue;
            }
            index_types.push(key);
        }
        attrs.push(CollectionAttribute {
            name,
            kind,
            index_types,
        });
    }
    attrs
}

fn normalize_definition(
    req: &CreateSimpleTableRequest,
) -> Result<SimpleTableDefinition, PlatformError> {
    let table = slug_segment(&req.table);
    if table.is_empty() {
        return Err(PlatformError::new(
            "PLATFORM_SEKEJAP_TABLE_INVALID",
            "table slug must not be empty",
        ));
    }
    let attrs = normalize_attributes(&req.attributes);
    let hash_indexed_fields = collect_index_fields(&attrs, &req.hash_indexed_fields, "hash");
    let range_indexed_fields = collect_index_fields(&attrs, &req.range_indexed_fields, "range");
    let fulltext_fields = collect_index_fields(&attrs, &[], "fulltext");
    let vector_fields = collect_index_fields(&attrs, &[], "vector");
    let spatial_fields = collect_index_fields(&attrs, &[], "spatial");

    Ok(SimpleTableDefinition {
        table: table.clone(),
        collection: table,
        attributes: attrs,
        hash_indexed_fields,
        range_indexed_fields,
        fulltext_fields,
        vector_fields,
        spatial_fields,
        row_count: 0,
    })
}

/// The one statement that creates a managed table.
///
/// `_key TEXT PRIMARY KEY` names the key every sekejap row has. The declared
/// indexes ride in the `WITH (...)` sugar so the table and its indexes are
/// one commit. `hash` and `range` are not written: sekejap gives every
/// scalar column a btree unasked (`docs/lang/INDEX_CONTRACT.md`), and that
/// one index answers both.
fn build_create_table_sql(def: &SimpleTableDefinition) -> Result<String, PlatformError> {
    let mut columns = vec!["_key TEXT PRIMARY KEY".to_string()];
    for attr in &def.attributes {
        columns.push(format!("{} {}", attr.name, map_field_type(&attr.kind)?));
    }
    let mut with = Vec::new();
    let declared = |fields: &[String]| -> Vec<String> {
        fields
            .iter()
            .filter(|f| def.attributes.iter().any(|a| &a.name == *f))
            .cloned()
            .collect()
    };
    let fulltext = declared(&def.fulltext_fields);
    if !fulltext.is_empty() {
        with.push(format!("fulltext: [{}]", fulltext.join(", ")));
    }
    let spatial = declared(&def.spatial_fields);
    if !spatial.is_empty() {
        with.push(format!("spatial: [{}]", spatial.join(", ")));
    }
    let vector = declared(&def.vector_fields);
    if !vector.is_empty() {
        with.push(format!("vector: [{}]", vector.join(", ")));
    }
    let mut sql = format!("CREATE TABLE {} ({})", def.collection, columns.join(", "));
    if !with.is_empty() {
        sql.push_str(&format!(" WITH ({})", with.join(", ")));
    }
    Ok(sql)
}

/// One declared index, by the family word the catalog names it with.
fn build_index_sql(collection: &str, kind: &str, field: &str) -> Option<String> {
    let method = match kind {
        "hash" | "range" => format!("btree ({field})"),
        "fulltext" => format!("gin (to_tsvector('simple', {field}))"),
        "spatial" => format!("gist ({field})"),
        "vector" => format!("exact ({field})"),
        _ => return None,
    };
    Some(format!("CREATE INDEX ON {collection} USING {method}"))
}

/// Every table in the project, read from the live catalog.
fn live_tables(db: &Db) -> Result<Vec<SimpleTableDefinition>, PlatformError> {
    let mut by_table = BTreeMap::new();
    for collection in db
        .collections()
        .map_err(|e| store_error("PLATFORM_SEKEJAP_CATALOG", e))?
    {
        let table = slug_segment(&collection);
        if table.is_empty() || by_table.contains_key(&table) {
            continue;
        }
        let Some(described) = db
            .describe(&collection)
            .map_err(|e| store_error("PLATFORM_SEKEJAP_CATALOG", e))?
        else {
            continue;
        };
        let row_count = match described.rows {
            Some(n) => n,
            None => db
                .count_rows(&collection)
                .map_err(|e| store_error("PLATFORM_SEKEJAP_CATALOG", e))?,
        } as usize;

        let mut hash = BTreeSet::new();
        let mut range = BTreeSet::new();
        let mut fulltext = BTreeSet::new();
        let mut vector = BTreeSet::new();
        let mut spatial = BTreeSet::new();
        for index in &described.indexes {
            for kind in index_family_kinds(&index.family) {
                match *kind {
                    "hash" => hash.insert(index.field.clone()),
                    "range" => range.insert(index.field.clone()),
                    "fulltext" => fulltext.insert(index.field.clone()),
                    "vector" => vector.insert(index.field.clone()),
                    "spatial" => spatial.insert(index.field.clone()),
                    _ => false,
                };
            }
        }
        let attributes = described
            .fields
            .iter()
            .filter(|f| !f.primary_key && !f.name.starts_with('_'))
            .map(|f| {
                let mut index_types = Vec::new();
                for (set, kind) in [
                    (&hash, "hash"),
                    (&range, "range"),
                    (&fulltext, "fulltext"),
                    (&vector, "vector"),
                    (&spatial, "spatial"),
                ] {
                    if set.contains(&f.name) {
                        index_types.push(kind.to_string());
                    }
                }
                CollectionAttribute {
                    name: f.name.clone(),
                    kind: field_kind_to_attribute(f),
                    index_types,
                }
            })
            .collect();

        by_table.insert(
            table.clone(),
            SimpleTableDefinition {
                table,
                attributes,
                hash_indexed_fields: hash.into_iter().collect(),
                range_indexed_fields: range.into_iter().collect(),
                fulltext_fields: fulltext.into_iter().collect(),
                vector_fields: vector.into_iter().collect(),
                spatial_fields: spatial.into_iter().collect(),
                row_count,
                collection: collection.clone(),
            },
        );
    }
    Ok(by_table.into_values().collect())
}

fn stable_list(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values.dedup();
    values
}

fn export_table_schema(def: SimpleTableDefinition) -> SekejapTableSchemaExport {
    let mut attributes = def.attributes;
    attributes.sort_by(|a, b| a.name.cmp(&b.name));
    for attr in &mut attributes {
        attr.index_types = stable_list(std::mem::take(&mut attr.index_types));
    }
    SekejapTableSchemaExport {
        table: def.table,
        collection: def.collection,
        attributes,
        hash_indexed_fields: stable_list(def.hash_indexed_fields),
        range_indexed_fields: stable_list(def.range_indexed_fields),
        fulltext_fields: stable_list(def.fulltext_fields),
        vector_fields: stable_list(def.vector_fields),
        spatial_fields: stable_list(def.spatial_fields),
    }
}

pub fn export_schema(
    data_root: &Path,
    owner: &str,
    project: &str,
) -> Result<SekejapSchemaExport, PlatformError> {
    let tables = export_tables_from_defs(list_tables(data_root, owner, project)?);
    Ok(SekejapSchemaExport {
        database: DB_KIND.to_string(),
        connection_slug: BUILTIN_CONNECTION_SLUG.to_string(),
        tables,
    })
}

/// Encodes a portable Sekejap schema using the canonical platform envelope.
pub fn encode_schema_export(export: SekejapSchemaExport) -> Result<Vec<u8>, PlatformError> {
    let name = export.connection_slug.clone();
    encode_contract::<DatabaseSchemaContract>(ContractMetadata::named(name), export).map_err(
        |err| {
            PlatformError::new(
                "PLATFORM_SEKEJAP_SCHEMA_EXPORT",
                format!("failed to serialise schema contract: {err}"),
            )
        },
    )
}

fn export_tables_from_defs(defs: Vec<SimpleTableDefinition>) -> Vec<SekejapTableSchemaExport> {
    let mut tables = defs
        .into_iter()
        .map(export_table_schema)
        .collect::<Vec<_>>();
    tables.sort_by(|a, b| a.table.cmp(&b.table));
    tables
}

fn imported_table_definition(table: &SekejapTableSchemaExport) -> Option<SimpleTableDefinition> {
    let table_slug = slug_segment(&table.table);
    if table_slug.is_empty() {
        return None;
    }
    let collection_slug = slug_segment(&table.collection);
    let collection = if collection_slug.is_empty() {
        table_slug.clone()
    } else {
        collection_slug
    };
    Some(SimpleTableDefinition {
        table: table_slug.clone(),
        collection,
        attributes: normalize_attributes(&table.attributes),
        hash_indexed_fields: stable_list(table.hash_indexed_fields.clone()),
        range_indexed_fields: stable_list(table.range_indexed_fields.clone()),
        fulltext_fields: stable_list(table.fulltext_fields.clone()),
        vector_fields: stable_list(table.vector_fields.clone()),
        spatial_fields: stable_list(table.spatial_fields.clone()),
        row_count: 0,
    })
}

/// Creates one managed table with its declared indexes.
fn create_managed_table(db: &Db, def: &SimpleTableDefinition) -> Result<(), PlatformError> {
    db.execute(&build_create_table_sql(def)?, &[])
        .map_err(|e| store_error("PLATFORM_SEKEJAP_TABLE_CREATE", e))?;
    Ok(())
}

pub fn apply_schema_export(
    data_root: &Path,
    owner: &str,
    project: &str,
    export: &SekejapSchemaExport,
) -> Result<SekejapSchemaApplyReport, PlatformError> {
    encode_schema_export(export.clone())?;
    let mut tables_created = Vec::new();
    let mut tables_skipped = Vec::new();
    let db = get_db(data_root, owner, project)?;
    let mut existing_tables = live_tables(&db)?
        .into_iter()
        .map(|def| def.table)
        .collect::<BTreeSet<_>>();

    for table in &export.tables {
        let Some(def) = imported_table_definition(table) else {
            continue;
        };
        if existing_tables.contains(&def.table) {
            tables_skipped.push(def.table.clone());
            continue;
        }
        create_managed_table(&db, &def)
            .map_err(|err| PlatformError::new("PLATFORM_SEKEJAP_SCHEMA_APPLY", err.message))?;
        existing_tables.insert(def.table.clone());
        tables_created.push(def.table);
    }

    sync_schema_to_repo(data_root, owner, project)?;

    Ok(SekejapSchemaApplyReport {
        table_count: export.tables.len(),
        tables_created,
        tables_skipped,
    })
}

pub fn apply_schema_from_repo(
    data_root: &Path,
    owner: &str,
    project: &str,
) -> Result<Option<SekejapSchemaApplyReport>, PlatformError> {
    let schema_path = repo_schema_dir(data_root, owner, project).join(SCHEMA_DOCUMENT_FILE);
    if !schema_path.is_file() {
        return Ok(None);
    }
    let raw = std::fs::read(&schema_path)?;
    let document = decode_contract::<DatabaseSchemaContract>(&raw).map_err(|err| {
        PlatformError::new(
            "PLATFORM_SEKEJAP_SCHEMA_READ",
            format!("failed to parse sekejap schema contract: {err}"),
        )
    })?;
    apply_schema_export(data_root, owner, project, &document.spec).map(Some)
}

fn write_bytes_if_changed(path: &Path, bytes: &[u8]) -> Result<bool, PlatformError> {
    if path.exists() && std::fs::read(path)? == bytes {
        return Ok(false);
    }
    atomic_write(path, bytes)?;
    Ok(true)
}

pub fn sync_schema_to_repo(
    data_root: &Path,
    owner: &str,
    project: &str,
) -> Result<SekejapSchemaSyncReport, PlatformError> {
    let db = get_db(data_root, owner, project)?;
    let defs = live_tables(&db)?;
    let export = SekejapSchemaExport {
        database: DB_KIND.to_string(),
        connection_slug: BUILTIN_CONNECTION_SLUG.to_string(),
        tables: export_tables_from_defs(defs),
    };
    let layout = repo_layout();
    let schema_root = repo_schema_dir(data_root, owner, project);
    std::fs::create_dir_all(&schema_root)?;

    let mut changed = false;
    let mut files_removed = Vec::new();

    let schema_path = schema_root.join(SCHEMA_DOCUMENT_FILE);
    let schema_bytes = encode_schema_export(export.clone())?;
    if write_bytes_if_changed(&schema_path, &schema_bytes)? {
        changed = true;
    }
    let files_written = vec![layout.schema_document_rel()];

    // A `tables/` directory beside the schema document is the residue of an
    // earlier writer. It duplicated what the schema document already carries,
    // no reader in the tree ever opened it, and no contract names it. Clear it
    // out of repositories that still hold one.
    let tables_dir = schema_root.join("tables");
    if tables_dir.is_dir() {
        for entry in std::fs::read_dir(&tables_dir)? {
            let path = entry?.path();
            if path.is_file() && path.extension().and_then(|v| v.to_str()) == Some("json") {
                if let Some(name) = path.file_name().and_then(|v| v.to_str()) {
                    files_removed.push(format!("{}/tables/{name}", layout.schema));
                }
            }
        }
        std::fs::remove_dir_all(&tables_dir)?;
        changed = true;
    }

    Ok(SekejapSchemaSyncReport {
        changed,
        root: layout.schema.clone(),
        files_written,
        files_removed,
        table_count: export.tables.len(),
    })
}

pub fn list_tables(
    data_root: &Path,
    owner: &str,
    project: &str,
) -> Result<Vec<SimpleTableDefinition>, PlatformError> {
    let db = get_db(data_root, owner, project)?;
    live_tables(&db)
}

pub fn create_table(
    data_root: &Path,
    owner: &str,
    project: &str,
    req: &CreateSimpleTableRequest,
) -> Result<SimpleTableDefinition, PlatformError> {
    let def = normalize_definition(req)?;
    let existing = list_tables(data_root, owner, project)?;
    if existing.iter().any(|item| item.table == def.table) {
        return Err(PlatformError::new(
            "PLATFORM_SEKEJAP_TABLE_EXISTS",
            format!("table '{}' already exists", def.table),
        ));
    }

    let db = get_db(data_root, owner, project)?;
    create_managed_table(&db, &def)?;
    sync_schema_to_repo(data_root, owner, project)?;
    live_tables(&db)?
        .into_iter()
        .find(|item| item.table == def.table)
        .ok_or_else(|| {
            PlatformError::new(
                "PLATFORM_SEKEJAP_TABLE_CREATE",
                format!("table '{}' was not in the catalog after CREATE TABLE", def.table),
            )
        })
}

pub fn delete_table(
    data_root: &Path,
    owner: &str,
    project: &str,
    table: &str,
) -> Result<(), PlatformError> {
    let table_slug = slug_segment(table);
    if table_slug.is_empty() {
        return Err(PlatformError::new(
            "PLATFORM_SEKEJAP_TABLE_INVALID",
            "table slug must not be empty",
        ));
    }

    let db = get_db(data_root, owner, project)?;
    let Some(def) = live_tables(&db)?
        .into_iter()
        .find(|d| d.table == table_slug)
    else {
        return Err(PlatformError::new(
            "PLATFORM_SEKEJAP_TABLE_NOT_FOUND",
            format!("table '{}' not found", table_slug),
        ));
    };

    // CASCADE: a managed table dropped from the Studio takes its edges with
    // it; RESTRICT would refuse the drop and name the graph contexts instead.
    db.execute(&format!("DROP TABLE IF EXISTS {} CASCADE", def.collection), &[])
        .map_err(|e| store_error("PLATFORM_SEKEJAP_TABLE_DROP", e))?;
    let still_there = db
        .collections()
        .map_err(|e| store_error("PLATFORM_SEKEJAP_TABLE_DROP", e))?
        .into_iter()
        .any(|collection| collection == def.collection);
    if still_there {
        return Err(PlatformError::new(
            "PLATFORM_SEKEJAP_TABLE_DROP",
            format!("table '{}' still exists after DROP TABLE", table_slug),
        ));
    }

    sync_schema_to_repo(data_root, owner, project)?;
    Ok(())
}

/// Reshapes a managed table: new columns are added, declared indexes are
/// brought in line with the request.
///
/// The btree every scalar column carries is sekejap's automatic index and is
/// never dropped here: without it a `WHERE` on that column is refused, and
/// nobody unticks "hash" in the Studio meaning that. The declared families
/// — full text, spatial, vector — are the ones this call adds and removes.
pub fn update_table(
    data_root: &Path,
    owner: &str,
    project: &str,
    table: &str,
    req: &UpdateSimpleTableRequest,
) -> Result<SimpleTableDefinition, PlatformError> {
    let table_slug = slug_segment(table);
    if table_slug.is_empty() {
        return Err(PlatformError::new(
            "PLATFORM_SEKEJAP_TABLE_INVALID",
            "table slug must not be empty",
        ));
    }

    let db = get_db(data_root, owner, project)?;
    let Some(existing) = live_tables(&db)?
        .into_iter()
        .find(|d| d.table == table_slug)
    else {
        return Err(PlatformError::new(
            "PLATFORM_SEKEJAP_TABLE_NOT_FOUND",
            format!("table '{}' not found", table_slug),
        ));
    };

    let attrs = normalize_attributes(&req.attributes);
    let fulltext_fields = collect_index_fields(&attrs, &[], "fulltext");
    let vector_fields = collect_index_fields(&attrs, &[], "vector");
    let spatial_fields = collect_index_fields(&attrs, &[], "spatial");

    // New columns. sekejap builds the automatic index over the rows already
    // there, so a column added here is filterable as soon as this returns.
    let old_names: BTreeSet<String> = existing.attributes.iter().map(|a| a.name.clone()).collect();
    for attr in &attrs {
        if !old_names.contains(&attr.name) {
            let sql = format!(
                "ALTER TABLE {} ADD COLUMN {} {}",
                existing.collection,
                attr.name,
                map_field_type(&attr.kind)?
            );
            db.execute(&sql, &[])
                .map_err(|e| store_error("PLATFORM_SEKEJAP_TABLE_ALTER", e))?;
        }
    }

    // Declared indexes: drop the ones no longer wanted, by the name the
    // catalog holds them under; create the wanted ones, which sekejap
    // answers with a notice when one already exists.
    let described = db
        .describe(&existing.collection)
        .map_err(|e| store_error("PLATFORM_SEKEJAP_TABLE_ALTER", e))?;
    if let Some(described) = described {
        for index in &described.indexes {
            let wanted = match index.family {
                IndexFamily::Scalar => true,
                IndexFamily::Text => fulltext_fields.contains(&index.field),
                IndexFamily::SpatialPoint | IndexFamily::SpatialGeometry => {
                    spatial_fields.contains(&index.field)
                }
                IndexFamily::ExactVector
                | IndexFamily::QuantizedVector
                | IndexFamily::VamanaGraph => vector_fields.contains(&index.field),
                #[allow(unreachable_patterns)]
                _ => true,
            };
            if !wanted {
                db.execute(&format!("DROP INDEX IF EXISTS {}", index.name), &[])
                    .map_err(|e| store_error("PLATFORM_SEKEJAP_INDEX_DROP", e))?;
            }
        }
    }
    for (kind, fields) in [
        ("fulltext", &fulltext_fields),
        ("spatial", &spatial_fields),
        ("vector", &vector_fields),
    ] {
        for field in fields {
            if let Some(sql) = build_index_sql(&existing.collection, kind, field) {
                db.execute(&sql, &[])
                    .map_err(|e| store_error("PLATFORM_SEKEJAP_INDEX_CREATE", e))?;
            }
        }
    }

    sync_schema_to_repo(data_root, owner, project)?;
    live_tables(&db)?
        .into_iter()
        .find(|item| item.table == table_slug)
        .ok_or_else(|| {
            PlatformError::new(
                "PLATFORM_SEKEJAP_TABLE_NOT_FOUND",
                format!("table '{}' vanished during update", table_slug),
            )
        })
}

fn table_to_node(def: &SimpleTableDefinition) -> DbObjectNode {
    DbObjectNode {
        kind: "table".to_string(),
        name: def.table.clone(),
        schema: Some("default".to_string()),
        children: Vec::new(),
        meta: json!({
            "collection": def.collection,
            "row_count": def.row_count,
            "attributes": def.attributes,
            "hash_indexed_fields": def.hash_indexed_fields,
            "range_indexed_fields": def.range_indexed_fields,
            "fulltext_fields": def.fulltext_fields,
            "vector_fields": def.vector_fields,
            "spatial_fields": def.spatial_fields,
        }),
    }
}

pub fn describe_tables(
    data_root: &Path,
    owner: &str,
    project: &str,
) -> Result<Vec<DbObjectNode>, PlatformError> {
    Ok(list_tables(data_root, owner, project)?
        .into_iter()
        .map(|item| table_to_node(&item))
        .collect())
}

pub fn describe_schemas(
    data_root: &Path,
    owner: &str,
    project: &str,
) -> Result<Vec<DbObjectNode>, PlatformError> {
    let has_tables = !list_tables(data_root, owner, project)?.is_empty();
    if !has_tables {
        return Ok(Vec::new());
    }
    Ok(vec![DbObjectNode {
        kind: "schema".to_string(),
        name: "default".to_string(),
        schema: None,
        children: Vec::new(),
        meta: json!({}),
    }])
}

pub fn describe_tree(
    data_root: &Path,
    owner: &str,
    project: &str,
) -> Result<Vec<DbObjectNode>, PlatformError> {
    let tables = describe_tables(data_root, owner, project)?;
    if tables.is_empty() {
        return Ok(Vec::new());
    }
    Ok(vec![DbObjectNode {
        kind: "schema".to_string(),
        name: "default".to_string(),
        schema: None,
        children: tables,
        meta: json!({}),
    }])
}

pub fn describe_columns(
    data_root: &Path,
    owner: &str,
    project: &str,
    table: &str,
) -> Result<Vec<DbObjectNode>, PlatformError> {
    let wanted = slug_segment(table.rsplit('.').next().unwrap_or(table));
    let defs = list_tables(data_root, owner, project)?;
    let Some(def) = defs.into_iter().find(|item| item.table == wanted) else {
        return Ok(Vec::new());
    };
    // Every sekejap row is identified by `_key`, which is not one of the
    // declared attributes but is a structure fact the engine knows. Reporting
    // it keeps the studio's column list the same as the engine's own row shape.
    let mut nodes = vec![DbObjectNode {
        kind: "column".to_string(),
        name: "_key".to_string(),
        schema: Some("default".to_string()),
        children: Vec::new(),
        meta: json!({
            "data_type": "string",
            "type": "string",
            "pk": true,
            "index_types": Vec::<String>::new(),
        }),
    }];
    nodes.extend(def.attributes.into_iter().map(|attr| DbObjectNode {
        kind: "column".to_string(),
        name: attr.name.clone(),
        schema: Some("default".to_string()),
        children: Vec::new(),
        meta: json!({
            "data_type": attr.kind,
            // `type` mirrors what the SQL drivers emit so one structure table
            // renders every engine.
            "type": attr.kind,
            "index_types": attr.index_types,
        }),
    }));
    Ok(nodes)
}

// ── SQL ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct QueryPayload {
    pub columns: Vec<DbQueryColumn>,
    pub rows: Vec<Vec<Value>>,
    pub row_count: usize,
    pub truncated: bool,
    pub affected_rows: Option<u64>,
    pub duration_ms: u64,
}

#[derive(Debug, Clone)]
pub struct StructuredInsertRecord {
    pub key: String,
    pub fields: Map<String, Value>,
}

#[derive(Debug, Clone)]
pub struct StructuredInsertEdge {
    pub from_target: String,
    pub from_key: String,
    pub edge_type: String,
    pub to_target: String,
    pub to_key: String,
    pub fields: Map<String, Value>,
    pub strength: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructuredWriteMode {
    Upsert,
    Insert,
    Update,
    Merge,
}

#[derive(Debug, Clone)]
pub struct StructuredWritePayload {
    pub affected_rows: usize,
    pub optimized_fields: Vec<String>,
    pub field_dimensions: BTreeMap<String, usize>,
    pub duration_ms: u64,
}

fn first_word(sql: &str) -> String {
    sql.trim_start()
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_uppercase()
}

fn statement_is_write(sql: &str) -> bool {
    matches!(
        first_word(sql).as_str(),
        "INSERT" | "UPDATE" | "DELETE" | "CREATE" | "DROP" | "ALTER" | "BEGIN" | "COMMIT" | "ROLLBACK"
    )
}

pub fn statement_changes_schema(sql: &str) -> bool {
    let mut words = sql
        .trim_start()
        .trim_end_matches(';')
        .split_whitespace()
        .map(|part| part.to_ascii_uppercase());
    let first = words.next().unwrap_or_default();
    let second = words.next().unwrap_or_default();
    matches!(
        (first.as_str(), second.as_str()),
        ("CREATE", "TABLE")
            | ("CREATE", "COLLECTION")
            | ("CREATE", "INDEX")
            | ("CREATE", "UNIQUE")
            | ("ALTER", "TABLE")
            | ("ALTER", "COLLECTION")
            | ("DROP", "TABLE")
            | ("DROP", "COLLECTION")
            | ("DROP", "INDEX")
    )
}

fn statement_is_explain(sql: &str) -> bool {
    first_word(sql) == "EXPLAIN"
}

fn statement_is_explain_analyze(sql: &str) -> bool {
    let mut words = sql.trim_start().split_whitespace();
    let first = words.next().unwrap_or("").to_ascii_uppercase();
    let second = words.next().unwrap_or("").to_ascii_uppercase();
    first == "EXPLAIN" && second == "ANALYZE"
}

/// The statement after `EXPLAIN`.
fn strip_explain_prefix(sql: &str) -> &str {
    let rest = sql.trim_start();
    let rest = rest
        .get(..7)
        .filter(|head| head.eq_ignore_ascii_case("EXPLAIN"))
        .map(|_| &rest[7..])
        .unwrap_or(rest);
    rest.trim_start()
}

fn statement_is_show_tables(sql: &str) -> bool {
    let parts = sql
        .trim()
        .trim_end_matches(';')
        .split_whitespace()
        .map(|part| part.to_ascii_uppercase())
        .collect::<Vec<_>>();
    parts.len() == 2 && parts[0] == "SHOW" && parts[1] == "TABLES"
}

/// `SHOW <one word>`: the word, when the statement is that shape.
fn show_collection_target(sql: &str) -> Option<String> {
    let parts = sql
        .trim()
        .trim_end_matches(';')
        .split_whitespace()
        .collect::<Vec<_>>();
    if parts.len() != 2 || !parts[0].eq_ignore_ascii_case("SHOW") {
        return None;
    }
    let word = parts[1];
    if matches!(
        word.to_ascii_uppercase().as_str(),
        "TABLES" | "EDGES" | "INDEXES" | "INDEX" | "CREATE" | "ALL"
    ) {
        return None;
    }
    Some(word.to_string())
}

/// An `INSERT INTO t (columns)` whose column list names no `_key`.
///
/// sekejap 0.17 takes the FIRST column as the row's key in that case, with a
/// notice nobody reading a pipeline result sees. A title silently becoming
/// a key is exactly the kind of thing to refuse by name: every Zebflow row is
/// addressed by `_key`, so the statement has to say what it is.
fn insert_without_key(sql: &str) -> bool {
    let trimmed = sql.trim_start();
    if !trimmed
        .get(..6)
        .is_some_and(|head| head.eq_ignore_ascii_case("INSERT"))
    {
        return false;
    }
    let Some(open) = trimmed.find('(') else {
        return false;
    };
    let Some(close) = trimmed[open..].find(')') else {
        return false;
    };
    let columns = &trimmed[open + 1..open + close];
    !columns
        .split(',')
        .any(|column| column.trim().trim_matches('"') == "_key")
}

fn payload_from_rows(rows: sekejap::Rows, limit: usize) -> (Vec<DbQueryColumn>, Vec<Vec<Value>>, bool) {
    let columns = rows
        .columns
        .iter()
        .map(|name| DbQueryColumn {
            name: name.clone(),
            data_type: None,
        })
        .collect::<Vec<_>>();
    let truncated = rows.rows.len() > limit;
    let rows = rows
        .rows
        .into_iter()
        .take(limit)
        .map(|row| row.values.iter().map(sekejap::value_to_json).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    (columns, rows, truncated)
}

fn payload(
    columns: Vec<DbQueryColumn>,
    rows: Vec<Vec<Value>>,
    truncated: bool,
    started: Instant,
) -> QueryPayload {
    QueryPayload {
        row_count: rows.len(),
        columns,
        rows,
        truncated,
        affected_rows: None,
        duration_ms: started.elapsed().as_millis() as u64,
    }
}

pub fn execute_sql(
    data_root: &Path,
    owner: &str,
    project: &str,
    sql: &str,
    params: &[Value],
    limit: usize,
    read_only: bool,
) -> Result<QueryPayload, PlatformError> {
    let trimmed = sql.trim().trim_end_matches(';').trim();
    if trimmed.is_empty() {
        return Err(PlatformError::new(
            "PLATFORM_SEKEJAP_QUERY_INVALID",
            "query.sql must not be empty for sekejap",
        ));
    }
    let started = Instant::now();
    let max_rows = limit.clamp(1, 5_000);

    if statement_is_write(trimmed) {
        if read_only {
            return Err(PlatformError::new(
                "PLATFORM_SEKEJAP_QUERY_READ_ONLY",
                "write statement rejected in read-only mode",
            ));
        }
        if insert_without_key(trimmed) {
            return Err(PlatformError::new(
                "PLATFORM_SEKEJAP_INSERT_NO_KEY",
                "INSERT names no `_key` column. Every row is addressed by `_key`, and sekejap would otherwise take \
                 the first column as the key. Add `_key` to the column list and bind a value for it — \
                 `INSERT INTO t (_key, title) VALUES ($1, $2)` with `--params \"{{ [$nodes.id.hex, input.body.title] }}\"`, \
                 where `$nodes.id` is a `crypto --op random_hex` node.",
            ));
        }
        let db = get_db(data_root, owner, project)?;
        let affected_rows = db
            .execute(trimmed, params)
            .map_err(|e| store_error("PLATFORM_SEKEJAP_QUERY_FAILED", e))?;
        if statement_changes_schema(trimmed) {
            sync_schema_to_repo(data_root, owner, project)?;
        }
        record_project_write(data_root, owner, project, (affected_rows as usize).max(1));
        return Ok(QueryPayload {
            columns: Vec::new(),
            rows: Vec::new(),
            row_count: 0,
            truncated: false,
            affected_rows: Some(affected_rows),
            duration_ms: started.elapsed().as_millis() as u64,
        });
    }

    if statement_is_explain(trimmed) {
        if statement_is_explain_analyze(trimmed) {
            return Err(PlatformError::new(
                "PLATFORM_SEKEJAP_QUERY_FAILED",
                "EXPLAIN ANALYZE is not built in sekejap 0.17 (QL_CONTRACT: options refused); \
                 EXPLAIN prints the plan the engine would build",
            ));
        }
        let db = get_db(data_root, owner, project)?;
        let plan = db
            .explain(strip_explain_prefix(trimmed), params)
            .map_err(|e| store_error("PLATFORM_SEKEJAP_QUERY_FAILED", e))?;
        let lines = plan
            .lines()
            .map(|line| vec![Value::String(line.to_string())])
            .collect::<Vec<_>>();
        let truncated = lines.len() > max_rows;
        let rows = lines.into_iter().take(max_rows).collect();
        return Ok(payload(
            vec![DbQueryColumn {
                name: "plan".to_string(),
                data_type: None,
            }],
            rows,
            truncated,
            started,
        ));
    }

    if statement_is_show_tables(trimmed) {
        let defs = list_tables(data_root, owner, project)?;
        let truncated = defs.len() > max_rows;
        let rows = defs
            .into_iter()
            .take(max_rows)
            .map(|def| vec![Value::String(def.table), json!(def.row_count)])
            .collect::<Vec<_>>();
        return Ok(payload(
            vec![
                DbQueryColumn {
                    name: "name".to_string(),
                    data_type: None,
                },
                DbQueryColumn {
                    name: "count".to_string(),
                    data_type: None,
                },
            ],
            rows,
            truncated,
            started,
        ));
    }

    let db = get_db(data_root, owner, project)?;

    // `SHOW <collection>`: the structure, from the catalog, in the shape the
    // Studio's structure table reads. Any other SHOW is the engine's.
    if let Some(target) = show_collection_target(trimmed) {
        if let Some(described) = db
            .describe(&target)
            .map_err(|e| store_error("PLATFORM_SEKEJAP_QUERY_FAILED", e))?
        {
            let rows = described
                .fields
                .iter()
                .map(|field| {
                    let indexes = described
                        .indexes_on(&field.name)
                        .flat_map(|index| index_family_kinds(&index.family).iter().copied())
                        .collect::<BTreeSet<_>>()
                        .into_iter()
                        .map(|kind| Value::String(kind.to_string()))
                        .collect::<Vec<_>>();
                    vec![
                        Value::String(field.name.clone()),
                        Value::String(field.declared.clone().unwrap_or_else(|| format!("{:?}", field.kind).to_ascii_uppercase())),
                        Value::Array(indexes),
                        Value::Bool(field.primary_key),
                    ]
                })
                .collect::<Vec<_>>();
            let columns = ["name", "type", "indexes", "primary_key"]
                .iter()
                .map(|name| DbQueryColumn {
                    name: name.to_string(),
                    data_type: None,
                })
                .collect();
            return Ok(payload(columns, rows, false, started));
        }
    }

    let rows = db
        .query(trimmed, params)
        .map_err(|e| store_error("PLATFORM_SEKEJAP_QUERY_FAILED", e))?;
    let (columns, rows, truncated) = payload_from_rows(rows, max_rows);
    Ok(payload(columns, rows, truncated, started))
}

pub fn execute_connection_query(
    data_root: &Path,
    owner: &str,
    project: &str,
    connection_id: &str,
    connection_slug: &str,
    req: &QueryProjectDbConnectionRequest,
) -> Result<ProjectDbConnectionQueryResult, PlatformError> {
    let result = execute_sql(
        data_root,
        owner,
        project,
        &req.sql,
        &[],
        req.limit.unwrap_or(200),
        req.read_only.unwrap_or(true),
    )?;
    Ok(ProjectDbConnectionQueryResult {
        connection_id: connection_id.to_string(),
        connection_slug: connection_slug.to_string(),
        database_kind: DB_KIND.to_string(),
        columns: result.columns,
        rows: result.rows,
        row_count: result.row_count,
        truncated: result.truncated,
        affected_rows: result.affected_rows,
        duration_ms: result.duration_ms,
    })
}

// ── structured writes ────────────────────────────────────────────────────

/// A document as sekejap stores it, checked against the collection's
/// declared fields.
///
/// A declared field is checked by its `Kind`; a vector is checked for its
/// dimension and stays in the document, because in 0.17 a `VECTOR(n)`
/// column is a typed field and not a sidecar. A field the collection does
/// not declare is kept as an extra — sekejap's own rule: a declaration is a
/// floor, not a fence.
fn typed_document(
    collection: &str,
    declared: &sekejap::Collection,
    fields: Map<String, Value>,
    field_dimensions: &mut BTreeMap<String, usize>,
) -> Result<Map<String, Value>, PlatformError> {
    let mut out = Map::with_capacity(fields.len());
    for (field, value) in fields {
        if matches!(field.as_str(), "_id" | "_key" | "_collection") {
            continue;
        }
        let Some(spec) = declared.field(&field) else {
            out.insert(field, value);
            continue;
        };
        if value.is_null() {
            out.insert(field, Value::Null);
            continue;
        }
        let ok = match &spec.kind {
            FieldKind::Text => value.is_string(),
            FieldKind::Int => match spec.declared.as_deref() {
                // TIMESTAMPTZ and DATE are Int on disk and take an ISO string
                // or the integer itself.
                Some("TIMESTAMPTZ") | Some("DATE") => value.is_string() || value.as_i64().is_some(),
                _ => value.as_i64().is_some() || value.as_u64().is_some(),
            },
            FieldKind::Real => value.as_f64().is_some(),
            FieldKind::Bool => value.is_boolean(),
            FieldKind::Json => true,
            FieldKind::Geo | FieldKind::Point => value.is_object() || value.is_array() || value.is_string(),
            FieldKind::Vector(n) => {
                let Some(vector) = value_as_f32_vec(&value) else {
                    return Err(schema_type_error(collection, &field, &format!("VECTOR({n})"), &value));
                };
                if vector.len() != *n {
                    return Err(PlatformError::new(
                        "PLATFORM_SEKEJAP_INSERT_INVALID",
                        format!(
                            "field '{field}' in collection '{collection}' is VECTOR({n}) and the value has {} dimension(s)",
                            vector.len()
                        ),
                    ));
                }
                field_dimensions.insert(field.clone(), *n);
                true
            }
        };
        if !ok {
            let expected = spec
                .declared
                .clone()
                .unwrap_or_else(|| format!("{:?}", spec.kind).to_ascii_uppercase());
            return Err(schema_type_error(collection, &field, &expected, &value));
        }
        out.insert(field, value);
    }
    Ok(out)
}

/// Records and edges, under one commit.
fn write_batch(
    db: &Db,
    collection: &str,
    rows: Vec<StructuredInsertRecord>,
    edges: Vec<StructuredInsertEdge>,
    mode: StructuredWriteMode,
) -> Result<StructuredWritePayload, PlatformError> {
    let started = Instant::now();
    let declared = db
        .describe(collection)
        .map_err(|e| store_error("PLATFORM_SEKEJAP_INSERT_FAILED", e))?
        .ok_or_else(|| {
            PlatformError::new(
                "PLATFORM_SEKEJAP_INSERT_UNKNOWN_COLLECTION",
                format!(
                    "collection '{collection}' does not exist; create it first — `CREATE TABLE {collection} (_key TEXT PRIMARY KEY, …)`"
                ),
            )
        })?;

    let mut field_dimensions = BTreeMap::<String, usize>::new();
    let mut tx = db
        .transaction()
        .map_err(|e| store_error("PLATFORM_SEKEJAP_INSERT_FAILED", e))?;
    let mut written = 0usize;
    for row in rows {
        let key = row.key.trim().to_string();
        let existing = {
            let engine = tx.database();
            let id = engine
                .collection(collection)
                .map_err(|e| store_error("PLATFORM_SEKEJAP_INSERT_FAILED", e))?
                .ok_or_else(|| {
                    PlatformError::new(
                        "PLATFORM_SEKEJAP_INSERT_UNKNOWN_COLLECTION",
                        format!("collection '{collection}' does not exist"),
                    )
                })?;
            engine
                .get(id, &key)
                .map_err(|e| store_error("PLATFORM_SEKEJAP_INSERT_FAILED", e))?
                .map(|entity| entity.document)
        };
        match mode {
            StructuredWriteMode::Insert if existing.is_some() => {
                return Err(PlatformError::new(
                    "PLATFORM_SEKEJAP_INSERT_EXISTS",
                    format!("row '{collection}/{key}' already exists"),
                ));
            }
            StructuredWriteMode::Update if existing.is_none() => {
                return Err(PlatformError::new(
                    "PLATFORM_SEKEJAP_INSERT_MISSING",
                    format!("row '{collection}/{key}' does not exist"),
                ));
            }
            _ => {}
        }
        let mut document = match (mode, existing) {
            (StructuredWriteMode::Merge, Some(Value::Object(current))) => current,
            _ => Map::new(),
        };
        let typed = typed_document(collection, &declared, row.fields, &mut field_dimensions)?;
        document.extend(typed);
        tx.put((collection, key.as_str()), &Value::Object(document))
            .map_err(|e| store_error("PLATFORM_SEKEJAP_INSERT_FAILED", e))?;
        written += 1;
    }
    for edge in &edges {
        let properties = Value::Object(edge.fields.clone());
        tx.link_with(
            (edge.from_target.trim(), edge.from_key.trim()),
            edge.edge_type.trim(),
            (edge.to_target.trim(), edge.to_key.trim()),
            &properties,
        )
        .map_err(|e| store_error("PLATFORM_SEKEJAP_INSERT_EDGE", e))?;
    }
    tx.commit()
        .map_err(|e| store_error("PLATFORM_SEKEJAP_INSERT_FAILED", e))?;

    Ok(StructuredWritePayload {
        affected_rows: written + edges.len(),
        optimized_fields: field_dimensions.keys().cloned().collect(),
        field_dimensions,
        duration_ms: started.elapsed().as_millis() as u64,
    })
}

fn check_records(
    collection: &str,
    rows: &[StructuredInsertRecord],
) -> Result<(), PlatformError> {
    if collection.trim().is_empty() {
        return Err(PlatformError::new(
            "PLATFORM_SEKEJAP_INSERT_INVALID",
            "collection must not be empty",
        ));
    }
    for row in rows {
        if row.key.trim().is_empty() {
            return Err(PlatformError::new(
                "PLATFORM_SEKEJAP_INSERT_INVALID",
                "row key must not be empty",
            ));
        }
    }
    Ok(())
}

pub fn bulk_write_records(
    data_root: &Path,
    owner: &str,
    project: &str,
    collection: &str,
    rows: Vec<StructuredInsertRecord>,
    mode: StructuredWriteMode,
) -> Result<StructuredWritePayload, PlatformError> {
    bulk_insert(data_root, owner, project, collection, rows, Vec::new(), mode)
}

/// Records into one collection and edges between any rows, as one commit.
///
/// Both endpoints of an edge must exist when the edge is written — sekejap
/// validates them — so an edge to a row this same batch writes is fine, and
/// one to a row nothing wrote is refused naming it.
pub fn bulk_insert(
    data_root: &Path,
    owner: &str,
    project: &str,
    collection: &str,
    rows: Vec<StructuredInsertRecord>,
    edges: Vec<StructuredInsertEdge>,
    mode: StructuredWriteMode,
) -> Result<StructuredWritePayload, PlatformError> {
    let collection = collection.trim();
    check_records(collection, &rows)?;
    for edge in &edges {
        if edge.from_target.trim().is_empty()
            || edge.from_key.trim().is_empty()
            || edge.edge_type.trim().is_empty()
            || edge.to_target.trim().is_empty()
            || edge.to_key.trim().is_empty()
        {
            return Err(PlatformError::new(
                "PLATFORM_SEKEJAP_INSERT_EDGE_INVALID",
                "edge from.target, from.key, type, to.target, and to.key must be non-empty",
            ));
        }
    }
    let db = get_db(data_root, owner, project)?;
    let out = write_batch(&db, collection, rows, edges, mode)?;
    record_project_write(data_root, owner, project, out.affected_rows);
    Ok(out)
}

fn value_as_f32_vec(value: &Value) -> Option<Vec<f32>> {
    let arr = value.as_array()?;
    if arr.is_empty() {
        return None;
    }
    let mut out = Vec::with_capacity(arr.len());
    for item in arr {
        out.push(item.as_f64()? as f32);
    }
    Some(out)
}

fn schema_type_error(
    collection: &str,
    field: &str,
    expected: &str,
    value: &Value,
) -> PlatformError {
    PlatformError::new(
        "PLATFORM_SEKEJAP_INSERT_TYPE",
        format!(
            "field '{field}' in collection '{collection}' expects {expected}, got {}",
            json_type_name(value)
        ),
    )
}

fn json_type_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_root() -> tempfile::TempDir {
        tempfile::tempdir().expect("temp dir")
    }

    fn attribute(name: &str, kind: &str, index_types: &[&str]) -> CollectionAttribute {
        CollectionAttribute {
            name: name.to_string(),
            kind: kind.to_string(),
            index_types: index_types.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn create(root: &Path, owner: &str, project: &str, table: &str, attrs: Vec<CollectionAttribute>) -> SimpleTableDefinition {
        create_table(
            root,
            owner,
            project,
            &CreateSimpleTableRequest {
                table: table.to_string(),
                attributes: attrs,
                hash_indexed_fields: Vec::new(),
                range_indexed_fields: Vec::new(),
            },
        )
        .expect("create table")
    }

    #[test]
    fn create_table_persists_empty_table_definition() {
        let tmp = tmp_root();
        let created = create(tmp.path(), "alice", "demo", "posts", vec![attribute("title", "string", &["hash"])]);
        assert_eq!(created.table, "posts");
        assert_eq!(created.row_count, 0);

        let items = list_tables(tmp.path(), "alice", "demo").expect("list tables");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].table, "posts");
        // The scalar index sekejap gives every text column answers equality
        // and range alike, so the column reads back as both.
        let title = items[0].attributes.iter().find(|a| a.name == "title").expect("title");
        assert!(title.index_types.contains(&"hash".to_string()));
        assert!(title.index_types.contains(&"range".to_string()));
    }

    #[test]
    fn create_table_syncs_portable_schema_to_repo() {
        let tmp = tmp_root();
        create(tmp.path(), "alice", "demo", "posts", vec![attribute("title", "string", &["hash"])]);

        let schema_path = tmp
            .path()
            .join("users/alice/demo/repo/schemas/sekejap/schema.json");
        assert!(schema_path.is_file());
        assert!(
            !tmp.path()
                .join("users/alice/demo/repo/schemas/sekejap/tables")
                .exists()
        );

        let schema: Value =
            serde_json::from_str(&std::fs::read_to_string(schema_path).expect("schema file"))
                .expect("schema json");
        assert_eq!(schema["apiVersion"], "zebflow.com/v1");
        assert_eq!(schema["kind"], "DatabaseSchema");
        assert_eq!(schema["metadata"]["name"], BUILTIN_CONNECTION_SLUG);
        assert_eq!(schema["spec"]["tables"][0]["table"], "posts");
        assert!(schema["spec"]["tables"][0].get("row_count").is_none());
    }

    #[test]
    fn apply_schema_from_repo_hydrates_live_store() {
        let tmp = tmp_root();
        create(
            tmp.path(),
            "alice",
            "source",
            "places",
            vec![
                attribute("name", "string", &["hash", "fulltext"]),
                attribute("geometry", "geo", &["spatial"]),
                attribute("embedding", "vector(3)", &["vector"]),
            ],
        );

        let source_schema = tmp
            .path()
            .join("users/alice/source/repo/schemas/sekejap/schema.json");
        let target_schema = tmp
            .path()
            .join("users/alice/clone/repo/schemas/sekejap/schema.json");
        std::fs::create_dir_all(target_schema.parent().expect("schema parent"))
            .expect("target schema dir");
        std::fs::copy(source_schema, &target_schema).expect("copy schema");

        let report = apply_schema_from_repo(tmp.path(), "alice", "clone")
            .expect("apply schema")
            .expect("schema report");
        assert_eq!(report.tables_created, vec!["places"]);

        let tables = list_tables(tmp.path(), "alice", "clone").expect("list target tables");
        assert_eq!(tables.len(), 1);
        let places = &tables[0];
        assert_eq!(places.table, "places");
        assert!(places.hash_indexed_fields.contains(&"name".to_string()));
        assert!(places.fulltext_fields.contains(&"name".to_string()));
        assert!(places.spatial_fields.contains(&"geometry".to_string()));
        assert!(places.vector_fields.contains(&"embedding".to_string()));
        let embedding = places.attributes.iter().find(|a| a.name == "embedding").expect("embedding");
        assert_eq!(embedding.kind, "vector(3)", "the dimension survives the round trip");
    }

    #[test]
    fn a_vector_attribute_without_its_dimension_is_refused_by_name() {
        let tmp = tmp_root();
        let err = create_table(
            tmp.path(),
            "alice",
            "demo",
            &CreateSimpleTableRequest {
                table: "docs".to_string(),
                attributes: vec![attribute("embedding", "vector", &["vector"])],
                hash_indexed_fields: Vec::new(),
                range_indexed_fields: Vec::new(),
            },
        )
        .unwrap_err();
        assert_eq!(err.code, "PLATFORM_SEKEJAP_VECTOR_DIMENSION");
        assert!(err.message.contains("vector(384)"), "{}", err.message);
    }

    #[test]
    fn apply_schema_from_repo_rejects_legacy_unversioned_shape() {
        let tmp = tmp_root();
        let schema_path = tmp
            .path()
            .join("users/alice/demo/repo/schemas/sekejap/schema.json");
        std::fs::create_dir_all(schema_path.parent().expect("schema parent"))
            .expect("create schema dir");
        std::fs::write(
            &schema_path,
            r#"{"schema_version":"zebflow.sekejap.schema.v1","database":"sekejap","connection_slug":"default-multimodel","tables":[]}"#,
        )
        .expect("legacy schema");

        let err = apply_schema_from_repo(tmp.path(), "alice", "demo").unwrap_err();
        assert_eq!(err.code, "PLATFORM_SEKEJAP_SCHEMA_READ");
        assert!(err.message.contains("invalid contract"));
        assert!(err.message.contains("schema_version"));
    }

    /// A repository written by the earlier schema writer still carries a
    /// `tables/` directory. Syncing clears it out, because nothing reads it.
    #[test]
    fn sync_schema_clears_the_legacy_tables_directory() {
        let tmp = tmp_root();
        create(tmp.path(), "alice", "demo", "posts", Vec::new());
        let tables_dir = tmp
            .path()
            .join("users/alice/demo/repo/schemas/sekejap/tables");
        std::fs::create_dir_all(&tables_dir).expect("create legacy dir");
        std::fs::write(tables_dir.join("stale.json"), "{}").expect("stale file");

        let report = sync_schema_to_repo(tmp.path(), "alice", "demo").expect("sync schema");
        assert!(report.changed);
        assert!(!tables_dir.exists());
        assert!(
            report
                .files_removed
                .iter()
                .any(|path| path == "schemas/sekejap/tables/stale.json")
        );
    }

    #[test]
    fn delete_table_removes_live_discovered_collection() {
        let tmp = tmp_root();
        {
            let db = get_db(tmp.path(), "alice", "demo").expect("open db");
            db.execute("CREATE TABLE live_only (_key TEXT PRIMARY KEY, note TEXT)", &[])
                .expect("create live table");
            db.execute("INSERT INTO live_only (_key, note) VALUES ('row-1', 'x')", &[])
                .expect("insert live row");
        }

        assert!(
            list_tables(tmp.path(), "alice", "demo")
                .expect("list before delete")
                .iter()
                .any(|item| item.table == "live_only")
        );

        delete_table(tmp.path(), "alice", "demo", "live_only").expect("delete table");

        assert!(
            !list_tables(tmp.path(), "alice", "demo")
                .expect("list after delete")
                .iter()
                .any(|item| item.table == "live_only")
        );
    }

    #[test]
    fn schema_changing_sql_syncs_repo_schema() {
        let tmp = tmp_root();
        assert!(statement_changes_schema("CREATE TABLE posts (_key TEXT PRIMARY KEY)"));
        assert!(statement_changes_schema("DROP INDEX posts_title_btree"));
        assert!(!statement_changes_schema("INSERT INTO posts (_key) VALUES ('a')"));

        execute_sql(
            tmp.path(),
            "alice",
            "demo",
            "CREATE TABLE posts (_key TEXT PRIMARY KEY, title TEXT)",
            &[],
            100,
            false,
        )
        .expect("create table sql");

        let schema_path = tmp
            .path()
            .join("users/alice/demo/repo/schemas/sekejap/schema.json");
        assert!(schema_path.is_file());
        let schema: Value =
            serde_json::from_str(&std::fs::read_to_string(schema_path).expect("schema file"))
                .expect("schema json");
        assert_eq!(schema["spec"]["tables"][0]["table"], "posts");
    }

    #[test]
    fn execute_sql_reads_and_writes_rows() {
        let tmp = tmp_root();
        create(tmp.path(), "alice", "demo", "posts", vec![attribute("title", "string", &[])]);

        let write = execute_sql(
            tmp.path(),
            "alice",
            "demo",
            "INSERT INTO posts (_key, title) VALUES ('first', 'Hello')",
            &[],
            100,
            false,
        )
        .expect("insert");
        assert_eq!(write.affected_rows, Some(1));

        let read = execute_sql(
            tmp.path(),
            "alice",
            "demo",
            "SELECT _key, title FROM posts LIMIT 20",
            &[],
            100,
            true,
        )
        .expect("select");
        assert_eq!(read.row_count, 1);
        assert_eq!(read.columns.len(), 2);
        assert_eq!(read.rows[0][1], Value::String("Hello".to_string()));
    }

    /// sekejap would take the first column as the key; Zebflow says so
    /// instead of letting a title become one.
    #[test]
    fn an_insert_without_key_is_refused_by_name() {
        let tmp = tmp_root();
        create(tmp.path(), "alice", "demo", "posts", vec![attribute("title", "string", &[])]);
        let err = execute_sql(
            tmp.path(),
            "alice",
            "demo",
            "INSERT INTO posts (title) VALUES ('Hello')",
            &[],
            100,
            false,
        )
        .unwrap_err();
        assert_eq!(err.code, "PLATFORM_SEKEJAP_INSERT_NO_KEY");
        assert!(err.message.contains("_key"), "{}", err.message);
    }

    #[test]
    fn execute_sql_supports_show_tables() {
        let tmp = tmp_root();
        create(tmp.path(), "alice", "demo", "posts", vec![attribute("title", "string", &[])]);

        let read = execute_sql(tmp.path(), "alice", "demo", "SHOW TABLES", &[], 100, true)
            .expect("show tables");
        assert_eq!(read.row_count, 1);
        assert_eq!(read.columns.len(), 2);
        assert_eq!(read.rows[0][0], Value::String("posts".to_string()));
        assert_eq!(read.rows[0][1], json!(0));
    }

    #[test]
    fn execute_sql_supports_show_collection_structure() {
        let tmp = tmp_root();
        create(
            tmp.path(),
            "alice",
            "demo",
            "posts",
            vec![attribute("title", "string", &[]), attribute("views", "number", &[])],
        );

        let read = execute_sql(tmp.path(), "alice", "demo", "SHOW posts", &[], 100, true)
            .expect("show structure");
        assert!(read.row_count >= 3, "_key, title, views");
        assert_eq!(read.columns.len(), 4);
        assert!(
            read.rows
                .iter()
                .any(|row| row.first() == Some(&Value::String("title".to_string())))
        );
    }

    #[test]
    fn execute_sql_explains_a_select() {
        let tmp = tmp_root();
        create(tmp.path(), "alice", "demo", "posts", vec![attribute("title", "string", &[])]);
        let read = execute_sql(
            tmp.path(),
            "alice",
            "demo",
            "EXPLAIN SELECT _key FROM posts WHERE title = 'x'",
            &[],
            100,
            true,
        )
        .expect("explain");
        assert_eq!(read.columns[0].name, "plan");
        assert!(read.row_count >= 1);
    }

    #[test]
    fn execute_sql_supports_graph_table_traversal() {
        let tmp = tmp_root();
        create(tmp.path(), "alice", "demo", "people", vec![attribute("name", "string", &[])]);

        let db = get_db(tmp.path(), "alice", "demo").expect("db");
        db.execute("INSERT INTO people (_key, name) VALUES ('alice', 'Alice')", &[])
            .expect("insert alice");
        db.execute("INSERT INTO people (_key, name) VALUES ('bob', 'Bob')", &[])
            .expect("insert bob");
        db.link(("people", "alice"), "knows", ("people", "bob"))
            .expect("link");

        let read = execute_sql(
            tmp.path(),
            "alice",
            "demo",
            "SELECT _key, name FROM GRAPH_TABLE (base MATCH (a:people WHERE a._key = 'alice')-[:knows]->(b:people) COLUMNS (b._key AS _key, b.name AS name))",
            &[],
            100,
            true,
        )
        .expect("graph table");
        assert_eq!(read.row_count, 1);
        assert_eq!(read.rows[0][0], Value::String("bob".to_string()));
        assert_eq!(read.rows[0][1], Value::String("Bob".to_string()));

        let incoming = execute_sql(
            tmp.path(),
            "alice",
            "demo",
            "SELECT _key, name FROM GRAPH_TABLE (base MATCH (b:people WHERE b._key = 'bob')<-[:knows]-(a:people) COLUMNS (a._key AS _key, a.name AS name))",
            &[],
            100,
            true,
        )
        .expect("incoming");
        assert_eq!(incoming.row_count, 1);
        assert_eq!(incoming.rows[0][0], Value::String("alice".to_string()));
    }

    #[test]
    fn execute_sql_supports_bind_params_for_write_and_read() {
        let tmp = tmp_root();
        create(tmp.path(), "alice", "demo", "posts", vec![attribute("title", "string", &[])]);

        let write = execute_sql(
            tmp.path(),
            "alice",
            "demo",
            "INSERT INTO posts (_key, title) VALUES ($1, $2)",
            &[json!("first"), json!("Hello")],
            100,
            false,
        )
        .expect("insert with params");
        assert_eq!(write.affected_rows, Some(1));

        let read = execute_sql(
            tmp.path(),
            "alice",
            "demo",
            "SELECT _key, title FROM posts WHERE _key = $1",
            &[json!("first")],
            100,
            true,
        )
        .expect("select with params");
        assert_eq!(read.row_count, 1);
        assert_eq!(read.rows[0][0], Value::String("first".to_string()));
        assert_eq!(read.rows[0][1], Value::String("Hello".to_string()));
    }

    #[test]
    fn bulk_write_records_types_declared_fields_and_merge_preserves_fields() {
        let tmp = tmp_root();
        execute_sql(
            tmp.path(),
            "alice",
            "demo",
            "CREATE TABLE docs (_key TEXT PRIMARY KEY, title TEXT, category TEXT, embedding VECTOR(3)) WITH (vector: [embedding])",
            &[],
            100,
            false,
        )
        .expect("create docs table");

        let first = bulk_write_records(
            tmp.path(),
            "alice",
            "demo",
            "docs",
            vec![StructuredInsertRecord {
                key: "d1".to_string(),
                fields: Map::from_iter([
                    ("title".to_string(), json!("First")),
                    ("category".to_string(), json!("alpha")),
                    ("embedding".to_string(), json!([0.1, 0.2, 0.3])),
                ]),
            }],
            StructuredWriteMode::Upsert,
        )
        .expect("first insert");
        assert_eq!(first.affected_rows, 1);
        assert_eq!(first.field_dimensions.get("embedding"), Some(&3));

        let merged = bulk_write_records(
            tmp.path(),
            "alice",
            "demo",
            "docs",
            vec![StructuredInsertRecord {
                key: "d1".to_string(),
                fields: Map::from_iter([
                    ("title".to_string(), json!("Updated")),
                    ("embedding".to_string(), json!([0.4, 0.5, 0.6])),
                ]),
            }],
            StructuredWriteMode::Merge,
        )
        .expect("merge insert");
        assert_eq!(merged.affected_rows, 1);

        let db = get_db(tmp.path(), "alice", "demo").expect("db");
        let document = db.get(("docs", "d1")).expect("get").expect("row");
        assert_eq!(document["title"], json!("Updated"));
        assert_eq!(document["category"], json!("alpha"), "merge kept the field it was not given");
        // A VECTOR(n) column is stored as f32, so it reads back as the f64
        // of each f32 — compare at f32 precision.
        let stored: Vec<f32> = document["embedding"]
            .as_array()
            .expect("vector array")
            .iter()
            .map(|v| v.as_f64().expect("number") as f32)
            .collect();
        assert_eq!(stored, vec![0.4f32, 0.5, 0.6]);

        let wrong_type = bulk_write_records(
            tmp.path(),
            "alice",
            "demo",
            "docs",
            vec![StructuredInsertRecord {
                key: "d2".to_string(),
                fields: Map::from_iter([("title".to_string(), json!([4, 5]))]),
            }],
            StructuredWriteMode::Upsert,
        )
        .unwrap_err();
        assert_eq!(wrong_type.code, "PLATFORM_SEKEJAP_INSERT_TYPE");

        let wrong_dimension = bulk_write_records(
            tmp.path(),
            "alice",
            "demo",
            "docs",
            vec![StructuredInsertRecord {
                key: "d3".to_string(),
                fields: Map::from_iter([("embedding".to_string(), json!([1.0, 2.0]))]),
            }],
            StructuredWriteMode::Upsert,
        )
        .unwrap_err();
        assert!(wrong_dimension.message.contains("VECTOR(3)"), "{}", wrong_dimension.message);
    }

    /// Records and their edges are one commit, and an edge to a row nothing
    /// wrote is refused naming it rather than dangling.
    #[test]
    fn bulk_insert_links_rows_in_the_same_batch_and_refuses_a_missing_endpoint() {
        let tmp = tmp_root();
        create(tmp.path(), "alice", "demo", "people", vec![attribute("name", "string", &[])]);

        let out = bulk_insert(
            tmp.path(),
            "alice",
            "demo",
            "people",
            vec![
                StructuredInsertRecord { key: "a".into(), fields: Map::from_iter([("name".to_string(), json!("A"))]) },
                StructuredInsertRecord { key: "b".into(), fields: Map::from_iter([("name".to_string(), json!("B"))]) },
            ],
            vec![StructuredInsertEdge {
                from_target: "people".into(),
                from_key: "a".into(),
                edge_type: "knows".into(),
                to_target: "people".into(),
                to_key: "b".into(),
                fields: Map::from_iter([("since".to_string(), json!(2026))]),
                strength: 1.0,
            }],
            StructuredWriteMode::Insert,
        )
        .expect("insert with edge");
        assert_eq!(out.affected_rows, 3);

        let db = get_db(tmp.path(), "alice", "demo").expect("db");
        let friends = db
            .neighbours(("people", "a"), Some("knows"), sekejap::Direction::Outgoing, 16)
            .expect("neighbours");
        assert_eq!(friends.len(), 1);
        assert_eq!(friends[0].key, "b");

        let dangling = bulk_insert(
            tmp.path(),
            "alice",
            "demo",
            "people",
            vec![StructuredInsertRecord { key: "c".into(), fields: Map::new() }],
            vec![StructuredInsertEdge {
                from_target: "people".into(),
                from_key: "c".into(),
                edge_type: "knows".into(),
                to_target: "people".into(),
                to_key: "nobody".into(),
                fields: Map::new(),
                strength: 1.0,
            }],
            StructuredWriteMode::Insert,
        )
        .unwrap_err();
        assert_eq!(dangling.code, "PLATFORM_SEKEJAP_INSERT_EDGE");
        assert!(dangling.message.contains("nobody"), "{}", dangling.message);
        // The failed batch wrote nothing: `c` is not there either.
        assert!(!db.exists(("people", "c")).expect("exists"));
    }

    #[test]
    fn project_health_and_compact_report_the_store() {
        let tmp = tmp_root();
        execute_sql(
            tmp.path(),
            "alice",
            "demo",
            "CREATE TABLE docs (_key TEXT PRIMARY KEY, title TEXT)",
            &[],
            100,
            false,
        )
        .expect("create table");
        execute_sql(
            tmp.path(),
            "alice",
            "demo",
            "INSERT INTO docs (_key, title) VALUES ('d1', 'First')",
            &[],
            100,
            false,
        )
        .expect("insert");

        let before = project_health(tmp.path(), "alice", "demo").expect("health");
        assert_eq!(before.node_count, 1);
        assert_eq!(before.edge_count, 0);
        assert!(before.data_bytes + before.wal_bytes > 0);

        let report = compact_project(tmp.path(), "alice", "demo").expect("compact");
        assert_eq!(report.operation, "compact");
        assert_eq!(report.after.node_count, 1);
        let sync = sync_project(tmp.path(), "alice", "demo").expect("sync");
        assert_eq!(sync.operation, "sync");
    }

    /// A 0.16 directory that never held a row is retired and the store is
    /// created fresh beside it; one that held rows is refused by name.
    #[test]
    fn a_legacy_store_is_retired_when_empty_and_refused_when_it_holds_rows() {
        let tmp = tmp_root();
        let dir = project_dir(tmp.path(), "alice", "demo");
        std::fs::create_dir_all(&dir).expect("dir");
        std::fs::write(dir.join("wal.log"), [0u8; 8]).expect("wal header");
        std::fs::write(dir.join("payloads.bin"), b"").expect("payloads");
        std::fs::write(dir.join("gin.bin"), [0u8; 12]).expect("gin");
        std::fs::write(dir.join("db.lock"), b"").expect("lock");

        create(tmp.path(), "alice", "demo", "posts", Vec::new());
        assert!(dir.join("legacy-0.16/wal.log").is_file());
        assert!(!dir.join("wal.log").exists());
        assert!(dir.join("data").exists(), "the 0.17 store was created");

        let full = project_dir(tmp.path(), "alice", "loaded");
        std::fs::create_dir_all(&full).expect("dir");
        std::fs::write(full.join("wal.log"), [0u8; 64]).expect("wal with rows");
        std::fs::write(full.join("payloads.bin"), b"rows").expect("payloads");
        let err = match get_db(tmp.path(), "alice", "loaded") {
            Ok(_) => panic!("a 0.16 store with rows must not open"),
            Err(err) => err,
        };
        assert_eq!(err.code, "PLATFORM_SEKEJAP_LEGACY_STORE");
        assert!(full.join("wal.log").exists(), "nothing was moved or destroyed");
    }
}
