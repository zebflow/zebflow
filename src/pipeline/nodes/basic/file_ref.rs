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
//! - `backend: "zebfs"` is implemented. The value is the *native* backend the
//!   project declares in `spec.files.backend` (see `src/zebfs/backend.rs`), not
//!   the place bytes were fetched from: a node that downloads from an external
//!   bucket still writes into the native store and still emits the native
//!   backend word.
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

pub const FILE_REF_TYPE: &str = "file_ref";
/// The native backend that owns locally-stored bytes.
///
/// Re-exported rather than redefined: the word a FileRef carries and the word
/// `spec.files.backend` declares name the same thing, so there is one constant.
pub use crate::zebfs::backend::BACKEND_ZEBFS;
pub const LIFECYCLE_TEMPORARY: &str = "temporary";
pub const LIFECYCLE_DURABLE: &str = "durable";

/// The frozen `kind` vocabulary (`kinds/file-ref/README.md`).
///
/// "A shorthand pipeline authors branch on, so the eight values are the
/// promise." A writer that emits a ninth breaks every author who matched the
/// eight exhaustively, so every producer here derives through [`infer_kind`]
/// and nothing else.
pub const FILE_REF_KINDS: [&str; 8] = [
    "geojson", "json", "csv", "image", "pdf", "archive", "parquet", "binary",
];

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
}

pub fn file_ref_path(value: &Value) -> Option<&str> {
    if !is_file_ref(value) {
        return None;
    }
    value
        .get("ref")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|path| !path.is_empty())
}

pub fn file_ref_backend(value: &Value) -> &str {
    value
        .get("backend")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("")
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
    let zebfs = layout.open_files();
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
        "filename": clean_name,
        "mime": mime,
        "kind": kind,
        "size": input.bytes.len(),
        "sha256": sha256,
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
    validate_file_ref(value)?;
    let path = file_ref_path(value).ok_or_else(|| {
        PipelineError::new("FW_FILE_REF_READ", "value is not a FileRef with a ref")
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
    let zebfs = layout.open_files();
    let object = zebfs
        .get(path)
        .map_err(|err| PipelineError::new("FW_FILE_REF_READ", err.to_string()))?;
    let expected_size = value["size"].as_u64().unwrap_or_default();
    if object.bytes.len() as u64 != expected_size {
        return Err(PipelineError::new(
            "FW_FILE_REF_INTEGRITY",
            format!(
                "FileRef size mismatch: expected {expected_size}, got {}",
                object.bytes.len()
            ),
        ));
    }
    let expected_sha256 = value["sha256"].as_str().unwrap_or_default();
    let actual_sha256 = format!("sha256:{:x}", Sha256::digest(&object.bytes));
    if !actual_sha256.eq_ignore_ascii_case(expected_sha256) {
        return Err(PipelineError::new(
            "FW_FILE_REF_INTEGRITY",
            format!("FileRef digest mismatch for '{path}'"),
        ));
    }
    Ok(object.bytes)
}

/// Performs the fixed, shallow validation required at a FileRef consumer.
///
/// This is intentionally not a full document-envelope decode. FileRef is a
/// runtime payload value and this check stays constant-time with respect to the
/// referenced file size.
pub fn validate_file_ref(value: &Value) -> Result<(), PipelineError> {
    if !is_file_ref(value) {
        return Err(PipelineError::new(
            "FW_FILE_REF_INVALID",
            "value is missing __zf_type=file_ref",
        ));
    }
    required_non_empty_string(value, "backend")?;
    required_non_empty_string(value, "ref")?;
    required_non_empty_string(value, "filename")?;
    required_non_empty_string(value, "mime")?;
    required_non_empty_string(value, "sha256")?;
    // `origin` and `trust` are required and open: the contract's Rejections
    // close only `kind` and `lifecycle`, and a writer naming a new ingress is
    // adding a word, not breaking the shape.
    required_non_empty_string(value, "origin")?;
    required_non_empty_string(value, "trust")?;
    let kind = required_non_empty_string(value, "kind")?;
    if !FILE_REF_KINDS.contains(&kind) {
        return Err(PipelineError::new(
            "FW_FILE_REF_INVALID",
            format!(
                "FileRef kind '{kind}' is not one of {}",
                FILE_REF_KINDS.join(", ")
            ),
        ));
    }
    let lifecycle = required_non_empty_string(value, "lifecycle")?;
    if !matches!(lifecycle, LIFECYCLE_TEMPORARY | LIFECYCLE_DURABLE) {
        return Err(PipelineError::new(
            "FW_FILE_REF_INVALID",
            "FileRef lifecycle must be temporary or durable",
        ));
    }
    if value.get("size").and_then(Value::as_u64).is_none() {
        return Err(PipelineError::new(
            "FW_FILE_REF_INVALID",
            "FileRef size must be an unsigned integer",
        ));
    }
    let digest = value["sha256"].as_str().unwrap_or_default();
    let Some(hex) = digest.strip_prefix("sha256:") else {
        return Err(PipelineError::new(
            "FW_FILE_REF_INVALID",
            "FileRef sha256 must use sha256:<hex>",
        ));
    };
    if hex.len() != 64 || !hex.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(PipelineError::new(
            "FW_FILE_REF_INVALID",
            "FileRef sha256 must contain exactly 64 hexadecimal digits",
        ));
    }
    Ok(())
}

fn required_non_empty_string<'a>(value: &'a Value, field: &str) -> Result<&'a str, PipelineError> {
    value
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| {
            PipelineError::new(
                "FW_FILE_REF_INVALID",
                format!("FileRef {field} must be a non-empty string"),
            )
        })
}

