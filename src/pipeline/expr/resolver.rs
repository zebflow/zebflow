//! Resolve `{{ expr }}` expressions in a node config Value before the node executes.
//!
//! Each expression runs natively when [`super::native`] compiles it (once,
//! cached by its text) and has an exact answer for the values at hand. The
//! rest of one config's expressions are batched into a single Deno pool
//! invocation.
//!
//! # Expression-mode sandbox security
//!
//! The sandbox runs with **`capabilities: []`** (no `n.*` bridge — no database access,
//! no HTTP, no side effects at all) and a tight **`maxOps: 500`** budget.
//! Pre-existing permanent locks (`eval`, `Function`, `fetch`, timers) remain in effect.
//! This makes expression evaluation hermetically sandboxed to pure value computation only.
//!
//! # Scope variables
//!
//! | Variable       | Contents                                                   |
//! |----------------|-------------------------------------------------------------|
//! | `$input`       | Current node's input payload                               |
//! | `$item`        | Current foreach item (`input.item` when present)           |
//! | `$index`       | Current foreach index (`input.index` when present)         |
//! | `$count`       | Current foreach count (`input.count` when present)         |
//! | `$trigger`     | Immutable trigger snapshot (`body`, `files`, `auth`, `params`, `query`, `headers`) |
//! | `$nodes`       | The answers of this node's ancestors by id; a skipped one is `null`, and a path through it is `null` (`$nodes.a.b` is read as `$nodes.a?.b`) |
//! | `$placeholder` | Opaque credential references for composite/WASM nodes. Values are placeholder strings resolved only by platform consumers (HTTP client, script sandbox), never in traces or logs. |
//!
//! # Type preservation
//!
//! Whole-field expressions (`{{ expr }}` and nothing else) return the native JS type.
//! Interpolated expressions (`"Hello {{ name }}!"`) stringify the result and concatenate.

use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, LazyLock, RwLock};

use serde_json::{Value, json};

use super::native::{self, Names, Scope};
use super::scanner::{ExprField, Segment, scan};
use crate::language::{
    COMPILE_TARGET_BACKEND, CompileOptions, ExecutionContext, LanguageEngine, ModuleSource,
    SourceKind,
};
use crate::pipeline::PipelineError;

/// Build the flat DSL expression scope used by config expressions and logic nodes.
/// Metadata key: `{ item, index, count }` of the `logic.foreach` frame a node
/// runs in. The engine sets it on every node of the frame, in the metadata,
/// not the payload, so a node in the loop body may replace the payload
/// without cutting a later `$item` off from its element.
pub const FOREACH_METADATA_KEY: &str = "foreach";

/// `$item`, `$index` and `$count` come from the loop run the engine carries in
/// the metadata, so they hold anywhere down a `logic.foreach` branch; the
/// payload's own fields are the fallback for a node run outside the engine.
pub fn build_expression_scope_input(input: &Value, metadata: &Value) -> Value {
    let run = metadata.get(FOREACH_METADATA_KEY).filter(|v| v.is_object());
    let loop_field = |name: &str| {
        run.and_then(|r| r.get(name))
            .or_else(|| input.get(name))
            .cloned()
            .unwrap_or(Value::Null)
    };
    json!({
        "$input":       input,
        "$item":        loop_field("item"),
        "$index":       loop_field("index"),
        "$count":       loop_field("count"),
        "$trigger":     metadata.get("trigger").cloned().unwrap_or(Value::Null),
        "$nodes":       metadata.get("nodes").cloned().unwrap_or_else(|| json!({})),
        "$placeholder": metadata.get("placeholder").cloned().unwrap_or_else(|| json!({})),
    })
}

