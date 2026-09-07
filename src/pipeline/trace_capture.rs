//! Bounded invocation-log capture, independent of execution payload ownership.
//!
//! A borrowed tree is serialized through a sampling/redaction view into a capped
//! buffer. No complete payload clone is made. Secret markers are discovered in
//! the **whole** tree first, including omitted array elements: a secret declared
//! there may also occur in a retained value. That scan is linear in input size;
//! sampling reduces copying/serialization, not the cost of discovering secrets.
//!
//! Byte budgets cover captured config/input/output JSON across a node or run.
//! Fixed omission markers and trace bookkeeping (IDs, timings, error messages)
//! are outside these budgets. They are not a cap on the entire database record.
//! Exhausting a budget replaces that capture with a typed marker and consumes
//! the attempted allowance. Execution values are never truncated or modified.

use std::collections::HashSet;
use std::io::{self, Write};

use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Deserialize, Serialize, Serializer};
use serde_json::{Value, json};

use crate::pipeline::nodes::NodeExecutionOutput;

#[cfg(test)]
mod tests;

/// Optional capture overrides. Missing fields inherit the project setting,
/// then the built-in default. Retention count/age are configured separately.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TraceCaptureSettings {
    /// First N elements of every logged array; 0 disables array sampling only.
    /// Default: 6. Strings, depth, and byte budgets still apply with 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub array_sample_count: Option<u32>,
    /// Maximum Unicode characters retained per string. Default: 8192.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_string_chars: Option<u32>,
    /// Maximum container depth, with the payload root at depth 0. Default: 8.
    /// An additional encoded-depth ceiling of 112 accounts for sample wrappers
    /// and reserves room for trace/API envelopes under JSON's parser limit.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_depth: Option<u32>,
    /// Captured config/input/output bytes per node execution. Default: 65536.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_node_bytes: Option<u32>,
    /// Captured data bytes per invocation. Child function runs currently have
    /// independent budgets. Default: 1048576. Does not change log retention.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_run_bytes: Option<u32>,
}

/// Fully resolved capture policy, snapshotted once at invocation start.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct TraceCaptureLimits {
    /// See [`TraceCaptureSettings::array_sample_count`].
    pub array_sample_count: u32,
    /// See [`TraceCaptureSettings::max_string_chars`].
    pub max_string_chars: u32,
    /// See [`TraceCaptureSettings::max_depth`].
    pub max_depth: u32,
    /// See [`TraceCaptureSettings::max_node_bytes`].
    pub max_node_bytes: u32,
    /// See [`TraceCaptureSettings::max_run_bytes`].
    pub max_run_bytes: u32,
}

impl TraceCaptureSettings {
    /// Validate configured limits at API/contract boundaries. Only array
    /// sampling accepts zero; finite limits prevent accidental unlimited logs.
    pub fn validate(&self) -> Result<(), String> {
        for (name, value, min, max) in [
            ("array_sample_count", self.array_sample_count, 0, 1_000_000),
            ("max_string_chars", self.max_string_chars, 1, 1_048_576),
            ("max_depth", self.max_depth, 1, 64),
            ("max_node_bytes", self.max_node_bytes, 256, 16_777_216),
            ("max_run_bytes", self.max_run_bytes, 256, 67_108_864),
        ] {
            if value.is_some_and(|value| value < min || value > max) {
                return Err(format!(
                    "trace_capture.{name} must be between {min} and {max}"
                ));
            }
        }
        Ok(())
    }

    /// Resolve each field independently: pipeline override → project → default.
    /// In particular, `Some(0)` is a real array override, never inheritance.
    pub fn resolve(&self, overrides: Option<&Self>) -> TraceCaptureLimits {
        let empty = Self::default();
        let over = overrides.unwrap_or(&empty);
        TraceCaptureLimits {
            array_sample_count: over
                .array_sample_count
                .or(self.array_sample_count)
                .unwrap_or(6),
            max_string_chars: over
                .max_string_chars
                .or(self.max_string_chars)
                .unwrap_or(8192),
            max_depth: over.max_depth.or(self.max_depth).unwrap_or(8),
            max_node_bytes: over.max_node_bytes.or(self.max_node_bytes).unwrap_or(65536),
            max_run_bytes: over.max_run_bytes.or(self.max_run_bytes).unwrap_or(1048576),
        }
    }
}

