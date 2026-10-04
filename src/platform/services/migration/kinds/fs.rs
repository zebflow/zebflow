//! Files: put, the object operations, archives, images, PDFs and barcodes.
//! Every 0.10 file node merged its answer; the source a node read without a
//! flag (`saved`, `files.file`, `svg`) becomes an explicit `--from`.

use serde_json::{Value, json};

use super::{Config, Rule, words};
use crate::platform::services::migration::RewriteContext;
use crate::platform::services::migration::graph::OldOutput;
use crate::platform::services::migration::node::NodeRewrite;

pub fn rule(kind: &str) -> Option<Rule> {
    Some(match kind {
        "n.fs.save" => Rule::new("fs.file.put", save_output, save),
        "n.fs.put" => Rule::new("fs.file.put", object_output, put),
        "n.fs.list" => Rule::new("fs.folder.list", list_output, list),
        "n.fs.head" => Rule::new("fs.file.head", object_output, head),
        "n.fs.get" => Rule::new("fs.file.get", object_output, get),
        "n.fs.delete" => Rule::new("fs.file.delete", delete_output, delete),
        "n.fs.copy" => Rule::new("fs.file.copy", copy_output, copy_move),
        "n.fs.move" => Rule::new("fs.file.move", copy_output, copy_move),
        "n.fs.mkdir" => Rule::new("fs.folder.create", mkdir_output, mkdir),
        "n.fs.compress" => Rule::new("fs.archive.create", compress_output, compress),
        "n.fs.decompress" => Rule::new("fs.archive.extract", decompress_output, decompress),
        "n.fs.image.thumbnail" => Rule::new("fs.image.thumbnail", thumbnail_output, thumbnail),
        "n.fs.image.chromakey" => Rule::new("fs.image.chromakey", chromakey_output, chromakey),
        "n.fs.svg.convert" => Rule::new("fs.image.render", svg_output, svg),
        "n.fs.pdf.convert" => Rule::new("fs.pdf.convert", pdf_output, pdf),
        "n.fs.barcode.qr" => Rule::new("fs.barcode.render", qr_output, barcode),
        "n.fs.barcode.code128" => Rule::new("fs.barcode.render", code128_output, barcode),
        _ => return None,
    })
}

/// Kinds whose answer at a key is a FileRef in 0.10, so a value read there
/// is a stored file (`--from`), never text.
fn produces_file(kind: &str) -> bool {
    kind.starts_with("n.fs.") || kind == "n.trigger.webhook" || kind.starts_with("n.input.")
}

fn keep_all(n: &mut NodeRewrite<'_>, keys: &[&str]) {
    for key in keys {
        n.keep(key);
    }
}

const DESTINATION: &[&str] = &["folder", "filename", "path", "store", "on_conflict"];

/// The source an old node read from the payload at `default` (or at the
/// path its `source_key` named), as `--from`.
fn source_from(n: &mut NodeRewrite<'_>, default: &str) {
    let key = n.take_str("source_key").unwrap_or_else(|| default.to_string());
    let path: Vec<String> = key.split('.').map(str::to_string).collect();
    let refs: Vec<&str> = path.iter().map(String::as_str).collect();
    if let Some(expr) = n.implicit(&refs) {
        n.note(format!("the source read from input.{key} → from {expr}"));
        n.set("from", Value::String(expr));
    }
}

// ── put ──────────────────────────────────────────────────────────────────────

fn save_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::merge(&[("saved", "file")])
}

