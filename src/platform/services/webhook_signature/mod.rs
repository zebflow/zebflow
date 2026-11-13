//! Webhook signatures, described entirely by an `hmac` credential.
//!
//! A vendor signs a text built from the request (the raw body, a timestamp,
//! another header) with a shared secret and sends the result in a header.
//! The credential says which text, which header, which encoding and how fresh
//! the request must be; this module checks one request against it. A preset is
//! only a filled-in credential without the secret, so a new vendor needs a
//! form, not code. Checked at the webhook's door, before any node runs.
//!
//! Nothing here returns or logs a secret, the received or computed signature,
//! or the signed text: a refusal carries a code and a sentence naming the
//! setting or header that failed.

mod presets;
mod template;

use hmac::{Hmac, Mac};
use serde_json::Value;

pub use presets::presets;
use template::Piece;

/// Why a request was refused. `code` is stable; `message` names the setting
/// or header, never a value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refusal {
    pub code: &'static str,
    pub message: String,
}

fn refuse(code: &'static str, message: impl Into<String>) -> Refusal {
    Refusal { code: code.into(), message: message.into() }
}

/// The parts of a request a signature can cover.
pub struct Request<'a> {
    /// A header's first value by its name, any case.
    pub header: &'a dyn Fn(&str) -> Option<String>,
    pub body: &'a [u8],
    pub method: &'a str,
    /// The path the vendor called, as it called it.
    pub path: &'a str,
    /// The full address the vendor called.
    pub url: &'a str,
    /// Unix seconds.
    pub now: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Algorithm {
    Sha1,
    Sha256,
    Sha512,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Encoding {
    Hex,
    Base64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Source {
    Header(String),
    Part(String),
}

#[derive(Debug, Clone)]
struct Signature {
    from: Source,
    prefix: String,
    encoding: Encoding,
    split: String,
}

#[derive(Debug, Clone)]
struct Timestamp {
    from: Source,
    milliseconds: bool,
    tolerance: u64,
}

#[derive(Debug, Clone)]
struct Parts {
    header: String,
    separator: String,
    assign: String,
}

/// One credential's check, read and validated.
#[derive(Debug, Clone)]
pub struct Settings {
    secrets: Vec<Vec<u8>>,
    algorithm: Algorithm,
    signed: Vec<Piece>,
    signature: Signature,
    timestamp: Option<Timestamp>,
    parts: Option<Parts>,
}

/// The keys an `hmac` credential may hold. `secrets` is the only secret one.
pub const KEYS: &[&str] = &["preset", "secrets", "secret_encoding", "secret_prefix", "algorithm", "signed", "signature", "timestamp", "parts"];

/// The keys shown as they are when a credential is read; `secrets` is masked.
pub const PLAIN_KEYS: &[&str] = &["preset", "secret_encoding", "secret_prefix", "algorithm", "signed", "signature", "timestamp", "parts"];

/// The settings before 0.11.28. Refused by name, so an old credential says
/// what to do instead of failing on an unknown key.
const OLD_KEYS: &[&str] = &["provider", "secret", "signature_header", "signature_prefix", "signature_encoding", "replay_tolerance"];

const DEFAULT_TOLERANCE: u64 = 300;
const MAX_TOLERANCE: u64 = 86_400;

/// Checks an `hmac` credential when it is saved. It always holds the key
/// (`secrets`); the check's settings (`signed`, `signature`, …) are needed
/// only by `--auth hmac`, so a credential that `crypto.signature.*` signs with
/// may hold the key alone.
pub fn validate(value: &Value) -> Result<(), String> {
    let has_check = ["signed", "signature", "timestamp", "parts"]
        .iter()
        .any(|key| value.get(key).is_some_and(|v| !v.is_null()));
    if has_check { Settings::from_value(value).map(|_| ()) } else { key(value).map(|_| ()) }
}

/// The key an `hmac` credential signs with: its current secret, decoded as
/// `secret_encoding` says. What `crypto.signature.*` uses.
pub fn key(value: &Value) -> Result<Vec<u8>, String> {
    let map = value.as_object().ok_or("an hmac credential is an object of settings")?;
    check_keys(map)?;
    let secret_encoding = text(map.get("secret_encoding"), "text", "secret_encoding")?;
    let secret_prefix = text(map.get("secret_prefix"), "", "secret_prefix")?;
    let mut secrets = read_secrets(map.get("secrets"), &secret_encoding, &secret_prefix)?;
    Ok(secrets.remove(0))
}

/// An `hmac` credential saved before 0.11.28 (`provider`, `secret`,
/// `signature_header`, …) as the settings that run the same check: the same
/// secret, header, prefix, encoding and algorithm, and for `stripe` and
/// `slack` the same signed text and timestamp window. `None` when it is not in
/// the old shape. Used once per credential at startup
/// (`CredentialService::convert_old_hmac_credentials`) so a webhook that
/// worked before the upgrade keeps working; delete with the last 0.11.27 office.
pub fn from_old_shape(value: &Value) -> Option<Value> {
    let map = value.as_object()?;
    if !OLD_KEYS.iter().any(|key| map.contains_key(*key)) {
        return None;
    }
    let text = |key: &str| map.get(key).and_then(Value::as_str).map(str::trim).filter(|v| !v.is_empty());
    let provider = text("provider").unwrap_or("generic");
    let header = text("signature_header").unwrap_or("X-Hub-Signature-256");
    let prefix = text("signature_prefix").unwrap_or("");
    let encoding = if text("signature_encoding") == Some("base64") { "base64" } else { "hex" };
    let algorithm = if text("algorithm") == Some("sha1") { "sha1" } else { "sha256" };
    let tolerance = map.get("replay_tolerance").and_then(Value::as_u64).filter(|t| *t > 0);
    let secret = map.get("secret").and_then(Value::as_str).unwrap_or("");
    let mut settings = serde_json::json!({
        "secrets": { "current": secret },
        "algorithm": algorithm,
    });
    match provider {
        // `t=…,v1=…` in the signature header, `{t}.{body}` signed; the window
        // only when a tolerance was set.
        "stripe" => {
            settings["parts"] = serde_json::json!({ "from": format!("header:{header}"), "separator": ",", "assign": "=" });
            settings["signature"] = serde_json::json!({ "from": "part:v1", "encoding": encoding });
            match tolerance {
                Some(t) => {
                    settings["signed"] = Value::String("{timestamp}.{body}".into());
                    settings["timestamp"] = serde_json::json!({ "from": "part:t", "unit": "seconds", "tolerance": t.min(MAX_TOLERANCE) });
                }
                None => settings["signed"] = Value::String("{part:t}.{body}".into()),
            }
        }
        // `v0:{ts}:{body}` with the timestamp in X-Slack-Request-Timestamp.
        "slack" => {
            settings["signature"] = serde_json::json!({ "from": format!("header:{header}"), "prefix": prefix, "encoding": encoding });
            match tolerance {
                Some(t) => {
                    settings["signed"] = Value::String("v0:{timestamp}:{body}".into());
                    settings["timestamp"] = serde_json::json!({ "from": "header:X-Slack-Request-Timestamp", "unit": "seconds", "tolerance": t.min(MAX_TOLERANCE) });
                }
                None => settings["signed"] = Value::String("v0:{header:X-Slack-Request-Timestamp}:{body}".into()),
            }
        }
        // generic, github, shopify: the raw body.
        _ => {
            settings["signed"] = Value::String("{body}".into());
            settings["signature"] = serde_json::json!({ "from": format!("header:{header}"), "prefix": prefix, "encoding": encoding });
        }
    }
    let preset = match provider {
        "github" | "stripe" | "slack" | "shopify" => Some(provider),
        _ => None,
    };
    if let Some(preset) = preset {
        settings["preset"] = Value::String(preset.into());
    }
    Some(settings)
}

fn check_keys(map: &serde_json::Map<String, Value>) -> Result<(), String> {
    if let Some(old) = OLD_KEYS.iter().find(|key| map.contains_key(**key)) {
        return Err(format!(
            "`{old}` is an hmac setting from before 0.11.28: put the key in secrets.current (and choose a preset or fill signed and signature for --auth hmac), and save again"
        ));
    }
    if let Some(unknown) = map.keys().find(|key| !KEYS.contains(&key.as_str())) {
        return Err(format!("`{unknown}` is not an hmac setting (known: {})", KEYS.join(", ")));
    }
    Ok(())
}

impl Settings {
    /// Reads a credential's settings. Every rule is checked here, when the
    /// credential is saved, so a request never meets a setting it cannot use.
    pub fn from_value(value: &Value) -> Result<Settings, String> {
        let map = value.as_object().ok_or("an hmac credential is an object of settings")?;
        check_keys(map)?;

        text(map.get("preset"), "", "preset")?;
        let secret_encoding = text(map.get("secret_encoding"), "text", "secret_encoding")?;
        let secret_prefix = text(map.get("secret_prefix"), "", "secret_prefix")?;
        let secrets = read_secrets(map.get("secrets"), &secret_encoding, &secret_prefix)?;

        let algorithm = match text(map.get("algorithm"), "sha256", "algorithm")?.as_str() {
            "sha1" => Algorithm::Sha1,
            "sha256" => Algorithm::Sha256,
            "sha512" => Algorithm::Sha512,
            other => return Err(format!("algorithm `{other}` is not sha1, sha256 or sha512")),
        };

        let signed_text = map
            .get("signed")
            .and_then(Value::as_str)
            .ok_or("this hmac credential holds a key but no signature check: fill signed (what the vendor signs, for example {body}) and signature to use it with --auth hmac")?;
        let signed = template::parse(signed_text)?;

        let parts = map.get("parts").filter(|v| !v.is_null()).map(read_parts).transpose()?;
        let signature = read_signature(map.get("signature"))?;
        let timestamp = map.get("timestamp").filter(|v| !v.is_null()).map(read_timestamp).transpose()?;

        let uses_part = signed.iter().any(|p| matches!(p, Piece::Part(_)))
            || matches!(signature.from, Source::Part(_))
            || timestamp.as_ref().is_some_and(|t| matches!(t.from, Source::Part(_)));
        if uses_part && parts.is_none() {
            return Err("a part: is read but parts does not name the header to read it from".to_string());
        }
        if signed.contains(&Piece::Timestamp) && timestamp.is_none() {
            return Err("signed uses {timestamp} but timestamp does not say where it is".to_string());
        }
        // A timestamp the signature does not cover proves nothing: a copied
        // delivery would pass again with a new time beside it.
        if let Some(ts) = &timestamp {
            let covered = signed.iter().any(|piece| match (piece, &ts.from) {
                (Piece::Timestamp, _) => true,
                (Piece::Header(name), Source::Header(from)) => name == from,
                (Piece::Part(key), Source::Part(from)) => key == from,
                _ => false,
            });
            if !covered {
                return Err(
                    "timestamp is read but not signed: put {timestamp} in signed, or a copied delivery could be sent again with a new time"
                        .to_string(),
                );
            }
        }

        Ok(Settings { secrets, algorithm, signed, signature, timestamp, parts })
    }

    /// The same check with no time window: for a delivery pasted into the
    /// Studio's test panel, which is older than any window by the time it is
    /// pasted. The timestamp is still read and still signed.
    pub fn ignoring_time(mut self) -> Settings {
        if let Some(ts) = self.timestamp.as_mut() {
            ts.tolerance = u64::MAX;
        }
        self
    }

    /// Checks one request. `Ok` when any signature it carries matches any
    /// secret; otherwise the first step that failed.
    pub fn verify(&self, request: &Request) -> Result<(), Refusal> {
        let parts = match &self.parts {
            Some(parts) => {
                let raw = (request.header)(&parts.header)
                    .ok_or_else(|| refuse("HMAC_HEADER_MISSING", format!("the request has no {} header", parts.header)))?;
                split_parts(&raw, &parts.separator, &parts.assign)
            }
            None => Vec::new(),
        };
        let read = |source: &Source| -> Option<Vec<String>> {
            match source {
                Source::Header(name) => (request.header)(name).map(|value| vec![value]),
                Source::Part(key) => {
                    let found: Vec<String> = parts.iter().filter(|(k, _)| k == key).map(|(_, v)| v.clone()).collect();
                    (!found.is_empty()).then_some(found)
                }
            }
        };

        let timestamp = match &self.timestamp {
            Some(ts) => {
                let raw = read(&ts.from)
                    .and_then(|values| values.into_iter().next())
                    .ok_or_else(|| refuse("HMAC_TIMESTAMP_MISSING", format!("the request has no timestamp in {}", describe(&ts.from))))?;
                let value: i64 = raw
                    .trim()
                    .parse()
                    .map_err(|_| refuse("HMAC_TIMESTAMP_INVALID", format!("the timestamp in {} is not a whole number", describe(&ts.from))))?;
                let seconds = if ts.milliseconds { value / 1000 } else { value };
                if request.now.abs_diff(seconds) > ts.tolerance {
                    return Err(refuse(
                        "HMAC_TIMESTAMP_STALE",
                        format!("the request's timestamp is more than {} seconds from now", ts.tolerance),
                    ));
                }
                Some(raw)
            }
            None => None,
        };

        let sent = read(&self.signature.from).ok_or_else(|| {
            refuse("HMAC_HEADER_MISSING", format!("the request has no signature in {}", describe(&self.signature.from)))
        })?;
        let items: Vec<String> = sent
            .iter()
            .flat_map(|value| {
                if self.signature.split.is_empty() {
                    vec![value.trim().to_string()]
                } else {
                    value.split(self.signature.split.as_str()).map(|s| s.trim().to_string()).collect()
                }
            })
            .filter(|item| !item.is_empty())
            .collect();
        let prefixed: Vec<&str> = items.iter().filter_map(|item| item.strip_prefix(self.signature.prefix.as_str())).collect();
        if prefixed.is_empty() {
            return Err(refuse(
                "HMAC_PREFIX_MISSING",
                format!("the signature in {} does not start with `{}`", describe(&self.signature.from), self.signature.prefix),
            ));
        }
        let candidates: Vec<Vec<u8>> = prefixed.iter().filter_map(|item| decode(item, self.signature.encoding)).collect();
        if candidates.is_empty() {
            return Err(refuse(
                "HMAC_SIGNATURE_UNREADABLE",
                format!("the signature in {} is not {}", describe(&self.signature.from), encoding_name(self.signature.encoding)),
            ));
        }

        let mut signed = Vec::with_capacity(request.body.len() + 64);
        for piece in &self.signed {
            match piece {
                Piece::Text(text) => signed.extend_from_slice(text.as_bytes()),
                Piece::Body => signed.extend_from_slice(request.body),
                Piece::Timestamp => signed.extend_from_slice(timestamp.as_deref().unwrap_or_default().as_bytes()),
                Piece::Header(name) => {
                    let value = (request.header)(name)
                        .ok_or_else(|| refuse("HMAC_HEADER_MISSING", format!("the request has no {name} header, which signed uses")))?;
                    signed.extend_from_slice(value.as_bytes());
                }
                Piece::Part(key) => {
                    let value = read(&Source::Part(key.clone()))
                        .and_then(|values| values.into_iter().next())
                        .ok_or_else(|| refuse("HMAC_HEADER_MISSING", format!("the request has no `{key}` part, which signed uses")))?;
                    signed.extend_from_slice(value.as_bytes());
                }
                Piece::Method => signed.extend_from_slice(request.method.as_bytes()),
                Piece::Path => signed.extend_from_slice(request.path.as_bytes()),
                Piece::Url => signed.extend_from_slice(request.url.as_bytes()),
            }
        }

        let matched = self.secrets.iter().any(|secret| match self.algorithm {
            Algorithm::Sha1 => matches::<Hmac<sha1::Sha1>>(secret, &signed, &candidates),
            Algorithm::Sha256 => matches::<Hmac<sha2::Sha256>>(secret, &signed, &candidates),
            Algorithm::Sha512 => matches::<Hmac<sha2::Sha512>>(secret, &signed, &candidates),
        });
        if matched {
            Ok(())
        } else {
            Err(refuse("HMAC_SIGNATURE_MISMATCH", "the signature does not match this credential's secrets"))
        }
    }
}

/// Constant-time, per candidate: `verify_slice` compares without an early exit.
fn matches<M: Mac + hmac::digest::KeyInit + Clone>(secret: &[u8], signed: &[u8], candidates: &[Vec<u8>]) -> bool {
    let Ok(mut mac) = <M as hmac::digest::KeyInit>::new_from_slice(secret) else {
        return false;
    };
    mac.update(signed);
    candidates.iter().any(|candidate| mac.clone().verify_slice(candidate).is_ok())
}

fn read_secrets(value: Option<&Value>, encoding: &str, prefix: &str) -> Result<Vec<Vec<u8>>, String> {
    let map = value
        .and_then(Value::as_object)
        .ok_or("secrets is missing: { \"current\": \"...\" }, and \"previous\" while a secret is being replaced")?;
    if let Some(unknown) = map.keys().find(|key| *key != "current" && *key != "previous") {
        return Err(format!("secrets.{unknown} is not a slot: use current and previous"));
    }
    let mut out = Vec::new();
    for slot in ["current", "previous"] {
        let text = match map.get(slot) {
            None | Some(Value::Null) => "",
            Some(Value::String(text)) => text.as_str(),
            Some(_) => return Err(format!("secrets.{slot} is text")),
        };
        if text.trim().is_empty() {
            if slot == "current" {
                return Err("secrets.current is empty".to_string());
            }
            continue;
        }
        // A text secret is the key byte for byte, spaces included: the vendor
        // signs with exactly what it shows. Encoded ones ignore spaces around.
        let bytes = match encoding {
            "text" => text.strip_prefix(prefix).unwrap_or(text).as_bytes().to_vec(),
            "base64" => {
                let text = text.trim();
                decode(text.strip_prefix(prefix).unwrap_or(text), Encoding::Base64).ok_or(format!("secrets.{slot} is not base64"))?
            }
            "hex" => {
                let text = text.trim();
                decode(text.strip_prefix(prefix).unwrap_or(text), Encoding::Hex).ok_or(format!("secrets.{slot} is not hex"))?
            }
            other => return Err(format!("secret_encoding `{other}` is not text, base64 or hex")),
        };
        if bytes.is_empty() {
            return Err(format!("secrets.{slot} is empty after decoding"));
        }
        out.push(bytes);
    }
    Ok(out)
}

fn read_signature(value: Option<&Value>) -> Result<Signature, String> {
    let map = value.and_then(Value::as_object).ok_or("signature is missing: { \"from\": \"header:Name\" }")?;
    only_keys(map, "signature", &["from", "prefix", "encoding", "split"])?;
    Ok(Signature {
        from: read_source(map.get("from"), "signature.from")?,
        prefix: text(map.get("prefix"), "", "signature.prefix")?,
        encoding: match text(map.get("encoding"), "hex", "signature.encoding")?.as_str() {
            "hex" => Encoding::Hex,
            "base64" => Encoding::Base64,
            other => return Err(format!("signature.encoding `{other}` is not hex or base64")),
        },
        split: text(map.get("split"), "", "signature.split")?,
    })
}

fn read_timestamp(value: &Value) -> Result<Timestamp, String> {
    let map = value.as_object().ok_or("timestamp is an object: { \"from\": \"header:Name\" }")?;
    only_keys(map, "timestamp", &["from", "unit", "tolerance"])?;
    let milliseconds = match text(map.get("unit"), "seconds", "timestamp.unit")?.as_str() {
        "seconds" => false,
        "milliseconds" => true,
        other => return Err(format!("timestamp.unit `{other}` is not seconds or milliseconds")),
    };
    let tolerance = match map.get("tolerance") {
        None | Some(Value::Null) => DEFAULT_TOLERANCE,
        Some(v) => v.as_u64().ok_or("timestamp.tolerance is a whole number of seconds")?,
    };
    if tolerance == 0 || tolerance > MAX_TOLERANCE {
        return Err(format!("timestamp.tolerance must be between 1 and {MAX_TOLERANCE} seconds"));
    }
    Ok(Timestamp { from: read_source(map.get("from"), "timestamp.from")?, milliseconds, tolerance })
}

fn read_parts(value: &Value) -> Result<Parts, String> {
    let map = value.as_object().ok_or("parts is an object: { \"from\": \"header:Name\" }")?;
    only_keys(map, "parts", &["from", "separator", "assign"])?;
    let header = match read_source(map.get("from"), "parts.from")? {
        Source::Header(name) => name,
        Source::Part(_) => return Err("parts.from is a header: header:Name".to_string()),
    };
    let separator = text(map.get("separator"), ",", "parts.separator")?;
    let assign = text(map.get("assign"), "=", "parts.assign")?;
    if separator.is_empty() || assign.is_empty() {
        return Err("parts.separator and parts.assign cannot be empty".to_string());
    }
    Ok(Parts { header, separator, assign })
}

fn read_source(value: Option<&Value>, field: &str) -> Result<Source, String> {
    let text = value.and_then(Value::as_str).map(str::trim).unwrap_or("");
    if let Some(name) = text.strip_prefix("header:") {
        let name = name.trim();
        if !valid_header_name(name) {
            return Err(format!("{field}: `{name}` is not a header name"));
        }
        return Ok(Source::Header(name.to_ascii_lowercase()));
    }
    if let Some(key) = text.strip_prefix("part:") {
        let key = key.trim();
        if key.is_empty() {
            return Err(format!("{field}: part: names no part"));
        }
        return Ok(Source::Part(key.to_string()));
    }
    Err(format!("{field} is header:Name or part:key"))
}

fn only_keys(map: &serde_json::Map<String, Value>, field: &str, known: &[&str]) -> Result<(), String> {
    match map.keys().find(|key| !known.contains(&key.as_str())) {
        Some(unknown) => Err(format!("{field}.{unknown} is not a setting (known: {})", known.join(", "))),
        None => Ok(()),
    }
}

/// A text setting: its default when absent, refused when it is anything but
/// text, so a mistyped setting never quietly becomes the default.
fn text(value: Option<&Value>, default: &str, field: &str) -> Result<String, String> {
    match value {
        None | Some(Value::Null) => Ok(default.to_string()),
        Some(Value::String(text)) => Ok(text.clone()),
        Some(_) => Err(format!("{field} is text")),
    }
}

/// RFC 9110 token characters.
pub(crate) fn valid_header_name(name: &str) -> bool {
    !name.is_empty()
        && name.bytes().all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
}

fn split_parts(raw: &str, separator: &str, assign: &str) -> Vec<(String, String)> {
    raw.split(separator)
        .filter_map(|piece| piece.trim().split_once(assign))
        .map(|(k, v)| (k.trim().to_string(), v.trim().to_string()))
        .collect()
}

fn decode(text: &str, encoding: Encoding) -> Option<Vec<u8>> {
    match encoding {
        Encoding::Hex => hex::decode(text).ok(),
        Encoding::Base64 => {
            use base64::Engine as _;
            base64::engine::general_purpose::STANDARD.decode(text).ok()
        }
    }
}

fn encoding_name(encoding: Encoding) -> &'static str {
    match encoding {
        Encoding::Hex => "hex",
        Encoding::Base64 => "base64",
    }
}

fn describe(source: &Source) -> String {
    match source {
        Source::Header(name) => format!("the {name} header"),
        Source::Part(key) => format!("the `{key}` part"),
    }
}

#[cfg(test)]
mod tests;
