//! The 0.10 → 0.11 table, one rule per old kind: what it becomes, what it
//! did to the payload in 0.10 (`v0.10.12`, the release the offices run), and
//! how its config is rewritten.
//!
//! Sources: each node's definition and handler at `v0.10.12` and at HEAD,
//! and the 0.11 phase commits (`git log 79c7149..HEAD`). Where the commits
//! after the tag (`79c7149`) renamed a key, both spellings are read; where
//! they changed what a node answered, the tag's behaviour is the one
//! modelled, because the tag is what ran.

mod composite;
mod data;
mod fs;
pub mod known;
mod logic;
mod messaging;
mod trigger;
mod web;

use serde_json::{Map, Value};

use super::RewriteContext;
use super::graph::OldOutput;
use super::node::NodeRewrite;

pub use trigger::function_result;

pub type Config = Map<String, Value>;

/// What an old kind becomes.
#[derive(Clone, Copy)]
pub enum NewKind {
    Fixed(&'static str),
    /// Decided by the old config (`n.crypto --op`, `n.script --lang`).
    By(fn(&Config) -> String),
    /// The old kind without its `n.` prefix (`n.input.text`).
    Unprefixed,
}

/// One old kind's rule.
#[derive(Clone, Copy)]
pub struct Rule {
    pub new_kind: NewKind,
    /// What the node did to the payload in 0.10.
    pub output: fn(&Config, &RewriteContext) -> OldOutput,
    /// What its `error` pin carried in 0.10.
    pub error_output: fn(&Config) -> OldOutput,
    /// 0.10 delivered its own failure on `error` and, with that pin
    /// unwired, carried on as if nothing failed; 0.11 fails the node.
    pub self_routed: bool,
    /// Config keys holding code or a bare expression (no `{{ }}`).
    pub code_keys: &'static [(&'static str, Code)],
    /// The config rewrite.
    pub rewrite: fn(&mut NodeRewrite<'_>),
}

impl Rule {
    pub fn new(
        new_kind: &'static str,
        output: fn(&Config, &RewriteContext) -> OldOutput,
        rewrite: fn(&mut NodeRewrite<'_>),
    ) -> Self {
        Rule { new_kind: NewKind::Fixed(new_kind), output, error_output: engine_error, self_routed: false, code_keys: &[], rewrite }
    }

    pub fn by(
        new_kind: fn(&Config) -> String,
        output: fn(&Config, &RewriteContext) -> OldOutput,
        rewrite: fn(&mut NodeRewrite<'_>),
    ) -> Self {
        Rule { new_kind: NewKind::By(new_kind), output, error_output: engine_error, self_routed: false, code_keys: &[], rewrite }
    }

    pub fn code(mut self, keys: &'static [(&'static str, Code)]) -> Self {
        self.code_keys = keys;
        self
    }

    pub fn errors(mut self, error_output: fn(&Config) -> OldOutput) -> Self {
        self.error_output = error_output;
        self.self_routed = true;
        self
    }

    pub fn kind_for(&self, old_kind: &str, config: &Config) -> String {
        match self.new_kind {
            NewKind::Fixed(kind) => kind.to_string(),
            NewKind::By(f) => f(config),
            NewKind::Unprefixed => old_kind.trim_start_matches("n.").to_string(),
        }
    }

    pub fn unprefixed(mut self) -> Self {
        self.new_kind = NewKind::Unprefixed;
        self
    }
}

/// How a config key holding code is read.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Code {
    /// A JavaScript expression without `{{ }}` (`logic.if --expr`).
    Expression,
    /// A map whose values are such expressions (`n.http.request --bind`).
    ExpressionMap,
    /// An expression whose `$input` changed meaning in 0.11 (`logic.reduce
    /// --init-expr`): a payload reference in it is reported.
    ChangedScope,
    /// A script body (`n.script`).
    Body,
    /// Code that runs elsewhere (a browser script): never rewritten; a
    /// payload reference in it is reported.
    Foreign,
}

/// The rule for an old kind, `None` when the kind is not a known 0.10 kind.
pub fn rule(kind: &str) -> Option<Rule> {
    data::rule(kind)
        .or_else(|| fs::rule(kind))
        .or_else(|| trigger::rule(kind))
        .or_else(|| logic::rule(kind))
        .or_else(|| web::rule(kind))
        .or_else(|| messaging::rule(kind))
        .or_else(|| composite::rule(kind))
}

/// The engine's failure: what a failed node's `error` pin carried in 0.10,
/// rewritten to where 0.11 delivers it (`OldOutput::ErrorEnvelope`).
pub fn engine_error(_: &Config) -> OldOutput {
    OldOutput::ErrorEnvelope
}

pub fn pass(_: &Config, _: &RewriteContext) -> OldOutput {
    OldOutput::Pass
}

/// Whether `kind` is a 0.10 kind: every stored 0.10 kind starts `n.`.
pub fn is_old_kind(kind: &str) -> bool {
    kind.starts_with("n.")
}

/// The keys every 0.10 node may carry that the engine reads, not the node.
pub fn engine_keys(n: &mut NodeRewrite<'_>) {
    n.keep("title");
    n.keep("preview");
    n.keep("ui");
    // The engine-wide timeout was a number of seconds; it is a duration.
    n.duration("timeout_secs", "timeout", "s");
}

/// A comma list or a list, as a list of trimmed words.
pub fn words(value: &Value) -> Option<Vec<String>> {
    super::node::word_list(value)
}

/// The statement keyword a SQL text starts with, lowercased, comments and
/// whitespace skipped.
pub fn sql_keyword(sql: &str) -> String {
    let mut rest = sql.trim_start();
    loop {
        if let Some(after) = rest.strip_prefix("--") {
            rest = after.split_once('\n').map(|(_, r)| r).unwrap_or("").trim_start();
        } else if let Some(after) = rest.strip_prefix("/*") {
            rest = after.split_once("*/").map(|(_, r)| r).unwrap_or("").trim_start();
        } else if let Some(after) = rest.strip_prefix('(') {
            rest = after.trim_start();
        } else {
            break;
        }
    }
    rest.split(|c: char| !c.is_ascii_alphabetic()).next().unwrap_or("").to_ascii_lowercase()
}