fn save(n: &mut NodeRewrite<'_>) {
    n.kind("fs.file.put");
    if n.has("source_key") {
        // 79c7149 spelling: a dot path into the payload.
        source_from(n, "files.file");
    } else {
        let field = n.take_str("field").unwrap_or_else(|| "file".to_string());
        // 0.10 looked in files.<field>, then <field>, then response.body.
        let candidates: [Vec<&str>; 3] = [vec!["files", field.as_str()], vec![field.as_str()], vec!["response", "body"]];
        let mut found = None;
        for candidate in &candidates {
            match n.locate(candidate) {
                Ok(Some(expr)) => {
                    found = Some((candidate.join("."), expr));
                    break;
                }
                Ok(None) => continue,
                Err(why) => {
                    n.unresolved(format!("the upload n.fs.save read (input.{}) cannot be traced: {why}", candidate.join(".")));
                    return;
                }
            }
        }
        match found {
            Some((path, expr)) => {
                let expr = format!("{{{{ {expr} }}}}");
                n.note(format!("the upload read from input.{path} → from {expr}"));
                n.set("from", Value::String(expr));
            }
            None => n.unresolved(format!(
                "n.fs.save found its file by looking through the payload (files.{field}, {field}, response.body, any one FileRef) and none of those is answered upstream"
            )),
        }
    }
    path_or_folder(n);
    keep_all(n, &["folder", "filename", "store", "on_conflict"]);
    if let Some(kinds) = n.take("allowed_kinds") {
        match words(&kinds) {
            Some(list) => {
                let accept: Vec<String> = list.into_iter().map(|w| if w == "images" { "image".to_string() } else { w }).collect();
                n.note(format!("allowed_kinds {kinds} → accept {accept:?}"));
                n.set("accept", json!(accept));
            }
            None => n.unresolved(format!("allowed_kinds {kinds} is not a list of kinds")),
        }
    }
    match n.take("max_size_mb") {
        Some(value) => match value.as_f64().or_else(|| value.as_str().and_then(|s| s.trim().parse().ok())) {
            Some(mb) => {
                // 0.10 multiplied by 1024 × 1024: the unit was MiB.
                let text = format!("{}MiB", trim(mb));
                n.note(format!("max_size_mb {value} → max_size {text}"));
                n.set("max_size", Value::String(text));
            }
            None => n.unresolved(format!("max_size_mb {value} is not a number")),
        },
        None => n.set_default("max_size", json!("10MiB"), "the 0.10 default was 10 × 1024 × 1024 bytes"),
    }
}

fn trim(number: f64) -> String {
    if number.fract() == 0.0 { format!("{}", number as i64) } else { format!("{number}") }
}

/// A 0.10 `--path` ending in `/` named a folder.
fn path_or_folder(n: &mut NodeRewrite<'_>) {
    if let Some(path) = n.take_str("path") {
        if let Some(folder) = path.strip_suffix('/') {
            n.note(format!("path {path} → folder {folder} (a trailing / named a folder in 0.10)"));
            n.set("folder", Value::String(folder.to_string()));
        } else {
            n.set("path", Value::String(path));
        }
    }
}

fn object_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::merge(&[
        ("fs", "!the fs answer is file in 0.11; read file.<field>"),
        ("fs.object", "file"),
        ("fs.path", "file.path"),
        ("fs.operation", "!0.11 does not echo the operation"),
        ("fs.skipped", "!0.11 has no skipped flag"),
    ])
}

fn put(n: &mut NodeRewrite<'_>) {
    n.kind("fs.file.put");
    let key = n.take_str("from_key").or_else(|| n.take_str("source_key"));
    if let Some(key) = key {
        let path: Vec<String> = key.split('.').map(str::to_string).collect();
        let resolution = n.graph.resolve_input(n.index, &path);
        match resolution {
            Ok(origin) if produces_file(&n.graph.nodes[origin.producer].kind) => {
                if let Some(expr) = n.implicit(&path.iter().map(String::as_str).collect::<Vec<_>>()) {
                    n.note(format!("source_key {key} → from {expr}"));
                    n.set("from", Value::String(expr));
                }
            }
            Ok(origin) => n.unresolved(format!(
                "n.fs.put wrote whatever was at input.{key} (text as text, JSON as JSON, a FileRef as the file), and what node `{}` answers there cannot be told before it runs",
                n.graph.nodes[origin.producer].id
            )),
            Err(failure) => n.unresolved(format!("n.fs.put read input.{key}: {}", failure.why)),
        }
    }
    n.keep("text");
    if let Some(b64) = n.take("base64") {
        n.set("text", b64);
        n.set("encoding", json!("base64"));
        n.note("base64 → text with encoding base64");
    }
    keep_all(n, &["filename", "path", "store", "on_conflict", "folder"]);
    n.set_default("folder", json!("files"), "n.fs.put wrote under files/ by default; fs.file.put under uploads/");
}

