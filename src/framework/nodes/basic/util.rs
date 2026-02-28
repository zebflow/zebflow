//! Small helpers shared by built-in framework nodes.

use serde_json::Value;

use crate::framework::FrameworkError;

pub fn metadata_scope(metadata: &Value) -> Result<(&str, &str, &str, &str), FrameworkError> {
    let owner = metadata.get("owner").and_then(Value::as_str).ok_or_else(|| {
        FrameworkError::new("FW_NODE_SCOPE", "missing metadata.owner for project-scoped node")
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