/// `$nodes.a.b.c` → `$nodes.a?.b?.c` (and `$nodes["a"].b` → `$nodes["a"]?.b`).
///
/// A node that was skipped — a branch not taken — or has not run is `null`
/// in `$nodes` (`node-conventions.md` §4), and a path through it is `null`,
/// never an error: `{{ $nodes.big.text ?? $nodes.small.text }}` joins two
/// branches. Only the member accesses after the node's name are made
/// optional; text inside string literals is left alone.
pub fn optional_nodes_paths(expr: &str) -> String {
    const MARKER: &str = "$nodes";
    let bytes = expr.as_bytes();
    let ident = |b: u8| b == b'_' || b == b'$' || b.is_ascii_alphanumeric();
    let mut out = String::with_capacity(expr.len() + 8);
    let mut i = 0;
    // The end of a `[ … ]` group starting at `start`, quotes respected.
    let bracket_end = |start: usize| -> usize {
        let mut depth = 0usize;
        let mut j = start;
        let mut quote: Option<u8> = None;
        while j < bytes.len() {
            let b = bytes[j];
            match quote {
                Some(_) if b == b'\\' => j += 1,
                Some(q) if b == q => quote = None,
                Some(_) => {}
                None if b == b'\'' || b == b'"' || b == b'`' => quote = Some(b),
                None if b == b'[' => depth += 1,
                None if b == b']' => {
                    depth -= 1;
                    if depth == 0 {
                        return j + 1;
                    }
                }
                None => {}
            }
            j += 1;
        }
        bytes.len()
    };
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'\'' || b == b'"' || b == b'`' {
            let start = i;
            i += 1;
            while i < bytes.len() && bytes[i] != b {
                if bytes[i] == b'\\' {
                    i += 1;
                }
                i += 1;
            }
            i = (i + 1).min(bytes.len());
            out.push_str(&expr[start..i]);
            continue;
        }
        let at_marker = expr[i..].starts_with(MARKER)
            && (i == 0 || !(ident(bytes[i - 1]) || bytes[i - 1] == b'.'))
            && !bytes.get(i + MARKER.len()).is_some_and(|&n| ident(n));
        if !at_marker {
            let ch = expr[i..].chars().next().unwrap_or_default();
            out.push(ch);
            i += ch.len_utf8().max(1);
            continue;
        }
        out.push_str(MARKER);
        i += MARKER.len();
        // The node's own name stays as written.
        if bytes.get(i) == Some(&b'.') {
            let start = i;
            i += 1;
            while i < bytes.len() && ident(bytes[i]) {
                i += 1;
            }
            out.push_str(&expr[start..i]);
        } else if bytes.get(i) == Some(&b'[') {
            let end = bracket_end(i);
            out.push_str(&expr[i..end]);
            i = end;
        } else {
            continue;
        }
        // Every access after it is optional.
        loop {
            if expr[i..].starts_with("?.") {
                out.push_str("?.");
                i += 2;
                continue;
            }
            match bytes.get(i) {
                Some(b'.') if bytes.get(i + 1).is_some_and(|&n| ident(n)) => {
                    out.push_str("?.");
                    i += 1;
                    let start = i;
                    while i < bytes.len() && ident(bytes[i]) {
                        i += 1;
                    }
                    out.push_str(&expr[start..i]);
                }
                Some(b'[') => {
                    let end = bracket_end(i);
                    if !out.ends_with("?.") {
                        out.push_str("?.");
                    }
                    out.push_str(&expr[i..end]);
                    i = end;
                }
                Some(&n) if ident(n) && out.ends_with("?.") => {
                    let start = i;
                    while i < bytes.len() && ident(bytes[i]) {
                        i += 1;
                    }
                    out.push_str(&expr[start..i]);
                }
                _ => break,
            }
        }
    }
    out
}

/// Each expression's native form, compiled the first time it is seen:
/// `None` when it needs Deno. Keyed by the expression as written.
static NATIVE: LazyLock<RwLock<HashMap<String, Option<Arc<native::Expr>>>>> = LazyLock::new(Default::default);

/// Expressions are config text, so the set is the pipelines' own; a server
/// that somehow sees more than this many starts the cache again.
const NATIVE_KEPT: usize = 50_000;

fn native_of(expr: &str) -> Option<Arc<native::Expr>> {
    if let Some(found) = NATIVE.read().unwrap_or_else(|e| e.into_inner()).get(expr) {
        return found.clone();
    }
    let compiled = native::compile(&optional_nodes_paths(expr), Names::Expression).ok().map(Arc::new);
    let mut cache = NATIVE.write().unwrap_or_else(|e| e.into_inner());
    if cache.len() >= NATIVE_KEPT {
        cache.clear();
    }
    cache.insert(expr.to_string(), compiled.clone());
    compiled
}