fn list_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::merge(&[
        ("fs", "!the fs answer is folder in 0.11; read folder.<field>"),
        ("fs.entries", "folder.items"),
        ("fs.count", "folder.count"),
        ("fs.path", "folder.path"),
        ("fs.operation", "!0.11 does not echo the operation"),
    ])
}

fn list(n: &mut NodeRewrite<'_>) {
    n.kind("fs.folder.list");
    let path = n.take("path").or_else(|| n.take("prefix")).unwrap_or_else(|| json!("/"));
    n.set("from", path);
    n.keep("store");
}

fn head(n: &mut NodeRewrite<'_>) {
    n.kind("fs.file.head");
    n.rename("path", "from");
    n.keep("store");
}

fn get(n: &mut NodeRewrite<'_>) {
    n.kind("fs.file.get");
    n.rename("path", "from");
    n.keep("store");
    match n.take_str("encoding") {
        None => {}
        Some(e) if e.eq_ignore_ascii_case("base64") => n.set("encoding", json!("base64")),
        Some(e) if e.eq_ignore_ascii_case("text") => n.set("encoding", json!("text")),
        Some(e) if e.contains("{{") => n.unresolved(format!("encoding `{e}` is chosen at run time; 0.11 takes text or base64 only")),
        Some(e) => n.note(format!("encoding {e} dropped (0.10 read anything but base64 as text)")),
    }
}

fn delete_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::merge(&[
        ("fs", "!the fs answer is file in 0.11; read file.<field>"),
        ("fs.path", "file.ref"),
        ("fs.deleted", "file.deleted"),
        ("fs.operation", "!0.11 does not echo the operation"),
    ])
}

fn delete(n: &mut NodeRewrite<'_>) {
    n.kind("fs.file.delete");
    n.rename("path", "from");
    n.keep("store");
    n.set("recursive", Value::Bool(true));
    n.note("recursive = true (0.10 deleted a folder and everything in it)");
}

fn copy_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::merge(&[
        ("fs", "!the fs answer is file in 0.11; read file.<field>"),
        ("fs.object", "file"),
        ("fs.path", "file.ref"),
        ("fs.source_path", "!0.11 does not echo the source"),
        ("fs.skipped", "!0.11 has no skipped flag"),
        ("fs.operation", "!0.11 does not echo the operation"),
    ])
}

fn copy_move(n: &mut NodeRewrite<'_>) {
    let kind = if n.old_kind == "n.fs.move" { "fs.file.move" } else { "fs.file.copy" };
    n.kind(kind);
    keep_all(n, &["from", "folder", "filename", "path", "store", "on_conflict"]);
}

fn mkdir_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::merge(&[
        ("fs", "!the fs answer is folder in 0.11; read folder.<field>"),
        ("fs.path", "folder.path"),
        ("fs.object", "!0.11 answers path and created"),
        ("fs.operation", "!0.11 does not echo the operation"),
    ])
}

fn mkdir(n: &mut NodeRewrite<'_>) {
    n.kind("fs.folder.create");
    n.rename("path", "folder");
    n.keep("store");
}

// ── archives ─────────────────────────────────────────────────────────────────

fn compress_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::merge(&[("compressed", "archive")])
}

