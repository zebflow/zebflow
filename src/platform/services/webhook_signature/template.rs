//! The `signed` template: the exact text a vendor signed, written as literal
//! text and a few named pieces. Parsed once when the credential is saved and
//! again when it is read; a request never meets a template it cannot build.

/// One piece of the signed text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Piece {
    Text(String),
    /// The request body, the raw bytes as they arrived.
    Body,
    /// The timestamp as the vendor sent it, never reformatted.
    Timestamp,
    /// One request header's value, by name.
    Header(String),
    /// One `key=value` piece of the header `parts` names.
    Part(String),
    Method,
    /// The path the vendor called, as it called it.
    Path,
    /// The full address the vendor called: scheme, host, path and query.
    Url,
}

/// `{body}`, `{timestamp}`, `{header:Name}`, `{part:key}`, `{method}`,
/// `{path}` and `{url}`, with `{{` and `}}` for literal braces. Anything else
/// in braces is refused with its name.
pub fn parse(template: &str) -> Result<Vec<Piece>, String> {
    let mut pieces = Vec::new();
    let mut text = String::new();
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '{' if chars.peek() == Some(&'{') => {
                chars.next();
                text.push('{');
            }
            '}' if chars.peek() == Some(&'}') => {
                chars.next();
                text.push('}');
            }
            '}' => return Err("a lone `}` in signed: write `}}` for a literal brace".to_string()),
            '{' => {
                let mut name = String::new();
                loop {
                    match chars.next() {
                        Some('}') => break,
                        Some(ch) => name.push(ch),
                        None => return Err(format!("`{{{name}` in signed is never closed")),
                    }
                }
                if !text.is_empty() {
                    pieces.push(Piece::Text(std::mem::take(&mut text)));
                }
                pieces.push(piece(&name)?);
            }
            other => text.push(other),
        }
    }
    if !text.is_empty() {
        pieces.push(Piece::Text(text));
    }
    if pieces.is_empty() {
        return Err("signed is empty: write what the vendor signs, for example {body}".to_string());
    }
    Ok(pieces)
}

fn piece(name: &str) -> Result<Piece, String> {
    let name = name.trim();
    if let Some(header) = name.strip_prefix("header:") {
        let header = header.trim();
        if !super::valid_header_name(header) {
            return Err(format!("`{{header:{header}}}` does not name a header"));
        }
        return Ok(Piece::Header(header.to_ascii_lowercase()));
    }
    if let Some(key) = name.strip_prefix("part:") {
        let key = key.trim();
        if key.is_empty() {
            return Err("`{part:}` names no part".to_string());
        }
        return Ok(Piece::Part(key.to_string()));
    }
    match name {
        "body" => Ok(Piece::Body),
        "timestamp" => Ok(Piece::Timestamp),
        "method" => Ok(Piece::Method),
        "path" => Ok(Piece::Path),
        "url" => Ok(Piece::Url),
        other => Err(format!(
            "`{{{other}}}` is not a piece signed can use: body, timestamp, header:Name, part:key, method, path, url"
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_template_reads_its_pieces_and_its_literal_braces() {
        assert_eq!(
            parse("v0:{timestamp}:{body}").unwrap(),
            vec![Piece::Text("v0:".into()), Piece::Timestamp, Piece::Text(":".into()), Piece::Body]
        );
        assert_eq!(
            parse("{header:Webhook-Id}.{part:t}.{{x}}").unwrap(),
            vec![
                Piece::Header("webhook-id".into()),
                Piece::Text(".".into()),
                Piece::Part("t".into()),
                Piece::Text(".{x}".into()),
            ]
        );
    }

    #[test]
    fn a_template_refuses_what_it_cannot_build() {
        assert!(parse("").is_err());
        assert!(parse("{bdy}").unwrap_err().contains("bdy"));
        assert!(parse("{body").is_err());
        assert!(parse("body}").is_err());
        assert!(parse("{header:}").is_err());
        assert!(parse("{header:Bad Name}").is_err());
        assert!(parse("{part:}").is_err());
    }
}