/// Which engine evaluates `{{ }}`. Only tests ask for Deno alone, to hold
/// the native answers to Deno's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExprEngine {
    /// Native where the expression and its values allow, Deno for the rest.
    NativeFirst,
    /// Every expression in Deno.
    DenoOnly,
}

/// Resolve all `{{ expr }}` expressions in `config` and return the mutated copy.
///
/// Returns the original `config` unchanged (zero mutation) if no expressions are found.
///
/// Each expression runs natively when [`native`] compiles it and its values
/// have an exact answer there; the rest go to Deno in one batch, as before.
/// A config whose expressions are all native never reaches Deno.
pub fn resolve_config_expressions(
    config: Value,
    input: &Value,
    metadata: &Value,
    language: &Arc<dyn LanguageEngine>,
) -> Result<Value, PipelineError> {
    resolve_config_expressions_with(config, input, metadata, language, ExprEngine::NativeFirst)
}

pub fn resolve_config_expressions_with(
    mut config: Value,
    input: &Value,
    metadata: &Value,
    language: &Arc<dyn LanguageEngine>,
    engine: ExprEngine,
) -> Result<Value, PipelineError> {
    let fields = scan(&config);
    if fields.is_empty() {
        return Ok(config);
    }

    // Deduplicate expressions and assign stable indices.
    let mut exprs: Vec<String> = Vec::new();
    for field in &fields {
        for seg in &field.segments {
            if let Segment::Expr(e) = seg {
                if !exprs.contains(e) {
                    exprs.push(e.clone());
                }
            }
        }
    }

    // Native first. An answer is kept under `e{i}` as the Deno batch keys
    // it; `undefined` is no key at all, as `JSON.stringify` leaves it.
    let mut result_map = serde_json::Map::new();
    let mut deno: Vec<usize> = Vec::new();
    if engine == ExprEngine::NativeFirst {
        let scope = Scope::of_node(input, metadata);
        for (i, expr) in exprs.iter().enumerate() {
            match native_of(expr).map(|e| e.eval(&scope).map(native::Js::into_json)) {
                Some(Ok(Some(value))) => {
                    result_map.insert(format!("e{i}"), value);
                }
                Some(Ok(None)) => {}
                Some(Err(native::Fallback)) | None => deno.push(i),
            }
        }
    } else {
        deno = (0..exprs.len()).collect();
    }
    if !deno.is_empty() {
        let batch: Vec<&str> = deno.iter().map(|i| exprs[*i].as_str()).collect();
        // An unexpected answer shape leaves the config as written, as it
        // always has.
        let Some(answers) = run_in_deno(&batch, input, metadata, language)? else {
            return Ok(config);
        };
        for (j, i) in deno.iter().enumerate() {
            for tag in ["e", "x"] {
                if let Some(v) = answers.get(&format!("{tag}{j}")) {
                    result_map.insert(format!("{tag}{i}"), v.clone());
                }
            }
        }
    }

    // An expression that threw is the author's mistake, and it is reported as
    // one. Silently substituting null makes a wrong pipeline look like a
    // working one, which is the most expensive failure mode a platform has.
    // Authors who genuinely want a missing value write `?.` or `??`, both of
    // which the sandbox supports.
    for (i, expr) in exprs.iter().enumerate() {
        let Some(message) = result_map.get(&format!("x{i}")).and_then(Value::as_str) else {
            continue;
        };
        let where_used = fields
            .iter()
            .find(|f| {
                f.segments
                    .iter()
                    .any(|seg| matches!(seg, Segment::Expr(e) if e == expr))
            })
            .map(|f| f.ptr.clone())
            .unwrap_or_default();
        return Err(PipelineError::new(
            "FW_EXPR_EVAL",
            format!("expression {{{{ {expr} }}}} at config{where_used} failed: {message}"),
        ));
    }

    // Substitute evaluation results back into the config.
    for field in &fields {
        let replacement = build_replacement(field, &exprs, &result_map);
        apply_json_ptr(&mut config, &field.ptr, replacement);
    }

    Ok(config)
}