fn compress(n: &mut NodeRewrite<'_>) {
    n.kind("fs.archive.create");
    let first = n.take_str("source_key").unwrap_or_else(|| "saved".to_string());
    let extra = n.take("extra_source_keys").and_then(|v| words(&v)).unwrap_or_default();
    let mut sources = Vec::new();
    for key in std::iter::once(first).chain(extra) {
        let path: Vec<String> = key.split('.').map(str::to_string).collect();
        match n.implicit(&path.iter().map(String::as_str).collect::<Vec<_>>()) {
            Some(expr) => sources.push(Value::String(expr)),
            None => return,
        }
    }
    n.note(format!("the sources read from the payload → from {}", Value::Array(sources.clone())));
    n.set("from", if sources.len() == 1 { sources.remove(0) } else { Value::Array(sources) });
    keep_all(n, &["folder", "filename", "path", "store", "on_conflict", "format"]);
}

fn decompress_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::merge(&[
        ("decompressed", "!the decompressed answer is archive in 0.11; read archive.<field>"),
        ("decompressed.files", "archive.items"),
        ("decompressed.folder", "archive.folder"),
        ("decompressed.source", "!0.11 does not echo the source"),
        ("decompressed.store", "!0.11 does not echo the store"),
        ("decompressed.format", "!0.11 does not echo the format"),
    ])
}

fn decompress(n: &mut NodeRewrite<'_>) {
    n.kind("fs.archive.extract");
    source_from(n, "saved");
    keep_all(n, &["folder", "format", "delete_source", "store", "on_conflict"]);
    if n.config.get("on_conflict").and_then(Value::as_str) == Some("overwrite") {
        n.behaviour("--on-conflict overwrite empties the destination folder first in 0.11; 0.10 left files the archive did not hold");
    }
}

// ── images ───────────────────────────────────────────────────────────────────

fn thumbnail_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::merge(&[("thumbnail", "image")])
}

/// A closed word 0.10 did not check: an unknown one behaved as the default.
fn closed(n: &mut NodeRewrite<'_>, key: &str, words: &[&str], default: &str) {
    let Some(value) = n.take_str(key) else { return };
    let word = value.trim().to_ascii_lowercase();
    if words.contains(&word.as_str()) {
        n.set(key, Value::String(word));
    } else if value.contains("{{") {
        n.unresolved(format!("{key} `{value}` is chosen at run time; 0.11 takes {} only", words.join("|")));
    } else {
        n.note(format!("{key} {value} → {default} (0.10 treated an unknown word as {default})"));
        n.set(key, Value::String(default.to_string()));
    }
}

fn thumbnail(n: &mut NodeRewrite<'_>) {
    n.kind("fs.image.thumbnail");
    source_from(n, "saved");
    keep_all(n, &["width", "height", "folder", "delete_source"]);
    keep_all(n, DESTINATION);
    closed(n, "fit", &["cover", "contain", "fill"], "cover");
    closed(n, "format", &["jpg", "png", "webp"], "jpg");
    if let Some(quality) = n.take("quality") {
        match quality.as_f64() {
            Some(q) if !(1.0..=100.0).contains(&q) => {
                let clamped = q.clamp(1.0, 100.0) as i64;
                n.note(format!("quality {q} → {clamped} (0.10 clamped it)"));
                n.set("quality", json!(clamped));
            }
            _ => n.set("quality", quality),
        }
    }
}

fn chromakey_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::merge(&[("image", "image")])
}

fn chromakey(n: &mut NodeRewrite<'_>) {
    n.kind("fs.image.chromakey");
    source_from(n, "saved");
    keep_all(n, &["color", "tolerance", "soften", "format", "delete_source"]);
    keep_all(n, DESTINATION);
}

fn svg_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::merge(&[("image", "image"), ("layout", "image.layout")])
}

