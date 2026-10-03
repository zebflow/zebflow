//! The template half of `web.response.send`, shared with `web.site.generate`:
//! compile a TSX page once (the engine caches it) and render it with a state.

use serde_json::{Map, Value, json};

use super::{NODE_KIND, OUTPUT_PIN_OUT};
use crate::language::LanguageEngine;
use crate::pipeline::PipelineError;
use crate::pipeline::nodes::NodeExecutionOutput;
use crate::rwe::{CompiledTemplate, ReactiveWebEngine, ReactiveWebOptions, TemplateSource};

/// A compiled TSX page held in the engine's render cache.
pub struct CompiledPage {
    pub node_id: String,
    pub template: CompiledTemplate,
}

/// Compile a TSX template into a cached page artifact.
pub fn compile_page(
    node_id: &str,
    template: &TemplateSource,
    options: &ReactiveWebOptions,
    rwe: &dyn ReactiveWebEngine,
    language: &dyn LanguageEngine,
) -> Result<CompiledPage, PipelineError> {
    let compiled_template = rwe
        .compile_template(template, language, options)
        .map_err(|e| {
            PipelineError::new(
                "FW_NODE_WEB_RESPONSE_SEND_COMPILE",
                format!("failed compiling node '{}': {}", node_id, e),
            )
        })?;
    Ok(CompiledPage {
        node_id: node_id.to_string(),
        template: compiled_template,
    })
}

/// Strip private JWT claims from `payload["auth"]` before it reaches the browser.
///
/// Only keys listed in `_zf_public` survive. If no keys are marked public,
/// `auth` is set to `null` (secure by default). Pipeline nodes upstream still
/// see the full claims — this filtering only applies at the render boundary.
fn strip_private_auth_claims(mut payload: Value) -> Value {
    let auth = match payload.get("auth") {
        Some(Value::Object(m)) => m.clone(),
        _ => return payload,
    };

    let public_keys: Vec<String> = match auth.get("_zf_public") {
        Some(Value::Array(arr)) => arr
            .iter()
            .filter_map(|v| v.as_str().map(String::from))
            .collect(),
        _ => vec![],
    };

    if let Some(obj) = payload.as_object_mut() {
        if public_keys.is_empty() {
            obj.insert("auth".to_string(), Value::Null);
        } else {
            let mut public_auth = Map::new();
            for key in &public_keys {
                if let Some(v) = auth.get(key) {
                    public_auth.insert(key.clone(), v.clone());
                }
            }
            obj.insert("auth".to_string(), Value::Object(public_auth));
        }
    }

    payload
}

/// Inject trigger-context fields into state so templates always have
/// `ctx.auth`, `ctx.params`, `ctx.query` and raw URL context regardless of what
/// upstream nodes did to the payload. Explicit page routes take precedence;
/// otherwise the browser pathname wins over the pipeline-relative route.
fn inject_trigger_fields(mut state: Value, metadata: &Value) -> Value {
    // The route the request arrived on. `usePathname` reads this during server
    // rendering; without it the server has no idea what path it is rendering
    // and the browser's `location.pathname` disagrees on the first client
    // render — which is a hydration tear, not a cosmetic difference.
    let route = metadata
        .get("trigger")
        .and_then(|trigger| trigger.get("pathname"))
        .and_then(Value::as_str)
        .or_else(|| metadata.get("route").and_then(Value::as_str));
    if let Some(route) = route {
        if let Value::Object(ref mut map) = state {
            if !map.contains_key("route") {
                map.insert("route".to_string(), Value::String(route.to_string()));
            }
        }
    }

    let Some(trigger) = metadata.get("trigger") else {
        return state;
    };
    let Value::Object(ref mut map) = state else {
        return state;
    };

    // Preserve raw search separately from the legacy query object, including
    // repeated parameters and percent escapes needed by SSR URL hooks.
    // Explicit page-state fields retain their existing precedence.
    for key in &["params", "query", "search", "headers"] {
        if !map.contains_key(*key) {
            if let Some(v) = trigger.get(*key) {
                map.insert(key.to_string(), v.clone());
            }
        }
    }
    // auth: always prefer trigger.auth — it carries _zf_public for correct
    // filtering by strip_private_auth_claims which runs right after.
    if !map.contains_key("auth") || map.get("auth") == Some(&Value::Null) {
        if let Some(auth) = trigger.get("auth") {
            map.insert("auth".to_string(), auth.clone());
        }
    }
    state
}

