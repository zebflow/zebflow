use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::time::Instant;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};

use crate::platform::error::PlatformError;
use crate::platform::model::{
    CollectionAttribute, CreateSimpleTableRequest, DbObjectNode, DbQueryColumn,
    ProjectDbConnectionQueryResult, QueryProjectDbConnectionRequest, SimpleTableDefinition,
    UpdateSimpleTableRequest, now_ts, slug_segment,
};

// ── CoreDB connection pool ───────────────────────────────────────────────────
//
// Keyed by canonical directory path.  `RwLock<CoreDB>` gives concurrent readers
// and exclusive writers.  The outer `Mutex` protects the pool map itself.

type DbPool = HashMap<PathBuf, Arc<RwLock<sekejap::CoreDB>>>;

static POOL: OnceLock<Mutex<DbPool>> = OnceLock::new();

fn pool() -> &'static Mutex<DbPool> {
    POOL.get_or_init(|| Mutex::new(HashMap::new()))
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

pub const BUILTIN_CONNECTION_SLUG: &str = "default-multimodel";
pub const BUILTIN_CONNECTION_LABEL: &str = "Default Multimodel Store";
pub const DB_KIND: &str = "sekejap";
pub const REPO_SCHEMA_VERSION: &str = "zebflow.sekejap.schema.v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SekejapTableSchemaExport {
    pub table: String,
    pub title: String,
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
pub struct SekejapSchemaExport {
    pub schema_version: String,
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

pub fn project_dir(data_root: &Path, owner: &str, project: &str) -> PathBuf {
    data_root
        .join("users")
        .join(slug_segment(owner))
        .join(slug_segment(project))
        .join("data")
        .join("sekejap")
}

fn catalog_path(data_root: &Path, owner: &str, project: &str) -> PathBuf {
    project_dir(data_root, owner, project).join("tables.json")
}

fn repo_dir(data_root: &Path, owner: &str, project: &str) -> PathBuf {
    data_root
        .join("users")
        .join(slug_segment(owner))
        .join(slug_segment(project))
        .join("repo")
}

fn repo_schema_dir(data_root: &Path, owner: &str, project: &str) -> PathBuf {
    repo_dir(data_root, owner, project)
        .join("schemas")
        .join("sekejap")
}

fn ensure_project_dir(
    data_root: &Path,
    owner: &str,
    project: &str,
) -> Result<PathBuf, PlatformError> {
    let dir = project_dir(data_root, owner, project);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn load_catalog(
    data_root: &Path,
    owner: &str,
    project: &str,
) -> Result<Vec<SimpleTableDefinition>, PlatformError> {
    let path = catalog_path(data_root, owner, project);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(&path)?;
    serde_json::from_str::<Vec<SimpleTableDefinition>>(&raw).map_err(|err| {
        PlatformError::new(
            "PLATFORM_SEKEJAP_CATALOG_READ",
            format!("failed to parse sekejap table catalog: {err}"),
        )
    })
}

fn save_catalog(
    data_root: &Path,
    owner: &str,
    project: &str,
    defs: &[SimpleTableDefinition],
) -> Result<(), PlatformError> {
    let path = catalog_path(data_root, owner, project);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let encoded = serde_json::to_string_pretty(defs).map_err(|err| {
        PlatformError::new(
            "PLATFORM_SEKEJAP_CATALOG_WRITE",
            format!("failed to encode sekejap table catalog: {err}"),
        )
    })?;
    std::fs::write(path, encoded)?;
    Ok(())
}

fn map_declared_kind(kind: &str) -> &'static str {
    match kind {
        "number" | "real" | "integer" => "number",
        "text" => "text",
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
        "text" => "TEXT",
        "boolean" => "JSON",
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
        let default_value = attr
            .default_value
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(ToString::to_string);
        attrs.push(CollectionAttribute {
            name,
            kind,
            index_types,
            default_value,
        });
    }
    if !has_user_key {
        attrs.insert(
            0,
            CollectionAttribute {
                name: "_key".to_string(),
                kind: "string".to_string(),
                index_types: Vec::new(),
                default_value: Some("UUIDV4()".to_string()),
            },
        );
    }

    let hash_indexed_fields = collect_index_fields(&attrs, &req.hash_indexed_fields, "hash");
    let range_indexed_fields = collect_index_fields(&attrs, &req.range_indexed_fields, "range");
    let fulltext_fields = collect_index_fields(&attrs, &[], "fulltext");
    let vector_fields = collect_index_fields(&attrs, &[], "vector");
    let spatial_fields = collect_index_fields(&attrs, &[], "spatial");
    let now = now_ts();

    Ok(SimpleTableDefinition {
        table: table.clone(),
        title: req
            .title
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(&table)
            .to_string(),
        collection: table,
        attributes: attrs,
        hash_indexed_fields,
        range_indexed_fields,
        fulltext_fields,
        vector_fields,
        spatial_fields,
        row_count: 0,
        created_at: now,
        updated_at: now,
    })
}

fn build_create_table_sql(def: &SimpleTableDefinition) -> String {
    let mut columns = Vec::new();
    for attr in &def.attributes {
        let mut col = if attr.name == "_key" {
            format!("_key {} PRIMARY KEY", map_field_type(&attr.kind))
        } else {
            format!("{} {}", attr.name, map_field_type(&attr.kind))
        };
        if let Some(ref dv) = attr.default_value {
            col.push_str(&format!(" DEFAULT {dv}"));
        }
        columns.push(col);
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
            default_value: None,
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
                default_value: None,
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

fn merge_catalog_with_live(
    db: &sekejap::CoreDB,
    mut defs: Vec<SimpleTableDefinition>,
) -> Vec<SimpleTableDefinition> {
    let mut by_table = BTreeMap::new();
    for mut def in defs.drain(..) {
        def.row_count = row_count_for_collection(db, &def.collection);
        if def.attributes.is_empty() {
            let (attrs, hash, range, fulltext, vector, spatial) =
                backfill_from_schema(db, &def.collection);
            def.attributes = attrs;
            def.hash_indexed_fields = hash;
            def.range_indexed_fields = range;
            def.fulltext_fields = fulltext;
            def.vector_fields = vector;
            def.spatial_fields = spatial;
        }
        for attr in &mut def.attributes {
            attr.kind = map_declared_kind(&attr.kind).to_string();
        }
        by_table.insert(def.table.clone(), def);
    }

    for collection in db.collection_names() {
        let table = slug_segment(&collection);
        if table.is_empty() || by_table.contains_key(&table) {
            continue;
        }
        let collection_name = collection.clone();
        let (attrs, hash, range, fulltext, vector, spatial) = backfill_from_schema(db, &collection);
        by_table.insert(
            table.clone(),
            SimpleTableDefinition {
                table: table.clone(),
                title: table.clone(),
                collection,
                attributes: attrs,
                hash_indexed_fields: hash,
                range_indexed_fields: range,
                fulltext_fields: fulltext,
                vector_fields: vector,
                spatial_fields: spatial,
                row_count: row_count_for_collection(db, &collection_name),
                created_at: 0,
                updated_at: 0,
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
        title: def.title,
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
        schema_version: REPO_SCHEMA_VERSION.to_string(),
        database: DB_KIND.to_string(),
        connection_slug: BUILTIN_CONNECTION_SLUG.to_string(),
        tables,
    })
}

fn export_tables_from_defs(defs: Vec<SimpleTableDefinition>) -> Vec<SekejapTableSchemaExport> {
    let mut tables = defs
        .into_iter()
        .map(export_table_schema)
        .collect::<Vec<_>>();
    tables.sort_by(|a, b| a.table.cmp(&b.table));
    tables
}

fn sync_catalog_with_live(
    data_root: &Path,
    owner: &str,
    project: &str,
) -> Result<Vec<SimpleTableDefinition>, PlatformError> {
    let db_arc = get_db(data_root, owner, project)?;
    let db = db_arc.read().unwrap();
    let defs = load_catalog(data_root, owner, project)?;
    let merged = merge_catalog_with_live(&db, defs);
    drop(db);
    save_catalog(data_root, owner, project, &merged)?;
    Ok(merged)
}

fn write_json_if_changed(path: &Path, value: &Value) -> Result<bool, PlatformError> {
    let encoded = serde_json::to_string_pretty(value).map_err(|err| {
        PlatformError::new(
            "PLATFORM_SEKEJAP_SCHEMA_EXPORT",
            format!("failed to encode schema JSON: {err}"),
        )
    })? + "\n";
    if path.exists() {
        let existing = std::fs::read_to_string(path)?;
        if existing == encoded {
            return Ok(false);
        }
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, encoded)?;
    Ok(true)
}

pub fn sync_schema_to_repo(
    data_root: &Path,
    owner: &str,
    project: &str,
) -> Result<SekejapSchemaSyncReport, PlatformError> {
    let defs = sync_catalog_with_live(data_root, owner, project)?;
    let export = SekejapSchemaExport {
        schema_version: REPO_SCHEMA_VERSION.to_string(),
        database: DB_KIND.to_string(),
        connection_slug: BUILTIN_CONNECTION_SLUG.to_string(),
        tables: export_tables_from_defs(defs),
    };
    let schema_root = repo_schema_dir(data_root, owner, project);
    let tables_dir = schema_root.join("tables");
    std::fs::create_dir_all(&tables_dir)?;

    let mut changed = false;
    let mut files_written = Vec::new();
    let mut files_removed = Vec::new();

    let schema_path = schema_root.join("schema.json");
    let schema_value = serde_json::to_value(&export).map_err(|err| {
        PlatformError::new(
            "PLATFORM_SEKEJAP_SCHEMA_EXPORT",
            format!("failed to serialise schema export: {err}"),
        )
    })?;
    if write_json_if_changed(&schema_path, &schema_value)? {
        changed = true;
    }
    files_written.push("schemas/sekejap/schema.json".to_string());

    let mut expected = BTreeSet::new();
    for table in &export.tables {
        let file_name = format!("{}.json", slug_segment(&table.table));
        expected.insert(file_name.clone());
        let table_path = tables_dir.join(&file_name);
        let table_value = serde_json::to_value(table).map_err(|err| {
            PlatformError::new(
                "PLATFORM_SEKEJAP_SCHEMA_EXPORT",
                format!("failed to serialise table schema: {err}"),
            )
        })?;
        if write_json_if_changed(&table_path, &table_value)? {
            changed = true;
        }
        files_written.push(format!("schemas/sekejap/tables/{file_name}"));
    }

    if tables_dir.exists() {
        for entry in std::fs::read_dir(&tables_dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() || path.extension().and_then(|v| v.to_str()) != Some("json") {
                continue;
            }
            let Some(name) = path.file_name().and_then(|v| v.to_str()) else {
                continue;
            };
            if !expected.contains(name) {
                std::fs::remove_file(&path)?;
                changed = true;
                files_removed.push(format!("schemas/sekejap/tables/{name}"));
            }
        }
    }

    Ok(SekejapSchemaSyncReport {
        changed,
        root: "schemas/sekejap".to_string(),
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
    let defs = load_catalog(data_root, owner, project)?;
    Ok(merge_catalog_with_live(&db, defs))
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

    let mut defs = load_catalog(data_root, owner, project)?;
    defs.push(def.clone());
    defs.sort_by(|a, b| a.table.cmp(&b.table));
    save_catalog(data_root, owner, project, &defs)?;

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
    let mut defs = merge_catalog_with_live(&db, load_catalog(data_root, owner, project)?);
    let pos = defs.iter().position(|d| d.table == table_slug);
    let def = match pos {
        Some(i) => defs.remove(i),
        None => {
            return Err(PlatformError::new(
                "PLATFORM_SEKEJAP_TABLE_NOT_FOUND",
                format!("table '{}' not found", table_slug),
            ));
        }
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

    save_catalog(data_root, owner, project, &defs)?;
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

    let mut defs = load_catalog(data_root, owner, project)?;
    let pos = defs.iter().position(|d| d.table == table_slug);
    let idx = match pos {
        Some(i) => i,
        None => {
            return Err(PlatformError::new(
                "PLATFORM_SEKEJAP_TABLE_NOT_FOUND",
                format!("table '{}' not found", table_slug),
            ));
        }
    };

    let existing = &defs[idx];

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
        let default_value = attr
            .default_value
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .map(ToString::to_string);
        attrs.push(CollectionAttribute {
            name,
            kind,
            index_types,
            default_value,
        });
    }

    let hash_indexed_fields = collect_index_fields(&attrs, &req.hash_indexed_fields, "hash");
    let range_indexed_fields = collect_index_fields(&attrs, &req.range_indexed_fields, "range");
    let fulltext_fields = collect_index_fields(&attrs, &[], "fulltext");
    let vector_fields = collect_index_fields(&attrs, &[], "vector");
    let spatial_fields = collect_index_fields(&attrs, &[], "spatial");

    let title = req
        .title
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or(&existing.title)
        .to_string();

    let db_arc = get_db(data_root, owner, project)?;
    let mut db = db_arc.write().unwrap();

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
        title,
        collection: existing.collection.clone(),
        attributes: attrs,
        hash_indexed_fields,
        range_indexed_fields,
        fulltext_fields,
        vector_fields,
        spatial_fields,
        row_count,
        created_at: existing.created_at,
        updated_at: now_ts(),
    };

    defs[idx] = updated.clone();
    save_catalog(data_root, owner, project, &defs)?;
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
            "created_at": def.created_at,
            "updated_at": def.updated_at,
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
    Ok(def
        .attributes
        .into_iter()
        .map(|attr| DbObjectNode {
            kind: "column".to_string(),
            name: attr.name.clone(),
            schema: Some("default".to_string()),
            children: Vec::new(),
            meta: json!({
                "data_type": attr.kind,
                "index_types": attr.index_types,
            }),
        })
        .collect())
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SchemaStatementKind {
    CreateOrAlter,
    Drop,
}

fn schema_statement_table(sql: &str) -> Option<(SchemaStatementKind, String)> {
    let normalized = sql.trim_start().trim_end_matches(';');
    let parts = normalized.split_whitespace().collect::<Vec<_>>();
    if parts.len() < 3 {
        return None;
    }
    let first = parts[0].to_ascii_uppercase();
    let second = parts[1].to_ascii_uppercase();
    let (kind, mut idx) = match (first.as_str(), second.as_str()) {
        ("CREATE", "TABLE") | ("CREATE", "COLLECTION") => (SchemaStatementKind::CreateOrAlter, 2),
        ("ALTER", "TABLE") | ("ALTER", "COLLECTION") => (SchemaStatementKind::CreateOrAlter, 2),
        ("DROP", "TABLE") | ("DROP", "COLLECTION") => (SchemaStatementKind::Drop, 2),
        _ => return None,
    };
    if matches!(kind, SchemaStatementKind::CreateOrAlter)
        && parts
            .get(idx)
            .is_some_and(|part| part.eq_ignore_ascii_case("IF"))
    {
        idx += 3;
    }
    if matches!(kind, SchemaStatementKind::Drop)
        && parts
            .get(idx)
            .is_some_and(|part| part.eq_ignore_ascii_case("IF"))
    {
        idx += 2;
    }
    let raw = parts.get(idx)?;
    let name = raw
        .trim_matches(['`', '"', '\''])
        .trim_end_matches('(')
        .trim_end_matches(';');
    let table = slug_segment(name.rsplit('.').next().unwrap_or(name));
    if table.is_empty() {
        None
    } else {
        Some((kind, table))
    }
}

fn refresh_catalog_table_from_live(
    data_root: &Path,
    owner: &str,
    project: &str,
    db: &sekejap::CoreDB,
    table: &str,
) -> Result<(), PlatformError> {
    let table = slug_segment(table);
    if table.is_empty() {
        return Ok(());
    }
    let mut defs = load_catalog(data_root, owner, project)?;
    let existing = defs.iter().find(|def| def.table == table).cloned();
    defs.retain(|def| def.table != table);
    let collection = existing
        .as_ref()
        .map(|def| def.collection.clone())
        .unwrap_or_else(|| table.clone());
    let (attrs, hash, range, fulltext, vector, spatial) = backfill_from_schema(db, &collection);
    let now = now_ts();
    defs.push(SimpleTableDefinition {
        table: table.clone(),
        title: existing
            .as_ref()
            .map(|def| def.title.clone())
            .unwrap_or_else(|| table.clone()),
        collection: collection.clone(),
        attributes: attrs,
        hash_indexed_fields: hash,
        range_indexed_fields: range,
        fulltext_fields: fulltext,
        vector_fields: vector,
        spatial_fields: spatial,
        row_count: row_count_for_collection(db, &collection),
        created_at: existing.as_ref().map(|def| def.created_at).unwrap_or(now),
        updated_at: now,
    });
    defs.sort_by(|a, b| a.table.cmp(&b.table));
    save_catalog(data_root, owner, project, &defs)
}

fn remove_catalog_table(
    data_root: &Path,
    owner: &str,
    project: &str,
    table: &str,
) -> Result<(), PlatformError> {
    let table = slug_segment(table);
    if table.is_empty() {
        return Ok(());
    }
    let mut defs = load_catalog(data_root, owner, project)?;
    let before = defs.len();
    defs.retain(|def| def.table != table);
    if defs.len() != before {
        save_catalog(data_root, owner, project, &defs)?;
    }
    Ok(())
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
        if let Some((kind, table)) = schema_statement_table(trimmed) {
            match kind {
                SchemaStatementKind::CreateOrAlter => {
                    refresh_catalog_table_from_live(data_root, owner, project, &db, &table)?;
                }
                SchemaStatementKind::Drop => {
                    remove_catalog_table(data_root, owner, project, &table)?;
                }
            }
        }
        drop(db);
        if should_sync_schema {
            sync_schema_to_repo(data_root, owner, project)?;
        }
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
            title: Some("Posts".to_string()),
            attributes: vec![CollectionAttribute {
                name: "title".to_string(),
                kind: "string".to_string(),
                index_types: vec!["hash".to_string()],
                default_value: None,
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
                title: Some("Posts".to_string()),
                attributes: vec![CollectionAttribute {
                    name: "title".to_string(),
                    kind: "string".to_string(),
                    index_types: vec!["hash".to_string()],
                    default_value: None,
                }],
                hash_indexed_fields: Vec::new(),
                range_indexed_fields: Vec::new(),
            },
        )
        .expect("create table");

        let schema_path = tmp
            .path()
            .join("users/alice/demo/repo/schemas/sekejap/schema.json");
        let table_path = tmp
            .path()
            .join("users/alice/demo/repo/schemas/sekejap/tables/posts.json");
        assert!(schema_path.is_file());
        assert!(table_path.is_file());

        let schema: Value =
            serde_json::from_str(&std::fs::read_to_string(schema_path).expect("schema file"))
                .expect("schema json");
        assert_eq!(schema["schema_version"], REPO_SCHEMA_VERSION);
        assert_eq!(schema["tables"][0]["table"], "posts");
        assert!(schema["tables"][0].get("row_count").is_none());
        assert!(schema["tables"][0].get("updated_at").is_none());
    }

    #[test]
    fn sync_schema_removes_stale_table_files() {
        let tmp = tmp_root();
        create_table(
            tmp.path(),
            "alice",
            "demo",
            &CreateSimpleTableRequest {
                table: "posts".to_string(),
                title: None,
                attributes: Vec::new(),
                hash_indexed_fields: Vec::new(),
                range_indexed_fields: Vec::new(),
            },
        )
        .expect("create table");
        let stale_path = tmp
            .path()
            .join("users/alice/demo/repo/schemas/sekejap/tables/stale.json");
        std::fs::write(&stale_path, "{}").expect("stale file");

        let report = sync_schema_to_repo(tmp.path(), "alice", "demo").expect("sync schema");
        assert!(report.changed);
        assert!(!stale_path.exists());
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
            load_catalog(tmp.path(), "alice", "demo")
                .expect("catalog")
                .is_empty()
        );
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
        assert!(
            !load_catalog(tmp.path(), "alice", "demo")
                .expect("catalog after delete")
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
        assert_eq!(schema["tables"][0]["table"], "posts");
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
                title: None,
                attributes: vec![CollectionAttribute {
                    name: "title".to_string(),
                    kind: "string".to_string(),
                    index_types: Vec::new(),
                    default_value: None,
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
                title: None,
                attributes: vec![CollectionAttribute {
                    name: "title".to_string(),
                    kind: "string".to_string(),
                    index_types: Vec::new(),
                    default_value: None,
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
                title: None,
                attributes: vec![
                    CollectionAttribute {
                        name: "title".to_string(),
                        kind: "string".to_string(),
                        index_types: Vec::new(),
                        default_value: None,
                    },
                    CollectionAttribute {
                        name: "views".to_string(),
                        kind: "number".to_string(),
                        index_types: Vec::new(),
                        default_value: None,
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
                title: None,
                attributes: vec![CollectionAttribute {
                    name: "name".to_string(),
                    kind: "string".to_string(),
                    index_types: Vec::new(),
                    default_value: None,
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
                title: None,
                attributes: vec![CollectionAttribute {
                    name: "name".to_string(),
                    kind: "string".to_string(),
                    index_types: Vec::new(),
                    default_value: None,
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
                title: None,
                attributes: vec![CollectionAttribute {
                    name: "title".to_string(),
                    kind: "string".to_string(),
                    index_types: Vec::new(),
                    default_value: None,
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
}
