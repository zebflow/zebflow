//! `n.input.*` — declare and check one field of the trigger envelope.
//!
//! A trigger delivers one envelope: `body` (fields) and `files` (FileRefs).
//! An input node names one field of it, checks it, and passes the envelope
//! through **unchanged** — with one exception: a body field that was not
//! sent and has a `--default` is **written into the envelope** at
//! `body.<name>` as the checked default, so a DSL or MCP `execute` that
//! omits it, a webhook post, and the Run form all leave the same envelope
//! (files have no default). It fetches nothing and stores nothing. The set of
//! input nodes reachable from a trigger *is* that trigger's declaration: the
//! Studio builds a Run form from it, and an agent reads what a pipeline
//! expects from it. The same nodes work after `trigger.manual` and after
//! `trigger.webhook`.
//!
//! | kind | value | checks |
//! |---|---|---|
//! | `input.text` | string | non-empty unless optional; `--max` length |
//! | `input.number` | number | `--min`, `--max` |
//! | `input.boolean` | bool | — |
//! | `input.json` | any JSON | parses (a JSON string in `body` is parsed) |
//! | `input.file` | one FileRef | `--accept` (FileRef kinds, mimes, extensions) |
//! | `input.files` | FileRef array | `--accept`, `--max` count |
//! | `input.image` / `input.audio` / `input.video` | one FileRef | `input.file` with `--accept` preset |
//!
//! Body fields are read at `payload.body.<name>`, files at
//! `payload.files.<name>`. Required by default: a missing or invalid value
//! fails the node with `FW_NODE_INPUT_MISSING` / `FW_NODE_INPUT_INVALID`.
//!
//! The payload leaves as it arrived (plus a filled default), so
//! `$trigger.files.x` and the next node's `input.body.x` keep working. The
//! node's *own* value — what `$nodes.<id>` answers — is the checked value
//! (the string, the number, the FileRef); the engine asks [`scope_value`]
//! for it on the payload the node passed, the same way it asks a kind for
//! its secret paths, and a filled default checks to itself.

pub mod audio;
pub mod boolean;
pub mod file;
pub mod files;
pub mod image;
pub mod json;
pub mod number;
pub mod text;
pub mod video;

/// The family list, one definition per input kind.
pub fn definitions() -> Vec<NodeDefinition> {
    vec![
        text::definition(),
        number::definition(),
        boolean::definition(),
        json::definition(),
        file::definition(),
        files::definition(),
        image::definition(),
        audio::definition(),
        video::definition(),
    ]
}

use async_trait::async_trait;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Value, json};

use crate::pipeline::nodes::shared::file_ref::is_file_ref;
use crate::pipeline::nodes::shared::util::resolve_path;
use crate::pipeline::model::{
    DslFlag, DslFlagKind, NodeExample, NodeFieldDef, NodeFieldType, PipelineGraph,
};
use crate::pipeline::{
    NodeDefinition, PipelineError,
    nodes::{NodeExecutionInput, NodeExecutionOutput, NodeHandler},
};

pub const KIND_PREFIX: &str = "n.input.";
const INPUT_PIN_IN: &str = "in";
const OUTPUT_PIN_OUT: &str = "out";

/// One member of the family. The table above, as a type.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputKind {
    Text,
    Number,
    Boolean,
    Json,
    File,
    Files,
    Image,
    Audio,
    Video,
}

impl InputKind {
    pub const ALL: [InputKind; 9] = [
        InputKind::Text,
        InputKind::Number,
        InputKind::Boolean,
        InputKind::Json,
        InputKind::File,
        InputKind::Files,
        InputKind::Image,
        InputKind::Audio,
        InputKind::Video,
    ];

    pub fn from_kind(kind: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|k| k.node_kind() == kind)
    }

    pub fn node_kind(self) -> &'static str {
        match self {
            InputKind::Text => text::NODE_KIND,
            InputKind::Number => number::NODE_KIND,
            InputKind::Boolean => boolean::NODE_KIND,
            InputKind::Json => json::NODE_KIND,
            InputKind::File => file::NODE_KIND,
            InputKind::Files => files::NODE_KIND,
            InputKind::Image => image::NODE_KIND,
            InputKind::Audio => audio::NODE_KIND,
            InputKind::Video => video::NODE_KIND,
        }
    }

    /// The word after `input.`.
    pub fn short(self) -> &'static str {
        self.node_kind().trim_start_matches(KIND_PREFIX)
    }

    pub fn is_file(self) -> bool {
        matches!(
            self,
            InputKind::File | InputKind::Files | InputKind::Image | InputKind::Audio | InputKind::Video
        )
    }

    /// The FileRef kind the media kinds accept unless `--accept` narrows it.
    pub fn preset_accept(self) -> Option<&'static str> {
        match self {
            InputKind::Image => Some("image"),
            InputKind::Audio => Some("audio"),
            InputKind::Video => Some("video"),
            _ => None,
        }
    }

    /// Where the value lives in the envelope: `body` or `files`.
    pub fn envelope_slot(self) -> &'static str {
        if self.is_file() { "files" } else { "body" }
    }

    /// What the node expects, in the words an error message uses.
    pub fn expects(self) -> &'static str {
        match self {
            InputKind::Text => "a non-empty string",
            InputKind::Number => "a number",
            InputKind::Boolean => "a boolean",
            InputKind::Json => "a JSON value",
            InputKind::File => "one file",
            InputKind::Files => "one or more files",
            InputKind::Image => "one image file",
            InputKind::Audio => "one audio file",
            InputKind::Video => "one video file",
        }
    }

    fn title(self) -> &'static str {
        match self {
            InputKind::Text => "Input: Text",
            InputKind::Number => "Input: Number",
            InputKind::Boolean => "Input: Boolean",
            InputKind::Json => "Input: JSON",
            InputKind::File => "Input: File",
            InputKind::Files => "Input: Files",
            InputKind::Image => "Input: Image",
            InputKind::Audio => "Input: Audio",
            InputKind::Video => "Input: Video",
        }
    }
}