/// Per-invocation budget. Start a fresh node allowance with [`Self::begin_node`].
pub(crate) struct TraceCapture {
    limits: TraceCaptureLimits,
    run_left: usize,
    node_left: usize,
}

impl TraceCapture {
    pub(crate) fn new(limits: TraceCaptureLimits) -> Self {
        Self {
            run_left: limits.max_run_bytes as usize,
            node_left: 0,
            limits,
        }
    }

    pub(crate) fn begin_node(&mut self) {
        self.node_left = self.limits.max_node_bytes as usize;
    }

    /// Capture one borrowed payload, preserving private-marker and exception
    /// semantics. Serialization stops immediately when the byte buffer is full.
    pub(crate) fn capture(&mut self, value: &Value) -> Value {
        self.capture_mode(value, false)
    }

    /// Capture resolved config using the existing config secret-key policy;
    /// editor-only `ui` properties are omitted at every level.
    pub(crate) fn config(&mut self, value: &Value) -> Option<Value> {
        if value.is_null() || value.as_object().is_some_and(|v| v.is_empty()) {
            return None;
        }
        Some(self.capture_mode(value, true))
    }

    fn capture_mode(&mut self, value: &Value, config: bool) -> Value {
        if self.node_left.min(self.run_left) == 0 {
            return byte_marker();
        }
        let secrets = Secrets::discover(value);
        let limits = self.limits;
        self.encode(&View {
            value,
            secrets: &secrets,
            limits: &limits,
            depth: 0,
            wire_depth: 0,
            path: Vec::new(),
            masked: false,
            excepted: false,
            config,
        })
    }

    /// Treat multiple node emissions as one logged array. Avoid constructing a
    /// second owned array containing all execution payloads just to sample it.
    pub(crate) fn outputs(&mut self, outputs: &[NodeExecutionOutput]) -> Value {
        if outputs.len() == 1 {
            return self.capture(&outputs[0].payload);
        }
        if self.node_left.min(self.run_left) == 0 {
            return byte_marker();
        }
        let limits = self.limits;
        self.encode(&Emissions { outputs, limits })
    }

    fn encode(&mut self, value: &impl Serialize) -> Value {
        let allowance = self.node_left.min(self.run_left);
        let mut buffer = CappedBuffer {
            bytes: Vec::new(),
            limit: allowance,
        };
        let result = serde_json::to_writer(&mut buffer, value);
        let charged = if result.is_ok() {
            buffer.bytes.len()
        } else {
            allowance
        };
        self.node_left -= charged;
        self.run_left -= charged;
        if result.is_err() {
            return byte_marker();
        }
        serde_json::from_slice(&buffer.bytes).unwrap_or_else(|_| byte_marker())
    }
}

fn byte_marker() -> Value {
    json!({"__zf_trace_summary": "bytes", "reason": "capture budget exhausted"})
}

struct CappedBuffer {
    bytes: Vec<u8>,
    limit: usize,
}