/// The one door from a FileRef to a path in the project's local ZebFS.
///
/// `ref` is opaque: "no node parses it, joins it to a path, or assumes a local
/// file" (`kinds/file-ref/README.md`). Every node that needs a local relative
/// path comes through here, so the assertion — this handle names bytes the
/// local store owns — is made once instead of once per node. The day a second
/// backend lands, this is the only function that has to learn about it, and
/// nothing silently joins a remote handle to the project's files directory.
///
/// `Ok(None)` means the value is not a FileRef at all; the caller decides
/// whether that is an error. A FileRef on another backend is always an error.
pub fn zebfs_rel_path(value: &Value) -> Result<Option<String>, PipelineError> {
    if !is_file_ref(value) {
        return Ok(None);
    }
    let backend = file_ref_backend(value);
    if backend != BACKEND_ZEBFS {
        return Err(PipelineError::new(
            "FW_FILE_REF_BACKEND",
            format!(
                "FileRef backend '{backend}' owns these bytes; its ref is opaque                  to this node and cannot be read as a local path"
            ),
        ));
    }
    Ok(file_ref_path(value).map(ToString::to_string))
}

/// [`zebfs_rel_path`] for the nodes that also accept a bare object path string.
///
/// A FileRef is resolved through the backend check; anything else is taken as
/// a literal path only when it is a plain string, never by reading a `path`
/// field off an object that merely looks file-shaped.
pub fn zebfs_rel_path_or_string(value: &Value) -> Result<Option<String>, PipelineError> {
    if is_file_ref(value) {
        return zebfs_rel_path(value);
    }
    Ok(value
        .as_str()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(ToString::to_string))
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

/// Derives the contract's `kind` from `mime` and the object name.
///
/// The only producer of the field, so every writer lands inside
/// [`FILE_REF_KINDS`]. Anything unrecognised is `binary`: `mime` already
/// carries the exact type, and widening the frozen vocabulary is a contract
/// change, not a writer's decision.
pub fn infer_kind(mime: &str, filename: &str) -> &'static str {
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

    use super::{
        FILE_REF_KINDS, file_ref_path, is_file_ref, validate_file_ref, zebfs_rel_path,
        zebfs_rel_path_or_string,
    };

    fn contract_file_ref() -> serde_json::Value {
        json!({
            "__zf_type": "file_ref",
            "backend": "zebfs",
            "ref": "tmp/runs/abc123/files/9f2c8d.jpg",
            "filename": "photo.jpg",
            "mime": "image/jpeg",
            "kind": "image",
            "size": 51234,
            "sha256": "sha256:e3b0c44298fc1c14a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718",
            "lifecycle": "temporary",
            "origin": "webhook",
            "trust": "untrusted"
        })
    }

    #[test]
    fn detects_file_ref_shape() {
        let value = contract_file_ref();
        assert!(is_file_ref(&value));
        validate_file_ref(&value).unwrap();
        assert_eq!(
            file_ref_path(&value),
            Some("tmp/runs/abc123/files/9f2c8d.jpg")
        );
    }

    /// `kinds/file-ref/README.md`: "All eleven fields are required."
    #[test]
    fn every_one_of_the_eleven_fields_is_required() {
        for field in [
            "__zf_type",
            "backend",
            "ref",
            "filename",
            "mime",
            "kind",
            "size",
            "sha256",
            "lifecycle",
            "origin",
            "trust",
        ] {
            let mut value = contract_file_ref();
            value.as_object_mut().unwrap().remove(field);
            assert!(
                validate_file_ref(&value).is_err(),
                "FileRef without '{field}' must be refused"
            );
        }
        validate_file_ref(&contract_file_ref()).unwrap();
    }

    /// "the eight values are the promise".
    #[test]
    fn kind_is_one_of_the_eight_contract_values() {
        for kind in FILE_REF_KINDS {
            let mut value = contract_file_ref();
            value["kind"] = json!(kind);
            validate_file_ref(&value).unwrap();
        }
        for kind in ["text", "object", "video", ""] {
            let mut value = contract_file_ref();
            value["kind"] = json!(kind);
            assert!(
                validate_file_ref(&value).is_err(),
                "FileRef kind '{kind}' is outside the contract's eight"
            );
        }
    }

    /// The four dropped fields are gone from every producer, so a consumer
    /// that still reads them fails loudly instead of drifting.
    #[test]
    fn producers_do_not_emit_the_dropped_legacy_fields() {
        let value = contract_file_ref();
        for field in ["path", "name", "content_type", "url"] {
            assert!(value.get(field).is_none());
        }
    }

    #[test]
    fn rejects_legacy_zebfs_file_metadata() {
        let value = json!({
            "path": "tmp/e1-agent-upload-file-ref-smoke.csv",
            "url": "/fs/superadmin/default/tmp/e1-agent-upload-file-ref-smoke.csv",
            "size": 228,
            "kind": "object",
            "content_type": "text/csv",
        });

        assert!(!is_file_ref(&value));
        assert_eq!(file_ref_path(&value), None);
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
        let file_ref = contract_file_ref();
        assert_eq!(
            zebfs_rel_path_or_string(&file_ref).unwrap(),
            Some("tmp/runs/abc123/files/9f2c8d.jpg".to_string())
        );
        assert_eq!(
            zebfs_rel_path_or_string(&json!("uploads/a.csv")).unwrap(),
            Some("uploads/a.csv".to_string())
        );
        assert_eq!(
            zebfs_rel_path_or_string(&json!({ "path": "x" })).unwrap(),
            None
        );
    }

    /// "`ref` … **opaque.** Only the named backend may interpret it — no node
    /// parses it, joins it to a path, or assumes a local file."
    #[test]
    fn a_foreign_backend_ref_is_never_resolved_to_a_local_path() {
        let mut value = contract_file_ref();
        value["backend"] = json!("s3");
        value["ref"] = json!("s3://bucket/key.jpg");

        assert_eq!(
            zebfs_rel_path(&value).unwrap_err().code,
            "FW_FILE_REF_BACKEND"
        );
        assert_eq!(
            zebfs_rel_path_or_string(&value).unwrap_err().code,
            "FW_FILE_REF_BACKEND"
        );
        // And it is emphatically not silently read as a bare string path.
        assert!(value.as_str().is_none());
    }
}
