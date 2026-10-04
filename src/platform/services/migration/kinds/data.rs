//! Databases, tables, geodata and map layers.

use serde_json::{Map, Value, json};

use super::{Config, Rule, sql_keyword, words};
use crate::platform::services::migration::RewriteContext;
use crate::platform::services::migration::expr::{array_elements, is_member_path, whole_expression};
use crate::platform::services::migration::graph::OldOutput;
use crate::platform::services::migration::node::NodeRewrite;

pub fn rule(kind: &str) -> Option<Rule> {
    Some(match kind {
        "n.pg.query" => Rule::new("postgres.query.run", query_output, postgres),
        "n.sqlite.query" => Rule::new("sqlite.query.run", query_output, sqlite_query),
        "n.sqlite.mutate" => Rule::new("sqlite.query.run", mutate_output, sqlite_mutate),
        "n.sekejap.query" => Rule::new("sekejap.query.run", sekejap_output, sekejap_query),
        "n.sekejap.insert" => Rule::new("sekejap.record.create", insert_output, sekejap_insert),
        "n.table.convert" => Rule::new("table.data.convert", table_convert_output, table_convert),
        "n.table.query" => Rule::new("table.query.run", table_query_output, table_query),
        "n.geo.convert" => Rule::new("geo.dataset.convert", geo_convert_output, geo_convert),
        "n.geo.inspect" => Rule::new("geo.dataset.inspect", geo_inspect_output, geo_inspect),
        "n.ms.publish" => Rule::new("mapserver.layer.publish", ms_output, ms_publish),
        "n.ms.unpublish" => Rule::new("mapserver.layer.unpublish", ms_output, ms_name_only),
        "n.ms.get" => Rule::new("mapserver.layer.get", ms_output, ms_name_only),
        "n.ms.list" => Rule::new("mapserver.layer.list", ms_output, ms_name_only),
        _ => return None,
    })
}

// ── query nodes ──────────────────────────────────────────────────────────────

fn query_output(_: &Config, _: &RewriteContext) -> OldOutput {
    // v0.10.12 replaced the payload with `{ rows }` or `{ affected_rows }`.
    OldOutput::replace(&[("rows", "query.rows"), ("affected_rows", "query.rows_affected")], None)
}

fn mutate_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::replace(&[("ok", "=true"), ("affected_rows", "query.rows_affected")], None)
}

fn sekejap_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::replace(
        &[
            ("columns", "query.columns"),
            ("rows", "query.rows"),
            ("row_count", "query.row_count"),
            ("truncated", "query.truncated"),
            ("affected_rows", "query.rows_affected"),
            ("duration_ms", "!0.11 does not report how long a query took"),
        ],
        None,
    )
}

/// The old `params` (a list, one value, or one `{{ }}` holding either) as
/// the 0.11 `param` map `{ "1": … }`.
pub fn param_map(n: &mut NodeRewrite<'_>, key: &str) {
    let Some(value) = n.take(key) else { return };
    let mut map = Map::new();
    let mut push = |v: Value| {
        let position = (map.len() + 1).to_string();
        map.insert(position, v);
    };
    match &value {
        Value::Array(items) => items.iter().cloned().for_each(&mut push),
        Value::String(text) => match whole_expression(text) {
            Some(expr) => match array_elements(expr) {
                Some(elements) => elements.into_iter().for_each(|e| push(Value::String(format!("{{{{ {e} }}}}")))),
                None if is_member_path(expr) => push(value.clone()),
                None => {
                    n.unresolved(format!(
                        "`{key}` is `{text}`: 0.10 bound a list it evaluated to position by position and anything else as $1, and which this is cannot be told before it runs"
                    ));
                    return;
                }
            },
            None => push(value.clone()),
        },
        Value::Number(_) | Value::Bool(_) => push(value.clone()),
        _ => {
            n.unresolved(format!("`{key}` = {value} is not a list of bind values"));
            return;
        }
    }
    if !map.is_empty() {
        n.note(format!("{key} {value} → param {}", Value::Object(map.clone())));
        n.set("param", Value::Object(map));
    }
}