impl Write for CappedBuffer {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        if bytes.len() > self.limit.saturating_sub(self.bytes.len()) {
            return Err(io::Error::other("trace capture byte budget"));
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn private_key(key: &str) -> bool {
    matches!(
        key,
        "__zf_private_redact" | "__zf_private_trace_redact" | "__zf_private_redact_except_paths"
    )
}

/// Borrowed traversal frames avoid allocating an iterator box for every row.
enum ScanFrame<'a> {
    One(Option<&'a Value>),
    Array(std::slice::Iter<'a, Value>),
    Object(serde_json::map::Iter<'a>),
}
impl<'a> ScanFrame<'a> {
    fn next(&mut self) -> Option<&'a Value> {
        match self {
            Self::One(item) => item.take(),
            Self::Array(items) => items.next(),
            Self::Object(items) => items
                .find(|(key, _)| !private_key(key))
                .map(|(_, item)| item),
        }
    }
}

struct Secrets<'a> {
    tokens: Vec<&'a str>,
    exceptions: Vec<Vec<&'a str>>,
}

impl<'a> Secrets<'a> {
    fn discover(value: &'a Value) -> Self {
        let mut result = Self {
            tokens: Vec::new(),
            exceptions: Vec::new(),
        };
        if let Some(paths) = value
            .get("__zf_private_redact_except_paths")
            .and_then(Value::as_array)
        {
            for path in paths {
                let parts: Vec<_> = match path {
                    Value::String(path) => path
                        .split('.')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .collect(),
                    Value::Array(path) => path
                        .iter()
                        .filter_map(Value::as_str)
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .collect(),
                    _ => Vec::new(),
                };
                if !parts.is_empty() {
                    result.exceptions.push(parts);
                }
            }
        }
        let mut seen = HashSet::new();
        // Iterator stack is O(depth), not O(array length); omitted data is
        // scanned without allocating a parallel list of references to it.
        let mut stack = vec![ScanFrame::One(Some(value))];
        while let Some(iter) = stack.last_mut() {
            let Some(item) = iter.next() else {
                stack.pop();
                continue;
            };
            match item {
                Value::Object(map) => {
                    for key in ["__zf_private_trace_redact", "__zf_private_redact"] {
                        if let Some(items) = map.get(key).and_then(Value::as_array) {
                            for token in items
                                .iter()
                                .filter_map(Value::as_str)
                                .map(str::trim)
                                .filter(|s| !s.is_empty())
                            {
                                if seen.insert(token) {
                                    result.tokens.push(token);
                                }
                            }
                        }
                    }
                    stack.push(ScanFrame::Object(map.iter()));
                }
                Value::Array(items) => stack.push(ScanFrame::Array(items.iter())),
                _ => {}
            }
        }
        result
    }
}

struct View<'a> {
    value: &'a Value,
    secrets: &'a Secrets<'a>,
    limits: &'a TraceCaptureLimits,
    depth: u32,
    wire_depth: u32,
    path: Vec<&'a str>,
    masked: bool,
    excepted: bool,
    config: bool,
}

impl View<'_> {
    fn child<'a>(&'a self, value: &'a Value, key: Option<&'a str>) -> View<'a> {
        let mut path = self.path.clone();
        if let Some(key) = key {
            path.push(key);
        }
        let excepted = self.excepted || (!self.config && self.secrets.exceptions.contains(&path));
        let sensitive = key.is_some_and(|key| {
            if self.config {
                crate::pipeline::engines::basic::is_sensitive_trace_config_key(key)
            } else {
                crate::pipeline::nodes::basic::trigger::webhook::is_sensitive_payload_key(key)
            }
        });
        View {
            value,
            secrets: self.secrets,
            limits: self.limits,
            depth: self.depth + 1,
            // A sampled array adds a summary object around its preview array.
            wire_depth: self.wire_depth
                + if self
                    .value
                    .as_array()
                    .is_some_and(|items| sample_count(items.len(), self.limits) < items.len())
                {
                    2
                } else {
                    1
                },
            path,
            masked: self.masked || (!excepted && sensitive),
            excepted,
            config: self.config,
        }
    }
}

impl Serialize for View<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        if self.masked
            && (self.config
                || !matches!(
                    self.value,
                    Value::Null | Value::Bool(_) | Value::Array(_) | Value::Object(_)
                ))
        {
            return serializer.serialize_str("••••••");
        }
        match self.value {
            Value::Array(items) => {
                if self.depth >= self.limits.max_depth || self.wire_depth >= 112 {
                    return json!({"__zf_trace_summary":"array","len":items.len(),"reason":"depth"}).serialize(serializer);
                }
                let count = sample_count(items.len(), self.limits);
                if count < items.len() {
                    let mut map = serializer.serialize_map(Some(4))?;
                    map.serialize_entry("__zf_trace_summary", "array")?;
                    map.serialize_entry("len", &items.len())?;
                    map.serialize_entry("omitted", &(items.len() - count))?;
                    map.serialize_entry(
                        "preview",
                        &Preview {
                            parent: self,
                            items: &items[..count],
                        },
                    )?;
                    map.end()
                } else {
                    Preview {
                        parent: self,
                        items,
                    }
                    .serialize(serializer)
                }
            }
            Value::Object(map) => {
                if self.depth >= self.limits.max_depth || self.wire_depth >= 112 {
                    return json!({"__zf_trace_summary":"object","keys":map.len(),"reason":"depth"}).serialize(serializer);
                }
                let mut out = serializer.serialize_map(None)?;
                for (key, value) in map {
                    if private_key(key) || (self.config && key == "ui") {
                        continue;
                    }
                    out.serialize_entry(key, &self.child(value, Some(key)))?;
                }
                out.end()
            }
            Value::String(text) => {
                let chars = text.chars().count();
                let cap = self.limits.max_string_chars as usize;
                let (preview, truncated) = redacted_preview(
                    text,
                    if self.excepted {
                        &[]
                    } else {
                        &self.secrets.tokens
                    },
                    cap,
                );
                if truncated {
                    json!({"__zf_trace_summary":"string","chars":chars,"preview":preview})
                        .serialize(serializer)
                } else {
                    serializer.serialize_str(&preview)
                }
            }
            value => value.serialize(serializer),
        }
    }
}

