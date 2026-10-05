//! `web.response.send --file <FileRef>` and `--path <key>`: a stored file as
//! the response.
//!
//! The node decides everything that can be decided before a byte moves: which
//! store and key (a FileRef's own `store` and `ref`, or a key in `--store`),
//! that the object is there, its type, its name, its validator. The bytes are
//! not read here — the envelope carries a `stored` description and the HTTP
//! layer streams the object from its store, answering `Range`, `HEAD` and the
//! conditional headers (`platform/web/stored_answer.rs`). A file of any size
//! is therefore never held in memory or base64'd into the run.
//!
//! A source that names nothing — `--file` resolved to `null`, a FileRef or a
//! key whose object is not in its store — fails with [`CODE_NOT_FOUND`], which
//! a route answers `404` when no `:error` edge takes it. The run keeps the
//! reason; the visitor sees a 404.

use std::sync::Arc;

use serde_json::{Value, json};

use super::Head;
use crate::pipeline::PipelineError;
use crate::pipeline::nodes::shared::file_ref::{is_file_ref, mime_for_filename};
use crate::pipeline::nodes::shared::project_store::open_source;
use crate::platform::services::PlatformService;
use crate::zebfs::model::ZebFsEntryKind;

/// The stored file a response names is not there.
pub const CODE_NOT_FOUND: &str = "FW_NODE_WEB_RESPONSE_SEND_NOT_FOUND";
/// The envelope key the HTTP layer streams from.
pub const STORED_KEY: &str = "stored";

/// The envelope for a stored file: the head with its content headers, and
/// `stored: { store, key, size, modified, etag }` for the HTTP layer.
pub fn stored_envelope(
    platform: &Arc<PlatformService>,
    owner: &str,
    project: &str,
    source: &Value,
    node_store: Option<&str>,
    download_name: Option<&str>,
    mut head: Head,
) -> Result<Value, PipelineError> {
    if source.is_null() {
        return Err(PipelineError::new(CODE_NOT_FOUND, "--file or --path resolved to nothing: there is no file to answer"));
    }
    if !(source.is_string() || is_file_ref(source)) {
        return Err(PipelineError::new(
            super::CODE_FILE,
            "--file is a project path, or a FileRef from fs.file.put / fs.file.get --return file; --path is a store key",
        ));
    }
    let Some((store, key)) = open_source(platform, owner, project, source, node_store)? else {
        return Err(PipelineError::new(CODE_NOT_FOUND, "--path is empty: there is no file to answer"));
    };
    // The one normaliser: `..`, absolute keys, the store's reserved `.zebfs/`
    // metadata and a key through a link are refused before a byte is read.
    // The key usually comes from the request, so a refusal is the visitor's
    // "no such file" — a 404 — and the run keeps why.
    let outside = |why: String| PipelineError::new(CODE_NOT_FOUND, format!("'{key}' is not a file in the project's stores: {why}"));
    if key.starts_with('/') || key.starts_with('\\') {
        return Err(outside("a store key is relative".to_string()));
    }
    let key = crate::zebfs::normalize_object_path(&key).map_err(|err| outside(err.message))?;
    let stat = match store.fs.head(&key) {
        Ok(stat) if stat.kind == ZebFsEntryKind::Object => stat,
        Ok(_) => return Err(PipelineError::new(CODE_NOT_FOUND, format!("'{key}' in store '{}' is a folder, not a file", store.id))),
        Err(err) if err.code == "ZEBFS_NOT_FOUND" => {
            return Err(PipelineError::new(CODE_NOT_FOUND, format!("'{key}' is not in store '{}'", store.id)));
        }
        Err(err) if matches!(err.code, "ZEBFS_INVALID_PATH" | "ZEBFS_RESERVED_PATH") => {
            return Err(PipelineError::new(CODE_NOT_FOUND, format!("'{key}' is not a file in store '{}': {}", store.id, err.message)));
        }
        Err(err) => return Err(PipelineError::new(super::CODE_FILE, format!("'{key}': {err}"))),
    };
    let leaf = key.rsplit('/').next().unwrap_or(&key).to_string();
    let (mime, name, etag) = if is_file_ref(source) {
        // A FileRef promises its size and digest; a different size is a
        // different object, and its digest would be a lie as a validator.
        let promised = source.get("size").and_then(Value::as_u64).unwrap_or_default();
        if promised != stat.size {
            return Err(PipelineError::new(
                super::CODE_FILE,
                format!("'{key}' is {} bytes but its FileRef says {promised}: the object changed after the FileRef was made", stat.size),
            ));
        }
        let digest = source["sha256"].as_str().unwrap_or_default().trim_start_matches("sha256:");
        (
            source["mime"].as_str().unwrap_or("application/octet-stream").to_string(),
            source["filename"].as_str().filter(|n| !n.trim().is_empty()).unwrap_or(&leaf).to_string(),
            format!("\"{digest}\""),
        )
    } else {
        // A bare key has no digest without reading it: size and time, weak.
        let secs = stat.modified.and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0);
        (mime_for_filename(&leaf).to_string(), leaf.clone(), format!("W/\"{:x}-{secs:x}\"", stat.size))
    };
    head.default_header("Content-Type", &mime);
    match download_name {
        Some(name) => head.default_header("Content-Disposition", &content_disposition("attachment", name)),
        None => head.default_header("Content-Disposition", &content_disposition("inline", &name)),
    }
    // A person's file, checked again on every visit: no shared cache keeps
    // it, and the validator makes the check a 304.
    head.default_header("Cache-Control", "private, no-cache");
    let modified = stat.modified.and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs());
    let mut envelope = head.envelope();
    envelope.insert(
        STORED_KEY.to_string(),
        json!({ "store": store.id, "key": key, "size": stat.size, "modified": modified, "etag": etag }),
    );
    Ok(Value::Object(envelope))
}