/// `--write` when the statement changes data: 0.10 ran anything, 0.11 runs
/// read-only unless told.
fn write_unless_select(n: &mut NodeRewrite<'_>, query: &str) {
    let keyword = if query.contains("{{") { String::new() } else { sql_keyword(query) };
    if keyword == "select" {
        return;
    }
    n.set("write", Value::Bool(true));
    if keyword.is_empty() {
        n.note("write = true (the statement is built at run time; 0.10 let it change data)");
    } else {
        n.note(format!("write = true (a {keyword} statement; 0.11 runs read-only without it)"));
    }
}

fn row_ceiling(n: &mut NodeRewrite<'_>) {
    if !n.config.contains_key("limit") {
        n.set_default("limit", json!(5000), "0.10 answered every row; 5000 is the 0.11 ceiling");
        n.behaviour("0.10 answered every row a query returned; 0.11 answers at most 5000 and says so in query.truncated");
    }
}

fn postgres(n: &mut NodeRewrite<'_>) {
    n.kind("postgres.query.run");
    n.keep("credential_id");
    let query = n.take_str("query").unwrap_or_default();
    write_unless_select(n, &query);
    n.set("query", Value::String(query));
    if n.has("params") {
        n.behaviour("numbers bind as numbers in 0.11 (0.10 bound them as text)");
    }
    param_map(n, "params");
    row_ceiling(n);
}

fn sqlite_sql(n: &mut NodeRewrite<'_>) -> String {
    match n.take_str("query") {
        Some(query) => {
            n.take("sql");
            query
        }
        None => {
            let query = n.take_str("sql").unwrap_or_default();
            if !query.is_empty() {
                n.note("sql → query");
            }
            query
        }
    }
}

fn sqlite_query(n: &mut NodeRewrite<'_>) {
    n.kind("sqlite.query.run");
    let query = sqlite_sql(n);
    write_unless_select(n, &query);
    n.set("query", Value::String(query));
    param_map(n, "params");
    row_ceiling(n);
}

fn sqlite_mutate(n: &mut NodeRewrite<'_>) {
    n.kind("sqlite.query.run");
    let query = sqlite_sql(n);
    n.set("query", Value::String(query));
    n.set("write", Value::Bool(true));
    n.note("write = true (n.sqlite.mutate is sqlite.query.run --write)");
    param_map(n, "params");
}

fn sekejap_query(n: &mut NodeRewrite<'_>) {
    n.kind("sekejap.query.run");
    let query = n.take_str("query").unwrap_or_default();
    param_map(n, "params");
    let read_only = n.take_bool("read_only").unwrap_or(false);
    if read_only {
        n.note("read_only → write omitted");
    } else {
        // 0.10 let a Sekejap query write unless --read-only.
        write_unless_select(n, &query);
    }
    n.set("query", Value::String(query));
    if let Some(limit) = n.take("limit") {
        match limit.as_u64().or_else(|| limit.as_str().and_then(|s| s.trim().parse().ok())) {
            Some(value) if value > 5000 => {
                n.set("limit", json!(5000));
                n.note(format!("limit {value} → 5000 (the 0.11 ceiling)"));
                n.behaviour(format!("0.10 answered up to {value} rows; 0.11 answers at most 5000 and says so in query.truncated"));
            }
            _ => n.set("limit", limit),
        }
    }
}

fn insert_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::replace(
        &[
            ("inserted_records", "record.created"),
            ("inserted_edges", "record.edges"),
            ("affected_rows", "!0.11 answers created and edges only"),
            ("optimized_fields", "!0.11 does not report optimised fields"),
            ("field_dimensions", "!0.11 does not report field dimensions"),
            ("duration_ms", "!0.11 does not report how long an insert took"),
        ],
        None,
    )
}

