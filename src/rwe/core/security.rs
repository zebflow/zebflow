use super::config::SecurityPolicy;
use super::error::EngineError;
use super::model::Diagnostic;

pub fn analyze(source: &str, policy: &SecurityPolicy) -> Result<Vec<Diagnostic>, EngineError> {
    let mut diagnostics = Vec::new();

    if !policy.allow_dynamic_import && has_identifier_call(source, "import") {
        return Err(EngineError::new(
            "RWE_SECURITY_DYNAMIC_IMPORT",
            "dynamic import() is blocked by security policy",
        ));
    }

    if !policy.allow_raw_html && source.contains("dangerouslySetInnerHTML") {
        return Err(EngineError::new(
            "RWE_SECURITY_RAW_HTML",
            "dangerouslySetInnerHTML is blocked by security policy",
        ));
    }

    for blocked in &policy.blocked_globals {
        if contains_blocked_global(source, blocked) {
            return Err(EngineError::new(
                "RWE_SECURITY_GLOBAL",
                format!("blocked global found in source: {blocked}"),
            ));
        }
    }

    check_fetch_domain_allowlist(source, policy)?;

    for (idx, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("<script") {
            diagnostics.push(Diagnostic {
                code: "RWE_INLINE_SCRIPT_WARN".to_string(),
                message: "inline <script> tag in TSX source".to_string(),
                line: Some((idx + 1) as u32),
            });
        }
    }

    Ok(diagnostics)
}

/// Check all `fetch("…")` / `fetch('…')` calls with static string literal URLs.
///
/// If `policy.network_allowlist` is empty, all `fetch()` calls are permitted.
/// If it is non-empty, every statically-detectable URL must have its hostname
/// listed in the allowlist (exact match or subdomain match).
///
/// Dynamic `fetch()` calls (e.g. `fetch(url)`) cannot be checked at compile
/// time and are not flagged — runtime enforcement must complement this.
fn check_fetch_domain_allowlist(
    source: &str,
    policy: &SecurityPolicy,
) -> Result<(), EngineError> {
    if policy.network_allowlist.is_empty() {
        return Ok(()); // empty = no restriction
    }

    let bytes = source.as_bytes();
    let pattern = b"fetch(";
    let mut i = 0;

    while i + pattern.len() <= bytes.len() {
        if &bytes[i..i + pattern.len()] == pattern {
            let mut j = i + pattern.len();
            // Skip whitespace between `fetch(` and the argument.
            while j < bytes.len() && (bytes[j] as char).is_ascii_whitespace() {
                j += 1;
            }
            // Only string literal arguments can be inspected statically.
            if j < bytes.len() && (bytes[j] == b'"' || bytes[j] == b'\'') {
                let quote = bytes[j];
                j += 1;
                let url_start = j;
                while j < bytes.len() && bytes[j] != quote && bytes[j] != b'\n' {
                    j += 1;
                }
                let url = std::str::from_utf8(&bytes[url_start..j]).unwrap_or("");
                if url.starts_with("http://") || url.starts_with("https://") {
                    let domain = extract_fetch_domain(url);
                    let allowed = policy.network_allowlist.iter().any(|a| {
                        domain == a.as_str()
                            || domain.ends_with(&format!(".{a}"))
                    });
                    if !allowed {
                        return Err(EngineError::new(
                            "RWE_SECURITY_FETCH",
                            format!(
                                "fetch() to '{domain}' is not in network_allowlist"
                            ),
                        ));
                    }
                }
            }
        }
        i += 1;
    }

    Ok(())
}

fn extract_fetch_domain(url: &str) -> &str {
    let without_scheme = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    let host = without_scheme.split('/').next().unwrap_or(without_scheme);
    host.split(':').next().unwrap_or(host) // strip port if present
}

fn contains_blocked_global(source: &str, blocked: &str) -> bool {
    match blocked {
        // Block `eval(...)` calls (not words like "re-evaluate").
        "eval" => has_identifier_call(source, "eval"),
        // Block `Function(...)` and `new Function(...)` (not text like "Functions").
        "Function" => has_identifier_call(source, "Function") || source.contains("new Function"),
        // Keep exact high-risk channel.
        "globalThis.Function" => source.contains("globalThis.Function"),
        // Fallback: conservative exact match, avoids broad substring checks.
        other => source.contains(other),
    }
}

fn has_identifier_call(source: &str, ident: &str) -> bool {
    let bytes = source.as_bytes();
    let ident_bytes = ident.as_bytes();
    if ident_bytes.is_empty() || bytes.len() < ident_bytes.len() {
        return false;
    }

    let mut i = 0usize;
    while i + ident_bytes.len() <= bytes.len() {
        if &bytes[i..i + ident_bytes.len()] == ident_bytes {
            let before_ok = i == 0 || !is_ident_char(bytes[i - 1] as char);
            if before_ok {
                let mut j = i + ident_bytes.len();
                let after_ident_ok = j >= bytes.len() || !is_ident_char(bytes[j] as char);
                if after_ident_ok {
                    while j < bytes.len() && (bytes[j] as char).is_ascii_whitespace() {
                        j += 1;
                    }
                    if j < bytes.len() && bytes[j] == b'(' {
                        return true;
                    }
                }
            }
        }
        i += 1;
    }
    false
}

fn is_ident_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'
}
