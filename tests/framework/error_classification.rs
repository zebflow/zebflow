//! What the refused/failed split actually does, end to end.
//!
//! Registry lookups alone cannot show this. The bugs these cover were all at
//! *seams*: the code was registered correctly on one side and flattened into
//! the wrong class as it crossed into the other.

use serde_json::json;
use zebflow::pipeline::error_class::{ErrorClass, class_of, wrapper_for};

/// A policy violation cannot succeed on a second attempt. It was classed
/// `Failed` — unregistered codes default to that — so `logic.retry` spent the
/// full attempt budget re-running a script that could never compile.
#[test]
fn a_policy_violation_is_refused_not_retried() {
    assert_eq!(class_of("LANG_DENO_POLICY"), ErrorClass::Refused);
    assert_eq!(class_of("LANG_DENO_SYNTAX"), ErrorClass::Refused);
    assert_eq!(class_of("LANG_DENO_SOURCE_TOO_LARGE"), ErrorClass::Refused);
}

/// The mirror mistake: infrastructure faults must stay retryable.
#[test]
fn an_engine_fault_stays_failed() {
    assert_eq!(class_of("LANG_DENO_ARTIFACT_DECODE"), ErrorClass::Failed);
    assert_eq!(class_of("LANG_COMPILE"), ErrorClass::Failed);
    assert_eq!(class_of("LANG_RUN_DECODE"), ErrorClass::Failed);
}

/// The seam itself: a wrapper must carry the cause's class rather than impose
/// its own. `FW_NODE_SCRIPT_COMPILE` is Failed, and every compile error used to
/// arrive under it regardless of cause.
#[test]
fn the_wrapper_preserves_the_cause_class() {
    let refused = wrapper_for(
        "LANG_DENO_POLICY",
        "FW_NODE_SCRIPT_REJECTED",
        "FW_NODE_SCRIPT_COMPILE",
    );
    assert_eq!(refused, "FW_NODE_SCRIPT_REJECTED");
    assert_eq!(class_of(refused), ErrorClass::Refused);

    let failed = wrapper_for(
        "LANG_DENO_ARTIFACT_DECODE",
        "FW_NODE_SCRIPT_REJECTED",
        "FW_NODE_SCRIPT_COMPILE",
    );
    assert_eq!(failed, "FW_NODE_SCRIPT_COMPILE");
    assert_eq!(class_of(failed), ErrorClass::Failed);
}

/// `FW_EXPR_RUN` is Refused, which was applied to *every* wrapped execution
/// error — so a dead worker became permanently unretryable.
#[test]
fn an_expression_engine_fault_is_not_blamed_on_the_author() {
    assert_eq!(
        wrapper_for("LANG_DENO_ARTIFACT_DECODE", "FW_EXPR_RUN", "FW_EXPR_ENGINE"),
        "FW_EXPR_ENGINE"
    );
    assert_eq!(class_of("FW_EXPR_ENGINE"), ErrorClass::Failed);
    assert_eq!(class_of("FW_EXPR_RUN"), ErrorClass::Refused);
}

/// An expression that throws must say so. It used to evaluate to `null` and the
/// pipeline carried on, so `{{ input.custmer.id }}` (a typo) wrote null into a
/// database and reported success — the platform's most expensive failure mode,
/// because nothing looks wrong.
#[test]
fn a_throwing_expression_is_reported_not_nulled() {
    let engine = std::sync::Arc::new(zebflow::language::DenoSandboxEngine::default())
        as std::sync::Arc<dyn zebflow::language::LanguageEngine>;
    let config = json!({ "value": "{{ input.missing.deep }}" });
    let err = zebflow::pipeline::expr::resolve_config_expressions(
        config,
        &json!({ "present": 1 }),
        &json!({}),
        &engine,
    )
    .expect_err("an expression that throws must not silently become null");
    assert_eq!(err.code, "FW_EXPR_EVAL");
    assert!(
        err.message.contains("input.missing.deep"),
        "the message must name the expression that failed: {}",
        err.message
    );
    assert_eq!(class_of(&err.code), ErrorClass::Refused);
}