/// A config's `{{ }}`, found and compiled once, for a pipeline compiled when
/// it is activated: [`CompiledConfig::fill`] gives the config
/// [`resolve_config_expressions`] would, without scanning the config or
/// looking an expression up on each run.
pub struct CompiledConfig {
    fields: Vec<ExprField>,
    exprs: Vec<String>,
    native: Vec<Option<Arc<native::Expr>>>,
}

impl CompiledConfig {
    /// `None` when the config holds no `{{ }}`.
    pub fn of(config: &Value) -> Option<CompiledConfig> {
        let fields = scan(config);
        if fields.is_empty() {
            return None;
        }
        let mut exprs: Vec<String> = Vec::new();
        for field in &fields {
            for seg in &field.segments {
                if let Segment::Expr(e) = seg
                    && !exprs.contains(e)
                {
                    exprs.push(e.clone());
                }
            }
        }
        let native = exprs.iter().map(|e| native_of(e)).collect();
        Some(CompiledConfig { fields, exprs, native })
    }

    /// `config` with its expressions filled in. An expression native code
    /// does not compile, or has no exact answer for these values, sends the
    /// whole config to [`resolve_config_expressions`], which asks Deno for
    /// that one; the answer is the same either way.
    pub fn fill(
        &self,
        config: &Value,
        input: &Value,
        metadata: &Value,
        language: &Arc<dyn LanguageEngine>,
    ) -> Result<Value, PipelineError> {
        let slow = || resolve_config_expressions(config.clone(), input, metadata, language);
        let scope = Scope::of_node(input, metadata);
        let mut result_map = serde_json::Map::new();
        for (i, expr) in self.native.iter().enumerate() {
            let Some(expr) = expr else { return slow() };
            match expr.eval(&scope).map(native::Js::into_json) {
                Ok(Some(value)) => {
                    result_map.insert(format!("e{i}"), value);
                }
                Ok(None) => {}
                Err(native::Fallback) => return slow(),
            }
        }
        let mut out = config.clone();
        for field in &self.fields {
            let replacement = build_replacement(field, &self.exprs, &result_map);
            apply_json_ptr(&mut out, &field.ptr, replacement);
        }
        Ok(out)
    }
}

