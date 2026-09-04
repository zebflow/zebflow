use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::time::Instant;

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

// ── CoreDB connection pool ───────────────────────────────────────────────────
//
// Keyed by canonical directory path.  `RwLock<CoreDB>` gives concurrent readers
// and exclusive writers.  The outer `Mutex` protects the pool map itself.

type DbPool = HashMap<PathBuf, Arc<RwLock<sekejap::CoreDB>>>;
type MaintenancePool = HashMap<PathBuf, SekejapAutoMaintenanceState>;

static POOL: OnceLock<Mutex<DbPool>> = OnceLock::new();
static MAINTENANCE: OnceLock<Mutex<MaintenancePool>> = OnceLock::new();

const AUTO_COMPACT_WRITE_UNITS: usize = 10_000;
const AUTO_COMPACT_WAL_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Default)]
struct SekejapAutoMaintenanceState {
    write_units_since_compact: usize,
}

fn pool() -> &'static Mutex<DbPool> {
    POOL.get_or_init(|| Mutex::new(HashMap::new()))
}

fn maintenance_pool() -> &'static Mutex<MaintenancePool> {
    MAINTENANCE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn get_db(
    data_root: &Path,
    owner: &str,
    project: &str,
) -> Result<Arc<RwLock<sekejap::CoreDB>>, PlatformError> {
    let dir = ensure_project_dir(data_root, owner, project)?;
    let mut map = pool().lock().unwrap();
    if let Some(db) = map.get(&dir) {
        return Ok(Arc::clone(db));
    }
    let db = sekejap::CoreDB::open(&dir).map_err(|err| {
        PlatformError::new(
            "PLATFORM_SEKEJAP_OPEN",
            format!("failed to open sekejap store: {err}"),
        )
    })?;
    let arc = Arc::new(RwLock::new(db));
    map.insert(dir, Arc::clone(&arc));
    Ok(arc)
}

/// Record project write pressure and checkpoint the WAL when it is large enough.
///
/// This is deliberately best-effort. The write already succeeded by the time
/// this function runs, so automatic maintenance must not turn an accepted
/// mutation into a failed pipeline result. Explicit `compact_project` calls
/// still report failures to the caller.
fn record_project_write(data_root: &Path, owner: &str, project: &str, write_units: usize) {
    if write_units == 0 {
        return;
    }
    let Ok(dir) = ensure_project_dir(data_root, owner, project) else {
        return;
    };
    let wal_bytes = file_len_or_zero(&dir.join("wal.log"));
    let should_compact = {
        let mut map = maintenance_pool().lock().unwrap();
        let state = map.entry(dir.clone()).or_default();
        state.write_units_since_compact =
            state.write_units_since_compact.saturating_add(write_units);
        state.write_units_since_compact >= AUTO_COMPACT_WRITE_UNITS
            || wal_bytes >= AUTO_COMPACT_WAL_BYTES
    };
    if !should_compact {
        return;
    }
    if compact_project(data_root, owner, project).is_ok() {
        let mut map = maintenance_pool().lock().unwrap();
        let state = map.entry(dir).or_default();
        state.write_units_since_compact = 0;
    }
}

/// Return cheap project-scoped Sekejap health and persistence stats.
///
/// This is intentionally small and side-effect free so settings pages, health
/// checks, and maintenance jobs can call it without touching pipeline runtime
/// state. Reads use the project DB `RwLock` read guard; compaction/sync use the
/// write guard below.
pub fn project_health(
    data_root: &Path,
    owner: &str,
    project: &str,
) -> Result<SekejapProjectHealth, PlatformError> {
    let dir = ensure_project_dir(data_root, owner, project)?;
    let started = Instant::now();
    let db_arc = get_db(data_root, owner, project)?;
    let db = db_arc.read().unwrap();
    let node_count = db.node_count();
    let edge_count = db.edge_count();
    drop(db);

    Ok(SekejapProjectHealth {
        owner: owner.to_string(),
        project: project.to_string(),
        root: dir.to_string_lossy().to_string(),
        node_count,
        edge_count,
        wal_bytes: file_len_or_zero(&dir.join("wal.log")),
        snapshot_bytes: file_len_or_zero(&dir.join("snapshot.json")),
        payload_bytes: file_len_or_zero(&dir.join("payloads.bin")),
        sidecar_bytes: sekejap_sidecar_bytes(&dir),
        duration_ms: started.elapsed().as_millis() as u64,
    })
}

/// Force the project's open Sekejap WAL to disk.
///
/// Use after critical single-write batches that are not wrapped in SQL
/// transactions. SQL `COMMIT` already calls Sekejap's `sync()` internally.
pub fn sync_project(
    data_root: &Path,
    owner: &str,
    project: &str,
) -> Result<SekejapMaintenanceReport, PlatformError> {
    let started = Instant::now();
    let before = project_health(data_root, owner, project)?;
    let db_arc = get_db(data_root, owner, project)?;
    let mut db = db_arc.write().unwrap();
    db.sync().map_err(|err| {
        PlatformError::new(
            "PLATFORM_SEKEJAP_SYNC",
            format!("failed to sync sekejap WAL: {err}"),
        )
    })?;
    drop(db);
    let after = project_health(data_root, owner, project)?;
    Ok(SekejapMaintenanceReport {
        operation: "sync".to_string(),
        before,
        after,
        duration_ms: started.elapsed().as_millis() as u64,
    })
}

/// Compact the project's open Sekejap DB: snapshot current state and reset the WAL.
///
/// This is the foundational WAL checkpoint primitive. It should be called by
/// explicit admin actions, low-traffic scheduled maintenance, and graceful
/// shutdown hooks for projects that have been opened in-process. Sekejap 0.12+
/// retains the fresh WAL's 8-byte format header after a successful checkpoint.
pub fn compact_project(
    data_root: &Path,
    owner: &str,
    project: &str,
) -> Result<SekejapMaintenanceReport, PlatformError> {
    let started = Instant::now();
    let before = project_health(data_root, owner, project)?;
    let db_arc = get_db(data_root, owner, project)?;
    let mut db = db_arc.write().unwrap();
    db.compact().map_err(|err| {
        PlatformError::new(
            "PLATFORM_SEKEJAP_COMPACT",
            format!("failed to compact sekejap store: {err}"),
        )
    })?;
    drop(db);
    let after = project_health(data_root, owner, project)?;
    Ok(SekejapMaintenanceReport {
        operation: "compact".to_string(),
        before,
        after,
        duration_ms: started.elapsed().as_millis() as u64,
    })
}

