//! One node's rewrite: the old config is taken key by key into the new one,
//! so a key no rule took is reported rather than dropped.

use serde_json::{Map, Value, json};

use super::RewriteContext;
use super::graph::{OldGraph, nodes_root};

/// What a node becomes, and what its rewrite could not say.
pub struct NodeRewrite<'g> {
    pub graph: &'g OldGraph,
    pub context: &'g RewriteContext,
    pub index: usize,
    pub old_kind: String,
    /// The old config not yet taken (its expressions already rewritten).
    pub old: Map<String, Value>,
    pub kind: String,
    pub config: Map<String, Value>,
    pub changes: Vec<String>,
    /// Behaviour that differs in 0.11 and cannot be written away; reported,
    /// never blocking.
    pub notes: Vec<String>,
    pub unresolved: Vec<String>,
    /// Output pins renamed: old pin → new pin.
    pub pins: Vec<(String, String)>,
}

impl<'g> NodeRewrite<'g> {
    pub fn new(graph: &'g OldGraph, context: &'g RewriteContext, index: usize, old: Map<String, Value>) -> Self {
        let old_kind = graph.nodes[index].kind.clone();
        Self {
            graph,
            context,
            index,
            kind: old_kind.clone(),
            old_kind,
            old,
            config: Map::new(),
            changes: Vec::new(),
            notes: Vec::new(),
            unresolved: Vec::new(),
            pins: Vec::new(),
        }
    }

    /// A behaviour difference the owner should know about.
    pub fn behaviour(&mut self, text: impl Into<String>) {
        self.notes.push(text.into());
    }

    /// Whether the old config has `key` with a non-empty value.
    pub fn has(&self, key: &str) -> bool {
        self.old.get(key).is_some_and(|v| !is_empty(v))
    }

    pub fn peek(&self, key: &str) -> Option<&Value> {
        self.old.get(key).filter(|v| !is_empty(v))
    }

    pub fn id(&self) -> &str {
        &self.graph.nodes[self.index].id
    }

    pub fn kind(&mut self, kind: &str) {
        if self.old_kind != kind {
            self.changes.push(format!("kind {} → {kind}", self.old_kind));
        }
        self.kind = kind.to_string();
    }

    pub fn note(&mut self, text: impl Into<String>) {
        self.changes.push(text.into());
    }

    pub fn unresolved(&mut self, text: impl Into<String>) {
        self.unresolved.push(text.into());
    }

