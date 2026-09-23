//! The S3 native store: a project's `files/` tree kept in a bucket.
//!
//! Same seven verbs as [`super::local::LocalZebFs`], same normalised paths,
//! same reserved `.zebfs/` prefix, so a project that declares
//! `spec.files.backend: s3` behaves the way it did on disk and every FileRef
//! it writes keeps its shape — only `backend` reads `s3`, and only this store
//! interprets `ref`.
//!
//! Talks the S3 REST API with Signature Version 4 over a blocking HTTP
//! client, because the store's callers are synchronous today and a
//! signature is forty lines that do not deserve a dependency the size of a
//! cloud SDK. Works against AWS S3, Cloudflare R2, MinIO, SeaweedFS, Garage,
//! Backblaze B2 and Tigris; the two knobs that differ between them are the
//! endpoint and whether the bucket is a path segment or a host label.
//!
//! What S3 has not got, this store supplies the way every S3 client does:
//! a *prefix* is a key that ends in `/` (created by `create_prefix`, hidden
//! from listings), and deleting a prefix deletes every key beneath it.

use std::fmt;
use std::io::Read;
use std::time::{Duration, SystemTime};

use hmac::{Hmac, Mac};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::acl::is_reserved_acl_path;
use super::error::ZebFsError;
use super::local::normalize_object_path;
use super::model::{ZebFsEntry, ZebFsEntryKind, ZebFsObject, ZebFsStat};

const CODE: &str = "ZEBFS_S3";

/// Everything needed to reach one bucket. Read from a credential of kind `s3`.
#[derive(Clone, PartialEq, Eq)]
pub struct S3Config {
    /// `https://s3.amazonaws.com`, `https://<account>.r2.cloudflarestorage.com`,
    /// `http://127.0.0.1:8333` — scheme and host, no path.
    pub endpoint: String,
    /// The signing region. `us-east-1` for most self-hosted stores, `auto` for R2.
    pub region: String,
    pub bucket: String,
    /// A key prefix every object of this project lives under; empty means the
    /// bucket root. Lets several projects share one bucket.
    pub prefix: String,
    pub access_key_id: String,
    pub secret_access_key: String,
    /// `true`: `endpoint/bucket/key` (MinIO, SeaweedFS, Garage, R2).
    /// `false`: `bucket.endpoint/key` (AWS's default).
    pub path_style: bool,
}

impl fmt::Debug for S3Config {
    /// The secret never reaches a log line, whatever printed the layout.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("S3Config")
            .field("endpoint", &self.endpoint)
            .field("region", &self.region)
            .field("bucket", &self.bucket)
            .field("prefix", &self.prefix)
            .field("access_key_id", &self.access_key_id)
            .field("secret_access_key", &"***")
            .field("path_style", &self.path_style)
            .finish()
    }
}

impl S3Config {
    /// Reads a credential of kind `s3` — the fields the credential form
    /// declares in `builtin_credential_types` — refusing a missing one by name.
    pub fn from_credential(secret: &Value) -> Result<Self, ZebFsError> {
        let field = |key: &str| -> String {
            secret
                .get(key)
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or("")
                .to_string()
        };
        let required = |key: &str| -> Result<String, ZebFsError> {
            let value = field(key);
            if value.is_empty() {
                return Err(ZebFsError::new(
                    "ZEBFS_S3_CREDENTIAL",
                    format!("the s3 credential has no '{key}'"),
                ));
            }
            Ok(value)
        };
        let endpoint = required("endpoint")?.trim_end_matches('/').to_string();
        if !(endpoint.starts_with("http://") || endpoint.starts_with("https://")) {
            return Err(ZebFsError::new(
                "ZEBFS_S3_CREDENTIAL",
                "endpoint must start with http:// or https://",
            ));
        }
        let region = {
            let value = field("region");
            if value.is_empty() { "us-east-1".to_string() } else { value }
        };
        let prefix = field("prefix");
        let prefix = if prefix.is_empty() {
            String::new()
        } else {
            normalize_object_path(&prefix)?
        };
        let path_style = !field("addressing").eq_ignore_ascii_case("virtual");
        Ok(Self {
            endpoint,
            region,
            bucket: required("bucket")?,
            prefix,
            access_key_id: required("access_key_id")?,
            secret_access_key: required("secret_access_key")?,
            path_style,
        })
    }

