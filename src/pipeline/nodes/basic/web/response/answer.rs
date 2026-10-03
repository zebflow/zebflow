//! What `web.response.send` answers, decided before anything is sent: which
//! of `--body`, `--template` or `--file`, the status, every header in order —
//! a repeated header stays repeated, every value sent as written — and the
//! redirect rule.
//!
//! The decision travels to the HTTP layer as an envelope under
//! [`super::ENVELOPE_KEY`]; the engine lifts it out of the payload, so the
//! next node sees the payload it was given.

use serde_json::{Map, Value, json};

use super::Config;
use crate::pipeline::PipelineError;

pub const CODE_CONFIG: &str = "FW_NODE_WEB_RESPONSE_SEND_CONFIG";
pub const CODE_HEADER: &str = "FW_NODE_WEB_RESPONSE_SEND_HEADER";
pub const CODE_REDIRECT: &str = "FW_NODE_WEB_RESPONSE_SEND_REDIRECT";

/// The status and headers of a response, checked.
#[derive(Debug, Clone, PartialEq)]
pub struct Head {
    pub status: Option<u16>,
    /// `(name, value)` in the order given; a name may repeat.
    pub headers: Vec<(String, String)>,
}

impl Head {
    pub fn has(&self, name: &str) -> bool {
        self.headers.iter().any(|(key, _)| key.eq_ignore_ascii_case(name))
    }

    /// Adds a header unless the author set one of that name.
    pub fn default_header(&mut self, name: &str, value: &str) {
        if !self.has(name) {
            self.headers.push((name.to_string(), value.to_string()));
        }
    }

    /// A redirect with nothing to say: a 3xx answers no body.
    pub fn is_redirect(&self) -> bool {
        self.status.is_some_and(|status| (300..400).contains(&status))
    }

    /// The envelope's head; the body keys are added beside it.
    pub fn envelope(&self) -> Map<String, Value> {
        let mut map = Map::new();
        map.insert("status".to_string(), json!(self.status));
        map.insert(
            "headers".to_string(),
            Value::Array(self.headers.iter().map(|(k, v)| json!([k, v])).collect()),
        );
        map
    }
}

impl Config {
    /// Everything about the configuration that can be refused before a byte
    /// is read: at most one source, `--root` only with `--file`, and the head.
    pub fn check(&self) -> Result<Head, PipelineError> {
        let sources = [self.body.is_some(), self.template.is_some(), self.file.is_some()];
        if sources.iter().filter(|given| **given).count() > 1 {
            return Err(PipelineError::new(
                CODE_CONFIG,
                "web.response.send answers one of --body, --template or --file",
            ));
        }
        if self.root.is_some() && self.file.is_none() {
            return Err(PipelineError::new(CODE_CONFIG, "--root confines --file; give --file too"));
        }
        self.head()
    }

    /// The status and headers: every value one line of text sent as
    /// written (a `Set-Cookie` too — nothing is added), every name a header
    /// name, and a redirect whole.
    pub fn head(&self) -> Result<Head, PipelineError> {
        let mut headers = Vec::new();
        for (name, value) in &self.headers {
            if name.is_empty() || !name.bytes().all(is_token_byte) {
                return Err(PipelineError::new(CODE_HEADER, format!("--header '{name}' is not a header name")));
            }
            let values: Vec<&Value> = match value {
                Value::Array(items) => items.iter().collect(),
                other => vec![other],
            };
            for value in values {
                headers.push((name.clone(), header_text(name, value)?));
            }
        }
        let head = Head { status: self.status, headers };
        let locations = head.headers.iter().filter(|(key, _)| key.eq_ignore_ascii_case("location")).count();
        if locations > 1 {
            return Err(PipelineError::new(CODE_REDIRECT, "a response has one Location header"));
        }
        match (head.status, locations) {
            (Some(status), 1) if (300..400).contains(&status) => Ok(head),
            (_, 1) => Err(PipelineError::new(
                CODE_REDIRECT,
                "a Location header needs a 3xx --status: --status 303 --header \"Location=/home\"",
            )),
            // 304 Not Modified is the one 3xx that points nowhere.
            (Some(status), _) if (300..400).contains(&status) && status != 304 => Err(PipelineError::new(
                CODE_REDIRECT,
                format!("--status {status} redirects; say where with --header \"Location=/home\""),
            )),
            _ => Ok(head),
        }
    }
}

/// RFC 9110 `tchar`.
fn is_token_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&byte)
}

fn header_text(name: &str, value: &Value) -> Result<String, PipelineError> {
    let text = match value {
        Value::String(text) => text.clone(),
        Value::Number(number) => number.to_string(),
        Value::Bool(flag) => flag.to_string(),
        _ => return Err(PipelineError::new(CODE_HEADER, format!("--header {name} must be text"))),
    };
    if text.contains(['\r', '\n']) {
        return Err(PipelineError::new(CODE_HEADER, format!("--header {name} must be one line")));
    }
    Ok(text)
}

