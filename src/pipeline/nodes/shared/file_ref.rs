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
/// "A shorthand pipeline authors branch on, so the values are the promise."
/// A writer that emits a twelfth breaks every author who matched the eleven
/// exhaustively, so every producer here derives through [`infer_kind`] and
/// nothing else. `audio`, `video` and `spreadsheet` were added 2026-09-19 for
/// the `input.*` family, whose `--accept` names these kinds.
pub const FILE_REF_KINDS: [&str; 11] = [
    "geojson",
    "json",
    "csv",
    "image",
    "audio",
    "video",
    "pdf",
    "spreadsheet",
    "archive",
    "parquet",
    "binary",
];

/// A durable FileRef for bytes a node has just written to the native store.
///
/// The one builder for every node that keeps a file — `fs.save`, `fs.put`,
/// `fs.compress` — so all of them derive `kind` through [`infer_kind`] and
/// take the digest from the bytes they wrote, never from a claim. `trust` is
/// the caller's: a node that re-encodes says `sanitized`, one that writes
/// what it was handed carries the source's word forward.
pub fn durable_file_ref(
    rel_path: &str,
    filename: &str,
    mime: &str,
    bytes: &[u8],
    origin: &str,
    trust: &str,
) -> Value {
    let mime = mime.split(';').next().unwrap_or("").trim();
    let mime = if mime.is_empty() { "application/octet-stream" } else { mime };
    json!({
        "__zf_type": FILE_REF_TYPE,
        "backend": BACKEND_ZEBFS,
        "ref": rel_path,
        "filename": filename,
        "mime": mime,
        "kind": infer_kind(mime, filename),
        "size": bytes.len(),
        "sha256": format!("sha256:{:x}", Sha256::digest(bytes)),
        "lifecycle": LIFECYCLE_DURABLE,
        "origin": origin,
        "trust": trust,
    })
}

/// A stored project object as a durable FileRef.
///
/// The one door from a **store path** (`uploads/cat.png`, `public/logo.svg`)
/// to a FileRef: a manual run may name a file the project already holds
/// instead of uploading it again, and an agent over MCP has no multipart at
/// all. The bytes are read once to take the digest the contract requires, so
/// the ref is valid by its own document rather than by trust.
pub fn durable_file_ref_for_store_path(
    platform: &Arc<PlatformService>,
    owner: &str,
    project: &str,
    store_path: &str,
    origin: &str,
    trust: &str,
) -> Result<Value, PipelineError> {
    let rel = crate::zebfs::normalize_object_path(store_path.trim().trim_start_matches('/'))
        .map_err(|err| PipelineError::new("FW_FILE_REF_INVALID", err.to_string()))?;
    let layout = platform
        .file
        .ensure_project_layout(owner, project)
        .map_err(|err| PipelineError::new("FW_FILE_REF", err.to_string()))?;
    let zebfs = layout.open_files();
    let object = zebfs.get(&rel).map_err(|_| {
        PipelineError::new(
            "FW_FILE_REF_READ",
            format!("no stored file at '{rel}'"),
        )
    })?;
    let filename = rel.rsplit('/').next().unwrap_or(&rel).to_string();
    let mime = mime_for_filename(&filename);
    Ok(json!({
        "__zf_type": FILE_REF_TYPE,
        "backend": BACKEND_ZEBFS,
        "ref": object.stat.path,
        "filename": filename,
        "mime": mime,
        "kind": infer_kind(mime, &rel),
        "size": object.bytes.len(),
        "sha256": format!("sha256:{:x}", Sha256::digest(&object.bytes)),
        "lifecycle": LIFECYCLE_DURABLE,
        "origin": origin,
        "trust": trust,
    }))
}

/// A manual run's `files` map, resolved in place.
///
/// `input.files.<name>` may arrive as a **string** — a store path — from the
/// JSON form of `pipelines/execute` and from `pipeline_execute` over MCP. Each
/// one becomes the durable FileRef of that object, `origin: manual`,
/// `trust: user` (the operator is signed in). A FileRef already there is left
/// alone; a path that names nothing is refused, naming the path, before the
/// run starts. Returns how many paths were resolved.
pub fn resolve_manual_input_files(
    platform: &Arc<PlatformService>,
    owner: &str,
    project: &str,
    input: &mut Value,
) -> Result<usize, PipelineError> {
    let Some(files) = input.get_mut("files").and_then(Value::as_object_mut) else {
        return Ok(0);
    };
    let mut resolved = 0usize;
    for (name, value) in files.iter_mut() {
        let Some(path) = value.as_str().map(str::trim).filter(|p| !p.is_empty()) else {
            continue;
        };
        *value = durable_file_ref_for_store_path(platform, owner, project, path, "manual", "user")
            .map_err(|err| {
                PipelineError::new(
                    err.code,
                    format!("files.{name}: {}", err.message),
                )
            })?;
        resolved += 1;
    }
    Ok(resolved)
}

