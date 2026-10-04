//! The response node and the two site generators.

use serde_json::{Map, Value, json};

use super::{Config, Rule};
use crate::platform::services::migration::RewriteContext;
use crate::platform::services::migration::expr::{is_object_literal, whole_expression};
use crate::platform::services::migration::graph::OldOutput;
use crate::platform::services::migration::node::NodeRewrite;

pub fn rule(kind: &str) -> Option<Rule> {
    Some(match kind {
        "n.web.response" => Rule::new("web.response.send", response_output, response),
        "n.web.static.generate" => Rule::new("web.site.generate", static_output, static_generate),
        "n.web.docs.generate" => Rule::new("web.site.generate", docs_output, docs_generate),
        _ => return None,
    })
}

fn response_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::replace_more(&[], "a node after n.web.response saw only the response envelope in 0.10")
}

/// The old cookie spec (`name=s,value={{ … }},max-age=86400,secure`) as
/// the `Set-Cookie` header 0.10 built from it, defaults written out.
pub fn cookie_header(spec: &str) -> Option<String> {
    let mut name = String::new();
    let mut value = String::new();
    let mut max_age = "900".to_string();
    let mut http_only = true;
    let mut secure = false;
    let mut same_site = "Lax".to_string();
    let mut path = "/".to_string();
    for part in split_outside_braces(spec) {
        let part = part.trim();
        if let Some(v) = part.strip_prefix("name=") {
            name = v.to_string();
        } else if let Some(v) = part.strip_prefix("value=") {
            value = v.to_string();
        } else if let Some(v) = part.strip_prefix("max-age=") {
            // 0.10 read an unparseable number as 900.
            max_age = if v.trim().parse::<i64>().is_ok() || v.contains("{{") { v.trim().to_string() } else { "900".to_string() };
        } else if let Some(v) = part.strip_prefix("same-site=") {
            same_site = v.to_string();
        } else if let Some(v) = part.strip_prefix("path=") {
            path = v.to_string();
        } else if part == "http-only" {
            http_only = true;
        } else if part == "no-http-only" {
            http_only = false;
        } else if part == "secure" {
            secure = true;
        }
    }
    if name.is_empty() {
        return None;
    }
    let mut parts = vec![format!("{name}={value}"), format!("Path={path}"), format!("Max-Age={max_age}"), format!("SameSite={same_site}")];
    if http_only {
        parts.push("HttpOnly".to_string());
    }
    if secure {
        parts.push("Secure".to_string());
    }
    Some(parts.join("; "))
}

/// `text` split on commas that are not inside `{{ }}`.
fn split_outside_braces(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut depth = 0usize;
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if text[i..].starts_with("{{") {
            depth += 1;
            current.push_str("{{");
            i += 2;
        } else if text[i..].starts_with("}}") && depth > 0 {
            depth -= 1;
            current.push_str("}}");
            i += 2;
        } else {
            let ch = text[i..].chars().next().unwrap_or(',');
            if ch == ',' && depth == 0 {
                out.push(std::mem::take(&mut current));
            } else {
                current.push(ch);
            }
            i += ch.len_utf8();
        }
    }
    out.push(current);
    out
}

fn add_header(headers: &mut Map<String, Value>, name: &str, value: String) {
    let existing = headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(k, _)| k.clone());
    match existing {
        None => {
            headers.insert(name.to_string(), Value::String(value));
        }
        Some(key) => {
            let current = headers.remove(&key).unwrap_or(Value::Null);
            let mut list = match current {
                Value::Array(items) => items,
                Value::Null => Vec::new(),
                other => vec![other],
            };
            // The cookie the spec built came first in 0.10.
            list.insert(0, Value::String(value));
            headers.insert(key, Value::Array(list));
        }
    }
}