    fn scheme_and_host(&self) -> (&str, &str) {
        match self.endpoint.split_once("://") {
            Some((scheme, host)) => (scheme, host.trim_end_matches('/')),
            None => ("http", self.endpoint.as_str()),
        }
    }
}

/// A project's files in a bucket.
#[derive(Debug, Clone)]
pub struct S3ZebFs {
    config: S3Config,
    agent: ureq::Agent,
}

impl S3ZebFs {
    pub fn new(config: S3Config) -> Self {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(10))
            .timeout_read(Duration::from_secs(120))
            .timeout_write(Duration::from_secs(120))
            .build();
        Self { config, agent }
    }

    pub fn config(&self) -> &S3Config {
        &self.config
    }

    // ── the seven verbs ─────────────────────────────────────────────────

    pub fn put(&self, path: &str, bytes: &[u8]) -> Result<ZebFsStat, ZebFsError> {
        let rel = normalize_object_path(path)?;
        ensure_user_object_path(&rel)?;
        self.put_key(&self.key_for(&rel), bytes)?;
        self.head(&rel)
    }

    pub fn get(&self, path: &str) -> Result<ZebFsObject, ZebFsError> {
        let rel = normalize_object_path(path)?;
        ensure_user_object_path(&rel)?;
        let (bytes, stat) = self.get_key(&self.key_for(&rel), &rel)?;
        Ok(ZebFsObject { path: rel, bytes, stat })
    }

    pub fn head(&self, path: &str) -> Result<ZebFsStat, ZebFsError> {
        let rel = normalize_object_path(path)?;
        ensure_user_object_path(&rel)?;
        let key = self.key_for(&rel);
        match self.request("HEAD", &key, &[], None, &[]) {
            Ok(response) => Ok(ZebFsStat {
                path: rel,
                size: header_u64(&response, "content-length"),
                modified: header_time(&response, "last-modified"),
                kind: ZebFsEntryKind::Object,
            }),
            Err(err) if err.code == "ZEBFS_NOT_FOUND" => {
                // Not an object. A prefix, if anything lives under it.
                if self.any_under(&format!("{key}/"))? {
                    Ok(ZebFsStat {
                        path: rel,
                        size: 0,
                        modified: None,
                        kind: ZebFsEntryKind::Prefix,
                    })
                } else {
                    Err(ZebFsError::new("ZEBFS_NOT_FOUND", "object not found"))
                }
            }
            Err(err) => Err(err),
        }
    }

    pub fn list(&self, prefix: &str) -> Result<Vec<ZebFsEntry>, ZebFsError> {
        let rel = normalize_optional_prefix(prefix)?;
        if !rel.is_empty() {
            ensure_user_object_path(&rel)?;
        }
        let base = self.listing_base(&rel);
        let mut out = Vec::new();
        let mut token: Option<String> = None;
        loop {
            let mut query: Vec<(String, String)> = vec![
                ("delimiter".into(), "/".into()),
                ("list-type".into(), "2".into()),
                ("max-keys".into(), "1000".into()),
                ("prefix".into(), base.clone()),
            ];
            if let Some(t) = &token {
                query.push(("continuation-token".into(), t.clone()));
            }
            let page = self.list_page(&query)?;
            for (key, size, modified) in page.objects {
                let Some(name) = key.strip_prefix(&base) else { continue };
                // The prefix's own marker, or a key that belongs to a deeper
                // level the delimiter should have folded.
                if name.is_empty() || name.contains('/') {
                    continue;
                }
                if rel.is_empty() && is_reserved_acl_path(name) {
                    continue;
                }
                out.push(ZebFsEntry {
                    path: join(&rel, name),
                    name: name.to_string(),
                    size,
                    modified,
                    kind: ZebFsEntryKind::Object,
                });
            }
            for common in page.prefixes {
                let Some(name) = common.strip_prefix(&base) else { continue };
                let name = name.trim_end_matches('/');
                if name.is_empty() || (rel.is_empty() && is_reserved_acl_path(name)) {
                    continue;
                }
                out.push(ZebFsEntry {
                    path: join(&rel, name),
                    name: name.to_string(),
                    size: 0,
                    modified: None,
                    kind: ZebFsEntryKind::Prefix,
                });
            }
            match page.next {
                Some(next) if !next.is_empty() => token = Some(next),
                _ => break,
            }
        }
        out.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(out)
    }

    /// A prefix is a zero-byte key ending in `/`, the convention every S3
    /// console uses; listings hide it.
    pub fn create_prefix(&self, prefix: &str) -> Result<ZebFsStat, ZebFsError> {
        let rel = normalize_object_path(prefix)?;
        ensure_user_object_path(&rel)?;
        self.put_key(&format!("{}/", self.key_for(&rel)), &[])?;
        Ok(ZebFsStat {
            path: rel,
            size: 0,
            modified: None,
            kind: ZebFsEntryKind::Prefix,
        })
    }

    /// Deletes the object at `path` and everything beneath `path/`. Absent
    /// is not an error, as on disk.
    pub fn delete(&self, path: &str) -> Result<(), ZebFsError> {
        let rel = normalize_object_path(path)?;
        ensure_user_object_path(&rel)?;
        let key = self.key_for(&rel);
        self.delete_key(&key)?;
        for under in self.keys_under(&format!("{key}/"))? {
            self.delete_key(&under)?;
        }
        Ok(())
    }

    pub fn copy(&self, from: &str, to: &str) -> Result<ZebFsStat, ZebFsError> {
        let from_rel = normalize_object_path(from)?;
        let to_rel = normalize_object_path(to)?;
        ensure_user_object_path(&from_rel)?;
        ensure_user_object_path(&to_rel)?;
        let source = format!(
            "/{}/{}",
            self.config.bucket,
            uri_encode(&self.key_for(&from_rel), false)
        );
        match self.request(
            "PUT",
            &self.key_for(&to_rel),
            &[],
            Some(&[]),
            &[("x-amz-copy-source", source.as_str())],
        ) {
            Ok(_) => {}
            Err(err) if err.code == "ZEBFS_NOT_FOUND" => {
                return Err(ZebFsError::new("ZEBFS_NOT_FOUND", "source object not found"));
            }
            Err(err) => return Err(err),
        }
        self.head(&to_rel)
    }

    // ── the metadata document, which lives under the reserved prefix ────

    /// Bytes of a reserved-prefix document, `None` when it was never written.
    pub fn read_reserved(&self, path: &str) -> Result<Option<Vec<u8>>, ZebFsError> {
        let rel = normalize_object_path(path)?;
        match self.get_key(&self.key_for(&rel), &rel) {
            Ok((bytes, _)) => Ok(Some(bytes)),
            Err(err) if err.code == "ZEBFS_NOT_FOUND" => Ok(None),
            Err(err) => Err(err),
        }
    }

    pub fn write_reserved(&self, path: &str, bytes: &[u8]) -> Result<(), ZebFsError> {
        let rel = normalize_object_path(path)?;
        self.put_key(&self.key_for(&rel), bytes)
    }

    // ── keys and requests ───────────────────────────────────────────────

    fn key_for(&self, rel: &str) -> String {
        if self.config.prefix.is_empty() {
            rel.to_string()
        } else {
            format!("{}/{rel}", self.config.prefix)
        }
    }

    /// The listing prefix for `rel`: `prefix/rel/`, `prefix/`, `rel/` or ``.
    fn listing_base(&self, rel: &str) -> String {
        match (self.config.prefix.is_empty(), rel.is_empty()) {
            (true, true) => String::new(),
            (true, false) => format!("{rel}/"),
            (false, true) => format!("{}/", self.config.prefix),
            (false, false) => format!("{}/{rel}/", self.config.prefix),
        }
    }

    fn put_key(&self, key: &str, bytes: &[u8]) -> Result<(), ZebFsError> {
        self.request("PUT", key, &[], Some(bytes), &[]).map(|_| ())
    }

    fn delete_key(&self, key: &str) -> Result<(), ZebFsError> {
        match self.request("DELETE", key, &[], None, &[]) {
            Ok(_) => Ok(()),
            Err(err) if err.code == "ZEBFS_NOT_FOUND" => Ok(()),
            Err(err) => Err(err),
        }
    }

    fn get_key(&self, key: &str, rel: &str) -> Result<(Vec<u8>, ZebFsStat), ZebFsError> {
        let response = self.request("GET", key, &[], None, &[])?;
        let size = header_u64(&response, "content-length");
        let modified = header_time(&response, "last-modified");
        let mut bytes = Vec::with_capacity(size as usize);
        response
            .into_reader()
            .read_to_end(&mut bytes)
            .map_err(|err| ZebFsError::new(CODE, format!("reading {key}: {err}")))?;
        let stat = ZebFsStat {
            path: rel.to_string(),
            size: bytes.len() as u64,
            modified,
            kind: ZebFsEntryKind::Object,
        };
        Ok((bytes, stat))
    }

    fn any_under(&self, base: &str) -> Result<bool, ZebFsError> {
        let page = self.list_page(&[
            ("list-type".into(), "2".into()),
            ("max-keys".into(), "1".into()),
            ("prefix".into(), base.to_string()),
        ])?;
        Ok(!page.objects.is_empty() || !page.prefixes.is_empty())
    }

    /// Every key under `base`, at any depth.
    fn keys_under(&self, base: &str) -> Result<Vec<String>, ZebFsError> {
        let mut keys = Vec::new();
        let mut token: Option<String> = None;
        loop {
            let mut query: Vec<(String, String)> = vec![
                ("list-type".into(), "2".into()),
                ("max-keys".into(), "1000".into()),
                ("prefix".into(), base.to_string()),
            ];
            if let Some(t) = &token {
                query.push(("continuation-token".into(), t.clone()));
            }
            let page = self.list_page(&query)?;
            keys.extend(page.objects.into_iter().map(|(key, _, _)| key));
            match page.next {
                Some(next) if !next.is_empty() => token = Some(next),
                _ => return Ok(keys),
            }
        }
    }

    fn list_page(&self, query: &[(String, String)]) -> Result<ListPage, ZebFsError> {
        let response = self.request("GET", "", query, None, &[])?;
        let text = response
            .into_string()
            .map_err(|err| ZebFsError::new(CODE, format!("reading listing: {err}")))?;
        parse_listing(&text)
    }

    /// One signed request. `key` empty means the bucket itself.
    fn request(
        &self,
        method: &str,
        key: &str,
        query: &[(String, String)],
        body: Option<&[u8]>,
        extra: &[(&str, &str)],
    ) -> Result<ureq::Response, ZebFsError> {
        let (scheme, host) = self.config.scheme_and_host();
        let encoded_key = uri_encode(key, false);
        let (host_header, canonical_uri) = if self.config.path_style {
            let uri = if key.is_empty() {
                format!("/{}", self.config.bucket)
            } else {
                format!("/{}/{encoded_key}", self.config.bucket)
            };
            (host.to_string(), uri)
        } else {
            (format!("{}.{host}", self.config.bucket), format!("/{encoded_key}"))
        };
        let mut pairs: Vec<(String, String)> = query
            .iter()
            .map(|(k, v)| (uri_encode(k, true), uri_encode(v, true)))
            .collect();
        pairs.sort();
        let canonical_query = pairs
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>()
            .join("&");
        let url = if canonical_query.is_empty() {
            format!("{scheme}://{host_header}{canonical_uri}")
        } else {
            format!("{scheme}://{host_header}{canonical_uri}?{canonical_query}")
        };

        let payload_hash = hex::encode(Sha256::digest(body.unwrap_or(&[])));
        let now = chrono::Utc::now();
        let signed = sign(
            &self.config,
            method,
            &canonical_uri,
            &canonical_query,
            &host_header,
            &payload_hash,
            now.format("%Y%m%dT%H%M%SZ").to_string().as_str(),
            extra,
        );

        let mut request = self.agent.request(method, &url);
        for (name, value) in &signed {
            if name != "host" {
                request = request.set(name, value);
            }
        }
        let result = match body {
            Some(bytes) => request.send_bytes(bytes),
            None => request.call(),
        };
        match result {
            Ok(response) => Ok(response),
            Err(ureq::Error::Status(404, _)) => {
                Err(ZebFsError::new("ZEBFS_NOT_FOUND", "object not found"))
            }
            Err(ureq::Error::Status(code, response)) => {
                let detail = response
                    .into_string()
                    .unwrap_or_default()
                    .chars()
                    .take(300)
                    .collect::<String>();
                Err(ZebFsError::new(
                    CODE,
                    format!("{method} {canonical_uri}: the store answered {code}: {detail}"),
                ))
            }
            Err(ureq::Error::Transport(err)) => Err(ZebFsError::new(
                CODE,
                format!("{method} {canonical_uri}: cannot reach the store: {err}"),
            )),
        }
    }
}

