//! Shared FileRef helpers for file-like content moving through pipelines.
//!
//! For the general node authoring rules, read `src/pipeline/nodes/mod.rs`; for the
//! built-in node registry and cross-node conventions, read
//! `src/pipeline/nodes/basic/mod.rs`.
//!
//! FileRef is the pipeline IR for bytes that should not be copied into JSON as
//! base64. ZebFS is storage; FileRef is the transport handle passed between
//! nodes. Producers write bytes to a storage backend and pass a small metadata
//! object downstream. Consumers validate `backend`, read bytes through this
//! module, and avoid assuming the backend is always a local file path.
//!
//! Current implementation:
//!
//! - `backend: "zebfs"` is implemented.
//! - `lifecycle: "temporary"` is used for webhook and HTTP ingress/intermediate
//!   bytes under `tmp/runs/{request_id}/files/...`.
//! - `lifecycle: "durable"` is used when a node writes a final project FS object,
//!   for example thumbnails.
//!
//! Future backends such as S3/R2 should add backend-specific read/write helpers
//! here so node handlers keep depending on FileRef IR instead of storage details.

use std::sync::Arc;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::pipeline::PipelineError;
use crate::platform::services::PlatformService;
use crate::zebfs::LocalZebFs;

pub const FILE_REF_TYPE: &str = "file_ref";
pub const BACKEND_ZEBFS: &str = "zebfs";
pub const LIFECYCLE_TEMPORARY: &str = "temporary";
pub const LIFECYCLE_DURABLE: &str = "durable";

#[derive(Debug, Clone)]
pub struct FileRefInput<'a> {
    pub owner: &'a str,
    pub project: &'a str,
    pub request_id: &'a str,
    pub bytes: &'a [u8],
    pub filename: Option<&'a str>,
    pub mime: Option<&'a str>,
    pub origin: &'a str,
    pub trust: &'a str,
}

pub fn is_file_ref(value: &Value) -> bool {
    value
        .get("__zf_type")
        .and_then(Value::as_str)
        .is_some_and(|kind| kind == FILE_REF_TYPE)
        || value
            .get("ref")
            .and_then(Value::as_str)
            .is_some_and(|path| !path.trim().is_empty())
            && value.get("sha256").is_some()
        || is_legacy_zebfs_file_ref(value)
}

pub fn file_ref_path(value: &Value) -> Option<&str> {
    if !is_file_ref(value) {
        return None;
    }
    value
        .get("ref")
        .and_then(Value::as_str)
        .or_else(|| value.get("path").and_then(Value::as_str))
        .map(str::trim)
        .filter(|path| !path.is_empty())
}

fn is_legacy_zebfs_file_ref(value: &Value) -> bool {
    let Some(path) = value.get("path").and_then(Value::as_str).map(str::trim) else {
        return false;
    };
    if path.is_empty() {
        return false;
    }
    if value.get("size").and_then(Value::as_u64).is_none() {
        return false;
    }
    let backend = value
        .get("backend")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(BACKEND_ZEBFS);
    if backend != BACKEND_ZEBFS {
        return false;
    }
    value
        .get("url")
        .and_then(Value::as_str)
        .map(str::trim)
        .is_some_and(|url| url.starts_with("/fs/"))
}

pub fn file_ref_backend(value: &Value) -> &str {
    value
        .get("backend")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or(BACKEND_ZEBFS)
}

pub fn file_ref_lifecycle(value: &Value) -> Option<&str> {
    value
        .get("lifecycle")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

pub fn write_tmp_file_ref(
    platform: &Arc<PlatformService>,
    input: FileRefInput<'_>,
) -> Result<Value, PipelineError> {
    let layout = platform
        .file
        .ensure_project_layout(input.owner, input.project)
        .map_err(|err| PipelineError::new("FW_FILE_REF", err.to_string()))?;
    let zebfs = LocalZebFs::new(layout.files_dir);
    let clean_name = sanitize_filename(input.filename.unwrap_or("content.bin"));
    let extension = extension_for(&clean_name, input.mime);
    let object_name = if extension.is_empty() {
        Uuid::new_v4().to_string()
    } else {
        format!("{}.{}", Uuid::new_v4(), extension)
    };
    let request_part = sanitize_path_part(if input.request_id.trim().is_empty() {
        "run"
    } else {
        input.request_id
    });
    let rel_path = format!("tmp/runs/{request_part}/files/{object_name}");
    let stat = zebfs
        .put(&rel_path, input.bytes)
        .map_err(|err| PipelineError::new("FW_FILE_REF_WRITE", err.to_string()))?;
    let sha256 = format!("sha256:{:x}", Sha256::digest(input.bytes));
    let mime = input.mime.unwrap_or("application/octet-stream");
    let kind = infer_kind(mime, &clean_name);
    Ok(json!({
        "__zf_type": FILE_REF_TYPE,
        "backend": BACKEND_ZEBFS,
        "ref": stat.path,
        "path": stat.path,
        "url": format!("/fs/{}/{}/{}", input.owner, input.project, stat.path),
        "filename": clean_name,
        "name": clean_name,
        "mime": mime,
        "content_type": mime,
        "size": input.bytes.len(),
        "sha256": sha256,
        "kind": kind,
        "lifecycle": LIFECYCLE_TEMPORARY,
        "origin": input.origin,
        "trust": input.trust,
    }))
}

pub fn read_file_ref_bytes(
    platform: &Arc<PlatformService>,
    owner: &str,
    project: &str,
    value: &Value,
) -> Result<Vec<u8>, PipelineError> {
    let path = file_ref_path(value).ok_or_else(|| {
        PipelineError::new("FW_FILE_REF_READ", "value is not a FileRef with a ref/path")
    })?;
    let backend = file_ref_backend(value);
    if backend != BACKEND_ZEBFS {
        return Err(PipelineError::new(
            "FW_FILE_REF_BACKEND",
            format!("unsupported FileRef backend '{backend}'"),
        ));
    }
    let layout = platform
        .file
        .ensure_project_layout(owner, project)
        .map_err(|err| PipelineError::new("FW_FILE_REF_READ", err.to_string()))?;
    let zebfs = LocalZebFs::new(layout.files_dir);
    let object = zebfs
        .get(path)
        .map_err(|err| PipelineError::new("FW_FILE_REF_READ", err.to_string()))?;
    Ok(object.bytes)
}

pub fn file_ref_to_rel_path(value: &Value) -> Option<String> {
    file_ref_path(value).map(ToString::to_string)
}

pub fn file_ref_to_rel_path_or_string(value: &Value) -> Option<String> {
    file_ref_to_rel_path(value).or_else(|| {
        value
            .as_str()
            .map(str::trim)
            .filter(|path| !path.is_empty())
            .map(ToString::to_string)
    })
}

fn sanitize_filename(raw: &str) -> String {
    let name = raw.rsplit(['/', '\\']).next().unwrap_or(raw).trim();
    let mut out = String::new();
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '.' | '-' | '_') {
            out.push(ch);
        } else if ch.is_whitespace() {
            out.push('-');
        }
    }
    let trimmed = out.trim_matches(['.', '-']).to_string();
    if trimmed.is_empty() {
        "content.bin".to_string()
    } else {
        trimmed
    }
}