pub fn is_input_kind(kind: &str) -> bool {
    InputKind::from_kind(kind).is_some()
}

/// The node's checked value for the `$nodes` scope, or `None` for any other
/// kind. Deterministic on the same payload the node passed, so the engine can
/// ask after the fact without the node reporting anything.
pub fn scope_value(kind: &str, config: &Value, payload: &Value) -> Option<Value> {
    let kind = InputKind::from_kind(kind)?;
    let config: Config = serde_json::from_value(config.clone()).ok()?;
    check(kind, &config, payload).ok()
}

/// `--accept` may be typed in the dialog as `csv,xlsx` or arrive from the DSL
/// as a list; both are the same declaration.
fn string_or_list<'de, D: Deserializer<'de>>(de: D) -> Result<Vec<String>, D::Error> {
    let raw = Value::deserialize(de)?;
    Ok(match raw {
        Value::Array(items) => items
            .into_iter()
            .filter_map(|v| v.as_str().map(|s| s.trim().to_ascii_lowercase()))
            .filter(|s| !s.is_empty())
            .collect(),
        Value::String(s) => s
            .split(',')
            .map(|p| p.trim().to_ascii_lowercase())
            .filter(|p| !p.is_empty())
            .collect(),
        _ => Vec::new(),
    })
}

/// A bound typed in the dialog is a string; from the DSL it is a number.
fn number_or_string<'de, D: Deserializer<'de>>(de: D) -> Result<Option<f64>, D::Error> {
    let raw = Value::deserialize(de)?;
    Ok(match raw {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    })
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    /// The envelope field this node declares.
    #[serde(default)]
    pub name: String,
    /// Form label; the field name when empty.
    #[serde(default)]
    pub label: Option<String>,
    /// A missing value is allowed; the node's value is then `null`.
    #[serde(default)]
    pub optional: bool,
    /// Used when the field is missing (text/number/boolean/json only).
    #[serde(default)]
    pub default: Option<Value>,
    /// FileRef kinds, mimes, or extensions a file must match.
    #[serde(default, deserialize_with = "string_or_list")]
    pub accept: Vec<String>,
    /// Lower bound (number).
    #[serde(default, deserialize_with = "number_or_string")]
    pub min: Option<f64>,
    /// Upper bound: value (number), length (text), or count (files).
    #[serde(default, deserialize_with = "number_or_string")]
    pub max: Option<f64>,
}

impl Config {
    /// The kinds a file must match, after the media presets.
    pub fn effective_accept(&self, kind: InputKind) -> Vec<String> {
        if !self.accept.is_empty() {
            return self.accept.clone();
        }
        kind.preset_accept().map(|k| vec![k.to_string()]).unwrap_or_default()
    }
}

fn missing(kind: InputKind, name: &str) -> PipelineError {
    PipelineError::new(
        "FW_NODE_INPUT_MISSING",
        format!(
            "input '{name}' is required: expected {} at {}.{name}, nothing was sent",
            kind.expects(),
            kind.envelope_slot()
        ),
    )
}

fn invalid(kind: InputKind, name: &str, detail: impl AsRef<str>) -> PipelineError {
    PipelineError::new(
        "FW_NODE_INPUT_INVALID",
        format!(
            "input '{name}' expected {} at {}.{name}: {}",
            kind.expects(),
            kind.envelope_slot(),
            detail.as_ref()
        ),
    )
}

fn describe(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(_) => "a boolean".to_string(),
        Value::Number(_) => "a number".to_string(),
        Value::String(s) => format!("a string ({} chars)", s.chars().count()),
        Value::Array(items) => format!("an array of {}", items.len()),
        Value::Object(_) if is_file_ref(value) => "a file".to_string(),
        Value::Object(_) => "an object".to_string(),
    }
}

/// Does one FileRef satisfy one `--accept` entry? A kind (`image`), a mime
/// (`image/png`), a mime family (`image/`), or an extension (`xlsx`).
fn file_matches(file: &Value, accept: &str) -> bool {
    let kind = file["kind"].as_str().unwrap_or("").to_ascii_lowercase();
    let mime = file["mime"].as_str().unwrap_or("").to_ascii_lowercase();
    let mime = mime.split(';').next().unwrap_or("").trim();
    let ext = file["filename"]
        .as_str()
        .and_then(|f| f.rsplit_once('.'))
        .map(|(_, e)| e.to_ascii_lowercase())
        .unwrap_or_default();
    let accept = accept.trim().trim_start_matches('.').to_ascii_lowercase();
    if accept.is_empty() {
        return true;
    }
    kind == accept
        || mime == accept
        || (accept.ends_with('/') && mime.starts_with(&accept))
        || (accept.ends_with("/*") && mime.starts_with(&accept[..accept.len() - 1]))
        || ext == accept
}

fn check_file(kind: InputKind, name: &str, accept: &[String], file: &Value) -> Result<(), PipelineError> {
    if !is_file_ref(file) {
        return Err(invalid(kind, name, format!("got {}", describe(file))));
    }
    if !accept.is_empty() && !accept.iter().any(|a| file_matches(file, a)) {
        let got = format!(
            "{} ({})",
            file["filename"].as_str().unwrap_or("file"),
            file["mime"].as_str().unwrap_or("unknown type")
        );
        return Err(invalid(
            kind,
            name,
            format!("accepts {} — got {got}", accept.join(", ")),
        ));
    }
    Ok(())
}