/// `Content-Disposition` for a name (RFC 6266): an ASCII `filename="…"` any
/// browser reads, and `filename*=UTF-8''…` (RFC 5987) carrying the name
/// exactly when it is not plain ASCII. Quotes, backslashes, control
/// characters and path separators never reach the quoted form.
pub fn content_disposition(disposition: &str, name: &str) -> String {
    let name = name.rsplit(['/', '\\']).next().unwrap_or(name).trim();
    let name = if name.is_empty() { "file" } else { name };
    let fallback: String = name
        .chars()
        .map(|c| if c.is_ascii_graphic() && c != '"' && c != '\\' || c == ' ' { c } else { '_' })
        .collect();
    if fallback == name {
        return format!("{disposition}; filename=\"{fallback}\"");
    }
    let mut encoded = String::new();
    for byte in name.bytes() {
        if byte.is_ascii_alphanumeric() || b"!#$&+-.^_`|~".contains(&byte) {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    format!("{disposition}; filename=\"{fallback}\"; filename*=UTF-8''{encoded}")
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{CODE_NOT_FOUND, content_disposition, stored_envelope};
    use crate::pipeline::nodes::basic::web::response::{CODE_FILE, Head};
    use crate::pipeline::nodes::shared::project_store::open_store;

    fn head() -> Head {
        Head { status: None, headers: Vec::new() }
    }

    fn header<'a>(envelope: &'a Value, name: &str) -> Option<&'a str> {
        envelope["headers"].as_array()?.iter().find(|pair| pair[0] == name).and_then(|pair| pair[1].as_str())
    }

    /// The node reads no bytes: it answers where they are, what they are and
    /// their validator — or why there is nothing to answer.
    #[test]
    fn a_stored_file_is_described_not_read_and_a_missing_one_is_not_found() {
        let platform = crate::pipeline::nodes::shared::test_platform::test_platform();
        let store = open_store(&platform, "demo", "demo", None).expect("store");
        store.fs.put("reports/a1.pdf", b"%PDF-1.7 demo").unwrap();
        let file = store.file_ref("reports/a1.pdf", "Quarter one.pdf", "application/pdf", b"%PDF-1.7 demo", "fs.file.put", "user");
        let digest = file["sha256"].as_str().unwrap().trim_start_matches("sha256:").to_string();

        let envelope = stored_envelope(&platform, "demo", "demo", &file, None, None, head()).expect("envelope");
        assert_eq!(envelope["stored"]["key"], "reports/a1.pdf");
        assert_eq!(envelope["stored"]["size"], 13);
        assert_eq!(envelope["stored"]["etag"], json!(format!("\"{digest}\"")));
        assert_eq!(header(&envelope, "Content-Type"), Some("application/pdf"));
        assert_eq!(header(&envelope, "Content-Disposition"), Some("inline; filename=\"Quarter one.pdf\""));
        assert_eq!(header(&envelope, "Cache-Control"), Some("private, no-cache"));
        assert!(envelope.get("body_base64").is_none() && envelope.get("text").is_none(), "{envelope}");

        let download = stored_envelope(&platform, "demo", "demo", &json!("reports/a1.pdf"), None, Some("q1.pdf"), head()).unwrap();
        assert_eq!(header(&download, "Content-Disposition"), Some("attachment; filename=\"q1.pdf\""));
        assert!(download["stored"]["etag"].as_str().unwrap().starts_with("W/\""), "a bare key has a weak validator");

        // Nothing there, nothing named, or not a key of the store: 404.
        for source in [json!(null), json!("reports/none.pdf"), json!("reports"), json!("../escape.txt"), json!("/etc/hosts"), json!(".zebfs/acl.json")] {
            let err = stored_envelope(&platform, "demo", "demo", &source, None, None, head()).unwrap_err();
            assert_eq!(err.code, CODE_NOT_FOUND, "{source}: {}", err.message);
        }
        // A FileRef whose object changed since it was made is not served under its digest.
        store.fs.put("reports/a1.pdf", b"%PDF-1.7 a longer demo").unwrap();
        assert_eq!(stored_envelope(&platform, "demo", "demo", &file, None, None, head()).unwrap_err().code, CODE_FILE);
    }

    #[test]
    fn a_download_name_is_safe_in_both_forms() {
        assert_eq!(content_disposition("attachment", "report.pdf"), "attachment; filename=\"report.pdf\"");
        assert_eq!(content_disposition("inline", "a b.pdf"), "inline; filename=\"a b.pdf\"");
        assert_eq!(
            content_disposition("attachment", "say \"hi\".txt"),
            "attachment; filename=\"say _hi_.txt\"; filename*=UTF-8''say%20%22hi%22.txt"
        );
        assert_eq!(
            content_disposition("attachment", "résumé.pdf"),
            "attachment; filename=\"r_sum_.pdf\"; filename*=UTF-8''r%C3%A9sum%C3%A9.pdf"
        );
        // A path is never a name, and a header is one line.
        assert_eq!(content_disposition("attachment", "../../etc/passwd"), "attachment; filename=\"passwd\"");
        let injected = content_disposition("attachment", "a\r\nSet-Cookie: x=1");
        assert!(!injected.contains('\r') && !injected.contains('\n'), "{injected}");
        assert_eq!(content_disposition("attachment", ""), "attachment; filename=\"file\"");
    }
}