/// Optional chaining is how an author asks for a possibly-absent value, and it
/// must keep working — otherwise the fix above turns every defensive expression
/// into an error.
#[test]
fn optional_chaining_still_yields_a_value() {
    let engine = std::sync::Arc::new(zebflow::language::DenoSandboxEngine::default())
        as std::sync::Arc<dyn zebflow::language::LanguageEngine>;
    let out = zebflow::pipeline::expr::resolve_config_expressions(
        json!({ "value": "{{ input.missing?.deep ?? 'fallback' }}" }),
        &json!({ "present": 1 }),
        &json!({}),
        &engine,
    )
    .expect("an author who wrote ?. asked for absence and must get it");
    assert_eq!(out.get("value").and_then(|v| v.as_str()), Some("fallback"));
}

/// `input` is the payload in a logic node's expression, as it is in every
/// `{{ }}` block and in `n.script`.
///
/// Binding only `$input` left `input` pointing at the scope object, so
/// `input.rows` evaluated to undefined and a guard silently took the wrong
/// branch — no error, no trace entry, just the false path. Found by a real
/// pipeline taking a 404 on a row that plainly existed.
#[tokio::test]
async fn a_logic_expression_reads_input_as_the_payload() {
    use zebflow::pipeline::engines::basic::BasicPipelineEngine;
    use zebflow::pipeline::interface::PipelineEngine;
    use zebflow::pipeline::model::PipelineContext;
    use zebflow::platform::shell::parser::build_pipeline_graph;

    let graph = build_pipeline_graph(
        "input-binding",
        "[a] trigger.manual\n\
         [b] logic.if --expr \"input.rows && input.rows.length > 0\"\n\
         [yes] script -- \"return { took: 'true' };\"\n\
         [no] script -- \"return { took: 'false' };\"\n\
         [a] -> [b]\n[b]:true -> [yes]\n[b]:false -> [no]\n",
    )
    .expect("graph");

    let ctx = PipelineContext {
        owner: "test".into(),
        project: "test".into(),
        pipeline: "input-binding".into(),
        request_id: "binding-test".into(),
        route: String::new(),
        input: json!({ "rows": [ { "id": 1 } ] }),
        trigger: None,
        placeholder: None,
    };
    let out = BasicPipelineEngine::default()
        .execute_async(&graph, &ctx)
        .await
        .expect("execution");
    assert_eq!(
        out.value.get("took").and_then(|v| v.as_str()),
        Some("true"),
        "`input.rows` must see the payload, not the scope object"
    );

    // And `$input` keeps working, because documents and pipelines use both.
    let graph = build_pipeline_graph(
        "input-binding-dollar",
        "[a] trigger.manual\n\
         [b] logic.if --expr \"$input.rows.length > 0\"\n\
         [yes] script -- \"return { took: 'true' };\"\n\
         [no] script -- \"return { took: 'false' };\"\n\
         [a] -> [b]\n[b]:true -> [yes]\n[b]:false -> [no]\n",
    )
    .expect("graph");
    let out = BasicPipelineEngine::default()
        .execute_async(&graph, &ctx)
        .await
        .expect("execution");
    assert_eq!(out.value.get("took").and_then(|v| v.as_str()), Some("true"));
}

/// A node declares its own pins, and the DSL reads them.
///
/// `default_pins` used to keep a hardcoded table that duplicated every
/// definition. A kind nobody remembered to add silently got `in`/`out`, so
/// `n.crypto` — which declares `true`/`false` for its verify operations —
/// could not be branched on from the DSL at all: the edge was refused as
/// "'true' is not declared by node". Two sources of truth, and the quiet one
/// won.
#[test]
fn the_dsl_takes_a_nodes_pins_from_its_definition() {
    use zebflow::platform::shell::parser::default_pins;

    for def in zebflow::pipeline::nodes::builtin_node_definitions() {
        // `n.logic.match` sets its pins per instance from `cases`, so its
        // definition cannot answer for one node.
        if def.kind == "n.logic.match" || def.output_pins.is_empty() {
            continue;
        }
        let (inputs, outputs) = default_pins(&def.kind);
        assert_eq!(
            outputs, def.output_pins,
            "{} declares output pins {:?} but the DSL gives it {:?}",
            def.kind, def.output_pins, outputs
        );
        assert_eq!(
            inputs, def.input_pins,
            "{} declares input pins {:?} but the DSL gives it {:?}",
            def.kind, def.input_pins, inputs
        );
    }
}

/// The case that exposed it: a verify operation has to be branchable.
#[test]
fn crypto_verify_pins_are_reachable_from_the_dsl() {
    let (_, outputs) = zebflow::platform::shell::parser::default_pins("n.crypto");
    assert!(outputs.contains(&"true".to_string()), "{outputs:?}");
    assert!(outputs.contains(&"false".to_string()), "{outputs:?}");
}
