//! Lightweight source-level policy checks and loop instrumentation.

use super::config::DenoSandboxConfig;

/// Performs coarse static checks before script compilation.
///
/// This is a lightweight guard and intentionally conservative. For strict
/// production hardening, AST-based policy transforms are recommended.
pub(crate) fn forbid_patterns(source: &str, cfg: &DenoSandboxConfig) -> Result<(), String> {
    let low = source.to_ascii_lowercase();

    if !cfg.danger_zone.allow_dynamic_code {
        for b in ["eval(", "new function("] {
            if low.contains(b) {
                return Err(format!(
                    "DenoSandboxError: dynamic code is disabled (matched '{b}')"
                ));
            }
        }
    }

    if !cfg.danger_zone.allow_import && (low.contains("import(") || low.contains("import ")) {
        return Err("DenoSandboxError: import is disabled".into());
    }

    if !cfg.danger_zone.allow_timers {
        for b in ["settimeout(", "setinterval(", "queuemicrotask("] {
            if low.contains(b) {
                return Err(format!(
                    "DenoSandboxError: timer api disabled (matched '{b}')"
                ));
            }
        }
    }

    Ok(())
}

/// Injects operation budget ticks into `while`, `for`, and `do` loops.
pub(crate) fn inject_loop_guards(source: &str) -> String {
    let mut out = String::with_capacity(source.len() + source.len() / 8);
    let chars: Vec<char> = source.chars().collect();
    let mut i = 0;
    let mut state = ScanState::Code;

    while i < chars.len() {
        let c = chars[i];
        match state {
            ScanState::Code => {
                if c == '\'' {
                    state = ScanState::SingleQuote;
                    out.push(c);
                    i += 1;
                    continue;
                }
                if c == '"' {
                    state = ScanState::DoubleQuote;
                    out.push(c);
                    i += 1;
                    continue;
                }
                if c == '`' {
                    state = ScanState::Template;
                    out.push(c);
                    i += 1;
                    continue;
                }
                if c == '/' && i + 1 < chars.len() && chars[i + 1] == '/' {
                    state = ScanState::LineComment;
                    out.push(c);
                    out.push(chars[i + 1]);
                    i += 2;
                    continue;
                }
                if c == '/' && i + 1 < chars.len() && chars[i + 1] == '*' {
                    state = ScanState::BlockComment;
                    out.push(c);
                    out.push(chars[i + 1]);
                    i += 2;
                    continue;
                }

                if starts_with_keyword(&chars, i, "while") {
                    let (next_i, fragment) = rewrite_while(&chars, i);
                    out.push_str(&fragment);
                    i = next_i;
                    continue;
                }
                if starts_with_keyword(&chars, i, "for") {
                    let (next_i, fragment) = rewrite_for(&chars, i);
                    out.push_str(&fragment);
                    i = next_i;
                    continue;
                }
                if starts_with_keyword(&chars, i, "do") {
                    let (next_i, fragment) = rewrite_do(&chars, i);
                    out.push_str(&fragment);
                    i = next_i;
                    continue;
                }

                out.push(c);
                i += 1;
            }
            ScanState::SingleQuote => {
                out.push(c);
                if c == '\\' && i + 1 < chars.len() {
                    out.push(chars[i + 1]);
                    i += 2;
                    continue;
                }
                if c == '\'' {
                    state = ScanState::Code;
                }
                i += 1;
            }
            ScanState::DoubleQuote => {
                out.push(c);
                if c == '\\' && i + 1 < chars.len() {
                    out.push(chars[i + 1]);
                    i += 2;
                    continue;
                }
                if c == '"' {
                    state = ScanState::Code;
                }
                i += 1;
            }
            ScanState::Template => {
                out.push(c);
                if c == '\\' && i + 1 < chars.len() {
                    out.push(chars[i + 1]);
                    i += 2;
                    continue;
                }
                if c == '`' {
                    state = ScanState::Code;
                }
                i += 1;
            }
            ScanState::LineComment => {
                out.push(c);
                i += 1;
                if c == '\n' {
                    state = ScanState::Code;
                }
            }
            ScanState::BlockComment => {
                out.push(c);
                if c == '*' && i + 1 < chars.len() && chars[i + 1] == '/' {
                    out.push('/');
                    i += 2;
                    state = ScanState::Code;
                    continue;
                }
                i += 1;
            }
        }
    }

    out
}

#[derive(Clone, Copy)]
enum ScanState {
    Code,
    SingleQuote,
    DoubleQuote,
    Template,
    LineComment,
    BlockComment,
}

fn rewrite_while(chars: &[char], start: usize) -> (usize, String) {
    let mut out = String::from("while");
    let mut i = start + 5;
    while i < chars.len() && chars[i].is_whitespace() {
        out.push(chars[i]);
        i += 1;
    }
    if i >= chars.len() || chars[i] != '(' {
        return (start + 1, chars[start].to_string());
    }

    if let Some((cond, close_idx)) = parse_balanced_paren(chars, i) {
        out.push_str(&format!("((__tj_tick(), ({cond})))"));
        (close_idx + 1, out)
    } else {
        (start + 1, chars[start].to_string())
    }
}

