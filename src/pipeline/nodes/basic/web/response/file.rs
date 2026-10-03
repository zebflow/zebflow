//! `web.response.send --file`: a project file as the response.
//!
//! The content type follows the extension (`.webmanifest`, `.json`, `.js`,
//! `.css`, `.txt`, `.xml`, `.svg`, `.png`, `.jpg`, `.webp`, `.ico`, `.woff2`,
//! `.pdf`, `.md`, `.html`). A `.ts` file is compiled to JavaScript first. Every
//! script (`.ts`, `.js`) starts with one line the platform adds,
//! `self.__ZF = Object.freeze({ version, source })` — the build and the file's
//! hash — so a service worker can key its cache on them. Everything else is
//! served byte for byte.
//!
//! With `--root`, `--file` is a bare filename — usually a `{{ }}` from the
//! route — and may only name a file directly inside that folder: no slashes,
//! no `..`. That is what makes `/pwa/{file}` safe to expose.

use sha2::{Digest, Sha256};
use serde_json::json;

use crate::pipeline::PipelineError;

/// What `--file` answers with, worked out from the name and the bytes. Pure,
/// so a test can hand it a name and a source without a project.
#[derive(Debug)]
pub struct FileResponse {
    pub content_type: &'static str,
    pub body: FileBody,
}

#[derive(Debug)]
pub enum FileBody {
    Text(String),
    /// Base64 — the response envelope travels as JSON.
    Bytes(String),
}

impl FileResponse {
    pub fn from_bytes(rel: &str, bytes: Vec<u8>) -> Result<Self, PipelineError> {
        let ext = rel.rsplit('.').next().unwrap_or("").to_ascii_lowercase();
        let (content_type, text) = match ext.as_str() {
            "webmanifest" => ("application/manifest+json; charset=utf-8", true),
            "json" | "map" => ("application/json; charset=utf-8", true),
            "js" | "mjs" | "ts" => ("text/javascript; charset=utf-8", true),
            "css" => ("text/css; charset=utf-8", true),
            "txt" => ("text/plain; charset=utf-8", true),
            "md" => ("text/markdown; charset=utf-8", true),
            "html" | "htm" => ("text/html; charset=utf-8", true),
            "xml" => ("application/xml; charset=utf-8", true),
            "svg" => ("image/svg+xml", true),
            "csv" => ("text/csv; charset=utf-8", true),
            "png" => ("image/png", false),
            "jpg" | "jpeg" => ("image/jpeg", false),
            "gif" => ("image/gif", false),
            "webp" => ("image/webp", false),
            "ico" => ("image/x-icon", false),
            "woff2" => ("font/woff2", false),
            "woff" => ("font/woff", false),
            "pdf" => ("application/pdf", false),
            "wasm" => ("application/wasm", false),
            _ => ("application/octet-stream", false),
        };
        if !text {
            use base64::Engine as _;
            return Ok(Self { content_type, body: FileBody::Bytes(base64::engine::general_purpose::STANDARD.encode(bytes)) });
        }
        let source = String::from_utf8(bytes).map_err(|_| PipelineError::new("FW_NODE_WEB_RESPONSE_SEND_FILE", format!("'{rel}' is not UTF-8 text")))?;
        let body = match ext.as_str() {
            "ts" => {
                let compiled = crate::rwe::core::deno_worker::transpile_ts(&source)
                    .map_err(|e| PipelineError::new("FW_NODE_WEB_RESPONSE_SEND_COMPILE", format!("{rel}: {}", e.message)))?;
                format!("{}{compiled}", script_prelude(&source))
            }
            "js" | "mjs" => format!("{}{source}", script_prelude(&source)),
            _ => source,
        };
        Ok(Self { content_type, body: FileBody::Text(body) })
    }
}

/// `self.__ZF = Object.freeze({ version, source })` — the platform build and the
/// file's hash, so a worker keys its cache on both.
fn script_prelude(source: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(source.as_bytes());
    let digest = format!("{:x}", hasher.finalize());
    format!("self.__ZF = Object.freeze({{ version: {}, source: {} }});\n", json!(crate::version::APP_VERSION), json!(&digest[..16]))
}