fn svg(n: &mut NodeRewrite<'_>) {
    n.kind("fs.image.render");
    let key = n.take_str("source_key").unwrap_or_else(|| "svg".to_string());
    let path: Vec<String> = key.split('.').map(str::to_string).collect();
    match n.graph.resolve_input(n.index, &path) {
        Ok(origin) if produces_file(&n.graph.nodes[origin.producer].kind) => {
            if let Some(expr) = n.implicit(&path.iter().map(String::as_str).collect::<Vec<_>>()) {
                n.note(format!("the stored SVG at input.{key} → from {expr}"));
                n.set("from", Value::String(expr));
            }
        }
        Ok(origin) => n.unresolved(format!(
            "n.fs.svg.convert decided per request whether input.{key} was SVG markup or a stored file; node `{}` answers it, and which one it is cannot be told before it runs (0.11 takes --text or --from)",
            n.graph.nodes[origin.producer].id
        )),
        Err(failure) => n.unresolved(format!("n.fs.svg.convert read input.{key}: {}", failure.why)),
    }
    keep_all(n, &["width", "height", "fit", "format", "quality", "delete_source"]);
    keep_all(n, DESTINATION);
}

// ── pdf ──────────────────────────────────────────────────────────────────────

fn pdf_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::merge(&[
        ("pdf_convert", "!the pdf_convert answer is pdf in 0.11; read pdf.<field>"),
        ("pdf_convert.files", "pdf.items"),
        ("pdf_convert.folder", "pdf.folder"),
        ("pdf_convert.manifest_path", "pdf.manifest_path"),
        ("pdf_convert.page_count", "pdf.page_count"),
        ("pdf_convert.pages", "pdf.pages"),
        ("pdf_convert.source", "!0.11 does not echo the source"),
        ("pdf_convert.store", "!0.11 does not echo the store"),
        ("pdf_convert.options", "!0.11 does not echo the options"),
        ("pdf_convert.skipped", "!0.11 has no skipped flag"),
    ])
}

fn pdf(n: &mut NodeRewrite<'_>) {
    n.kind("fs.pdf.convert");
    source_from(n, "saved");
    keep_all(n, &["folder", "store", "on_conflict"]);
    let flags = [("emit_fulltext", "text"), ("emit_page_images", "image"), ("emit_page_raster", "raster")];
    let mut include = Vec::new();
    for (key, word) in flags {
        if n.take_bool(key).unwrap_or(true) {
            include.push(word);
        }
    }
    match include.len() {
        3 => {}
        0 => n.unresolved("every emit_* switch is off; 0.11 always converts something (an empty --include means all)"),
        _ => {
            n.note(format!("emit_* → include {include:?}"));
            n.set("include", json!(include));
        }
    }
    if let Some(dpi) = n.take("dpi") {
        match dpi.as_f64() {
            Some(d) if !(36.0..=600.0).contains(&d) => n.unresolved(format!("dpi {d} is outside the 0.11 range 36–600")),
            _ => n.set("dpi", dpi),
        }
    }
}

// ── barcodes ─────────────────────────────────────────────────────────────────

fn qr_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::merge(&[
        ("qr", "barcode"),
        ("qr.version", "!0.11 does not report the QR version"),
        ("qr.ecc", "!0.11 does not report the error correction"),
        ("qr.modules", "!0.11 does not report the module count"),
        ("qr.svg", "!0.11 does not inline the SVG markup; read the stored file"),
    ])
}

fn code128_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::merge(&[
        ("barcode", "barcode"),
        ("barcode.svg", "!0.11 does not inline the SVG markup; read the stored file"),
    ])
}

fn barcode(n: &mut NodeRewrite<'_>) {
    n.kind("fs.barcode.render");
    if n.old_kind == "n.fs.barcode.code128" {
        n.set("symbology", json!("code128"));
        n.note("symbology = code128");
        n.keep("height");
    } else {
        n.keep("ecc");
    }
    n.rename("size", "width");
    keep_all(n, &["text", "format", "margin", "color", "background"]);
    keep_all(n, DESTINATION);
}