fn rewrite_for(chars: &[char], start: usize) -> (usize, String) {
    let mut out = String::from("for");
    let mut i = start + 3;
    while i < chars.len() && chars[i].is_whitespace() {
        out.push(chars[i]);
        i += 1;
    }
    if i >= chars.len() || chars[i] != '(' {
        return (start + 1, chars[start].to_string());
    }

    if let Some((header, close_idx)) = parse_balanced_paren(chars, i) {
        let parts = split_top_level_semicolons(&header);
        if parts.len() == 3 {
            let cond = if parts[1].trim().is_empty() {
                "__tj_tick(), true".to_string()
            } else {
                format!("__tj_tick(), ({})", parts[1])
            };
            out.push('(');
            out.push_str(parts[0].trim_end());
            out.push_str("; (");
            out.push_str(&cond);
            out.push_str("); ");
            out.push_str(parts[2].trim_start());
            out.push(')');
        } else {
            out.push('(');
            out.push_str(&header);
            out.push(')');
        }
        (close_idx + 1, out)
    } else {
        (start + 1, chars[start].to_string())
    }
}

fn rewrite_do(chars: &[char], start: usize) -> (usize, String) {
    let mut out = String::from("do");
    let mut i = start + 2;

    while i < chars.len() && chars[i].is_whitespace() {
        out.push(chars[i]);
        i += 1;
    }

    if i < chars.len() && chars[i] == '{' {
        out.push('{');
        out.push_str(" __tj_tick();");
        return (i + 1, out);
    }

    (start + 1, chars[start].to_string())
}

fn starts_with_keyword(chars: &[char], start: usize, kw: &str) -> bool {
    let kw_chars: Vec<char> = kw.chars().collect();
    if start + kw_chars.len() > chars.len() {
        return false;
    }

    for (idx, kc) in kw_chars.iter().enumerate() {
        if chars[start + idx] != *kc {
            return false;
        }
    }

    if start > 0 {
        let prev = chars[start - 1];
        if prev.is_alphanumeric() || prev == '_' || prev == '$' {
            return false;
        }
    }

    if start + kw_chars.len() < chars.len() {
        let next = chars[start + kw_chars.len()];
        if next.is_alphanumeric() || next == '_' || next == '$' {
            return false;
        }
    }

    true
}

fn parse_balanced_paren(chars: &[char], open_idx: usize) -> Option<(String, usize)> {
    if chars.get(open_idx) != Some(&'(') {
        return None;
    }

    let mut out = String::new();
    let mut depth = 0usize;
    let mut i = open_idx;
    let mut state = ScanState::Code;

    while i < chars.len() {
        let c = chars[i];
        match state {
            ScanState::Code => {
                if c == '\'' {
                    state = ScanState::SingleQuote;
                } else if c == '"' {
                    state = ScanState::DoubleQuote;
                } else if c == '`' {
                    state = ScanState::Template;
                } else if c == '(' {
                    depth += 1;
                    if depth > 1 {
                        out.push(c);
                    }
                } else if c == ')' {
                    if depth == 0 {
                        return None;
                    }
                    depth -= 1;
                    if depth == 0 {
                        return Some((out, i));
                    }
                    out.push(c);
                } else {
                    out.push(c);
                }
                i += 1;
            }
            ScanState::SingleQuote => {
                out.push(c);
                if c == '\\' && i + 1 < chars.len() {
                    out.push(chars[i + 1]);
                    i += 2;
                    continue;
                }
                if c == '\'' {
                    state = ScanState::Code;
                }
                i += 1;
            }
            ScanState::DoubleQuote => {
                out.push(c);
                if c == '\\' && i + 1 < chars.len() {
                    out.push(chars[i + 1]);
                    i += 2;
                    continue;
                }
                if c == '"' {
                    state = ScanState::Code;
                }
                i += 1;
            }
            ScanState::Template => {
                out.push(c);
                if c == '\\' && i + 1 < chars.len() {
                    out.push(chars[i + 1]);
                    i += 2;
                    continue;
                }
                if c == '`' {
                    state = ScanState::Code;
                }
                i += 1;
            }
            ScanState::LineComment | ScanState::BlockComment => unreachable!(),
        }
    }

    None
}

fn split_top_level_semicolons(input: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0usize;
    let mut depth_paren = 0usize;
    let mut depth_brace = 0usize;
    let mut depth_bracket = 0usize;
    let chars: Vec<char> = input.chars().collect();
    let mut i = 0usize;
    let mut state = ScanState::Code;

    while i < chars.len() {
        let c = chars[i];
        match state {
            ScanState::Code => {
                match c {
                    '\'' => state = ScanState::SingleQuote,
                    '"' => state = ScanState::DoubleQuote,
                    '`' => state = ScanState::Template,
                    '(' => depth_paren += 1,
                    ')' => depth_paren = depth_paren.saturating_sub(1),
                    '{' => depth_brace += 1,
                    '}' => depth_brace = depth_brace.saturating_sub(1),
                    '[' => depth_bracket += 1,
                    ']' => depth_bracket = depth_bracket.saturating_sub(1),
                    ';' => {
                        if depth_paren == 0 && depth_brace == 0 && depth_bracket == 0 {
                            parts.push(&input[start..i]);
                            start = i + 1;
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            ScanState::SingleQuote => {
                if c == '\\' && i + 1 < chars.len() {
                    i += 2;
                    continue;
                }
                if c == '\'' {
                    state = ScanState::Code;
                }
                i += 1;
            }
            ScanState::DoubleQuote => {
                if c == '\\' && i + 1 < chars.len() {
                    i += 2;
                    continue;
                }
                if c == '"' {
                    state = ScanState::Code;
                }
                i += 1;
            }
            ScanState::Template => {
                if c == '\\' && i + 1 < chars.len() {
                    i += 2;
                    continue;
                }
                if c == '`' {
                    state = ScanState::Code;
                }
                i += 1;
            }
            ScanState::LineComment | ScanState::BlockComment => unreachable!(),
        }
    }

    parts.push(&input[start..]);
    parts
}
