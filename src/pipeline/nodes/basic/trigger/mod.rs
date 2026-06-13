//! Built-in trigger nodes.
//!
//! Triggers are entry nodes: they have no input pins and they create the first
//! payload for a pipeline run. For the general node authoring contract, read
//! `src/pipeline/nodes/mod.rs`; for registration, read
//! `src/pipeline/nodes/basic/mod.rs`.
//!
//! Ingress payloads must be stable enough for downstream nodes and frontend
//! generated clients to depend on. `trigger.webhook` uses this shape:
//!
//! - `input.body`: user-submitted JSON, form fields, multipart text fields, or
//!   `null` for empty requests.
//! - `input.query`, `input.params`, `input.path`, `input.method`: request context.
//! - `input.files.<field>`: uploaded files as FileRef metadata. Repeated file
//!   field names, `field[]`, and `field[0]` are represented as arrays under
//!   `input.files.<field>`.
//!
//! Keep trigger-specific details in each trigger module, and keep shared file byte
//! rules in `src/pipeline/nodes/basic/file_ref.rs`.

pub mod function;
pub mod kv_subscribe;
pub mod manual;
pub mod mcp_trigger;
pub mod schedule;
pub mod weberror;
pub mod webhook;
pub mod ws_client;