fn sekejap_insert(n: &mut NodeRewrite<'_>) {
    n.kind("sekejap.record.create");
    n.rename("target", "table");
    let records = n.take_str("records_path").or_else(|| n.take_str("records_key")).unwrap_or_else(|| "records".into());
    let edges = n.take_str("edges_path").or_else(|| n.take_str("edges_key")).unwrap_or_else(|| "edges".into());
    let segments = |p: &str| p.split('.').map(str::to_string).collect::<Vec<_>>();
    let records_path = segments(&records);
    let records_refs: Vec<&str> = records_path.iter().map(String::as_str).collect();
    if let Some(expr) = n.implicit(&records_refs) {
        n.note(format!("records read from input.{records} → record {expr}"));
        n.set("record", Value::String(expr));
    }
    let edges_path = segments(&edges);
    let edges_refs: Vec<&str> = edges_path.iter().map(String::as_str).collect();
    match n.locate(&edges_refs) {
        Ok(Some(expr)) => {
            let expr = format!("{{{{ {expr} }}}}");
            n.note(format!("edges read from input.{edges} → edge {expr}"));
            n.set("edge", Value::String(expr));
        }
        Ok(None) => {}
        Err(why) => n.unresolved(format!("the 0.10 node read `input.{edges}` without a flag: {why}")),
    }
    if let Some(key) = n.take_str("key_path").or_else(|| n.take_str("record_key")) {
        n.set("key", Value::String(key));
    }
    let records_max = n.take("max_records").and_then(|v| v.as_u64()).unwrap_or(1000);
    let edges_max = n.take("max_edges").and_then(|v| v.as_u64()).unwrap_or(1000);
    let total = records_max + edges_max;
    if total > 10_000 {
        n.unresolved(format!("max_records {records_max} + max_edges {edges_max} is above the 0.11 ceiling of 10000 items"));
    } else {
        n.set("max_items", json!(total));
        n.note(format!("max_records {records_max} + max_edges {edges_max} → max_items {total}"));
        n.behaviour("0.11 caps records and edges together, not each on its own");
    }
}

// ── tables ───────────────────────────────────────────────────────────────────

fn table_convert_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::merge(&[
        ("table", "!the table answer changed shape; read data.<field>"),
        ("table.rows", "data.row_count"),
        ("table.data", "data.rows"),
        ("table.columns", "data.columns"),
        ("table.file", "data"),
        ("table.to", "data.ref"),
        ("table.from_format", "data.parse"),
        ("table.to_format", "data.format"),
        ("table.from", "!0.11 does not echo the source"),
        ("table.preview", "!0.11 has no preview rows"),
        ("table.streamed", "!0.11 does not report streaming"),
        ("table.size", "data.size"),
    ])
}

fn table_convert(n: &mut NodeRewrite<'_>) {
    n.kind("table.data.convert");
    for key in ["from", "folder", "filename", "path", "store", "on_conflict", "limit"] {
        n.keep(key);
    }
    n.rename("from_format", "parse");
    n.rename("to_format", "format");
    if n.take_bool("to_json") == Some(true) {
        n.set("rows", Value::Bool(true));
        n.note("to_json → rows");
    }
    if n.take("preview_rows").is_some() {
        n.note("preview_rows dropped (0.11 has no preview rows)");
    }
}

fn table_query_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::merge(&[
        ("table", "!the table answer changed shape; read query.<field>"),
        ("table.rows", "query.row_count"),
        ("table.data", "query.rows"),
        ("table.columns", "query.columns"),
        ("table.file", "query"),
        ("table.to", "query.ref"),
        ("table.to_format", "query.format"),
        ("table.engine", "!0.11 has one engine and does not name it"),
        ("table.sources", "!0.11 does not echo the sources"),
        ("table.preview", "!0.11 has no preview rows"),
    ])
}

