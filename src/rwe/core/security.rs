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