/// Checks the field this node declares and returns its value; the payload is
/// not touched. `Ok(Value::Null)` is an optional field that was not sent.
pub fn check(kind: InputKind, config: &Config, payload: &Value) -> Result<Value, PipelineError> {
    let name = config.name.trim();
    if name.is_empty() {
        return Err(PipelineError::new(
            "FW_NODE_INPUT_CONFIG",
            format!("{}: the field name is required — `{} <name>`", kind.node_kind(), kind.short()),
        ));
    }
    let raw = sent_value(kind, name, payload);
    if is_absent(&raw) {
        if !kind.is_file() {
            if let Some(default) = config.default.as_ref().filter(|d| !d.is_null()) {
                return check_value(kind, config, name, default.clone());
            }
        }
        if config.optional {
            return Ok(Value::Null);
        }
        return Err(missing(kind, name));
    }
    check_value(kind, config, name, raw)
}

/// What arrived for the field, `Null` when nothing did.
fn sent_value(kind: InputKind, name: &str, payload: &Value) -> Value {
    let slot = payload.get(kind.envelope_slot()).unwrap_or(&Value::Null);
    resolve_path(slot, name).cloned().unwrap_or(Value::Null)
}

/// Absent: `null`, an empty string, or an empty list all mean "nothing was
/// sent" — a form posts "" for a field left blank.
fn is_absent(raw: &Value) -> bool {
    match raw {
        Value::Null => true,
        Value::String(s) => s.trim().is_empty(),
        Value::Array(items) => items.is_empty(),
        _ => false,
    }
}

/// True when this node's `--default` stands in for a field that was not
/// sent — the case where the node writes the default into the envelope.
fn default_stands_in(kind: InputKind, config: &Config, payload: &Value) -> bool {
    !kind.is_file()
        && config.default.as_ref().is_some_and(|d| !d.is_null())
        && is_absent(&sent_value(kind, config.name.trim(), payload))
}

/// Writes `value` at `body.<name>` (dotted names nest), making `body` an
/// object when the trigger delivered none.
fn write_body_field(payload: &mut Value, name: &str, value: Value) {
    if !payload.is_object() {
        *payload = json!({});
    }
    let map = payload.as_object_mut().expect("object");
    let body = map.entry("body").or_insert_with(|| json!({}));
    if !body.is_object() {
        *body = json!({});
    }
    let mut current = body;
    let segments: Vec<&str> = name.split('.').map(str::trim).filter(|s| !s.is_empty()).collect();
    let Some((last, parents)) = segments.split_last() else {
        return;
    };
    for segment in parents {
        let map = current.as_object_mut().expect("object");
        let next = map.entry(segment.to_string()).or_insert_with(|| json!({}));
        if !next.is_object() {
            *next = json!({});
        }
        current = next;
    }
    current
        .as_object_mut()
        .expect("object")
        .insert(last.to_string(), value);
}

fn check_value(kind: InputKind, config: &Config, name: &str, raw: Value) -> Result<Value, PipelineError> {
    match kind {
        InputKind::Text => {
            let text = match raw {
                Value::String(s) => s,
                other => return Err(invalid(kind, name, format!("got {}", describe(&other)))),
            };
            if let Some(max) = config.max {
                let len = text.chars().count();
                if (len as f64) > max {
                    return Err(invalid(kind, name, format!("at most {max} characters, got {len}")));
                }
            }
            Ok(Value::String(text))
        }
        InputKind::Number => {
            let n = match &raw {
                Value::Number(n) => n.as_f64(),
                Value::String(s) => s.trim().parse::<f64>().ok(),
                _ => None,
            }
            .ok_or_else(|| invalid(kind, name, format!("got {}", describe(&raw))))?;
            if let Some(min) = config.min {
                if n < min {
                    return Err(invalid(kind, name, format!("at least {min}, got {n}")));
                }
            }
            if let Some(max) = config.max {
                if n > max {
                    return Err(invalid(kind, name, format!("at most {max}, got {n}")));
                }
            }
            Ok(serde_json::Number::from_f64(n)
                .map(Value::Number)
                .unwrap_or(raw))
        }
        InputKind::Boolean => {
            let b = match &raw {
                Value::Bool(b) => Some(*b),
                Value::Number(n) => n.as_f64().map(|v| v != 0.0),
                Value::String(s) => match s.trim().to_ascii_lowercase().as_str() {
                    "true" | "1" | "on" | "yes" => Some(true),
                    "false" | "0" | "off" | "no" => Some(false),
                    _ => None,
                },
                _ => None,
            }
            .ok_or_else(|| invalid(kind, name, format!("got {}", describe(&raw))))?;
            Ok(Value::Bool(b))
        }
        InputKind::Json => match raw {
            Value::String(s) => serde_json::from_str::<Value>(&s)
                .map_err(|err| invalid(kind, name, format!("the string does not parse as JSON ({err})"))),
            other => Ok(other),
        },
        InputKind::File | InputKind::Image | InputKind::Audio | InputKind::Video => {
            let accept = config.effective_accept(kind);
            // A repeated field name arrives as a one-element array; one file is one file.
            let file = match raw {
                Value::Array(mut items) if items.len() == 1 => items.remove(0),
                Value::Array(items) => {
                    return Err(invalid(kind, name, format!("got {} files", items.len())));
                }
                other => other,
            };
            check_file(kind, name, &accept, &file)?;
            Ok(file)
        }
        InputKind::Files => {
            let accept = config.effective_accept(kind);
            let items = match raw {
                Value::Array(items) => items,
                single if is_file_ref(&single) => vec![single],
                other => return Err(invalid(kind, name, format!("got {}", describe(&other)))),
            };
            if let Some(max) = config.max {
                if (items.len() as f64) > max {
                    return Err(invalid(kind, name, format!("at most {max} files, got {}", items.len())));
                }
            }
            for item in &items {
                check_file(kind, name, &accept, item)?;
            }
            Ok(Value::Array(items))
        }
    }
}