fn table_query(n: &mut NodeRewrite<'_>) {
    n.kind("table.query.run");
    match n.take_str("engine") {
        None => {}
        Some(engine) if engine == "geodatafusion" => n.note("engine geodatafusion dropped (the one engine)"),
        Some(engine) => n.unresolved(format!("engine `{engine}` does not exist in 0.11")),
    }
    n.rename("sources", "from");
    let query = sqlite_sql(n);
    n.set("query", Value::String(query));
    param_map(n, "params");
    n.rename("to_format", "format");
    let destination = ["folder", "filename", "path"].iter().any(|k| n.has(k));
    for key in ["folder", "filename", "path", "store", "on_conflict", "limit"] {
        n.keep(key);
    }
    if n.take_bool("to_json") == Some(true) {
        n.set("rows", Value::Bool(true));
        n.note("to_json → rows");
    }
    if n.take("preview_rows").is_some() {
        n.note("preview_rows dropped (0.11 has no preview rows)");
    }
    if !destination && !n.config.contains_key("limit") {
        n.set_default("limit", json!(5000), "0.10 answered up to 10000 rows inline; 5000 is the 0.11 ceiling");
        n.behaviour("0.10 answered up to 10000 rows inline; 0.11 answers at most 5000");
    }
}

// ── geodata ──────────────────────────────────────────────────────────────────

fn geo_convert_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::merge(&[
        ("converted", "!the converted answer changed shape; read dataset.<field>"),
        ("converted.features", "dataset.feature_count"),
        ("converted.file", "dataset"),
        ("converted.files", "dataset.files"),
        ("converted.store", "dataset.store"),
        ("converted.source", "!0.11 does not echo the source"),
        ("converted.elapsed_secs", "!0.11 does not report the time taken"),
        ("converted.skipped", "!0.11 has no skipped flag"),
    ])
}

fn geo_convert(n: &mut NodeRewrite<'_>) {
    n.kind("geo.dataset.convert");
    for key in ["from", "folder", "filename", "path", "store", "on_conflict", "layer", "hilbert", "batch_size"] {
        n.keep(key);
    }
    n.rename("to_crs", "crs");
}

fn geo_inspect_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::merge(&[
        ("inspect", "!the inspect answer changed shape; read dataset.<field>"),
        ("inspect.source", "dataset.source"),
        ("inspect.store", "dataset.store"),
        ("inspect.report", "!each layer's geometry, crs and fields changed shape"),
    ])
}

fn geo_inspect(n: &mut NodeRewrite<'_>) {
    n.kind("geo.dataset.inspect");
    n.keep("from");
    n.keep("store");
    if n.take("layer").is_some() {
        n.note("layer dropped (0.10 ignored it and reported every layer; 0.11 would filter)");
    }
}

// ── map layers ───────────────────────────────────────────────────────────────

fn ms_output(_: &Config, _: &RewriteContext) -> OldOutput {
    // v0.10.12 replaced the payload with `{ ms: … }`.
    OldOutput::replace(
        &[
            ("ms", "!the ms answer is layer in 0.11; read layer.<field>"),
            ("ms.operation", "!0.11 does not echo the operation"),
            ("ms.layer", "!the layer object changed field names"),
            ("ms.layer.layer_id", "layer.name"),
            ("ms.layer.path", "layer.route"),
            ("ms.layer.url", "!0.11 answers no url"),
            ("ms.layer.source_path", "layer.source"),
            ("ms.layer.source_kind", "layer.source_kind"),
            ("ms.layer.mode", "!0.11 has no mode"),
            ("ms.layer.min_zoom", "layer.min_zoom"),
            ("ms.layer.max_zoom", "layer.max_zoom"),
            ("ms.layer.bbox_required", "layer.bbox_required"),
            ("ms.layer.max_features", "layer.max_items"),
            ("ms.layer.allowed_properties", "!0.11 answers fields as a list"),
            ("ms.layer.feature_count", "layer.feature_count"),
            ("ms.layer.chunk_count", "layer.chunk_count"),
            ("ms.layer.style", "layer.style"),
            ("ms.layer.filter", "layer.filter"),
            ("ms.layer.function_slug", "layer.function"),
            ("ms.layer.cache_ttl_secs", "!0.11 answers ttl as a duration"),
            ("ms.layer_id", "layer.name"),
            ("ms.removed", "layer.removed"),
            ("ms.found", "layer.found"),
            ("ms.count", "layer.count"),
            ("ms.layers", "!each listed layer changed field names"),
            ("ms.optimization", "!0.11 does not report the optimisation"),
        ],
        None,
    )
}

