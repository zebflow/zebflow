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
    ("FW_NODE_AI_AGENT_BAD_SCHEMA", ErrorClass::Refused),
    // A store a node or a FileRef names cannot be opened.
    ("FW_NODE_STORE", ErrorClass::Failed),
    ("FW_NODE_AI_AGENT_CALL", ErrorClass::Failed),
    ("FW_NODE_AI_AGENT_CONFIG", ErrorClass::Refused),
    ("FW_NODE_AI_AGENT_CREDENTIAL", ErrorClass::Refused),
    ("FW_NODE_AI_AGENT_INPUT_PIN", ErrorClass::Refused),
    ("FW_NODE_AI_AGENT_QUERY", ErrorClass::Refused),
    ("FW_NODE_AI_TTS_CONFIG", ErrorClass::Refused),
    ("FW_NODE_AUTH_TOKEN_ALGORITHM", ErrorClass::Refused),
    ("FW_NODE_AUTH_TOKEN_CONFIG", ErrorClass::Refused),
    ("FW_NODE_AUTH_CLAIM_PUBLIC_ON_VALUE", ErrorClass::Refused),
    ("FW_NODE_AUTH_CLAIM_NAME", ErrorClass::Refused),
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
    ("FW_NODE_CRYPTO_BASE64_DECODE_CONFIG", ErrorClass::Refused),
    ("FW_NODE_CRYPTO_BASE64_DECODE_EMPTY", ErrorClass::Refused),
    ("FW_NODE_CRYPTO_BASE64_DECODE_INVALID", ErrorClass::Refused),
    ("FW_NODE_CRYPTO_BASE64_ENCODE_CONFIG", ErrorClass::Refused),
    ("FW_NODE_CRYPTO_BASE64_ENCODE_EMPTY", ErrorClass::Refused),
    ("FW_NODE_CRYPTO_DIGEST_CREATE_CONFIG", ErrorClass::Refused),
    ("FW_NODE_CRYPTO_DIGEST_CREATE_EMPTY", ErrorClass::Refused),
    ("FW_NODE_CRYPTO_PASSWORD_HASH", ErrorClass::Failed),
    ("FW_NODE_CRYPTO_PASSWORD_HASH_CONFIG", ErrorClass::Refused),
    ("FW_NODE_CRYPTO_PASSWORD_HASH_EMPTY", ErrorClass::Refused),
    ("FW_NODE_CRYPTO_PASSWORD_VERIFY", ErrorClass::Failed),
    ("FW_NODE_CRYPTO_PASSWORD_VERIFY_CONFIG", ErrorClass::Refused),
    ("FW_NODE_CRYPTO_PASSWORD_VERIFY_EMPTY", ErrorClass::Refused),
    ("FW_NODE_CRYPTO_PASSWORD_VERIFY_FORMAT", ErrorClass::Refused),
    ("FW_NODE_CRYPTO_RANDOM_GENERATE_CONFIG", ErrorClass::Refused),
    ("FW_NODE_CRYPTO_SIGNATURE_SIGN", ErrorClass::Failed),
    ("FW_NODE_CRYPTO_SIGNATURE_SIGN_CONFIG", ErrorClass::Refused),
    ("FW_NODE_CRYPTO_SIGNATURE_SIGN_CREDENTIAL", ErrorClass::Refused),
    ("FW_NODE_CRYPTO_SIGNATURE_SIGN_EMPTY", ErrorClass::Refused),
    ("FW_NODE_CRYPTO_SIGNATURE_VERIFY", ErrorClass::Failed),
    ("FW_NODE_CRYPTO_SIGNATURE_VERIFY_CONFIG", ErrorClass::Refused),
    ("FW_NODE_CRYPTO_SIGNATURE_VERIFY_CREDENTIAL", ErrorClass::Refused),
    ("FW_NODE_CRYPTO_SIGNATURE_VERIFY_EMPTY", ErrorClass::Refused),
    ("FW_NODE_FS_COMPRESS", ErrorClass::Failed),
    ("FW_NODE_FS_DECOMPRESS", ErrorClass::Failed),
    ("FW_NODE_FS_COPY", ErrorClass::Failed),
    ("FW_NODE_FS_DELETE", ErrorClass::Failed),
    ("FW_NODE_FS_GET", ErrorClass::Failed),
    ("FW_NODE_FS_GET_UTF8", ErrorClass::Failed),
    ("FW_NODE_FS_HEAD", ErrorClass::Failed),
    ("FW_NODE_FS_LIST", ErrorClass::Failed),
    ("FW_NODE_FS_MKDIR", ErrorClass::Failed),
    ("FW_NODE_FS_MOVE", ErrorClass::Failed),
    ("FW_NODE_FS_OBJECT", ErrorClass::Failed),
    ("FW_NODE_FS_FILE_PUT", ErrorClass::Failed),
    ("FW_NODE_FS_FILE_PUT_ACCEPT", ErrorClass::Refused),
    ("FW_NODE_FS_FILE_PUT_CONFIG", ErrorClass::Refused),
    ("FW_NODE_FS_FILE_PUT_ENCODING", ErrorClass::Refused),
    ("FW_NODE_FS_FILE_PUT_EXTENSION", ErrorClass::Refused),
    ("FW_NODE_FS_FILE_PUT_SIZE", ErrorClass::Refused),
    ("FW_NODE_FS_FILE_PUT_SOURCE", ErrorClass::Refused),
    ("FW_NODE_FUNCTION_RESULT_CALL_CONFIG", ErrorClass::Refused),
    ("FW_NODE_FUNCTION_RESULT_CALL_NO_PLATFORM", ErrorClass::Failed),
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
    // `fs.image.render`: the flags, the SVG, its pictures and its fonts are
    // the author's; only the rasteriser and the store write can fail on their own.
    ("FW_NODE_FS_SVG_CONVERT_CONFIG", ErrorClass::Refused),
    ("FW_NODE_FS_SVG_CONVERT_SOURCE", ErrorClass::Refused),
    ("FW_NODE_FS_SVG_CONVERT_FONT", ErrorClass::Refused),
    ("FW_NODE_FS_SVG_CONVERT_RASTER", ErrorClass::Failed),
    // `fs.image.chromakey`: the key, the source and its limits are the author's.
    ("FW_NODE_FS_IMAGE_CHROMAKEY_CONFIG", ErrorClass::Refused),
    ("FW_NODE_FS_IMAGE_CHROMAKEY_SOURCE", ErrorClass::Refused),
    ("FW_NODE_FS_IMAGE_CHROMAKEY_RASTER", ErrorClass::Failed),
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
    ("FW_NODE_KV_DEL_KEY", ErrorClass::Refused),
    ("FW_NODE_KV_EXISTS_KEY", ErrorClass::Refused),
    ("FW_NODE_KV_EXPIRE_KEY", ErrorClass::Refused),
    ("FW_NODE_KV_GET_KEY", ErrorClass::Refused),
    ("FW_NODE_KV_INCR_AMOUNT", ErrorClass::Refused),
    ("FW_NODE_KV_INCR_KEY", ErrorClass::Refused),
    ("FW_NODE_KV_PUBLISH_CHANNEL", ErrorClass::Refused),
    ("FW_NODE_KV_SET_KEY", ErrorClass::Refused),
    ("FW_NODE_KV_PUBLISH_CONFIG", ErrorClass::Refused),
    ("FW_NODE_KV_SET_CONFIG", ErrorClass::Refused),
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
    // The relay's own two refusals, which are not the same refusal. A 4xx
    // is "not now" — greylisting says this, and the answer is to try again.
    // A rejected password is not going to become right by itself.
    ("FW_NODE_MAIL_DEFERRED", ErrorClass::Failed),
    ("FW_NODE_MAIL_AUTH", ErrorClass::Refused),
    // A path that names no stored file is the author's to fix.
    ("FW_NODE_MAIL_ATTACH", ErrorClass::Refused),
    ("FW_NODE_MAIL_BUILD", ErrorClass::Refused),
    ("FW_NODE_MAIL_CONFIG", ErrorClass::Refused),
    ("FW_NODE_MAIL_CREDENTIAL", ErrorClass::Refused),
    ("FW_NODE_MAIL_CREDENTIAL_KIND", ErrorClass::Refused),
    ("FW_NODE_MAIL_CREDENTIAL_MISSING", ErrorClass::Refused),
    ("FW_NODE_MAIL_SEND", ErrorClass::Failed),
    ("FW_NODE_MAIL_TRANSPORT", ErrorClass::Refused),
    ("FW_NODE_MAIL_UNAVAILABLE", ErrorClass::Failed),
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
    ("FW_NODE_FS_PDF_CONVERT", ErrorClass::Failed),
    // `pg.query.run`: the credential, flags and a write without --write are
    // the author's; the server and the network can recover.
    ("FW_NODE_PG_QUERY_RUN", ErrorClass::Failed),
    ("FW_NODE_PG_QUERY_RUN_CONFIG", ErrorClass::Refused),
    ("FW_NODE_PG_QUERY_RUN_CONNECT", ErrorClass::Failed),
    ("FW_NODE_PG_QUERY_RUN_CREDENTIAL", ErrorClass::Refused),
    ("FW_NODE_PG_QUERY_RUN_LIMIT", ErrorClass::Refused),
    ("FW_NODE_PG_QUERY_RUN_PARAM", ErrorClass::Refused),
    ("FW_NODE_PG_QUERY_RUN_UNAVAILABLE", ErrorClass::Failed),
    ("FW_NODE_PG_QUERY_RUN_WRITE", ErrorClass::Refused),
    ("FW_NODE_RUNTIME", ErrorClass::Failed),
    ("FW_NODE_SCOPE", ErrorClass::Failed),
    ("FW_NODE_SCRIPT_BINDING", ErrorClass::Refused),
    ("FW_NODE_SCRIPT_COMPILE", ErrorClass::Failed),
    ("FW_NODE_SCRIPT_CONFIG", ErrorClass::Refused),
    ("FW_NODE_SCRIPT_INPUT_PIN", ErrorClass::Refused),
    ("FW_NODE_SCRIPT_PARSE", ErrorClass::Refused),
    ("FW_NODE_SCRIPT_REJECTED", ErrorClass::Refused),
    ("FW_NODE_SCRIPT_RUN", ErrorClass::Failed),
    // `sekejap.record.create` and `sekejap.query.run`.
    ("FW_NODE_SEKEJAP_QUERY_RUN", ErrorClass::Failed),
    ("FW_NODE_SEKEJAP_QUERY_RUN_CONFIG", ErrorClass::Refused),
    ("FW_NODE_SEKEJAP_QUERY_RUN_LIMIT", ErrorClass::Refused),
    ("FW_NODE_SEKEJAP_QUERY_RUN_PARAM", ErrorClass::Refused),
    ("FW_NODE_SEKEJAP_QUERY_RUN_UNAVAILABLE", ErrorClass::Failed),
    ("FW_NODE_SEKEJAP_QUERY_RUN_WRITE", ErrorClass::Refused),
    ("FW_NODE_SEKEJAP_RECORD_CREATE", ErrorClass::Failed),
    ("FW_NODE_SEKEJAP_RECORD_CREATE_CONFIG", ErrorClass::Refused),
    ("FW_NODE_SEKEJAP_RECORD_CREATE_INPUT", ErrorClass::Refused),
    ("FW_NODE_SEKEJAP_RECORD_CREATE_MAX_ITEMS", ErrorClass::Refused),
    ("FW_NODE_SEKEJAP_RECORD_CREATE_UNAVAILABLE", ErrorClass::Failed),
    // `sqlite.query.run`, reads and writes in one kind.
    ("FW_NODE_SQLITE_QUERY_RUN", ErrorClass::Failed),
    ("FW_NODE_SQLITE_QUERY_RUN_CONFIG", ErrorClass::Refused),
    ("FW_NODE_SQLITE_QUERY_RUN_LIMIT", ErrorClass::Refused),
    ("FW_NODE_SQLITE_QUERY_RUN_MIGRATE", ErrorClass::Failed),
    ("FW_NODE_SQLITE_QUERY_RUN_PARAM", ErrorClass::Refused),
    ("FW_NODE_SQLITE_QUERY_RUN_UNAVAILABLE", ErrorClass::Failed),
    ("FW_NODE_SQLITE_QUERY_RUN_WRITE", ErrorClass::Refused),
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
    ("FW_NODE_TRIGGER_ERROR_CONFIG", ErrorClass::Refused),
    ("FW_NODE_TRIGGER_MANUAL_CONFIG", ErrorClass::Refused),
    ("FW_NODE_TRIGGER_MCP_CONFIG", ErrorClass::Refused),
    ("FW_NODE_TRIGGER_ROOM_CONFIG", ErrorClass::Refused),
    ("FW_NODE_TRIGGER_SCHEDULE_CONFIG", ErrorClass::Refused),
    ("FW_NODE_TRIGGER_SOCKET_CONFIG", ErrorClass::Refused),
    ("FW_NODE_TRIGGER_TOPIC_CONFIG", ErrorClass::Refused),
    ("FW_NODE_TRIGGER_WEBHOOK_CONFIG", ErrorClass::Refused),
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
    ("FW_NODE_WEB_RENDER_COMPILE", ErrorClass::Failed),
    ("FW_NODE_WS_MESSAGE_SEND_CONFIG", ErrorClass::Refused),
    ("FW_NODE_WS_MESSAGE_SEND_CONNECTION", ErrorClass::Failed),
    ("FW_NODE_WS_MESSAGE_SEND_EMPTY", ErrorClass::Refused),
    ("FW_NODE_WS_MESSAGE_SEND_ROOM", ErrorClass::Refused),
    ("FW_NODE_WS_MESSAGE_SEND_SESSION", ErrorClass::Refused),
    ("FW_NODE_WS_MESSAGE_SEND_UNAVAILABLE", ErrorClass::Failed),
    ("FW_NODE_WS_STATE_DELETE_CONFIG", ErrorClass::Refused),
    ("FW_NODE_WS_STATE_DELETE_KEY", ErrorClass::Refused),
    ("FW_NODE_WS_STATE_DELETE_ROOM", ErrorClass::Refused),
    ("FW_NODE_WS_STATE_DELETE_UNAVAILABLE", ErrorClass::Failed),
    ("FW_NODE_WS_STATE_PUT_CONFIG", ErrorClass::Refused),
    ("FW_NODE_WS_STATE_PUT_EMPTY", ErrorClass::Refused),
    ("FW_NODE_WS_STATE_PUT_KEY", ErrorClass::Refused),
    ("FW_NODE_WS_STATE_PUT_ROOM", ErrorClass::Refused),
    ("FW_NODE_WS_STATE_PUT_UNAVAILABLE", ErrorClass::Failed),
    ("FW_NODE_WS_STATE_UPDATE_CONFIG", ErrorClass::Refused),
    ("FW_NODE_WS_STATE_UPDATE_EMPTY", ErrorClass::Refused),
    ("FW_NODE_WS_STATE_UPDATE_KEY", ErrorClass::Refused),
    ("FW_NODE_WS_STATE_UPDATE_ROOM", ErrorClass::Refused),
    ("FW_NODE_WS_STATE_UPDATE_UNAVAILABLE", ErrorClass::Failed),
    ("FW_NODE_WS_STATE_UPDATE_VALUE", ErrorClass::Refused),
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

    // The store namespace. A `ZebFsError` converts into a `PipelineError`
    // keeping its own code, so the store's reason reaches the caller by name;
    // these say which reasons are the author's to fix.
    // The path names no object, is malformed, is the reserved metadata prefix,
    // or asks a bucket project for an operation that streams from local disk:
    // none of these succeeds on a second attempt.
    ("ZEBFS_NOT_FOUND", ErrorClass::Refused),
    ("ZEBFS_INVALID_PATH", ErrorClass::Refused),
    ("ZEBFS_RESERVED_PATH", ErrorClass::Refused),
    ("ZEBFS_UNKNOWN_BACKEND", ErrorClass::Refused),
    ("ZEBFS_S3_CREDENTIAL", ErrorClass::Refused),
    ("ZEBFS_READ_ONLY", ErrorClass::Refused),
    // Disk and the bucket's own answers: a retry may well succeed.
    ("ZEBFS_IO", ErrorClass::Failed),
    ("ZEBFS_S3", ErrorClass::Failed),
    ("ZEBFS_ACL_READ", ErrorClass::Failed),
    ("ZEBFS_ACL_WRITE", ErrorClass::Failed),
    // Node codes moved under FW_NODE_ by docs/contracts/node-conventions.md §6 (2026-10-03).
    ("FW_NODE_AI_TTS_BLOB", ErrorClass::Failed),
    ("FW_NODE_AI_TTS_CREDENTIAL", ErrorClass::Refused),
    ("FW_NODE_AI_TTS_CREDENTIAL_KIND", ErrorClass::Refused),
    ("FW_NODE_AI_TTS_CREDENTIAL_SECRET", ErrorClass::Refused),
    ("FW_NODE_AI_TTS_CREDENTIALS", ErrorClass::Failed),
    ("FW_NODE_AI_TTS_ESPEAK", ErrorClass::Refused),
    ("FW_NODE_AI_TTS_FILE", ErrorClass::Failed),
    ("FW_NODE_AI_TTS_LAYOUT", ErrorClass::Failed),
    ("FW_NODE_AI_TTS_MODEL", ErrorClass::Refused),
    ("FW_NODE_AI_TTS_OUTPUT_PATH", ErrorClass::Refused),
    ("FW_NODE_AI_TTS_PATH", ErrorClass::Refused),
    ("FW_NODE_AI_TTS_PIPER", ErrorClass::Failed),
    ("FW_NODE_AI_TTS_PLATFORM", ErrorClass::Failed),
    ("FW_NODE_AI_TTS_PROVIDER", ErrorClass::Refused),
    ("FW_NODE_AI_TTS_PROVIDER_MISMATCH", ErrorClass::Refused),
    ("FW_NODE_AI_TTS_SPEED", ErrorClass::Refused),
    ("FW_NODE_AI_TTS_TEXT", ErrorClass::Refused),
    // `fs.barcode.render`: the flags and the text are the author's; only the
    // store write and the PNG encoder can fail on their own.
    ("FW_NODE_FS_BARCODE_RENDER", ErrorClass::Failed),
    ("FW_NODE_FS_BARCODE_RENDER_CONFIG", ErrorClass::Refused),
    ("FW_NODE_FS_BARCODE_RENDER_SIZE", ErrorClass::Refused),
    ("FW_NODE_FS_BARCODE_RENDER_TEXT", ErrorClass::Refused),
    ("FW_NODE_FS_IMAGE_THUMBNAIL", ErrorClass::Failed),
    ("FW_NODE_KV_DEL_STATE_BUS", ErrorClass::Failed),
    ("FW_NODE_KV_EXISTS_STATE_BUS", ErrorClass::Failed),
    ("FW_NODE_KV_EXPIRE_STATE_BUS", ErrorClass::Failed),
    ("FW_NODE_KV_GET_STATE_BUS", ErrorClass::Failed),
    ("FW_NODE_KV_INCR_STATE_BUS", ErrorClass::Failed),
    ("FW_NODE_KV_PUBLISH_STATE_BUS", ErrorClass::Failed),
    ("FW_NODE_KV_SET_STATE_BUS", ErrorClass::Failed),
    ("FW_NODE_MS_PUBLISH_FILTER", ErrorClass::Refused),
    ("FW_NODE_MS_PUBLISH_STYLE", ErrorClass::Refused),
    // `web.response.send`: what it answers is the author's; reading the file
    // and the template's compile and render can fail on their own.
    ("FW_NODE_WEB_RESPONSE_SEND_COMPILE", ErrorClass::Failed),
    ("FW_NODE_WEB_RESPONSE_SEND_CONFIG", ErrorClass::Refused),
    ("FW_NODE_WEB_RESPONSE_SEND_FILE", ErrorClass::Failed),
    ("FW_NODE_WEB_RESPONSE_SEND_HEADER", ErrorClass::Refused),
    ("FW_NODE_WEB_RESPONSE_SEND_REDIRECT", ErrorClass::Refused),
    ("FW_NODE_WEB_RESPONSE_SEND_RENDER", ErrorClass::Failed),
    // `web.site.generate`, both modes and the shared site machinery.
    ("FW_NODE_WEB_SITE_GENERATE_ASSET_DELETE", ErrorClass::Failed),
    ("FW_NODE_WEB_SITE_GENERATE_ASSET_MISSING", ErrorClass::Refused),
    ("FW_NODE_WEB_SITE_GENERATE_ASSET_PATH", ErrorClass::Refused),
    ("FW_NODE_WEB_SITE_GENERATE_ASSET_URL", ErrorClass::Refused),
    ("FW_NODE_WEB_SITE_GENERATE_ASSET_UTF8", ErrorClass::Failed),
    ("FW_NODE_WEB_SITE_GENERATE_COMPILE", ErrorClass::Failed),
    ("FW_NODE_WEB_SITE_GENERATE_CONFIG", ErrorClass::Refused),
    ("FW_NODE_WEB_SITE_GENERATE_CONFLICT", ErrorClass::Refused),
    ("FW_NODE_WEB_SITE_GENERATE_DEPLOY_BASE_PATH", ErrorClass::Refused),
    ("FW_NODE_WEB_SITE_GENERATE_EMPTY", ErrorClass::Refused),
    ("FW_NODE_WEB_SITE_GENERATE_MANIFEST_SERIALIZE", ErrorClass::Failed),
    ("FW_NODE_WEB_SITE_GENERATE_META_READ", ErrorClass::Failed),
    ("FW_NODE_WEB_SITE_GENERATE_MODE", ErrorClass::Refused),
    ("FW_NODE_WEB_SITE_GENERATE_OUTPUT_PATH", ErrorClass::Refused),
    ("FW_NODE_WEB_SITE_GENERATE_PAGE_INDEX", ErrorClass::Failed),
    ("FW_NODE_WEB_SITE_GENERATE_PATH", ErrorClass::Refused),
    ("FW_NODE_WEB_SITE_GENERATE_PROJECT_ASSET_MISSING", ErrorClass::Refused),
    ("FW_NODE_WEB_SITE_GENERATE_PROJECT_ASSET_READ", ErrorClass::Failed),
    ("FW_NODE_WEB_SITE_GENERATE_PROJECT_ASSETS", ErrorClass::Failed),
    ("FW_NODE_WEB_SITE_GENERATE_READ", ErrorClass::Failed),
    ("FW_NODE_WEB_SITE_GENERATE_READ_DIR", ErrorClass::Failed),
    ("FW_NODE_WEB_SITE_GENERATE_READ_PAGE", ErrorClass::Failed),
    ("FW_NODE_WEB_SITE_GENERATE_REL_DIR", ErrorClass::Refused),
    ("FW_NODE_WEB_SITE_GENERATE_REL_PATH", ErrorClass::Refused),
    ("FW_NODE_WEB_SITE_GENERATE_RENDER", ErrorClass::Failed),
    ("FW_NODE_WEB_SITE_GENERATE_ROOT", ErrorClass::Refused),
    ("FW_NODE_WEB_SITE_GENERATE_ROOT_MISSING", ErrorClass::Refused),
    ("FW_NODE_WEB_SITE_GENERATE_TEMPLATE_DIR", ErrorClass::Failed),
    ("FW_NODE_WEB_SITE_GENERATE_TEMPLATE_MISSING", ErrorClass::Refused),
    ("FW_NODE_WEB_SITE_GENERATE_TEMPLATE_PATH", ErrorClass::Refused),
    ("FW_NODE_WEB_SITE_GENERATE_TEMPLATE_READ", ErrorClass::Failed),
    ("FW_NODE_WEB_SITE_GENERATE_TEMPLATE_ROOT", ErrorClass::Failed),
    ("FW_NODE_WEB_SITE_GENERATE_TEMPLATE_WRITE", ErrorClass::Failed),
    ("FW_NODE_WEB_SITE_GENERATE_UNAVAILABLE", ErrorClass::Failed),
    ("FW_NODE_WEB_SITE_GENERATE_WRITE", ErrorClass::Failed),
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

/// The class a code was registered with, `None` for an unregistered code.
pub fn registered_class(code: &str) -> Option<ErrorClass> {
    ERROR_CLASS_REGISTRY.iter().find(|(c, _)| *c == code).map(|(_, class)| *class)
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
