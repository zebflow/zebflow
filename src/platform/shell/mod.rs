//! Pipeline DSL shell — text command language for pipeline management.
//!
//! Accessible from 4 channels: REST API, CLI panel, MCP tool, and chat assistant.
//! Same DSL string in, same line-output response out.

pub mod executor;
pub mod parser;

use serde::{Deserialize, Serialize};

/// HTTP request body for the DSL endpoint.
#[derive(Debug, Clone, Deserialize)]
pub struct DslRequest {
    /// DSL command string (supports && chaining and \ continuations).
    pub dsl: String,
}

/// One terminal output line.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DslLine {
    /// Display text.
    pub text: String,
    /// CSS class: `cli-success`, `cli-muted`, `cli-error`, `cli-info`, `cli-blank`.
    pub cls: String,
}

impl DslLine {
    pub fn success(text: impl Into<String>) -> Self {
        Self { text: text.into(), cls: "cli-success".to_string() }
    }

    pub fn muted(text: impl Into<String>) -> Self {
        Self { text: text.into(), cls: "cli-muted".to_string() }
    }

    pub fn error(text: impl Into<String>) -> Self {
        Self { text: text.into(), cls: "cli-error".to_string() }
    }

    pub fn info(text: impl Into<String>) -> Self {
        Self { text: text.into(), cls: "cli-info".to_string() }
    }

    pub fn blank() -> Self {
        Self { text: String::new(), cls: "cli-blank".to_string() }
    }
}

/// Collected output from a DSL execution run.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DslOutput {
    pub ok: bool,
    pub lines: Vec<DslLine>,
}

impl DslOutput {
    pub fn new_ok() -> Self {
        Self { ok: true, lines: Vec::new() }
    }

    pub fn new_err(msg: impl Into<String>) -> Self {
        Self { ok: false, lines: vec![DslLine::error(msg)] }
    }

    pub fn push(&mut self, line: DslLine) {
        self.lines.push(line);
    }

    pub fn extend(&mut self, lines: impl IntoIterator<Item = DslLine>) {
        self.lines.extend(lines);
    }

    /// Convenience: create an error output from a message string.
    pub fn err(msg: impl Into<String>) -> Self {
        Self::new_err(msg)
    }
}