/// Walk the original string so truncation can never expose the prefix of a
/// secret cut by the preview boundary. Retain at most `cap` output characters;
/// token matches consume borrowed input without copying the token or its tail.
fn redacted_preview(text: &str, tokens: &[&str], cap: usize) -> (String, bool) {
    if tokens.is_empty() {
        let end = text
            .char_indices()
            .nth(cap)
            .map(|(i, _)| i)
            .unwrap_or(text.len());
        return (text[..end].to_owned(), end < text.len());
    }
    let mut output = String::with_capacity(text.len().min(cap));
    let mut rest = text;
    let mut written = 0;
    while !rest.is_empty() && written < cap {
        if let Some(token) = tokens
            .iter()
            .filter(|token| rest.starts_with(**token))
            .max_by_key(|token| token.len())
        {
            rest = &rest[token.len()..];
            let mask_chars = (cap - written).min(6);
            for _ in 0..mask_chars {
                output.push('•');
            }
            written += mask_chars;
        } else {
            let ch = rest.chars().next().expect("nonempty string");
            output.push(ch);
            written += 1;
            rest = &rest[ch.len_utf8()..];
        }
    }
    (output, !rest.is_empty())
}

fn sample_count(len: usize, limits: &TraceCaptureLimits) -> usize {
    if limits.array_sample_count == 0 {
        len
    } else {
        len.min(limits.array_sample_count as usize)
    }
}

struct Preview<'a> {
    parent: &'a View<'a>,
    items: &'a [Value],
}
impl Serialize for Preview<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.items.len()))?;
        for item in self.items {
            seq.serialize_element(&self.parent.child(item, None))?;
        }
        seq.end()
    }
}

struct Emissions<'a> {
    outputs: &'a [NodeExecutionOutput],
    limits: TraceCaptureLimits,
}
impl Serialize for Emissions<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("count", &self.outputs.len())?;
        map.serialize_entry("emissions", &EmissionArray(self))?;
        map.end()
    }
}

struct EmissionArray<'a>(&'a Emissions<'a>);
impl Serialize for EmissionArray<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let count = sample_count(self.0.outputs.len(), &self.0.limits);
        if count == self.0.outputs.len() {
            return EmissionItems(self.0, count).serialize(serializer);
        }
        let mut map = serializer.serialize_map(Some(4))?;
        map.serialize_entry("__zf_trace_summary", "array")?;
        map.serialize_entry("len", &self.0.outputs.len())?;
        map.serialize_entry("omitted", &(self.0.outputs.len() - count))?;
        map.serialize_entry("preview", &EmissionItems(self.0, count))?;
        map.end()
    }
}

struct EmissionItems<'a>(&'a Emissions<'a>, usize);
impl Serialize for EmissionItems<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut seq = serializer.serialize_seq(Some(self.1))?;
        for out in &self.0.outputs[..self.1] {
            let secrets = Secrets::discover(&out.payload);
            seq.serialize_element(&View {
                value: &out.payload,
                secrets: &secrets,
                limits: &self.0.limits,
                depth: 0,
                wire_depth: 0,
                path: Vec::new(),
                masked: false,
                excepted: false,
                config: false,
            })?;
        }
        seq.end()
    }
}
