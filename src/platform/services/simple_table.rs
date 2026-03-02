//! Project Simple Table management service backed by project-local Sekejap.

use std::fs;
use std::sync::Arc;

use sekejap::SekejapDB;
use serde_json::{Value, json};

use crate::platform::adapters::file::FileAdapter;
use crate::platform::adapters::project_data::ProjectDataFactory;
use crate::platform::error::PlatformError;
use crate::platform::model::{
    CreateSimpleTableRequest, ProjectFileLayout, SimpleTableDefinition, SimpleTableQueryRequest,
    SimpleTableQueryResult, UpsertSimpleTableRowRequest, now_ts, slug_segment,
};

const SIMPLE_TABLE_META_COLLECTION: &str = "sjtable_meta";
const SIMPLE_TABLE_COLLECTION_PREFIX: &str = "sjtable__";
const SIMPLE_TABLE_QUERY_LIMIT_MAX: usize = 500;

/// Project-scoped Simple Table management and querying.
pub struct SimpleTableService {
    file: Arc<dyn FileAdapter>,
    project_data: Arc<dyn ProjectDataFactory>,
}

impl SimpleTableService {
    /// Creates the service.
    pub fn new(file: Arc<dyn FileAdapter>, project_data: Arc<dyn ProjectDataFactory>) -> Self {
        Self { file, project_data }
    }

