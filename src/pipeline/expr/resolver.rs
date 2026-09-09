//! Resolve `{{ expr }}` expressions in a node config Value before the node executes.
//!
//! All expressions found in one config object are batched into a single Deno
//! pool invocation for efficiency.
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
//! | `$trigger`     | Immutable trigger snapshot (`auth`, `params`, `query`, `headers`) |
//! | `$nodes`       | Map of completed node IDs → their output payloads          |
//! | `$placeholder` | Opaque credential references for composite/WASM nodes. Values are placeholder strings resolved only by platform consumers (HTTP client, script sandbox), never in traces or logs. |
//!
//! # Type preservation
//!
//! Whole-field expressions (`{{ expr }}` and nothing else) return the native JS type.
//! Interpolated expressions (`"Hello {{ name }}!"`) stringify the result and concatenate.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use serde_json::{Value, json};

use super::scanner::{ExprField, Segment, scan};
use crate::language::{
    COMPILE_TARGET_BACKEND, CompileOptions, ExecutionContext, LanguageEngine, ModuleSource,
    SourceKind,
};
use crate::pipeline::PipelineError;

/// Build the flat DSL expression scope used by config expressions and logic nodes.
pub fn build_expression_scope_input(input: &Value, metadata: &Value) -> Value {
    json!({
        "$input":       input,
        "$item":        input.get("item").cloned().unwrap_or(Value::Null),
        "$index":       input.get("index").cloned().unwrap_or(Value::Null),
        "$count":       input.get("count").cloned().unwrap_or(Value::Null),
        "$trigger":     metadata.get("trigger").cloned().unwrap_or(Value::Null),
        "$nodes":       metadata.get("nodes").cloned().unwrap_or_else(|| json!({})),
        "$placeholder": metadata.get("placeholder").cloned().unwrap_or_else(|| json!({})),
    })
}

/// Resolve all `{{ expr }}` expressions in `config` and return the mutated copy.
///
/// Returns the original `config` unchanged (zero mutation) if no expressions are found.
pub fn resolve_config_expressions(
    mut config: Value,
    input: &Value,
    metadata: &Value,
    language: &Arc<dyn LanguageEngine>,
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

    let result_map = match run_output.value {
        Value::Object(m) => m,
        _ => return Ok(config), // Unexpected result shape — leave config unchanged.
    };

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
