use serde_json::json;
use zebflow::rwe::{
    CompileTemplateRequest, CompileTemplateResponse, ProtocolMeta, RenderTemplateRequest,
    RenderTemplateResponse, TemplateSource,
};

#[test]
fn rwe_protocol_envelopes_are_json_roundtrip_friendly() {
    let compile_req = CompileTemplateRequest {
        meta: ProtocolMeta::default(),
        template: TemplateSource {
            id: "page.api".to_string(),
            source_path: None,
            markup: r#"
export const page = {
  head: { title: "Protocol Test" },
  navigation: "history"
};

export default function Page(input) {
  return (
    <Page>
      <h1>{input.title}</h1>
    </Page>
  );
}
"#
            .to_string(),
        },
        options: Default::default(),
    };

    let encoded = serde_json::to_value(&compile_req).expect("encode compile request");
    assert_eq!(
        encoded
            .get("meta")
            .and_then(|v| v.get("version"))
            .and_then(|v| v.as_str()),
        Some("rwe.v1")
    );
    let decoded: CompileTemplateRequest =
        serde_json::from_value(encoded).expect("decode compile request");
    assert_eq!(decoded.template.id, "page.api");

    let compile_resp = CompileTemplateResponse {
        meta: decoded.meta.clone(),
        compiled: zebflow::rwe::CompiledTemplate {
            engine_id: "rwe.noop".to_string(),
            template_id: "page.api".to_string(),
            html_ir: "<html></html>".to_string(),
            control_script_source: Some("return { state: {} };".to_string()),
            compiled_logic: None,
            runtime_bundle: zebflow::rwe::RuntimeBundle {
                name: "rwe-runtime.js".to_string(),
                source: "window.__ZEBFLOW_RWE__={};".to_string(),
            },
            reactive_bindings: Vec::new(),
            diagnostics: Vec::new(),
            needs_runtime_tailwind_rebuild: false,
            tailwind_variant_exact_tokens: Vec::new(),
            tailwind_variant_patterns: Vec::new(),
            options: Default::default(),
        },
    };
    let compile_resp_json = serde_json::to_string(&compile_resp).expect("encode compile response");
    assert!(compile_resp_json.contains("rwe.noop"));

    let render_req = RenderTemplateRequest {
        meta: compile_resp.meta.clone(),
        compiled: compile_resp.compiled.clone(),
        state: json!({ "title": "Hello Adapter" }),
        ctx: zebflow::rwe::RenderContext {
            route: "/api-preview".to_string(),
            request_id: "req-adapter-1".to_string(),
            metadata: json!({ "from": "fastapi" }),
        },
    };
    let render_req_json = serde_json::to_value(&render_req).expect("encode render request");
    assert_eq!(
        render_req_json
            .get("ctx")
            .and_then(|v| v.get("route"))
            .and_then(|v| v.as_str()),
        Some("/api-preview")
    );

    let render_resp = RenderTemplateResponse {
        meta: render_req.meta.clone(),
        output: zebflow::rwe::RenderOutput {
            html: "<h1>Hello Adapter</h1>".to_string(),
            hydration_payload: json!({ "input": { "title": "Hello Adapter" } }),
            trace: vec!["engine=rwe.noop".to_string()],
        },
    };
    let render_resp_json = serde_json::to_string(&render_resp).expect("encode render response");
    assert!(render_resp_json.contains("Hello Adapter"));
}
