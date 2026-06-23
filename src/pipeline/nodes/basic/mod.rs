//! Built-in framework node registry and shared conventions.
//!
//! Read [`crate::pipeline::nodes`] (`src/pipeline/nodes/mod.rs`) before adding
//! or changing a node. That parent module documents the node authoring contract:
//! module docs, `definition()`, typed config, runtime handler, DSL flags, schemas,
//! and registration expectations.
//!
//! This module is the local catalog for framework-provided nodes. Add a new node
//! here only after its `definition()` is complete, then register it in
//! [`builtin_node_definitions`]. Keep cross-node payload conventions documented in
//! the smallest shared module that owns them:
//!
//! - [`file_ref`] owns FileRef IR for file-like bytes moving through pipelines.
//! - [`util`] owns metadata scope and dot-path resolution shared by node handlers.
//! - [`trigger`] owns ingress triggers and their root payload shape.
//!
//! ZebFS is the project storage backend; FileRef is the payload IR for passing
//! file-like bytes between nodes without embedding bytes in JSON. File-like
//! content should move as FileRef metadata, not inline base64, unless a node is
//! explicitly preserving a legacy shape. Multipart webhook files and
//! `http.request --response-type bytes` produce temporary FileRefs; FS nodes either
//! read those bytes (`fs.put`) or validate/promote them (`fs.save`). Durable
//! dataset nodes such as table, geo, and mapserver nodes operate on ZebFS paths,
//! but payload path keys should accept either a plain path string or a FileRef and
//! resolve it through [`file_ref`].

use crate::pipeline::NodeDefinition;

pub mod agent;
pub mod ai_tts;
pub mod auth_token_create;
pub mod browser_run;
pub mod crypto;
pub mod file_ref;
pub mod fs_compress;
pub mod fs_decompress;
pub mod fs_object;
pub mod fs_pdf_convert;
pub mod fs_save;
pub mod fs_thumbnail;
pub mod function_call;
pub mod geo_convert;
pub mod geo_inspect;
pub mod http_request;
pub mod kv_del;
pub mod kv_exists;
pub mod kv_expire;
pub mod kv_get;
pub mod kv_incr;
pub mod kv_publish;
pub mod kv_set;
pub mod logic;
pub mod mapserver_crud;
pub mod pg_query;
pub mod script;
pub mod sekejap_insert;
pub mod sekejap_query;
pub mod sqlite_mutate;
pub mod sqlite_query;
pub mod table_convert;
pub mod table_query;
pub mod trigger;
mod util;
pub mod web_docs_generate;
pub mod web_response;
pub mod web_static_generate;
pub mod web_static_site;
pub mod ws_client_send;
pub mod ws_emit;
pub mod ws_sync_state;
pub mod ws_trigger;

/// Returns built-in node definitions sorted by kind.
pub fn builtin_node_definitions() -> Vec<NodeDefinition> {
    let mut items = vec![
        agent::definition(),
        ai_tts::definition(),
        auth_token_create::definition(),
        browser_run::definition(),
        fs_compress::definition(),
        fs_decompress::definition(),
        fs_object::list_definition(),
        fs_object::head_definition(),
        fs_object::get_definition(),
        fs_object::put_definition(),
        fs_object::delete_definition(),
        fs_object::copy_definition(),
        fs_object::move_definition(),
        fs_object::mkdir_definition(),
        mapserver_crud::publish_definition(),
        mapserver_crud::unpublish_definition(),
        mapserver_crud::get_definition(),
        mapserver_crud::list_definition(),
        geo_convert::definition(),
        geo_inspect::definition(),
        fs_pdf_convert::definition(),
        fs_save::definition(),
        function_call::definition(),
        fs_thumbnail::definition(),
        kv_del::definition(),
        kv_exists::definition(),
        kv_expire::definition(),
        kv_get::definition(),
        kv_incr::definition(),
        kv_publish::definition(),
        kv_set::definition(),
        trigger::function::definition(),
        trigger::kv_subscribe::definition(),
        trigger::webhook::definition(),
        trigger::schedule::definition(),
        trigger::manual::definition(),
        trigger::mcp_trigger::definition(),
        ws_trigger::definition(),
        script::definition(),
        http_request::definition(),
        sekejap_insert::definition(),
        sekejap_query::definition(),
        sqlite_mutate::definition(),
        sqlite_query::definition(),
        table_convert::definition(),
        table_query::definition(),
        pg_query::definition(),
        web_response::definition(),
        web_docs_generate::definition(),
        web_static_generate::definition(),
        ws_client_send::definition(),
        ws_sync_state::definition(),
        ws_emit::definition(),
        trigger::ws_client::definition(),
        logic::if_::definition(),
        logic::match_::definition(),
        logic::collect::definition(),
        logic::foreach_::definition(),
        logic::reduce::definition(),
        logic::retry::definition(),
        crypto::definition(),
        trigger::weberror::definition(),
    ];
    // Inject engine-level common flags and fields into every node definition.
    let common_flags = crate::pipeline::model::engine_common_dsl_flags();
    let common_fields = crate::pipeline::model::engine_common_fields();
    for def in &mut items {
        for flag in &common_flags {
            if !def
                .dsl_flags
                .iter()
                .any(|f| f.config_key == flag.config_key)
            {
                def.dsl_flags.push(flag.clone());
            }
        }
        for field in &common_fields {
            if !def.fields.iter().any(|f| f.name == field.name) {
                def.fields.push(field.clone());
            }
        }
    }

    // Assign ui_category by prefix for builtins that don't declare one explicitly.
    for def in &mut items {
        if !def.ui_category.is_empty() {
            continue;
        }
        let (cat, label) = ui_category_for_kind(&def.kind);
        def.ui_category = cat.to_string();
        def.ui_category_label = label.to_string();
    }

    items.sort_by(|a, b| a.kind.cmp(&b.kind));
    items
}

/// Derives the UI category and subcategory label from a node kind string.
fn ui_category_for_kind(kind: &str) -> (&'static str, &'static str) {
    if kind.starts_with("n.trigger.") {
        return ("trigger", "");
    }
    if kind.starts_with("n.sekejap.") {
        return ("data.sekejap", "Sekejap");
    }
    if kind.starts_with("n.sqlite.") {
        return ("data.sqlite", "SQLite");
    }
    if kind.starts_with("n.pg.") {
        return ("data.postgres", "Postgres");
    }
    if kind.starts_with("n.kv.") {
        return ("data.kv", "KV Store");
    }
    if kind.starts_with("n.table.") {
        return ("data.table", "Table");
    }
    if kind.starts_with("n.geo.") {
        return ("data.geo", "Geo");
    }
    if kind.starts_with("n.ms.") {
        return ("data.mapserver", "MapServer");
    }
    if kind.starts_with("n.ai.") {
        return ("logic.ai", "AI");
    }
    if kind.starts_with("n.logic.") || kind.starts_with("n.function.") || kind == "n.script" {
        return ("logic", "");
    }
    if kind.starts_with("n.browser.") {
        return ("web.browser", "Browser");
    }
    if kind.starts_with("n.ws.") {
        return ("web.websocket", "WebSocket");
    }
    if kind.starts_with("n.http.") || kind.starts_with("n.web.") {
        return ("web", "");
    }
    if kind.starts_with("n.auth.") || kind == "n.crypto" {
        return ("security", "");
    }
    if kind.starts_with("n.fs.") {
        return ("files.fs", "File System");
    }
    if kind.starts_with("n.c.") {
        return ("composite", "");
    }
    if kind.starts_with("n.wasm.") {
        return ("wasm", "");
    }
    ("other", "")
}
