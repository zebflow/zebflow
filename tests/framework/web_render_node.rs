use serde_json::json;
use zebflow::framework::nodes::basic::web_render::{
    self, Config as WebRenderConfig, Node as WebRenderNode,
};
use zebflow::framework::nodes::{FrameworkNode, NodeExecutionInput};
use zebflow::language::NoopLanguageEngine;
use zebflow::rwe::{NoopReactiveWebEngine, TemplateSource};

#[test]
fn web_render_node_interface_lives_in_framework_nodes() {
    let rwe = NoopReactiveWebEngine;
    let lang = NoopLanguageEngine;

    let template = TemplateSource {
        id: "page.home".into(),
        source_path: None,
        markup: "<h1>{{ state.title }}</h1>".into(),
    };
    let config = WebRenderConfig {
        template_id: "page.home".into(),
        route: "/home".into(),
        ..Default::default()
    };

    let compiled = WebRenderNode::compile("node.web.home", &config, &template, &rwe, &lang)
        .expect("compile web render node");
    let node = WebRenderNode::new(compiled.clone());

    assert_eq!(node.kind(), web_render::NODE_KIND);
    assert_eq!(node.input_pins(), &[web_render::INPUT_PIN_IN]);
    assert_eq!(
        node.output_pins(),
        &[web_render::OUTPUT_PIN_OUT, web_render::OUTPUT_PIN_ERROR]
    );

    let out = node
        .execute(NodeExecutionInput {
            node_id: "node.web.home".into(),
            input_pin: web_render::INPUT_PIN_IN.into(),
            payload: json!({"title":"Hello"}),
            metadata: json!({"source":"framework"}),
        })
        .expect("execute node");
    assert_eq!(out.output_pin, web_render::OUTPUT_PIN_OUT);

    let rendered = web_render::render_with_engines(
        &compiled,
        json!({"title":"Hello"}),
        json!({"source":"framework"}),
        &rwe,
        &lang,
        "req-123",
    )
    .expect("render via pure rwe/language");
    assert_eq!(rendered.output_pin, web_render::OUTPUT_PIN_OUT);
    assert!(
        rendered
            .payload
            .get("html")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .contains("<h1>")
    );
}