fn response(n: &mut NodeRewrite<'_>) {
    n.kind("web.response.send");
    n.keep("template");
    n.keep("file");
    n.rename("folder", "root");
    let mut headers = match n.take("headers") {
        Some(Value::Object(map)) => map,
        Some(other) => {
            n.unresolved(format!("headers {other} is not a map"));
            Map::new()
        }
        None => Map::new(),
    };
    let status = n.take("status");
    let location = n.take("location");
    let message = n.take("message");
    let body = n.take("body");
    let file_mode = n.config.contains_key("file") || n.config.contains_key("root");
    let template = n.config.contains_key("template");

    if let Some(spec) = n.take_str("set_cookie") {
        match cookie_header(&spec) {
            Some(cookie) => {
                n.note(format!("set_cookie {spec} → header Set-Cookie {cookie}"));
                add_header(&mut headers, "Set-Cookie", cookie);
            }
            None => n.note(format!("set_cookie {spec} dropped (it named no cookie, so 0.10 sent none)")),
        }
    }
    if let Some(scripts) = n.take("load_scripts") {
        match super::words(&scripts) {
            Some(list) if !list.is_empty() => {
                n.note(format!("load_scripts {scripts} → script {list:?}"));
                n.set("scripts", json!(list));
            }
            _ => {}
        }
    }

    match (&location, file_mode) {
        (Some(location), false) => {
            let status = status.clone().unwrap_or(json!(302));
            match status.as_u64() {
                Some(code) if (300..400).contains(&code) => {
                    n.set("status", json!(code));
                    add_header(&mut headers, "Location", location.as_str().map(str::to_string).unwrap_or_else(|| location.to_string()));
                    n.note(format!("location → status {code} with a Location header"));
                }
                _ => n.unresolved(format!(
                    "location with status {status}: 0.10 redirected on any status, 0.11 sends a Location only with a 3xx"
                )),
            }
        }
        _ => {
            if let Some(status) = status.clone() {
                n.set("status", status);
            }
        }
    }

    if !file_mode && location.is_none() && !template {
        match (message, body) {
            (Some(message), _) => {
                // 0.10 answered a message as text; a string body is text in 0.11.
                n.set("body", message);
                n.note("message → body (text)");
            }
            (None, Some(body)) => answer_body(n, body, &mut headers),
            (None, None) => {
                // 0.10 answered the whole payload as JSON.
                if let Some(expr) = n.implicit_whole() {
                    n.note(format!("the whole payload answered → body {expr}"));
                    n.set("body", Value::String(expr));
                }
            }
        }
    } else if file_mode || template || location.is_some() {
        if let Some(message) = message {
            if !file_mode && !template {
                n.note(format!("message {message} dropped (a redirect answered no text)"));
            } else {
                n.note(format!("message {message} dropped (the file or template answered)"));
            }
        }
        if let Some(body) = body {
            n.note(format!("body {body} dropped (the file, template or redirect answered)"));
        }
    }
    if !headers.is_empty() {
        n.set("headers", Value::Object(headers));
    }
}

/// An explicit 0.10 body: any value was answered as JSON. A literal string
/// is written as the JSON text it was; an expression keeps its value, and a
/// string value is answered as text in 0.11.
fn answer_body(n: &mut NodeRewrite<'_>, body: Value, headers: &mut Map<String, Value>) {
    match &body {
        Value::String(text) => match whole_expression(text) {
            Some(expr) if is_object_literal(expr) => n.set("body", body),
            Some(_) => {
                n.set("body", body.clone());
                n.behaviour("a body that evaluates to a string is answered as text in 0.11 (0.10 answered it as a JSON string)");
            }
            None => {
                let has_type = headers.keys().any(|k| k.eq_ignore_ascii_case("content-type"));
                n.set("body", Value::String(serde_json::to_string(text).unwrap_or_default()));
                if !has_type {
                    headers.insert("Content-Type".to_string(), json!("application/json"));
                }
                n.note("a literal string body is written as the JSON text 0.10 answered");
            }
        },
        _ => n.set("body", body),
    }
}

// ── site generators ──────────────────────────────────────────────────────────