struct ListPage {
    /// `(key, size, modified)`
    objects: Vec<(String, u64, Option<SystemTime>)>,
    prefixes: Vec<String>,
    next: Option<String>,
}

fn parse_listing(xml: &str) -> Result<ListPage, ZebFsError> {
    let doc = roxmltree::Document::parse(xml)
        .map_err(|err| ZebFsError::new(CODE, format!("listing is not XML: {err}")))?;
    let text_of = |node: roxmltree::Node, tag: &str| -> Option<String> {
        node.children()
            .find(|c| c.has_tag_name(tag))
            .and_then(|c| c.text())
            .map(str::to_string)
    };
    let mut page = ListPage {
        objects: Vec::new(),
        prefixes: Vec::new(),
        next: None,
    };
    let root = doc.root_element();
    for node in root.children() {
        if node.has_tag_name("Contents") {
            let Some(key) = text_of(node, "Key") else { continue };
            let size = text_of(node, "Size")
                .and_then(|s| s.trim().parse::<u64>().ok())
                .unwrap_or(0);
            let modified = text_of(node, "LastModified").and_then(|s| {
                chrono::DateTime::parse_from_rfc3339(s.trim())
                    .ok()
                    .map(SystemTime::from)
            });
            page.objects.push((key, size, modified));
        } else if node.has_tag_name("CommonPrefixes") {
            if let Some(prefix) = text_of(node, "Prefix") {
                page.prefixes.push(prefix);
            }
        } else if node.has_tag_name("NextContinuationToken") {
            page.next = node.text().map(str::to_string);
        }
    }
    Ok(page)
}