/// The Deno batch: every expression in one sandbox trip, each answer under
/// `e{i}` and each thrown error under `x{i}`, `i` its place in `exprs`.
fn run_in_deno(
    exprs: &[&str],
    input: &Value,
    metadata: &Value,
    language: &Arc<dyn LanguageEngine>,
) -> Result<Option<serde_json::Map<String, Value>>, PipelineError> {

    // Build the batch JS function body.
    // Each expression is assigned to a plain var inside a try/catch so one error doesn't
    // abort the batch.  Flat try/catch (no IIFEs) avoids any interaction with loop-guard
    // instrumentation.
    let mut body = String::from(
        // The scope object arrives as `input`; alias it first so the payload
        // can take that name. `input` is what every doc, every example and
        // the script node call the payload — binding only `$input` was why
        // `{{ input.field }}` silently evaluated to undefined everywhere it
        // was used as a whole value.
        "var __scope = input;\n\
         var input = __scope.$input;\n\
         var $input = input;\n\
         var $item = __scope.$item;\n\
         var $index = __scope.$index;\n\
         var $count = __scope.$count;\n\
         var $trigger = __scope.$trigger || null;\n\
         var $nodes = __scope.$nodes || {};\n\
         var $placeholder = __scope.$placeholder || {};\n",
    );
    // Each expression evaluates into its own slot so one failure does not abort
    // the batch — but the failure is *kept*, not discarded. An empty catch here
    // turned every throwing expression into `null`, which the pipeline then
    // wrote onward as if it were the author's intended value: a typo in
    // `{{ input.custmer.id }}` inserted null and reported success.
    for (i, expr) in exprs.iter().enumerate() {
        let expr = optional_nodes_paths(expr);
        body.push_str(&format!("var _e{i} = null; var _x{i} = null;\n"));
        body.push_str(&format!(
            "try {{ _e{i} = ({expr}); }} catch (_zfe) {{ _x{i} = String((_zfe && _zfe.message) || _zfe); }}\n"
        ));
    }
    body.push_str("return {\n");
    for (i, _) in exprs.iter().enumerate() {
        body.push_str(&format!("  \"e{i}\": _e{i},\n"));
        let comma = if i + 1 < exprs.len() { "," } else { "" };
        body.push_str(&format!("  \"x{i}\": _x{i}{comma}\n"));
    }
    body.push_str("};\n");

    // Compile the batch body using the language engine.
    let module = ModuleSource {
        id: format!("pipeline:expr:{}", hash_str(&body)),
        source_path: None,
        kind: SourceKind::Tsx,
        code: body,
    };
    let ir = language
        .parse(&module)
        .map_err(|e| PipelineError::new("FW_EXPR_PARSE", e.to_string()))?;
    let compiled = language
        .compile(
            &ir,
            &CompileOptions {
                target: COMPILE_TARGET_BACKEND.to_string(),
                optimize_level: 1,
                emit_trace_hints: false,
            },
        )
        .map_err(|e| PipelineError::new("FW_EXPR_COMPILE", e.to_string()))?;

    // Build the scope input: all variables are namespaced under `$` keys so
    // they don't collide with any `input` fields the user might have.
    let scope_input = build_expression_scope_input(input, metadata);

    // Build execution context with expression-mode sandbox patch.
    // capabilities: [] → n is Object.freeze({}) — no n.* bridge at all.
    // maxOps: 500 → tight budget for pure value expressions.
    let mut patched_metadata = metadata.clone();
    if let Value::Object(ref mut map) = patched_metadata {
        map.insert(
            "languageRunPatch".to_string(),
            json!({ "capabilities": [], "maxOps": 500 }),
        );
    }

    let expr_ctx = ExecutionContext {
        project: metadata
            .get("project")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        pipeline: metadata
            .get("pipeline")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        request_id: metadata
            .get("request_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        trigger: metadata.get("trigger").cloned().unwrap_or(Value::Null),
        metadata: patched_metadata,
    };

    let run_output = language
        .run(&compiled, scope_input, &expr_ctx)
        .map_err(|e| {
            // FW_EXPR_RUN is Refused, which is right for an expression that
            // misbehaved and wrong for a worker that died — that mistake made
            // genuine infrastructure faults permanently unretryable.
            PipelineError::new(
                crate::pipeline::error_class::wrapper_for(
                    &e.code,
                    "FW_EXPR_RUN",
                    "FW_EXPR_ENGINE",
                ),
                e.to_string(),
            )
        })?;

    match run_output.value {
        Value::Object(m) => Ok(Some(m)),
        _ => Ok(None),
    }
}

/// Build the replacement value for one `ExprField`.
fn build_replacement(
    field: &ExprField,
    exprs: &[String],
    results: &serde_json::Map<String, Value>,
) -> Value {
    if field.is_whole {
        // Single whole-field expression → return native JS type.
        let Segment::Expr(expr) = &field.segments[0] else {
            return Value::Null;
        };
        let idx = exprs.iter().position(|e| e == expr).unwrap_or(0);
        results
            .get(&format!("e{idx}"))
            .cloned()
            .unwrap_or(Value::Null)
    } else {
        // Interpolated — concatenate all parts as strings.
        let mut s = String::new();
        for seg in &field.segments {
            match seg {
                Segment::Literal(lit) => s.push_str(lit),
                Segment::Expr(expr) => {
                    let idx = exprs.iter().position(|e| e == expr).unwrap_or(0);
                    let val = results.get(&format!("e{idx}")).unwrap_or(&Value::Null);
                    let rendered = match val {
                        Value::String(sv) => sv.clone(),
                        Value::Null => String::new(),
                        other => other.to_string(),
                    };
                    s.push_str(&rendered);
                }
            }
        }
        Value::String(s)
    }
}

/// Write `value` at the location identified by RFC 6901 JSON Pointer `ptr`.
///
/// Intermediate objects are auto-vivified (created as `null` if missing).
/// If navigation fails (non-object/array encountered mid-path, or array index
/// out of bounds) the write is silently skipped.
fn apply_json_ptr(root: &mut Value, ptr: &str, value: Value) {
    if ptr.is_empty() {
        return;
    }
    let tokens: Vec<String> = ptr
        .trim_start_matches('/')
        .split('/')
        .map(|s| s.replace("~1", "/").replace("~0", "~"))
        .collect();

    if tokens.is_empty() {
        return;
    }
    let Some((last, parents)) = tokens.split_last() else {
        return;
    };

    // Thread a mutable reference through the parent path (same pattern as serde_json's
    // `pointer_mut`).
    let parent = parents
        .iter()
        .try_fold(root as &mut Value, |node, tok| match node {
            Value::Object(map) => Some(map.entry(tok.clone()).or_insert(Value::Null)),
            Value::Array(arr) => tok.parse::<usize>().ok().and_then(|i| arr.get_mut(i)),
            _ => None,
        });

    let Some(parent) = parent else {
        return;
    };

    match parent {
        Value::Object(map) => {
            map.insert(last.clone(), value);
        }
        Value::Array(arr) => {
            if let Ok(i) = last.parse::<usize>() {
                if i < arr.len() {
                    arr[i] = value;
                }
            }
        }
        _ => {}
    }
}

fn hash_str(s: &str) -> String {
    let mut h = DefaultHasher::new();
    s.hash(&mut h);
    format!("{:x}", h.finish())
}

#[cfg(test)]
mod tests {
    use super::optional_nodes_paths;

    /// Every access after the node's name becomes optional; the name, other
    /// scopes and string literals are left as written.
    #[test]
    fn a_path_through_nodes_is_optional_after_the_node_name() {
        for (expr, want) in [
            ("$nodes.big.text ?? $nodes.small.text", "$nodes.big?.text ?? $nodes.small?.text"),
            ("$nodes.a.b.c", "$nodes.a?.b?.c"),
            ("$nodes['geo-convert'].ref", "$nodes['geo-convert']?.ref"),
            ("$nodes.q.query.rows[0].id", "$nodes.q?.query?.rows?.[0]?.id"),
            ("$nodes.a?.b.c", "$nodes.a?.b?.c"),
            ("$nodes.a", "$nodes.a"),
            ("$nodes.q.query.rows.map(r => r.id)", "$nodes.q?.query?.rows?.map(r => r.id)"),
            ("input.a.b + '$nodes.x.y'", "input.a.b + '$nodes.x.y'"),
            ("my$nodes.a.b", "my$nodes.a.b"),
        ] {
            assert_eq!(optional_nodes_paths(expr), want, "{expr}");
        }
    }

    /// Native and Deno give the same config for every expression the
    /// generator makes, over values that pull JavaScript's rules apart:
    /// integers and doubles, `-0`, big integers, numeric strings, empty
    /// strings, non-ASCII text, `null` and missing keys, number-like object
    /// keys, skipped nodes. Where native has no exact answer it hands the
    /// expression to Deno, so a failure here is a wrong native answer.
    #[test]
    fn native_answers_are_denos_answers() {
        use super::{ExprEngine, resolve_config_expressions_with};
        use crate::language::{DenoSandboxEngine, LanguageEngine};
        use serde_json::{Value, json};
        use std::sync::Arc;

        let language: Arc<dyn LanguageEngine> = Arc::new(DenoSandboxEngine::default());
        let input = json!({
            "a": 1, "b": 2.5, "c": -3, "z": 0, "neg": -0.0, "f1": 1.0, "big": 9007199254740993u64, "huge": 1e300,
            "s": "héllo", "n": "42", "fs": "1.5", "ws": " 7 ", "e": "", "w": "abc", "u": "日本",
            "t": true, "f": false, "nil": null,
            "arr": [1, "x", null, { "k": 1 }, 2.0],
            "obj": { "k": "v", "2": "two", "1": "one", "inner": { "d": 4 } },
            "webhook": { "body": { "id": 7, "tags": ["a", "b"] } }
        });
        let metadata = json!({
            "trigger": { "body": { "id": 7 }, "query": { "q": "x" } },
            "nodes": { "n1": { "x": 1, "rows": [{ "id": 3 }] }, "skipped": null },
        });
        const PATHS: &[&str] = &[
            "input.a", "input.b", "input.c", "input.z", "input.neg", "input.f1", "input.big", "input.huge",
            "input.s", "input.n", "input.fs", "input.ws", "input.e", "input.w", "input.u", "input.t", "input.f",
            "input.nil", "input.missing", "input.arr", "input.arr[0]", "input.arr[1]", "input.arr[4]", "input.arr[9]",
            "input.arr.length", "input.obj", "input.obj.k", "input.obj['2']", "input.obj[1]", "input.obj.inner.d",
            "input.s.length", "input.w[1]", "input.missing?.x", "input.nil?.x.y", "$input.a", "$trigger.body.id",
            "$trigger.query", "$nodes.n1.x", "$nodes.n1.rows[0].id", "$nodes.skipped.x", "$nodes.absent.y",
            "input.webhook.body", "input.webhook.body.tags", "$item", "$index", "input.obj.toString", "input.missing.deep",
        ];
        const LITERALS: &[&str] = &[
            "0", "1", "-1", "2.5", "0.1", "1e21", "1e-7", "3", "'x'", "''", "'7'", "' 12 '", "'é'", "true", "false",
            "null", "undefined", "NaN", "Infinity", "{ b: 1, 2: 'two', a: input.missing }", "[1, input.missing, 'z']",
            "{}", "[]",
        ];
        const BINARY: &[&str] = &[
            "+", "-", "*", "/", "%", "<", "<=", ">", ">=", "===", "!==", "==", "!=", "&&", "||", "??",
        ];
        let mut seed: u64 = 0x2545_F491_4F6C_DD1D;
        let mut next = |n: usize| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            (seed % n as u64) as usize
        };
        fn make(depth: usize, next: &mut dyn FnMut(usize) -> usize) -> String {
            let pick = if depth == 0 { next(2) } else { next(7) };
            match pick {
                0 => PATHS[next(PATHS.len())].to_string(),
                1 => LITERALS[next(LITERALS.len())].to_string(),
                2 | 3 => format!("({} {} {})", make(depth - 1, next), BINARY[next(BINARY.len())], make(depth - 1, next)),
                4 => format!("({} ? {} : {})", make(depth - 1, next), make(depth - 1, next), make(depth - 1, next)),
                5 => format!("{}({})", ["!", "-", "+", "typeof "][next(4)], make(depth - 1, next)),
                _ => format!("{{ k: {}, j: [{}] }}", make(depth - 1, next), make(depth - 1, next)),
            }
        }
        let (mut native_answered, mut cases) = (0, 0);
        for _ in 0..4000 {
            let expr = make(3, &mut next);
            let config = json!({ "whole": format!("{{{{ {expr} }}}}"), "text": format!("<{{{{ {expr} }}}}>") });
            let deno = resolve_config_expressions_with(config.clone(), &input, &metadata, &language, ExprEngine::DenoOnly);
            let native = resolve_config_expressions_with(config, &input, &metadata, &language, ExprEngine::NativeFirst);
            cases += 1;
            if super::native_of(&expr).is_some_and(|e| e.eval(&crate::pipeline::expr::native::Scope::of_node(&input, &metadata)).is_ok()) {
                native_answered += 1;
            }
            match (&deno, &native) {
                (Ok(d), Ok(n)) => assert_eq!(d, n, "{expr}"),
                (Err(d), Err(n)) => assert_eq!((d.code, &d.message), (n.code, &n.message), "{expr}"),
                _ => panic!("{expr}: deno {deno:?} native {native:?}"),
            }
        }
        eprintln!("{cases} expressions, {native_answered} answered natively, all equal to Deno");
        assert!(native_answered * 2 > cases, "the generator should mostly hit the native path");
        let _: Value = Value::Null;
    }
}