fn ms_name_only(n: &mut NodeRewrite<'_>) {
    let kind = match n.old_kind.as_str() {
        "n.ms.unpublish" => "mapserver.layer.unpublish",
        "n.ms.get" => "mapserver.layer.get",
        _ => "mapserver.layer.list",
    };
    n.kind(kind);
    n.keep("name");
    n.keep("store");
}

fn ms_publish(n: &mut NodeRewrite<'_>) {
    n.kind("mapserver.layer.publish");
    n.keep("name");
    if !n.has("route") {
        n.rename("path", "route");
    } else {
        n.keep("route");
    }
    let function = n.has("function");
    match n.take("from").or_else(|| n.take("source_path")) {
        Some(from) => n.set("from", from),
        None if !function => {
            // v0.10.12 fell back to the payload's `source_path`.
            if let Some(expr) = n.implicit(&["source_path"]) {
                n.note(format!("source_path read from the payload → from {expr}"));
                n.set("from", Value::String(expr));
            }
        }
        None => {}
    }
    match n.take_str("source_kind").as_deref() {
        None | Some("") | Some("geojson_function") => {}
        Some("geojson_file") => n.set("parse", json!("geojson")),
        Some("geoparquet") => n.set("parse", json!("geoparquet")),
        Some("geojson_artifact") => {
            n.set("parse", json!("geojson"));
            n.set("build_artifact", Value::Bool(true));
            n.set("skip_optimize", Value::Bool(true));
            n.note("source_kind geojson_artifact → parse geojson, build_artifact, skip_optimize");
        }
        Some(other) => n.unresolved(format!("source_kind `{other}` has no 0.11 equivalent")),
    }
    // bbox: v0.10.12 had two switches; 79c7149 one.
    let optional = n.take_bool("bbox_optional").unwrap_or(false)
        || n.take_bool("no_bbox_required").unwrap_or(false)
        || n.take_bool("bbox_required") == Some(false);
    if optional {
        n.set("bbox_optional", Value::Bool(true));
    }
    n.rename("max_features", "max_items");
    if let Some(list) = n.take("allowed_properties") {
        match words(&list) {
            Some(fields) if !fields.is_empty() => {
                n.note(format!("allowed_properties {list} → field {fields:?}"));
                n.set("field", json!(fields));
            }
            _ => n.unresolved(format!("allowed_properties {list} is not a list of names")),
        }
    }
    for key in ["min_zoom", "max_zoom", "fill", "stroke", "stroke_width", "point_radius", "point_color", "filter", "function", "store"] {
        n.keep(key);
    }
    if !n.config.contains_key("build_artifact") {
        n.keep("build_artifact");
    } else {
        n.take("build_artifact");
    }
    n.rename("style_dsl", "style");
    if n.take("optimize").is_some() {
        n.note("optimize dropped (it did nothing in 0.10)");
    }
    if n.take_bool("no_optimize") == Some(true) {
        n.set("skip_optimize", Value::Bool(true));
        n.note("no_optimize → skip_optimize");
    }
    if n.take_bool("skip_optimize") == Some(true) {
        n.set("skip_optimize", Value::Bool(true));
    }
    n.duration("cache_ttl", "ttl", "s");
}