fn sanitize_path_part(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
            out.push(ch);
        } else {
            out.push('-');
        }
    }
    out.trim_matches('-').to_string()
}

fn extension_for(filename: &str, mime: Option<&str>) -> String {
    if let Some(ext) = filename
        .rsplit_once('.')
        .map(|(_, ext)| ext.trim().to_ascii_lowercase())
        .filter(|ext| !ext.is_empty() && ext.len() <= 12)
    {
        return ext;
    }
    match mime.unwrap_or("").split(';').next().unwrap_or("").trim() {
        "application/json" => "json",
        "application/geo+json" => "geojson",
        "text/csv" => "csv",
        "text/plain" => "txt",
        "image/jpeg" => "jpg",
        "image/png" => "png",
        "image/webp" => "webp",
        "image/gif" => "gif",
        "application/pdf" => "pdf",
        "application/zip" => "zip",
        "application/vnd.apache.parquet" => "parquet",
        _ => "bin",
    }
    .to_string()
}

fn infer_kind(mime: &str, filename: &str) -> &'static str {
    let mime = mime.split(';').next().unwrap_or("").trim();
    let ext = filename
        .rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase())
        .unwrap_or_default();
    match (mime, ext.as_str()) {
        ("application/geo+json", _) | (_, "geojson") => "geojson",
        ("application/json", _) | (_, "json") => "json",
        ("text/csv", _) | (_, "csv") => "csv",
        ("image/jpeg" | "image/png" | "image/webp" | "image/gif", _) => "image",
        ("application/pdf", _) | (_, "pdf") => "pdf",
        ("application/zip", _) | (_, "zip") => "archive",
        ("application/vnd.apache.parquet", _) | (_, "parquet") => "parquet",
        _ => "binary",
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{file_ref_path, file_ref_to_rel_path_or_string, is_file_ref};

    #[test]
    fn detects_file_ref_shape() {
        let value = json!({
            "__zf_type": "file_ref",
            "backend": "zebfs",
            "ref": "tmp/runs/r/files/a.bin",
            "lifecycle": "temporary",
            "sha256": "sha256:abc"
        });

        assert!(is_file_ref(&value));
        assert_eq!(file_ref_path(&value), Some("tmp/runs/r/files/a.bin"));
    }

    #[test]
    fn detects_legacy_zebfs_file_metadata_as_file_ref() {
        let value = json!({
            "path": "tmp/e1-agent-upload-file-ref-smoke.csv",
            "url": "/fs/superadmin/default/tmp/e1-agent-upload-file-ref-smoke.csv",
            "size": 228,
            "kind": "object",
            "content_type": "text/csv",
        });

        assert!(is_file_ref(&value));
        assert_eq!(
            file_ref_path(&value),
            Some("tmp/e1-agent-upload-file-ref-smoke.csv")
        );
    }

    #[test]
    fn does_not_treat_arbitrary_path_object_as_file_ref() {
        let value = json!({
            "path": "tmp/e1-agent-upload-file-ref-smoke.csv",
            "size": 228,
        });

        assert!(!is_file_ref(&value));
        assert_eq!(file_ref_path(&value), None);
    }

    #[test]
    fn resolves_file_ref_or_plain_path_for_path_only_nodes() {
        let file_ref = json!({
            "__zf_type": "file_ref",
            "backend": "zebfs",
            "ref": "tmp/runs/r/files/a.csv",
            "sha256": "sha256:abc"
        });
        assert_eq!(
            file_ref_to_rel_path_or_string(&file_ref),
            Some("tmp/runs/r/files/a.csv".to_string())
        );
        assert_eq!(
            file_ref_to_rel_path_or_string(&json!("uploads/a.csv")),
            Some("uploads/a.csv".to_string())
        );
        assert_eq!(
            file_ref_to_rel_path_or_string(&json!({ "path": "x" })),
            None
        );
    }
}
