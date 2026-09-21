//! Built-in framework node registry and shared conventions.
//!
//! Read [`crate::pipeline::nodes`] (`src/pipeline/nodes/mod.rs`) before adding
//! or changing a node. That parent module documents the node authoring contract:
//! module docs, `definition()`, typed config, runtime handler, DSL flags, schemas,
//! and registration expectations.
//!
//! This module is the local catalog for framework-provided nodes. A node's file
//! is `basic/<family>/<node>.rs`, mirroring its kind; add a new node to its
//! family's `definitions()` only after its `definition()` is complete — that
//! list is what [`builtin_node_definitions`] reads. Keep cross-node payload
//! conventions documented in the smallest shared module that owns them:
//!
//! - [`crate::pipeline::nodes::shared::file_ref`] owns FileRef IR for file-like
//!   bytes moving through pipelines.
//! - [`crate::pipeline::nodes::shared::util`] owns metadata scope and dot-path
//!   resolution shared by node handlers.
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
//! resolve it through [`crate::pipeline::nodes::shared::file_ref`].

use crate::pipeline::NodeDefinition;

// One folder per DSL family, one file per node, the path mirroring the kind:
// `n.fs.save` is `fs/save.rs`, `n.kv.get` is `kv/get.rs`. A family with one
// node and no submodules keeps that node in its `mod.rs` (`concept`, `crypto`,
// `script`). Every family exposes `definitions()`; nothing else is registered
// here. A test in `crate::pipeline::nodes` refuses a `.rs` file beside this
// one and a folder that is not a family of the catalogue.
pub mod ai;
pub mod auth;
pub mod browser;
pub mod concept;
pub mod crypto;
pub mod fs;
pub mod function;
pub mod geo;
pub mod http;
pub mod input;
pub mod kv;
pub mod logic;
pub mod mail;
pub mod ms;
pub mod pg;
pub mod script;
pub mod sekejap;
pub mod sqlite;
pub mod table;
pub mod trigger;
pub mod web;
pub mod ws;

/// The family lists, in the order the node index shows them.
fn family_definitions() -> Vec<NodeDefinition> {
    let mut items = Vec::new();
    items.extend(ai::definitions());
    items.extend(auth::definitions());
    items.extend(browser::definitions());
    items.extend(concept::definitions());
    items.extend(crypto::definitions());
    items.extend(fs::definitions());
    items.extend(function::definitions());
    items.extend(geo::definitions());
    items.extend(http::definitions());
    items.extend(input::definitions());
    items.extend(kv::definitions());
    items.extend(logic::definitions());
    items.extend(mail::definitions());
    items.extend(ms::definitions());
    items.extend(pg::definitions());
    items.extend(script::definitions());
    items.extend(sekejap::definitions());
    items.extend(sqlite::definitions());
    items.extend(table::definitions());
    items.extend(trigger::definitions());
    items.extend(web::definitions());
    items.extend(ws::definitions());
    items
}

/// Returns built-in node definitions sorted by kind.
pub fn builtin_node_definitions() -> Vec<NodeDefinition> {
    let mut items = family_definitions();
    // Inject engine-level common flags and fields into every node definition.
    let common_flags = crate::pipeline::model::engine_common_dsl_flags();
    let common_fields = crate::pipeline::model::engine_common_fields();
    for def in &mut items {
        for flag in &common_flags {
            if !def
                .dsl_flags
                .iter()
                .any(|f| f.config_key == flag.config_key || f.flag == flag.flag)
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
    // The trigger's declaration, next to the trigger: its own family, not
    // "other" and not a data node.
    if kind.starts_with(input::KIND_PREFIX) {
        return ("input", "");
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
    // `n.concept` is a step described but not built: a logic placeholder, not
    // an "other" of its own.
    if kind.starts_with("n.logic.") || kind.starts_with("n.function.") || kind == "n.script" || kind == "n.concept" {
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
    if kind.starts_with("n.mail.") {
        return ("communication.mail", "Mail");
    }
    if kind.starts_with("n.auth.") || kind == "n.crypto" {
        return ("security", "");
    }
    if kind.starts_with("n.fs.") {
        return ("files.fs", "File System");
    }
    if kind.starts_with(crate::contracts::kinds::INSTALLED_NODE_KIND_PREFIX) {
        return ("installed", "Installed");
    }
    ("other", "")
}