fn file_len_or_zero(path: &Path) -> u64 {
    std::fs::metadata(path).map(|meta| meta.len()).unwrap_or(0)
}

fn sekejap_sidecar_bytes(dir: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                return false;
            };
            name == "gin.bin"
                || name == "edge_meta.bin"
                || name.starts_with("vectors_") && name.ends_with(".bin")
        })
        .map(|path| file_len_or_zero(&path))
        .sum()
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SekejapProjectHealth {
    pub owner: String,
    pub project: String,
    pub root: String,
    pub node_count: usize,
    pub edge_count: usize,
    pub wal_bytes: u64,
    pub snapshot_bytes: u64,
    pub payload_bytes: u64,
    pub sidecar_bytes: u64,
    pub duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SekejapMaintenanceReport {
    pub operation: String,
    pub before: SekejapProjectHealth,
    pub after: SekejapProjectHealth,
    pub duration_ms: u64,
}

/// Drops the pooled in-memory handle for one project store.
///
/// A `ProjectBundle` import swaps `data/store/` as a unit
/// (`kinds/project-bundle/README.md`), so a `CoreDB` opened against the
/// displaced directory must not keep serving its bytes. Callers still holding
/// a cloned `Arc` finish their in-flight call on the old handle; the next
/// `get_db` reopens from the swapped-in directory.
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

fn ensure_project_dir(
    data_root: &Path,
    owner: &str,
    project: &str,
) -> Result<PathBuf, PlatformError> {
    let dir = project_dir(data_root, owner, project);
    migrate_legacy_sekejap_dir(data_root, owner, project, &dir)?;
    std::fs::create_dir_all(&dir)?;
    // `tables.json` was a hand-kept mirror of the table list from before
    // sekejap could be asked directly. `live_tables` reads the database now, so
    // the file is only stale weight; drop it the first time a project is
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
/// This is the one chokepoint every Sekejap access already runs through
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

fn map_declared_kind(kind: &str) -> &'static str {
    match kind {
        "number" | "real" | "integer" => "number",
        "boolean" => "boolean",
        "json" => "json",
        "vector" => "vector",
        "geo" => "geo",
        _ => "string",
    }
}

fn map_field_type(kind: &str) -> &'static str {
    match kind {
        "number" | "real" => "REAL",
        "boolean" => "BOOLEAN",
        "json" => "JSON",
        "vector" => "VECTOR",
        "geo" => "GEO",
        _ => "TEXT",
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

    let mut attrs = Vec::new();
    let mut seen = BTreeSet::new();
    let mut has_user_key = false;
    for attr in &req.attributes {
        let name = slug_segment(&attr.name);
        if name.is_empty() || !seen.insert(name.clone()) {
            continue;
        }
        if name == "_key" {
            has_user_key = true;
        }
        let kind = map_declared_kind(&slug_segment(&attr.kind)).to_string();
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
    if !has_user_key {
        attrs.insert(
            0,
            CollectionAttribute {
                name: "_key".to_string(),
                kind: "string".to_string(),
                index_types: Vec::new(),
            },
        );
    }

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

fn build_create_table_sql(def: &SimpleTableDefinition) -> String {
    let mut columns = Vec::new();
    for attr in &def.attributes {
        // `_key` is the primary key and auto-fills with a UUID when an INSERT
        // omits it. `UUIDV4()` and `UUIDV5(..)` are the only DEFAULT forms
        // sekejap's parser honours — see `sql.rs::parse_field_default`, which
        // silently discards every other expression — so no other column
        // carries one.
        if attr.name == "_key" {
            columns.push(format!(
                "_key {} PRIMARY KEY DEFAULT UUIDV4()",
                map_field_type(&attr.kind)
            ));
        } else {
            columns.push(format!("{} {}", attr.name, map_field_type(&attr.kind)));
        }
    }
    format!("CREATE TABLE {} ({})", def.collection, columns.join(", "))
}

fn build_index_sql(collection: &str, method: &str, field: &str) -> String {
    format!(
        "CREATE INDEX ON {} USING {} ({})",
        collection, method, field
    )
}

fn row_count_for_collection(db: &sekejap::CoreDB, collection: &str) -> usize {
    db.collection(collection).count()
}

fn field_type_to_kind(ty: &sekejap::sql::FieldType) -> &'static str {
    match ty {
        sekejap::sql::FieldType::Text => "string",
        sekejap::sql::FieldType::Integer => "number",
        sekejap::sql::FieldType::Real => "number",
        sekejap::sql::FieldType::Bool => "boolean",
        sekejap::sql::FieldType::Timestamptz => "number",
        sekejap::sql::FieldType::Geo => "geo",
        sekejap::sql::FieldType::Vector => "vector",
        sekejap::sql::FieldType::Json => "json",
    }
}

fn infer_kind_from_value(val: &Value) -> &'static str {
    match val {
        Value::String(_) => "string",
        Value::Number(_) => "number",
        Value::Bool(_) => "boolean",
        Value::Object(obj) => {
            if obj.contains_key("type") && obj.contains_key("coordinates") {
                "geo"
            } else {
                "json"
            }
        }
        Value::Array(_) => "json",
        Value::Null => "string",
    }
}

fn backfill_from_sample(db: &sekejap::CoreDB, collection: &str) -> Vec<CollectionAttribute> {
    let hits: Vec<sekejap::Hit> = db.collection(collection).take(1).collect();
    let payload = match hits.first().and_then(|h| h.payload.as_ref()) {
        Some(Value::Object(map)) => map,
        _ => return Vec::new(),
    };
    payload
        .keys()
        .filter(|k| !k.starts_with('_'))
        .map(|k| CollectionAttribute {
            name: k.clone(),
            kind: infer_kind_from_value(&payload[k]).to_string(),
            index_types: Vec::new(),
        })
        .collect()
}

fn backfill_from_schema(
    db: &sekejap::CoreDB,
    collection: &str,
) -> (
    Vec<CollectionAttribute>,
    Vec<String>,
    Vec<String>,
    Vec<String>,
    Vec<String>,
    Vec<String>,
) {
    let schema = match db.table_schema(collection) {
        Some(s) => s,
        None => {
            let attrs = backfill_from_sample(db, collection);
            return (
                attrs,
                vec!["_key".to_string()],
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            );
        }
    };

    let attrs: Vec<CollectionAttribute> = schema
        .fields
        .iter()
        .filter(|f| !f.name.starts_with('_'))
        .map(|f| {
            let kind = field_type_to_kind(&f.ty).to_string();
            let mut index_types = Vec::new();
            if schema.indexes.hash.contains(&f.name) {
                index_types.push("hash".to_string());
            }
            if schema.indexes.range.contains(&f.name) {
                index_types.push("range".to_string());
            }
            if schema.indexes.fulltext.contains(&f.name) || schema.indexes.bm25.contains(&f.name) {
                index_types.push("fulltext".to_string());
            }
            if schema.indexes.vector.contains(&f.name) {
                index_types.push("vector".to_string());
            }
            if schema.indexes.spatial.contains(&f.name) {
                index_types.push("spatial".to_string());
            }
            CollectionAttribute {
                name: f.name.clone(),
                kind,
                index_types,
            }
        })
        .collect();

    let hash = schema.indexes.hash.clone();
    let range = schema.indexes.range.clone();
    let fulltext = {
        let mut v = schema.indexes.fulltext.clone();
        for f in &schema.indexes.bm25 {
            if !v.contains(f) {
                v.push(f.clone());
            }
        }
        v
    };
    let vector = schema.indexes.vector.clone();
    let spatial = schema.indexes.spatial.clone();

    (attrs, hash, range, fulltext, vector, spatial)
}

/// Every table in the project, read from the live database.
///
/// `SHOW TABLES` is the listing: sekejap answers it by unioning the
/// collections that hold rows with the schemas that `CREATE TABLE` declared,
/// so a table that has never been written to is still reported. This is the
/// reason it is the listing and `collection_names()` is not — that one walks
/// stored rows, so an empty table is invisible to it.
///
/// Each name is then filled in by `table_schema()`, which reports the fields
/// with their types and index kinds, and `collection().count()`.
///
/// Nothing is cached alongside this. A structure fact the database cannot
/// answer is a fact Zebflow does not keep.
fn live_tables(db: &sekejap::CoreDB) -> Vec<SimpleTableDefinition> {
    let Ok(hits) = db.show("SHOW TABLES") else {
        return Vec::new();
    };
    let mut by_table = BTreeMap::new();
    for hit in hits {
        let Some(collection) = hit
            .payload
            .as_ref()
            .and_then(|row| row.get("name"))
            .and_then(Value::as_str)
            .map(str::to_string)
        else {
            continue;
        };
        let table = slug_segment(&collection);
        if table.is_empty() || by_table.contains_key(&table) {
            continue;
        }
        let (attributes, hash, range, fulltext, vector, spatial) =
            backfill_from_schema(db, &collection);
        by_table.insert(
            table.clone(),
            SimpleTableDefinition {
                table,
                attributes,
                hash_indexed_fields: hash,
                range_indexed_fields: range,
                fulltext_fields: fulltext,
                vector_fields: vector,
                spatial_fields: spatial,
                row_count: row_count_for_collection(db, &collection),
                collection: collection.clone(),
            },
        );
    }
    by_table.into_values().collect()
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
        attributes: table.attributes.clone(),
        hash_indexed_fields: stable_list(table.hash_indexed_fields.clone()),
        range_indexed_fields: stable_list(table.range_indexed_fields.clone()),
        fulltext_fields: stable_list(table.fulltext_fields.clone()),
        vector_fields: stable_list(table.vector_fields.clone()),
        spatial_fields: stable_list(table.spatial_fields.clone()),
        row_count: 0,
    })
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
    let db_arc = get_db(data_root, owner, project)?;
    let mut db = db_arc.write().unwrap();
    let mut existing_tables = live_tables(&db)
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

        db.execute(&build_create_table_sql(&def))
            .map_err(|err| PlatformError::new("PLATFORM_SEKEJAP_SCHEMA_APPLY", err.to_string()))?;
        for field in &def.hash_indexed_fields {
            if field == "_key" {
                continue;
            }
            db.execute(&build_index_sql(&def.collection, "hash", field))
                .map_err(|err| {
                    PlatformError::new("PLATFORM_SEKEJAP_SCHEMA_APPLY", err.to_string())
                })?;
        }
        for field in &def.range_indexed_fields {
            db.execute(&build_index_sql(&def.collection, "btree", field))
                .map_err(|err| {
                    PlatformError::new("PLATFORM_SEKEJAP_SCHEMA_APPLY", err.to_string())
                })?;
        }
        for field in &def.fulltext_fields {
            db.execute(&build_index_sql(&def.collection, "gist", field))
                .map_err(|err| {
                    PlatformError::new("PLATFORM_SEKEJAP_SCHEMA_APPLY", err.to_string())
                })?;
        }
        for field in &def.vector_fields {
            db.execute(&build_index_sql(&def.collection, "hnsw", field))
                .map_err(|err| {
                    PlatformError::new("PLATFORM_SEKEJAP_SCHEMA_APPLY", err.to_string())
                })?;
        }
        for field in &def.spatial_fields {
            db.execute(&build_index_sql(&def.collection, "spatial", field))
                .map_err(|err| {
                    PlatformError::new("PLATFORM_SEKEJAP_SCHEMA_APPLY", err.to_string())
                })?;
        }

        existing_tables.insert(def.table.clone());
        tables_created.push(def.table);
    }

    drop(db);
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
    let db_arc = get_db(data_root, owner, project)?;
    let db = db_arc.read().unwrap();
    let defs = live_tables(&db);
    drop(db);
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
    let db_arc = get_db(data_root, owner, project)?;
    let db = db_arc.read().unwrap();
    Ok(live_tables(&db))
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

    let db_arc = get_db(data_root, owner, project)?;
    let mut db = db_arc.write().unwrap();
    db.execute(&build_create_table_sql(&def))
        .map_err(|err| PlatformError::new("PLATFORM_SEKEJAP_TABLE_CREATE", err.to_string()))?;

    for field in &def.hash_indexed_fields {
        if field == "_key" {
            continue;
        }
        db.execute(&build_index_sql(&def.collection, "hash", field))
            .map_err(|err| PlatformError::new("PLATFORM_SEKEJAP_INDEX_CREATE", err.to_string()))?;
    }
    for field in &def.range_indexed_fields {
        db.execute(&build_index_sql(&def.collection, "btree", field))
            .map_err(|err| PlatformError::new("PLATFORM_SEKEJAP_INDEX_CREATE", err.to_string()))?;
    }
    for field in &def.fulltext_fields {
        db.execute(&build_index_sql(&def.collection, "gist", field))
            .map_err(|err| PlatformError::new("PLATFORM_SEKEJAP_INDEX_CREATE", err.to_string()))?;
    }
    for field in &def.vector_fields {
        db.execute(&build_index_sql(&def.collection, "hnsw", field))
            .map_err(|err| PlatformError::new("PLATFORM_SEKEJAP_INDEX_CREATE", err.to_string()))?;
    }
    for field in &def.spatial_fields {
        db.execute(&build_index_sql(&def.collection, "spatial", field))
            .map_err(|err| PlatformError::new("PLATFORM_SEKEJAP_INDEX_CREATE", err.to_string()))?;
    }

    let mut created = def;
    created.row_count = row_count_for_collection(&db, &created.collection);
    drop(db);
    sync_schema_to_repo(data_root, owner, project)?;
    Ok(created)
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

    let db_arc = get_db(data_root, owner, project)?;
    let mut db = db_arc.write().unwrap();
    let Some(def) = live_tables(&db).into_iter().find(|d| d.table == table_slug) else {
        return Err(PlatformError::new(
            "PLATFORM_SEKEJAP_TABLE_NOT_FOUND",
            format!("table '{}' not found", table_slug),
        ));
    };

    db.execute(&format!("DROP TABLE IF EXISTS {}", def.collection))
        .map_err(|err| PlatformError::new("PLATFORM_SEKEJAP_TABLE_DROP", err.to_string()))?;
    if db
        .collection_names()
        .into_iter()
        .any(|collection| collection == def.collection)
    {
        return Err(PlatformError::new(
            "PLATFORM_SEKEJAP_TABLE_DROP",
            format!("table '{}' still exists after DROP TABLE", table_slug),
        ));
    }
    drop(db);

    sync_schema_to_repo(data_root, owner, project)?;
    Ok(())
}

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

    let db_arc = get_db(data_root, owner, project)?;
    let mut db = db_arc.write().unwrap();
    let Some(existing) = live_tables(&db).into_iter().find(|d| d.table == table_slug) else {
        return Err(PlatformError::new(
            "PLATFORM_SEKEJAP_TABLE_NOT_FOUND",
            format!("table '{}' not found", table_slug),
        ));
    };

    let mut attrs = Vec::new();
    let mut seen = BTreeSet::new();
    for attr in &req.attributes {
        let name = slug_segment(&attr.name);
        if name.is_empty() || !seen.insert(name.clone()) {
            continue;
        }
        let kind = map_declared_kind(&slug_segment(&attr.kind)).to_string();
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

    let hash_indexed_fields = collect_index_fields(&attrs, &req.hash_indexed_fields, "hash");
    let range_indexed_fields = collect_index_fields(&attrs, &req.range_indexed_fields, "range");
    let fulltext_fields = collect_index_fields(&attrs, &[], "fulltext");
    let vector_fields = collect_index_fields(&attrs, &[], "vector");
    let spatial_fields = collect_index_fields(&attrs, &[], "spatial");

    // Add new columns that don't exist yet.
    let old_names: BTreeSet<String> = existing.attributes.iter().map(|a| a.name.clone()).collect();
    for attr in &attrs {
        if !old_names.contains(&attr.name) {
            let sql = format!(
                "ALTER TABLE {} ADD COLUMN {} {}",
                existing.collection,
                attr.name,
                map_field_type(&attr.kind)
            );
            let _ = db.execute(&sql);
        }
    }

    // Rebuild indexes: drop all then recreate.
    for field in &existing.hash_indexed_fields {
        let _ = db.execute(&format!(
            "DROP INDEX ON {} USING hash ({})",
            existing.collection, field
        ));
    }
    for field in &existing.range_indexed_fields {
        let _ = db.execute(&format!(
            "DROP INDEX ON {} USING btree ({})",
            existing.collection, field
        ));
    }
    for field in &existing.fulltext_fields {
        let _ = db.execute(&format!(
            "DROP INDEX ON {} USING gist ({})",
            existing.collection, field
        ));
    }
    for field in &existing.vector_fields {
        let _ = db.execute(&format!(
            "DROP INDEX ON {} USING hnsw ({})",
            existing.collection, field
        ));
    }
    for field in &existing.spatial_fields {
        let _ = db.execute(&format!(
            "DROP INDEX ON {} USING spatial ({})",
            existing.collection, field
        ));
    }

    for field in &hash_indexed_fields {
        if field == "_key" {
            continue;
        }
        let _ = db.execute(&build_index_sql(&existing.collection, "hash", field));
    }
    for field in &range_indexed_fields {
        let _ = db.execute(&build_index_sql(&existing.collection, "btree", field));
    }
    for field in &fulltext_fields {
        let _ = db.execute(&build_index_sql(&existing.collection, "gist", field));
    }
    for field in &vector_fields {
        let _ = db.execute(&build_index_sql(&existing.collection, "hnsw", field));
    }
    for field in &spatial_fields {
        let _ = db.execute(&build_index_sql(&existing.collection, "spatial", field));
    }

    let row_count = row_count_for_collection(&db, &existing.collection);
    drop(db);

    let updated = SimpleTableDefinition {
        table: existing.table.clone(),
        collection: existing.collection.clone(),
        attributes: attrs,
        hash_indexed_fields,
        range_indexed_fields,
        fulltext_fields,
        vector_fields,
        spatial_fields,
        row_count,
    };

    sync_schema_to_repo(data_root, owner, project)?;

    Ok(updated)
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

fn statement_is_write(sql: &str) -> bool {
    let first = sql
        .trim_start()
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_uppercase();
    matches!(
        first.as_str(),
        "INSERT" | "UPDATE" | "DELETE" | "CREATE" | "DROP" | "ALTER"
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
            | ("ALTER", "TABLE")
            | ("ALTER", "COLLECTION")
            | ("DROP", "TABLE")
            | ("DROP", "COLLECTION")
            | ("DROP", "INDEX")
    )
}

fn statement_is_show(sql: &str) -> bool {
    let first = sql
        .trim_start()
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_uppercase();
    first == "SHOW"
}

/// Detect `EXPLAIN [ANALYZE] ...` statements.
fn statement_is_explain(sql: &str) -> bool {
    let first = sql
        .trim_start()
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_uppercase();
    first == "EXPLAIN"
}

/// Detect `EXPLAIN ANALYZE ...` specifically.
fn statement_is_explain_analyze(sql: &str) -> bool {
    let mut words = sql.trim_start().split_whitespace();
    let first = words.next().unwrap_or("").to_ascii_uppercase();
    let second = words.next().unwrap_or("").to_ascii_uppercase();
    first == "EXPLAIN" && second == "ANALYZE"
}

/// Strip the `EXPLAIN [ANALYZE]` prefix and return the inner SQL.
fn strip_explain_prefix(sql: &str) -> &str {
    let rest = sql
        .trim_start()
        .strip_prefix("EXPLAIN")
        .unwrap_or(sql)
        .trim_start();
    // Also strip ANALYZE if present
    rest.strip_prefix("ANALYZE")
        .or_else(|| rest.strip_prefix("analyze"))
        .unwrap_or(rest)
        .trim_start()
}

fn statement_is_show_tables(sql: &str) -> bool {
    let normalized = sql.trim().trim_end_matches(';');
    let parts = normalized
        .split_whitespace()
        .map(|part| part.to_ascii_uppercase())
        .collect::<Vec<_>>();
    parts.len() == 2 && parts[0] == "SHOW" && parts[1] == "TABLES"
}

fn hit_to_row_map(hit: sekejap::Hit) -> Map<String, Value> {
    match hit.payload {
        Some(Value::Object(map)) => map,
        Some(other) => {
            let mut out = Map::new();
            out.insert("value".to_string(), other);
            out
        }
        None => {
            let mut out = Map::new();
            out.insert("slug".to_string(), Value::String(hit.slug));
            out.insert("slug_hash".to_string(), json!(hit.slug_hash));
            out
        }
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
    let trimmed = sql.trim();
    if trimmed.is_empty() {
        return Err(PlatformError::new(
            "PLATFORM_SEKEJAP_QUERY_INVALID",
            "query.sql must not be empty for sekejap",
        ));
    }

    let started = Instant::now();
    if statement_is_write(trimmed) {
        if read_only {
            return Err(PlatformError::new(
                "PLATFORM_SEKEJAP_QUERY_READ_ONLY",
                "write statement rejected in read-only mode",
            ));
        }
        let db_arc = get_db(data_root, owner, project)?;
        let mut db = db_arc.write().unwrap();
        let affected_rows = if params.is_empty() {
            db.execute(trimmed)
        } else {
            db.execute_params(trimmed, params)
        }
        .map_err(|err| PlatformError::new("PLATFORM_SEKEJAP_QUERY_FAILED", err.to_string()))?;
        let should_sync_schema = statement_changes_schema(trimmed);
        drop(db);
        if should_sync_schema {
            sync_schema_to_repo(data_root, owner, project)?;
        }
        record_project_write(data_root, owner, project, affected_rows.max(1));
        return Ok(QueryPayload {
            columns: Vec::new(),
            rows: Vec::new(),
            row_count: 0,
            truncated: false,
            affected_rows: Some(affected_rows as u64),
            duration_ms: started.elapsed().as_millis() as u64,
        });
    }

    if statement_is_explain(trimmed) {
        let db_arc = get_db(data_root, owner, project)?;
        let db = db_arc.read().unwrap();
        let inner_sql = strip_explain_prefix(trimmed);
        let hits = if statement_is_explain_analyze(trimmed) {
            db.explain_analyze(inner_sql)
        } else {
            db.explain(inner_sql)
        }
        .map_err(|err| PlatformError::new("PLATFORM_SEKEJAP_QUERY_FAILED", err.to_string()))?;

        let max_rows = limit.clamp(1, 5_000);
        let truncated = hits.len() > max_rows;
        let mut column_names = Vec::<String>::new();
        let mut row_maps = Vec::<Map<String, Value>>::new();
        for hit in hits.into_iter().take(max_rows) {
            let row = hit_to_row_map(hit);
            for key in row.keys() {
                if !column_names.iter().any(|existing| existing == key) {
                    column_names.push(key.clone());
                }
            }
            row_maps.push(row);
        }
        let columns = column_names
            .iter()
            .map(|name| DbQueryColumn {
                name: name.clone(),
                data_type: None,
            })
            .collect::<Vec<_>>();
        let rows = row_maps
            .into_iter()
            .map(|row| {
                column_names
                    .iter()
                    .map(|name| row.get(name).cloned().unwrap_or(Value::Null))
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();

        return Ok(QueryPayload {
            row_count: rows.len(),
            columns,
            rows,
            truncated,
            affected_rows: None,
            duration_ms: started.elapsed().as_millis() as u64,
        });
    }

    if statement_is_show_tables(trimmed) {
        let defs = list_tables(data_root, owner, project)?;
        let max_rows = limit.clamp(1, 5_000);
        let truncated = defs.len() > max_rows;
        let rows = defs
            .into_iter()
            .take(max_rows)
            .map(|def| vec![Value::String(def.table), json!(def.row_count)])
            .collect::<Vec<_>>();
        return Ok(QueryPayload {
            columns: vec![
                DbQueryColumn {
                    name: "name".to_string(),
                    data_type: None,
                },
                DbQueryColumn {
                    name: "count".to_string(),
                    data_type: None,
                },
            ],
            row_count: rows.len(),
            rows,
            truncated,
            affected_rows: None,
            duration_ms: started.elapsed().as_millis() as u64,
        });
    }

    let db_arc = get_db(data_root, owner, project)?;
    let db = db_arc.read().unwrap();
    let hits = if statement_is_show(trimmed) {
        db.show(trimmed)
            .map_err(|err| PlatformError::new("PLATFORM_SEKEJAP_QUERY_FAILED", err.to_string()))?
    } else if params.is_empty() {
        db.query(trimmed)
            .map_err(|err| PlatformError::new("PLATFORM_SEKEJAP_QUERY_FAILED", err.to_string()))?
            .collect()
    } else {
        db.query_params(trimmed, params)
            .map_err(|err| PlatformError::new("PLATFORM_SEKEJAP_QUERY_FAILED", err.to_string()))?
            .collect()
    };

    let max_rows = limit.clamp(1, 5_000);
    let truncated = hits.len() > max_rows;
    let mut column_names = Vec::<String>::new();
    let mut row_maps = Vec::<Map<String, Value>>::new();
    for hit in hits.into_iter().take(max_rows) {
        let row = hit_to_row_map(hit);
        for key in row.keys() {
            if !column_names.iter().any(|existing| existing == key) {
                column_names.push(key.clone());
            }
        }
        row_maps.push(row);
    }
    let columns = column_names
        .iter()
        .map(|name| DbQueryColumn {
            name: name.clone(),
            data_type: None,
        })
        .collect::<Vec<_>>();
    let rows = row_maps
        .into_iter()
        .map(|row| {
            column_names
                .iter()
                .map(|name| row.get(name).cloned().unwrap_or(Value::Null))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    Ok(QueryPayload {
        row_count: rows.len(),
        columns,
        rows,
        truncated,
        affected_rows: None,
        duration_ms: started.elapsed().as_millis() as u64,
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_root() -> tempfile::TempDir {
        tempfile::tempdir().expect("temp dir")
    }

    #[test]
    fn create_table_persists_empty_table_definition() {
        let tmp = tmp_root();
        let req = CreateSimpleTableRequest {
            table: "posts".to_string(),
            attributes: vec![CollectionAttribute {
                name: "title".to_string(),
                kind: "string".to_string(),
                index_types: vec!["hash".to_string()],
            }],
            hash_indexed_fields: Vec::new(),
            range_indexed_fields: Vec::new(),
        };

        let created = create_table(tmp.path(), "alice", "demo", &req).expect("create table");
        assert_eq!(created.table, "posts");

        let items = list_tables(tmp.path(), "alice", "demo").expect("list tables");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].table, "posts");
        assert_eq!(items[0].row_count, 0);
    }

    #[test]
    fn create_table_syncs_portable_schema_to_repo() {
        let tmp = tmp_root();
        create_table(
            tmp.path(),
            "alice",
            "demo",
            &CreateSimpleTableRequest {
                table: "posts".to_string(),
                attributes: vec![CollectionAttribute {
                    name: "title".to_string(),
                    kind: "string".to_string(),
                    index_types: vec!["hash".to_string()],
                }],
                hash_indexed_fields: Vec::new(),
                range_indexed_fields: Vec::new(),
            },
        )
        .expect("create table");

        let schema_path = tmp
            .path()
            .join("users/alice/demo/repo/schemas/sekejap/schema.json");
        assert!(schema_path.is_file());
        // The schema document is the whole of it. No per-table sidecar is
        // written beside it.
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
        assert!(schema["spec"]["tables"][0].get("updated_at").is_none());
    }

    #[test]
    fn apply_schema_from_repo_hydrates_live_store() {
        let tmp = tmp_root();
        create_table(
            tmp.path(),
            "alice",
            "source",
            &CreateSimpleTableRequest {
                table: "places".to_string(),
                attributes: vec![
                    CollectionAttribute {
                        name: "name".to_string(),
                        kind: "string".to_string(),
                        index_types: vec!["hash".to_string(), "fulltext".to_string()],
                    },
                    CollectionAttribute {
                        name: "geometry".to_string(),
                        kind: "geo".to_string(),
                        index_types: vec!["spatial".to_string()],
                    },
                    CollectionAttribute {
                        name: "embedding".to_string(),
                        kind: "vector".to_string(),
                        index_types: vec!["vector".to_string()],
                    },
                ],
                hash_indexed_fields: Vec::new(),
                range_indexed_fields: Vec::new(),
            },
        )
        .expect("create source table");

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
        create_table(
            tmp.path(),
            "alice",
            "demo",
            &CreateSimpleTableRequest {
                table: "posts".to_string(),
                attributes: Vec::new(),
                hash_indexed_fields: Vec::new(),
                range_indexed_fields: Vec::new(),
            },
        )
        .expect("create table");
        let tables_dir = tmp
            .path()
            .join("users/alice/demo/repo/schemas/sekejap/tables");
        std::fs::create_dir_all(&tables_dir).expect("create legacy dir");
        let stale_path = tables_dir.join("stale.json");
        std::fs::write(&stale_path, "{}").expect("stale file");

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
            let db_arc = get_db(tmp.path(), "alice", "demo").expect("open db");
            let mut db = db_arc.write().unwrap();
            db.execute("CREATE TABLE live_only (_key TEXT PRIMARY KEY)")
                .expect("create live table");
            db.execute("INSERT INTO live_only (_key) VALUES ('row-1')")
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
        assert!(statement_changes_schema(
            "CREATE TABLE posts (_key TEXT PRIMARY KEY)"
        ));
        assert!(statement_changes_schema(
            "DROP INDEX ON posts USING hash (title)"
        ));
        assert!(!statement_changes_schema(
            "INSERT INTO posts (_key) VALUES ('a')"
        ));

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
        create_table(
            tmp.path(),
            "alice",
            "demo",
            &CreateSimpleTableRequest {
                table: "posts".to_string(),
                attributes: vec![CollectionAttribute {
                    name: "title".to_string(),
                    kind: "string".to_string(),
                    index_types: Vec::new(),
                }],
                hash_indexed_fields: Vec::new(),
                range_indexed_fields: Vec::new(),
            },
        )
        .expect("table");

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
    }

    #[test]
    fn execute_sql_supports_show_tables() {
        let tmp = tmp_root();
        create_table(
            tmp.path(),
            "alice",
            "demo",
            &CreateSimpleTableRequest {
                table: "posts".to_string(),
                attributes: vec![CollectionAttribute {
                    name: "title".to_string(),
                    kind: "string".to_string(),
                    index_types: Vec::new(),
                }],
                hash_indexed_fields: Vec::new(),
                range_indexed_fields: Vec::new(),
            },
        )
        .expect("table");

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
        create_table(
            tmp.path(),
            "alice",
            "demo",
            &CreateSimpleTableRequest {
                table: "posts".to_string(),
                attributes: vec![
                    CollectionAttribute {
                        name: "title".to_string(),
                        kind: "string".to_string(),
                        index_types: Vec::new(),
                    },
                    CollectionAttribute {
                        name: "views".to_string(),
                        kind: "number".to_string(),
                        index_types: Vec::new(),
                    },
                ],
                hash_indexed_fields: Vec::new(),
                range_indexed_fields: Vec::new(),
            },
        )
        .expect("table");

        let read = execute_sql(tmp.path(), "alice", "demo", "SHOW posts", &[], 100, true)
            .expect("show structure");
        assert!(read.row_count >= 2);
        assert_eq!(read.columns.len(), 4);
        assert!(
            read.rows
                .iter()
                .any(|row| { row.first() == Some(&Value::String("title".to_string())) })
        );
    }

    #[test]
    fn execute_sql_supports_select_from_match() {
        let tmp = tmp_root();
        create_table(
            tmp.path(),
            "alice",
            "demo",
            &CreateSimpleTableRequest {
                table: "people".to_string(),
                attributes: vec![CollectionAttribute {
                    name: "name".to_string(),
                    kind: "string".to_string(),
                    index_types: Vec::new(),
                }],
                hash_indexed_fields: Vec::new(),
                range_indexed_fields: Vec::new(),
            },
        )
        .expect("table");

        let db_arc = get_db(tmp.path(), "alice", "demo").expect("db");
        let mut db = db_arc.write().unwrap();
        db.execute("INSERT INTO people (_key, name) VALUES ('alice', 'Alice')")
            .expect("insert alice");
        db.execute("INSERT INTO people (_key, name) VALUES ('bob', 'Bob')")
            .expect("insert bob");
        db.execute("INSERT ('people/alice')-[:knows]->('people/bob')")
            .expect("insert edge");
        drop(db);

        let read = execute_sql(
            tmp.path(),
            "alice",
            "demo",
            "SELECT b._key AS _key, b.name AS name FROM MATCH (a:people)-[:knows]->(b:people) WHERE a._key = 'alice'",
            &[],
            100,
            true,
        )
        .expect("select from match");
        assert_eq!(read.row_count, 1);
        assert_eq!(read.columns.len(), 2);
        assert_eq!(read.rows[0][0], Value::String("bob".to_string()));
        assert_eq!(read.rows[0][1], Value::String("Bob".to_string()));
    }

    #[test]
    fn execute_sql_supports_select_incoming_relation_with_rhs_filter() {
        let tmp = tmp_root();
        create_table(
            tmp.path(),
            "alice",
            "demo",
            &CreateSimpleTableRequest {
                table: "people".to_string(),
                attributes: vec![CollectionAttribute {
                    name: "name".to_string(),
                    kind: "string".to_string(),
                    index_types: Vec::new(),
                }],
                hash_indexed_fields: Vec::new(),
                range_indexed_fields: Vec::new(),
            },
        )
        .expect("table");

        let db_arc = get_db(tmp.path(), "alice", "demo").expect("db");
        let mut db = db_arc.write().unwrap();
        db.execute("INSERT INTO people (_key, name) VALUES ('alice', 'Alice')")
            .expect("insert alice");
        db.execute("INSERT INTO people (_key, name) VALUES ('bob', 'Bob')")
            .expect("insert bob");
        db.execute("INSERT ('people/alice')-[:knows]->('people/bob')")
            .expect("insert edge");
        drop(db);

        let read = execute_sql(
            tmp.path(),
            "alice",
            "demo",
            "SELECT a._key AS _key, a.name AS name FROM MATCH (a:people)-[:knows]->(b:people) WHERE b._key = 'bob'",
            &[],
            100,
            true,
        )
        .expect("select incoming from match");
        assert_eq!(read.row_count, 1);
        assert_eq!(read.columns.len(), 2);
        assert_eq!(read.rows[0][0], Value::String("alice".to_string()));
        assert_eq!(read.rows[0][1], Value::String("Alice".to_string()));
    }

    #[test]
    fn execute_sql_supports_bind_params_for_write_and_read() {
        let tmp = tmp_root();
        create_table(
            tmp.path(),
            "alice",
            "demo",
            &CreateSimpleTableRequest {
                table: "posts".to_string(),
                attributes: vec![CollectionAttribute {
                    name: "title".to_string(),
                    kind: "string".to_string(),
                    index_types: Vec::new(),
                }],
                hash_indexed_fields: Vec::new(),
                range_indexed_fields: Vec::new(),
            },
        )
        .expect("table");

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
    fn bulk_write_records_uses_schema_for_native_fields_and_merge_preserves_fields() {
        let tmp = tmp_root();
        execute_sql(
            tmp.path(),
            "alice",
            "demo",
            "CREATE TABLE docs (_key TEXT PRIMARY KEY, title TEXT, category TEXT, embedding VECTOR)",
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

        let db_arc = get_db(tmp.path(), "alice", "demo").expect("db");
        let db = db_arc.read().unwrap();
        let payload: Value =
            serde_json::from_str(&db.get("docs/d1").expect("payload")).expect("payload json");
        assert_eq!(payload["title"], json!("Updated"));
        assert_eq!(payload["category"], json!("alpha"));
        assert!(payload.get("embedding").is_none());
        assert_eq!(
            db.get_vector("docs/d1", "embedding").expect("vector"),
            &[0.4, 0.5, 0.6]
        );
        drop(db);

        let invalid = bulk_write_records(
            tmp.path(),
            "alice",
            "demo",
            "docs",
            vec![StructuredInsertRecord {
                key: "d2".to_string(),
                fields: Map::from_iter([("title".to_string(), json!([4, 5]))]),
            }],
            StructuredWriteMode::Upsert,
        );
        assert!(invalid.is_err());
    }

    #[test]
    fn project_health_and_compact_report_wal_checkpoint() {
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
        assert!(before.wal_bytes > 8);

        let report = compact_project(tmp.path(), "alice", "demo").expect("compact");
        assert_eq!(report.operation, "compact");
        assert_eq!(report.after.node_count, 1);
        assert_eq!(report.after.wal_bytes, 8);
        assert!(report.after.snapshot_bytes > 0);
    }
}

pub fn bulk_write_records(
    data_root: &Path,
    owner: &str,
    project: &str,
    collection: &str,
    rows: Vec<StructuredInsertRecord>,
    mode: StructuredWriteMode,
) -> Result<StructuredWritePayload, PlatformError> {
    let collection = collection.trim();
    if collection.is_empty() {
        return Err(PlatformError::new(
            "PLATFORM_SEKEJAP_INSERT_INVALID",
            "collection must not be empty",
        ));
    }

    let started = Instant::now();
    for row in &rows {
        let key = row.key.trim();
        if key.is_empty() {
            return Err(PlatformError::new(
                "PLATFORM_SEKEJAP_INSERT_INVALID",
                "row key must not be empty",
            ));
        }
    }

    let db_arc = get_db(data_root, owner, project)?;
    let mut db = db_arc.write().unwrap();
    let schema = db.table_schema(collection).cloned();
    let schema_fields = schema
        .as_ref()
        .map(|schema| {
            schema
                .fields
                .iter()
                .map(|field| (field.name.clone(), field.ty.clone()))
                .collect::<BTreeMap<_, _>>()
        })
        .unwrap_or_default();
    let mut field_dimensions = BTreeMap::<String, usize>::new();
    let mut prepared = Vec::with_capacity(rows.len());
    for row in rows {
        let key = row.key.trim().to_string();
        let slug = format!("{collection}/{key}");
        let exists = db.contains(&slug);
        match mode {
            StructuredWriteMode::Insert if exists => {
                return Err(PlatformError::new(
                    "PLATFORM_SEKEJAP_INSERT_EXISTS",
                    format!("row '{slug}' already exists"),
                ));
            }
            StructuredWriteMode::Update if !exists => {
                return Err(PlatformError::new(
                    "PLATFORM_SEKEJAP_INSERT_MISSING",
                    format!("row '{slug}' does not exist"),
                ));
            }
            _ => {}
        }

        let mut payload = if mode == StructuredWriteMode::Merge && exists {
            db.get(&slug)
                .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
                .and_then(|value| value.as_object().cloned())
                .unwrap_or_default()
        } else {
            Map::new()
        };
        let mut native_values = BTreeMap::<String, Vec<f32>>::new();
        for (field, value) in row.fields {
            if matches!(field.as_str(), "_id" | "_key" | "_collection") {
                continue;
            }
            let Some(field_type) = schema_fields.get(&field) else {
                if schema.is_some() {
                    return Err(PlatformError::new(
                        "PLATFORM_SEKEJAP_INSERT_UNKNOWN_FIELD",
                        format!("field '{field}' is not declared in collection '{collection}'"),
                    ));
                }
                payload.insert(field, value);
                continue;
            };
            match classify_schema_value(collection, &field, field_type, value)? {
                SchemaWriteValue::Payload(value) => {
                    payload.insert(field, value);
                }
                SchemaWriteValue::NativeVector(vector) => {
                    let dimensions = vector.len();
                    match field_dimensions.get(&field) {
                        Some(expected) if *expected != dimensions => {
                            return Err(PlatformError::new(
                                "PLATFORM_SEKEJAP_INSERT_INVALID",
                                format!(
                                    "field '{field}' dimensions mismatch: expected {expected}, got {dimensions}"
                                ),
                            ));
                        }
                        Some(_) => {}
                        None => {
                            field_dimensions.insert(field.clone(), dimensions);
                        }
                    }
                    payload.remove(&field);
                    native_values.insert(field, vector);
                }
            }
        }
        payload.insert(
            "_collection".to_string(),
            Value::String(collection.to_string()),
        );
        payload.insert("_key".to_string(), Value::String(key));
        payload.insert("_id".to_string(), Value::String(slug.clone()));
        prepared.push((slug, Value::Object(payload), native_values));
    }

    let affected_rows = prepared.len();
    let mut payload_rows = Vec::with_capacity(prepared.len());
    let mut vector_rows = Vec::new();
    for (slug, payload, native_values) in prepared {
        payload_rows.push((slug.clone(), payload));
        for (field, vector) in native_values {
            vector_rows.push((slug.clone(), field, vector));
        }
    }
    db.put_value_bulk(payload_rows)
        .map_err(|err| PlatformError::new("PLATFORM_SEKEJAP_INSERT_FAILED", err.to_string()))?;
    for (slug, field, vector) in vector_rows {
        db.put_vector(&slug, &field, &vector)
            .map_err(|err| PlatformError::new("PLATFORM_SEKEJAP_INSERT_FAILED", err.to_string()))?;
    }
    drop(db);
    record_project_write(data_root, owner, project, affected_rows);

    Ok(StructuredWritePayload {
        affected_rows,
        optimized_fields: field_dimensions.keys().cloned().collect(),
        field_dimensions,
        duration_ms: started.elapsed().as_millis() as u64,
    })
}

pub fn bulk_insert(
    data_root: &Path,
    owner: &str,
    project: &str,
    collection: &str,
    rows: Vec<StructuredInsertRecord>,
    edges: Vec<StructuredInsertEdge>,
    mode: StructuredWriteMode,
) -> Result<StructuredWritePayload, PlatformError> {
    let started = Instant::now();
    let mut payload = bulk_write_records(data_root, owner, project, collection, rows, mode)?;
    if edges.is_empty() {
        payload.duration_ms = started.elapsed().as_millis() as u64;
        return Ok(payload);
    }

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

    let db_arc = get_db(data_root, owner, project)?;
    let mut db = db_arc.write().unwrap();
    let edge_count = edges.len();
    let mut prepared_edges = Vec::with_capacity(edge_count);
    for edge in edges {
        let from_slug = format!("{}/{}", edge.from_target.trim(), edge.from_key.trim());
        let to_slug = format!("{}/{}", edge.to_target.trim(), edge.to_key.trim());
        let edge_type = edge.edge_type.trim().to_string();
        let meta_json = Value::Object(edge.fields).to_string();
        prepared_edges.push((from_slug, to_slug, edge_type, meta_json));
    }
    db.link_meta_many(prepared_edges.iter().map(|(from, to, edge_type, meta)| {
        (
            from.as_str(),
            to.as_str(),
            edge_type.as_str(),
            Some(meta.as_str()),
        )
    }))
    .map_err(|err| PlatformError::new("PLATFORM_SEKEJAP_INSERT_EDGE", err.to_string()))?;
    drop(db);
    record_project_write(data_root, owner, project, edge_count);

    payload.affected_rows += edge_count;
    payload.duration_ms = started.elapsed().as_millis() as u64;
    Ok(payload)
}

enum SchemaWriteValue {
    Payload(Value),
    NativeVector(Vec<f32>),
}

fn classify_schema_value(
    collection: &str,
    field: &str,
    field_type: &sekejap::FieldType,
    value: Value,
) -> Result<SchemaWriteValue, PlatformError> {
    if value.is_null() {
        return Ok(SchemaWriteValue::Payload(Value::Null));
    }
    match field_type {
        sekejap::FieldType::Text => {
            if value.is_string() {
                Ok(SchemaWriteValue::Payload(value))
            } else {
                Err(schema_type_error(collection, field, "TEXT", &value))
            }
        }
        sekejap::FieldType::Integer => {
            if value.as_i64().is_some() || value.as_u64().is_some() {
                Ok(SchemaWriteValue::Payload(value))
            } else {
                Err(schema_type_error(collection, field, "INTEGER", &value))
            }
        }
        sekejap::FieldType::Real => {
            if value.as_f64().is_some() {
                Ok(SchemaWriteValue::Payload(value))
            } else {
                Err(schema_type_error(collection, field, "REAL", &value))
            }
        }
        sekejap::FieldType::Bool => {
            if value.is_boolean() {
                Ok(SchemaWriteValue::Payload(value))
            } else {
                Err(schema_type_error(collection, field, "BOOLEAN", &value))
            }
        }
        sekejap::FieldType::Timestamptz => {
            if value.is_string() || value.as_f64().is_some() {
                Ok(SchemaWriteValue::Payload(value))
            } else {
                Err(schema_type_error(collection, field, "TIMESTAMPTZ", &value))
            }
        }
        sekejap::FieldType::Geo => {
            if value.is_object() || value.is_array() || value.is_string() {
                Ok(SchemaWriteValue::Payload(value))
            } else {
                Err(schema_type_error(collection, field, "GEO", &value))
            }
        }
        sekejap::FieldType::Vector => {
            let vector = value_as_f32_vec(&value)
                .ok_or_else(|| schema_type_error(collection, field, "VECTOR", &value))?;
            Ok(SchemaWriteValue::NativeVector(vector))
        }
        sekejap::FieldType::Json => Ok(SchemaWriteValue::Payload(value)),
    }
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