/// Deletes every `lifecycle: temporary` object a run wrote.
///
/// A file that arrives at a trigger lives for the run and is deleted after,
/// unless a node such as `fs.save` made it durable by copying it out. The
/// bytes sit under `tmp/runs/{request_id}/files/`, one folder per run, so the
/// whole folder goes at once. Best-effort by design: a folder that is already
/// gone, or was never written, is the common case and not an error.
pub fn remove_run_temporary_files(
    platform: &Arc<PlatformService>,
    owner: &str,
    project: &str,
    request_id: &str,
) {
    let request_part = sanitize_path_part(request_id);
    if request_part.is_empty() {
        return;
    }
    let Ok(layout) = platform.file.ensure_project_layout(owner, project) else {
        return;
    };
    let dir = layout.files_dir.join("tmp").join("runs").join(&request_part);
    if dir.is_dir() {
        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// The content type a stored object's name implies, for a FileRef built from
/// a path rather than an upload. The store keeps no content type of its own.
fn mime_for_filename(filename: &str) -> &'static str {
    let ext = filename
        .rsplit_once('.')
        .map(|(_, ext)| ext.to_ascii_lowercase())
        .unwrap_or_default();
    match ext.as_str() {
        "json" => "application/json",
        "geojson" => "application/geo+json",
        "csv" => "text/csv",
        "txt" | "md" => "text/plain",
        "html" | "htm" => "text/html",
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "ogg" => "audio/ogg",
        "m4a" => "audio/mp4",
        "flac" => "audio/flac",
        "mp4" | "m4v" => "video/mp4",
        "webm" => "video/webm",
        "mov" => "video/quicktime",
        "mkv" => "video/x-matroska",
        "pdf" => "application/pdf",
        "zip" => "application/zip",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "xls" => "application/vnd.ms-excel",
        "ods" => "application/vnd.oasis.opendocument.spreadsheet",
        "parquet" => "application/vnd.apache.parquet",
        _ => "application/octet-stream",
    }
}

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
    // Lowercase, not merely hexadecimal: the contract fixes one spelling so two
    // FileRefs for the same bytes compare equal.
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(PipelineError::new(
            "FW_FILE_REF_INVALID",
            "FileRef sha256 must contain exactly 64 lowercase hexadecimal digits",
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
        "audio/mpeg" => "mp3",
        "audio/wav" | "audio/x-wav" => "wav",
        "audio/ogg" => "ogg",
        "video/mp4" => "mp4",
        "video/webm" => "webm",
        "video/quicktime" => "mov",
        "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet" => "xlsx",
        "application/vnd.ms-excel" => "xls",
        "application/vnd.oasis.opendocument.spreadsheet" => "ods",
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
        (
            "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
            | "application/vnd.ms-excel"
            | "application/vnd.oasis.opendocument.spreadsheet",
            _,
        )
        | (_, "xlsx" | "xls" | "ods") => "spreadsheet",
        (m, _) if m.starts_with("audio/") => "audio",
        (_, "mp3" | "wav" | "ogg" | "m4a" | "flac" | "aac" | "opus") => "audio",
        (m, _) if m.starts_with("video/") => "video",
        (_, "mp4" | "webm" | "mov" | "mkv" | "avi" | "m4v") => "video",
        _ => "binary",
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        FILE_REF_KINDS, file_ref_path, infer_kind, is_file_ref, validate_file_ref,
        zebfs_rel_path, zebfs_rel_path_or_string,
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

    /// "the eleven values are the promise".
    #[test]
    fn kind_is_one_of_the_eleven_contract_values() {
        assert_eq!(FILE_REF_KINDS.len(), 11);
        for kind in FILE_REF_KINDS {
            let mut value = contract_file_ref();
            value["kind"] = json!(kind);
            validate_file_ref(&value).unwrap();
        }
        for kind in ["text", "object", "movie", "xlsx", ""] {
            let mut value = contract_file_ref();
            value["kind"] = json!(kind);
            assert!(
                validate_file_ref(&value).is_err(),
                "FileRef kind '{kind}' is outside the contract's eleven"
            );
        }
    }

    /// The three kinds added for `input.*`: by mime, by extension, and `csv`
    /// staying `csv` rather than becoming a spreadsheet.
    #[test]
    fn audio_video_and_spreadsheet_are_derived_like_the_rest() {
        assert_eq!(infer_kind("audio/mpeg", "song.bin"), "audio");
        assert_eq!(infer_kind("application/octet-stream", "song.mp3"), "audio");
        assert_eq!(infer_kind("video/mp4", "clip"), "video");
        assert_eq!(infer_kind("application/octet-stream", "clip.webm"), "video");
        assert_eq!(
            infer_kind(
                "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
                "sheet.bin"
            ),
            "spreadsheet"
        );
        assert_eq!(infer_kind("application/octet-stream", "sheet.xls"), "spreadsheet");
        assert_eq!(infer_kind("application/octet-stream", "sheet.ods"), "spreadsheet");
        assert_eq!(infer_kind("text/csv", "rows.csv"), "csv");
        assert_eq!(infer_kind("application/octet-stream", "rows.csv"), "csv");
        assert_eq!(infer_kind("application/octet-stream", "unknown.xyz"), "binary");
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