/// The project-relative path `--file` (and `--root`) name, or why not. With
/// a root the file is a bare name and stays directly inside it; without one
/// it is a project path that never climbs out. Pure.
pub fn resolve_file_rel_path(root: Option<&str>, file: &str) -> Result<String, PipelineError> {
    fn clean(raw: &str, what: &str) -> Result<Vec<String>, PipelineError> {
        let mut parts = Vec::new();
        for part in raw.trim().replace('\\', "/").split('/') {
            match part.trim() {
                "" | "." => continue,
                ".." => return Err(PipelineError::new("FW_NODE_WEB_RESPONSE_SEND_FILE", format!("{what} '{raw}' escapes the project"))),
                p => parts.push(p.to_string()),
            }
        }
        Ok(parts)
    }
    let file = file.trim();
    if file.is_empty() {
        return Err(PipelineError::new("FW_NODE_WEB_RESPONSE_SEND_FILE", "web.response.send --file needs a path, e.g. pwa/manifest.webmanifest"));
    }
    match root.map(str::trim).filter(|f| !f.is_empty()) {
        Some(root) => {
            if file.contains('/') || file.contains('\\') || file == ".." || file == "." {
                return Err(PipelineError::new("FW_NODE_WEB_RESPONSE_SEND_FILE", format!("with --root, --file must be a bare filename; got '{file}'")));
            }
            let mut parts = clean(root, "root")?;
            if parts.is_empty() {
                return Err(PipelineError::new("FW_NODE_WEB_RESPONSE_SEND_FILE", "--root is empty"));
            }
            parts.push(file.to_string());
            Ok(parts.join("/"))
        }
        None => {
            let parts = clean(file, "file")?;
            if parts.is_empty() {
                return Err(PipelineError::new("FW_NODE_WEB_RESPONSE_SEND_FILE", "web.response.send --file needs a path"));
            }
            Ok(parts.join("/"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_manifest_is_typed_by_its_extension_and_served_as_written() {
        let r = FileResponse::from_bytes("pwa/manifest.webmanifest", br#"{"name":"RESEARCHSITE"}"#.to_vec()).unwrap();
        assert_eq!(r.content_type, "application/manifest+json; charset=utf-8");
        assert!(matches!(r.body, FileBody::Text(ref t) if t == r#"{"name":"RESEARCHSITE"}"#));
    }

    #[test]
    fn a_typescript_worker_is_compiled_and_gets_the_prelude() {
        let src = "const SHELL: string[] = [\"/\"];\nself.addEventListener(\"install\", (e: any) => e.waitUntil(caches.open(self.__ZF.version)));\n";
        let r = FileResponse::from_bytes("pwa/site.sw.ts", src.as_bytes().to_vec()).unwrap();
        assert_eq!(r.content_type, "text/javascript; charset=utf-8");
        let FileBody::Text(js) = r.body else { panic!("text") };
        assert!(js.starts_with("self.__ZF = Object.freeze({ version: \""), "{js}");
        assert!(!js.contains(": string[]"), "types stripped: {js}");
        assert!(js.contains("addEventListener(\"install\""));
    }

    #[test]
    fn a_png_travels_as_base64_with_its_type() {
        let r = FileResponse::from_bytes("pwa/icons/icon-192.png", vec![0x89, b'P', b'N', b'G']).unwrap();
        assert_eq!(r.content_type, "image/png");
        assert!(matches!(r.body, FileBody::Bytes(ref b) if b == "iVBORw=="));
    }

    #[test]
    fn a_broken_typescript_names_the_file() {
        let err = FileResponse::from_bytes("pwa/bad.sw.ts", b"const = ;".to_vec()).unwrap_err();
        assert_eq!(err.code, "FW_NODE_WEB_RESPONSE_SEND_COMPILE");
        assert!(err.message.starts_with("pwa/bad.sw.ts"));
    }

    #[test]
    fn a_file_path_never_climbs_out_of_the_project() {
        assert_eq!(resolve_file_rel_path(None, "pwa/manifest.webmanifest").unwrap(), "pwa/manifest.webmanifest");
        assert_eq!(resolve_file_rel_path(None, "./pwa//site.sw.ts").unwrap(), "pwa/site.sw.ts");
        assert_eq!(resolve_file_rel_path(None, "../secrets").unwrap_err().code, "FW_NODE_WEB_RESPONSE_SEND_FILE");
        assert_eq!(resolve_file_rel_path(None, "").unwrap_err().code, "FW_NODE_WEB_RESPONSE_SEND_FILE");
    }

    #[test]
    fn with_a_root_the_name_from_the_route_stays_inside_it() {
        assert_eq!(resolve_file_rel_path(Some("pwa/icons"), "icon-192.png").unwrap(), "pwa/icons/icon-192.png");
        for bad in ["../site.sw.ts", "sub/icon.png", "..", "", "pwa/icons/x.png"] {
            assert_eq!(resolve_file_rel_path(Some("pwa/icons"), bad).unwrap_err().code, "FW_NODE_WEB_RESPONSE_SEND_FILE", "{bad}");
        }
        assert_eq!(resolve_file_rel_path(Some("../"), "x.png").unwrap_err().code, "FW_NODE_WEB_RESPONSE_SEND_FILE");
    }
}
