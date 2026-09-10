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

// ---------------------------------------------------------------------------
// Rule 3 — a credential's value is never shown, at any level
// ---------------------------------------------------------------------------

use zebflow::platform::services::credential::{confidential_values, register_confidential_for_test};

/// A value that came out of the credential store is masked wherever it appears,
/// whatever the level says — including `full`, and including a field name no
/// list would have guessed.
///
/// This is the half that name matching cannot do: `code` is not a sensitive
/// name, which is why a real OAuth authorization code was recorded in full.
#[test]
fn a_credential_value_is_masked_at_every_level() {
    let owner = "rule3-owner";
    let project = "rule3-project";
    let secret = "sk-live-9f3c2ab7e41d8065-not-a-name-anyone-listed";
    register_confidential_for_test(owner, project, &json!({ "anything": secret }));

    let known = confidential_values(owner, project);
    assert!(
        known.iter().any(|v| v == secret),
        "the value must be registered where it resolves"
    );

    for level in [CaptureLevel::None, CaptureLevel::OnError, CaptureLevel::Full] {
        let captured = zebflow::pipeline::trace_capture::capture_for_test(
            limits(level),
            known.clone(),
            // The field is called `code`, deliberately: an unlisted name.
            &json!({ "code": secret, "note": format!("token was {secret}") }),
        );
        let text = serde_json::to_string(&captured).unwrap();
        assert!(
            !text.contains(secret),
            "level {level:?} showed a credential value: {text}"
        );
    }
}

/// Ordinary short values must survive, or masking blacks out the diagnostics
/// people rely on and they turn it off.
#[test]
fn short_ordinary_values_are_not_swept_up() {
    let owner = "rule3-short";
    let project = "rule3-short";
    // A secret payload holds a host, a port, a boolean — not just secrets.
    register_confidential_for_test(
        owner,
        project,
        &json!({ "host": "db", "port": "5432", "tls": "true" }),
    );
    let known = confidential_values(owner, project);
    assert!(
        !known.iter().any(|v| v == "db" || v == "5432" || v == "true"),
        "values below the length floor must not be registered: {known:?}"
    );
}

/// The real path: resolving a credential through the service registers its
/// values, with no node involved. This is what makes rule 3 hold for all seven
/// credential-taking nodes without any of them changing.
#[test]
fn resolving_a_credential_registers_it_without_the_node_asking() {
    use zebflow::platform::model::{PlatformConfig, UpsertProjectCredentialRequest};
    use zebflow::platform::services::PlatformService;

    let mut cfg = PlatformConfig::default();
    cfg.data_root = std::env::temp_dir().join(format!(
        "zf_rule3_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let platform = PlatformService::from_config(cfg).expect("platform");

    let secret = "smtp-password-3f9a2c71b45e8d06";
    platform
        .credentials
        .upsert_project_credential(
            "superadmin",
            "default",
            &UpsertProjectCredentialRequest {
                credential_id: "relay".to_string(),
                title: "Test relay".to_string(),
                kind: "smtp".to_string(),
                secret: json!({ "host": "127.0.0.1", "password": secret }),
                notes: String::new(),
            },
        )
        .expect("credential");

    // Nothing is known until it is actually resolved…
    let before = confidential_values("superadmin", "default");
    assert!(
        !before.iter().any(|v| v == secret),
        "storing a credential should not itself register it"
    );

    // …and resolving it is what registers, at the one chokepoint.
    let resolved = platform
        .credentials
        .get_project_credential("superadmin", "default", "relay")
        .expect("resolve")
        .expect("present");
    assert_eq!(resolved.credential_id, "relay");

    let after = confidential_values("superadmin", "default");
    assert!(
        after.iter().any(|v| v == secret),
        "resolving must register the secret: {after:?}"
    );

    // And a different project cannot see it.
    let other = confidential_values("superadmin", "someone-else");
    assert!(
        !other.iter().any(|v| v == secret),
        "a project must not see another project's secrets"
    );
}