    /// The old value, taken; empty strings and nulls are taken as absent.
    pub fn take(&mut self, key: &str) -> Option<Value> {
        match self.old.remove(key) {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) if s.trim().is_empty() => None,
            Some(value) => Some(value),
        }
    }

    pub fn take_str(&mut self, key: &str) -> Option<String> {
        match self.take(key)? {
            Value::String(s) => Some(s),
            other => Some(other.to_string()),
        }
    }

    pub fn take_bool(&mut self, key: &str) -> Option<bool> {
        match self.take(key)? {
            Value::Bool(b) => Some(b),
            Value::String(s) if s == "true" => Some(true),
            Value::String(s) if s == "false" => Some(false),
            other => {
                self.unresolved(format!("`{key}` is `{other}`, not a switch"));
                None
            }
        }
    }

    /// The old key set under the same name, unchanged.
    pub fn keep(&mut self, key: &str) {
        if let Some(value) = self.take(key) {
            self.config.insert(key.to_string(), value);
        }
    }

    /// The old key set under a new name, its value unchanged.
    pub fn rename(&mut self, old: &str, new: &str) {
        if let Some(value) = self.take(old) {
            if old != new {
                self.changes.push(format!("{old} → {new}"));
            }
            self.config.insert(new.to_string(), value);
        }
    }

    pub fn set(&mut self, key: &str, value: Value) {
        self.config.insert(key.to_string(), value);
    }

    /// A 0.10 default written out because the 0.11 default differs.
    pub fn set_default(&mut self, key: &str, value: Value, why: &str) {
        if !self.config.contains_key(key) {
            self.changes.push(format!("{key} = {value} written out ({why})"));
            self.config.insert(key.to_string(), value);
        }
    }

    /// An old number in `unit` (`s`, `ms`) as a 0.11 duration string.
    pub fn duration(&mut self, old: &str, new: &str, unit: &str) {
        let Some(value) = self.take(old) else { return };
        match duration_value(&value, unit) {
            Some(text) => {
                self.changes.push(format!("{old} {value} → {new} {text}"));
                self.config.insert(new.to_string(), Value::String(text));
            }
            None => self.unresolved(format!("`{old}` is `{value}`, not a number of {unit}")),
        }
    }

    /// Where `input.<path>` of this node's input is in 0.11, as an
    /// expression body (no braces); `Err` says why it cannot be said, and a
    /// key the 0.10 payload did not hold is `Ok(None)`.
    pub fn locate(&self, path: &[&str]) -> Result<Option<String>, String> {
        let path: Vec<String> = path.iter().map(|s| s.to_string()).collect();
        match self.graph.resolve_input(self.index, &path) {
            Ok(origin) => {
                let root = if self.graph.shadowed(&origin, None) || self.graph.ambiguous(self.index, &origin) {
                    nodes_root(&self.graph.nodes[origin.producer].id)
                } else {
                    "input".to_string()
                };
                Ok(Some(origin.render(&root)))
            }
            Err(failure) if failure.dead => Ok(None),
            Err(failure) => Err(failure.why),
        }
    }

    /// The 0.11 `{{ expression }}` for what the old node read implicitly
    /// at `input.<path>` (`files.file`), resolved through the graph; a key
    /// the 0.10 payload never held is reported.
    pub fn implicit(&mut self, path: &[&str]) -> Option<String> {
        match self.locate(path) {
            Ok(Some(expr)) => Some(format!("{{{{ {expr} }}}}")),
            Ok(None) => {
                self.unresolved(format!(
                    "the 0.10 node read `input.{}` without a flag, and no upstream node answers it",
                    path.join(".")
                ));
                None
            }
            Err(why) => {
                self.unresolved(format!("the 0.10 node read `input.{}` without a flag: {why}", path.join(".")));
                None
            }
        }
    }

    /// The 0.11 expression for the whole payload the old node read
    /// implicitly (a body defaulting to the payload).
    pub fn implicit_whole(&mut self) -> Option<String> {
        match self.graph.whole_input_expr(self.index, "input", Some("$nodes")) {
            Ok((expr, dropped)) => {
                if !dropped.is_empty() {
                    self.behaviour(format!("the payload sent leaves out {} (no 0.11 equivalent)", dropped.join(", ")));
                }
                Some(format!("{{{{ {expr} }}}}"))
            }
            Err(why) => {
                self.unresolved(format!("the 0.10 node read its whole payload without a flag, and that payload cannot be rebuilt in 0.11: {why}"));
                None
            }
        }
    }

    /// An expression written against the 0.10 payload (`input.message ||
    /// input.text`), rewritten for 0.11 as read at this node.
    pub fn old_expression(&mut self, expression: &str) -> Option<String> {
        let rewritten = super::refs::rewrite_code(
            self.graph,
            self.index,
            expression,
            super::expr::Wrap::Expression,
            super::refs::Scope::Node,
        );
        self.notes.extend(rewritten.notes);
        if !rewritten.unresolved.is_empty() {
            self.unresolved.extend(rewritten.unresolved);
            return None;
        }
        Some(rewritten.text)
    }

    pub fn pin(&mut self, old: &str, new: &str) {
        self.pins.push((old.to_string(), new.to_string()));
    }

    /// Every old key no rule took: one the 0.10.12 node read is reported;
    /// one it did not know (left by an older release) is dropped, as 0.10
    /// ignored it.
    pub fn leftovers(&mut self) {
        let keys: Vec<String> = self.old.keys().cloned().collect();
        for key in keys {
            let value = self.old.remove(&key).unwrap_or(Value::Null);
            if is_empty(&value) {
                continue;
            }
            if super::kinds::known::known(&self.old_kind, &key) {
                self.unresolved(format!("config `{key}` = {value} has no 0.11 equivalent"));
            } else {
                self.note(format!("config `{key}` dropped: the 0.10.12 node did not read it"));
            }
        }
    }
}

fn is_empty(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(s) => s.trim().is_empty(),
        Value::Array(a) => a.is_empty(),
        Value::Object(o) => o.is_empty(),
        _ => false,
    }
}

/// A number of `unit` as a duration string (`600` s → `"600s"`); a whole
/// `{{ expression }}` keeps its expression and gains the unit.
pub fn duration_value(value: &Value, unit: &str) -> Option<String> {
    match value {
        Value::Number(n) => {
            let number = n.as_f64()?;
            if number < 0.0 {
                return None;
            }
            Some(format!("{}{unit}", trim_number(number)))
        }
        Value::String(s) => {
            let t = s.trim();
            if let Ok(number) = t.parse::<f64>() {
                return duration_value(&json!(number), unit);
            }
            let inner = t.strip_prefix("{{")?.strip_suffix("}}")?.trim();
            if inner.contains("{{") || inner.contains("}}") {
                return None;
            }
            Some(format!("{{{{ ({inner}) + '{unit}' }}}}"))
        }
        _ => None,
    }
}

fn trim_number(number: f64) -> String {
    if number.fract() == 0.0 {
        format!("{}", number as i64)
    } else {
        format!("{number}")
    }
}

/// A comma list (`"a, b"`) or a list as an array of trimmed words.
pub fn word_list(value: &Value) -> Option<Vec<String>> {
    match value {
        Value::String(s) => Some(s.split(',').map(str::trim).filter(|w| !w.is_empty()).map(str::to_string).collect()),
        Value::Array(items) => items.iter().map(|v| v.as_str().map(|s| s.trim().to_string())).collect(),
        _ => None,
    }
}
