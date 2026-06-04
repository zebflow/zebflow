use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, RwLock};
use std::time::Instant;

use serde_json::{Map, Value, json};

use crate::platform::error::PlatformError;
use crate::platform::model::{
    CollectionAttribute, CreateSimpleTableRequest, DbObjectNode, DbQueryColumn,
    ProjectDbConnectionQueryResult, QueryProjectDbConnectionRequest, SimpleTableDefinition, now_ts,
    slug_segment,
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

fn get_db(data_root: &Path, owner: &str, project: &str) -> Result<Arc<RwLock<sekejap::CoreDB>>, PlatformError> {
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
        "number" => "real",
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
        "number" => "REAL",
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
    for attr in &req.attributes {
        let name = slug_segment(&attr.name);
        if name.is_empty() || name == "_key" || !seen.insert(name.clone()) {
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
    let mut columns = vec!["_key TEXT PRIMARY KEY".to_string()];
    for attr in &def.attributes {
        columns.push(format!("{} {}", attr.name, map_field_type(&attr.kind)));
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

fn merge_catalog_with_live(
    db: &sekejap::CoreDB,
    mut defs: Vec<SimpleTableDefinition>,
) -> Vec<SimpleTableDefinition> {
    let mut by_table = BTreeMap::new();
    for mut def in defs.drain(..) {
        def.row_count = row_count_for_collection(db, &def.collection);
        by_table.insert(def.table.clone(), def);
    }

    for collection in db.collection_names() {
        let table = slug_segment(&collection);
        if table.is_empty() || by_table.contains_key(&table) {
            continue;
        }
        let collection_name = collection.clone();
        by_table.insert(
            table.clone(),
            SimpleTableDefinition {
                table: table.clone(),
                title: table.clone(),
                collection,
                attributes: Vec::new(),
                hash_indexed_fields: vec!["_key".to_string()],
                range_indexed_fields: Vec::new(),
                fulltext_fields: Vec::new(),
                vector_fields: Vec::new(),
                spatial_fields: Vec::new(),
                row_count: row_count_for_collection(db, &collection_name),
                created_at: 0,
                updated_at: 0,
            },
        );
    }

    by_table.into_values().collect()
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
    Ok(created)
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
    let rest = sql.trim_start().strip_prefix("EXPLAIN").unwrap_or(sql).trim_start();
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
                title: None,
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
