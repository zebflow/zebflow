//! The error-code registry — `NodeIO`'s status-line half.
//!
//! Every pipeline error code is declared here exactly once with its class,
//! IANA-style: append-only, never renamed, never reused, class fixed at
//! birth. `docs/contracts/kinds/node-io/README.md` is the contract this file
//! implements; the tests below are its enforcement.
//!
//! The two classes are HTTP's 4xx/5xx split, which is the only distinction a
//! machine can act on without reading prose:
//!
//! - [`ErrorClass::Refused`] — the caller's fault: bad config, bad input, a
//!   wrong credential kind. Retrying without changing something is useless.
//! - [`ErrorClass::Failed`] — the world's fault: a relay down, a timeout, a
//!   disk full. Retrying may help, and `logic.retry` retries only these.
//!
//! A code not in the table classifies as `Failed`, which preserves the
//! pre-registry behaviour (retry used to retry everything) for any code the
//! sweep missed — and the coverage test below makes that state temporary: a
//! new code that never registers fails the build.

use serde::{Deserialize, Serialize};

/// Which side of the 4xx/5xx line an error falls on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ErrorClass {
    /// The caller's fault; retrying is useless.
    Refused,
    /// The world's fault; retrying may help.
    Failed,
}

impl ErrorClass {
    /// The status word this class contributes to a `NodeOutput` record.
    pub fn as_status_word(self) -> &'static str {
        match self {
            ErrorClass::Refused => "refused",
            ErrorClass::Failed => "failed",
        }
    }
}