/// The envelope for `--body`, or for no source at all (the payload). A
/// string answers text, anything else JSON; a `Content-Type` header the
/// author set wins over either. A redirect without `--body` answers nothing.
pub fn body_envelope(config: &Config, head: &Head, payload: &Value) -> Value {
    let mut envelope = head.envelope();
    let body = match &config.body {
        Some(body) => Some(body.clone()),
        None if head.is_redirect() => None,
        None => Some(payload.clone()),
    };
    match body {
        Some(Value::String(text)) => {
            envelope.insert("text".to_string(), Value::String(text));
        }
        Some(value) => {
            envelope.insert("json".to_string(), value);
        }
        None => {}
    }
    Value::Object(envelope)
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::{Config, body_envelope};

    fn config(value: Value) -> Config {
        serde_json::from_value(value).expect("config")
    }

    #[test]
    fn a_redirect_is_a_3xx_with_a_location_and_nothing_less() {
        let head = config(json!({ "status": 303, "headers": { "Location": "/home" } })).check().unwrap();
        assert_eq!(head.headers, vec![("Location".to_string(), "/home".to_string())]);
        assert!(head.is_redirect());
        for refused in [
            json!({ "headers": { "Location": "/home" } }),
            json!({ "status": 200, "headers": { "location": "/home" } }),
            json!({ "status": 302 }),
            json!({ "status": 303, "headers": { "Location": ["/a", "/b"] } }),
        ] {
            let err = config(refused.clone()).check().unwrap_err();
            assert_eq!(err.code, super::CODE_REDIRECT, "{refused}");
        }
        assert!(config(json!({ "status": 304 })).check().is_ok(), "304 points nowhere");
    }

    #[test]
    fn a_repeated_header_stays_two_headers_in_order() {
        let head = config(json!({
            "headers": { "Set-Cookie": ["session=a1; Path=/; Max-Age=86400; SameSite=Lax; HttpOnly", "theme=dark"], "X-Trace": "t1" }
        }))
        .check()
        .unwrap();
        assert_eq!(head.envelope()["headers"][2], json!(["X-Trace", "t1"]));
        assert_eq!(head.headers.iter().filter(|(k, _)| k == "Set-Cookie").count(), 2);
    }

    /// A cookie is a header like any other: what the author wrote is what
    /// the browser gets — nothing added, nothing reordered.
    #[test]
    fn a_set_cookie_header_is_sent_exactly_as_written() {
        for written in [
            "theme=dark",
            "session=a1; Path=/; Max-Age=86400; SameSite=Lax; HttpOnly; Secure",
            "session=; Path=/; Max-Age=0",
            "id=7; expires=Wed, 21 Oct 2026 07:28:00 GMT",
        ] {
            let head = config(json!({ "headers": { "Set-Cookie": written } })).check().unwrap();
            assert_eq!(head.headers, vec![("Set-Cookie".to_string(), written.to_string())]);
        }
    }

    #[test]
    fn one_source_at_most_and_a_root_only_for_a_file() {
        for refused in [
            json!({ "body": "x", "template": "pages/a.tsx" }),
            json!({ "body": "x", "file": "a.txt" }),
            json!({ "template": "pages/a.tsx", "file": "a.txt" }),
            json!({ "root": "pwa/icons" }),
        ] {
            assert_eq!(config(refused.clone()).check().unwrap_err().code, super::CODE_CONFIG, "{refused}");
        }
        assert!(config(json!({ "root": "pwa/icons", "file": "a.png" })).check().is_ok());
    }

    #[test]
    fn a_header_is_one_line_of_text_under_a_header_name() {
        for refused in [
            json!({ "headers": { "X Bad": "v" } }),
            json!({ "headers": { "X-A": "one\r\nX-B: two" } }),
            json!({ "headers": { "X-A": { "nested": true } } }),
        ] {
            assert_eq!(config(refused.clone()).check().unwrap_err().code, super::CODE_HEADER, "{refused}");
        }
        let head = config(json!({ "headers": { "X-Count": 3 } })).check().unwrap();
        assert_eq!(head.headers, vec![("X-Count".to_string(), "3".to_string())]);
    }

    #[test]
    fn a_string_body_is_text_and_anything_else_json() {
        let payload = json!({ "rows": [1] });
        let text = config(json!({ "body": "hello" }));
        assert_eq!(body_envelope(&text, &text.check().unwrap(), &payload)["text"], "hello");
        let rows = config(json!({ "body": [1, 2] }));
        assert_eq!(body_envelope(&rows, &rows.check().unwrap(), &payload)["json"], json!([1, 2]));
        let none = config(json!({}));
        assert_eq!(body_envelope(&none, &none.check().unwrap(), &payload)["json"], payload);
        let redirect = config(json!({ "status": 303, "headers": { "Location": "/home" } }));
        let envelope = body_envelope(&redirect, &redirect.check().unwrap(), &payload);
        assert!(envelope.get("json").is_none() && envelope.get("text").is_none(), "{envelope}");
    }
}
