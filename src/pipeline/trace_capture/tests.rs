//! Regression coverage for log-only sampling, redaction, and capture budgets.

use super::*;

fn capture_with(settings: TraceCaptureSettings, payload: &Value) -> Value {
    let mut capture = TraceCapture::new(settings.resolve(None));
    capture.begin_node();
    capture.capture(payload)
}

#[test]
fn recursive_sampling_preserves_execution_data_and_reports_omitted_counts() {
    let payload = json!({"rows": [
        {"values": [1, 2, 3], "children": [{"names": ["a", "b"]}, {}]},
        {"values": [4, 5]}
    ]});
    let original = payload.clone();
    let captured = capture_with(
        TraceCaptureSettings {
            array_sample_count: Some(1),
            ..Default::default()
        },
        &payload,
    );
    assert_eq!(payload, original);
    assert_eq!(captured["rows"]["len"], 2);
    assert_eq!(captured["rows"]["omitted"], 1);
    let row = &captured["rows"]["preview"][0];
    assert_eq!(row["values"]["preview"], json!([1]));
    assert_eq!(row["values"]["omitted"], 2);
    assert_eq!(
        row["children"]["preview"][0]["names"]["preview"],
        json!(["a"])
    );
}

#[test]
fn zero_array_sampling_keeps_arrays_but_preserves_other_limits() {
    let settings = TraceCaptureSettings {
        array_sample_count: Some(0),
        max_string_chars: Some(3),
        max_depth: Some(2),
        ..Default::default()
    };
    let captured = capture_with(
        settings,
        &json!({
            "numbers": [0, 1, 2, 3, 4, 5, 6, 7],
            "text": "abcdef",
            "deep": {"object": {"inside": "hidden by depth"}}
        }),
    );
    assert_eq!(captured["numbers"], json!([0, 1, 2, 3, 4, 5, 6, 7]));
    assert_eq!(captured["text"]["preview"], "abc");
    assert_eq!(captured["deep"]["object"]["reason"], "depth");
}

#[test]
fn markers_in_omitted_array_elements_still_redact_retained_strings() {
    let captured = capture_with(
        TraceCaptureSettings {
            array_sample_count: Some(1),
            ..Default::default()
        },
        &json!({"rows": [
            {"message": "request used hidden-secret"},
            {"__zf_private_trace_redact": ["hidden-secret"]}
        ]}),
    );
    assert_eq!(
        captured["rows"]["preview"][0]["message"],
        "request used ••••••"
    );
    assert!(!captured.to_string().contains("hidden-secret"));
    assert!(!captured.to_string().contains("__zf_private"));
}

#[test]
fn shortening_repeated_secrets_never_exposes_a_cut_token_prefix() {
    let secret = "ABCDEFGHIJKLMNOPQRST";
    let captured = capture_with(
        TraceCaptureSettings {
            max_string_chars: Some(10),
            ..Default::default()
        },
        &json!({
            "__zf_private_trace_redact": [secret],
            "message": secret.repeat(2)
        }),
    );
    let message = &captured["message"];
    let preview = message
        .as_str()
        .or_else(|| message["preview"].as_str())
        .unwrap();
    assert!(preview.chars().all(|ch| ch == '•'), "{preview}");
    assert!(preview.chars().count() <= 10);
}

#[test]
fn huge_secret_and_unicode_boundary_are_redacted_without_token_fragments() {
    let secret = "界".repeat(100_000);
    let captured = capture_with(
        TraceCaptureSettings {
            max_string_chars: Some(4),
            ..Default::default()
        },
        &json!({"__zf_private_trace_redact": [&secret], "message": format!("é{secret}tail")}),
    );
    assert_eq!(captured["message"]["preview"], "é•••");
    assert!(!captured.to_string().contains('界'));
}

#[test]
fn child_exception_cannot_unmask_a_sensitive_ancestor() {
    let captured = capture_with(
        TraceCaptureSettings::default(),
        &json!({
            "__zf_private_redact_except_paths": ["password.inner", "response.token"],
            "password": {"inner": "must-stay-hidden"},
            "response": {"token": "deliberately-visible"}
        }),
    );
    assert_eq!(captured["password"]["inner"], "••••••");
    assert_eq!(captured["response"]["token"], "deliberately-visible");
}

