//! Small helpers shared by built-in framework nodes.

use std::hash::{Hash, Hasher};

use serde_json::Value;

use crate::pipeline::FrameworkError;
use crate::language::{
    COMPILE_TARGET_BACKEND, CompileOptions, ExecutionContext, LanguageEngine, ModuleSource,
    SourceKind,
};

pub fn metadata_scope(metadata: &Value) -> Result<(&str, &str, &str, &str), FrameworkError> {
    let owner = metadata
        .get("owner")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            FrameworkError::new(
                "FW_NODE_SCOPE",
                "missing metadata.owner for project-scoped node",
            )
        })?;
    let project = metadata
        .get("project")
        .and_then(Value::as_str)
        .ok_or_else(|| {
            FrameworkError::new(
                "FW_NODE_SCOPE",
                "missing metadata.project for project-scoped node",
            )
        })?;
    let pipeline = metadata
        .get("pipeline")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let request_id = metadata
        .get("request_id")
        .and_then(Value::as_str)
        .unwrap_or_default();
    Ok((owner, project, pipeline, request_id))
}

pub fn resolve_path<'a>(root: &'a Value, path: &str) -> Option<&'a Value> {
    let path = path.trim();
    if path.is_empty() {
        return Some(root);
    }

    let mut current = root;
    for segment in path.split('.') {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }
        current = match current {
            Value::Object(map) => map.get(segment)?,
            _ => return None,
        };
    }
    Some(current)
}

pub fn resolve_path_cloned(root: &Value, path: Option<&str>) -> Option<Value> {
    path.and_then(|p| resolve_path(root, p).cloned())
}

pub fn resolve_array_values(root: &Value, path: Option<&str>) -> Vec<Value> {
    let Some(value) = resolve_path_cloned(root, path) else {
        return Vec::new();
    };
    match value {
        Value::Array(items) => items,
        other => vec![other],
    }
}

pub fn eval_deno_expr(
    language: &dyn LanguageEngine,
    expr: &str,
    input: &Value,
    metadata: &Value,
) -> Result<Value, FrameworkError> {
    let expr = expr.trim();
    if expr.is_empty() {
        return Err(FrameworkError::new(
            "FW_NODE_BINDING_EXPR",
            "binding expression must not be empty",
        ));
    }

    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    expr.hash(&mut hasher);
    let source = format!("return ({expr});");
    let module = ModuleSource {
        id: format!("pipeline:binding:{:x}", hasher.finish()),
        source_path: None,
        kind: SourceKind::Tsx,
        code: source,
    };
    let ir = language
        .parse(&module)
        .map_err(|err| FrameworkError::new("FW_NODE_BINDING_PARSE", err.to_string()))?;
    let compiled = language
        .compile(
            &ir,
            &CompileOptions {
                target: COMPILE_TARGET_BACKEND.to_string(),
                optimize_level: 1,
                emit_trace_hints: false,
            },
        )
        .map_err(|err| FrameworkError::new("FW_NODE_BINDING_COMPILE", err.to_string()))?;
    let ctx = ExecutionContext {
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
        metadata: metadata.clone(),
    };
    language
        .run(&compiled, input.clone(), &ctx)
        .map(|out| out.value)
        .map_err(|err| FrameworkError::new("FW_NODE_BINDING_RUN", err.to_string()))
}