/// The registry. Append-only; a shipped row never changes.
///
/// Initial classification 2026-09-09: mechanical rules over the code's own
/// vocabulary (DUPLICATE/INVALID/MISSING/CONFIG/… ⇒ refused;
/// UNAVAILABLE/TIMEOUT/IO/… ⇒ failed; everything else failed), with explicit
/// rows where a word misleads — `FW_NODE_MAIL_SEND` is failed because the
/// relay may recover, while `FW_NODE_MAIL_TRANSPORT` is refused because the
/// credential named a host or TLS mode that cannot work.
pub const ERROR_CLASS_REGISTRY: &[(&str, ErrorClass)] = &[
    ("FW_DUPLICATE_EDGE", ErrorClass::Refused),
    ("FW_DUPLICATE_NODE", ErrorClass::Refused),
    ("FW_DUPLICATE_PIN", ErrorClass::Refused),
    ("FW_EDGE_FROM_NODE", ErrorClass::Refused),
    ("FW_EDGE_FROM_PIN", ErrorClass::Refused),
    ("FW_EDGE_TO_NODE", ErrorClass::Refused),
    ("FW_EDGE_TO_PIN", ErrorClass::Refused),
    ("FW_EGRESS", ErrorClass::Failed),
    ("FW_EGRESS_DENIED", ErrorClass::Refused),
    ("FW_EGRESS_DNS", ErrorClass::Failed),
    ("FW_EGRESS_UNCHECKED_NODE", ErrorClass::Failed),
    ("FW_EGRESS_UNDECLARED_HOST", ErrorClass::Failed),
    ("FW_EGRESS_URL_INVALID", ErrorClass::Refused),
    ("FW_EMPTY_GRAPH", ErrorClass::Refused),
    ("FW_ENGINE_RUNTIME", ErrorClass::Failed),
    ("FW_ENGINE_SYNC_IN_ASYNC", ErrorClass::Failed),
    ("FW_ENTRY_NODE", ErrorClass::Refused),
    ("FW_EXEC_EDGE", ErrorClass::Refused),
    ("FW_EXEC_NODE", ErrorClass::Failed),
    ("FW_EXPR_COMPILE", ErrorClass::Refused),
    ("FW_EXPR_EVAL", ErrorClass::Refused),
    ("FW_EXPR_ENGINE", ErrorClass::Failed),
    ("FW_EXPR_PARSE", ErrorClass::Refused),
    ("FW_EXPR_RUN", ErrorClass::Refused),
    ("FW_FILE_REF", ErrorClass::Failed),
    ("FW_FILE_REF_BACKEND", ErrorClass::Failed),
    ("FW_FILE_REF_INTEGRITY", ErrorClass::Failed),
    ("FW_FILE_REF_INVALID", ErrorClass::Refused),
    ("FW_FILE_REF_READ", ErrorClass::Failed),
    ("FW_FILE_REF_WRITE", ErrorClass::Failed),
    ("FW_FUNCTION_INPUT_INVALID", ErrorClass::Refused),
    ("FW_FUNCTION_NOT_FOUND", ErrorClass::Refused),
    ("FW_MY_NODE_CODE", ErrorClass::Failed),
    ("FW_NODE_AGENT_BAD_SCHEMA", ErrorClass::Refused),
    ("FW_NODE_AGENT_CALL", ErrorClass::Failed),
    ("FW_NODE_AGENT_CONFIG", ErrorClass::Refused),
    ("FW_NODE_AGENT_CREDENTIAL", ErrorClass::Refused),
    ("FW_NODE_AGENT_INPUT_PIN", ErrorClass::Refused),
    ("FW_NODE_AGENT_QUERY", ErrorClass::Refused),
    ("FW_NODE_AI_TTS_CONFIG", ErrorClass::Refused),
    ("FW_NODE_AUTH_TOKEN_ALGORITHM", ErrorClass::Refused),
    ("FW_NODE_AUTH_TOKEN_CONFIG", ErrorClass::Refused),
    ("FW_NODE_AUTH_TOKEN_CREDENTIAL", ErrorClass::Refused),
    ("FW_NODE_AUTH_TOKEN_CREDENTIAL_KIND", ErrorClass::Refused),
    ("FW_NODE_AUTH_TOKEN_CREDENTIAL_MISSING", ErrorClass::Refused),
    ("FW_NODE_AUTH_TOKEN_KEY_INVALID", ErrorClass::Refused),
    ("FW_NODE_AUTH_TOKEN_KEY_MISSING", ErrorClass::Refused),
    ("FW_NODE_AUTH_TOKEN_SECRET_MISSING", ErrorClass::Refused),
    ("FW_NODE_AUTH_TOKEN_SIGN", ErrorClass::Refused),
    ("FW_NODE_AUTH_TOKEN_UNAVAILABLE", ErrorClass::Refused),
    // Verification. All refused: every one of these is a misconfigured node or
    // credential, and none of them succeeds on a second attempt. A token that
    // simply does not verify is not here at all — that leaves on the `invalid`
    // pin, because a logged-out visitor is an outcome, not an error.
    ("FW_NODE_AUTH_VERIFY_ALGORITHM", ErrorClass::Refused),
    ("FW_NODE_AUTH_VERIFY_CONFIG", ErrorClass::Refused),
    ("FW_NODE_AUTH_VERIFY_CREDENTIAL", ErrorClass::Refused),
    ("FW_NODE_AUTH_VERIFY_CREDENTIAL_KIND", ErrorClass::Refused),
    ("FW_NODE_AUTH_VERIFY_CREDENTIAL_MISSING", ErrorClass::Refused),
    ("FW_NODE_AUTH_VERIFY_KEY", ErrorClass::Refused),
    ("FW_NODE_AUTH_VERIFY_SECRET_MISSING", ErrorClass::Refused),
    ("FW_NODE_AUTH_VERIFY_UNAVAILABLE", ErrorClass::Refused),
    ("FW_NODE_BINDING_COMPILE", ErrorClass::Refused),
    ("FW_NODE_BINDING_EXPR", ErrorClass::Refused),
    ("FW_NODE_BINDING_PARSE", ErrorClass::Refused),
    ("FW_NODE_BINDING_RUN", ErrorClass::Refused),
    ("FW_NODE_BROWSER_RUN_CONFIG", ErrorClass::Refused),
    ("FW_NODE_BROWSER_RUN_CREDENTIAL", ErrorClass::Failed),
    ("FW_NODE_BROWSER_RUN_CREDENTIAL_KIND", ErrorClass::Refused),
    ("FW_NODE_BROWSER_RUN_CREDENTIAL_MISSING", ErrorClass::Refused),
    ("FW_NODE_BROWSER_RUN_HTTP", ErrorClass::Failed),
    ("FW_NODE_BROWSER_RUN_PIN", ErrorClass::Refused),
    ("FW_NODE_BROWSER_RUN_READ", ErrorClass::Failed),
    ("FW_NODE_BROWSER_RUN_SECRET", ErrorClass::Failed),
    ("FW_NODE_BROWSER_RUN_STATUS", ErrorClass::Failed),
    ("FW_NODE_BROWSER_RUN_UNAVAILABLE", ErrorClass::Failed),
    ("FW_NODE_CONCEPT_CONFIG", ErrorClass::Refused),
    ("FW_NODE_CONFIG", ErrorClass::Refused),
    ("FW_NODE_CRYPTO_ARGON2_HASH", ErrorClass::Failed),
    ("FW_NODE_CRYPTO_BASE64_DECODE", ErrorClass::Failed),
    ("FW_NODE_CRYPTO_BASE64_UTF8", ErrorClass::Failed),
    ("FW_NODE_CRYPTO_BCRYPT_HASH", ErrorClass::Failed),
    ("FW_NODE_CRYPTO_CONFIG", ErrorClass::Refused),
    ("FW_NODE_CRYPTO_HMAC_KEY", ErrorClass::Failed),
    ("FW_NODE_CRYPTO_OP", ErrorClass::Failed),
    ("FW_NODE_CRYPTO_SPAWN", ErrorClass::Failed),
    ("FW_NODE_FILE_COMPRESS", ErrorClass::Failed),
    ("FW_NODE_FILE_DECOMPRESS", ErrorClass::Failed),
    ("FW_NODE_FILE_SAVE", ErrorClass::Failed),
    ("FW_NODE_FS_COPY", ErrorClass::Failed),
    ("FW_NODE_FS_DELETE", ErrorClass::Failed),
    ("FW_NODE_FS_GET", ErrorClass::Failed),
    ("FW_NODE_FS_GET_UTF8", ErrorClass::Failed),
    ("FW_NODE_FS_HEAD", ErrorClass::Failed),
    ("FW_NODE_FS_LIST", ErrorClass::Failed),
    ("FW_NODE_FS_MKDIR", ErrorClass::Failed),
    ("FW_NODE_FS_MOVE", ErrorClass::Failed),
    ("FW_NODE_FS_OBJECT", ErrorClass::Failed),
    ("FW_NODE_FS_PUT", ErrorClass::Failed),
    ("FW_NODE_FS_PUT_BASE64", ErrorClass::Failed),
    ("FW_NODE_FS_PUT_JSON", ErrorClass::Failed),
    ("FW_NODE_FS_PUT_SOURCE", ErrorClass::Failed),
    ("FW_NODE_FUNCTION_CALL_NO_PLATFORM", ErrorClass::Failed),
    ("FW_NODE_GEO_CONVERT", ErrorClass::Failed),
    ("FW_NODE_GEO_INSPECT", ErrorClass::Failed),
    ("FW_NODE_HTTP_REQUEST_BINDING", ErrorClass::Refused),
    ("FW_NODE_HTTP_REQUEST_BODY", ErrorClass::Failed),
    ("FW_NODE_HTTP_REQUEST_CLIENT", ErrorClass::Failed),
    ("FW_NODE_HTTP_REQUEST_CONFIG", ErrorClass::Refused),
    ("FW_NODE_HTTP_REQUEST_CREDENTIAL", ErrorClass::Failed),
    ("FW_NODE_HTTP_REQUEST_CREDENTIAL_KIND", ErrorClass::Refused),
    ("FW_NODE_HTTP_REQUEST_CREDENTIAL_MISSING", ErrorClass::Refused),
    ("FW_NODE_HTTP_REQUEST_CREDENTIALS_UNAVAILABLE", ErrorClass::Failed),
    ("FW_NODE_HTTP_REQUEST_EGRESS", ErrorClass::Failed),
    ("FW_NODE_HTTP_REQUEST_FILE_REF", ErrorClass::Failed),
    ("FW_NODE_HTTP_REQUEST_INPUT_PIN", ErrorClass::Refused),
    ("FW_NODE_HTTP_REQUEST_METHOD", ErrorClass::Failed),
    ("FW_NODE_HTTP_REQUEST_OAUTH2", ErrorClass::Failed),
    ("FW_NODE_HTTP_REQUEST_READ_BODY", ErrorClass::Failed),
    ("FW_NODE_HTTP_REQUEST_SECURE_REQUEST", ErrorClass::Failed),
    ("FW_NODE_HTTP_REQUEST_TRANSPORT", ErrorClass::Failed),
    // `fs.svg.convert`: the flags, the SVG, its pictures and its fonts are
    // the author's; only the rasteriser and the store write can fail on their own.
    ("FW_NODE_FS_SVG_CONVERT_CONFIG", ErrorClass::Refused),
    ("FS_SVG_CONVERT_SOURCE", ErrorClass::Refused),
    ("FS_SVG_CONVERT_FONT", ErrorClass::Refused),
    ("FS_SVG_CONVERT_RASTER", ErrorClass::Failed),
    // `fs.image.chromakey`: the key, the source and its limits are the author's.
    ("FW_NODE_FS_IMAGE_CHROMAKEY_CONFIG", ErrorClass::Refused),
    ("FS_IMAGE_CHROMAKEY_SOURCE", ErrorClass::Refused),
    ("FS_IMAGE_CHROMAKEY_RASTER", ErrorClass::Failed),
    ("FS_IMAGE_DECODE", ErrorClass::Refused),
    // The `input.*` family: every one of these is the caller's envelope
    // disagreeing with the pipeline's declaration, so none is retryable.
    ("FW_NODE_INPUT_CONFIG", ErrorClass::Refused),
    ("FW_NODE_INPUT_INVALID", ErrorClass::Refused),
    ("FW_NODE_INPUT_MISSING", ErrorClass::Refused),
    // Raised at activation, not at run time: a required input sits after a
    // trigger that never delivers a body.
    ("FW_NODE_INPUT_UNREACHABLE", ErrorClass::Refused),
    ("FW_NODE_INSTALLED_NO_PLATFORM", ErrorClass::Failed),
    ("FW_NODE_KIND_UNSUPPORTED", ErrorClass::Refused),
    ("FW_NODE_KV_DEL_CONFIG", ErrorClass::Refused),
    ("FW_NODE_KV_EXISTS_CONFIG", ErrorClass::Refused),
    ("FW_NODE_KV_EXPIRE_CONFIG", ErrorClass::Refused),
    ("FW_NODE_KV_GET_CONFIG", ErrorClass::Refused),
    ("FW_NODE_KV_INCR_CONFIG", ErrorClass::Refused),
    ("FW_NODE_KV_PUBLISH_CONFIG", ErrorClass::Refused),
    ("FW_NODE_KV_SET_CONFIG", ErrorClass::Refused),
    ("FW_NODE_KV_SUBSCRIBE_CONFIG", ErrorClass::Refused),
    ("FW_NODE_LOGIC_COLLECT_CONFIG", ErrorClass::Refused),
    ("FW_NODE_LOGIC_FOREACH_CHUNK", ErrorClass::Failed),
    ("FW_NODE_LOGIC_FOREACH_COMPILE", ErrorClass::Failed),
    ("FW_NODE_LOGIC_FOREACH_CONFIG", ErrorClass::Refused),
    ("FW_NODE_LOGIC_FOREACH_EMPTY", ErrorClass::Refused),
    ("FW_NODE_LOGIC_FOREACH_PARSE", ErrorClass::Refused),
    ("FW_NODE_LOGIC_FOREACH_RUN", ErrorClass::Failed),
    ("FW_NODE_LOGIC_FOREACH_TYPE", ErrorClass::Failed),
    ("FW_NODE_LOGIC_IF_COMPILE", ErrorClass::Failed),
    ("FW_NODE_LOGIC_IF_CONFIG", ErrorClass::Refused),
    ("FW_NODE_LOGIC_IF_PARSE", ErrorClass::Refused),
    ("FW_NODE_LOGIC_IF_RUN", ErrorClass::Failed),
    ("FW_NODE_LOGIC_MATCH_COMPILE", ErrorClass::Failed),
    ("FW_NODE_LOGIC_MATCH_CONFIG", ErrorClass::Refused),
    ("FW_NODE_LOGIC_MATCH_PARSE", ErrorClass::Refused),
    ("FW_NODE_LOGIC_MATCH_RUN", ErrorClass::Failed),
    ("FW_NODE_LOGIC_REDUCE_COMPILE", ErrorClass::Failed),
    ("FW_NODE_LOGIC_REDUCE_CONFIG", ErrorClass::Refused),
    ("FW_NODE_LOGIC_REDUCE_PARSE", ErrorClass::Refused),
    ("FW_NODE_LOGIC_REDUCE_RUN", ErrorClass::Failed),
    ("FW_NODE_LOGIC_RETRY_CONFIG", ErrorClass::Refused),
    ("FW_NODE_LOGIC_RETRY_INPUT", ErrorClass::Failed),
    ("FW_NODE_LOGIC_RETRY_WHEN_COMPILE", ErrorClass::Failed),
    ("FW_NODE_LOGIC_RETRY_WHEN_PARSE", ErrorClass::Refused),
    ("FW_NODE_LOGIC_RETRY_WHEN_RUN", ErrorClass::Failed),
    ("FW_NODE_MAIL_ADDRESS", ErrorClass::Refused),
    ("FW_NODE_MAIL_BUILD", ErrorClass::Refused),
    ("FW_NODE_MAIL_CONFIG", ErrorClass::Refused),
    ("FW_NODE_MAIL_CREDENTIAL", ErrorClass::Refused),
    ("FW_NODE_MAIL_CREDENTIAL_KIND", ErrorClass::Refused),
    ("FW_NODE_MAIL_CREDENTIAL_MISSING", ErrorClass::Refused),
    ("FW_NODE_MAIL_SEND", ErrorClass::Failed),
    ("FW_NODE_MAIL_TRANSPORT", ErrorClass::Refused),
    ("FW_NODE_MAIL_UNAVAILABLE", ErrorClass::Failed),
    ("FW_NODE_MANUAL_CONFIG", ErrorClass::Refused),
    ("FW_NODE_MCP_TRIGGER_CONFIG", ErrorClass::Refused),
    ("FW_NODE_MEM_UNAVAILABLE", ErrorClass::Failed),
    ("FW_NODE_MS", ErrorClass::Failed),
    ("FW_NODE_MS_GET", ErrorClass::Failed),
    ("FW_NODE_MS_PARSE", ErrorClass::Refused),
    ("FW_NODE_MS_PUBLISH", ErrorClass::Failed),
    ("FW_NODE_MS_UNPUBLISH", ErrorClass::Failed),
    ("FW_NODE_MS_WRITE", ErrorClass::Failed),
    ("FW_NODE_OUTPUT_FILE", ErrorClass::Failed),
    ("FW_NODE_OUTPUT_FILE_BASE64", ErrorClass::Failed),
    ("FW_NODE_OUTPUT_FILE_CONTEXT", ErrorClass::Failed),
    ("FW_NODE_OUTPUT_FILE_ENCODING", ErrorClass::Failed),
    ("FW_NODE_OUTPUT_FILE_JSON", ErrorClass::Failed),
    ("FW_NODE_OUTPUT_FILE_TOO_LARGE", ErrorClass::Refused),
    ("FW_NODE_OUTPUT_FILE_WRITE", ErrorClass::Failed),
    ("FW_NODE_PACKAGE_NOT_EXECUTABLE", ErrorClass::Failed),
    ("FW_NODE_PACKAGE_NOT_FOUND", ErrorClass::Refused),
    ("FW_NODE_PDF_CONVERT", ErrorClass::Failed),
    ("FW_NODE_PG_BINDING", ErrorClass::Refused),
    ("FW_NODE_PG_CONFIG", ErrorClass::Refused),
    ("FW_NODE_PG_CONNECT", ErrorClass::Failed),
    ("FW_NODE_PG_CREDENTIAL", ErrorClass::Failed),
    ("FW_NODE_PG_CREDENTIAL_KIND", ErrorClass::Refused),
    ("FW_NODE_PG_CREDENTIAL_MISSING", ErrorClass::Refused),
    ("FW_NODE_PG_QUERY", ErrorClass::Failed),
    ("FW_NODE_PG_SECRET", ErrorClass::Failed),
    ("FW_NODE_PG_UNAVAILABLE", ErrorClass::Failed),
    ("FW_NODE_RUNTIME", ErrorClass::Failed),
    ("FW_NODE_SCHEDULE_CONFIG", ErrorClass::Refused),
    ("FW_NODE_SCOPE", ErrorClass::Failed),
    ("FW_NODE_SCRIPT_BINDING", ErrorClass::Refused),
    ("FW_NODE_SCRIPT_COMPILE", ErrorClass::Failed),
    ("FW_NODE_SCRIPT_CONFIG", ErrorClass::Refused),
    ("FW_NODE_SCRIPT_INPUT_PIN", ErrorClass::Refused),
    ("FW_NODE_SCRIPT_PARSE", ErrorClass::Refused),
    ("FW_NODE_SCRIPT_REJECTED", ErrorClass::Refused),
    ("FW_NODE_SCRIPT_RUN", ErrorClass::Failed),
    ("FW_NODE_SEKEJAP_INSERT", ErrorClass::Failed),
    ("FW_NODE_SEKEJAP_INSERT_CONFIG", ErrorClass::Refused),
    ("FW_NODE_SEKEJAP_INSERT_INPUT", ErrorClass::Failed),
    ("FW_NODE_SEKEJAP_INSERT_JOIN", ErrorClass::Failed),
    ("FW_NODE_SEKEJAP_INSERT_LIMIT", ErrorClass::Refused),
    ("FW_NODE_SEKEJAP_QUERY", ErrorClass::Failed),
    ("FW_NODE_SEKEJAP_QUERY_BINDING", ErrorClass::Refused),
    ("FW_NODE_SEKEJAP_QUERY_CONFIG", ErrorClass::Refused),
    ("FW_NODE_SEKEJAP_QUERY_JOIN", ErrorClass::Failed),
    ("FW_NODE_SEKEJAP_UNAVAILABLE", ErrorClass::Failed),
    ("FW_NODE_SQLITE_MUTATE", ErrorClass::Failed),
    ("FW_NODE_SQLITE_MUTATE_CONFIG", ErrorClass::Refused),
    ("FW_NODE_SQLITE_MUTATE_MIGRATE", ErrorClass::Failed),
    ("FW_NODE_SQLITE_QUERY", ErrorClass::Failed),
    ("FW_NODE_SQLITE_QUERY_CONFIG", ErrorClass::Refused),
    ("FW_NODE_SQLITE_QUERY_MIGRATE", ErrorClass::Failed),
    ("FW_NODE_SQLITE_UNAVAILABLE", ErrorClass::Failed),
    ("FW_NODE_SYNC_IN_ASYNC", ErrorClass::Failed),
    ("FW_NODE_TABLE_CONVERT", ErrorClass::Failed),
    ("FW_NODE_TABLE_CONVERT_CSV", ErrorClass::Failed),
    ("FW_NODE_TABLE_CONVERT_LIMIT", ErrorClass::Refused),
    ("FW_NODE_TABLE_CONVERT_PARQUET", ErrorClass::Failed),
    ("FW_NODE_TABLE_QUERY", ErrorClass::Failed),
    ("FW_NODE_TABLE_QUERY_CONFIG", ErrorClass::Refused),
    ("FW_NODE_TABLE_QUERY_ENGINE", ErrorClass::Failed),
    ("FW_NODE_TABLE_QUERY_EXECUTE", ErrorClass::Failed),
    ("FW_NODE_TABLE_QUERY_LIMIT", ErrorClass::Refused),
    ("FW_NODE_TABLE_QUERY_PREPARE", ErrorClass::Failed),
    ("FW_NODE_TABLE_QUERY_REGISTER", ErrorClass::Failed),
    ("FW_NODE_TABLE_QUERY_SOURCE", ErrorClass::Failed),
    ("FW_NODE_TABLE_QUERY_SQL", ErrorClass::Failed),
    ("FW_NODE_TABLE_QUERY_UNAVAILABLE", ErrorClass::Failed),
    ("FW_NODE_TIMEOUT", ErrorClass::Failed),
    ("FW_NODE_WASM_ABI", ErrorClass::Failed),
    ("FW_NODE_WASM_ALLOC", ErrorClass::Failed),
    ("FW_NODE_WASM_ALLOC_CALL", ErrorClass::Failed),
    ("FW_NODE_WASM_COMPILE", ErrorClass::Failed),
    ("FW_NODE_WASM_CONTEXT", ErrorClass::Failed),
    ("FW_NODE_WASM_INPUT_TOO_LARGE", ErrorClass::Refused),
    ("FW_NODE_WASM_INSTANTIATE", ErrorClass::Failed),
    ("FW_NODE_WASM_JOIN", ErrorClass::Failed),
    ("FW_NODE_WASM_JSON_INPUT", ErrorClass::Failed),
    ("FW_NODE_WASM_JSON_OUTPUT", ErrorClass::Failed),
    ("FW_NODE_WASM_MEMORY", ErrorClass::Failed),
    ("FW_NODE_WASM_MODULE", ErrorClass::Failed),
    ("FW_NODE_WASM_MODULE_TOO_LARGE", ErrorClass::Refused),
    ("FW_NODE_WASM_OUTPUT_TOO_LARGE", ErrorClass::Refused),
    ("FW_NODE_WASM_PACKAGE_NOT_FOUND", ErrorClass::Refused),
    ("FW_NODE_WASM_PATH", ErrorClass::Failed),
    ("FW_NODE_WASM_RUN", ErrorClass::Failed),
    ("FW_NODE_WASM_RUN_CALL", ErrorClass::Failed),
    ("FW_NODE_WASM_RUNTIME", ErrorClass::Failed),
    ("FW_NODE_WASM_SOURCE", ErrorClass::Failed),
    ("FW_NODE_WEB_DOCS_CONFIG", ErrorClass::Refused),
    ("FW_NODE_WEB_DOCS_RENDER", ErrorClass::Failed),
    ("FW_NODE_WEB_DOCS_TEMPLATE_ROOT", ErrorClass::Failed),
    ("FW_NODE_WEB_DOCS_UNAVAILABLE", ErrorClass::Failed),
    ("FW_NODE_WEB_RENDER_COMPILE", ErrorClass::Failed),
    ("FW_NODE_WEB_RESPONSE_CONFIG", ErrorClass::Refused),
    ("FW_NODE_WEB_STATIC_CONFIG", ErrorClass::Refused),
    ("FW_NODE_WEB_STATIC_OUTPUT_NAME", ErrorClass::Failed),
    ("FW_NODE_WEB_STATIC_RENDER", ErrorClass::Failed),
    ("FW_NODE_WEB_STATIC_UNAVAILABLE", ErrorClass::Failed),
    ("FW_NODE_WEBERROR_CONFIG", ErrorClass::Refused),
    ("FW_NODE_WEBHOOK_CONFIG", ErrorClass::Refused),
    ("FW_NODE_WS_CLIENT_SEND_CONFIG", ErrorClass::Refused),
    ("FW_NODE_WS_CLIENT_SEND_UNAVAILABLE", ErrorClass::Failed),
    ("FW_NODE_WS_CLIENT_TRIGGER_CONFIG", ErrorClass::Refused),
    ("FW_NODE_WS_EMIT_CONFIG", ErrorClass::Refused),
    ("FW_NODE_WS_EMIT_UNAVAILABLE", ErrorClass::Failed),
    ("FW_NODE_WS_SYNC_STATE_CONFIG", ErrorClass::Refused),
    ("FW_NODE_WS_SYNC_STATE_UNAVAILABLE", ErrorClass::Failed),
    ("FW_NODE_WS_TRIGGER_CONFIG", ErrorClass::Refused),
    ("FW_NODES_SCOPE_DYNAMIC", ErrorClass::Failed),
    ("FW_PIPELINE_CONTRACT", ErrorClass::Failed),
    ("FW_PIPELINE_ID", ErrorClass::Failed),
    ("FW_PIPELINE_IDENTIFIER", ErrorClass::Failed),
    ("FW_PIPELINE_LIMIT", ErrorClass::Refused),
    ("FW_PIPELINE_METADATA", ErrorClass::Failed),
    ("FW_PIPELINE_RETENTION", ErrorClass::Failed),
    ("FW_PIPELINE_TEXT", ErrorClass::Failed),
    ("FW_PIPELINE_TRACE_CAPTURE", ErrorClass::Failed),
    ("FW_TRACE_CONFIG", ErrorClass::Refused),
    ("FW_WS_CLIENT_SEND", ErrorClass::Failed),
    ("FW_WS_EMIT_NO_ROOM", ErrorClass::Failed),
    ("FW_WS_SYNC_STATE_NO_ROOM", ErrorClass::Failed),

    // The language namespace. These were emitted for as long as the sandbox
    // has existed and registered nowhere, because the coverage test below only
    // scanned for `"FW_`. An unregistered code defaults to Failed, so a policy
    // violation that cannot possibly succeed on a second attempt was being
    // handed to logic.retry as though the world had merely hiccuped.
    ("LANG_DENO_SYNTAX", ErrorClass::Refused),
    ("LANG_DENO_POLICY", ErrorClass::Refused),
    ("LANG_DENO_SOURCE_TOO_LARGE", ErrorClass::Refused),
    ("LANG_DENO_RUN_PATCH", ErrorClass::Refused),
    // A script that threw, timed out, or exhausted its budget. Kept Failed:
    // the cause is not distinguishable here today, and treating a contended
    // timeout as permanent would be the more expensive mistake. Splitting this
    // into typed causes is tracked work.
    ("LANG_DENO_RUN", ErrorClass::Failed),
    ("LANG_DENO_COMPILE_INPUT", ErrorClass::Failed),
    ("LANG_DENO_COMPILE_ENCODE", ErrorClass::Failed),
    ("LANG_DENO_ARTIFACT_DECODE", ErrorClass::Failed),
    // The noop engine, found by widening the scan above — nobody had looked
    // for these either.
    ("LANG_PARSE_TPJSON", ErrorClass::Refused),
    ("LANG_COMPILE", ErrorClass::Failed),
    ("LANG_RUN_DECODE", ErrorClass::Failed),
];