    /// Lists all managed Simple Tables for one project.
    pub fn list_tables(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<Vec<SimpleTableDefinition>, PlatformError> {
        let layout = self.project_layout(owner, project)?;
        let db = self.open_db(&layout)?;
        let metas = self.query_meta_rows(&db)?;
        let mut tables = metas
            .into_iter()
            .filter_map(|row| simple_table_definition_from_meta(&db, &row).ok())
            .collect::<Vec<_>>();
        tables.sort_by(|a, b| a.table.cmp(&b.table));
        Ok(tables)
    }

    /// Creates one Simple Table definition and backing collection.
    pub fn create_table(
        &self,
        owner: &str,
        project: &str,
        req: &CreateSimpleTableRequest,
    ) -> Result<SimpleTableDefinition, PlatformError> {
        let layout = self.project_layout(owner, project)?;
        let db = self.open_db(&layout)?;

        let table = slug_segment(&req.table);
        if table.is_empty() {
            return Err(PlatformError::new(
                "PLATFORM_SIMPLE_TABLE_INVALID",
                "table name must not be empty",
            ));
        }
        if self.find_meta_row(&db, &table)?.is_some() {
            return Err(PlatformError::new(
                "PLATFORM_SIMPLE_TABLE_EXISTS",
                format!("simple table '{}' already exists", table),
            ));
        }

        let collection = simple_table_collection_name(&table);
        let hash_indexed_fields = normalize_field_list(&req.hash_indexed_fields);
        let range_indexed_fields = normalize_field_list(&req.range_indexed_fields);
        let schema = json!({
            "vector_fields": [],
            "spatial_fields": [],
            "fulltext_fields": [],
            "hash_indexed_fields": hash_indexed_fields,
            "range_indexed_fields": range_indexed_fields,
        });
        db.schema()
            .define(&collection, &schema.to_string())
            .map_err(|e| PlatformError::new("PLATFORM_SIMPLE_TABLE_DEFINE", e.to_string()))?;

        let now = now_ts();
        let meta = json!({
            "_id": format!("{}/{}", SIMPLE_TABLE_META_COLLECTION, table),
            "_collection": SIMPLE_TABLE_META_COLLECTION,
            "_key": table,
            "table": table,
            "title": req.title.as_deref().unwrap_or_default(),
            "collection": collection,
            "hash_indexed_fields": hash_indexed_fields,
            "range_indexed_fields": range_indexed_fields,
            "created_at": now,
            "updated_at": now,
        });
        db.nodes()
            .put_json(&meta.to_string())
            .map_err(|e| PlatformError::new("PLATFORM_SIMPLE_TABLE_META", e.to_string()))?;

        self.get_table(owner, project, &table)?.ok_or_else(|| {
            PlatformError::new(
                "PLATFORM_SIMPLE_TABLE_DEFINE",
                "table created but not readable",
            )
        })
    }

    /// Resolves one Simple Table definition.
    pub fn get_table(
        &self,
        owner: &str,
        project: &str,
        table: &str,
    ) -> Result<Option<SimpleTableDefinition>, PlatformError> {
        let layout = self.project_layout(owner, project)?;
        let db = self.open_db(&layout)?;
        let table = slug_segment(table);
        if table.is_empty() {
            return Ok(None);
        }
        let Some(row) = self.find_meta_row(&db, &table)? else {
            return Ok(None);
        };
        Ok(Some(simple_table_definition_from_meta(&db, &row)?))
    }

    /// Deletes one Simple Table metadata row and all stored records.
    pub fn delete_table(
        &self,
        owner: &str,
        project: &str,
        table: &str,
    ) -> Result<(), PlatformError> {
        let layout = self.project_layout(owner, project)?;
        let db = self.open_db(&layout)?;
        let table = slug_segment(table);
        let Some(def) = self.get_table(owner, project, &table)? else {
            return Err(PlatformError::new(
                "PLATFORM_SIMPLE_TABLE_MISSING",
                format!("simple table '{}' not found", table),
            ));
        };

        let rows =
            self.query_collection_rows(&db, &def.collection, SIMPLE_TABLE_QUERY_LIMIT_MAX)?;
        for row in rows {
            if let Some(slug) = row.get("_id").and_then(Value::as_str) {
                db.nodes().remove(slug).map_err(|e| {
                    PlatformError::new("PLATFORM_SIMPLE_TABLE_DELETE", e.to_string())
                })?;
            }
        }
        db.nodes()
            .remove(&format!("{}/{}", SIMPLE_TABLE_META_COLLECTION, table))
            .map_err(|e| PlatformError::new("PLATFORM_SIMPLE_TABLE_DELETE", e.to_string()))?;
        Ok(())
    }

    /// Upserts one row into a managed Simple Table.
    pub fn upsert_row(
        &self,
        owner: &str,
        project: &str,
        req: &UpsertSimpleTableRowRequest,
    ) -> Result<Value, PlatformError> {
        let layout = self.project_layout(owner, project)?;
        let db = self.open_db(&layout)?;
        let Some(table) = self.get_table(owner, project, &req.table)? else {
            return Err(PlatformError::new(
                "PLATFORM_SIMPLE_TABLE_MISSING",
                format!("simple table '{}' not found", slug_segment(&req.table)),
            ));
        };
        let row_id = slug_segment(&req.row_id);
        if row_id.is_empty() {
            return Err(PlatformError::new(
                "PLATFORM_SIMPLE_TABLE_INVALID",
                "row id must not be empty",
            ));
        }
        let document = compose_simple_table_row(&table.collection, &row_id, &req.data);
        db.nodes()
            .put_json(&document.to_string())
            .map_err(|e| PlatformError::new("PLATFORM_SIMPLE_TABLE_WRITE", e.to_string()))?;
        Ok(document)
    }

    /// Queries one Simple Table.
    pub fn query_rows(
        &self,
        owner: &str,
        project: &str,
        req: &SimpleTableQueryRequest,
    ) -> Result<SimpleTableQueryResult, PlatformError> {
        let layout = self.project_layout(owner, project)?;
        let db = self.open_db(&layout)?;
        let Some(table) = self.get_table(owner, project, &req.table)? else {
            return Err(PlatformError::new(
                "PLATFORM_SIMPLE_TABLE_MISSING",
                format!("simple table '{}' not found", slug_segment(&req.table)),
            ));
        };
        let mut pipeline = vec![json!({"op":"collection","name":table.collection})];
        if let Some(field) = req.where_field.as_deref() {
            let field = field.trim();
            if !field.is_empty() {
                let Some(value) = req.where_value.clone() else {
                    return Err(PlatformError::new(
                        "PLATFORM_SIMPLE_TABLE_QUERY",
                        "where_value is required when where_field is provided",
                    ));
                };
                pipeline.push(json!({"op":"where_eq","field":field,"value":value}));
            }
        }
        let limit = req.limit.clamp(1, SIMPLE_TABLE_QUERY_LIMIT_MAX);
        pipeline.push(json!({"op":"take","n":limit}));
        let rows = query_payload_rows(&db, pipeline)?;
        Ok(SimpleTableQueryResult { table, rows })
    }

    /// Executes one native Sekejap query payload against project DB and returns payload rows.
    ///
    /// Expected payload shape follows Sekejap query API, usually:
    /// `{ "pipeline": [ ...ops ] }`.
    pub fn query_native_rows(
        &self,
        owner: &str,
        project: &str,
        payload: &Value,
    ) -> Result<Vec<Value>, PlatformError> {
        let layout = self.project_layout(owner, project)?;
        let db = self.open_db(&layout)?;
        query_json_rows(&db, payload)
    }

    fn project_layout(
        &self,
        owner: &str,
        project: &str,
    ) -> Result<ProjectFileLayout, PlatformError> {
        let owner = slug_segment(owner);
        let project = slug_segment(project);
        let layout = self.file.ensure_project_layout(&owner, &project)?;
        self.project_data.initialize_project(&layout)?;
        Ok(layout)
    }

    fn open_db(&self, layout: &ProjectFileLayout) -> Result<SekejapDB, PlatformError> {
        fs::create_dir_all(&layout.data_sekejap_dir)?;
        SekejapDB::new(&layout.data_sekejap_dir, 500_000)
            .map_err(|e| PlatformError::new("PLATFORM_SIMPLE_TABLE_OPEN", e.to_string()))
    }

    fn query_meta_rows(&self, db: &SekejapDB) -> Result<Vec<Value>, PlatformError> {
        query_payload_rows(
            db,
            vec![
                json!({"op":"collection","name":SIMPLE_TABLE_META_COLLECTION}),
                json!({"op":"take","n":SIMPLE_TABLE_QUERY_LIMIT_MAX}),
            ],
        )
    }

    fn find_meta_row(&self, db: &SekejapDB, table: &str) -> Result<Option<Value>, PlatformError> {
        let mut rows = query_payload_rows(
            db,
            vec![
                json!({"op":"collection","name":SIMPLE_TABLE_META_COLLECTION}),
                json!({"op":"where_eq","field":"table","value":table}),
                json!({"op":"take","n":1}),
            ],
        )?;
        Ok(rows.pop())
    }

    fn query_collection_rows(
        &self,
        db: &SekejapDB,
        collection: &str,
        limit: usize,
    ) -> Result<Vec<Value>, PlatformError> {
        query_payload_rows(
            db,
            vec![
                json!({"op":"collection","name":collection}),
                json!({"op":"take","n":limit}),
            ],
        )
    }
}

fn query_payload_rows(db: &SekejapDB, pipeline: Vec<Value>) -> Result<Vec<Value>, PlatformError> {
    query_json_rows(db, &json!({ "pipeline": pipeline }))
}

fn query_json_rows(db: &SekejapDB, payload: &Value) -> Result<Vec<Value>, PlatformError> {
    let raw_payload = serde_json::to_string(payload)
        .map_err(|err| PlatformError::new("PLATFORM_SIMPLE_TABLE_QUERY", err.to_string()))?;
    let out = db
        .query(&raw_payload)
        .map_err(|e| PlatformError::new("PLATFORM_SIMPLE_TABLE_QUERY", e.to_string()))?;
    let mut rows = Vec::new();
    for hit in out.data {
        if let Some(payload) = hit.payload
            && let Ok(value) = serde_json::from_str::<Value>(&payload)
        {
            rows.push(value);
        }
    }
    Ok(rows)
}

fn simple_table_definition_from_meta(
    db: &SekejapDB,
    row: &Value,
) -> Result<SimpleTableDefinition, PlatformError> {
    let table = row
        .get("table")
        .and_then(Value::as_str)
        .ok_or_else(|| PlatformError::new("PLATFORM_SIMPLE_TABLE_META", "missing table field"))?
        .to_string();
    let collection = row
        .get("collection")
        .and_then(Value::as_str)
        .unwrap_or(&simple_table_collection_name(&table))
        .to_string();
    let describe = db.describe_collection(&collection);
    let row_count = describe.get("count").and_then(Value::as_u64).unwrap_or(0) as usize;
    Ok(SimpleTableDefinition {
        table: table.clone(),
        title: row
            .get("title")
            .and_then(Value::as_str)
            .filter(|v| !v.trim().is_empty())
            .unwrap_or(&table)
            .to_string(),
        collection,
        hash_indexed_fields: string_vec(row.get("hash_indexed_fields")),
        range_indexed_fields: string_vec(row.get("range_indexed_fields")),
        row_count,
        created_at: row.get("created_at").and_then(Value::as_i64).unwrap_or(0),
        updated_at: row.get("updated_at").and_then(Value::as_i64).unwrap_or(0),
    })
}

fn simple_table_collection_name(table: &str) -> String {
    format!("{SIMPLE_TABLE_COLLECTION_PREFIX}{table}")
}

fn normalize_field_list(input: &[String]) -> Vec<String> {
    let mut out = input
        .iter()
        .map(|field| slug_segment(field))
        .filter(|field| !field.is_empty())
        .collect::<Vec<_>>();
    out.sort();
    out.dedup();
    out
}

fn string_vec(value: Option<&Value>) -> Vec<String> {
    let Some(Value::Array(items)) = value else {
        return Vec::new();
    };
    items
        .iter()
        .filter_map(Value::as_str)
        .map(ToString::to_string)
        .collect()
}

fn compose_simple_table_row(collection: &str, row_id: &str, data: &Value) -> Value {
    let mut object = serde_json::Map::new();
    object.insert(
        "_id".to_string(),
        Value::String(format!("{collection}/{row_id}")),
    );
    object.insert(
        "_collection".to_string(),
        Value::String(collection.to_string()),
    );
    object.insert("_key".to_string(), Value::String(row_id.to_string()));
    object.insert("row_id".to_string(), Value::String(row_id.to_string()));

    match data {
        Value::Object(map) => {
            for (key, value) in map {
                object.insert(key.clone(), value.clone());
            }
        }
        other => {
            object.insert("value".to_string(), other.clone());
        }
    }

    Value::Object(object)
}