fn flag(flag: &str, key: &str, description: &str, kind: DslFlagKind) -> DslFlag {
    DslFlag {
        flag: flag.to_string(),
        config_key: key.to_string(),
        description: description.to_string(),
        kind,
        required: false,
    }
}

fn field(name: &str, label: &str, field_type: NodeFieldType, help: &str) -> NodeFieldDef {
    NodeFieldDef {
        name: name.to_string(),
        label: label.to_string(),
        field_type,
        help: Some(help.to_string()),
        ..Default::default()
    }
}

fn example(kind: InputKind) -> NodeExample {
    match kind {
        InputKind::Text => NodeExample::dsl("A caption the Run form asks for", r#"input.text prompt --label "Caption" --max 200"#)
            .input(json!({ "body": { "prompt": "A cat on a mat" } }))
            .output(json!({ "body": { "prompt": "A cat on a mat" } }))
            .note("The payload is unchanged; `$nodes.<id>` is the string."),
        InputKind::Number => NodeExample::dsl("A bounded count", "input.number count --min 1 --max 50 --default 10")
            .input(json!({ "body": {} }))
            .output(json!({ "body": { "count": 10.0 } }))
            .note("Nothing was sent, so the default stands and is written into the envelope: `input.body.count` and `$nodes.<id>` are both 10."),
        InputKind::Boolean => NodeExample::dsl("A switch", "input.boolean dry_run --optional")
            .input(json!({ "body": { "dry_run": "on" } }))
            .output(json!({ "body": { "dry_run": "on" } }))
            .note("A form posts `on`; the node's value is `true`."),
        InputKind::Json => NodeExample::dsl("A JSON document typed into the form", r#"input.json config --label "Settings""#)
            .input(json!({ "body": { "config": "{\"theme\":\"dark\"}" } }))
            .output(json!({ "body": { "config": "{\"theme\":\"dark\"}" } }))
            .note("A JSON string in `body` is parsed; `$nodes.<id>` is the object."),
        InputKind::File => NodeExample::dsl("A spreadsheet or CSV", "input.file sheet --accept csv,xlsx")
            .input(json!({ "body": {}, "files": { "sheet": { "__zf_type": "file_ref", "filename": "q3.xlsx", "mime": "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", "kind": "spreadsheet", "size": 18211 } } }))
            .output(json!({ "body": {}, "files": { "sheet": { "__zf_type": "file_ref", "filename": "q3.xlsx", "mime": "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", "kind": "spreadsheet", "size": 18211 } } }))
            .note("`--accept` takes FileRef kinds, mimes (`image/png`, `image/`) or extensions."),
        InputKind::Files => NodeExample::dsl("Up to five attachments", "input.files attachments --accept pdf,image --max 5 --optional")
            .note("`$nodes.<id>` is the array of FileRefs, `[]` when none were sent."),
        InputKind::Image => NodeExample::dsl("A photo to process", "input.image photo")
            .input(json!({ "body": {}, "files": { "photo": { "__zf_type": "file_ref", "filename": "cat.png", "mime": "image/png", "kind": "image", "size": 4321 } } }))
            .output(json!({ "body": {}, "files": { "photo": { "__zf_type": "file_ref", "filename": "cat.png", "mime": "image/png", "kind": "image", "size": 4321 } } }))
            .note("`--accept` is preset to `image`; `--accept png,webp` narrows it."),
        InputKind::Audio => NodeExample::dsl("A recording", "input.audio clip --optional")
            .note("`--accept` is preset to `audio`."),
        InputKind::Video => NodeExample::dsl("A clip", "input.video clip --accept mp4,webm")
            .note("`--accept` is preset to `video`; here narrowed to two containers."),
    }
}

/// One definition per kind, from one builder: the flags every kind takes,
/// plus the ones that mean something for this value type.
pub fn definition_for(kind: InputKind) -> NodeDefinition {
    let short = kind.short();
    let mut dsl_flags = vec![
        DslFlag {
            flag: "--name".to_string(),
            config_key: "name".to_string(),
            description: format!("The envelope field this node declares — also the first bare token: `{short} <name>`."),
            kind: DslFlagKind::Scalar,
            required: true,
        },
        flag("--label", "label", "Form label shown in the Run form. Default: the field name.", DslFlagKind::Scalar),
        flag("--optional", "optional", "A missing value is allowed; the node's value is then null.", DslFlagKind::Bool),
    ];
    let mut fields = vec![
        field("name", "Field name", NodeFieldType::Text, "The envelope field this node declares: read at body.<name> (or files.<name> for file kinds)."),
        field("label", "Label", NodeFieldType::Text, "Form label shown in the Run form. Defaults to the field name."),
        field("optional", "Optional", NodeFieldType::Checkbox, "A missing value is allowed; the node's value is then null. Required by default."),
    ];
    let mut properties = json!({
        "name": { "type": "string", "description": "The envelope field this node declares." },
        "label": { "type": "string", "description": "Form label; the field name when empty." },
        "optional": { "type": "boolean", "description": "A missing value is allowed." },
    });
    if !kind.is_file() {
        dsl_flags.push(flag("--default", "default", "Value used when the field is missing (text, number, boolean, json).", DslFlagKind::Scalar));
        fields.push(field("default", "Default", NodeFieldType::Text, "Value used when the field is missing. Checked like a sent value."));
        properties["default"] = json!({ "description": "Value used when the field is missing." });
    }
    if kind.is_file() {
        let preset = kind.preset_accept().map(|k| format!(" Preset: `{k}`.")).unwrap_or_default();
        dsl_flags.push(flag("--accept", "accept", &format!("FileRef kinds, mimes or extensions the file must match, comma-separated (`csv,xlsx`, `image/png`, `image/`).{preset}"), DslFlagKind::CommaSeparatedList));
        fields.push(field("accept", "Accept", NodeFieldType::Text, &format!("Comma-separated FileRef kinds, mimes or extensions the file must match.{preset}")));
        properties["accept"] = json!({ "type": "array", "items": { "type": "string" }, "description": "What the file must match." });
    }
    if kind == InputKind::Number {
        dsl_flags.push(flag("--min", "min", "Smallest value accepted.", DslFlagKind::Scalar));
        fields.push(field("min", "Min", NodeFieldType::Number, "Smallest value accepted."));
        properties["min"] = json!({ "type": "number", "description": "Smallest value accepted." });
    }
    let max_help = match kind {
        InputKind::Text => Some("Longest string accepted, in characters."),
        InputKind::Number => Some("Largest value accepted."),
        InputKind::Files => Some("Most files accepted."),
        _ => None,
    };
    if let Some(help) = max_help {
        dsl_flags.push(flag("--max", "max", help, DslFlagKind::Scalar));
        fields.push(field("max", "Max", NodeFieldType::Number, help));
        properties["max"] = json!({ "type": "number", "description": help });
    }
    let value_word = match kind {
        InputKind::Text => "a string",
        InputKind::Number => "a number",
        InputKind::Boolean => "a boolean",
        InputKind::Json => "any JSON value",
        InputKind::Files => "an array of FileRefs",
        _ => "one FileRef",
    };
    let where_ = kind.envelope_slot();
    let description = format!(
        "Declares and checks one field of the trigger envelope: {value_word} at `{where_}.<name>`. \
         Required by default — a missing or invalid value refuses the run with `FW_NODE_INPUT_MISSING` / `FW_NODE_INPUT_INVALID`, \
         naming the field and what was expected; `--optional` allows absence. The payload passes through unchanged, so the next node \
         still reads `input.{where_}.<name>` and `$trigger.{where_}.<name>`; the node's own value, `$nodes.<id>`, is the checked value.{} \
         The input nodes after a trigger are its declaration: the Studio builds the Run form from them and an agent reads them to know \
         what to send. Works after `trigger.manual` and after `trigger.webhook`. Fetches nothing, stores nothing.{}",
        if kind.is_file() {
            ""
        } else {
            " A field that was not sent and has `--default` is written into the envelope at `body.<name>`, so `input.body.<name>` and `$nodes.<id>` agree on every path (DSL/MCP execute, webhook, Run form)."
        },
        match kind {
            InputKind::Json => " A JSON string in `body` is parsed.",
            InputKind::Image | InputKind::Audio | InputKind::Video => " `--accept` is preset to the media kind.",
            _ => "",
        }
    );
    NodeDefinition {
        kind: kind.node_kind().to_string(),
        capabilities: vec![],
        title: kind.title().to_string(),
        description,
        input_schema: json!({
            "type": "object",
            "description": format!("The trigger envelope; this node reads `{where_}.<name>`."),
            "properties": { "body": { "type": ["object", "null"] }, "files": { "type": "object" } }
        }),
        output_schema: json!({
            "type": "object",
            "description": "The same envelope, unchanged except that a missing body field with a `--default` is filled in. `$nodes.<id>` holds the checked value."
        }),
        input_pins: vec![INPUT_PIN_IN.to_string()],
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        script_available: false,
        script_bridge: None,
        config_schema: json!({ "type": "object", "required": ["name"], "properties": properties }),
        dsl_flags,
        fields,
        layout: vec![],
        ai_tool: Default::default(),
        examples: vec![example(kind)],
        positional: Some("name".to_string()),
        ui_category: "input".to_string(),
        ..Default::default()
    }
}

/// The trigger kinds whose tick delivers an empty envelope: no `body`, no
/// `files`. A required input after one of these would refuse every tick.
const EMPTY_ENVELOPE_TRIGGERS: [&str; 1] = ["n.trigger.schedule"];

/// A node kind as the catalogue spells it: `x.n.trigger.schedule` is the same
/// kind as `n.trigger.schedule` for the purpose of what it delivers.
fn canonical_kind(kind: &str) -> &str {
    kind.strip_prefix("x.").unwrap_or(kind)
}

/// Whether this input can be satisfied by an envelope that carries nothing.
fn satisfied_by_an_empty_envelope(kind: InputKind, config: &Config) -> bool {
    if config.optional {
        return true;
    }
    // `--default` stands in for a missing body field; a file has no default.
    !kind.is_file() && config.default.as_ref().is_some_and(|d| !d.is_null())
}

/// Refuses a required input that a trigger with an empty envelope can reach.
///
/// An input node validates whatever the trigger delivered. Webhook, function
/// and WebSocket deliver a caller's envelope, so a missing field is that
/// caller's mistake and `FW_NODE_INPUT_MISSING` at run time is the right
/// answer. A schedule tick delivers an empty envelope, so a required input
/// after it would refuse every tick: that pipeline can never run as
/// scheduled, and activation is where to say so, naming the fix. Each trigger
/// is walked on its own — a manual trigger in the same graph does not rescue
/// an input the schedule also reaches, and an input only the manual trigger
/// reaches is left alone. A schedule pipeline run by hand still receives the
/// real envelope and validates it as always.
pub fn ensure_inputs_reachable_from_empty_triggers(
    graph: &PipelineGraph,
) -> Result<(), PipelineError> {
    for trigger in graph
        .nodes
        .iter()
        .filter(|node| EMPTY_ENVELOPE_TRIGGERS.contains(&canonical_kind(&node.kind)))
    {
        let trigger_kind = canonical_kind(&trigger.kind).trim_start_matches("n.");
        let mut seen: std::collections::HashSet<&str> = std::collections::HashSet::new();
        let mut queue = std::collections::VecDeque::from([trigger.id.as_str()]);
        while let Some(current) = queue.pop_front() {
            if !seen.insert(current) {
                continue;
            }
            for edge in graph.edges.iter().filter(|edge| edge.from_node == current) {
                queue.push_back(edge.to_node.as_str());
            }
            let Some(node) = graph.nodes.iter().find(|node| node.id == current) else {
                continue;
            };
            let Some(kind) = InputKind::from_kind(canonical_kind(&node.kind)) else {
                continue;
            };
            let config = serde_json::from_value::<Config>(node.config.clone())
                .map_err(|err| PipelineError::new("FW_NODE_INPUT_CONFIG", err.to_string()))?;
            if satisfied_by_an_empty_envelope(kind, &config) {
                continue;
            }
            let fix = if kind.is_file() { "--optional" } else { "--default or --optional" };
            return Err(PipelineError::new(
                "FW_NODE_INPUT_UNREACHABLE",
                format!(
                    "input '{}' (node {}) is required but {trigger_kind} delivers no {} — add {fix}",
                    config.name.trim(),
                    node.id,
                    kind.envelope_slot()
                ),
            ));
        }
    }
    Ok(())
}

pub struct Node {
    kind: InputKind,
    config: Config,
}

impl Node {
    pub fn new(kind: InputKind, config: Config) -> Self {
        Self { kind, config }
    }

    /// Builds the handler for any kind in the family; `None` for other kinds.
    pub fn for_kind(kind: &str, config: &Value) -> Option<Result<Self, PipelineError>> {
        let input_kind = InputKind::from_kind(kind)?;
        Some(
            serde_json::from_value::<Config>(config.clone())
                .map(|config| Self::new(input_kind, config))
                .map_err(|err| PipelineError::new("FW_NODE_INPUT_CONFIG", err.to_string())),
        )
    }
}

#[async_trait]
impl NodeHandler for Node {
    fn kind(&self) -> &'static str {
        self.kind.node_kind()
    }
    fn input_pins(&self) -> &'static [&'static str] {
        &[INPUT_PIN_IN]
    }
    fn output_pins(&self) -> &'static [&'static str] {
        &[OUTPUT_PIN_OUT]
    }

    async fn execute_async(
        &self,
        input: NodeExecutionInput,
    ) -> Result<NodeExecutionOutput, PipelineError> {
        let value = check(self.kind, &self.config, &input.payload)?;
        let name = self.config.name.trim();
        let mut trace = format!(
            "node_kind={} field={} value={}",
            self.kind.node_kind(),
            name,
            describe(&value)
        );
        // Pass-through, like `logic.if`: what arrived is what leaves — except
        // a default standing in for a field nobody sent, which is written
        // into the envelope so `input.body.<name>` and `$nodes.<id>` agree
        // whether the run came from the Run form, the DSL, MCP or a webhook.
        let mut payload = input.payload;
        if default_stands_in(self.kind, &self.config, &payload) {
            write_body_field(&mut payload, name, value.clone());
            trace.push_str(" default=filled");
        }
        Ok(NodeExecutionOutput {
            output_pins: vec![OUTPUT_PIN_OUT.to_string()],
            payload,
            trace: vec![trace],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(name: &str) -> Config {
        Config { name: name.to_string(), ..Default::default() }
    }

    fn file_ref(filename: &str, mime: &str, kind: &str) -> Value {
        json!({
            "__zf_type": "file_ref", "backend": "zebfs", "ref": format!("tmp/runs/r/files/{filename}"),
            "filename": filename, "mime": mime, "kind": kind, "size": 12,
            "sha256": "sha256:e3b0c44298fc1c14a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718",
            "lifecycle": "temporary", "origin": "manual", "trust": "user"
        })
    }

    #[test]
    fn a_required_field_that_was_not_sent_is_missing_and_names_the_field() {
        let err = check(InputKind::Text, &cfg("prompt"), &json!({ "body": {} })).unwrap_err();
        assert_eq!(err.code, "FW_NODE_INPUT_MISSING");
        assert!(err.message.contains("'prompt'") && err.message.contains("body.prompt"), "{}", err.message);
        // A blank form field is nothing sent.
        let err = check(InputKind::Text, &cfg("prompt"), &json!({ "body": { "prompt": "  " } })).unwrap_err();
        assert_eq!(err.code, "FW_NODE_INPUT_MISSING");
    }

    #[test]
    fn optional_and_default_stand_in_for_an_absent_value() {
        let optional = Config { optional: true, ..cfg("note") };
        assert_eq!(check(InputKind::Text, &optional, &json!({})).unwrap(), Value::Null);
        let with_default = Config { default: Some(json!(10)), ..cfg("count") };
        assert_eq!(check(InputKind::Number, &with_default, &json!({ "body": {} })).unwrap(), json!(10.0));
    }

    #[test]
    fn text_number_boolean_and_json_coerce_what_a_form_posts() {
        let long = Config { max: Some(3.0), ..cfg("t") };
        assert_eq!(check(InputKind::Text, &long, &json!({ "body": { "t": "abc" } })).unwrap(), json!("abc"));
        let err = check(InputKind::Text, &long, &json!({ "body": { "t": "abcd" } })).unwrap_err();
        assert_eq!(err.code, "FW_NODE_INPUT_INVALID");

        let bounded = Config { min: Some(1.0), max: Some(5.0), ..cfg("n") };
        assert_eq!(check(InputKind::Number, &bounded, &json!({ "body": { "n": "4" } })).unwrap(), json!(4.0));
        assert_eq!(check(InputKind::Number, &bounded, &json!({ "body": { "n": 9 } })).unwrap_err().code, "FW_NODE_INPUT_INVALID");
        assert_eq!(check(InputKind::Number, &bounded, &json!({ "body": { "n": "four" } })).unwrap_err().code, "FW_NODE_INPUT_INVALID");

        assert_eq!(check(InputKind::Boolean, &cfg("b"), &json!({ "body": { "b": "on" } })).unwrap(), json!(true));
        assert_eq!(check(InputKind::Boolean, &cfg("b"), &json!({ "body": { "b": false } })).unwrap(), json!(false));
        assert_eq!(check(InputKind::Boolean, &cfg("b"), &json!({ "body": { "b": "maybe" } })).unwrap_err().code, "FW_NODE_INPUT_INVALID");

        assert_eq!(check(InputKind::Json, &cfg("j"), &json!({ "body": { "j": "{\"a\":1}" } })).unwrap(), json!({ "a": 1 }));
        assert_eq!(check(InputKind::Json, &cfg("j"), &json!({ "body": { "j": { "a": 1 } } })).unwrap(), json!({ "a": 1 }));
        assert_eq!(check(InputKind::Json, &cfg("j"), &json!({ "body": { "j": "{nope" } })).unwrap_err().code, "FW_NODE_INPUT_INVALID");
    }

    #[test]
    fn a_file_is_read_from_files_and_checked_against_accept() {
        let photo = file_ref("cat.png", "image/png", "image");
        let payload = json!({ "body": {}, "files": { "photo": photo.clone() } });
        assert_eq!(check(InputKind::Image, &cfg("photo"), &payload).unwrap(), photo);
        // The media preset refuses a PDF where an image was declared.
        let pdf = json!({ "files": { "photo": file_ref("cv.pdf", "application/pdf", "pdf") } });
        let err = check(InputKind::Image, &cfg("photo"), &pdf).unwrap_err();
        assert_eq!(err.code, "FW_NODE_INPUT_INVALID");
        assert!(err.message.contains("accepts image"), "{}", err.message);
        // `--accept` by extension and by mime family.
        let sheet = Config { accept: vec!["csv".into(), "xlsx".into()], ..cfg("sheet") };
        let xlsx = file_ref("q3.xlsx", "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet", "spreadsheet");
        assert!(check(InputKind::File, &sheet, &json!({ "files": { "sheet": xlsx } })).is_ok());
        let family = Config { accept: vec!["image/".into()], ..cfg("f") };
        assert!(check(InputKind::File, &family, &json!({ "files": { "f": file_ref("a.webp", "image/webp", "image") } })).is_ok());
        // A file field that is not a file at all.
        let err = check(InputKind::File, &cfg("f"), &json!({ "files": { "f": "uploads/x.png" } })).unwrap_err();
        assert_eq!(err.code, "FW_NODE_INPUT_INVALID");
    }

    #[test]
    fn files_takes_a_list_bounded_by_max() {
        let two = json!({ "files": { "docs": [file_ref("a.pdf", "application/pdf", "pdf"), file_ref("b.pdf", "application/pdf", "pdf")] } });
        let capped = Config { max: Some(1.0), ..cfg("docs") };
        assert_eq!(check(InputKind::Files, &capped, &two).unwrap_err().code, "FW_NODE_INPUT_INVALID");
        let value = check(InputKind::Files, &cfg("docs"), &two).unwrap();
        assert_eq!(value.as_array().map(Vec::len), Some(2));
        // One file where a list was declared is a list of one.
        let one = json!({ "files": { "docs": file_ref("a.pdf", "application/pdf", "pdf") } });
        assert_eq!(check(InputKind::Files, &cfg("docs"), &one).unwrap().as_array().map(Vec::len), Some(1));
        // Nothing sent, required.
        assert_eq!(check(InputKind::Files, &cfg("docs"), &json!({ "files": {} })).unwrap_err().code, "FW_NODE_INPUT_MISSING");
    }

    #[tokio::test]
    async fn the_node_passes_the_envelope_through_unchanged() {
        let payload = json!({ "body": { "prompt": "hi", "other": 1 }, "files": {} });
        let node = Node::new(InputKind::Text, cfg("prompt"));
        let out = node
            .execute_async(NodeExecutionInput {
                node_id: "n1".into(),
                input_pin: "in".into(),
                payload: payload.clone(),
                metadata: json!({}),
                bus: None,
            })
            .await
            .unwrap();
        assert_eq!(out.payload, payload);
        assert_eq!(scope_value(text::NODE_KIND, &json!({ "name": "prompt" }), &payload), Some(json!("hi")));
        assert_eq!(scope_value("n.script", &json!({}), &payload), None);
    }

    /// A default for a field nobody sent is written into the envelope, so the
    /// next node's `input.body.<name>` and `$nodes.<id>` agree; a sent value
    /// wins and the envelope is untouched.
    #[tokio::test]
    async fn a_default_fills_the_missing_field_in_the_envelope_and_a_sent_value_wins() {
        let node = Node::new(InputKind::Text, Config { default: Some(json!("world")), optional: true, ..cfg("who") });
        let run = |payload: Value| {
            let node = &node;
            async move {
                node.execute_async(NodeExecutionInput {
                    node_id: "n1".into(),
                    input_pin: "in".into(),
                    payload,
                    metadata: json!({}),
                    bus: None,
                })
                .await
                .unwrap()
            }
        };
        // Omitted: filled, and `scope_value` on the passed payload agrees.
        let out = run(json!({ "body": {} })).await;
        assert_eq!(out.payload, json!({ "body": { "who": "world" } }));
        assert_eq!(scope_value(text::NODE_KIND, &json!({ "name": "who", "default": "world" }), &out.payload), Some(json!("world")));
        assert!(out.trace[0].ends_with("default=filled"), "{}", out.trace[0]);
        // No body at all (a schedule tick): body is made.
        let out = run(json!({})).await;
        assert_eq!(out.payload, json!({ "body": { "who": "world" } }));
        // Sent: the given value wins, nothing else changes.
        let out = run(json!({ "body": { "who": "x", "other": 1 }, "files": {} })).await;
        assert_eq!(out.payload, json!({ "body": { "who": "x", "other": 1 }, "files": {} }));
        assert!(!out.trace[0].contains("default=filled"));
        // A number default is written as the checked number, a dotted name nests.
        let node = Node::new(InputKind::Number, Config { default: Some(json!("10")), ..cfg("page.size") });
        let out = node
            .execute_async(NodeExecutionInput { node_id: "n2".into(), input_pin: "in".into(), payload: json!({ "body": { "page": {} } }), metadata: json!({}), bus: None })
            .await
            .unwrap();
        assert_eq!(out.payload, json!({ "body": { "page": { "size": 10.0 } } }));
    }

    /// An optional field without a default stays absent — nothing is invented.
    #[tokio::test]
    async fn an_optional_field_without_a_default_leaves_the_envelope_alone() {
        let node = Node::new(InputKind::Text, Config { optional: true, ..cfg("who") });
        let out = node
            .execute_async(NodeExecutionInput { node_id: "n1".into(), input_pin: "in".into(), payload: json!({ "body": {} }), metadata: json!({}), bus: None })
            .await
            .unwrap();
        assert_eq!(out.payload, json!({ "body": {} }));
    }

    #[test]
    fn every_kind_has_a_definition_with_the_positional_name() {
        for kind in InputKind::ALL {
            let def = definition_for(kind);
            assert_eq!(def.positional.as_deref(), Some("name"), "{}", def.kind);
            assert_eq!(def.ui_category, "input");
            assert!(def.dsl_flags.iter().any(|f| f.flag == "--name"));
            assert_eq!(def.dsl_flags.iter().any(|f| f.flag == "--accept"), kind.is_file(), "{}", def.kind);
            assert_eq!(def.dsl_flags.iter().any(|f| f.flag == "--default"), !kind.is_file(), "{}", def.kind);
            assert_eq!(InputKind::from_kind(&def.kind), Some(kind));
        }
    }

    #[test]
    fn accept_and_bounds_read_the_dialog_spelling_too() {
        let config: Config = serde_json::from_value(json!({ "name": "f", "accept": "CSV, xlsx", "max": "3" })).unwrap();
        assert_eq!(config.accept, vec!["csv", "xlsx"]);
        assert_eq!(config.max, Some(3.0));
        let config: Config = serde_json::from_value(json!({ "name": "f", "accept": ["pdf"], "max": 2 })).unwrap();
        assert_eq!(config.accept, vec!["pdf"]);
        assert_eq!(config.max, Some(2.0));
    }

    fn graph(body: &str) -> PipelineGraph {
        crate::platform::shell::parser::build_pipeline_graph("schedule-inputs", body).expect("graph")
    }

    #[test]
    fn a_required_input_after_a_schedule_is_refused_naming_the_fix() {
        let err = ensure_inputs_reachable_from_empty_triggers(&graph(
            "[a] trigger.schedule --cron \"0 * * * *\"\n[b] input.text prompt\n[a] -> [b]\n",
        ))
        .unwrap_err();
        assert_eq!(err.code, "FW_NODE_INPUT_UNREACHABLE");
        assert_eq!(
            err.message,
            "input 'prompt' (node b) is required but trigger.schedule delivers no body — add --default or --optional"
        );
        // Two hops away is still reachable; a file has no default to add.
        let err = ensure_inputs_reachable_from_empty_triggers(&graph(
            "[a] trigger.schedule --cron \"0 * * * *\"\n[b] input.text note --optional\n[c] input.image photo\n[a] -> [b]\n[b] -> [c]\n",
        ))
        .unwrap_err();
        assert_eq!(err.code, "FW_NODE_INPUT_UNREACHABLE");
        assert_eq!(
            err.message,
            "input 'photo' (node c) is required but trigger.schedule delivers no files — add --optional"
        );
    }

    #[test]
    fn a_default_or_optional_input_after_a_schedule_passes() {
        ensure_inputs_reachable_from_empty_triggers(&graph(
            "[a] trigger.schedule --cron \"0 * * * *\"\n[b] input.text prompt --default hi\n[a] -> [b]\n",
        ))
        .expect("--default satisfies the tick");
        ensure_inputs_reachable_from_empty_triggers(&graph(
            "[a] trigger.schedule --cron \"0 * * * *\"\n[b] input.text prompt --optional\n[a] -> [b]\n",
        ))
        .expect("--optional satisfies the tick");
        ensure_inputs_reachable_from_empty_triggers(&graph(
            "[a] trigger.schedule --cron \"0 * * * *\"\n[b] input.file doc --optional\n[a] -> [b]\n",
        ))
        .expect("an optional file satisfies the tick");
    }

    #[test]
    fn each_trigger_is_checked_on_its_own() {
        // A manual trigger feeding the same input does not rescue it: the
        // schedule still reaches it with nothing.
        let err = ensure_inputs_reachable_from_empty_triggers(&graph(
            "[a] trigger.schedule --cron \"0 * * * *\"\n[m] trigger.manual\n[b] input.text prompt\n[a] -> [b]\n[m] -> [b]\n",
        ))
        .unwrap_err();
        assert_eq!(err.code, "FW_NODE_INPUT_UNREACHABLE");
        // An input only the manual trigger reaches is that trigger's business.
        ensure_inputs_reachable_from_empty_triggers(&graph(
            "[a] trigger.schedule --cron \"0 * * * *\"\n[m] trigger.manual\n[b] input.text prompt\n[s] script -- \"return input;\"\n[a] -> [s]\n[m] -> [b]\n",
        ))
        .expect("the schedule reaches no input");
        // Webhook, function and manual triggers deliver a caller's envelope.
        ensure_inputs_reachable_from_empty_triggers(&graph(
            "[w] trigger.webhook --path /x --method POST\n[b] input.text prompt\n[w] -> [b]\n",
        ))
        .expect("a webhook's missing field is the run's refusal, not activation's");
    }
}
