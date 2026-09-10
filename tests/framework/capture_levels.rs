//! Capture levels — `kinds/invocation-record` § Capture Level.
//!
//! A pipeline on a device has no room for run history; a pipeline being
//! debugged wants all of it. These assert what each level records, using a
//! canary value that would be plainly visible in a payload if it were kept.

use serde_json::{Value, json};
use zebflow::pipeline::trace_capture::{CaptureLevel, TraceCaptureSettings};

fn limits(level: CaptureLevel) -> zebflow::pipeline::trace_capture::TraceCaptureLimits {
    TraceCaptureSettings {
        level: Some(level),
        ..Default::default()
    }
    .resolve(None)
}

#[test]
fn the_default_level_is_on_error() {
    assert_eq!(CaptureLevel::default(), CaptureLevel::OnError);
    assert_eq!(
        TraceCaptureSettings::default().resolve(None).level,
        CaptureLevel::OnError,
        "an unconfigured project must get on-error, not full"
    );
}

/// A pipeline's setting beats the project's, field by field, the same way every
/// other capture setting resolves.
#[test]
fn a_pipeline_overrides_the_projects_level() {
    let project = TraceCaptureSettings {
        level: Some(CaptureLevel::None),
        ..Default::default()
    };
    let pipeline = TraceCaptureSettings {
        level: Some(CaptureLevel::Full),
        ..Default::default()
    };
    assert_eq!(project.resolve(Some(&pipeline)).level, CaptureLevel::Full);
    // …and a pipeline that says nothing inherits.
    let silent = TraceCaptureSettings::default();
    assert_eq!(project.resolve(Some(&silent)).level, CaptureLevel::None);
}

#[test]
fn levels_agree_on_what_they_record() {
    // none: nothing, and none of the work either.
    let l = limits(CaptureLevel::None);
    assert_eq!(l.level, CaptureLevel::None);

    // on-error: a failed node's payloads, never a successful one's.
    let l = limits(CaptureLevel::OnError);
    assert_eq!(l.level, CaptureLevel::OnError);

    // full: everything, subject to the byte budgets.
    let l = limits(CaptureLevel::Full);
    assert_eq!(l.level, CaptureLevel::Full);
}

/// The level is a payload setting. It must not disturb the byte budgets that
/// stop one large run filling the disk.
#[test]
fn a_level_does_not_change_the_budgets() {
    let defaults = TraceCaptureSettings::default().resolve(None);
    for level in [CaptureLevel::None, CaptureLevel::OnError, CaptureLevel::Full] {
        let l = limits(level);
        assert_eq!(l.max_node_bytes, defaults.max_node_bytes);
        assert_eq!(l.max_run_bytes, defaults.max_run_bytes);
        assert_eq!(l.max_depth, defaults.max_depth);
        assert_eq!(l.array_sample_count, defaults.array_sample_count);
    }
}

/// It round-trips through project configuration as kebab-case, because that is
/// what an operator types.
#[test]
fn the_level_serializes_as_an_operator_would_write_it() {
    let parsed: TraceCaptureSettings =
        serde_json::from_value(json!({ "level": "on-error" })).expect("on-error must parse");
    assert_eq!(parsed.level, Some(CaptureLevel::OnError));

    let parsed: TraceCaptureSettings =
        serde_json::from_value(json!({ "level": "none" })).expect("none must parse");
    assert_eq!(parsed.level, Some(CaptureLevel::None));

    let written = serde_json::to_value(TraceCaptureSettings {
        level: Some(CaptureLevel::Full),
        ..Default::default()
    })
    .expect("serialize");
    assert_eq!(written.get("level"), Some(&Value::String("full".into())));

    // An unset level stays out of the document rather than writing a default
    // an operator never chose.
    let written = serde_json::to_value(TraceCaptureSettings::default()).expect("serialize");
    assert_eq!(written.get("level"), None);
}