/// Signature Version 4 over the canonical request, answering every header
/// the request must carry, `host` included so the canonical form is exact.
#[allow(clippy::too_many_arguments)]
fn sign(
    config: &S3Config,
    method: &str,
    canonical_uri: &str,
    canonical_query: &str,
    host: &str,
    payload_hash: &str,
    amz_date: &str,
    extra: &[(&str, &str)],
) -> Vec<(String, String)> {
    let mut headers: Vec<(String, String)> = vec![
        ("host".into(), host.to_string()),
        ("x-amz-content-sha256".into(), payload_hash.to_string()),
        ("x-amz-date".into(), amz_date.to_string()),
    ];
    for (name, value) in extra {
        headers.push((name.to_ascii_lowercase(), value.trim().to_string()));
    }
    headers.sort();
    let canonical_headers = headers
        .iter()
        .map(|(k, v)| format!("{k}:{v}\n"))
        .collect::<String>();
    let signed_headers = headers
        .iter()
        .map(|(k, _)| k.as_str())
        .collect::<Vec<_>>()
        .join(";");
    let canonical_request = format!(
        "{method}\n{canonical_uri}\n{canonical_query}\n{canonical_headers}\n{signed_headers}\n{payload_hash}"
    );
    let date = &amz_date[..8];
    let scope = format!("{date}/{}/s3/aws4_request", config.region);
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{scope}\n{}",
        hex::encode(Sha256::digest(canonical_request.as_bytes()))
    );
    let mut key = hmac_sha256(format!("AWS4{}", config.secret_access_key).as_bytes(), date.as_bytes());
    for part in [config.region.as_str(), "s3", "aws4_request"] {
        key = hmac_sha256(&key, part.as_bytes());
    }
    let signature = hex::encode(hmac_sha256(&key, string_to_sign.as_bytes()));
    headers.push((
        "authorization".into(),
        format!(
            "AWS4-HMAC-SHA256 Credential={}/{scope}, SignedHeaders={signed_headers}, Signature={signature}",
            config.access_key_id
        ),
    ));
    headers
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("hmac accepts any key length");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