#[test]
fn node_allowance_resets_but_run_budget_does_not() {
    let settings = TraceCaptureSettings {
        max_node_bytes: Some(256),
        max_run_bytes: Some(512),
        ..Default::default()
    };
    let mut capture = TraceCapture::new(settings.resolve(None));
    let oversized = json!("x".repeat(400));
    capture.begin_node();
    assert_eq!(capture.capture(&oversized)["__zf_trace_summary"], "bytes");
    assert_eq!(capture.capture(&json!(1))["__zf_trace_summary"], "bytes");
    capture.begin_node();
    assert_eq!(capture.capture(&json!(1)), 1);
    assert_eq!(capture.capture(&oversized)["__zf_trace_summary"], "bytes");
    capture.begin_node();
    assert_eq!(capture.capture(&json!(1))["__zf_trace_summary"], "bytes");
}

#[test]
fn config_and_payload_share_the_node_allowance() {
    let settings = TraceCaptureSettings {
        max_node_bytes: Some(256),
        ..Default::default()
    };
    let mut capture = TraceCapture::new(settings.resolve(None));
    capture.begin_node();
    let config = capture
        .config(&json!({"query": "x".repeat(190), "password": "hidden", "ui": {"x": 1}}))
        .unwrap();
    assert_eq!(config["password"], "••••••");
    assert!(config.get("ui").is_none());
    assert_eq!(
        capture.capture(&json!("x".repeat(50)))["__zf_trace_summary"],
        "bytes"
    );
}

#[test]
fn per_field_inheritance_preserves_explicit_zero() {
    let project = TraceCaptureSettings {
        array_sample_count: Some(7),
        max_depth: Some(3),
        ..Default::default()
    };
    let pipeline = TraceCaptureSettings {
        array_sample_count: Some(0),
        max_node_bytes: Some(2048),
        ..Default::default()
    };
    let resolved = project.resolve(Some(&pipeline));
    assert_eq!(resolved.array_sample_count, 0);
    assert_eq!(resolved.max_depth, 3);
    assert_eq!(resolved.max_node_bytes, 2048);
    assert_eq!(resolved.max_run_bytes, 1_048_576);
}

#[test]
fn multiple_emissions_are_sampled_without_losing_nested_redaction() {
    let outputs = vec![
        NodeExecutionOutput {
            output_pins: vec!["out".into()],
            payload: json!({"values": [1, 2], "password": "hidden"}),
            trace: vec![],
        },
        NodeExecutionOutput {
            output_pins: vec!["out".into()],
            payload: json!({"values": [3, 4]}),
            trace: vec![],
        },
    ];
    let mut capture = TraceCapture::new(
        TraceCaptureSettings {
            array_sample_count: Some(1),
            ..Default::default()
        }
        .resolve(None),
    );
    capture.begin_node();
    let captured = capture.outputs(&outputs);
    assert_eq!(captured["count"], 2);
    assert_eq!(captured["emissions"]["omitted"], 1);
    assert_eq!(
        captured["emissions"]["preview"][0]["values"]["preview"],
        json!([1])
    );
    assert_eq!(captured["emissions"]["preview"][0]["password"], "••••••");
    assert_eq!(outputs[0].payload["password"], "hidden");
}

#[test]
fn sampled_arrays_at_maximum_depth_remain_readable_json() {
    let mut payload = json!("leaf");
    for _ in 0..64 {
        payload = json!([payload, null]);
    }
    let captured = capture_with(
        TraceCaptureSettings {
            array_sample_count: Some(1),
            max_depth: Some(64),
            ..Default::default()
        },
        &payload,
    );
    assert_eq!(captured["__zf_trace_summary"], "array");
    // Invocation storage also uses serde_json's ordinary recursion limit. The
    // capture must remain readable when nested inside a normal trace envelope.
    let persisted = json!([{"input": captured}]);
    let encoded = serde_json::to_vec(&persisted).unwrap();
    let decoded: Value = serde_json::from_slice(&encoded).expect("read persisted nested trace");
    assert!(decoded[0]["input"]["preview"].is_array());
}
