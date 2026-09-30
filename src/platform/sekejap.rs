//! The project's embedded multimodel store, on sekejap 0.19.
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
    CollectionAttribute, CreateSimpleTableRequest, DbObjectNode, DbQueryColumn, EdgeTableDefinition,
    EdgeTableReference,
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
    move_to_current_format(&dir)?;
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

/// Move a store written by sekejap 0.18 into the format this build reads.
///
/// 0.19 opens no 0.18-format store. `Db::upgrade` builds the new store beside
/// the old one (`sekejap.v019-upgrading`), verifies it, and swaps it in,
/// keeping the original untouched as `sekejap.v018-backup`; an interrupted
/// move is finished by the next call, and a 0.19 store is left alone. No
/// handle may be open meanwhile: callers hold the pool lock, or have just
/// taken the store out of the pool.
fn move_to_current_format(dir: &Path) -> Result<(), PlatformError> {
    // Zebflow's move to sekejap 0.17 retired an empty 0.16 store's files into
    // `legacy-0.16/` inside the store. The 0.19 upgrader reads a store as flat
    // files and refuses a folder in it, so it goes beside the store, kept.
    let retired = dir.join(LEGACY_016_DIR);
    if retired.is_dir() {
        let beside = dir.with_file_name(format!("{STORE_DIR}.{LEGACY_016_DIR}"));
        std::fs::rename(&retired, &beside).map_err(|err| {
            PlatformError::new(
                "PLATFORM_SEKEJAP_UPGRADE",
                format!("could not move {} out of the store before its format move: {err}", retired.display()),
            )
        })?;
    }
    let backup = Db::upgrade(dir).map_err(|err| {
        PlatformError::new(
            "PLATFORM_SEKEJAP_UPGRADE",
            format!("could not move the sekejap store at {} to this version's format: {err}", dir.display()),
        )
    })?;
    if let Some(backup) = backup {
        eprintln!(
            "sekejap: moved {} to this version's format; the original is kept at {}",
            dir.display(),
            backup.display()
        );
    }
    Ok(())
}

/// The store's folder name, and the suffixes of the folders a format move
/// leaves beside it. A transfer carries the store, never these.
pub const STORE_DIR: &str = "sekejap";
pub const BACKUP_SUFFIX: &str = ".v018-backup";
pub const STAGING_SUFFIX: &str = ".v019-upgrading";
/// The folder the 0.17 move retired an empty 0.16 store into; beside the
/// store as `sekejap.legacy-0.16` once 0.19 has moved it out.
pub const LEGACY_016_DIR: &str = "legacy-0.16";

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
    move_to_current_format(dir)?;
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
    /// `_key`'s own `DEFAULT` (`ulid()`), when the table mints its keys.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub key_default: String,
    /// Named schema; empty for `public`.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub schema: String,
    /// Present for an edge table.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edge: Option<EdgeTableDefinition>,
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
        .join(STORE_DIR)
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
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
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
        "number" | "real" | "integer" | "int" | "float" | "bigint" | "smallint" | "double precision" => {
            "number".to_string()
        }
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

/// A kind the Studio sends that is already one of sekejap's SQL types,
/// written the way the catalog writes it back (`INT`, `TIMESTAMPTZ`,
/// `GEOMETRY(Point,4326)`, `VECTOR(384)`). Coarse kinds (`string`,
/// `number`) answer nothing and go through `map_field_type`.
fn sql_type_of(kind: &str) -> Option<String> {
    let raw = kind.trim();
    let upper = raw.to_ascii_uppercase();
    let base = upper.split('(').next().unwrap_or("").trim();
    let known = [
        "TEXT", "INT", "INTEGER", "BIGINT", "SMALLINT", "REAL", "DOUBLE PRECISION", "BOOLEAN",
        "JSONB", "TIMESTAMPTZ", "DATE", "GEOMETRY", "VECTOR",
    ];
    if !known.contains(&base) {
        return None;
    }
    if base == "GEOMETRY" && raw.contains('(') {
        // Keep the subtype's own case: `GEOMETRY(Point,4326)`.
        let inner = raw[raw.find('(')? + 1..].trim_end_matches(')').trim();
        let parts = inner.split(',').map(str::trim).collect::<Vec<_>>();
        let shape = parts.first().copied().unwrap_or("");
        let valid = !shape.is_empty() && shape.chars().all(|c| c.is_ascii_alphabetic())
            && parts.get(1).is_none_or(|srid| srid.chars().all(|c| c.is_ascii_digit()));
        return valid.then(|| format!("GEOMETRY({})", parts.join(",")));
    }
    if base == "VECTOR" {
        let dims = upper.split('(').nth(1)?.trim_end_matches(')').trim().parse::<usize>().ok()?;
        return (dims > 0).then(|| format!("VECTOR({dims})"));
    }
    Some(base.to_string())
}

/// A column DEFAULT as SQL, never as raw text: a number, a boolean, `NULL`,
/// one of the generators, or an already quoted string stay as written;
/// anything else is a text constant and is quoted here.
fn default_sql(raw: &str) -> String {
    let value = raw.trim();
    if value.is_empty() {
        return String::new();
    }
    let lower = value.to_ascii_lowercase();
    let is_number = value.parse::<f64>().is_ok() && !value.contains(char::is_whitespace);
    let is_keyword = matches!(lower.as_str(), "true" | "false" | "null" | "now()" | "ulid()" | "uuid4()");
    let is_quoted = value.len() >= 2
        && value.starts_with('\'')
        && value.ends_with('\'')
        && !value[1..value.len() - 1].replace("''", "").contains('\'');
    if is_number || is_keyword || is_quoted {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "''"))
    }
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
            unique: attr.unique,
            // The exact SQL type: as the catalog declared it, or as the Studio
            // chose it from the SQL types the driver lists. Either way it is
            // read back through the type grammar, because it lands in a
            // statement: a word that is not a type is dropped, never pasted.
            declared: sql_type_of(&attr.declared)
                .or_else(|| sql_type_of(&attr.kind))
                .unwrap_or_default(),
            default: default_sql(&attr.default),
            not_null: attr.not_null,
            primary_key: attr.primary_key,
        });
    }
    attrs
}

fn normalize_definition(
    req: &CreateSimpleTableRequest,
) -> Result<SimpleTableDefinition, PlatformError> {
    // `geo.places` creates `places` in schema `geo`; a bare name is `public`.
    let table = qualified_slug(&req.table);
    if table.is_empty() {
        return Err(PlatformError::new(
            "PLATFORM_SEKEJAP_TABLE_INVALID",
            "table slug must not be empty",
        ));
    }
    let key_default = req.key_default.trim().to_string();
    if !key_default.is_empty() && key_default != "ulid()" && key_default != "uuid4()" {
        return Err(PlatformError::new(
            "PLATFORM_SEKEJAP_TABLE_INVALID",
            format!("a key is generated by ulid() or uuid4(), not '{key_default}'"),
        ));
    }
    let schema = table.split_once('.').map(|(schema, _)| schema.to_string()).unwrap_or_default();
    let mut attrs = normalize_attributes(&req.attributes);
    let edge = match &req.edge {
        Some(edge) => {
            if !key_default.is_empty() {
                return Err(PlatformError::new(
                    "PLATFORM_SEKEJAP_TABLE_INVALID",
                    "an edge table has no generated key: its two ends are its rows' address",
                ));
            }
            Some(normalize_edge(&table, edge, &mut attrs)?)
        }
        None => None,
    };
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
        key_default,
        schema,
        edge,
    })
}

