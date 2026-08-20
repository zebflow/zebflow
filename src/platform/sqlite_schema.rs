//! Portable SQLite schema export/import for project bundles.
//!
//! Project bundles do not ship local SQLite data through Hub. When requested,
//! they carry the project's SQLite schema export, which is enough to
//! initialize a new project's `data/local.db` with the same
//! table/index/view/trigger structure. The directory it lives in is declared
//! by `spec.layout.sqlite_schema` rather than named here.

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::platform::error::PlatformError;
use crate::platform::model::{ResolvedProjectLayout, slug_segment};

fn project_repo_dir(data_root: &Path, owner: &str, project: &str) -> PathBuf {
    data_root
        .join("users")
        .join(slug_segment(owner))
        .join(slug_segment(project))
        .join("repo")
}

fn project_data_dir(data_root: &Path, owner: &str, project: &str) -> PathBuf {
    data_root
        .join("users")
        .join(slug_segment(owner))
        .join(slug_segment(project))
        .join("data")
}

pub fn local_db_path(data_root: &Path, owner: &str, project: &str) -> PathBuf {
    project_data_dir(data_root, owner, project).join("local.db")
}

pub fn repo_schema_path(
    data_root: &Path,
    owner: &str,
    project: &str,
    layout: &ResolvedProjectLayout,
) -> PathBuf {
    project_repo_dir(data_root, owner, project).join(layout.sqlite_schema_document_rel())
}

pub fn has_schema(data_root: &Path, owner: &str, project: &str) -> Result<bool, PlatformError> {
    let path = local_db_path(data_root, owner, project);
    if !path.is_file() {
        return Ok(false);
    }
    let conn = Connection::open(&path)
        .map_err(|err| PlatformError::new("SQLITE_SCHEMA_OPEN", err.to_string()))?;
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master \
             WHERE type IN ('table','index','view','trigger') \
             AND name NOT LIKE 'sqlite_%' \
             AND sql IS NOT NULL",
            [],
            |row| row.get(0),
        )
        .map_err(|err| PlatformError::new("SQLITE_SCHEMA_READ", err.to_string()))?;
    Ok(count > 0)
}

pub fn export_schema_sql(
    data_root: &Path,
    owner: &str,
    project: &str,
) -> Result<Option<String>, PlatformError> {
    if !has_schema(data_root, owner, project)? {
        return Ok(None);
    }
    let path = local_db_path(data_root, owner, project);
    let conn = Connection::open(&path)
        .map_err(|err| PlatformError::new("SQLITE_SCHEMA_OPEN", err.to_string()))?;
    let mut stmt = conn
        .prepare(
            "SELECT sql FROM sqlite_master \
             WHERE type IN ('table','index','view','trigger') \
             AND name NOT LIKE 'sqlite_%' \
             AND sql IS NOT NULL \
             ORDER BY CASE type \
                WHEN 'table' THEN 0 WHEN 'view' THEN 1 WHEN 'index' THEN 2 ELSE 3 END, name",
        )
        .map_err(|err| PlatformError::new("SQLITE_SCHEMA_READ", err.to_string()))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|err| PlatformError::new("SQLITE_SCHEMA_READ", err.to_string()))?;
    let mut statements = Vec::new();
    for row in rows {
        let sql = row.map_err(|err| PlatformError::new("SQLITE_SCHEMA_READ", err.to_string()))?;
        let sql = sql.trim().trim_end_matches(';');
        if !sql.is_empty() {
            statements.push(format!("{sql};"));
        }
    }
    if statements.is_empty() {
        Ok(None)
    } else {
        Ok(Some(format!(
            "-- Zebflow portable SQLite schema\n{}\n",
            statements.join("\n")
        )))
    }
}

pub fn apply_schema_from_repo(
    data_root: &Path,
    owner: &str,
    project: &str,
    layout: &ResolvedProjectLayout,
) -> Result<bool, PlatformError> {
    let schema_path = repo_schema_path(data_root, owner, project, layout);
    if !schema_path.is_file() {
        return Ok(false);
    }
    if has_schema(data_root, owner, project)? {
        return Ok(false);
    }
    let sql = std::fs::read_to_string(&schema_path)?;
    let db_path = local_db_path(data_root, owner, project);
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(&db_path)
        .map_err(|err| PlatformError::new("SQLITE_SCHEMA_OPEN", err.to_string()))?;
    conn.execute_batch(&sql)
        .map_err(|err| PlatformError::new("SQLITE_SCHEMA_APPLY", err.to_string()))?;
    Ok(true)
}

pub fn execute_sql(
    data_root: &Path,
    owner: &str,
    project: &str,
    sql: &str,
) -> Result<(), PlatformError> {
    let trimmed = sql.trim();
    if trimmed.is_empty() {
        return Ok(());
    }
    let db_path = local_db_path(data_root, owner, project);
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(&db_path)
        .map_err(|err| PlatformError::new("SQLITE_INITIAL_DATA_OPEN", err.to_string()))?;
    conn.execute_batch(trimmed)
        .map_err(|err| PlatformError::new("SQLITE_INITIAL_DATA_APPLY", err.to_string()))?;
    Ok(())
}