/// The class of one code. Unregistered codes are `Failed` — see module docs.
/// Picks a pipeline error code that preserves the class of a language error.
///
/// The seam between the language engine and the pipeline used to flatten every
/// cause into one wrapper: `FW_NODE_SCRIPT_COMPILE` is `Failed`, so a syntax
/// error — which cannot possibly succeed on a second attempt — was handed to
/// `logic.retry` as a transient fault. Passing the cause's own class through
/// keeps `refused` meaning what the NodeIO contract says it means.
pub fn wrapper_for(language_code: &str, refused: &'static str, failed: &'static str) -> &'static str {
    match class_of(language_code) {
        ErrorClass::Refused => refused,
        ErrorClass::Failed => failed,
    }
}

pub fn class_of(code: &str) -> ErrorClass {
    ERROR_CLASS_REGISTRY
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, class)| *class)
        .unwrap_or(ErrorClass::Failed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    /// A code is declared once. A duplicate row is how a class silently
    /// changes, so it fails the build.
    #[test]
    fn every_code_is_declared_exactly_once() {
        let mut seen = HashSet::new();
        for (code, _) in ERROR_CLASS_REGISTRY {
            assert!(seen.insert(*code), "duplicate registry row: {code}");
        }
    }

    /// Every `FW_*` code used anywhere in the source is registered.
    ///
    /// This is the append-only registry's teeth: writing a new error code
    /// without declaring its class fails here, with the file it appeared in.
    #[test]
    fn every_code_in_the_source_is_registered() {
        let registered: HashSet<&str> =
            ERROR_CLASS_REGISTRY.iter().map(|(c, _)| *c).collect();
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut missing = Vec::new();
        let mut stack = vec![root];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).expect("read src") {
                let path = entry.expect("entry").path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|e| e == "rs")
                    && !path.ends_with("error_class.rs")
                {
                    let text = std::fs::read_to_string(&path).expect("read file");
                    let bytes = text.as_bytes();
                    // Both namespaces. Scanning only `"FW_` is why eight
                    // `LANG_DENO_*` codes went unregistered for the whole life
                    // of the sandbox while this test stayed green — the module
                    // header promised that an unregistered code fails the
                    // build, and that promise covered one prefix.
                    for prefix in ["\"FW_", "\"LANG_"] {
                    let mut i = 0;
                    while let Some(pos) = text[i..].find(prefix) {
                        let start = i + pos + 1;
                        let mut end = start;
                        while end < bytes.len()
                            && (bytes[end].is_ascii_uppercase()
                                || bytes[end].is_ascii_digit()
                                || bytes[end] == b'_')
                        {
                            end += 1;
                        }
                        let code = &text[start..end];
                        if code.len() > 3
                            && bytes.get(end) == Some(&b'"')
                            && !registered.contains(code)
                        {
                            missing.push(format!("{code} ({})", path.display()));
                        }
                        i = end;
                    }
                    }
                }
            }
        }
        missing.sort();
        missing.dedup();
        assert!(
            missing.is_empty(),
            "codes used but never registered — add each to ERROR_CLASS_REGISTRY \
             with its class:\n{}",
            missing.join("\n")
        );
    }

    #[test]
    fn the_mail_family_splits_where_the_relay_might_recover() {
        assert_eq!(class_of("FW_NODE_MAIL_ADDRESS"), ErrorClass::Refused);
        assert_eq!(class_of("FW_NODE_MAIL_SEND"), ErrorClass::Failed);
        assert_eq!(class_of("FW_NODE_MAIL_CREDENTIAL_KIND"), ErrorClass::Refused);
    }
}