/// The edge part of a create request, checked, as the catalog will report
/// it. The two end columns are put first among the columns, as `TEXT`
/// holding the ends' keys; a property may not reuse either name.
fn normalize_edge(
    table: &str,
    req: &crate::platform::model::CreateEdgeTableRequest,
    attrs: &mut Vec<CollectionAttribute>,
) -> Result<EdgeTableDefinition, PlatformError> {
    let invalid = |message: String| PlatformError::new("PLATFORM_SEKEJAP_TABLE_INVALID", message);
    let source = slug_segment(&req.source);
    let destination = slug_segment(&req.destination);
    let source_table = qualified_slug(&req.source_table);
    let destination_table = qualified_slug(&req.destination_table);
    if source.is_empty() || destination.is_empty() || source.starts_with('_') || destination.starts_with('_') {
        return Err(invalid("an edge table names a column for each end, not starting with `_`".to_string()));
    }
    if source == destination {
        return Err(invalid(format!("the two ends need two columns, not `{source}` twice")));
    }
    if source_table.is_empty() || destination_table.is_empty() {
        return Err(invalid("an edge table names the table each end reaches".to_string()));
    }
    if let Some(taken) = attrs.iter().find(|attr| attr.name == source || attr.name == destination) {
        return Err(invalid(format!("`{}` is an end column and cannot also be a property", taken.name)));
    }
    let end = |name: &str| CollectionAttribute {
        name: name.to_string(),
        kind: "string".to_string(),
        declared: "TEXT".to_string(),
        ..Default::default()
    };
    attrs.splice(0..0, [end(&source), end(&destination)]);
    let bare = table.rsplit('.').next().unwrap_or(table);
    let label = slug_segment(&req.label);
    Ok(EdgeTableDefinition {
        references: vec![
            EdgeTableReference { column: source.clone(), table: source_table.clone() },
            EdgeTableReference { column: destination.clone(), table: destination_table.clone() },
        ],
        key: if req.one_per_pair { vec![source.clone(), destination.clone()] } else { Vec::new() },
        source,
        source_table,
        destination,
        destination_table,
        label: if label.is_empty() { bare.to_string() } else { label },
        graph: String::new(),
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
    if let Some(edge) = &def.edge {
        return build_create_edge_table_sql(def, edge);
    }
    // A table whose key is a named column declares that column PRIMARY KEY
    // and has no `_key` line of its own.
    let mut columns = Vec::new();
    if !def.attributes.iter().any(|attr| attr.primary_key) {
        let mut key = "_key TEXT PRIMARY KEY".to_string();
        if !def.key_default.is_empty() {
            key.push_str(&format!(" DEFAULT {}", def.key_default));
        }
        columns.push(key);
    }
    for attr in &def.attributes {
        columns.push(column_sql(attr)?);
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

/// An edge table's `CREATE TABLE`: no `_key` of its own, its `REFERENCES`
/// columns naming the rows an edge joins, and the primary key that decides
/// how many edges one pair may have. It becomes an edge table when a
/// property graph declares it (`build_graph_sql`).
/// One column as it is declared again: the database's own type when it
/// reported one, then PRIMARY KEY, NOT NULL, DEFAULT and UNIQUE.
fn column_sql(attr: &CollectionAttribute) -> Result<String, PlatformError> {
    let kind = if attr.declared.is_empty() {
        map_field_type(&attr.kind)?
    } else {
        attr.declared.clone()
    };
    let mut column = format!("{} {kind}", attr.name);
    if attr.primary_key {
        column.push_str(" PRIMARY KEY");
    }
    if attr.not_null {
        column.push_str(" NOT NULL");
    }
    if !attr.default.is_empty() {
        column.push_str(&format!(" DEFAULT {}", attr.default));
    }
    if attr.unique {
        column.push_str(" UNIQUE");
    }
    Ok(column)
}

fn build_create_edge_table_sql(
    def: &SimpleTableDefinition,
    edge: &EdgeTableDefinition,
) -> Result<String, PlatformError> {
    let mut columns = Vec::new();
    for attr in &def.attributes {
        let mut column = column_sql(attr)?;
        if let Some(reference) = edge.references.iter().find(|r| r.column == attr.name) {
            column.push_str(&format!(" REFERENCES {}", reference.table));
        }
        columns.push(column);
    }
    if !edge.key.is_empty() {
        columns.push(format!("PRIMARY KEY ({})", edge.key.join(", ")));
    }
    Ok(format!("CREATE TABLE {} ({})", def.collection, columns.join(", ")))
}

/// The graph that holds every table and edge; it is never created or dropped.
const BASE_GRAPH: &str = "base";

/// One edge table as a property graph declares it.
fn edge_declaration(def: &SimpleTableDefinition, edge: &EdgeTableDefinition) -> String {
    let mut declaration = format!(
        "{} SOURCE KEY ({}) REFERENCES {} (_key) DESTINATION KEY ({}) REFERENCES {} (_key)",
        def.collection, edge.source, edge.source_table, edge.destination, edge.destination_table
    );
    // The catalog names an edge table's type with its schema when it has
    // one (`geo.connects`), so two schemas' types never meet; the label a
    // graph declares is the bare word. A label the same as the table's own
    // name is the default, and only another one is written.
    let bare = def.collection.rsplit('.').next().unwrap_or(&def.collection);
    let label = match def.collection.split_once('.') {
        Some((schema, _)) => edge.label.strip_prefix(&format!("{schema}.")).unwrap_or(&edge.label),
        None => edge.label.as_str(),
    };
    if !label.is_empty() && label != bare {
        declaration.push_str(&format!(" LABEL {label}"));
    }
    declaration
}

/// The statement that declares a property graph over its edge tables, or
/// adds edge tables to a graph that already exists. A graph is not stored in
/// the schema document on its own: it is the `graph` its edge tables name,
/// and its vertex tables are the tables those edges join.
fn build_graph_sql(graph: &str, edges: &[&SimpleTableDefinition], exists: bool) -> String {
    let declarations = edges
        .iter()
        .filter_map(|def| def.edge.as_ref().map(|edge| edge_declaration(def, edge)))
        .collect::<Vec<_>>()
        .join(", ");
    if exists {
        return format!("ALTER PROPERTY GRAPH {graph} ADD EDGE TABLES ({declarations})");
    }
    let vertices = edges
        .iter()
        .filter_map(|def| def.edge.as_ref())
        .flat_map(|edge| [edge.source_table.clone(), edge.destination_table.clone()])
        .filter(|table| !table.is_empty())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(", ");
    format!("CREATE PROPERTY GRAPH {graph} VERTEX TABLES ({vertices}) EDGE TABLES ({declarations})")
}

/// A table name as the catalog gives it, `table` or `schema.table`, with each
/// part slugged. `public.x` is `x`, because a bare name resolves in `public`.
fn qualified_slug(raw: &str) -> String {
    let parts = raw
        .split('.')
        .map(slug_segment)
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>();
    match parts.as_slice() {
        [schema, table] if schema == "public" => table.clone(),
        [schema, table] => format!("{schema}.{table}"),
        [table] => table.clone(),
        _ => String::new(),
    }
}

fn edge_definition(info: &sekejap::EdgeTableInfo) -> EdgeTableDefinition {
    let text = |value: &Option<String>| value.clone().unwrap_or_default();
    EdgeTableDefinition {
        references: info
            .references
            .iter()
            .map(|(column, table)| EdgeTableReference {
                column: column.clone(),
                table: table.clone(),
            })
            .collect(),
        key: info.key.clone(),
        source: text(&info.source),
        source_table: text(&info.source_table),
        destination: text(&info.destination),
        destination_table: text(&info.destination_table),
        label: text(&info.label),
        graph: text(&info.graph),
    }
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
        let table = qualified_slug(&collection);
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
            // An edge table's rows are its graph's edges, not a collection
            // count; a count it cannot answer is not a catalog failure.
            None if described.edge.is_some() => db.count_rows(&collection).unwrap_or(0),
            None => db
                .count_rows(&collection)
                .map_err(|e| store_error("PLATFORM_SEKEJAP_CATALOG", e))?,
        } as usize;

        let mut hash = BTreeSet::new();
        let mut range = BTreeSet::new();
        let mut fulltext = BTreeSet::new();
        let mut vector = BTreeSet::new();
        let mut spatial = BTreeSet::new();
        let mut unique = BTreeSet::new();
        for index in &described.indexes {
            if index.unique {
                unique.insert(index.field.clone());
            }
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
            .filter(|f| !f.name.starts_with('_'))
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
                    unique: unique.contains(&f.name),
                    declared: f.declared.clone().unwrap_or_default(),
                    default: f.default.clone().unwrap_or_default(),
                    // A key column is NOT NULL by being the key; saying so
                    // again would only repeat PRIMARY KEY.
                    not_null: f.not_null && !f.primary_key,
                    primary_key: f.primary_key,
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
                key_default: described
                    .fields
                    .iter()
                    .find(|f| f.name == "_key")
                    .and_then(|f| f.default.clone())
                    .unwrap_or_default(),
                schema: if described.schema == "public" {
                    String::new()
                } else {
                    described.schema.clone()
                },
                edge: described.edge.as_ref().map(edge_definition),
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
    // Columns keep the order the table declares them in: the catalog answers
    // it the same way every time, so the document is stable, and a table
    // recreated from it is declared exactly as the original.
    let mut attributes = def.attributes;
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
        key_default: def.key_default,
        schema: def.schema,
        edge: def.edge,
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
    let table_slug = qualified_slug(&table.table);
    if table_slug.is_empty() {
        return None;
    }
    let collection_slug = qualified_slug(&table.collection);
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
        key_default: table.key_default.trim().to_string(),
        schema: slug_segment(&table.schema),
        edge: table.edge.clone(),
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

    let defs = export
        .tables
        .iter()
        .filter_map(imported_table_definition)
        .collect::<Vec<_>>();
    let apply_error = |err: PlatformError| PlatformError::new("PLATFORM_SEKEJAP_SCHEMA_APPLY", err.message);
    let run = |sql: String| {
        db.execute(&sql, &[])
            .map(|_| ())
            .map_err(|e| apply_error(store_error("PLATFORM_SEKEJAP_SCHEMA_APPLY", e)))
    };

    // Schemas before the tables in them.
    for schema in defs
        .iter()
        .map(|def| def.schema.as_str())
        .filter(|schema| !schema.is_empty())
        .collect::<BTreeSet<_>>()
    {
        run(format!("CREATE SCHEMA IF NOT EXISTS {schema}"))?;
    }

    // Tables of rows before edge tables, which reference them.
    let (edge_defs, row_defs): (Vec<_>, Vec<_>) = defs.iter().partition(|def| def.edge.is_some());
    let existed_before = existing_tables.clone();
    let mut created_edges = Vec::new();
    for def in row_defs.into_iter().chain(edge_defs.iter().copied()) {
        if existing_tables.contains(&def.table) {
            tables_skipped.push(def.table.clone());
            continue;
        }
        create_managed_table(&db, def).map_err(apply_error)?;
        existing_tables.insert(def.table.clone());
        tables_created.push(def.table.clone());
        if def.edge.is_some() {
            created_edges.push(def);
        }
    }

    // Then each graph, declared over the edge tables just created. A graph
    // one of whose edge tables was already here exists, and gains the rest.
    // An edge table no named graph shows is fixed in `base`, which always
    // exists.
    let mut graphs: BTreeMap<&str, Vec<&SimpleTableDefinition>> = BTreeMap::new();
    for def in created_edges {
        if let Some(edge) = def.edge.as_ref() {
            let graph = if edge.graph.is_empty() { BASE_GRAPH } else { edge.graph.as_str() };
            graphs.entry(graph).or_default().push(def);
        }
    }
    for (graph, edges) in graphs {
        let exists = graph == BASE_GRAPH
            || edge_defs.iter().any(|def| {
                existed_before.contains(&def.table)
                    && def.edge.as_ref().is_some_and(|edge| edge.graph == graph)
            });
        run(build_graph_sql(graph, &edges, exists))?;
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

    if let Some(edge) = &def.edge {
        for end in [&edge.source_table, &edge.destination_table] {
            match existing.iter().find(|item| &item.table == end) {
                Some(item) if item.edge.is_none() => {}
                Some(_) => {
                    return Err(PlatformError::new(
                        "PLATFORM_SEKEJAP_TABLE_INVALID",
                        format!("'{end}' is an edge table; an edge joins two tables of rows"),
                    ));
                }
                None => {
                    return Err(PlatformError::new(
                        "PLATFORM_SEKEJAP_TABLE_INVALID",
                        format!("an edge reaches a table that exists, and there is no '{end}'"),
                    ));
                }
            }
        }
    }

    let db = get_db(data_root, owner, project)?;
    create_managed_table(&db, &def)?;
    // An edge table's direction is fixed in `base`, which needs no named
    // graph. If that is refused the table is dropped again, so no table is
    // left that looks like an edge table and is not one.
    if def.edge.is_some() {
        if let Err(err) = db.execute(&build_graph_sql(BASE_GRAPH, &[&def], true), &[]) {
            let _ = db.execute(&format!("DROP TABLE {}", def.collection), &[]);
            return Err(store_error("PLATFORM_SEKEJAP_TABLE_CREATE", err));
        }
    }
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

/// Adds one row from the Studio's grid and answers its key.
///
/// The table is found by the name the tree gave, schema included, and only
/// the columns it declares are written, so a name that reaches the statement
/// is one the catalog already holds. The key is the caller's when it chose
/// one; otherwise a table with a generated key makes it, and any other table
/// gets one minted here. Either way `RETURNING _key` answers what was stored.
pub fn insert_row(
    data_root: &Path,
    owner: &str,
    project: &str,
    table: &str,
    values: &serde_json::Map<String, Value>,
) -> Result<Value, PlatformError> {
    let wanted = qualified_slug(table);
    let def = list_tables(data_root, owner, project)?
        .into_iter()
        .find(|item| item.table == wanted)
        .ok_or_else(|| {
            PlatformError::new("PLATFORM_SEKEJAP_TABLE_MISSING", format!("table '{table}' not found"))
        })?;

    let mut columns = Vec::new();
    let mut literals = Vec::new();
    let chosen_key = values
        .get("_key")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|key| !key.is_empty())
        .map(str::to_string);
    let key = match chosen_key {
        Some(key) => Some(key),
        None if !def.key_default.is_empty() => None,
        None => Some(uuid::Uuid::new_v4().to_string()),
    };
    if let Some(key) = key {
        columns.push("_key".to_string());
        literals.push(sql_literal(&Value::String(key)));
    }
    for (name, value) in values {
        if name == "_key" {
            continue;
        }
        if !def.attributes.iter().any(|attr| &attr.name == name) {
            return Err(PlatformError::new(
                "PLATFORM_SEKEJAP_ROW_INVALID",
                format!("'{}' has no column '{name}'", def.table),
            ));
        }
        columns.push(name.clone());
        literals.push(sql_literal(value));
    }

    let sql = if columns.is_empty() {
        format!("INSERT INTO {} DEFAULT VALUES RETURNING _key", def.collection)
    } else {
        format!(
            "INSERT INTO {} ({}) VALUES ({}) RETURNING _key",
            def.collection,
            columns.join(", "),
            literals.join(", ")
        )
    };
    let payload = execute_sql(data_root, owner, project, &sql, &[], 1, false)?;
    payload
        .rows
        .first()
        .and_then(|row| row.first())
        .cloned()
        .ok_or_else(|| PlatformError::new("PLATFORM_SEKEJAP_ROW_INSERT", "the insert answered no key"))
}

/// One JSON value as a SQL literal: text quoted with its quotes doubled.
fn sql_literal(value: &Value) -> String {
    match value {
        Value::Null => "NULL".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(text) => format!("'{}'", text.replace('\'', "''")),
        other => format!("'{}'", other.to_string().replace('\'', "''")),
    }
}

pub fn delete_table(
    data_root: &Path,
    owner: &str,
    project: &str,
    table: &str,
) -> Result<(), PlatformError> {
    let table_slug = qualified_slug(table);
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
    let table_slug = qualified_slug(table);
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

    let alter = |sql: String| {
        db.execute(&sql, &[])
            .map(|_| ())
            .map_err(|e| store_error("PLATFORM_SEKEJAP_TABLE_ALTER", e))
    };
    for attr in &attrs {
        let Some(old) = existing.attributes.iter().find(|a| a.name == attr.name) else {
            // A new column, with its NOT NULL and DEFAULT. sekejap builds the
            // automatic index over the rows already there, so it is filterable
            // as soon as this returns. UNIQUE goes on as its own constraint:
            // an ADD COLUMN ... UNIQUE is accepted and not applied (sekejap
            // 0.18.2 and 0.18.3), where ADD UNIQUE is.
            let column = column_sql(&CollectionAttribute { unique: false, ..attr.clone() })?;
            alter(format!("ALTER TABLE {} ADD COLUMN {column}", existing.collection))?;
            if attr.unique {
                alter(format!("ALTER TABLE {} ADD UNIQUE ({})", existing.collection, attr.name))?;
            }
            continue;
        };
        // An existing column: its type can change and it can become UNIQUE.
        // Its NOT NULL and DEFAULT are written whole when the column is made,
        // and sekejap has no way to change them after; asked to, this says so
        // rather than pretend.
        if attr.not_null != old.not_null || attr.default != old.default {
            return Err(PlatformError::new(
                "PLATFORM_SEKEJAP_TABLE_ALTER",
                format!(
                    "column '{}': NOT NULL and DEFAULT are set when a column is added and cannot be changed after in sekejap yet",
                    attr.name
                ),
            ));
        }
        if !attr.declared.is_empty() && !old.declared.is_empty() && attr.declared != old.declared {
            alter(format!(
                "ALTER TABLE {} ALTER COLUMN {} TYPE {}",
                existing.collection, attr.name, attr.declared
            ))?;
        }
        if attr.unique && !old.unique {
            alter(format!("ALTER TABLE {} ADD UNIQUE ({})", existing.collection, attr.name))?;
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

/// The schema a table lives in, `public` when it names none.
fn table_schema(def: &SimpleTableDefinition) -> String {
    if def.schema.is_empty() {
        "public".to_string()
    } else {
        def.schema.clone()
    }
}

/// A table under its schema, by its bare name — the shape every engine's
/// tree has, so the Studio reads sekejap and PostgreSQL alike.
fn table_to_node(def: &SimpleTableDefinition) -> DbObjectNode {
    DbObjectNode {
        kind: "table".to_string(),
        name: def.table.rsplit('.').next().unwrap_or(&def.table).to_string(),
        schema: Some(table_schema(def)),
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
            // An edge table's shape — the tables it joins, its key, its graph
            // — so the Studio can tell edge tables from tables of rows.
            "edge": def.edge,
            "key_default": def.key_default,
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

/// Every schema the store has, `public` first, including one with no tables
/// yet — `collections()` only names schemas that hold one, the catalog view
/// names them all.
fn store_schemas(db: &Db) -> Result<Vec<String>, PlatformError> {
    let rows = db
        .query("SELECT nspname FROM pg_namespace", &[])
        .map_err(|e| store_error("PLATFORM_SEKEJAP_CATALOG", e))?;
    let mut names = rows
        .iter()
        .filter_map(|row| row.value("nspname").map(sekejap::value_to_json))
        .filter_map(|value| value.as_str().map(str::to_string))
        .filter(|name| name != "pg_catalog" && name != "information_schema" && name != "public")
        .collect::<Vec<_>>();
    names.sort();
    names.insert(0, "public".to_string());
    Ok(names)
}

fn schema_node(name: &str, children: Vec<DbObjectNode>) -> DbObjectNode {
    DbObjectNode {
        kind: "schema".to_string(),
        name: name.to_string(),
        schema: None,
        children,
        meta: json!({}),
    }
}

/// The schemas, as tree nodes. A store with no tables and no schema of its
/// own answers none, so a fresh project shows "no tables yet".
pub fn describe_schemas(
    data_root: &Path,
    owner: &str,
    project: &str,
) -> Result<Vec<DbObjectNode>, PlatformError> {
    Ok(describe_tree(data_root, owner, project)?
        .into_iter()
        .map(|mut node| {
            node.children.clear();
            node
        })
        .collect())
}

/// Each schema with its tables, edge tables among them.
pub fn describe_tree(
    data_root: &Path,
    owner: &str,
    project: &str,
) -> Result<Vec<DbObjectNode>, PlatformError> {
    let tables = list_tables(data_root, owner, project)?;
    let db = get_db(data_root, owner, project)?;
    let schemas = store_schemas(&db)?;
    if tables.is_empty() && schemas.len() == 1 {
        return Ok(Vec::new());
    }
    Ok(schemas
        .iter()
        .map(|schema| {
            let children = tables
                .iter()
                .filter(|def| &table_schema(def) == schema)
                .map(table_to_node)
                .collect();
            schema_node(schema, children)
        })
        .collect())
}

pub fn describe_columns(
    data_root: &Path,
    owner: &str,
    project: &str,
    table: &str,
) -> Result<Vec<DbObjectNode>, PlatformError> {
    // Found by the name the tree gave, schema included: `geo.places` and a
    // `places` in `public` are two tables.
    let wanted = qualified_slug(table);
    let defs = list_tables(data_root, owner, project)?;
    let Some(def) = defs.into_iter().find(|item| item.table == wanted) else {
        return Ok(Vec::new());
    };
    let schema = table_schema(&def);
    // Each column in the shape every driver reports — `type`, `full_type`,
    // `nullable`, `pk`, `default` — plus `unique`, so one structure panel
    // renders every engine.
    let column = |name: &str, declared: &str, nullable: bool, pk: bool, default: &str, unique: bool, indexes: &[String]| {
        let mut meta = json!({
            "data_type": declared,
            "type": declared,
            "full_type": declared,
            "nullable": nullable,
            "index_types": indexes,
        });
        if pk {
            meta["pk"] = json!(true);
        }
        if unique {
            meta["unique"] = json!(true);
        }
        if !default.is_empty() {
            meta["default"] = json!(default);
        }
        DbObjectNode {
            kind: "column".to_string(),
            name: name.to_string(),
            schema: Some(schema.clone()),
            children: Vec::new(),
            meta,
        }
    };
    // `_key` is every row's address. A table whose key is a named column
    // reports that column as the key instead, since `_key` mirrors it.
    let mut nodes = Vec::new();
    if def.edge.is_none() && !def.attributes.iter().any(|attr| attr.primary_key) {
        nodes.push(column("_key", "TEXT", false, true, &def.key_default, false, &[]));
    }
    for attr in &def.attributes {
        let declared = if attr.declared.is_empty() {
            map_field_type(&attr.kind).unwrap_or_else(|_| attr.kind.clone())
        } else {
            attr.declared.clone()
        };
        nodes.push(column(
            &attr.name,
            &declared,
            !attr.not_null && !attr.primary_key,
            attr.primary_key,
            &attr.default,
            attr.unique,
            &attr.index_types,
        ));
    }
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
            // sekejap 0.18: named schemas, and property graphs whose edge
            // tables are views over the graph's own edges.
            | ("CREATE", "SCHEMA")
            | ("DROP", "SCHEMA")
            | ("CREATE", "PROPERTY")
            | ("ALTER", "PROPERTY")
            | ("DROP", "PROPERTY")
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

/// A write that names a `RETURNING` clause, outside any quoted string.
///
/// Such a statement answers the rows it wrote — among them a key its
/// `DEFAULT ulid()` minted, which the caller has no other way to learn — so
/// it is run as a query and its rows come back.
fn statement_has_returning(sql: &str) -> bool {
    let mut outside = String::with_capacity(sql.len());
    let mut quoted = false;
    for c in sql.chars() {
        match c {
            '\'' => {
                quoted = !quoted;
                outside.push(' ');
            }
            _ if quoted => {}
            _ => outside.push(c),
        }
    }
    outside
        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
        .any(|word| word.eq_ignore_ascii_case("RETURNING"))
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
        let db = get_db(data_root, owner, project)?;
        if statement_has_returning(trimmed) {
            let rows = db
                .query(trimmed, params)
                .map_err(|e| store_error("PLATFORM_SEKEJAP_QUERY_FAILED", e))?;
            let written = rows.len();
            let (columns, rows, truncated) = payload_from_rows(rows, max_rows);
            record_project_write(data_root, owner, project, written.max(1));
            return Ok(QueryPayload {
                columns,
                row_count: rows.len(),
                rows,
                truncated,
                affected_rows: Some(written as u64),
                duration_ms: started.elapsed().as_millis() as u64,
            });
        }
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
                "EXPLAIN ANALYZE is not built in sekejap (QL_CONTRACT: options refused); \
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
    // The request's `$1, $2, …` values, bound by the engine: the Studio's
    // query tab and the DB API send them, and a value never has to be pasted
    // into the statement's text.
    let result = execute_sql(
        data_root,
        owner,
        project,
        &req.sql,
        &req.params,
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

    if let Some(edge) = &declared.edge {
        return Err(PlatformError::new(
            "PLATFORM_SEKEJAP_INSERT_EDGE_TABLE_TARGET",
            format!(
                "'{collection}' is an edge table: its rows are the edges between two tables. Send them as `edges` \
                 with `type: \"{}\"`, and set the target to a table of rows.",
                edge.label.clone().unwrap_or_else(|| collection.to_string())
            ),
        ));
    }
    // sekejap 0.18: an edge whose type belongs to an edge table carries typed
    // properties and a key, and is written with INSERT INTO that table;
    // `link_with` refuses it. Looked up once, and only when there are edges.
    let edge_tables = if edges.is_empty() {
        HashMap::new()
    } else {
        edge_tables_by_label(db)?
    };

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
        if let Some((table, info)) = edge_tables.get(edge.edge_type.trim()) {
            insert_edge_table_row(&mut tx, table, info, edge)?;
            continue;
        }
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

/// Every edge table, by the label its edges carry.
fn edge_tables_by_label(
    db: &Db,
) -> Result<HashMap<String, (String, sekejap::EdgeTableInfo)>, PlatformError> {
    let mut tables = HashMap::new();
    for collection in db
        .collections()
        .map_err(|e| store_error("PLATFORM_SEKEJAP_CATALOG", e))?
    {
        let Some(described) = db
            .describe(&collection)
            .map_err(|e| store_error("PLATFORM_SEKEJAP_CATALOG", e))?
        else {
            continue;
        };
        if let Some(info) = described.edge {
            let label = info.label.clone().unwrap_or_else(|| collection.clone());
            tables.entry(label).or_insert((collection, info));
        }
    }
    Ok(tables)
}

/// One edge written into its edge table, inside the batch's transaction.
///
/// The endpoints must be the tables the edge table joins, in its direction.
/// Property names become column names, so each must already be a plain
/// identifier; their values travel as parameters, never inside the SQL. A
/// plain INSERT, as the records in this batch are: an edge whose key is
/// taken is refused (23505), and one to a row that does not exist is 23503.
fn insert_edge_table_row(
    tx: &mut sekejap::Tx<'_>,
    table: &str,
    info: &sekejap::EdgeTableInfo,
    edge: &StructuredInsertEdge,
) -> Result<(), PlatformError> {
    let refuse = |message: String| PlatformError::new("PLATFORM_SEKEJAP_INSERT_EDGE", message);
    let (Some(source), Some(destination)) = (&info.source, &info.destination) else {
        return Err(refuse(format!(
            "edge table '{table}' is not declared by a property graph yet, so it has no source or destination"
        )));
    };
    let expected_from = info.source_table.clone().unwrap_or_default();
    let expected_to = info.destination_table.clone().unwrap_or_default();
    let (from, to) = (qualified_slug(&edge.from_target), qualified_slug(&edge.to_target));
    if from != qualified_slug(&expected_from) || to != qualified_slug(&expected_to) {
        return Err(refuse(format!(
            "edge type '{}' goes from '{expected_from}' to '{expected_to}' (edge table '{table}'); \
             this edge goes from '{}' to '{}'",
            edge.edge_type, edge.from_target, edge.to_target
        )));
    }
    let mut columns = vec![source.clone(), destination.clone()];
    let mut params = vec![
        Value::String(edge.from_key.trim().to_string()),
        Value::String(edge.to_key.trim().to_string()),
    ];
    let mut names = edge.fields.keys().collect::<Vec<_>>();
    names.sort();
    for name in names {
        if slug_segment(name) != *name || name.contains('-') {
            return Err(refuse(format!(
                "edge property '{name}' is not a column name of '{table}'; use lowercase letters, digits and _"
            )));
        }
        columns.push(name.clone());
        params.push(edge.fields[name].clone());
    }
    let placeholders = (1..=columns.len())
        .map(|n| format!("${n}"))
        .collect::<Vec<_>>()
        .join(", ");
    let sql = format!(
        "INSERT INTO {table} ({}) VALUES ({placeholders})",
        columns.join(", ")
    );
    tx.execute(&sql, &params)
        .map_err(|e| refuse(format!("edge '{}' into '{table}': {e}", edge.edge_type)))?;
    Ok(())
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
            ..Default::default()
        }
    }

    fn create(root: &Path, owner: &str, project: &str, table: &str, attrs: Vec<CollectionAttribute>) -> SimpleTableDefinition {
        create_table(
            root,
            owner,
            project,
            &CreateSimpleTableRequest {
                key_default: String::new(),
                edge: None,
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
                key_default: String::new(),
                edge: None,
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

    /// `UNIQUE` is read from the live catalog into the table's attributes,
    /// and a table created from that definition declares it again, so the
    /// schema document carries it both ways.
    #[test]
    fn a_unique_column_is_read_from_the_catalog_and_declared_again() {
        let tmp = tmp_root();
        let db = get_db(tmp.path(), "alice", "demo").expect("open db");
        db.execute(
            "CREATE TABLE members (_key TEXT PRIMARY KEY, email TEXT UNIQUE, name TEXT)",
            &[],
        )
        .expect("create table");
        let tables = live_tables(&db).expect("live tables");
        let members = tables
            .iter()
            .find(|t| t.table == "members")
            .expect("members");
        let flag = |name: &str| {
            members
                .attributes
                .iter()
                .find(|a| a.name == name)
                .expect("attribute")
                .unique
        };
        assert!(flag("email"));
        assert!(!flag("name"));
        let sql = build_create_table_sql(members).expect("create sql");
        assert!(sql.contains("email TEXT UNIQUE"), "{sql}");
        assert!(!sql.contains("name TEXT UNIQUE"), "{sql}");
    }

    /// Named schemas and edge tables survive the schema document: a project
    /// built from it has the same tables, the same edge tables declared by
    /// the same graph, and a GQL walk over them answers.
    #[test]
    fn schemas_and_edge_tables_round_trip_through_the_schema_document() {
        let tmp = tmp_root();
        {
            let db = get_db(tmp.path(), "alice", "source").expect("open source");
            for ddl in [
                "CREATE SCHEMA geo",
                "CREATE TABLE geo.city (_key TEXT PRIMARY KEY, name TEXT)",
                "CREATE TABLE geo.road (from_id TEXT REFERENCES geo.city, to_id TEXT REFERENCES geo.city, km INT, PRIMARY KEY (from_id, to_id))",
                "CREATE TABLE notes (_key TEXT PRIMARY KEY, body TEXT)",
                "CREATE PROPERTY GRAPH roads VERTEX TABLES (geo.city) EDGE TABLES (geo.road SOURCE KEY (from_id) REFERENCES geo.city (_key) DESTINATION KEY (to_id) REFERENCES geo.city (_key) LABEL connects)",
                "CREATE TABLE twins (a TEXT REFERENCES geo.city, b TEXT REFERENCES geo.city, PRIMARY KEY (a, b))",
                "ALTER PROPERTY GRAPH base ADD EDGE TABLES (twins SOURCE KEY (a) REFERENCES geo.city (_key) DESTINATION KEY (b) REFERENCES geo.city (_key))",
            ] {
                db.execute(ddl, &[]).expect(ddl);
            }
        }
        sync_schema_to_repo(tmp.path(), "alice", "source").expect("sync");
        let document = std::fs::read(
            repo_schema_dir(tmp.path(), "alice", "source").join(SCHEMA_DOCUMENT_FILE),
        )
        .expect("schema document");

        let target_schema = repo_schema_dir(tmp.path(), "alice", "target");
        std::fs::create_dir_all(&target_schema).expect("target repo");
        std::fs::write(target_schema.join(SCHEMA_DOCUMENT_FILE), &document).expect("copy");
        let report = apply_schema_from_repo(tmp.path(), "alice", "target")
            .expect("apply")
            .expect("a document to apply");
        assert_eq!(report.tables_created.len(), 4, "{report:?}");

        let db = get_db(tmp.path(), "alice", "target").expect("open target");
        let tables = live_tables(&db).expect("live tables");
        let road = tables.iter().find(|t| t.table == "geo.road").expect("geo.road");
        assert_eq!(road.schema, "geo");
        let edge = road.edge.as_ref().expect("an edge table");
        assert_eq!(edge.graph, "roads");
        // The catalog's name for the type: the schema keeps two schemas'
        // `connects` apart.
        assert_eq!(edge.label, "geo.connects");
        let twins = tables.iter().find(|t| t.table == "twins").expect("twins");
        let twins_edge = twins.edge.as_ref().expect("an edge table in base only");
        assert!(twins_edge.graph.is_empty(), "no named graph: {twins_edge:?}");
        assert_eq!(twins_edge.source_table, "geo.city");
        assert_eq!(edge.source_table, "geo.city");
        assert_eq!(edge.key, vec!["from_id".to_string(), "to_id".to_string()]);
        assert!(tables.iter().any(|t| t.table == "geo.city" && t.edge.is_none()));
        assert!(tables.iter().any(|t| t.table == "notes" && t.schema.is_empty()));

        for dml in [
            "INSERT INTO geo.city (_key, name) VALUES ('a', 'Alpha'), ('b', 'Beta')",
            "INSERT INTO geo.road (from_id, to_id, km) VALUES ('a', 'b', 12)",
        ] {
            db.execute(dml, &[]).expect(dml);
        }
        let rows = db
            .query(
                "SELECT name, km FROM GRAPH_TABLE (roads MATCH (x WHERE x._key = 'a')-[e:connects]->(y) RETURN y.name AS name, e.km AS km)",
                &[],
            )
            .expect("walk");
        assert_eq!(rows.len(), 1);
        assert_eq!(
            rows.iter().next().and_then(|row| row.value("name")).map(sekejap::value_to_json),
            Some(json!("Beta"))
        );
    }

    /// The edge-table path refuses by name: a taken edge key, an edge
    /// between the wrong tables, a property that is not a column name, and
    /// an edge table as the batch's target. A refused edge takes the whole
    /// batch with it, records included.
    #[test]
    fn edge_table_edges_refuse_by_name_and_take_the_batch_with_them() {
        let tmp = tmp_root();
        {
            let db = get_db(tmp.path(), "alice", "demo").expect("open db");
            for ddl in [
                "CREATE TABLE people (_key TEXT PRIMARY KEY, name TEXT)",
                "CREATE TABLE places (_key TEXT PRIMARY KEY, name TEXT)",
                "CREATE TABLE knows (src TEXT REFERENCES people, dst TEXT REFERENCES people, since INT, PRIMARY KEY (src, dst))",
                "CREATE PROPERTY GRAPH social VERTEX TABLES (people) EDGE TABLES (knows SOURCE KEY (src) REFERENCES people (_key) DESTINATION KEY (dst) REFERENCES people (_key))",
                "INSERT INTO people (_key, name) VALUES ('ann', 'Ann'), ('bo', 'Bo')",
                "INSERT INTO places (_key, name) VALUES ('here', 'Here')",
            ] {
                db.execute(ddl, &[]).expect(ddl);
            }
        }
        let record = |key: &str| StructuredInsertRecord {
            key: key.to_string(),
            fields: json!({ "name": key }).as_object().cloned().unwrap_or_default(),
        };
        let edge = |from: (&str, &str), to: (&str, &str), fields: Value| StructuredInsertEdge {
            from_target: from.0.to_string(),
            from_key: from.1.to_string(),
            edge_type: "knows".to_string(),
            to_target: to.0.to_string(),
            to_key: to.1.to_string(),
            fields: fields.as_object().cloned().unwrap_or_default(),
            strength: 1.0,
        };
        let insert = |target: &str, rows, edges| {
            bulk_insert(tmp.path(), "alice", "demo", target, rows, edges, StructuredWriteMode::Insert)
        };

        insert("people", vec![], vec![edge(("people", "ann"), ("people", "bo"), json!({ "since": 2020 }))])
            .expect("first edge");
        let taken = insert("people", vec![], vec![edge(("people", "ann"), ("people", "bo"), json!({}))])
            .expect_err("a taken key");
        assert!(taken.message.contains("23505"), "{}", taken.message);

        let wrong = insert("people", vec![], vec![edge(("people", "ann"), ("places", "here"), json!({}))])
            .expect_err("wrong tables");
        assert!(wrong.message.contains("goes from 'people' to 'people'"), "{}", wrong.message);

        let column = insert("people", vec![], vec![edge(("people", "bo"), ("people", "ann"), json!({ "since; DROP": 1 }))])
            .expect_err("not a column name");
        assert!(column.message.contains("not a column name"), "{}", column.message);

        let target = insert("knows", vec![record("x")], vec![]).expect_err("edge table target");
        assert_eq!(target.code, "PLATFORM_SEKEJAP_INSERT_EDGE_TABLE_TARGET");

        // One commit: the record goes back with the refused edge.
        insert("people", vec![record("cy")], vec![edge(("people", "cy"), ("places", "here"), json!({}))])
            .expect_err("refused edge");
        let db = get_db(tmp.path(), "alice", "demo").expect("open db");
        let rows = db.query("SELECT _key FROM people WHERE _key = 'cy'", &[]).expect("read");
        assert_eq!(rows.len(), 0, "the record was rolled back with the edge");
    }

    /// A table recreated from the schema document is declared exactly as the
    /// original: types, NOT NULL, DEFAULTs (the key's own included), UNIQUE,
    /// a named key column, and an edge table's columns. `SHOW CREATE TABLE`
    /// on both sides is the same text.
    #[test]
    fn a_recreated_table_is_declared_exactly_as_the_original() {
        let tmp = tmp_root();
        let ddl = [
            "CREATE TABLE members (_key TEXT PRIMARY KEY DEFAULT ulid(), name TEXT NOT NULL, email TEXT UNIQUE, status TEXT DEFAULT 'pending', score INT DEFAULT 0, admin BOOLEAN DEFAULT false, joined TIMESTAMPTZ DEFAULT now(), loc GEOMETRY(Point,4326))",
            "CREATE TABLE things (id TEXT PRIMARY KEY DEFAULT uuid4(), label TEXT NOT NULL)",
            "CREATE TABLE follows (src TEXT REFERENCES members, dst TEXT REFERENCES members, since INT DEFAULT 2000, PRIMARY KEY (src, dst))",
            "CREATE PROPERTY GRAPH social VERTEX TABLES (members) EDGE TABLES (follows SOURCE KEY (src) REFERENCES members (_key) DESTINATION KEY (dst) REFERENCES members (_key))",
        ];
        {
            let db = get_db(tmp.path(), "alice", "source").expect("open source");
            for sql in ddl {
                db.execute(sql, &[]).expect(sql);
            }
        }
        sync_schema_to_repo(tmp.path(), "alice", "source").expect("sync");
        let document = std::fs::read(
            repo_schema_dir(tmp.path(), "alice", "source").join(SCHEMA_DOCUMENT_FILE),
        )
        .expect("schema document");
        let target_schema = repo_schema_dir(tmp.path(), "alice", "target");
        std::fs::create_dir_all(&target_schema).expect("target repo");
        std::fs::write(target_schema.join(SCHEMA_DOCUMENT_FILE), &document).expect("copy");
        apply_schema_from_repo(tmp.path(), "alice", "target")
            .expect("apply")
            .expect("a document");

        let declared = |project: &str, table: &str| {
            let db = get_db(tmp.path(), "alice", project).expect("open");
            let rows = db
                .query(&format!("SHOW CREATE TABLE {table}"), &[])
                .expect("show create table");
            rows.iter()
                .next()
                .and_then(|row| row.value("create_table"))
                .map(sekejap::value_to_json)
                .expect("create_table")
        };
        // The CREATE TABLE statement is the same text; the index statements
        // are the same set, listed in the order each store built them.
        let split = |value: Value| {
            let text = value.as_str().unwrap_or_default().to_string();
            let (table, indexes) = text.split_once(");").unwrap_or((&text, ""));
            let mut indexes = indexes
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>();
            indexes.sort();
            (table.to_string(), indexes)
        };
        for table in ["members", "things", "follows"] {
            assert_eq!(
                split(declared("source", table)),
                split(declared("target", table)),
                "{table}"
            );
        }

        // And the recreated table still mints its keys and fills its defaults.
        let made = execute_sql(
            tmp.path(),
            "alice",
            "target",
            "INSERT INTO members (name) VALUES ('Ann') RETURNING _key, status, score",
            &[],
            10,
            false,
        )
        .expect("insert into the recreated table");
        assert_eq!(made.rows[0][0].as_str().map(str::len), Some(26));
        assert_eq!(made.rows[0][1], json!("pending"));
        assert_eq!(made.rows[0][2], json!(0));
    }

    /// The statements help topic `db/sekejap` teaches run as written through
    /// `execute_sql`, the path every pipeline and the Studio take.
    #[test]
    fn the_help_page_statements_run_through_execute_sql() {
        let tmp = tmp_root();
        let run = |sql: &str, params: &[Value]| {
            execute_sql(tmp.path(), "alice", "demo", sql, params, 100, false)
                .unwrap_or_else(|err| panic!("{sql}: {}", err.message))
        };
        for sql in [
            "CREATE TABLE members (_key TEXT PRIMARY KEY DEFAULT ulid(), name TEXT NOT NULL, email TEXT UNIQUE, status TEXT DEFAULT 'active')",
            "CREATE TABLE institutions (_key TEXT PRIMARY KEY, name TEXT)",
            "CREATE TABLE affiliated_with (member_id TEXT REFERENCES members, institution_id TEXT REFERENCES institutions, position TEXT, PRIMARY KEY (member_id, institution_id))",
            "CREATE PROPERTY GRAPH network VERTEX TABLES (members, institutions) EDGE TABLES (affiliated_with SOURCE KEY (member_id) REFERENCES members (_key) DESTINATION KEY (institution_id) REFERENCES institutions (_key))",
            "CREATE SCHEMA geo",
            "CREATE TABLE geo.places (_key TEXT PRIMARY KEY, name TEXT)",
            "INSERT INTO institutions (_key, name) VALUES ('u1', 'University One'), ('u2', 'University Two')",
        ] {
            run(sql, &[]);
        }
        let ann = run("INSERT INTO members (name, email) VALUES ($1, $2) RETURNING _key", &[json!("Ann Doe"), json!("ann@example.com")]);
        let ann = ann.rows[0][0].as_str().expect("key").to_string();
        let bo = run("INSERT INTO members (name, email) VALUES ($1, $2) RETURNING _key", &[json!("Bo Roe"), json!("bo@example.com")]);
        let bo = bo.rows[0][0].as_str().expect("key").to_string();

        run("INSERT INTO members (_key, name) VALUES ($1, $2) ON CONFLICT (_key) DO UPDATE SET name = EXCLUDED.name", &[json!(ann), json!("Ann Doe")]);
        run("INSERT INTO members (_key, name) VALUES ($1, $2) ON CONFLICT (_key) DO NOTHING", &[json!(ann), json!("ignored")]);
        run("INSERT INTO affiliated_with (member_id, institution_id, position) VALUES ($1, $2, $3)", &[json!(ann), json!("u1"), json!("Lecturer")]);
        run("INSERT INTO affiliated_with (member_id, institution_id, position) VALUES ($1, $2, $3)", &[json!(bo), json!("u1"), json!("Researcher")]);
        run("UPDATE affiliated_with SET position = $3 WHERE member_id = $1 AND institution_id = $2", &[json!(ann), json!("u1"), json!("Professor")]);

        let found = run("SELECT _key, name FROM members WHERE name ILIKE $1", &[json!("%doe%")]);
        assert_eq!(found.row_count, 1);
        let newest = run("SELECT _key, name FROM members ORDER BY _key DESC LIMIT 1", &[]);
        assert_eq!(newest.rows[0][1], json!("Bo Roe"));
        let next = run("SELECT _key, name FROM members WHERE _key < $1 ORDER BY _key DESC LIMIT 1", &[json!(bo)]);
        assert_eq!(next.rows[0][1], json!("Ann Doe"));
        let edges = run("SELECT institution_id, position FROM affiliated_with WHERE member_id = $1", &[json!(ann)]);
        assert_eq!(edges.rows, vec![vec![json!("u1"), json!("Professor")]]);
        let walked = run(
            "SELECT name, position FROM GRAPH_TABLE (network MATCH (m WHERE m._key = $1)-[a:affiliated_with]->(i) RETURN i.name AS name, a.position AS position)",
            &[json!(ann)],
        );
        assert_eq!(walked.rows, vec![vec![json!("University One"), json!("Professor")]]);
        let colleagues = run(
            "SELECT g.name, COUNT(*) AS shared FROM GRAPH_TABLE (network MATCH (m WHERE m._key = $1)-[:affiliated_with]->(i)<-[:affiliated_with]-(o) WHERE o._key <> $1 RETURN DISTINCT o.name AS name) AS g GROUP BY g.name",
            &[json!(ann)],
        );
        assert_eq!(colleagues.rows[0][0], json!("Bo Roe"));
        run("DELETE FROM affiliated_with WHERE member_id = $1 AND institution_id = $2", &[json!(bo), json!("u1")]);
        let shown = run("SHOW CREATE TABLE members", &[]);
        assert!(shown.rows[0][0].as_str().unwrap_or_default().contains("DEFAULT ulid()"));
        run("INSERT INTO geo.places (_key, name) VALUES ('p1', 'Here')", &[]);
    }

    /// The tree answers the store's own schemas, `public` first and an empty
    /// one included, each table under its schema by its bare name — the shape
    /// the Studio reads from every engine.
    #[test]
    fn the_tree_lists_real_schemas_with_tables_under_them() {
        let tmp = tmp_root();
        assert!(describe_tree(tmp.path(), "alice", "demo").expect("empty tree").is_empty());
        {
            let db = get_db(tmp.path(), "alice", "demo").expect("open db");
            for ddl in [
                "CREATE SCHEMA geo",
                "CREATE SCHEMA archive",
                "CREATE TABLE geo.places (_key TEXT PRIMARY KEY, name TEXT)",
                "CREATE TABLE people (_key TEXT PRIMARY KEY, name TEXT)",
            ] {
                db.execute(ddl, &[]).expect(ddl);
            }
        }
        let tree = describe_tree(tmp.path(), "alice", "demo").expect("tree");
        let names = tree.iter().map(|node| node.name.as_str()).collect::<Vec<_>>();
        assert_eq!(names, vec!["public", "archive", "geo"]);
        let tables = |schema: &str| {
            tree.iter()
                .find(|node| node.name == schema)
                .map(|node| node.children.iter().map(|t| (t.name.clone(), t.schema.clone())).collect::<Vec<_>>())
                .unwrap_or_default()
        };
        assert_eq!(tables("public"), vec![("people".to_string(), Some("public".to_string()))]);
        assert_eq!(tables("geo"), vec![("places".to_string(), Some("geo".to_string()))]);
        assert!(tables("archive").is_empty());
    }

    /// A table made from the Studio carries what its columns declare: SQL
    /// types, NOT NULL, a DEFAULT (a typed word quoted, never pasted), UNIQUE,
    /// a generated key and a schema. A column added later keeps its UNIQUE;
    /// changing an existing column's NOT NULL is refused by name.
    #[test]
    fn a_studio_table_carries_its_column_declarations() {
        let tmp = tmp_root();
        let column = |name: &str, kind: &str| CollectionAttribute {
            name: name.to_string(),
            kind: kind.to_string(),
            ..Default::default()
        };
        let created = create_table(
            tmp.path(),
            "alice",
            "demo",
            &CreateSimpleTableRequest {
                table: "people".to_string(),
                attributes: vec![
                    CollectionAttribute { not_null: true, ..column("name", "TEXT") },
                    CollectionAttribute {
                        declared: "TEXT, pwned TEXT".to_string(),
                        ..column("bio", "TEXT")
                    },
                    CollectionAttribute { unique: true, ..column("email", "TEXT") },
                    CollectionAttribute { default: "pending".to_string(), ..column("status", "TEXT") },
                    CollectionAttribute { default: "x'); DROP TABLE people; --".to_string(), ..column("note", "TEXT") },
                    column("joined", "TIMESTAMPTZ"),
                    column("loc", "GEOMETRY(Point,4326)"),
                ],
                hash_indexed_fields: Vec::new(),
                range_indexed_fields: Vec::new(),
                key_default: "ulid()".to_string(),
                edge: None,
            },
        )
        .expect("create");
        assert_eq!(created.key_default, "ulid()");
        let shown = |table: &str| {
            let db = get_db(tmp.path(), "alice", "demo").expect("open");
            let rows = db.query(&format!("SHOW CREATE TABLE {table}"), &[]).expect("show");
            rows.iter()
                .next()
                .and_then(|row| row.value("create_table"))
                .map(sekejap::value_to_json)
                .and_then(|value| value.as_str().map(str::to_string))
                .unwrap_or_default()
        };
        let ddl = shown("people");
        for part in [
            "_key TEXT PRIMARY KEY DEFAULT ulid()",
            "name TEXT NOT NULL",
            "status TEXT DEFAULT 'pending'",
            "note TEXT DEFAULT 'x''); DROP TABLE people; --'",
            "joined TIMESTAMPTZ",
            "bio TEXT",
            "loc GEOMETRY(Point,4326)",
            "CREATE UNIQUE INDEX",
        ] {
            assert!(ddl.contains(part), "{part} in {ddl}");
        }

        assert!(!ddl.contains("pwned"), "a declared type is never pasted: {ddl}");
        let mut attributes = created.attributes.clone();
        attributes.push(CollectionAttribute { unique: true, ..column("handle", "TEXT") });
        update_table(
            tmp.path(),
            "alice",
            "demo",
            "people",
            &UpdateSimpleTableRequest { attributes: attributes.clone(), ..Default::default() },
        )
        .expect("add a unique column");
        assert!(shown("people").contains("people_handle"), "handle is indexed");
        let tables = list_tables(tmp.path(), "alice", "demo").expect("tables");
        let people = tables.iter().find(|t| t.table == "people").expect("people");
        assert!(people.attributes.iter().any(|a| a.name == "handle" && a.unique), "handle is unique");

        let mut changed = people.attributes.clone();
        if let Some(name) = changed.iter_mut().find(|a| a.name == "name") {
            name.not_null = false;
        }
        let refused = update_table(
            tmp.path(),
            "alice",
            "demo",
            "people",
            &UpdateSimpleTableRequest { attributes: changed, ..Default::default() },
        )
        .expect_err("NOT NULL cannot change after");
        assert!(refused.message.contains("cannot be changed"), "{}", refused.message);

        get_db(tmp.path(), "alice", "demo")
            .expect("open")
            .execute("CREATE SCHEMA geo", &[])
            .expect("schema");
        create_table(
            tmp.path(),
            "alice",
            "demo",
            &CreateSimpleTableRequest {
                table: "geo.places".to_string(),
                attributes: vec![column("name", "TEXT")],
                hash_indexed_fields: Vec::new(),
                range_indexed_fields: Vec::new(),
                key_default: String::new(),
                edge: None,
            },
        )
        .expect("a table in a schema");
        let places = list_tables(tmp.path(), "alice", "demo")
            .expect("tables")
            .into_iter()
            .find(|t| t.table == "geo.places" && t.schema == "geo")
            .expect("geo.places");
        let mut attributes = places.attributes.clone();
        attributes.push(column("loc", "GEOMETRY(Point,4326)"));
        update_table(
            tmp.path(),
            "alice",
            "demo",
            "geo.places",
            &UpdateSimpleTableRequest { attributes, ..Default::default() },
        )
        .expect("altered by its full name");
        delete_table(tmp.path(), "alice", "demo", "geo.places").expect("dropped by its full name");
    }

    /// A row added from the grid lands in the table the tree named, takes
    /// the caller's key or the table's generated one, and cannot name a
    /// column the table does not have.
    #[test]
    fn a_grid_row_takes_its_key_from_the_caller_or_the_table() {
        let tmp = tmp_root();
        let db = get_db(tmp.path(), "alice", "demo").expect("open");
        for sql in [
            "CREATE TABLE notes (_key TEXT PRIMARY KEY, body TEXT)",
            "CREATE TABLE posts (_key TEXT PRIMARY KEY DEFAULT ulid(), title TEXT)",
            "CREATE SCHEMA geo",
            "CREATE TABLE geo.places (_key TEXT PRIMARY KEY, name TEXT)",
        ] {
            db.execute(sql, &[]).expect(sql);
        }
        let row = |pairs: &[(&str, &str)]| {
            pairs.iter().map(|(k, v)| (k.to_string(), json!(v))).collect::<serde_json::Map<_, _>>()
        };

        let chosen = insert_row(tmp.path(), "alice", "demo", "notes", &row(&[("_key", "n1"), ("body", "it's")]))
            .expect("chosen key");
        assert_eq!(chosen, json!("n1"));
        let minted = insert_row(tmp.path(), "alice", "demo", "notes", &row(&[("body", "b")])).expect("minted");
        assert_eq!(minted.as_str().map(str::len), Some(36), "a uuid: {minted}");
        let generated = insert_row(tmp.path(), "alice", "demo", "posts", &row(&[("title", "t")])).expect("generated");
        assert_eq!(generated.as_str().map(str::len), Some(26), "a ulid: {generated}");
        insert_row(tmp.path(), "alice", "demo", "geo.places", &row(&[("_key", "p1"), ("name", "Somewhere")]))
            .expect("a table in a schema");
        let places = execute_sql(tmp.path(), "alice", "demo", "SELECT _key FROM geo.places", &[], 10, true)
            .expect("read");
        assert_eq!(places.rows, vec![vec![json!("p1")]]);

        let refused = insert_row(
            tmp.path(),
            "alice",
            "demo",
            "notes",
            &row(&[("body) VALUES ('x'); DROP TABLE notes; --", "x")]),
        )
        .expect_err("an unknown column is refused");
        assert_eq!(refused.code, "PLATFORM_SEKEJAP_ROW_INVALID");
    }

    /// An edge table made from the Studio: its two end columns come first,
    /// its direction is fixed in `base` with no named graph, one edge per
    /// pair when asked, and a label of its own or its name. What cannot be an
    /// edge table is refused before anything is created.
    #[test]
    fn a_studio_edge_table_is_fixed_in_base() {
        let tmp = tmp_root();
        let db = get_db(tmp.path(), "alice", "demo").expect("open");
        for sql in [
            "CREATE TABLE people (_key TEXT PRIMARY KEY, name TEXT)",
            "CREATE SCHEMA atlas",
            "CREATE TABLE atlas.places (_key TEXT PRIMARY KEY, name TEXT)",
            "INSERT INTO people (_key, name) VALUES ('a', 'Ann'), ('b', 'Ben')",
            "INSERT INTO atlas.places (_key, name) VALUES ('p', 'Park'), ('q', 'Quay')",
        ] {
            db.execute(sql, &[]).expect(sql);
        }
        let edge = |source: &str, source_table: &str, destination: &str, destination_table: &str, label: &str| {
            Some(crate::platform::model::CreateEdgeTableRequest {
                source: source.to_string(),
                source_table: source_table.to_string(),
                destination: destination.to_string(),
                destination_table: destination_table.to_string(),
                label: label.to_string(),
                one_per_pair: true,
            })
        };
        let request = |table: &str, attributes: Vec<CollectionAttribute>, edge| CreateSimpleTableRequest {
            table: table.to_string(),
            attributes,
            hash_indexed_fields: Vec::new(),
            range_indexed_fields: Vec::new(),
            key_default: String::new(),
            edge,
        };
        let since = CollectionAttribute { name: "since".to_string(), kind: "INT".to_string(), ..Default::default() };

        let knows = create_table(
            tmp.path(),
            "alice",
            "demo",
            &request("knows", vec![since.clone()], edge("person", "people", "friend", "people", "")),
        )
        .expect("an edge table in public");
        let info = knows.edge.as_ref().expect("an edge table");
        assert_eq!((info.source.as_str(), info.destination.as_str()), ("person", "friend"));
        assert_eq!(info.label, "knows");
        assert!(info.graph.is_empty(), "no named graph: {info:?}");
        assert_eq!(info.key, vec!["person".to_string(), "friend".to_string()]);
        assert_eq!(
            knows.attributes.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(),
            vec!["person", "friend", "since"]
        );

        let near = create_table(
            tmp.path(),
            "alice",
            "demo",
            &request("atlas.near", Vec::new(), edge("here", "atlas.places", "there", "atlas.places", "close_to")),
        )
        .expect("an edge table in a schema");
        assert_eq!(near.edge.as_ref().map(|e| e.label.as_str()), Some("atlas.close_to"));

        for sql in [
            "INSERT INTO knows (person, friend, since) VALUES ('a', 'b', 2020)",
            "INSERT INTO atlas.near (here, there) VALUES ('p', 'q')",
        ] {
            db.execute(sql, &[]).expect(sql);
        }
        assert!(db.execute("INSERT INTO knows (person, friend) VALUES ('a', 'b')", &[]).is_err(), "one edge per pair");
        let walk = |sql: &str| db.query(sql, &[]).expect(sql).len();
        assert_eq!(walk("SELECT * FROM GRAPH_TABLE (base MATCH (x:people WHERE x._key = 'a')-[:knows]->(y:people) RETURN y._key AS k)"), 1);
        assert_eq!(walk("SELECT * FROM GRAPH_TABLE (base MATCH (x:places WHERE x._key = 'p')-[:\"atlas.close_to\"]->(y) RETURN y._key AS k)"), 1);

        for (bad, why) in [
            (request("e1", Vec::new(), edge("person", "people", "person", "people", "")), "twice"),
            (request("e2", Vec::new(), edge("person", "people", "place", "nowhere", "")), "no 'nowhere'"),
            (request("e3", Vec::new(), edge("person", "people", "k", "knows", "")), "is an edge table"),
            (request("e4", vec![CollectionAttribute { name: "person".to_string(), kind: "TEXT".to_string(), ..Default::default() }], edge("person", "people", "friend", "people", "")), "cannot also be a property"),
            (CreateSimpleTableRequest { key_default: "ulid()".to_string(), ..request("e5", Vec::new(), edge("person", "people", "friend", "people", "")) }, "no generated key"),
        ] {
            let err = create_table(tmp.path(), "alice", "demo", &bad).expect_err(why);
            assert!(err.message.contains(why), "{why}: {}", err.message);
        }
        assert!(!list_tables(tmp.path(), "alice", "demo").expect("tables").iter().any(|t| t.table.starts_with('e')));
    }

    /// The DB API's parameters reach the engine: a value is bound to `$1`,
    /// never pasted into the statement, so a quote in it is just a quote.
    #[test]
    fn a_connection_query_binds_its_parameters() {
        let tmp = tmp_root();
        get_db(tmp.path(), "alice", "demo")
            .expect("open")
            .execute("CREATE TABLE notes (_key TEXT PRIMARY KEY, body TEXT)", &[])
            .expect("table");
        let run = |sql: &str, params: Vec<Value>, read_only: bool| {
            execute_connection_query(
                tmp.path(),
                "alice",
                "demo",
                "cid",
                "default-multimodel",
                &QueryProjectDbConnectionRequest {
                    sql: sql.to_string(),
                    params,
                    read_only: Some(read_only),
                    ..Default::default()
                },
            )
        };
        run("INSERT INTO notes (_key, body) VALUES ($1, $2)", vec![json!("n1"), json!("it's bound")], false)
            .expect("insert with parameters");
        let read = run("SELECT body FROM notes WHERE _key = $1", vec![json!("n1")], true).expect("read");
        assert_eq!(read.rows, vec![vec![json!("it's bound")]]);
    }

    /// An edge with properties reads back in THIS build. Zebflow compiles
    /// serde_json with `preserve_order` (deno and arrow turn it on, and Cargo
    /// unifies features), and sekejap before 0.18.5 wrote such objects in
    /// insertion order and then refused to read them: the first edge made
    /// the table unreadable, undeletable and undroppable.
    #[test]
    fn an_edge_with_properties_reads_back_in_this_build() {
        let tmp = tmp_root();
        let run = |sql: &str| execute_sql(tmp.path(), "alice", "demo", sql, &[], 10, false).unwrap_or_else(|e| panic!("{sql}: {e:?}"));
        for sql in [
            "CREATE TABLE people (_key TEXT PRIMARY KEY, name TEXT)",
            "CREATE TABLE places (_key TEXT PRIMARY KEY, name TEXT)",
            "CREATE TABLE visited (person TEXT REFERENCES people, place TEXT REFERENCES places, position TEXT, department TEXT, from_year INT, is_current BOOLEAN, is_public BOOLEAN DEFAULT true)",
            "ALTER PROPERTY GRAPH base ADD EDGE TABLES (visited SOURCE KEY (person) REFERENCES people (_key) DESTINATION KEY (place) REFERENCES places (_key))",
            "INSERT INTO people (_key, name) VALUES ('a', 'Ann')",
            "INSERT INTO places (_key, name) VALUES ('p', 'Park')",
            "INSERT INTO visited (person, place, position, department, from_year, is_current) VALUES ('a', 'p', 'PhD', 'Engineering', 2024, true)",
        ] {
            run(sql);
        }
        let read = execute_sql(tmp.path(), "alice", "demo",
            "SELECT * FROM GRAPH_TABLE (base MATCH (a:people)-[e:visited]->(b:places) RETURN e.position AS position, e.is_public AS is_public)",
            &[], 10, true).expect("the edge reads back");
        assert_eq!(read.rows, vec![vec![json!("PhD"), json!(true)]]);
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
        for ddl in [
            "CREATE SCHEMA geo",
            "DROP SCHEMA geo",
            "CREATE PROPERTY GRAPH g VERTEX TABLES (a) EDGE TABLES (e SOURCE KEY (x) REFERENCES a (_key) DESTINATION KEY (y) REFERENCES a (_key))",
            "ALTER PROPERTY GRAPH g ADD VERTEX TABLES (b)",
            "drop property graph g",
        ] {
            assert!(statement_changes_schema(ddl), "{ddl}");
        }

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

    /// sekejap refuses an INSERT that gives no key (23502) itself, a key its
    /// DEFAULT mints comes back through RETURNING, and an edge table — which
    /// has no `_key` — takes a plain INSERT.
    #[test]
    fn keys_come_from_the_table_and_returning_answers_them() {
        let tmp = tmp_root();
        let run = |sql: &str| execute_sql(tmp.path(), "alice", "demo", sql, &[], 100, false);
        run("CREATE TABLE posts (_key TEXT PRIMARY KEY, title TEXT)").expect("posts");
        let missing = run("INSERT INTO posts (title) VALUES ('Hello')").unwrap_err();
        assert!(missing.message.contains("23502"), "{}", missing.message);

        run("CREATE TABLE notes (_key TEXT PRIMARY KEY DEFAULT ulid(), body TEXT, status TEXT DEFAULT 'draft')")
            .expect("notes");
        let made = run("INSERT INTO notes (body) VALUES ('first') RETURNING _key, status").expect("insert");
        assert_eq!(made.affected_rows, Some(1));
        assert_eq!(made.rows.len(), 1);
        let key = made.rows[0][0].as_str().expect("a minted key").to_string();
        assert_eq!(key.len(), 26, "a ULID: {key}");
        assert_eq!(made.rows[0][1], Value::String("draft".to_string()));
        // The word inside a value is not a clause.
        let plain = run("INSERT INTO notes (body) VALUES ('returning soon')").expect("plain insert");
        assert_eq!(plain.affected_rows, Some(1));
        assert!(plain.rows.is_empty());

        for sql in [
            "CREATE TABLE people (_key TEXT PRIMARY KEY, name TEXT)",
            "CREATE TABLE knows (src TEXT REFERENCES people, dst TEXT REFERENCES people, PRIMARY KEY (src, dst))",
            "CREATE PROPERTY GRAPH social VERTEX TABLES (people) EDGE TABLES (knows SOURCE KEY (src) REFERENCES people (_key) DESTINATION KEY (dst) REFERENCES people (_key))",
            "INSERT INTO people (_key, name) VALUES ('a', 'A'), ('b', 'B')",
            "INSERT INTO knows (src, dst) VALUES ('a', 'b')",
        ] {
            run(sql).expect(sql);
        }
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
            "SELECT _key, name FROM GRAPH_TABLE (base MATCH (a:people WHERE a._key = 'alice')-[:knows]->(b:people) RETURN b._key AS _key, b.name AS name)",
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
            "SELECT _key, name FROM GRAPH_TABLE (base MATCH (b:people WHERE b._key = 'bob')<-[:knows]-(a:people) RETURN a._key AS _key, a.name AS name)",
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

}

#[cfg(test)]
mod format_move {
    use super::*;

    /// A project whose store folder exists but holds nothing yet — the folder
    /// is made before the first open — has nothing to move, and opens.
    #[test]
    fn an_empty_store_folder_is_not_a_store_to_move() {
        let root = tempfile::tempdir().expect("tempdir");
        let dir = ensure_project_dir(root.path(), "demo", "site-a").expect("dir");
        move_to_current_format(&dir).expect("an empty folder moves nothing");
        let db = get_db(root.path(), "demo", "site-a").expect("opens");
        drop(db);
        evict_project_pool(root.path(), "demo", "site-a");
    }

    /// A folder that does not exist yet is not an error either.
    #[test]
    fn a_missing_store_folder_is_not_a_store_to_move() {
        let root = tempfile::tempdir().expect("tempdir");
        move_to_current_format(&root.path().join("none")).expect("nothing to move");
    }
}

#[cfg(test)]
mod legacy_016 {
    use super::*;

    /// A store carrying the `legacy-0.16/` folder Zebflow's 0.17 move made
    /// has it moved beside the store before the format move reads the store.
    #[test]
    fn the_retired_016_folder_leaves_the_store_before_the_move() {
        let root = tempfile::tempdir().expect("tempdir");
        let dir = ensure_project_dir(root.path(), "demo", "site-a").expect("dir");
        drop(get_db(root.path(), "demo", "site-a").expect("create a 0.19 store"));
        evict_project_pool(root.path(), "demo", "site-a");
        std::fs::create_dir_all(dir.join(LEGACY_016_DIR)).expect("legacy dir");
        std::fs::write(dir.join(LEGACY_016_DIR).join("wal.log"), b"header").expect("file");
        move_to_current_format(&dir).expect("moves");
        assert!(!dir.join(LEGACY_016_DIR).exists());
        assert!(dir.with_file_name(format!("{STORE_DIR}.{LEGACY_016_DIR}")).join("wal.log").is_file());
        drop(get_db(root.path(), "demo", "site-a").expect("still opens"));
        evict_project_pool(root.path(), "demo", "site-a");
    }
}