/// Render a previously compiled page artifact.
pub fn render_compiled_page(
    compiled: &CompiledPage,
    state: Value,
    metadata: Value,
    rwe: &dyn ReactiveWebEngine,
    language: &dyn LanguageEngine,
    request_id: &str,
    enabled_libraries: Vec<String>,
) -> Result<NodeExecutionOutput, PipelineError> {
    let route = metadata
        .get("route")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .unwrap_or("/")
        .to_string();

    // Inject trigger fields before filtering — ensures ctx.auth/params/query are
    // always available in templates even when upstream nodes replaced the payload.
    let state = inject_trigger_fields(state, &metadata);
    // Strip private JWT claims before the payload reaches the browser DOM.
    let state = strip_private_auth_claims(state);

    let rendered = rwe
        .render(
            &compiled.template,
            state,
            language,
            &crate::rwe::RenderContext {
                route,
                request_id: request_id.to_string(),
                metadata,
                enabled_libraries,
            },
        )
        .map_err(|e| {
            PipelineError::new(
                "FW_NODE_WEB_RESPONSE_SEND_RENDER",
                format!("failed rendering node '{}': {}", compiled.node_id, e),
            )
        })?;

    let mut trace = vec![
        format!("node={}", compiled.node_id),
        format!("node_kind={NODE_KIND}"),
        format!("output_pin={OUTPUT_PIN_OUT}"),
    ];
    trace.extend(rendered.trace);

    Ok(NodeExecutionOutput {
        output_pins: vec![OUTPUT_PIN_OUT.to_string()],
        payload: json!({
            "html": rendered.html,
            "compiled_scripts": rendered.compiled_scripts,
            "hydration_payload": rendered.hydration_payload,
        }),
        trace,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn page_route_uses_browser_path_without_changing_pipeline_route() {
        let metadata = json!({
            "route": "/probe",
            "trigger": {"pathname": "/wh/owner/project/probe"}
        });
        let injected = super::inject_trigger_fields(json!({}), &metadata);
        assert_eq!(injected["route"], "/wh/owner/project/probe");
        assert_eq!(metadata["route"], "/probe");
        let explicit = super::inject_trigger_fields(json!({"route": "/custom"}), &metadata);
        assert_eq!(explicit["route"], "/custom");
        let fallback = super::inject_trigger_fields(json!({}), &json!({"route": "/preview"}));
        assert_eq!(fallback["route"], "/preview");
    }

    #[test]
    fn render_boundary_injects_trigger_fields_and_filters_auth_claims() {
        let state = json!({
            "title": "Hello",
            "params": { "slug": "keep-existing" },
            "auth": null
        });
        let metadata = json!({
            "trigger": {
                "params": { "slug": "from-trigger" },
                "query": { "page": "1" },
                "search": "?page=1&tag=one&tag=two",
                "headers": { "x-request-id": "req-123" },
                "auth": {
                    "sub": "user-1",
                    "role": "admin",
                    "secret": "internal-only",
                    "_zf_public": ["sub", "role"]
                }
            }
        });

        let injected = super::inject_trigger_fields(state, &metadata);
        assert_eq!(injected["params"]["slug"], "keep-existing");
        assert_eq!(injected["query"]["page"], "1");
        assert_eq!(injected["search"], "?page=1&tag=one&tag=two");
        assert_eq!(injected["headers"]["x-request-id"], "req-123");
        assert_eq!(injected["auth"]["secret"], "internal-only");

        let filtered = super::strip_private_auth_claims(injected);
        assert_eq!(
            filtered["auth"],
            json!({
                "sub": "user-1",
                "role": "admin"
            })
        );
        assert!(filtered["auth"].get("secret").is_none());
    }
}