/// RFC 3986 percent-encoding the way SigV4 canonicalises it: unreserved
/// characters pass, everything else is `%XX` upper-case, and `/` passes in a
/// path but not in a query value.
fn uri_encode(input: &str, encode_slash: bool) -> String {
    let mut out = String::with_capacity(input.len());
    for byte in input.bytes() {
        let keep = byte.is_ascii_alphanumeric()
            || matches!(byte, b'-' | b'_' | b'.' | b'~')
            || (byte == b'/' && !encode_slash);
        if keep {
            out.push(byte as char);
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

fn header_u64(response: &ureq::Response, name: &str) -> u64 {
    response
        .header(name)
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(0)
}

fn header_time(response: &ureq::Response, name: &str) -> Option<SystemTime> {
    response
        .header(name)
        .and_then(|v| chrono::DateTime::parse_from_rfc2822(v.trim()).ok())
        .map(SystemTime::from)
}

fn join(rel: &str, name: &str) -> String {
    if rel.is_empty() {
        name.to_string()
    } else {
        format!("{rel}/{name}")
    }
}

fn normalize_optional_prefix(path: &str) -> Result<String, ZebFsError> {
    let trimmed = path.trim().trim_matches('/');
    if trimmed.is_empty() {
        return Ok(String::new());
    }
    normalize_object_path(trimmed)
}

fn ensure_user_object_path(path: &str) -> Result<(), ZebFsError> {
    if is_reserved_acl_path(path) {
        return Err(ZebFsError::new(
            "ZEBFS_RESERVED_PATH",
            "path is reserved for ZebFS metadata",
        ));
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex};

    /// The signature for one fixed request, computed independently in Python
    /// with the same inputs. If this changes, it is the signer that broke.
    #[test]
    fn the_signature_matches_an_independent_signer() {
        let config = S3Config {
            endpoint: "http://127.0.0.1:8333".into(),
            region: "us-east-1".into(),
            bucket: "zebflow-dev".into(),
            prefix: String::new(),
            access_key_id: "zebflowdev".into(),
            secret_access_key: "zebflowdev-secret-local-only".into(),
            path_style: true,
        };
        let empty = hex::encode(Sha256::digest(b""));
        let headers = sign(
            &config,
            "GET",
            "/zebflow-dev/hello/world.txt",
            "",
            "127.0.0.1:8333",
            &empty,
            "20260924T000000Z",
            &[],
        );
        let authorization = headers
            .iter()
            .find(|(k, _)| k == "authorization")
            .map(|(_, v)| v.clone())
            .expect("authorization header");
        assert!(authorization.contains("SignedHeaders=host;x-amz-content-sha256;x-amz-date"), "{authorization}");
        assert!(authorization.ends_with(EXPECTED_SIGNATURE), "{authorization}");
    }

    #[test]
    fn keys_are_percent_encoded_the_way_sigv4_canonicalises() {
        assert_eq!(uri_encode("a b/c~d.e", false), "a%20b/c~d.e");
        assert_eq!(uri_encode("a/b", true), "a%2Fb");
        assert_eq!(uri_encode("ünï", false), "%C3%BCn%C3%AF");
    }

    #[test]
    fn a_credential_becomes_a_config_and_a_missing_field_is_named() {
        let secret = serde_json::json!({
            "endpoint": "http://127.0.0.1:8333/", "bucket": "b", "prefix": "/projects/demo/",
            "access_key_id": "ak", "secret_access_key": "sk", "addressing": "virtual"
        });
        let config = S3Config::from_credential(&secret).unwrap();
        assert_eq!(config.endpoint, "http://127.0.0.1:8333");
        assert_eq!(config.region, "us-east-1");
        assert_eq!(config.prefix, "projects/demo");
        assert!(!config.path_style);
        assert!(!format!("{config:?}").contains("sk"), "the secret must not print");

        let err = S3Config::from_credential(&serde_json::json!({ "endpoint": "http://x", "bucket": "b" })).unwrap_err();
        assert_eq!(err.code, "ZEBFS_S3_CREDENTIAL");
        assert!(err.message.contains("access_key_id"), "{}", err.message);
    }

    // ── an S3 that lives in the test process ─────────────────────────────

    pub(crate) type Objects = Arc<Mutex<BTreeMap<String, Vec<u8>>>>;

    /// An S3 that lives in the test process: path-style, one bucket, every
    /// request must be signed. For any test that needs a bucket.
    pub(crate) fn fake_s3() -> (u16, Objects) {
        use axum::{
            body::Bytes,
            extract::{Path, Query, State},
            http::{HeaderMap, StatusCode},
            routing::get,
            Router,
        };
        let objects: Objects = Arc::new(Mutex::new(BTreeMap::new()));
        let state = Arc::clone(&objects);
        let (port_tx, port_rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();
            runtime.block_on(async move {
                async fn list(
                    State(objects): State<Objects>,
                    Query(query): Query<BTreeMap<String, String>>,
                ) -> (StatusCode, String) {
                    let prefix = query.get("prefix").cloned().unwrap_or_default();
                    let delimiter = query.get("delimiter").cloned();
                    let objects = objects.lock().unwrap();
                    let mut contents = String::new();
                    let mut commons = std::collections::BTreeSet::new();
                    for (key, bytes) in objects.iter() {
                        let Some(rest) = key.strip_prefix(&prefix) else { continue };
                        if let Some(delimiter) = &delimiter {
                            if let Some(at) = rest.find(delimiter.as_str()) {
                                commons.insert(format!("{prefix}{}{delimiter}", &rest[..at]));
                                continue;
                            }
                        }
                        contents.push_str(&format!(
                            "<Contents><Key>{key}</Key><Size>{}</Size><LastModified>2026-09-24T00:00:00.000Z</LastModified></Contents>",
                            bytes.len()
                        ));
                    }
                    let commons = commons
                        .into_iter()
                        .map(|p| format!("<CommonPrefixes><Prefix>{p}</Prefix></CommonPrefixes>"))
                        .collect::<String>();
                    (
                        StatusCode::OK,
                        format!("<?xml version=\"1.0\"?><ListBucketResult><IsTruncated>false</IsTruncated>{contents}{commons}</ListBucketResult>"),
                    )
                }
                async fn object(
                    State(objects): State<Objects>,
                    method: axum::http::Method,
                    Path((_bucket, key)): Path<(String, String)>,
                    headers: HeaderMap,
                    body: Bytes,
                ) -> axum::response::Response {
                    use axum::response::IntoResponse;
                    assert!(headers.get("authorization").is_some(), "every request is signed");
                    let mut objects = objects.lock().unwrap();
                    match method.as_str() {
                        "PUT" => {
                            if let Some(source) = headers.get("x-amz-copy-source") {
                                let source = source.to_str().unwrap();
                                let source_key = source.splitn(3, '/').nth(2).unwrap_or("").to_string();
                                match objects.get(&source_key).cloned() {
                                    Some(bytes) => {
                                        objects.insert(key, bytes);
                                        (StatusCode::OK, "<CopyObjectResult/>").into_response()
                                    }
                                    None => StatusCode::NOT_FOUND.into_response(),
                                }
                            } else {
                                objects.insert(key, body.to_vec());
                                StatusCode::OK.into_response()
                            }
                        }
                        "GET" | "HEAD" => match objects.get(&key) {
                            Some(bytes) => (
                                [("content-length", bytes.len().to_string()), ("last-modified", "Thu, 24 Sep 2026 00:00:00 GMT".to_string())],
                                if method == "HEAD" { Vec::new() } else { bytes.clone() },
                            )
                                .into_response(),
                            None => StatusCode::NOT_FOUND.into_response(),
                        },
                        "DELETE" => {
                            objects.remove(&key);
                            StatusCode::NO_CONTENT.into_response()
                        }
                        _ => StatusCode::METHOD_NOT_ALLOWED.into_response(),
                    }
                }
                let app = Router::new()
                    .route("/{bucket}", get(list))
                    .route("/{bucket}/{*key}", axum::routing::any(object))
                    .with_state(state);
                let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
                port_tx.send(listener.local_addr().unwrap().port()).unwrap();
                axum::serve(listener, app).await.unwrap();
            });
        });
        (port_rx.recv().unwrap(), objects)
    }

    pub(crate) fn store(port: u16, prefix: &str) -> S3ZebFs {
        S3ZebFs::new(S3Config {
            endpoint: format!("http://127.0.0.1:{port}"),
            region: "us-east-1".into(),
            bucket: "test".into(),
            prefix: prefix.into(),
            access_key_id: "ak".into(),
            secret_access_key: "sk".into(),
            path_style: true,
        })
    }

    #[test]
    fn the_seven_verbs_behave_as_they_do_on_disk() {
        let (port, objects) = fake_s3();
        let fs = store(port, "projects/demo");

        // put + get + head, with the prefix applied to the key and hidden from the path
        let stat = fs.put("uploads/a.txt", b"hello").unwrap();
        assert_eq!(stat.path, "uploads/a.txt");
        assert_eq!(stat.size, 5);
        assert_eq!(stat.kind, ZebFsEntryKind::Object);
        assert!(objects.lock().unwrap().contains_key("projects/demo/uploads/a.txt"));
        assert_eq!(fs.get("uploads/a.txt").unwrap().bytes, b"hello");
        assert!(fs.head("uploads/a.txt").unwrap().modified.is_some());

        // a prefix is a head that answers Prefix, and lists its children
        fs.put("uploads/deep/b.txt", b"x").unwrap();
        assert_eq!(fs.head("uploads").unwrap().kind, ZebFsEntryKind::Prefix);
        let names: Vec<(String, ZebFsEntryKind)> = fs
            .list("uploads")
            .unwrap()
            .into_iter()
            .map(|e| (e.name, e.kind))
            .collect();
        assert_eq!(names, vec![("a.txt".to_string(), ZebFsEntryKind::Object), ("deep".to_string(), ZebFsEntryKind::Prefix)]);
        assert_eq!(fs.list("uploads").unwrap()[1].path, "uploads/deep");

        // create_prefix writes a marker the listing hides
        fs.create_prefix("empty").unwrap();
        assert!(objects.lock().unwrap().contains_key("projects/demo/empty/"));
        assert_eq!(fs.head("empty").unwrap().kind, ZebFsEntryKind::Prefix);
        assert!(fs.list("empty").unwrap().is_empty());
        let root: Vec<String> = fs.list("").unwrap().into_iter().map(|e| e.name).collect();
        assert_eq!(root, vec!["empty", "uploads"]);

        // copy, then delete a tree
        fs.copy("uploads/a.txt", "archive/a.txt").unwrap();
        assert_eq!(fs.get("archive/a.txt").unwrap().bytes, b"hello");
        assert_eq!(fs.copy("uploads/missing", "x").unwrap_err().code, "ZEBFS_NOT_FOUND");
        fs.delete("uploads").unwrap();
        assert_eq!(fs.get("uploads/a.txt").unwrap_err().code, "ZEBFS_NOT_FOUND");
        assert_eq!(fs.get("uploads/deep/b.txt").unwrap_err().code, "ZEBFS_NOT_FOUND");
        fs.delete("never-there").unwrap();

        // the reserved document, and the reserved path refused to a user verb
        assert!(fs.read_reserved(".zebfs/acl.json").unwrap().is_none());
        fs.write_reserved(".zebfs/acl.json", b"{}").unwrap();
        assert_eq!(fs.read_reserved(".zebfs/acl.json").unwrap().unwrap(), b"{}");
        assert_eq!(fs.put(".zebfs/acl.json", b"no").unwrap_err().code, "ZEBFS_RESERVED_PATH");
        assert!(fs.list("").unwrap().iter().all(|e| e.name != ".zebfs"));
    }

    #[test]
    fn a_bucket_root_store_has_no_prefix_in_its_keys() {
        let (port, objects) = fake_s3();
        let fs = store(port, "");
        fs.put("public/logo.png", b"png").unwrap();
        assert!(objects.lock().unwrap().contains_key("public/logo.png"));
        assert_eq!(fs.list("").unwrap()[0].name, "public");
        assert_eq!(fs.list("public").unwrap()[0].path, "public/logo.png");
    }

    #[test]
    fn a_store_that_is_not_there_says_so_by_name() {
        let fs = store(1, "");
        let err = fs.get("a").unwrap_err();
        assert_eq!(err.code, "ZEBFS_S3");
        assert!(err.message.contains("cannot reach the store"), "{}", err.message);
    }

    const EXPECTED_SIGNATURE: &str = "5ca375ac8dbc0e2b1ffadba8f005f7238a43b127699269453699634d1732a380";
}