fn static_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::merge(&[
        ("generated", "!the generated answer is site in 0.11; read site.<field>"),
        ("generated.status", "site.status"),
        ("generated.path", "site.path"),
        ("generated.route", "site.route"),
        ("generated.store", "site.store"),
        ("generated.file", "site.file"),
        ("generated.template", "site.template"),
        ("generated.site_root", "site.folder"),
        ("generated.manifest_path", "site.manifest_path"),
        ("generated.asset_group", "site.asset_group"),
        ("generated.bytes", "site.bytes"),
        ("generated.deploy_base_url", "!0.11 takes the origin from how the folder is served"),
        ("generated.deploy_base_path", "!0.11 takes the origin from how the folder is served"),
    ])
}

fn deploy_flags(n: &mut NodeRewrite<'_>) {
    for key in ["deploy_base_url", "base_url", "deploy_base_path", "base_path", "origin"] {
        if let Some(value) = n.take(key) {
            n.unresolved(format!(
                "{key} {value}: 0.11 takes a site's address from how its folder is served (Studio → Files), not from the generator"
            ));
        }
    }
}

fn static_generate(n: &mut NodeRewrite<'_>) {
    n.kind("web.site.generate");
    for key in ["template", "path", "store", "route", "on_conflict"] {
        n.keep(key);
    }
    n.rename("site_root", "folder");
    deploy_flags(n);
}

fn docs_output(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::merge(&[
        ("docs_generated", "!the docs_generated answer is site in 0.11; read site.<field>"),
        ("docs_generated.status", "site.status"),
        ("docs_generated.site_title", "site.name"),
        ("docs_generated.template", "site.template"),
        ("docs_generated.docs_root", "site.from"),
        ("docs_generated.site_root", "site.folder"),
        ("docs_generated.manifest_path", "site.manifest_path"),
        ("docs_generated.asset_group", "site.asset_group"),
        ("docs_generated.page_count", "site.page_count"),
        ("docs_generated.generated_files", "site.generated_files"),
        ("docs_generated.skipped_files", "site.skipped_files"),
        ("docs_generated.sitemap_path", "site.sitemap_path"),
        ("docs_generated.search_index_path", "site.search_index_path"),
        ("docs_generated.urls", "site.routes"),
        ("docs_generated.deploy_base_url", "!0.11 takes the origin from how the folder is served"),
        ("docs_generated.deploy_base_path", "!0.11 takes the origin from how the folder is served"),
    ])
}

fn docs_generate(n: &mut NodeRewrite<'_>) {
    n.kind("web.site.generate");
    n.rename("docs_root", "from");
    n.rename("site_root", "folder");
    n.keep("store");
    n.rename("site_title", "name");
    let folder = n.take_str("template_folder").unwrap_or_default();
    let folder = folder.trim().trim_matches('/');
    let template = if folder.is_empty() { "docs.template.tsx".to_string() } else { format!("{folder}/docs.template.tsx") };
    n.note(format!("template_folder {folder} → template {template}"));
    n.set("template", Value::String(template));
    match n.take_str("meta_file").as_deref() {
        None | Some("_meta.yaml") => {}
        Some(other) => n.unresolved(format!("meta_file {other}: 0.11 reads _meta.yaml only")),
    }
    n.keep("on_conflict");
    deploy_flags(n);
}

#[cfg(test)]
mod tests {
    use super::cookie_header;

    #[test]
    fn a_cookie_spec_becomes_the_header_0_10_sent() {
        assert_eq!(
            cookie_header("name=session,value={{ input.token.access_token }}").as_deref(),
            Some("session={{ input.token.access_token }}; Path=/; Max-Age=900; SameSite=Lax; HttpOnly")
        );
        assert_eq!(
            cookie_header("name=s,value={{ f(a, b) }},max-age=86400,secure,same-site=Strict,no-http-only").as_deref(),
            Some("s={{ f(a, b) }}; Path=/; Max-Age=86400; SameSite=Strict; Secure")
        );
        assert_eq!(cookie_header("value=x"), None);
    }
}
